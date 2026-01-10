/**
 * Error Handling Utilities Tests
 */
import { toast } from 'sonner';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { handleMutationError } from './errorHandling';

// Mock sonner toast
vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
  },
}));

describe('handleMutationError', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('calls toast.error with default message and error description', () => {
    const error = new Error('Network error');
    handleMutationError(error, 'Failed to save');

    expect(toast.error).toHaveBeenCalledWith('Failed to save', {
      description: 'Network error',
    });
  });

  it('handles non-Error objects with generic description', () => {
    const error = 'String error';
    handleMutationError(error, 'Failed to save');

    expect(toast.error).toHaveBeenCalledWith('Failed to save', {
      description: 'An error occurred',
    });
  });

  it('handles null errors', () => {
    handleMutationError(null, 'Failed to save');

    expect(toast.error).toHaveBeenCalledWith('Failed to save', {
      description: 'An error occurred',
    });
  });

  it('handles undefined errors', () => {
    handleMutationError(undefined, 'Failed to save');

    expect(toast.error).toHaveBeenCalledWith('Failed to save', {
      description: 'An error occurred',
    });
  });

  it('handles Error objects with empty message', () => {
    const error = new Error('Unknown error');
    handleMutationError(error, 'Failed to save');

    expect(toast.error).toHaveBeenCalledWith('Failed to save', {
      description: 'Unknown error',
    });
  });

  it('handles Error objects with complex messages', () => {
    const error = new Error('Validation failed: username is required');
    handleMutationError(error, 'Failed to create user');

    expect(toast.error).toHaveBeenCalledWith('Failed to create user', {
      description: 'Validation failed: username is required',
    });
  });
});
