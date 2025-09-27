import React from 'react';
import { useAppDispatch, useAppSelector } from '../hooks/redux';
import { initializePTZ, stopPTZ, movePTZ } from '../store/slices/onvifSlice';
import { createCameraConfig } from '../config/cameraConfig';
import type { CameraConfig } from '../types';

const PTZControls: React.FC = () => {
  const dispatch = useAppDispatch();
  const camera = useAppSelector((state) => state.camera);

  const getCameraConfig = (): CameraConfig => createCameraConfig(camera.ip);

  const handlePTZAction = async (action: 'init' | 'stop' | 'move', direction?: string): Promise<void> => {
    const config = getCameraConfig();
    
    try {
      switch (action) {
        case 'init':
          await dispatch(initializePTZ(config)).unwrap();
          console.log('PTZ initialized successfully');
          break;
        case 'stop':
          await dispatch(stopPTZ(config)).unwrap();
          console.log('PTZ stopped');
          break;
        case 'move':
          if (direction) {
            await dispatch(movePTZ({ config, direction })).unwrap();
            console.log(`PTZ moved ${direction}`);
          }
          break;
        default:
          break;
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      console.error(`PTZ action failed: ${errorMessage}`);
    }
  };


  const handleDirectionClick = (direction: string) => {
    handlePTZAction('move', direction);
  };

  return (
    <div className="card">
      <h2 className="text-xl font-semibold mb-4">PTZ Controls</h2>
      
      <div className="flex justify-center mb-6">
        <button
          onClick={() => handlePTZAction('init')}
          className="px-6 py-2 bg-accent-red hover:bg-red-600 text-white rounded-lg transition-colors mr-4"
        >
          Initialize PTZ
        </button>
        <button
          onClick={() => handlePTZAction('stop')}
          className="px-6 py-2 bg-gray-600 hover:bg-gray-500 text-white rounded-lg transition-colors"
        >
          Stop PTZ
        </button>
      </div>
      
      {/* Main Direction Pad */}
      <div className="flex justify-center">
        <div className="grid grid-cols-3 gap-2">
          {/* Top row */}
          <div></div>
          <button
            onClick={() => handleDirectionClick('up')}
            className="ptz-button bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
            title="Move Up"
          >
            ↑
          </button>
          <div></div>
          
          {/* Middle row */}
          <button
            onClick={() => handleDirectionClick('left')}
            className="ptz-button bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
            title="Move Left"
          >
            ←
          </button>
          <div className="ptz-button bg-gray-800 rounded-lg flex items-center justify-center text-sm font-semibold">
            PTZ
          </div>
          <button
            onClick={() => handleDirectionClick('right')}
            className="ptz-button bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
            title="Move Right"
          >
            →
          </button>
          
          {/* Bottom row */}
          <div></div>
          <button
            onClick={() => handleDirectionClick('down')}
            className="ptz-button bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
            title="Move Down"
          >
            ↓
          </button>
          <div></div>
        </div>
      </div>
      
      {/* Diagonal Controls */}
      <div className="flex justify-center mt-4">
        <div className="grid grid-cols-3 gap-2">
          <button
            onClick={() => handleDirectionClick('left_up')}
            className="ptz-button bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors text-sm"
            title="Move Left Up"
          >
            ↖
          </button>
          <div></div>
          <button
            onClick={() => handleDirectionClick('right_up')}
            className="ptz-button bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors text-sm"
            title="Move Right Up"
          >
            ↗
          </button>
          
          <div></div>
          <div></div>
          <div></div>
          
          <button
            onClick={() => handleDirectionClick('left_down')}
            className="ptz-button bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors text-sm"
            title="Move Left Down"
          >
            ↙
          </button>
          <div></div>
          <button
            onClick={() => handleDirectionClick('right_down')}
            className="ptz-button bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors text-sm"
            title="Move Right Down"
          >
            ↘
          </button>
        </div>
      </div>
    </div>
  );
};

export default PTZControls;
