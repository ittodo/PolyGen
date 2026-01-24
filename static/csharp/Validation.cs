// This file is a part of the Polygen common utility library.
// It provides the validation system for checking data integrity after loading.
using System;
using System.Collections.Generic;
using System.Text.RegularExpressions;

namespace Polygen.Common
{
    /// <summary>
    /// Severity level for validation errors.
    /// </summary>
    public enum ValidationSeverity
    {
        /// <summary>
        /// Critical error that should prevent data usage.
        /// </summary>
        Error,

        /// <summary>
        /// Warning that may indicate potential issues.
        /// </summary>
        Warning
    }

    /// <summary>
    /// Represents a single validation error found during data validation.
    /// </summary>
    public class ValidationError
    {
        /// <summary>
        /// Gets the name of the table where the error occurred.
        /// </summary>
        public string TableName { get; }

        /// <summary>
        /// Gets the name of the field that failed validation.
        /// </summary>
        public string FieldName { get; }

        /// <summary>
        /// Gets the row key (primary key value) of the record with the error.
        /// </summary>
        public object? RowKey { get; }

        /// <summary>
        /// Gets a human-readable description of the validation error.
        /// </summary>
        public string Message { get; }

        /// <summary>
        /// Gets the severity level of this error.
        /// </summary>
        public ValidationSeverity Severity { get; }

        /// <summary>
        /// Gets the type of constraint that was violated.
        /// </summary>
        public string ConstraintType { get; }

        /// <summary>
        /// Gets the actual value that failed validation (if available).
        /// </summary>
        public object? ActualValue { get; }

        /// <summary>
        /// Creates a new validation error.
        /// </summary>
        public ValidationError(
            string tableName,
            string fieldName,
            object? rowKey,
            string message,
            string constraintType,
            ValidationSeverity severity = ValidationSeverity.Error,
            object? actualValue = null)
        {
            TableName = tableName;
            FieldName = fieldName;
            RowKey = rowKey;
            Message = message;
            ConstraintType = constraintType;
            Severity = severity;
            ActualValue = actualValue;
        }

        /// <inheritdoc/>
        public override string ToString()
        {
            var location = RowKey != null
                ? $"{TableName}[{RowKey}].{FieldName}"
                : $"{TableName}.{FieldName}";
            return $"[{Severity}] {location}: {Message}";
        }
    }

    /// <summary>
    /// Aggregates validation errors from data validation operations.
    /// </summary>
    public class ValidationResult
    {
        private readonly List<ValidationError> _errors = new();

        /// <summary>
        /// Gets all validation errors.
        /// </summary>
        public IReadOnlyList<ValidationError> Errors => _errors.AsReadOnly();

        /// <summary>
        /// Gets whether the validation passed (no errors).
        /// </summary>
        public bool IsValid => _errors.Count == 0;

        /// <summary>
        /// Gets the total number of errors.
        /// </summary>
        public int ErrorCount => _errors.Count;

        /// <summary>
        /// Gets errors filtered by severity.
        /// </summary>
        public IEnumerable<ValidationError> GetErrors(ValidationSeverity severity)
        {
            foreach (var error in _errors)
            {
                if (error.Severity == severity)
                    yield return error;
            }
        }

        /// <summary>
        /// Gets errors filtered by table name.
        /// </summary>
        public IEnumerable<ValidationError> GetErrorsForTable(string tableName)
        {
            foreach (var error in _errors)
            {
                if (error.TableName == tableName)
                    yield return error;
            }
        }

        /// <summary>
        /// Gets errors filtered by constraint type.
        /// </summary>
        public IEnumerable<ValidationError> GetErrorsByConstraint(string constraintType)
        {
            foreach (var error in _errors)
            {
                if (error.ConstraintType == constraintType)
                    yield return error;
            }
        }

        /// <summary>
        /// Adds a validation error to the result.
        /// </summary>
        public void AddError(ValidationError error)
        {
            _errors.Add(error);
        }

        /// <summary>
        /// Adds a validation error to the result.
        /// </summary>
        public void AddError(
            string tableName,
            string fieldName,
            object? rowKey,
            string message,
            string constraintType,
            ValidationSeverity severity = ValidationSeverity.Error,
            object? actualValue = null)
        {
            _errors.Add(new ValidationError(
                tableName, fieldName, rowKey, message, constraintType, severity, actualValue));
        }

        /// <summary>
        /// Merges another validation result into this one.
        /// </summary>
        public void Merge(ValidationResult other)
        {
            _errors.AddRange(other._errors);
        }

        /// <summary>
        /// Clears all errors.
        /// </summary>
        public void Clear()
        {
            _errors.Clear();
        }

        /// <inheritdoc/>
        public override string ToString()
        {
            if (IsValid)
                return "Validation passed: no errors.";

            return $"Validation failed with {_errors.Count} error(s):\n" +
                   string.Join("\n", _errors);
        }
    }

    /// <summary>
    /// Exception thrown when validation fails and the caller requested strict validation.
    /// </summary>
    public class ValidationException : Exception
    {
        /// <summary>
        /// Gets the validation result containing all errors.
        /// </summary>
        public ValidationResult Result { get; }

        /// <summary>
        /// Creates a new validation exception.
        /// </summary>
        public ValidationException(ValidationResult result)
            : base($"Data validation failed with {result.ErrorCount} error(s).")
        {
            Result = result;
        }

        /// <summary>
        /// Creates a new validation exception with a custom message.
        /// </summary>
        public ValidationException(ValidationResult result, string message)
            : base(message)
        {
            Result = result;
        }

        /// <inheritdoc/>
        public override string ToString()
        {
            return $"{base.ToString()}\n{Result}";
        }
    }

    /// <summary>
    /// Helper methods for validating field values against constraints.
    /// </summary>
    public static class ValidationHelpers
    {
        /// <summary>
        /// Validates that a string does not exceed the maximum length.
        /// </summary>
        public static bool ValidateMaxLength(string? value, int maxLength)
        {
            if (value == null) return true;
            return value.Length <= maxLength;
        }

        /// <summary>
        /// Validates that a value falls within the specified range (inclusive).
        /// </summary>
        public static bool ValidateRange<T>(T value, T min, T max) where T : IComparable<T>
        {
            return value.CompareTo(min) >= 0 && value.CompareTo(max) <= 0;
        }

        /// <summary>
        /// Validates that a nullable value falls within the specified range (inclusive).
        /// Returns true if value is null (nullable fields are valid when null).
        /// </summary>
        public static bool ValidateRangeNullable<T>(T? value, T min, T max) where T : struct, IComparable<T>
        {
            if (!value.HasValue) return true;
            return value.Value.CompareTo(min) >= 0 && value.Value.CompareTo(max) <= 0;
        }

        /// <summary>
        /// Validates that a string matches the specified regex pattern.
        /// </summary>
        public static bool ValidateRegex(string? value, string pattern)
        {
            if (value == null) return true;
            try
            {
                return Regex.IsMatch(value, pattern);
            }
            catch (ArgumentException)
            {
                // Invalid regex pattern - treat as validation failure
                return false;
            }
        }

        /// <summary>
        /// Validates that a string matches the specified regex pattern using a compiled Regex.
        /// </summary>
        public static bool ValidateRegex(string? value, Regex regex)
        {
            if (value == null) return true;
            return regex.IsMatch(value);
        }

        /// <summary>
        /// Validates that a non-optional field is not null.
        /// </summary>
        public static bool ValidateRequired<T>(T? value) where T : class
        {
            return value != null;
        }

        /// <summary>
        /// Validates that a non-optional value type field has been set.
        /// </summary>
        public static bool ValidateRequiredValue<T>(T? value) where T : struct
        {
            return value.HasValue;
        }

        /// <summary>
        /// Creates a MaxLength validation error.
        /// </summary>
        public static ValidationError MaxLengthError(
            string tableName,
            string fieldName,
            object? rowKey,
            int maxLength,
            int actualLength)
        {
            return new ValidationError(
                tableName,
                fieldName,
                rowKey,
                $"Value length ({actualLength}) exceeds maximum ({maxLength})",
                "MaxLength",
                ValidationSeverity.Error,
                actualLength);
        }

        /// <summary>
        /// Creates a Range validation error.
        /// </summary>
        public static ValidationError RangeError<T>(
            string tableName,
            string fieldName,
            object? rowKey,
            T min,
            T max,
            T actualValue)
        {
            return new ValidationError(
                tableName,
                fieldName,
                rowKey,
                $"Value ({actualValue}) is outside valid range [{min}, {max}]",
                "Range",
                ValidationSeverity.Error,
                actualValue);
        }

        /// <summary>
        /// Creates a Regex validation error.
        /// </summary>
        public static ValidationError RegexError(
            string tableName,
            string fieldName,
            object? rowKey,
            string pattern,
            string? actualValue)
        {
            return new ValidationError(
                tableName,
                fieldName,
                rowKey,
                $"Value does not match pattern: {pattern}",
                "Regex",
                ValidationSeverity.Error,
                actualValue);
        }

        /// <summary>
        /// Creates a ForeignKey validation error.
        /// </summary>
        public static ValidationError ForeignKeyError(
            string tableName,
            string fieldName,
            object? rowKey,
            string targetTable,
            object? foreignKeyValue)
        {
            return new ValidationError(
                tableName,
                fieldName,
                rowKey,
                $"Referenced record not found in {targetTable} (key: {foreignKeyValue})",
                "ForeignKey",
                ValidationSeverity.Error,
                foreignKeyValue);
        }

        /// <summary>
        /// Creates a Required field validation error.
        /// </summary>
        public static ValidationError RequiredError(
            string tableName,
            string fieldName,
            object? rowKey)
        {
            return new ValidationError(
                tableName,
                fieldName,
                rowKey,
                "Required field has null/empty value",
                "Required",
                ValidationSeverity.Error,
                null);
        }

        /// <summary>
        /// Creates a Unique constraint validation error.
        /// </summary>
        public static ValidationError UniqueError(
            string tableName,
            string fieldName,
            object? rowKey,
            object? duplicateValue)
        {
            return new ValidationError(
                tableName,
                fieldName,
                rowKey,
                $"Duplicate value found: {duplicateValue}",
                "Unique",
                ValidationSeverity.Error,
                duplicateValue);
        }
    }
}
