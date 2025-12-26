/**
 * Connection Status Indicator
 *
 * Shows online/offline status of the camera device.
 */
import React from 'react';

import { Loader2, Wifi, WifiOff } from 'lucide-react';

import { cn } from '@/lib/utils';

export type ConnectionState = 'connected' | 'disconnected' | 'checking';

interface ConnectionStatusProps {
  readonly status: ConnectionState;
  readonly className?: string;
}

export function ConnectionStatus({ status, className }: ConnectionStatusProps) {
  return (
    <output
      className={cn('flex items-center gap-2 text-sm', className)}
      aria-live="polite"
      data-testid={`connection-status-${status}`}
    >
      {status === 'connected' && (
        <>
          <Wifi className="h-4 w-4 text-green-500" aria-hidden="true" />
          <span className="text-green-500">Connected</span>
        </>
      )}
      {status === 'disconnected' && (
        <>
          <WifiOff className="text-destructive h-4 w-4" aria-hidden="true" />
          <span className="text-destructive">Disconnected</span>
        </>
      )}
      {status === 'checking' && (
        <>
          <Loader2 className="text-muted-foreground h-4 w-4 animate-spin" aria-hidden="true" />
          <span className="text-muted-foreground">Checking...</span>
        </>
      )}
    </output>
  );
}

/**
 * Compact version for header
 */
export function ConnectionStatusBadge({ status, className }: ConnectionStatusProps) {
  let statusText: string;
  if (status === 'connected') {
    statusText = 'Connected to device';
  } else if (status === 'disconnected') {
    statusText = 'Disconnected from device';
  } else {
    statusText = 'Checking connection status';
  }

  return (
    <output
      className={cn(
        'flex items-center gap-1.5 rounded-full px-2 py-1 text-xs font-medium',
        status === 'connected' && 'bg-green-500/10 text-green-500',
        status === 'disconnected' && 'bg-destructive/10 text-destructive',
        status === 'checking' && 'bg-muted text-muted-foreground',
        className,
      )}
      aria-live="polite"
      data-testid={`connection-status-badge-${status}`}
    >
      {status === 'connected' && <Wifi className="h-3 w-3" aria-hidden="true" />}
      {status === 'disconnected' && <WifiOff className="h-3 w-3" aria-hidden="true" />}
      {status === 'checking' && <Loader2 className="h-3 w-3 animate-spin" aria-hidden="true" />}
      <span className="sr-only">{statusText}</span>
    </output>
  );
}
