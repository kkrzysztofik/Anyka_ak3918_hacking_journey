/**
 * AlertDialog Component Integration Tests
 */
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from './alert-dialog';

describe('AlertDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Basic Rendering', () => {
    it('should render alert dialog trigger', () => {
      render(
        <AlertDialog>
          <AlertDialogTrigger data-testid="alert-trigger">Open Alert</AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Alert Title</AlertDialogTitle>
              <AlertDialogDescription>Alert Description</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction>Confirm</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>,
      );

      expect(screen.getByTestId('alert-trigger')).toBeInTheDocument();
    });

    it('should render alert dialog content when open', async () => {
      render(
        <AlertDialog defaultOpen>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Alert Title</AlertDialogTitle>
              <AlertDialogDescription>Alert Description</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction>Confirm</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('alert-dialog-title')).toHaveTextContent('Alert Title');
        expect(screen.getByTestId('alert-dialog-description')).toHaveTextContent(
          'Alert Description',
        );
      });
    });
  });

  describe('AlertDialog Interactions', () => {
    it('should open alert dialog when trigger is clicked', async () => {
      const user = userEvent.setup();
      render(
        <AlertDialog>
          <AlertDialogTrigger data-testid="alert-trigger">Open Alert</AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Alert Title</AlertDialogTitle>
              <AlertDialogDescription>Are you sure you want to continue?</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction>Confirm</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>,
      );

      const trigger = screen.getByTestId('alert-trigger');
      await user.click(trigger);

      await waitFor(() => {
        expect(screen.getByTestId('alert-dialog-content')).toBeInTheDocument();
        expect(screen.getByTestId('alert-dialog-title')).toHaveTextContent('Alert Title');
      });
    });

    it('should close alert dialog when cancel is clicked', async () => {
      const user = userEvent.setup();
      render(
        <AlertDialog defaultOpen>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Alert Title</AlertDialogTitle>
              <AlertDialogDescription>Are you sure you want to cancel?</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction>Confirm</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('alert-dialog-title')).toBeInTheDocument();
      });

      const cancelButton = screen.getByTestId('alert-dialog-cancel');
      await user.click(cancelButton);

      await waitFor(() => {
        expect(screen.queryByTestId('alert-dialog-title')).not.toBeInTheDocument();
      });
    });

    it('should close alert dialog when action is clicked', async () => {
      const user = userEvent.setup();
      render(
        <AlertDialog defaultOpen>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Alert Title</AlertDialogTitle>

              <AlertDialogDescription>Are you sure?</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction>Confirm</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('alert-dialog-title')).toBeInTheDocument();
      });

      const confirmButton = screen.getByTestId('alert-dialog-action');
      await user.click(confirmButton);

      await waitFor(() => {
        expect(screen.queryByTestId('alert-dialog-title')).not.toBeInTheDocument();
      });
    });
  });

  describe('AlertDialog Components', () => {
    it('should render header with title and description', () => {
      render(
        <AlertDialog defaultOpen>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Delete Item</AlertDialogTitle>
              <AlertDialogDescription>This action cannot be undone.</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction>Delete</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>,
      );

      expect(screen.getByTestId('alert-dialog-title')).toHaveTextContent('Delete Item');
      expect(screen.getByTestId('alert-dialog-description')).toHaveTextContent(
        'This action cannot be undone.',
      );
    });

    it('should render footer with cancel and action buttons', () => {
      render(
        <AlertDialog defaultOpen>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Test</AlertDialogTitle>
              <AlertDialogDescription>Test description</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction>Confirm</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>,
      );

      expect(screen.getByTestId('alert-dialog-cancel')).toBeInTheDocument();
      expect(screen.getByTestId('alert-dialog-action')).toBeInTheDocument();
    });
  });
});
