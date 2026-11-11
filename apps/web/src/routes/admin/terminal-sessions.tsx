import { Link } from '@tanstack/react-router';
import {
  Loader2,
  MonitorSmartphone,
  Power,
  RefreshCw,
  ShieldCheck,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { describeAuthError } from '@/lib/auth';
import {
  adminTerminalSessions,
  adminTerminateTerminal,
  type TerminalSession,
} from '@/lib/admin';
import {
  ConfirmDialog,
  EmptyState,
  ErrorNotice,
  LoadingPanel,
  ModeBadge,
  PageHeader,
  formatDateTime,
  formatRelative,
} from '@/components/admin/admin-ui';
import { Button } from '@/components/ui/button';

const REFRESH_MS = 10_000;

export function AdminTerminalSessionsPage() {
  const [sessions, setSessions] = useState<TerminalSession[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [target, setTarget] = useState<TerminalSession | null>(null);
  const [pending, setPending] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const load = useCallback(async (quiet = false) => {
    if (!quiet) {
      setLoading(true);
    }
    setError(null);
    try {
      const body = await adminTerminalSessions();
      setSessions(body.sessions);
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

  async function terminate() {
    if (!target) {
      return;
    }

    setPending(true);
    setActionError(null);
    try {
      await adminTerminateTerminal(target.id);
      setTarget(null);
      await load(true);
    } catch (cause) {
      setActionError(describeAuthError(cause));
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="Terminal sessions"
        description="Live engine and PTY connections owned by signed-in accounts. Command input and terminal output are never exposed here."
        actions={
          <Button
            type="button"
            variant="secondary"
            size="sm"
            onClick={() => void load()}
            disabled={loading}
          >
            {loading ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <RefreshCw className="size-3.5" />
            )}
            Refresh
          </Button>
        }
      />

      <div className="flex flex-wrap items-center gap-2 rounded-xl border border-primary/20 bg-primary/5 px-4 py-3 text-[11px] text-muted-foreground">
        <ShieldCheck className="size-3.5 shrink-0 text-primary" aria-hidden="true" />
        <span>
          Session isolation is enforced by the backend. Terminating a session closes only that
          connection; local CLI and offline desktop shells are not affected.
        </span>
      </div>

      {error ? <ErrorNotice onRetry={() => void load()}>{error}</ErrorNotice> : null}
      {actionError ? <ErrorNotice>{actionError}</ErrorNotice> : null}

      {loading && sessions === null ? <LoadingPanel label="Loading terminal sessions" /> : null}

      {sessions ? (
        <section className="overflow-hidden rounded-2xl border border-border bg-surface/50">
          <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border px-4 py-3">
            <p className="text-xs text-muted-foreground">
              {sessions.length === 0
                ? 'No live terminal sessions'
                : `${sessions.length} live session${sessions.length === 1 ? '' : 's'}`}
            </p>
            <p className="font-mono text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
              Auto-refresh 10s
            </p>
          </div>

          {sessions.length === 0 ? (
            <div className="p-5">
              <EmptyState
                title="No terminal sessions are open"
                description="Engine and PTY sessions appear here while an authenticated account has them open."
              />
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[860px] border-collapse text-left">
                <caption className="sr-only">
                  Live KERNEXT terminal sessions with owner, mode, platform, and activity
                </caption>
                <thead>
                  <tr className="border-b border-border text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                    <th scope="col" className="px-4 py-3 font-medium">Session</th>
                    <th scope="col" className="px-4 py-3 font-medium">Owner</th>
                    <th scope="col" className="px-4 py-3 font-medium">Mode</th>
                    <th scope="col" className="px-4 py-3 font-medium">Platform</th>
                    <th scope="col" className="px-4 py-3 font-medium">Created</th>
                    <th scope="col" className="px-4 py-3 font-medium">Last activity</th>
                    <th scope="col" className="px-4 py-3 font-medium">Status</th>
                    <th scope="col" className="px-4 py-3 text-right font-medium">
                      <span className="sr-only">Actions</span>
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border">
                  {sessions.map((session) => (
                    <tr key={session.id} className="transition-colors hover:bg-surface-soft/50">
                      <td className="px-4 py-3 font-mono text-[11px] text-foreground">
                        {session.id}
                      </td>
                      <td className="px-4 py-3">
                        {session.owner && session.owner_email ? (
                          <Link
                            to="/admin/users/$userId"
                            params={{ userId: session.owner }}
                            className="block max-w-52"
                          >
                            <span className="block truncate text-xs font-medium text-foreground hover:text-primary">
                              {session.owner_name ?? session.owner_email}
                            </span>
                            <span className="block truncate text-[10px] text-muted-foreground">
                              {session.owner_email}
                            </span>
                          </Link>
                        ) : (
                          <span className="text-xs text-muted-foreground">
                            {session.owner_name ?? session.owner_email ?? 'Local session'}
                          </span>
                        )}
                      </td>
                      <td className="px-4 py-3">
                        <ModeBadge mode={session.mode} />
                      </td>
                      <td className="px-4 py-3 text-xs text-muted-foreground">
                        {session.platform}
                      </td>
                      <td className="px-4 py-3 text-xs text-muted-foreground">
                        <time dateTime={session.created_at} title={formatDateTime(session.created_at)}>
                          {formatRelative(session.created_at)}
                        </time>
                      </td>
                      <td className="px-4 py-3 text-xs text-muted-foreground">
                        <time
                          dateTime={session.last_active_at}
                          title={formatDateTime(session.last_active_at)}
                        >
                          {formatRelative(session.last_active_at)}
                        </time>
                      </td>
                      <td className="px-4 py-3">
                        <span className="inline-flex items-center gap-1.5 text-xs capitalize text-success">
                          <span className="size-1.5 rounded-full bg-success" aria-hidden="true" />
                          {session.status}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-right">
                        <Button
                          type="button"
                          variant="destructive"
                          size="sm"
                          onClick={() => setTarget(session)}
                        >
                          <Power className="size-3.5" />
                          Terminate
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      ) : null}

      <p className="flex items-center gap-2 text-[11px] text-muted-foreground">
        <MonitorSmartphone className="size-3.5" aria-hidden="true" />
        Only session metadata is shown by design. Administrators cannot impersonate an account or
        read its terminal transcript.
      </p>

      <ConfirmDialog
        open={target !== null}
        title="Terminate this terminal session?"
        description={
          target
            ? `${target.id} owned by ${target.owner_name ?? target.owner_email ?? 'a local session'} will be closed immediately. The account remains signed in to the website.`
            : ''
        }
        confirmLabel="Terminate session"
        pending={pending}
        onConfirm={() => void terminate()}
        onClose={() => setTarget(null)}
      />
    </div>
  );
}
