using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
 

 
namespace game.common
{    /// <summary>
    /// 스킬이나 공격의 속성을 나타내는 열거형입니다.
    /// </summary>
    public enum Element
    {
        PHYSICAL,
        FIRE,
        ICE,
        LIGHTNING,
    }     /// <summary>
    /// 게임 월드 내의 2D 좌표를 나타내는 복합 타입입니다.
    /// `embed`를 사용하여 여러 테이블에서 재사용할 수 있습니다.
    /// </summary>
    public struct Position
    {
        public float X { get; set; }
        public float Y { get; set; }
    }     /// <summary>
    /// 캐릭터의 기본 능력치를 묶은 구조체입니다.
    /// </summary>
    public struct StatBlock
    {
        public uint Health { get; set; }
        public uint Mana { get; set; }
        public uint Attack { get; set; }
        public uint Defense { get; set; }
    } 
}
 


 
namespace game.item
{    /// <summary>
    /// 아이템의 종류를 나타내는 열거형입니다.
    /// </summary>
    public enum ItemType
    {
        WEAPON,
        ARMOR,
        POTION,
        MATERIAL,
    }     /// <summary>
    /// 아이템 정보를 정의하는 테이블입니다.
    /// </summary>
    public class Item
    {
        [Key]
        public uint Id { get; set; }
        public string Name { get; set; }
        public ItemType ItemType { get; set; }
        public string Description { get; set; }
    } 
}
 


 
namespace game.character
{    /// <summary>
    /// 플레이어 캐릭터 정보를 정의하는 테이블입니다.
    /// </summary>
    public class Player
    {
        [Key]
        public uint Id { get; set; }
        [MaxLength(30)]
        public string Name { get; set; }
        public ushort Level { get; set; }
        public StatBlock Stats { get; set; }
    }     /// <summary>
    /// 몬스터 정보를 정의하는 테이블입니다.
    /// 모든 테이블은 `@taggable` 어노테이션을 통해 자유로운 태그를 붙일 수 있습니다.
    /// </summary>
    public class Monster
    {
        public class DropItems
        {
            public uint ItemId { get; set; }
            public float DropChance { get; set; }
        }
        
        [Key]
        public uint Id { get; set; }
        public string Name { get; set; }
        public StatBlock Stats { get; set; }
        public Position SpawnPoint { get; set; }
        public List<Position> PatrolPoints { get; set; }
        public List<DropItems> DropItems { get; set; }
    } 
}
 


 
namespace game.character.skill
{    /// <summary>
    /// 스킬 정보를 정의하는 테이블입니다.
    /// </summary>
    public class Skill
    {
        [Key]
        public uint Id { get; set; }
        public string Name { get; set; }
        public string Description { get; set; }
        public Element Element { get; set; }
        public uint Power { get; set; }
    } 
}
 


 
namespace game.junction
{    /// <summary>
    /// 플레이어와 스킬의 다대다(N:M) 관계를 위한 연결 테이블입니다.
    /// </summary>
    public class PlayerSkill
    {
        public uint PlayerId { get; set; }
        public uint SkillId { get; set; }
        public ushort SkillLevel { get; set; }
    }     /// <summary>
    /// 플레이어 인벤토리 항목을 나타내는 테이블입니다. (1:N 관계의 'N'쪽)
    /// </summary>
    public class InventoryItem
    {
        [Key]
        public uint Id { get; set; }
        public uint PlayerId { get; set; }
        public uint ItemId { get; set; }
        public uint Quantity { get; set; }
    } 
}
 