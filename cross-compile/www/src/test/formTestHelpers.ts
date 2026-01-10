/**
 * Form-specific test helpers
 * Provides reusable functions for testing form interactions, validation, and submission
 */
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { type Mock, expect } from 'vitest';

/**
 * Wait for a form field to have an expected value
 * @param fieldTestId - Test ID of the form field
 * @param expectedValue - Expected value
 * @param timeout - Timeout in milliseconds (default: 2000)
 */
export async function waitForFormValue(
  fieldTestId: string,
  expectedValue: string,
  timeout = 2000,
): Promise<void> {
  const field = screen.getByTestId(fieldTestId);
  await waitFor(
    () => {
      expect(field).toHaveValue(expectedValue);
    },
    { timeout },
  );
}

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
 * Fill multiple form fields with values
 * @param user - User event instance
 * @param fields - Array of { testId, value } objects
 */
export async function fillFormFields(
  user: ReturnType<typeof userEvent.setup>,
  fields: Array<{ testId: string; value: string }>,
): Promise<void> {
  for (const { testId, value } of fields) {
    const field = screen.getByTestId(testId);
    await user.type(field, value);
  }
}

/**
 * Submit a form by dispatching a submit event
 * @param fieldTestId - Test ID of any field in the form (used to find the form element)
 */
export async function submitFormByEvent(fieldTestId: string): Promise<void> {
  const field = screen.getByTestId(fieldTestId);
  const form = field.closest('form');
  const submitEvent = new Event('submit', { bubbles: true, cancelable: true });
  await act(async () => {
    form?.dispatchEvent(submitEvent);
  });
}

/**
 * Submit form and wait for submission callback
 * @param user - User event instance
 * @param submitButtonTestId - Test ID of the submit button
 * @param onSubmitMock - Mock function that should be called
 * @param expectedArgs - Expected arguments for the onSubmit call
 * @param timeout - Timeout in milliseconds (default: 3000)
 */
export async function submitFormAndWait(
  user: ReturnType<typeof userEvent.setup>,
  submitButtonTestId: string,
  onSubmitMock?: Mock,
  expectedArgs?: unknown[],
  timeout = 3000,
): Promise<void> {
  const submitButton = screen.getByTestId(submitButtonTestId);
  await user.click(submitButton);

  if (onSubmitMock) {
    await waitFor(
      () => {
        if (expectedArgs) {
          expect(onSubmitMock).toHaveBeenCalledWith(...expectedArgs);
        } else {
          expect(onSubmitMock).toHaveBeenCalled();
        }
      },
      { timeout },
    );
  }
}

/**
 * Test form validation by attempting to submit and verifying the submit handler is not called
 * @param user - User event instance
 * @param submitButtonTestId - Test ID of the submit button
 * @param onSubmitMock - Mock function that should NOT be called
 * @param timeout - Timeout in milliseconds (default: 1000)
 */
export async function testFormValidation(
  user: ReturnType<typeof userEvent.setup>,
  submitButtonTestId: string,
  onSubmitMock: Mock,
  timeout = 1000,
): Promise<void> {
  const submitButton = screen.getByTestId(submitButtonTestId);
  await user.click(submitButton);

  await waitFor(
    () => {
      expect(onSubmitMock).not.toHaveBeenCalled();
    },
    { timeout },
  );
}

/**
 * Fill form fields, wait for values to update, and submit the form
 * @param user - User event instance
 * @param fieldTestId - Test ID of any field in the form (for finding the form)
 * @param fields - Array of { testId, value } objects to fill
 * @param submitButtonTestId - Test ID of the submit button
 * @param onSubmitMock - Optional mock function to verify was called
 * @param expectedArgs - Optional expected arguments for onSubmit
 */
export async function fillAndSubmitForm(
  user: ReturnType<typeof userEvent.setup>,
  fieldTestId: string,
  fields: Array<{ testId: string; value: string }>,
  submitButtonTestId: string,
  onSubmitMock?: Mock,
  expectedArgs?: unknown[],
): Promise<void> {
  // Fill all fields
  await fillFormFields(user, fields);

  // Wait for form state to update
  await waitForFormValues(
    fields.map((f) => ({ testId: f.testId, value: f.value })),
    2000,
  );

  // Submit form
  await submitFormByEvent(fieldTestId);

  // Wait for submission
  if (onSubmitMock) {
    await waitFor(
      () => {
        if (expectedArgs) {
          expect(onSubmitMock).toHaveBeenCalledWith(...expectedArgs);
        } else {
          expect(onSubmitMock).toHaveBeenCalled();
        }
      },
      { timeout: 3000 },
    );
  }
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
