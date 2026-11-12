import { Link, useRouterState } from '@tanstack/react-router';
import { ArrowUpRight, Menu, ShieldCheck, TerminalSquare, UserRound, X } from 'lucide-react';
import { AnimatePresence, motion } from 'motion/react';
import { useEffect, useRef, useState } from 'react';
import { useAuth } from '@/components/auth/auth-provider';
import { usePrefersReducedMotion } from '@/hooks/use-prefers-reduced-motion';
import { Logo } from '@/components/site/logo';
import { ThemeToggle } from '@/components/site/theme-toggle';
import { Button } from '@/components/ui/button';

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
    const onScroll = () => setScrolled(window.scrollY > 16);
    onScroll();
    window.addEventListener('scroll', onScroll, { passive: true });
    return () => window.removeEventListener('scroll', onScroll);
  }, []);

  return (
    <>
      <header
        className="site-header"
        data-scrolled={scrolled ? 'true' : undefined}
        data-compact={compact ? 'true' : undefined}
      >
        <div className="site-header__inner">
          <div className="justify-self-start">
            <Logo />
          </div>

          <nav className="nav-cluster" aria-label="Primary">
            {NAV_ITEMS.map((item) => (
              <Link
                key={item.label}
                to={item.to}
                className="nav-link"
                activeProps={{ className: 'is-active' }}
                activeOptions={{ exact: true }}
              >
                {item.label}
              </Link>
            ))}
          </nav>

          <div className="site-header__actions">
            <ThemeToggle />
            {signedIn && user?.role === 'admin' ? (
              <Button asChild variant="ghost" size="sm" className="hidden lg:inline-flex">
                <Link to="/admin">
                  <ShieldCheck className="size-4" />
                  Admin
                </Link>
              </Button>
            ) : null}
            <Button asChild variant="ghost" size="sm" className="hidden md:inline-flex">
              <Link to={signedIn ? '/account' : '/sign-in'}>
                <UserRound className="size-4" />
                {signedIn ? (user?.full_name.split(' ')[0] ?? 'Account') : 'Sign in'}
              </Link>
            </Button>
            <Link to="/terminal" className="site-header__terminal">
              <TerminalSquare className="size-4" aria-hidden="true" />
              <span>Open terminal</span>
              <ArrowUpRight className="size-3.5 opacity-65" aria-hidden="true" />
            </Link>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              className="size-11 xl:hidden"
              ref={menuButtonRef}
              onClick={() => setOpen((value) => !value)}
              aria-expanded={open}
              aria-controls="site-mobile-navigation"
              aria-label={open ? 'Close menu' : 'Open menu'}
            >
              {open ? <X className="size-5" /> : <Menu className="size-5" />}
            </Button>
          </div>
        </div>
      </header>

      <AnimatePresence initial={false}>
        {open ? (
          <motion.div
            key="site-mobile-navigation"
            id="site-mobile-navigation"
            ref={menuDialogRef}
            role="dialog"
            aria-modal="true"
            aria-label="Site navigation"
            className="site-mobile"
            initial={reducedMotion ? false : { opacity: 0, y: -6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={{ duration: reducedMotion ? 0 : 0.18, ease: 'easeOut' }}
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
            <div className="mx-auto w-full max-w-site">
              <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                Navigation / KERNEXT
              </p>

              <nav className="site-mobile__index" aria-label="Mobile">
                {NAV_ITEMS.map((item, index) => (
                  <Link
                    key={item.label}
                    to={item.to}
                    onClick={() => setOpen(false)}
                    className="site-mobile__link"
                    activeProps={{ className: 'is-active' }}
                    activeOptions={{ exact: true }}
                  >
                    <span>{String(index + 1).padStart(2, '0')}</span>
                    <span className="flex-1">{item.label}</span>
                    <ArrowUpRight className="size-5 shrink-0 opacity-40" aria-hidden="true" />
                  </Link>
                ))}
              </nav>

              <div className="mt-5 grid gap-2">
                {signedIn && user?.role === 'admin' ? (
                  <Button asChild variant="secondary" className="justify-between">
                    <Link to="/admin" onClick={() => setOpen(false)}>
                      <span className="flex items-center gap-2">
                        <ShieldCheck className="size-4" />
                        Administration
                      </span>
                      <ArrowUpRight className="size-4 opacity-50" />
                    </Link>
                  </Button>
                ) : null}
                <Button asChild variant="secondary" className="justify-between">
                  <Link to={signedIn ? '/account' : '/sign-in'} onClick={() => setOpen(false)}>
                    <span className="flex items-center gap-2">
                      <UserRound className="size-4" />
                      {signedIn ? 'Your account' : 'Sign in'}
                    </span>
                    <ArrowUpRight className="size-4 opacity-50" />
                  </Link>
                </Button>
                <Button asChild className="justify-between">
                  <Link to="/terminal" onClick={() => setOpen(false)}>
                    <span className="flex items-center gap-2">
                      <TerminalSquare className="size-4" />
                      Open terminal
                    </span>
                    <ArrowUpRight className="size-4 opacity-60" />
                  </Link>
                </Button>
              </div>

              <p className="site-mobile__footer">
                Independent · Native · Engineered
                <br />
                Build. Execute. Innovate.
              </p>
            </div>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </>
  );
}
