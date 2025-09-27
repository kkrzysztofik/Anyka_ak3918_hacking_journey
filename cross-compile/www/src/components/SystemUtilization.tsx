import React, { useEffect } from 'react';
import { 
  Cpu, 
  MemoryStick, 
  Clock, 
  RefreshCw,
  Activity
} from '../utils/icons';
import { useAppDispatch, useAppSelector } from '../hooks/redux';
import { 
  fetchSystemInfo, 
  toggleAutoRefresh
} from '../store/slices/systemUtilizationSlice';
import { SystemUtilizationUtils } from '../services/systemUtilizationService';

const SystemUtilization: React.FC = () => {
  const dispatch = useAppDispatch();
  
  // Get state from Redux store
  const {
    systemInfo,
    cpuHistory,
    tempHistory,
    isLoading,
    error,
    isAutoRefresh
  } = useAppSelector((state) => state.systemUtilization);
  
  // Get camera IP from Redux store
  const cameraIp = useAppSelector((state) => state.camera.ip);

  const handleFetchSystemInfo = () => {
    dispatch(fetchSystemInfo(cameraIp));
  };

  const handleToggleAutoRefresh = () => {
    dispatch(toggleAutoRefresh());
  };


  useEffect(() => {
    // Initial fetch
    handleFetchSystemInfo();
    
    if (isAutoRefresh) {
      const interval = setInterval(handleFetchSystemInfo, 3000); // Update every 3 seconds
      return () => clearInterval(interval);
    }
  }, [isAutoRefresh, cameraIp]);

  // Use utility functions from the service
  const formatBytes = SystemUtilizationUtils.formatBytes;
  const formatUptime = SystemUtilizationUtils.formatUptime;
  const generateChartData = SystemUtilizationUtils.generateChartData;

  const renderChart = (data: any[], color: string) => {
    if (data.length < 2) return null;

    const pathData = data.map((point, index) => 
      `${index === 0 ? 'M' : 'L'} ${point.x} ${100 - point.y}`
    ).join(' ');

    return (
      <div className="relative h-32 w-full">
        <svg className="w-full h-full" viewBox="0 0 100 100" preserveAspectRatio="none">
          <defs>
            <linearGradient id={`gradient-${color}`} x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor={color} stopOpacity="0.3" />
              <stop offset="100%" stopColor={color} stopOpacity="0.1" />
            </linearGradient>
          </defs>
          <path
            d={`${pathData} L 100 100 L 0 100 Z`}
            fill={`url(#gradient-${color})`}
          />
          <path
            d={pathData}
            fill="none"
            stroke={color}
            strokeWidth="0.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
          {data.map((point, index) => (
            <circle
              key={index}
              cx={point.x}
              cy={100 - point.y}
              r="0.5"
              fill={color}
            />
          ))}
        </svg>
        <div className="absolute top-0 left-0 right-0 flex justify-between text-xs text-gray-400">
          <span>{data[0]?.time}</span>
          <span>{data[data.length - 1]?.time}</span>
        </div>
      </div>
    );
  };

  if (error) {
    return (
      <div className="space-y-8">
        <div className="bg-red-900 border border-red-700 rounded-lg p-4">
          <div className="flex items-center">
            <Activity className="h-5 w-5 text-red-400 mr-2" />
            <span className="text-red-200">Error: {error}</span>
          </div>
        </div>
        <button
          onClick={handleFetchSystemInfo}
          className="inline-flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors"
        >
          <RefreshCw className="h-4 w-4 mr-2" />
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {/* Header Controls */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">System Utilization</h2>
        <div className="flex items-center gap-4">
          <button
            onClick={handleToggleAutoRefresh}
            className={`inline-flex items-center px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
              isAutoRefresh 
                ? 'bg-green-600 hover:bg-green-700 text-white' 
                : 'bg-gray-600 hover:bg-gray-700 text-white'
            }`}
          >
            <Activity className="h-4 w-4 mr-2" />
            {isAutoRefresh ? 'Auto Refresh ON' : 'Auto Refresh OFF'}
          </button>
          <button
            onClick={handleFetchSystemInfo}
            disabled={isLoading}
            className="inline-flex items-center px-3 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white text-sm rounded-lg transition-colors"
          >
            <RefreshCw className={`h-4 w-4 mr-2 ${isLoading ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </div>
      </div>

      {/* CPU Utilization Section */}
      <div className="bg-gray-800 rounded-lg p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-white">CPU Utilization</h3>
          <div className="flex items-center gap-4">
            <div className="text-right">
              <div className="text-2xl font-bold text-red-400">
                {systemInfo ? `${systemInfo.cpu_usage.toFixed(1)}%` : '--'}
              </div>
              <div className="text-xs text-gray-400">Current</div>
            </div>
          </div>
        </div>
        
        <div className="mb-4">
          {renderChart(generateChartData(cpuHistory), '#ef4444')}
        </div>
        
        <div className="flex items-center justify-between text-sm text-gray-400">
          <span>0%</span>
          <span>50%</span>
          <span>100%</span>
        </div>
      </div>

      {/* CPU Temperature Section */}
      <div className="bg-gray-800 rounded-lg p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-white">CPU Temperature</h3>
          <div className="flex items-center gap-4">
            <div className="text-right">
              <div className="text-2xl font-bold text-red-400">
                {systemInfo ? `${systemInfo.cpu_temperature.toFixed(1)}°` : '--'}
              </div>
              <div className="text-xs text-gray-400">Current</div>
            </div>
          </div>
        </div>
        
        <div className="mb-4">
          {renderChart(generateChartData(tempHistory), '#ef4444')}
        </div>
        
        <div className="flex items-center justify-between text-sm text-gray-400">
          <span>0°</span>
          <span>50°</span>
          <span>100°</span>
        </div>
      </div>

      {/* System Information Grid */}
      {systemInfo && (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {/* Memory Usage */}
          <div className="bg-gray-800 rounded-lg p-6">
            <div className="flex items-center mb-4">
              <MemoryStick className="h-5 w-5 text-blue-400 mr-2" />
              <h3 className="text-lg font-semibold text-white">Memory Usage</h3>
            </div>
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Used</span>
                <span className="text-white">{formatBytes(systemInfo.memory_used)}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Free</span>
                <span className="text-white">{formatBytes(systemInfo.memory_free)}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Total</span>
                <span className="text-white">{formatBytes(systemInfo.memory_total)}</span>
              </div>
              <div className="w-full bg-gray-700 rounded-full h-2 mt-3">
                <div 
                  className="bg-blue-500 h-2 rounded-full" 
                  style={{ 
                    width: `${(systemInfo.memory_used / systemInfo.memory_total) * 100}%` 
                  }}
                ></div>
              </div>
            </div>
          </div>

          {/* System Uptime */}
          <div className="bg-gray-800 rounded-lg p-6">
            <div className="flex items-center mb-4">
              <Clock className="h-5 w-5 text-green-400 mr-2" />
              <h3 className="text-lg font-semibold text-white">System Uptime</h3>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-green-400">
                {formatUptime(systemInfo.uptime_ms)}
              </div>
              <div className="text-sm text-gray-400 mt-1">Since last boot</div>
            </div>
          </div>

          {/* System Status */}
          <div className="bg-gray-800 rounded-lg p-6">
            <div className="flex items-center mb-4">
              <Cpu className="h-5 w-5 text-purple-400 mr-2" />
              <h3 className="text-lg font-semibold text-white">System Status</h3>
            </div>
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm text-gray-400">CPU Load</span>
                <span className={`text-sm font-medium ${SystemUtilizationUtils.getCpuLoadStatus(systemInfo.cpu_usage).color}`}>
                  {SystemUtilizationUtils.getCpuLoadStatus(systemInfo.cpu_usage).level}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-gray-400">Temperature</span>
                <span className={`text-sm font-medium ${SystemUtilizationUtils.getTemperatureStatus(systemInfo.cpu_temperature).color}`}>
                  {SystemUtilizationUtils.getTemperatureStatus(systemInfo.cpu_temperature).level}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-gray-400">Memory</span>
                <span className={`text-sm font-medium ${SystemUtilizationUtils.getMemoryStatus(systemInfo.memory_used, systemInfo.memory_total).color}`}>
                  {SystemUtilizationUtils.getMemoryStatus(systemInfo.memory_used, systemInfo.memory_total).percentage}% Used
                </span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default SystemUtilization;
