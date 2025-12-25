/**
 * useAuth Hook Tests
 */
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { AuthProvider, useAuth } from './useAuth';

// Mock crypto utilities
const mockEncrypt = vi.fn();
const mockDecrypt = vi.fn();

vi.mock('../utils/crypto', () => ({
  encrypt: (password: string) => mockEncrypt(password),
  decrypt: (encrypted: unknown) => mockDecrypt(encrypted),
}));

describe('useAuth', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    sessionStorage.clear();
    mockEncrypt.mockResolvedValue({ iv: 'test-iv', data: 'test-data', tag: 'test-tag' });
    mockDecrypt.mockResolvedValue('decrypted-password');
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
        encryptedPassword: { iv: 'test-iv', data: 'test-data', tag: 'test-tag' },
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

      await act(async () => {
        await result.current.login('testuser', 'testpassword');
      });

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
  });

  describe('logout', () => {
    it('should logout and clear sessionStorage', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await act(async () => {
        await result.current.login('testuser', 'testpassword');
      });

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      act(() => {
        result.current.logout();
      });

      expect(result.current.isAuthenticated).toBe(false);
      expect(result.current.username).toBeNull();
      expect(sessionStorage.getItem('onvif_camera_auth')).toBeNull();
    });
  });

  describe('getCredentials', () => {
    it('should return credentials when authenticated', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await act(async () => {
        await result.current.login('testuser', 'testpassword');
      });

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      const credentials = await act(async () => {
        return await result.current.getCredentials();
      });

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

      const credentials = await act(async () => {
        return await result.current.getCredentials();
      });

      expect(credentials).toBeNull();
      expect(mockDecrypt).not.toHaveBeenCalled();
    });

    it('should return null when decryption fails', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await act(async () => {
        await result.current.login('testuser', 'testpassword');
      });

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      mockDecrypt.mockRejectedValue(new Error('Decryption failed'));

      const credentials = await act(async () => {
        return await result.current.getCredentials();
      });

      expect(credentials).toBeNull();
    });
  });

  describe('getBasicAuthHeader', () => {
    it('should return Basic auth header when authenticated', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await act(async () => {
        await result.current.login('testuser', 'testpassword');
      });

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      const header = await act(async () => {
        return await result.current.getBasicAuthHeader();
      });

      expect(header).toBe('Basic ' + btoa('testuser:decrypted-password'));
      expect(mockDecrypt).toHaveBeenCalled();
    });

    it('should return null when not authenticated', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      const header = await act(async () => {
        return await result.current.getBasicAuthHeader();
      });

      expect(header).toBeNull();
      expect(mockDecrypt).not.toHaveBeenCalled();
    });

    it('should return null when decryption fails', async () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await act(async () => {
        await result.current.login('testuser', 'testpassword');
      });

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      mockDecrypt.mockRejectedValue(new Error('Decryption failed'));

      const header = await act(async () => {
        return await result.current.getBasicAuthHeader();
      });

      expect(header).toBeNull();
    });
  });

  describe('useAuth hook error handling', () => {
    it('should throw error when used outside AuthProvider', () => {
      // Suppress console.error for this test
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      expect(() => {
        renderHook(() => useAuth());
      }).toThrow('useAuth must be used within an AuthProvider');

      consoleSpy.mockRestore();
    });
  });
});
