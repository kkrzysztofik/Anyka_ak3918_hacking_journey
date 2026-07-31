---
name: camera-webui-components
description: |
  React 19 component builder for Camera WebUI using shadcn/ui and the design system.
  Triggers on: "React component", "settings page", "shadcn", "create component", "webui", "form component", "card component".
version: 1.0.0
---

# Camera WebUI Component Development

Build production-grade React 19 components for the Anyka camera WebUI using shadcn/ui, TypeScript, and the Camera.UI dark theme with red accents.

## Design System Compliance

All components MUST adhere to the design system specified in the Camera.UI theme. Reference `docs/design/ONVIF.fig` (Figma) and `docs/design/styles/globals.css` for authoritative styles.

### Color Palette

```typescript
// CSS Variables (defined in globals.css)
const colors = {
  background: '#0d0d0d',          // --background
  'nav-background': '#121212',    // --nav-background
  card: '#1c1c1e',                // --card
  border: '#3a3a3c',              // --border
  foreground: '#ffffff',          // --foreground
  'muted-foreground': '#a1a1a6',  // --muted-foreground
  primary: '#ff3b30',             // Red accent (CTA)
  'primary-hover': '#dc2626',     // Dark red
  success: '#34c759',             // Green
  warning: '#ff9500',             // Yellow
  destructive: '#ff3b30',         // Red
};
```

## Component File Structure

Place all new components in `src/components/`:

```
src/components/
├── ui/                         # shadcn/ui components (generated)
│   ├── button.tsx
│   ├── card.tsx
│   ├── input.tsx
│   └── ...
├── layout/                     # Layout components
│   ├── Navbar.tsx
│   └── Sidebar.tsx
├── settings/                   # Settings page components
│   ├── SettingsList.tsx
│   ├── SettingsForm.tsx
│   └── DeviceSettings.tsx
└── common/                     # Shared/reusable components
    ├── StatusBadge.tsx
    ├── LoadingSpinner.tsx
    └── ErrorAlert.tsx
```

## Component Template

Use this structure for all new components:

```typescript
'use client';

import React, { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardHeader, CardTitle, CardContent, CardFooter } from '@/components/ui/card';

interface DeviceSettingsProps {
  onSave: (data: DeviceData) => Promise<void>;
  initialData?: DeviceData;
}

interface DeviceData {
  name: string;
  model: string;
}

/**
 * DeviceSettings component for editing device identification.
 *
 * Features:
 * - Form validation
 * - Save/cancel actions
 * - Loading state during submission
 * - Error handling with user feedback
 */
export function DeviceSettings({
  onSave,
  initialData = { name: '', model: '' }
}: DeviceSettingsProps) {
  const [data, setData] = useState<DeviceData>(initialData);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSave = async () => {
    try {
      setIsLoading(true);
      setError(null);

      // Validate
      if (!data.name.trim()) {
        setError('Device name is required');
        return;
      }

      await onSave(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Card className="bg-card border-border">
      <CardHeader>
        <CardTitle className="text-foreground">Device Information</CardTitle>
      </CardHeader>

      <CardContent className="space-y-4">
        {error && (
          <div
            className="p-3 bg-destructive/10 border border-destructive rounded text-destructive text-sm"
            data-testid="error-message"
            role="alert"
          >
            {error}
          </div>
        )}

        <div className="space-y-2">
          <label
            htmlFor="device-name"
            className="text-sm font-medium text-foreground"
          >
            Device Name
          </label>
          <Input
            id="device-name"
            data-testid="device-name-input"
            placeholder="Enter device name"
            value={data.name}
            onChange={(e) => setData({ ...data, name: e.target.value })}
            disabled={isLoading}
            className="bg-background border-border text-foreground"
          />
        </div>

        <div className="space-y-2">
          <label
            htmlFor="device-model"
            className="text-sm font-medium text-foreground"
          >
            Model
          </label>
          <Input
            id="device-model"
            data-testid="device-model-input"
            placeholder="AK3918"
            value={data.model}
            onChange={(e) => setData({ ...data, model: e.target.value })}
            disabled={isLoading}
            className="bg-background border-border text-foreground"
          />
        </div>
      </CardContent>

      <CardFooter className="gap-2 justify-end pt-4 border-t border-border">
        <Button
          variant="outline"
          data-testid="device-settings-cancel-button"
          onClick={() => setData(initialData)}
          disabled={isLoading}
        >
          Cancel
        </Button>
        <Button
          data-testid="device-settings-save-button"
          onClick={handleSave}
          disabled={isLoading}
          className="bg-primary hover:bg-primary-hover"
        >
          {isLoading ? 'Saving...' : 'Save Changes'}
        </Button>
      </CardFooter>
    </Card>
  );
}
```

## Styling Patterns

### Using CSS Variables (Preferred)

Always use CSS variables defined in `globals.css`:

```typescript
// ✅ CORRECT - Use CSS variables
<div className="bg-card border border-border text-foreground">
  Content
</div>

// ❌ WRONG - Don't hardcode colors
<div className="bg-[#1c1c1e] border border-[#3a3a3c] text-white">
  Content
</div>
```

### Tailwind Extensions

The `tailwind.config.ts` extends with design system colors:

```typescript
// tailwind.config.ts
export default {
  theme: {
    extend: {
      colors: {
        background: 'hsl(var(--background))',
        foreground: 'hsl(var(--foreground))',
        card: 'hsl(var(--card))',
        primary: 'hsl(var(--primary))',
        // ... more colors
      },
    },
  },
};
```

## Form Components

### Input with Validation

```typescript
interface FormInputProps {
  label: string;
  id: string;
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
  error?: string;
  disabled?: boolean;
  required?: boolean;
  testId?: string;
}

export function FormInput({
  label,
  id,
  placeholder,
  value,
  onChange,
  error,
  disabled,
  required,
  testId,
}: FormInputProps) {
  return (
    <div className="space-y-2">
      <label
        htmlFor={id}
        className="text-sm font-medium text-foreground"
      >
        {label}
        {required && <span className="text-destructive ml-1">*</span>}
      </label>
      <Input
        id={id}
        data-testid={testId}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        className={error ? 'border-destructive' : ''}
      />
      {error && (
        <p className="text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
```

### Select Component

```typescript
interface SelectOption {
  value: string;
  label: string;
}

interface FormSelectProps {
  label: string;
  id: string;
  options: SelectOption[];
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  testId?: string;
}

export function FormSelect({
  label,
  id,
  options,
  value,
  onChange,
  disabled,
  testId,
}: FormSelectProps) {
  return (
    <div className="space-y-2">
      <label
        htmlFor={id}
        className="text-sm font-medium text-foreground"
      >
        {label}
      </label>
      <select
        id={id}
        data-testid={testId}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        className="w-full px-3 py-2 bg-background border border-border rounded-md text-foreground"
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}
```

## Layout Components

### Settings Section

```typescript
interface SettingsSectionProps {
  title: string;
  description?: string;
  children: React.ReactNode;
}

export function SettingsSection({
  title,
  description,
  children,
}: SettingsSectionProps) {
  return (
    <Card className="bg-card border-border">
      <CardHeader>
        <CardTitle className="text-lg text-foreground">{title}</CardTitle>
        {description && (
          <p className="text-sm text-muted-foreground mt-1">
            {description}
          </p>
        )}
      </CardHeader>
      <CardContent>
        {children}
      </CardContent>
    </Card>
  );
}
```

## Status and Feedback Components

### Status Badge

```typescript
type StatusType = 'active' | 'inactive' | 'warning' | 'error';

interface StatusBadgeProps {
  status: StatusType;
  label: string;
  testId?: string;
}

export function StatusBadge({ status, label, testId }: StatusBadgeProps) {
  const statusStyles: Record<StatusType, string> = {
    active: 'bg-success/10 text-success border-success/30',
    inactive: 'bg-muted-foreground/10 text-muted-foreground border-muted-foreground/30',
    warning: 'bg-warning/10 text-warning border-warning/30',
    error: 'bg-destructive/10 text-destructive border-destructive/30',
  };

  return (
    <span
      data-testid={testId}
      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-sm font-medium border ${statusStyles[status]}`}
    >
      {label}
    </span>
  );
}
```

### Loading Spinner

```typescript
interface LoadingSpinnerProps {
  size?: 'sm' | 'md' | 'lg';
  testId?: string;
}

export function LoadingSpinner({
  size = 'md',
  testId
}: LoadingSpinnerProps) {
  const sizeClass = {
    sm: 'h-4 w-4',
    md: 'h-8 w-8',
    lg: 'h-12 w-12',
  }[size];

  return (
    <div
      data-testid={testId}
      className={`${sizeClass} animate-spin rounded-full border-2 border-border border-t-primary`}
      role="status"
      aria-label="Loading"
    />
  );
}
```

## Dialog Components

```typescript
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogTrigger,
} from '@/components/ui/dialog';

export function ConfirmDialog({
  title,
  description,
  onConfirm,
  onCancel,
  isOpen,
  isLoading,
}: ConfirmDialogProps) {
  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onCancel?.()}>
      <DialogContent data-testid="confirm-dialog">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button
            variant="outline"
            data-testid="dialog-cancel-button"
            onClick={onCancel}
            disabled={isLoading}
          >
            Cancel
          </Button>
          <Button
            data-testid="dialog-confirm-button"
            onClick={onConfirm}
            disabled={isLoading}
            className="bg-destructive hover:bg-destructive/90"
          >
            {isLoading ? 'Confirming...' : 'Confirm'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
```

## Accessibility Requirements

All components must be fully accessible:

```typescript
// ✅ CORRECT - Proper ARIA labels
<button
  aria-label="Save device settings"
  aria-busy={isLoading}
  disabled={isLoading}
>
  Save
</button>

// ✅ CORRECT - Semantic HTML
<main>
  <section aria-labelledby="settings-title">
    <h1 id="settings-title">Device Settings</h1>
    {/* Content */}
  </section>
</main>

// ❌ WRONG - Missing accessibility
<div onClick={handleSave}>Save</div>
```

## Reference

For detailed component patterns and examples, see `references/component-patterns.md`.
