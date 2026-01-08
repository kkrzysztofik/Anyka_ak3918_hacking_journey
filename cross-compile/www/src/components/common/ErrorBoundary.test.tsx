/**
 * ErrorBoundary Component Tests
 */
import React from 'react';

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { ErrorBoundary } from './ErrorBoundary';

// Component that throws an error
function ThrowError({ shouldThrow = false }: Readonly<{ shouldThrow?: boolean }>) {
  if (shouldThrow) {
    throw new Error('Test error message');
  }
  return <div data-testid="no-error">No error</div>;
}

describe('ErrorBoundary', () => {
  it('renders children when there is no error', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={false} />
      </ErrorBoundary>,
    );

    expect(screen.getByTestId('no-error')).toBeInTheDocument();
  });

  it('renders error fallback when child throws error', () => {
    // Suppress console.error for this test
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>,
    );

    expect(screen.getByTestId('error-boundary-fallback')).toBeInTheDocument();
    expect(screen.getByTestId('error-boundary-title')).toHaveTextContent('Something went wrong');
    expect(screen.getByTestId('error-boundary-message')).toHaveTextContent('Test error message');
    expect(screen.getByTestId('error-boundary-retry-button')).toBeInTheDocument();

    consoleSpy.mockRestore();
  });

  it('renders custom fallback when provided', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const customFallback = <div data-testid="custom-fallback">Custom error UI</div>;

    render(
      <ErrorBoundary fallback={customFallback}>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>,
    );

    expect(screen.getByTestId('custom-fallback')).toBeInTheDocument();
    expect(screen.queryByTestId('error-boundary-fallback')).not.toBeInTheDocument();

    consoleSpy.mockRestore();
  });

  it('displays retry button in error state', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>,
    );

    expect(screen.getByTestId('error-boundary-fallback')).toBeInTheDocument();
    expect(screen.getByTestId('error-boundary-retry-button')).toBeInTheDocument();
    expect(screen.getByTestId('error-boundary-retry-button')).toHaveTextContent('Try Again');

    consoleSpy.mockRestore();
  });
});
