/**
 * useAuth Hook Tests
 */
import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createEncryptedFixture } from '@/test/componentTestHelpers';

import { type AuthContextValue, AuthProvider, useAuth } from './useAuth';

// Mock crypto utilities
const mockEncrypt = vi.fn();
const mockDecrypt = vi.fn();
const mockClearSessionKey = vi.fn();

vi.mock('../utils/crypto', () => ({
  encrypt: (password: string) => mockEncrypt(password),
  decrypt: (encrypted: unknown) => mockDecrypt(encrypted),
  clearSessionKey: () => mockClearSessionKey(),
}));

// Helper functions to reduce nesting depth
async function loginUser(
  result: { current: AuthContextValue },
  username: string,
  password: string,
) {
  await act(async () => {
    await result.current.login(username, password);
  });
}

async function getCredentialsWithAct(result: { current: AuthContextValue }) {
  return await act(async () => {
    return await result.current.getCredentials();
  });
}

async function getBasicAuthHeaderWithAct(result: { current: AuthContextValue }) {
  return await act(async () => {
    return await result.current.getBasicAuthHeader();
  });
}

function renderHookOutsideProvider() {
  renderHook(() => useAuth());
}

describe('useAuth', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    sessionStorage.clear();
    mockEncrypt.mockResolvedValue(createEncryptedFixture());
    mockDecrypt.mockResolvedValue('decrypted-password');
  });

  afterEach(() => {
    // Clean up any event listeners
    vi.restoreAllMocks();
  });

  describe('AuthProvider', () => {
    it('should provide initial unauthenticated state', () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      expect(result.current.isAuthenticated).toBe(false);
      expect(result.current.username).toBeNull();
    });

    it('should restore authenticated state from sessionStorage', async () => {
      const storedData = {
        username: 'admin',
        encryptedPassword: createEncryptedFixture(),
      };
      sessionStorage.setItem('onvif_camera_auth', JSON.stringify(storedData));

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
        expect(result.current.username).toBe('admin');
      });
    });

    it('should handle invalid JSON in sessionStorage', () => {
      sessionStorage.setItem('onvif_camera_auth', 'invalid-json');

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      expect(result.current.isAuthenticated).toBe(false);
      expect(sessionStorage.getItem('onvif_camera_auth')).toBeNull();
    });

    it('should handle corrupted sessionStorage data', () => {
      sessionStorage.setItem('onvif_camera_auth', '{invalid');

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      expect(result.current.isAuthenticated).toBe(false);
    });
  });

  describe('login', () => {
    it('should login and store encrypted credentials', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await loginUser(result, 'testuser', 'testpassword');

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
        expect(result.current.username).toBe('testuser');
        expect(mockEncrypt).toHaveBeenCalledWith('testpassword');
      });

      const stored = sessionStorage.getItem('onvif_camera_auth');
      expect(stored).toBeTruthy();
      const parsed = JSON.parse(stored!);
      expect(parsed.username).toBe('testuser');
      expect(parsed.encryptedPassword).toBeDefined();
    });

    it('should fall back to memory-only auth when crypto fails (HTTP)', async () => {
      mockEncrypt.mockRejectedValue(new Error('Credential storage requires HTTPS'));

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await loginUser(result, 'testuser', 'testpassword');

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
        expect(result.current.username).toBe('testuser');
      });

      // Should NOT be persisted to sessionStorage
      expect(sessionStorage.getItem('onvif_camera_auth')).toBeNull();
    });

    it('should provide credentials from memory-only auth', async () => {
      mockEncrypt.mockRejectedValue(new Error('Credential storage requires HTTPS'));

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await loginUser(result, 'testuser', 'testpassword');

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      const credentials = await getCredentialsWithAct(result);
      expect(credentials).toEqual({
        username: 'testuser',
        password: 'testpassword',
      });
      // Decrypt should NOT be called for memory-only auth
      expect(mockDecrypt).not.toHaveBeenCalled();
    });

    it('should provide basic auth header from memory-only auth', async () => {
      mockEncrypt.mockRejectedValue(new Error('Credential storage requires HTTPS'));

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await loginUser(result, 'testuser', 'testpassword');

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      const header = await getBasicAuthHeaderWithAct(result);
      expect(header).toBe('Basic ' + btoa('testuser:testpassword'));
      expect(mockDecrypt).not.toHaveBeenCalled();
    });
  });

  describe('logout', () => {
    it('should logout and clear sessionStorage', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await loginUser(result, 'testuser', 'testpassword');

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      act(() => {
        result.current.logout();
      });

      expect(result.current.isAuthenticated).toBe(false);
      expect(result.current.username).toBeNull();
      expect(sessionStorage.getItem('onvif_camera_auth')).toBeNull();
      expect(mockClearSessionKey).toHaveBeenCalled();
    });

    it('should logout on auth:unauthorized event', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await loginUser(result, 'testuser', 'testpassword');

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      act(() => {
        globalThis.dispatchEvent(new CustomEvent('auth:unauthorized'));
      });

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(false);
        expect(result.current.username).toBeNull();
        expect(mockClearSessionKey).toHaveBeenCalled();
      });
    });
  });

  describe('getCredentials', () => {
    it('should return credentials when authenticated', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await loginUser(result, 'testuser', 'testpassword');

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      const credentials = await getCredentialsWithAct(result);

      expect(credentials).toEqual({
        username: 'testuser',
        password: 'decrypted-password',
      });
      expect(mockDecrypt).toHaveBeenCalled();
    });

    it('should return null when not authenticated', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      const credentials = await getCredentialsWithAct(result);

      expect(credentials).toBeNull();
      expect(mockDecrypt).not.toHaveBeenCalled();
    });

    it('should return null and clear state when decryption fails', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await loginUser(result, 'testuser', 'testpassword');

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      mockDecrypt.mockRejectedValue(new Error('Decryption failed'));

      const credentials = await getCredentialsWithAct(result);

      expect(credentials).toBeNull();
      // Should clear stale sessionStorage data
      expect(sessionStorage.getItem('onvif_camera_auth')).toBeNull();
    });
  });

  describe('getBasicAuthHeader', () => {
    it('should return Basic auth header when authenticated', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await loginUser(result, 'testuser', 'testpassword');

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      const header = await getBasicAuthHeaderWithAct(result);

      expect(header).toBe('Basic ' + btoa('testuser:decrypted-password'));
      expect(mockDecrypt).toHaveBeenCalled();
    });

    it('should return null when not authenticated', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      const header = await getBasicAuthHeaderWithAct(result);

      expect(header).toBeNull();
      expect(mockDecrypt).not.toHaveBeenCalled();
    });

    it('should return null and clear state when decryption fails', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await loginUser(result, 'testuser', 'testpassword');

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      mockDecrypt.mockRejectedValue(new Error('Decryption failed'));

      const header = await getBasicAuthHeaderWithAct(result);

      expect(header).toBeNull();
      expect(sessionStorage.getItem('onvif_camera_auth')).toBeNull();
    });
  });

  describe('useAuth hook error handling', () => {
    it('should throw error when used outside AuthProvider', () => {
      // Suppress console.error for this test
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      expect(renderHookOutsideProvider).toThrow('useAuth must be used within an AuthProvider');

      consoleSpy.mockRestore();
    });
  });
});
