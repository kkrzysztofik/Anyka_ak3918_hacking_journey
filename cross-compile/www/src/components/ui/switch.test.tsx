/**
 * Switch Component Tests
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { Switch } from './switch';

describe('Switch', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render switch', () => {
    render(<Switch />);
    const switchElement = screen.getByTestId('switch');
    expect(switchElement).toBeInTheDocument();
  });

  it('should be checked when checked prop is true', () => {
    render(<Switch checked={true} />);
    const switchElement = screen.getByTestId('switch');
    expect(switchElement).toBeChecked();
  });

  it('should be unchecked when checked prop is false', () => {
    render(<Switch checked={false} />);
    const switchElement = screen.getByTestId('switch');
    expect(switchElement).not.toBeChecked();
  });

  it('should call onChange when toggled', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Switch checked={false} onCheckedChange={onChange} />);

    const switchElement = screen.getByTestId('switch');
    await user.click(switchElement);

    expect(onChange).toHaveBeenCalledWith(true);
  });

  it('should be disabled when disabled prop is true', () => {
    render(<Switch disabled={true} />);
    const switchElement = screen.getByTestId('switch');
    expect(switchElement).toBeDisabled();
  });
});
