/**
 * Dialog-specific test helpers
 * Provides reusable functions for testing dialog interactions, rendering, and states
 */
import { screen, waitFor } from '@testing-library/react';
import { expect } from 'vitest';

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
