/**
 * Network Schema Tests
 */
import { describe, expect, it } from 'vitest';

import { networkSchema } from './network';

describe('networkSchema', () => {
  describe('valid network configs', () => {
    it('should validate DHCP enabled config', () => {
      const config = {
        dhcp: true,
        dnsFromDHCP: true,
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.dhcp).toBe(true);
        expect(result.data.dnsFromDHCP).toBe(true);
      }
    });

    it('should validate static IP config with all fields', () => {
      const config = {
        dhcp: false,
        address: '192.168.1.100',
        prefixLength: 24,
        gateway: '192.168.1.1',
        dnsFromDHCP: false,
        dns1: '8.8.8.8',
        dns2: '8.8.4.4',
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.dhcp).toBe(false);
        expect(result.data.address).toBe('192.168.1.100');
        expect(result.data.prefixLength).toBe(24);
        expect(result.data.gateway).toBe('192.168.1.1');
        expect(result.data.dns1).toBe('8.8.8.8');
        expect(result.data.dns2).toBe('8.8.4.4');
      }
    });

    it('should validate static IP config with minimal fields', () => {
      const config = {
        dhcp: false,
        address: '10.0.0.1',
        prefixLength: 16,
        dnsFromDHCP: false,
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(true);
    });

    it('should coerce prefixLength from string', () => {
      const config = {
        dhcp: false,
        address: '192.168.1.1',
        prefixLength: '24' as unknown as number,
        dnsFromDHCP: false,
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.prefixLength).toBe(24);
      }
    });
  });

  describe('invalid IP addresses', () => {
    const invalidIPCases = [
      { field: 'address', value: '256.1.1.1', description: 'invalid IP format in address' },
      { field: 'gateway', value: 'invalid', description: 'invalid IP format in gateway' },
      { field: 'dns1', value: 'not.an.ip', description: 'invalid IP format in dns1' },
      { field: 'dns2', value: '999.999.999.999', description: 'invalid IP format in dns2' },
    ];

    it.each(invalidIPCases)('should reject $description', ({ field, value }) => {
      const config = {
        dhcp: field !== 'address' && field !== 'gateway',
        address: field === 'address' ? value : '192.168.1.1',
        prefixLength: 24,
        [field]: value,
        dnsFromDHCP: false,
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(false);
    });
  });

  describe('prefixLength validation', () => {
    it.each([
      { prefix: 0, expected: false, description: 'prefixLength less than 1' },
      { prefix: 33, expected: false, description: 'prefixLength greater than 32' },
      { prefix: 1, expected: true, description: 'valid prefixLength 1' },
      { prefix: 8, expected: true, description: 'valid prefixLength 8' },
      { prefix: 16, expected: true, description: 'valid prefixLength 16' },
      { prefix: 24, expected: true, description: 'valid prefixLength 24' },
      { prefix: 32, expected: true, description: 'valid prefixLength 32' },
    ])('should $description', ({ prefix, expected }) => {
      const config = {
        dhcp: false,
        address: '192.168.1.1',
        prefixLength: prefix,
        dnsFromDHCP: false,
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(expected);
    });
  });

  describe('DHCP disabled validation', () => {
    it('should require address when DHCP is disabled', () => {
      const config = {
        dhcp: false,
        prefixLength: 24,
        dnsFromDHCP: false,
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(false);
      if (!result.success) {
        // The refine error is placed on the 'address' path
        const addressError = result.error.issues.find((issue) => issue.path.includes('address'));
        expect(addressError).toBeDefined();
      }
    });

    it('should require prefixLength when DHCP is disabled', () => {
      const config = {
        dhcp: false,
        address: '192.168.1.1',
        dnsFromDHCP: false,
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(false);
      if (!result.success) {
        // The refine error is placed on the 'address' path
        const addressError = result.error.issues.find((issue) => issue.path.includes('address'));
        expect(addressError).toBeDefined();
      }
    });

    it('should allow optional fields when DHCP is disabled', () => {
      const config = {
        dhcp: false,
        address: '192.168.1.1',
        prefixLength: 24,
        dnsFromDHCP: false,
        // gateway, dns1, dns2 are optional
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(true);
    });
  });

  describe('optional fields', () => {
    it('should allow empty address when DHCP is enabled', () => {
      const config = {
        dhcp: true,
        address: '',
        dnsFromDHCP: true,
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(true);
    });

    it('should allow empty gateway', () => {
      const config = {
        dhcp: false,
        address: '192.168.1.1',
        prefixLength: 24,
        gateway: '',
        dnsFromDHCP: false,
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(true);
    });

    it('should allow empty dns1', () => {
      const config = {
        dhcp: true,
        dnsFromDHCP: false,
        dns1: '',
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(true);
    });

    it('should allow empty dns2', () => {
      const config = {
        dhcp: true,
        dnsFromDHCP: false,
        dns2: '',
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(true);
    });
  });

  describe('valid IP addresses', () => {
    it.each(['0.0.0.0', '127.0.0.1', '192.168.1.1', '10.0.0.1', '172.16.0.1', '255.255.255.255'])(
      'should accept valid IP: %s',
      (ip) => {
        const config = {
          dhcp: false,
          address: ip,
          prefixLength: 24,
          dnsFromDHCP: false,
        };

        const result = networkSchema.safeParse(config);
        expect(result.success).toBe(true);
      },
    );
  });
});
