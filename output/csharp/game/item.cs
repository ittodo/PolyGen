
using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

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

        /// <summary>
        /// <see cref="Item"/> 클래스의 새 인스턴스를 초기화합니다.
        /// 이 매개변수 없는 생성자는 Entity Framework와 같은 프레임워크에 필요합니다.
        /// </summary>
        public Item()
        {
        }
        /// <summary>
        /// 지정된 값으로 <see cref="Item"/> 클래스의 새 인스턴스를 초기화합니다.
        /// </summary>
        /// <param name="id">The value for id.</param>
        /// <param name="name">The value for name.</param>
        /// <param name="item_type">The value for item_type.</param>
        /// <param name="description">The value for description.</param>
        public Item(
            uint id,
            string name,
            ItemType item_type,
            string? description
        )
        {
            this.Id = id;
            this.Name = name;
            this.ItemType = item_type;
            this.Description = description;
        }
    }
}