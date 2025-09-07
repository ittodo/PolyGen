using System;
using System.IO;
using System.Linq;
using Polygen.Common;
using System.Collections.Generic;
using System.Text.Json;
// Note: Generated BinaryReaders/BinaryWriters are excluded in this demo build.

class Program
{
    static object? ToRuntime(JsonElement e)
    {
        switch (e.ValueKind)
        {
            case JsonValueKind.Object:
                var dict = new Dictionary<string, object?>(StringComparer.Ordinal);
                foreach (var p in e.EnumerateObject())
                {
                    dict[p.Name] = ToRuntime(p.Value);
                }
                return dict;
            case JsonValueKind.Array:
                var list = new List<object?>();
                foreach (var v in e.EnumerateArray()) list.Add(ToRuntime(v));
                return list;
            case JsonValueKind.String:
                return e.GetString();
            case JsonValueKind.Number:
                if (e.TryGetInt64(out var l)) return l;
                if (e.TryGetDouble(out var d)) return d;
                return e.GetRawText();
            case JsonValueKind.True: return true;
            case JsonValueKind.False: return false;
            case JsonValueKind.Null: return null;
            default: return e.GetRawText();
        }
    }

    static void Main()
    {
        var baseDir = AppContext.BaseDirectory;
        var dataDir = Path.Combine(baseDir, "data");
        Directory.CreateDirectory(dataDir);

        // 입력 CSV 경로 (없으면 샘플 생성)
        var playerCsv = Path.Combine(dataDir, "players.csv");
        if (!File.Exists(playerCsv))
        {
            File.WriteAllLines(playerCsv, new[]
            {
                "id,name,level,stats.health,stats.mana,stats.attack,stats.defense,status",
                "1,Alice,10,100,50,20,10,Active",
                "2,Bob,7,80,35,15,8,Inactive"
            });
        }

        var monsterCsv = Path.Combine(dataDir, "monsters.csv");
        if (!File.Exists(monsterCsv))
        {
            File.WriteAllLines(monsterCsv, new[]
            {
                "id,name,stats.health,stats.mana,stats.attack,stats.defense,spawn_point.x,spawn_point.y,patrol_points[0].x,patrol_points[0].y,status,drop_items[0].item_id,drop_items[0].drop_chance,drop_items[0].enchantment.enchant_id,drop_items[0].enchantment.strength",
                "100,Slime,30,0,5,1,10.5,20.25,11.0,20.75,Active,1,0.5,101,0.9",
                "101,Goblin,60,5,12,4,30.0,40.0,, ,Inactive,2,0.25,,"
            });
        }

        Console.WriteLine("== ReadCsvFast (Player) ==");
        var players = Csv.game.character.Player.ReadCsvFast(playerCsv, ',', CsvUtils.GapMode.Sparse).ToList();
        foreach (var p in players)
        {
            Console.WriteLine($"Player {p.id}: {p.name} lv{p.level}, HP={p.stats?.health}, Status={p.status}");
        }

        Console.WriteLine();
        Console.WriteLine("== ReadCsvFast (Monster) ==");
        var monsters = Csv.game.character.Monster.ReadCsvFast(monsterCsv, ',', CsvUtils.GapMode.Sparse).ToList();
        foreach (var m in monsters)
        {
            var patrol0 = (m.patrol_points != null && m.patrol_points.Count > 0) ? $"({m.patrol_points[0].x},{m.patrol_points[0].y})" : "-";
            var drop0 = (m.drop_items != null && m.drop_items.Count > 0) ? m.drop_items[0] : null;
            Console.WriteLine($"Monster {m.id}: {m.name}, HP={m.stats?.health}, Spawn=({m.spawn_point?.x},{m.spawn_point?.y}), Patrol0={patrol0}, Status={m.status}, Drop0={(drop0 != null ? drop0.item_id.ToString() : "-")}");
        }

        // 쓰기 예제: 읽은 데이터를 새 CSV로 출력
        Console.WriteLine();
        Console.WriteLine("== WriteCsv (Player, Monster) ==");
        var outDir = Path.Combine(baseDir, "out");
        Directory.CreateDirectory(outDir);
        var playersOut = Path.Combine(outDir, "players_out.csv");
        var monstersOut = Path.Combine(outDir, "monsters_out.csv");
        Csv.game.character.Player.WriteCsv(players, playersOut, ',', CsvUtils.GapMode.Sparse);
        Csv.game.character.Monster.WriteCsv(monsters, monstersOut, ',', CsvUtils.GapMode.Sparse);
        Console.WriteLine($"Wrote: {playersOut}");
        Console.WriteLine($"Wrote: {monstersOut}");

        // CSV -> JSON
        Console.WriteLine();
        Console.WriteLine("== CSV -> JSON ==");
        var jsonDir = Path.Combine(baseDir, "json");
        Directory.CreateDirectory(jsonDir);
        var playersJson = Path.Combine(jsonDir, "players.json");
        var monstersJson = Path.Combine(jsonDir, "monsters.json");
        var jsonOpts = new JsonSerializerOptions { WriteIndented = true, IncludeFields = true };
        File.WriteAllText(playersJson, JsonSerializer.Serialize(players, jsonOpts));
        File.WriteAllText(monstersJson, JsonSerializer.Serialize(monsters, jsonOpts));
        Console.WriteLine($"Wrote: {playersJson}");
        Console.WriteLine($"Wrote: {monstersJson}");

        // JSON -> CSV (generic converter)
        Console.WriteLine();
        Console.WriteLine("== JSON -> CSV (generic) ==");
        var playersJsonText = File.ReadAllText(playersJson);
        var monstersJsonText = File.ReadAllText(monstersJson);
        var playersElem = JsonSerializer.Deserialize<JsonElement>(playersJsonText);
        var monstersElem = JsonSerializer.Deserialize<JsonElement>(monstersJsonText);
        var cfg = new JsonCsvConverter.Config { ListStrategy = "dynamic", IncludeHeader = true, Sep = ',' };
        var playersFromJsonCsv = JsonCsvConverter.JsonToCsv(ToRuntime(playersElem), cfg);
        var monstersFromJsonCsv = JsonCsvConverter.JsonToCsv(ToRuntime(monstersElem), cfg);
        var playersFromJsonCsvPath = Path.Combine(jsonDir, "players_from_json.csv");
        var monstersFromJsonCsvPath = Path.Combine(jsonDir, "monsters_from_json.csv");
        File.WriteAllText(playersFromJsonCsvPath, playersFromJsonCsv);
        File.WriteAllText(monstersFromJsonCsvPath, monstersFromJsonCsv);
        Console.WriteLine($"Wrote: {playersFromJsonCsvPath}");
        Console.WriteLine($"Wrote: {monstersFromJsonCsvPath}");

        // Binary write -> read -> write
        Console.WriteLine();
        Console.WriteLine("== Binary (write -> read -> write) ==");
        var binDir = Path.Combine(baseDir, "binout");
        Directory.CreateDirectory(binDir);
        var playersBin = Path.Combine(binDir, "players.bin");
        var monstersBin = Path.Combine(binDir, "monsters.bin");
        using (var fs = File.Create(playersBin))
        using (var bw = new BinaryWriter(fs))
        {
            bw.Write(players.Count);

            foreach (var p in players)
            {
                game.character.BinaryWriters.WritePlayer(bw, p);
            }
        }
        using (var fs = File.Create(monstersBin))
        using (var bw = new BinaryWriter(fs))
        {
            bw.Write(monsters.Count);
            foreach (var m in monsters)
            {
                game.character.BinaryWriters.WriteMonster(bw, m);
            }
        }
        Console.WriteLine($"Wrote: {playersBin}");
        Console.WriteLine($"Wrote: {monstersBin}");

        // Read back
        var players2 = new List<game.character.Player>();
        using (var fs = File.OpenRead(playersBin))
        using (var br = new BinaryReader(fs))
        {
            int n = br.ReadInt32();
            for (int i = 0; i < n; i++)
            {
                players2.Add(game.character.BinaryReaders.ReadPlayer(br));
            }
        }
        var monsters2 = new List<game.character.Monster>();
        using (var fs = File.OpenRead(monstersBin))
        using (var br = new BinaryReader(fs))
        {
            int n = br.ReadInt32();
            for (int i = 0; i < n; i++)
            {
                monsters2.Add(game.character.BinaryReaders.ReadMonster(br));
            }
        }

        // Write again
        var playersBin2 = Path.Combine(binDir, "players2.bin");
        var monstersBin2 = Path.Combine(binDir, "monsters2.bin");
        using (var fs = File.Create(playersBin2))
        using (var bw = new BinaryWriter(fs))
        {
            bw.Write(players2.Count);
            foreach (var p in players2)
            {
                game.character.BinaryWriters.WritePlayer(bw, p);
            }
        }
        using (var fs = File.Create(monstersBin2))
        using (var bw = new BinaryWriter(fs))
        {
            bw.Write(monsters2.Count);
            foreach (var m in monsters2)
            {
                game.character.BinaryWriters.WriteMonster(bw, m);
            }
        }
        Console.WriteLine($"Wrote: {playersBin2}");
        Console.WriteLine($"Wrote: {monstersBin2}");
    }
}
