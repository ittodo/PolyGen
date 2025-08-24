// This template generates a C# enum definition.
// It expects 'e' to be an EnumDef object.


public enum ${e.name}
{
    
    ${ render_items(e.items, "templates/csharp/enum/csharp_enum_body.rhai") }
    
}