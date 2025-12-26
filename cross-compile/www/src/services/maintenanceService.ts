/**
 * Maintenance Service
 *
 * SOAP operations for device maintenance (reboot, factory reset, backup, restore).
 */
import { ENDPOINTS } from '@/services/api';
import { soapRequest } from '@/services/soap/client';

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
  BackupFiles?: {
    BackupFile?: BackupFile | BackupFile[];
  };
}

/**
 * Reboot the device
 */
export async function systemReboot(): Promise<void> {
  await soapRequest(ENDPOINTS.device, '<tds:SystemReboot />');
}

/**
 * Reset device to factory defaults
 */
export async function setSystemFactoryDefault(type: FactoryDefaultType): Promise<void> {
  const body = `<tds:SetSystemFactoryDefault>
    <tds:FactoryDefault>${type}</tds:FactoryDefault>
  </tds:SetSystemFactoryDefault>`;

  await soapRequest(ENDPOINTS.device, body);
}

/**
 * Get system backup
 *
 * Returns backup data containing base64-encoded TOML configuration.
 */
export async function getSystemBackup(): Promise<BackupFile[]> {
  const data = await soapRequest<GetSystemBackupResponse>(
    ENDPOINTS.device,
    '<tds:GetSystemBackup />',
    'GetSystemBackupResponse',
  );

  if (!data) {
    throw new Error('Invalid backup response: missing GetSystemBackupResponse');
  }

  // BackupFiles might be missing, empty object, or contain BackupFile
  const backupFiles = data?.BackupFiles;
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

  await soapRequest(ENDPOINTS.device, body);
}
