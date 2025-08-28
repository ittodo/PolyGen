// Common JSON helper utilities for generated mappers
// Copied to output/<lang>/Common and safe to customize.
using System;
using System.Buffers;
using System.Collections.Generic;
using System.Text.Json;

namespace Polygen.Common
{
    public static class JsonUtils
    {
        // Case-insensitive property lookup on a JsonElement object
        public static bool TryGetPropertyCaseInsensitive(in JsonElement obj, string name, out JsonElement value)
        {
            if (obj.ValueKind != JsonValueKind.Object)
            {
                value = default;
                return false;
            }

            // Fast path: exact case
            if (obj.TryGetProperty(name, out value)) return true;

            foreach (var prop in obj.EnumerateObject())
            {
                if (string.Equals(prop.Name, name, StringComparison.OrdinalIgnoreCase))
                {
                    value = prop.Value;
                    return true;
                }
            }
            value = default;
            return false;
        }

        // Flexible enum reader: accepts string (case-insensitive) or number (i32)
        public static T ReadEnum<T>(in JsonElement e) where T : struct, Enum
        {
            if (e.ValueKind == JsonValueKind.String)
            {
                var s = e.GetString();
                if (Enum.TryParse<T>(s, ignoreCase: true, out var val)) return val;
                throw new FormatException($"Invalid enum value '{s}' for {typeof(T).Name}");
            }
            if (e.ValueKind == JsonValueKind.Number)
            {
                int v = e.GetInt32();
                return (T)Enum.ToObject(typeof(T), v);
            }
            throw new FormatException($"Expected string or number for enum {typeof(T).Name}, got {e.ValueKind}");
        }

        // Reads list from a JSON array using provided element reader
        public static List<T> ReadList<T>(in JsonElement e, Func<JsonElement, T> reader)
        {
            if (e.ValueKind == JsonValueKind.Null) return new List<T>();
            if (e.ValueKind != JsonValueKind.Array) throw new FormatException("Expected JSON array");
            var list = new List<T>(e.GetArrayLength());
            foreach (var item in e.EnumerateArray()) list.Add(reader(item));
            return list;
        }

        // Writes enum as string by default
        public static void WriteEnum<T>(Utf8JsonWriter w, in T value) where T : struct, Enum
        {
            w.WriteStringValue(value.ToString());
        }

        public static void WriteOptionRef<T>(Utf8JsonWriter w, string name, T value, Action<Utf8JsonWriter, T> writer) where T : class
        {
            if (value is null) { w.WriteNull(name); return; }
            w.WritePropertyName(name);
            writer(w, value);
        }

        public static void WriteOptionStruct<T>(Utf8JsonWriter w, string name, T? value, Action<Utf8JsonWriter, T> writer) where T : struct
        {
            if (!value.HasValue) { w.WriteNull(name); return; }
            w.WritePropertyName(name);
            writer(w, value.Value);
        }

        public static void WriteList<T>(Utf8JsonWriter w, string name, IReadOnlyList<T> list, Action<Utf8JsonWriter, T> writer)
        {
            w.WritePropertyName(name);
            w.WriteStartArray();
            if (list != null)
            {
                for (int i = 0; i < list.Count; i++) writer(w, list[i]);
            }
            w.WriteEndArray();
        }
    }
}

