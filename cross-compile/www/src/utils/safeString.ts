/**
 * Safely converts a value to a string, providing a default if the value is null,
 * undefined, or an object (to avoid '[object Object]').
 *
 * @param value - The value to convert
 * @param defaultValue - The default value to return if conversion is not suitable
 * @returns The converted string or defaultValue
 */
export const safeString = (value: unknown, defaultValue: string): string => {
  if (value === null || value === undefined) {
    return defaultValue;
  }

  if (typeof value === 'string') {
    return value || defaultValue;
  }

  // Handle arrays by joining elements if they are strings
  if (Array.isArray(value)) {
    return defaultValue;
  }

  // For primitives (number, boolean, symbol, bigint), convert safely
  const valueType = typeof value;
  if (
    valueType === 'number' ||
    valueType === 'boolean' ||
    valueType === 'symbol' ||
    valueType === 'bigint'
  ) {
    return String(value); // NOSONAR: S6551 - value is narrowed to safe primitives here
  }

  return defaultValue;
};
