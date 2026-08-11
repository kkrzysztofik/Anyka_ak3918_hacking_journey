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
