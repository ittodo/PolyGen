
using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

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

        /// <summary>
        /// <see cref="Position"/> 클래스의 새 인스턴스를 초기화합니다.
        /// 이 매개변수 없는 생성자는 Entity Framework와 같은 프레임워크에 필요합니다.
        /// </summary>
        public Position()
        {
        }
        /// <summary>
        /// 지정된 값으로 <see cref="Position"/> 클래스의 새 인스턴스를 초기화합니다.
        /// </summary>
        /// <param name="x">The value for x.</param>
        /// <param name="y">The value for y.</param>
        public Position(
            f32 x,
            f32 y
        )
        {
            this.X = x;
            this.Y = y;
        }
    }
    public partial class StatBlock
    {

        public uint Health { get; set; }

        public uint Mana { get; set; }

        public uint Attack { get; set; }

        public uint Defense { get; set; }

        /// <summary>
        /// <see cref="StatBlock"/> 클래스의 새 인스턴스를 초기화합니다.
        /// 이 매개변수 없는 생성자는 Entity Framework와 같은 프레임워크에 필요합니다.
        /// </summary>
        public StatBlock()
        {
        }
        /// <summary>
        /// 지정된 값으로 <see cref="StatBlock"/> 클래스의 새 인스턴스를 초기화합니다.
        /// </summary>
        /// <param name="health">The value for health.</param>
        /// <param name="mana">The value for mana.</param>
        /// <param name="attack">The value for attack.</param>
        /// <param name="defense">The value for defense.</param>
        public StatBlock(
            uint health,
            uint mana,
            uint attack,
            uint defense
        )
        {
            this.Health = health;
            this.Mana = mana;
            this.Attack = attack;
            this.Defense = defense;
        }
    }
}