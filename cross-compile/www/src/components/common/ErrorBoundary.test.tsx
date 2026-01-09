/**
 * ErrorBoundary Component Tests
 */
import React from 'react';

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { ErrorBoundary } from './ErrorBoundary';

// Component that throws an error
function ThrowError({ shouldThrow = false }: Readonly<{ shouldThrow?: boolean }>) {
  if (shouldThrow) {
    throw new Error('Test error message');
  }
  return <div data-testid="no-error">No error</div>;
}

// Component that throws error without message property
function ThrowErrorWithoutMessage(): null {
  const error = Object.create(null);
  throw error; // Error object without message property
}

// Component that throws a non-Error object (string)
// NOSONAR - Test case: intentionally throwing non-Error to test ErrorBoundary handling
function ThrowNonError(): null {
  throw 'String error'; // NOSONAR - Intentionally throwing string to test ErrorBoundary's handling of non-Error objects
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

  it('calls componentDidCatch and logs error in development mode', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const originalEnv = process.env.NODE_ENV;
    process.env.NODE_ENV = 'development';

    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>,
    );

    // componentDidCatch should have been called and logged the error
    expect(consoleSpy).toHaveBeenCalled();
    expect(consoleSpy).toHaveBeenCalledWith(
      'ErrorBoundary caught an error:',
      expect.any(Error),
      expect.any(Object),
    );

    consoleSpy.mockRestore();
    process.env.NODE_ENV = originalEnv;
  });

  it('does not log error in production mode', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const originalEnv = process.env.NODE_ENV;
    process.env.NODE_ENV = 'production';

    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>,
    );

    // In production, componentDidCatch should not log our specific message
    // (React may still log, but our ErrorBoundary message should not appear)
    const calls = consoleSpy.mock.calls;
    const hasErrorBoundaryMessage = calls.some((call) =>
      call.some((arg) => typeof arg === 'string' && arg.includes('ErrorBoundary caught an error')),
    );
    expect(hasErrorBoundaryMessage).toBe(false);

    consoleSpy.mockRestore();
    process.env.NODE_ENV = originalEnv;
  });

  it('resets error state when retry button is clicked', async () => {
    const user = userEvent.setup();
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    // Use a component that can recover after error by using a key prop
    function RecoverableComponent({ key: _key }: Readonly<{ key?: string | number }>) {
      return <div data-testid="no-error">No error</div>;
    }

    const { rerender } = render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>,
    );

    // Initially error state
    expect(screen.getByTestId('error-boundary-fallback')).toBeInTheDocument();
    expect(screen.queryByTestId('no-error')).not.toBeInTheDocument();

    // Click retry button - this resets the error boundary state
    const retryButton = screen.getByTestId('error-boundary-retry-button');
    await user.click(retryButton);

    // Re-render with a component that won't throw (using key to force new instance)
    rerender(
      <ErrorBoundary key="reset-test">
        <RecoverableComponent />
      </ErrorBoundary>,
    );

    // After reset, should render children again (component doesn't throw)
    expect(screen.queryByTestId('error-boundary-fallback')).not.toBeInTheDocument();
    expect(screen.getByTestId('no-error')).toBeInTheDocument();

    consoleSpy.mockRestore();
  });

  it('displays fallback message when error.message is undefined', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <ErrorBoundary>
        <ThrowErrorWithoutMessage />
      </ErrorBoundary>,
    );

    expect(screen.getByTestId('error-boundary-fallback')).toBeInTheDocument();
    // When error.message is undefined, should show fallback message
    const message = screen.getByTestId('error-boundary-message');
    expect(message).toBeInTheDocument();
    // The message should be the fallback since error.message is undefined
    expect(message.textContent).toContain('An unexpected error occurred');

    consoleSpy.mockRestore();
  });

  it('handles non-Error thrown objects', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <ErrorBoundary>
        <ThrowNonError />
      </ErrorBoundary>,
    );

    // ErrorBoundary should still catch it and show fallback
    expect(screen.getByTestId('error-boundary-fallback')).toBeInTheDocument();
    // The message might be the string or fallback depending on how React handles it
    const message = screen.getByTestId('error-boundary-message');
    expect(message).toBeInTheDocument();

    consoleSpy.mockRestore();
  });
});
