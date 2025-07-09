
using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

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

        /// <summary>
        /// <see cref="Player"/> 클래스의 새 인스턴스를 초기화합니다.
        /// 이 매개변수 없는 생성자는 Entity Framework와 같은 프레임워크에 필요합니다.
        /// </summary>
        public Player()
        {
        }
        /// <summary>
        /// 지정된 값으로 <see cref="Player"/> 클래스의 새 인스턴스를 초기화합니다.
        /// </summary>
        /// <param name="id">The value for id.</param>
        /// <param name="name">The value for name.</param>
        /// <param name="level">The value for level.</param>
        /// <param name="stats">The value for stats.</param>
        public Player(
            uint id,
            string name,
            u16 level,
            game.common.StatBlock stats
        )
        {
            this.Id = id;
            this.Name = name;
            this.Level = level;
            this.Stats = stats;
        }
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

        /// <summary>
        /// <see cref="Monster"/> 클래스의 새 인스턴스를 초기화합니다.
        /// 이 매개변수 없는 생성자는 Entity Framework와 같은 프레임워크에 필요합니다.
        /// </summary>
        public Monster()
        {
        }
        /// <summary>
        /// 지정된 값으로 <see cref="Monster"/> 클래스의 새 인스턴스를 초기화합니다.
        /// </summary>
        /// <param name="id">The value for id.</param>
        /// <param name="name">The value for name.</param>
        /// <param name="stats">The value for stats.</param>
        /// <param name="spawn_point">The value for spawn_point.</param>
        /// <param name="patrol_points">The value for patrol_points.</param>
        /// <param name="drop_items">The value for drop_items.</param>
        public Monster(
            uint id,
            string name,
            game.common.StatBlock stats,
            game.common.Position spawn_point,
            System.Collections.Generic.List<game.common.Position> patrol_points,
            System.Collections.Generic.List<DropItems> drop_items
        )
        {
            this.Id = id;
            this.Name = name;
            this.Stats = stats;
            this.SpawnPoint = spawn_point;
            this.PatrolPoints = patrol_points;
            this.DropItems = drop_items;
        }
    }
    public partial class DropItems
    {

        public uint ItemId { get; set; }

        public f32 DropChance { get; set; }

        /// <summary>
        /// <see cref="DropItems"/> 클래스의 새 인스턴스를 초기화합니다.
        /// 이 매개변수 없는 생성자는 Entity Framework와 같은 프레임워크에 필요합니다.
        /// </summary>
        public DropItems()
        {
        }
        /// <summary>
        /// 지정된 값으로 <see cref="DropItems"/> 클래스의 새 인스턴스를 초기화합니다.
        /// </summary>
        /// <param name="item_id">The value for item_id.</param>
        /// <param name="drop_chance">The value for drop_chance.</param>
        public DropItems(
            uint item_id,
            f32 drop_chance
        )
        {
            this.ItemId = item_id;
            this.DropChance = drop_chance;
        }
    }
}