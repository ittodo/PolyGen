
using System;
using System.Collections.Generic;

namespace game.item
{

public enum ItemType
{
WEAPON,
ARMOR,
POTION,
MATERIAL,
}
public partial class Item
{
[Key]
public uint Id { get; set; }
[Index(IsUnique = true)]
public string Name { get; set; }
public ItemType ItemType { get; set; }
public string? Description { get; set; }
}
}