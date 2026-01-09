/**
 * QueryClient Configuration Tests
 */
import { describe, expect, it } from 'vitest';

import { queryClient } from './queryClient';

describe('queryClient', () => {
  it('should create a QueryClient instance', () => {
    expect(queryClient).toBeDefined();
  });

  it('should have default query options configured', () => {
    const defaultOptions = queryClient.getDefaultOptions();
    expect(defaultOptions.queries).toBeDefined();
    expect(defaultOptions.queries?.staleTime).toBe(30 * 1000); // 30 seconds
    expect(defaultOptions.queries?.retry).toBe(1);
    expect(defaultOptions.queries?.refetchOnWindowFocus).toBe(false);
  });

  it('should have default mutation options configured', () => {
    const defaultOptions = queryClient.getDefaultOptions();
    expect(defaultOptions.mutations).toBeDefined();
    expect(defaultOptions.mutations?.retry).toBe(0);
  });
});
