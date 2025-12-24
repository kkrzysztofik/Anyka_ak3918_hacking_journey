/**
 * User Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import { createUser, deleteUser, getUsers, setUser } from '@/services/userService';
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

describe('userService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getUsers', () => {
    it('should parse user list', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetUsersResponse>
          <User>
            <Username>admin</Username>
            <UserLevel>Administrator</UserLevel>
          </User>
          <User>
            <Username>operator</Username>
            <UserLevel>Operator</UserLevel>
          </User>
        </GetUsersResponse>
      `);

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getUsers();

      expect(result).toHaveLength(2);
      expect(result[0].username).toBe('admin');
      expect(result[0].userLevel).toBe('Administrator');
      expect(result[1].username).toBe('operator');
      expect(result[1].userLevel).toBe('Operator');
    });

    it('should return empty array when no users', async () => {
      const mockResponse = createMockSOAPResponse('<GetUsersResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const result = await getUsers();

      expect(result).toEqual([]);
    });
  });

  describe('createUser', () => {
    it('should send create user request', async () => {
      const mockResponse = createMockSOAPResponse('<CreateUsersResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await createUser('newuser', 'password123', 'User');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tt:Username>newuser</tt:Username>'),
      );
      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tt:UserLevel>User</tt:UserLevel>'),
      );
    });
  });

  describe('deleteUser', () => {
    it('should send delete user request', async () => {
      const mockResponse = createMockSOAPResponse('<DeleteUsersResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await deleteUser('olduser');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tds:Username>olduser</tds:Username>'),
      );
    });
  });

  describe('setUser', () => {
    it('should send update user request', async () => {
      const mockResponse = createMockSOAPResponse('<SetUserResponse />');

      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setUser('admin', 'newpassword', 'Administrator');

      expect(apiClient.post).toHaveBeenCalledWith(
        '/onvif/device_service',
        expect.stringContaining('<tt:Username>admin</tt:Username>'),
      );
    });
  });
});
