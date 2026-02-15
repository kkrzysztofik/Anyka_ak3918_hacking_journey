/**
 * Cryptographic Utilities Tests
 */
import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  type EncryptedData,
  clearSessionKey,
  decrypt,
  deserializeEncrypted,
  encrypt,
  serializeEncrypted,
} from './crypto';

describe('crypto utilities', () => {
  afterEach(() => {
    // Reset session key between tests to ensure isolation
    clearSessionKey();
  });

  describe('encrypt', () => {
    it('should encrypt a plaintext string', async () => {
      const plaintext = 'test password';
      const encrypted = await encrypt(plaintext);

      expect(encrypted).toHaveProperty('data');
      expect(encrypted).toHaveProperty('iv');
      expect(encrypted).toHaveProperty('method');
      expect(encrypted.method).toBe('aes-gcm');
      expect(encrypted.data).toBeTruthy();
      expect(encrypted.iv).toBeTruthy();
    });

    it('should produce different output for same input (due to random IV)', async () => {
      const plaintext = 'same password';
      const encrypted1 = await encrypt(plaintext);
      const encrypted2 = await encrypt(plaintext);

      // IV should be different
      expect(encrypted1.iv).not.toBe(encrypted2.iv);
      // Encrypted data should be different
      expect(encrypted1.data).not.toBe(encrypted2.data);
    });

    it('should encrypt empty string', async () => {
      const encrypted = await encrypt('');
      expect(encrypted.data).toBeTruthy();
      expect(encrypted.iv).toBeTruthy();
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

    it('should fail to decrypt after session key is cleared', async () => {
      const encrypted = await encrypt('secret');
      clearSessionKey();
      // New key is generated — cannot decrypt data from old key
      await expect(decrypt(encrypted)).rejects.toThrow();
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
        method: 'aes-gcm',
      };

      const serialized = serializeEncrypted(encrypted);
      expect(typeof serialized).toBe('string');
      expect(serialized).toContain('base64data');
      expect(serialized).toContain('base64iv');
    });

    it('should produce valid JSON', () => {
      const encrypted: EncryptedData = {
        data: 'test',
        iv: 'test',
        method: 'aes-gcm',
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
        method: 'aes-gcm',
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

  describe('clearSessionKey', () => {
    it('should clear the session key so encryption creates a new one', async () => {
      const encrypted1 = await encrypt('test');
      const decrypted1 = await decrypt(encrypted1);
      expect(decrypted1).toBe('test');

      clearSessionKey();

      // New encryption should work with a new key
      const encrypted2 = await encrypt('test2');
      const decrypted2 = await decrypt(encrypted2);
      expect(decrypted2).toBe('test2');

      // But old ciphertext from the first key should fail
      await expect(decrypt(encrypted1)).rejects.toThrow();
    });
  });

  describe('non-secure context handling', () => {
    it('should throw when crypto.subtle is unavailable', async () => {
      const originalCrypto = globalThis.crypto;

      const mockCrypto = {
        getRandomValues: (arr: Uint8Array) => originalCrypto.getRandomValues(arr),
      };

      vi.stubGlobal('crypto', mockCrypto);

      try {
        await expect(encrypt('test password')).rejects.toThrow(
          'Credential storage requires HTTPS',
        );
      } finally {
        vi.unstubAllGlobals();
      }
    });

    it('should throw on decrypt when crypto.subtle is unavailable', async () => {
      const originalCrypto = globalThis.crypto;

      const mockCrypto = {
        getRandomValues: (arr: Uint8Array) => originalCrypto.getRandomValues(arr),
      };

      vi.stubGlobal('crypto', mockCrypto);

      try {
        const fakeEncrypted: EncryptedData = {
          data: btoa('fake'),
          iv: btoa('fakeiv'),
          method: 'aes-gcm',
        };
        await expect(decrypt(fakeEncrypted)).rejects.toThrow(
          'Credential storage requires HTTPS',
        );
      } finally {
        vi.unstubAllGlobals();
      }
    });
  });
});
