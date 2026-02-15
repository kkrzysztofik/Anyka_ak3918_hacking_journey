/**
 * Time Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import {
  getDateTime,
  getSystemDateAndTime,
  setDateTime,
  setNTP,
  setSystemDateAndTime,
} from '@/services/timeService';
import { createMockSOAPResponse } from '@/test/utils';

// Mock the api module
vi.mock('@/services/api', () => ({
  apiClient: {
    post: vi.fn(),
  },
  ENDPOINTS: {
    device: '/onvif/device_service',
  },
}));

describe('timeService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getSystemDateAndTime', () => {
    it('should parse NTP configuration', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetSystemDateAndTimeResponse>
          <SystemDateAndTime>
            <DateTimeType>NTP</DateTimeType>
            <DaylightSavings>false</DaylightSavings>
            <TimeZone>
              <TZ>UTC+0</TZ>
            </TimeZone>
            <UTCDateTime>
              <Time>
                <Hour>12</Hour>
                <Minute>30</Minute>
                <Second>45</Second>
              </Time>
              <Date>
                <Year>2024</Year>
                <Month>6</Month>
                <Day>15</Day>
              </Date>
            </UTCDateTime>
          </SystemDateAndTime>
        </GetSystemDateAndTimeResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getSystemDateAndTime();

      expect(result.dateTimeType).toBe('NTP');
      expect(result.daylightSavings).toBe(false);
      expect(result.timezone).toBe('UTC+0');
      expect(result.utcDateTime).toBeInstanceOf(Date);
    });

    it('should parse Manual configuration', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetSystemDateAndTimeResponse>
          <SystemDateAndTime>
            <DateTimeType>Manual</DateTimeType>
            <DaylightSavings>true</DaylightSavings>
            <TimeZone><TZ>PST+8</TZ></TimeZone>
            <UTCDateTime>
              <Time><Hour>8</Hour><Minute>0</Minute><Second>0</Second></Time>
              <Date><Year>2024</Year><Month>1</Month><Day>1</Day></Date>
            </UTCDateTime>
          </SystemDateAndTime>
        </GetSystemDateAndTimeResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getSystemDateAndTime();

      expect(result.dateTimeType).toBe('Manual');
      expect(result.daylightSavings).toBe(true);
    });
  });

  describe('setSystemDateAndTime', () => {
    it('should send NTP configuration', async () => {
      const mockResponse = createMockSOAPResponse('<SetSystemDateAndTimeResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setSystemDateAndTime('NTP', false, 'UTC+0');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tds:DateTimeType>NTP</tds:DateTimeType>'),
      );
    });

    it('should escape XML special characters in timezone payload', async () => {
      const mockResponse = createMockSOAPResponse('<SetSystemDateAndTimeResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setSystemDateAndTime('NTP', false, 'UTC+0<bad>&"\'"');

      const payload = vi.mocked(apiClient.post).mock.calls[0][1] as string;
      expect(payload).toContain('<tt:TZ>UTC+0&lt;bad&gt;&amp;&quot;&apos;&quot;</tt:TZ>');
    });

    it('should send Manual configuration with date', async () => {
      const mockResponse = createMockSOAPResponse('<SetSystemDateAndTimeResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const testDate = new Date('2024-06-15T12:00:00Z');
      await setSystemDateAndTime('Manual', false, 'UTC+0', testDate);

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tds:DateTimeType>Manual</tds:DateTimeType>'),
      );
      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tt:Year>2024</tt:Year>'),
      );
    });

    it('should handle missing date fields with defaults', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetSystemDateAndTimeResponse>
          <SystemDateAndTime>
            <DateTimeType>NTP</DateTimeType>
            <DaylightSavings>false</DaylightSavings>
            <TimeZone><TZ>UTC+0</TZ></TimeZone>
            <UTCDateTime>
              <Time>
                <Hour>12</Hour>
                <Minute>30</Minute>
              </Time>
              <Date>
                <Year>2024</Year>
              </Date>
            </UTCDateTime>
          </SystemDateAndTime>
        </GetSystemDateAndTimeResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getSystemDateAndTime();

      expect(result.utcDateTime).toBeInstanceOf(Date);
      // Should use defaults for missing fields (Second=0, Month=1, Day=1)
    });

    it('should handle missing timezone with default', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetSystemDateAndTimeResponse>
          <SystemDateAndTime>
            <DateTimeType>NTP</DateTimeType>
            <DaylightSavings>false</DaylightSavings>
            <UTCDateTime>
              <Time><Hour>12</Hour><Minute>0</Minute><Second>0</Second></Time>
              <Date><Year>2024</Year><Month>6</Month><Day>15</Day></Date>
            </UTCDateTime>
          </SystemDateAndTime>
        </GetSystemDateAndTimeResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getSystemDateAndTime();

      expect(result.timezone).toBe('UTC'); // Default when TZ is missing
    });

    it('should throw error when SystemDateAndTime is missing', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetSystemDateAndTimeResponse>
        </GetSystemDateAndTimeResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getSystemDateAndTime()).rejects.toThrow(
        'Invalid response: missing SystemDateAndTime',
      );
    });

    it('should handle daylightSavings as string "true"', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetSystemDateAndTimeResponse>
          <SystemDateAndTime>
            <DateTimeType>NTP</DateTimeType>
            <DaylightSavings>true</DaylightSavings>
            <TimeZone><TZ>UTC+0</TZ></TimeZone>
            <UTCDateTime>
              <Time><Hour>12</Hour><Minute>0</Minute><Second>0</Second></Time>
              <Date><Year>2024</Year><Month>6</Month><Day>15</Day></Date>
            </UTCDateTime>
          </SystemDateAndTime>
        </GetSystemDateAndTimeResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getSystemDateAndTime();

      expect(result.daylightSavings).toBe(true);
    });
  });

  describe('getDateTime', () => {
    it('should return DateTimeConfig with NTP enabled', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetSystemDateAndTimeResponse>
          <SystemDateAndTime>
            <DateTimeType>NTP</DateTimeType>
            <DaylightSavings>false</DaylightSavings>
            <TimeZone><TZ>UTC+0</TZ></TimeZone>
            <UTCDateTime>
              <Time><Hour>12</Hour><Minute>30</Minute><Second>45</Second></Time>
              <Date><Year>2024</Year><Month>6</Month><Day>15</Day></Date>
            </UTCDateTime>
          </SystemDateAndTime>
        </GetSystemDateAndTimeResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getDateTime();

      expect(result.ntp.enabled).toBe(true);
      expect(result.ntp.fromDHCP).toBe(true);
      expect(result.timezone).toBe('UTC+0');
      expect(result.datetime).toBeInstanceOf(Date);
    });

    it('should return DateTimeConfig with NTP disabled for Manual mode', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetSystemDateAndTimeResponse>
          <SystemDateAndTime>
            <DateTimeType>Manual</DateTimeType>
            <DaylightSavings>false</DaylightSavings>
            <TimeZone><TZ>PST+8</TZ></TimeZone>
            <UTCDateTime>
              <Time><Hour>8</Hour><Minute>0</Minute><Second>0</Second></Time>
              <Date><Year>2024</Year><Month>1</Month><Day>1</Day></Date>
            </UTCDateTime>
          </SystemDateAndTime>
        </GetSystemDateAndTimeResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getDateTime();

      expect(result.ntp.enabled).toBe(false);
      expect(result.timezone).toBe('PST+8');
    });
  });

  describe('setNTP', () => {
    it('should set NTP mode preserving current timezone', async () => {
      const getResponse = createMockSOAPResponse(`
        <GetSystemDateAndTimeResponse>
          <SystemDateAndTime>
            <DateTimeType>Manual</DateTimeType>
            <DaylightSavings>true</DaylightSavings>
            <TimeZone><TZ>PST+8</TZ></TimeZone>
            <UTCDateTime>
              <Time><Hour>8</Hour><Minute>0</Minute><Second>0</Second></Time>
              <Date><Year>2024</Year><Month>1</Month><Day>1</Day></Date>
            </UTCDateTime>
          </SystemDateAndTime>
        </GetSystemDateAndTimeResponse>
      `);

      const setResponse = createMockSOAPResponse('<SetSystemDateAndTimeResponse />');

      vi.mocked(apiClient.post)
        .mockResolvedValueOnce(getResponse)
        .mockResolvedValueOnce(setResponse);

      await setNTP(false);

      expect(apiClient.post).toHaveBeenCalledTimes(2);
      // Second call should set NTP with preserved timezone
      expect(apiClient.post).toHaveBeenNthCalledWith(
        2,
        '/onvif/device_service',
        expect.stringContaining('<tds:DateTimeType>NTP</tds:DateTimeType>'),
      );
      expect(apiClient.post).toHaveBeenNthCalledWith(
        2,
        '/onvif/device_service',
        expect.stringContaining('<tt:TZ>PST+8</tt:TZ>'),
      );
    });

    it('should preserve daylightSavings when setting NTP', async () => {
      const getResponse = createMockSOAPResponse(`
        <GetSystemDateAndTimeResponse>
          <SystemDateAndTime>
            <DateTimeType>Manual</DateTimeType>
            <DaylightSavings>true</DaylightSavings>
            <TimeZone><TZ>UTC+0</TZ></TimeZone>
            <UTCDateTime>
              <Time><Hour>12</Hour><Minute>0</Minute><Second>0</Second></Time>
              <Date><Year>2024</Year><Month>6</Month><Day>15</Day></Date>
            </UTCDateTime>
          </SystemDateAndTime>
        </GetSystemDateAndTimeResponse>
      `);

      const setResponse = createMockSOAPResponse('<SetSystemDateAndTimeResponse />');

      vi.mocked(apiClient.post)
        .mockResolvedValueOnce(getResponse)
        .mockResolvedValueOnce(setResponse);

      await setNTP(true);

      expect(apiClient.post).toHaveBeenNthCalledWith(
        2,
        '/onvif/device_service',
        expect.stringContaining('<tds:DaylightSavings>true</tds:DaylightSavings>'),
      );
    });
  });

  describe('setDateTime', () => {
    it('should set manual date time with ISO date string', async () => {
      const mockResponse = createMockSOAPResponse('<SetSystemDateAndTimeResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setDateTime('manual', '2024-06-15T12:30:45Z', 'UTC+0');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tds:DateTimeType>Manual</tds:DateTimeType>'),
      );
      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tt:Year>2024</tt:Year>'),
      );
      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tt:Month>6</tt:Month>'),
      );
      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tt:Day>15</tt:Day>'),
      );
      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tt:TZ>UTC+0</tt:TZ>'),
      );
    });

    it('should set daylightSavings to false for manual mode', async () => {
      const mockResponse = createMockSOAPResponse('<SetSystemDateAndTimeResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setDateTime('manual', '2024-06-15T12:00:00Z', 'PST+8');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tds:DaylightSavings>false</tds:DaylightSavings>'),
      );
    });

    it('should handle different timezones', async () => {
      const mockResponse = createMockSOAPResponse('<SetSystemDateAndTimeResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setDateTime('manual', '2024-06-15T12:00:00Z', 'EST-5');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tt:TZ>EST-5</tt:TZ>'),
      );
    });
  });
});
