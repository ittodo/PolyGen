use pest::iterators::Pair;
use crate::Rule;

/// Helper function to parse a path (e.g., "game.common")
pub fn parse_path(pair: Pair<Rule>) -> Vec<String> {
    pair.into_inner()
        .filter(|p| p.as_rule() == Rule::IDENT)
        .map(|p| p.as_str().to_string())
        .collect()
}

/// Extract the content from a comment pair, removing comment markers
pub fn extract_comment_content(comment_pair: Pair<Rule>) -> String {
    let s = comment_pair.as_str();
    if s.starts_with("///") {
        s.strip_prefix("///").unwrap().trim().to_string()
    } else if s.starts_with("//") {
        s.strip_prefix("//").unwrap().trim().to_string()
    } else if s.starts_with("/*") {
        s.strip_prefix("/*")
            .unwrap()
            .trim_end_matches("*/")
            .trim()
            .to_string()
    } else {
        s.trim().to_string()
    }
}
