/**
 * Sheet Component Integration Tests
 */
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  Sheet,
  SheetClose,
  SheetContent,
  SheetDescription,
  SheetTitle,
  SheetTrigger,
} from './sheet';

describe('Sheet', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Basic Rendering', () => {
    it('should render sheet trigger', () => {
      render(
        <Sheet>
          <SheetTrigger data-testid="sheet-trigger">Open Sheet</SheetTrigger>
          <SheetContent>Sheet Content</SheetContent>
        </Sheet>,
      );

      expect(screen.getByTestId('sheet-trigger')).toBeInTheDocument();
    });

    it('should render sheet content when open', async () => {
      render(
        <Sheet defaultOpen>
          <SheetContent>
            <SheetTitle>Sheet Title</SheetTitle>
            <SheetDescription>Sheet Description</SheetDescription>
            Sheet Content
          </SheetContent>
        </Sheet>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('sheet-content')).toHaveTextContent('Sheet Content');
      });
    });
  });

  describe('Sheet Interactions', () => {
    it('should open sheet when trigger is clicked', async () => {
      const user = userEvent.setup();
      render(
        <Sheet>
          <SheetTrigger data-testid="sheet-trigger">Open Sheet</SheetTrigger>
          <SheetContent>
            <SheetTitle>Sheet Title</SheetTitle>
            <SheetDescription>Sheet Description</SheetDescription>
            Sheet Content
          </SheetContent>
        </Sheet>,
      );

      const trigger = screen.getByTestId('sheet-trigger');
      await user.click(trigger);

      await waitFor(() => {
        expect(screen.getByTestId('sheet-content')).toHaveTextContent('Sheet Content');
      });
    });

    it('should close sheet when close button is clicked', async () => {
      render(
        <Sheet defaultOpen>
          <SheetContent>
            <SheetTitle>Sheet Title</SheetTitle>
            <SheetDescription>Sheet Description</SheetDescription>
            <SheetClose>Close</SheetClose>
            Sheet Content
          </SheetContent>
        </Sheet>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('sheet-content')).toHaveTextContent('Sheet Content');
      });
    });

    it('should close sheet when close button is clicked', async () => {
      const user = userEvent.setup();
      render(
        <Sheet defaultOpen>
          <SheetContent>
            <SheetTitle>Sheet Title</SheetTitle>
            <SheetDescription>Sheet Description</SheetDescription>
            <SheetClose>Close</SheetClose>
            Sheet Content
          </SheetContent>
        </Sheet>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('sheet-content')).toHaveTextContent('Sheet Content');
      });

      const closeButton = screen.getByTestId('sheet-close');
      await user.click(closeButton);

      await waitFor(() => {
        expect(screen.queryByTestId('sheet-content')).not.toBeInTheDocument();
      });
    });
  });

  describe('Sheet Side Variants', () => {
    it('should render sheet on left side', () => {
      render(
        <Sheet defaultOpen>
          <SheetContent side="left">
            <SheetTitle>Sheet Title</SheetTitle>
            <SheetDescription>Sheet Description</SheetDescription>
            Left Sheet
          </SheetContent>
        </Sheet>,
      );

      expect(screen.getByTestId('sheet-content')).toHaveTextContent('Left Sheet');
    });

    it('should render sheet on right side by default', () => {
      render(
        <Sheet defaultOpen>
          <SheetContent>Right Sheet</SheetContent>
        </Sheet>,
      );

      expect(screen.getByTestId('sheet-content')).toHaveTextContent('Right Sheet');
    });
  });
});
