use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use polysheet_core::git::{git_attributes_lines, snapshot_at_revision, stage_sheet_files};
use polysheet_core::{
    diff_snapshots, merge_snapshots, write_transaction, FileUpdate, OpenProjectOptions,
    PolySheetProject,
};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "polysheet",
    version,
    about = "Git-friendly PolyGen spreadsheet tooling"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Canonically format a PolySheet project and its JSON sources.
    Fmt {
        project: PathBuf,
        /// Approve the one-time normalization of existing JSON sources.
        #[arg(long)]
        approve_normalization: bool,
    },
    /// Validate schema-bound data and row identities.
    Validate {
        project: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Show semantic changes between Git revisions or the working tree.
    Diff {
        project: PathBuf,
        #[arg(long, default_value = "HEAD")]
        base: String,
        #[arg(long, default_value = "WORKTREE")]
        target: String,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Perform a field-level three-way merge into the working tree.
    Merge {
        project: PathBuf,
        #[arg(long)]
        base: String,
        #[arg(long)]
        ours: String,
        #[arg(long)]
        theirs: String,
        /// Stage all resolved sheet files after validation.
        #[arg(long)]
        stage: bool,
        /// Approve the one-time normalization of existing JSON sources.
        #[arg(long)]
        approve_normalization: bool,
    },
    /// Print or install repository-local Git diff integration.
    GitConfig {
        project: PathBuf,
        /// Write .gitattributes and repository-local diff driver configuration.
        #[arg(long)]
        apply: bool,
    },
    #[command(hide = true)]
    GitTextconv { project: PathBuf, file: PathBuf },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Fmt {
            project,
            approve_normalization,
        } => {
            let mut project = open(&project)?;
            project.save(approve_normalization)?;
            println!("Formatted {} sheet(s).", project.sheets.len());
        }
        Command::Validate { project, format } => {
            let project = open(&project)?;
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&project.diagnostics)?)
                }
                OutputFormat::Text => {
                    if project.diagnostics.is_empty() {
                        println!("No validation issues.");
                    } else {
                        for diagnostic in &project.diagnostics {
                            println!(
                                "{:?}: {}: {}",
                                diagnostic.severity, diagnostic.path, diagnostic.message
                            );
                        }
                    }
                }
            }
            if project.has_errors() {
                bail!("validation failed");
            }
        }
        Command::Diff {
            project,
            base,
            target,
            format,
        } => {
            let project = open(&project)?;
            let base = snapshot_at_revision(&project, &base)?;
            let target = snapshot_at_revision(&project, &target)?;
            let report = diff_snapshots(&base, &target);
            match format {
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
                OutputFormat::Text => print_diff(&report),
            }
        }
        Command::Merge {
            project,
            base,
            ours,
            theirs,
            stage,
            approve_normalization,
        } => {
            let mut project = open(&project)?;
            let base_snapshot = snapshot_at_revision(&project, &base)?;
            let ours_snapshot = snapshot_at_revision(&project, &ours)?;
            let theirs_snapshot = snapshot_at_revision(&project, &theirs)?;
            let report = merge_snapshots(&base_snapshot, &ours_snapshot, &theirs_snapshot);
            if !report.conflicts.is_empty() {
                println!("{}", serde_json::to_string_pretty(&report.conflicts)?);
                bail!(
                    "merge has {} unresolved conflict(s)",
                    report.conflicts.len()
                );
            }
            project.apply_snapshot(&report.merged)?;
            if report.recalculation_required {
                bail!("merged formulas require recalculation in the PolySheet app");
            }
            project.save(approve_normalization)?;
            if stage {
                let sheet_ids = project.manifest.sheets.clone();
                let staged = stage_sheet_files(&project, &sheet_ids)?;
                println!("Merged and staged {} file(s).", staged.len());
            } else {
                println!("Merged into the working tree.");
            }
        }
        Command::GitConfig { project, apply } => configure_git(&project, apply)?,
        Command::GitTextconv { project: _, file } => {
            let content = fs::read_to_string(file)?;
            let value: serde_json::Value = serde_json::from_str(&content)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }
    Ok(())
}

fn open(path: &Path) -> Result<PolySheetProject> {
    PolySheetProject::open(path, OpenProjectOptions::default())
}

fn print_diff(report: &polysheet_core::DiffReport) {
    if report.changes.is_empty() {
        println!("No changes.");
        return;
    }
    for change in &report.changes {
        let row = change
            .row_id
            .as_deref()
            .map(|id| format!("[{id}]"))
            .unwrap_or_default();
        println!(
            "{:?} {}{} {}: {} -> {}",
            change.kind,
            change.sheet_name,
            row,
            change.path,
            display_value(change.before.as_ref()),
            display_value(change.after.as_ref())
        );
    }
}

fn display_value(value: Option<&serde_json::Value>) -> String {
    value
        .map(ToString::to_string)
        .unwrap_or_else(|| "<missing>".to_string())
}

fn configure_git(project_path: &Path, apply: bool) -> Result<()> {
    let project = open(project_path)?;
    let lines = git_attributes_lines(&project)?;
    println!("Suggested .gitattributes entries:");
    for line in &lines {
        println!("{line}");
    }
    if !apply {
        println!("\nRe-run with --apply to install repository-local integration.");
        return Ok(());
    }

    let repository = git2::Repository::discover(&project.root)?;
    let workdir = repository
        .workdir()
        .context("bare Git repositories are not supported")?;
    let executable = std::env::current_exe()?;
    let textconv_command = format!(
        "{} git-textconv {}",
        quote_git_shell_argument(&executable)?,
        quote_git_shell_argument(&project.root)?
    );
    let mut config = repository.config()?;
    let attributes_path = workdir.join(".gitattributes");
    install_git_attributes(&project.root, &attributes_path, &lines)?;

    config.set_str("diff.polysheet.textconv", &textconv_command)?;
    println!("Installed repository-local PolySheet diff integration.");
    Ok(())
}

fn install_git_attributes(
    project_root: &Path,
    attributes_path: &Path,
    lines: &[String],
) -> Result<()> {
    let mut attributes = match fs::symlink_metadata(attributes_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!(
                ".gitattributes is a symbolic link and will not be replaced: {}",
                attributes_path.display()
            )
        }
        Ok(metadata) if !metadata.is_file() => {
            bail!(
                ".gitattributes is not a regular file: {}",
                attributes_path.display()
            )
        }
        Ok(_) => fs::read_to_string(attributes_path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    for line in lines {
        if !attributes.lines().any(|existing| existing.trim() == line) {
            if !attributes.is_empty() && !attributes.ends_with('\n') {
                attributes.push('\n');
            }
            attributes.push_str(line);
            attributes.push('\n');
        }
    }
    write_transaction(
        project_root,
        &[FileUpdate {
            path: attributes_path.to_path_buf(),
            contents: attributes.into_bytes(),
        }],
    )
    .with_context(|| {
        format!(
            "failed to install Git attributes at {}",
            attributes_path.display()
        )
    })
}

fn quote_git_shell_argument(path: &Path) -> Result<String> {
    let value = path
        .to_str()
        .with_context(|| format!("Git command path is not valid UTF-8: {}", path.display()))?;
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
}

#[cfg(test)]
mod tests {
    use super::{install_git_attributes, quote_git_shell_argument, Cli, Command};
    use anyhow::Result;
    use clap::Parser;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn merge_requires_an_explicit_normalization_approval_flag() {
        let without_approval = Cli::try_parse_from([
            "polysheet",
            "merge",
            "game.polysheet",
            "--base",
            "BASE",
            "--ours",
            "OURS",
            "--theirs",
            "THEIRS",
        ])
        .unwrap();
        let Command::Merge {
            approve_normalization,
            ..
        } = without_approval.command
        else {
            panic!("expected merge command");
        };
        assert!(!approve_normalization);

        let with_approval = Cli::try_parse_from([
            "polysheet",
            "merge",
            "game.polysheet",
            "--base",
            "BASE",
            "--ours",
            "OURS",
            "--theirs",
            "THEIRS",
            "--approve-normalization",
        ])
        .unwrap();
        let Command::Merge {
            approve_normalization,
            ..
        } = with_approval.command
        else {
            panic!("expected merge command");
        };
        assert!(approve_normalization);
    }

    #[test]
    fn git_textconv_paths_are_literal_shell_arguments() {
        let quoted =
            quote_git_shell_argument(Path::new("C:/Poly Sheet/it's-$(touch injected)-`echo bad`"))
                .unwrap();

        assert_eq!(
            quoted,
            "'C:/Poly Sheet/it'\\''s-$(touch injected)-`echo bad`'"
        );
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn git_attributes_install_rejects_symlink_without_touching_target() -> Result<()> {
        let workspace = tempfile::tempdir()?;
        let project_root = workspace.path().join("project");
        fs::create_dir_all(&project_root)?;
        let (attributes_path, outside) = create_linked_attributes(workspace.path())?;

        let error = install_git_attributes(
            &project_root,
            &attributes_path,
            &["*.json diff=polysheet".to_string()],
        )
        .unwrap_err();

        let error = format!("{error:#}");
        assert!(
            error.contains("symbolic link")
                || error.contains("junction")
                || error.contains("not a regular directory"),
            "unexpected link rejection: {error}"
        );
        assert_eq!(fs::read_to_string(outside)?, "keep me\n");
        Ok(())
    }

    #[cfg(unix)]
    fn create_linked_attributes(workspace: &Path) -> Result<(PathBuf, PathBuf)> {
        let repository_root = workspace.join("repository");
        fs::create_dir_all(&repository_root)?;
        let outside = workspace.join("outside.txt");
        fs::write(&outside, "keep me\n")?;
        let attributes_path = repository_root.join(".gitattributes");
        std::os::unix::fs::symlink(&outside, &attributes_path)?;
        Ok((attributes_path, outside))
    }

    #[cfg(windows)]
    fn create_linked_attributes(workspace: &Path) -> Result<(PathBuf, PathBuf)> {
        use anyhow::bail;
        use std::process::Command as ProcessCommand;

        let outside_repository = workspace.join("outside-repository");
        fs::create_dir_all(&outside_repository)?;
        let outside = outside_repository.join(".gitattributes");
        fs::write(&outside, "keep me\n")?;

        // Directory junctions exercise the same no-follow parent check as a
        // symbolic link and do not require Windows developer-mode privileges.
        let linked_repository = workspace.join("repository");
        let output = ProcessCommand::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&linked_repository)
            .arg(&outside_repository)
            .output()?;
        if !output.status.success() {
            bail!(
                "failed to create test junction: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok((linked_repository.join(".gitattributes"), outside))
    }
}
