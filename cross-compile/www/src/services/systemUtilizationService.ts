import type { SystemInfo, SystemUtilizationResponse } from '../types';

/**
 * Service for fetching system utilization data from the camera
 */
export class SystemUtilizationService {
  private baseUrl: string;
  private timeout: number;

  constructor(cameraIP: string, port: number = 8081, timeout: number = 10000) {
    this.baseUrl = `http://${cameraIP}:${port}`;
    this.timeout = timeout;
  }

  /**
   * Fetch current system utilization data
   * @returns Promise<SystemUtilizationResponse>
   */
  async getSystemInfo(): Promise<SystemUtilizationResponse> {
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), this.timeout);

      const response = await fetch(`${this.baseUrl}/utilization`, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        return {
          success: false,
          error: `HTTP error! status: ${response.status}`,
        };
      }

      const data: SystemInfo = await response.json();
      
      return {
        success: true,
        data,
      };
    } catch (error) {
      if (error instanceof Error) {
        if (error.name === 'AbortError') {
          return {
            success: false,
            error: 'Request timeout - camera may be unreachable',
          };
        }
        return {
          success: false,
          error: error.message,
        };
      }
      return {
        success: false,
        error: 'Failed to fetch system information',
      };
    }
  }

  /**
   * Test connection to the system utilization endpoint
   * @returns Promise<SystemUtilizationResponse>
   */
  async testConnection(): Promise<SystemUtilizationResponse> {
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000); // Shorter timeout for connection test

      const response = await fetch(`${this.baseUrl}/utilization`, {
        method: 'HEAD', // Use HEAD request for connection test
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      return {
        success: response.ok,
        error: response.ok ? undefined : `Connection failed: ${response.status}`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Connection test failed',
      };
    }
  }

  /**
   * Update the camera IP and port
   * @param cameraIP New camera IP address
   * @param port New port number (default: 8081)
   */
  updateConfig(cameraIP: string, port: number = 8081): void {
    this.baseUrl = `http://${cameraIP}:${port}`;
  }

  /**
   * Get the current base URL
   * @returns string
   */
  getBaseUrl(): string {
    return this.baseUrl;
  }

  /**
   * Set request timeout
   * @param timeout Timeout in milliseconds
   */
  setTimeout(timeout: number): void {
    this.timeout = timeout;
  }

  /**
   * Get current timeout setting
   * @returns number
   */
  getTimeout(): number {
    return this.timeout;
  }
}

/**
 * Factory function to create a SystemUtilizationService instance
 * @param cameraIP Camera IP address
 * @param port Port number (default: 8081)
 * @param timeout Request timeout in milliseconds (default: 10000)
 * @returns SystemUtilizationService
 */
export const createSystemUtilizationService = (
  cameraIP: string, 
  port: number = 8081, 
  timeout: number = 10000
): SystemUtilizationService => {
  return new SystemUtilizationService(cameraIP, port, timeout);
};

/**
 * Utility functions for system data formatting
 */
export const SystemUtilizationUtils = {
  /**
   * Format bytes to human readable string
   * @param bytes Number of bytes
   * @returns Formatted string
   */
  formatBytes: (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  },

  /**
   * Format uptime in milliseconds to human readable string
   * @param ms Uptime in milliseconds
   * @returns Formatted string
   */
  formatUptime: (ms: number): string => {
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);
    
    if (days > 0) return `${days}d ${hours % 24}h ${minutes % 60}m`;
    if (hours > 0) return `${hours}h ${minutes % 60}m`;
    if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
    return `${seconds}s`;
  },

  /**
   * Format timestamp to time string
   * @param timestamp Timestamp in milliseconds
   * @returns Formatted time string
   */
  formatTime: (timestamp: number): string => {
    return new Date(timestamp).toLocaleTimeString();
  },

  /**
   * Generate chart data from history points
   * @param history Array of data points
   * @param maxPoints Maximum number of points to include
   * @returns Chart data array
   */
  generateChartData: (history: Array<{ timestamp: number; value: number }>, maxPoints: number = 20) => {
    const data = history.slice(-maxPoints);
    const minValue = Math.min(...data.map(d => d.value));
    const maxValue = Math.max(...data.map(d => d.value));
    const range = maxValue - minValue || 1;
    
    return data.map((point, index) => ({
      x: (index / (data.length - 1)) * 100,
      y: ((point.value - minValue) / range) * 100,
      value: point.value,
      time: SystemUtilizationUtils.formatTime(point.timestamp)
    }));
  },

  /**
   * Get CPU load status based on usage percentage
   * @param usage CPU usage percentage
   * @returns Status object with level and color
   */
  getCpuLoadStatus: (usage: number) => {
    if (usage > 80) return { level: 'High', color: 'text-red-400' };
    if (usage > 60) return { level: 'Medium', color: 'text-yellow-400' };
    return { level: 'Low', color: 'text-green-400' };
  },

  /**
   * Get temperature status based on temperature value
   * @param temperature Temperature in Celsius
   * @returns Status object with level and color
   */
  getTemperatureStatus: (temperature: number) => {
    if (temperature > 70) return { level: 'Hot', color: 'text-red-400' };
    if (temperature > 50) return { level: 'Warm', color: 'text-yellow-400' };
    return { level: 'Cool', color: 'text-green-400' };
  },

  /**
   * Get memory usage status based on usage percentage
   * @param used Used memory in bytes
   * @param total Total memory in bytes
   * @returns Status object with percentage and color
   */
  getMemoryStatus: (used: number, total: number) => {
    const percentage = (used / total) * 100;
    if (percentage > 80) return { percentage: Math.round(percentage), color: 'text-red-400' };
    if (percentage > 60) return { percentage: Math.round(percentage), color: 'text-yellow-400' };
    return { percentage: Math.round(percentage), color: 'text-green-400' };
  }
};
