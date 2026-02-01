//! Parser for `.ptpl` template files.
//!
//! Converts template text into a tree of [`TemplateNode`]s. Every non-directive
//! line becomes an [`OutputLine`] node, maintaining a 1:1 correspondence between
//! template lines and output lines for source mapping.

use crate::template::expr::{self, CollectionExpr, Expr};

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
        condition: String,
        then_body: Vec<TemplateNode>,
        elif_branches: Vec<(String, Vec<TemplateNode>)>,
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
    BlankLine { line: usize },
    /// `%let name = expr` or `%set name = expr` — assign a variable in flat scope.
    LetSet {
        line: usize,
        /// Variable name.
        name: String,
        /// Raw expression string (may be literal, path, or Rhai expression).
        expr: String,
    },
    /// `%logic` ... `%endlogic` — raw Rhai code block.
    LogicBlock {
        line: usize,
        /// Raw Rhai source code (lines between `%logic` and `%endlogic`).
        body: String,
    },
    /// `%match subject` ... `%when pattern` ... `%else` ... `%endmatch`
    Match {
        line: usize,
        /// Subject expression string.
        subject: String,
        /// Match arms (`%when pattern [if guard]`).
        arms: Vec<MatchArm>,
        /// Optional else body (default case).
        else_body: Option<Vec<TemplateNode>>,
    },
    /// `%block name(params)` ... `%endblock` — reusable template block definition.
    BlockDef {
        line: usize,
        /// Block name.
        name: String,
        /// Parameter names.
        params: Vec<String>,
        /// Block body nodes.
        body: Vec<TemplateNode>,
    },
    /// `%render block_name with bindings` — invoke a defined block.
    Render {
        line: usize,
        /// Block name (or `$var_name` for dynamic dispatch).
        target: String,
        /// Optional context binding expression.
        binding: Option<String>,
    },
    /// `%while condition` ... `%endwhile` — loop while condition is true.
    While {
        line: usize,
        /// Loop condition (raw Rhai expression string).
        condition: String,
        /// Loop body nodes.
        body: Vec<TemplateNode>,
    },
}

/// A match arm for `%when pattern [if guard]`.
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// 1-based line number of `%when`.
    pub line: usize,
    /// Pattern string (literal, `_` wildcard, or tuple).
    pub pattern: String,
    /// Optional guard condition string.
    pub guard: Option<String>,
    /// Body nodes for this arm.
    pub body: Vec<TemplateNode>,
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
            } else if directive.starts_with("let ") || directive.starts_with("set ") {
                let node = parse_let_set(directive, line_num)?;
                nodes.push(node);
                *pos += 1;
                continue;
            } else if directive == "logic" {
                let node = parse_logic_block(lines, pos, source_file)?;
                nodes.push(node);
                continue;
            } else if directive.starts_with("match ") {
                let node = parse_match(lines, pos, source_file)?;
                nodes.push(node);
                continue;
            } else if directive.starts_with("block ") {
                let node = parse_block_def(lines, pos, source_file)?;
                nodes.push(node);
                continue;
            } else if directive.starts_with("render ") {
                let node = parse_render(directive, line_num)?;
                nodes.push(node);
                *pos += 1;
                continue;
            } else if directive.starts_with("while ") {
                let node = parse_while(lines, pos, source_file)?;
                nodes.push(node);
                continue;
            } else if directive == "blank" {
                nodes.push(TemplateNode::BlankLine { line: line_num });
                *pos += 1;
                continue;
            } else if directive.starts_with("elif ")
                || directive == "else"
                || directive == "endif"
                || directive == "endfor"
                || directive == "endlogic"
                || directive == "endmatch"
                || directive.starts_with("when ")
                || directive == "endblock"
                || directive == "endwhile"
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
    let condition = cond_str.trim().to_string();

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
            let elif_cond = elif_cond_str.trim().to_string();
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
        return Err(format!(
            "Line {}: Expected quoted path in %include",
            line_num
        ));
    };

    let template_path = path.to_string();
    let mut bindings = Vec::new();
    let mut indent = None;

    // Parse remaining: `with var, key=val indent N` or `with var indent=N`
    if !remainder.is_empty() {
        let mut parts = remainder;

        // Parse "with ..."
        if let Some(after_with) = parts.strip_prefix("with ") {
            parts = after_with.trim();
        }

        // Extract `indent N` or `indent=N` from the end before splitting by comma
        // This handles both "with s indent 1" and "with s, indent=1" syntax.
        let indent_re_space = " indent ";
        let indent_re_eq = " indent=";
        if let Some(pos) = parts.rfind(indent_re_space) {
            let val_str = parts[pos + indent_re_space.len()..].trim();
            indent =
                Some(val_str.parse::<usize>().map_err(|_| {
                    format!("Line {}: Invalid indent value: {}", line_num, val_str)
                })?);
            parts = parts[..pos].trim();
        } else if let Some(pos) = parts.rfind(indent_re_eq) {
            let val_str = parts[pos + indent_re_eq.len()..].trim();
            indent =
                Some(val_str.parse::<usize>().map_err(|_| {
                    format!("Line {}: Invalid indent value: {}", line_num, val_str)
                })?);
            parts = parts[..pos].trim();
        }

        // Parse bindings
        for token in parts.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }

            if let Some(val) = token.strip_prefix("indent=") {
                indent =
                    Some(val.parse::<usize>().map_err(|_| {
                        format!("Line {}: Invalid indent value: {}", line_num, val)
                    })?);
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

/// Parse `%let name = expr` or `%set name = expr`.
fn parse_let_set(directive: &str, line_num: usize) -> Result<TemplateNode, String> {
    // directive = "let name = expr" or "set name = expr"
    let rest = &directive[4..].trim(); // Skip "let " or "set "

    let eq_pos = rest.find('=').ok_or_else(|| {
        format!(
            "Line {}: Expected '=' in %let/%set directive: %{}",
            line_num, directive
        )
    })?;

    let name = rest[..eq_pos].trim().to_string();
    let expr = rest[eq_pos + 1..].trim().to_string();

    if name.is_empty() {
        return Err(format!(
            "Line {}: Empty variable name in %let/%set",
            line_num
        ));
    }
    if expr.is_empty() {
        return Err(format!("Line {}: Empty expression in %let/%set", line_num));
    }

    Ok(TemplateNode::LetSet {
        line: line_num,
        name,
        expr,
    })
}

/// Parse `%logic` ... `%endlogic` block.
///
/// Collects all lines between `%logic` and `%endlogic` as raw Rhai source.
fn parse_logic_block(
    lines: &[&str],
    pos: &mut usize,
    source_file: &str,
) -> Result<TemplateNode, String> {
    let line_num = *pos + 1;
    *pos += 1; // Skip the `%logic` line

    let mut body_lines = Vec::new();
    while *pos < lines.len() {
        let trimmed = lines[*pos].trim();
        if let Some(after_pct) = trimmed.strip_prefix('%') {
            if after_pct.trim_start() == "endlogic" {
                *pos += 1;
                let body = body_lines.join("\n");
                return Ok(TemplateNode::LogicBlock {
                    line: line_num,
                    body,
                });
            }
        }
        body_lines.push(lines[*pos]);
        *pos += 1;
    }

    Err(format!(
        "{}:{}: Unclosed %logic block (missing %endlogic)",
        source_file, line_num
    ))
}

/// Parse `%match subject` ... `%when pattern` ... `%else` ... `%endmatch` block.
fn parse_match(lines: &[&str], pos: &mut usize, source_file: &str) -> Result<TemplateNode, String> {
    let line_num = *pos + 1;
    let line = lines[*pos].trim();
    let subject = line[1..].trim_start()[6..].trim().to_string(); // Skip "%match "

    if subject.is_empty() {
        return Err(format!(
            "{}:{}: Empty subject in %match",
            source_file, line_num
        ));
    }

    *pos += 1;

    let mut arms = Vec::new();
    let mut else_body = None;

    // Skip any lines before the first %when (allow blank/comment lines)
    while *pos < lines.len() {
        let trimmed = lines[*pos].trim();
        if let Some(after_pct) = trimmed.strip_prefix('%') {
            let directive = after_pct.trim_start();
            if directive.starts_with("when ") || directive == "else" {
                break;
            } else if directive == "endmatch" {
                *pos += 1;
                return Ok(TemplateNode::Match {
                    line: line_num,
                    subject,
                    arms,
                    else_body,
                });
            } else if directive.starts_with("--") {
                *pos += 1;
                continue;
            } else {
                return Err(format!(
                    "{}:{}: Expected %when, %else, or %endmatch after %match",
                    source_file,
                    *pos + 1
                ));
            }
        } else if trimmed.is_empty() {
            *pos += 1;
            continue;
        } else {
            return Err(format!(
                "{}:{}: Unexpected output line inside %match (before %when)",
                source_file,
                *pos + 1
            ));
        }
    }

    // Parse %when arms
    while *pos < lines.len() {
        let trimmed = lines[*pos].trim();
        if let Some(after_pct) = trimmed.strip_prefix('%') {
            let directive = after_pct.trim_start();

            if let Some(when_rest) = directive.strip_prefix("when ") {
                let when_line = *pos + 1;
                let (pattern, guard) = parse_when_pattern(when_rest.trim());
                *pos += 1;

                let body = parse_block(
                    lines,
                    pos,
                    source_file,
                    Some(&["when ", "else", "endmatch"]),
                )?;

                arms.push(MatchArm {
                    line: when_line,
                    pattern,
                    guard,
                    body,
                });
            } else if directive == "else" {
                *pos += 1;
                else_body = Some(parse_block(lines, pos, source_file, Some(&["endmatch"]))?);
            } else if directive == "endmatch" {
                *pos += 1;
                break;
            } else {
                return Err(format!(
                    "{}:{}: Expected %when, %else, or %endmatch",
                    source_file,
                    *pos + 1
                ));
            }
        } else {
            return Err(format!(
                "{}:{}: Expected %when, %else, or %endmatch",
                source_file,
                *pos + 1
            ));
        }
    }

    Ok(TemplateNode::Match {
        line: line_num,
        subject,
        arms,
        else_body,
    })
}

/// Parse a `%when` pattern, extracting optional `if guard` clause.
///
/// Examples:
/// - `"value"` → pattern=`"value"`, guard=None
/// - `"value" if x > 0` → pattern=`"value"`, guard=Some("x > 0")
/// - `_` → pattern=`_`, guard=None
fn parse_when_pattern(input: &str) -> (String, Option<String>) {
    // Look for ` if ` guard clause (not inside quotes)
    let bytes = input.as_bytes();
    let mut in_string = false;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            in_string = !in_string;
        } else if !in_string && i + 4 <= bytes.len() && &bytes[i..i + 4] == b" if " {
            let pattern = input[..i].trim().to_string();
            let guard = input[i + 4..].trim().to_string();
            return (pattern, if guard.is_empty() { None } else { Some(guard) });
        }
        i += 1;
    }

    (input.trim().to_string(), None)
}

/// Parse `%block name(params)` ... `%endblock`.
fn parse_block_def(
    lines: &[&str],
    pos: &mut usize,
    source_file: &str,
) -> Result<TemplateNode, String> {
    let line_num = *pos + 1;
    let line = lines[*pos].trim();
    let rest = line[1..].trim_start()[6..].trim(); // Skip "%block "

    // Parse "name(param1, param2)" or just "name"
    let (name, params) = if let Some(paren_start) = rest.find('(') {
        let paren_end = rest.find(')').ok_or_else(|| {
            format!(
                "{}:{}: Unclosed parenthesis in %block",
                source_file, line_num
            )
        })?;
        let name = rest[..paren_start].trim().to_string();
        let params_str = &rest[paren_start + 1..paren_end];
        let params: Vec<String> = params_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        (name, params)
    } else {
        (rest.to_string(), Vec::new())
    };

    if name.is_empty() {
        return Err(format!(
            "{}:{}: Empty block name in %block",
            source_file, line_num
        ));
    }

    *pos += 1;
    let body = parse_block(lines, pos, source_file, Some(&["endblock"]))?;

    // Consume %endblock
    if *pos < lines.len() {
        let trimmed = lines[*pos].trim();
        if let Some(after_pct) = trimmed.strip_prefix('%') {
            if after_pct.trim_start() == "endblock" {
                *pos += 1;
            }
        }
    }

    Ok(TemplateNode::BlockDef {
        line: line_num,
        name,
        params,
        body,
    })
}

/// Parse `%render target [with binding]`.
fn parse_render(directive: &str, line_num: usize) -> Result<TemplateNode, String> {
    let rest = directive[7..].trim(); // Skip "render "

    // Split on "with" keyword
    let (target, binding) = if let Some(with_pos) = rest.find(" with ") {
        let target = rest[..with_pos].trim().to_string();
        let binding = rest[with_pos + 6..].trim().to_string();
        (
            target,
            if binding.is_empty() {
                None
            } else {
                Some(binding)
            },
        )
    } else {
        (rest.to_string(), None)
    };

    if target.is_empty() {
        return Err(format!("Line {}: Empty target in %render", line_num));
    }

    Ok(TemplateNode::Render {
        line: line_num,
        target,
        binding,
    })
}

/// Parse `%while condition` ... `%endwhile` block.
fn parse_while(lines: &[&str], pos: &mut usize, source_file: &str) -> Result<TemplateNode, String> {
    let line_num = *pos + 1;
    let line = lines[*pos].trim();
    let cond_str = &line[1..].trim_start()[6..]; // Skip "%while "
    let condition = cond_str.trim().to_string();

    *pos += 1;
    let body = parse_block(lines, pos, source_file, Some(&["endwhile"]))?;

    // Consume %endwhile
    if *pos < lines.len() {
        let trimmed = lines[*pos].trim();
        if let Some(after_pct) = trimmed.strip_prefix('%') {
            if after_pct.trim_start() == "endwhile" {
                *pos += 1;
            }
        }
    }

    Ok(TemplateNode::While {
        line: line_num,
        condition,
        body,
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
    fn test_parse_let_set() {
        let template = "%let count = 10";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        match &parsed.nodes[0] {
            TemplateNode::LetSet { name, expr, .. } => {
                assert_eq!(name, "count");
                assert_eq!(expr, "10");
            }
            _ => panic!("Expected LetSet"),
        }
    }

    #[test]
    fn test_parse_set() {
        let template = "%set name = struct.name";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        match &parsed.nodes[0] {
            TemplateNode::LetSet { name, expr, .. } => {
                assert_eq!(name, "name");
                assert_eq!(expr, "struct.name");
            }
            _ => panic!("Expected LetSet"),
        }
    }

    #[test]
    fn test_parse_let_string_literal() {
        let template = "%let greeting = \"hello world\"";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        match &parsed.nodes[0] {
            TemplateNode::LetSet { name, expr, .. } => {
                assert_eq!(name, "greeting");
                assert_eq!(expr, "\"hello world\"");
            }
            _ => panic!("Expected LetSet"),
        }
    }

    #[test]
    fn test_parse_logic_block() {
        let template = "%logic\nlet x = 1;\nlet y = x + 2;\n%endlogic";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        match &parsed.nodes[0] {
            TemplateNode::LogicBlock { body, .. } => {
                assert_eq!(body, "let x = 1;\nlet y = x + 2;");
            }
            _ => panic!("Expected LogicBlock"),
        }
    }

    #[test]
    fn test_parse_logic_block_empty() {
        let template = "%logic\n%endlogic";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        match &parsed.nodes[0] {
            TemplateNode::LogicBlock { body, .. } => {
                assert_eq!(body, "");
            }
            _ => panic!("Expected LogicBlock"),
        }
    }

    #[test]
    fn test_parse_match_simple() {
        let template = "%match field.field_type.type_name\n%when \"u32\"\n    uint\n%when \"string\"\n    string\n%else\n    object\n%endmatch";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        match &parsed.nodes[0] {
            TemplateNode::Match {
                subject,
                arms,
                else_body,
                ..
            } => {
                assert_eq!(subject, "field.field_type.type_name");
                assert_eq!(arms.len(), 2);
                assert_eq!(arms[0].pattern, "\"u32\"");
                assert_eq!(arms[1].pattern, "\"string\"");
                assert!(else_body.is_some());
            }
            _ => panic!("Expected Match"),
        }
    }

    #[test]
    fn test_parse_match_with_guard() {
        let template = "%match field.field_type.type_name\n%when \"u32\" if field.is_optional\n    nullable_uint\n%endmatch";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        match &parsed.nodes[0] {
            TemplateNode::Match { arms, .. } => {
                assert_eq!(arms.len(), 1);
                assert_eq!(arms[0].pattern, "\"u32\"");
                assert_eq!(arms[0].guard, Some("field.is_optional".to_string()));
            }
            _ => panic!("Expected Match"),
        }
    }

    #[test]
    fn test_parse_match_wildcard() {
        let template = "%match value\n%when _\n    default\n%endmatch";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        match &parsed.nodes[0] {
            TemplateNode::Match { arms, .. } => {
                assert_eq!(arms.len(), 1);
                assert_eq!(arms[0].pattern, "_");
            }
            _ => panic!("Expected Match"),
        }
    }

    #[test]
    fn test_parse_block_def() {
        let template = "%block field_line(f)\n    public {{f.name}};\n%endblock";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        match &parsed.nodes[0] {
            TemplateNode::BlockDef {
                name, params, body, ..
            } => {
                assert_eq!(name, "field_line");
                assert_eq!(params, &vec!["f".to_string()]);
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected BlockDef"),
        }
    }

    #[test]
    fn test_parse_block_no_params() {
        let template = "%block header\n    // Header\n%endblock";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        match &parsed.nodes[0] {
            TemplateNode::BlockDef { name, params, .. } => {
                assert_eq!(name, "header");
                assert!(params.is_empty());
            }
            _ => panic!("Expected BlockDef"),
        }
    }

    #[test]
    fn test_parse_render() {
        let template = "%render field_line with field";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        match &parsed.nodes[0] {
            TemplateNode::Render {
                target, binding, ..
            } => {
                assert_eq!(target, "field_line");
                assert_eq!(*binding, Some("field".to_string()));
            }
            _ => panic!("Expected Render"),
        }
    }

    #[test]
    fn test_parse_render_no_binding() {
        let template = "%render header";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        match &parsed.nodes[0] {
            TemplateNode::Render {
                target, binding, ..
            } => {
                assert_eq!(target, "header");
                assert!(binding.is_none());
            }
            _ => panic!("Expected Render"),
        }
    }

    #[test]
    fn test_parse_render_dynamic() {
        let template = "%render $block_name with item";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        match &parsed.nodes[0] {
            TemplateNode::Render {
                target, binding, ..
            } => {
                assert_eq!(target, "$block_name");
                assert_eq!(*binding, Some("item".to_string()));
            }
            _ => panic!("Expected Render"),
        }
    }

    #[test]
    fn test_parse_while() {
        let template = "%while counter.has_remaining\n    line\n%endwhile";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        match &parsed.nodes[0] {
            TemplateNode::While {
                condition, body, ..
            } => {
                assert_eq!(condition, "counter.has_remaining");
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected While"),
        }
    }

    #[test]
    fn test_parse_for_with_where() {
        let template =
            "%for field in struct.fields | where field.is_primary_key\n    {{field.name}}\n%endfor";
        let parsed = parse_template(template, "test.ptpl").unwrap();
        match &parsed.nodes[0] {
            TemplateNode::ForLoop {
                variable,
                collection,
                body,
                ..
            } => {
                assert_eq!(variable, "field");
                assert_eq!(collection.path, vec!["struct", "fields"]);
                assert!(collection.where_filter.is_some());
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected ForLoop"),
        }
    }

    #[test]
    fn test_parse_let_error_no_equals() {
        let result = parse_template("%let x", "test.ptpl");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_logic_unclosed() {
        let result = parse_template("%logic\nlet x = 1;", "test.ptpl");
        assert!(result.is_err());
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
