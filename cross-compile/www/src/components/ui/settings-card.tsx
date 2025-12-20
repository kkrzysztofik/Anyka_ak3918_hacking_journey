import * as React from 'react';

import { cn } from '@/lib/utils';

const SettingsCard = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(
        'overflow-hidden rounded-[12px] border border-[#3a3a3c] bg-[#1c1c1e]',
        className,
      )}
      {...props}
    />
  ),
);
SettingsCard.displayName = 'SettingsCard';

const SettingsCardHeader = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn('border-b border-[#3a3a3c] p-[24px]', className)} {...props} />
  ),
);
SettingsCardHeader.displayName = 'SettingsCardHeader';

const SettingsCardTitle = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLHeadingElement>
>(({ className, ...props }, ref) => (
  <h3
    ref={ref}
    className={cn(
      'mb-[4px] text-[18px] leading-none font-semibold tracking-tight text-white',
      className,
    )}
    {...props}
  />
));
SettingsCardTitle.displayName = 'SettingsCardTitle';

const SettingsCardDescription = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLParagraphElement>
>(({ className, ...props }, ref) => (
  <p ref={ref} className={cn('text-[14px] text-[#a1a1a6]', className)} {...props} />
));
SettingsCardDescription.displayName = 'SettingsCardDescription';

const SettingsCardContent = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn('p-[24px]', className)} {...props} />
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
