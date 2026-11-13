import type { ReactNode } from 'react';
import { Outlet, useRouterState } from '@tanstack/react-router';
import { motion } from 'motion/react';
import { SiteFooter } from '@/components/site/footer';
import { SiteHeader } from '@/components/site/header';
import { usePrefersReducedMotion } from '@/hooks/use-prefers-reduced-motion';

/** Chrome for marketing and error pages. */
export function SiteFrame({ children }: { children: ReactNode }) {
  return (
    <div className="flex min-h-screen flex-col">
      <SiteHeader />
      <main className="site-main flex-1">{children}</main>
      <SiteFooter />
    </div>
  );
}

/**
 * A short opacity fade on navigation. Keyed by pathname so each page enters
 * once; no exit animation, which keeps TanStack Router's navigation and scroll
 * restoration immediate. Reduced motion renders the new page directly.
 */
function RouteTransition({ children }: { children: ReactNode }) {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const reducedMotion = usePrefersReducedMotion();

  return (
    <motion.div
      key={pathname}
      initial={reducedMotion ? false : { opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: reducedMotion ? 0 : 0.22, ease: [0.22, 1, 0.36, 1] }}
    >
      {children}
    </motion.div>
  );
}

/** Chrome for marketing and documentation pages. */
export function SiteLayout() {
  return (
    <SiteFrame>
      <RouteTransition>
        <Outlet />
      </RouteTransition>
    </SiteFrame>
  );
}

/**
 * Chrome for the full-height web terminal: navigation stays available, but
 * the footer is dropped so the terminal keeps the vertical space.
 */
export function TerminalLayout() {
  return (
    <div className="flex min-h-screen flex-col">
      <SiteHeader compact />
      <main className="flex-1">
        <RouteTransition>
          <Outlet />
        </RouteTransition>
      </main>
    </div>
  );
}
