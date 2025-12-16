/**
 * Main Application Component
 *
 * Root component with QueryClientProvider, AuthProvider, and Router.
 */

import React, { useEffect } from 'react'
import { QueryClientProvider } from '@tanstack/react-query'
import { Toaster } from '@/components/ui/sonner'
import { queryClient } from '@/lib/queryClient'
import { AuthProvider, useAuth } from '@/hooks/useAuth'
import { setAuthHeaderGetter } from '@/services/api'
import AppRouter from '@/router'
import '@/styles/globals.css'

/**
 * Auth integration - connects useAuth to API client interceptor
 */
function AuthIntegration({ children }: { children: React.ReactNode }) {
  const { getBasicAuthHeader } = useAuth()

  useEffect(() => {
    // Register auth header getter with API client
    setAuthHeaderGetter(getBasicAuthHeader)
  }, [getBasicAuthHeader])

  return <>{children}</>
}

function App(): React.ReactElement {
  return (
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <AuthIntegration>
          <AppRouter />
          <Toaster position="top-right" richColors />
        </AuthIntegration>
      </AuthProvider>
    </QueryClientProvider>
  )
}

export default App
