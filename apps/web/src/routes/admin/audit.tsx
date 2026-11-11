import {
  ChevronLeft,
  ChevronRight,
  Filter,
  Loader2,
  RefreshCw,
  ScrollText,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { describeAuthError } from '@/lib/auth';
import { adminAuditLogs, type AuditPage } from '@/lib/admin';
import {
  EmptyState,
  ErrorNotice,
  LoadingPanel,
  PageHeader,
  eventLabel,
  formatDateTime,
} from '@/components/admin/admin-ui';
import { Button } from '@/components/ui/button';

const PER_PAGE = 25;

export function AdminAuditPage() {
  const [category, setCategory] = useState('all');
  const [eventType, setEventType] = useState('');
  const [appliedEventType, setAppliedEventType] = useState('');
  const [page, setPage] = useState(1);

  const [data, setData] = useState<AuditPage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setData(
        await adminAuditLogs({
          category,
          event_type: appliedEventType,
          page,
          per_page: PER_PAGE,
        }),
      );
    } catch (cause) {
      setError(describeAuthError(cause));
    } finally {
      setLoading(false);
    }
  }, [category, appliedEventType, page]);

  useEffect(() => {
    void load();
  }, [load]);

  const total = data?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / PER_PAGE));
  const first = total === 0 ? 0 : (page - 1) * PER_PAGE + 1;
  const last = Math.min(page * PER_PAGE, total);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Audit log"
        description="Persisted security and administrative events. Passwords, session tokens, and terminal input are never recorded."
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

      <form
        onSubmit={(event) => {
          event.preventDefault();
          setPage(1);
          setAppliedEventType(eventType.trim().toLowerCase());
        }}
        className="flex flex-col gap-3 rounded-2xl border border-border bg-surface/50 p-3 sm:flex-row sm:items-end"
      >
        <label className="block space-y-1.5">
          <span className="text-[10px] font-medium uppercase tracking-[0.12em] text-muted-foreground">
            Category
          </span>
          <select
            value={category}
            onChange={(event) => {
              setCategory(event.target.value);
              setPage(1);
            }}
            className="h-10 w-full rounded-xl border border-border bg-background/60 px-3 text-sm outline-none focus:border-primary/60 focus:ring-2 focus:ring-primary/15 sm:w-44"
          >
            <option value="all">All categories</option>
            <option value="auth">Authentication</option>
            <option value="password">Passwords</option>
            <option value="user">Users</option>
            <option value="session">Account sessions</option>
            <option value="system">System</option>
          </select>
        </label>

        <label className="flex-1 space-y-1.5">
          <span className="text-[10px] font-medium uppercase tracking-[0.12em] text-muted-foreground">
            Exact event type (optional)
          </span>
          <input
            type="text"
            value={eventType}
            onChange={(event) => setEventType(event.target.value)}
            placeholder="user.role_changed"
            className="h-10 w-full rounded-xl border border-border bg-background/60 px-3 font-mono text-xs outline-none placeholder:text-muted-foreground focus:border-primary/60 focus:ring-2 focus:ring-primary/15"
          />
        </label>

        <Button type="submit" variant="secondary" size="sm" className="h-10">
          <Filter className="size-3.5" />
          Apply filter
        </Button>
      </form>

      {loading && !data ? <LoadingPanel label="Loading audit log" /> : null}

      {data ? (
        <section className="overflow-hidden rounded-2xl border border-border bg-surface/50">
          <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border px-4 py-3">
            <p className="text-xs text-muted-foreground">
              {total === 0 ? 'No events' : `Showing ${first}–${last} of ${total}`}
            </p>
            <p className="font-mono text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
              Page {page} of {totalPages}
            </p>
          </div>

          {data.events.length === 0 ? (
            <div className="p-5">
              <EmptyState
                title="No audit events match this filter"
                description="Events are written as sign-ins, account changes, and session actions happen."
              />
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[900px] border-collapse text-left">
                <caption className="sr-only">
                  Administrative and security events with actor, target, and timestamp
                </caption>
                <thead>
                  <tr className="border-b border-border text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                    <th scope="col" className="px-4 py-3 font-medium">Time</th>
                    <th scope="col" className="px-4 py-3 font-medium">Event</th>
                    <th scope="col" className="px-4 py-3 font-medium">Actor</th>
                    <th scope="col" className="px-4 py-3 font-medium">Target</th>
                    <th scope="col" className="px-4 py-3 font-medium">Address</th>
                    <th scope="col" className="px-4 py-3 font-medium">Metadata</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border">
                  {data.events.map((event) => (
                    <tr key={event.id} className="align-top transition-colors hover:bg-surface-soft/40">
                      <td className="whitespace-nowrap px-4 py-3 font-mono text-[10px] text-muted-foreground">
                        <time dateTime={event.created_at}>{formatDateTime(event.created_at)}</time>
                      </td>
                      <td className="px-4 py-3">
                        <span className="block text-xs font-medium text-foreground">
                          {eventLabel(event.event_type)}
                        </span>
                        <code className="mt-0.5 block font-mono text-[10px] text-muted-foreground">
                          {event.event_type}
                        </code>
                      </td>
                      <td className="px-4 py-3 text-[11px] text-muted-foreground">
                        {event.actor_email ?? 'System'}
                      </td>
                      <td className="px-4 py-3 text-[11px] text-muted-foreground">
                        {event.target_label ?? '—'}
                      </td>
                      <td className="px-4 py-3 font-mono text-[10px] text-muted-foreground">
                        {event.ip_address ?? '—'}
                      </td>
                      <td className="px-4 py-3">
                        {event.metadata ? (
                          <details className="max-w-xs">
                            <summary className="cursor-pointer text-[11px] text-primary">
                              Details
                            </summary>
                            <pre className="mt-1 max-h-40 overflow-auto rounded-lg border border-border bg-background/60 p-2 font-mono text-[10px] leading-relaxed text-muted-foreground">
                              {JSON.stringify(event.metadata, null, 2)}
                            </pre>
                          </details>
                        ) : (
                          <span className="text-[11px] text-muted-foreground">—</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          <div className="flex items-center justify-between gap-2 border-t border-border px-4 py-3">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => setPage((value) => Math.max(1, value - 1))}
              disabled={page <= 1 || loading}
            >
              <ChevronLeft className="size-3.5" />
              Previous
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => setPage((value) => Math.min(totalPages, value + 1))}
              disabled={page >= totalPages || loading}
            >
              Next
              <ChevronRight className="size-3.5" />
            </Button>
          </div>
        </section>
      ) : null}

      <p className="flex items-center gap-2 text-[11px] text-muted-foreground">
        <ScrollText className="size-3.5" aria-hidden="true" />
        Audit rows outlive account deletion; the table deliberately has no foreign keys.
      </p>
    </div>
  );
}
