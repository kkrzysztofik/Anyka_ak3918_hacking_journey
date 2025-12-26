/**
 * ConnectionStatus Component Tests
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { ConnectionStatus, ConnectionStatusBadge } from './ConnectionStatus';

describe('ConnectionStatus', () => {
  describe('Connected State', () => {
    it('should render connected status', () => {
      render(<ConnectionStatus status="connected" />);
      expect(screen.getByTestId('connection-status-connected')).toHaveTextContent('Connected');
    });

    it('should have green color for connected state', () => {
      render(<ConnectionStatus status="connected" />);
      const status = screen.getByTestId('connection-status-connected');
      expect(status.querySelector('.text-green-500')).toBeInTheDocument();
    });
  });

  describe('Disconnected State', () => {
    it('should render disconnected status', () => {
      render(<ConnectionStatus status="disconnected" />);
      expect(screen.getByTestId('connection-status-disconnected')).toHaveTextContent(
        'Disconnected',
      );
    });

    it('should have destructive color for disconnected state', () => {
      render(<ConnectionStatus status="disconnected" />);
      const status = screen.getByTestId('connection-status-disconnected');
      expect(status.querySelector('.text-destructive')).toBeInTheDocument();
    });
  });

  describe('Checking State', () => {
    it('should render checking status', () => {
      render(<ConnectionStatus status="checking" />);
      expect(screen.getByTestId('connection-status-checking')).toHaveTextContent('Checking...');
    });

    it('should have muted color for checking state', () => {
      render(<ConnectionStatus status="checking" />);
      const status = screen.getByTestId('connection-status-checking');
      expect(status.querySelector('.text-muted-foreground')).toBeInTheDocument();
    });
  });

  describe('Accessibility', () => {
    it('should have aria-live attribute', () => {
      render(<ConnectionStatus status="connected" />);
      const output = screen.getByTestId('connection-status-connected');
      expect(output).toHaveAttribute('aria-live', 'polite');
    });

    it('should have aria-hidden on icons', () => {
      render(<ConnectionStatus status="connected" />);
      const output = screen.getByTestId('connection-status-connected');
      const icon = output.querySelector('svg');
      expect(icon).toHaveAttribute('aria-hidden', 'true');
    });
  });

  describe('Custom ClassName', () => {
    it('should apply custom className', () => {
      render(<ConnectionStatus status="connected" className="custom-class" />);
      const output = screen.getByTestId('connection-status-connected');
      expect(output).toHaveClass('custom-class');
    });
  });
});

describe('ConnectionStatusBadge', () => {
  describe('Connected State', () => {
    it('should render connected badge', () => {
      render(<ConnectionStatusBadge status="connected" />);
      const badge = screen.getByTestId('connection-status-badge-connected');
      expect(badge).toBeInTheDocument();
    });

    it('should have green styling for connected state', () => {
      render(<ConnectionStatusBadge status="connected" />);
      const badge = screen.getByTestId('connection-status-badge-connected');
      expect(badge).toHaveClass('bg-green-500/10', 'text-green-500');
    });

    it('should have correct screen reader text', () => {
      render(<ConnectionStatusBadge status="connected" />);
      const srText = screen.getByText('Connected to device');
      expect(srText).toHaveClass('sr-only');
    });
  });

  describe('Disconnected State', () => {
    it('should render disconnected badge', () => {
      render(<ConnectionStatusBadge status="disconnected" />);
      const badge = screen.getByTestId('connection-status-badge-disconnected');
      expect(badge).toBeInTheDocument();
    });

    it('should have destructive styling for disconnected state', () => {
      render(<ConnectionStatusBadge status="disconnected" />);
      const badge = screen.getByTestId('connection-status-badge-disconnected');
      expect(badge).toHaveClass('bg-destructive/10', 'text-destructive');
    });

    it('should have correct screen reader text', () => {
      render(<ConnectionStatusBadge status="disconnected" />);
      const srText = screen.getByText('Disconnected from device');
      expect(srText).toHaveClass('sr-only');
    });
  });

  describe('Checking State', () => {
    it('should render checking badge', () => {
      render(<ConnectionStatusBadge status="checking" />);
      const badge = screen.getByTestId('connection-status-badge-checking');
      expect(badge).toBeInTheDocument();
    });

    it('should have muted styling for checking state', () => {
      render(<ConnectionStatusBadge status="checking" />);
      const badge = screen.getByTestId('connection-status-badge-checking');
      expect(badge).toHaveClass('bg-muted', 'text-muted-foreground');
    });

    it('should have correct screen reader text', () => {
      render(<ConnectionStatusBadge status="checking" />);
      const srText = screen.getByText('Checking connection status');
      expect(srText).toHaveClass('sr-only');
    });
  });

  describe('Accessibility', () => {
    it('should have aria-live attribute', () => {
      render(<ConnectionStatusBadge status="connected" />);
      const badge = screen.getByTestId('connection-status-badge-connected');
      expect(badge).toHaveAttribute('aria-live', 'polite');
    });

    it('should have aria-hidden on icons', () => {
      render(<ConnectionStatusBadge status="connected" />);
      const badge = screen.getByTestId('connection-status-badge-connected');
      const icon = badge.querySelector('svg');
      expect(icon).toHaveAttribute('aria-hidden', 'true');
    });
  });

  describe('Custom ClassName', () => {
    it('should apply custom className', () => {
      render(<ConnectionStatusBadge status="connected" className="custom-class" />);
      const badge = screen.getByTestId('connection-status-badge-connected');
      expect(badge).toHaveClass('custom-class');
    });
  });
});
