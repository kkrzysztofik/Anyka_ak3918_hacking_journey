# API Contract: User Management

**Service Endpoint**: `POST /onvif/device_service`

All operations use SOAP 1.2 with WS-UsernameToken authentication.
User management operations require Administrator role.

---

## GetUsers

Retrieves list of user accounts.

**ONVIF Action**: `http://www.onvif.org/ver10/device/wsdl/GetUsers`

### Request

```xml
<tds:GetUsers />
```

### Response

```xml
<tds:GetUsersResponse>
  <tds:User>
    <tt:Username>string</tt:Username>
    <tt:UserLevel>Administrator|Operator|User</tt:UserLevel>
  </tds:User>
  <!-- Multiple User elements -->
</tds:GetUsersResponse>
```

### Frontend Mapping

```typescript
interface User {
  username: string;
  role: 'Administrator' | 'Operator' | 'User';
}
```

---

## CreateUsers

Creates new user accounts.

**ONVIF Action**: `http://www.onvif.org/ver10/device/wsdl/CreateUsers`

### Request

```xml
<tds:CreateUsers>
  <tds:User>
    <tt:Username>string</tt:Username>
    <tt:Password>string</tt:Password>
    <tt:UserLevel>Administrator|Operator|User</tt:UserLevel>
  </tds:User>
</tds:CreateUsers>
```

### Response

```xml
<tds:CreateUsersResponse />
```

### Validation

- Username: 1-32 characters, alphanumeric + underscore
- Password: Minimum 6 characters
- UserLevel: Must be one of Administrator, Operator, User

### Error Codes

| Fault | Meaning |
|-------|---------|
| `ter:UsernameClash` | Username already exists |
| `ter:TooManyUsers` | Maximum user limit reached |
| `ter:InvalidArgs` | Invalid username/password format |

---

## SetUser

Updates existing user (password or role).

**ONVIF Action**: `http://www.onvif.org/ver10/device/wsdl/SetUser`

### Request

```xml
<tds:SetUser>
  <tds:User>
    <tt:Username>string</tt:Username>
    <tt:Password>string</tt:Password><!-- optional -->
    <tt:UserLevel>Administrator|Operator|User</tt:UserLevel><!-- optional -->
  </tds:User>
</tds:SetUser>
```

### Response

```xml
<tds:SetUserResponse />
```

### Notes

- Only include Password if changing password
- Only include UserLevel if changing role
- Cannot demote last Administrator

---

## DeleteUsers

Deletes user accounts.

**ONVIF Action**: `http://www.onvif.org/ver10/device/wsdl/DeleteUsers`

### Request

```xml
<tds:DeleteUsers>
  <tds:Username>string</tds:Username>
  <!-- Multiple Username elements allowed -->
</tds:DeleteUsers>
```

### Response

```xml
<tds:DeleteUsersResponse />
```

### Constraints

- Cannot delete last Administrator account
- Cannot delete currently authenticated user

### Error Codes

| Fault | Meaning |
|-------|---------|
| `ter:NoUser` | User does not exist |
| `ter:FixedUser` | Cannot delete protected user |

---

## Frontend Service Implementation

```typescript
// services/userService.ts

export const userService = {
  async getUsers(): Promise<User[]> {
    const response = await onvifClient.sendSOAPRequest(
      'device_service',
      'GetUsers',
      { 'tds:GetUsers': {} }
    );
    return parseUsers(response);
  },

  async createUser(user: CreateUserRequest): Promise<void> {
    await onvifClient.sendSOAPRequest(
      'device_service',
      'CreateUsers',
      {
        'tds:CreateUsers': {
          'tds:User': {
            'tt:Username': user.username,
            'tt:Password': user.password,
            'tt:UserLevel': user.role
          }
        }
      }
    );
  },

  async updateUser(user: UpdateUserRequest): Promise<void> {
    const userElement: Record<string, string> = {
      'tt:Username': user.username
    };
    if (user.password) userElement['tt:Password'] = user.password;
    if (user.role) userElement['tt:UserLevel'] = user.role;

    await onvifClient.sendSOAPRequest(
      'device_service',
      'SetUser',
      { 'tds:SetUser': { 'tds:User': userElement } }
    );
  },

  async deleteUser(username: string): Promise<void> {
    await onvifClient.sendSOAPRequest(
      'device_service',
      'DeleteUsers',
      { 'tds:DeleteUsers': { 'tds:Username': username } }
    );
  }
};
```

---

## Role Permissions

| Action | Administrator | Operator | User |
|--------|---------------|----------|------|
| View users | ✅ | ❌ | ❌ |
| Create user | ✅ | ❌ | ❌ |
| Update user | ✅ | ❌ | ❌ |
| Delete user | ✅ | ❌ | ❌ |
| Change own password | ✅ | ✅ | ✅ |
