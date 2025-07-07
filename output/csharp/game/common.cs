
using System;
using System.Collections.Generic;

namespace game.common
{

public enum Element
{
PHYSICAL,
FIRE,
ICE,
LIGHTNING,
}
public partial class Position
{
public f32 X { get; set; }
public f32 Y { get; set; }
}
public partial class StatBlock
{
public uint Health { get; set; }
public uint Mana { get; set; }
public uint Attack { get; set; }
public uint Defense { get; set; }
}
}