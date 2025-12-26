import * as React from 'react';

import { cn } from '@/lib/utils';

interface StatusCardProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
}

const StatusCard = React.forwardRef<HTMLDivElement, StatusCardProps>(
  ({ className, children, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(
        'mb-[32px] rounded-[12px] border border-[#3a3a3c] bg-[#1c1c1e] p-[24px]',
        className,
      )}
      data-testid={props['data-testid' as keyof typeof props] || 'status-card'}
      {...props}
    >
      <div className="flex items-start gap-[24px]">{children}</div>
    </div>
  ),
);
StatusCard.displayName = 'StatusCard';

const StatusCardImage = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, children, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(
        'flex size-[120px] flex-shrink-0 items-center justify-center rounded-[8px] bg-[#2c2c2e] text-[12px] text-[#6b6b6f]',
        className,
      )}
      data-testid={props['data-testid' as keyof typeof props] || 'status-card-image'}
      {...props}
    >
      {children}
    </div>
  ),
);
StatusCardImage.displayName = 'StatusCardImage';

const StatusCardContent = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn('grid flex-1 grid-cols-1 gap-[24px] md:grid-cols-2', className)}
      data-testid={props['data-testid' as keyof typeof props] || 'status-card-content'}
      {...props}
    />
  ),
);
StatusCardContent.displayName = 'StatusCardContent';

const StatusCardItem = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement> & {
    label: string;
    value: React.ReactNode;
  }
>(({ className, label, value, ...props }, ref) => (
  <div
    ref={ref}
    className={cn('', className)}
    data-testid={props['data-testid' as keyof typeof props] || 'status-card-item'}
    {...props}
  >
    <h4 className="mb-[4px] text-[13px] text-[#6b6b6f]">{label}</h4>
    <div className="font-mono text-[15px] text-white">{value}</div>
  </div>
));
StatusCardItem.displayName = 'StatusCardItem';

export { StatusCard, StatusCardImage, StatusCardContent, StatusCardItem };
