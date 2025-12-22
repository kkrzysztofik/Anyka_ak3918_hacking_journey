/**
 * Cryptographic utilities for secure credential storage
 *
 * Uses Web Crypto API (AES-GCM) for password encryption.
 * Keys are derived from a session-specific salt to prevent
 * cross-session key extraction.
 */

// Encryption result interface
export interface EncryptedData {
  data: string; // Base64-encoded ciphertext
  iv: string; // Base64-encoded initialization vector
  salt: string; // Base64-encoded salt for key derivation
}

/**
 * Generate a cryptographically strong random key
 */
async function generateKey(salt: Uint8Array): Promise<CryptoKey> {
  // Import the salt as a key material
  const keyMaterial = await crypto.subtle.importKey('raw', salt, { name: 'PBKDF2' }, false, [
    'deriveKey',
  ]);

  // Derive a key using PBKDF2
  return crypto.subtle.deriveKey(
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
 * Encrypt a string using AES-GCM
 *
 * @param plaintext - The string to encrypt
 * @returns EncryptedData containing base64-encoded ciphertext, IV, and salt
 */
export async function encrypt(plaintext: string): Promise<EncryptedData> {
  const salt = generateSalt();
  const iv = generateIV();
  const key = await generateKey(salt);

  const encoder = new TextEncoder();
  const data = encoder.encode(plaintext);

  const ciphertext = await crypto.subtle.encrypt({ name: 'AES-GCM', iv: iv }, key, data);

  return {
    data: btoa(String.fromCharCode(...new Uint8Array(ciphertext))),
    iv: btoa(String.fromCharCode(...iv)),
    salt: btoa(String.fromCharCode(...salt)),
  };
}

/**
 * Decrypt a string using AES-GCM
 *
 * @param encrypted - EncryptedData containing ciphertext, IV, and salt
 * @returns The decrypted plaintext string
 * @throws Error if decryption fails
 */
export async function decrypt(encrypted: EncryptedData): Promise<string> {
  const salt = Uint8Array.from(atob(encrypted.salt), (c) => c.charCodeAt(0));
  const iv = Uint8Array.from(atob(encrypted.iv), (c) => c.charCodeAt(0));
  const ciphertext = Uint8Array.from(atob(encrypted.data), (c) => c.charCodeAt(0));
  const key = await generateKey(salt);

  const decrypted = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: iv }, key, ciphertext);

  const decoder = new TextDecoder();
  return decoder.decode(decrypted);
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
