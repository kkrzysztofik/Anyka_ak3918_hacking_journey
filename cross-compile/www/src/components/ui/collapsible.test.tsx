/**
 * Collapsible Component Tests
 */
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { Collapsible, CollapsibleContent, CollapsibleTrigger } from './collapsible';

describe('Collapsible', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render collapsible trigger', () => {
    render(
      <Collapsible>
        <CollapsibleTrigger>Toggle</CollapsibleTrigger>
        <CollapsibleContent>Content</CollapsibleContent>
      </Collapsible>,
    );

    expect(screen.getByTestId('collapsible-trigger')).toHaveTextContent('Toggle');
  });

  it('should show content when open', async () => {
    render(
      <Collapsible defaultOpen>
        <CollapsibleTrigger>Toggle</CollapsibleTrigger>
        <CollapsibleContent>Content</CollapsibleContent>
      </Collapsible>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('collapsible-content')).toHaveTextContent('Content');
    });
  });

  it('should toggle content when trigger is clicked', async () => {
    const user = userEvent.setup();
    render(
      <Collapsible data-testid="collapsible-root">
        <CollapsibleTrigger>Toggle</CollapsibleTrigger>
        <CollapsibleContent>Content</CollapsibleContent>
      </Collapsible>,
    );

    const trigger = screen.getByTestId('collapsible-trigger');
    await user.click(trigger);

    await waitFor(() => {
      expect(screen.getByTestId('collapsible-content')).toHaveTextContent('Content');
    });
  });
});
