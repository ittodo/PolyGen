using System;

[AttributeUsage(AttributeTargets.All, Inherited = false, AllowMultiple = true)]
sealed class taggableAttribute : Attribute { }

[AttributeUsage(AttributeTargets.All, Inherited = false, AllowMultiple = true)]
sealed class loadAttribute : Attribute { public string? type { get; set; } public string? path { get; set; } }

[AttributeUsage(AttributeTargets.All, Inherited = false, AllowMultiple = true)]
sealed class link_rowsAttribute : Attribute { public string? partition_by { get; set; } public string? link_with { get; set; } }

namespace System.ComponentModel.DataAnnotations.Schema { [System.AttributeUsage(System.AttributeTargets.All, AllowMultiple=true, Inherited=false)] sealed class IndexAttribute : System.Attribute { public string? Name { get; set; } public bool IsUnique { get; set; } public int Order { get; set; } public IndexAttribute(){} public IndexAttribute(string name, int order = 0){ Name=name; Order=order; } } }
