use crate::model::{FormulaBinding, FormulaReference};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaSheetLayout {
    pub id: String,
    pub name: String,
    pub row_ids: Vec<String>,
    pub column_ids: Vec<String>,
    pub row_offset: usize,
    pub column_offset: usize,
}

#[derive(Debug, Clone, Default)]
pub struct FormulaWorkbookLayout {
    sheets: BTreeMap<String, FormulaSheetLayout>,
    names: BTreeMap<String, Option<String>>,
}

impl FormulaWorkbookLayout {
    pub fn new(sheets: impl IntoIterator<Item = FormulaSheetLayout>) -> Self {
        let mut layout = Self::default();
        for sheet in sheets {
            let normalized_name = sheet.name.to_lowercase();
            match layout.names.entry(normalized_name) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(Some(sheet.id.clone()));
                }
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    entry.insert(None);
                }
            }
            layout.sheets.insert(sheet.id.clone(), sheet);
        }
        layout
    }

    pub fn sheet(&self, id: &str) -> Option<&FormulaSheetLayout> {
        self.sheets.get(id)
    }

    fn sheet_by_name(&self, name: &str) -> Option<&FormulaSheetLayout> {
        self.names
            .get(&name.to_lowercase())
            .and_then(|id| id.as_deref())
            .and_then(|id| self.sheets.get(id))
    }
}

pub fn capture_formula(
    source: &str,
    owner_sheet_id: &str,
    layout: &FormulaWorkbookLayout,
) -> FormulaBinding {
    let mut references = Vec::new();
    for captures in reference_regex().captures_iter(source) {
        let Some(full) = captures.get(0) else {
            continue;
        };
        if inside_string_literal(source, full.start())
            || invalid_reference_boundary(
                source,
                full.start(),
                full.end(),
                captures.name("sheet").is_some(),
            )
        {
            continue;
        }
        let target_sheet = captures
            .name("sheet")
            .map(|sheet| unquote_sheet_name(sheet.as_str()))
            .and_then(|name| layout.sheet_by_name(&name))
            .or_else(|| {
                if captures.name("sheet").is_none() {
                    layout.sheet(owner_sheet_id)
                } else {
                    None
                }
            });
        let Some(target_sheet) = target_sheet else {
            continue;
        };
        let Some(column_match) = captures.name("column") else {
            continue;
        };
        let Some(row_match) = captures.name("row") else {
            continue;
        };
        let Some(column_index) = column_index(column_match.as_str()) else {
            continue;
        };
        let Ok(row_number) = row_match.as_str().parse::<usize>() else {
            continue;
        };
        let Some(row_index) = row_number
            .checked_sub(1)
            .and_then(|index| index.checked_sub(target_sheet.row_offset))
        else {
            continue;
        };
        let Some(column_index) = column_index.checked_sub(target_sheet.column_offset) else {
            continue;
        };
        let (Some(row_id), Some(column_id)) = (
            target_sheet.row_ids.get(row_index),
            target_sheet.column_ids.get(column_index),
        ) else {
            continue;
        };
        references.push(FormulaReference {
            start: full.start(),
            end: full.end(),
            sheet_id: target_sheet.id.clone(),
            row_id: row_id.clone(),
            column_id: column_id.clone(),
            absolute_row: captures
                .name("row_abs")
                .is_some_and(|value| value.as_str() == "$"),
            absolute_column: captures
                .name("column_abs")
                .is_some_and(|value| value.as_str() == "$"),
            explicit_sheet: captures.name("sheet").is_some(),
        });
    }
    FormulaBinding {
        source: source.to_string(),
        references,
    }
}

pub fn render_formula(
    binding: &FormulaBinding,
    owner_sheet_id: &str,
    layout: &FormulaWorkbookLayout,
) -> String {
    let mut rendered = binding.source.clone();
    let mut references = binding.references.iter().collect::<Vec<_>>();
    references.sort_by_key(|reference| std::cmp::Reverse(reference.start));
    let mut occupied = BTreeSet::new();
    for reference in references {
        if reference.start > reference.end
            || reference.end > rendered.len()
            || !rendered.is_char_boundary(reference.start)
            || !rendered.is_char_boundary(reference.end)
            || (reference.start..reference.end).any(|index| occupied.contains(&index))
        {
            continue;
        }
        let replacement = render_reference(reference, owner_sheet_id, layout);
        rendered.replace_range(reference.start..reference.end, &replacement);
        occupied.extend(reference.start..reference.end);
    }
    rendered
}

pub fn binding_has_broken_reference(
    binding: &FormulaBinding,
    layout: &FormulaWorkbookLayout,
) -> bool {
    binding.references.iter().any(|reference| {
        layout.sheet(&reference.sheet_id).is_none_or(|sheet| {
            !sheet.row_ids.contains(&reference.row_id)
                || !sheet.column_ids.contains(&reference.column_id)
        })
    })
}

fn render_reference(
    reference: &FormulaReference,
    owner_sheet_id: &str,
    layout: &FormulaWorkbookLayout,
) -> String {
    let Some(sheet) = layout.sheet(&reference.sheet_id) else {
        return "#REF!".to_string();
    };
    let Some(row_index) = sheet
        .row_ids
        .iter()
        .position(|row_id| row_id == &reference.row_id)
    else {
        return "#REF!".to_string();
    };
    let Some(column_index) = sheet
        .column_ids
        .iter()
        .position(|column_id| column_id == &reference.column_id)
    else {
        return "#REF!".to_string();
    };
    let column = column_label(column_index + sheet.column_offset);
    let row = row_index + sheet.row_offset + 1;
    let qualifier = if reference.explicit_sheet || reference.sheet_id != owner_sheet_id {
        format!("'{}'!", sheet.name.replace('\'', "''"))
    } else {
        String::new()
    };
    format!(
        "{qualifier}{}{}{}{}",
        if reference.absolute_column { "$" } else { "" },
        column,
        if reference.absolute_row { "$" } else { "" },
        row
    )
}

fn reference_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?:(?P<sheet>'(?:[^']|'')+'|[A-Za-z_][A-Za-z0-9_.]*)!)?(?P<column_abs>\$?)(?P<column>[A-Za-z]{1,3})(?P<row_abs>\$?)(?P<row>[1-9][0-9]*)",
        )
        .expect("formula reference regex must compile")
    })
}

fn inside_string_literal(source: &str, position: usize) -> bool {
    let bytes = source.as_bytes();
    let mut inside = false;
    let mut index = 0;
    while index < position {
        if bytes[index] == b'"' {
            if inside && index + 1 < position && bytes[index + 1] == b'"' {
                index += 2;
                continue;
            }
            inside = !inside;
        }
        index += 1;
    }
    inside
}

fn invalid_reference_boundary(
    source: &str,
    start: usize,
    end: usize,
    explicit_sheet: bool,
) -> bool {
    let previous = source[..start].chars().next_back();
    if previous.is_some_and(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '_' | '.')
    }) {
        return true;
    }
    let following = source[end..].chars().next();
    if following.is_some_and(|character| character.is_ascii_alphanumeric() || character == '_') {
        return true;
    }
    if !explicit_sheet && source[end..].trim_start().starts_with('(') {
        return true;
    }
    false
}

fn unquote_sheet_name(name: &str) -> String {
    if name.starts_with('\'') && name.ends_with('\'') && name.len() >= 2 {
        name[1..name.len() - 1].replace("''", "'")
    } else {
        name.to_string()
    }
}

fn column_index(label: &str) -> Option<usize> {
    let mut result = 0usize;
    for byte in label.bytes() {
        let upper = byte.to_ascii_uppercase();
        if !upper.is_ascii_uppercase() {
            return None;
        }
        result = result
            .checked_mul(26)?
            .checked_add(usize::from(upper - b'A' + 1))?;
    }
    result.checked_sub(1)
}

fn column_label(mut index: usize) -> String {
    let mut label = Vec::new();
    loop {
        label.push((b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    label.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(columns: &[&str]) -> FormulaWorkbookLayout {
        FormulaWorkbookLayout::new([FormulaSheetLayout {
            id: "sheet".into(),
            name: "Sheet 1".into(),
            row_ids: vec!["row-1".into(), "row-2".into()],
            column_ids: columns.iter().map(|value| (*value).to_string()).collect(),
            row_offset: 0,
            column_offset: 0,
        }])
    }

    #[test]
    fn stable_reference_moves_when_column_is_inserted() {
        let original = layout(&["name", "price"]);
        let binding = capture_formula("=IF($B$1>A2,\"B2\",B2)", "sheet", &original);
        assert_eq!(binding.references.len(), 3);

        let inserted = layout(&["name", "category", "price"]);
        assert_eq!(
            render_formula(&binding, "sheet", &inserted),
            "=IF($C$1>A2,\"B2\",C2)"
        );
    }

    #[test]
    fn deleted_target_renders_ref_error() {
        let original = layout(&["name", "price"]);
        let binding = capture_formula("=B1", "sheet", &original);
        let deleted = layout(&["name"]);
        assert_eq!(render_formula(&binding, "sheet", &deleted), "=#REF!");
        assert!(binding_has_broken_reference(&binding, &deleted));
    }
}
