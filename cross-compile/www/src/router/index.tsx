/**
 * @file Router configuration with code splitting
 * @description Route-based code splitting for better performance
 */

import React, { Suspense, lazy } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';

// Lazy load route components
const CameraView = lazy(() => import('../pages/CameraView'));
const DeviceManagement = lazy(() => import('../pages/DeviceManagement'));
const SystemInfo = lazy(() => import('../pages/SystemInfo'));
const Settings = lazy(() => import('../pages/Settings'));

// Loading component for routes
const RouteLoader: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div className="flex items-center justify-center min-h-screen">
    <div className="text-center">
      <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mx-auto mb-4"></div>
      <div className="text-gray-400">Loading page...</div>
    </div>
  </div>
);

// Error boundary for routes
class RouteErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { hasError: boolean; error?: Error }
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('Route error:', error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex items-center justify-center min-h-screen bg-dark-bg text-white">
          <div className="text-center max-w-md mx-auto p-6">
            <div className="text-red-400 text-6xl mb-4">⚠️</div>
            <h1 className="text-2xl font-bold mb-4">Page Error</h1>
            <p className="text-gray-400 mb-6">
              Something went wrong loading this page. Please try refreshing.
            </p>
            <button
              onClick={() => window.location.reload()}
              className="bg-blue-600 hover:bg-blue-700 px-6 py-2 rounded-lg transition-colors"
            >
              Refresh Page
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

const AppRouter: React.FC = () => {
  return (
    <Router>
      <RouteErrorBoundary>
        <Suspense fallback={<RouteLoader>Loading...</RouteLoader>}>
          <Routes>
            <Route path="/" element={<Navigate to="/camera" replace />} />
            <Route path="/camera" element={<CameraView />} />
            <Route path="/device" element={<DeviceManagement />} />
            <Route path="/system" element={<SystemInfo />} />
            <Route path="/settings" element={<Settings />} />
            <Route path="*" element={<Navigate to="/camera" replace />} />
          </Routes>
        </Suspense>
      </RouteErrorBoundary>
    </Router>
  );
};

export default AppRouter;
