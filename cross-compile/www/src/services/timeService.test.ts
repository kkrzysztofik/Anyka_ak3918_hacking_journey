/**
 * Time Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import { getSystemDateAndTime, setSystemDateAndTime } from '@/services/timeService';
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
  });
});
