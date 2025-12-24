/**
 * Schema test helpers for validation testing patterns
 */
import { expect, it } from 'vitest';
import type { ZodType } from 'zod';

/**
 * Test case for schema validation
 */
export interface SchemaTestCase<T> {
  /** Test case name/description */
  name: string;
  /** Input data to test */
  input: unknown;
  /** Expected result - true if valid, false if invalid */
  valid: boolean;
  /** Optional: expected output data (for valid cases) */
  expectedOutput?: T;
  /** Optional: expected error message pattern (for invalid cases) */
  expectedError?: string | RegExp;
}

/**
 * Test multiple valid schema cases
 * @param schema - Zod schema to test
 * @param validCases - Array of valid test cases
 */
export function testValidSchema<T>(
  schema: ZodType<T, unknown>,
  validCases: Array<{ name: string; input: unknown; expectedOutput?: T }>,
): void {
  validCases.forEach((testCase) => {
    it(`should validate: ${testCase.name}`, () => {
      const result = schema.safeParse(testCase.input);
      expect(result.success).toBe(true);
      if (result.success && testCase.expectedOutput !== undefined) {
        expect(result.data).toEqual(testCase.expectedOutput);
      }
    });
  });
}

/**
 * Test multiple invalid schema cases
 * @param schema - Zod schema to test
 * @param invalidCases - Array of invalid test cases
 */
export function testInvalidSchema(
  schema: ZodType<unknown, unknown>,
  invalidCases: Array<{
    name: string;
    input: unknown;
    expectedError?: string | RegExp;
  }>,
): void {
  invalidCases.forEach((testCase) => {
    it(`should reject: ${testCase.name}`, () => {
      const result = schema.safeParse(testCase.input);
      expect(result.success).toBe(false);
      if (!result.success && testCase.expectedError) {
        const errorMessage = result.error.issues[0]?.message ?? '';
        if (typeof testCase.expectedError === 'string') {
          expect(errorMessage).toContain(testCase.expectedError);
        } else {
          expect(errorMessage).toMatch(testCase.expectedError);
        }
      }
    });
  });
}

/**
 * Generic schema validation test helper
 * @param schema - Zod schema to test
 * @param testCases - Array of test cases (both valid and invalid)
 */
export function testSchemaValidation<T>(
  schema: ZodType<T, unknown>,
  testCases: SchemaTestCase<T>[],
): void {
  testCases.forEach((testCase) => {
    if (testCase.valid) {
      it(`should validate: ${testCase.name}`, () => {
        const result = schema.safeParse(testCase.input);
        expect(result.success).toBe(true);
        if (result.success && testCase.expectedOutput !== undefined) {
          expect(result.data).toEqual(testCase.expectedOutput);
        }
      });
    } else {
      it(`should reject: ${testCase.name}`, () => {
        const result = schema.safeParse(testCase.input);
        expect(result.success).toBe(false);
        if (!result.success && testCase.expectedError) {
          const errorMessage = result.error.issues[0]?.message ?? '';
          if (typeof testCase.expectedError === 'string') {
            expect(errorMessage).toContain(testCase.expectedError);
          } else {
            expect(errorMessage).toMatch(testCase.expectedError);
          }
        }
      });
    }
  });
}

/**
 * Test schema with multiple values for a single field
 * Useful for testing ranges, formats, etc.
 * @param schema - Zod schema to test
 * @param fieldName - Name of the field being tested
 * @param values - Array of {value, valid, name?} objects
 */
export function testSchemaField(
  schema: ZodType<unknown, unknown>,
  fieldName: string,
  values: Array<{
    value: unknown;
    valid: boolean;
    name?: string;
  }>,
): void {
  values.forEach((testValue) => {
    const testName = testValue.name ?? String(testValue.value);
    const input = { [fieldName]: testValue.value };

    if (testValue.valid) {
      it(`should accept ${fieldName}: ${testName}`, () => {
        const result = schema.safeParse(input);
        expect(result.success).toBe(true);
      });
    } else {
      it(`should reject ${fieldName}: ${testName}`, () => {
        const result = schema.safeParse(input);
        expect(result.success).toBe(false);
      });
    }
  });
}
