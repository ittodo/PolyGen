using System;

// 간단한 컴파일 테스트 프로그램
// 모든 생성된 타입들이 제대로 정의되었는지만 확인합니다.
public class CompileTest
{
    public static void Main(string[] args)
    {
        Console.WriteLine("✅ Game Schema compiled successfully!");
        Console.WriteLine("All types, attributes, and namespaces are valid.");
        
        // 타입들이 제대로 정의되었는지 확인
        var playerType = typeof(game.character.Player);
        var itemType = typeof(game.item.Item);
        var monsterType = typeof(game.character.Monster);
        var skillType = typeof(game.character.skill.Skill);
        
        Console.WriteLine($"- Player: {playerType.FullName}");
        Console.WriteLine($"- Item: {itemType.FullName}");
        Console.WriteLine($"- Monster: {monsterType.FullName}");
        Console.WriteLine($"- Skill: {skillType.FullName}");
        
        // 어트리뷰트 확인
        var playerAttrs = playerType.GetCustomAttributes(false);
        Console.WriteLine($"\nPlayer has {playerAttrs.Length} custom attributes");
        foreach (var attr in playerAttrs)
        {
            Console.WriteLine($"  - {attr.GetType().Name}");
        }
    }
}
