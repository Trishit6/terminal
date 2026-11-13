import {
  Link,
  Outlet,
  useNavigate,
  useRouterState,
} from '@tanstack/react-router';
import {
  Activity,
  ArrowLeft,
  ChevronRight,
  LayoutDashboard,
  Loader2,
  LogOut,
  Menu,
  MonitorSmartphone,
  ScrollText,
  Settings,
  ShieldAlert,
  ShieldCheck,
  Users,
  X,
  type LucideIcon,
} from 'lucide-react';
import { motion } from 'motion/react';
import { useEffect, useRef, useState } from 'react';
import { useAuth } from '@/components/auth/auth-provider';
import { LogoMark } from '@/components/site/logo';
import { ThemeToggle } from '@/components/site/theme-toggle';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

interface NavItem {
  label: string;
  to: string;
  icon: LucideIcon;
}

const NAV_ITEMS: NavItem[] = [
  { label: 'Overview', to: '/admin', icon: LayoutDashboard },
  { label: 'Users', to: '/admin/users', icon: Users },
  {
    label: 'Terminal sessions',
    to: '/admin/terminal-sessions',
    icon: MonitorSmartphone,
  },
  { label: 'System health', to: '/admin/health', icon: Activity },
  { label: 'Audit log', to: '/admin/audit', icon: ScrollText },
  { label: 'Settings', to: '/admin/settings', icon: Settings },
];

function breadcrumbLabels(pathname: string): string[] {
  const rest = pathname.replace(/^\/admin\/?/, '');
  if (!rest) {
    return [];
  }

  const [section, detail] = rest.split('/');

  switch (section) {
    case 'users':
      return detail ? ['Users', 'User details'] : ['Users'];
    case 'terminal-sessions':
      return ['Terminal sessions'];
    case 'health':
      return ['System health'];
    case 'audit':
      return ['Audit log'];
    case 'settings':
      return ['Settings'];
    default:
      return [];
  }
}

function SidebarNav({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <nav aria-label="Administration" className="flex flex-1 flex-col gap-1 p-3">
      {NAV_ITEMS.map((item) => {
        const Icon = item.icon;
        return (
          <Link
            key={item.to}
            to={item.to}
            onClick={onNavigate}
            activeOptions={{ exact: item.to === '/admin' }}
            activeProps={{
              className:
                'border-primary/25 bg-primary/10 text-primary',
              'aria-current': 'page',
            }}
            className="flex items-center gap-3 rounded-xl border border-transparent px-3 py-2.5 text-sm text-muted-foreground transition-colors hover:bg-surface-soft hover:text-foreground"
          >
            <Icon className="size-4" aria-hidden="true" />
            <span>{item.label}</span>
          </Link>
        );
      })}
    </nav>
  );
}

function AdminBrand() {
  return (
    <div className="flex h-16 items-center gap-3 border-b border-border px-5">
      <LogoMark className="size-8" />
      <div className="min-w-0 leading-none">
        <p className="font-mono text-[12px] font-semibold tracking-[0.24em] text-foreground">
          KERNEXT
        </p>
        <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.22em] text-muted-foreground">
          Administration
        </p>
      </div>
    </div>
  );
}

function CenteredNotice({
  icon,
  title,
  description,
  action,
}: {
  icon: React.ReactNode;
  title: string;
  description: string;
  action?: React.ReactNode;
}) {
  return (
    <div className="grid min-h-screen place-items-center bg-background px-4">
      <div className="w-full max-w-md rounded-2xl border border-border bg-surface p-6 text-center">
        <div className="mx-auto grid size-11 place-items-center rounded-xl border border-border bg-surface-soft text-primary">
          {icon}
        </div>
        <h1 className="mt-4 text-lg font-semibold tracking-tight">{title}</h1>
        <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
          {description}
        </p>
        {action ? <div className="mt-5 flex justify-center gap-2">{action}</div> : null}
      </div>
    </div>
  );
}

function AccountMenu() {
  const { user, signOut } = useAuth();
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const [open, setOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setOpen(false);
  }, [pathname]);

  useEffect(() => {
    if (!open) {
      return;
    }

    function onPointerDown(event: MouseEvent) {
      if (!menuRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    }

    function onKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        setOpen(false);
      }
    }

    document.addEventListener('mousedown', onPointerDown);
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('mousedown', onPointerDown);
      document.removeEventListener('keydown', onKeyDown);
    };
  }, [open]);

  if (!user) {
    return null;
  }

  return (
    <div className="relative" ref={menuRef}>
      <button
        type="button"
        onClick={() => setOpen((value) => !value)}
        aria-expanded={open}
        aria-haspopup="menu"
        className="flex items-center gap-2 rounded-xl border border-border bg-surface/60 py-1.5 pl-1.5 pr-3 text-left transition-colors hover:border-border-strong"
      >
        <span className="grid size-7 place-items-center rounded-lg bg-surface-soft font-mono text-[10px] font-semibold text-primary">
          {user.full_name
            .split(/\s+/)
            .map((part) => part[0])
            .filter(Boolean)
            .slice(0, 2)
            .join('')
            .toUpperCase()}
        </span>
        <span className="hidden min-w-0 sm:block">
          <span className="block max-w-36 truncate text-xs font-medium text-foreground">
            {user.full_name}
          </span>
          <span className="block max-w-36 truncate text-[10px] text-muted-foreground">
            {user.email}
          </span>
        </span>
      </button>

      {open ? (
        <div
          role="menu"
          aria-label="Account"
          className="absolute right-0 top-full z-50 mt-2 w-56 rounded-xl border border-border bg-surface p-1.5 shadow-2xl"
        >
          <Link
            role="menuitem"
            to="/account"
            className="block rounded-lg px-3 py-2 text-sm text-muted-foreground hover:bg-surface-soft hover:text-foreground"
          >
            Your account
          </Link>
          <Link
            role="menuitem"
            to="/admin/settings"
            className="block rounded-lg px-3 py-2 text-sm text-muted-foreground hover:bg-surface-soft hover:text-foreground"
          >
            Admin settings
          </Link>
          <div className="my-1 border-t border-border" />
          <button
            role="menuitem"
            type="button"
            onClick={() => {
              setOpen(false);
              void signOut().then(() => {
                void navigate({ to: '/' });
              });
            }}
            className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-muted-foreground hover:bg-surface-soft hover:text-foreground"
          >
            <LogOut className="size-4" aria-hidden="true" />
            Sign out
          </button>
        </div>
      ) : null}
    </div>
  );
}

/**
 * Protected administration chrome.
 *
 * The backend rejects non-administrators on every `/api/admin` route; this
 * guard only decides what to render and where to send a visitor.
 */
export function AdminLayout() {
  const { ready, signedIn, user, accountsEnabled } = useAuth();
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const [mobileOpen, setMobileOpen] = useState(false);
  const mobileMenuButtonRef = useRef<HTMLButtonElement>(null);
  const mobileDrawerRef = useRef<HTMLDivElement>(null);
  const crumbs = breadcrumbLabels(pathname);

  useEffect(() => {
    setMobileOpen(false);
  }, [pathname]);

  useEffect(() => {
    if (!mobileOpen) {
      return;
    }

    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    const frame = window.requestAnimationFrame(() => {
      mobileDrawerRef.current
        ?.querySelector<HTMLElement>('a, button')
        ?.focus();
    });
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setMobileOpen(false);
        mobileMenuButtonRef.current?.focus();
        return;
      }

      if (event.key === 'Tab' && mobileDrawerRef.current) {
        const items = mobileDrawerRef.current.querySelectorAll<HTMLElement>(
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
      }
    };

    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.body.style.overflow = previousOverflow;
      window.cancelAnimationFrame(frame);
      document.removeEventListener('keydown', onKeyDown);
    };
  }, [mobileOpen]);

  useEffect(() => {
    if (ready && accountsEnabled && !signedIn) {
      void navigate({ to: '/sign-in', search: { redirect: pathname } });
    }
  }, [ready, accountsEnabled, signedIn, navigate, pathname]);

  if (!ready) {
    return (
      <div className="grid min-h-screen place-items-center bg-background">
        <p className="flex items-center gap-2 text-sm text-muted-foreground" aria-busy="true">
          <Loader2 className="size-4 animate-spin text-primary" aria-hidden="true" />
          Checking administrator access…
        </p>
      </div>
    );
  }

  if (!accountsEnabled) {
    return (
      <CenteredNotice
        icon={<ShieldAlert className="size-5" />}
        title="Accounts are not configured"
        description="The administration console needs the MariaDB account database. Set DATABASE_URL and restart the KERNEXT server."
        action={
          <Button asChild variant="outline" size="sm">
            <Link to="/">Back to KERNEXT</Link>
          </Button>
        }
      />
    );
  }

  if (!signedIn || !user) {
    return (
      <CenteredNotice
        icon={<Loader2 className="size-5 animate-spin" />}
        title="Sign in required"
        description="Redirecting to the KERNEXT sign-in page…"
        action={
          <Button asChild variant="outline" size="sm">
            <Link to="/sign-in" search={{ redirect: pathname }}>
              Sign in
            </Link>
          </Button>
        }
      />
    );
  }

  if (user.role !== 'admin') {
    return (
      <CenteredNotice
        icon={<ShieldAlert className="size-5" />}
        title="Administrator access required"
        description="This account is signed in, but it does not hold the administrator role. Ask an administrator for access if you need it."
        action={
          <>
            <Button asChild variant="outline" size="sm">
              <Link to="/account">Your account</Link>
            </Button>
            <Button asChild size="sm">
              <Link to="/">Back to KERNEXT</Link>
            </Button>
          </>
        }
      />
    );
  }

  return (
    <div className="min-h-screen bg-background">
      <a
        href="#admin-main"
        className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-lg focus:bg-surface focus:px-3 focus:py-2 focus:text-sm focus:text-foreground"
      >
        Skip to content
      </a>

      <aside className="fixed inset-y-0 left-0 z-40 hidden w-64 flex-col border-r border-border bg-surface/50 lg:flex">
        <AdminBrand />
        <SidebarNav />
        <div className="border-t border-border p-3">
          <Link
            to="/"
            className="flex items-center gap-2 rounded-xl px-3 py-2 text-xs text-muted-foreground transition-colors hover:bg-surface-soft hover:text-foreground"
          >
            <ArrowLeft className="size-3.5" aria-hidden="true" />
            Back to website
          </Link>
          <p className="mt-2 px-3 font-mono text-[9px] uppercase tracking-[0.18em] text-muted-foreground/70">
            Session secured · role enforced
          </p>
        </div>
      </aside>

      {mobileOpen ? (
        <div className="fixed inset-0 z-50 lg:hidden">
          <button
            type="button"
            aria-label="Close navigation"
            className="absolute inset-0 bg-[#040a12]/75"
            onClick={() => setMobileOpen(false)}
          />
          <div
            ref={mobileDrawerRef}
            id="admin-mobile-navigation"
            role="dialog"
            aria-modal="true"
            aria-label="Administration navigation"
            className="relative flex h-full w-72 max-w-[85vw] flex-col border-r border-border bg-surface"
          >
            <div className="flex items-center justify-between border-b border-border pr-2">
              <AdminBrand />
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label="Close navigation"
                onClick={() => setMobileOpen(false)}
              >
                <X className="size-4" />
              </Button>
            </div>
            <SidebarNav onNavigate={() => setMobileOpen(false)} />
            <div className="border-t border-border p-3">
              <Link
                to="/"
                className="flex items-center gap-2 rounded-xl px-3 py-2 text-xs text-muted-foreground hover:text-foreground"
              >
                <ArrowLeft className="size-3.5" aria-hidden="true" />
                Back to website
              </Link>
            </div>
          </div>
        </div>
      ) : null}

      <div className="lg:pl-64">
        <header className="sticky top-0 z-30 flex h-16 items-center gap-3 border-b border-border bg-background/85 px-4 backdrop-blur-xl sm:px-6">
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            className="lg:hidden"
            ref={mobileMenuButtonRef}
            aria-label="Open navigation"
            aria-expanded={mobileOpen}
            aria-controls="admin-mobile-navigation"
            onClick={() => setMobileOpen(true)}
          >
            <Menu className="size-4" />
          </Button>

          <nav aria-label="Breadcrumb" className="min-w-0">
            <ol className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <li>
                <Link to="/admin" className="rounded transition-colors hover:text-foreground">
                  Admin
                </Link>
              </li>
              {crumbs.map((crumb, index) => (
                <li key={`${crumb}-${index}`} className="flex min-w-0 items-center gap-1.5">
                  <ChevronRight className="size-3 shrink-0 opacity-60" aria-hidden="true" />
                  <span
                    aria-current={index === crumbs.length - 1 ? 'page' : undefined}
                    className={cn(
                      'truncate',
                      index === crumbs.length - 1
                        ? 'font-medium text-foreground'
                        : 'text-muted-foreground',
                    )}
                  >
                    {crumb}
                  </span>
                </li>
              ))}
            </ol>
          </nav>

          <div className="ml-auto flex items-center gap-2">
            <span className="hidden items-center gap-1.5 rounded-full border border-primary/25 bg-primary/5 px-2.5 py-1 font-mono text-[9px] uppercase tracking-[0.16em] text-primary sm:inline-flex">
              <ShieldCheck className="size-3" aria-hidden="true" />
              Administrator
            </span>
            <ThemeToggle />
            <AccountMenu />
          </div>
        </header>

        <main
          id="admin-main"
          className="mx-auto w-full max-w-7xl px-4 py-6 sm:px-6 sm:py-8 lg:px-8"
        >
          <motion.div
            key={pathname}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.22, ease: 'easeOut' }}
          >
            <Outlet />
          </motion.div>
        </main>
      </div>
    </div>
  );
}
