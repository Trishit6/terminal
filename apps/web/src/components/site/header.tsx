import { Link, useRouterState } from '@tanstack/react-router';
import { ArrowUpRight, Menu, ShieldCheck, TerminalSquare, UserRound, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useAuth } from '@/components/auth/auth-provider';
import { Logo } from '@/components/site/logo';
import { ThemeToggle } from '@/components/site/theme-toggle';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

const NAV_ITEMS = [
  { label: 'Home', to: '/' },
  { label: 'Features', to: '/features' },
  { label: 'Docs', to: '/docs' },
  { label: 'Downloads', to: '/downloads' },
  { label: 'About', to: '/about' },
  { label: 'Contact', to: '/contact' },
] as const;

export function SiteHeader({ compact = false }: { compact?: boolean }) {
  const [open, setOpen] = useState(false);
  const [scrolled, setScrolled] = useState(false);
  const { signedIn, user } = useAuth();
  const pathname = useRouterState({ select: (state) => state.location.pathname });

  useEffect(() => {
    setOpen(false);
  }, [pathname]);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 24);
    onScroll();
    window.addEventListener('scroll', onScroll, { passive: true });
    return () => window.removeEventListener('scroll', onScroll);
  }, []);

  return (
    <header
      className={cn(
        'sticky top-0 z-40 border-b transition-colors duration-300',
        scrolled || compact
          ? 'border-border bg-background/85 backdrop-blur-xl supports-[backdrop-filter]:bg-background/70'
          : 'border-transparent bg-transparent backdrop-blur-0',
      )}
    >
      <div className="mx-auto flex h-16 w-full max-w-site items-center justify-between gap-4 px-4 sm:px-6 lg:px-8">
        <Logo />

        <nav className="hidden items-center gap-1 lg:flex" aria-label="Primary">
          {NAV_ITEMS.map((item) => (
            <Link
              key={item.label}
              to={item.to}
              className="rounded-lg px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-surface-soft hover:text-foreground"
              activeProps={{
                className:
                  'bg-surface-soft text-foreground shadow-[inset_0_-2px_0_var(--color-primary)]',
              }}
              activeOptions={{ exact: true }}
            >
              {item.label}
            </Link>
          ))}
        </nav>

        <div className="flex items-center gap-2">
          <ThemeToggle />
          {signedIn && user?.role === 'admin' ? (
            <Button asChild variant="ghost" size="sm" className="hidden sm:inline-flex">
              <Link to="/admin">
                <ShieldCheck className="size-4" />
                Admin
              </Link>
            </Button>
          ) : null}
          <Button asChild variant="ghost" size="sm" className="hidden sm:inline-flex">
            <Link to={signedIn ? '/account' : '/sign-in'}>
              <UserRound className="size-4" />
              {signedIn ? (user?.full_name.split(' ')[0] ?? 'Account') : 'Sign in'}
            </Link>
          </Button>
          <Button asChild size="sm" className="hidden sm:inline-flex">
            <Link to="/terminal">
              <TerminalSquare className="size-4" />
              Open terminal
              <ArrowUpRight className="size-3.5 opacity-70" />
            </Link>
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            className="lg:hidden"
            onClick={() => setOpen((value) => !value)}
            aria-expanded={open}
            aria-label={open ? 'Close menu' : 'Open menu'}
          >
            {open ? <X className="size-4" /> : <Menu className="size-4" />}
          </Button>
        </div>
      </div>

      {open ? (
        <div className="border-t border-border bg-background/95 px-4 py-3 lg:hidden">
          <nav className="flex flex-col gap-1" aria-label="Mobile">
            {NAV_ITEMS.map((item) => (
              <Link
                key={item.label}
                to={item.to}
                className="rounded-lg px-3 py-2.5 text-sm text-muted-foreground transition-colors hover:bg-surface-soft hover:text-foreground"
                activeProps={{
                  className: 'bg-surface-soft text-foreground',
                }}
                activeOptions={{ exact: true }}
              >
                {item.label}
              </Link>
            ))}
            {signedIn && user?.role === 'admin' ? (
              <Button asChild variant="secondary" className="mt-2">
                <Link to="/admin">
                  <ShieldCheck className="size-4" />
                  Administration
                </Link>
              </Button>
            ) : null}
            <Button asChild variant="secondary" className="mt-2">
              <Link to={signedIn ? '/account' : '/sign-in'}>
                <UserRound className="size-4" />
                {signedIn ? 'Your account' : 'Sign in'}
              </Link>
            </Button>
            <Button asChild className="mt-1">
              <Link to="/terminal">
                <TerminalSquare className="size-4" />
                Open terminal
              </Link>
            </Button>
          </nav>
        </div>
      ) : null}
    </header>
  );
}
