/**
 * Cryptographic Utilities Tests
 */
import { describe, expect, it } from 'vitest';

import {
  type EncryptedData,
  decrypt,
  deserializeEncrypted,
  encrypt,
  serializeEncrypted,
} from './crypto';

describe('crypto utilities', () => {
  describe('encrypt', () => {
    it('should encrypt a plaintext string', async () => {
      const plaintext = 'test password';
      const encrypted = await encrypt(plaintext);

      expect(encrypted).toHaveProperty('data');
      expect(encrypted).toHaveProperty('iv');
      expect(encrypted).toHaveProperty('salt');
      expect(encrypted.data).toBeTruthy();
      expect(encrypted.iv).toBeTruthy();
      expect(encrypted.salt).toBeTruthy();
    });

    it('should produce different output for same input (due to random salt/IV)', async () => {
      const plaintext = 'same password';
      const encrypted1 = await encrypt(plaintext);
      const encrypted2 = await encrypt(plaintext);

      // Salt and IV should be different
      expect(encrypted1.salt).not.toBe(encrypted2.salt);
      expect(encrypted1.iv).not.toBe(encrypted2.iv);
      // Encrypted data should be different
      expect(encrypted1.data).not.toBe(encrypted2.data);
    });

    it('should encrypt empty string', async () => {
      const encrypted = await encrypt('');
      expect(encrypted.data).toBeTruthy();
      expect(encrypted.iv).toBeTruthy();
      expect(encrypted.salt).toBeTruthy();
    });

    it('should encrypt long strings', async () => {
      const longString = 'a'.repeat(1000);
      const encrypted = await encrypt(longString);
      expect(encrypted.data).toBeTruthy();
    });
  });

  describe('decrypt', () => {
    it('should decrypt encrypted data correctly', async () => {
      const plaintext = 'test password';
      const encrypted = await encrypt(plaintext);
      const decrypted = await decrypt(encrypted);

      expect(decrypted).toBe(plaintext);
    });

    it('should decrypt empty string', async () => {
      const encrypted = await encrypt('');
      const decrypted = await decrypt(encrypted);
      expect(decrypted).toBe('');
    });

    it('should decrypt long strings', async () => {
      const longString = 'a'.repeat(1000);
      const encrypted = await encrypt(longString);
      const decrypted = await decrypt(encrypted);
      expect(decrypted).toBe(longString);
    });

    it('should decrypt special characters', async () => {
      const specialChars = '!@#$%^&*()_+-=[]{}|;:,.<>?';
      const encrypted = await encrypt(specialChars);
      const decrypted = await decrypt(encrypted);
      expect(decrypted).toBe(specialChars);
    });

    it('should decrypt unicode characters', async () => {
      const unicode = '测试 🎉 émojis';
      const encrypted = await encrypt(unicode);
      const decrypted = await decrypt(encrypted);
      expect(decrypted).toBe(unicode);
    });
  });

  describe('encrypt/decrypt roundtrip', () => {
    it('should successfully encrypt and decrypt various strings', async () => {
      const testCases = [
        'simple',
        'password123',
        'P@ssw0rd!',
        'very long password with spaces and special chars !@#$%',
        '1234567890',
        'a',
      ];

      for (const plaintext of testCases) {
        const encrypted = await encrypt(plaintext);
        const decrypted = await decrypt(encrypted);
        expect(decrypted).toBe(plaintext);
      }
    });
  });

  describe('serializeEncrypted', () => {
    it('should serialize encrypted data to JSON string', () => {
      const encrypted: EncryptedData = {
        data: 'base64data',
        iv: 'base64iv',
        salt: 'base64salt',
      };

      const serialized = serializeEncrypted(encrypted);
      expect(typeof serialized).toBe('string');
      expect(serialized).toContain('base64data');
      expect(serialized).toContain('base64iv');
      expect(serialized).toContain('base64salt');
    });

    it('should produce valid JSON', () => {
      const encrypted: EncryptedData = {
        data: 'test',
        iv: 'test',
        salt: 'test',
      };

      const serialized = serializeEncrypted(encrypted);
      expect(() => JSON.parse(serialized)).not.toThrow();
      const parsed = JSON.parse(serialized);
      expect(parsed).toEqual(encrypted);
    });
  });

  describe('deserializeEncrypted', () => {
    it('should deserialize valid JSON string', () => {
      const encrypted: EncryptedData = {
        data: 'base64data',
        iv: 'base64iv',
        salt: 'base64salt',
      };

      const serialized = serializeEncrypted(encrypted);
      const deserialized = deserializeEncrypted(serialized);

      expect(deserialized).toEqual(encrypted);
    });

    it('should throw error for invalid JSON', () => {
      const invalidJson = 'not valid json';

      expect(() => deserializeEncrypted(invalidJson)).toThrow('Invalid encrypted data format');
    });

    it('should throw error for empty string', () => {
      expect(() => deserializeEncrypted('')).toThrow('Invalid encrypted data format');
    });

    it('should throw error for malformed JSON', () => {
      const malformed = '{ "data": "test"'; // Missing closing brace

      expect(() => deserializeEncrypted(malformed)).toThrow('Invalid encrypted data format');
    });
  });

  describe('serialize/deserialize roundtrip', () => {
    it('should successfully serialize and deserialize encrypted data', async () => {
      const plaintext = 'test password';
      const encrypted = await encrypt(plaintext);
      const serialized = serializeEncrypted(encrypted);
      const deserialized = deserializeEncrypted(serialized);

      expect(deserialized).toEqual(encrypted);
      const decrypted = await decrypt(deserialized);
      expect(decrypted).toBe(plaintext);
    });
  });
});
