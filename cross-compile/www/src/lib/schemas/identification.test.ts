import { describe, expect, it } from 'vitest';

import { identificationSchema } from './identification';

describe('identificationSchema', () => {
  const validate = (data: unknown) => identificationSchema.safeParse(data);

  const defaultDeviceInfo = {
    manufacturer: 'Test Manufacturer',
    model: 'Test Model',
    firmwareVersion: '1.0.0',
    serialNumber: 'SN123456',
    hardwareId: 'HW123456',
  };

  describe('valid identification data', () => {
    const validCases = [
      {
        deviceInfo: defaultDeviceInfo,
        name: 'Camera1',
        location: '',
        description: 'minimal valid config',
      },
      {
        deviceInfo: defaultDeviceInfo,
        name: 'Office Camera',
        location: 'Building A, Room 101',
        description: 'name and location',
      },
      {
        deviceInfo: defaultDeviceInfo,
        name: 'A',
        location: 'Test',
        description: 'single character name',
      },
      {
        deviceInfo: defaultDeviceInfo,
        name: 'A'.repeat(64),
        location: 'Test',
        description: 'maximum length name (64 chars)',
      },
      {
        deviceInfo: defaultDeviceInfo,
        name: 'Camera',
        location: 'A'.repeat(128),
        description: 'maximum length location (128 chars)',
      },
      {
        deviceInfo: defaultDeviceInfo,
        name: '123',
        location: '456',
        description: 'numeric strings',
      },
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
      expect(
        validate({ deviceInfo: defaultDeviceInfo, name: '   ', location: 'Test' }).success,
      ).toBe(true);
      expect(
        validate({ deviceInfo: defaultDeviceInfo, name: 'Camera', location: '   ' }).success,
      ).toBe(true);
    });

    it('should accept location with special and unicode characters', () => {
      expect(
        validate({
          deviceInfo: defaultDeviceInfo,
          name: 'Camera',
          location: 'Building A, Room 101 - Floor 2',
        }).success,
      ).toBe(true);
      expect(
        validate({ deviceInfo: defaultDeviceInfo, name: 'Camera', location: '测试地点 🏢' })
          .success,
      ).toBe(true);
    });
  });

  describe('scopes', () => {
    const base = {
      deviceInfo: defaultDeviceInfo,
      name: 'Camera',
      location: 'Hall',
    };

    it('should accept a valid ONVIF scope', () => {
      const result = validate({
        ...base,
        scopes: [
          {
            scopeDef: 'Configurable',
            scopeItem: 'onvif://www.onvif.org/name/Cam',
          },
        ],
      });
      expect(result.success).toBe(true);
    });

    it('should reject an empty scope', () => {
      const result = validate({
        ...base,
        scopes: [{ scopeDef: 'Configurable', scopeItem: '' }],
      });
      expect(result.success).toBe(false);
    });

    it('should reject a scope that is too long', () => {
      const result = validate({
        ...base,
        scopes: [
          {
            scopeDef: 'Configurable',
            scopeItem: `onvif://www.onvif.org/${'x'.repeat(256)}`,
          },
        ],
      });
      expect(result.success).toBe(false);
    });

    it('should reject a scope with the wrong prefix', () => {
      const result = validate({
        ...base,
        scopes: [{ scopeDef: 'Configurable', scopeItem: 'http://example.com/scope' }],
      });
      expect(result.success).toBe(false);
    });

    it('should reject a scope that contains spaces', () => {
      const result = validate({
        ...base,
        scopes: [
          {
            scopeDef: 'Configurable',
            scopeItem: 'onvif://www.onvif.org/name/Front Door',
          },
        ],
      });
      expect(result.success).toBe(false);
    });

    it.each([
      ['U+00A0', 'onvif://www.onvif.org/name/Cam\u00A0era'],
      ['U+007F', 'onvif://www.onvif.org/name/Cam\u007Fera'],
      ['U+0085', 'onvif://www.onvif.org/name/Cam\u0085era'],
    ])('should reject a scope containing %s', (_label, scopeItem) => {
      const result = validate({
        ...base,
        scopes: [{ scopeDef: 'Configurable', scopeItem }],
      });
      expect(result.success).toBe(false);
    });
  });

  describe('invalid identification data', () => {
    const invalidCases = [
      {
        data: { deviceInfo: defaultDeviceInfo, name: '', location: 'Test' },
        error: 'required',
        description: 'empty name',
      },
      {
        data: { deviceInfo: defaultDeviceInfo, name: 'A'.repeat(65), location: 'Test' },
        error: 'too long',
        description: 'name too long',
      },
      {
        data: { deviceInfo: defaultDeviceInfo, location: 'Test' },
        error: 'invalid input',
        description: 'missing name',
      },
      {
        data: { deviceInfo: defaultDeviceInfo, name: 'Camera', location: 'A'.repeat(129) },
        error: 'too long',
        description: 'location too long',
      },
      {
        data: { deviceInfo: defaultDeviceInfo, name: 'Camera', location: 'Hall', hostname: '' },
        error: 'required',
        description: 'empty hostname',
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
