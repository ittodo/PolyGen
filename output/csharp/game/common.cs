
using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

namespace game.common
{

    /// <summary>
    /// 스킬이나 공격의 속성을 나타내는 열거형입니다.
    /// </summary>
    public enum Element
    {
        /// <summary>
        /// 테스트
        /// </summary>
        PHYSICAL,
        FIRE,
        ICE,
        LIGHTNING,
    }
    /// <summary>
    /// 게임 월드 내의 2D 좌표를 나타내는 복합 타입입니다.
    /// `embed`를 사용하여 여러 테이블에서 재사용할 수 있습니다.
    /// </summary>
    public partial class Position
    {
        /// <summary>
        /// 테스트2
        /// </summary>

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
        /// <param name="x">테스트2</param>
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
    /// <summary>
    /// 캐릭터의 기본 능력치를 묶은 구조체입니다.
    /// </summary>
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