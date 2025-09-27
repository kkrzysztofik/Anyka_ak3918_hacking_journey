/**
 * @file Dynamic import utilities
 * @description Utilities for dynamic imports and code splitting
 */

/**
 * Dynamically import ONVIF client with error handling
 */
export const loadONVIFClient = async () => {
  try {
    const module = await import('../services/onvifClient');
    return module;
  } catch (error) {
    console.error('Failed to load ONVIF client:', error);
    throw new Error('ONVIF client unavailable');
  }
};

/**
 * Dynamically import device management service
 */
export const loadDeviceManagementService = async () => {
  try {
    const module = await import('../services/deviceManagementService');
    return module;
  } catch (error) {
    console.error('Failed to load device management service:', error);
    throw new Error('Device management service unavailable');
  }
};

/**
 * Dynamically import system utilization service
 */
export const loadSystemUtilizationService = async () => {
  try {
    const module = await import('../services/systemUtilizationService');
    return module;
  } catch (error) {
    console.error('Failed to load system utilization service:', error);
    throw new Error('System utilization service unavailable');
  }
};

/**
 * Dynamically import XML utilities
 */
export const loadXMLUtils = async () => {
  try {
    const module = await import('./xmlParser');
    return module;
  } catch (error) {
    console.error('Failed to load XML utilities:', error);
    throw new Error('XML utilities unavailable');
  }
};

/**
 * Dynamically import validation utilities
 */
export const loadValidationUtils = async () => {
  try {
    const module = await import('./validation');
    return module;
  } catch (error) {
    console.error('Failed to load validation utilities:', error);
    throw new Error('Validation utilities unavailable');
  }
};

/**
 * Preload critical services for better performance
 */
export const preloadCriticalServices = async () => {
  const preloadPromises = [
    loadONVIFClient(),
    loadDeviceManagementService(),
  ];

  try {
    await Promise.allSettled(preloadPromises);
    console.log('Critical services preloaded');
  } catch (error) {
    console.warn('Some services failed to preload:', error);
  }
};

/**
 * Load service on demand with caching
 */
const serviceCache = new Map<string, Promise<any>>();

export const loadService = async <T>(serviceName: string, loader: () => Promise<T>): Promise<T> => {
  if (serviceCache.has(serviceName)) {
    return serviceCache.get(serviceName)!;
  }

  const promise = loader();
  serviceCache.set(serviceName, promise);
  
  try {
    const result = await promise;
    return result;
  } catch (error) {
    serviceCache.delete(serviceName);
    throw error;
  }
};
