/**
 * PasswordInput Component Tests
 */
import React from 'react';

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { PasswordInput } from './PasswordInput';

// Mock react-hook-form ControllerRenderProps
const createMockField = (value = '') => ({
  onChange: vi.fn(),
  onBlur: vi.fn(),
  value,
  name: 'password',
  ref: vi.fn(),
});

// Type guard to narrow HTMLElement to HTMLInputElement
function isHTMLInputElement(element: HTMLElement): element is HTMLInputElement {
  return element instanceof HTMLInputElement;
}

describe('PasswordInput', () => {
  it('renders password input with toggle button', () => {
    const field = createMockField();
    render(<PasswordInput field={field} testId="test-password-input" />);

    const input = screen.getByTestId('test-password-input');
    expect(input).toBeInTheDocument();
    expect(input).toHaveAttribute('type', 'password');

    const toggleButton = screen.getByTestId('test-password-input-toggle-button');
    expect(toggleButton).toBeInTheDocument();
  });

  it('toggles password visibility when button is clicked', async () => {
    const user = userEvent.setup();
    const field = createMockField();
    render(<PasswordInput field={field} testId="test-password-input" />);

    const inputElement = screen.getByTestId('test-password-input');
    if (!isHTMLInputElement(inputElement)) {
      throw new Error('Expected HTMLInputElement');
    }
    const input = inputElement;
    const toggleButton = screen.getByTestId('test-password-input-toggle-button');

    // Initially password type
    expect(input.type).toBe('password');

    // Click toggle to show password
    await user.click(toggleButton);
    expect(input.type).toBe('text');

    // Click toggle again to hide password
    await user.click(toggleButton);
    expect(input.type).toBe('password');
  });

  it('applies custom className', () => {
    const field = createMockField();
    render(<PasswordInput field={field} testId="test-password-input" className="custom-class" />);

    const input = screen.getByTestId('test-password-input');
    expect(input).toHaveClass('custom-class');
  });

  it('applies custom placeholder', () => {
    const field = createMockField();
    render(
      <PasswordInput
        field={field}
        testId="test-password-input"
        placeholder="Enter your password"
      />,
    );

    const input = screen.getByTestId('test-password-input');
    expect(input).toHaveAttribute('placeholder', 'Enter your password');
  });

  it('disables input when disabled prop is true', () => {
    const field = createMockField();
    render(<PasswordInput field={field} testId="test-password-input" disabled={true} />);

    const input = screen.getByTestId('test-password-input');
    const toggleButton = screen.getByTestId('test-password-input-toggle-button');

    expect(input).toBeDisabled();
    expect(toggleButton).toBeDisabled();
  });

  it('hides toggle button when showToggle is false', () => {
    const field = createMockField();
    render(<PasswordInput field={field} testId="test-password-input" showToggle={false} />);

    expect(screen.queryByTestId('test-password-input-toggle-button')).not.toBeInTheDocument();
  });

  it('forwards field props correctly', () => {
    const field = createMockField('test-value');
    render(<PasswordInput field={field} testId="test-password-input" />);

    const inputElement = screen.getByTestId('test-password-input');
    if (!isHTMLInputElement(inputElement)) {
      throw new Error('Expected HTMLInputElement');
    }
    const input = inputElement;
    expect(input.value).toBe('test-value');
  });
});
