// JSON → Table → CSV converter per docs/json-to-csv-conversion-spec.md
// Assumes JSON is already materialized as Dictionary<string, object?> and List<object?>
// and primitive CLR types (string, bool, numeric types, null).
using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;
using System.Text.Json;

namespace Polygen.Common
{
    public static class JsonCsvConverter
    {
        public sealed class Config
        {
            // 'dynamic' or 'fixed'
            public string ListStrategy { get; set; } = "dynamic";
            // Used when ListStrategy == 'fixed' (interpreted as max index inclusive)
            public int FixedListMax { get; set; } = 0;
            public char Sep { get; set; } = ',';
            public string Newline { get; set; } = "\n"; // "\n" or "\r\n"
            public bool Bom { get; set; } = false;
            public bool IncludeHeader { get; set; } = true;
        }

        public sealed class Table
        {
            public string[] Header { get; }
            public List<string[]> Rows { get; }
            public Table(string[] header, List<string[]> rows)
            {
                Header = header;
                Rows = rows;
            }
        }

        private struct Token
        {
            public string? Key;
            public int? Index;
        }

        public static Table JsonToTable(object? json, Config cfg)
        {
            var (prototypeHeader, observedMaxes) = ScanSchema(json);

            Dictionary<string, int> maxes;
            if (string.Equals(cfg.ListStrategy, "fixed", StringComparison.OrdinalIgnoreCase))
            {
                // Ensure at least K, but never less than observed (avoid truncation)
                maxes = new Dictionary<string, int>(observedMaxes, StringComparer.Ordinal);
                foreach (var col in prototypeHeader)
                {
                    if (ContainsIndexZero(col))
                    {
                        var root = RootOf(col);
                        maxes.TryGetValue(root, out var observed);
                        var target = cfg.FixedListMax;
                        if (target < observed) target = observed; // expand to cover data
                        maxes[root] = target;
                    }
                }
            }
            else
            {
                maxes = observedMaxes;
            }

            var header = BuildHeader(prototypeHeader, maxes);

            // Rows: if top-level is a list, each element is a row object; otherwise single row
            var rows = new List<string[]>();
            if (json is List<object?> list)
            {
                foreach (var elem in list)
                {
                    rows.Add(FlattenToRow(elem, header));
                }
            }
            else
            {
                rows.Add(FlattenToRow(json, header));
            }

            return new Table(header, rows);
        }

        public static string JsonToCsv(object? json, Config cfg)
        {
            var table = JsonToTable(json, cfg);
            var sep = cfg.Sep;
            var nl = cfg.Newline ?? "\n";

            // Escape helper delegates to CsvUtils to keep quoting rules identical
            static string Esc(string? s, char sep) => CsvUtils.Escape(s ?? string.Empty, sep);

            var sb = new StringBuilder();
            if (cfg.Bom) sb.Append('\uFEFF');

            if (cfg.IncludeHeader)
            {
                sb.Append(string.Join(sep, table.Header.Select(h => Esc(h, sep))));
                if (table.Rows.Count > 0) sb.Append(nl);
            }

            for (int r = 0; r < table.Rows.Count; r++)
            {
                var row = table.Rows[r];
                sb.Append(string.Join(sep, row.Select(c => Esc(c, sep))));
                if (r < table.Rows.Count - 1) sb.Append(nl);
            }

            return sb.ToString();
        }

        private static (List<string> prototypeHeader, Dictionary<string, int> observedMaxes) ScanSchema(object? json)
        {
            var prototype = new List<string>();
            var observed = new Dictionary<string, int>(StringComparer.Ordinal);

            void Walk(object? value, string path, bool isRoot)
            {
                if (value is Dictionary<string, object?> obj)
                {
                    foreach (var kv in obj)
                    {
                        var childPath = string.IsNullOrEmpty(path) ? kv.Key : path + "." + kv.Key;
                        Walk(kv.Value, childPath, isRoot: false);
                    }
                    return;
                }

                if (value is List<object?> arr)
                {
                    // Special case: if this is the top-level array (path == "" and isRoot == true),
                    // do not record list root or index in path; header is relative to elements.
                    if (string.IsNullOrEmpty(path) && isRoot)
                    {
                        for (int i = 0; i < arr.Count; i++)
                        {
                            Walk(arr[i], path, isRoot: false);
                        }
                    }
                    else
                    {
                        var root = path;
                        for (int i = 0; i < arr.Count; i++)
                        {
                            if (!observed.TryGetValue(root, out var m) || i > m) observed[root] = i;
                            Walk(arr[i], path + "[" + i.ToString(CultureInfo.InvariantCulture) + "]", isRoot: false);
                        }
                    }
                    return;
                }

                // Primitive leaf -> record prototype path with indices normalized to [0]
                var proto = NormalizeIndicesToZero(path);
                if (proto.Length > 0)
                {
                    if (!prototype.Contains(proto, StringComparer.Ordinal))
                        prototype.Add(proto);
                }
            }

            Walk(json, path: string.Empty, isRoot: true);
            return (prototype, observed);
        }

        private static string[] BuildHeader(List<string> prototypeHeader, Dictionary<string, int> listMaxes)
        {
            var outCols = new List<string>();
            foreach (var col in prototypeHeader)
            {
                if (ContainsIndexZero(col))
                {
                    var root = RootOf(col);
                    listMaxes.TryGetValue(root, out var maxI);
                    for (int i = 0; i <= maxI; i++)
                    {
                        outCols.Add(ReplaceFirstIndex(col, i));
                    }
                }
                else
                {
                    outCols.Add(col);
                }
            }
            return outCols.ToArray();
        }

        private static string[] FlattenToRow(object? obj, string[] header)
        {
            var row = new string[header.Length];
            for (int i = 0; i < header.Length; i++)
            {
                var tokens = ParsePath(header[i]);
                var v = GetAt(obj, tokens);
                row[i] = ToStringValue(v);
            }
            return row;
        }

        private static List<Token> ParsePath(string col)
        {
            var tokens = new List<Token>();
            int i = 0;
            while (i < col.Length)
            {
                // parse key segment
                string? key = null;
                int start = i;
                while (i < col.Length && col[i] != '.' && col[i] != '[') i++;
                if (i > start)
                {
                    key = col.Substring(start, i - start);
                }

                int? index = null;
                if (i < col.Length && col[i] == '[')
                {
                    i++; // skip '['
                    int idxStart = i;
                    while (i < col.Length && col[i] != ']') i++;
                    if (i >= col.Length) throw new FormatException($"Invalid path: {col}");
                    var idxStr = col.Substring(idxStart, i - idxStart);
                    if (!int.TryParse(idxStr, NumberStyles.Integer, CultureInfo.InvariantCulture, out var idx))
                        throw new FormatException($"Invalid index in path: {col}");
                    index = idx;
                    i++; // skip ']'
                }

                tokens.Add(new Token { Key = key, Index = index });

                if (i < col.Length && col[i] == '.') i++; // skip '.'
            }
            return tokens;
        }

        private static object? GetAt(object? obj, List<Token> tokens)
        {
            object? cur = obj;
            foreach (var t in tokens)
            {
                if (cur == null) return null;

                if (t.Key != null)
                {
                    if (cur is Dictionary<string, object?> dict)
                    {
                        dict.TryGetValue(t.Key, out cur);
                    }
                    else
                    {
                        return null;
                    }
                }

                if (t.Index.HasValue)
                {
                    if (cur is List<object?> list)
                    {
                        var idx = t.Index.Value;
                        if (idx < 0 || idx >= list.Count) return null;
                        cur = list[idx];
                    }
                    else
                    {
                        return null;
                    }
                }
            }
            return cur;
        }

        private static string ToStringValue(object? v)
        {
            if (v is null) return string.Empty;
            switch (v)
            {
                case string s:
                    return s;
                case bool b:
                    return b ? "true" : "false";
                case sbyte or byte or short or ushort or int or uint or long or ulong or float or double or decimal:
                    return Convert.ToString(v, CultureInfo.InvariantCulture) ?? string.Empty;
                case Dictionary<string, object?> or List<object?>:
                    return JsonSerializer.Serialize(v);
                default:
                    return v.ToString() ?? string.Empty;
            }
        }

        private static bool ContainsIndexZero(string col) => col.Contains("[0]", StringComparison.Ordinal);

        private static string RootOf(string col)
        {
            var idx = col.IndexOf("[0]", StringComparison.Ordinal);
            if (idx < 0) return col;
            // Trim trailing dot if any (e.g., "a.b" from "a.b[0]")
            return col.Substring(0, idx);
        }

        private static string ReplaceFirstIndex(string col, int i)
        {
            var idx = col.IndexOf("[0]", StringComparison.Ordinal);
            if (idx < 0) return col;
            var before = col.Substring(0, idx);
            var after = col.Substring(idx + 3);
            return before + "[" + i.ToString(CultureInfo.InvariantCulture) + "]" + after;
        }

        private static string NormalizeIndicesToZero(string path)
        {
            if (string.IsNullOrEmpty(path)) return path;
            var sb = new StringBuilder(path.Length);
            for (int i = 0; i < path.Length; i++)
            {
                char c = path[i];
                if (c == '[')
                {
                    // write "[0]" and skip until ']'
                    sb.Append("[0]");
                    i++;
                    while (i < path.Length && path[i] != ']') i++;
                    if (i >= path.Length) break; // malformed; best effort
                }
                else
                {
                    sb.Append(c);
                }
            }
            return sb.ToString();
        }
    }
}
