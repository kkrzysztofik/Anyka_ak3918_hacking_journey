/**
 * @file cameraConfig.ts
 * @brief Centralized camera configuration management
 * @author ONVIF Camera Web Interface
 * @date 2024
 */
import type { CameraConfig } from '../types';

/**
 * @brief Default camera configuration values
 */
export const DEFAULT_CAMERA_CONFIG: CameraConfig = {
  ip: '192.168.1.100',
  onvifPort: 8080,
  snapshotPort: 3000,
  rtspPort: 554,
};

/**
 * @brief Create camera configuration with custom IP
 * @param ip Camera IP address
 * @returns Complete camera configuration object
 * @note Uses default ports for ONVIF, snapshot, and RTSP services
 */
export const createCameraConfig = (ip: string): CameraConfig => {
  if (!ip || typeof ip !== 'string') {
    throw new Error('IP address is required and must be a string');
  }

  // Basic IP validation
  const ipRegex =
    /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;
  if (!ipRegex.test(ip)) {
    throw new Error('Invalid IP address format');
  }

  return {
    ...DEFAULT_CAMERA_CONFIG,
    ip,
  };
};

/**
 * @brief Create camera configuration with custom values
 * @param config Partial camera configuration
 * @returns Complete camera configuration object
 * @note Merges provided values with defaults
 */
export const createCustomCameraConfig = (config: Partial<CameraConfig>): CameraConfig => {
  const validatedConfig = { ...DEFAULT_CAMERA_CONFIG };

  if (config.ip) {
    if (typeof config.ip !== 'string') {
      throw new Error('IP address must be a string');
    }
    const ipRegex =
      /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;
    if (!ipRegex.test(config.ip)) {
      throw new Error('Invalid IP address format');
    }
    validatedConfig.ip = config.ip;
  }

  if (config.onvifPort !== undefined) {
    if (typeof config.onvifPort !== 'number' || config.onvifPort < 1 || config.onvifPort > 65535) {
      throw new Error('ONVIF port must be a number between 1 and 65535');
    }
    validatedConfig.onvifPort = config.onvifPort;
  }

  if (config.snapshotPort !== undefined) {
    if (
      typeof config.snapshotPort !== 'number' ||
      config.snapshotPort < 1 ||
      config.snapshotPort > 65535
    ) {
      throw new Error('Snapshot port must be a number between 1 and 65535');
    }
    validatedConfig.snapshotPort = config.snapshotPort;
  }

  if (config.rtspPort !== undefined) {
    if (typeof config.rtspPort !== 'number' || config.rtspPort < 1 || config.rtspPort > 65535) {
      throw new Error('RTSP port must be a number between 1 and 65535');
    }
    validatedConfig.rtspPort = config.rtspPort;
  }

  return validatedConfig;
};

/**
 * @brief Validate camera configuration
 * @param config Camera configuration to validate
 * @returns True if valid, false otherwise
 */
export const validateCameraConfig = (config: CameraConfig): boolean => {
  try {
    createCustomCameraConfig(config);
    return true;
  } catch {
    return false;
  }
};

/**
 * @brief Get default configuration for development
 * @returns Default development configuration
 */
export const getDevelopmentConfig = (): CameraConfig => DEFAULT_CAMERA_CONFIG;

/**
 * @brief Get configuration for production
 * @param ip Production camera IP
 * @returns Production configuration
 */
export const getProductionConfig = (ip: string): CameraConfig => createCameraConfig(ip);
