using System;
using System.IO;
using System.Linq;
using Polygen.Common;

class Program
{
    static void Main()
    {
        var baseDir = AppContext.BaseDirectory;
        var dataDir = Path.Combine(baseDir, "data");
        Directory.CreateDirectory(dataDir);

        // Write dummy CSVs matching generated schemas
        var playerCsv = Path.Combine(dataDir, "players.csv");
        File.WriteAllLines(playerCsv, new[]
        {
            "id,name,level,stats.health,stats.mana,stats.attack,stats.defense,status",
            "1,Alice,10,100,50,20,10,Active",
            "2,Bob,7,80,35,15,8,Inactive"
        });

        var monsterCsv = Path.Combine(dataDir, "monsters.csv");
        File.WriteAllLines(monsterCsv, new[]
        {
            "id,name,stats.health,stats.mana,stats.attack,stats.defense,spawn_point.x,spawn_point.y,patrol_points[0].x,patrol_points[0].y,status,drop_items[0].item_id,drop_items[0].drop_chance,drop_items[0].enchantment.enchant_id,drop_items[0].enchantment.strength",
            "100,Slime,30,0,5,1,10.5,20.25,11.0,20.75,Active,1,0.5,101,0.9",
            "101,Goblin,60,5,12,4,30.0,40.0,, ,Inactive,2,0.25,,"
        });

        Console.WriteLine("== ReadCsvFast (Player) ==");
        foreach (var p in game.character.PlayerCsv.ReadCsvFast(playerCsv, ',' , CsvUtils.GapMode.Sparse))
        {
            Console.WriteLine($"Player {p.id}: {p.name} lv{p.level}, HP={p.stats?.health}, Status={p.status}");
        }

        Console.WriteLine();
        Console.WriteLine("== ReadCsvFast (Monster) ==");
        foreach (var m in game.character.MonsterCsv.ReadCsvFast(monsterCsv, ',' , CsvUtils.GapMode.Sparse))
        {
            var patrol0 = (m.patrol_points != null && m.patrol_points.Count > 0) ? $"({m.patrol_points[0].x},{m.patrol_points[0].y})" : "-";
            var drop0 = (m.drop_items != null && m.drop_items.Count > 0) ? m.drop_items[0] : null;
            Console.WriteLine($"Monster {m.id}: {m.name}, HP={m.stats?.health}, Spawn=({m.spawn_point?.x},{m.spawn_point?.y}), Patrol0={patrol0}, Status={m.status}, Drop0={(drop0!=null?drop0.item_id.ToString():"-")}");
        }

        Console.WriteLine();
        Console.WriteLine("== ReadCsvWithHeader (Monster, reused header tree) ==");
        var header = File.ReadLines(monsterCsv).First().Split(',');
        var h = game.character.MonsterCsv.BuildHeader(header, "");
        foreach (var line in File.ReadLines(monsterCsv).Skip(1))
        {
            var row = line.Split(',');
            var m = game.character.MonsterCsv.FromRowWithHeader(h, row, CsvUtils.GapMode.Sparse);
            Console.WriteLine($"Monster {m.id}: {m.name}, Status={m.status}");
        }
    }
}
