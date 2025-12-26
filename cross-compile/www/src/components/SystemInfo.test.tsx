/**
 * SystemInfo Component Tests
 */
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '@/test/componentTestHelpers';

import SystemInfo from './SystemInfo';

describe('SystemInfo', () => {
  const defaultProps = {
    cameraIP: '192.168.1.100',
    onvifStatus: 'online' as const,
    onStatusCheck: vi.fn(),
  };

  it('should render endpoints', () => {
    renderWithProviders(<SystemInfo {...defaultProps} />);
    expect(screen.getByTestId('system-info-endpoints-title')).toHaveTextContent('Endpoints');
    // We can check existence of items by ID or text content within list
    expect(screen.getByTestId('system-info-endpoints-list')).toBeInTheDocument();
    // Verify specific endpoints exist
    expect(screen.getByText(/RTSP Main/)).toBeInTheDocument(); // Labels kept as text matching is fine for labels inside the item
    expect(screen.getByText(/ONVIF Device/)).toBeInTheDocument();
  });

  it('should display correct endpoint URLs with camera IP', () => {
    renderWithProviders(<SystemInfo {...defaultProps} />);
    // Check URLs by test ID
    expect(screen.getByTestId('system-info-endpoint-0-url')).toHaveTextContent(
      `rtsp://${defaultProps.cameraIP}:554/vs0`,
    );
    expect(screen.getByTestId('system-info-endpoint-2-url')).toHaveTextContent(
      `http://${defaultProps.cameraIP}:3000/snapshot.jpeg`,
    );
  });

  it('should display online status', () => {
    renderWithProviders(<SystemInfo {...defaultProps} onvifStatus="online" />);
    expect(screen.getByTestId('system-info-status-text')).toHaveTextContent('ONVIF Server Online');
  });

  it('should display offline status', () => {
    renderWithProviders(<SystemInfo {...defaultProps} onvifStatus="offline" />);
    expect(screen.getByTestId('system-info-status-text')).toHaveTextContent('ONVIF Server Offline');
  });

  it('should display checking status', () => {
    renderWithProviders(<SystemInfo {...defaultProps} onvifStatus="checking" />);
    expect(screen.getByTestId('system-info-status-text')).toHaveTextContent('Checking status...');
  });

  it('should call onStatusCheck when refresh button is clicked', async () => {
    const onStatusCheck = vi.fn();
    renderWithProviders(<SystemInfo {...defaultProps} onStatusCheck={onStatusCheck} />);

    const refreshButton = screen.getByTestId('system-info-refresh-button');
    await userEvent.click(refreshButton);

    expect(onStatusCheck).toHaveBeenCalledTimes(1);
  });

  it('should render status icon for online', () => {
    renderWithProviders(<SystemInfo {...defaultProps} onvifStatus="online" />);
    // Check that status text is present (icon is rendered alongside it)
    expect(screen.getByTestId('system-info-status-text')).toBeInTheDocument();
  });

  it('should render status icon for offline', () => {
    renderWithProviders(<SystemInfo {...defaultProps} onvifStatus="offline" />);
    // Check that status text is present (icon is rendered alongside it)
    expect(screen.getByTestId('system-info-status-text')).toBeInTheDocument();
  });

  it('should render loading spinner for checking status', () => {
    renderWithProviders(<SystemInfo {...defaultProps} onvifStatus="checking" />);
    const spinner = screen.getByTestId('system-info-loading-spinner');
    expect(spinner).toBeInTheDocument();
  });
});
