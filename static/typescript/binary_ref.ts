// PolyGen indexed binary document support for TypeScript.
// Mirrors the C# BinaryRef format: eager indexes, lazy row field access.

const MAGIC = new Uint8Array([0x50, 0x47, 0x42, 0x52, 0x45, 0x46, 0x31, 0x00]); // PGBREF1\0
const VERSION = 1;
const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

export class BinaryDocumentOwner {
  readonly bytes: Uint8Array;
  readonly view: DataView;

  constructor(input: ArrayBuffer | Uint8Array) {
    this.bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
    this.view = new DataView(this.bytes.buffer, this.bytes.byteOffset, this.bytes.byteLength);
  }

  get length(): number {
    return this.bytes.byteLength;
  }

  reader(offset = 0): BinaryRefReader {
    return new BinaryRefReader(this.bytes, offset);
  }
}

export class BinaryRefReader {
  readonly bytes: Uint8Array;
  readonly view: DataView;
  offset: number;

  constructor(input: ArrayBuffer | Uint8Array, offset = 0) {
    this.bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
    this.view = new DataView(this.bytes.buffer, this.bytes.byteOffset, this.bytes.byteLength);
    this.offset = offset;
    BinaryRefFormat.checkRange(this.bytes, offset, 0);
  }

  position(): number {
    return this.offset;
  }

  seek(offset: number): void {
    BinaryRefFormat.checkRange(this.bytes, offset, 0);
    this.offset = offset;
  }

  skip(length: number): void {
    BinaryRefFormat.checkRange(this.bytes, this.offset, length);
    this.offset += length;
  }

  readBool(): boolean {
    return this.readU8() !== 0;
  }

  readU8(): number {
    BinaryRefFormat.checkRange(this.bytes, this.offset, 1);
    return this.bytes[this.offset++];
  }

  readI8(): number {
    const value = this.readU8();
    return value > 127 ? value - 256 : value;
  }

  readU16(): number {
    BinaryRefFormat.checkRange(this.bytes, this.offset, 2);
    const value = this.view.getUint16(this.offset, true);
    this.offset += 2;
    return value;
  }

  readI16(): number {
    BinaryRefFormat.checkRange(this.bytes, this.offset, 2);
    const value = this.view.getInt16(this.offset, true);
    this.offset += 2;
    return value;
  }

  readU32(): number {
    BinaryRefFormat.checkRange(this.bytes, this.offset, 4);
    const value = this.view.getUint32(this.offset, true);
    this.offset += 4;
    return value;
  }

  readI32(): number {
    BinaryRefFormat.checkRange(this.bytes, this.offset, 4);
    const value = this.view.getInt32(this.offset, true);
    this.offset += 4;
    return value;
  }

  readU64(): number {
    BinaryRefFormat.checkRange(this.bytes, this.offset, 8);
    const value = Number(this.view.getBigUint64(this.offset, true));
    this.offset += 8;
    return value;
  }

  readI64(): number {
    BinaryRefFormat.checkRange(this.bytes, this.offset, 8);
    const value = Number(this.view.getBigInt64(this.offset, true));
    this.offset += 8;
    return value;
  }

  readF32(): number {
    BinaryRefFormat.checkRange(this.bytes, this.offset, 4);
    const value = this.view.getFloat32(this.offset, true);
    this.offset += 4;
    return value;
  }

  readF64(): number {
    BinaryRefFormat.checkRange(this.bytes, this.offset, 8);
    const value = this.view.getFloat64(this.offset, true);
    this.offset += 8;
    return value;
  }

  readBytes(length: number): Uint8Array {
    BinaryRefFormat.checkRange(this.bytes, this.offset, length);
    const value = this.bytes.subarray(this.offset, this.offset + length);
    this.offset += length;
    return value;
  }
}

export class BinaryRefWriter {
  private chunks: Uint8Array[] = [];
  private totalLength = 0;

  get length(): number {
    return this.totalLength;
  }

  writeBool(value: boolean): void {
    this.writeU8(value ? 1 : 0);
  }

  writeU8(value: number): void {
    this.push(new Uint8Array([value & 0xff]));
  }

  writeI8(value: number): void {
    this.writeU8(value);
  }

  writeU16(value: number): void {
    const b = new Uint8Array(2);
    new DataView(b.buffer).setUint16(0, value, true);
    this.push(b);
  }

  writeI16(value: number): void {
    const b = new Uint8Array(2);
    new DataView(b.buffer).setInt16(0, value, true);
    this.push(b);
  }

  writeU32(value: number): void {
    const b = new Uint8Array(4);
    new DataView(b.buffer).setUint32(0, value, true);
    this.push(b);
  }

  writeI32(value: number): void {
    const b = new Uint8Array(4);
    new DataView(b.buffer).setInt32(0, value, true);
    this.push(b);
  }

  writeU64(value: number): void {
    const b = new Uint8Array(8);
    new DataView(b.buffer).setBigUint64(0, BigInt(value), true);
    this.push(b);
  }

  writeI64(value: number): void {
    const b = new Uint8Array(8);
    new DataView(b.buffer).setBigInt64(0, BigInt(value), true);
    this.push(b);
  }

  writeF32(value: number): void {
    const b = new Uint8Array(4);
    new DataView(b.buffer).setFloat32(0, value, true);
    this.push(b);
  }

  writeF64(value: number): void {
    const b = new Uint8Array(8);
    new DataView(b.buffer).setFloat64(0, value, true);
    this.push(b);
  }

  writeRaw(bytes: Uint8Array): void {
    this.push(bytes);
  }

  toUint8Array(): Uint8Array {
    const out = new Uint8Array(this.totalLength);
    let cursor = 0;
    for (const chunk of this.chunks) {
      out.set(chunk, cursor);
      cursor += chunk.byteLength;
    }
    return out;
  }

  private push(bytes: Uint8Array): void {
    this.chunks.push(bytes);
    this.totalLength += bytes.byteLength;
  }
}

export class BinaryRefFormat {
  static writeHeader(writer: BinaryRefWriter): void {
    writer.writeRaw(MAGIC);
    writer.writeI32(VERSION);
  }

  static readHeader(reader: BinaryRefReader): void {
    const magic = reader.readBytes(MAGIC.byteLength);
    for (let i = 0; i < MAGIC.byteLength; i++) {
      if (magic[i] !== MAGIC[i]) {
        throw new Error("Invalid PolyGen binary ref header.");
      }
    }
    const version = reader.readI32();
    if (version !== VERSION) {
      throw new Error(`Unsupported PolyGen binary ref version: ${version}.`);
    }
  }

  static writeString(writer: BinaryRefWriter, value: string): void {
    const bytes = textEncoder.encode(value);
    writer.writeI32(bytes.byteLength);
    writer.writeRaw(bytes);
  }

  static readString(reader: BinaryRefReader): string {
    const length = reader.readI32();
    if (length < 0) throw new Error("Negative string length.");
    return textDecoder.decode(reader.readBytes(length));
  }

  static writeBytes(writer: BinaryRefWriter, value: Uint8Array): void {
    writer.writeI32(value.byteLength);
    writer.writeRaw(value);
  }

  static readLengthPrefixedBytes(buffer: Uint8Array, offset: number): Uint8Array {
    this.checkRange(buffer, offset, 4);
    const view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);
    const length = view.getInt32(offset, true);
    if (length < 0) throw new Error("Negative binary payload length.");
    const payloadOffset = offset + 4;
    this.checkRange(buffer, payloadOffset, length);
    return buffer.subarray(payloadOffset, payloadOffset + length);
  }

  static readUtf8String(buffer: Uint8Array, offset: number): string {
    return textDecoder.decode(this.readLengthPrefixedBytes(buffer, offset));
  }

  static requireFieldOffset(buffer: Uint8Array, rowOffset: number, fieldIndex: number): number {
    const offset = this.getFieldOffset(buffer, rowOffset, fieldIndex);
    if (offset < 0) throw new Error(`Missing required binary field at index ${fieldIndex}.`);
    return offset;
  }

  static getFieldOffset(buffer: Uint8Array, rowOffset: number, fieldIndex: number): number {
    this.checkRange(buffer, rowOffset, 4);
    const view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);
    const fieldCount = view.getInt32(rowOffset, true);
    if (fieldCount < 0) throw new Error("Negative binary field count.");
    if (fieldIndex < 0 || fieldIndex >= fieldCount) return -1;

    const tableOffset = rowOffset + 4;
    this.checkRange(buffer, tableOffset, fieldCount * 4);
    const relative = view.getInt32(tableOffset + fieldIndex * 4, true);
    if (relative < 0) return -1;
    const absolute = rowOffset + relative;
    this.checkRange(buffer, absolute, 0);
    return absolute;
  }

  static readBool(buffer: Uint8Array, offset: number): boolean {
    this.checkRange(buffer, offset, 1);
    return buffer[offset] !== 0;
  }

  static readU8(buffer: Uint8Array, offset: number): number {
    this.checkRange(buffer, offset, 1);
    return buffer[offset];
  }

  static readI8(buffer: Uint8Array, offset: number): number {
    const value = this.readU8(buffer, offset);
    return value > 127 ? value - 256 : value;
  }

  static readU16(buffer: Uint8Array, offset: number): number {
    this.checkRange(buffer, offset, 2);
    return new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength).getUint16(offset, true);
  }

  static readI16(buffer: Uint8Array, offset: number): number {
    this.checkRange(buffer, offset, 2);
    return new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength).getInt16(offset, true);
  }

  static readU32(buffer: Uint8Array, offset: number): number {
    this.checkRange(buffer, offset, 4);
    return new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength).getUint32(offset, true);
  }

  static readI32(buffer: Uint8Array, offset: number): number {
    this.checkRange(buffer, offset, 4);
    return new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength).getInt32(offset, true);
  }

  static readU64(buffer: Uint8Array, offset: number): number {
    this.checkRange(buffer, offset, 8);
    return Number(new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength).getBigUint64(offset, true));
  }

  static readI64(buffer: Uint8Array, offset: number): number {
    this.checkRange(buffer, offset, 8);
    return Number(new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength).getBigInt64(offset, true));
  }

  static readF32(buffer: Uint8Array, offset: number): number {
    this.checkRange(buffer, offset, 4);
    return new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength).getFloat32(offset, true);
  }

  static readF64(buffer: Uint8Array, offset: number): number {
    this.checkRange(buffer, offset, 8);
    return new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength).getFloat64(offset, true);
  }

  static readTimestamp(buffer: Uint8Array, offset: number): Date {
    const ticks = this.readI64(buffer, offset);
    return new Date(ticks / 10000 - 62135596800000);
  }

  static writeTimestamp(writer: BinaryRefWriter, value: Date): void {
    writer.writeI64((value.getTime() + 62135596800000) * 10000);
  }

  static checkRange(buffer: Uint8Array, offset: number, length: number): void {
    if (offset < 0 || length < 0 || offset > buffer.byteLength || length > buffer.byteLength - offset) {
      throw new Error("Binary ref offset is outside the document.");
    }
  }
}

export class BinaryRefRowBuilder {
  private readonly fields: Array<Uint8Array | null>;

  constructor(fieldCount: number) {
    if (fieldCount < 0) throw new Error("Negative binary field count.");
    this.fields = new Array(fieldCount).fill(null);
  }

  setField(index: number, write: (writer: BinaryRefWriter) => void): void {
    const writer = new BinaryRefWriter();
    write(writer);
    this.fields[index] = writer.toUint8Array();
  }

  toUint8Array(): Uint8Array {
    const writer = new BinaryRefWriter();
    writer.writeI32(this.fields.length);

    let cursor = 4 + this.fields.length * 4;
    for (const field of this.fields) {
      if (field == null) {
        writer.writeI32(-1);
      } else {
        writer.writeI32(cursor);
        cursor += field.byteLength;
      }
    }

    for (const field of this.fields) {
      if (field != null) writer.writeRaw(field);
    }

    return writer.toUint8Array();
  }
}
