// PolyGen C++ Support Library
// Auto-generated support utilities for PolyGen generated code
// Binary format is compatible with C# and Rust implementations

#pragma once

#include <cstdint>
#include <string>
#include <vector>
#include <optional>
#include <stdexcept>
#include <cstring>
#include <fstream>
#include <sstream>
#include <memory>
#include <type_traits>
#include <functional>
#include <unordered_map>
#include <map>
#include <deque>

namespace polygen {

// ============================================================================
// Binary I/O Utilities
// ============================================================================

// Binary reader for little-endian data
class BinaryReader {
public:
    explicit BinaryReader(std::istream& stream) : stream_(stream) {}
    explicit BinaryReader(const std::vector<uint8_t>& data)
        : owned_stream_(new std::istringstream(
            std::string(reinterpret_cast<const char*>(data.data()), data.size()),
            std::ios::binary)),
          stream_(*owned_stream_) {}

    // Primitive reads
    uint8_t read_u8() {
        uint8_t value;
        stream_.read(reinterpret_cast<char*>(&value), sizeof(value));
        check_read();
        return value;
    }

    uint16_t read_u16() {
        uint16_t value;
        stream_.read(reinterpret_cast<char*>(&value), sizeof(value));
        check_read();
        return le_to_native(value);
    }

    uint32_t read_u32() {
        uint32_t value;
        stream_.read(reinterpret_cast<char*>(&value), sizeof(value));
        check_read();
        return le_to_native(value);
    }

    uint64_t read_u64() {
        uint64_t value;
        stream_.read(reinterpret_cast<char*>(&value), sizeof(value));
        check_read();
        return le_to_native(value);
    }

    int8_t read_i8() { return static_cast<int8_t>(read_u8()); }
    int16_t read_i16() { return static_cast<int16_t>(read_u16()); }
    int32_t read_i32() { return static_cast<int32_t>(read_u32()); }
    int64_t read_i64() { return static_cast<int64_t>(read_u64()); }

    float read_f32() {
        uint32_t bits = read_u32();
        float value;
        std::memcpy(&value, &bits, sizeof(value));
        return value;
    }

    double read_f64() {
        uint64_t bits = read_u64();
        double value;
        std::memcpy(&value, &bits, sizeof(value));
        return value;
    }

    bool read_bool() { return read_u8() != 0; }

    // String: u32 length + UTF-8 bytes
    std::string read_string() {
        uint32_t len = read_u32();
        if (len == 0) return "";
        std::string value(len, '\0');
        stream_.read(value.data(), len);
        check_read();
        return value;
    }

    // Optional<T>: u8 flag + T if present
    template<typename T, typename ReadFunc>
    std::optional<T> read_optional(ReadFunc read_func) {
        uint8_t has_value = read_u8();
        if (has_value == 0) return std::nullopt;
        return read_func(*this);
    }

    // Optional string
    std::optional<std::string> read_optional_string() {
        uint8_t has_value = read_u8();
        if (has_value == 0) return std::nullopt;
        return read_string();
    }

    // Vector<T>: u32 count + T items
    template<typename T, typename ReadFunc>
    std::vector<T> read_vector(ReadFunc read_func) {
        uint32_t count = read_u32();
        std::vector<T> result;
        result.reserve(count);
        for (uint32_t i = 0; i < count; ++i) {
            result.push_back(read_func(*this));
        }
        return result;
    }

    // Bytes: u32 length + raw bytes
    std::vector<uint8_t> read_bytes() {
        uint32_t len = read_u32();
        std::vector<uint8_t> result(len);
        if (len > 0) {
            stream_.read(reinterpret_cast<char*>(result.data()), len);
            check_read();
        }
        return result;
    }

    // Enum: i32 value
    template<typename E>
    E read_enum() {
        static_assert(std::is_enum_v<E>, "Type must be an enum");
        return static_cast<E>(read_i32());
    }

private:
    std::unique_ptr<std::istringstream> owned_stream_;
    std::istream& stream_;

    void check_read() {
        if (!stream_) {
            throw std::runtime_error("Binary read error: unexpected end of stream");
        }
    }

    template<typename T>
    static T le_to_native(T value) {
        // Assuming little-endian architecture (most common)
        // For big-endian, byte swap would be needed
        return value;
    }
};

// Binary writer for little-endian data
class BinaryWriter {
public:
    explicit BinaryWriter(std::ostream& stream) : stream_(stream) {}
    explicit BinaryWriter(std::vector<uint8_t>& data)
        : vec_(&data), owned_stream_(new std::ostringstream(std::ios::binary)),
          stream_(*owned_stream_) {}

    ~BinaryWriter() {
        flush();
    }

    void flush() {
        if (vec_ && owned_stream_) {
            std::string str = owned_stream_->str();
            vec_->assign(str.begin(), str.end());
        }
    }

    // Primitive writes
    void write_u8(uint8_t value) {
        stream_.write(reinterpret_cast<const char*>(&value), sizeof(value));
    }

    void write_u16(uint16_t value) {
        value = native_to_le(value);
        stream_.write(reinterpret_cast<const char*>(&value), sizeof(value));
    }

    void write_u32(uint32_t value) {
        value = native_to_le(value);
        stream_.write(reinterpret_cast<const char*>(&value), sizeof(value));
    }

    void write_u64(uint64_t value) {
        value = native_to_le(value);
        stream_.write(reinterpret_cast<const char*>(&value), sizeof(value));
    }

    void write_i8(int8_t value) { write_u8(static_cast<uint8_t>(value)); }
    void write_i16(int16_t value) { write_u16(static_cast<uint16_t>(value)); }
    void write_i32(int32_t value) { write_u32(static_cast<uint32_t>(value)); }
    void write_i64(int64_t value) { write_u64(static_cast<uint64_t>(value)); }

    void write_f32(float value) {
        uint32_t bits;
        std::memcpy(&bits, &value, sizeof(bits));
        write_u32(bits);
    }

    void write_f64(double value) {
        uint64_t bits;
        std::memcpy(&bits, &value, sizeof(bits));
        write_u64(bits);
    }

    void write_bool(bool value) { write_u8(value ? 1 : 0); }

    // String: u32 length + UTF-8 bytes
    void write_string(const std::string& value) {
        write_u32(static_cast<uint32_t>(value.size()));
        if (!value.empty()) {
            stream_.write(value.data(), value.size());
        }
    }

    // Optional<T>: u8 flag + T if present
    template<typename T, typename WriteFunc>
    void write_optional(const std::optional<T>& value, WriteFunc write_func) {
        if (value.has_value()) {
            write_u8(1);
            write_func(*this, *value);
        } else {
            write_u8(0);
        }
    }

    // Optional string
    void write_optional_string(const std::optional<std::string>& value) {
        if (value.has_value()) {
            write_u8(1);
            write_string(*value);
        } else {
            write_u8(0);
        }
    }

    // Vector<T>: u32 count + T items
    template<typename T, typename WriteFunc>
    void write_vector(const std::vector<T>& items, WriteFunc write_func) {
        write_u32(static_cast<uint32_t>(items.size()));
        for (const auto& item : items) {
            write_func(*this, item);
        }
    }

    // Bytes: u32 length + raw bytes
    void write_bytes(const std::vector<uint8_t>& value) {
        write_u32(static_cast<uint32_t>(value.size()));
        if (!value.empty()) {
            stream_.write(reinterpret_cast<const char*>(value.data()), value.size());
        }
    }

    // Enum: i32 value
    template<typename E>
    void write_enum(E value) {
        static_assert(std::is_enum_v<E>, "Type must be an enum");
        write_i32(static_cast<int32_t>(value));
    }

private:
    std::vector<uint8_t>* vec_ = nullptr;
    std::unique_ptr<std::ostringstream> owned_stream_;
    std::ostream& stream_;

    template<typename T>
    static T native_to_le(T value) {
        // Assuming little-endian architecture
        return value;
    }
};

// ============================================================================
// CSV Utilities
// ============================================================================

class CsvReader {
public:
    explicit CsvReader(std::istream& stream, char delimiter = ',')
        : stream_(stream), delimiter_(delimiter) {
        // Read header
        if (std::getline(stream_, current_line_)) {
            headers_ = parse_line(current_line_);
            for (size_t i = 0; i < headers_.size(); ++i) {
                header_index_[headers_[i]] = i;
            }
        }
    }

    bool next() {
        if (std::getline(stream_, current_line_)) {
            current_values_ = parse_line(current_line_);
            return true;
        }
        return false;
    }

    std::string get(const std::string& column) const {
        auto it = header_index_.find(column);
        if (it == header_index_.end()) return "";
        if (it->second >= current_values_.size()) return "";
        return current_values_[it->second];
    }

    std::string get(size_t index) const {
        if (index >= current_values_.size()) return "";
        return current_values_[index];
    }

    const std::vector<std::string>& headers() const { return headers_; }

    // Parse helpers
    static int32_t parse_i32(const std::string& s) {
        if (s.empty()) return 0;
        return std::stoi(s);
    }

    static int64_t parse_i64(const std::string& s) {
        if (s.empty()) return 0;
        return std::stoll(s);
    }

    static uint32_t parse_u32(const std::string& s) {
        if (s.empty()) return 0;
        return static_cast<uint32_t>(std::stoul(s));
    }

    static uint64_t parse_u64(const std::string& s) {
        if (s.empty()) return 0;
        return std::stoull(s);
    }

    static float parse_f32(const std::string& s) {
        if (s.empty()) return 0.0f;
        return std::stof(s);
    }

    static double parse_f64(const std::string& s) {
        if (s.empty()) return 0.0;
        return std::stod(s);
    }

    static bool parse_bool(const std::string& s) {
        if (s.empty()) return false;
        return s == "true" || s == "1" || s == "True" || s == "TRUE";
    }

    template<typename E>
    static E parse_enum(const std::string& s) {
        return static_cast<E>(parse_i32(s));
    }

private:
    std::istream& stream_;
    char delimiter_;
    std::string current_line_;
    std::vector<std::string> headers_;
    std::vector<std::string> current_values_;
    std::unordered_map<std::string, size_t> header_index_;

    std::vector<std::string> parse_line(const std::string& line) {
        std::vector<std::string> result;
        std::string current;
        bool in_quotes = false;

        for (size_t i = 0; i < line.size(); ++i) {
            char c = line[i];
            if (c == '"') {
                if (in_quotes && i + 1 < line.size() && line[i + 1] == '"') {
                    current += '"';
                    ++i;
                } else {
                    in_quotes = !in_quotes;
                }
            } else if (c == delimiter_ && !in_quotes) {
                result.push_back(current);
                current.clear();
            } else {
                current += c;
            }
        }
        result.push_back(current);
        return result;
    }
};

// ============================================================================
// JSON Utilities (minimal implementation)
// ============================================================================

// For full JSON support, consider using nlohmann/json or similar library
// This is a minimal implementation for basic use cases

class JsonValue;
using JsonObject = std::map<std::string, JsonValue>;
using JsonArray = std::vector<JsonValue>;

class JsonValue {
public:
    enum class Type { Null, Bool, Number, String, Array, Object };

    JsonValue() : type_(Type::Null) {}
    JsonValue(bool b) : type_(Type::Bool), bool_val_(b) {}
    JsonValue(int32_t n) : type_(Type::Number), num_val_(n) {}
    JsonValue(int64_t n) : type_(Type::Number), num_val_(static_cast<double>(n)) {}
    JsonValue(double n) : type_(Type::Number), num_val_(n) {}
    JsonValue(const std::string& s) : type_(Type::String), str_val_(s) {}
    JsonValue(const char* s) : type_(Type::String), str_val_(s) {}
    JsonValue(const JsonArray& arr) : type_(Type::Array), arr_val_(arr) {}
    JsonValue(const JsonObject& obj) : type_(Type::Object), obj_val_(obj) {}

    Type type() const { return type_; }
    bool is_null() const { return type_ == Type::Null; }
    bool is_bool() const { return type_ == Type::Bool; }
    bool is_number() const { return type_ == Type::Number; }
    bool is_string() const { return type_ == Type::String; }
    bool is_array() const { return type_ == Type::Array; }
    bool is_object() const { return type_ == Type::Object; }

    bool as_bool() const { return bool_val_; }
    double as_number() const { return num_val_; }
    int32_t as_i32() const { return static_cast<int32_t>(num_val_); }
    int64_t as_i64() const { return static_cast<int64_t>(num_val_); }
    const std::string& as_string() const { return str_val_; }
    const JsonArray& as_array() const { return arr_val_; }
    const JsonObject& as_object() const { return obj_val_; }

    JsonValue& operator[](const std::string& key) { return obj_val_[key]; }
    const JsonValue& operator[](const std::string& key) const {
        static JsonValue null_val;
        auto it = obj_val_.find(key);
        return it != obj_val_.end() ? it->second : null_val;
    }

    bool has(const std::string& key) const {
        return obj_val_.find(key) != obj_val_.end();
    }

private:
    Type type_;
    bool bool_val_ = false;
    double num_val_ = 0.0;
    std::string str_val_;
    JsonArray arr_val_;
    JsonObject obj_val_;
};

// Simple JSON parser (supports basic JSON)
class JsonParser {
public:
    static JsonValue parse(const std::string& json) {
        size_t pos = 0;
        return parse_value(json, pos);
    }

private:
    static void skip_whitespace(const std::string& json, size_t& pos) {
        while (pos < json.size() && std::isspace(json[pos])) ++pos;
    }

    static JsonValue parse_value(const std::string& json, size_t& pos) {
        skip_whitespace(json, pos);
        if (pos >= json.size()) return JsonValue();

        char c = json[pos];
        if (c == '"') return parse_string(json, pos);
        if (c == '{') return parse_object(json, pos);
        if (c == '[') return parse_array(json, pos);
        if (c == 't' || c == 'f') return parse_bool(json, pos);
        if (c == 'n') return parse_null(json, pos);
        if (c == '-' || std::isdigit(c)) return parse_number(json, pos);
        return JsonValue();
    }

    static JsonValue parse_string(const std::string& json, size_t& pos) {
        ++pos; // skip opening quote
        std::string result;
        while (pos < json.size() && json[pos] != '"') {
            if (json[pos] == '\\' && pos + 1 < json.size()) {
                ++pos;
                switch (json[pos]) {
                    case '"': result += '"'; break;
                    case '\\': result += '\\'; break;
                    case 'n': result += '\n'; break;
                    case 'r': result += '\r'; break;
                    case 't': result += '\t'; break;
                    default: result += json[pos];
                }
            } else {
                result += json[pos];
            }
            ++pos;
        }
        if (pos < json.size()) ++pos; // skip closing quote
        return JsonValue(result);
    }

    static JsonValue parse_number(const std::string& json, size_t& pos) {
        size_t start = pos;
        if (json[pos] == '-') ++pos;
        while (pos < json.size() && (std::isdigit(json[pos]) || json[pos] == '.' || json[pos] == 'e' || json[pos] == 'E' || json[pos] == '+' || json[pos] == '-')) {
            ++pos;
        }
        return JsonValue(std::stod(json.substr(start, pos - start)));
    }

    static JsonValue parse_bool(const std::string& json, size_t& pos) {
        if (json.substr(pos, 4) == "true") { pos += 4; return JsonValue(true); }
        if (json.substr(pos, 5) == "false") { pos += 5; return JsonValue(false); }
        return JsonValue();
    }

    static JsonValue parse_null(const std::string& json, size_t& pos) {
        if (json.substr(pos, 4) == "null") { pos += 4; }
        return JsonValue();
    }

    static JsonValue parse_array(const std::string& json, size_t& pos) {
        ++pos; // skip [
        JsonArray arr;
        skip_whitespace(json, pos);
        if (pos < json.size() && json[pos] == ']') { ++pos; return JsonValue(arr); }
        while (pos < json.size()) {
            arr.push_back(parse_value(json, pos));
            skip_whitespace(json, pos);
            if (pos < json.size() && json[pos] == ',') { ++pos; continue; }
            if (pos < json.size() && json[pos] == ']') { ++pos; break; }
        }
        return JsonValue(arr);
    }

    static JsonValue parse_object(const std::string& json, size_t& pos) {
        ++pos; // skip {
        JsonObject obj;
        skip_whitespace(json, pos);
        if (pos < json.size() && json[pos] == '}') { ++pos; return JsonValue(obj); }
        while (pos < json.size()) {
            skip_whitespace(json, pos);
            auto key = parse_string(json, pos);
            skip_whitespace(json, pos);
            if (pos < json.size() && json[pos] == ':') ++pos;
            obj[key.as_string()] = parse_value(json, pos);
            skip_whitespace(json, pos);
            if (pos < json.size() && json[pos] == ',') { ++pos; continue; }
            if (pos < json.size() && json[pos] == '}') { ++pos; break; }
        }
        return JsonValue(obj);
    }
};

// ============================================================================
// Container Index Types
// ============================================================================

// Unique index: maps a single key to a single row
template<typename K, typename V>
class UniqueIndex {
public:
    void insert(const K& key, V* value) {
        index_[key] = value;
    }

    V* get(const K& key) const {
        auto it = index_.find(key);
        return it != index_.end() ? it->second : nullptr;
    }

    bool contains(const K& key) const {
        return index_.find(key) != index_.end();
    }

    void clear() { index_.clear(); }

private:
    std::unordered_map<K, V*> index_;
};

// Specialization for string keys
template<typename V>
class UniqueIndex<std::string, V> {
public:
    void insert(const std::string& key, V* value) {
        index_[key] = value;
    }

    V* get(const std::string& key) const {
        auto it = index_.find(key);
        return it != index_.end() ? it->second : nullptr;
    }

    bool contains(const std::string& key) const {
        return index_.find(key) != index_.end();
    }

    void clear() { index_.clear(); }

private:
    std::unordered_map<std::string, V*> index_;
};

// Group index: maps a single key to multiple rows
template<typename K, typename V>
class GroupIndex {
public:
    void insert(const K& key, V* value) {
        index_[key].push_back(value);
    }

    const std::vector<V*>& get(const K& key) const {
        auto it = index_.find(key);
        return it != index_.end() ? it->second : empty_;
    }

    bool contains(const K& key) const {
        return index_.find(key) != index_.end();
    }

    void clear() { index_.clear(); }

private:
    std::unordered_map<K, std::vector<V*>> index_;
    static inline std::vector<V*> empty_;
};

// Specialization for string keys
template<typename V>
class GroupIndex<std::string, V> {
public:
    void insert(const std::string& key, V* value) {
        index_[key].push_back(value);
    }

    const std::vector<V*>& get(const std::string& key) const {
        auto it = index_.find(key);
        return it != index_.end() ? it->second : empty_;
    }

    bool contains(const std::string& key) const {
        return index_.find(key) != index_.end();
    }

    void clear() { index_.clear(); }

private:
    std::unordered_map<std::string, std::vector<V*>> index_;
    static inline std::vector<V*> empty_;
};

// ============================================================================
// Data Table (container for rows with indexes)
// ============================================================================

// NOTE: Using std::deque instead of std::vector to ensure pointer stability.
// When elements are added, existing element pointers remain valid, which is
// required for the index system to work correctly.
template<typename T>
class DataTable {
public:
    using RowType = T;

    void add_row(T&& row) {
        rows_.push_back(std::move(row));
    }

    void add_row(const T& row) {
        rows_.push_back(row);
    }

    size_t count() const { return rows_.size(); }
    bool empty() const { return rows_.empty(); }

    T& operator[](size_t index) { return rows_[index]; }
    const T& operator[](size_t index) const { return rows_[index]; }

    typename std::deque<T>::iterator begin() { return rows_.begin(); }
    typename std::deque<T>::iterator end() { return rows_.end(); }
    typename std::deque<T>::const_iterator begin() const { return rows_.begin(); }
    typename std::deque<T>::const_iterator end() const { return rows_.end(); }

    std::deque<T>& rows() { return rows_; }
    const std::deque<T>& rows() const { return rows_; }

    void clear() { rows_.clear(); }

private:
    std::deque<T> rows_;
};

// ============================================================================
// File Loading Utilities
// ============================================================================

inline std::string read_file(const std::string& path) {
    std::ifstream file(path, std::ios::binary);
    if (!file) {
        throw std::runtime_error("Failed to open file: " + path);
    }
    std::ostringstream ss;
    ss << file.rdbuf();
    return ss.str();
}

inline std::vector<uint8_t> read_binary_file(const std::string& path) {
    std::ifstream file(path, std::ios::binary);
    if (!file) {
        throw std::runtime_error("Failed to open file: " + path);
    }
    return std::vector<uint8_t>(
        std::istreambuf_iterator<char>(file),
        std::istreambuf_iterator<char>()
    );
}

inline void write_binary_file(const std::string& path, const std::vector<uint8_t>& data) {
    std::ofstream file(path, std::ios::binary);
    if (!file) {
        throw std::runtime_error("Failed to create file: " + path);
    }
    file.write(reinterpret_cast<const char*>(data.data()), data.size());
}

} // namespace polygen
