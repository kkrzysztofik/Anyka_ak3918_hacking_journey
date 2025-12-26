/**
 * RadioGroup Component Tests
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { RadioGroup, RadioGroupItem } from './radio-group';

describe('RadioGroup', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render radio group', () => {
    render(
      <RadioGroup>
        <RadioGroupItem value="option1" />
        <RadioGroupItem value="option2" />
      </RadioGroup>,
    );

    const radios = screen.getAllByTestId('radio-group-item');
    expect(radios).toHaveLength(2);
  });

  it('should select radio item when clicked', async () => {
    const user = userEvent.setup();
    render(
      <RadioGroup>
        <RadioGroupItem value="option1" />
        <RadioGroupItem value="option2" />
      </RadioGroup>,
    );

    const radios = screen.getAllByTestId('radio-group-item');
    await user.click(radios[0]);

    expect(radios[0]).toBeChecked();
  });

  it('should call onValueChange when value changes', async () => {
    const user = userEvent.setup();
    const onValueChange = vi.fn();
    render(
      <RadioGroup onValueChange={onValueChange}>
        <RadioGroupItem value="option1" />
        <RadioGroupItem value="option2" />
      </RadioGroup>,
    );

    const radios = screen.getAllByTestId('radio-group-item');
    await user.click(radios[1]);

    expect(onValueChange).toHaveBeenCalledWith('option2');
  });
});
