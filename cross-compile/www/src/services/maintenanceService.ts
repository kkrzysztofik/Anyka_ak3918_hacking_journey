/**
 * Maintenance Service
 *
 * SOAP operations for device maintenance (reboot, factory reset).
 */

import { apiClient, ENDPOINTS } from '@/services/api'
import { createSOAPEnvelope, parseSOAPResponse } from '@/services/soap/client'

export type FactoryDefaultType = 'Soft' | 'Hard'

/**
 * Reboot the device
 */
export async function systemReboot(): Promise<void> {
  const envelope = createSOAPEnvelope('<tds:SystemReboot />')

  const response = await apiClient.post(ENDPOINTS.device, envelope)
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data)

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to reboot system')
  }
}

/**
 * Reset device to factory defaults
 */
export async function setSystemFactoryDefault(type: FactoryDefaultType): Promise<void> {
  const body = `<tds:SetSystemFactoryDefault>
    <tds:FactoryDefault>${type}</tds:FactoryDefault>
  </tds:SetSystemFactoryDefault>`

  const envelope = createSOAPEnvelope(body)

  const response = await apiClient.post(ENDPOINTS.device, envelope)
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data)

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to reset to factory defaults')
  }
}
