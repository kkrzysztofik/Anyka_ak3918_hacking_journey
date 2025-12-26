/**
 * Camera Config Tests
 */
import { describe, expect, it } from 'vitest';

import type { CameraConfig } from '@/types';

import {
  DEFAULT_CAMERA_CONFIG,
  createCameraConfig,
  createCustomCameraConfig,
  getDevelopmentConfig,
  getProductionConfig,
  validateCameraConfig,
} from './cameraConfig';

describe('cameraConfig', () => {
  describe('DEFAULT_CAMERA_CONFIG', () => {
    it('should have default values', () => {
      expect(DEFAULT_CAMERA_CONFIG.ip).toBe('192.168.1.100');
      expect(DEFAULT_CAMERA_CONFIG.onvifPort).toBe(8080);
      expect(DEFAULT_CAMERA_CONFIG.snapshotPort).toBe(3000);
      expect(DEFAULT_CAMERA_CONFIG.rtspPort).toBe(554);
    });
  });

  describe('createCameraConfig', () => {
    it('should create config with valid IP', () => {
      const config = createCameraConfig('192.168.1.1');
      expect(config.ip).toBe('192.168.1.1');
      expect(config.onvifPort).toBe(DEFAULT_CAMERA_CONFIG.onvifPort);
      expect(config.snapshotPort).toBe(DEFAULT_CAMERA_CONFIG.snapshotPort);
      expect(config.rtspPort).toBe(DEFAULT_CAMERA_CONFIG.rtspPort);
    });

    it('should create config with different valid IPs', () => {
      const testIPs = ['10.0.0.1', '172.16.0.1', '255.255.255.255', '0.0.0.0', '127.0.0.1'];

      for (const ip of testIPs) {
        const config = createCameraConfig(ip);
        expect(config.ip).toBe(ip);
      }
    });

    it('should throw error for empty IP', () => {
      expect(() => createCameraConfig('')).toThrow('IP address is required and must be a string');
    });

    it('should throw error for null IP', () => {
      expect(() => createCameraConfig(null as unknown as string)).toThrow(
        'IP address is required and must be a string',
      );
    });

    it('should throw error for invalid IP format', () => {
      const invalidIPs = [
        '256.1.1.1',
        '1.256.1.1',
        '1.1.256.1',
        '1.1.1.256',
        'not.an.ip',
        '192.168.1',
        '192.168.1.1.1',
        '192.168.1.',
        '.192.168.1.1',
      ];

      for (const ip of invalidIPs) {
        expect(() => createCameraConfig(ip)).toThrow('Invalid IP address format');
      }
    });

    it('should throw error for non-string IP', () => {
      expect(() => createCameraConfig(123 as unknown as string)).toThrow(
        'IP address is required and must be a string',
      );
    });
  });

  describe('createCustomCameraConfig', () => {
    it('should create config with default values when empty', () => {
      const config = createCustomCameraConfig({});
      expect(config).toEqual(DEFAULT_CAMERA_CONFIG);
    });

    it('should merge custom IP with defaults', () => {
      const config = createCustomCameraConfig({ ip: '10.0.0.1' });
      expect(config.ip).toBe('10.0.0.1');
      expect(config.onvifPort).toBe(DEFAULT_CAMERA_CONFIG.onvifPort);
      expect(config.snapshotPort).toBe(DEFAULT_CAMERA_CONFIG.snapshotPort);
      expect(config.rtspPort).toBe(DEFAULT_CAMERA_CONFIG.rtspPort);
    });

    it('should merge custom ports with defaults', () => {
      const config = createCustomCameraConfig({
        onvifPort: 9000,
        snapshotPort: 4000,
        rtspPort: 8554,
      });
      expect(config.ip).toBe(DEFAULT_CAMERA_CONFIG.ip);
      expect(config.onvifPort).toBe(9000);
      expect(config.snapshotPort).toBe(4000);
      expect(config.rtspPort).toBe(8554);
    });

    it('should merge all custom values', () => {
      const config = createCustomCameraConfig({
        ip: '10.0.0.1',
        onvifPort: 9000,
        snapshotPort: 4000,
        rtspPort: 8554,
      });
      expect(config.ip).toBe('10.0.0.1');
      expect(config.onvifPort).toBe(9000);
      expect(config.snapshotPort).toBe(4000);
      expect(config.rtspPort).toBe(8554);
    });

    it('should validate IP in custom config', () => {
      expect(() => createCustomCameraConfig({ ip: 'invalid' })).toThrow(
        'Invalid IP address format',
      );
    });

    it('should validate IP type in custom config', () => {
      expect(() => createCustomCameraConfig({ ip: 123 as unknown as string })).toThrow(
        'IP address must be a string',
      );
    });

    it('should validate onvifPort range', () => {
      expect(() => createCustomCameraConfig({ onvifPort: 0 })).toThrow(
        'ONVIF port must be a number between 1 and 65535',
      );
      expect(() => createCustomCameraConfig({ onvifPort: 65536 })).toThrow(
        'ONVIF port must be a number between 1 and 65535',
      );
      expect(() => createCustomCameraConfig({ onvifPort: -1 })).toThrow(
        'ONVIF port must be a number between 1 and 65535',
      );
    });

    it('should validate snapshotPort range', () => {
      expect(() => createCustomCameraConfig({ snapshotPort: 0 })).toThrow(
        'Snapshot port must be a number between 1 and 65535',
      );
      expect(() => createCustomCameraConfig({ snapshotPort: 65536 })).toThrow(
        'Snapshot port must be a number between 1 and 65535',
      );
    });

    it('should validate rtspPort range', () => {
      expect(() => createCustomCameraConfig({ rtspPort: 0 })).toThrow(
        'RTSP port must be a number between 1 and 65535',
      );
      expect(() => createCustomCameraConfig({ rtspPort: 65536 })).toThrow(
        'RTSP port must be a number between 1 and 65535',
      );
    });

    it('should accept valid port ranges', () => {
      const config1 = createCustomCameraConfig({ onvifPort: 1 });
      expect(config1.onvifPort).toBe(1);

      const config2 = createCustomCameraConfig({ onvifPort: 65535 });
      expect(config2.onvifPort).toBe(65535);

      const config3 = createCustomCameraConfig({ onvifPort: 8080 });
      expect(config3.onvifPort).toBe(8080);
    });
  });

  describe('validateCameraConfig', () => {
    it('should return true for valid config', () => {
      const validConfig: CameraConfig = {
        ip: '192.168.1.1',
        onvifPort: 8080,
        snapshotPort: 3000,
        rtspPort: 554,
      };
      expect(validateCameraConfig(validConfig)).toBe(true);
    });

    it('should return false for invalid IP', () => {
      const invalidConfig: CameraConfig = {
        ip: 'invalid',
        onvifPort: 8080,
        snapshotPort: 3000,
        rtspPort: 554,
      };
      expect(validateCameraConfig(invalidConfig)).toBe(false);
    });

    it('should return false for invalid port', () => {
      const invalidConfig: CameraConfig = {
        ip: '192.168.1.1',
        onvifPort: 0,
        snapshotPort: 3000,
        rtspPort: 554,
      };
      expect(validateCameraConfig(invalidConfig)).toBe(false);
    });

    it('should return true for default config', () => {
      expect(validateCameraConfig(DEFAULT_CAMERA_CONFIG)).toBe(true);
    });
  });

  describe('getDevelopmentConfig', () => {
    it('should return default config', () => {
      const config = getDevelopmentConfig();
      expect(config).toEqual(DEFAULT_CAMERA_CONFIG);
    });
  });

  describe('getProductionConfig', () => {
    it('should create config with provided IP', () => {
      const config = getProductionConfig('10.0.0.1');
      expect(config.ip).toBe('10.0.0.1');
      expect(config.onvifPort).toBe(DEFAULT_CAMERA_CONFIG.onvifPort);
      expect(config.snapshotPort).toBe(DEFAULT_CAMERA_CONFIG.snapshotPort);
      expect(config.rtspPort).toBe(DEFAULT_CAMERA_CONFIG.rtspPort);
    });

    it('should validate IP in production config', () => {
      expect(() => getProductionConfig('invalid')).toThrow('Invalid IP address format');
    });
  });
});
