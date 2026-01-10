/**
 * Error Boundary Component
 *
 * Catches errors in child components and displays a fallback UI.
 * Used to handle lazy-loading errors and component crashes gracefully.
 */
import React, { Component, type ErrorInfo, type ReactNode } from 'react';

import { AlertTriangle, RefreshCw } from 'lucide-react';

import { Button } from '@/components/ui/button';

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
    };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return {
      hasError: true,
      error,
    };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    // Log error for debugging (in development)
    // Note: console.error is allowed in development mode for debugging
    if (process.env.NODE_ENV === 'development') {
      console.error('ErrorBoundary caught an error:', error, errorInfo);
    }
  }

  handleReset = (): void => {
    this.setState({
      hasError: false,
      error: null,
    });
  };

  render(): ReactNode {
    if (this.state.hasError) {
      // Use custom fallback if provided
      if (this.props.fallback) {
        return this.props.fallback;
      }

      // Default error fallback UI
      return (
        <div
          className="flex min-h-screen items-center justify-center bg-[#0d0d0d] p-4"
          data-testid="error-boundary-fallback"
        >
          <div className="bg-dark-sidebar border-dark-border w-full max-w-md rounded-lg border p-6 text-center">
            <div className="mb-4 flex justify-center">
              <div className="bg-accent-red/10 flex size-16 items-center justify-center rounded-full">
                <AlertTriangle className="text-accent-red size-8" />
              </div>
            </div>
            <h2
              className="mb-2 text-xl font-semibold text-white"
              data-testid="error-boundary-title"
            >
              Something went wrong
            </h2>
            <p
              className="text-dark-secondary-text mb-6 text-sm"
              data-testid="error-boundary-message"
            >
              {this.state.error?.message || 'An unexpected error occurred. Please try again.'}
            </p>
            <Button
              onClick={this.handleReset}
              className="bg-accent-red hover:bg-accent-red/90 text-white"
              data-testid="error-boundary-retry-button"
            >
              <RefreshCw className="mr-2 size-4" />
              Try Again
            </Button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
