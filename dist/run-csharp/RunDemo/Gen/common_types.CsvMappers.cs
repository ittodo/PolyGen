using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;
using System.Globalization;
using System.IO;
using System.Text;
using Polygen.Common;
namespace game.common
{
internal static class PositionCsv
{
        private static readonly string[] __Headers_Position = new string[] { "x", "y" };
        internal static int ColumnCount_Position() => __Headers_Position.Length;
        internal static string[] GetHeader_Position() => (string[])__Headers_Position.Clone();
        internal static void AppendRow(Position obj, List<string> cols)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.x));
cols.Add(CsvUtils.ToStringInvariant(obj.y));
        }
        internal static string[] ToRow(Position obj) { var list = new List<string>(ColumnCount_Position()); AppendRow(obj, list); return list.ToArray(); }
        internal static void WriteCsv(IEnumerable<Position> items, string path, bool writeHeader = true, char sep = ',') { using var sw = new StreamWriter(path, false, new UTF8Encoding(false)); if (writeHeader) sw.WriteLine(CsvUtils.Join(GetHeader_Position(), sep)); foreach (var it in items) { var row = ToRow(it); sw.WriteLine(CsvUtils.Join(row, sep)); } }
        internal static System.Collections.Generic.Dictionary<string,int> ComputeListMaxes(System.Collections.Generic.IEnumerable<Position> items)
        {
            var d = new System.Collections.Generic.Dictionary<string,int>(System.StringComparer.OrdinalIgnoreCase);
            foreach (var it in items) {
            }
            return d;
        }
        internal static string[] GetDynamicHeader(System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
            var cols = new System.Collections.Generic.List<string>();
            cols.Add("x");
            cols.Add("y");
            return cols.ToArray();
        }
        internal static void AppendRowDynamic(Position obj, System.Collections.Generic.List<string> cols, System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.x));
cols.Add(CsvUtils.ToStringInvariant(obj.y));
        }
        internal static void WriteCsvDynamic(System.Collections.Generic.IEnumerable<Position> items, string path, bool writeHeader = true, char sep = ',')
        {
            var maxes = ComputeListMaxes(items);
            using var sw = new System.IO.StreamWriter(path, false, new System.Text.UTF8Encoding(false));
            if (writeHeader) { var hdr = GetDynamicHeader(maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(hdr, sep)); }
            foreach (var it in items) { var row = new System.Collections.Generic.List<string>(256); AppendRowDynamic(it, row, maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep)); }
        }
        internal sealed class CsvIndex
        {
            public int idx_x = -1;
            public int idx_y = -1;
        }
        internal static bool CsvIndexHasAny(Index idx)
        {
            if (idx.idx_x >= 0) return true;
            if (idx.idx_y >= 0) return true;
            return false;
        }
        internal static bool CsvIndexHasValues(Index idx, string[] row)
        {
            if (idx.idx_x >= 0 && idx.idx_x < row.Length && !string.IsNullOrEmpty(row[idx.idx_x])) return true;
            if (idx.idx_y >= 0 && idx.idx_y < row.Length && !string.IsNullOrEmpty(row[idx.idx_y])) return true;
            return false;
        }
        internal static CsvIndex BuildCsvIndex(string[] header, string prefix)
        {
            var idx = new CsvIndex();
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            if (!map.TryGetValue(prefix + "x", out idx.idx_x)) idx.idx_x = -1;
            if (!map.TryGetValue(prefix + "y", out idx.idx_y)) idx.idx_y = -1;
            return idx;
        }
        internal static Position FromRowWithCsvIndex(Index idx, string[] row, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new Position();
            if (idx.idx_x >= 0 && idx.idx_x < row.Length) { var __cell = row[idx.idx_x]; obj.x = DataSourceFactory.ConvertValue<float>( __cell ); }
            if (idx.idx_y >= 0 && idx.idx_y < row.Length) { var __cell = row[idx.idx_y]; obj.y = DataSourceFactory.ConvertValue<float>( __cell ); }
            return obj;
        }
        internal sealed class Reader
        {
            public readonly CsvIndex Index;
            public readonly char Sep;
            public readonly Polygen.Common.CsvUtils.GapMode Gap;
            public Reader(Index index, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { CsvIndex = index; Sep = sep; Gap = gap; }
            public Position Parse(string[] row) => FromRowWithCsvIndex(Index, row, Gap);
            internal static Reader FromHeader(string[] header, char sep = ',', string prefix = "", Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { var idx = BuildCsvIndex(header, prefix); return new Reader(idx, sep, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<Position> ReadCsvWithIndex(string path, CsvIndex idx, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length <= 1) yield break;
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<Position> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var idx = BuildCsvIndex(header, string.Empty);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static Position FromRow(IDictionary<string,string> row) => FromRowWithPrefix(row, string.Empty);
        internal static Position FromRowWithPrefix(IDictionary<string,string> row, string prefix)
        {
            var obj = new Position();
obj.x = DataSourceFactory.ConvertSingleValue<float>(row, prefix + "x");
obj.y = DataSourceFactory.ConvertSingleValue<float>(row, prefix + "y");
            return obj;
        }
        internal static IEnumerable<Position> ReadCsv(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); yield return FromRowWithPrefixAndHeaderMap(map, row, string.Empty, gap); }
        }
        internal static Position FromRow(string[] header, string[] row, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) => FromRowWithPrefixAndHeader(header, row, string.Empty, gap);
        internal static Position FromRowWithPrefixAndHeader(string[] header, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            return FromRowWithPrefixAndHeaderMap(map, row, prefix, gap);
        }
        internal static Position FromRowWithPrefixAndHeaderMap(System.Collections.Generic.Dictionary<string,int> map, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new Position();
            int gap = (int)gap; // 0=Break,1=Sparse
            bool __pfxEmpty = string.IsNullOrEmpty(prefix);
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "x" : prefix + "x"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.x = DataSourceFactory.ConvertValue<float>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "y" : prefix + "y"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.y = DataSourceFactory.ConvertValue<float>(__cell); }
            return obj;
        }
}
}

namespace game.common
{
internal static class StatBlockCsv
{
        private static readonly string[] __Headers_StatBlock = new string[] { "health", "mana", "attack", "defense" };
        internal static int ColumnCount_StatBlock() => __Headers_StatBlock.Length;
        internal static string[] GetHeader_StatBlock() => (string[])__Headers_StatBlock.Clone();
        internal static void AppendRow(StatBlock obj, List<string> cols)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.health));
cols.Add(CsvUtils.ToStringInvariant(obj.mana));
cols.Add(CsvUtils.ToStringInvariant(obj.attack));
cols.Add(CsvUtils.ToStringInvariant(obj.defense));
        }
        internal static string[] ToRow(StatBlock obj) { var list = new List<string>(ColumnCount_StatBlock()); AppendRow(obj, list); return list.ToArray(); }
        internal static void WriteCsv(IEnumerable<StatBlock> items, string path, bool writeHeader = true, char sep = ',') { using var sw = new StreamWriter(path, false, new UTF8Encoding(false)); if (writeHeader) sw.WriteLine(CsvUtils.Join(GetHeader_StatBlock(), sep)); foreach (var it in items) { var row = ToRow(it); sw.WriteLine(CsvUtils.Join(row, sep)); } }
        internal static System.Collections.Generic.Dictionary<string,int> ComputeListMaxes(System.Collections.Generic.IEnumerable<StatBlock> items)
        {
            var d = new System.Collections.Generic.Dictionary<string,int>(System.StringComparer.OrdinalIgnoreCase);
            foreach (var it in items) {
            }
            return d;
        }
        internal static string[] GetDynamicHeader(System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
            var cols = new System.Collections.Generic.List<string>();
            cols.Add("health");
            cols.Add("mana");
            cols.Add("attack");
            cols.Add("defense");
            return cols.ToArray();
        }
        internal static void AppendRowDynamic(StatBlock obj, System.Collections.Generic.List<string> cols, System.Collections.Generic.Dictionary<string,int> listMaxes)
        {
cols.Add(CsvUtils.ToStringInvariant(obj.health));
cols.Add(CsvUtils.ToStringInvariant(obj.mana));
cols.Add(CsvUtils.ToStringInvariant(obj.attack));
cols.Add(CsvUtils.ToStringInvariant(obj.defense));
        }
        internal static void WriteCsvDynamic(System.Collections.Generic.IEnumerable<StatBlock> items, string path, bool writeHeader = true, char sep = ',')
        {
            var maxes = ComputeListMaxes(items);
            using var sw = new System.IO.StreamWriter(path, false, new System.Text.UTF8Encoding(false));
            if (writeHeader) { var hdr = GetDynamicHeader(maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(hdr, sep)); }
            foreach (var it in items) { var row = new System.Collections.Generic.List<string>(256); AppendRowDynamic(it, row, maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep)); }
        }
        internal sealed class CsvIndex
        {
            public int idx_health = -1;
            public int idx_mana = -1;
            public int idx_attack = -1;
            public int idx_defense = -1;
        }
        internal static bool CsvIndexHasAny(Index idx)
        {
            if (idx.idx_health >= 0) return true;
            if (idx.idx_mana >= 0) return true;
            if (idx.idx_attack >= 0) return true;
            if (idx.idx_defense >= 0) return true;
            return false;
        }
        internal static bool CsvIndexHasValues(Index idx, string[] row)
        {
            if (idx.idx_health >= 0 && idx.idx_health < row.Length && !string.IsNullOrEmpty(row[idx.idx_health])) return true;
            if (idx.idx_mana >= 0 && idx.idx_mana < row.Length && !string.IsNullOrEmpty(row[idx.idx_mana])) return true;
            if (idx.idx_attack >= 0 && idx.idx_attack < row.Length && !string.IsNullOrEmpty(row[idx.idx_attack])) return true;
            if (idx.idx_defense >= 0 && idx.idx_defense < row.Length && !string.IsNullOrEmpty(row[idx.idx_defense])) return true;
            return false;
        }
        internal static CsvIndex BuildCsvIndex(string[] header, string prefix)
        {
            var idx = new CsvIndex();
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            if (!map.TryGetValue(prefix + "health", out idx.idx_health)) idx.idx_health = -1;
            if (!map.TryGetValue(prefix + "mana", out idx.idx_mana)) idx.idx_mana = -1;
            if (!map.TryGetValue(prefix + "attack", out idx.idx_attack)) idx.idx_attack = -1;
            if (!map.TryGetValue(prefix + "defense", out idx.idx_defense)) idx.idx_defense = -1;
            return idx;
        }
        internal static StatBlock FromRowWithCsvIndex(Index idx, string[] row, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new StatBlock();
            if (idx.idx_health >= 0 && idx.idx_health < row.Length) { var __cell = row[idx.idx_health]; obj.health = DataSourceFactory.ConvertValue<uint>( __cell ); }
            if (idx.idx_mana >= 0 && idx.idx_mana < row.Length) { var __cell = row[idx.idx_mana]; obj.mana = DataSourceFactory.ConvertValue<uint>( __cell ); }
            if (idx.idx_attack >= 0 && idx.idx_attack < row.Length) { var __cell = row[idx.idx_attack]; obj.attack = DataSourceFactory.ConvertValue<uint>( __cell ); }
            if (idx.idx_defense >= 0 && idx.idx_defense < row.Length) { var __cell = row[idx.idx_defense]; obj.defense = DataSourceFactory.ConvertValue<uint>( __cell ); }
            return obj;
        }
        internal sealed class Reader
        {
            public readonly CsvIndex Index;
            public readonly char Sep;
            public readonly Polygen.Common.CsvUtils.GapMode Gap;
            public Reader(Index index, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { CsvIndex = index; Sep = sep; Gap = gap; }
            public StatBlock Parse(string[] row) => FromRowWithCsvIndex(Index, row, Gap);
            internal static Reader FromHeader(string[] header, char sep = ',', string prefix = "", Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { var idx = BuildCsvIndex(header, prefix); return new Reader(idx, sep, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<StatBlock> ReadCsvWithIndex(string path, CsvIndex idx, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length <= 1) yield break;
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static System.Collections.Generic.IEnumerable<StatBlock> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var idx = BuildCsvIndex(header, string.Empty);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }
        }
        internal static StatBlock FromRow(IDictionary<string,string> row) => FromRowWithPrefix(row, string.Empty);
        internal static StatBlock FromRowWithPrefix(IDictionary<string,string> row, string prefix)
        {
            var obj = new StatBlock();
obj.health = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "health");
obj.mana = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "mana");
obj.attack = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "attack");
obj.defense = DataSourceFactory.ConvertSingleValue<uint>(row, prefix + "defense");
            return obj;
        }
        internal static IEnumerable<StatBlock> ReadCsv(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
        {
            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;
            var header = lines[0].Split(sep);
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); yield return FromRowWithPrefixAndHeaderMap(map, row, string.Empty, gap); }
        }
        internal static StatBlock FromRow(string[] header, string[] row, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) => FromRowWithPrefixAndHeader(header, row, string.Empty, gap);
        internal static StatBlock FromRowWithPrefixAndHeader(string[] header, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
            return FromRowWithPrefixAndHeaderMap(map, row, prefix, gap);
        }
        internal static StatBlock FromRowWithPrefixAndHeaderMap(System.Collections.Generic.Dictionary<string,int> map, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)
        {
            var obj = new StatBlock();
            int gap = (int)gap; // 0=Break,1=Sparse
            bool __pfxEmpty = string.IsNullOrEmpty(prefix);
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "health" : prefix + "health"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.health = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "mana" : prefix + "mana"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.mana = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "attack" : prefix + "attack"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.attack = DataSourceFactory.ConvertValue<uint>(__cell); }
{ int __idx; string __cell=null; if (map.TryGetValue((__pfxEmpty ? "defense" : prefix + "defense"), out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; obj.defense = DataSourceFactory.ConvertValue<uint>(__cell); }
            return obj;
        }
}
}


