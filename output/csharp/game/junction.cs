
using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

namespace game.junction
{

    /// <summary>
    /// 플레이어와 스킬의 다대다(N:M) 관계를 위한 연결 테이블입니다.
    /// // @link_rows 어노테이션을 사용하여, player_id로 그룹화된 스킬들을
    /// // skill_id를 기준으로 연결 리스트처럼 만듭니다.
    /// </summary>
    public partial class PlayerSkill
    {
        /// <summary>
        /// // 외래 키(foreign_key)에 'as'를 사용하여 참조되는 테이블에 관계를 생성합니다.
        /// // Player 테이블에 'skills' 라는 이름의 N:M 관계를 생성합니다.
        /// </summary>

        public uint PlayerId { get; set; }
        /// <summary>
        /// // Skill 테이블에 'users' 라는 이름의 N:M 관계를 생성합니다.
        /// </summary>

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
        /// <param name="player_id">// 외래 키(foreign_key)에 'as'를 사용하여 참조되는 테이블에 관계를 생성합니다. // Player 테이블에 'skills' 라는 이름의 N:M 관계를 생성합니다.</param>
        /// <param name="skill_id">// Skill 테이블에 'users' 라는 이름의 N:M 관계를 생성합니다.</param>
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
    /// <summary>
    /// 플레이어 인벤토리 항목을 나타내는 테이블입니다. (1:N 관계의 'N'쪽)
    /// </summary>
    public partial class InventoryItem
    {
        [Key]

        public uint Id { get; set; }
        /// <summary>
        /// // 'as inventory'를 통해 Player 테이블에 'inventory'라는 1:N 관계를 생성합니다.
        /// </summary>

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
        /// <param name="player_id">// 'as inventory'를 통해 Player 테이블에 'inventory'라는 1:N 관계를 생성합니다.</param>
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