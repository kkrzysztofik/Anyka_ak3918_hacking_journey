/**
 * Form-specific test helpers
 * Provides reusable functions for testing form interactions, validation, and submission
 */
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { type Mock, expect } from 'vitest';

/**
 * Wait for multiple form fields to have expected values
 * @param fields - Array of { testId, value } objects
 * @param timeout - Timeout in milliseconds (default: 2000)
 */
export async function waitForFormValues(
  fields: Array<{ testId: string; value: string }>,
  timeout = 2000,
): Promise<void> {
  await waitFor(
    () => {
      fields.forEach(({ testId, value }) => {
        const field = screen.getByTestId(testId);
        expect(field).toHaveValue(value);
      });
    },
    { timeout },
  );
}

/**
 * Submit a form by dispatching a submit event
 * @param fieldTestId - Test ID of any field in the form (used to find the form element)
 * @param user - Optional user event instance to reuse (avoids creating a new session)
 */
export async function submitFormByEvent(
  fieldTestId: string,
  user: ReturnType<typeof userEvent.setup>,
): Promise<void> {
  const field = screen.getByTestId(fieldTestId);
  const form = field.closest('form');

  // Validate that the field belongs to a form
  if (!form) {
    throw new Error(
      `submitFormByEvent: No form found for field with testId "${fieldTestId}". ` +
        `The field must be a descendant of a <form> element.`,
    );
  }

  const submitButton = form.querySelector<HTMLButtonElement>('button[type="submit"]');

  if (submitButton) {
    await user.click(submitButton);
    return;
  }

  await act(async () => {
    form.requestSubmit();
  });
}

/**
 * Test form field validation with custom validation logic
 * @param user - User event instance
 * @param fields - Array of { testId, value } objects to fill before validation
 * @param submitButtonTestId - Test ID of the submit button
 * @param onSubmitMock - Mock function that should NOT be called
 * @param timeout - Timeout in milliseconds (default: 1000)
 */
export async function testFormFieldValidation(
  user: ReturnType<typeof userEvent.setup>,
  fields: Array<{ testId: string; value: string }>,
  submitButtonTestId: string,
  onSubmitMock: Mock,
  timeout = 1000,
): Promise<void> {
  // Fill fields
  for (const { testId, value } of fields) {
    const field = screen.getByTestId(testId);
    await user.type(field, value);
  }

  // Attempt to submit
  const submitButton = screen.getByTestId(submitButtonTestId);
  await user.click(submitButton);

  // Verify submission was prevented
  await waitFor(
    () => {
      expect(onSubmitMock).not.toHaveBeenCalled();
    },
    { timeout },
  );
}
