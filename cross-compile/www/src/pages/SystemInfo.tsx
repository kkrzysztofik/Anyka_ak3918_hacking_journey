/**
 * @file System Info Page
 * @description System information and status display
 */

import React, { Suspense, lazy } from 'react';
import { useAppDispatch, useAppSelector } from '../hooks/redux';
import { checkONVIFStatus } from '../store/slices/onvifSlice';
import { createCameraConfig } from '../config/cameraConfig';
import Header from '../components/Header';
import Sidebar from '../components/Sidebar';

// Lazy load system components
const SystemInfoComponent = lazy(() => import('../components/SystemInfo'));

// Loading component for system components
const SystemLoader: React.FC = () => (
  <div className="flex items-center justify-center p-8">
    <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
    <span className="ml-2 text-gray-400">Loading system info...</span>
  </div>
);

const SystemInfoPage: React.FC = () => {
  const dispatch = useAppDispatch();
  const camera = useAppSelector((state) => state.camera);
  const onvif = useAppSelector((state) => state.onvif);
  const ui = useAppSelector((state) => state.ui);

  const handleStatusCheck = () => {
    const cameraConfig = createCameraConfig(camera.ip);
    dispatch(checkONVIFStatus(cameraConfig));
  };

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
            <Suspense fallback={<SystemLoader>Loading system info...</SystemLoader>}>
              <SystemInfoComponent 
                cameraIP={camera.ip}
                onvifStatus={onvif.status}
                onStatusCheck={handleStatusCheck}
              />
            </Suspense>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SystemInfoPage;
