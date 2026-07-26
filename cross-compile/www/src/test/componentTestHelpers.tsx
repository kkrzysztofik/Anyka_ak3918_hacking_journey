/**
 * Shared test utilities for React component tests
 * Provides common wrappers, QueryClient setup, and mock factories
 */
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
import { toast } from 'sonner';
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
        address: '192.168.1.100', // NOSONAR - Test data: Hardcoded IP for testing network configuration
        prefixLength: 24,
        gateway: '192.168.1.1', // NOSONAR - Test data: Hardcoded gateway IP for testing
        hwAddress: '00:11:22:33:44:55',
      },
    ],
    dns: {
      fromDHCP: false,
      dnsServers: ['8.8.8.8', '8.8.4.4'], // NOSONAR - Test data: Google DNS IPs for testing DNS configuration
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
 * Wait for a page to load by checking for a specific test ID
 * @param testId - Test ID of the page title or main element
 */
export async function waitForPageLoad(testId: string): Promise<void> {
  await waitFor(() => {
    expect(screen.getByTestId(testId)).toBeInTheDocument();
  });
}

/**
 * Render a page component and wait for it to load
 * @param component - React component to render
 * @param testId - Test ID to wait for
 * @param options - Optional render options
 */
export async function renderPageAndWait(
  component: ReactElement,
  testId: string,
  options?: RenderWithProvidersOptions,
) {
  const result = renderWithProviders(component, options);
  await waitForPageLoad(testId);
  return result;
}

/**
 * Open a dialog by clicking the trigger and verify it opens
 * @param user - User event instance
 * @param triggerTestId - Test ID of the button/trigger that opens the dialog
 * @param dialogTestId - Test ID of the dialog to verify it opened
 */
export async function openDialog(
  user: ReturnType<typeof userEvent.setup>,
  triggerTestId: string,
  dialogTestId: string,
): Promise<void> {
  const trigger = screen.getByTestId(triggerTestId);
  await user.click(trigger);

  await waitFor(() => {
    expect(screen.getByTestId(dialogTestId)).toBeInTheDocument();
  });
}

/**
 * Close a dialog by clicking the cancel/close button
 * @param user - User event instance
 * @param cancelTestId - Test ID of the cancel/close button
 * @param dialogTestId - Test ID of the dialog to verify it closed
 */
export async function closeDialog(
  user: ReturnType<typeof userEvent.setup>,
  cancelTestId: string,
  dialogTestId: string,
): Promise<void> {
  const cancelButton = screen.getByTestId(cancelTestId);
  await user.click(cancelButton);

  await waitFor(() => {
    expect(screen.queryByTestId(dialogTestId)).not.toBeInTheDocument();
  });
}

/**
 * Submit a dialog form and verify the expected function was called
 * @param user - User event instance
 * @param submitTestId - Test ID of the submit button
 * @param expectedCall - Mock function that should be called
 */
export async function submitDialog(
  user: ReturnType<typeof userEvent.setup>,
  submitTestId: string,
  expectedCall?: ReturnType<typeof import('vitest').vi.fn>,
): Promise<void> {
  const submitButton = screen.getByTestId(submitTestId);
  await user.click(submitButton);

  if (expectedCall) {
    await waitFor(() => {
      expect(expectedCall).toHaveBeenCalled();
    });
  }
}

/**
 * Fill a form field with a value
 * @param user - User event instance
 * @param testId - Test ID of the input field
 * @param value - Value to fill
 */
export async function fillFormField(
  user: ReturnType<typeof userEvent.setup>,
  testId: string,
  value: string,
): Promise<void> {
  const input = screen.getByTestId(testId);
  await user.clear(input);
  await user.type(input, value);
}

/**
 * Toggle a switch or checkbox
 * @param user - User event instance
 * @param testId - Test ID of the switch/checkbox
 */
export async function toggleSwitch(
  user: ReturnType<typeof userEvent.setup>,
  testId: string,
): Promise<void> {
  const switchElement = screen.getByTestId(testId);
  await user.click(switchElement);
}

/**
 * Select an option from a dropdown/select
 * @param user - User event instance
 * @param selectTestId - Test ID of the select element
 * @param optionValue - Value of the option to select
 */
export async function selectOption(
  user: ReturnType<typeof userEvent.setup>,
  selectTestId: string,
  optionValue: string,
): Promise<void> {
  const select = screen.getByTestId(selectTestId);
  await user.selectOptions(select, optionValue);
}

/**
 * Find a profile by name from the mock profiles array
 * @param profiles - Array of media profiles
 * @param name - Name of the profile to find
 * @returns The found profile or undefined
 */
export function findProfileByName(
  profiles: MediaProfile[],
  name: string,
): MediaProfile | undefined {
  return profiles.find((p) => p.name === name);
}

/**
 * Expand a profile card by clicking the expand button
 * @param user - User event instance
 * @param profileToken - Token of the profile to expand
 */
export async function expandProfile(
  user: ReturnType<typeof userEvent.setup>,
  profileToken: string,
): Promise<void> {
  const expandButton = screen.getByTestId(`profile-expand-${profileToken}`);
  await user.click(expandButton);

  await waitFor(
    () => {
      expect(screen.getByTestId(`video-source-config-${profileToken}`)).toBeInTheDocument();
    },
    { timeout: 3000 },
  );
}

/**
 * Wait for the video encoder section to appear in an expanded profile
 */
export async function waitForVideoEncoderSection(profileToken = 'ProfileToken1'): Promise<void> {
  await waitFor(
    () => {
      expect(screen.getByTestId(`video-encoder-config-${profileToken}`)).toBeInTheDocument();
    },
    { timeout: 3000 },
  );
}

/**
 * Setup device information mock for AboutDialog tests
 * @param mockData - Device information data to mock
 */
export async function setupDeviceInfoMock(mockData: {
  manufacturer: string;
  model: string;
  firmwareVersion: string;
  serialNumber: string;
  hardwareId: string;
}): Promise<void> {
  const { getDeviceInformation } = await import('@/services/deviceService');
  const { vi } = await import('vitest');
  vi.mocked(getDeviceInformation).mockResolvedValue(mockData);
}

/**
 * Make a form dirty by changing a field value, typically used before testing form submission
 * @param user - User event instance
 * @param fieldTestId - Test ID of the field to change
 * @param newValue - New value to set
 */
export async function makeFormDirty(
  user: ReturnType<typeof userEvent.setup>,
  fieldTestId: string,
  newValue: string,
): Promise<void> {
  const field = screen.getByTestId(fieldTestId);
  await user.clear(field);
  await user.type(field, newValue);
}

/**
 * Helper to interact with user menu (open menu, perform action)
 * @param user - User event instance
 * @param actionTestId - Test ID of the menu action button
 */
export async function performUserMenuAction(
  user: ReturnType<typeof userEvent.setup>,
  actionTestId: string,
): Promise<void> {
  const userButtons = screen.getAllByTestId('layout-user-menu-button');
  const userButton = userButtons[0]; // Use desktop version
  await user.click(userButton);

  await waitFor(() => {
    expect(screen.getByTestId(actionTestId)).toBeInTheDocument();
  });

  const actionButton = screen.getByTestId(actionTestId);
  await user.click(actionButton);
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
 * Helper function to create a promise that can be resolved/rejected manually.
 * Useful for testing loading states where you need precise control over when the promise completes.
 */
export function createControllablePromise<T>() {
  let resolve: (value: T | PromiseLike<T>) => void;
  let reject: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve: resolve!, reject: reject! };
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
  DialogContent: ({
    children,
    className,
    ...props
  }: {
    children: React.ReactNode;
    className?: string;
    [key: string]: unknown;
  }) => (
    <div data-testid="dialog-content" className={className} {...props}>
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
  DialogFooter: ({
    children,
    className,
    ...props
  }: {
    children: React.ReactNode;
    className?: string;
    [key: string]: unknown;
  }) => (
    <div data-testid="dialog-footer" className={className} {...props}>
      {children}
    </div>
  ),
  DialogHeader: ({
    children,
    className,
    ...props
  }: {
    children: React.ReactNode;
    className?: string;
    [key: string]: unknown;
  }) => (
    <div data-testid="dialog-header" className={className} {...props}>
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
    size,
    asChild: _asChild,
    className,
    ...props
  }: {
    children: React.ReactNode;
    onClick?: (e: React.MouseEvent) => void;
    disabled?: boolean;
    type?: 'submit' | 'reset' | 'button';
    variant?: string;
    size?: string;
    asChild?: boolean;
    className?: string;
    [key: string]: unknown;
  }) => (
    <button
      onClick={onClick}
      disabled={disabled}
      type={type}
      data-variant={variant}
      data-size={size}
      className={className}
      {...props}
    >
      {children}
    </button>
  ),
  buttonVariants: () => 'mock-button-variant',
};

/**
 * Create an encrypted data fixture with unique values for testing.
 * Generates unique IV and data strings to prevent cross-test contamination
 * in sessionStorage-based auth tests.
 */
export function createEncryptedFixture() {
  const unique = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  return {
    iv: btoa(`iv-${unique}`),
    data: btoa(`data-${unique}`),
    method: 'aes-gcm' as const,
  };
}

/**
 * Generic error toast verification helper
 * @param message - Expected error message
 * @param description - Optional error description
 */
export function testErrorToast(message: string, description?: string): void {
  if (description) {
    expect(toast.error).toHaveBeenCalledWith(message, { description });
  } else {
    expect(toast.error).toHaveBeenCalledWith(message);
  }
}

/**
 * Generic success toast verification helper
 * @param message - Expected success message
 * @param description - Optional success description
 */
export function testSuccessToast(message: string, description?: string): void {
  if (description) {
    expect(toast.success).toHaveBeenCalledWith(message, { description });
  } else {
    expect(toast.success).toHaveBeenCalledWith(message);
  }
}
