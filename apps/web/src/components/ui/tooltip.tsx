import * as TooltipPrimitive from '@radix-ui/react-tooltip';
import * as React from 'react';
import { cn } from '@/lib/utils';

const TooltipProvider = TooltipPrimitive.Provider;

// Radix's Root requires a TooltipProvider ancestor and throws
// "`Tooltip` must be used within `TooltipProvider`" without one. Embedding a
// provider here keeps every Tooltip self-contained (the shared shadcn/ui
// pattern), while callers that want one provider for a group of tooltips —
// sharing delay and skip timing — can still mount TooltipProvider higher in
// the tree and pass Root props (e.g. delayDuration) through to override.
function Tooltip(props: React.ComponentProps<typeof TooltipPrimitive.Root>) {
  return (
    <TooltipProvider delayDuration={300}>
      <TooltipPrimitive.Root {...props} />
    </TooltipProvider>
  );
}

const TooltipTrigger = TooltipPrimitive.Trigger;

const TooltipContent = React.forwardRef<
  React.ComponentRef<typeof TooltipPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof TooltipPrimitive.Content>
>(({ className, sideOffset = 8, ...props }, ref) => (
  <TooltipPrimitive.Portal>
    <TooltipPrimitive.Content
      ref={ref}
      sideOffset={sideOffset}
      className={cn(
        'z-50 overflow-hidden rounded-lg border border-border-strong bg-surface px-2.5 py-1.5 font-mono text-[11px] text-foreground shadow-xl',
        className,
      )}
      {...props}
    />
  </TooltipPrimitive.Portal>
));
TooltipContent.displayName = TooltipPrimitive.Content.displayName;

export { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger };
