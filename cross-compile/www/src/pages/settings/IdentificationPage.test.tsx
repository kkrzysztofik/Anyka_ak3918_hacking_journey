/**
 * IdentificationPage Tests
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { AuthProvider } from '@/hooks/useAuth';

import IdentificationPage from './IdentificationPage';

// Mock services
vi.mock('@/services/deviceService', () => ({
  getDeviceIdentification: vi.fn(),
  setDeviceInformation: vi.fn(),
}));

vi.mock('@/services/networkService', () => ({
  getNetworkInterfaces: vi.fn(),
}));

// Mock sonner toast
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

describe('IdentificationPage', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    vi.clearAllMocks();
  });

  const renderWithProviders = (component: React.ReactElement) => {
    return render(
      <QueryClientProvider client={queryClient}>
        <AuthProvider>{component}</AuthProvider>
      </QueryClientProvider>,
    );
  };

  it('should render identification form', async () => {
    const { getDeviceIdentification } = await import('@/services/deviceService');
    vi.mocked(getDeviceIdentification).mockResolvedValue({
      deviceInfo: {
        manufacturer: 'Test Manufacturer',
        model: 'Test Model',
        firmwareVersion: '1.0.0',
        serialNumber: 'SN123',
        hardwareId: 'HW456',
      },
      name: 'Test Device',
      location: 'Test Location',
    });

    const { getNetworkInterfaces } = await import('@/services/networkService');
    vi.mocked(getNetworkInterfaces).mockResolvedValue([]);

    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByText(/device identification/i)).toBeInTheDocument();
    });
  });

  it('should display device information when loaded', async () => {
    const { getDeviceIdentification } = await import('@/services/deviceService');
    vi.mocked(getDeviceIdentification).mockResolvedValue({
      deviceInfo: {
        manufacturer: 'Test Manufacturer',
        model: 'Test Model',
        firmwareVersion: '1.0.0',
        serialNumber: 'SN123',
        hardwareId: 'HW456',
      },
      name: 'Test Device',
      location: 'Test Location',
    });

    const { getNetworkInterfaces } = await import('@/services/networkService');
    vi.mocked(getNetworkInterfaces).mockResolvedValue([]);

    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('Test Device')).toBeInTheDocument();
      expect(screen.getByDisplayValue('Test Location')).toBeInTheDocument();
    });
  });

  it('should allow editing device name', async () => {
    const { getDeviceIdentification } = await import('@/services/deviceService');
    vi.mocked(getDeviceIdentification).mockResolvedValue({
      deviceInfo: {
        manufacturer: 'Test Manufacturer',
        model: 'Test Model',
        firmwareVersion: '1.0.0',
        serialNumber: 'SN123',
        hardwareId: 'HW456',
      },
      name: 'Test Device',
      location: 'Test Location',
    });

    const { getNetworkInterfaces } = await import('@/services/networkService');
    vi.mocked(getNetworkInterfaces).mockResolvedValue([]);

    const user = userEvent.setup();
    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('Test Device')).toBeInTheDocument();
    });

    const nameInput = screen.getByDisplayValue('Test Device');
    await user.clear(nameInput);
    await user.type(nameInput, 'Updated Device Name');

    expect(nameInput).toHaveValue('Updated Device Name');
  });

  it('should allow editing device location', async () => {
    const { getDeviceIdentification } = await import('@/services/deviceService');
    vi.mocked(getDeviceIdentification).mockResolvedValue({
      deviceInfo: {
        manufacturer: 'Test Manufacturer',
        model: 'Test Model',
        firmwareVersion: '1.0.0',
        serialNumber: 'SN123',
        hardwareId: 'HW456',
      },
      name: 'Test Device',
      location: 'Test Location',
    });

    const { getNetworkInterfaces } = await import('@/services/networkService');
    vi.mocked(getNetworkInterfaces).mockResolvedValue([]);

    const user = userEvent.setup();
    renderWithProviders(<IdentificationPage />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('Test Location')).toBeInTheDocument();
    });

    const locationInput = screen.getByDisplayValue('Test Location');
    await user.clear(locationInput);
    await user.type(locationInput, 'Updated Location');

    expect(locationInput).toHaveValue('Updated Location');
  });
});
