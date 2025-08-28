// Helper functions for readers generation (string utilities)
fn is_option(t) { t.starts_with("Option<") }
fn is_list(t) { t.starts_with("List<") }
fn unwrap_option(t) { let s = "Option<".len(); let l = t.len() - s - 1; t.sub_string(s, l) }
fn unwrap_list(t) { let s = "List<".len(); let l = t.len() - s - 1; t.sub_string(s, l) }
fn short_name(t) { let p = t.split("."); p[p.len-1] }
fn ns_prefix(t) { if !t.contains(".") { "" } else { let i = t.last_index_of("."); t.sub_string(0, i) } }

