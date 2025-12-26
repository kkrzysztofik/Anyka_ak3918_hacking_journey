/**
 * Shared test utilities for React component tests
 * Provides common wrappers, QueryClient setup, and mock factories
 */
/* eslint-disable react-refresh/only-export-components */
import type { ReactElement, ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
  type RenderOptions,
  type RenderResult,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect } from 'vitest';

import { AuthProvider } from '@/hooks/useAuth';
import type { ImagingOptions, ImagingSettings } from '@/services/imagingService';
import type { MediaProfile } from '@/services/profileService';

/**
 * Create a QueryClient configured for testing
 * Disables retries for predictable test behavior
 */
export function createTestQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
}

interface WrapperProps {
  children: ReactNode;
}

interface RenderWithProvidersOptions extends Omit<RenderOptions, 'wrapper'> {
  queryClient?: QueryClient;
}

/**
 * Create a wrapper with all required providers for testing
 */
function createWrapper(queryClient: QueryClient) {
  return function Wrapper({ children }: WrapperProps) {
    return (
      <QueryClientProvider client={queryClient}>
        <AuthProvider>{children}</AuthProvider>
      </QueryClientProvider>
    );
  };
}

/**
 * Render a component with all required providers
 * Use this instead of RTL's render for page/component tests
 *
 * @example
 * ```tsx
 * const { getByTestId } = renderWithProviders(<MyPage />);
 * ```
 */
export function renderWithProviders(
  ui: ReactElement,
  { queryClient = createTestQueryClient(), ...options }: RenderWithProvidersOptions = {},
): RenderResult & { queryClient: QueryClient } {
  const result = render(ui, {
    wrapper: createWrapper(queryClient),
    ...options,
  });

  return {
    ...result,
    queryClient,
  };
}

export { toast as mockToast } from 'sonner';

/**
 * API endpoints constants for mock setup
 */
export const MOCK_ENDPOINTS = {
  device: '/onvif/device_service',
  media: '/onvif/media_service',
  ptz: '/onvif/ptz_service',
  imaging: '/onvif/imaging_service',
} as const;

/**
 * Common mock data for tests
 */
export const MOCK_DATA = {
  profiles: [
    {
      token: 'ProfileToken1',
      name: 'MainStream',
      fixed: false,
      videoSourceConfiguration: {
        token: 'VideoSourceToken1',
        name: 'Video Source 1',
      },
      videoEncoderConfiguration: {
        token: 'VideoEncoderToken1',
        name: 'H.264 Encoder',
        encoding: 'H264',
      },
    },
    {
      token: 'ProfileToken2',
      name: 'SubStream',
      fixed: true,
      videoSourceConfiguration: {
        token: 'VideoSourceToken2',
        name: 'Video Source 2',
      },
    },
  ] as MediaProfile[],
  network: {
    interfaces: [
      {
        token: 'eth0',
        name: 'eth0',
        enabled: true,
        ipv4Enabled: true,
        dhcp: false,
        address: '192.168.1.100',
        prefixLength: 24,
        gateway: '192.168.1.1',
        hwAddress: '00:11:22:33:44:55',
      },
    ],
    dns: {
      fromDHCP: false,
      dnsServers: ['8.8.8.8', '8.8.4.4'],
      searchDomain: [],
    },
  },
  device: {
    deviceInfo: {
      manufacturer: 'Test Manufacturer',
      model: 'Test Model',
      firmwareVersion: '1.0.0',
      serialNumber: 'SN123',
      hardwareId: 'HW456',
    },
    name: 'Test Device',
    location: 'Test Location',
  },
  imaging: {
    settings: {
      brightness: 60,
      contrast: 70,
      saturation: 50,
      sharpness: 55,
      irCutFilter: 'AUTO',
      wideDynamicRange: {
        mode: 'ON',
        level: 60,
      },
      backlightCompensation: {
        mode: 'ON',
        level: 45,
      },
    } as ImagingSettings,
    options: {
      brightness: { min: 0, max: 100 },
      contrast: { min: 0, max: 100 },
      saturation: { min: 0, max: 100 },
      sharpness: { min: 0, max: 100 },
      irCutFilterModes: ['AUTO', 'ON', 'OFF'],
      wideDynamicRange: {
        modes: ['ON', 'OFF'],
        level: { min: 0, max: 100 },
      },
      backlightCompensation: {
        modes: ['ON', 'OFF'],
        level: { min: 0, max: 100 },
      },
    } as ImagingOptions,
  },
  videoEncoder: {
    configuration: {
      token: 'VideoEncoderToken1',
      name: 'H.264 Encoder',
      encoding: 'H264',
      resolution: { width: 1920, height: 1080 },
      quality: 80,
      rateControl: {
        frameRateLimit: 30,
        encodingInterval: 1,
        bitrateLimit: 4000,
      },
      h264: {
        govLength: 30,
        h264Profile: 'Main',
      },
      sessionTimeout: 'PT60S',
    },
    options: {
      qualityRange: { min: 0, max: 100 },
      h264: {
        resolutionsAvailable: [
          { width: 1920, height: 1080 },
          { width: 1280, height: 720 },
        ],
        frameRateRange: { min: 1, max: 30 },
        encodingIntervalRange: { min: 1, max: 30 },
        bitrateRange: { min: 64, max: 8192 },
        h264ProfilesSupported: ['Main', 'High'],
        govLengthRange: { min: 1, max: 300 },
      },
    },
  },
  users: [
    {
      username: 'admin',
      userLevel: 'Administrator',
    },
    {
      username: 'operator',
      userLevel: 'Operator',
    },
    {
      username: 'user1',
      userLevel: 'User',
    },
  ] as const,
};

/**
 * Helper to verify password visibility toggle behavior
 */
export async function verifyPasswordVisibilityToggle(
  user: ReturnType<typeof userEvent.setup>,
  passwordInputTestId: string,
  toggleButtonTestId: string,
) {
  const passwordInput = screen.getByTestId(passwordInputTestId);
  expect(passwordInput).toHaveAttribute('type', 'password');

  const toggleButton = screen.getByTestId(toggleButtonTestId);
  await user.click(toggleButton);

  await waitFor(() => {
    expect(passwordInput).toHaveAttribute('type', 'text');
  });

  await user.click(toggleButton);

  await waitFor(() => {
    expect(passwordInput).toHaveAttribute('type', 'password');
  });
}

/**
 * Helper function to create delayed promise for testing loading states
 */
export async function createDelayedPromise<T>(value: T, delay = 100): Promise<T> {
  return new Promise((resolve) => {
    setTimeout(() => resolve(value), delay);
  });
}

/**
 * Common mock for Dialog components to be used in vi.mock
 */
export const MockDialog = {
  Dialog: ({
    children,
    open,
    onOpenChange,
  }: {
    children: React.ReactNode;
    open: boolean;
    onOpenChange: (open: boolean) => void;
  }) => (
    <div data-testid="dialog" data-open={open}>
      {open && (
        <div data-testid="dialog-container">
          <button onClick={() => onOpenChange(false)} data-testid="dialog-overlay">
            Close
          </button>
          {children}
        </div>
      )}
    </div>
  ),
  DialogContent: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div data-testid="dialog-content" className={className}>
      {children}
    </div>
  ),
  DialogDescription: ({
    children,
    ...props
  }: { children: React.ReactNode } & Record<string, unknown>) => (
    <div data-testid="dialog-description" {...props}>
      {children}
    </div>
  ),
  DialogFooter: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div data-testid="dialog-footer" className={className}>
      {children}
    </div>
  ),
  DialogHeader: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div data-testid="dialog-header" className={className}>
      {children}
    </div>
  ),
  DialogTitle: ({
    children,
    ...props
  }: { children: React.ReactNode } & Record<string, unknown>) => (
    <h2 data-testid="dialog-title" {...props}>
      {children}
    </h2>
  ),
};

/**
 * Common mock for Button component to be used in vi.mock
 */
export const MockButton = {
  Button: ({
    children,
    onClick,
    disabled,
    type,
    variant,
    className,
    ...props
  }: {
    children: React.ReactNode;
    onClick?: (e: React.MouseEvent) => void;
    disabled?: boolean;
    type?: 'submit' | 'reset' | 'button';
    variant?: string;
    className?: string;
    [key: string]: unknown;
  }) => (
    <button
      onClick={onClick}
      disabled={disabled}
      type={type}
      data-variant={variant}
      className={className}
      {...props}
    >
      {children}
    </button>
  ),
};
