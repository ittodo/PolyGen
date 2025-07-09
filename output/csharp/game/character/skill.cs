
using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

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

        /// <summary>
        /// <see cref="Skill"/> 클래스의 새 인스턴스를 초기화합니다.
        /// 이 매개변수 없는 생성자는 Entity Framework와 같은 프레임워크에 필요합니다.
        /// </summary>
        public Skill()
        {
        }
        /// <summary>
        /// 지정된 값으로 <see cref="Skill"/> 클래스의 새 인스턴스를 초기화합니다.
        /// </summary>
        /// <param name="id">The value for id.</param>
        /// <param name="name">The value for name.</param>
        /// <param name="description">The value for description.</param>
        /// <param name="element">The value for element.</param>
        /// <param name="power">The value for power.</param>
        public Skill(
            uint id,
            string name,
            string? description,
            game.common.Element element,
            uint power
        )
        {
            this.Id = id;
            this.Name = name;
            this.Description = description;
            this.Element = element;
            this.Power = power;
        }
    }
}