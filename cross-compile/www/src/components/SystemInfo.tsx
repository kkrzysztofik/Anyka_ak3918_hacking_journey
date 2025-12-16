import React from 'react';
import { Wifi, WifiOff } from 'lucide-react';
import type { SystemInfoProps } from '../types';

const SystemInfo: React.FC<SystemInfoProps> = ({ cameraIP, onvifStatus, onStatusCheck }) => {
  const endpoints = [
    { label: 'RTSP Main', url: `rtsp://${cameraIP}:554/vs0` },
    { label: 'RTSP Sub', url: `rtsp://${cameraIP}:554/vs1` },
    { label: 'Snapshot', url: `http://${cameraIP}:3000/snapshot.jpeg` },
    { label: 'ONVIF Device', url: `http://${cameraIP}:8080/onvif/device_service` },
    { label: 'ONVIF PTZ', url: `http://${cameraIP}:8080/onvif/ptz_service` },
    { label: 'ONVIF Media', url: `http://${cameraIP}:8080/onvif/media_service` },
  ];

  const getStatusIcon = () => {
    switch (onvifStatus) {
      case 'online':
        return <Wifi className="w-4 h-4 text-green-500" />;
      case 'offline':
        return <WifiOff className="w-4 h-4 text-red-500" />;
      default:
        return <div className="w-4 h-4 border-2 border-accent-red border-t-transparent rounded-full animate-spin" />;
    }
  };

  const getStatusText = () => {
    switch (onvifStatus) {
      case 'online':
        return 'ONVIF Server Online';
      case 'offline':
        return 'ONVIF Server Offline';
      default:
        return 'Checking status...';
    }
  };

  const getStatusColor = () => {
    switch (onvifStatus) {
      case 'online':
        return 'text-green-400';
      case 'offline':
        return 'text-red-400';
      default:
        return 'text-gray-400';
    }
  };

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      {/* Endpoints */}
      <div className="card">
        <h2 className="text-xl font-semibold mb-4">Endpoints</h2>
        <div className="space-y-3 text-sm">
          {endpoints.map((endpoint, index) => (
            <div key={index} className="flex justify-between">
              <span className="text-gray-400">{endpoint.label}:</span>
              <code className="text-blue-400 break-all">{endpoint.url}</code>
            </div>
          ))}
        </div>
      </div>

      {/* ONVIF Status */}
      <div className="card">
        <h2 className="text-xl font-semibold mb-4">ONVIF Status</h2>
        <div className="flex items-center space-x-2 mb-6">
          {getStatusIcon()}
          <span className={getStatusColor()}>{getStatusText()}</span>
        </div>

        <div className="mb-4">
          <button
            onClick={onStatusCheck}
            className="px-4 py-2 bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors text-sm"
          >
            Refresh Status
          </button>
        </div>

      </div>
    </div>
  );
};

export default SystemInfo;
