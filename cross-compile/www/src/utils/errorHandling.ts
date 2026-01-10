/**
 * Error Handling Utilities
 *
 * Standardized error handling patterns for mutations and API calls.
 */
import { toast } from 'sonner';

/**
 * Handle mutation errors with consistent toast notification pattern
 *
 * @param error - The error object (unknown type for flexibility)
 * @param defaultMessage - Default error message to display
 */
export function handleMutationError(error: unknown, defaultMessage: string): void {
  toast.error(defaultMessage, {
    description: error instanceof Error ? error.message : 'An error occurred',
  });
}
