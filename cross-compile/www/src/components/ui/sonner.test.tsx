/**
 * Sonner Toaster Component Tests
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { Toaster } from './sonner';

// Mock sonner
vi.mock('sonner', () => ({
  Toaster: ({
    theme,
    className,
    toastOptions: _toastOptions,
    ...props
  }: {
    theme?: string;
    className?: string;
    toastOptions?: unknown;
    'data-testid'?: string;
  } & Record<string, unknown>) => (
    <div
      data-testid={props['data-testid'] || 'sonner-toaster'}
      data-theme={theme}
      className={className}
      {...props}
    />
  ),
}));

describe('Toaster', () => {
  it('should render toaster with dark theme', () => {
    render(<Toaster />);
    const toaster = screen.getByTestId('sonner-toaster');
    expect(toaster).toBeInTheDocument();
    expect(toaster).toHaveAttribute('data-theme', 'dark');
  });

  it('should apply custom className', () => {
    render(<Toaster className="custom-toaster" />);
    const toaster = screen.getByTestId('sonner-toaster');
    expect(toaster).toHaveClass('custom-toaster');
  });
});
