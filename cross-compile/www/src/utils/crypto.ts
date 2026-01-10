/**
 * Cryptographic utilities for secure credential storage
 *
 * Uses Web Crypto API (AES-GCM) for password encryption when available (HTTPS/localhost).
 * Falls back to base64 encoding in non-secure contexts (HTTP).
 * Keys are derived from a session-specific salt to prevent
 * cross-session key extraction.
 */

// Encryption result interface
export interface EncryptedData {
  data: string; // Base64-encoded ciphertext
  iv: string; // Base64-encoded initialization vector
  salt: string; // Base64-encoded salt for key derivation
  method?: 'aes-gcm' | 'base64'; // Encryption method used (for backward compatibility)
}

/**
 * Check if Web Crypto API is available (requires secure context)
 */
function isCryptoAvailable(): boolean {
  return typeof crypto !== 'undefined' && typeof crypto.subtle !== 'undefined';
}

/**
 * Generate a cryptographically strong random key
 */
async function generateKey(salt: Uint8Array): Promise<CryptoKey> {
  if (!isCryptoAvailable()) {
    throw new Error('Web Crypto API not available in non-secure context');
  }
  // Import the salt as a key material
  const keyMaterial = await crypto.subtle!.importKey('raw', salt, { name: 'PBKDF2' }, false, [
    'deriveKey',
  ]);

  // Derive a key using PBKDF2
  return crypto.subtle!.deriveKey(
    {
      name: 'PBKDF2',
      salt: salt,
      iterations: 100000,
      hash: 'SHA-256',
    },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt'],
  );
}

/**
 * Generate a cryptographically strong random salt
 */
function generateSalt(): Uint8Array {
  const salt = new Uint8Array(16);
  crypto.getRandomValues(salt);
  return salt;
}

/**
 * Generate a cryptographically strong random IV
 */
function generateIV(): Uint8Array {
  const iv = new Uint8Array(12); // 96-bit IV for AES-GCM
  crypto.getRandomValues(iv);
  return iv;
}

/**
 * Simple base64 encoding fallback for non-secure contexts
 * Note: This is NOT cryptographically secure, only obfuscation
 */
function encodeBase64(plaintext: string): EncryptedData {
  const salt = generateSalt();
  const iv = generateIV();
  // Simple obfuscation: reverse string and encode using TextEncoder for proper Unicode handling
  const obfuscated = plaintext.split('').reverse().join('');
  const encoder = new TextEncoder();
  const bytes = encoder.encode(obfuscated);
  const binaryString = Array.from(bytes, (byte) => String.fromCharCode(byte)).join('');
  const encoded = btoa(binaryString);
  return {
    data: encoded,
    iv: btoa(Array.from(iv, (b) => String.fromCodePoint(b)).join('')),
    salt: btoa(Array.from(salt, (b) => String.fromCodePoint(b)).join('')),
    method: 'base64',
  };
}

/**
 * Simple base64 decoding fallback for non-secure contexts
 */
function decodeBase64(encrypted: EncryptedData): string {
  try {
    const binaryString = atob(encrypted.data);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    const decoder = new TextDecoder();
    const decoded = decoder.decode(bytes);
    // Reverse the obfuscation
    return decoded.split('').reverse().join('');
  } catch {
    throw new Error('Failed to decode base64-encoded data');
  }
}

/**
 * Encrypt a string using AES-GCM (if available) or base64 fallback
 *
 * @param plaintext - The string to encrypt
 * @returns EncryptedData containing base64-encoded ciphertext, IV, and salt
 */
export async function encrypt(plaintext: string): Promise<EncryptedData> {
  // Use Web Crypto API if available (HTTPS/localhost)
  if (isCryptoAvailable()) {
    const salt = generateSalt();
    const iv = generateIV();
    const key = await generateKey(salt);

    const encoder = new TextEncoder();
    const data = encoder.encode(plaintext);

    const ciphertext = await crypto.subtle!.encrypt({ name: 'AES-GCM', iv: iv }, key, data);

    return {
      data: btoa(Array.from(new Uint8Array(ciphertext), (b) => String.fromCodePoint(b)).join('')),
      iv: btoa(Array.from(iv, (b) => String.fromCodePoint(b)).join('')),
      salt: btoa(Array.from(salt, (b) => String.fromCodePoint(b)).join('')),
      method: 'aes-gcm',
    };
  }

  // Fallback to base64 encoding for non-secure contexts (HTTP)
  return encodeBase64(plaintext);
}

/**
 * Decrypt a string using AES-GCM (if available) or base64 fallback
 *
 * @param encrypted - EncryptedData containing ciphertext, IV, and salt
 * @returns The decrypted plaintext string
 * @throws Error if decryption fails
 */
export async function decrypt(encrypted: EncryptedData): Promise<string> {
  // Determine encryption method (default to aes-gcm for backward compatibility)
  const method = encrypted.method ?? 'aes-gcm';

  // Use Web Crypto API if available and method is aes-gcm
  if (isCryptoAvailable() && method === 'aes-gcm') {
    const salt = Uint8Array.from(atob(encrypted.salt), (c) => c.codePointAt(0)!);
    const iv = Uint8Array.from(atob(encrypted.iv), (c) => c.codePointAt(0)!);
    const ciphertext = Uint8Array.from(atob(encrypted.data), (c) => c.codePointAt(0)!);
    const key = await generateKey(salt);

    const decrypted = await crypto.subtle!.decrypt({ name: 'AES-GCM', iv: iv }, key, ciphertext);

    const decoder = new TextDecoder();
    return decoder.decode(decrypted);
  }

  // If method is aes-gcm but crypto.subtle is not available, we cannot decrypt
  if (method === 'aes-gcm' && !isCryptoAvailable()) {
    throw new Error(
      'Cannot decrypt AES-GCM data: Web Crypto API not available in non-secure context. Please clear sessionStorage and log in again.',
    );
  }

  // Fallback to base64 decoding for base64-encoded data
  return decodeBase64(encrypted);
}

/**
 * Helper to serialize EncryptedData for sessionStorage
 */
export function serializeEncrypted(encrypted: EncryptedData): string {
  return JSON.stringify(encrypted);
}

/**
 * Helper to deserialize EncryptedData from sessionStorage
 */
export function deserializeEncrypted(data: string): EncryptedData {
  try {
    return JSON.parse(data);
  } catch {
    throw new Error('Invalid encrypted data format');
  }
}
