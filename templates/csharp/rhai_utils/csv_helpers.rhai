// CSV helper: Embedded struct finder
fn find_embedded_struct(cur_s, name) {
    for it in cur_s.items { if it.is_embedded_struct() { let es = it.as_embedded_struct(); if es.name == name { return es; } } }
    ()
}
