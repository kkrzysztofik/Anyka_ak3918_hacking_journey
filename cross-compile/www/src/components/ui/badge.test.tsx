/**
 * Badge Component Tests
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { Badge } from './badge';

describe('Badge', () => {
  it('should render badge with default variant', () => {
    render(<Badge>Test Badge</Badge>);
    const badge = screen.getByTestId('badge');
    expect(badge).toHaveTextContent('Test Badge');
    expect(badge).toHaveClass('bg-primary');
  });

  it('should render badge with secondary variant', () => {
    render(<Badge variant="secondary">Secondary</Badge>);
    const badge = screen.getByTestId('badge');
    expect(badge).toHaveTextContent('Secondary');
    expect(badge).toHaveClass('bg-secondary');
  });

  it('should render badge with destructive variant', () => {
    render(<Badge variant="destructive">Destructive</Badge>);
    const badge = screen.getByTestId('badge');
    expect(badge).toHaveTextContent('Destructive');
    expect(badge).toHaveClass('bg-destructive');
  });

  it('should render badge with outline variant', () => {
    render(<Badge variant="outline">Outline</Badge>);
    const badge = screen.getByTestId('badge');
    expect(badge).toHaveTextContent('Outline');
    expect(badge).toHaveClass('text-foreground');
  });

  it('should apply custom className', () => {
    render(<Badge className="custom-class">Custom</Badge>);
    const badge = screen.getByTestId('badge');
    expect(badge).toHaveClass('custom-class');
  });
});
