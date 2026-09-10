use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

const JOURNAL_FILE_NAME: &str = ".transaction.json";
const COMMITTED_JOURNAL_FILE_NAME: &str = ".transaction.committed.json";
const PREVIOUS_JOURNAL_FORMAT_VERSION: u32 = 2;
const JOURNAL_FORMAT_VERSION: u32 = 3;
const MAX_JOURNAL_SIZE: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct FileUpdate {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TransactionPhase {
    Prepared,
    Committed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TransactionOperation {
    #[default]
    Update,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Journal {
    #[serde(default)]
    format_version: u32,
    #[serde(default)]
    transaction_id: Option<String>,
    #[serde(default)]
    phase: Option<TransactionPhase>,
    entries: Vec<JournalEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct JournalEntry {
    #[serde(default, skip_serializing_if = "operation_is_update")]
    operation: TransactionOperation,
    target: PathBuf,
    temporary: PathBuf,
    backup: PathBuf,
    existed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    creation_marker: Option<PathBuf>,
}

fn operation_is_update(operation: &TransactionOperation) -> bool {
    *operation == TransactionOperation::Update
}

#[derive(Debug)]
struct ValidatedJournal {
    phase: TransactionPhase,
    is_legacy: bool,
    transaction_id: Option<String>,
    entries: Vec<ValidatedEntry>,
}

#[derive(Debug)]
struct ValidatedEntry {
    operation: TransactionOperation,
    target: PathBuf,
    temporary: PathBuf,
    backup: PathBuf,
    existed: bool,
    creation_marker: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
struct EntryState {
    target: bool,
    temporary: bool,
    backup: bool,
    creation_marker: bool,
}

#[derive(Debug)]
struct UpdateTargetPlan {
    target: PathBuf,
    directories_to_create: Vec<PathBuf>,
}

/// Detect an interrupted transaction without trusting project-controlled journal data.
///
/// A non-empty journal is rejected without changing any target or artifact. Call
/// [`recover_transaction_with_allowed_targets`] only after the caller independently
/// derives and explicitly approves every exact target, including project-internal
/// targets.
pub fn recover_transaction(project_root: &Path) -> Result<()> {
    recover_transaction_with_allowed_targets(project_root, &[])
}

/// Recover an interrupted transaction with exact, trusted target grants.
///
/// `allowed_targets` must come from trusted application state or explicit user
/// approval, never from `.transaction.json` itself. It must list every journal
/// target, including files inside the project. Directories are not accepted as
/// grants, and every existing path component must be free of symbolic links.
pub fn recover_transaction_with_allowed_targets(
    project_root: &Path,
    allowed_targets: &[PathBuf],
) -> Result<()> {
    if !project_root.exists() {
        return Ok(());
    }
    let project_root = dunce::canonicalize(project_root).with_context(|| {
        format!(
            "failed to resolve transaction project root: {}",
            project_root.display()
        )
    })?;
    let prepared_path = project_root.join(JOURNAL_FILE_NAME);
    let committed_path = project_root.join(COMMITTED_JOURNAL_FILE_NAME);
    let prepared = read_journal(&prepared_path)?;
    let committed = read_journal(&committed_path)?;
    if prepared.is_none() && committed.is_none() {
        return Ok(());
    }

    let allowed_targets = allowed_targets
        .iter()
        .map(|path| canonical_target_path(&project_root, path))
        .collect::<Result<HashSet<_>>>()?;
    let journal = select_recovery_journal(prepared, committed)?;
    let validated = validate_journal(&project_root, journal, &allowed_targets).map_err(|error| {
        anyhow::anyhow!(
            "transaction journal is unsafe or inconsistent; no files were changed; manual recovery is required: {error}"
        )
    })?;
    validate_all_entry_states(&validated).map_err(|error| {
        anyhow::anyhow!(
            "transaction artifacts are inconsistent; no files were changed; manual recovery is required: {error}"
        )
    })?;

    match validated.phase {
        TransactionPhase::Prepared => rollback_entries(&validated.entries)?,
        TransactionPhase::Committed => cleanup_committed_entries(&validated.entries)?,
    }

    // The committed record must outlive the prepared record. If cleanup is
    // interrupted after removing the prepared record, the committed record alone
    // still instructs the next recovery to keep the complete new file set.
    remove_regular_file_if_present(&prepared_path)?;
    remove_regular_file_if_present(&committed_path)?;
    sync_directory(&project_root)?;
    Ok(())
}

/// Atomically replace a set of files with a recovery journal.
pub fn write_transaction(project_root: &Path, updates: &[FileUpdate]) -> Result<()> {
    write_transaction_with_deletions(project_root, updates, &[])
}

/// Atomically replace files and remove exact, project-internal files.
///
/// Deletions are intentionally separate from [`FileUpdate`] so existing callers
/// keep their API and cannot accidentally interpret empty contents as removal.
/// Unlike updates, deletion targets must be inside `project_root`.
pub(crate) fn write_transaction_with_deletions(
    project_root: &Path,
    updates: &[FileUpdate],
    deletions: &[PathBuf],
) -> Result<()> {
    fs::create_dir_all(project_root)?;
    let project_root = dunce::canonicalize(project_root).with_context(|| {
        format!(
            "failed to resolve transaction project root: {}",
            project_root.display()
        )
    })?;

    // Resolve and validate every requested path before creating any target parent.
    // External update directories must already exist; only project-internal leaf
    // directories may be created after the complete preflight succeeds.
    let update_plans = updates
        .iter()
        .map(|update| plan_update_target(&project_root, &update.path))
        .collect::<Result<Vec<_>>>()?;
    let deletion_candidates = deletions
        .iter()
        .map(|path| {
            let absolute = live_absolute_path(&project_root, path)?;
            let target = canonical_target_path(&project_root, &absolute)?;
            if !path_is_within(&target, &project_root) {
                bail!(
                    "transaction deletion target is outside the project: {}",
                    target.display()
                );
            }
            Ok(target)
        })
        .collect::<Result<Vec<_>>>()?;
    let recovery_targets = update_plans
        .iter()
        .map(|plan| plan.target.clone())
        .chain(deletion_candidates.iter().cloned())
        .collect::<Vec<_>>();
    recover_transaction_with_allowed_targets(&project_root, &recovery_targets)?;

    for plan in &update_plans {
        create_planned_directories(plan)?;
    }
    let update_targets = update_plans
        .iter()
        .map(|plan| canonical_target_path(&project_root, &plan.target))
        .collect::<Result<Vec<_>>>()?;
    let deletion_targets = deletion_candidates
        .into_iter()
        .filter_map(|target| match fs::symlink_metadata(&target) {
            Ok(_) => Some(Ok(target)),
            Err(error) if error.kind() == ErrorKind::NotFound => None,
            Err(error) => Some(Err(error.into())),
        })
        .collect::<Result<Vec<_>>>()?;

    let recovery_targets = update_targets
        .iter()
        .chain(&deletion_targets)
        .cloned()
        .collect::<Vec<_>>();

    let transaction_id = Uuid::now_v7().to_string();
    let mut journal = Journal {
        format_version: JOURNAL_FORMAT_VERSION,
        transaction_id: Some(transaction_id.clone()),
        phase: Some(TransactionPhase::Prepared),
        entries: Vec::with_capacity(updates.len() + deletions.len()),
    };
    let prepared_path = project_root.join(JOURNAL_FILE_NAME);
    let committed_path = project_root.join(COMMITTED_JOURNAL_FILE_NAME);
    let mut claimed_paths = HashSet::new();
    let preparation = (|| -> Result<()> {
        for target in &update_targets {
            reject_reserved_target(&project_root, target)?;
            let (temporary, backup, creation_marker) =
                expected_artifact_paths(target, &transaction_id, !target.exists())?;
            for path in [target, &temporary, &backup] {
                if !claimed_paths.insert(path.clone()) {
                    bail!(
                        "transaction contains duplicate or overlapping path: {}",
                        path.display()
                    );
                }
            }
            if let Some(marker) = &creation_marker {
                if !claimed_paths.insert(marker.clone()) {
                    bail!(
                        "transaction contains duplicate or overlapping path: {}",
                        marker.display()
                    );
                }
            }

            ensure_regular_or_missing(target)?;
            ensure_regular_or_missing(&temporary)?;
            ensure_regular_or_missing(&backup)?;
            if temporary.exists() || backup.exists() {
                bail!(
                    "transaction artifact already exists for target: {}",
                    target.display()
                );
            }

            let existed = target.exists();
            journal.entries.push(JournalEntry {
                operation: TransactionOperation::Update,
                target: target.clone(),
                temporary,
                backup,
                existed,
                creation_marker,
            });
        }
        for target in &deletion_targets {
            reject_reserved_target(&project_root, target)?;
            let (temporary, backup, _) = expected_artifact_paths(target, &transaction_id, false)?;
            for path in [target, &temporary, &backup] {
                if !claimed_paths.insert(path.clone()) {
                    bail!(
                        "transaction contains duplicate or overlapping path: {}",
                        path.display()
                    );
                }
            }

            ensure_regular_or_missing(target)?;
            ensure_regular_or_missing(&temporary)?;
            ensure_regular_or_missing(&backup)?;
            if !target.exists() {
                bail!(
                    "transaction deletion target disappeared while preparing: {}",
                    target.display()
                );
            }
            if temporary.exists() || backup.exists() {
                bail!(
                    "transaction artifact already exists for target: {}",
                    target.display()
                );
            }

            journal.entries.push(JournalEntry {
                operation: TransactionOperation::Delete,
                target: target.clone(),
                temporary,
                backup,
                existed: true,
                creation_marker: None,
            });
        }
        Ok(())
    })();
    preparation?;

    let allowed_targets = recovery_targets.iter().cloned().collect::<HashSet<_>>();
    // Persist the complete intent before creating any target-adjacent artifact.
    // A crash can therefore never leave an unjournaled `.tmp` or `.new` file.
    persist_new_journal(&project_root, &prepared_path, &journal, "prepare")?;

    let staging = (|| -> Result<()> {
        for (update, entry) in updates.iter().zip(&journal.entries) {
            debug_assert_eq!(entry.operation, TransactionOperation::Update);
            let parent = entry
                .target
                .parent()
                .context("transaction target has no parent")?;
            if let Some(marker) = &entry.creation_marker {
                // The marker is the proof that a newly present target belongs to
                // this transaction, so make it durable before its temporary file.
                write_new_durable_file(marker, transaction_id.as_bytes())?;
                sync_directory(parent)?;
            }
            write_new_durable_file(&entry.temporary, &update.contents)?;
            sync_directory(parent)?;
        }
        Ok(())
    })();
    if let Err(error) = staging {
        return match recover_transaction_with_allowed_targets(&project_root, &recovery_targets) {
            Ok(()) => Err(error),
            Err(recovery_error) => Err(error.context(format!(
                "failed to roll back transaction preparation: {recovery_error:#}"
            ))),
        };
    }

    let validated = validate_journal(&project_root, journal.clone(), &allowed_targets)?;
    validate_all_entry_states(&validated)?;

    for entry in &validated.entries {
        ensure_regular_or_missing(&entry.target)?;
        ensure_regular_or_missing(&entry.temporary)?;
        ensure_regular_or_missing(&entry.backup)?;
        match entry.operation {
            TransactionOperation::Update => {
                if entry.existed {
                    fs::rename(&entry.target, &entry.backup).with_context(|| {
                        format!(
                            "failed to move {} to {}",
                            entry.target.display(),
                            entry.backup.display()
                        )
                    })?;
                } else if entry.target.exists() {
                    bail!(
                        "transaction target appeared while preparing write: {}",
                        entry.target.display()
                    );
                }
                fs::rename(&entry.temporary, &entry.target).with_context(|| {
                    format!(
                        "failed to move {} to {}",
                        entry.temporary.display(),
                        entry.target.display()
                    )
                })?;
            }
            TransactionOperation::Delete => {
                fs::rename(&entry.target, &entry.backup).with_context(|| {
                    format!(
                        "failed to move deleted file {} to {}",
                        entry.target.display(),
                        entry.backup.display()
                    )
                })?;
            }
        }
        sync_directory(
            entry
                .target
                .parent()
                .context("transaction target has no parent")?,
        )?;
    }

    let mut committed_journal = journal;
    committed_journal.phase = Some(TransactionPhase::Committed);
    persist_new_journal(&project_root, &committed_path, &committed_journal, "commit")?;

    // From this point forward recovery must finish the committed new state, even
    // if removing one of the backups fails midway.
    recover_transaction_with_allowed_targets(&project_root, &recovery_targets)
}

fn select_recovery_journal(
    prepared: Option<Journal>,
    committed: Option<Journal>,
) -> Result<Journal> {
    match (prepared, committed) {
        (Some(prepared), Some(committed)) => {
            validate_journal_header(&prepared)?;
            validate_journal_header(&committed)?;
            if committed.phase != Some(TransactionPhase::Committed) {
                bail!("committed transaction record does not have committed phase");
            }
            if prepared.format_version != committed.format_version
                || prepared.transaction_id != committed.transaction_id
                || prepared.entries != committed.entries
            {
                bail!("prepared and committed transaction records do not match");
            }
            Ok(committed)
        }
        (Some(journal), None) => {
            validate_journal_header(&journal)?;
            Ok(journal)
        }
        (None, Some(journal)) => {
            validate_journal_header(&journal)?;
            if journal.phase != Some(TransactionPhase::Committed) {
                bail!("orphan transaction record is not committed");
            }
            Ok(journal)
        }
        (None, None) => unreachable!("caller checked that a journal exists"),
    }
}

fn validate_journal_header(journal: &Journal) -> Result<()> {
    match journal.format_version {
        0 => {
            if journal.transaction_id.is_some() || journal.phase.is_some() {
                bail!("legacy transaction journal contains versioned fields");
            }
            if journal.entries.iter().any(|entry| {
                entry.operation != TransactionOperation::Update || entry.creation_marker.is_some()
            }) {
                bail!("legacy transaction journal contains versioned entry fields");
            }
        }
        PREVIOUS_JOURNAL_FORMAT_VERSION | JOURNAL_FORMAT_VERSION => {
            let transaction_id = journal
                .transaction_id
                .as_deref()
                .context("transaction journal has no transaction id")?;
            let parsed = Uuid::parse_str(transaction_id)
                .context("transaction journal has an invalid transaction id")?;
            if parsed.to_string() != transaction_id {
                bail!("transaction journal id is not in canonical UUID form");
            }
            journal
                .phase
                .context("transaction journal has no transaction phase")?;
            if journal.format_version == PREVIOUS_JOURNAL_FORMAT_VERSION
                && journal
                    .entries
                    .iter()
                    .any(|entry| entry.operation != TransactionOperation::Update)
            {
                bail!("v2 transaction journal contains a v3 deletion entry");
            }
        }
        version => bail!("unsupported transaction journal version: {version}"),
    }
    Ok(())
}

fn validate_journal(
    project_root: &Path,
    journal: Journal,
    allowed_targets: &HashSet<PathBuf>,
) -> Result<ValidatedJournal> {
    validate_journal_header(&journal)?;
    let is_legacy = journal.format_version == 0;
    let format_version = journal.format_version;
    let phase = journal.phase.unwrap_or(TransactionPhase::Prepared);
    let transaction_id = journal.transaction_id.as_deref();
    let mut claimed_paths = HashSet::new();
    let mut entries = Vec::with_capacity(journal.entries.len());

    for entry in journal.entries {
        let target = canonical_target_path(project_root, &entry.target)?;
        let target_is_internal = path_is_within(&target, project_root);
        if entry.operation == TransactionOperation::Delete && !target_is_internal {
            bail!(
                "transaction deletion target is outside the project: {}",
                target.display()
            );
        }
        if !allowed_targets.contains(&target) {
            bail!(
                "transaction target was not independently approved for recovery: {}",
                target.display()
            );
        }
        reject_reserved_target(project_root, &target)?;

        let (temporary, backup, creation_marker) = if let Some(transaction_id) = transaction_id {
            if entry.operation == TransactionOperation::Delete {
                if format_version != JOURNAL_FORMAT_VERSION {
                    bail!("deletion entries require the current transaction journal version");
                }
                if !entry.existed || entry.creation_marker.is_some() {
                    bail!("transaction deletion entry has invalid target metadata");
                }
            }
            let expected = expected_artifact_paths(
                &target,
                transaction_id,
                entry.operation == TransactionOperation::Update && !entry.existed,
            )?;
            require_same_stored_path(project_root, &entry.temporary, &expected.0, "temporary")?;
            require_same_stored_path(project_root, &entry.backup, &expected.1, "backup")?;
            match (&entry.creation_marker, &expected.2) {
                (Some(stored), Some(expected)) => {
                    require_same_stored_path(project_root, stored, expected, "creation marker")?;
                }
                (None, None) => {}
                _ => bail!("transaction creation marker does not match target state"),
            }
            expected
        } else {
            validate_legacy_artifact_paths(project_root, &target, &entry)?
        };

        for path in [&target, &temporary, &backup] {
            if !claimed_paths.insert(path.clone()) {
                bail!(
                    "transaction journal contains overlapping path: {}",
                    path.display()
                );
            }
            ensure_regular_or_missing(path)?;
        }
        if let Some(marker) = &creation_marker {
            if !claimed_paths.insert(marker.clone()) {
                bail!(
                    "transaction journal contains overlapping path: {}",
                    marker.display()
                );
            }
            ensure_regular_or_missing(marker)?;
        }

        entries.push(ValidatedEntry {
            operation: entry.operation,
            target,
            temporary,
            backup,
            existed: entry.existed,
            creation_marker,
        });
    }

    Ok(ValidatedJournal {
        phase,
        is_legacy,
        transaction_id: journal.transaction_id,
        entries,
    })
}

fn validate_all_entry_states(journal: &ValidatedJournal) -> Result<()> {
    for entry in &journal.entries {
        let state = EntryState {
            target: entry.target.exists(),
            temporary: entry.temporary.exists(),
            backup: entry.backup.exists(),
            creation_marker: entry
                .creation_marker
                .as_ref()
                .is_some_and(|path| path.exists()),
        };
        if state.creation_marker {
            let expected = journal
                .transaction_id
                .as_deref()
                .context("transaction creation marker has no transaction id")?;
            let marker = entry
                .creation_marker
                .as_deref()
                .context("transaction creation marker path is missing")?;
            if fs::read(marker)? != expected.as_bytes() {
                bail!(
                    "transaction creation marker has invalid contents: {}",
                    marker.display()
                );
            }
        }
        match journal.phase {
            TransactionPhase::Prepared => validate_prepared_state(entry, state, journal.is_legacy)?,
            TransactionPhase::Committed => validate_committed_state(entry, state)?,
        }
    }
    Ok(())
}

fn validate_prepared_state(
    entry: &ValidatedEntry,
    state: EntryState,
    is_legacy: bool,
) -> Result<()> {
    let valid = if entry.operation == TransactionOperation::Delete {
        entry.existed
            && !state.creation_marker
            && matches!(
                (state.target, state.temporary, state.backup),
                (true, false, false) | (false, false, true)
            )
    } else if entry.existed {
        matches!(
            (state.target, state.temporary, state.backup),
            (true, true, false)
                | (false, true, true)
                | (true, false, true)
                | (false, false, true)
                | (true, false, false)
        ) && !state.creation_marker
    } else if is_legacy {
        // A legacy journal has no proof that a present target was created by the
        // transaction. Removing it could delete an unrelated file, so only the
        // unambiguous pre-swap/already-rolled-back states are automatic.
        !state.target && !state.backup && !state.creation_marker
    } else {
        matches!(
            (
                state.target,
                state.temporary,
                state.backup,
                state.creation_marker
            ),
            (false, true, false, true)
                | (true, false, false, true)
                | (false, false, false, true)
                | (false, false, false, false)
        )
    };
    if !valid {
        bail!(
            "invalid prepared transaction state for target {}",
            entry.target.display()
        );
    }
    Ok(())
}

fn validate_committed_state(entry: &ValidatedEntry, state: EntryState) -> Result<()> {
    let valid = match entry.operation {
        TransactionOperation::Update => {
            state.target
                && !state.temporary
                && if entry.existed {
                    !state.creation_marker
                } else {
                    !state.backup
                }
        }
        TransactionOperation::Delete => !state.target && !state.temporary && !state.creation_marker,
    };
    if !valid {
        bail!(
            "invalid committed transaction state for target {}",
            entry.target.display()
        );
    }
    Ok(())
}

fn rollback_entries(entries: &[ValidatedEntry]) -> Result<()> {
    for entry in entries.iter().rev() {
        ensure_regular_or_missing(&entry.target)?;
        ensure_regular_or_missing(&entry.temporary)?;
        ensure_regular_or_missing(&entry.backup)?;
        if entry.existed {
            if entry.backup.exists() {
                remove_regular_file_if_present(&entry.target)?;
                fs::rename(&entry.backup, &entry.target)?;
            }
        } else if entry
            .creation_marker
            .as_ref()
            .is_some_and(|path| path.exists())
        {
            remove_regular_file_if_present(&entry.target)?;
        }
        remove_regular_file_if_present(&entry.temporary)?;
        if let Some(marker) = &entry.creation_marker {
            remove_regular_file_if_present(marker)?;
        }
        sync_directory(
            entry
                .target
                .parent()
                .context("transaction target has no parent")?,
        )?;
    }
    Ok(())
}

fn cleanup_committed_entries(entries: &[ValidatedEntry]) -> Result<()> {
    for entry in entries {
        ensure_regular_or_missing(&entry.target)?;
        ensure_regular_or_missing(&entry.temporary)?;
        ensure_regular_or_missing(&entry.backup)?;
        remove_regular_file_if_present(&entry.temporary)?;
        remove_regular_file_if_present(&entry.backup)?;
        if let Some(marker) = &entry.creation_marker {
            remove_regular_file_if_present(marker)?;
        }
        sync_directory(
            entry
                .target
                .parent()
                .context("transaction target has no parent")?,
        )?;
    }
    Ok(())
}

fn read_journal(path: &Path) -> Result<Option<Journal>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(
            "transaction journal is not a regular file: {}",
            path.display()
        );
    }
    if metadata.len() > MAX_JOURNAL_SIZE {
        bail!("transaction journal is too large: {}", path.display());
    }
    let bytes = fs::read(path)?;
    let journal = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse transaction journal: {}", path.display()))?;
    Ok(Some(journal))
}

fn persist_new_journal(
    project_root: &Path,
    destination: &Path,
    journal: &Journal,
    label: &str,
) -> Result<()> {
    if destination.exists() {
        bail!(
            "transaction journal already exists: {}",
            destination.display()
        );
    }
    let transaction_id = journal
        .transaction_id
        .as_deref()
        .context("versioned transaction journal has no id")?;
    let temporary = project_root.join(format!(".transaction.{transaction_id}.{label}.journal.tmp"));
    let result = (|| -> Result<()> {
        let bytes = serde_json::to_vec_pretty(journal)?;
        write_new_durable_file(&temporary, &bytes)?;
        fs::rename(&temporary, destination)?;
        sync_directory(project_root)
    })();
    if result.is_err() && !destination.exists() {
        let _ = remove_regular_file_if_present(&temporary);
    }
    result
}

fn expected_artifact_paths(
    target: &Path,
    transaction_id: &str,
    needs_creation_marker: bool,
) -> Result<(PathBuf, PathBuf, Option<PathBuf>)> {
    let parent = target
        .parent()
        .context("transaction target has no parent")?;
    let file_name = target
        .file_name()
        .context("transaction target has no file name")?
        .to_string_lossy();
    let prefix = format!(".{file_name}.{transaction_id}");
    Ok((
        parent.join(format!("{prefix}.tmp")),
        parent.join(format!("{prefix}.bak")),
        needs_creation_marker.then(|| parent.join(format!("{prefix}.new"))),
    ))
}

fn validate_legacy_artifact_paths(
    project_root: &Path,
    target: &Path,
    entry: &JournalEntry,
) -> Result<(PathBuf, PathBuf, Option<PathBuf>)> {
    let temporary = lexical_absolute_path(project_root, &entry.temporary)?;
    let backup = lexical_absolute_path(project_root, &entry.backup)?;
    let parent = target
        .parent()
        .context("transaction target has no parent")?;
    if !paths_equal(temporary.parent().unwrap_or(Path::new("")), parent)
        || !paths_equal(backup.parent().unwrap_or(Path::new("")), parent)
    {
        bail!("legacy transaction artifacts are not target siblings");
    }
    let file_name = target
        .file_name()
        .context("transaction target has no file name")?
        .to_string_lossy();
    let prefix = format!(".{file_name}.");
    let temporary_name = temporary
        .file_name()
        .and_then(|name| name.to_str())
        .context("legacy temporary path is not valid UTF-8")?;
    let backup_name = backup
        .file_name()
        .and_then(|name| name.to_str())
        .context("legacy backup path is not valid UTF-8")?;
    let temporary_id = temporary_name
        .strip_prefix(&prefix)
        .and_then(|name| name.strip_suffix(".tmp"))
        .context("legacy temporary name does not match its target")?;
    let backup_id = backup_name
        .strip_prefix(&prefix)
        .and_then(|name| name.strip_suffix(".bak"))
        .context("legacy backup name does not match its target")?;
    if temporary_id != backup_id || Uuid::parse_str(temporary_id).is_err() {
        bail!("legacy transaction artifacts do not share a valid transaction id");
    }
    ensure_regular_or_missing(&temporary)?;
    ensure_regular_or_missing(&backup)?;
    Ok((temporary, backup, None))
}

fn require_same_stored_path(
    project_root: &Path,
    stored: &Path,
    expected: &Path,
    label: &str,
) -> Result<()> {
    let stored = lexical_absolute_path(project_root, stored)?;
    if !paths_equal(&stored, expected) {
        bail!("transaction {label} path does not match its target");
    }
    Ok(())
}

fn lexical_absolute_path(project_root: &Path, path: &Path) -> Result<PathBuf> {
    if !path.is_absolute()
        && path.components().any(|component| {
            matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::ParentDir
            )
        })
    {
        bail!(
            "transaction path contains a relative escape: {}",
            path.display()
        );
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        bail!("transaction path contains '..': {}", path.display());
    }
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                bail!("transaction path contains '..': {}", path.display())
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    Ok(dunce::simplified(&normalized).to_path_buf())
}

fn live_absolute_path(project_root: &Path, path: &Path) -> Result<PathBuf> {
    let joined = if path.is_absolute() {
        dunce::simplified(path).to_path_buf()
    } else {
        project_root.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    bail!(
                        "transaction update escapes the filesystem root: {}",
                        path.display()
                    );
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    Ok(dunce::simplified(&normalized).to_path_buf())
}

fn plan_update_target(project_root: &Path, path: &Path) -> Result<UpdateTargetPlan> {
    let target = live_absolute_path(project_root, path)?;
    validate_windows_transaction_path(&target)?;
    let parent = target
        .parent()
        .context("transaction target has no parent")?;
    match fs::symlink_metadata(parent) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                bail!(
                    "transaction target parent is not a regular directory: {}",
                    parent.display()
                );
            }
            let target = canonical_target_path(project_root, &target)?;
            Ok(UpdateTargetPlan {
                target,
                directories_to_create: Vec::new(),
            })
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            if !path_is_within(&target, project_root) {
                bail!(
                    "external transaction target parent must already exist: {}",
                    parent.display()
                );
            }

            let mut directories_to_create = Vec::new();
            let mut current = parent;
            loop {
                match fs::symlink_metadata(current) {
                    Ok(metadata) => {
                        if metadata.file_type().is_symlink() || !metadata.is_dir() {
                            bail!(
                                "transaction target ancestor is not a regular directory: {}",
                                current.display()
                            );
                        }
                        let canonical = dunce::canonicalize(current)?;
                        if !paths_equal(current, &canonical)
                            || !path_is_within(&canonical, project_root)
                        {
                            bail!(
                                "transaction target traverses an unsafe ancestor: {}",
                                current.display()
                            );
                        }
                        break;
                    }
                    Err(error) if error.kind() == ErrorKind::NotFound => {
                        directories_to_create.push(current.to_path_buf());
                        current = current
                            .parent()
                            .context("transaction target has no existing ancestor")?;
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            directories_to_create.reverse();
            Ok(UpdateTargetPlan {
                target,
                directories_to_create,
            })
        }
        Err(error) => Err(error.into()),
    }
}

fn create_planned_directories(plan: &UpdateTargetPlan) -> Result<()> {
    for directory in &plan.directories_to_create {
        let parent = directory
            .parent()
            .context("planned transaction directory has no parent")?;
        let canonical_parent = dunce::canonicalize(parent)?;
        if !paths_equal(parent, &canonical_parent) {
            bail!(
                "transaction directory parent changed during preparation: {}",
                parent.display()
            );
        }
        match fs::create_dir(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let metadata = fs::symlink_metadata(directory)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!(
                "transaction directory is not a regular directory: {}",
                directory.display()
            );
        }
        let canonical = dunce::canonicalize(directory)?;
        if !paths_equal(directory, &canonical) {
            bail!(
                "transaction directory became a symbolic link or junction: {}",
                directory.display()
            );
        }
        sync_directory(parent)?;
    }
    Ok(())
}

fn canonical_target_path(project_root: &Path, path: &Path) -> Result<PathBuf> {
    let absolute = lexical_absolute_path(project_root, path)?;
    validate_windows_transaction_path(&absolute)?;
    let parent = absolute
        .parent()
        .context("transaction target has no parent")?;
    let canonical_parent = dunce::canonicalize(parent).with_context(|| {
        format!(
            "transaction target parent does not exist or cannot be resolved: {}",
            parent.display()
        )
    })?;
    if !paths_equal(parent, &canonical_parent) {
        bail!(
            "transaction path traverses a symbolic link or junction: {}",
            absolute.display()
        );
    }
    let target = canonical_parent.join(
        absolute
            .file_name()
            .context("transaction target has no file name")?,
    );
    ensure_regular_or_missing(&target)?;
    Ok(target)
}

fn validate_windows_transaction_path(path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        for component in path.components() {
            let Component::Normal(component) = component else {
                continue;
            };
            let component = component.to_str().with_context(|| {
                format!("transaction path is not valid UTF-8: {}", path.display())
            })?;
            if component.ends_with(' ')
                || component.ends_with('.')
                || component.split('.').any(|segment| segment.ends_with(' '))
                || component
                    .chars()
                    .any(|character| character <= '\u{1f}' || r#"<>:\"/\\|?*"#.contains(character))
            {
                bail!(
                    "transaction path is not safely representable on Windows: {}",
                    path.display()
                );
            }
            let stem = component
                .split('.')
                .next()
                .unwrap_or(component)
                .to_ascii_uppercase();
            let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || stem
                    .strip_prefix("COM")
                    .or_else(|| stem.strip_prefix("LPT"))
                    .is_some_and(|suffix| {
                        suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9')
                    });
            if reserved {
                bail!(
                    "transaction path uses a reserved Windows device name: {}",
                    path.display()
                );
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = path;
    }
    Ok(())
}

fn reject_reserved_target(project_root: &Path, target: &Path) -> Result<()> {
    if paths_equal(target, &project_root.join(JOURNAL_FILE_NAME))
        || paths_equal(target, &project_root.join(COMMITTED_JOURNAL_FILE_NAME))
    {
        bail!("transaction cannot update its own journal files");
    }
    Ok(())
}

fn ensure_regular_or_missing(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!("transaction path is a symbolic link: {}", path.display())
        }
        Ok(metadata) if !metadata.is_file() => {
            bail!("transaction path is not a regular file: {}", path.display())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn remove_regular_file_if_present(path: &Path) -> Result<()> {
    ensure_regular_or_missing(path)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn write_new_durable_file(path: &Path, contents: &[u8]) -> Result<()> {
    ensure_regular_or_missing(path)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .with_context(|| format!("failed to create {}", path.display()))?;
    let result = file.write_all(contents).and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = result {
        let _ = remove_regular_file_if_present(path);
        return Err(error.into());
    }
    Ok(())
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    let mut path_components = path.components();
    for root_component in root.components() {
        let Some(path_component) = path_components.next() else {
            return false;
        };
        if !component_equal(path_component, root_component) {
            return false;
        }
    }
    true
}

#[cfg(windows)]
fn component_equal(left: Component<'_>, right: Component<'_>) -> bool {
    left.as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
}

#[cfg(not(windows))]
fn component_equal(left: Component<'_>, right: Component<'_>) -> bool {
    left == right
}

#[cfg(windows)]
fn paths_equal(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(not(windows))]
fn paths_equal(left: &Path, right: &Path) -> bool {
    left == right
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
    fs::File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn id() -> String {
        Uuid::now_v7().to_string()
    }

    fn versioned_entry(target: &Path, transaction_id: &str, existed: bool) -> JournalEntry {
        let (temporary, backup, creation_marker) =
            expected_artifact_paths(target, transaction_id, !existed).unwrap();
        JournalEntry {
            operation: TransactionOperation::Update,
            target: target.to_path_buf(),
            temporary,
            backup,
            existed,
            creation_marker,
        }
    }

    fn deletion_entry(target: &Path, transaction_id: &str) -> JournalEntry {
        let (temporary, backup, _) =
            expected_artifact_paths(target, transaction_id, false).unwrap();
        JournalEntry {
            operation: TransactionOperation::Delete,
            target: target.to_path_buf(),
            temporary,
            backup,
            existed: true,
            creation_marker: None,
        }
    }

    fn journal(
        transaction_id: &str,
        phase: TransactionPhase,
        entries: Vec<JournalEntry>,
    ) -> Journal {
        Journal {
            format_version: JOURNAL_FORMAT_VERSION,
            transaction_id: Some(transaction_id.to_string()),
            phase: Some(phase),
            entries,
        }
    }

    fn write_journal(root: &Path, file_name: &str, journal: &Journal) -> Result<()> {
        fs::write(root.join(file_name), serde_json::to_vec_pretty(journal)?)?;
        Ok(())
    }

    fn create_marker(entry: &JournalEntry, transaction_id: &str) -> Result<()> {
        fs::write(
            entry
                .creation_marker
                .as_ref()
                .context("entry has no creation marker")?,
            transaction_id,
        )?;
        Ok(())
    }

    #[test]
    fn writes_multiple_files_and_removes_journals() -> Result<()> {
        let temp = tempdir()?;
        let first = temp.path().join("one.txt");
        let second = temp.path().join("nested/two.txt");
        fs::write(&first, "old")?;
        write_transaction(
            temp.path(),
            &[
                FileUpdate {
                    path: first.clone(),
                    contents: b"new".to_vec(),
                },
                FileUpdate {
                    path: second.clone(),
                    contents: b"two".to_vec(),
                },
            ],
        )?;
        assert_eq!(fs::read_to_string(first)?, "new");
        assert_eq!(fs::read_to_string(second)?, "two");
        assert!(!temp.path().join(JOURNAL_FILE_NAME).exists());
        assert!(!temp.path().join(COMMITTED_JOURNAL_FILE_NAME).exists());
        Ok(())
    }

    #[test]
    fn writes_an_explicit_external_target_during_a_live_transaction() -> Result<()> {
        let workspace = tempdir()?;
        let root = workspace.path().join("project");
        let outside = workspace.path().join("data");
        fs::create_dir_all(&root)?;
        fs::create_dir_all(&outside)?;
        let target = outside.join("items.json");
        fs::write(&target, "old")?;

        write_transaction(
            &root,
            &[FileUpdate {
                path: target.clone(),
                contents: b"new".to_vec(),
            }],
        )?;

        assert_eq!(fs::read_to_string(target)?, "new");
        assert!(!root.join(JOURNAL_FILE_NAME).exists());
        assert!(!root.join(COMMITTED_JOURNAL_FILE_NAME).exists());
        Ok(())
    }

    #[test]
    fn writes_and_deletes_files_in_one_transaction() -> Result<()> {
        let temp = tempdir()?;
        let workbook = temp.path().join("workbook.toml");
        let sidecar = temp.path().join("sheets/one/formulas.json");
        fs::create_dir_all(sidecar.parent().unwrap())?;
        fs::write(&workbook, "old workbook")?;
        fs::write(&sidecar, "old sidecar")?;

        write_transaction_with_deletions(
            temp.path(),
            &[FileUpdate {
                path: workbook.clone(),
                contents: b"new workbook".to_vec(),
            }],
            std::slice::from_ref(&sidecar),
        )?;

        assert_eq!(fs::read_to_string(workbook)?, "new workbook");
        assert!(!sidecar.exists());
        assert!(!temp.path().join(JOURNAL_FILE_NAME).exists());
        assert!(!temp.path().join(COMMITTED_JOURNAL_FILE_NAME).exists());
        Ok(())
    }

    #[test]
    fn rejects_all_inputs_before_creating_an_internal_parent() -> Result<()> {
        let workspace = tempdir()?;
        let root = workspace.path().join("project");
        fs::create_dir_all(&root)?;
        let internal = root.join("new/sheet.toml");
        let external = workspace.path().join("missing/data/items.json");

        let error = write_transaction(
            &root,
            &[
                FileUpdate {
                    path: internal.clone(),
                    contents: b"internal".to_vec(),
                },
                FileUpdate {
                    path: external.clone(),
                    contents: b"external".to_vec(),
                },
            ],
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("external transaction target parent must already exist"));
        assert!(!internal.parent().unwrap().exists());
        assert!(!external.parent().unwrap().exists());
        assert!(!root.join(JOURNAL_FILE_NAME).exists());
        Ok(())
    }

    #[test]
    fn prepared_recovery_restores_update_and_deletion_together() -> Result<()> {
        let temp = tempdir()?;
        let transaction_id = id();
        let workbook = temp.path().join("workbook.toml");
        let sidecar = temp.path().join("formulas.json");
        let update = versioned_entry(&workbook, &transaction_id, true);
        let deletion = deletion_entry(&sidecar, &transaction_id);
        fs::write(&workbook, "new workbook")?;
        fs::write(&update.backup, "old workbook")?;
        fs::write(&deletion.backup, "old sidecar")?;
        let prepared = journal(
            &transaction_id,
            TransactionPhase::Prepared,
            vec![update.clone(), deletion.clone()],
        );
        write_journal(temp.path(), JOURNAL_FILE_NAME, &prepared)?;

        recover_transaction_with_allowed_targets(
            temp.path(),
            &[workbook.clone(), sidecar.clone()],
        )?;

        assert_eq!(fs::read_to_string(workbook)?, "old workbook");
        assert_eq!(fs::read_to_string(sidecar)?, "old sidecar");
        assert!(!update.backup.exists());
        assert!(!deletion.backup.exists());
        Ok(())
    }

    #[test]
    fn committed_recovery_keeps_update_and_finishes_deletion_together() -> Result<()> {
        let temp = tempdir()?;
        let transaction_id = id();
        let workbook = temp.path().join("workbook.toml");
        let sidecar = temp.path().join("formulas.json");
        let update = versioned_entry(&workbook, &transaction_id, true);
        let deletion = deletion_entry(&sidecar, &transaction_id);
        fs::write(&workbook, "new workbook")?;
        fs::write(&update.backup, "old workbook")?;
        fs::write(&deletion.backup, "old sidecar")?;
        let prepared = journal(
            &transaction_id,
            TransactionPhase::Prepared,
            vec![update.clone(), deletion.clone()],
        );
        let committed = journal(
            &transaction_id,
            TransactionPhase::Committed,
            vec![update.clone(), deletion.clone()],
        );
        write_journal(temp.path(), JOURNAL_FILE_NAME, &prepared)?;
        write_journal(temp.path(), COMMITTED_JOURNAL_FILE_NAME, &committed)?;

        recover_transaction_with_allowed_targets(
            temp.path(),
            &[workbook.clone(), sidecar.clone()],
        )?;

        assert_eq!(fs::read_to_string(workbook)?, "new workbook");
        assert!(!sidecar.exists());
        assert!(!update.backup.exists());
        assert!(!deletion.backup.exists());
        Ok(())
    }

    #[test]
    fn invalid_deletion_state_blocks_all_recovery_mutation() -> Result<()> {
        let temp = tempdir()?;
        let transaction_id = id();
        let workbook = temp.path().join("workbook.toml");
        let sidecar = temp.path().join("formulas.json");
        let update = versioned_entry(&workbook, &transaction_id, true);
        let deletion = deletion_entry(&sidecar, &transaction_id);
        fs::write(&workbook, "new workbook")?;
        fs::write(&update.backup, "old workbook")?;
        fs::write(&sidecar, "current sidecar")?;
        fs::write(&deletion.backup, "conflicting sidecar backup")?;
        let prepared = journal(
            &transaction_id,
            TransactionPhase::Prepared,
            vec![update.clone(), deletion.clone()],
        );
        write_journal(temp.path(), JOURNAL_FILE_NAME, &prepared)?;

        let error = recover_transaction_with_allowed_targets(
            temp.path(),
            &[workbook.clone(), sidecar.clone()],
        )
        .unwrap_err();

        assert!(error.to_string().contains("no files were changed"));
        assert_eq!(fs::read_to_string(&workbook)?, "new workbook");
        assert_eq!(fs::read_to_string(&update.backup)?, "old workbook");
        assert_eq!(fs::read_to_string(&sidecar)?, "current sidecar");
        assert_eq!(
            fs::read_to_string(&deletion.backup)?,
            "conflicting sidecar backup"
        );
        Ok(())
    }

    #[test]
    fn journal_first_created_file_state_recovers_without_orphan_artifacts() -> Result<()> {
        let temp = tempdir()?;
        let transaction_id = id();
        let target = temp.path().join("new.json");
        let entry = versioned_entry(&target, &transaction_id, false);
        let prepared = journal(
            &transaction_id,
            TransactionPhase::Prepared,
            vec![entry.clone()],
        );
        write_journal(temp.path(), JOURNAL_FILE_NAME, &prepared)?;

        recover_transaction_with_allowed_targets(temp.path(), std::slice::from_ref(&target))?;

        assert!(!target.exists());
        assert!(!entry.temporary.exists());
        assert!(!entry.creation_marker.unwrap().exists());
        assert!(!temp.path().join(JOURNAL_FILE_NAME).exists());
        Ok(())
    }

    #[test]
    fn prepared_recovery_rolls_back_every_intermediate_state() -> Result<()> {
        let temp = tempdir()?;
        let transaction_id = id();
        let untouched = temp.path().join("untouched.json");
        let swapped = temp.path().join("swapped.json");
        let between_renames = temp.path().join("between.json");
        let created = temp.path().join("created.json");
        let untouched_entry = versioned_entry(&untouched, &transaction_id, true);
        let swapped_entry = versioned_entry(&swapped, &transaction_id, true);
        let between_entry = versioned_entry(&between_renames, &transaction_id, true);
        let created_entry = versioned_entry(&created, &transaction_id, false);

        fs::write(&untouched, "old-untouched")?;
        fs::write(&untouched_entry.temporary, "new-untouched")?;
        fs::write(&swapped, "new-swapped")?;
        fs::write(&swapped_entry.backup, "old-swapped")?;
        fs::write(&between_entry.temporary, "new-between")?;
        fs::write(&between_entry.backup, "old-between")?;
        fs::write(&created, "new-created")?;
        create_marker(&created_entry, &transaction_id)?;

        let prepared = journal(
            &transaction_id,
            TransactionPhase::Prepared,
            vec![untouched_entry, swapped_entry, between_entry, created_entry],
        );
        write_journal(temp.path(), JOURNAL_FILE_NAME, &prepared)?;

        let approved = prepared
            .entries
            .iter()
            .map(|entry| entry.target.clone())
            .collect::<Vec<_>>();
        recover_transaction_with_allowed_targets(temp.path(), &approved)?;
        assert_eq!(fs::read_to_string(untouched)?, "old-untouched");
        assert_eq!(fs::read_to_string(swapped)?, "old-swapped");
        assert_eq!(fs::read_to_string(between_renames)?, "old-between");
        assert!(!created.exists());
        assert!(!temp.path().join(JOURNAL_FILE_NAME).exists());
        Ok(())
    }

    #[test]
    fn committed_recovery_keeps_all_new_files_after_partial_backup_cleanup() -> Result<()> {
        let temp = tempdir()?;
        let transaction_id = id();
        let cleaned = temp.path().join("cleaned.json");
        let pending = temp.path().join("pending.json");
        let created = temp.path().join("created.json");
        let cleaned_entry = versioned_entry(&cleaned, &transaction_id, true);
        let pending_entry = versioned_entry(&pending, &transaction_id, true);
        let created_entry = versioned_entry(&created, &transaction_id, false);

        fs::write(&cleaned, "new-cleaned")?;
        fs::write(&pending, "new-pending")?;
        fs::write(&pending_entry.backup, "old-pending")?;
        fs::write(&created, "new-created")?;
        create_marker(&created_entry, &transaction_id)?;

        let prepared = journal(
            &transaction_id,
            TransactionPhase::Prepared,
            vec![
                cleaned_entry.clone(),
                pending_entry.clone(),
                created_entry.clone(),
            ],
        );
        let committed = journal(
            &transaction_id,
            TransactionPhase::Committed,
            vec![cleaned_entry, pending_entry.clone(), created_entry.clone()],
        );
        write_journal(temp.path(), JOURNAL_FILE_NAME, &prepared)?;
        write_journal(temp.path(), COMMITTED_JOURNAL_FILE_NAME, &committed)?;

        let approved = committed
            .entries
            .iter()
            .map(|entry| entry.target.clone())
            .collect::<Vec<_>>();
        recover_transaction_with_allowed_targets(temp.path(), &approved)?;
        assert_eq!(fs::read_to_string(cleaned)?, "new-cleaned");
        assert_eq!(fs::read_to_string(pending)?, "new-pending");
        assert_eq!(fs::read_to_string(created)?, "new-created");
        assert!(!pending_entry.backup.exists());
        assert!(!created_entry.creation_marker.unwrap().exists());
        assert!(!temp.path().join(JOURNAL_FILE_NAME).exists());
        assert!(!temp.path().join(COMMITTED_JOURNAL_FILE_NAME).exists());
        Ok(())
    }

    #[test]
    fn committed_record_alone_finishes_cleanup_without_rollback() -> Result<()> {
        let temp = tempdir()?;
        let transaction_id = id();
        let target = temp.path().join("value.json");
        let entry = versioned_entry(&target, &transaction_id, true);
        fs::write(&target, "new")?;
        fs::write(&entry.backup, "old")?;
        let committed = journal(
            &transaction_id,
            TransactionPhase::Committed,
            vec![entry.clone()],
        );
        write_journal(temp.path(), COMMITTED_JOURNAL_FILE_NAME, &committed)?;

        recover_transaction_with_allowed_targets(temp.path(), std::slice::from_ref(&target))?;
        assert_eq!(fs::read_to_string(target)?, "new");
        assert!(!entry.backup.exists());
        Ok(())
    }

    #[test]
    fn validates_every_entry_before_touching_an_external_path() -> Result<()> {
        let workspace = tempdir()?;
        let root = workspace.path().join("project");
        let outside = workspace.path().join("outside");
        fs::create_dir_all(&root)?;
        fs::create_dir_all(&outside)?;
        let transaction_id = id();
        let internal_target = root.join("internal.json");
        let external_target = outside.join("external.json");
        let internal = versioned_entry(&internal_target, &transaction_id, true);
        let external = versioned_entry(&external_target, &transaction_id, true);
        fs::write(&internal_target, "new-internal")?;
        fs::write(&internal.backup, "old-internal")?;
        fs::write(&external_target, "new-external")?;
        fs::write(&external.backup, "old-external")?;
        let prepared = journal(
            &transaction_id,
            TransactionPhase::Prepared,
            vec![internal.clone(), external.clone()],
        );
        write_journal(&root, JOURNAL_FILE_NAME, &prepared)?;

        let error =
            recover_transaction_with_allowed_targets(&root, std::slice::from_ref(&internal_target))
                .unwrap_err();
        assert!(error.to_string().contains("manual recovery"));
        assert!(error
            .to_string()
            .contains(&external_target.display().to_string()));
        assert_eq!(fs::read_to_string(&internal_target)?, "new-internal");
        assert_eq!(fs::read_to_string(&internal.backup)?, "old-internal");
        assert_eq!(fs::read_to_string(&external_target)?, "new-external");
        assert_eq!(fs::read_to_string(&external.backup)?, "old-external");
        Ok(())
    }

    #[test]
    fn automatic_recovery_does_not_trust_a_crafted_internal_journal() -> Result<()> {
        let temp = tempdir()?;
        let transaction_id = id();
        let target = temp.path().join("workbook.toml");
        let entry = versioned_entry(&target, &transaction_id, true);
        fs::write(&target, "current")?;
        fs::write(&entry.backup, "attacker-controlled")?;
        let prepared = journal(
            &transaction_id,
            TransactionPhase::Prepared,
            vec![entry.clone()],
        );
        write_journal(temp.path(), JOURNAL_FILE_NAME, &prepared)?;

        let error = recover_transaction(temp.path()).unwrap_err();
        assert!(error.to_string().contains("manual recovery"));
        assert!(error.to_string().contains("independently approved"));
        assert_eq!(fs::read_to_string(&target)?, "current");
        assert_eq!(fs::read_to_string(&entry.backup)?, "attacker-controlled");
        assert!(temp.path().join(JOURNAL_FILE_NAME).is_file());
        Ok(())
    }

    #[test]
    fn explicit_exact_target_allows_external_recovery() -> Result<()> {
        let workspace = tempdir()?;
        let root = workspace.path().join("project");
        let outside = workspace.path().join("outside");
        fs::create_dir_all(&root)?;
        fs::create_dir_all(&outside)?;
        let transaction_id = id();
        let target = outside.join("data.json");
        let entry = versioned_entry(&target, &transaction_id, true);
        fs::write(&target, "new")?;
        fs::write(&entry.backup, "old")?;
        let prepared = journal(&transaction_id, TransactionPhase::Prepared, vec![entry]);
        write_journal(&root, JOURNAL_FILE_NAME, &prepared)?;

        recover_transaction_with_allowed_targets(&root, std::slice::from_ref(&target))?;
        assert_eq!(fs::read_to_string(target)?, "old");
        Ok(())
    }

    #[test]
    fn rejects_relative_parent_escape_without_touching_outside_file() -> Result<()> {
        let workspace = tempdir()?;
        let root = workspace.path().join("project");
        fs::create_dir_all(&root)?;
        let transaction_id = id();
        let outside_target = workspace.path().join("outside.json");
        let outside_backup = workspace
            .path()
            .join(format!(".outside.json.{transaction_id}.bak"));
        fs::write(&outside_target, "new")?;
        fs::write(&outside_backup, "old")?;
        let entry = JournalEntry {
            operation: TransactionOperation::Update,
            target: PathBuf::from("../outside.json"),
            temporary: PathBuf::from(format!("../.outside.json.{transaction_id}.tmp")),
            backup: PathBuf::from(format!("../.outside.json.{transaction_id}.bak")),
            existed: true,
            creation_marker: None,
        };
        let prepared = journal(&transaction_id, TransactionPhase::Prepared, vec![entry]);
        write_journal(&root, JOURNAL_FILE_NAME, &prepared)?;

        assert!(recover_transaction(&root).is_err());
        assert_eq!(fs::read_to_string(outside_target)?, "new");
        assert_eq!(fs::read_to_string(outside_backup)?, "old");
        Ok(())
    }

    #[test]
    fn explicitly_recovers_an_update_only_v2_journal() -> Result<()> {
        let temp = tempdir()?;
        let transaction_id = id();
        let target = temp.path().join("v2.json");
        let entry = versioned_entry(&target, &transaction_id, true);
        fs::write(&target, "new")?;
        fs::write(&entry.backup, "old")?;
        let v2 = Journal {
            format_version: PREVIOUS_JOURNAL_FORMAT_VERSION,
            transaction_id: Some(transaction_id),
            phase: Some(TransactionPhase::Prepared),
            entries: vec![entry],
        };
        assert!(!serde_json::to_string(&v2)?.contains("operation"));
        write_journal(temp.path(), JOURNAL_FILE_NAME, &v2)?;

        recover_transaction_with_allowed_targets(temp.path(), std::slice::from_ref(&target))?;

        assert_eq!(fs::read_to_string(target)?, "old");
        assert!(!temp.path().join(JOURNAL_FILE_NAME).exists());
        Ok(())
    }

    #[test]
    fn recovers_unambiguous_legacy_existing_file_journal() -> Result<()> {
        let temp = tempdir()?;
        let transaction_id = id();
        let target = temp.path().join("legacy.json");
        let temporary = temp
            .path()
            .join(format!(".legacy.json.{transaction_id}.tmp"));
        let backup = temp
            .path()
            .join(format!(".legacy.json.{transaction_id}.bak"));
        fs::write(&target, "new")?;
        fs::write(&backup, "old")?;
        let legacy = Journal {
            format_version: 0,
            transaction_id: None,
            phase: None,
            entries: vec![JournalEntry {
                operation: TransactionOperation::Update,
                target: target.clone(),
                temporary,
                backup,
                existed: true,
                creation_marker: None,
            }],
        };
        write_journal(temp.path(), JOURNAL_FILE_NAME, &legacy)?;

        recover_transaction_with_allowed_targets(temp.path(), std::slice::from_ref(&target))?;
        assert_eq!(fs::read_to_string(target)?, "old");
        Ok(())
    }

    #[test]
    fn legacy_created_target_requires_manual_recovery() -> Result<()> {
        let temp = tempdir()?;
        let transaction_id = id();
        let target = temp.path().join("created.json");
        let temporary = temp
            .path()
            .join(format!(".created.json.{transaction_id}.tmp"));
        let backup = temp
            .path()
            .join(format!(".created.json.{transaction_id}.bak"));
        fs::write(&target, "possibly-unrelated")?;
        let legacy = Journal {
            format_version: 0,
            transaction_id: None,
            phase: None,
            entries: vec![JournalEntry {
                operation: TransactionOperation::Update,
                target: target.clone(),
                temporary,
                backup,
                existed: false,
                creation_marker: None,
            }],
        };
        write_journal(temp.path(), JOURNAL_FILE_NAME, &legacy)?;

        assert!(recover_transaction(temp.path()).is_err());
        assert_eq!(fs::read_to_string(target)?, "possibly-unrelated");
        Ok(())
    }

    #[test]
    fn preparation_error_removes_unjournaled_temporary_files() -> Result<()> {
        let temp = tempdir()?;
        let target = temp.path().join("duplicate.json");
        fs::write(&target, "old")?;
        let updates = [
            FileUpdate {
                path: target.clone(),
                contents: b"first".to_vec(),
            },
            FileUpdate {
                path: target,
                contents: b"second".to_vec(),
            },
        ];

        assert!(write_transaction(temp.path(), &updates).is_err());
        assert!(!temp.path().join(JOURNAL_FILE_NAME).exists());
        assert!(!temp.path().join(COMMITTED_JOURNAL_FILE_NAME).exists());
        let transaction_artifacts = fs::read_dir(temp.path())?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".duplicate.json.")
            })
            .count();
        assert_eq!(transaction_artifacts, 0);
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn rejects_windows_unsafe_targets_before_journaling() -> Result<()> {
        let temp = tempdir()?;
        for file_name in ["CON.json", "name. ", "file.json:ads"] {
            let error = write_transaction(
                temp.path(),
                &[FileUpdate {
                    path: temp.path().join(file_name),
                    contents: b"unsafe".to_vec(),
                }],
            )
            .unwrap_err();
            assert!(
                error.to_string().contains("Windows"),
                "unexpected error for {file_name}: {error:#}"
            );
            assert!(!temp.path().join(JOURNAL_FILE_NAME).exists());
            assert!(!temp.path().join(COMMITTED_JOURNAL_FILE_NAME).exists());
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_parent_without_touching_link_target() -> Result<()> {
        use std::os::unix::fs::symlink;

        let workspace = tempdir()?;
        let root = workspace.path().join("project");
        let outside = workspace.path().join("outside");
        fs::create_dir_all(&root)?;
        fs::create_dir_all(&outside)?;
        symlink(&outside, root.join("linked"))?;
        let transaction_id = id();
        let target = root.join("linked/value.json");
        let entry = versioned_entry(&target, &transaction_id, true);
        fs::write(outside.join("value.json"), "new")?;
        fs::write(
            outside.join(format!(".value.json.{transaction_id}.bak")),
            "old",
        )?;
        let prepared = journal(&transaction_id, TransactionPhase::Prepared, vec![entry]);
        write_journal(&root, JOURNAL_FILE_NAME, &prepared)?;

        assert!(recover_transaction(&root).is_err());
        assert_eq!(fs::read_to_string(outside.join("value.json"))?, "new");
        assert_eq!(
            fs::read_to_string(outside.join(format!(".value.json.{transaction_id}.bak")))?,
            "old"
        );
        Ok(())
    }
}
