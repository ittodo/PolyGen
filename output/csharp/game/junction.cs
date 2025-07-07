
using System;
using System.Collections.Generic;

namespace game.junction
{

public partial class PlayerSkill
{
public uint PlayerId { get; set; }
public uint SkillId { get; set; }
public u16 SkillLevel { get; set; }
}
public partial class InventoryItem
{
[Key]
public uint Id { get; set; }
public uint PlayerId { get; set; }
public uint ItemId { get; set; }
public uint Quantity { get; set; }
}
}