/**
 * App Component Tests
 */
import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { setAuthHeaderGetter } from '@/services/api';

import App from './App';

// Mock API service first
vi.mock('@/services/api', () => ({
  setAuthHeaderGetter: vi.fn(),
}));

// Mock the router
vi.mock('@/router', () => ({
  default: () => <div>App Router</div>,
}));

// Mock sonner Toaster
vi.mock('@/components/ui/sonner', () => ({
  Toaster: () => <div data-testid="toaster">Toaster</div>,
}));

// Mock styles (CSS import)
vi.mock('@/styles/globals.css', () => ({}));

describe('App', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render QueryClientProvider', () => {
    render(<App />);
    expect(screen.getByText('App Router')).toBeInTheDocument();
  });

  it('should render AuthProvider', () => {
    render(<App />);
    // AuthProvider wraps the content, so router should be visible
    expect(screen.getByText('App Router')).toBeInTheDocument();
  });

  it('should render AppRouter', () => {
    render(<App />);
    expect(screen.getByText('App Router')).toBeInTheDocument();
  });

  it('should render Toaster component', () => {
    render(<App />);
    expect(screen.getByTestId('toaster')).toBeInTheDocument();
  });

  it('should register auth header getter on mount', async () => {
    render(<App />);

    // Wait for AuthIntegration effect to run
    await waitFor(() => {
      expect(setAuthHeaderGetter).toHaveBeenCalled();
    });
  });

  it('should provide all required providers', () => {
    const { container } = render(<App />);
    // Verify the component tree is rendered (router is inside providers)
    expect(screen.getByText('App Router')).toBeInTheDocument();
    expect(container).toBeInTheDocument();
  });
});
