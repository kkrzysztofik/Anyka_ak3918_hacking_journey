/**
 * SettingsCard Component Tests
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import {
  SettingsCard,
  SettingsCardContent,
  SettingsCardDescription,
  SettingsCardHeader,
  SettingsCardTitle,
} from './settings-card';

describe('SettingsCard', () => {
  it('should render SettingsCard with default testId', () => {
    render(<SettingsCard>Test content</SettingsCard>);
    const card = screen.getByTestId('settings-card');
    expect(card).toBeInTheDocument();
  });

  it('should render SettingsCard with custom testId', () => {
    render(<SettingsCard data-testid="custom-card">Test content</SettingsCard>);
    const card = screen.getByTestId('custom-card');
    expect(card).toBeInTheDocument();
  });

  it('should apply custom className', () => {
    render(<SettingsCard className="custom-class">Test content</SettingsCard>);
    const card = screen.getByTestId('settings-card');
    expect(card).toHaveClass('custom-class');
  });

  it('should forward ref correctly', () => {
    const ref = vi.fn();
    render(<SettingsCard ref={ref}>Test content</SettingsCard>);
    expect(ref).toHaveBeenCalled();
  });
});

describe('SettingsCardHeader', () => {
  it('should render SettingsCardHeader with default testId', () => {
    render(<SettingsCardHeader>Header content</SettingsCardHeader>);
    const header = screen.getByTestId('settings-card-header');
    expect(header).toBeInTheDocument();
  });

  it('should render SettingsCardHeader with custom testId', () => {
    render(<SettingsCardHeader data-testid="custom-header">Header content</SettingsCardHeader>);
    const header = screen.getByTestId('custom-header');
    expect(header).toBeInTheDocument();
  });

  it('should apply custom className', () => {
    render(<SettingsCardHeader className="custom-class">Header content</SettingsCardHeader>);
    const header = screen.getByTestId('settings-card-header');
    expect(header).toHaveClass('custom-class');
  });
});

describe('SettingsCardTitle', () => {
  it('should render SettingsCardTitle with default testId', () => {
    render(<SettingsCardTitle>Card Title</SettingsCardTitle>);
    const title = screen.getByTestId('settings-card-title');
    expect(title).toBeInTheDocument();
    expect(title).toHaveTextContent('Card Title');
  });

  it('should render SettingsCardTitle with custom testId', () => {
    render(<SettingsCardTitle data-testid="custom-title">Card Title</SettingsCardTitle>);
    const title = screen.getByTestId('custom-title');
    expect(title).toBeInTheDocument();
  });

  it('should render as h3 element', () => {
    render(<SettingsCardTitle>Card Title</SettingsCardTitle>);
    const title = screen.getByTestId('settings-card-title');
    expect(title.tagName).toBe('H3');
  });
});

describe('SettingsCardDescription', () => {
  it('should render SettingsCardDescription with default testId', () => {
    render(<SettingsCardDescription>Card description</SettingsCardDescription>);
    const description = screen.getByTestId('settings-card-description');
    expect(description).toBeInTheDocument();
    expect(description).toHaveTextContent('Card description');
  });

  it('should render SettingsCardDescription with custom testId', () => {
    render(
      <SettingsCardDescription data-testid="custom-description">
        Card description
      </SettingsCardDescription>,
    );
    const description = screen.getByTestId('custom-description');
    expect(description).toBeInTheDocument();
  });
});

describe('SettingsCardContent', () => {
  it('should render SettingsCardContent with default testId', () => {
    render(<SettingsCardContent>Content</SettingsCardContent>);
    const content = screen.getByTestId('settings-card-content');
    expect(content).toBeInTheDocument();
  });

  it('should render SettingsCardContent with custom testId', () => {
    render(<SettingsCardContent data-testid="custom-content">Content</SettingsCardContent>);
    const content = screen.getByTestId('custom-content');
    expect(content).toBeInTheDocument();
  });

  it('should apply custom className', () => {
    render(<SettingsCardContent className="custom-class">Content</SettingsCardContent>);
    const content = screen.getByTestId('settings-card-content');
    expect(content).toHaveClass('custom-class');
  });
});

describe('SettingsCard Composition', () => {
  it('should render complete SettingsCard structure', () => {
    render(
      <SettingsCard>
        <SettingsCardHeader>
          <SettingsCardTitle>Test Title</SettingsCardTitle>
          <SettingsCardDescription>Test Description</SettingsCardDescription>
        </SettingsCardHeader>
        <SettingsCardContent>Test Content</SettingsCardContent>
      </SettingsCard>,
    );

    expect(screen.getByTestId('settings-card')).toBeInTheDocument();
    expect(screen.getByTestId('settings-card-header')).toBeInTheDocument();
    expect(screen.getByTestId('settings-card-title')).toHaveTextContent('Test Title');
    expect(screen.getByTestId('settings-card-description')).toHaveTextContent('Test Description');
    expect(screen.getByTestId('settings-card-content')).toHaveTextContent('Test Content');
  });
});
