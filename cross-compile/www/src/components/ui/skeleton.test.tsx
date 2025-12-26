/**
 * Skeleton Component Tests
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { Skeleton } from './skeleton';

describe('Skeleton', () => {
  it('should render skeleton with default variant', () => {
    render(<Skeleton />);
    const skeleton = screen.getByTestId('skeleton');
    expect(skeleton).toBeInTheDocument();
    expect(skeleton).toHaveClass('animate-pulse');
  });

  it('should render skeleton with text variant', () => {
    render(<Skeleton variant="text" />);
    const skeleton = screen.getByTestId('skeleton');
    expect(skeleton).toHaveClass('h-4', 'w-full');
  });

  it('should render skeleton with title variant', () => {
    render(<Skeleton variant="title" />);
    const skeleton = screen.getByTestId('skeleton');
    expect(skeleton).toHaveClass('h-6', 'w-3/4');
  });

  it('should render skeleton with avatar variant', () => {
    render(<Skeleton variant="avatar" />);
    const skeleton = screen.getByTestId('skeleton');
    expect(skeleton).toHaveClass('h-10', 'w-10', 'rounded-full');
  });

  it('should render skeleton with card variant', () => {
    render(<Skeleton variant="card" />);
    const skeleton = screen.getByTestId('skeleton');
    expect(skeleton).toHaveClass('h-32', 'w-full', 'rounded-lg');
  });

  it('should apply custom className', () => {
    render(<Skeleton className="custom-class" />);
    const skeleton = screen.getByTestId('skeleton');
    expect(skeleton).toHaveClass('custom-class');
  });
});
