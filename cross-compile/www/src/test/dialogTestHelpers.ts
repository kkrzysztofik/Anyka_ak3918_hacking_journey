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
