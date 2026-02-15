/* eslint-disable react-refresh/only-export-components */
/**
 * Authentication Context and Hook
 *
 * Manages Basic Auth credentials with encrypted password storage.
 * Passwords are encrypted using AES-GCM before storing in sessionStorage.
 * Provides AuthProvider wrapper and useAuth hook.
 *
 * When crypto is unavailable (HTTP), credentials are kept in React state
 * only — they are not persisted to sessionStorage.
 */
import React, {
  type ReactNode,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';

import { type EncryptedData, clearSessionKey, decrypt, encrypt } from '../utils/crypto';

interface AuthCredentials {
  username: string;
  password: string;
}

interface StoredAuthData {
  username: string;
  encryptedPassword: EncryptedData;
}

/**
 * In-memory-only auth state for when persistence is unavailable (HTTP context).
 * The plaintext password lives only in React state and is lost on refresh.
 */
interface MemoryOnlyAuth {
  username: string;
  password: string;
}

interface AuthContextValue {
  isAuthenticated: boolean;
  username: string | null;
  login: (username: string, password: string) => Promise<void>;
  logout: () => void;
  getCredentials: () => Promise<AuthCredentials | null>;
  getBasicAuthHeader: () => Promise<string | null>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

interface AuthProviderProps {
  readonly children: ReactNode;
}

const AUTH_STORAGE_KEY = 'onvif_camera_auth';

// Helper function to hydrate initial state from sessionStorage
function getInitialStoredData(): StoredAuthData | null {
  const stored = sessionStorage.getItem(AUTH_STORAGE_KEY);
  if (stored) {
    try {
      return JSON.parse(stored) as StoredAuthData;
    } catch {
      // Clear invalid stored data
      sessionStorage.removeItem(AUTH_STORAGE_KEY);
      return null;
    }
  }
  return null;
}

export function AuthProvider({ children }: AuthProviderProps) {
  // Use lazy initialization to avoid setState in effect
  const [storedData, setStoredData] = useState<StoredAuthData | null>(getInitialStoredData);
  const [memoryAuth, setMemoryAuth] = useState<MemoryOnlyAuth | null>(null);

  // Listen for 401 unauthorized events from the API interceptor
  const logout = useCallback(() => {
    setStoredData(null);
    setMemoryAuth(null);
    sessionStorage.removeItem(AUTH_STORAGE_KEY);
    clearSessionKey();
  }, []);

  useEffect(() => {
    const handler = () => logout();
    window.addEventListener('auth:unauthorized', handler);
    return () => window.removeEventListener('auth:unauthorized', handler);
  }, [logout]);

  const login = useCallback(async (username: string, password: string) => {
    try {
      // Encrypt password before storing
      const encryptedPassword = await encrypt(password);

      // Store only encrypted data (password never stored in clear text in state)
      const data: StoredAuthData = {
        username,
        encryptedPassword,
      };
      setStoredData(data);
      setMemoryAuth(null);
      sessionStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(data));
    } catch {
      // Crypto unavailable (HTTP) — keep credentials in memory only
      setMemoryAuth({ username, password });
      setStoredData(null);
    }
  }, []);

  const getCredentials = useCallback(async (): Promise<AuthCredentials | null> => {
    // Memory-only auth (HTTP context)
    if (memoryAuth) {
      return { username: memoryAuth.username, password: memoryAuth.password };
    }
    if (!storedData) return null;
    try {
      // Decrypt password only when needed
      const password = await decrypt(storedData.encryptedPassword);
      return { username: storedData.username, password };
    } catch {
      // Key was lost (page refresh) — clear stale data, user must re-login
      sessionStorage.removeItem(AUTH_STORAGE_KEY);
      setStoredData(null);
      return null;
    }
  }, [storedData, memoryAuth]);

  const getBasicAuthHeader = useCallback(async (): Promise<string | null> => {
    // Memory-only auth (HTTP context)
    if (memoryAuth) {
      const encoded = btoa(`${memoryAuth.username}:${memoryAuth.password}`);
      return `Basic ${encoded}`;
    }
    if (!storedData) return null;
    try {
      // Decrypt password only when needed for auth header
      const password = await decrypt(storedData.encryptedPassword);
      const encoded = btoa(`${storedData.username}:${password}`);
      return `Basic ${encoded}`;
    } catch {
      // Key was lost (page refresh) — clear stale data
      sessionStorage.removeItem(AUTH_STORAGE_KEY);
      setStoredData(null);
      return null;
    }
  }, [storedData, memoryAuth]);

  const value = useMemo<AuthContextValue>(
    () => ({
      isAuthenticated: storedData !== null || memoryAuth !== null,
      username: storedData?.username ?? memoryAuth?.username ?? null,
      login,
      logout,
      getCredentials,
      getBasicAuthHeader,
    }),
    [storedData, memoryAuth, login, logout, getCredentials, getBasicAuthHeader],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}

export { AuthContext };
export type { AuthCredentials, AuthContextValue };
