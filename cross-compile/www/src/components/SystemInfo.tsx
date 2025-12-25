import React from 'react';

import { Wifi, WifiOff } from 'lucide-react';

import type { SystemInfoProps } from '../types';

const SystemInfo: React.FC<SystemInfoProps> = ({ cameraIP, onvifStatus, onStatusCheck }) => {
  const endpoints = [
    { label: 'RTSP Main', url: `rtsp://${cameraIP}:554/vs0` },
    { label: 'RTSP Sub', url: `rtsp://${cameraIP}:554/vs1` },
    { label: 'Snapshot', url: `http://${cameraIP}:3000/snapshot.jpeg` },
    {
      label: 'ONVIF Device',
      url: `http://${cameraIP}:8080/onvif/device_service`,
    },
    { label: 'ONVIF PTZ', url: `http://${cameraIP}:8080/onvif/ptz_service` },
    {
      label: 'ONVIF Media',
      url: `http://${cameraIP}:8080/onvif/media_service`,
    },
  ];

  const getStatusIcon = () => {
    switch (onvifStatus) {
      case 'online':
        return <Wifi className="h-4 w-4 text-green-500" />;
      case 'offline':
        return <WifiOff className="h-4 w-4 text-red-500" />;
      default:
        return (
          <div
            className="border-accent-red h-4 w-4 animate-spin rounded-full border-2 border-t-transparent"
            data-testid="system-info-loading-spinner"
          />
        );
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
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
      {/* Endpoints */}
      <div className="card" data-testid="system-info-endpoints-card">
        <h2 className="mb-4 text-xl font-semibold" data-testid="system-info-endpoints-title">
          Endpoints
        </h2>
        <div className="space-y-3 text-sm" data-testid="system-info-endpoints-list">
          {endpoints.map((endpoint, index) => (
            <div
              key={endpoint.label}
              className="flex justify-between"
              data-testid={`system-info-endpoint-${index}`}
            >
              <span className="text-gray-400">{endpoint.label}:</span>
              <code
                className="break-all text-blue-400"
                data-testid={`system-info-endpoint-${index}-url`}
              >
                {endpoint.url}
              </code>
            </div>
          ))}
        </div>
      </div>

      {/* ONVIF Status */}
      <div className="card" data-testid="system-info-status-card">
        <h2 className="mb-4 text-xl font-semibold" data-testid="system-info-status-title">
          ONVIF Status
        </h2>
        <div className="mb-6 flex items-center space-x-2" data-testid="system-info-status-display">
          {getStatusIcon()}
          <span className={getStatusColor()} data-testid="system-info-status-text">
            {getStatusText()}
          </span>
        </div>

        <div className="mb-4">
          <button
            onClick={onStatusCheck}
            className="rounded-lg bg-gray-700 px-4 py-2 text-sm transition-colors hover:bg-gray-600"
            data-testid="system-info-refresh-button"
          >
            Refresh Status
          </button>
        </div>
      </div>
    </div>
  );
};

export default SystemInfo;
