import { useEffect, useState } from 'react';
import { Link, Outlet, useRouterState } from '@tanstack/react-router';
import {
  Command,
  FolderOpen,
  Home,
  Info,
  Layers,
  Minus,
  PanelLeft,
  Settings,
  Square,
  Terminal,
  X,
} from 'lucide-react';
import { LogoMark } from '@/components/site/logo';
import { DesktopStartupAnimation } from '@/components/desktop/startup-animation';
import { desktopSettingsGet, type DesktopSettings } from '@/lib/desktop';
import { closeWindow, minimizeWindow, toggleMaximizeWindow } from '@/lib/window';
import { cn } from '@/lib/utils';

const NAV_ITEMS = [
  { to: '/desktop', label: 'Home', icon: Home, exact: true },
  { to: '/desktop/terminal', label: 'Terminal', icon: Terminal, exact: false },
  { to: '/desktop/commands', label: 'Command Explorer', icon: Command, exact: false },
  { to: '/desktop/files', label: 'File Explorer', icon: FolderOpen, exact: false },
  { to: '/desktop/sessions', label: 'Sessions', icon: Layers, exact: false },
  { to: '/desktop/settings', label: 'Settings', icon: Settings, exact: false },
  { to: '/desktop/about', label: 'About KERNEXT', icon: Info, exact: false },
] as const;

export function DesktopShell() {
  const [settings, setSettings] = useState<DesktopSettings | null>(null);
  const [showStartup, setShowStartup] = useState(true);
  const [navigationOpen, setNavigationOpen] = useState(true);
  const pathname = useRouterState({ select: (state) => state.location.pathname });

  useEffect(() => {
    let cancelled = false;

    void desktopSettingsGet()
      .then((next) => {
        if (cancelled) {
          return;
        }

        setSettings(next);

        if (!next.startup_animation || next.reduce_motion) {
          setShowStartup(false);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setShowStartup(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!settings) {
      return;
    }

    const root = document.documentElement;
    const dark =
      settings.theme === 'midnight' ||
      (settings.theme === 'system' &&
        window.matchMedia('(prefers-color-scheme: dark)').matches);
    root.classList.toggle('dark', dark);
  }, [settings]);

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
      <DesktopStartupAnimation visible={showStartup} />

      <header
        data-tauri-drag-region
        className="relative z-20 flex h-11 shrink-0 select-none items-center justify-between border-b border-border bg-surface/90 pl-3 pr-1 backdrop-blur"
      >
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => setNavigationOpen((open) => !open)}
            className="grid size-7 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-surface-soft hover:text-foreground"
            aria-label="Toggle navigation"
          >
            <PanelLeft className="size-4" />
          </button>
          <LogoMark className="size-5" />
          <span className="font-mono text-xs font-medium uppercase tracking-[0.24em] text-foreground/80">
            kernext
          </span>
          <span className="ml-2 hidden font-mono text-[10px] uppercase tracking-[0.18em] text-muted-foreground sm:inline">
            the next core of computing
          </span>
        </div>

        <div className="flex items-center gap-0.5">
          <TitleBarButton onClick={() => void minimizeWindow()} label="Minimize">
            <Minus className="size-3.5" />
          </TitleBarButton>
          <TitleBarButton onClick={() => void toggleMaximizeWindow()} label="Maximize">
            <Square className="size-3" />
          </TitleBarButton>
          <TitleBarButton onClick={() => void closeWindow()} label="Close" danger>
            <X className="size-4" />
          </TitleBarButton>
        </div>
      </header>

      <div className="flex min-h-0 flex-1">
        <nav
          className={cn(
            'relative z-10 flex shrink-0 flex-col justify-between border-r border-border bg-surface/70 backdrop-blur transition-all',
            navigationOpen ? 'w-56' : 'w-14',
          )}
        >
          <div className="flex flex-col gap-1 p-2">
            {NAV_ITEMS.map((item) => {
              const active = item.exact
                ? pathname === item.to
                : pathname.startsWith(item.to);

              return (
                <Link
                  key={item.to}
                  to={item.to}
                  className={cn(
                    'group flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors',
                    active
                      ? 'bg-primary/10 text-foreground'
                      : 'text-muted-foreground hover:bg-surface-soft hover:text-foreground',
                    !navigationOpen && 'justify-center px-0',
                  )}
                  title={item.label}
                >
                  <item.icon
                    className={cn(
                      'size-4 shrink-0',
                      active ? 'text-primary' : 'text-muted-foreground',
                    )}
                  />
                  {navigationOpen ? <span>{item.label}</span> : null}
                </Link>
              );
            })}
          </div>

          <div className="p-2">
            <div className="flex items-center gap-2 rounded-lg px-2 py-2 text-[10px] font-mono uppercase tracking-[0.16em] text-muted-foreground">
              <span className="size-1.5 rounded-full bg-success" />
              {navigationOpen ? 'local runtime' : null}
            </div>
          </div>
        </nav>

        <main className="relative min-h-0 flex-1 overflow-y-auto">
          <Outlet />
        </main>
      </div>
    </div>
  );
}

function TitleBarButton({
  children,
  onClick,
  label,
  danger = false,
}: {
  children: React.ReactNode;
  onClick: () => void;
  label: string;
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={label}
      className={cn(
        'grid size-8 place-items-center rounded-md text-muted-foreground transition-colors',
        danger
          ? 'hover:bg-danger hover:text-white'
          : 'hover:bg-surface-soft hover:text-foreground',
      )}
    >
      {children}
    </button>
  );
}
