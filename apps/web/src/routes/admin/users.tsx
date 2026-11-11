import { Link } from '@tanstack/react-router';
import {
  ChevronLeft,
  ChevronRight,
  Loader2,
  RefreshCw,
  Search,
  Users,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { describeAuthError } from '@/lib/auth';
import { adminUsers, type UserPage } from '@/lib/admin';
import {
  AccountStatusBadge,
  EmptyState,
  ErrorNotice,
  LoadingPanel,
  PageHeader,
  RoleBadge,
  formatDateTime,
  formatRelative,
  initials,
} from '@/components/admin/admin-ui';
import { Button } from '@/components/ui/button';

const PER_PAGE = 25;

export function AdminUsersPage() {
  const [search, setSearch] = useState('');
  const [debouncedSearch, setDebouncedSearch] = useState('');
  const [role, setRole] = useState('all');
  const [status, setStatus] = useState('all');
  const [page, setPage] = useState(1);

  const [data, setData] = useState<UserPage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(search.trim());
      setPage(1);
    }, 300);
    return () => window.clearTimeout(timer);
  }, [search]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setData(
        await adminUsers({
          search: debouncedSearch || undefined,
          role,
          status,
          page,
          per_page: PER_PAGE,
        }),
      );
    } catch (cause) {
      setError(describeAuthError(cause));
    } finally {
      setLoading(false);
    }
  }, [debouncedSearch, role, status, page]);

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
        title="Users"
        description="Search accounts, inspect roles and status, and open an account to manage it."
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

      <div className="flex flex-col gap-3 rounded-2xl border border-border bg-surface/50 p-3 sm:flex-row sm:items-center">
        <label className="relative flex-1">
          <span className="sr-only">Search users by name or email</span>
          <Search
            className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden="true"
          />
          <input
            type="search"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search by name or email"
            className="h-10 w-full rounded-xl border border-border bg-background/60 pl-9 pr-3 text-sm outline-none transition-colors placeholder:text-muted-foreground focus:border-primary/60 focus:ring-2 focus:ring-primary/15"
          />
        </label>

        <div className="flex gap-2">
          <label className="flex-1 sm:flex-none">
            <span className="sr-only">Filter by role</span>
            <select
              value={role}
              onChange={(event) => {
                setRole(event.target.value);
                setPage(1);
              }}
              className="h-10 w-full rounded-xl border border-border bg-background/60 px-3 text-sm outline-none focus:border-primary/60 focus:ring-2 focus:ring-primary/15"
            >
              <option value="all">All roles</option>
              <option value="admin">Administrators</option>
              <option value="user">Members</option>
            </select>
          </label>

          <label className="flex-1 sm:flex-none">
            <span className="sr-only">Filter by account status</span>
            <select
              value={status}
              onChange={(event) => {
                setStatus(event.target.value);
                setPage(1);
              }}
              className="h-10 w-full rounded-xl border border-border bg-background/60 px-3 text-sm outline-none focus:border-primary/60 focus:ring-2 focus:ring-primary/15"
            >
              <option value="all">All statuses</option>
              <option value="active">Active</option>
              <option value="disabled">Disabled</option>
            </select>
          </label>
        </div>
      </div>

      {loading && !data ? <LoadingPanel label="Loading users" /> : null}

      {data ? (
        <section className="overflow-hidden rounded-2xl border border-border bg-surface/50">
          <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border px-4 py-3">
            <p className="text-xs text-muted-foreground">
              {total === 0 ? 'No accounts' : `Showing ${first}–${last} of ${total}`}
            </p>
            <p className="font-mono text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
              Page {page} of {totalPages}
            </p>
          </div>

          {data.users.length === 0 ? (
            <div className="p-5">
              <EmptyState
                title={debouncedSearch || role !== 'all' || status !== 'all' ? 'No accounts match these filters' : 'No accounts yet'}
                description={
                  debouncedSearch || role !== 'all' || status !== 'all'
                    ? 'Try a different search term, role, or status.'
                    : 'Accounts appear here after registration.'
                }
              />
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[760px] border-collapse text-left">
                <caption className="sr-only">
                  KERNEXT accounts with role, status, activity, and registration date
                </caption>
                <thead>
                  <tr className="border-b border-border text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                    <th scope="col" className="px-4 py-3 font-medium">Account</th>
                    <th scope="col" className="px-4 py-3 font-medium">Role</th>
                    <th scope="col" className="px-4 py-3 font-medium">Status</th>
                    <th scope="col" className="px-4 py-3 font-medium">Sessions</th>
                    <th scope="col" className="px-4 py-3 font-medium">Registered</th>
                    <th scope="col" className="px-4 py-3 font-medium">Last sign-in</th>
                    <th scope="col" className="px-4 py-3 text-right font-medium">
                      <span className="sr-only">Actions</span>
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border">
                  {data.users.map((user) => (
                    <tr key={user.id} className="transition-colors hover:bg-surface-soft/50">
                      <td className="px-4 py-3">
                        <div className="flex items-center gap-3">
                          <span className="grid size-8 shrink-0 place-items-center rounded-lg border border-border bg-surface-soft font-mono text-[10px] font-semibold text-primary">
                            {initials(user.full_name)}
                          </span>
                          <div className="min-w-0">
                            <Link
                              to="/admin/users/$userId"
                              params={{ userId: user.id }}
                              className="block truncate text-sm font-medium text-foreground hover:text-primary"
                            >
                              {user.full_name}
                            </Link>
                            <p className="truncate text-[11px] text-muted-foreground">
                              {user.email}
                              {user.email_verified ? '' : ' · unverified'}
                            </p>
                          </div>
                        </div>
                      </td>
                      <td className="px-4 py-3">
                        <RoleBadge role={user.role} />
                      </td>
                      <td className="px-4 py-3">
                        <AccountStatusBadge status={user.account_status} />
                      </td>
                      <td className="px-4 py-3 font-mono text-xs text-foreground">
                        {user.active_sessions}
                      </td>
                      <td className="px-4 py-3 text-xs text-muted-foreground">
                        <time dateTime={user.created_at}>{formatDateTime(user.created_at)}</time>
                      </td>
                      <td className="px-4 py-3 text-xs text-muted-foreground">
                        {user.last_login_at ? (
                          <time dateTime={user.last_login_at} title={formatDateTime(user.last_login_at)}>
                            {formatRelative(user.last_login_at)}
                          </time>
                        ) : (
                          <span>Never</span>
                        )}
                      </td>
                      <td className="px-4 py-3 text-right">
                        <Button asChild variant="outline" size="sm">
                          <Link to="/admin/users/$userId" params={{ userId: user.id }}>
                            Manage
                          </Link>
                        </Button>
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
      ) : !loading && !error ? (
        <EmptyState
          title="No accounts yet"
          description="Registered accounts will appear here."
        />
      ) : null}

      <p className="flex items-center gap-2 text-[11px] text-muted-foreground">
        <Users className="size-3.5" aria-hidden="true" />
        Role and status changes are enforced by the backend and written to the audit log.
      </p>
    </div>
  );
}
