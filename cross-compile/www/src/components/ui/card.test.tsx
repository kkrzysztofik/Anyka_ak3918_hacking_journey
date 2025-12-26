/**
 * Card Component Tests
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from './card';

describe('Card', () => {
  it('should render card with content', () => {
    render(
      <Card>
        <CardHeader>
          <CardTitle>Card Title</CardTitle>
          <CardDescription>Card Description</CardDescription>
        </CardHeader>
        <CardContent>Card Content</CardContent>
        <CardFooter>Card Footer</CardFooter>
      </Card>,
    );

    expect(screen.getByTestId('card-title')).toHaveTextContent('Card Title');
    expect(screen.getByTestId('card-description')).toHaveTextContent('Card Description');
    expect(screen.getByTestId('card-content')).toHaveTextContent('Card Content');
    expect(screen.getByTestId('card-footer')).toHaveTextContent('Card Footer');
  });

  it('should apply custom className to card', () => {
    const { getByTestId } = render(<Card className="custom-card">Content</Card>);
    const card = getByTestId('card');
    expect(card).toHaveClass('custom-card');
  });

  it('should render card header with title and description', () => {
    render(
      <Card>
        <CardHeader>
          <CardTitle>Title</CardTitle>
          <CardDescription>Description</CardDescription>
        </CardHeader>
      </Card>,
    );

    expect(screen.getByTestId('card-title')).toHaveTextContent('Title');
    expect(screen.getByTestId('card-description')).toHaveTextContent('Description');
  });
});
