/**
 * StatusCard Component Tests
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { StatusCard, StatusCardContent, StatusCardImage, StatusCardItem } from './status-card';

describe('StatusCard', () => {
  it('should render StatusCard with default testId', () => {
    render(<StatusCard>Test content</StatusCard>);
    const card = screen.getByTestId('status-card');
    expect(card).toBeInTheDocument();
  });

  it('should render StatusCard with custom testId', () => {
    render(<StatusCard data-testid="custom-card">Test content</StatusCard>);
    const card = screen.getByTestId('custom-card');
    expect(card).toBeInTheDocument();
  });

  it('should apply custom className', () => {
    render(<StatusCard className="custom-class">Test content</StatusCard>);
    const card = screen.getByTestId('status-card');
    expect(card).toHaveClass('custom-class');
  });

  it('should forward ref correctly', () => {
    const ref = vi.fn();
    render(<StatusCard ref={ref}>Test content</StatusCard>);
    expect(ref).toHaveBeenCalled();
  });
});

describe('StatusCardImage', () => {
  it('should render StatusCardImage with default testId', () => {
    render(<StatusCardImage>Image content</StatusCardImage>);
    const image = screen.getByTestId('status-card-image');
    expect(image).toBeInTheDocument();
  });

  it('should render StatusCardImage with custom testId', () => {
    render(<StatusCardImage data-testid="custom-image">Image content</StatusCardImage>);
    const image = screen.getByTestId('custom-image');
    expect(image).toBeInTheDocument();
  });

  it('should apply custom className', () => {
    render(<StatusCardImage className="custom-class">Image content</StatusCardImage>);
    const image = screen.getByTestId('status-card-image');
    expect(image).toHaveClass('custom-class');
  });
});

describe('StatusCardContent', () => {
  it('should render StatusCardContent with default testId', () => {
    render(<StatusCardContent>Content</StatusCardContent>);
    const content = screen.getByTestId('status-card-content');
    expect(content).toBeInTheDocument();
  });

  it('should render StatusCardContent with custom testId', () => {
    render(<StatusCardContent data-testid="custom-content">Content</StatusCardContent>);
    const content = screen.getByTestId('custom-content');
    expect(content).toBeInTheDocument();
  });

  it('should apply custom className', () => {
    render(<StatusCardContent className="custom-class">Content</StatusCardContent>);
    const content = screen.getByTestId('status-card-content');
    expect(content).toHaveClass('custom-class');
  });
});

describe('StatusCardItem', () => {
  it('should render StatusCardItem with default testId', () => {
    render(<StatusCardItem label="Test Label" value="Test Value" />);
    const item = screen.getByTestId('status-card-item');
    expect(item).toBeInTheDocument();
  });

  it('should render StatusCardItem with custom testId', () => {
    render(<StatusCardItem label="Test Label" value="Test Value" data-testid="custom-item" />);
    const item = screen.getByTestId('custom-item');
    expect(item).toBeInTheDocument();
  });

  it('should display label and value', () => {
    render(<StatusCardItem label="Status" value="Online" />);
    expect(screen.getByText('Status')).toBeInTheDocument();
    expect(screen.getByText('Online')).toBeInTheDocument();
  });

  it('should render ReactNode as value', () => {
    render(<StatusCardItem label="Count" value={<span data-testid="value-node">42</span>} />);
    expect(screen.getByText('Count')).toBeInTheDocument();
    expect(screen.getByTestId('value-node')).toHaveTextContent('42');
  });

  it('should apply custom className', () => {
    render(<StatusCardItem label="Test Label" value="Test Value" className="custom-class" />);
    const item = screen.getByTestId('status-card-item');
    expect(item).toHaveClass('custom-class');
  });
});

describe('StatusCard Composition', () => {
  it('should render complete StatusCard structure', () => {
    render(
      <StatusCard>
        <StatusCardImage>Image</StatusCardImage>
        <StatusCardContent>
          <StatusCardItem label="Label 1" value="Value 1" />
          <StatusCardItem label="Label 2" value="Value 2" />
        </StatusCardContent>
      </StatusCard>,
    );

    expect(screen.getByTestId('status-card')).toBeInTheDocument();
    expect(screen.getByTestId('status-card-image')).toBeInTheDocument();
    expect(screen.getByTestId('status-card-content')).toBeInTheDocument();
    expect(screen.getByText('Label 1')).toBeInTheDocument();
    expect(screen.getByText('Value 1')).toBeInTheDocument();
    expect(screen.getByText('Label 2')).toBeInTheDocument();
    expect(screen.getByText('Value 2')).toBeInTheDocument();
  });
});
