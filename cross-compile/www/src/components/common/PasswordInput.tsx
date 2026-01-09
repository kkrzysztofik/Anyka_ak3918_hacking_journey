/**
 * Password Input Component
 *
 * Reusable password input with visibility toggle functionality.
 * Replaces duplicate password toggle logic across the application.
 */
import React, { useState } from 'react';

import { Eye, EyeOff } from 'lucide-react';
import type { ControllerRenderProps, FieldValues } from 'react-hook-form';

import { Input } from '@/components/ui/input';

interface PasswordInputProps<TFieldValues extends FieldValues = FieldValues> {
  field: ControllerRenderProps<TFieldValues>;
  disabled?: boolean;
  testId?: string;
  placeholder?: string;
  className?: string;
  showToggle?: boolean;
  autoComplete?: string;
}

export function PasswordInput<TFieldValues extends FieldValues = FieldValues>({
  field,
  disabled = false,
  testId,
  placeholder = 'Enter password',
  className,
  showToggle = true,
  autoComplete,
}: Readonly<PasswordInputProps<TFieldValues>>) {
  const [showPassword, setShowPassword] = useState(false);

  return (
    <div className="relative">
      <Input
        type={showPassword ? 'text' : 'password'}
        placeholder={placeholder}
        disabled={disabled}
        className={className}
        data-testid={testId}
        autoComplete={autoComplete}
        {...field}
      />
      {showToggle && (
        <button
          type="button"
          onClick={() => setShowPassword(!showPassword)}
          className="text-dark-secondary-text absolute top-1/2 right-3 -translate-y-1/2 transition-colors hover:text-white"
          disabled={disabled}
          data-testid={testId ? `${testId}-toggle-button` : undefined}
          aria-label={showPassword ? 'Hide password' : 'Show password'}
        >
          {showPassword ? <EyeOff className="size-5" /> : <Eye className="size-5" />}
        </button>
      )}
    </div>
  );
}
