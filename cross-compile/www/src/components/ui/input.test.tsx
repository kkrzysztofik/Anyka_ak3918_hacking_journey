/**
 * Input Component Tests
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { Input } from './input';

describe('Input', () => {
  it('should render input with default testId', () => {
    render(<Input />);
    const input = screen.getByTestId('input');
    expect(input).toBeInTheDocument();
    expect(input.tagName).toBe('INPUT');
  });

  it('should render input with custom testId', () => {
    render(<Input data-testid="custom-input" />);
    const input = screen.getByTestId('custom-input');
    expect(input).toBeInTheDocument();
  });

  it('should handle input value changes', async () => {
    const user = userEvent.setup();
    render(<Input data-testid="test-input" />);
    const input = screen.getByTestId('test-input');

    await user.type(input, 'test value');
    expect((input as HTMLInputElement).value).toBe('test value');
  });

  it('should support different input types', () => {
    const { rerender } = render(<Input type="text" data-testid="test-input" />);
    let input = screen.getByTestId('test-input');
    expect((input as HTMLInputElement).type).toBe('text');

    rerender(<Input type="password" data-testid="test-input" />);
    input = screen.getByTestId('test-input');
    expect((input as HTMLInputElement).type).toBe('password');

    rerender(<Input type="email" data-testid="test-input" />);
    input = screen.getByTestId('test-input');
    expect((input as HTMLInputElement).type).toBe('email');
  });

  it('should apply custom className', () => {
    render(<Input className="custom-class" data-testid="test-input" />);
    const input = screen.getByTestId('test-input');
    expect(input).toHaveClass('custom-class');
  });

  it('should be disabled when disabled prop is true', () => {
    render(<Input disabled data-testid="test-input" />);
    const input = screen.getByTestId('test-input');
    expect(input).toBeDisabled();
  });

  it('should support placeholder', () => {
    render(<Input placeholder="Enter text" data-testid="test-input" />);
    const input = screen.getByTestId('test-input');
    expect(input).toHaveAttribute('placeholder', 'Enter text');
  });

  it('should forward ref correctly', () => {
    const ref = vi.fn();
    render(<Input ref={ref} data-testid="test-input" />);
    expect(ref).toHaveBeenCalled();
  });

  it('should forward other input props', () => {
    render(<Input name="username" required maxLength={10} data-testid="test-input" />);
    const input = screen.getByTestId('test-input');
    expect(input).toHaveAttribute('name', 'username');
    expect(input).toBeRequired();
    expect((input as HTMLInputElement).maxLength).toBe(10);
  });
});
