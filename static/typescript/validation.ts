// PolyGen Validation Support for TypeScript
// Provides validation utilities for checking data integrity after loading.

/**
 * Severity level for validation errors.
 */
export enum ValidationSeverity {
  /** Critical error that should prevent data usage. */
  Error = 'Error',
  /** Warning that may indicate potential issues. */
  Warning = 'Warning',
}

/**
 * Represents a single validation error found during data validation.
 */
export interface ValidationError {
  /** The name of the table where the error occurred. */
  tableName: string;
  /** The name of the field that failed validation. */
  fieldName: string;
  /** The row key (primary key value) of the record with the error. */
  rowKey: string;
  /** A human-readable description of the validation error. */
  message: string;
  /** The type of constraint that was violated. */
  constraintType: string;
  /** The severity level of this error. */
  severity: ValidationSeverity;
  /** The actual value that failed validation (if available). */
  actualValue?: unknown;
}

/**
 * Creates a validation error object.
 */
export function createValidationError(
  tableName: string,
  fieldName: string,
  rowKey: string,
  message: string,
  constraintType: string,
  severity: ValidationSeverity = ValidationSeverity.Error,
  actualValue?: unknown
): ValidationError {
  return {
    tableName,
    fieldName,
    rowKey,
    message,
    constraintType,
    severity,
    actualValue,
  };
}

/**
 * Formats a validation error as a string.
 */
export function formatValidationError(error: ValidationError): string {
  const location = error.rowKey
    ? `${error.tableName}[${error.rowKey}].${error.fieldName}`
    : `${error.tableName}.${error.fieldName}`;
  return `[${error.severity}] ${location}: ${error.message}`;
}

/**
 * Aggregates validation errors from data validation operations.
 */
export class ValidationResult {
  private _errors: ValidationError[] = [];

  /** Returns all validation errors. */
  get errors(): ReadonlyArray<ValidationError> {
    return this._errors;
  }

  /** Returns whether the validation passed (no errors). */
  get isValid(): boolean {
    return this._errors.length === 0;
  }

  /** Returns the total number of errors. */
  get errorCount(): number {
    return this._errors.length;
  }

  /** Adds a validation error to the result. */
  addError(error: ValidationError): void {
    this._errors.push(error);
  }

  /** Merges another validation result into this one. */
  merge(other: ValidationResult): void {
    this._errors.push(...other._errors);
  }

  /** Clears all errors. */
  clear(): void {
    this._errors = [];
  }

  /** Gets errors filtered by severity. */
  getErrorsBySeverity(severity: ValidationSeverity): ValidationError[] {
    return this._errors.filter((e) => e.severity === severity);
  }

  /** Gets errors filtered by table name. */
  getErrorsForTable(tableName: string): ValidationError[] {
    return this._errors.filter((e) => e.tableName === tableName);
  }

  /** Gets errors filtered by constraint type. */
  getErrorsByConstraint(constraintType: string): ValidationError[] {
    return this._errors.filter((e) => e.constraintType === constraintType);
  }

  /** Formats the validation result as a string. */
  toString(): string {
    if (this.isValid) {
      return 'Validation passed: no errors.';
    }
    const errorLines = this._errors.map(formatValidationError).join('\n  ');
    return `Validation failed with ${this._errors.length} error(s):\n  ${errorLines}`;
  }
}

/**
 * Error thrown when validation fails and the caller requested strict validation.
 */
export class ValidationException extends Error {
  public readonly result: ValidationResult;

  constructor(result: ValidationResult, message?: string) {
    super(message ?? `Data validation failed with ${result.errorCount} error(s).`);
    this.name = 'ValidationException';
    this.result = result;
  }
}

/**
 * Helper functions for validation.
 */
export const ValidationHelpers = {
  /**
   * Validates that a string does not exceed the maximum length.
   */
  validateMaxLength(value: string | null | undefined, maxLength: number): boolean {
    if (value == null) return true;
    return value.length <= maxLength;
  },

  /**
   * Validates that a value falls within the specified range (inclusive).
   */
  validateRange(value: number, min: number, max: number): boolean {
    return value >= min && value <= max;
  },

  /**
   * Validates that an optional value falls within the specified range.
   */
  validateRangeOptional(value: number | null | undefined, min: number, max: number): boolean {
    if (value == null) return true;
    return value >= min && value <= max;
  },

  /**
   * Validates that a string matches the specified regex pattern.
   */
  validateRegex(value: string | null | undefined, pattern: string): boolean {
    if (value == null) return true;
    try {
      const regex = new RegExp(pattern);
      return regex.test(value);
    } catch {
      return false;
    }
  },

  /**
   * Validates that a non-optional field is not null/undefined.
   */
  validateRequired<T>(value: T | null | undefined): value is T {
    return value != null;
  },

  /**
   * Creates a MaxLength validation error.
   */
  maxLengthError(
    tableName: string,
    fieldName: string,
    rowKey: string,
    maxLength: number,
    actualLength: number
  ): ValidationError {
    return createValidationError(
      tableName,
      fieldName,
      rowKey,
      `Value length (${actualLength}) exceeds maximum (${maxLength})`,
      'MaxLength',
      ValidationSeverity.Error,
      actualLength
    );
  },

  /**
   * Creates a Range validation error.
   */
  rangeError(
    tableName: string,
    fieldName: string,
    rowKey: string,
    min: number,
    max: number,
    actualValue: number
  ): ValidationError {
    return createValidationError(
      tableName,
      fieldName,
      rowKey,
      `Value (${actualValue}) is outside valid range [${min}, ${max}]`,
      'Range',
      ValidationSeverity.Error,
      actualValue
    );
  },

  /**
   * Creates a Regex validation error.
   */
  regexError(
    tableName: string,
    fieldName: string,
    rowKey: string,
    pattern: string,
    actualValue: string | null | undefined
  ): ValidationError {
    return createValidationError(
      tableName,
      fieldName,
      rowKey,
      `Value does not match pattern: ${pattern}`,
      'Regex',
      ValidationSeverity.Error,
      actualValue
    );
  },

  /**
   * Creates a ForeignKey validation error.
   */
  foreignKeyError(
    tableName: string,
    fieldName: string,
    rowKey: string,
    targetTable: string,
    foreignKeyValue: unknown
  ): ValidationError {
    return createValidationError(
      tableName,
      fieldName,
      rowKey,
      `Referenced record not found in ${targetTable} (key: ${foreignKeyValue})`,
      'ForeignKey',
      ValidationSeverity.Error,
      foreignKeyValue
    );
  },

  /**
   * Creates a Required field validation error.
   */
  requiredError(tableName: string, fieldName: string, rowKey: string): ValidationError {
    return createValidationError(
      tableName,
      fieldName,
      rowKey,
      'Required field has null/empty value',
      'Required',
      ValidationSeverity.Error,
      null
    );
  },

  /**
   * Creates a Unique constraint validation error.
   */
  uniqueError(
    tableName: string,
    fieldName: string,
    rowKey: string,
    duplicateValue: unknown
  ): ValidationError {
    return createValidationError(
      tableName,
      fieldName,
      rowKey,
      `Duplicate value found: ${duplicateValue}`,
      'Unique',
      ValidationSeverity.Error,
      duplicateValue
    );
  },
};
