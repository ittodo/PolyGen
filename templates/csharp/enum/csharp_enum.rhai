public enum ${e.name}
{
${ render_items(e.items, "templates/csharp/enum/csharp_enum_body.rhai", "item") }
}
