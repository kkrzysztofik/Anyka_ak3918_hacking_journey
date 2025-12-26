/**
 * AboutDialog Component Tests
 */
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { mockToast, renderWithProviders } from '@/test/componentTestHelpers';

import { AboutDialog } from './AboutDialog';

// Mock deviceService
vi.mock('@/services/deviceService', () => ({
  getDeviceInformation: vi.fn(),
}));

// Mock UI components
vi.mock('@/components/ui/dialog', () => ({
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
        <div>
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
  }: React.ComponentPropsWithoutRef<'div'> & { 'data-testid'?: string }) => (
    <div data-testid={props['data-testid'] || 'dialog-content'} className={className}>
      {children}
    </div>
  ),
  DialogDescription: ({
    children,
    ...props
  }: React.ComponentPropsWithoutRef<'div'> & { 'data-testid'?: string }) => (
    <div data-testid={props['data-testid'] || 'dialog-description'}>{children}</div>
  ),
  DialogFooter: ({
    children,
    ...props
  }: React.ComponentPropsWithoutRef<'div'> & { 'data-testid'?: string }) => (
    <div data-testid={props['data-testid'] || 'dialog-footer'}>{children}</div>
  ),
  DialogHeader: ({
    children,
    ...props
  }: React.ComponentPropsWithoutRef<'div'> & { 'data-testid'?: string }) => (
    <div data-testid={props['data-testid'] || 'dialog-header'}>{children}</div>
  ),
  DialogTitle: ({
    children,
    ...props
  }: React.ComponentPropsWithoutRef<'h2'> & { 'data-testid'?: string }) => (
    <h2 data-testid={props['data-testid'] || 'dialog-title'}>{children}</h2>
  ),
}));

vi.mock('@/components/ui/button', () => ({
  Button: ({
    children,
    onClick,
    className,
  }: {
    children: React.ReactNode;
    onClick?: () => void;
    className?: string;
  }) => (
    <button onClick={onClick} className={className} data-testid="dialog-close-button">
      {children}
    </button>
  ),
}));

describe('AboutDialog', () => {
  const mockDeviceInfo = {
    manufacturer: 'Anyka',
    model: 'AK3918E',
    firmwareVersion: '1.0.0',
    serialNumber: 'ABC123',
    hardwareId: 'HW001',
  };

  beforeEach(async () => {
    vi.clearAllMocks();
    const { getDeviceInformation } = await import('@/services/deviceService');
    vi.mocked(getDeviceInformation).mockResolvedValue(mockDeviceInfo);
  });

  describe('Dialog Open/Close', () => {
    it('should render dialog when open is true', () => {
      renderWithProviders(<AboutDialog open={true} onOpenChange={vi.fn()} />);
      expect(screen.getByTestId('dialog')).toHaveAttribute('data-open', 'true');
    });

    it('should not render dialog content when open is false', () => {
      renderWithProviders(<AboutDialog open={false} onOpenChange={vi.fn()} />);
      expect(screen.getByTestId('dialog')).toHaveAttribute('data-open', 'false');
    });

    it('should call onOpenChange when close button is clicked', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();

      renderWithProviders(<AboutDialog open={true} onOpenChange={onOpenChange} />);

      const closeButton = screen.getByTestId('dialog-close-button');
      await act(async () => {
        await user.click(closeButton);
      });

      expect(onOpenChange).toHaveBeenCalledWith(false);
    });

    it('should call onOpenChange when overlay is clicked', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();

      renderWithProviders(<AboutDialog open={true} onOpenChange={onOpenChange} />);

      const overlay = screen.getByTestId('dialog-overlay');
      await act(async () => {
        await user.click(overlay);
      });

      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });

  describe('Loading State', () => {
    it('should show loading state when fetching device info', async () => {
      // Create a promise that we can control
      let resolvePromise: (value: typeof mockDeviceInfo) => void;
      const promise = new Promise<typeof mockDeviceInfo>((resolve) => {
        resolvePromise = resolve;
      });

      const { getDeviceInformation } = await import('@/services/deviceService');
      vi.mocked(getDeviceInformation).mockReturnValue(promise);

      renderWithProviders(<AboutDialog open={true} onOpenChange={vi.fn()} />);

      // Should show loading
      await waitFor(() => {
        expect(screen.getByTestId('about-loading')).toBeInTheDocument();
      });

      // Resolve the promise
      resolvePromise!(mockDeviceInfo);
      await promise;
    });
  });

  describe('Successful Device Info Fetch', () => {
    it('should fetch and display device information', async () => {
      const { getDeviceInformation } = await import('@/services/deviceService');
      vi.mocked(getDeviceInformation).mockResolvedValue(mockDeviceInfo);

      renderWithProviders(<AboutDialog open={true} onOpenChange={vi.fn()} />);

      await waitFor(() => {
        expect(screen.getByTestId('about-manufacturer-value')).toHaveTextContent('Anyka');
        expect(screen.getByTestId('about-model-value')).toHaveTextContent('AK3918E');
        expect(screen.getByTestId('about-firmware-value')).toHaveTextContent('1.0.0');
        expect(screen.getByTestId('about-serial-value')).toHaveTextContent('ABC123');
        expect(screen.getByTestId('about-hardware-id-value')).toHaveTextContent('HW001');
      });
    });

    it('should display all device info fields', async () => {
      const { getDeviceInformation } = await import('@/services/deviceService');
      vi.mocked(getDeviceInformation).mockResolvedValue(mockDeviceInfo);

      renderWithProviders(<AboutDialog open={true} onOpenChange={vi.fn()} />);

      await waitFor(() => {
        expect(screen.getByText('Manufacturer')).toBeInTheDocument();
        expect(screen.getByText('Model')).toBeInTheDocument();
        expect(screen.getByText('Firmware Version')).toBeInTheDocument();
        expect(screen.getByText('Serial Number')).toBeInTheDocument();
        expect(screen.getByText('Hardware ID')).toBeInTheDocument();
      });
    });

    it('should display dialog title and description', async () => {
      const { getDeviceInformation } = await import('@/services/deviceService');
      vi.mocked(getDeviceInformation).mockResolvedValue(mockDeviceInfo);

      renderWithProviders(<AboutDialog open={true} onOpenChange={vi.fn()} />);

      await waitFor(() => {
        expect(screen.getByTestId('about-dialog-title')).toBeInTheDocument();
        expect(screen.getByTestId('about-dialog-description')).toBeInTheDocument();
      });
    });

    it('should display GitHub link', async () => {
      const { getDeviceInformation } = await import('@/services/deviceService');
      vi.mocked(getDeviceInformation).mockResolvedValue(mockDeviceInfo);

      renderWithProviders(<AboutDialog open={true} onOpenChange={vi.fn()} />);

      await waitFor(() => {
        const githubLink = screen.getByTestId('about-github-link');
        expect(githubLink).toBeInTheDocument();
        expect(githubLink.closest('a')).toHaveAttribute(
          'href',
          'https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey',
        );
        expect(githubLink.closest('a')).toHaveAttribute('target', '_blank');
      });
    });

    it('should display license information', async () => {
      const { getDeviceInformation } = await import('@/services/deviceService');
      vi.mocked(getDeviceInformation).mockResolvedValue(mockDeviceInfo);

      renderWithProviders(<AboutDialog open={true} onOpenChange={vi.fn()} />);

      await waitFor(() => {
        const licenseLink = screen.getByTestId('about-license-link');
        expect(licenseLink).toBeInTheDocument();
        expect(licenseLink.closest('a')).toHaveAttribute(
          'href',
          'https://www.gnu.org/licenses/gpl-3.0.html',
        );
      });
    });

    it('should display copyright notice', async () => {
      const { getDeviceInformation } = await import('@/services/deviceService');
      vi.mocked(getDeviceInformation).mockResolvedValue(mockDeviceInfo);

      renderWithProviders(<AboutDialog open={true} onOpenChange={vi.fn()} />);

      await waitFor(() => {
        expect(screen.getByTestId('about-copyright')).toBeInTheDocument();
      });
    });
  });

  describe('Error Handling', () => {
    it('should show error toast when device info fetch fails', async () => {
      const error = new Error('Network error');
      const { getDeviceInformation } = await import('@/services/deviceService');
      vi.mocked(getDeviceInformation).mockRejectedValue(error);
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      renderWithProviders(<AboutDialog open={true} onOpenChange={vi.fn()} />);

      await waitFor(() => {
        expect(mockToast.error).toHaveBeenCalledWith('Failed to load device information');
        expect(consoleSpy).toHaveBeenCalledWith('Failed to fetch device info:', error);
      });

      consoleSpy.mockRestore();
    });

    it('should show error message when device info is not available', async () => {
      const { getDeviceInformation } = await import('@/services/deviceService');
      vi.mocked(getDeviceInformation).mockResolvedValue(mockDeviceInfo);

      const { rerender } = renderWithProviders(<AboutDialog open={false} onOpenChange={vi.fn()} />);

      // Open dialog
      rerender(<AboutDialog open={true} onOpenChange={vi.fn()} />);

      await waitFor(() => {
        expect(screen.getByTestId('about-manufacturer-value')).toHaveTextContent('Anyka');
      });

      // Close and reopen - should not fetch again if deviceInfo exists
      rerender(<AboutDialog open={false} onOpenChange={vi.fn()} />);
      rerender(<AboutDialog open={true} onOpenChange={vi.fn()} />);

      // Should still show the device info (not fetch again)
      const deviceService = await import('@/services/deviceService');
      expect(deviceService.getDeviceInformation).toHaveBeenCalledTimes(1);
    });
  });

  describe('Data Fetching Logic', () => {
    it('should only fetch device info when dialog opens and deviceInfo is null', async () => {
      const { getDeviceInformation } = await import('@/services/deviceService');
      vi.mocked(getDeviceInformation).mockResolvedValue(mockDeviceInfo);

      const { rerender } = renderWithProviders(<AboutDialog open={false} onOpenChange={vi.fn()} />);

      // Should not fetch when closed
      const deviceService = await import('@/services/deviceService');
      expect(deviceService.getDeviceInformation).not.toHaveBeenCalled();

      // Open dialog
      rerender(<AboutDialog open={true} onOpenChange={vi.fn()} />);

      await waitFor(() => {
        expect(deviceService.getDeviceInformation).toHaveBeenCalledTimes(1);
      });

      // Close and reopen - should not fetch again
      rerender(<AboutDialog open={false} onOpenChange={vi.fn()} />);
      rerender(<AboutDialog open={true} onOpenChange={vi.fn()} />);

      // Should still be called only once
      expect(deviceService.getDeviceInformation).toHaveBeenCalledTimes(1);
    });
  });
});
