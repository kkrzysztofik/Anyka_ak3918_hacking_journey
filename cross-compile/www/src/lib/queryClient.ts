import { QueryClient } from '@tanstack/react-query'

/**
 * TanStack Query client singleton with default caching rules.
 *
 * Configuration:
 * - staleTime: 30s - data considered fresh for 30 seconds
 * - retry: 1 - retry failed requests once
 * - refetchOnWindowFocus: false - disable auto-refetch on window focus (embedded device)
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30 * 1000, // 30 seconds
      retry: 1,
      refetchOnWindowFocus: false, // Disable for embedded device context
    },
    mutations: {
      retry: 0, // Don't retry mutations
    },
  },
})
