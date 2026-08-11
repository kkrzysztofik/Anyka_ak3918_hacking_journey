/**
 * Mutation testing helpers
 * Provides reusable functions for testing mutations with success/error/loading states
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect } from 'vitest';

import { mockToast } from './componentTestHelpers';

// Type for mutation mocks - accepts both vitest mocks and regular functions
type MutationMock =
  | ReturnType<typeof import('vitest').vi.fn>
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  | ((...args: any[]) => Promise<void> | void);

/**
 * Test mutation with success toast
 * @param user - User event instance
 * @param triggerButtonTestId - Test ID of the button that triggers the mutation
 * @param mutationMock - Mock function for the mutation
 * @param successMessage - Expected success toast message
 * @param expectedArgs - Optional expected arguments for the mutation call
 * @param timeout - Timeout in milliseconds (default: 5000)
 */
export async function testMutationWithSuccessToast(
  user: ReturnType<typeof userEvent.setup>,
  triggerButtonTestId: string,
  mutationMock: MutationMock,
  successMessage: string,
  expectedArgs?: unknown[],
  timeout = 5000,
): Promise<void> {
  const triggerButton = screen.getByTestId(triggerButtonTestId);
  await user.click(triggerButton);

  await waitFor(
    () => {
      if (expectedArgs) {
        expect(mutationMock).toHaveBeenCalledWith(...expectedArgs);
      } else {
        expect(mutationMock).toHaveBeenCalled();
      }
      expect(mockToast.success).toHaveBeenCalledWith(successMessage);
    },
    { timeout },
  );
}

/**
 * Test mutation with success toast and optional description
 * @param user - User event instance
 * @param triggerButtonTestId - Test ID of the button that triggers the mutation
 * @param mutationMock - Mock function for the mutation
 * @param successMessage - Expected success toast message
 * @param successDescription - Optional success toast description
 * @param expectedArgs - Optional expected arguments for the mutation call
 * @param timeout - Timeout in milliseconds (default: 5000)
 */
export async function testMutationWithSuccessToastAndDescription(
  user: ReturnType<typeof userEvent.setup>,
  triggerButtonTestId: string,
  mutationMock: MutationMock,
  successMessage: string,
  successDescription?: string,
  expectedArgs?: unknown[],
  timeout = 5000,
): Promise<void> {
  const triggerButton = screen.getByTestId(triggerButtonTestId);
  await user.click(triggerButton);

  await waitFor(
    () => {
      if (expectedArgs && expectedArgs.length > 0) {
        expect(mutationMock).toHaveBeenCalledWith(...expectedArgs);
      } else {
        expect(mutationMock).toHaveBeenCalled();
      }
      if (successDescription) {
        // Check if toast was called with description (may also include duration or other properties)
        expect(mockToast.success).toHaveBeenCalledWith(
          successMessage,
          expect.objectContaining({
            description: successDescription,
          }),
        );
      } else {
        expect(mockToast.success).toHaveBeenCalledWith(successMessage);
      }
    },
    { timeout },
  );
}

/**
 * Test mutation with error toast
 * @param user - User event instance
 * @param triggerButtonTestId - Test ID of the button that triggers the mutation
 * @param mutationMock - Mock function for the mutation (should be set to reject)
 * @param errorMessage - Expected error toast message
 * @param errorDescription - Optional error description (extracted from error or provided)
 * @param timeout - Timeout in milliseconds (default: 5000)
 */
export async function testMutationWithErrorToast(
  user: ReturnType<typeof userEvent.setup>,
  triggerButtonTestId: string,
  mutationMock: MutationMock,
  errorMessage: string,
  errorDescription?: string,
  timeout = 5000,
): Promise<void> {
  const triggerButton = screen.getByTestId(triggerButtonTestId);
  await user.click(triggerButton);

  await waitFor(
    () => {
      if (errorDescription) {
        expect(mockToast.error).toHaveBeenCalledWith(errorMessage, {
          description: errorDescription,
        });
      } else {
        expect(mockToast.error).toHaveBeenCalledWith(errorMessage);
      }
    },
    { timeout },
  );
}
