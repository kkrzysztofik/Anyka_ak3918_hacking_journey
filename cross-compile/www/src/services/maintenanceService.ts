/**
 * Maintenance Service
 *
 * SOAP operations for device maintenance (reboot, factory reset, backup, restore).
 */
import { ENDPOINTS, apiClient } from '@/services/api';
import { createSOAPEnvelope, parseSOAPResponse } from '@/services/soap/client';

export type FactoryDefaultType = 'Soft' | 'Hard';

/**
 * Backup file structure
 */
export interface BackupFile {
  Name: string;
  Data: string; // base64-encoded
}

/**
 * GetSystemBackup response structure
 */
export interface GetSystemBackupResponse {
  GetSystemBackupResponse?: {
    BackupFiles?: {
      BackupFile?: BackupFile | BackupFile[];
    };
  };
}

/**
 * Reboot the device
 */
export async function systemReboot(): Promise<void> {
  const envelope = createSOAPEnvelope('<tds:SystemReboot />');

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to reboot system');
  }
}

/**
 * Reset device to factory defaults
 */
export async function setSystemFactoryDefault(type: FactoryDefaultType): Promise<void> {
  const body = `<tds:SetSystemFactoryDefault>
    <tds:FactoryDefault>${type}</tds:FactoryDefault>
  </tds:SetSystemFactoryDefault>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to reset to factory defaults');
  }
}

/**
 * Get system backup
 *
 * Returns backup data containing base64-encoded TOML configuration.
 */
export async function getSystemBackup(): Promise<BackupFile[]> {
  const envelope = createSOAPEnvelope('<tds:GetSystemBackup />');

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<GetSystemBackupResponse>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to get system backup');
  }

  // Extract backup files from response
  const backupResponse = parsed.data?.GetSystemBackupResponse;
  if (!backupResponse) {
    throw new Error('Invalid backup response: missing GetSystemBackupResponse');
  }

  // BackupFiles might be missing, empty object, or contain BackupFile
  const backupFiles = backupResponse.BackupFiles;
  if (!backupFiles) {
    return [];
  }

  const backupFile = backupFiles.BackupFile;
  if (!backupFile) {
    return [];
  }

  // Handle both single file and array of files
  if (Array.isArray(backupFile)) {
    return backupFile;
  }
  return [backupFile];
}

/**
 * Restore system from backup file
 *
 * Reads a TOML backup file, encodes it as base64, and sends it to the device.
 */
export async function restoreSystem(backupFile: File): Promise<void> {
  // Read file as text using modern Blob API
  const fileContent = await backupFile.text();

  // Encode content as base64
  const base64Data = btoa(fileContent);

  // Build SOAP request with BackupFiles
  const body = `<tds:RestoreSystem>
    <tds:BackupFiles>
      <tds:BackupFile>
        <tds:Name>config.toml</tds:Name>
        <tds:Data>${base64Data}</tds:Data>
      </tds:BackupFile>
    </tds:BackupFiles>
  </tds:RestoreSystem>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to restore system');
  }
}
