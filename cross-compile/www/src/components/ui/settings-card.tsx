import * as React from 'react';

import { cn } from '@/lib/utils';

const SettingsCard = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn('border-border bg-card overflow-hidden rounded-xl border', className)}
      data-testid={props['data-testid' as keyof typeof props] || 'settings-card'}
      {...props}
    />
  ),
);
SettingsCard.displayName = 'SettingsCard';

const SettingsCardHeader = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn('border-border border-b p-5', className)}
      data-testid={props['data-testid' as keyof typeof props] || 'settings-card-header'}
      {...props}
    />
  ),
);
SettingsCardHeader.displayName = 'SettingsCardHeader';

const SettingsCardTitle = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLHeadingElement> & { children: React.ReactNode }
>(({ className, children, ...props }, ref) => (
  <h3
    ref={ref}
    className={cn('text-foreground text-sm leading-none font-semibold tracking-tight', className)}
    data-testid={props['data-testid' as keyof typeof props] || 'settings-card-title'}
    {...props}
  >
    {children}
  </h3>
));
SettingsCardTitle.displayName = 'SettingsCardTitle';

const SettingsCardDescription = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLParagraphElement>
>(({ className, ...props }, ref) => (
  <p
    ref={ref}
    className={cn('text-muted-foreground text-xs', className)}
    data-testid={props['data-testid' as keyof typeof props] || 'settings-card-description'}
    {...props}
  />
));
SettingsCardDescription.displayName = 'SettingsCardDescription';

const SettingsCardContent = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn('p-5', className)}
      data-testid={props['data-testid' as keyof typeof props] || 'settings-card-content'}
      {...props}
    />
  ),
);
SettingsCardContent.displayName = 'SettingsCardContent';

export {
  SettingsCard,
  SettingsCardHeader,
  SettingsCardTitle,
  SettingsCardDescription,
  SettingsCardContent,
};
