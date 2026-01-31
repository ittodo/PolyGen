//! Parser for `.ptpl` template files.
//!
//! Converts template text into a tree of [`TemplateNode`]s. Every non-directive
//! line becomes an [`OutputLine`] node, maintaining a 1:1 correspondence between
//! template lines and output lines for source mapping.

use crate::template::expr::{self, CollectionExpr, CondExpr, Expr};

/// A parsed template ready for rendering.
#[derive(Debug, Clone)]
pub struct ParsedTemplate {
    /// Source file path (relative to templates dir).
    pub source_file: String,
    /// Top-level nodes.
    pub nodes: Vec<TemplateNode>,
}

/// A node in the parsed template tree.
#[derive(Debug, Clone)]
pub enum TemplateNode {
    /// A literal output line with interpolation spans.
    OutputLine {
        /// 1-based line number in the source .ptpl file.
        line: usize,
        /// Segments that make up this line.
        segments: Vec<LineSegment>,
    },
    /// `%if condition` ... `%elif` ... `%else` ... `%endif`
    Conditional {
        line: usize,
        condition: CondExpr,
        then_body: Vec<TemplateNode>,
        elif_branches: Vec<(CondExpr, Vec<TemplateNode>)>,
        else_body: Option<Vec<TemplateNode>>,
    },
    /// `%for var in collection` ... `%endfor`
    ForLoop {
        line: usize,
        variable: String,
        collection: CollectionExpr,
        body: Vec<TemplateNode>,
    },
    /// `%include "path" with expr`
    Include {
        line: usize,
        template_path: String,
        context_bindings: Vec<IncludeBinding>,
        indent: Option<usize>,
    },
    /// `%blank` — emit an empty output line.
    BlankLine {
        line: usize,
    },
}

/// A segment within an output line.
#[derive(Debug, Clone)]
pub enum LineSegment {
    /// Literal text (no interpolation).
    Literal(String),
    /// An interpolation expression `{{expr | filter}}`.
    Expression(Expr),
}

/// A binding in `%include "path" with var, key=value`.
#[derive(Debug, Clone)]
pub enum IncludeBinding {
    /// Context focus: `with struct` — sets the primary context object.
    Focus(String),
    /// Key-value: `depth=2`, `ns_prefix="game"`.
    KeyValue(String, String),
}

/// Parse a `.ptpl` template file.
///
/// Returns a [`ParsedTemplate`] or an error with line information.
pub fn parse_template(source: &str, source_file: &str) -> Result<ParsedTemplate, String> {
    let lines: Vec<&str> = source.lines().collect();
    let nodes = parse_block(&lines, &mut 0, source_file, None)?;
    Ok(ParsedTemplate {
        source_file: source_file.to_string(),
        nodes,
    })
}

/// Parse a block of lines until a terminator is found.
///
/// `terminator` is the directive keyword that ends this block (e.g. "endif", "endfor").
fn parse_block(
    lines: &[&str],
    pos: &mut usize,
    source_file: &str,
    terminator: Option<&[&str]>,
) -> Result<Vec<TemplateNode>, String> {
    let mut nodes = Vec::new();

    while *pos < lines.len() {
        let line_num = *pos + 1; // 1-based
        let line = lines[*pos];

        // Check for directive lines
        let trimmed = line.trim();

        if let Some(after_pct) = trimmed.strip_prefix('%') {
            let directive = after_pct.trim_start();

            // Check terminators
            if let Some(terms) = terminator {
                for term in terms {
                    if directive.starts_with(term) {
                        return Ok(nodes);
                    }
                }
            }

            // Parse directive
            if directive.starts_with("--") {
                // Comment: skip
                *pos += 1;
                continue;
            } else if directive.starts_with("if ") {
                let node = parse_conditional(lines, pos, source_file)?;
                nodes.push(node);
                continue;
            } else if directive.starts_with("for ") {
                let node = parse_for_loop(lines, pos, source_file)?;
                nodes.push(node);
                continue;
            } else if directive.starts_with("include ") {
                let node = parse_include(directive, line_num)?;
                nodes.push(node);
                *pos += 1;
                continue;
            } else if directive == "blank" {
                nodes.push(TemplateNode::BlankLine { line: line_num });
                *pos += 1;
                continue;
            } else if directive.starts_with("elif ")
                || directive == "else"
                || directive == "endif"
                || directive == "endfor"
            {
                // These are handled by parent parsers
                return Ok(nodes);
            } else {
                return Err(format!(
                    "{}:{}: Unknown directive: %{}",
                    source_file, line_num, directive
                ));
            }
        }

        // Not a directive — it's an output line
        let segments = parse_line_segments(line)?;
        nodes.push(TemplateNode::OutputLine {
            line: line_num,
            segments,
        });
        *pos += 1;
    }

    if terminator.is_some() {
        return Err(format!(
            "{}: Unexpected end of file (unclosed block)",
            source_file
        ));
    }

    Ok(nodes)
}

/// Parse a `%if` ... `%elif` ... `%else` ... `%endif` block.
fn parse_conditional(
    lines: &[&str],
    pos: &mut usize,
    source_file: &str,
) -> Result<TemplateNode, String> {
    let line_num = *pos + 1;
    let line = lines[*pos].trim();
    let cond_str = &line[1..].trim_start()[3..]; // Skip "%if "
    let condition = expr::parse_condition(cond_str.trim())
        .map_err(|e| format!("{}:{}: {}", source_file, line_num, e))?;

    *pos += 1;

    // Parse then body
    let then_body = parse_block(lines, pos, source_file, Some(&["elif ", "else", "endif"]))?;

    let mut elif_branches = Vec::new();
    let mut else_body = None;

    // Handle elif/else/endif
    while *pos < lines.len() {
        let trimmed = lines[*pos].trim();
        let directive = trimmed.strip_prefix('%').unwrap_or(trimmed).trim_start();

        if let Some(elif_cond_str) = directive.strip_prefix("elif ") {
            let elif_line = *pos + 1;
            let elif_cond = expr::parse_condition(elif_cond_str.trim())
                .map_err(|e| format!("{}:{}: {}", source_file, elif_line, e))?;
            *pos += 1;
            let elif_body =
                parse_block(lines, pos, source_file, Some(&["elif ", "else", "endif"]))?;
            elif_branches.push((elif_cond, elif_body));
        } else if directive == "else" {
            *pos += 1;
            else_body = Some(parse_block(lines, pos, source_file, Some(&["endif"]))?);
        } else if directive == "endif" {
            *pos += 1;
            break;
        } else {
            return Err(format!(
                "{}:{}: Expected %elif, %else, or %endif",
                source_file,
                *pos + 1
            ));
        }
    }

    Ok(TemplateNode::Conditional {
        line: line_num,
        condition,
        then_body,
        elif_branches,
        else_body,
    })
}

/// Parse a `%for var in collection` ... `%endfor` block.
fn parse_for_loop(
    lines: &[&str],
    pos: &mut usize,
    source_file: &str,
) -> Result<TemplateNode, String> {
    let line_num = *pos + 1;
    let line = lines[*pos].trim();
    let for_str = &line[1..].trim_start()[4..].trim(); // Skip "%for "

    // Parse "var in collection"
    let parts: Vec<&str> = for_str.splitn(3, ' ').collect();
    if parts.len() != 3 || parts[1] != "in" {
        return Err(format!(
            "{}:{}: Expected '%for variable in collection'",
            source_file, line_num
        ));
    }

    let variable = parts[0].to_string();
    let collection = expr::parse_collection(parts[2])
        .map_err(|e| format!("{}:{}: {}", source_file, line_num, e))?;

    *pos += 1;
    let body = parse_block(lines, pos, source_file, Some(&["endfor"]))?;

    // Consume %endfor
    if *pos < lines.len() {
        let trimmed = lines[*pos].trim();
        if trimmed.trim_start_matches('%').trim() == "endfor" {
            *pos += 1;
        }
    }

    Ok(TemplateNode::ForLoop {
        line: line_num,
        variable,
        collection,
        body,
    })
}

/// Parse a `%include "path" with ...` directive.
fn parse_include(directive: &str, line_num: usize) -> Result<TemplateNode, String> {
    // directive = `include "path" with expr, key=val indent=N`
    let rest = directive[8..].trim(); // Skip "include "

    // Extract quoted path
    let (path, remainder) = if let Some(after_quote) = rest.strip_prefix('"') {
        let end = after_quote
            .find('"')
            .ok_or_else(|| format!("Line {}: Unclosed quote in %include", line_num))?;
        (&after_quote[..end], after_quote[end + 1..].trim())
    } else {
        return Err(format!("Line {}: Expected quoted path in %include", line_num));
    };

    let template_path = path.to_string();
    let mut bindings = Vec::new();
    let mut indent = None;

    // Parse remaining: `with var, key=val indent=N`
    if !remainder.is_empty() {
        let mut parts = remainder;

        // Parse "with ..."
        if let Some(after_with) = parts.strip_prefix("with ") {
            parts = after_with.trim();
        }

        // Parse bindings and indent
        for token in parts.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }

            if let Some(val) = token.strip_prefix("indent=") {
                indent = Some(
                    val.parse::<usize>()
                        .map_err(|_| format!("Line {}: Invalid indent value: {}", line_num, val))?,
                );
            } else if token.contains('=') {
                let mut kv = token.splitn(2, '=');
                let key = kv.next().unwrap().trim().to_string();
                let value = kv.next().unwrap().trim().trim_matches('"').to_string();
                bindings.push(IncludeBinding::KeyValue(key, value));
            } else {
                bindings.push(IncludeBinding::Focus(token.to_string()));
            }
        }
    }

    Ok(TemplateNode::Include {
        line: line_num,
        template_path,
        context_bindings: bindings,
        indent,
    })
}

/// Parse an output line into segments (literal text and `{{expr}}` interpolations).
fn parse_line_segments(line: &str) -> Result<Vec<LineSegment>, String> {
    let mut segments = Vec::new();
    let mut remaining = line;

    while !remaining.is_empty() {
        if let Some(start) = remaining.find("{{") {
            // Add literal before the expression
            if start > 0 {
                segments.push(LineSegment::Literal(remaining[..start].to_string()));
            }

            // Find closing }}
            let after_open = &remaining[start + 2..];
            let end = after_open
                .find("}}")
                .ok_or_else(|| format!("Unclosed '{{{{' in line: {}", line))?;

            let expr_str = &after_open[..end];
            let expression = expr::parse_expr(expr_str)
                .map_err(|e| format!("In expression '{{{{{}}}}}': {}", expr_str, e))?;
            segments.push(LineSegment::Expression(expression));

            remaining = &after_open[end + 2..];
        } else {
            // Rest is all literal
            segments.push(LineSegment::Literal(remaining.to_string()));
            break;
        }
    }

    Ok(segments)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_output() {
        let template = "using System;\nusing System.IO;";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        assert_eq!(parsed.nodes.len(), 2);
        match &parsed.nodes[0] {
            TemplateNode::OutputLine { line, segments } => {
                assert_eq!(*line, 1);
                assert_eq!(segments.len(), 1);
            }
            _ => panic!("Expected OutputLine"),
        }
    }

    #[test]
    fn test_parse_interpolation() {
        let template = "public class {{struct.name}} : IDataRow";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        match &parsed.nodes[0] {
            TemplateNode::OutputLine { segments, .. } => {
                assert_eq!(segments.len(), 3); // "public class " + expr + " : IDataRow"
                match &segments[1] {
                    LineSegment::Expression(expr) => {
                        assert_eq!(expr.path, vec!["struct", "name"]);
                    }
                    _ => panic!("Expected Expression"),
                }
            }
            _ => panic!("Expected OutputLine"),
        }
    }

    #[test]
    fn test_parse_comment() {
        let template = "%-- This is a comment\noutput line";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        assert_eq!(parsed.nodes.len(), 1); // Comment is skipped
        match &parsed.nodes[0] {
            TemplateNode::OutputLine { line, .. } => assert_eq!(*line, 2),
            _ => panic!("Expected OutputLine"),
        }
    }

    #[test]
    fn test_parse_blank() {
        let template = "%blank";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        assert!(matches!(parsed.nodes[0], TemplateNode::BlankLine { .. }));
    }

    #[test]
    fn test_parse_for_loop() {
        let template = "%for field in struct.fields\n    public {{field.name}};\n%endfor";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        match &parsed.nodes[0] {
            TemplateNode::ForLoop {
                variable,
                collection,
                body,
                ..
            } => {
                assert_eq!(variable, "field");
                assert_eq!(collection.path, vec!["struct", "fields"]);
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected ForLoop"),
        }
    }

    #[test]
    fn test_parse_conditional() {
        let template = "%if field.is_primary_key\n    [Key]\n%else\n    // normal\n%endif";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        match &parsed.nodes[0] {
            TemplateNode::Conditional {
                then_body,
                else_body,
                ..
            } => {
                assert_eq!(then_body.len(), 1);
                assert!(else_body.is_some());
                assert_eq!(else_body.as_ref().unwrap().len(), 1);
            }
            _ => panic!("Expected Conditional"),
        }
    }

    #[test]
    fn test_parse_include() {
        let template = "%include \"section/class_body\" with struct, indent=1";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        match &parsed.nodes[0] {
            TemplateNode::Include {
                template_path,
                context_bindings,
                indent,
                ..
            } => {
                assert_eq!(template_path, "section/class_body");
                assert_eq!(context_bindings.len(), 1);
                assert_eq!(*indent, Some(1));
            }
            _ => panic!("Expected Include"),
        }
    }

    #[test]
    fn test_parse_elif() {
        let template = "%if item.is_struct\n  struct\n%elif item.is_enum\n  enum\n%elif item.is_namespace\n  ns\n%else\n  other\n%endif";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        match &parsed.nodes[0] {
            TemplateNode::Conditional {
                then_body,
                elif_branches,
                else_body,
                ..
            } => {
                assert_eq!(then_body.len(), 1);
                assert_eq!(elif_branches.len(), 2);
                assert!(else_body.is_some());
            }
            _ => panic!("Expected Conditional"),
        }
    }
}
