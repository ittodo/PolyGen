// Package polygen provides common utilities for PolyGen generated code.
// This file contains validation, loading, and binary I/O support.
package polygen

import (
	"bytes"
	"encoding/binary"
	"encoding/csv"
	"encoding/json"
	"fmt"
	"io"
	"math"
	"os"
	"regexp"
	"strconv"
	"strings"
)

// ============ Validation ============

// ValidationSeverity indicates the severity level of a validation error.
type ValidationSeverity int

const (
	// SeverityError indicates a critical validation failure.
	SeverityError ValidationSeverity = iota
	// SeverityWarning indicates a non-critical validation issue.
	SeverityWarning
)

func (s ValidationSeverity) String() string {
	switch s {
	case SeverityError:
		return "Error"
	case SeverityWarning:
		return "Warning"
	default:
		return "Unknown"
	}
}

// ValidationError represents a single validation failure.
type ValidationError struct {
	TableName      string
	FieldName      string
	RowKey         string
	Message        string
	Severity       ValidationSeverity
	ConstraintType string
}

func (e ValidationError) String() string {
	return fmt.Sprintf("[%s] %s.%s (row %s): %s", e.Severity, e.TableName, e.FieldName, e.RowKey, e.Message)
}

// ValidationResult collects validation errors.
type ValidationResult struct {
	Errors []ValidationError
}

// NewValidationResult creates a new empty validation result.
func NewValidationResult() *ValidationResult {
	return &ValidationResult{Errors: make([]ValidationError, 0)}
}

// AddError adds a validation error to the result.
func (r *ValidationResult) AddError(err ValidationError) {
	r.Errors = append(r.Errors, err)
}

// IsValid returns true if there are no errors.
func (r *ValidationResult) IsValid() bool {
	return len(r.Errors) == 0
}

// ErrorCount returns the number of errors.
func (r *ValidationResult) ErrorCount() int {
	return len(r.Errors)
}

// Merge combines another validation result into this one.
func (r *ValidationResult) Merge(other *ValidationResult) {
	if other != nil {
		r.Errors = append(r.Errors, other.Errors...)
	}
}

func (r *ValidationResult) String() string {
	if r.IsValid() {
		return "Validation passed"
	}
	var sb strings.Builder
	sb.WriteString(fmt.Sprintf("Validation failed with %d error(s):\n", len(r.Errors)))
	for _, err := range r.Errors {
		sb.WriteString("  - ")
		sb.WriteString(err.String())
		sb.WriteString("\n")
	}
	return sb.String()
}

// ValidationException wraps a validation result as an error.
type ValidationException struct {
	Result *ValidationResult
}

func (e *ValidationException) Error() string {
	return e.Result.String()
}

// NewValidationException creates a new validation exception.
func NewValidationException(result *ValidationResult) *ValidationException {
	return &ValidationException{Result: result}
}

// ============ Validation Helpers ============

// ValidateMaxLength checks if a string's length is within the maximum.
func ValidateMaxLength(value string, maxLen int) bool {
	return len(value) <= maxLen
}

// ValidateMaxLengthPtr checks if an optional string's length is within the maximum.
func ValidateMaxLengthPtr(value *string, maxLen int) bool {
	if value == nil {
		return true
	}
	return len(*value) <= maxLen
}

// ValidateRangeInt checks if an integer is within the specified range.
func ValidateRangeInt[T ~int | ~int8 | ~int16 | ~int32 | ~int64](value T, min, max T) bool {
	return value >= min && value <= max
}

// ValidateRangeUint checks if an unsigned integer is within the specified range.
func ValidateRangeUint[T ~uint | ~uint8 | ~uint16 | ~uint32 | ~uint64](value T, min, max T) bool {
	return value >= min && value <= max
}

// ValidateRangeFloat checks if a float is within the specified range.
func ValidateRangeFloat[T ~float32 | ~float64](value T, min, max T) bool {
	return value >= min && value <= max
}

// ValidateRegex checks if a string matches the specified pattern.
func ValidateRegex(value, pattern string) bool {
	re, err := regexp.Compile(pattern)
	if err != nil {
		return false
	}
	return re.MatchString(value)
}

// ValidateRegexPtr checks if an optional string matches the specified pattern.
func ValidateRegexPtr(value *string, pattern string) bool {
	if value == nil {
		return true
	}
	return ValidateRegex(*value, pattern)
}

// ValidateRequired checks if a pointer value is not nil.
func ValidateRequired[T any](value *T) bool {
	return value != nil
}

// ============ Error Creators ============

// MaxLengthError creates a validation error for max length constraint violation.
func MaxLengthError(tableName, fieldName, rowKey string, maxLen, actualLen int) ValidationError {
	return ValidationError{
		TableName:      tableName,
		FieldName:      fieldName,
		RowKey:         rowKey,
		Message:        fmt.Sprintf("length %d exceeds maximum %d", actualLen, maxLen),
		Severity:       SeverityError,
		ConstraintType: "MaxLength",
	}
}

// RangeError creates a validation error for range constraint violation.
func RangeError[T any](tableName, fieldName, rowKey string, min, max, actual T) ValidationError {
	return ValidationError{
		TableName:      tableName,
		FieldName:      fieldName,
		RowKey:         rowKey,
		Message:        fmt.Sprintf("value %v is outside range [%v, %v]", actual, min, max),
		Severity:       SeverityError,
		ConstraintType: "Range",
	}
}

// RegexError creates a validation error for regex constraint violation.
func RegexError(tableName, fieldName, rowKey, pattern, actual string) ValidationError {
	return ValidationError{
		TableName:      tableName,
		FieldName:      fieldName,
		RowKey:         rowKey,
		Message:        fmt.Sprintf("value '%s' does not match pattern '%s'", actual, pattern),
		Severity:       SeverityError,
		ConstraintType: "Regex",
	}
}

// RequiredError creates a validation error for required field constraint violation.
func RequiredError(tableName, fieldName, rowKey string) ValidationError {
	return ValidationError{
		TableName:      tableName,
		FieldName:      fieldName,
		RowKey:         rowKey,
		Message:        "required field is nil",
		Severity:       SeverityError,
		ConstraintType: "Required",
	}
}

// ForeignKeyError creates a validation error for foreign key constraint violation.
func ForeignKeyError(tableName, fieldName, rowKey, refTable string, refKey interface{}) ValidationError {
	return ValidationError{
		TableName:      tableName,
		FieldName:      fieldName,
		RowKey:         rowKey,
		Message:        fmt.Sprintf("foreign key %v not found in %s", refKey, refTable),
		Severity:       SeverityError,
		ConstraintType: "ForeignKey",
	}
}

// UniqueError creates a validation error for unique constraint violation.
func UniqueError(tableName, fieldName, rowKey string, value interface{}) ValidationError {
	return ValidationError{
		TableName:      tableName,
		FieldName:      fieldName,
		RowKey:         rowKey,
		Message:        fmt.Sprintf("duplicate value '%v' violates unique constraint", value),
		Severity:       SeverityError,
		ConstraintType: "Unique",
	}
}

// ============ Packed Embed Helpers ============

func packValues(sep string, values ...interface{}) string {
	parts := make([]string, len(values))
	for i, value := range values {
		parts[i] = formatPackedValue(value)
	}
	return strings.Join(parts, sep)
}

func formatPackedValue(value interface{}) string {
	switch v := value.(type) {
	case string:
		return v
	case bool:
		return strconv.FormatBool(v)
	case float32:
		return strconv.FormatFloat(float64(v), 'f', -1, 32)
	case float64:
		return strconv.FormatFloat(v, 'f', -1, 64)
	case int:
		return strconv.FormatInt(int64(v), 10)
	case int8:
		return strconv.FormatInt(int64(v), 10)
	case int16:
		return strconv.FormatInt(int64(v), 10)
	case int32:
		return strconv.FormatInt(int64(v), 10)
	case int64:
		return strconv.FormatInt(v, 10)
	case uint:
		return strconv.FormatUint(uint64(v), 10)
	case uint8:
		return strconv.FormatUint(uint64(v), 10)
	case uint16:
		return strconv.FormatUint(uint64(v), 10)
	case uint32:
		return strconv.FormatUint(uint64(v), 10)
	case uint64:
		return strconv.FormatUint(v, 10)
	default:
		return fmt.Sprint(v)
	}
}

func splitPacked(value, sep, typeName string, expected int) ([]string, error) {
	parts := strings.Split(value, sep)
	if len(parts) != expected {
		return nil, fmt.Errorf(
			"expected %d parts but got %d when unpacking %s",
			expected,
			len(parts),
			typeName,
		)
	}
	return parts, nil
}

func parsePackedInt(value, typeName, fieldName string, bitSize int) (int64, error) {
	parsed, err := strconv.ParseInt(value, 10, bitSize)
	if err != nil {
		return 0, fmt.Errorf("failed to parse %s.%s as signed integer: %w", typeName, fieldName, err)
	}
	return parsed, nil
}

func parsePackedUint(value, typeName, fieldName string, bitSize int) (uint64, error) {
	parsed, err := strconv.ParseUint(value, 10, bitSize)
	if err != nil {
		return 0, fmt.Errorf("failed to parse %s.%s as unsigned integer: %w", typeName, fieldName, err)
	}
	return parsed, nil
}

func parsePackedInt8(value, typeName, fieldName string) (int8, error) {
	parsed, err := parsePackedInt(value, typeName, fieldName, 8)
	return int8(parsed), err
}

func parsePackedInt16(value, typeName, fieldName string) (int16, error) {
	parsed, err := parsePackedInt(value, typeName, fieldName, 16)
	return int16(parsed), err
}

func parsePackedInt32(value, typeName, fieldName string) (int32, error) {
	parsed, err := parsePackedInt(value, typeName, fieldName, 32)
	return int32(parsed), err
}

func parsePackedInt64(value, typeName, fieldName string) (int64, error) {
	return parsePackedInt(value, typeName, fieldName, 64)
}

func parsePackedUint8(value, typeName, fieldName string) (uint8, error) {
	parsed, err := parsePackedUint(value, typeName, fieldName, 8)
	return uint8(parsed), err
}

func parsePackedUint16(value, typeName, fieldName string) (uint16, error) {
	parsed, err := parsePackedUint(value, typeName, fieldName, 16)
	return uint16(parsed), err
}

func parsePackedUint32(value, typeName, fieldName string) (uint32, error) {
	parsed, err := parsePackedUint(value, typeName, fieldName, 32)
	return uint32(parsed), err
}

func parsePackedUint64(value, typeName, fieldName string) (uint64, error) {
	return parsePackedUint(value, typeName, fieldName, 64)
}

func parsePackedFloat32(value, typeName, fieldName string) (float32, error) {
	parsed, err := strconv.ParseFloat(value, 32)
	if err != nil {
		return 0, fmt.Errorf("failed to parse %s.%s as float32: %w", typeName, fieldName, err)
	}
	if math.IsNaN(parsed) || math.IsInf(parsed, 0) {
		return 0, fmt.Errorf("failed to parse %s.%s as finite float32", typeName, fieldName)
	}
	return float32(parsed), nil
}

func parsePackedFloat64(value, typeName, fieldName string) (float64, error) {
	parsed, err := strconv.ParseFloat(value, 64)
	if err != nil {
		return 0, fmt.Errorf("failed to parse %s.%s as float64: %w", typeName, fieldName, err)
	}
	if math.IsNaN(parsed) || math.IsInf(parsed, 0) {
		return 0, fmt.Errorf("failed to parse %s.%s as finite float64", typeName, fieldName)
	}
	return parsed, nil
}

func parsePackedBool(value, typeName, fieldName string) (bool, error) {
	parsed, err := strconv.ParseBool(value)
	if err != nil {
		return false, fmt.Errorf("failed to parse %s.%s as bool: %w", typeName, fieldName, err)
	}
	return parsed, nil
}

func unsupportedPackedField(typeName, fieldName, fieldType string) error {
	return fmt.Errorf("unsupported packed field %s.%s type %s", typeName, fieldName, fieldType)
}

// ============ CSV Loading ============

// CsvRow represents a single row from a CSV file.
type CsvRow struct {
	headers map[string]int
	values  []string
}

// Get returns the raw column value and whether the column exists in this row.
func (r *CsvRow) Get(column string) (string, bool) {
	idx, ok := r.headers[column]
	if !ok || idx >= len(r.values) {
		return "", false
	}
	return r.values[idx], true
}

// GetString gets a string value by column name.
func (r *CsvRow) GetString(column string) string {
	if val, ok := r.Get(column); ok {
		return val
	}
	return ""
}

// GetStringPtr gets an optional string value by column name.
func (r *CsvRow) GetStringPtr(column string) *string {
	if idx, ok := r.headers[column]; ok && idx < len(r.values) {
		val := r.values[idx]
		if val == "" {
			return nil
		}
		return &val
	}
	return nil
}

// GetInt32 gets an int32 value by column name.
func (r *CsvRow) GetInt32(column string) int32 {
	if idx, ok := r.headers[column]; ok && idx < len(r.values) {
		val, _ := strconv.ParseInt(r.values[idx], 10, 32)
		return int32(val)
	}
	return 0
}

// GetInt64 gets an int64 value by column name.
func (r *CsvRow) GetInt64(column string) int64 {
	if idx, ok := r.headers[column]; ok && idx < len(r.values) {
		val, _ := strconv.ParseInt(r.values[idx], 10, 64)
		return val
	}
	return 0
}

// GetUint32 gets a uint32 value by column name.
func (r *CsvRow) GetUint32(column string) uint32 {
	if idx, ok := r.headers[column]; ok && idx < len(r.values) {
		val, _ := strconv.ParseUint(r.values[idx], 10, 32)
		return uint32(val)
	}
	return 0
}

// GetUint64 gets a uint64 value by column name.
func (r *CsvRow) GetUint64(column string) uint64 {
	if idx, ok := r.headers[column]; ok && idx < len(r.values) {
		val, _ := strconv.ParseUint(r.values[idx], 10, 64)
		return val
	}
	return 0
}

// GetFloat32 gets a float32 value by column name.
func (r *CsvRow) GetFloat32(column string) float32 {
	if idx, ok := r.headers[column]; ok && idx < len(r.values) {
		val, _ := strconv.ParseFloat(r.values[idx], 32)
		return float32(val)
	}
	return 0
}

// GetFloat64 gets a float64 value by column name.
func (r *CsvRow) GetFloat64(column string) float64 {
	if idx, ok := r.headers[column]; ok && idx < len(r.values) {
		val, _ := strconv.ParseFloat(r.values[idx], 64)
		return val
	}
	return 0
}

// GetBool gets a bool value by column name.
func (r *CsvRow) GetBool(column string) bool {
	if idx, ok := r.headers[column]; ok && idx < len(r.values) {
		val := strings.ToLower(r.values[idx])
		return val == "true" || val == "1" || val == "yes"
	}
	return false
}

// CsvReader reads CSV files with header support.
type CsvReader struct {
	headers map[string]int
	reader  *csv.Reader
	file    *os.File
}

// NewCsvReader creates a new CSV reader from a file path.
func NewCsvReader(path string) (*CsvReader, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}

	reader := csv.NewReader(file)

	// Read header row
	headerRow, err := reader.Read()
	if err != nil {
		file.Close()
		return nil, err
	}

	headers := make(map[string]int)
	for i, h := range headerRow {
		headers[strings.TrimSpace(h)] = i
	}

	return &CsvReader{
		headers: headers,
		reader:  reader,
		file:    file,
	}, nil
}

// ReadRow reads the next row from the CSV file.
func (r *CsvReader) ReadRow() (*CsvRow, error) {
	values, err := r.reader.Read()
	if err != nil {
		return nil, err
	}
	return &CsvRow{headers: r.headers, values: values}, nil
}

// ReadAll reads all remaining rows from the CSV file.
func (r *CsvReader) ReadAll() ([]*CsvRow, error) {
	var rows []*CsvRow
	for {
		row, err := r.ReadRow()
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, err
		}
		rows = append(rows, row)
	}
	return rows, nil
}

// Close closes the CSV reader.
func (r *CsvReader) Close() error {
	return r.file.Close()
}

// ============ JSON Loading ============

// LoadJSON loads a JSON file into the given target.
func LoadJSON[T any](path string, target *T) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	return json.Unmarshal(data, target)
}

// LoadJSONSlice loads a JSON array file into a slice.
func LoadJSONSlice[T any](path string) ([]T, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var result []T
	if err := json.Unmarshal(data, &result); err != nil {
		return nil, err
	}
	return result, nil
}

// ============ Binary I/O ============

// BinaryReader provides binary reading utilities.
type BinaryReader struct {
	reader io.Reader
	order  binary.ByteOrder
}

// NewBinaryReader creates a new binary reader with little-endian byte order.
func NewBinaryReader(reader io.Reader) *BinaryReader {
	return &BinaryReader{reader: reader, order: binary.LittleEndian}
}

// ReadUint8 reads a uint8.
func (r *BinaryReader) ReadUint8() (uint8, error) {
	var val uint8
	err := binary.Read(r.reader, r.order, &val)
	return val, err
}

// ReadUint16 reads a uint16.
func (r *BinaryReader) ReadUint16() (uint16, error) {
	var val uint16
	err := binary.Read(r.reader, r.order, &val)
	return val, err
}

// ReadUint32 reads a uint32.
func (r *BinaryReader) ReadUint32() (uint32, error) {
	var val uint32
	err := binary.Read(r.reader, r.order, &val)
	return val, err
}

// ReadUint64 reads a uint64.
func (r *BinaryReader) ReadUint64() (uint64, error) {
	var val uint64
	err := binary.Read(r.reader, r.order, &val)
	return val, err
}

// ReadInt8 reads an int8.
func (r *BinaryReader) ReadInt8() (int8, error) {
	var val int8
	err := binary.Read(r.reader, r.order, &val)
	return val, err
}

// ReadInt16 reads an int16.
func (r *BinaryReader) ReadInt16() (int16, error) {
	var val int16
	err := binary.Read(r.reader, r.order, &val)
	return val, err
}

// ReadInt32 reads an int32.
func (r *BinaryReader) ReadInt32() (int32, error) {
	var val int32
	err := binary.Read(r.reader, r.order, &val)
	return val, err
}

// ReadInt64 reads an int64.
func (r *BinaryReader) ReadInt64() (int64, error) {
	var val int64
	err := binary.Read(r.reader, r.order, &val)
	return val, err
}

// ReadFloat32 reads a float32.
func (r *BinaryReader) ReadFloat32() (float32, error) {
	var val float32
	err := binary.Read(r.reader, r.order, &val)
	return val, err
}

// ReadFloat64 reads a float64.
func (r *BinaryReader) ReadFloat64() (float64, error) {
	var val float64
	err := binary.Read(r.reader, r.order, &val)
	return val, err
}

// ReadBool reads a bool (as uint8).
func (r *BinaryReader) ReadBool() (bool, error) {
	val, err := r.ReadUint8()
	return val != 0, err
}

// ReadString reads a length-prefixed string.
func (r *BinaryReader) ReadString() (string, error) {
	length, err := r.ReadUint32()
	if err != nil {
		return "", err
	}
	bytes := make([]byte, length)
	_, err = io.ReadFull(r.reader, bytes)
	if err != nil {
		return "", err
	}
	return string(bytes), nil
}

// ReadBytes reads a length-prefixed byte slice.
func (r *BinaryReader) ReadBytes() ([]byte, error) {
	length, err := r.ReadUint32()
	if err != nil {
		return nil, err
	}
	bytes := make([]byte, length)
	_, err = io.ReadFull(r.reader, bytes)
	if err != nil {
		return nil, err
	}
	return bytes, nil
}

// BinaryWriter provides binary writing utilities.
type BinaryWriter struct {
	writer io.Writer
	order  binary.ByteOrder
}

// NewBinaryWriter creates a new binary writer with little-endian byte order.
func NewBinaryWriter(writer io.Writer) *BinaryWriter {
	return &BinaryWriter{writer: writer, order: binary.LittleEndian}
}

// WriteUint8 writes a uint8.
func (w *BinaryWriter) WriteUint8(val uint8) error {
	return binary.Write(w.writer, w.order, val)
}

// WriteUint16 writes a uint16.
func (w *BinaryWriter) WriteUint16(val uint16) error {
	return binary.Write(w.writer, w.order, val)
}

// WriteUint32 writes a uint32.
func (w *BinaryWriter) WriteUint32(val uint32) error {
	return binary.Write(w.writer, w.order, val)
}

// WriteUint64 writes a uint64.
func (w *BinaryWriter) WriteUint64(val uint64) error {
	return binary.Write(w.writer, w.order, val)
}

// WriteInt8 writes an int8.
func (w *BinaryWriter) WriteInt8(val int8) error {
	return binary.Write(w.writer, w.order, val)
}

// WriteInt16 writes an int16.
func (w *BinaryWriter) WriteInt16(val int16) error {
	return binary.Write(w.writer, w.order, val)
}

// WriteInt32 writes an int32.
func (w *BinaryWriter) WriteInt32(val int32) error {
	return binary.Write(w.writer, w.order, val)
}

// WriteInt64 writes an int64.
func (w *BinaryWriter) WriteInt64(val int64) error {
	return binary.Write(w.writer, w.order, val)
}

// WriteFloat32 writes a float32.
func (w *BinaryWriter) WriteFloat32(val float32) error {
	return binary.Write(w.writer, w.order, val)
}

// WriteFloat64 writes a float64.
func (w *BinaryWriter) WriteFloat64(val float64) error {
	return binary.Write(w.writer, w.order, val)
}

// WriteBool writes a bool (as uint8).
func (w *BinaryWriter) WriteBool(val bool) error {
	var b uint8
	if val {
		b = 1
	}
	return w.WriteUint8(b)
}

// WriteString writes a length-prefixed string.
func (w *BinaryWriter) WriteString(val string) error {
	if err := w.WriteUint32(uint32(len(val))); err != nil {
		return err
	}
	_, err := w.writer.Write([]byte(val))
	return err
}

// WriteBytes writes a length-prefixed byte slice.
func (w *BinaryWriter) WriteBytes(val []byte) error {
	if err := w.WriteUint32(uint32(len(val))); err != nil {
		return err
	}
	_, err := w.writer.Write(val)
	return err
}

// WriteRaw writes raw bytes without a length prefix.
func (w *BinaryWriter) WriteRaw(val []byte) error {
	_, err := w.writer.Write(val)
	return err
}

// ============ BinaryRef v2 ============

var binaryRefMagic = []byte{0x50, 0x47, 0x42, 0x52, 0x45, 0x46, 0x31, 0x00}

const binaryRefVersion int32 = 2

// BinaryDocumentOwner owns the bytes shared by lazy BinaryRef rows.
type BinaryDocumentOwner struct {
	Bytes []byte
}

// NewBinaryDocumentOwner creates a binary document owner.
func NewBinaryDocumentOwner(input []byte) *BinaryDocumentOwner {
	return &BinaryDocumentOwner{Bytes: input}
}

// BinaryRefCursor reads a BinaryRef document from a byte slice.
type BinaryRefCursor struct {
	bytes  []byte
	offset int
}

// NewBinaryRefCursor creates a cursor at offset zero.
func NewBinaryRefCursor(input []byte) *BinaryRefCursor {
	return &BinaryRefCursor{bytes: input}
}

// Position returns the current cursor offset.
func (r *BinaryRefCursor) Position() int { return r.offset }

// Skip advances the cursor by length bytes.
func (r *BinaryRefCursor) Skip(length int) error {
	if err := BinaryRefCheckRange(r.bytes, r.offset, length); err != nil {
		return err
	}
	r.offset += length
	return nil
}

func (r *BinaryRefCursor) read(length int) ([]byte, error) {
	if err := BinaryRefCheckRange(r.bytes, r.offset, length); err != nil {
		return nil, err
	}
	out := r.bytes[r.offset : r.offset+length]
	r.offset += length
	return out, nil
}

func (r *BinaryRefCursor) ReadUint8() (uint8, error) {
	b, err := r.read(1)
	if err != nil {
		return 0, err
	}
	return b[0], nil
}

func (r *BinaryRefCursor) ReadInt8() (int8, error) {
	v, err := r.ReadUint8()
	return int8(v), err
}

func (r *BinaryRefCursor) ReadUint16() (uint16, error) {
	b, err := r.read(2)
	if err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint16(b), nil
}

func (r *BinaryRefCursor) ReadInt16() (int16, error) {
	v, err := r.ReadUint16()
	return int16(v), err
}

func (r *BinaryRefCursor) ReadUint32() (uint32, error) {
	b, err := r.read(4)
	if err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint32(b), nil
}

func (r *BinaryRefCursor) ReadInt32() (int32, error) {
	v, err := r.ReadUint32()
	return int32(v), err
}

func (r *BinaryRefCursor) ReadUint64() (uint64, error) {
	b, err := r.read(8)
	if err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint64(b), nil
}

func (r *BinaryRefCursor) ReadInt64() (int64, error) {
	v, err := r.ReadUint64()
	return int64(v), err
}

func (r *BinaryRefCursor) ReadFloat32() (float32, error) {
	v, err := r.ReadUint32()
	return math.Float32frombits(v), err
}

func (r *BinaryRefCursor) ReadFloat64() (float64, error) {
	v, err := r.ReadUint64()
	return math.Float64frombits(v), err
}

func (r *BinaryRefCursor) ReadBool() (bool, error) {
	v, err := r.ReadUint8()
	return v != 0, err
}

func (r *BinaryRefCursor) ReadBytes() ([]byte, error) {
	length, err := r.ReadInt32()
	if err != nil {
		return nil, err
	}
	if length < 0 {
		return nil, fmt.Errorf("negative binary payload length")
	}
	return r.read(int(length))
}

func (r *BinaryRefCursor) ReadString() (string, error) {
	b, err := r.ReadBytes()
	if err != nil {
		return "", err
	}
	return string(b), nil
}

// BinaryRefWriteHeader writes the BinaryRef v2 document header.
func BinaryRefWriteHeader(writer *BinaryWriter) error {
	if err := writer.WriteRaw(binaryRefMagic); err != nil {
		return err
	}
	return writer.WriteInt32(binaryRefVersion)
}

// BinaryRefReadHeader validates the BinaryRef v2 document header.
func BinaryRefReadHeader(reader *BinaryRefCursor) error {
	magic, err := reader.read(len(binaryRefMagic))
	if err != nil {
		return err
	}
	if !bytes.Equal(magic, binaryRefMagic) {
		return fmt.Errorf("invalid PolyGen binary ref header")
	}
	version, err := reader.ReadInt32()
	if err != nil {
		return err
	}
	if version != binaryRefVersion {
		return fmt.Errorf("unsupported PolyGen binary ref version: %d", version)
	}
	return nil
}

// BinaryRefCheckRange validates that [offset, offset+length) is inside buffer.
func BinaryRefCheckRange(buffer []byte, offset int, length int) error {
	if offset < 0 || length < 0 || offset > len(buffer) || length > len(buffer)-offset {
		return fmt.Errorf("binary ref offset is outside the document")
	}
	return nil
}

// BinaryRefGetFieldOffset returns the absolute offset of a field or -1 if absent.
func BinaryRefGetFieldOffset(buffer []byte, rowOffset int, fieldIndex int) (int, error) {
	if err := BinaryRefCheckRange(buffer, rowOffset, 4); err != nil {
		return -1, err
	}
	fieldCount := int(int32(binary.LittleEndian.Uint32(buffer[rowOffset : rowOffset+4])))
	if fieldCount < 0 {
		return -1, fmt.Errorf("negative binary field count")
	}
	if fieldIndex < 0 || fieldIndex >= fieldCount {
		return -1, nil
	}
	tableOffset := rowOffset + 4
	if err := BinaryRefCheckRange(buffer, tableOffset, fieldCount*4); err != nil {
		return -1, err
	}
	relative := int(int32(binary.LittleEndian.Uint32(buffer[tableOffset+fieldIndex*4 : tableOffset+fieldIndex*4+4])))
	if relative < 0 {
		return -1, nil
	}
	absolute := rowOffset + relative
	if err := BinaryRefCheckRange(buffer, absolute, 0); err != nil {
		return -1, err
	}
	return absolute, nil
}

// BinaryRefRequireFieldOffset returns a field offset or an error if it is absent.
func BinaryRefRequireFieldOffset(buffer []byte, rowOffset int, fieldIndex int) (int, error) {
	offset, err := BinaryRefGetFieldOffset(buffer, rowOffset, fieldIndex)
	if err != nil {
		return -1, err
	}
	if offset < 0 {
		return -1, fmt.Errorf("missing required binary field at index %d", fieldIndex)
	}
	return offset, nil
}

func BinaryRefReadBytesAt(buffer []byte, offset int) ([]byte, error) {
	if err := BinaryRefCheckRange(buffer, offset, 4); err != nil {
		return nil, err
	}
	length := int(int32(binary.LittleEndian.Uint32(buffer[offset : offset+4])))
	if length < 0 {
		return nil, fmt.Errorf("negative binary payload length")
	}
	payloadOffset := offset + 4
	if err := BinaryRefCheckRange(buffer, payloadOffset, length); err != nil {
		return nil, err
	}
	return buffer[payloadOffset : payloadOffset+length], nil
}

func BinaryRefReadStringAt(buffer []byte, offset int) (string, error) {
	b, err := BinaryRefReadBytesAt(buffer, offset)
	if err != nil {
		return "", err
	}
	return string(b), nil
}

func BinaryRefReadBoolAt(buffer []byte, offset int) (bool, error) {
	v, err := BinaryRefReadUint8At(buffer, offset)
	return v != 0, err
}

func BinaryRefReadUint8At(buffer []byte, offset int) (uint8, error) {
	if err := BinaryRefCheckRange(buffer, offset, 1); err != nil {
		return 0, err
	}
	return buffer[offset], nil
}

func BinaryRefReadInt8At(buffer []byte, offset int) (int8, error) {
	v, err := BinaryRefReadUint8At(buffer, offset)
	return int8(v), err
}

func BinaryRefReadUint16At(buffer []byte, offset int) (uint16, error) {
	if err := BinaryRefCheckRange(buffer, offset, 2); err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint16(buffer[offset : offset+2]), nil
}

func BinaryRefReadInt16At(buffer []byte, offset int) (int16, error) {
	v, err := BinaryRefReadUint16At(buffer, offset)
	return int16(v), err
}

func BinaryRefReadUint32At(buffer []byte, offset int) (uint32, error) {
	if err := BinaryRefCheckRange(buffer, offset, 4); err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint32(buffer[offset : offset+4]), nil
}

func BinaryRefReadInt32At(buffer []byte, offset int) (int32, error) {
	v, err := BinaryRefReadUint32At(buffer, offset)
	return int32(v), err
}

func BinaryRefReadUint64At(buffer []byte, offset int) (uint64, error) {
	if err := BinaryRefCheckRange(buffer, offset, 8); err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint64(buffer[offset : offset+8]), nil
}

func BinaryRefReadInt64At(buffer []byte, offset int) (int64, error) {
	v, err := BinaryRefReadUint64At(buffer, offset)
	return int64(v), err
}

func BinaryRefReadFloat32At(buffer []byte, offset int) (float32, error) {
	v, err := BinaryRefReadUint32At(buffer, offset)
	return math.Float32frombits(v), err
}

func BinaryRefReadFloat64At(buffer []byte, offset int) (float64, error) {
	v, err := BinaryRefReadUint64At(buffer, offset)
	return math.Float64frombits(v), err
}

// BinaryRefRowBuilder builds a lazy row frame with field offsets.
type BinaryRefRowBuilder struct {
	fields [][]byte
}

// NewBinaryRefRowBuilder creates a row builder for fieldCount fields.
func NewBinaryRefRowBuilder(fieldCount int) (*BinaryRefRowBuilder, error) {
	if fieldCount < 0 {
		return nil, fmt.Errorf("negative binary field count")
	}
	return &BinaryRefRowBuilder{fields: make([][]byte, fieldCount)}, nil
}

// SetField writes a field payload into the row builder.
func (b *BinaryRefRowBuilder) SetField(index int, write func(*BinaryWriter) error) error {
	if index < 0 || index >= len(b.fields) {
		return fmt.Errorf("binary field index is outside the row")
	}
	var buf bytes.Buffer
	writer := NewBinaryWriter(&buf)
	if err := write(writer); err != nil {
		return err
	}
	b.fields[index] = buf.Bytes()
	return nil
}

// Bytes serializes the row frame.
func (b *BinaryRefRowBuilder) Bytes() ([]byte, error) {
	var buf bytes.Buffer
	writer := NewBinaryWriter(&buf)
	if err := writer.WriteInt32(int32(len(b.fields))); err != nil {
		return nil, err
	}
	cursor := 4 + len(b.fields)*4
	for _, field := range b.fields {
		if field == nil {
			if err := writer.WriteInt32(-1); err != nil {
				return nil, err
			}
		} else {
			if err := writer.WriteInt32(int32(cursor)); err != nil {
				return nil, err
			}
			cursor += len(field)
		}
	}
	for _, field := range b.fields {
		if field != nil {
			if err := writer.WriteRaw(field); err != nil {
				return nil, err
			}
		}
	}
	return buf.Bytes(), nil
}

// ============ Index Types ============

// UniqueIndex provides O(1) lookup by a unique key.
type UniqueIndex[K comparable, V any] struct {
	data map[K]V
}

// NewUniqueIndex creates a new unique index.
func NewUniqueIndex[K comparable, V any]() *UniqueIndex[K, V] {
	return &UniqueIndex[K, V]{data: make(map[K]V)}
}

// Insert adds a key-value pair to the index.
func (idx *UniqueIndex[K, V]) Insert(key K, value V) {
	idx.data[key] = value
}

// Get retrieves a value by key, returning nil if not found.
func (idx *UniqueIndex[K, V]) Get(key K) (V, bool) {
	val, ok := idx.data[key]
	return val, ok
}

// Clear removes all entries from the index.
func (idx *UniqueIndex[K, V]) Clear() {
	idx.data = make(map[K]V)
}

// GroupIndex provides O(1) lookup for multiple values by key.
type GroupIndex[K comparable, V any] struct {
	data map[K][]V
}

// NewGroupIndex creates a new group index.
func NewGroupIndex[K comparable, V any]() *GroupIndex[K, V] {
	return &GroupIndex[K, V]{data: make(map[K][]V)}
}

// Add adds a value to the group for the given key.
func (idx *GroupIndex[K, V]) Add(key K, value V) {
	idx.data[key] = append(idx.data[key], value)
}

// Get retrieves all values for a key.
func (idx *GroupIndex[K, V]) Get(key K) []V {
	if vals, ok := idx.data[key]; ok {
		return vals
	}
	return nil
}

// Clear removes all entries from the index.
func (idx *GroupIndex[K, V]) Clear() {
	idx.data = make(map[K][]V)
}
