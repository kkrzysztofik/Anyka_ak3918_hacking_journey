import { cn } from '@/lib/utils';

interface SkeletonProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Variant for common skeleton shapes */
  variant?: 'text' | 'title' | 'avatar' | 'card' | 'default';
}

/**
 * Skeleton loading component for placeholder content
 */
function Skeleton({ className, variant = 'default', ...props }: SkeletonProps) {
  const variantClasses = {
    default: '',
    text: 'h-4 w-full rounded',
    title: 'h-6 w-3/4 rounded',
    avatar: 'h-10 w-10 rounded-full',
    card: 'h-32 w-full rounded-lg',
  };

  return (
    <div
      className={cn('bg-muted animate-pulse rounded', variantClasses[variant], className)}
      {...props}
    />
  );
}

export { Skeleton };
