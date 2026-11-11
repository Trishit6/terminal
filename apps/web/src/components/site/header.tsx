import { Link, useRouterState } from '@tanstack/react-router';
import { ArrowUpRight, Menu, ShieldCheck, TerminalSquare, UserRound, X } from 'lucide-react';
import { AnimatePresence, motion } from 'motion/react';
import { useEffect, useRef, useState } from 'react';
import { useAuth } from '@/components/auth/auth-provider';
import { usePrefersReducedMotion } from '@/hooks/use-prefers-reduced-motion';
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
  const menuButtonRef = useRef<HTMLButtonElement>(null);
  const menuDialogRef = useRef<HTMLDivElement>(null);
  const { signedIn, user } = useAuth();
  const reducedMotion = usePrefersReducedMotion();
  const pathname = useRouterState({ select: (state) => state.location.pathname });

  useEffect(() => {
    setOpen(false);
  }, [pathname]);

  useEffect(() => {
    if (!open) {
      return;
    }

    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    const frame = window.requestAnimationFrame(() => {
      menuDialogRef.current?.querySelector<HTMLElement>('a, button')?.focus();
    });

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setOpen(false);
        menuButtonRef.current?.focus();
      }
    };

    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.body.style.overflow = previousOverflow;
      window.cancelAnimationFrame(frame);
      document.removeEventListener('keydown', onKeyDown);
    };
  }, [open]);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 24);
    onScroll();
    window.addEventListener('scroll', onScroll, { passive: true });
    return () => window.removeEventListener('scroll', onScroll);
  }, []);

  return (
    <>
    <header
      className={cn(
        'sticky top-0 z-40 border-b transition-colors duration-300',
        scrolled || compact
          ? 'border-border bg-background/95 backdrop-blur-md'
          : 'border-transparent bg-transparent backdrop-blur-0',
      )}
    >
      <div className="mx-auto grid h-16 w-full max-w-site grid-cols-[1fr_auto_1fr] items-center gap-4 px-4 sm:px-6 lg:px-8">
        <div className="justify-self-start">
          <Logo />
        </div>

        <nav className="hidden items-center gap-4 xl:flex 2xl:gap-6" aria-label="Primary">
          {NAV_ITEMS.map((item) => (
            <Link
              key={item.label}
              to={item.to}
              className="relative py-2 text-[13px] text-muted-foreground transition-colors duration-200 after:absolute after:inset-x-0 after:-bottom-[1px] after:h-px after:origin-left after:scale-x-0 after:bg-primary after:transition-transform hover:text-foreground hover:after:scale-x-100"
              activeProps={{
                className: 'text-foreground after:scale-x-100',
              }}
              activeOptions={{ exact: true }}
            >
              {item.label}
            </Link>
          ))}
        </nav>

        <div className="flex items-center justify-self-end gap-1.5">
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
            className="xl:hidden"
            ref={menuButtonRef}
            onClick={() => setOpen((value) => !value)}
            aria-expanded={open}
            aria-controls="site-mobile-navigation"
            aria-label={open ? 'Close menu' : 'Open menu'}
          >
            {open ? <X className="size-4" /> : <Menu className="size-4" />}
          </Button>
        </div>
      </div>
    </header>
    <AnimatePresence initial={false}>
        {open ? (
          <motion.div
            key="site-mobile-navigation"
            className="fixed inset-x-0 bottom-0 top-16 z-40 bg-[#02040a]/55 backdrop-blur-sm xl:hidden"
            initial={reducedMotion ? false : { opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: reducedMotion ? 0 : 0.18 }}
            onClick={() => setOpen(false)}
          >
            <motion.div
              id="site-mobile-navigation"
              ref={menuDialogRef}
              role="dialog"
              aria-modal="true"
              aria-label="Site navigation"
              className="relative max-h-full overflow-y-auto border-b border-border bg-background px-4 py-4 shadow-2xl sm:px-6"
              initial={reducedMotion ? false : { opacity: 0, y: -8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: reducedMotion ? 0 : 0.2, ease: 'easeOut' }}
              onClick={(event) => event.stopPropagation()}
              onKeyDown={(event) => {
                if (event.key !== 'Tab' || !menuDialogRef.current) {
                  return;
                }

                const items = menuDialogRef.current.querySelectorAll<HTMLElement>(
                  'a[href], button:not(:disabled)',
                );
                const first = items[0];
                const last = items[items.length - 1];

                if (event.shiftKey && document.activeElement === first) {
                  event.preventDefault();
                  last?.focus();
                } else if (!event.shiftKey && document.activeElement === last) {
                  event.preventDefault();
                  first?.focus();
                }
              }}
            >
              <nav className="mx-auto flex w-full max-w-site flex-col gap-1" aria-label="Mobile">
            {NAV_ITEMS.map((item) => (
              <Link
                key={item.label}
                to={item.to}
                onClick={() => setOpen(false)}
                className="flex min-h-11 items-center rounded-lg px-3 py-2.5 text-sm text-muted-foreground transition-colors hover:bg-surface-soft hover:text-foreground"
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
            </motion.div>
          </motion.div>
        ) : null}
    </AnimatePresence>
    </>
  );
}
