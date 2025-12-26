/**
 * User Service
 *
 * SOAP operations for user management.
 */
import { ENDPOINTS } from '@/services/api';
import { soapRequest } from '@/services/soap/client';
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
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.device,
    '<tds:GetUsers />',
    'GetUsersResponse',
  );

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

  await soapRequest(ENDPOINTS.device, body);
}

/**
 * Delete a user
 */
export async function deleteUser(username: string): Promise<void> {
  const body = `<tds:DeleteUsers>
    <tds:Username>${username}</tds:Username>
  </tds:DeleteUsers>`;

  await soapRequest(ENDPOINTS.device, body);
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

  await soapRequest(ENDPOINTS.device, body);
}
