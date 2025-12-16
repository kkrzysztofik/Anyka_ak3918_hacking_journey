/**
 * Dashboard Page
 *
 * Landing page after login showing device info and navigation.
 */

import React from 'react'
import { Camera, Settings, Activity, Info, Loader2 } from 'lucide-react'
import { Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { useAuth } from '@/hooks/useAuth'
import { verifyCredentials, type DeviceInfo } from '@/services/authService'

export default function DashboardPage() {
  const { username, getCredentials } = useAuth()

  // Fetch device info
  const { data: deviceInfo, isLoading, error } = useQuery<DeviceInfo | undefined>({
    queryKey: ['deviceInfo'],
    queryFn: async () => {
      const creds = getCredentials()
      if (!creds) return undefined
      const result = await verifyCredentials(creds.username, creds.password)
      return result.deviceInfo
    },
    staleTime: 60 * 1000, // 1 minute
  })

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Welcome, {username}</h1>
        <p className="text-muted-foreground mt-1">Camera administration dashboard</p>
      </div>

      {/* Device Info Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Info className="w-5 h-5" />
            Device Information
          </CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Loader2 className="w-4 h-4 animate-spin" />
              Loading device info...
            </div>
          ) : error ? (
            <p className="text-destructive">Failed to load device information</p>
          ) : deviceInfo ? (
            <dl className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <dt className="text-muted-foreground">Manufacturer</dt>
                <dd className="font-medium">{deviceInfo.manufacturer}</dd>
              </div>
              <div>
                <dt className="text-muted-foreground">Model</dt>
                <dd className="font-medium">{deviceInfo.model}</dd>
              </div>
              <div>
                <dt className="text-muted-foreground">Firmware</dt>
                <dd className="font-medium">{deviceInfo.firmwareVersion}</dd>
              </div>
              <div>
                <dt className="text-muted-foreground">Serial</dt>
                <dd className="font-medium">{deviceInfo.serialNumber}</dd>
              </div>
            </dl>
          ) : (
            <p className="text-muted-foreground">No device information available</p>
          )}
        </CardContent>
      </Card>

      {/* Quick Actions */}
      <div className="grid gap-4 md:grid-cols-3">
        <Link to="/live">
          <Card className="hover:bg-accent transition-colors cursor-pointer h-full">
            <CardHeader className="flex flex-row items-center gap-4">
              <Camera className="w-8 h-8 text-primary" />
              <div>
                <CardTitle>Live View</CardTitle>
                <CardDescription>View camera stream</CardDescription>
              </div>
            </CardHeader>
          </Card>
        </Link>

        <Link to="/settings/identification">
          <Card className="hover:bg-accent transition-colors cursor-pointer h-full">
            <CardHeader className="flex flex-row items-center gap-4">
              <Settings className="w-8 h-8 text-primary" />
              <div>
                <CardTitle>Settings</CardTitle>
                <CardDescription>Configure device</CardDescription>
              </div>
            </CardHeader>
          </Card>
        </Link>

        <Link to="/diagnostics">
          <Card className="hover:bg-accent transition-colors cursor-pointer h-full">
            <CardHeader className="flex flex-row items-center gap-4">
              <Activity className="w-8 h-8 text-primary" />
              <div>
                <CardTitle>Diagnostics</CardTitle>
                <CardDescription>System status</CardDescription>
              </div>
            </CardHeader>
          </Card>
        </Link>
      </div>
    </div>
  )
}
