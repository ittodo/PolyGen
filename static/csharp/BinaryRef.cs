// This file is a part of the Polygen common utility library.
// It provides indexed binary document support for lazy row references.
using System;
using System.Buffers.Binary;
using System.IO;
using System.Text;

namespace Polygen.Common
{
    /// <summary>
    /// Owns the byte buffer behind binary row references.
    /// Row reference classes keep this object reachable, so the underlying buffer stays alive.
    /// </summary>
    public sealed class BinaryDocumentOwner
    {
        private readonly byte[] _buffer;

        public BinaryDocumentOwner(byte[] buffer)
        {
            _buffer = buffer ?? throw new ArgumentNullException(nameof(buffer));
        }

        public ReadOnlySpan<byte> Span => _buffer;

        public int Length => _buffer.Length;

        public static BinaryDocumentOwner Open(string filePath)
        {
            return new BinaryDocumentOwner(File.ReadAllBytes(filePath));
        }

        public BinaryReader OpenReader()
        {
            return new BinaryReader(new MemoryStream(_buffer, writable: false), Encoding.UTF8);
        }

        public BinaryReader OpenReaderAt(int offset)
        {
            BinaryRefFormat.CheckRange(_buffer, offset, 0);
            return new BinaryReader(
                new MemoryStream(_buffer, offset, _buffer.Length - offset, writable: false),
                Encoding.UTF8);
        }
    }

    public static class BinaryRefFormat
    {
        private static readonly byte[] Magic = { (byte)'P', (byte)'G', (byte)'B', (byte)'R', (byte)'E', (byte)'F', (byte)'1', 0 };
        private const int Version = 2;

        public static void WriteHeader(BinaryWriter writer)
        {
            writer.Write(Magic);
            writer.Write(Version);
        }

        public static void ReadHeader(BinaryReader reader)
        {
            var magic = reader.ReadBytes(Magic.Length);
            if (magic.Length != Magic.Length)
                throw new InvalidDataException("Invalid PolyGen binary ref header.");

            for (var i = 0; i < Magic.Length; i++)
            {
                if (magic[i] != Magic[i])
                    throw new InvalidDataException("Invalid PolyGen binary ref header.");
            }

            var version = reader.ReadInt32();
            if (version != Version)
                throw new InvalidDataException($"Unsupported PolyGen binary ref version: {version}.");
        }

        public static void WriteString(BinaryWriter writer, string value)
        {
            var bytes = Encoding.UTF8.GetBytes(value);
            writer.Write(bytes.Length);
            writer.Write(bytes);
        }

        public static string ReadString(BinaryReader reader)
        {
            var length = reader.ReadInt32();
            if (length < 0)
                throw new InvalidDataException("Negative string length.");
            var bytes = reader.ReadBytes(length);
            if (bytes.Length != length)
                throw new EndOfStreamException();
            return Encoding.UTF8.GetString(bytes);
        }

        public static int RequireFieldOffset(ReadOnlySpan<byte> buffer, int rowOffset, int fieldIndex)
        {
            var offset = GetFieldOffset(buffer, rowOffset, fieldIndex);
            if (offset < 0)
                throw new InvalidDataException($"Missing required binary field at index {fieldIndex}.");
            return offset;
        }

        public static int GetFieldOffset(ReadOnlySpan<byte> buffer, int rowOffset, int fieldIndex)
        {
            CheckRange(buffer, rowOffset, sizeof(int));
            var fieldCount = BinaryPrimitives.ReadInt32LittleEndian(buffer.Slice(rowOffset, sizeof(int)));
            if (fieldCount < 0)
                throw new InvalidDataException("Negative binary field count.");
            if (fieldIndex < 0 || fieldIndex >= fieldCount)
                return -1;

            var tableOffset = rowOffset + sizeof(int);
            CheckRange(buffer, tableOffset, checked(fieldCount * sizeof(int)));
            var relative = BinaryPrimitives.ReadInt32LittleEndian(buffer.Slice(tableOffset + fieldIndex * sizeof(int), sizeof(int)));
            if (relative < 0)
                return -1;

            var absolute = checked(rowOffset + relative);
            CheckRange(buffer, absolute, 0);
            return absolute;
        }

        public static ReadOnlySpan<byte> ReadLengthPrefixedBytes(ReadOnlySpan<byte> buffer, int offset)
        {
            CheckRange(buffer, offset, sizeof(int));
            var length = BinaryPrimitives.ReadInt32LittleEndian(buffer.Slice(offset, sizeof(int)));
            if (length < 0)
                throw new InvalidDataException("Negative binary payload length.");
            var payloadOffset = checked(offset + sizeof(int));
            CheckRange(buffer, payloadOffset, length);
            return buffer.Slice(payloadOffset, length);
        }

        public static string ReadUtf8String(ReadOnlySpan<byte> buffer, int offset)
        {
            return Encoding.UTF8.GetString(ReadLengthPrefixedBytes(buffer, offset));
        }

        public static bool ReadBool(ReadOnlySpan<byte> buffer, int offset)
        {
            CheckRange(buffer, offset, 1);
            return buffer[offset] != 0;
        }

        public static byte ReadU8(ReadOnlySpan<byte> buffer, int offset)
        {
            CheckRange(buffer, offset, 1);
            return buffer[offset];
        }

        public static sbyte ReadI8(ReadOnlySpan<byte> buffer, int offset)
        {
            CheckRange(buffer, offset, 1);
            return unchecked((sbyte)buffer[offset]);
        }

        public static ushort ReadU16(ReadOnlySpan<byte> buffer, int offset)
        {
            CheckRange(buffer, offset, sizeof(ushort));
            return BinaryPrimitives.ReadUInt16LittleEndian(buffer.Slice(offset, sizeof(ushort)));
        }

        public static short ReadI16(ReadOnlySpan<byte> buffer, int offset)
        {
            CheckRange(buffer, offset, sizeof(short));
            return BinaryPrimitives.ReadInt16LittleEndian(buffer.Slice(offset, sizeof(short)));
        }

        public static uint ReadU32(ReadOnlySpan<byte> buffer, int offset)
        {
            CheckRange(buffer, offset, sizeof(uint));
            return BinaryPrimitives.ReadUInt32LittleEndian(buffer.Slice(offset, sizeof(uint)));
        }

        public static int ReadI32(ReadOnlySpan<byte> buffer, int offset)
        {
            CheckRange(buffer, offset, sizeof(int));
            return BinaryPrimitives.ReadInt32LittleEndian(buffer.Slice(offset, sizeof(int)));
        }

        public static ulong ReadU64(ReadOnlySpan<byte> buffer, int offset)
        {
            CheckRange(buffer, offset, sizeof(ulong));
            return BinaryPrimitives.ReadUInt64LittleEndian(buffer.Slice(offset, sizeof(ulong)));
        }

        public static long ReadI64(ReadOnlySpan<byte> buffer, int offset)
        {
            CheckRange(buffer, offset, sizeof(long));
            return BinaryPrimitives.ReadInt64LittleEndian(buffer.Slice(offset, sizeof(long)));
        }

        public static float ReadF32(ReadOnlySpan<byte> buffer, int offset)
        {
            CheckRange(buffer, offset, sizeof(float));
            return BinaryPrimitives.ReadSingleLittleEndian(buffer.Slice(offset, sizeof(float)));
        }

        public static double ReadF64(ReadOnlySpan<byte> buffer, int offset)
        {
            CheckRange(buffer, offset, sizeof(double));
            return BinaryPrimitives.ReadDoubleLittleEndian(buffer.Slice(offset, sizeof(double)));
        }

        public static TEnum ReadEnumInt32<TEnum>(ReadOnlySpan<byte> buffer, int offset) where TEnum : struct, Enum
        {
            var raw = ReadI32(buffer, offset);
            if (!Enum.IsDefined(typeof(TEnum), raw))
                throw new InvalidDataException($"Invalid enum value for {typeof(TEnum).FullName}: {raw}");
            return (TEnum)Enum.ToObject(typeof(TEnum), raw);
        }

        public static DateTime ReadTimestamp(ReadOnlySpan<byte> buffer, int offset)
        {
            return new DateTime(ReadI64(buffer, offset), DateTimeKind.Utc);
        }

        public static void CheckRange(ReadOnlySpan<byte> buffer, int offset, int length)
        {
            if (offset < 0 || length < 0 || offset > buffer.Length || length > buffer.Length - offset)
                throw new InvalidDataException("Binary ref offset is outside the document.");
        }
    }

    public sealed class BinaryRefRowBuilder
    {
        private readonly byte[]?[] _fields;

        public BinaryRefRowBuilder(int fieldCount)
        {
            if (fieldCount < 0)
                throw new ArgumentOutOfRangeException(nameof(fieldCount));
            _fields = new byte[fieldCount][];
        }

        public void SetField(int index, Action<BinaryWriter> write)
        {
            using var stream = new MemoryStream();
            using (var writer = new BinaryWriter(stream, Encoding.UTF8, leaveOpen: true))
            {
                write(writer);
            }
            _fields[index] = stream.ToArray();
        }

        public byte[] ToArray()
        {
            using var stream = new MemoryStream();
            using var writer = new BinaryWriter(stream, Encoding.UTF8, leaveOpen: true);

            writer.Write(_fields.Length);
            var headerSize = sizeof(int) + _fields.Length * sizeof(int);
            var cursor = headerSize;

            foreach (var field in _fields)
            {
                if (field == null)
                {
                    writer.Write(-1);
                }
                else
                {
                    writer.Write(cursor);
                    cursor = checked(cursor + field.Length);
                }
            }

            foreach (var field in _fields)
            {
                if (field != null)
                    writer.Write(field);
            }

            return stream.ToArray();
        }
    }
}
