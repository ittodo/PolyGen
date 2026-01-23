// Common binary helper utilities for generated readers
// This file is copied to output/<lang>/Common and can be customized as needed.
using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace Polygen.Common
{
    public static class BinaryUtils
    {
        private static readonly UTF8Encoding Utf8Strict = new UTF8Encoding(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);

        // Reads a UTF-8 string prefixed by a little-endian u32 length (in bytes).
        public static string ReadUtf8String(BinaryReader br)
        {
            uint len = br.ReadUInt32();
            if (len == 0) return string.Empty;
            var bytes = br.ReadBytes(checked((int)len));
            return Utf8Strict.GetString(bytes);
        }

        // Reads an optional UTF-8 string using a 1-byte presence flag (0/!=0), then length-prefixed string if present.
        public static string? ReadUtf8StringOption(BinaryReader br)
        {
            byte has = br.ReadByte();
            if (has == 0) return null;
            return ReadUtf8String(br);
        }

        // Reads a list of items: u32 count followed by 'count' items via provided reader.
        public static List<T> ReadList<T>(BinaryReader br, Func<BinaryReader, T> reader)
        {
            uint n = br.ReadUInt32();
            var list = new List<T>(checked((int)n));
            for (int i = 0; i < (int)n; i++)
            {
                list.Add(reader(br));
            }
            return list;
        }

        // Reads an enum with i32 underlying value and casts to T.
        public static T ReadEnumInt32<T>(BinaryReader br) where T : struct, Enum
        {
            int raw = br.ReadInt32();
            return (T)Enum.ToObject(typeof(T), raw);
        }

        // Reads an Option<T> using a 1-byte presence flag (0/!=0), then provided reader when present.
        public static T ReadOption<T>(BinaryReader br, Func<BinaryReader, T> reader)
        {
            byte has = br.ReadByte();
            if (has == 0) return default!;
            return reader(br);
        }

        // Reads a byte array prefixed by a little-endian u32 length.
        public static byte[] ReadBytes(BinaryReader br)
        {
            uint len = br.ReadUInt32();
            if (len == 0) return Array.Empty<byte>();
            return br.ReadBytes(checked((int)len));
        }

        // Writes a UTF-8 string prefixed by a little-endian u32 length (in bytes).
        public static void WriteUtf8String(BinaryWriter bw, string value)
        {
            if (value == null) { bw.Write((uint)0); return; }
            var bytes = Utf8Strict.GetBytes(value);
            bw.Write((uint)bytes.Length);
            if (bytes.Length > 0) bw.Write(bytes);
        }

        // Writes a list with u32 count followed by elements using the provided writer.
        public static void WriteList<T>(BinaryWriter bw, IList<T> list, Action<BinaryWriter, T> writer)
        {
            if (list == null) { bw.Write((uint)0); return; }
            bw.Write((uint)list.Count);
            for (int i = 0; i < list.Count; i++) writer(bw, list[i]);
        }

        // Writes a byte array prefixed by a little-endian u32 length.
        public static void WriteBytes(BinaryWriter bw, byte[] value)
        {
            if (value == null) { bw.Write((uint)0); return; }
            bw.Write((uint)value.Length);
            if (value.Length > 0) bw.Write(value);
        }

        // Writes an enum with i32 underlying value.
        public static void WriteEnumInt32<T>(BinaryWriter bw, T value) where T : struct, Enum
        {
            int raw = Convert.ToInt32(value);
            bw.Write(raw);
        }

        // Writes an optional reference type value with 1-byte presence flag + payload via writer.
        public static void WriteOptionRef<T>(BinaryWriter bw, T value, Action<BinaryWriter, T> writer) where T : class
        {
            if (value == null) { bw.Write((byte)0); return; }
            bw.Write((byte)1);
            writer(bw, value);
        }

        // Writes an optional value type (nullable) with 1-byte presence flag + payload via writer.
        public static void WriteOptionStruct<T>(BinaryWriter bw, T? value, Action<BinaryWriter, T> writer) where T : struct
        {
            if (!value.HasValue) { bw.Write((byte)0); return; }
            bw.Write((byte)1);
            writer(bw, value.Value);
        }
    }
}
