/**
 * Form Component Tests
 */
import { render, screen } from '@testing-library/react';
import { useForm } from 'react-hook-form';
import { describe, expect, it, vi } from 'vitest';

import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from './form';
import { Input } from './input';

// Test wrapper component that provides form context
function TestFormWrapper({ children }: Readonly<{ children: React.ReactNode }>) {
  const form = useForm({
    defaultValues: {
      testField: '',
    },
  });

  return <Form {...form}>{children}</Form>;
}

describe('Form Components', () => {
  describe('FormItem', () => {
    it('should render FormItem with default testId', () => {
      render(
        <TestFormWrapper>
          <FormItem>
            <div>Test content</div>
          </FormItem>
        </TestFormWrapper>,
      );
      const formItem = screen.getByTestId('form-item');
      expect(formItem).toBeInTheDocument();
    });

    it('should render FormItem with custom testId', () => {
      render(
        <TestFormWrapper>
          <FormItem data-testid="custom-form-item">
            <div>Test content</div>
          </FormItem>
        </TestFormWrapper>,
      );
      const formItem = screen.getByTestId('custom-form-item');
      expect(formItem).toBeInTheDocument();
    });

    it('should apply custom className', () => {
      render(
        <TestFormWrapper>
          <FormItem className="custom-class">
            <div>Test content</div>
          </FormItem>
        </TestFormWrapper>,
      );
      const formItem = screen.getByTestId('form-item');
      expect(formItem).toHaveClass('custom-class');
    });

    it('should forward ref correctly', () => {
      const ref = vi.fn();
      render(
        <TestFormWrapper>
          <FormItem ref={ref}>
            <div>Test content</div>
          </FormItem>
        </TestFormWrapper>,
      );
      expect(ref).toHaveBeenCalled();
    });
  });

  describe('FormField', () => {
    it('should render FormField with input', () => {
      render(
        <TestFormWrapper>
          <FormField
            name="testField"
            render={({ field }) => <Input {...field} data-testid="test-input" />}
          />
        </TestFormWrapper>,
      );
      const input = screen.getByTestId('test-input');
      expect(input).toBeInTheDocument();
    });
  });

  describe('FormLabel', () => {
    it('should render FormLabel with default testId', () => {
      render(
        <TestFormWrapper>
          <FormField
            name="testField"
            render={() => (
              <FormItem>
                <FormLabel>Test Label</FormLabel>
              </FormItem>
            )}
          />
        </TestFormWrapper>,
      );
      const label = screen.getByTestId('form-label');
      expect(label).toBeInTheDocument();
      expect(label).toHaveTextContent('Test Label');
    });

    it('should render FormLabel within FormField context', () => {
      render(
        <TestFormWrapper>
          <FormField
            name="testField"
            render={() => (
              <FormItem>
                <FormLabel>Test Label</FormLabel>
              </FormItem>
            )}
          />
        </TestFormWrapper>,
      );
      const label = screen.getByTestId('form-label');
      expect(label).toBeInTheDocument();
      expect(label).toHaveTextContent('Test Label');
    });
  });

  describe('FormControl', () => {
    it('should render FormControl with default testId', () => {
      render(
        <TestFormWrapper>
          <FormField
            name="testField"
            render={({ field }) => (
              <FormItem>
                <FormControl>
                  <Input {...field} data-testid="test-input" />
                </FormControl>
              </FormItem>
            )}
          />
        </TestFormWrapper>,
      );
      // FormControl wraps the Input via Slot, so the Input should be rendered
      const input = screen.getByTestId('test-input');
      expect(input).toBeInTheDocument();
      // FormControl merges props with child via Slot component
      // The input should be rendered and functional
      expect(input).toBeInstanceOf(HTMLInputElement);
    });

    it('should render FormControl within FormField context', () => {
      render(
        <TestFormWrapper>
          <FormField
            name="testField"
            render={({ field }) => (
              <FormItem>
                <FormControl>
                  <Input {...field} data-testid="test-input" />
                </FormControl>
              </FormItem>
            )}
          />
        </TestFormWrapper>,
      );
      // FormControl wraps the Input via Slot
      const input = screen.getByTestId('test-input');
      expect(input).toBeInTheDocument();
      expect(input).toBeInstanceOf(HTMLInputElement);
    });
  });

  describe('FormDescription', () => {
    it('should render FormDescription with default testId', () => {
      render(
        <TestFormWrapper>
          <FormField
            name="testField"
            render={() => (
              <FormItem>
                <FormDescription>Test description</FormDescription>
              </FormItem>
            )}
          />
        </TestFormWrapper>,
      );
      const description = screen.getByTestId('form-description');
      expect(description).toBeInTheDocument();
      expect(description).toHaveTextContent('Test description');
    });
  });

  describe('FormMessage', () => {
    it('should render FormMessage with children when provided', () => {
      render(
        <TestFormWrapper>
          <FormField
            name="testField"
            render={() => (
              <FormItem>
                <FormMessage>Custom message</FormMessage>
              </FormItem>
            )}
          />
        </TestFormWrapper>,
      );
      // FormMessage should render when children are provided
      const message = screen.getByTestId('form-message');
      expect(message).toBeInTheDocument();
      expect(message).toHaveTextContent('Custom message');
    });

    it('should render FormMessage with children when no error', () => {
      render(
        <TestFormWrapper>
          <FormField
            name="testField"
            render={() => (
              <FormItem>
                <FormMessage>Custom message</FormMessage>
              </FormItem>
            )}
          />
        </TestFormWrapper>,
      );
      const message = screen.getByTestId('form-message');
      expect(message).toBeInTheDocument();
      expect(message).toHaveTextContent('Custom message');
    });

    it('should not render FormMessage when no error and no children', () => {
      render(
        <TestFormWrapper>
          <FormField
            name="testField"
            render={() => (
              <FormItem>
                <FormMessage />
              </FormItem>
            )}
          />
        </TestFormWrapper>,
      );
      const message = screen.queryByTestId('form-message');
      expect(message).not.toBeInTheDocument();
    });
  });
});
