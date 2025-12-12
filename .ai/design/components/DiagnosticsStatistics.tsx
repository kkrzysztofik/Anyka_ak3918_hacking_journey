import { useState, useEffect } from 'react';
import {
  Activity,
  Cpu,
  HardDrive,
  Thermometer,
  Wifi,
  Camera,
  AlertTriangle,
  CheckCircle,
  XCircle,
  Clock,
  TrendingUp,
  Server,
  BarChart3,
  FileText,
  RefreshCw,
  Download,
} from 'lucide-react';
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';

interface LogEntry {
  id: number;
  timestamp: string;
  level: 'info' | 'warning' | 'error';
  category: string;
  message: string;
}

export default function DiagnosticsStatistics() {
  const [uptime, setUptime] = useState('12d 5h 32m');
  const [cpuData, setCpuData] = useState([
    { time: '00:00', value: 45 },
    { time: '00:05', value: 52 },
    { time: '00:10', value: 48 },
    { time: '00:15', value: 55 },
    { time: '00:20', value: 62 },
    { time: '00:25', value: 58 },
    { time: '00:30', value: 51 },
  ]);
  const [memoryData, setMemoryData] = useState([
    { time: '00:00', value: 65 },
    { time: '00:05', value: 67 },
    { time: '00:10', value: 68 },
    { time: '00:15', value: 70 },
    { time: '00:20', value: 72 },
    { time: '00:25', value: 71 },
    { time: '00:30', value: 69 },
  ]);
  const [networkData, setNetworkData] = useState([
    { time: '00:00', upload: 2.5, download: 4.2 },
    { time: '00:05', upload: 2.8, download: 4.5 },
    { time: '00:10', upload: 2.6, download: 4.3 },
    { time: '00:15', upload: 3.1, download: 4.8 },
    { time: '00:20', upload: 2.9, download: 4.6 },
    { time: '00:25', upload: 2.7, download: 4.4 },
    { time: '00:30', upload: 2.8, download: 4.5 },
  ]);

  const [logs, setLogs] = useState<LogEntry[]>([
    {
      id: 1,
      timestamp: '2025-12-11 14:32:15',
      level: 'info',
      category: 'Stream',
      message: 'Main stream started successfully',
    },
    {
      id: 2,
      timestamp: '2025-12-11 14:30:42',
      level: 'warning',
      category: 'Network',
      message: 'High latency detected: 125ms',
    },
    {
      id: 3,
      timestamp: '2025-12-11 14:28:03',
      level: 'info',
      category: 'PTZ',
      message: 'Preset position 1 updated',
    },
    {
      id: 4,
      timestamp: '2025-12-11 14:25:17',
      level: 'error',
      category: 'Storage',
      message: 'Storage capacity at 85%',
    },
    {
      id: 5,
      timestamp: '2025-12-11 14:20:55',
      level: 'info',
      category: 'System',
      message: 'Firmware version 2.4.1 running',
    },
    {
      id: 6,
      timestamp: '2025-12-11 14:18:33',
      level: 'info',
      category: 'Auth',
      message: 'User admin logged in',
    },
    {
      id: 7,
      timestamp: '2025-12-11 14:15:21',
      level: 'warning',
      category: 'Temperature',
      message: 'Camera temperature: 68°C',
    },
    {
      id: 8,
      timestamp: '2025-12-11 14:10:09',
      level: 'info',
      category: 'Network',
      message: 'DHCP lease renewed',
    },
  ]);

  const [selectedLogLevel, setSelectedLogLevel] = useState<'all' | 'info' | 'warning' | 'error'>(
    'all'
  );

  const filteredLogs =
    selectedLogLevel === 'all' ? logs : logs.filter((log) => log.level === selectedLogLevel);

  return (
    <div className="absolute inset-0 lg:inset-[0_0_0_76.84px] overflow-auto bg-[#0d0d0d]">
      <div className="p-[16px] md:p-[32px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[24px] lg:mb-[32px]">
          <h1 className="text-white text-[22px] md:text-[28px] mb-[8px]">Diagnostics & Statistics</h1>
          <p className="text-[#a1a1a6] text-[13px] md:text-[14px]">
            Real-time system monitoring and performance metrics
          </p>
        </div>

        {/* System Status Overview */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-[16px] md:gap-[20px] mb-[20px] md:mb-[24px]">
          {/* Overall Status */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[20px]">
            <div className="flex items-center justify-between mb-[10px] md:mb-[12px]">
              <div className="flex items-center gap-[8px] md:gap-[10px]">
                <div className="size-[36px] md:size-[40px] rounded-[10px] bg-[#15803d20] flex items-center justify-center">
                  <CheckCircle className="size-4 md:size-5 text-[#4ade80]" />
                </div>
                <div>
                  <div className="text-[#a1a1a6] text-[12px] md:text-[13px]">System Status</div>
                  <div className="text-white text-[16px] md:text-[18px]">Healthy</div>
                </div>
              </div>
            </div>
            <div className="flex items-center gap-[6px] text-[#4ade80] text-[12px] md:text-[13px]">
              <div className="size-[6px] rounded-full bg-[#4ade80] animate-pulse"></div>
              Online
            </div>
          </div>

          {/* CPU Usage */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[20px]">
            <div className="flex items-center justify-between mb-[10px] md:mb-[12px]">
              <div className="flex items-center gap-[8px] md:gap-[10px]">
                <div className="size-[36px] md:size-[40px] rounded-[10px] bg-[#dc262620] flex items-center justify-center">
                  <Cpu className="size-4 md:size-5 text-[#dc2626]" />
                </div>
                <div>
                  <div className="text-[#a1a1a6] text-[12px] md:text-[13px]">CPU Usage</div>
                  <div className="text-white text-[16px] md:text-[18px]">51%</div>
                </div>
              </div>
            </div>
            <div className="text-[#a1a1a6] text-[12px] md:text-[13px]">Avg: 48%</div>
          </div>

          {/* Memory Usage */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[20px]">
            <div className="flex items-center justify-between mb-[10px] md:mb-[12px]">
              <div className="flex items-center gap-[8px] md:gap-[10px]">
                <div className="size-[36px] md:size-[40px] rounded-[10px] bg-[#ff8c0020] flex items-center justify-center">
                  <HardDrive className="size-4 md:size-5 text-[#ff8c00]" />
                </div>
                <div>
                  <div className="text-[#a1a1a6] text-[12px] md:text-[13px]">Memory</div>
                  <div className="text-white text-[16px] md:text-[18px]">69%</div>
                </div>
              </div>
            </div>
            <div className="text-[#a1a1a6] text-[12px] md:text-[13px]">1.4 GB / 2.0 GB</div>
          </div>

          {/* Temperature */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[20px]">
            <div className="flex items-center justify-between mb-[10px] md:mb-[12px]">
              <div className="flex items-center gap-[8px] md:gap-[10px]">
                <div className="size-[36px] md:size-[40px] rounded-[10px] bg-[#3b82f620] flex items-center justify-center">
                  <Thermometer className="size-4 md:size-5 text-[#3b82f6]" />
                </div>
                <div>
                  <div className="text-[#a1a1a6] text-[12px] md:text-[13px]">Temperature</div>
                  <div className="text-white text-[16px] md:text-[18px]">64°C</div>
                </div>
              </div>
            </div>
            <div className="text-[#a1a1a6] text-[12px] md:text-[13px]">Normal range</div>
          </div>
        </div>

        {/* Performance Charts */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-[20px] md:gap-[24px] mb-[20px] md:mb-[24px]">
          {/* CPU Usage Chart */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[24px]">
            <div className="flex items-center justify-between mb-[16px] md:mb-[20px]">
              <div className="flex items-center gap-[12px]">
                <Activity className="size-4 md:size-5 text-[#dc2626]" />
                <h2 className="text-white text-[16px] md:text-[18px]">CPU Usage</h2>
              </div>
              <button className="size-[28px] md:size-[32px] rounded-[8px] bg-[#3a3a3c] hover:bg-[#4a4a4c] flex items-center justify-center transition-colors">
                <RefreshCw className="size-3 md:size-4 text-white" />
              </button>
            </div>
            <ResponsiveContainer width="100%" height={200}>
              <AreaChart data={cpuData}>
                <defs>
                  <linearGradient id="cpuGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#dc2626" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#dc2626" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#3a3a3c" />
                <XAxis dataKey="time" stroke="#a1a1a6" style={{ fontSize: '12px' }} />
                <YAxis stroke="#a1a1a6" style={{ fontSize: '12px' }} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#1c1c1e',
                    border: '1px solid #3a3a3c',
                    borderRadius: '8px',
                    color: '#fff',
                  }}
                />
                <Area
                  type="monotone"
                  dataKey="value"
                  stroke="#dc2626"
                  strokeWidth={2}
                  fill="url(#cpuGradient)"
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>

          {/* Memory Usage Chart */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[24px]">
            <div className="flex items-center justify-between mb-[16px] md:mb-[20px]">
              <div className="flex items-center gap-[12px]">
                <HardDrive className="size-4 md:size-5 text-[#dc2626]" />
                <h2 className="text-white text-[16px] md:text-[18px]">Memory Usage</h2>
              </div>
              <button className="size-[28px] md:size-[32px] rounded-[8px] bg-[#3a3a3c] hover:bg-[#4a4a4c] flex items-center justify-center transition-colors">
                <RefreshCw className="size-3 md:size-4 text-white" />
              </button>
            </div>
            <ResponsiveContainer width="100%" height={200}>
              <AreaChart data={memoryData}>
                <defs>
                  <linearGradient id="memoryGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#ff8c00" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#ff8c00" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#3a3a3c" />
                <XAxis dataKey="time" stroke="#a1a1a6" style={{ fontSize: '12px' }} />
                <YAxis stroke="#a1a1a6" style={{ fontSize: '12px' }} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#1c1c1e',
                    border: '1px solid #3a3a3c',
                    borderRadius: '8px',
                    color: '#fff',
                  }}
                />
                <Area
                  type="monotone"
                  dataKey="value"
                  stroke="#ff8c00"
                  strokeWidth={2}
                  fill="url(#memoryGradient)"
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* Network Statistics */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[24px] mb-[20px] md:mb-[24px]">
          <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-[16px] md:mb-[20px] gap-[12px]">
            <div className="flex items-center gap-[12px]">
              <Wifi className="size-4 md:size-5 text-[#dc2626]" />
              <h2 className="text-white text-[16px] md:text-[18px]">Network Throughput</h2>
            </div>
            <div className="flex gap-[16px] md:gap-[20px]">
              <div className="flex items-center gap-[8px]">
                <div className="size-[10px] md:size-[12px] rounded-[3px] bg-[#4ade80]"></div>
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Upload</span>
              </div>
              <div className="flex items-center gap-[8px]">
                <div className="size-[10px] md:size-[12px] rounded-[3px] bg-[#3b82f6]"></div>
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Download</span>
              </div>
            </div>
          </div>
          <ResponsiveContainer width="100%" height={220}>
            <LineChart data={networkData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#3a3a3c" />
              <XAxis dataKey="time" stroke="#a1a1a6" style={{ fontSize: '12px' }} />
              <YAxis stroke="#a1a1a6" style={{ fontSize: '12px' }} />
              <Tooltip
                contentStyle={{
                  backgroundColor: '#1c1c1e',
                  border: '1px solid #3a3a3c',
                  borderRadius: '8px',
                  color: '#fff',
                }}
              />
              <Line
                type="monotone"
                dataKey="upload"
                stroke="#4ade80"
                strokeWidth={2}
                dot={false}
              />
              <Line
                type="monotone"
                dataKey="download"
                stroke="#3b82f6"
                strokeWidth={2}
                dot={false}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>

        {/* Device Information & System Metrics */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-[20px] md:gap-[24px] mb-[20px] md:mb-[24px]">
          {/* Device Information */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[24px]">
            <div className="flex items-center gap-[12px] mb-[16px] md:mb-[20px]">
              <Camera className="size-4 md:size-5 text-[#dc2626]" />
              <h2 className="text-white text-[16px] md:text-[18px]">Device Information</h2>
            </div>
            <div className="space-y-[12px] md:space-y-[14px]">
              <div className="flex justify-between items-center">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Model</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">ONVIF-CAM-4K-Pro</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Firmware</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">v2.4.1</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Serial Number</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">ONV-2025-ABC123</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">MAC Address</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">00:1A:2B:3C:4D:5E</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">IP Address</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">192.168.1.100</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Uptime</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">{uptime}</span>
              </div>
            </div>
          </div>

          {/* System Metrics */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[24px]">
            <div className="flex items-center gap-[12px] mb-[16px] md:mb-[20px]">
              <Server className="size-4 md:size-5 text-[#dc2626]" />
              <h2 className="text-white text-[16px] md:text-[18px]">System Metrics</h2>
            </div>
            <div className="space-y-[12px] md:space-y-[14px]">
              <div className="flex justify-between items-center">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Storage Used</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">85% (42.5 GB / 50 GB)</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Active Streams</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">2</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Recordings</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">1,247 files</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Frame Drops (24h)</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">12 frames</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Avg Bitrate</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">4.2 Mbps</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Packet Loss</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">0.02%</span>
              </div>
            </div>
          </div>
        </div>

        {/* System Logs */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[24px]">
          <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-[16px] md:mb-[20px] gap-[12px]">
            <div className="flex items-center gap-[12px]">
              <FileText className="size-4 md:size-5 text-[#dc2626]" />
              <h2 className="text-white text-[16px] md:text-[18px]">System Logs</h2>
            </div>
            <div className="flex flex-col sm:flex-row gap-[10px] sm:gap-[12px] w-full sm:w-auto">
              {/* Filter Buttons */}
              <div className="flex gap-[8px] overflow-x-auto">
                <button
                  onClick={() => setSelectedLogLevel('all')}
                  className={`px-[10px] md:px-[12px] py-[5px] md:py-[6px] rounded-[6px] text-[12px] md:text-[13px] transition-colors whitespace-nowrap ${
                    selectedLogLevel === 'all'
                      ? 'bg-[#dc2626] text-white'
                      : 'bg-[#3a3a3c] text-[#a1a1a6] hover:bg-[#4a4a4c]'
                  }`}
                >
                  All
                </button>
                <button
                  onClick={() => setSelectedLogLevel('info')}
                  className={`px-[10px] md:px-[12px] py-[5px] md:py-[6px] rounded-[6px] text-[12px] md:text-[13px] transition-colors whitespace-nowrap ${
                    selectedLogLevel === 'info'
                      ? 'bg-[#dc2626] text-white'
                      : 'bg-[#3a3a3c] text-[#a1a1a6] hover:bg-[#4a4a4c]'
                  }`}
                >
                  Info
                </button>
                <button
                  onClick={() => setSelectedLogLevel('warning')}
                  className={`px-[10px] md:px-[12px] py-[5px] md:py-[6px] rounded-[6px] text-[12px] md:text-[13px] transition-colors whitespace-nowrap ${
                    selectedLogLevel === 'warning'
                      ? 'bg-[#dc2626] text-white'
                      : 'bg-[#3a3a3c] text-[#a1a1a6] hover:bg-[#4a4a4c]'
                  }`}
                >
                  Warning
                </button>
                <button
                  onClick={() => setSelectedLogLevel('error')}
                  className={`px-[10px] md:px-[12px] py-[5px] md:py-[6px] rounded-[6px] text-[12px] md:text-[13px] transition-colors whitespace-nowrap ${
                    selectedLogLevel === 'error'
                      ? 'bg-[#dc2626] text-white'
                      : 'bg-[#3a3a3c] text-[#a1a1a6] hover:bg-[#4a4a4c]'
                  }`}
                >
                  Error
                </button>
              </div>
              <button className="px-[14px] md:px-[16px] py-[5px] md:py-[6px] bg-[#3a3a3c] hover:bg-[#4a4a4c] text-white rounded-[8px] transition-colors text-[12px] md:text-[13px] flex items-center justify-center gap-[6px]">
                <Download className="size-3 md:size-4" />
                Export
              </button>
            </div>
          </div>

          {/* Logs Table - Mobile Responsive */}
          <div className="border border-[#3a3a3c] rounded-[8px] overflow-hidden">
            {/* Desktop Table Header */}
            <div className="hidden md:block bg-[#0d0d0d] border-b border-[#3a3a3c]">
              <div className="grid grid-cols-[140px_80px_120px_1fr] gap-[16px] px-[16px] py-[12px]">
                <div className="text-[#a1a1a6] text-[13px]">Timestamp</div>
                <div className="text-[#a1a1a6] text-[13px]">Level</div>
                <div className="text-[#a1a1a6] text-[13px]">Category</div>
                <div className="text-[#a1a1a6] text-[13px]">Message</div>
              </div>
            </div>
            <div className="max-h-[400px] overflow-y-auto">
              {filteredLogs.map((log) => (
                <div
                  key={log.id}
                  className="grid md:grid-cols-[140px_80px_120px_1fr] gap-[8px] md:gap-[16px] px-[16px] py-[12px] border-b border-[#3a3a3c] hover:bg-[#0d0d0d] transition-colors"
                >
                  <div className="text-[#a1a1a6] text-[12px] md:text-[13px] font-mono flex items-center gap-[6px]">
                    <Clock className="size-3" />
                    {log.timestamp}
                  </div>
                  <div className="flex items-center">
                    {log.level === 'info' && (
                      <span className="px-[8px] py-[2px] rounded-[4px] bg-[#3b82f620] text-[#3b82f6] text-[11px] md:text-[12px] flex items-center gap-[4px]">
                        <CheckCircle className="size-3" />
                        Info
                      </span>
                    )}
                    {log.level === 'warning' && (
                      <span className="px-[8px] py-[2px] rounded-[4px] bg-[#ff8c0020] text-[#ff8c00] text-[11px] md:text-[12px] flex items-center gap-[4px]">
                        <AlertTriangle className="size-3" />
                        Warning
                      </span>
                    )}
                    {log.level === 'error' && (
                      <span className="px-[8px] py-[2px] rounded-[4px] bg-[#dc262620] text-[#dc2626] text-[11px] md:text-[12px] flex items-center gap-[4px]">
                        <XCircle className="size-3" />
                        Error
                      </span>
                    )}
                  </div>
                  <div className="text-white text-[12px] md:text-[13px]">{log.category}</div>
                  <div className="text-[#a1a1a6] text-[12px] md:text-[13px]">{log.message}</div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}