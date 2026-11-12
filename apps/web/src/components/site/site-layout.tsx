import type { ReactNode } from 'react';
import { Outlet } from '@tanstack/react-router';
import { SiteFooter } from '@/components/site/footer';
import { SiteHeader } from '@/components/site/header';

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

/** Chrome for marketing and documentation pages. */
export function SiteLayout() {
  return (
    <SiteFrame>
      <Outlet />
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
        <Outlet />
      </main>
    </div>
  );
}
