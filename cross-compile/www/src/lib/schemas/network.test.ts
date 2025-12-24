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
    it('should reject invalid IP format in address', () => {
      const config = {
        dhcp: false,
        address: '256.1.1.1',
        prefixLength: 24,
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(false);
    });

    it('should reject invalid IP format in gateway', () => {
      const config = {
        dhcp: false,
        address: '192.168.1.1',
        prefixLength: 24,
        gateway: 'invalid',
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(false);
    });

    it('should reject invalid IP format in dns1', () => {
      const config = {
        dhcp: true,
        dnsFromDHCP: false,
        dns1: 'not.an.ip',
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(false);
    });

    it('should reject invalid IP format in dns2', () => {
      const config = {
        dhcp: true,
        dnsFromDHCP: false,
        dns2: '999.999.999.999',
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(false);
    });
  });

  describe('prefixLength validation', () => {
    it('should reject prefixLength less than 1', () => {
      const config = {
        dhcp: false,
        address: '192.168.1.1',
        prefixLength: 0,
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(false);
    });

    it('should reject prefixLength greater than 32', () => {
      const config = {
        dhcp: false,
        address: '192.168.1.1',
        prefixLength: 33,
      };

      const result = networkSchema.safeParse(config);
      expect(result.success).toBe(false);
    });

    it('should accept valid prefixLength range', () => {
      const validPrefixes = [1, 8, 16, 24, 32];

      for (const prefix of validPrefixes) {
        const config = {
          dhcp: false,
          address: '192.168.1.1',
          prefixLength: prefix,
          dnsFromDHCP: false,
        };

        const result = networkSchema.safeParse(config);
        expect(result.success).toBe(true);
      }
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
    it('should accept various valid IP formats', () => {
      const validIPs = [
        '0.0.0.0',
        '127.0.0.1',
        '192.168.1.1',
        '10.0.0.1',
        '172.16.0.1',
        '255.255.255.255',
      ];

      for (const ip of validIPs) {
        const config = {
          dhcp: false,
          address: ip,
          prefixLength: 24,
          dnsFromDHCP: false,
        };

        const result = networkSchema.safeParse(config);
        expect(result.success).toBe(true);
      }
    });
  });
});
