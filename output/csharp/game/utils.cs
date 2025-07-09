using System;
using System.Collections.Generic;
using game.common; // Position 타입을 사용하기 위해 추가

namespace game.utils
{
    /// <summary>
    /// 게임 전반에서 사용될 수 있는 유용한 헬퍼 메서드를 제공하는 static 클래스입니다.
    /// </summary>
    public static class GameUtils
    {
        private static readonly Random _random = new Random();

        /// <summary>
        /// 숫자 값을 지정된 최소값과 최대값 사이로 제한(Clamp)합니다.
        /// 스키마의 'range' 제약조건을 코드에서 구현할 때 유용합니다.
        /// </summary>
        /// <typeparam name="T">비교 가능한 모든 숫자 타입 (int, float, uint 등).</typeparam>
        /// <param name="value">제한할 값입니다.</param>
        /// <param name="min">허용되는 최소값입니다.</param>
        /// <param name="max">허용되는 최대값입니다.</param>
        /// <returns>제한된 범위 내의 값입니다.</returns>
        public static T Clamp<T>(this T value, T min, T max) where T : IComparable<T>
        {
            if (value.CompareTo(min) < 0) return min;
            if (value.CompareTo(max) > 0) return max;
            return value;
        }

        /// <summary>
        /// 문자열을 지정된 열거형(Enum) 타입으로 안전하게 변환합니다.
        /// 변환에 실패하면 지정된 기본값을 반환합니다.
        /// </summary>
        /// <typeparam name="T">변환할 Enum 타입.</typeparam>
        /// <param name="value">변환할 문자열.</param>
        /// <param name="defaultValue">변환 실패 시 반환할 기본값.</param>
        /// <returns>변환된 Enum 값 또는 기본값.</returns>
        public static T ParseEnum<T>(string value, T defaultValue) where T : struct, Enum
        {
            if (Enum.TryParse<T>(value, true, out var result))
            {
                return result;
            }
            return defaultValue;
        }

        /// <summary>
        /// 두 Position 간의 유클리드 거리를 계산합니다.
        /// </summary>
        /// <param name="posA">첫 번째 위치.</param>
        /// <param name="posB">두 번째 위치.</param>
        /// <returns>두 위치 사이의 거리.</returns>
        public static float Distance(this Position posA, Position posB)
        {
            float dx = posA.X - posB.X;
            float dy = posA.Y - posB.Y;
            return (float)Math.Sqrt(dx * dx + dy * dy);
        }

        /// <summary>
        /// 주어진 확률(0.0 to 1.0)에 따라 성공 여부를 결정합니다.
        /// 몬스터의 'drop_chance'와 같은 확률 기반 이벤트에 유용합니다.
        /// </summary>
        /// <param name="probability">성공 확률 (0.0에서 1.0 사이).</param>
        /// <returns>확률에 따라 성공하면 true, 그렇지 않으면 false.</returns>
        public static bool CheckChance(float probability)
        {
            if (probability >= 1.0f) return true;
            if (probability <= 0.0f) return false;
            return _random.NextDouble() < probability;
        }
    }
}