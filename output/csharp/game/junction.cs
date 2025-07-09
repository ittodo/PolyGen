
using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

namespace game.junction
{

    public partial class PlayerSkill
    {

        public uint PlayerId { get; set; }

        public uint SkillId { get; set; }

        public u16 SkillLevel { get; set; }

        /// <summary>
        /// <see cref="PlayerSkill"/> 클래스의 새 인스턴스를 초기화합니다.
        /// 이 매개변수 없는 생성자는 Entity Framework와 같은 프레임워크에 필요합니다.
        /// </summary>
        public PlayerSkill()
        {
        }
        /// <summary>
        /// 지정된 값으로 <see cref="PlayerSkill"/> 클래스의 새 인스턴스를 초기화합니다.
        /// </summary>
        /// <param name="player_id">The value for player_id.</param>
        /// <param name="skill_id">The value for skill_id.</param>
        /// <param name="skill_level">The value for skill_level.</param>
        public PlayerSkill(
            uint player_id,
            uint skill_id,
            u16 skill_level
        )
        {
            this.PlayerId = player_id;
            this.SkillId = skill_id;
            this.SkillLevel = skill_level;
        }
    }
    public partial class InventoryItem
    {
        [Key]

        public uint Id { get; set; }

        public uint PlayerId { get; set; }

        public uint ItemId { get; set; }

        public uint Quantity { get; set; }

        /// <summary>
        /// <see cref="InventoryItem"/> 클래스의 새 인스턴스를 초기화합니다.
        /// 이 매개변수 없는 생성자는 Entity Framework와 같은 프레임워크에 필요합니다.
        /// </summary>
        public InventoryItem()
        {
        }
        /// <summary>
        /// 지정된 값으로 <see cref="InventoryItem"/> 클래스의 새 인스턴스를 초기화합니다.
        /// </summary>
        /// <param name="id">The value for id.</param>
        /// <param name="player_id">The value for player_id.</param>
        /// <param name="item_id">The value for item_id.</param>
        /// <param name="quantity">The value for quantity.</param>
        public InventoryItem(
            uint id,
            uint player_id,
            uint item_id,
            uint quantity
        )
        {
            this.Id = id;
            this.PlayerId = player_id;
            this.ItemId = item_id;
            this.Quantity = quantity;
        }
    }
}