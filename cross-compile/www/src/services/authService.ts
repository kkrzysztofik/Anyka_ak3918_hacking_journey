/**
 * Authentication Service
 *
 * Handles login verification by calling GetDeviceInformation SOAP operation.
 * If the call succeeds with provided credentials, user is authenticated.
 */

import { apiClient, ENDPOINTS } from '@/services/api'
import { createSOAPEnvelope, parseSOAPResponse, soapBodies, type SOAPFault } from '@/services/soap/client'

export interface DeviceInfo {
  manufacturer: string
  model: string
  firmwareVersion: string
  serialNumber: string
  hardwareId: string
}

export interface LoginResult {
  success: boolean
  deviceInfo?: DeviceInfo
  error?: string
}

/**
 * Verify credentials by attempting GetDeviceInformation call
 */
export async function verifyCredentials(username: string, password: string): Promise<LoginResult> {
  try {
    // Create Basic Auth header for this request
    const authHeader = `Basic ${btoa(`${username}:${password}`)}`

    const envelope = createSOAPEnvelope(soapBodies.getDeviceInformation())

    const response = await apiClient.post(ENDPOINTS.device, envelope, {
      headers: {
        'Authorization': authHeader,
      },
    })

    const parsed = parseSOAPResponse<Record<string, unknown>>(response.data)

    if (!parsed.success) {
      return {
        success: false,
        error: parsed.fault?.reason || 'Authentication failed',
      }
    }

    // Extract device info from response
    const deviceInfo = extractDeviceInfo(parsed.data)

    return {
      success: true,
      deviceInfo,
    }
  } catch (error) {
    // Handle network or other errors
    if (error instanceof Error) {
      if (error.message.includes('401') || error.message.includes('Unauthorized')) {
        return { success: false, error: 'Invalid username or password' }
      }
      if (error.message.includes('Network') || error.message.includes('ECONNREFUSED')) {
        return { success: false, error: 'Unable to connect to camera' }
      }
      return { success: false, error: error.message }
    }
    return { success: false, error: 'An unexpected error occurred' }
  }
}

/**
 * Extract DeviceInfo from parsed SOAP response
 */
function extractDeviceInfo(data: Record<string, unknown> | undefined): DeviceInfo | undefined {
  if (!data) return undefined

  // Navigate to GetDeviceInformationResponse
  const response = data.GetDeviceInformationResponse as Record<string, unknown> | undefined
  if (!response) return undefined

  return {
    manufacturer: String(response.Manufacturer || 'Unknown'),
    model: String(response.Model || 'Unknown'),
    firmwareVersion: String(response.FirmwareVersion || 'Unknown'),
    serialNumber: String(response.SerialNumber || 'Unknown'),
    hardwareId: String(response.HardwareId || 'Unknown'),
  }
}

/**
 * Check if device is reachable (without authentication)
 */
export async function checkDeviceReachable(): Promise<boolean> {
  try {
    // GetSystemDateAndTime doesn't require authentication on most ONVIF devices
    const envelope = createSOAPEnvelope(soapBodies.getSystemDateAndTime())

    const response = await apiClient.post(ENDPOINTS.device, envelope, {
      timeout: 5000, // Short timeout for connectivity check
    })

    return response.status === 200
  } catch {
    return false
  }
}
