// Package polygen provides common utilities for PolyGen generated code.
// This file contains validation, loading, and binary I/O support.
package polygen

import (
	"encoding/binary"
	"encoding/csv"
	"encoding/json"
	"fmt"
	"io"
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

// ============ CSV Loading ============

// CsvRow represents a single row from a CSV file.
type CsvRow struct {
	headers map[string]int
	values  []string
}

// GetString gets a string value by column name.
func (r *CsvRow) GetString(column string) string {
	if idx, ok := r.headers[column]; ok && idx < len(r.values) {
		return r.values[idx]
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
