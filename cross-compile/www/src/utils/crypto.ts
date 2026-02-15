/**
 * Cryptographic utilities for secure credential storage
 *
 * Uses Web Crypto API (AES-GCM) with a non-exportable session key held in memory.
 * The key is generated once per page load via crypto.subtle.generateKey() and is
 * never persisted — on page refresh the key is lost, stored ciphertext becomes
 * unreadable, and the user must re-authenticate. This is the correct security
 * model for sessionStorage-based credential caching.
 *
 * When SubtleCrypto is unavailable (plain HTTP), credential persistence is
 * refused entirely — callers must handle this by keeping credentials in
 * React state only.
 */

// Encryption result interface
export interface EncryptedData {
  data: string; // Base64-encoded ciphertext
  iv: string; // Base64-encoded initialization vector
  method: 'aes-gcm'; // Encryption method (only AES-GCM is supported)
}

/** Module-level session key — lives only in memory, never exported or persisted */
let sessionKey: CryptoKey | null = null;

/**
 * Check if Web Crypto API is available (requires secure context)
 */
function isCryptoAvailable(): boolean {
  if (globalThis.crypto === undefined) return false;
  if (globalThis.crypto.subtle === undefined) return false;
  return true;
}

/**
 * Returns the current session key, generating one if needed.
 * The key is non-exportable and uses AES-GCM with a 256-bit key length.
 */
async function getOrCreateKey(): Promise<CryptoKey> {
  if (!isCryptoAvailable()) {
    throw new Error('Credential storage requires HTTPS');
  }
  if (sessionKey) return sessionKey;

  sessionKey = await crypto.subtle.generateKey({ name: 'AES-GCM', length: 256 }, false, [
    'encrypt',
    'decrypt',
  ]);
  return sessionKey;
}

/**
 * Clear the in-memory session key.
 * Call on logout to ensure the key cannot be reused.
 */
export function clearSessionKey(): void {
  sessionKey = null;
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
 * Encrypt a string using AES-GCM with the session key.
 *
 * @param plaintext - The string to encrypt
 * @returns EncryptedData containing base64-encoded ciphertext and IV
 * @throws Error if SubtleCrypto is unavailable (non-HTTPS context)
 */
export async function encrypt(plaintext: string): Promise<EncryptedData> {
  const key = await getOrCreateKey();
  const iv = generateIV();

  const encoder = new TextEncoder();
  const data = encoder.encode(plaintext);

  // Convert IV to ArrayBuffer
  const ivBuffer = new ArrayBuffer(iv.byteLength);
  const ivView = new Uint8Array(ivBuffer);
  ivView.set(iv);

  const ciphertext = await crypto.subtle.encrypt({ name: 'AES-GCM', iv: ivBuffer }, key, data);

  return {
    data: btoa(Array.from(new Uint8Array(ciphertext), (b) => String.fromCodePoint(b)).join('')),
    iv: btoa(Array.from(iv, (b) => String.fromCodePoint(b)).join('')),
    method: 'aes-gcm',
  };
}

/**
 * Decrypt a string using AES-GCM with the session key.
 *
 * @param encrypted - EncryptedData containing ciphertext and IV
 * @returns The decrypted plaintext string
 * @throws Error if decryption fails (e.g. key was lost on page refresh)
 */
export async function decrypt(encrypted: EncryptedData): Promise<string> {
  const key = await getOrCreateKey();
  const iv = Uint8Array.from(atob(encrypted.iv), (c) => c.codePointAt(0) ?? 0);
  const ciphertext = Uint8Array.from(atob(encrypted.data), (c) => c.codePointAt(0) ?? 0);

  // Convert to ArrayBuffer to satisfy type requirements
  const ivBuffer = new ArrayBuffer(iv.byteLength);
  const ivView = new Uint8Array(ivBuffer);
  ivView.set(iv);

  const ciphertextBuffer = new ArrayBuffer(ciphertext.byteLength);
  const ciphertextView = new Uint8Array(ciphertextBuffer);
  ciphertextView.set(ciphertext);

  const decrypted = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: ivBuffer },
    key,
    ciphertextBuffer,
  );

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
