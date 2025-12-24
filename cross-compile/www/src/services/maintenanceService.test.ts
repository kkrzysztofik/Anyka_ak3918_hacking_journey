/**
 * Maintenance Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import {
  getSystemBackup,
  restoreSystem,
  setSystemFactoryDefault,
  systemReboot,
} from '@/services/maintenanceService';
import { createMockSOAPFaultResponse, createMockSOAPResponse } from '@/test/utils';

// Mock the api module
vi.mock('@/services/api', () => ({
  apiClient: {
    post: vi.fn(),
  },
  ENDPOINTS: {
    device: '/onvif/device_service',
  },
}));

describe('maintenanceService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('systemReboot', () => {
    it('should send reboot request', async () => {
      const mockResponse = createMockSOAPResponse(`
        <SystemRebootResponse>
          <Message>Rebooting...</Message>
        </SystemRebootResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await systemReboot();

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('tds:SystemReboot'),
      );
    });

    it('should throw on failure', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Reboot failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(systemReboot()).rejects.toThrow();
    });
  });

  describe('setSystemFactoryDefault', () => {
    it('should send soft reset request', async () => {
      const mockResponse = createMockSOAPResponse('<SetSystemFactoryDefaultResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setSystemFactoryDefault('Soft');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tds:FactoryDefault>Soft</tds:FactoryDefault>'),
      );
    });

    it('should send hard reset request', async () => {
      const mockResponse = createMockSOAPResponse('<SetSystemFactoryDefaultResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setSystemFactoryDefault('Hard');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tds:FactoryDefault>Hard</tds:FactoryDefault>'),
      );
    });
  });

  describe('getSystemBackup', () => {
    it('should retrieve backup with single file', async () => {
      const mockBackupData = btoa('[device]\nmanufacturer = "Test"');
      const mockResponse = createMockSOAPResponse(`
        <tds:GetSystemBackupResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
          <tds:BackupFiles>
            <tds:BackupFile>
              <tds:Name>config.toml</tds:Name>
              <tds:Data>${mockBackupData}</tds:Data>
            </tds:BackupFile>
          </tds:BackupFiles>
        </tds:GetSystemBackupResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getSystemBackup();

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('tds:GetSystemBackup'),
      );
      expect(result).toHaveLength(1);
      expect(result[0].Name).toBe('config.toml');
      expect(result[0].Data).toBe(mockBackupData);
    });

    it('should retrieve backup with multiple files', async () => {
      const mockBackupData1 = btoa('[device]\nmanufacturer = "Test"');
      const mockBackupData2 = btoa('[network]\nip = "192.168.1.1"');
      const mockResponse = createMockSOAPResponse(`
        <tds:GetSystemBackupResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
          <tds:BackupFiles>
            <tds:BackupFile>
              <tds:Name>config.toml</tds:Name>
              <tds:Data>${mockBackupData1}</tds:Data>
            </tds:BackupFile>
            <tds:BackupFile>
              <tds:Name>network.toml</tds:Name>
              <tds:Data>${mockBackupData2}</tds:Data>
            </tds:BackupFile>
          </tds:BackupFiles>
        </tds:GetSystemBackupResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getSystemBackup();

      expect(result).toHaveLength(2);
      expect(result[0].Name).toBe('config.toml');
      expect(result[1].Name).toBe('network.toml');
    });

    it('should return empty array when BackupFile is missing', async () => {
      // Test case where BackupFiles exists but BackupFile is missing/null
      // This simulates the parser returning BackupFiles but no BackupFile property
      const mockResponse = createMockSOAPResponse(`
        <tds:GetSystemBackupResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
          <tds:BackupFiles />
        </tds:GetSystemBackupResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      // When BackupFile is null/undefined, it should return empty array
      const result = await getSystemBackup();

      expect(result).toHaveLength(0);
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Backup failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getSystemBackup()).rejects.toThrow('Backup failed');
    });

    it('should throw on network error', async () => {
      vi.mocked(apiClient.post).mockRejectedValueOnce(new Error('Network error'));

      await expect(getSystemBackup()).rejects.toThrow('Network error');
    });

    it('should throw on invalid response structure', async () => {
      const mockResponse = createMockSOAPResponse(`
        <tds:GetSystemBackupResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl" />
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getSystemBackup()).rejects.toThrow('Invalid backup response');
    });
  });

  describe('restoreSystem', () => {
    // Mock File with text() method
    const createMockFile = (content: string, name = 'config.toml'): File => {
      const blob = new Blob([content], { type: 'text/plain' });
      const file = Object.assign(blob, { name }) as File;
      // Add text() method for modern Blob API
      file.text = async () => content;
      return file;
    };

    it('should restore system with valid TOML file', async () => {
      const tomlContent = '[device]\nmanufacturer = "Test"';
      const mockFile = createMockFile(tomlContent);
      const base64Data = btoa(tomlContent);

      const mockResponse = createMockSOAPResponse(`
        <tds:RestoreSystemResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl" />
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await restoreSystem(mockFile);

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('tds:RestoreSystem'),
      );
      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tds:Name>config.toml</tds:Name>'),
      );
      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining(`<tds:Data>${base64Data}</tds:Data>`),
      );
    });

    it('should encode file content as base64', async () => {
      const tomlContent = '[device]\nmanufacturer = "Anyka"\nmodel = "AK3918"';
      const mockFile = createMockFile(tomlContent);
      const expectedBase64 = btoa(tomlContent);

      const mockResponse = createMockSOAPResponse(`
        <tds:RestoreSystemResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl" />
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await restoreSystem(mockFile);

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      const envelope = callArgs[1] as string;
      expect(envelope).toContain(`<tds:Data>${expectedBase64}</tds:Data>`);
    });

    it('should set Name to config.toml in request', async () => {
      const tomlContent = '[device]\nmanufacturer = "Test"';
      const mockFile = createMockFile(tomlContent, 'backup.toml');

      const mockResponse = createMockSOAPResponse(`
        <tds:RestoreSystemResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl" />
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await restoreSystem(mockFile);

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      const envelope = callArgs[1] as string;
      expect(envelope).toContain('<tds:Name>config.toml</tds:Name>');
    });

    it('should throw on SOAP fault', async () => {
      const tomlContent = '[device]\nmanufacturer = "Test"';
      const mockFile = createMockFile(tomlContent);

      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Restore failed');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(restoreSystem(mockFile)).rejects.toThrow('Restore failed');
    });

    it('should throw on network error', async () => {
      const tomlContent = '[device]\nmanufacturer = "Test"';
      const mockFile = createMockFile(tomlContent);

      vi.mocked(apiClient.post).mockRejectedValueOnce(new Error('Network error'));

      await expect(restoreSystem(mockFile)).rejects.toThrow('Network error');
    });

    it('should throw on file read error', async () => {
      // Create a mock file that will fail to read using Blob.text()
      const mockFile = {
        name: 'config.toml',
        type: 'text/plain',
        size: 100,
        text: vi.fn().mockRejectedValue(new Error('File read error')),
      } as unknown as File;

      // Wait for the async error
      await expect(restoreSystem(mockFile)).rejects.toThrow('File read error');
    });
  });
});
