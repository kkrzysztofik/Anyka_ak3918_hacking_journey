/* eslint-disable react-refresh/only-export-components */
/**
 * Authentication Context and Hook
 *
 * Manages Basic Auth credentials in memory only (not localStorage for security).
 * Provides AuthProvider wrapper and useAuth hook.
 */

import React, { createContext, useContext, useState, useCallback, useMemo, type ReactNode } from 'react'

interface AuthCredentials {
  username: string
  password: string
}

interface AuthContextValue {
  isAuthenticated: boolean
  username: string | null
  login: (username: string, password: string) => void
  logout: () => void
  getCredentials: () => AuthCredentials | null
  getBasicAuthHeader: () => string | null
}

const AuthContext = createContext<AuthContextValue | null>(null)

interface AuthProviderProps {
  children: ReactNode
}

const AUTH_STORAGE_KEY = 'onvif_camera_auth'

export function AuthProvider({ children }: AuthProviderProps) {
  const [credentials, setCredentials] = useState<AuthCredentials | null>(() => {
    // Initialize from sessionStorage if available
    const stored = sessionStorage.getItem(AUTH_STORAGE_KEY)
    if (stored) {
      try {
        return JSON.parse(stored)
      } catch (e) {
        console.error('Failed to parse stored credentials', e)
        return null
      }
    }
    return null
  })

  const login = useCallback((username: string, password: string) => {
    const creds = { username, password }
    setCredentials(creds)
    sessionStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(creds))
  }, [])

  const logout = useCallback(() => {
    setCredentials(null)
    sessionStorage.removeItem(AUTH_STORAGE_KEY)
  }, [])

  const getCredentials = useCallback(() => credentials, [credentials])

  const getBasicAuthHeader = useCallback(() => {
    if (!credentials) return null
    const encoded = btoa(`${credentials.username}:${credentials.password}`)
    return `Basic ${encoded}`
  }, [credentials])

  const value = useMemo<AuthContextValue>(() => ({
    isAuthenticated: credentials !== null,
    username: credentials?.username ?? null,
    login,
    logout,
    getCredentials,
    getBasicAuthHeader,
  }), [credentials, login, logout, getCredentials, getBasicAuthHeader])

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}

export { AuthContext }
export type { AuthCredentials, AuthContextValue }
