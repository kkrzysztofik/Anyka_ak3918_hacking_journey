/**
 * Diagnostics Page
 *
 * System diagnostics and status monitoring.
 */

import React from 'react'
import { Activity, Cpu, HardDrive, Thermometer, Clock, Wifi } from 'lucide-react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'

// Mock data for demonstration
const mockStats = {
  uptime: '3d 14h 22m',
  cpuUsage: 23,
  memoryUsage: 45,
  temperature: 42,
  networkRx: '1.2 MB/s',
  networkTx: '0.8 MB/s',
}

function StatCard({
  icon: Icon,
  label,
  value,
  unit,
  color = 'text-primary'
}: {
  icon: React.ElementType
  label: string
  value: string | number
  unit?: string
  color?: string
}) {
  return (
    <Card>
      <CardContent className="pt-6">
        <div className="flex items-center gap-4">
          <div className={`p-3 rounded-lg bg-muted ${color}`}>
            <Icon className="w-6 h-6" />
          </div>
          <div>
            <p className="text-sm text-muted-foreground">{label}</p>
            <p className="text-2xl font-bold">
              {value}
              {unit && <span className="text-sm font-normal text-muted-foreground ml-1">{unit}</span>}
            </p>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

function ProgressBar({ value, max = 100, color = 'bg-primary' }: { value: number; max?: number; color?: string }) {
  const percentage = Math.min((value / max) * 100, 100)
  return (
    <div className="h-2 bg-muted rounded-full overflow-hidden">
      <div
        className={`h-full ${color} transition-all`}
        style={{ width: `${percentage}%` }}
      />
    </div>
  )
}

export default function DiagnosticsPage() {
  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Diagnostics</h1>

      {/* Quick Stats */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        <StatCard icon={Clock} label="Uptime" value={mockStats.uptime} />
        <StatCard icon={Thermometer} label="Temperature" value={mockStats.temperature} unit="°C" color="text-orange-500" />
        <StatCard icon={Wifi} label="Network RX" value={mockStats.networkRx} />
      </div>

      {/* System Resources */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="w-5 h-5" />
            System Resources
          </CardTitle>
          <CardDescription>
            Current resource utilization
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div>
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <Cpu className="w-4 h-4 text-muted-foreground" />
                <span className="text-sm font-medium">CPU Usage</span>
              </div>
              <span className="text-sm text-muted-foreground">{mockStats.cpuUsage}%</span>
            </div>
            <ProgressBar value={mockStats.cpuUsage} color="bg-blue-500" />
          </div>

          <div>
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <HardDrive className="w-4 h-4 text-muted-foreground" />
                <span className="text-sm font-medium">Memory Usage</span>
              </div>
              <span className="text-sm text-muted-foreground">{mockStats.memoryUsage}%</span>
            </div>
            <ProgressBar value={mockStats.memoryUsage} color="bg-green-500" />
          </div>
        </CardContent>
      </Card>

      {/* System Logs Placeholder */}
      <Card>
        <CardHeader>
          <CardTitle>System Logs</CardTitle>
          <CardDescription>Recent system events</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="font-mono text-xs bg-muted p-4 rounded-lg space-y-1 max-h-64 overflow-auto">
            <div className="text-muted-foreground">[2024-01-15 10:23:45] System started</div>
            <div className="text-muted-foreground">[2024-01-15 10:23:46] Network interface eth0 up</div>
            <div className="text-muted-foreground">[2024-01-15 10:23:47] ONVIF service started on port 8080</div>
            <div className="text-muted-foreground">[2024-01-15 10:23:48] Video encoder initialized</div>
            <div className="text-green-500">[2024-01-15 10:23:49] System ready</div>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
