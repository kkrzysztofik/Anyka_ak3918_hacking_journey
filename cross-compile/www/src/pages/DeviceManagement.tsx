/**
 * @file Device Management Page
 * @description Device configuration and management interface
 */

import React, { Suspense, lazy } from 'react';
import { useAppSelector } from '../hooks/redux';
import Header from '../components/Header';
import Sidebar from '../components/Sidebar';

// Lazy load device management components
const DeviceService = lazy(() => import('../components/DeviceService'));

// Loading component for device components
const DeviceLoader: React.FC = () => (
  <div className="flex items-center justify-center p-8">
    <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
    <span className="ml-2 text-gray-400">Loading device management...</span>
  </div>
);

const DeviceManagement: React.FC = () => {
  const camera = useAppSelector((state) => state.camera);
  const ui = useAppSelector((state) => state.ui);

  return (
    <div className="flex h-screen bg-dark-bg text-white">
      <Sidebar />
      
      <div className="flex-1 flex flex-col overflow-hidden">
        <Header 
          cameraName={camera.name} 
          cameraType={camera.type}
          currentTime={ui.currentTime}
        />
        
        <div className="flex-1 p-6 overflow-y-auto">
          <div className="max-w-6xl mx-auto">
            <Suspense fallback={<DeviceLoader>Loading device management...</DeviceLoader>}>
              <DeviceService />
            </Suspense>
          </div>
        </div>
      </div>
    </div>
  );
};

export default DeviceManagement;
