/**
 * Dialog Component Integration Tests
 */
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from './dialog';

describe('Dialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Basic Rendering', () => {
    it('should render dialog trigger', () => {
      render(
        <Dialog>
          <DialogTrigger data-testid="dialog-trigger">Open Dialog</DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Test Dialog</DialogTitle>
              <DialogDescription>Test description</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <DialogClose>Close</DialogClose>
            </DialogFooter>
          </DialogContent>
        </Dialog>,
      );

      expect(screen.getByTestId('dialog-trigger')).toBeInTheDocument();
    });

    it('should render dialog content when open', async () => {
      render(
        <Dialog defaultOpen>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Test Dialog</DialogTitle>
              <DialogDescription>Test description</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <DialogClose>Close</DialogClose>
            </DialogFooter>
          </DialogContent>
        </Dialog>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('dialog-title')).toHaveTextContent('Test Dialog');
        expect(screen.getByTestId('dialog-description')).toHaveTextContent('Test description');
      });
    });
  });

  describe('Dialog Interactions', () => {
    it('should open dialog when trigger is clicked', async () => {
      const user = userEvent.setup();
      render(
        <Dialog>
          <DialogTrigger data-testid="dialog-trigger">Open Dialog</DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Test Dialog</DialogTitle>
              <DialogDescription>Test description</DialogDescription>
            </DialogHeader>
          </DialogContent>
        </Dialog>,
      );

      const trigger = screen.getByTestId('dialog-trigger');
      await user.click(trigger);

      await waitFor(() => {
        expect(screen.getByTestId('dialog-content')).toBeInTheDocument();
        expect(screen.getByTestId('dialog-title')).toHaveTextContent('Test Dialog');
      });
    });

    it('should close dialog when close button is clicked', async () => {
      const user = userEvent.setup();
      render(
        <Dialog defaultOpen>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Test Dialog</DialogTitle>
              <DialogDescription>Test description</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <DialogClose>Close</DialogClose>
            </DialogFooter>
          </DialogContent>
        </Dialog>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('dialog-content')).toBeInTheDocument();
      });

      const closeButton = screen.getByTestId('dialog-close');
      await user.click(closeButton);

      await waitFor(() => {
        expect(screen.queryByTestId('dialog-content')).not.toBeInTheDocument();
      });
    });

    it('should close dialog when overlay is clicked', async () => {
      const user = userEvent.setup();
      render(
        <Dialog defaultOpen>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Test Dialog</DialogTitle>
              <DialogDescription>Test description</DialogDescription>
            </DialogHeader>
          </DialogContent>
        </Dialog>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('dialog-content')).toBeInTheDocument();
      });

      // Click on overlay (the backdrop)
      const overlay = screen.getByTestId('dialog-overlay');
      await user.click(overlay);
      await waitFor(() => {
        expect(screen.queryByTestId('dialog-content')).not.toBeInTheDocument();
      });
    });
  });

  describe('Controlled Dialog', () => {
    it('should respect open prop', () => {
      const { rerender } = render(
        <Dialog open={true}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Controlled Dialog</DialogTitle>
              <DialogDescription>Test description</DialogDescription>
            </DialogHeader>
          </DialogContent>
        </Dialog>,
      );

      expect(screen.getByTestId('dialog-title')).toHaveTextContent('Controlled Dialog');

      rerender(
        <Dialog open={false}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Controlled Dialog</DialogTitle>
              <DialogDescription>Test description</DialogDescription>
            </DialogHeader>
          </DialogContent>
        </Dialog>,
      );

      expect(screen.queryByTestId('dialog-title')).not.toBeInTheDocument();
    });

    it('should call onOpenChange when dialog state changes', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();

      render(
        <Dialog open={true} onOpenChange={onOpenChange}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Controlled Dialog</DialogTitle>
              <DialogDescription>Test description</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <DialogClose>Close</DialogClose>
            </DialogFooter>
          </DialogContent>
        </Dialog>,
      );

      const closeButton = screen.getByTestId('dialog-close');
      await user.click(closeButton);

      await waitFor(() => {
        expect(onOpenChange).toHaveBeenCalled();
      });
    });
  });

  describe('Dialog Components', () => {
    it('should render DialogHeader with title and description', () => {
      render(
        <Dialog defaultOpen>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Dialog Title</DialogTitle>
              <DialogDescription>Dialog Description</DialogDescription>
            </DialogHeader>
          </DialogContent>
        </Dialog>,
      );

      expect(screen.getByTestId('dialog-title')).toHaveTextContent('Dialog Title');
      expect(screen.getByTestId('dialog-description')).toHaveTextContent('Dialog Description');
    });

    it('should render DialogFooter', () => {
      render(
        <Dialog defaultOpen>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Test</DialogTitle>
              <DialogDescription>Test description</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <button data-testid="footer-action">Action</button>
            </DialogFooter>
          </DialogContent>
        </Dialog>,
      );

      expect(screen.getByTestId('footer-action')).toBeInTheDocument();
    });
  });
});
