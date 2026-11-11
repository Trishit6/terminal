import { Link } from '@tanstack/react-router';
import {
  Activity,
  Database,
  Loader2,
  RefreshCw,
  ShieldCheck,
  TerminalSquare,
  UserCheck,
  Users,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { describeAuthError } from '@/lib/auth';
import {
  adminOverview,
  type AuditEntry,
  type OverviewResponse,
} from '@/lib/admin';
import {
  EmptyState,
  ErrorNotice,
  LoadingPanel,
  PageHeader,
  ServiceStatusBadge,
  eventLabel,
  formatRelative,
  initials,
} from '@/components/admin/admin-ui';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

function EventList({
  events,
  emptyTitle,
  emptyDescription,
}: {
  events: AuditEntry[];
  emptyTitle: string;
  emptyDescription: string;
}) {
  if (events.length === 0) {
    return <EmptyState title={emptyTitle} description={emptyDescription} />;
  }

  return (
    <ul className="divide-y divide-border">
      {events.map((event) => (
        <li key={event.id} className="flex items-start gap-3 px-5 py-3.5">
          <span className="mt-1 size-1.5 shrink-0 rounded-full bg-primary/70" aria-hidden="true" />
          <div className="min-w-0 flex-1">
            <p className="text-xs font-medium text-foreground">{eventLabel(event.event_type)}</p>
            <p className="mt-0.5 truncate text-[11px] text-muted-foreground">
              {event.actor_email ?? 'System'}
              {event.target_label ? ` → ${event.target_label}` : ''}
            </p>
          </div>
          <time
            dateTime={event.created_at}
            className="shrink-0 font-mono text-[10px] text-muted-foreground"
          >
            {formatRelative(event.created_at)}
          </time>
        </li>
      ))}
    </ul>
  );
}

export function AdminOverviewPage() {
  const [data, setData] = useState<OverviewResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setData(await adminOverview());
    } catch (cause) {
      setError(describeAuthError(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const totals = data?.totals;

  const cards = [
    {
      label: 'Registered users',
      value: totals?.total_users,
      detail: 'All accounts in MariaDB',
      icon: Users,
    },
    {
      label: 'Active users',
      value: totals?.active_users,
      detail: 'Accounts allowed to sign in',
      icon: UserCheck,
    },
    {
      label: 'Administrators',
      value: totals?.administrators,
      detail: 'Active accounts with the admin role',
      icon: ShieldCheck,
    },
    {
      label: 'Account sessions',
      value: totals?.active_sessions,
      detail: 'Live authenticated browser sessions',
      icon: Database,
    },
    {
      label: 'Terminal sessions',
      value: totals?.active_terminal_sessions,
      detail: 'Live engine and PTY terminals',
      icon: TerminalSquare,
    },
  ];

  return (
    <div className="space-y-6">
      <PageHeader
        title="Overview"
        description="Live account, session, and service state from the Rust backend and MariaDB."
        actions={
          <Button type="button" variant="secondary" size="sm" onClick={() => void load()} disabled={loading}>
            {loading ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <RefreshCw className="size-3.5" />
            )}
            Refresh
          </Button>
        }
      />

      {error ? <ErrorNotice onRetry={() => void load()}>{error}</ErrorNotice> : null}

      {loading && !data ? <LoadingPanel label="Loading dashboard" /> : null}

      {data ? (
        <>
          <section aria-label="Account totals">
            <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
              {cards.map((card) => {
                const Icon = card.icon;
                return (
                  <div
                    key={card.label}
                    className="rounded-2xl border border-border bg-surface/60 p-4"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <p className="text-[11px] font-medium uppercase tracking-[0.12em] text-muted-foreground">
                        {card.label}
                      </p>
                      <Icon className="size-4 text-primary/80" aria-hidden="true" />
                    </div>
                    <p className="mt-3 font-mono text-2xl font-semibold tracking-tight text-foreground">
                      {card.value ?? '—'}
                    </p>
                    <p className="mt-1 text-[11px] leading-relaxed text-muted-foreground">
                      {card.detail}
                    </p>
                  </div>
                );
              })}
            </div>
          </section>

          <section
            aria-label="Service status"
            className="rounded-2xl border border-border bg-surface/60 p-4"
          >
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-2">
                <Activity className="size-4 text-primary" aria-hidden="true" />
                <h2 className="text-sm font-semibold">Backend services</h2>
              </div>
              <span className="font-mono text-[10px] text-muted-foreground">
                Generated {formatRelative(data.generated_at)} · KERNEXT v{data.server.version}
              </span>
            </div>
            <ul className="mt-3 grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
              {data.services.map((service) => (
                <li
                  key={service.id}
                  className="flex items-center justify-between gap-3 rounded-xl border border-border bg-background/40 px-3 py-2.5"
                >
                  <span className="text-xs text-foreground">{service.label}</span>
                  <ServiceStatusBadge status={service.status} />
                </li>
              ))}
            </ul>
          </section>

          <div className="grid gap-4 xl:grid-cols-2">
            <section className="overflow-hidden rounded-2xl border border-border bg-surface/60">
              <div className="flex items-center justify-between gap-3 border-b border-border px-5 py-4">
                <div>
                  <h2 className="text-sm font-semibold">Latest registrations</h2>
                  <p className="mt-0.5 text-[11px] text-muted-foreground">
                    Newest accounts, straight from the database
                  </p>
                </div>
                <Button asChild variant="ghost" size="sm">
                  <Link to="/admin/users">All users</Link>
                </Button>
              </div>

              {data.latest_registrations.length === 0 ? (
                <div className="p-5">
                  <EmptyState
                    title="No accounts yet"
                    description="Registered accounts will appear here."
                  />
                </div>
              ) : (
                <ul className="divide-y divide-border">
                  {data.latest_registrations.map((user) => (
                    <li key={user.id}>
                      <Link
                        to="/admin/users/$userId"
                        params={{ userId: user.id }}
                        className="flex items-center gap-3 px-5 py-3 transition-colors hover:bg-surface-soft/60"
                      >
                        <span className="grid size-8 shrink-0 place-items-center rounded-lg border border-border bg-surface-soft font-mono text-[10px] font-semibold text-primary">
                          {initials(user.full_name)}
                        </span>
                        <span className="min-w-0 flex-1">
                          <span className="block truncate text-xs font-medium text-foreground">
                            {user.full_name}
                          </span>
                          <span className="block truncate text-[11px] text-muted-foreground">
                            {user.email}
                          </span>
                        </span>
                        <span className="flex shrink-0 items-center gap-2">
                          {user.role === 'admin' ? (
                            <Badge variant="default">Admin</Badge>
                          ) : null}
                          <time
                            dateTime={user.created_at}
                            className="font-mono text-[10px] text-muted-foreground"
                          >
                            {formatRelative(user.created_at)}
                          </time>
                        </span>
                      </Link>
                    </li>
                  ))}
                </ul>
              )}
            </section>

            <section className="overflow-hidden rounded-2xl border border-border bg-surface/60">
              <div className="border-b border-border px-5 py-4">
                <h2 className="text-sm font-semibold">Recent security events</h2>
                <p className="mt-0.5 text-[11px] text-muted-foreground">
                  Sign-ins, failed attempts, and password activity
                </p>
              </div>
              <EventList
                events={data.recent_security_events}
                emptyTitle="No security events yet"
                emptyDescription="Successful and failed sign-ins are recorded in the audit log."
              />
            </section>

            <section className="overflow-hidden rounded-2xl border border-border bg-surface/60 xl:col-span-2">
              <div className="flex items-center justify-between gap-3 border-b border-border px-5 py-4">
                <div>
                  <h2 className="text-sm font-semibold">Recent administrative and system events</h2>
                  <p className="mt-0.5 text-[11px] text-muted-foreground">
                    Role changes, account status, session revocation, and server startup
                  </p>
                </div>
                <Button asChild variant="ghost" size="sm">
                  <Link to="/admin/audit">Open audit log</Link>
                </Button>
              </div>
              <EventList
                events={data.recent_system_events}
                emptyTitle="No administrative events yet"
                emptyDescription="Administrator actions are recorded here as they happen."
              />
            </section>
          </div>
        </>
      ) : null}
    </div>
  );
}
