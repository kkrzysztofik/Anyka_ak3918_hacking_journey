# Data Model: Frontend ↔ ONVIF Services

**Branch**: `003-frontend-onvif-spec` | **Date**: Saturday Dec 13, 2025

## Overview

This document defines the TypeScript types and data structures used in the frontend application for communicating with the onvif-rust backend.

---

## Core Entities

### Authentication

```typescript
/** User roles matching ONVIF UserLevel */
type UserRole = 'Administrator' | 'Operator' | 'User';

/** Authentication state stored in Redux */
interface AuthState {
  isAuthenticated: boolean;
  user: AuthUser | null;
  isLoading: boolean;
  error: string | null;
}

/** Authenticated user information */
interface AuthUser {
  username: string;
  role: UserRole;
}

/** Login credentials */
interface LoginCredentials {
  username: string;
  password: string;
}

/** Session info from backend */
interface SessionInfo {
  username: string;
  role: UserRole;
  expiresAt?: Date;
}
```

### Device Information

```typescript
/** Device identity from GetDeviceInformation */
interface DeviceInfo {
  manufacturer: string;
  model: string;
  firmwareVersion: string;
  serialNumber: string;
  hardwareId: string;
}

/** Device connection status */
type DeviceStatus = 'online' | 'offline' | 'checking';

/** Combined device state */
interface DeviceState {
  info: DeviceInfo | null;
  status: DeviceStatus;
  lastChecked: Date | null;
  error: string | null;
}
```

### Identification Settings

```typescript
/** Editable identification fields */
interface IdentificationSettings {
  /** Editable: Device display name */
  name: string;
  /** Editable: Physical location description */
  location: string;
}

/** Full identification view (includes read-only) */
interface IdentificationView extends IdentificationSettings {
  /** Read-only: From DeviceInfo */
  manufacturer: string;
  model: string;
  firmwareVersion: string;
  serialNumber: string;
  hardwareId: string;
  /** Read-only: From network */
  ipAddress: string;
  macAddress: string;
  onvifVersion: string;
}
```

### Network Settings

```typescript
/** Addressing mode */
type AddressingMode = 'DHCP' | 'Static';

/** Network interface configuration */
interface NetworkSettings {
  /** Hostname for the device */
  hostname: string;
  /** DHCP or Static addressing */
  addressingMode: AddressingMode;
  /** IP address (editable only if Static) */
  ipAddress: string;
  /** Subnet mask (editable only if Static) */
  subnetMask: string;
  /** Default gateway (editable only if Static) */
  gateway: string;
  /** Primary DNS server */
  primaryDns: string;
  /** Secondary DNS server */
  secondaryDns: string;
  /** HTTP service port */
  httpPort: number;
  /** HTTPS service port */
  httpsPort: number;
  /** RTSP streaming port */
  rtspPort: number;
  /** ONVIF discovery enabled */
  discoveryEnabled: boolean;
}

/** Read-only network status */
interface NetworkStatus {
  /** MAC address */
  macAddress: string;
  /** Connection type (WiFi, Ethernet) */
  connectionType: string;
  /** Signal strength (WiFi only, 0-100) */
  signalStrength?: number;
  /** Network uptime in seconds */
  uptimeSeconds: number;
  /** Current connection status */
  isConnected: boolean;
}
```

### Time Settings

```typescript
/** Time synchronization mode */
type TimeMode = 'NTP' | 'Manual';

/** Time configuration */
interface TimeSettings {
  /** NTP or Manual */
  timeMode: TimeMode;
  /** Timezone identifier (e.g., "America/New_York") */
  timezone: string;
  /** NTP servers (if mode is NTP) */
  ntpServers: string[];
  /** Manual date/time (if mode is Manual) */
  manualDateTime?: Date;
  /** Daylight saving time enabled */
  daylightSavingEnabled: boolean;
}

/** Current time state */
interface TimeState {
  /** Current device time (UTC) */
  utcDateTime: Date;
  /** Current device time (local) */
  localDateTime: Date;
  /** Applied timezone */
  timezone: string;
  /** Time mode in use */
  timeMode: TimeMode;
}
```

### Imaging Settings

```typescript
/** Imaging parameters */
interface ImagingSettings {
  /** Brightness level (0.0 - 1.0) */
  brightness: number;
  /** Contrast level (0.0 - 1.0) */
  contrast: number;
  /** Color saturation (0.0 - 1.0) */
  saturation: number;
  /** Sharpness level (0.0 - 1.0) */
  sharpness: number;
}

/** Valid ranges for imaging parameters */
interface ImagingOptions {
  brightness: { min: number; max: number };
  contrast: { min: number; max: number };
  saturation: { min: number; max: number };
  sharpness: { min: number; max: number };
}
```

### User Management

```typescript
/** User account */
interface User {
  /** Username (unique identifier) */
  username: string;
  /** User role/permission level */
  role: UserRole;
}

/** User creation request */
interface CreateUserRequest {
  username: string;
  password: string;
  role: UserRole;
}

/** User update request */
interface UpdateUserRequest {
  username: string;
  password?: string;
  role?: UserRole;
}
```

### Media Profiles

```typescript
/** Video encoding type */
type VideoEncoding = 'H264' | 'H265' | 'JPEG';

/** Video resolution */
interface Resolution {
  width: number;
  height: number;
}

/** Video encoder configuration */
interface VideoEncoderConfig {
  encoding: VideoEncoding;
  resolution: Resolution;
  frameRate: number;
  bitrate: number;
  quality: number;
}

/** Media profile */
interface Profile {
  /** Profile token (unique identifier) */
  token: string;
  /** Profile display name */
  name: string;
  /** Whether this is a fixed/system profile */
  fixed: boolean;
  /** Video encoder configuration */
  videoEncoder?: VideoEncoderConfig;
}

/** Profile creation request */
interface CreateProfileRequest {
  name: string;
  videoEncoderToken?: string;
}
```

### Maintenance Operations

```typescript
/** Maintenance action types */
type MaintenanceAction =
  | 'reboot'
  | 'factoryReset'
  | 'backupConfig'
  | 'restoreConfig';

/** Maintenance operation status */
type OperationStatus = 'pending' | 'in_progress' | 'completed' | 'failed';

/** Maintenance operation result */
interface MaintenanceResult {
  action: MaintenanceAction;
  status: OperationStatus;
  message: string;
  timestamp: Date;
}

/** Backup metadata */
interface BackupInfo {
  filename: string;
  size: number;
  createdAt: Date;
}
```

---

## UI State

### Toast Notifications

```typescript
type ToastType = 'success' | 'error' | 'warning' | 'info';

interface Toast {
  id: string;
  type: ToastType;
  message: string;
  duration?: number;
}
```

### UI State Interface

```typescript
interface UIState {
  /** Mobile sidebar open state */
  sidebarOpen: boolean;
  /** Settings category sidebar open (mobile) */
  settingsSidebarOpen: boolean;
  /** Active settings section */
  activeSettingsSection: string | null;
  /** Toast notifications */
  toasts: Toast[];
  /** Current theme (always dark for this app) */
  theme: 'dark';
}
```

---

## API Response Types

### Generic Response

```typescript
/** Standard API response wrapper */
interface ApiResponse<T = unknown> {
  success: boolean;
  data?: T;
  error?: string;
}

/** SOAP Fault information */
interface SoapFault {
  code: string;
  reason: string;
  detail?: string;
}
```

### Validation

```typescript
/** Form validation result */
interface ValidationResult {
  isValid: boolean;
  errors: Record<string, string>;
}

/** Field validation rule */
interface ValidationRule {
  required?: boolean;
  minLength?: number;
  maxLength?: number;
  pattern?: RegExp;
  custom?: (value: unknown) => string | null;
}
```

---

## State Relationships

```text
┌─────────────────┐
│   AuthState     │
│  (Redux slice)  │
└────────┬────────┘
         │ user.role determines permissions
         ▼
┌─────────────────┐     ┌─────────────────┐
│  DeviceState    │────▶│  SettingsPages  │
│  (Redux slice)  │     │  (local state)  │
└────────┬────────┘     └────────┬────────┘
         │                       │
         │ device.status         │ form data
         ▼                       ▼
┌─────────────────┐     ┌─────────────────┐
│    UIState      │     │   Services      │
│  (Redux slice)  │     │  (API calls)    │
└─────────────────┘     └─────────────────┘
```

---

## Validation Rules

| Entity | Field | Rule |
|--------|-------|------|
| User | username | Required, 1-32 chars, alphanumeric + underscore |
| User | password | Required, min 6 chars |
| Network | ipAddress | Valid IPv4 format |
| Network | subnetMask | Valid subnet mask |
| Network | port | Integer 1-65535 |
| Time | ntpServers | Valid hostname or IP |
| Imaging | brightness | 0.0 - 1.0 |
| Profile | name | Required, 1-64 chars |
