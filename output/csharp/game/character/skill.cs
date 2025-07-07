
using System;
using System.Collections.Generic;

namespace game.character.skill
{

public partial class Skill
{
[Key]
public uint Id { get; set; }
[Index(IsUnique = true)]
public string Name { get; set; }
public string? Description { get; set; }
public game.common.Element Element { get; set; }
public uint Power { get; set; }
}
}