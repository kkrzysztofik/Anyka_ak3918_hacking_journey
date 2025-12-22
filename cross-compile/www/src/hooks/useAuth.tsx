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
  useEffect,
  useMemo,
  useState,
} from 'react';
import {
  decrypt,
  encrypt,
  type EncryptedData,
} from '../utils/crypto';

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
  getCredentials: () => AuthCredentials | null;
  getBasicAuthHeader: () => string | null;
}

const AuthContext = createContext<AuthContextValue | null>(null);

interface AuthProviderProps {
  children: ReactNode;
}

const AUTH_STORAGE_KEY = 'onvif_camera_auth';

export function AuthProvider({ children }: AuthProviderProps) {
  const [credentials, setCredentials] = useState<AuthCredentials | null>(null);

  // Hydrate credentials from sessionStorage on mount
  useEffect(() => {
    const hydrateCredentials = async () => {
      const stored = sessionStorage.getItem(AUTH_STORAGE_KEY);
      if (stored) {
        try {
          const data: StoredAuthData = JSON.parse(stored);
          // Decrypt the password asynchronously
          const password = await decrypt(data.encryptedPassword);
          setCredentials({ username: data.username, password });
        } catch (e) {
          console.error('Failed to decrypt stored credentials', e);
          // Clear invalid stored data
          sessionStorage.removeItem(AUTH_STORAGE_KEY);
        }
      }
    };

    hydrateCredentials();
  }, []);

  const login = useCallback(async (username: string, password: string) => {
    // Encrypt password before storing
    const encryptedPassword = await encrypt(password);
    const creds = { username, password };
    setCredentials(creds);

    // Store username in clear text, password encrypted
    const storedData: StoredAuthData = {
      username,
      encryptedPassword,
    };
    sessionStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(storedData));
  }, []);

  const logout = useCallback(() => {
    setCredentials(null);
    sessionStorage.removeItem(AUTH_STORAGE_KEY);
  }, []);

  const getCredentials = useCallback(() => credentials, [credentials]);

  const getBasicAuthHeader = useCallback(() => {
    if (!credentials) return null;
    const encoded = btoa(`${credentials.username}:${credentials.password}`);
    return `Basic ${encoded}`;
  }, [credentials]);

  const value = useMemo<AuthContextValue>(
    () => ({
      isAuthenticated: credentials !== null,
      username: credentials?.username ?? null,
      login,
      logout,
      getCredentials,
      getBasicAuthHeader,
    }),
    [credentials, login, logout, getCredentials, getBasicAuthHeader],
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
