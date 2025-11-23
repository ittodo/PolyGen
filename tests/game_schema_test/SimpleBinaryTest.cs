using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

// 간단한 Position 타입 정의
public class Position
{
    public float x;
    public float y;
}

// 간단한 StatBlock 타입 정의
public class StatBlock
{
    public uint health;
    public uint mana;
    public uint attack;
    public uint defense;
}

// 간단한 Binary 유틸리티
public static class SimpleBinaryUtils
{
    public static void WriteUtf8String(BinaryWriter bw, string value)
    {
        var bytes = Encoding.UTF8.GetBytes(value);
        bw.Write(bytes.Length);
        bw.Write(bytes);
    }

    public static string ReadUtf8String(BinaryReader br)
    {
        int length = br.ReadInt32();
        var bytes = br.ReadBytes(length);
        return Encoding.UTF8.GetString(bytes);
    }

    public static void WritePosition(BinaryWriter bw, Position pos)
    {
        bw.Write(pos.x);
        bw.Write(pos.y);
    }

    public static Position ReadPosition(BinaryReader br)
    {
        return new Position
        {
            x = br.ReadSingle(),
            y = br.ReadSingle()
        };
    }

    public static void WriteStatBlock(BinaryWriter bw, StatBlock stats)
    {
        bw.Write(stats.health);
        bw.Write(stats.mana);
        bw.Write(stats.attack);
        bw.Write(stats.defense);
    }

    public static StatBlock ReadStatBlock(BinaryReader br)
    {
        return new StatBlock
        {
            health = br.ReadUInt32(),
            mana = br.ReadUInt32(),
            attack = br.ReadUInt32(),
            defense = br.ReadUInt32()
        };
    }

    public static void WriteList<T>(BinaryWriter bw, List<T> list, Action<BinaryWriter, T> writeElement)
    {
        bw.Write(list.Count);
        foreach (var item in list)
        {
            writeElement(bw, item);
        }
    }

    public static List<T> ReadList<T>(BinaryReader br, Func<BinaryReader, T> readElement)
    {
        int count = br.ReadInt32();
        var list = new List<T>(count);
        for (int i = 0; i < count; i++)
        {
            list.Add(readElement(br));
        }
        return list;
    }
}

// 간단한 Player 클래스
public enum PlayerStatus
{
    Active,
    Inactive,
    Banned
}

public class Player
{
    public uint id;
    public string name = "";
    public ushort level;
    public StatBlock stats = new StatBlock();
    public PlayerStatus status;

    public static void Write(BinaryWriter bw, Player obj)
    {
        bw.Write(obj.id);
        SimpleBinaryUtils.WriteUtf8String(bw, obj.name);
        bw.Write(obj.level);
        SimpleBinaryUtils.WriteStatBlock(bw, obj.stats);
        bw.Write((int)obj.status);
    }

    public static Player Read(BinaryReader br)
    {
        return new Player
        {
            id = br.ReadUInt32(),
            name = SimpleBinaryUtils.ReadUtf8String(br),
            level = br.ReadUInt16(),
            stats = SimpleBinaryUtils.ReadStatBlock(br),
            status = (PlayerStatus)br.ReadInt32()
        };
    }
}

// 테스트 프로그램
public class SimpleBinaryTest
{
    public static void Main(string[] args)
    {
        Console.WriteLine("=== Simple Binary Read/Write Test ===\n");

        try
        {
            TestPlayer();
            TestPosition();
            TestStatBlock();
            TestPlayerList();

            Console.WriteLine("\n✅ All tests passed successfully!");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"\n❌ Test failed: {ex.Message}");
            Console.WriteLine(ex.StackTrace);
            Environment.Exit(1);
        }
    }

    static void TestPlayer()
    {
        Console.WriteLine("1. Testing Player Binary Read/Write...");

        var original = new Player
        {
            id = 12345,
            name = "테스트플레이어",
            level = 99,
            stats = new StatBlock
            {
                health = 1000,
                mana = 500,
                attack = 150,
                defense = 200
            },
            status = PlayerStatus.Active
        };

        using var ms = new MemoryStream();
        using (var writer = new BinaryWriter(ms, Encoding.UTF8, true))
        {
            Player.Write(writer, original);
        }

        Console.WriteLine($"   Written {ms.Length} bytes");

        ms.Position = 0;
        Player loaded;
        using (var reader = new BinaryReader(ms, Encoding.UTF8, true))
        {
            loaded = Player.Read(reader);
        }

        AssertEqual(original.id, loaded.id, "Player.id");
        AssertEqual(original.name, loaded.name, "Player.name");
        AssertEqual(original.level, loaded.level, "Player.level");
        AssertEqual(original.stats.health, loaded.stats.health, "Player.stats.health");
        AssertEqual(original.stats.mana, loaded.stats.mana, "Player.stats.mana");
        AssertEqual(original.status, loaded.status, "Player.status");

        Console.WriteLine($"   ✓ Player: id={loaded.id}, name={loaded.name}, level={loaded.level}, hp={loaded.stats.health}");
    }

    static void TestPosition()
    {
        Console.WriteLine("\n2. Testing Position Binary Read/Write...");

        var original = new Position { x = 123.456f, y = 789.012f };

        using var ms = new MemoryStream();
        using (var writer = new BinaryWriter(ms, Encoding.UTF8, true))
        {
            SimpleBinaryUtils.WritePosition(writer, original);
        }

        ms.Position = 0;
        Position loaded;
        using (var reader = new BinaryReader(ms, Encoding.UTF8, true))
        {
            loaded = SimpleBinaryUtils.ReadPosition(reader);
        }

        AssertEqual(original.x, loaded.x, "Position.x");
        AssertEqual(original.y, loaded.y, "Position.y");

        Console.WriteLine($"   ✓ Position: ({loaded.x}, {loaded.y})");
    }

    static void TestStatBlock()
    {
        Console.WriteLine("\n3. Testing StatBlock Binary Read/Write...");

        var original = new StatBlock
        {
            health = 5000,
            mana = 3000,
            attack = 999,
            defense = 777
        };

        using var ms = new MemoryStream();
        using (var writer = new BinaryWriter(ms, Encoding.UTF8, true))
        {
            SimpleBinaryUtils.WriteStatBlock(writer, original);
        }

        ms.Position = 0;
        StatBlock loaded;
        using (var reader = new BinaryReader(ms, Encoding.UTF8, true))
        {
            loaded = SimpleBinaryUtils.ReadStatBlock(reader);
        }

        AssertEqual(original.health, loaded.health, "StatBlock.health");
        AssertEqual(original.mana, loaded.mana, "StatBlock.mana");

        Console.WriteLine($"   ✓ StatBlock: hp={loaded.health}, mana={loaded.mana}, atk={loaded.attack}, def={loaded.defense}");
    }

    static void TestPlayerList()
    {
        Console.WriteLine("\n4. Testing List<Player> Binary Read/Write...");

        var originalList = new List<Player>
        {
            new Player { id = 1, name = "플레이어1", level = 10, stats = new StatBlock { health = 100, mana = 50, attack = 10, defense = 5 }, status = PlayerStatus.Active },
            new Player { id = 2, name = "플레이어2", level = 20, stats = new StatBlock { health = 200, mana = 100, attack = 20, defense = 10 }, status = PlayerStatus.Inactive },
            new Player { id = 3, name = "플레이어3", level = 30, stats = new StatBlock { health = 300, mana = 150, attack = 30, defense = 15 }, status = PlayerStatus.Banned }
        };

        using var ms = new MemoryStream();
        using (var writer = new BinaryWriter(ms, Encoding.UTF8, true))
        {
            SimpleBinaryUtils.WriteList(writer, originalList, Player.Write);
        }

        Console.WriteLine($"   Written {ms.Length} bytes for {originalList.Count} players");

        ms.Position = 0;
        List<Player> loadedList;
        using (var reader = new BinaryReader(ms, Encoding.UTF8, true))
        {
            loadedList = SimpleBinaryUtils.ReadList(reader, Player.Read);
        }

        AssertEqual(originalList.Count, loadedList.Count, "List.Count");
        for (int i = 0; i < originalList.Count; i++)
        {
            AssertEqual(originalList[i].id, loadedList[i].id, $"Player[{i}].id");
            AssertEqual(originalList[i].name, loadedList[i].name, $"Player[{i}].name");
            AssertEqual(originalList[i].level, loadedList[i].level, $"Player[{i}].level");
        }

        Console.WriteLine($"   ✓ Loaded {loadedList.Count} players successfully");
        foreach (var player in loadedList)
        {
            Console.WriteLine($"      - {player.name}: lv{player.level}, hp={player.stats.health}, status={player.status}");
        }
    }

    static void AssertEqual<T>(T expected, T actual, string fieldName)
    {
        if (!EqualityComparer<T>.Default.Equals(expected, actual))
        {
            throw new Exception($"Assertion failed for {fieldName}: expected {expected}, got {actual}");
        }
    }
}
