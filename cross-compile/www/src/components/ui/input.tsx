import * as React from 'react';

import { cn } from '@/lib/utils';

const Input = React.forwardRef<HTMLInputElement, React.ComponentProps<'input'>>(
  ({ className, type, ...props }, ref) => {
    return (
      <input
        type={type}
        className={cn(
          // Base styles
          'flex h-9 w-full rounded-md border bg-transparent px-3 py-1 text-base md:text-sm',
          // Border and focus
          'border-input focus-visible:border-ring focus-visible:ring-ring/20 focus-visible:ring-2',
          // Placeholder and file input
          'placeholder:text-muted-foreground file:text-foreground file:border-0 file:bg-transparent file:text-sm file:font-medium',
          // Transitions and states
          'shadow-sm transition-all duration-200 focus-visible:outline-none',
          'disabled:cursor-not-allowed disabled:opacity-50',
          className,
        )}
        ref={ref}
        {...props}
      />
    );
  },
);
Input.displayName = 'Input';

export { Input };
