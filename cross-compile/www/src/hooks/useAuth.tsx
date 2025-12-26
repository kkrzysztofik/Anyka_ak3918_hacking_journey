/* eslint-disable react-refresh/only-export-components */
/**
 * Authentication Context and Hook
 *
 * Manages Basic Auth credentials with encrypted password storage.
 * Passwords are encrypted using AES-GCM before storing in sessionStorage.
 * Provides AuthProvider wrapper and useAuth hook.
 */
import React, {
  type ReactNode,
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
} from 'react';

import { type EncryptedData, decrypt, encrypt } from '../utils/crypto';

interface AuthCredentials {
  username: string;
  password: string;
}

interface StoredAuthData {
  username: string;
  encryptedPassword: EncryptedData;
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

  const login = useCallback(async (username: string, password: string) => {
    // Encrypt password before storing
    const encryptedPassword = await encrypt(password);

    // Store only encrypted data (password never stored in clear text in state)
    const data: StoredAuthData = {
      username,
      encryptedPassword,
    };
    setStoredData(data);
    sessionStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(data));
  }, []);

  const logout = useCallback(() => {
    setStoredData(null);
    sessionStorage.removeItem(AUTH_STORAGE_KEY);
  }, []);

  const getCredentials = useCallback(async (): Promise<AuthCredentials | null> => {
    if (!storedData) return null;
    try {
      // Decrypt password only when needed
      const password = await decrypt(storedData.encryptedPassword);
      return { username: storedData.username, password };
    } catch {
      return null;
    }
  }, [storedData]);

  const getBasicAuthHeader = useCallback(async (): Promise<string | null> => {
    if (!storedData) return null;
    try {
      // Decrypt password only when needed for auth header
      const password = await decrypt(storedData.encryptedPassword);
      const encoded = btoa(`${storedData.username}:${password}`);
      // Password is now in memory temporarily, but will be garbage collected
      return `Basic ${encoded}`;
    } catch {
      return null;
    }
  }, [storedData]);

  const value = useMemo<AuthContextValue>(
    () => ({
      isAuthenticated: storedData !== null,
      username: storedData?.username ?? null,
      login,
      logout,
      getCredentials,
      getBasicAuthHeader,
    }),
    [storedData, login, logout, getCredentials, getBasicAuthHeader],
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
