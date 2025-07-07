
using System;
using System.Collections.Generic;

namespace game.character
{

public partial class Player
{
[Key]
public uint Id { get; set; }
[Index(IsUnique = true)]
[MaxLength(30)]
public string Name { get; set; }
public u16 Level { get; set; }
public game.common.StatBlock Stats { get; set; }
}
public partial class Monster
{
[Key]
public uint Id { get; set; }
[MaxLength(50)]
public string Name { get; set; }
public game.common.StatBlock Stats { get; set; }
public game.common.Position SpawnPoint { get; set; }
public System.Collections.Generic.List<game.common.Position> PatrolPoints { get; set; }
public System.Collections.Generic.List<DropItems> DropItems { get; set; }
}
public partial class DropItems
{
public uint ItemId { get; set; }
public f32 DropChance { get; set; }
}
}