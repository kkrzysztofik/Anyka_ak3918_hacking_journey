/**
 * @file Camera View Page
 * @description Main camera interface with video feed and PTZ controls
 */

import React, { Suspense, lazy } from 'react';
import { useAppSelector } from '../hooks/redux';
import Header from '../components/Header';
import Sidebar from '../components/Sidebar';

// Lazy load camera-specific components
const VideoFeed = lazy(() => import('../components/VideoFeed'));
const PTZControls = lazy(() => import('../components/PTZControls'));

// Loading component for camera components
const CameraLoader: React.FC = () => (
  <div className="flex items-center justify-center p-8">
    <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
    <span className="ml-2 text-gray-400">Loading camera...</span>
  </div>
);

const CameraView: React.FC = () => {
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
          <div className="max-w-6xl mx-auto space-y-6">
            <Suspense fallback={<CameraLoader />}>
              <VideoFeed cameraIP={camera.ip} />
            </Suspense>
            
            <Suspense fallback={<CameraLoader />}>
              <PTZControls />
            </Suspense>
          </div>
        </div>
      </div>
    </div>
  );
};

export default CameraView;
