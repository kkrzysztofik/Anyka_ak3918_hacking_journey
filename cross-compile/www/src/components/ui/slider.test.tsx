/**
 * Slider Component Tests
 */
import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';

import { Slider } from './slider';

describe('Slider', () => {
  beforeEach(() => {
    // Clear any mocks
  });

  it('should render slider', () => {
    render(<Slider />);
    const slider = screen.getByTestId('slider');
    expect(slider).toBeInTheDocument();
  });

  it('should render slider with default value', () => {
    render(<Slider defaultValue={[50]} />);
    const sliderThumb = screen.getByTestId('slider-thumb');
    expect(sliderThumb).toHaveAttribute('aria-valuenow', '50');
  });

  it('should render slider with controlled value', () => {
    const onValueChange = () => {};
    render(<Slider value={[25]} onValueChange={onValueChange} />);
    const sliderThumb = screen.getByTestId('slider-thumb');
    expect(sliderThumb).toHaveAttribute('aria-valuenow', '25');
  });
});
