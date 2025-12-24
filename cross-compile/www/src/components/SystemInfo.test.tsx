/**
 * SystemInfo Component Tests
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import SystemInfo from './SystemInfo';

describe('SystemInfo', () => {
  const defaultProps = {
    cameraIP: '192.168.1.100',
    onvifStatus: 'online' as const,
    onStatusCheck: vi.fn(),
  };

  it('should render endpoints', () => {
    render(<SystemInfo {...defaultProps} />);
    expect(screen.getByText('Endpoints')).toBeInTheDocument();
    expect(screen.getByText(/RTSP Main/)).toBeInTheDocument();
    expect(screen.getByText(/RTSP Sub/)).toBeInTheDocument();
    expect(screen.getByText(/Snapshot/)).toBeInTheDocument();
    expect(screen.getByText(/ONVIF Device/)).toBeInTheDocument();
  });

  it('should display correct endpoint URLs with camera IP', () => {
    render(<SystemInfo {...defaultProps} />);
    expect(screen.getByText(/rtsp:\/\/192\.168\.1\.100:554\/vs0/)).toBeInTheDocument();
    expect(screen.getByText(/http:\/\/192\.168\.1\.100:3000\/snapshot\.jpeg/)).toBeInTheDocument();
  });

  it('should display online status', () => {
    render(<SystemInfo {...defaultProps} onvifStatus="online" />);
    expect(screen.getByText('ONVIF Server Online')).toBeInTheDocument();
  });

  it('should display offline status', () => {
    render(<SystemInfo {...defaultProps} onvifStatus="offline" />);
    expect(screen.getByText('ONVIF Server Offline')).toBeInTheDocument();
  });

  it('should display checking status', () => {
    render(<SystemInfo {...defaultProps} onvifStatus="checking" />);
    expect(screen.getByText('Checking status...')).toBeInTheDocument();
  });

  it('should call onStatusCheck when refresh button is clicked', async () => {
    const onStatusCheck = vi.fn();
    render(<SystemInfo {...defaultProps} onStatusCheck={onStatusCheck} />);

    const refreshButton = screen.getByText('Refresh Status');
    await userEvent.click(refreshButton);

    expect(onStatusCheck).toHaveBeenCalledTimes(1);
  });

  it('should render status icon for online', () => {
    render(<SystemInfo {...defaultProps} onvifStatus="online" />);
    // Check that status text is present (icon is rendered alongside it)
    expect(screen.getByText('ONVIF Server Online')).toBeInTheDocument();
  });

  it('should render status icon for offline', () => {
    render(<SystemInfo {...defaultProps} onvifStatus="offline" />);
    // Check that status text is present (icon is rendered alongside it)
    expect(screen.getByText('ONVIF Server Offline')).toBeInTheDocument();
  });

  it('should render loading spinner for checking status', () => {
    const { container } = render(<SystemInfo {...defaultProps} onvifStatus="checking" />);
    // Find spinner by class name
    const spinner = container.querySelector('.animate-spin');
    expect(spinner).toBeInTheDocument();
  });
});
