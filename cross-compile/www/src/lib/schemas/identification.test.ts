/**
 * Identification Schema Tests
 */
import { describe, expect, it } from 'vitest';

import { type IdentificationFormData, identificationSchema } from './identification';

describe('identificationSchema', () => {
  describe('valid identification data', () => {
    it('should validate minimal valid config', () => {
      const config = {
        name: 'Camera1',
        location: '',
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.name).toBe('Camera1');
        expect(result.data.location).toBe('');
      }
    });

    it('should validate config with name and location', () => {
      const config = {
        name: 'Office Camera',
        location: 'Building A, Room 101',
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.name).toBe('Office Camera');
        expect(result.data.location).toBe('Building A, Room 101');
      }
    });

    it('should validate single character name', () => {
      const config = {
        name: 'A',
        location: 'Test',
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.name).toBe('A');
      }
    });

    it('should validate maximum length name (64 chars)', () => {
      const config = {
        name: 'A'.repeat(64),
        location: 'Test',
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.name).toHaveLength(64);
      }
    });

    it('should validate maximum length location (128 chars)', () => {
      const config = {
        name: 'Camera',
        location: 'A'.repeat(128),
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.location).toHaveLength(128);
      }
    });

    it('should validate empty location', () => {
      const config = {
        name: 'Camera',
        location: '',
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.location).toBe('');
      }
    });
  });

  describe('name validation', () => {
    it('should reject empty name', () => {
      const config = {
        name: '',
        location: 'Test',
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues[0].message).toContain('required');
      }
    });

    it('should reject name longer than 64 characters', () => {
      const config = {
        name: 'A'.repeat(65),
        location: 'Test',
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues[0].message).toContain('too long');
      }
    });

    it('should reject missing name', () => {
      const config = {
        location: 'Test',
      } as unknown as IdentificationFormData;

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(false);
    });
  });

  describe('location validation', () => {
    it('should reject location longer than 128 characters', () => {
      const config = {
        name: 'Camera',
        location: 'A'.repeat(129),
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues[0].message).toContain('too long');
      }
    });

    it('should accept location with exactly 128 characters', () => {
      const config = {
        name: 'Camera',
        location: 'A'.repeat(128),
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(true);
    });

    it('should accept location with special characters', () => {
      const config = {
        name: 'Camera',
        location: 'Building A, Room 101 - Floor 2',
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(true);
    });

    it('should accept location with unicode characters', () => {
      const config = {
        name: 'Camera',
        location: '测试地点 🏢',
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(true);
    });
  });

  describe('edge cases', () => {
    it('should handle whitespace-only name', () => {
      const config = {
        name: '   ',
        location: 'Test',
      };

      const result = identificationSchema.safeParse(config);
      // Whitespace-only string has length > 0, so it should pass
      expect(result.success).toBe(true);
    });

    it('should handle whitespace-only location', () => {
      const config = {
        name: 'Camera',
        location: '   ',
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(true);
    });

    it('should handle numeric strings', () => {
      const config = {
        name: '123',
        location: '456',
      };

      const result = identificationSchema.safeParse(config);
      expect(result.success).toBe(true);
    });
  });
});
