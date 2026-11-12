import { Slot } from '@radix-ui/react-slot';
import { cva, type VariantProps } from 'class-variance-authority';
import * as React from 'react';
import { cn } from '@/lib/utils';

const buttonVariants = cva(
  'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-[color,background-color,border-color,box-shadow,transform] duration-200 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*=size-])]:size-4 active:translate-y-px',
  {
    variants: {
      variant: {
        default:
          'bg-primary text-[color:var(--primary-foreground)] shadow-[inset_0_1px_0_rgba(255,255,255,0.18)] hover:bg-primary-strong',
        secondary:
          'bg-surface-soft text-foreground border border-border hover:border-border-strong hover:bg-surface',
        outline:
          'border border-border-strong bg-surface/30 text-foreground hover:border-primary/55 hover:bg-primary/5',
        ghost: 'text-muted-foreground hover:bg-surface-soft hover:text-foreground',
        destructive: 'bg-danger/15 text-danger border border-danger/30 hover:bg-danger/25',
        link: 'text-primary underline-offset-4 hover:underline',
      },
      size: {
        default: 'h-10 px-4 py-2',
        sm: 'h-8 rounded-md px-3 text-xs',
        lg: 'h-11 rounded-md px-5 text-sm',
        icon: 'size-10',
        'icon-sm': 'size-8 rounded-md',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  },
);

export interface ButtonProps
  extends React.ComponentProps<'button'>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : 'button';
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  },
);
Button.displayName = 'Button';

export { Button, buttonVariants };
