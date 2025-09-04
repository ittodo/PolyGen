// Common CSV helper utilities for generated mappers
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;

namespace Polygen.Common
{
    // Hierarchical CSV header index structure
    // Either a single index (leaf) or a list of children (composite)
    public sealed class CsvIndexHeader
    {
        public int Index = -1;
        public List<CsvIndexHeader>? IndexList;

        public bool HasAny() => Index >= 0 || (IndexList != null && IndexList.Count > 0);
    }

    public static class CsvUtils
    {
        public enum GapMode { Break = 0, Sparse = 1 }

        public static string Escape(string? value, char sep = ',')
        {
            if (value == null) return string.Empty;
            bool needQuotes = value.Contains(sep) || value.Contains('"') || value.Contains('\n') || value.Contains('\r');
            if (!needQuotes) return value;
            var sb = new StringBuilder();
            sb.Append('"');
            foreach (var ch in value)
            {
                if (ch == '"') sb.Append("\"\"");
                else sb.Append(ch);
            }
            sb.Append('"');
            return sb.ToString();
        }

        public static string ToStringInvariant(object? value)
        {
            if (value == null) return string.Empty;
            switch (value)
            {
                case string s: return s;
                case bool b: return b ? "true" : "false";
                case IFormattable f: return f.ToString(null, CultureInfo.InvariantCulture) ?? string.Empty;
                default: return value.ToString() ?? string.Empty;
            }
        }

        public static void WriteCsv(string path, IEnumerable<string[]> rows, string[]? header = null, char sep = ',')
        {
            using var sw = new StreamWriter(path, false, new UTF8Encoding(false));
            if (header != null)
            {
                sw.WriteLine(Join(header, sep));
            }
            foreach (var row in rows)
            {
                sw.WriteLine(Join(row, sep));
            }
        }

        public static string Join(IReadOnlyList<string> fields, char sep = ',')
        {
            if (fields.Count == 0) return string.Empty;
            var sb = new StringBuilder();
            for (int i = 0; i < fields.Count; i++)
            {
                if (i > 0) sb.Append(sep);
                sb.Append(Escape(fields[i], sep));
            }
            return sb.ToString();
        }

        public static System.Collections.Generic.Dictionary<string,int> IndexHeader(string[] header)
        {
            var map = new System.Collections.Generic.Dictionary<string,int>(System.StringComparer.OrdinalIgnoreCase);
            for (int i = 0; i < header.Length; i++)
            {
                // duplicate keys keep first index
                if (!map.ContainsKey(header[i])) map[header[i]] = i;
            }
            return map;
        }

        // Backward/compat alias used by templates
        public static System.Collections.Generic.Dictionary<string,int> CsvIndexHeader(string[] header)
            => IndexHeader(header);

        // Recursively check if any mapped header cell contains a non-empty value
        public static bool HeaderHasValues(CsvIndexHeader? h, string[] row)
        {
            if (h == null) return false;
            if (h.Index >= 0)
            {
                return h.Index < row.Length && !string.IsNullOrEmpty(row[h.Index]);
            }
            if (h.IndexList != null)
            {
                foreach (var c in h.IndexList)
                {
                    if (HeaderHasValues(c, row)) return true;
                }
            }
            return false;
        }
    }
}
