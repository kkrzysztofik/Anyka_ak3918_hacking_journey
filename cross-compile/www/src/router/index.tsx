/**
 * React Router Configuration
 *
 * Routes with ProtectedRoute wrapper and React.lazy code splitting.
 */

import React, { Suspense, type ReactNode } from 'react'
import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router-dom'
import { useAuth } from '@/hooks/useAuth'
import Layout from '@/Layout'

// Loading fallback for lazy-loaded routes
function LoadingFallback() {
  return (
    <div className="flex items-center justify-center h-full">
      <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary" />
    </div>
  )
}

// Protected route wrapper - redirects to login if not authenticated
function ProtectedRoute({ children }: { children: ReactNode }) {
  const { isAuthenticated } = useAuth()
  const location = useLocation()

  if (!isAuthenticated) {
    return <Navigate to="/login" state={{ from: location }} replace />
  }

  return <>{children}</>
}

// Lazy-loaded page components
const LoginPage = React.lazy(() => import('@/pages/LoginPage'))
const DashboardPage = React.lazy(() => import('@/pages/DashboardPage'))
const LiveViewPage = React.lazy(() => import('@/pages/LiveViewPage'))
const DiagnosticsPage = React.lazy(() => import('@/pages/DiagnosticsPage'))

// Settings pages
const IdentificationPage = React.lazy(() => import('@/pages/settings/IdentificationPage'))
const NetworkPage = React.lazy(() => import('@/pages/settings/NetworkPage'))
const TimePage = React.lazy(() => import('@/pages/settings/TimePage'))
const ImagingPage = React.lazy(() => import('@/pages/settings/ImagingPage'))
const UserManagementPage = React.lazy(() => import('@/pages/settings/UserManagementPage'))
const MaintenancePage = React.lazy(() => import('@/pages/settings/MaintenancePage'))
const ProfilesPage = React.lazy(() => import('@/pages/settings/ProfilesPage'))

function AppRoutes() {
  return (
    <Suspense fallback={<LoadingFallback />}>
      <Routes>
        {/* Public routes */}
        <Route path="/login" element={<LoginPage />} />

        {/* Protected routes with Layout */}
        <Route
          element={
            <ProtectedRoute>
              <Layout />
            </ProtectedRoute>
          }
        >
          <Route path="/" element={<DashboardPage />} />
          <Route path="/live" element={<LiveViewPage />} />
          <Route path="/diagnostics" element={<DiagnosticsPage />} />

          {/* Settings routes */}
          <Route path="/settings">
            <Route index element={<Navigate to="/settings/identification" replace />} />
            <Route path="identification" element={<IdentificationPage />} />
            <Route path="network" element={<NetworkPage />} />
            <Route path="time" element={<TimePage />} />
            <Route path="imaging" element={<ImagingPage />} />
            <Route path="users" element={<UserManagementPage />} />
            <Route path="maintenance" element={<MaintenancePage />} />
            <Route path="profiles" element={<ProfilesPage />} />
          </Route>
        </Route>

        {/* Catch-all redirect */}
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Suspense>
  )
}

export default function AppRouter() {
  return (
    <BrowserRouter>
      <AppRoutes />
    </BrowserRouter>
  )
}

export { ProtectedRoute, LoadingFallback }
