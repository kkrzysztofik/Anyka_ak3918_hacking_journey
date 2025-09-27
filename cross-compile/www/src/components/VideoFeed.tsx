import React, { useState, useEffect } from 'react';
import { Play, Pause, RotateCcw, Maximize, Camera } from '../utils/icons';
import { useAppDispatch, useAppSelector } from '../hooks/redux';
import { setSnapshotUrl, togglePlayPause } from '../store/slices/cameraSlice';
import type { VideoFeedProps } from '../types';

const VideoFeed: React.FC<VideoFeedProps> = ({ cameraIP }) => {
  const dispatch = useAppDispatch();
  const { isPlaying, snapshotUrl } = useAppSelector((state) => state.camera);
  const [isFullscreen, setIsFullscreen] = useState(false);

  useEffect(() => {
    let interval: number | null = null;
    
    if (isPlaying) {
      updateSnapshot();
      interval = setInterval(updateSnapshot, 300);
    }
    
    return () => {
      if (interval) {
        clearInterval(interval);
      }
    };
  }, [isPlaying, cameraIP]);

  const updateSnapshot = () => {
    const timestamp = Math.round(Math.random() * 100000);
    const newUrl = `http://${cameraIP}:3000/${timestamp}.jpeg`;
    dispatch(setSnapshotUrl(newUrl));
  };

  const handleTogglePlayPause = () => {
    dispatch(togglePlayPause());
  };

  const handleRefreshSnapshot = () => {
    updateSnapshot();
  };

  const handleToggleFullscreen = () => {
    const videoContainer = document.querySelector('.video-container');
    if (videoContainer) {
      if (!isFullscreen) {
        if (videoContainer.requestFullscreen) {
          videoContainer.requestFullscreen();
          setIsFullscreen(true);
        }
      } else {
        if (document.exitFullscreen) {
          document.exitFullscreen();
          setIsFullscreen(false);
        }
      }
    }
  };

  return (
    <div className="card">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold">Live Feed</h2>
        <div className="flex space-x-2">
          <button
            onClick={handleTogglePlayPause}
            className="p-2 bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
            title={isPlaying ? 'Pause' : 'Play'}
          >
            {isPlaying ? <Pause className="w-5 h-5" /> : <Play className="w-5 h-5" />}
          </button>
          <button
            onClick={handleRefreshSnapshot}
            className="p-2 bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
            title="Refresh"
          >
            <RotateCcw className="w-5 h-5" />
          </button>
          <button
            onClick={handleToggleFullscreen}
            className="p-2 bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
            title="Fullscreen"
          >
            <Maximize className="w-5 h-5" />
          </button>
        </div>
      </div>
      
      <div className="relative bg-black rounded-lg overflow-hidden video-container">
        {snapshotUrl ? (
          <img
            src={snapshotUrl}
            alt="Camera Feed"
            className="w-full h-auto max-h-96 object-contain"
            onError={(e) => {
              e.currentTarget.style.display = 'none';
            }}
          />
        ) : (
          <div className="w-full h-96 bg-gray-800 flex items-center justify-center">
            <div className="text-center">
              <Camera className="w-16 h-16 mx-auto text-gray-600 mb-4" />
              <p className="text-gray-400">Camera Feed Loading...</p>
            </div>
          </div>
        )}
        <div className="absolute top-4 left-4 bg-black/50 px-3 py-1 rounded text-sm">
          Kamera
        </div>
      </div>
    </div>
  );
};

export default VideoFeed;
