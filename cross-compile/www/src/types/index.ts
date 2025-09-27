// Camera and ONVIF related types
export interface CameraConfig {
  ip: string;
  onvifPort: number;
  snapshotPort: number;
  rtspPort: number;
}

export interface PTZDirection {
  x: number;
  y: number;
}

export interface PTZSpeeds {
  up: PTZDirection;
  down: PTZDirection;
  left: PTZDirection;
  right: PTZDirection;
  left_up: PTZDirection;
  right_up: PTZDirection;
  left_down: PTZDirection;
  right_down: PTZDirection;
}

export interface ImagingSettings {
  brightness: number;
  contrast: number;
  saturation: number;
}

export interface ONVIFStatus {
  status: 'online' | 'offline' | 'checking';
  lastChecked: Date;
  error?: string;
}

export interface ONVIFResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
}

export interface DeviceInfo {
  manufacturer: string;
  model: string;
  firmwareVersion: string;
  serialNumber: string;
  hardwareId: string;
  systemDateAndTime: {
    utcDateTime: Date;
    timeZone: string;
    daylightSavingTime: boolean;
  };
}

export interface SystemCapabilities {
  network: {
    ipFilter: boolean;
    zeroConfiguration: boolean;
    ipVersion6: boolean;
    dynDNS: boolean;
    dot11Configuration: boolean;
    dot1XConfiguration: boolean;
    hostnameFromDHCP: boolean;
    ntp: boolean;
    dhcpv6: boolean;
  };
  security: {
    tls1_0: boolean;
    tls1_1: boolean;
    tls1_2: boolean;
    onboardKeyGeneration: boolean;
    accessPolicyConfig: boolean;
    x509Token: boolean;
    samlToken: boolean;
    kerberosToken: boolean;
    relToken: boolean;
  };
  system: {
    discoveryResolve: boolean;
    discoveryBye: boolean;
    remoteDiscovery: boolean;
    systemBackup: boolean;
    systemLogging: boolean;
    firmwareUpgrade: boolean;
    supportedVersions: {
      major: number;
      minor: number;
    }[];
  };
}

export interface DeviceStatus {
  online: boolean;
  lastChecked: Date;
  deviceInfo: string;
  capabilities: string;
  services: {
    device: boolean;
    media: boolean;
    ptz: boolean;
    imaging: boolean;
  };
}


export interface Endpoint {
  label: string;
  url: string;
  type: 'rtsp' | 'http' | 'onvif';
}

// Redux state types
export interface RootState {
  camera: CameraState;
  onvif: ONVIFState;
  ui: UIState;
  device: DeviceState;
  systemUtilization: SystemUtilizationState;
}

export interface CameraState {
  ip: string;
  name: string;
  type: string;
  isConnected: boolean;
  snapshotUrl: string;
  isPlaying: boolean;
  lastSnapshotTime: Date | null;
}

export interface ONVIFState {
  status: 'online' | 'offline' | 'checking';
  lastChecked: Date | null;
  error: string | null;
  isInitialized: boolean;
}

export interface UIState {
  currentTime: Date;
  sidebarCollapsed: boolean;
  fullscreenMode: boolean;
}

export interface DeviceState {
  // Add device state properties as needed
  // This should match the deviceSlice state
}

export interface SystemUtilizationState {
  systemInfo: SystemInfo | null;
  cpuHistory: DataPoint[];
  tempHistory: DataPoint[];
  isLoading: boolean;
  error: string | null;
  isAutoRefresh: boolean;
  lastUpdated: Date | null;
  connectionStatus: 'connected' | 'disconnected' | 'checking';
}

// Component prop types
export interface SidebarProps {
  // No props needed for now
}

export interface HeaderProps {
  cameraName: string;
  cameraType: string;
  currentTime: Date;
}

export interface VideoFeedProps {
  cameraIP: string;
}

export interface PTZControlsProps {
  onPTZAction: (action: 'init' | 'stop' | 'move', direction?: string) => Promise<void>;
}

export interface SystemInfoProps {
  cameraIP: string;
  onvifStatus: ONVIFStatus['status'];
  onStatusCheck: () => void;
}

// Service types
export interface ONVIFResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
}

export interface SOAPRequest {
  service: string;
  action: string;
  body: string;
}

// System utilization types
export interface SystemInfo {
  cpu_usage: number;
  cpu_temperature: number;
  memory_total: number;
  memory_free: number;
  memory_used: number;
  uptime_ms: number;
  timestamp: number;
}

export interface DataPoint {
  timestamp: number;
  value: number;
}

export interface SystemUtilizationResponse {
  success: boolean;
  data?: SystemInfo;
  error?: string;
}