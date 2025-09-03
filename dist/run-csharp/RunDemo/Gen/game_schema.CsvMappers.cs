using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;
using System.Globalization;
using System.IO;
using System.Text;
using Polygen.Common;
namespace game.item
{
internal static class ItemCsv
{
        private static readonly string[] __Headers_Item = new string[] { "id", "name", "item_type", "description" };
        internal static int ColumnCount_Item() => __Headers_Item.Length;
        internal static string[] GetHeader_Item() => (string[])__Headers_Item.Clone();
        internal static void AppendRow(Item obj, List<string> cols)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.id));
cols.Add(CsvUtils.ToStringInvariant(obj.name));
cols.Add((obj.item_type).ToString());
cols.Add(CsvUtils.ToStringInvariant(obj.description));
        }
        internal static string[] ToRow(Item obj) { var list = new List<string>(ColumnCount_Item()); AppendRow(obj, list); return list.ToArray(); }
        internal static void WriteCsv(IEnumerable<Item> items, string path, bool writeHeader = true, char sep = ',') { using var sw = new StreamWriter(path, false, new UTF8Encoding(false)); if (writeHeader) sw.WriteLine(CsvUtils.Join(GetHeader_Item(), sep)); foreach (var it in items) { var row = ToRow(it); sw.WriteLine(CsvUtils.Join(row, sep)); } }
        internal static System.Collections.Generic.Dictionary<string,int> ComputeListMaxes(System.Collections.Generic.IEnumerable<Item> items)
        {
            var d = new System.Collections.Generic.Dictionary<string,int>(System.StringComparer.OrdinalIgnoreCase);
            foreach (var it in items) {
            }
            return d;
        }
        internal static string[] GetDynamicHeader(System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
            var cols = new System.Collections.Generic.List<string>();
            cols.Add("id");
            cols.Add("name");
            cols.Add("item_type");
            cols.Add("description");
            return cols.ToArray();
        }
        internal static void AppendRowDynamic(Item obj, System.Collections.Generic.List<string> cols, System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.id));
cols.Add(CsvUtils.ToStringInvariant(obj.name));
cols.Add((obj.item_type).ToString());
cols.Add(CsvUtils.ToStringInvariant(obj.description));
        }
        internal static void WriteCsvDynamic(System.Collections.Generic.IEnumerable<Item> items, string path, bool writeHeader = true, char sep = ',')
        {
            var maxes = ComputeListMaxes(items);
            using var sw = new System.IO.StreamWriter(path, false, new System.Text.UTF8Encoding(false));
            if (writeHeader) { var hdr = GetDynamicHeader(maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(hdr, sep)); }
            foreach (var it in items) { var row = new System.Collections.Generic.List<string>(256); AppendRowDynamic(it, row, maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep)); }
        }
        internal sealed class CsvIndex
        {
            public int idx_id = -1;
            public int idx_name = -1;
            public int idx_item_type = -1;
            public int idx_description = -1;
        }
        internal static bool CsvIndexHasAny(Index idx)
        {
            if (idx.idx_id >= 0) return true;
            if (idx.idx_name >= 0) return true;
            if (idx.idx_item_type >= 0) return true;
            if (idx.idx_description >= 0) return true;
            return false;
        }
        internal static bool CsvIndexHasValues(Index idx, string[] row)
        {
            if (idx.idx_id >= 0 && idx.idx_id < row.Length && !string.IsNullOrEmpty(row[idx.idx_id])) return true;
            if (idx.idx_name >= 0 && idx.idx_name < row.Length && !string.IsNullOrEmpty(row[idx.idx_name])) return true;
            if (idx.idx_item_type >= 0 && idx.idx_item_type < row.Length && !string.IsNullOrEmpty(row[idx.idx_item_type])) return true;
            if (idx.idx_description >= 0 && idx.idx_description < row.Length && !string.IsNullOrEmpty(row[idx.idx_description])) return true;
            return false;
        }
        internal static CsvIndex BuildCsvIndex(string[] header, string prefix)
        {
            var idx = new CsvIndex();
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            if (!map.TryGetValue(prefix + "id", out idx.idx_id)) idx.idx_id = -1;
            if (!map.TryGetValue(prefix + "name", out idx.idx_name)) idx.idx_name = -1;
            if (!map.TryGetValue(prefix + "item_type", out idx.idx_item_type)) idx.idx_item_type = -1;
            if (!map.TryGetValue(prefix + "description", out idx.idx_description)) idx.idx_description = -1;
            return idx;
        }
        internal static Item FromRowWithCsvIndex(Index idx, string[] row, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new Item();
            if (idx.idx_id >= 0 && idx.idx_id < row.Length) { var __cell = row[idx.idx_id]; obj.id = DataSourceFactory.ConvertValue<uint>( __cell ); }
            if (idx.idx_name >= 0 && idx.idx_name < row.Length) { var __cell = row[idx.idx_name]; obj.name = DataSourceFactory.ConvertValue<string>( __cell ); }
            if (idx.idx_item_type >= 0 && idx.idx_item_type < row.Length) { var __cell = row[idx.idx_item_type]; obj.item_type = DataSourceFactory.ConvertValue<ItemType>( __cell ); }
            if (idx.idx_description >= 0 && idx.idx_description < row.Length) { var __cell = row[idx.idx_description]; obj.description = DataSourceFactory.ConvertValue<string>( __cell ); }
            return obj;
        }
        internal sealed class Reader
        {
            public readonly CsvIndex Index;
            public readonly char Sep;
            public readonly Polygen.Common.CsvUtils.GapMode Gap;
            public Reader(Index index, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { CsvIndex = index; Sep = sep; Gap = gap; }
            public Item Parse(string[] row) => FromRowWithCsvIndex(Index, row, Gap);
            internal static Reader FromHeader(string[] header, char sep = ',', string prefix = "", Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { var idx = BuildCsvIndex(header, prefix); return new Reader(idx, sep, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<Item> ReadCsvWithIndex(string path, CsvIndex idx, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length <= 1) yield break;
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<Item> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var idx = BuildCsvIndex(header, string.Empty);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static Item FromRow(IDictionary<string,string> row) => FromRowWithPrefix(row, string.Empty);
        internal static Item FromRowWithPrefix(IDictionary<string,string> row, string prefix)
        {
            var obj = new Item();
obj.id = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "id");
obj.name = DataSourceFactory.ConvertSingleValue<string>(row, prefix + "name");
obj.item_type = DataSourceFactory.ConvertSingleValue<ItemType>(row, prefix + "item_type");
obj.description = DataSourceFactory.ConvertSingleValue<string>(row, prefix + "description");
            return obj;
        }
        internal static IEnumerable<Item> ReadCsv(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); yield return FromRowWithPrefixAndHeaderMap(map, row, string.Empty, gap); }
        }
        internal static Item FromRow(string[] header, string[] row, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) => FromRowWithPrefixAndHeader(header, row, string.Empty, gap);
        internal static Item FromRowWithPrefixAndHeader(string[] header, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            return FromRowWithPrefixAndHeaderMap(map, row, prefix, gap);
        }
        internal static Item FromRowWithPrefixAndHeaderMap(System.Collections.Generic.Dictionary<string,int> map, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new Item();
            int gap = (int)gap; // 0=Break,1=Sparse
            bool __pfxEmpty = string.IsNullOrEmpty(prefix);
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "id" : prefix + "id"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.id = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "name" : prefix + "name"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.name = DataSourceFactory.ConvertValue<string>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "item_type" : prefix + "item_type"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.item_type = DataSourceFactory.ConvertValue<ItemType>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "description" : prefix + "description"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.description = DataSourceFactory.ConvertValue<string>(__cell); }
            return obj;
        }
}
}

namespace game.character
{
internal static class PlayerCsv
{
        private static readonly string[] __Headers_Player = new string[] { "id", "name", "level", "stats.health", "stats.mana", "stats.attack", "stats.defense", "status" };
        internal static int ColumnCount_Player() => __Headers_Player.Length;
        internal static string[] GetHeader_Player() => (string[])__Headers_Player.Clone();
        internal static void AppendRow(Player obj, List<string> cols)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.id));
cols.Add(CsvUtils.ToStringInvariant(obj.name));
cols.Add(CsvUtils.ToStringInvariant(obj.level));
if (obj.stats == null) { for (int i=0;i< game.common.StatBlockCsv.ColumnCount_StatBlock(); i++) cols.Add(string.Empty); } else { game.common.StatBlockCsv.AppendRow(obj.stats, cols); }
cols.Add((obj.status).ToString());
        }
        internal static string[] ToRow(Player obj) { var list = new List<string>(ColumnCount_Player()); AppendRow(obj, list); return list.ToArray(); }
        internal static void WriteCsv(IEnumerable<Player> items, string path, bool writeHeader = true, char sep = ',') { using var sw = new StreamWriter(path, false, new UTF8Encoding(false)); if (writeHeader) sw.WriteLine(CsvUtils.Join(GetHeader_Player(), sep)); foreach (var it in items) { var row = ToRow(it); sw.WriteLine(CsvUtils.Join(row, sep)); } }
        internal static System.Collections.Generic.Dictionary<string,int> ComputeListMaxes(System.Collections.Generic.IEnumerable<Player> items)
        {
            var d = new System.Collections.Generic.Dictionary<string,int>(System.StringComparer.OrdinalIgnoreCase);
            foreach (var it in items) {
            }
            return d;
        }
        internal static string[] GetDynamicHeader(System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
            var cols = new System.Collections.Generic.List<string>();
            cols.Add("id");
            cols.Add("name");
            cols.Add("level");
            cols.Add("stats.health");
            cols.Add("stats.mana");
            cols.Add("stats.attack");
            cols.Add("stats.defense");
            cols.Add("status");
            return cols.ToArray();
        }
        internal static void AppendRowDynamic(Player obj, System.Collections.Generic.List<string> cols, System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.id));
cols.Add(CsvUtils.ToStringInvariant(obj.name));
cols.Add(CsvUtils.ToStringInvariant(obj.level));
if (obj.stats == null) { for (int i=0;i< game.common.StatBlockCsv.ColumnCount_StatBlock(); i++) cols.Add(string.Empty); } else { game.common.StatBlockCsv.AppendRow(obj.stats, cols); }
cols.Add((obj.status).ToString());
        }
        internal static void WriteCsvDynamic(System.Collections.Generic.IEnumerable<Player> items, string path, bool writeHeader = true, char sep = ',')
        {
            var maxes = ComputeListMaxes(items);
            using var sw = new System.IO.StreamWriter(path, false, new System.Text.UTF8Encoding(false));
            if (writeHeader) { var hdr = GetDynamicHeader(maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(hdr, sep)); }
            foreach (var it in items) { var row = new System.Collections.Generic.List<string>(256); AppendRowDynamic(it, row, maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep)); }
        }
        internal sealed class CsvIndex
        {
            public int idx_id = -1;
            public int idx_name = -1;
            public int idx_level = -1;
            public StatBlockCsv.CsvIndex idx_stats;
            public int idx_status = -1;
        }
        internal static bool CsvIndexHasAny(Index idx)
        {
            if (idx.idx_id >= 0) return true;
            if (idx.idx_name >= 0) return true;
            if (idx.idx_level >= 0) return true;
            if (idx.idx_stats != null && StatBlockCsv.CsvIndexHasAny(idx.idx_stats)) return true;
            if (idx.idx_status >= 0) return true;
            return false;
        }
        internal static bool CsvIndexHasValues(Index idx, string[] row)
        {
            if (idx.idx_id >= 0 && idx.idx_id < row.Length && !string.IsNullOrEmpty(row[idx.idx_id])) return true;
            if (idx.idx_name >= 0 && idx.idx_name < row.Length && !string.IsNullOrEmpty(row[idx.idx_name])) return true;
            if (idx.idx_level >= 0 && idx.idx_level < row.Length && !string.IsNullOrEmpty(row[idx.idx_level])) return true;
            if (idx.idx_stats != null && StatBlockCsv.CsvIndexHasValues(idx.idx_stats, row)) return true;
            if (idx.idx_status >= 0 && idx.idx_status < row.Length && !string.IsNullOrEmpty(row[idx.idx_status])) return true;
            return false;
        }
        internal static CsvIndex BuildCsvIndex(string[] header, string prefix)
        {
            var idx = new CsvIndex();
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            if (!map.TryGetValue(prefix + "id", out idx.idx_id)) idx.idx_id = -1;
            if (!map.TryGetValue(prefix + "name", out idx.idx_name)) idx.idx_name = -1;
            if (!map.TryGetValue(prefix + "level", out idx.idx_level)) idx.idx_level = -1;
            idx.idx_stats = StatBlockCsv.BuildIndex(header, prefix + "stats.");
            if (!map.TryGetValue(prefix + "status", out idx.idx_status)) idx.idx_status = -1;
            return idx;
        }
        internal static Player FromRowWithCsvIndex(Index idx, string[] row, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new Player();
            if (idx.idx_id >= 0 && idx.idx_id < row.Length) { var __cell = row[idx.idx_id]; obj.id = DataSourceFactory.ConvertValue<uint>( __cell ); }
            if (idx.idx_name >= 0 && idx.idx_name < row.Length) { var __cell = row[idx.idx_name]; obj.name = DataSourceFactory.ConvertValue<string>( __cell ); }
            if (idx.idx_level >= 0 && idx.idx_level < row.Length) { var __cell = row[idx.idx_level]; obj.level = DataSourceFactory.ConvertValue<ushort>( __cell ); }
            if (!StatBlockCsv.CsvIndexHasValues(idx.idx_stats, row)) { obj.stats = null; } else { obj.stats = StatBlockCsv.FromRowWithIndex(idx.idx_stats, row, gap); }
            if (idx.idx_status >= 0 && idx.idx_status < row.Length) { var __cell = row[idx.idx_status]; obj.status = DataSourceFactory.ConvertValue<Status__Enum>( __cell ); }
            return obj;
        }
        internal sealed class Reader
        {
            public readonly CsvIndex Index;
            public readonly char Sep;
            public readonly Polygen.Common.CsvUtils.GapMode Gap;
            public Reader(Index index, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { CsvIndex = index; Sep = sep; Gap = gap; }
            public Player Parse(string[] row) => FromRowWithCsvIndex(Index, row, Gap);
            internal static Reader FromHeader(string[] header, char sep = ',', string prefix = "", Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { var idx = BuildCsvIndex(header, prefix); return new Reader(idx, sep, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<Player> ReadCsvWithIndex(string path, CsvIndex idx, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length <= 1) yield break;
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<Player> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var idx = BuildCsvIndex(header, string.Empty);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static Player FromRow(IDictionary<string,string> row) => FromRowWithPrefix(row, string.Empty);
        internal static Player FromRowWithPrefix(IDictionary<string,string> row, string prefix)
        {
            var obj = new Player();
obj.id = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "id");
obj.name = DataSourceFactory.ConvertSingleValue<string>(row, prefix + "name");
obj.level = DataSourceFactory.ConvertSingleValue<ushort>(row, prefix + "level");
{ bool any=false; string tmp; if (row.TryGetValue(prefix + "stats.health", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (row.TryGetValue(prefix + "stats.mana", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (row.TryGetValue(prefix + "stats.attack", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (row.TryGetValue(prefix + "stats.defense", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (!any) { obj.stats = null; } else { obj.stats = game.common.StatBlockCsv.FromRowWithPrefix(row, prefix + "stats."); } }
obj.status = DataSourceFactory.ConvertSingleValue<Player.Status__Enum>(row, prefix + "status");
            return obj;
        }
        internal static IEnumerable<Player> ReadCsv(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); yield return FromRowWithPrefixAndHeaderMap(map, row, string.Empty, gap); }
        }
        internal static Player FromRow(string[] header, string[] row, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) => FromRowWithPrefixAndHeader(header, row, string.Empty, gap);
        internal static Player FromRowWithPrefixAndHeader(string[] header, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            return FromRowWithPrefixAndHeaderMap(map, row, prefix, gap);
        }
        internal static Player FromRowWithPrefixAndHeaderMap(System.Collections.Generic.Dictionary<string,int> map, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new Player();
            int gap = (int)gap; // 0=Break,1=Sparse
            bool __pfxEmpty = string.IsNullOrEmpty(prefix);
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "id" : prefix + "id"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.id = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "name" : prefix + "name"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.name = DataSourceFactory.ConvertValue<string>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "level" : prefix + "level"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.level = DataSourceFactory.ConvertValue<ushort>(__cell); }
{ bool any=false; { int __idx; if (map.TryGetValue((__pfxEmpty ? "stats.health" : prefix + "stats.health"), out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } { int __idx; if (map.TryGetValue((__pfxEmpty ? "stats.mana" : prefix + "stats.mana"), out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } { int __idx; if (map.TryGetValue((__pfxEmpty ? "stats.attack" : prefix + "stats.attack"), out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } { int __idx; if (map.TryGetValue((__pfxEmpty ? "stats.defense" : prefix + "stats.defense"), out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } if (!any) { obj.stats = null; } else { obj.stats = game.common.StatBlockCsv.FromRowWithPrefixAndHeader(header, row, prefix + "stats.", gap); } }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "status" : prefix + "status"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.status = DataSourceFactory.ConvertValue<Player.Status__Enum>(__cell); }
            return obj;
        }
}
}

namespace game.character
{
internal static class MonsterCsv
{
        private static readonly string[] __Headers_Monster = new string[] { "id", "name", "stats.health", "stats.mana", "stats.attack", "stats.defense", "spawn_point.x", "spawn_point.y", "patrol_points[0].x", "patrol_points[0].y", "status", "drop_items[0].item_id", "drop_items[0].drop_chance", "drop_items[0].enchantment.enchant_id", "drop_items[0].enchantment.strength" };
        internal static int ColumnCount_Monster() => __Headers_Monster.Length;
        internal static string[] GetHeader_Monster() => (string[])__Headers_Monster.Clone();
        internal static void AppendRow(Monster obj, List<string> cols)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.id));
cols.Add(CsvUtils.ToStringInvariant(obj.name));
if (obj.stats == null) { for (int i=0;i< game.common.StatBlockCsv.ColumnCount_StatBlock(); i++) cols.Add(string.Empty); } else { game.common.StatBlockCsv.AppendRow(obj.stats, cols); }
if (obj.spawn_point == null) { for (int i=0;i< game.common.PositionCsv.ColumnCount_Position(); i++) cols.Add(string.Empty); } else { game.common.PositionCsv.AppendRow(obj.spawn_point, cols); }
if (obj.patrol_points != null && obj.patrol_points.Count > 0) {
if (obj.patrol_points[0] == null) { for (int i=0;i< game.common.PositionCsv.ColumnCount_Position(); i++) cols.Add(string.Empty); } else { game.common.PositionCsv.AppendRow(obj.patrol_points[0], cols); }
} else {
cols.Add(string.Empty);
cols.Add(string.Empty);
}
cols.Add(string.Empty);
if (obj.drop_items != null && obj.drop_items.Count > 0) {
if (obj.drop_items[0] == null) { for (int i=0;i< 4; i++) cols.Add(string.Empty); } else {
cols.Add(CsvUtils.ToStringInvariant(obj.drop_items[0].item_id));
cols.Add(CsvUtils.ToStringInvariant(obj.drop_items[0].drop_chance));
if (obj.drop_items[0].enchantment == null) { for (int i=0;i< 2; i++) cols.Add(string.Empty); } else {
cols.Add(CsvUtils.ToStringInvariant(obj.drop_items[0].enchantment.enchant_id));
cols.Add(CsvUtils.ToStringInvariant(obj.drop_items[0].enchantment.strength));
}
}
} else {
cols.Add(string.Empty);
cols.Add(string.Empty);
cols.Add(string.Empty);
cols.Add(string.Empty);
}
        }
        internal static string[] ToRow(Monster obj) { var list = new List<string>(ColumnCount_Monster()); AppendRow(obj, list); return list.ToArray(); }
        internal static void WriteCsv(IEnumerable<Monster> items, string path, bool writeHeader = true, char sep = ',') { using var sw = new StreamWriter(path, false, new UTF8Encoding(false)); if (writeHeader) sw.WriteLine(CsvUtils.Join(GetHeader_Monster(), sep)); foreach (var it in items) { var row = ToRow(it); sw.WriteLine(CsvUtils.Join(row, sep)); } }
        internal static System.Collections.Generic.Dictionary<string,int> ComputeListMaxes(System.Collections.Generic.IEnumerable<Monster> items)
        {
            var d = new System.Collections.Generic.Dictionary<string,int>(System.StringComparer.OrdinalIgnoreCase);
            foreach (var it in items) {
                var c_patrol_points = (it.patrol_points != null ? it.patrol_points.Count : 0); if (!d.TryGetValue("patrol_points", out var m_patrol_points) || c_patrol_points > m_patrol_points) d["patrol_points"] = c_patrol_points;
                var c_drop_items = (it.drop_items != null ? it.drop_items.Count : 0); if (!d.TryGetValue("drop_items", out var m_drop_items) || c_drop_items > m_drop_items) d["drop_items"] = c_drop_items;
            }
            return d;
        }
        internal static string[] GetDynamicHeader(System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
            var cols = new System.Collections.Generic.List<string>();
            cols.Add("id");
            cols.Add("name");
            cols.Add("stats.health");
            cols.Add("stats.mana");
            cols.Add("stats.attack");
            cols.Add("stats.defense");
            cols.Add("spawn_point.x");
            cols.Add("spawn_point.y");
            cols.Add("status");
            int __mx_patrol_points = 0; if (listMaxes != null) listMaxes.TryGetValue("patrol_points", out __mx_patrol_points);
            for (int __i=0; __i<__mx_patrol_points; __i++) {
                cols.Add(string.Format("patrol_points[{0}].x", __i));
                cols.Add(string.Format("patrol_points[{0}].y", __i));
            }
            int __mx_drop_items = 0; if (listMaxes != null) listMaxes.TryGetValue("drop_items", out __mx_drop_items);
            for (int __i=0; __i<__mx_drop_items; __i++) {
                cols.Add(string.Format("drop_items[{0}].item_id", __i));
                cols.Add(string.Format("drop_items[{0}].drop_chance", __i));
                cols.Add(string.Format("drop_items[{0}].enchantment.enchant_id", __i));
                cols.Add(string.Format("drop_items[{0}].enchantment.strength", __i));
            }
            return cols.ToArray();
        }
        internal static void AppendRowDynamic(Monster obj, System.Collections.Generic.List<string> cols, System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.id));
cols.Add(CsvUtils.ToStringInvariant(obj.name));
if (obj.stats == null) { for (int i=0;i< game.common.StatBlockCsv.ColumnCount_StatBlock(); i++) cols.Add(string.Empty); } else { game.common.StatBlockCsv.AppendRow(obj.stats, cols); }
if (obj.spawn_point == null) { for (int i=0;i< game.common.PositionCsv.ColumnCount_Position(); i++) cols.Add(string.Empty); } else { game.common.PositionCsv.AppendRow(obj.spawn_point, cols); }
            int __mx = 0; if (listMaxes != null) listMaxes.TryGetValue("patrol_points", out __mx);
            for (int __i=0; __i<__mx; __i++) {
                if (obj.patrol_points != null && obj.patrol_points.Count > __i) {
if (obj.patrol_points[__i] == null) { for (int i=0;i< game.common.PositionCsv.ColumnCount_Position(); i++) cols.Add(string.Empty); } else { game.common.PositionCsv.AppendRow(obj.patrol_points[__i], cols); }
                } else {
                    cols.Add(string.Empty);
                    cols.Add(string.Empty);
                }
            }
cols.Add(string.Empty);
            int __mx = 0; if (listMaxes != null) listMaxes.TryGetValue("drop_items", out __mx);
            for (int __i=0; __i<__mx; __i++) {
                if (obj.drop_items != null && obj.drop_items.Count > __i) {
if (obj.drop_items[__i] == null) { for (int i=0;i< 4; i++) cols.Add(string.Empty); } else {
cols.Add(CsvUtils.ToStringInvariant(obj.drop_items[__i].item_id));
cols.Add(CsvUtils.ToStringInvariant(obj.drop_items[__i].drop_chance));
if (obj.drop_items[__i].enchantment == null) { for (int i=0;i< 2; i++) cols.Add(string.Empty); } else {
cols.Add(CsvUtils.ToStringInvariant(obj.drop_items[__i].enchantment.enchant_id));
cols.Add(CsvUtils.ToStringInvariant(obj.drop_items[__i].enchantment.strength));
}
}
                } else {
                    cols.Add(string.Empty);
                    cols.Add(string.Empty);
                    cols.Add(string.Empty);
                    cols.Add(string.Empty);
                }
            }
        }
        internal static void WriteCsvDynamic(System.Collections.Generic.IEnumerable<Monster> items, string path, bool writeHeader = true, char sep = ',')
        {
            var maxes = ComputeListMaxes(items);
            using var sw = new System.IO.StreamWriter(path, false, new System.Text.UTF8Encoding(false));
            if (writeHeader) { var hdr = GetDynamicHeader(maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(hdr, sep)); }
            foreach (var it in items) { var row = new System.Collections.Generic.List<string>(256); AppendRowDynamic(it, row, maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep)); }
        }
        internal sealed class CsvIndex
        {
            public int idx_id = -1;
            public int idx_name = -1;
            public StatBlockCsv.CsvIndex idx_stats;
            public PositionCsv.CsvIndex idx_spawn_point;
            public System.Collections.Generic.List<PositionCsv.CsvIndex> idx_patrol_points = new System.Collections.Generic.List<PositionCsv.CsvIndex>();
            public int idx_status = -1;
            public System.Collections.Generic.List<game.character.DropItemsCsv.CsvIndex> idx_drop_items = new System.Collections.Generic.List<game.character.DropItemsCsv.CsvIndex>();
        }
        internal static bool CsvIndexHasAny(Index idx)
        {
            if (idx.idx_id >= 0) return true;
            if (idx.idx_name >= 0) return true;
            if (idx.idx_stats != null && StatBlockCsv.CsvIndexHasAny(idx.idx_stats)) return true;
            if (idx.idx_spawn_point != null && PositionCsv.CsvIndexHasAny(idx.idx_spawn_point)) return true;
            if (idx.idx_patrol_points != null && idx.idx_patrol_points.Count > 0) return true;
            if (idx.idx_status >= 0) return true;
            if (idx.idx_drop_items != null && idx.idx_drop_items.Count > 0) return true;
            return false;
        }
        internal static bool CsvIndexHasValues(Index idx, string[] row)
        {
            if (idx.idx_id >= 0 && idx.idx_id < row.Length && !string.IsNullOrEmpty(row[idx.idx_id])) return true;
            if (idx.idx_name >= 0 && idx.idx_name < row.Length && !string.IsNullOrEmpty(row[idx.idx_name])) return true;
            if (idx.idx_stats != null && StatBlockCsv.CsvIndexHasValues(idx.idx_stats, row)) return true;
            if (idx.idx_spawn_point != null && PositionCsv.CsvIndexHasValues(idx.idx_spawn_point, row)) return true;
            if (idx.idx_patrol_points != null) { for (int i=0;i<idx.idx_patrol_points.Count;i++) { var __ix = idx.idx_patrol_points[i]; if (__ix>=0 && __ix<row.Length && !string.IsNullOrEmpty(row[__ix])) return true; } }
            if (idx.idx_status >= 0 && idx.idx_status < row.Length && !string.IsNullOrEmpty(row[idx.idx_status])) return true;
            if (idx.idx_drop_items != null) { for (int i=0;i<idx.idx_drop_items.Count;i++) { var __ix = idx.idx_drop_items[i]; if (__ix>=0 && __ix<row.Length && !string.IsNullOrEmpty(row[__ix])) return true; } }
            return false;
        }
        internal static CsvIndex BuildCsvIndex(string[] header, string prefix)
        {
            var idx = new CsvIndex();
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            if (!map.TryGetValue(prefix + "id", out idx.idx_id)) idx.idx_id = -1;
            if (!map.TryGetValue(prefix + "name", out idx.idx_name)) idx.idx_name = -1;
            idx.idx_stats = StatBlockCsv.BuildIndex(header, prefix + "stats.");
            idx.idx_spawn_point = PositionCsv.BuildIndex(header, prefix + "spawn_point.");
            for (int i=0;;i++) { var sub = PositionCsv.BuildIndex(header, prefix + "patrol_points["+i+"]."); if (!PositionCsv.CsvIndexHasAny(sub)) break; idx.idx_patrol_points.Add(sub); }
            if (!map.TryGetValue(prefix + "status", out idx.idx_status)) idx.idx_status = -1;
            for (int i=0;;i++) { var sub = game.character.DropItemsCsv.BuildIndex(header, prefix + "drop_items["+i+"]."); if (!game.character.DropItemsCsv.CsvIndexHasAny(sub)) break; idx.idx_drop_items.Add(sub); }
            return idx;
        }
        internal static Monster FromRowWithCsvIndex(Index idx, string[] row, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new Monster();
            if (idx.idx_id >= 0 && idx.idx_id < row.Length) { var __cell = row[idx.idx_id]; obj.id = DataSourceFactory.ConvertValue<uint>( __cell ); }
            if (idx.idx_name >= 0 && idx.idx_name < row.Length) { var __cell = row[idx.idx_name]; obj.name = DataSourceFactory.ConvertValue<string>( __cell ); }
            if (!StatBlockCsv.CsvIndexHasValues(idx.idx_stats, row)) { obj.stats = null; } else { obj.stats = StatBlockCsv.FromRowWithIndex(idx.idx_stats, row, gap); }
            if (!PositionCsv.CsvIndexHasValues(idx.idx_spawn_point, row)) { obj.spawn_point = null; } else { obj.spawn_point = PositionCsv.FromRowWithIndex(idx.idx_spawn_point, row, gap); }
            { var list = new System.Collections.Generic.List<Position>(); for (int i=0;i<idx.idx_patrol_points.Count;i++) { var subIdx = idx.idx_patrol_points[i]; if (!PositionCsv.CsvIndexHasValues(subIdx, row)) { if (i==0 || gap==Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } var sub = PositionCsv.FromRowWithIndex(subIdx, row, gap); list.Add(sub); } obj.patrol_points = list; }
            if (idx.idx_status >= 0 && idx.idx_status < row.Length) { var __cell = row[idx.idx_status]; obj.status = DataSourceFactory.ConvertValue<Status>( __cell ); }
            { var list = new System.Collections.Generic.List<game.character.DropItems>(); for (int i=0;i<idx.idx_drop_items.Count;i++) { var subIdx = idx.idx_drop_items[i]; if (!game.character.DropItemsCsv.CsvIndexHasValues(subIdx, row)) { if (i==0 || gap==Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } var sub = game.character.DropItemsCsv.FromRowWithIndex(subIdx, row, gap); list.Add(sub); } obj.drop_items = list; }
            return obj;
        }
        internal sealed class Reader
        {
            public readonly CsvIndex Index;
            public readonly char Sep;
            public readonly Polygen.Common.CsvUtils.GapMode Gap;
            public Reader(Index index, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { CsvIndex = index; Sep = sep; Gap = gap; }
            public Monster Parse(string[] row) => FromRowWithCsvIndex(Index, row, Gap);
            internal static Reader FromHeader(string[] header, char sep = ',', string prefix = "", Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { var idx = BuildCsvIndex(header, prefix); return new Reader(idx, sep, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<Monster> ReadCsvWithIndex(string path, CsvIndex idx, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length <= 1) yield break;
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<Monster> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var idx = BuildCsvIndex(header, string.Empty);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static Monster FromRow(IDictionary<string,string> row) => FromRowWithPrefix(row, string.Empty);
        internal static Monster FromRowWithPrefix(IDictionary<string,string> row, string prefix)
        {
            var obj = new Monster();
obj.id = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "id");
obj.name = DataSourceFactory.ConvertSingleValue<string>(row, prefix + "name");
{ bool any=false; string tmp; if (row.TryGetValue(prefix + "stats.health", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (row.TryGetValue(prefix + "stats.mana", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (row.TryGetValue(prefix + "stats.attack", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (row.TryGetValue(prefix + "stats.defense", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (!any) { obj.stats = null; } else { obj.stats = game.common.StatBlockCsv.FromRowWithPrefix(row, prefix + "stats."); } }
{ bool any=false; string tmp; if (row.TryGetValue(prefix + "spawn_point.x", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (row.TryGetValue(prefix + "spawn_point.y", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (!any) { obj.spawn_point = null; } else { obj.spawn_point = game.common.PositionCsv.FromRowWithPrefix(row, prefix + "spawn_point."); } }
{ bool any=false; string tmp; if (row.TryGetValue(prefix + "patrol_points[0].x", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (row.TryGetValue(prefix + "patrol_points[0].y", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (!any) { obj.patrol_points = new List<game.common.Position>(); } else { var list = new List<game.common.Position>(); list.Add(game.common.PositionCsv.FromRowWithPrefix(row, prefix + "patrol_points[0].")); obj.patrol_points = list; } }
{ bool any=false; string tmp; if (row.TryGetValue(prefix + "drop_items[0].item_id", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (row.TryGetValue(prefix + "drop_items[0].drop_chance", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (row.TryGetValue(prefix + "drop_items[0].enchantment.enchant_id", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (row.TryGetValue(prefix + "drop_items[0].enchantment.strength", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (!any) { obj.drop_items = new List<Monster.DropItems>(); } else { var sub = new Monster.DropItems();
sub.item_id = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "drop_items[0]." + "item_id");
sub.drop_chance = DataSourceFactory.ConvertSingleValue<float>(row, prefix + "drop_items[0]." + "drop_chance");
{ bool any=false; string tmp; if (row.TryGetValue(prefix + "drop_items[0]." + "enchantment.enchant_id", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (row.TryGetValue(prefix + "drop_items[0]." + "enchantment.strength", out tmp) && !string.IsNullOrEmpty(tmp)) { any=true; } if (!any) { sub.enchantment = null; } else { var sub = new Monster.DropItems.Enchantment();
sub.enchant_id = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "drop_items[0]." + "enchantment." + "enchant_id");
sub.strength = DataSourceFactory.ConvertSingleValue<float>(row, prefix + "drop_items[0]." + "enchantment." + "strength");
sub.enchantment = sub; } }
var list = new List<Monster.DropItems>(); list.Add(sub); obj.drop_items = list; } }
            return obj;
        }
        internal static IEnumerable<Monster> ReadCsv(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); yield return FromRowWithPrefixAndHeaderMap(map, row, string.Empty, gap); }
        }
        internal static Monster FromRow(string[] header, string[] row, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) => FromRowWithPrefixAndHeader(header, row, string.Empty, gap);
        internal static Monster FromRowWithPrefixAndHeader(string[] header, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            return FromRowWithPrefixAndHeaderMap(map, row, prefix, gap);
        }
        internal static Monster FromRowWithPrefixAndHeaderMap(System.Collections.Generic.Dictionary<string,int> map, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new Monster();
            int gap = (int)gap; // 0=Break,1=Sparse
            bool __pfxEmpty = string.IsNullOrEmpty(prefix);
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "id" : prefix + "id"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.id = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "name" : prefix + "name"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.name = DataSourceFactory.ConvertValue<string>(__cell); }
{ bool any=false; { int __idx; if (map.TryGetValue((__pfxEmpty ? "stats.health" : prefix + "stats.health"), out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } { int __idx; if (map.TryGetValue((__pfxEmpty ? "stats.mana" : prefix + "stats.mana"), out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } { int __idx; if (map.TryGetValue((__pfxEmpty ? "stats.attack" : prefix + "stats.attack"), out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } { int __idx; if (map.TryGetValue((__pfxEmpty ? "stats.defense" : prefix + "stats.defense"), out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } if (!any) { obj.stats = null; } else { obj.stats = game.common.StatBlockCsv.FromRowWithPrefixAndHeader(header, row, prefix + "stats.", gap); } }
{ bool any=false; { int __idx; if (map.TryGetValue((__pfxEmpty ? "spawn_point.x" : prefix + "spawn_point.x"), out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } { int __idx; if (map.TryGetValue((__pfxEmpty ? "spawn_point.y" : prefix + "spawn_point.y"), out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } if (!any) { obj.spawn_point = null; } else { obj.spawn_point = game.common.PositionCsv.FromRowWithPrefixAndHeader(header, row, prefix + "spawn_point.", gap); } }
{ var list = new System.Collections.Generic.List<game.common.Position>(); int i=0; for(;;i++) { bool any=false; string __tmp; { int __idx; if (map.TryGetValue(prefix + "patrol_points["+i+"].x", out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } { int __idx; if (map.TryGetValue(prefix + "patrol_points["+i+"].y", out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } if (!any) { if (i==0 || gap==0) break; else continue; } list.Add(game.common.PositionCsv.FromRowWithPrefixAndHeader(header, row, prefix + "patrol_points["+i+"].", gap)); } obj.patrol_points = list; }
{ var list = new List<Monster.DropItems>(); int i=0; for(;;i++) { bool any=false; string __tmp; { int __idx; if (map.TryGetValue(prefix + "drop_items["+i+"].item_id", out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } { int __idx; if (map.TryGetValue(prefix + "drop_items["+i+"].drop_chance", out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } { int __idx; if (map.TryGetValue(prefix + "drop_items["+i+"].enchantment.enchant_id", out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } { int __idx; if (map.TryGetValue(prefix + "drop_items["+i+"].enchantment.strength", out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } if (!any) { if (i==0 || gap==0) break; else continue; } var sub = new Monster.DropItems();
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "item_id" : prefix + "drop_items["+i+"]." + "item_id"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; sub.item_id = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "drop_chance" : prefix + "drop_items["+i+"]." + "drop_chance"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; sub.drop_chance = DataSourceFactory.ConvertValue<float>(__cell); }
{ bool any=false; { int __idx; if (map.TryGetValue((__pfxEmpty ? "enchantment.enchant_id" : prefix + "drop_items["+i+"]." + "enchantment.enchant_id"), out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } { int __idx; if (map.TryGetValue((__pfxEmpty ? "enchantment.strength" : prefix + "drop_items["+i+"]." + "enchantment.strength"), out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; } if (!any) { sub.enchantment = null; } else { var sub = new Monster.DropItems.Enchantment(); { int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "enchant_id" : prefix + "drop_items["+i+"]." + "enchantment." + "enchant_id"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; sub.enchant_id = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "strength" : prefix + "drop_items["+i+"]." + "enchantment." + "strength"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; sub.strength = DataSourceFactory.ConvertValue<float>(__cell); }
sub.enchantment = sub; } }
list.Add(sub); } obj.drop_items = list; }
            return obj;
        }
}
}

namespace game.character.skill
{
internal static class SkillCsv
{
        private static readonly string[] __Headers_Skill = new string[] { "id", "name", "description", "element", "power" };
        internal static int ColumnCount_Skill() => __Headers_Skill.Length;
        internal static string[] GetHeader_Skill() => (string[])__Headers_Skill.Clone();
        internal static void AppendRow(Skill obj, List<string> cols)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.id));
cols.Add(CsvUtils.ToStringInvariant(obj.name));
cols.Add(CsvUtils.ToStringInvariant(obj.description));
cols.Add((obj.element).ToString());
cols.Add(CsvUtils.ToStringInvariant(obj.power));
        }
        internal static string[] ToRow(Skill obj) { var list = new List<string>(ColumnCount_Skill()); AppendRow(obj, list); return list.ToArray(); }
        internal static void WriteCsv(IEnumerable<Skill> items, string path, bool writeHeader = true, char sep = ',') { using var sw = new StreamWriter(path, false, new UTF8Encoding(false)); if (writeHeader) sw.WriteLine(CsvUtils.Join(GetHeader_Skill(), sep)); foreach (var it in items) { var row = ToRow(it); sw.WriteLine(CsvUtils.Join(row, sep)); } }
        internal static System.Collections.Generic.Dictionary<string,int> ComputeListMaxes(System.Collections.Generic.IEnumerable<Skill> items)
        {
            var d = new System.Collections.Generic.Dictionary<string,int>(System.StringComparer.OrdinalIgnoreCase);
            foreach (var it in items) {
            }
            return d;
        }
        internal static string[] GetDynamicHeader(System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
            var cols = new System.Collections.Generic.List<string>();
            cols.Add("id");
            cols.Add("name");
            cols.Add("description");
            cols.Add("element");
            cols.Add("power");
            return cols.ToArray();
        }
        internal static void AppendRowDynamic(Skill obj, System.Collections.Generic.List<string> cols, System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.id));
cols.Add(CsvUtils.ToStringInvariant(obj.name));
cols.Add(CsvUtils.ToStringInvariant(obj.description));
cols.Add((obj.element).ToString());
cols.Add(CsvUtils.ToStringInvariant(obj.power));
        }
        internal static void WriteCsvDynamic(System.Collections.Generic.IEnumerable<Skill> items, string path, bool writeHeader = true, char sep = ',')
        {
            var maxes = ComputeListMaxes(items);
            using var sw = new System.IO.StreamWriter(path, false, new System.Text.UTF8Encoding(false));
            if (writeHeader) { var hdr = GetDynamicHeader(maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(hdr, sep)); }
            foreach (var it in items) { var row = new System.Collections.Generic.List<string>(256); AppendRowDynamic(it, row, maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep)); }
        }
        internal sealed class CsvIndex
        {
            public int idx_id = -1;
            public int idx_name = -1;
            public int idx_description = -1;
            public int idx_element = -1;
            public int idx_power = -1;
        }
        internal static bool CsvIndexHasAny(Index idx)
        {
            if (idx.idx_id >= 0) return true;
            if (idx.idx_name >= 0) return true;
            if (idx.idx_description >= 0) return true;
            if (idx.idx_element >= 0) return true;
            if (idx.idx_power >= 0) return true;
            return false;
        }
        internal static bool CsvIndexHasValues(Index idx, string[] row)
        {
            if (idx.idx_id >= 0 && idx.idx_id < row.Length && !string.IsNullOrEmpty(row[idx.idx_id])) return true;
            if (idx.idx_name >= 0 && idx.idx_name < row.Length && !string.IsNullOrEmpty(row[idx.idx_name])) return true;
            if (idx.idx_description >= 0 && idx.idx_description < row.Length && !string.IsNullOrEmpty(row[idx.idx_description])) return true;
            if (idx.idx_element >= 0 && idx.idx_element < row.Length && !string.IsNullOrEmpty(row[idx.idx_element])) return true;
            if (idx.idx_power >= 0 && idx.idx_power < row.Length && !string.IsNullOrEmpty(row[idx.idx_power])) return true;
            return false;
        }
        internal static CsvIndex BuildCsvIndex(string[] header, string prefix)
        {
            var idx = new CsvIndex();
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            if (!map.TryGetValue(prefix + "id", out idx.idx_id)) idx.idx_id = -1;
            if (!map.TryGetValue(prefix + "name", out idx.idx_name)) idx.idx_name = -1;
            if (!map.TryGetValue(prefix + "description", out idx.idx_description)) idx.idx_description = -1;
            if (!map.TryGetValue(prefix + "element", out idx.idx_element)) idx.idx_element = -1;
            if (!map.TryGetValue(prefix + "power", out idx.idx_power)) idx.idx_power = -1;
            return idx;
        }
        internal static Skill FromRowWithCsvIndex(Index idx, string[] row, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new Skill();
            if (idx.idx_id >= 0 && idx.idx_id < row.Length) { var __cell = row[idx.idx_id]; obj.id = DataSourceFactory.ConvertValue<uint>( __cell ); }
            if (idx.idx_name >= 0 && idx.idx_name < row.Length) { var __cell = row[idx.idx_name]; obj.name = DataSourceFactory.ConvertValue<string>( __cell ); }
            if (idx.idx_description >= 0 && idx.idx_description < row.Length) { var __cell = row[idx.idx_description]; obj.description = DataSourceFactory.ConvertValue<string>( __cell ); }
            if (idx.idx_element >= 0 && idx.idx_element < row.Length) { var __cell = row[idx.idx_element]; obj.element = DataSourceFactory.ConvertValue<game.common.Element>( __cell ); }
            if (idx.idx_power >= 0 && idx.idx_power < row.Length) { var __cell = row[idx.idx_power]; obj.power = DataSourceFactory.ConvertValue<uint>( __cell ); }
            return obj;
        }
        internal sealed class Reader
        {
            public readonly CsvIndex Index;
            public readonly char Sep;
            public readonly Polygen.Common.CsvUtils.GapMode Gap;
            public Reader(Index index, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { CsvIndex = index; Sep = sep; Gap = gap; }
            public Skill Parse(string[] row) => FromRowWithCsvIndex(Index, row, Gap);
            internal static Reader FromHeader(string[] header, char sep = ',', string prefix = "", Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { var idx = BuildCsvIndex(header, prefix); return new Reader(idx, sep, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<Skill> ReadCsvWithIndex(string path, CsvIndex idx, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length <= 1) yield break;
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<Skill> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var idx = BuildCsvIndex(header, string.Empty);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static Skill FromRow(IDictionary<string,string> row) => FromRowWithPrefix(row, string.Empty);
        internal static Skill FromRowWithPrefix(IDictionary<string,string> row, string prefix)
        {
            var obj = new Skill();
obj.id = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "id");
obj.name = DataSourceFactory.ConvertSingleValue<string>(row, prefix + "name");
obj.description = DataSourceFactory.ConvertSingleValue<string>(row, prefix + "description");
obj.element = DataSourceFactory.ConvertSingleValue<game.common.Element>(row, prefix + "element");
obj.power = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "power");
            return obj;
        }
        internal static IEnumerable<Skill> ReadCsv(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); yield return FromRowWithPrefixAndHeaderMap(map, row, string.Empty, gap); }
        }
        internal static Skill FromRow(string[] header, string[] row, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) => FromRowWithPrefixAndHeader(header, row, string.Empty, gap);
        internal static Skill FromRowWithPrefixAndHeader(string[] header, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            return FromRowWithPrefixAndHeaderMap(map, row, prefix, gap);
        }
        internal static Skill FromRowWithPrefixAndHeaderMap(System.Collections.Generic.Dictionary<string,int> map, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new Skill();
            int gap = (int)gap; // 0=Break,1=Sparse
            bool __pfxEmpty = string.IsNullOrEmpty(prefix);
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "id" : prefix + "id"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.id = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "name" : prefix + "name"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.name = DataSourceFactory.ConvertValue<string>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "description" : prefix + "description"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.description = DataSourceFactory.ConvertValue<string>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "element" : prefix + "element"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.element = DataSourceFactory.ConvertValue<game.common.Element>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "power" : prefix + "power"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.power = DataSourceFactory.ConvertValue<uint>(__cell); }
            return obj;
        }
}
}

namespace game.junction
{
internal static class PlayerSkillCsv
{
        private static readonly string[] __Headers_PlayerSkill = new string[] { "player_id", "skill_id", "skill_level" };
        internal static int ColumnCount_PlayerSkill() => __Headers_PlayerSkill.Length;
        internal static string[] GetHeader_PlayerSkill() => (string[])__Headers_PlayerSkill.Clone();
        internal static void AppendRow(PlayerSkill obj, List<string> cols)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.player_id));
cols.Add(CsvUtils.ToStringInvariant(obj.skill_id));
cols.Add(CsvUtils.ToStringInvariant(obj.skill_level));
        }
        internal static string[] ToRow(PlayerSkill obj) { var list = new List<string>(ColumnCount_PlayerSkill()); AppendRow(obj, list); return list.ToArray(); }
        internal static void WriteCsv(IEnumerable<PlayerSkill> items, string path, bool writeHeader = true, char sep = ',') { using var sw = new StreamWriter(path, false, new UTF8Encoding(false)); if (writeHeader) sw.WriteLine(CsvUtils.Join(GetHeader_PlayerSkill(), sep)); foreach (var it in items) { var row = ToRow(it); sw.WriteLine(CsvUtils.Join(row, sep)); } }
        internal static System.Collections.Generic.Dictionary<string,int> ComputeListMaxes(System.Collections.Generic.IEnumerable<PlayerSkill> items)
        {
            var d = new System.Collections.Generic.Dictionary<string,int>(System.StringComparer.OrdinalIgnoreCase);
            foreach (var it in items) {
            }
            return d;
        }
        internal static string[] GetDynamicHeader(System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
            var cols = new System.Collections.Generic.List<string>();
            cols.Add("player_id");
            cols.Add("skill_id");
            cols.Add("skill_level");
            return cols.ToArray();
        }
        internal static void AppendRowDynamic(PlayerSkill obj, System.Collections.Generic.List<string> cols, System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.player_id));
cols.Add(CsvUtils.ToStringInvariant(obj.skill_id));
cols.Add(CsvUtils.ToStringInvariant(obj.skill_level));
        }
        internal static void WriteCsvDynamic(System.Collections.Generic.IEnumerable<PlayerSkill> items, string path, bool writeHeader = true, char sep = ',')
        {
            var maxes = ComputeListMaxes(items);
            using var sw = new System.IO.StreamWriter(path, false, new System.Text.UTF8Encoding(false));
            if (writeHeader) { var hdr = GetDynamicHeader(maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(hdr, sep)); }
            foreach (var it in items) { var row = new System.Collections.Generic.List<string>(256); AppendRowDynamic(it, row, maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep)); }
        }
        internal sealed class CsvIndex
        {
            public int idx_player_id = -1;
            public int idx_skill_id = -1;
            public int idx_skill_level = -1;
        }
        internal static bool CsvIndexHasAny(Index idx)
        {
            if (idx.idx_player_id >= 0) return true;
            if (idx.idx_skill_id >= 0) return true;
            if (idx.idx_skill_level >= 0) return true;
            return false;
        }
        internal static bool CsvIndexHasValues(Index idx, string[] row)
        {
            if (idx.idx_player_id >= 0 && idx.idx_player_id < row.Length && !string.IsNullOrEmpty(row[idx.idx_player_id])) return true;
            if (idx.idx_skill_id >= 0 && idx.idx_skill_id < row.Length && !string.IsNullOrEmpty(row[idx.idx_skill_id])) return true;
            if (idx.idx_skill_level >= 0 && idx.idx_skill_level < row.Length && !string.IsNullOrEmpty(row[idx.idx_skill_level])) return true;
            return false;
        }
        internal static CsvIndex BuildCsvIndex(string[] header, string prefix)
        {
            var idx = new CsvIndex();
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            if (!map.TryGetValue(prefix + "player_id", out idx.idx_player_id)) idx.idx_player_id = -1;
            if (!map.TryGetValue(prefix + "skill_id", out idx.idx_skill_id)) idx.idx_skill_id = -1;
            if (!map.TryGetValue(prefix + "skill_level", out idx.idx_skill_level)) idx.idx_skill_level = -1;
            return idx;
        }
        internal static PlayerSkill FromRowWithCsvIndex(Index idx, string[] row, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new PlayerSkill();
            if (idx.idx_player_id >= 0 && idx.idx_player_id < row.Length) { var __cell = row[idx.idx_player_id]; obj.player_id = DataSourceFactory.ConvertValue<uint>( __cell ); }
            if (idx.idx_skill_id >= 0 && idx.idx_skill_id < row.Length) { var __cell = row[idx.idx_skill_id]; obj.skill_id = DataSourceFactory.ConvertValue<uint>( __cell ); }
            if (idx.idx_skill_level >= 0 && idx.idx_skill_level < row.Length) { var __cell = row[idx.idx_skill_level]; obj.skill_level = DataSourceFactory.ConvertValue<ushort>( __cell ); }
            return obj;
        }
        internal sealed class Reader
        {
            public readonly CsvIndex Index;
            public readonly char Sep;
            public readonly Polygen.Common.CsvUtils.GapMode Gap;
            public Reader(Index index, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { CsvIndex = index; Sep = sep; Gap = gap; }
            public PlayerSkill Parse(string[] row) => FromRowWithCsvIndex(Index, row, Gap);
            internal static Reader FromHeader(string[] header, char sep = ',', string prefix = "", Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { var idx = BuildCsvIndex(header, prefix); return new Reader(idx, sep, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<PlayerSkill> ReadCsvWithIndex(string path, CsvIndex idx, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length <= 1) yield break;
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<PlayerSkill> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var idx = BuildCsvIndex(header, string.Empty);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static PlayerSkill FromRow(IDictionary<string,string> row) => FromRowWithPrefix(row, string.Empty);
        internal static PlayerSkill FromRowWithPrefix(IDictionary<string,string> row, string prefix)
        {
            var obj = new PlayerSkill();
obj.player_id = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "player_id");
obj.skill_id = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "skill_id");
obj.skill_level = DataSourceFactory.ConvertSingleValue<ushort>(row, prefix + "skill_level");
            return obj;
        }
        internal static IEnumerable<PlayerSkill> ReadCsv(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); yield return FromRowWithPrefixAndHeaderMap(map, row, string.Empty, gap); }
        }
        internal static PlayerSkill FromRow(string[] header, string[] row, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) => FromRowWithPrefixAndHeader(header, row, string.Empty, gap);
        internal static PlayerSkill FromRowWithPrefixAndHeader(string[] header, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            return FromRowWithPrefixAndHeaderMap(map, row, prefix, gap);
        }
        internal static PlayerSkill FromRowWithPrefixAndHeaderMap(System.Collections.Generic.Dictionary<string,int> map, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new PlayerSkill();
            int gap = (int)gap; // 0=Break,1=Sparse
            bool __pfxEmpty = string.IsNullOrEmpty(prefix);
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "player_id" : prefix + "player_id"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.player_id = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "skill_id" : prefix + "skill_id"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.skill_id = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "skill_level" : prefix + "skill_level"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.skill_level = DataSourceFactory.ConvertValue<ushort>(__cell); }
            return obj;
        }
}
}

namespace game.junction
{
internal static class InventoryItemCsv
{
        private static readonly string[] __Headers_InventoryItem = new string[] { "id", "player_id", "item_id", "quantity" };
        internal static int ColumnCount_InventoryItem() => __Headers_InventoryItem.Length;
        internal static string[] GetHeader_InventoryItem() => (string[])__Headers_InventoryItem.Clone();
        internal static void AppendRow(InventoryItem obj, List<string> cols)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.id));
cols.Add(CsvUtils.ToStringInvariant(obj.player_id));
cols.Add(CsvUtils.ToStringInvariant(obj.item_id));
cols.Add(CsvUtils.ToStringInvariant(obj.quantity));
        }
        internal static string[] ToRow(InventoryItem obj) { var list = new List<string>(ColumnCount_InventoryItem()); AppendRow(obj, list); return list.ToArray(); }
        internal static void WriteCsv(IEnumerable<InventoryItem> items, string path, bool writeHeader = true, char sep = ',') { using var sw = new StreamWriter(path, false, new UTF8Encoding(false)); if (writeHeader) sw.WriteLine(CsvUtils.Join(GetHeader_InventoryItem(), sep)); foreach (var it in items) { var row = ToRow(it); sw.WriteLine(CsvUtils.Join(row, sep)); } }
        internal static System.Collections.Generic.Dictionary<string,int> ComputeListMaxes(System.Collections.Generic.IEnumerable<InventoryItem> items)
        {
            var d = new System.Collections.Generic.Dictionary<string,int>(System.StringComparer.OrdinalIgnoreCase);
            foreach (var it in items) {
            }
            return d;
        }
        internal static string[] GetDynamicHeader(System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
            var cols = new System.Collections.Generic.List<string>();
            cols.Add("id");
            cols.Add("player_id");
            cols.Add("item_id");
            cols.Add("quantity");
            return cols.ToArray();
        }
        internal static void AppendRowDynamic(InventoryItem obj, System.Collections.Generic.List<string> cols, System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.id));
cols.Add(CsvUtils.ToStringInvariant(obj.player_id));
cols.Add(CsvUtils.ToStringInvariant(obj.item_id));
cols.Add(CsvUtils.ToStringInvariant(obj.quantity));
        }
        internal static void WriteCsvDynamic(System.Collections.Generic.IEnumerable<InventoryItem> items, string path, bool writeHeader = true, char sep = ',')
        {
            var maxes = ComputeListMaxes(items);
            using var sw = new System.IO.StreamWriter(path, false, new System.Text.UTF8Encoding(false));
            if (writeHeader) { var hdr = GetDynamicHeader(maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(hdr, sep)); }
            foreach (var it in items) { var row = new System.Collections.Generic.List<string>(256); AppendRowDynamic(it, row, maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep)); }
        }
        internal sealed class CsvIndex
        {
            public int idx_id = -1;
            public int idx_player_id = -1;
            public int idx_item_id = -1;
            public int idx_quantity = -1;
        }
        internal static bool CsvIndexHasAny(Index idx)
        {
            if (idx.idx_id >= 0) return true;
            if (idx.idx_player_id >= 0) return true;
            if (idx.idx_item_id >= 0) return true;
            if (idx.idx_quantity >= 0) return true;
            return false;
        }
        internal static bool CsvIndexHasValues(Index idx, string[] row)
        {
            if (idx.idx_id >= 0 && idx.idx_id < row.Length && !string.IsNullOrEmpty(row[idx.idx_id])) return true;
            if (idx.idx_player_id >= 0 && idx.idx_player_id < row.Length && !string.IsNullOrEmpty(row[idx.idx_player_id])) return true;
            if (idx.idx_item_id >= 0 && idx.idx_item_id < row.Length && !string.IsNullOrEmpty(row[idx.idx_item_id])) return true;
            if (idx.idx_quantity >= 0 && idx.idx_quantity < row.Length && !string.IsNullOrEmpty(row[idx.idx_quantity])) return true;
            return false;
        }
        internal static CsvIndex BuildCsvIndex(string[] header, string prefix)
        {
            var idx = new CsvIndex();
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            if (!map.TryGetValue(prefix + "id", out idx.idx_id)) idx.idx_id = -1;
            if (!map.TryGetValue(prefix + "player_id", out idx.idx_player_id)) idx.idx_player_id = -1;
            if (!map.TryGetValue(prefix + "item_id", out idx.idx_item_id)) idx.idx_item_id = -1;
            if (!map.TryGetValue(prefix + "quantity", out idx.idx_quantity)) idx.idx_quantity = -1;
            return idx;
        }
        internal static InventoryItem FromRowWithCsvIndex(Index idx, string[] row, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new InventoryItem();
            if (idx.idx_id >= 0 && idx.idx_id < row.Length) { var __cell = row[idx.idx_id]; obj.id = DataSourceFactory.ConvertValue<uint>( __cell ); }
            if (idx.idx_player_id >= 0 && idx.idx_player_id < row.Length) { var __cell = row[idx.idx_player_id]; obj.player_id = DataSourceFactory.ConvertValue<uint>( __cell ); }
            if (idx.idx_item_id >= 0 && idx.idx_item_id < row.Length) { var __cell = row[idx.idx_item_id]; obj.item_id = DataSourceFactory.ConvertValue<uint>( __cell ); }
            if (idx.idx_quantity >= 0 && idx.idx_quantity < row.Length) { var __cell = row[idx.idx_quantity]; obj.quantity = DataSourceFactory.ConvertValue<uint>( __cell ); }
            return obj;
        }
        internal sealed class Reader
        {
            public readonly CsvIndex Index;
            public readonly char Sep;
            public readonly Polygen.Common.CsvUtils.GapMode Gap;
            public Reader(Index index, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { CsvIndex = index; Sep = sep; Gap = gap; }
            public InventoryItem Parse(string[] row) => FromRowWithCsvIndex(Index, row, Gap);
            internal static Reader FromHeader(string[] header, char sep = ',', string prefix = "", Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { var idx = BuildCsvIndex(header, prefix); return new Reader(idx, sep, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<InventoryItem> ReadCsvWithIndex(string path, CsvIndex idx, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length <= 1) yield break;
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<InventoryItem> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var idx = BuildCsvIndex(header, string.Empty);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static InventoryItem FromRow(IDictionary<string,string> row) => FromRowWithPrefix(row, string.Empty);
        internal static InventoryItem FromRowWithPrefix(IDictionary<string,string> row, string prefix)
        {
            var obj = new InventoryItem();
obj.id = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "id");
obj.player_id = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "player_id");
obj.item_id = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "item_id");
obj.quantity = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "quantity");
            return obj;
        }
        internal static IEnumerable<InventoryItem> ReadCsv(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); yield return FromRowWithPrefixAndHeaderMap(map, row, string.Empty, gap); }
        }
        internal static InventoryItem FromRow(string[] header, string[] row, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) => FromRowWithPrefixAndHeader(header, row, string.Empty, gap);
        internal static InventoryItem FromRowWithPrefixAndHeader(string[] header, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            return FromRowWithPrefixAndHeaderMap(map, row, prefix, gap);
        }
        internal static InventoryItem FromRowWithPrefixAndHeaderMap(System.Collections.Generic.Dictionary<string,int> map, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new InventoryItem();
            int gap = (int)gap; // 0=Break,1=Sparse
            bool __pfxEmpty = string.IsNullOrEmpty(prefix);
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "id" : prefix + "id"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.id = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "player_id" : prefix + "player_id"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.player_id = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "item_id" : prefix + "item_id"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.item_id = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "quantity" : prefix + "quantity"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.quantity = DataSourceFactory.ConvertValue<uint>(__cell); }
            return obj;
        }
}
}


