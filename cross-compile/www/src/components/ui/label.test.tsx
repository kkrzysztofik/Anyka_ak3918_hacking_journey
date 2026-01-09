/**
 * Label Component Tests
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { Label } from './label';

describe('Label', () => {
  it('should render label with default testId', () => {
    render(<Label>Test Label</Label>);
    const label = screen.getByTestId('label');
    expect(label).toBeInTheDocument();
    expect(label).toHaveTextContent('Test Label');
  });

  it('should render label with custom testId', () => {
    render(<Label data-testid="custom-label">Custom</Label>);
    const label = screen.getByTestId('custom-label');
    expect(label).toBeInTheDocument();
  });

  it('should apply custom className', () => {
    render(<Label className="custom-class">Custom</Label>);
    const label = screen.getByTestId('label');
    expect(label).toHaveClass('custom-class');
  });

  it('should support htmlFor attribute', () => {
    render(<Label htmlFor="input-id">Label for input</Label>);
    const label = screen.getByTestId('label');
    expect(label).toHaveAttribute('for', 'input-id');
  });

  it('should forward ref correctly', () => {
    const ref = vi.fn();
    render(<Label ref={ref}>Ref test</Label>);
    expect(ref).toHaveBeenCalled();
  });

  it('should forward other label props', () => {
    render(
      <Label data-testid="test-label" aria-label="Accessible label" title="Tooltip">
        Label with props
      </Label>,
    );
    const label = screen.getByTestId('test-label');
    expect(label).toHaveAttribute('aria-label', 'Accessible label');
    expect(label).toHaveAttribute('title', 'Tooltip');
  });
});
