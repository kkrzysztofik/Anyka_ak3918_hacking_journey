/**
 * Dialog-specific test helpers
 * Provides reusable functions for testing dialog interactions, rendering, and states
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { type Mock, expect } from 'vitest';

/**
 * Test dialog is open and visible
 * @param dialogTestId - Test ID of the dialog content
 */
export async function testDialogOpen(dialogTestId: string): Promise<void> {
  await waitFor(() => {
    expect(screen.getByTestId(dialogTestId)).toBeInTheDocument();
  });
}

/**
 * Test dialog is closed and not visible
 * @param dialogTestId - Test ID of the dialog content
 */
export function testDialogClosed(dialogTestId: string): void {
  expect(screen.queryByTestId(dialogTestId)).not.toBeInTheDocument();
}

/**
 * Test dialog cancel action by clicking cancel button
 * @param user - User event instance
 * @param cancelButtonTestId - Test ID of the cancel button
 */
export async function testDialogCancelByButton(
  user: ReturnType<typeof userEvent.setup>,
  cancelButtonTestId: string,
): Promise<void> {
  const cancelButton = screen.getByTestId(cancelButtonTestId);
  await user.click(cancelButton);
}

/**
 * Test dialog cancel action with callback verification
 * @param user - User event instance
 * @param cancelButtonTestId - Test ID of the cancel button
 * @param onOpenChangeMock - Mock function to verify was called with false
 */
export async function testDialogCancelByButtonWithCallback(
  user: ReturnType<typeof userEvent.setup>,
  cancelButtonTestId: string,
  onOpenChangeMock: Mock,
): Promise<void> {
  const cancelButton = screen.getByTestId(cancelButtonTestId);
  await user.click(cancelButton);
  expect(onOpenChangeMock).toHaveBeenCalledWith(false);
}

export async function assertDialogClosed(
  dialogTestId: string,
  dialogContentTestId?: string,
): Promise<void> {
  // Wait for dialog to close (may take a moment for state update)
  await waitFor(
    () => {
      const dialog = screen.queryByTestId(dialogTestId);
      const contentTestId = dialogContentTestId ?? 'dialog-content';
      const dialogContent = screen.queryByTestId(contentTestId);

      expect(dialog).not.toBeInTheDocument();
      if (contentTestId !== dialogTestId) {
        expect(dialogContent).not.toBeInTheDocument();
      }
    },
    { timeout: 3000 },
  );
}

/**
 * Test dialog cancel action and verify it closes
 * @param user - User event instance
 * @param cancelButtonTestId - Test ID of the cancel button
 * @param dialogTestId - Test ID of the dialog to verify it closed
 */
export async function testDialogCancel(
  user: ReturnType<typeof userEvent.setup>,
  cancelButtonTestId: string,
  dialogTestId: string,
): Promise<void> {
  await testDialogCancelByButton(user, cancelButtonTestId);

  const dialogContentTestId = dialogTestId.includes('dialog-content') ? dialogTestId : undefined;

  await assertDialogClosed(dialogTestId, dialogContentTestId);
}

/**
 * Test dialog cancel action with callback verification and verify it closes
 * @param user - User event instance
 * @param cancelButtonTestId - Test ID of the cancel button
 * @param dialogTestId - Test ID of the dialog to verify it closed
 * @param onOpenChangeMock - Mock function to verify was called with false
 */
export async function testDialogCancelWithCallback(
  user: ReturnType<typeof userEvent.setup>,
  cancelButtonTestId: string,
  dialogTestId: string,
  onOpenChangeMock: Mock,
): Promise<void> {
  await testDialogCancelByButtonWithCallback(user, cancelButtonTestId, onOpenChangeMock);

  const dialogContentTestId = dialogTestId.includes('dialog-content') ? dialogTestId : undefined;

  await assertDialogClosed(dialogTestId, dialogContentTestId);
}

/**
 * Test dialog loading state
 * @param loadingTestId - Test ID of the loading indicator
 * @param promiseResolver - Function to resolve the promise that triggers loading
 * @param promise - The promise that triggers the loading state
 */
export async function testDialogLoadingState<T>(
  loadingTestId: string,
  promiseResolver: (value: T) => void,
  promise: Promise<T>,
): Promise<void> {
  // Should show loading
  await waitFor(() => {
    expect(screen.getByTestId(loadingTestId)).toBeInTheDocument();
  });

  // Resolve the promise
  promiseResolver({} as T);
  await promise;
}

/**
 * Render dialog and wait for content to appear
 * @param dialogContentTestId - Test ID of the dialog content
 * @param timeout - Timeout in milliseconds (default: 3000)
 */
export async function renderDialogAndWait(
  dialogContentTestId: string,
  timeout = 3000,
): Promise<void> {
  await waitFor(
    () => {
      expect(screen.getByTestId(dialogContentTestId)).toBeInTheDocument();
    },
    { timeout },
  );
}

/**
 * Test dialog title and description rendering
 * @param titleTestId - Test ID of the dialog title
 * @param descriptionTestId - Test ID of the dialog description
 * @param expectedTitle - Expected title text
 * @param expectedDescription - Expected description text
 */
export async function testDialogTitleAndDescription(
  titleTestId: string,
  descriptionTestId: string,
  expectedTitle: string,
  expectedDescription: string,
): Promise<void> {
  await waitFor(() => {
    const title = screen.getByTestId(titleTestId);
    const description = screen.getByTestId(descriptionTestId);
    expect(title).toHaveTextContent(expectedTitle);
    expect(description).toHaveTextContent(expectedDescription);
  });
}
