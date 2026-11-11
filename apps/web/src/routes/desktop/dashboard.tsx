import { useCallback, useEffect, useState } from 'react';
import { Link } from '@tanstack/react-router';
import {
  Activity,
  Clock,
  Cpu,
  FolderOpen,
  Gauge,
  HardDrive,
  Layers,
  Monitor,
  RefreshCw,
  Terminal,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { desktopDashboard, type DesktopDashboard as Dashboard } from '@/lib/desktop';
import { useAuth } from '@/components/auth/auth-provider';

export function DesktopDashboardPage() {
  const { signedIn, user, accountsEnabled } = useAuth();
  const [dashboard, setDashboard] = useState<Dashboard | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const next = await desktopDashboard();
      setDashboard(next);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const quickActions = [
    {
      to: '/desktop/terminal',
      icon: Terminal,
      title: 'Open terminal',
      description: 'Start a new PTY or engine session.',
    },
    {
      to: '/desktop/commands',
      icon: Monitor,
      title: 'Command explorer',
      description: 'Browse built-ins and installed programs.',
    },
    {
      to: '/desktop/files',
      icon: FolderOpen,
      title: 'File explorer',
      description: 'Browse, inspect, and manage files.',
    },
    {
      to: '/desktop/sessions',
      icon: Layers,
      title: 'Sessions',
      description: 'Review every live engine and PTY session.',
    },
  ];

  return (
    <div className="mx-auto w-full max-w-[1400px] px-6 py-8 lg:px-10">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="font-mono text-[10px] uppercase tracking-[0.22em] text-primary">
            Dashboard
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
            KERNEXT runtime
          </h1>
          <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
            Live values from the local Rust runtime. Nothing on this page is
            simulated.
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={() => void load()} className="gap-2">
          <RefreshCw className={loading ? 'size-3.5 animate-spin' : 'size-3.5'} />
          Refresh
        </Button>
      </div>

      {error ? (
        <div className="mt-6 rounded-lg border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger">
          {error}
        </div>
      ) : null}

      <div className="mt-6 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border bg-surface/60 px-4 py-3">
        <div className="flex items-center gap-3">
          <span
            className={
              signedIn
                ? 'size-2 rounded-full bg-success'
                : 'size-2 rounded-full bg-muted-foreground/50'
            }
          />
          <div>
            <p className="text-xs font-medium">
              {signedIn && user
                ? `Signed in as ${user.full_name}`
                : 'Local mode — shell features work without an account'}
            </p>
            <p className="mt-0.5 text-[11px] text-muted-foreground">
              {signedIn && user
                ? `${user.email} · account features enabled`
                : accountsEnabled
                  ? 'Sign in only for account and remote features; local terminals are unaffected.'
                  : 'The account server is not configured. Local shell remains fully available.'}
            </p>
          </div>
        </div>
        <Button variant="outline" size="sm" asChild>
          <Link to="/desktop/account">
            {signedIn ? 'Manage account' : 'Account'}
          </Link>
        </Button>
      </div>

      <div className="mt-8 grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          icon={Cpu}
          label="Version"
          value={dashboard ? `v${dashboard.version}` : '—'}
          description="Workspace version reported by shell-core."
        />
        <MetricCard
          icon={HardDrive}
          label="Platform"
          value={dashboard?.platform ?? '—'}
          description="Operating system and architecture."
        />
        <MetricCard
          icon={FolderOpen}
          label="Working directory"
          value={dashboard?.working_directory ?? '—'}
          description="Directory used for new sessions."
          mono
        />
        <MetricCard
          icon={Activity}
          label="Live sessions"
          value={dashboard ? String(dashboard.sessions.length) : '—'}
          description="Engine and PTY sessions combined."
        />
      </div>

      <div className="mt-6 grid gap-4 lg:grid-cols-[1.4fr_1fr]">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Clock className="size-4 text-primary" />
              Recent commands
            </CardTitle>
            <CardDescription>
              The most recent command lines executed through the desktop runtime.
            </CardDescription>
          </CardHeader>
          <CardContent>
            {dashboard && dashboard.recent_commands.length > 0 ? (
              <div className="divide-y divide-border">
                {dashboard.recent_commands.map((item, index) => (
                  <div
                    key={`${item.timestamp}-${index}`}
                    className="flex items-center justify-between gap-4 py-2.5"
                  >
                    <code className="truncate font-mono text-xs text-foreground">
                      {item.command}
                    </code>
                    <Badge
                      variant={
                        item.exit_code === 0 ? 'success' : item.exit_code ? 'danger' : 'secondary'
                      }
                    >
                      {item.exit_code === null || item.exit_code === undefined
                        ? 'running'
                        : `exit ${item.exit_code}`}
                    </Badge>
                  </div>
                ))}
              </div>
            ) : (
              <p className="py-6 text-center text-sm text-muted-foreground">
                No commands have been run in this desktop session yet.
              </p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Gauge className="size-4 text-primary" />
              Runtime
            </CardTitle>
            <CardDescription>Local process state.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">Uptime</span>
              <span className="font-mono">
                {dashboard ? `${dashboard.uptime_seconds}s` : '—'}
              </span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">PTY transport</span>
              <Badge variant={dashboard?.pty_available ? 'success' : 'warning'}>
                {dashboard?.pty_available ? 'available' : 'unavailable'}
              </Badge>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">Engine sessions</span>
              <span className="font-mono">
                {dashboard
                  ? dashboard.sessions.filter((session) => session.kind === 'engine').length
                  : '—'}
              </span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">PTY sessions</span>
              <span className="font-mono">
                {dashboard
                  ? dashboard.sessions.filter((session) => session.kind === 'pty').length
                  : '—'}
              </span>
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="mt-6 grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {quickActions.map((action) => (
          <Link
            key={action.to}
            to={action.to}
            className="group rounded-2xl border border-border bg-surface/60 p-5 transition-colors hover:border-primary/40 hover:bg-surface"
          >
            <div className="grid size-9 place-items-center rounded-xl border border-border bg-background/70 text-primary">
              <action.icon className="size-4" />
            </div>
            <div className="mt-3 text-sm font-medium">{action.title}</div>
            <p className="mt-1 text-xs text-muted-foreground">{action.description}</p>
          </Link>
        ))}
      </div>
    </div>
  );
}

function MetricCard({
  icon: Icon,
  label,
  value,
  description,
  mono = false,
}: {
  icon: typeof Cpu;
  label: string;
  value: string;
  description: string;
  mono?: boolean;
}) {
  return (
    <Card>
      <CardHeader className="gap-2">
        <div className="flex items-center gap-2">
          <div className="grid size-8 place-items-center rounded-lg border border-border bg-background/70 text-primary">
            <Icon className="size-4" />
          </div>
          <CardTitle className="text-sm font-medium">{label}</CardTitle>
        </div>
      </CardHeader>
      <CardContent>
        <div
          className={
            mono
              ? 'truncate font-mono text-lg font-semibold'
              : 'text-lg font-semibold'
          }
          title={value}
        >
          {value}
        </div>
        <p className="mt-1 text-xs text-muted-foreground">{description}</p>
      </CardContent>
    </Card>
  );
}
