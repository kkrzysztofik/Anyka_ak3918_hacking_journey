/**
 * Shared test utilities for React component tests
 * Provides common wrappers, QueryClient setup, and mock factories
 */
/* eslint-disable react-refresh/only-export-components */
import type { ReactElement, ReactNode } from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { type RenderOptions, type RenderResult, render } from '@testing-library/react';

import { AuthProvider } from '@/hooks/useAuth';

/**
 * Create a QueryClient configured for testing
 * Disables retries for predictable test behavior
 */
export function createTestQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
}

interface WrapperProps {
  children: ReactNode;
}

interface RenderWithProvidersOptions extends Omit<RenderOptions, 'wrapper'> {
  queryClient?: QueryClient;
}

/**
 * Create a wrapper with all required providers for testing
 */
function createWrapper(queryClient: QueryClient) {
  return function Wrapper({ children }: WrapperProps) {
    return (
      <QueryClientProvider client={queryClient}>
        <AuthProvider>{children}</AuthProvider>
      </QueryClientProvider>
    );
  };
}

/**
 * Render a component with all required providers
 * Use this instead of RTL's render for page/component tests
 *
 * @example
 * ```tsx
 * const { getByTestId } = renderWithProviders(<MyPage />);
 * ```
 */
export function renderWithProviders(
  ui: ReactElement,
  { queryClient = createTestQueryClient(), ...options }: RenderWithProvidersOptions = {},
): RenderResult & { queryClient: QueryClient } {
  const result = render(ui, {
    wrapper: createWrapper(queryClient),
    ...options,
  });

  return {
    ...result,
    queryClient,
  };
}

export { toast as mockToast } from 'sonner';

/**
 * API endpoints constants for mock setup
 */
export const MOCK_ENDPOINTS = {
  device: '/onvif/device_service',
  media: '/onvif/media_service',
  ptz: '/onvif/ptz_service',
  imaging: '/onvif/imaging_service',
} as const;
