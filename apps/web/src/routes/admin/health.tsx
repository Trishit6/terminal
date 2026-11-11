import {
  Activity,
  Cpu,
  Database,
  Loader2,
  RefreshCw,
  Server,
  Timer,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { describeAuthError } from '@/lib/auth';
import { adminHealth, type HealthResponse } from '@/lib/admin';
import {
  EmptyState,
  ErrorNotice,
  LoadingPanel,
  PageHeader,
  ServiceStatusBadge,
  formatDateTime,
  formatUptime,
  statusLabel,
} from '@/components/admin/admin-ui';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

const REFRESH_MS = 15_000;

const BANNER: Record<HealthResponse['status'], string> = {
  healthy: 'border-success/30 bg-success/5 text-success',
  degraded: 'border-warning/30 bg-warning/5 text-warning',
  unavailable: 'border-danger/30 bg-danger/5 text-danger',
  unknown: 'border-border bg-surface-soft text-muted-foreground',
};

export function AdminHealthPage() {
  const [data, setData] = useState<HealthResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async (quiet = false) => {
    if (!quiet) {
      setLoading(true);
    }
    setError(null);
    try {
      setData(await adminHealth());
    } catch (cause) {
      setError(describeAuthError(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      void load(true);
    }, REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [load]);

  return (
    <div className="space-y-6">
      <PageHeader
        title="System health"
        description="Real checks run by the backend against MariaDB, the schema, the WebSocket service, and the terminal binaries."
        actions={
          <Button type="button" variant="secondary" size="sm" onClick={() => void load()} disabled={loading}>
            {loading ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <RefreshCw className="size-3.5" />
            )}
            Run checks
          </Button>
        }
      />

      {error ? <ErrorNotice onRetry={() => void load()}>{error}</ErrorNotice> : null}
      {loading && !data ? <LoadingPanel label="Running health checks" /> : null}

      {data ? (
        <>
          <section
            aria-label="Overall status"
            className={cn('rounded-2xl border px-5 py-4', BANNER[data.status])}
          >
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-3">
                <Activity className="size-5" aria-hidden="true" />
                <div>
                  <p className="text-sm font-semibold">Overall: {statusLabel(data.status)}</p>
                  <p className="mt-0.5 text-[11px] opacity-80">
                    Generated {formatDateTime(data.generated_at)} · recommendations come from the
                    checks below, not from the page loading.
                  </p>
                </div>
              </div>
              <span className="font-mono text-[10px] uppercase tracking-[0.14em]">
                Auto-refresh 15s
              </span>
            </div>
          </section>

          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <div className="rounded-2xl border border-border bg-surface/60 p-4">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Server className="size-4" aria-hidden="true" />
                <p className="text-[10px] uppercase tracking-[0.12em]">Application</p>
              </div>
              <p className="mt-2 font-mono text-sm text-foreground">
                {data.server.product} v{data.server.version}
              </p>
              <p className="mt-1 text-[11px] text-muted-foreground">
                {data.server.os} · {data.server.arch}
              </p>
            </div>

            <div className="rounded-2xl border border-border bg-surface/60 p-4">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Timer className="size-4" aria-hidden="true" />
                <p className="text-[10px] uppercase tracking-[0.12em]">Uptime</p>
              </div>
              <p className="mt-2 font-mono text-sm text-foreground">
                {formatUptime(data.server.uptime_seconds)}
              </p>
              <p className="mt-1 text-[11px] text-muted-foreground">
                {data.server.started_at
                  ? `Started ${formatDateTime(data.server.started_at)}`
                  : 'Process start time unavailable'}
              </p>
            </div>

            <div className="rounded-2xl border border-border bg-surface/60 p-4">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Database className="size-4" aria-hidden="true" />
                <p className="text-[10px] uppercase tracking-[0.12em]">MariaDB</p>
              </div>
              <p className="mt-2 font-mono text-sm text-foreground">
                {data.database.latency_ms === undefined
                  ? 'No response'
                  : `${data.database.latency_ms} ms`}
              </p>
              <p className="mt-1 text-[11px] text-muted-foreground">
                {data.database.configured ? 'Accounts enabled' : 'Accounts not configured'}
              </p>
            </div>

            <div className="rounded-2xl border border-border bg-surface/60 p-4">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Cpu className="size-4" aria-hidden="true" />
                <p className="text-[10px] uppercase tracking-[0.12em]">Migrations</p>
              </div>
              <p className="mt-2 font-mono text-sm text-foreground">
                {data.database.migrations_applied ?? '—'} / {data.database.migrations_expected}
              </p>
              <p className="mt-1 text-[11px] text-muted-foreground">
                {data.database.migrations_latest === undefined
                  ? 'Latest version unknown'
                  : `Latest version ${data.database.migrations_latest}`}
              </p>
            </div>
          </div>

          <section className="overflow-hidden rounded-2xl border border-border bg-surface/50">
            <div className="border-b border-border px-5 py-4">
              <h2 className="text-sm font-semibold">Checks</h2>
              <p className="mt-0.5 text-[11px] text-muted-foreground">
                Each row is evaluated by the backend during this request.
              </p>
            </div>

            {data.checks.length === 0 ? (
              <div className="p-5">
                <EmptyState title="No checks ran" description="The backend returned no health checks." />
              </div>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full min-w-[680px] border-collapse text-left">
                  <caption className="sr-only">KERNEXT backend health checks</caption>
                  <thead>
                    <tr className="border-b border-border text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                      <th scope="col" className="px-5 py-3 font-medium">Check</th>
                      <th scope="col" className="px-5 py-3 font-medium">Status</th>
                      <th scope="col" className="px-5 py-3 font-medium">Detail</th>
                      <th scope="col" className="px-5 py-3 font-medium">Latency</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border">
                    {data.checks.map((check) => (
                      <tr key={check.id}>
                        <td className="px-5 py-3 text-xs font-medium text-foreground">
                          {check.label}
                        </td>
                        <td className="px-5 py-3">
                          <ServiceStatusBadge status={check.status} />
                        </td>
                        <td className="max-w-xl px-5 py-3 text-[11px] leading-relaxed text-muted-foreground">
                          {check.detail}
                        </td>
                        <td className="px-5 py-3 font-mono text-[11px] text-muted-foreground">
                          {check.latency_ms === undefined ? '—' : `${check.latency_ms} ms`}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>

          <p className="text-[11px] leading-relaxed text-muted-foreground">
            No credentials, environment variables, or host paths beyond the resolved terminal
            executable are returned by this endpoint.
          </p>
        </>
      ) : null}
    </div>
  );
}
