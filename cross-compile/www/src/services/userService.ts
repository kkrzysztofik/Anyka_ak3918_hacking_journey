/**
 * User Service
 *
 * SOAP operations for user management.
 */
import { ENDPOINTS, apiClient } from '@/services/api';
import { createSOAPEnvelope, parseSOAPResponse } from '@/services/soap/client';
import { safeString } from '@/utils/safeString';

export type UserLevel = 'Administrator' | 'Operator' | 'User' | 'Anonymous';

export interface OnvifUser {
  username: string;
  userLevel: UserLevel;
}

/**
 * Get list of users
 */
export async function getUsers(): Promise<OnvifUser[]> {
  const envelope = createSOAPEnvelope('<tds:GetUsers />');

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to get users');
  }

  const data = parsed.data?.GetUsersResponse as Record<string, unknown> | undefined;
  const users = data?.User;

  if (!users) {
    return [];
  }

  const usersList = Array.isArray(users) ? users : [users];

  return usersList.map((user: Record<string, unknown>) => ({
    username: safeString(user.Username, ''),
    userLevel: safeString(user.UserLevel, 'User') as UserLevel,
  }));
}

/**
 * Create a new user
 */
export async function createUser(
  username: string,
  password: string,
  userLevel: UserLevel,
): Promise<void> {
  const body = `<tds:CreateUsers>
    <tds:User>
      <tt:Username>${username}</tt:Username>
      <tt:Password>${password}</tt:Password>
      <tt:UserLevel>${userLevel}</tt:UserLevel>
    </tds:User>
  </tds:CreateUsers>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to create user');
  }
}

/**
 * Delete a user
 */
export async function deleteUser(username: string): Promise<void> {
  const body = `<tds:DeleteUsers>
    <tds:Username>${username}</tds:Username>
  </tds:DeleteUsers>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to delete user');
  }
}

/**
 * Update user (change password or level)
 */
export async function setUser(
  username: string,
  password: string,
  userLevel: UserLevel,
): Promise<void> {
  const body = `<tds:SetUser>
    <tds:User>
      <tt:Username>${username}</tt:Username>
      <tt:Password>${password}</tt:Password>
      <tt:UserLevel>${userLevel}</tt:UserLevel>
    </tds:User>
  </tds:SetUser>`;

  const envelope = createSOAPEnvelope(body);

  const response = await apiClient.post(ENDPOINTS.device, envelope);
  const parsed = parseSOAPResponse<Record<string, unknown>>(response.data);

  if (!parsed.success) {
    throw new Error(parsed.fault?.reason || 'Failed to update user');
  }
}
