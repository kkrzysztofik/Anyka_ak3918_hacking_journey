import { describe, expect, it } from 'vitest';

import { identificationSchema } from './identification';

describe('identificationSchema', () => {
  const validate = (data: unknown) => identificationSchema.safeParse(data);

  describe('valid identification data', () => {
    const validCases = [
      { name: 'Camera1', location: '', description: 'minimal valid config' },
      { name: 'Office Camera', location: 'Building A, Room 101', description: 'name and location' },
      { name: 'A', location: 'Test', description: 'single character name' },
      { name: 'A'.repeat(64), location: 'Test', description: 'maximum length name (64 chars)' },
      {
        name: 'Camera',
        location: 'A'.repeat(128),
        description: 'maximum length location (128 chars)',
      },
      { name: '123', location: '456', description: 'numeric strings' },
    ];

    it.each(validCases)('should validate $description', (config) => {
      const result = validate(config);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.name).toBe(config.name);
        expect(result.data.location).toBe(config.location);
      }
    });

    it('should handle whitespace-only strings', () => {
      expect(validate({ name: '   ', location: 'Test' }).success).toBe(true);
      expect(validate({ name: 'Camera', location: '   ' }).success).toBe(true);
    });

    it('should accept location with special and unicode characters', () => {
      expect(validate({ name: 'Camera', location: 'Building A, Room 101 - Floor 2' }).success).toBe(
        true,
      );
      expect(validate({ name: 'Camera', location: '测试地点 🏢' }).success).toBe(true);
    });
  });

  describe('invalid identification data', () => {
    const invalidCases = [
      { data: { name: '', location: 'Test' }, error: 'required', description: 'empty name' },
      {
        data: { name: 'A'.repeat(65), location: 'Test' },
        error: 'too long',
        description: 'name too long',
      },
      { data: { location: 'Test' }, error: 'invalid input', description: 'missing name' },
      {
        data: { name: 'Camera', location: 'A'.repeat(129) },
        error: 'too long',
        description: 'location too long',
      },
    ];

    it.each(invalidCases)('should reject $description', ({ data, error }) => {
      const result = validate(data);
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues[0].message.toLowerCase()).toContain(error.toLowerCase());
      }
    });
  });
});
