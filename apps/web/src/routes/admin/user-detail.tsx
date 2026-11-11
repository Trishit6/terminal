import { Link, useParams } from '@tanstack/react-router';
import {
  ArrowLeft,
  KeyRound,
  Loader2,
  Mail,
  Monitor,
  Save,
  ShieldCheck,
  ShieldOff,
  UserRound,
  UserX,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { useAuth } from '@/components/auth/auth-provider';
import {
  AccountStatusBadge,
  ConfirmDialog,
  EmptyState,
  ErrorNotice,
  LoadingPanel,
  RoleBadge,
  formatDateTime,
  formatRelative,
  initials,
} from '@/components/admin/admin-ui';
import { Button } from '@/components/ui/button';
import { describeAuthError } from '@/lib/auth';
import { isValidEmail } from '@/lib/validation';
import {
  adminRevokeSessions,
  adminSetRole,
  adminSetStatus,
  adminUpdateUser,
  adminUser,
  type UserDetailResponse,
} from '@/lib/admin';

type ConfirmKind = 'promote' | 'demote' | 'disable' | 'enable' | 'revoke';

export function AdminUserDetailPage() {
  const { userId } = useParams({ from: '/admin/users/$userId' });
  const { user: actor } = useAuth();

  const [data, setData] = useState<UserDetailResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [fullName, setFullName] = useState('');
  const [email, setEmail] = useState('');
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const [confirm, setConfirm] = useState<ConfirmKind | null>(null);
  const [actionPending, setActionPending] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const next = await adminUser(userId);
      setData(next);
      setFullName(next.user.full_name);
      setEmail(next.user.email);
    } catch (cause) {
      setError(describeAuthError(cause));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load();
    // The user id is the only input that selects a different account.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [userId]);

  async function handleProfile(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaveError(null);
    setNotice(null);

    if (fullName.trim().length < 2) {
      setSaveError('Enter the account display name.');
      return;
    }
    if (!isValidEmail(email)) {
      setSaveError('Enter a valid email address.');
      return;
    }

    setSaving(true);
    try {
      await adminUpdateUser(userId, {
        full_name: fullName.trim(),
        email: email.trim(),
      });
      setNotice('Account information updated.');
      await load();
    } catch (cause) {
      setSaveError(describeAuthError(cause));
    } finally {
      setSaving(false);
    }
  }

  async function runConfirmedAction() {
    if (!confirm) {
      return;
    }

    setActionPending(true);
    setActionError(null);
    try {
      switch (confirm) {
        case 'promote':
          await adminSetRole(userId, 'admin');
          break;
        case 'demote':
          await adminSetRole(userId, 'user');
          break;
        case 'disable':
          await adminSetStatus(userId, 'disabled');
          break;
        case 'enable':
          await adminSetStatus(userId, 'active');
          break;
        case 'revoke':
          await adminRevokeSessions(userId);
          break;
      }
      setConfirm(null);
      setNotice('Account updated.');
      await load();
    } catch (cause) {
      setActionError(describeAuthError(cause));
      setConfirm(null);
    } finally {
      setActionPending(false);
    }
  }

  if (loading && !data) {
    return <LoadingPanel label="Loading account" />;
  }

  if (error && !data) {
    return (
      <div className="space-y-4">
        <Button asChild variant="ghost" size="sm">
          <Link to="/admin/users">
            <ArrowLeft className="size-3.5" />
            Back to users
          </Link>
        </Button>
        <ErrorNotice onRetry={() => void load()}>{error}</ErrorNotice>
      </div>
    );
  }

  if (!data) {
    return null;
  }

  const { user, sessions } = data;
  const isSelf = actor?.id === user.id;
  const confirmCopy: Record<ConfirmKind, { title: string; description: string; label: string }> = {
    promote: {
      title: 'Grant administrator access?',
      description: `${user.full_name} will be able to manage accounts, terminal sessions, and audit logs.`,
      label: 'Grant administrator',
    },
    demote: {
      title: 'Remove administrator access?',
      description: `${user.full_name} will lose access to the administration console. Existing sessions remain valid as a member.`,
      label: 'Remove administrator',
    },
    disable: {
      title: 'Deactivate this account?',
      description: `${user.full_name} will be signed out everywhere, lose access to the web terminal, and be unable to sign in until reactivated.`,
      label: 'Deactivate account',
    },
    enable: {
      title: 'Reactivate this account?',
      description: `${user.full_name} will be able to sign in again immediately.`,
      label: 'Activate account',
    },
    revoke: {
      title: 'Revoke every session?',
      description: `Every browser session for ${user.full_name} will end immediately. Terminal sessions are not closed by this action.`,
      label: 'Revoke sessions',
    },
  };

  return (
    <div className="space-y-6">
      <div>
        <Button asChild variant="ghost" size="sm">
          <Link to="/admin/users">
            <ArrowLeft className="size-3.5" />
            Back to users
          </Link>
        </Button>
      </div>

      <header className="flex flex-col gap-4 border-b border-border pb-6 sm:flex-row sm:items-start sm:justify-between">
        <div className="flex items-start gap-4">
          <span className="grid size-12 shrink-0 place-items-center rounded-2xl border border-border bg-surface font-mono text-sm font-semibold text-primary">
            {initials(user.full_name)}
          </span>
          <div className="min-w-0">
            <h1 className="truncate text-2xl font-semibold tracking-[-0.02em]">
              {user.full_name}
            </h1>
            <p className="mt-0.5 flex items-center gap-1.5 text-sm text-muted-foreground">
              <Mail className="size-3.5" aria-hidden="true" />
              <span className="truncate">{user.email}</span>
            </p>
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <RoleBadge role={user.role} />
              <AccountStatusBadge status={user.account_status} />
              {isSelf ? (
                <span className="rounded-full border border-border px-2 py-0.5 text-[10px] text-muted-foreground">
                  This is your account
                </span>
              ) : null}
            </div>
          </div>
        </div>
      </header>

      {error ? <ErrorNotice onRetry={() => void load()}>{error}</ErrorNotice> : null}
      {actionError ? <ErrorNotice>{actionError}</ErrorNotice> : null}
      {notice ? (
        <p
          role="status"
          className="rounded-xl border border-success/25 bg-success/5 px-4 py-3 text-sm text-success"
        >
          {notice}
        </p>
      ) : null}

      <div className="grid gap-4 xl:grid-cols-3">
        <section className="rounded-2xl border border-border bg-surface/60 p-5 xl:col-span-2">
          <div className="flex items-center gap-2">
            <UserRound className="size-4 text-primary" aria-hidden="true" />
            <h2 className="text-sm font-semibold">Account information</h2>
          </div>
          <p className="mt-1 text-[11px] leading-relaxed text-muted-foreground">
            Changing the email address clears its verification timestamp. Passwords are never
            shown or editable here.
          </p>

          <form onSubmit={handleProfile} className="mt-4 space-y-4" noValidate>
            {saveError ? <ErrorNotice>{saveError}</ErrorNotice> : null}

            <div className="grid gap-4 sm:grid-cols-2">
              <label className="block space-y-1.5">
                <span className="text-xs font-medium">Full name</span>
                <input
                  type="text"
                  value={fullName}
                  onChange={(event) => setFullName(event.target.value)}
                  autoComplete="name"
                  disabled={saving}
                  className="h-10 w-full rounded-xl border border-border bg-background/60 px-3 text-sm outline-none focus:border-primary/60 focus:ring-2 focus:ring-primary/15 disabled:opacity-60"
                />
              </label>
              <label className="block space-y-1.5">
                <span className="text-xs font-medium">Email address</span>
                <input
                  type="email"
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                  autoComplete="email"
                  disabled={saving}
                  className="h-10 w-full rounded-xl border border-border bg-background/60 px-3 text-sm outline-none focus:border-primary/60 focus:ring-2 focus:ring-primary/15 disabled:opacity-60"
                />
              </label>
            </div>

            <div className="flex justify-end">
              <Button type="submit" size="sm" disabled={saving}>
                {saving ? <Loader2 className="size-3.5 animate-spin" /> : <Save className="size-3.5" />}
                Save changes
              </Button>
            </div>
          </form>
        </section>

        <section className="rounded-2xl border border-border bg-surface/60 p-5">
          <h2 className="text-sm font-semibold">Account facts</h2>
          <dl className="mt-3 space-y-3 text-xs">
            <div>
              <dt className="text-muted-foreground">Registered</dt>
              <dd className="mt-0.5 font-mono text-[11px] text-foreground">
                {formatDateTime(user.created_at)}
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Last sign-in</dt>
              <dd className="mt-0.5 font-mono text-[11px] text-foreground">
                {user.last_login_at ? formatDateTime(user.last_login_at) : 'Never'}
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Email verification</dt>
              <dd className="mt-0.5 text-[11px] text-foreground">
                {user.email_verified ? 'Verified' : 'Not verified'}
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Live account sessions</dt>
              <dd className="mt-0.5 font-mono text-[11px] text-foreground">
                {user.active_sessions}
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Account ID</dt>
              <dd className="mt-0.5 break-all font-mono text-[10px] text-foreground">{user.id}</dd>
            </div>
          </dl>
        </section>
      </div>

      <div className="grid gap-4 xl:grid-cols-2">
        <section className="rounded-2xl border border-border bg-surface/60 p-5">
          <div className="flex items-center gap-2">
            <ShieldCheck className="size-4 text-primary" aria-hidden="true" />
            <h2 className="text-sm font-semibold">Privileges</h2>
          </div>
          <p className="mt-1 text-[11px] leading-relaxed text-muted-foreground">
            The server refuses self-promotion, self-demotion, and removing the last active
            administrator.
          </p>
          <div className="mt-4 flex flex-wrap gap-2">
            {user.role === 'admin' ? (
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={isSelf}
                onClick={() => setConfirm('demote')}
              >
                <ShieldOff className="size-3.5" />
                Remove administrator
              </Button>
            ) : (
              <Button
                type="button"
                variant="secondary"
                size="sm"
                disabled={isSelf}
                onClick={() => setConfirm('promote')}
              >
                <ShieldCheck className="size-3.5" />
                Grant administrator
              </Button>
            )}
          </div>
          {isSelf ? (
            <p className="mt-2 text-[11px] text-muted-foreground">
              You cannot change your own role.
            </p>
          ) : null}
        </section>

        <section className="rounded-2xl border border-border bg-surface/60 p-5">
          <div className="flex items-center gap-2">
            <UserX className="size-4 text-danger" aria-hidden="true" />
            <h2 className="text-sm font-semibold">Account status</h2>
          </div>
          <p className="mt-1 text-[11px] leading-relaxed text-muted-foreground">
            Deactivation revokes every account session and closes this account's live terminal
            sessions.
          </p>
          <div className="mt-4">
            {user.account_status === 'active' ? (
              <Button
                type="button"
                variant="destructive"
                size="sm"
                disabled={isSelf}
                onClick={() => setConfirm('disable')}
              >
                Deactivate account
              </Button>
            ) : (
              <Button
                type="button"
                variant="secondary"
                size="sm"
                disabled={isSelf}
                onClick={() => setConfirm('enable')}
              >
                Activate account
              </Button>
            )}
          </div>
          {isSelf ? (
            <p className="mt-2 text-[11px] text-muted-foreground">
              You cannot change your own account status.
            </p>
          ) : null}
        </section>
      </div>

      <section className="overflow-hidden rounded-2xl border border-border bg-surface/60">
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border px-5 py-4">
          <div className="flex items-center gap-2">
            <Monitor className="size-4 text-primary" aria-hidden="true" />
            <div>
              <h2 className="text-sm font-semibold">Active account sessions</h2>
              <p className="mt-0.5 text-[11px] text-muted-foreground">
                Only session identifiers, agents, and timestamps are shown.
              </p>
            </div>
          </div>
          <Button
            type="button"
            variant="destructive"
            size="sm"
            disabled={sessions.length === 0}
            onClick={() => setConfirm('revoke')}
          >
            <KeyRound className="size-3.5" />
            Revoke all sessions
          </Button>
        </div>

        {sessions.length === 0 ? (
          <div className="p-5">
            <EmptyState
              title="No active sessions"
              description="This account is not signed in on any browser right now."
            />
          </div>
        ) : (
          <ul className="divide-y divide-border">
            {sessions.map((session) => (
              <li key={session.id} className="flex flex-wrap items-center justify-between gap-2 px-5 py-3">
                <div className="min-w-0">
                  <p className="font-mono text-[11px] text-foreground">{session.id.slice(0, 8)}…</p>
                  <p className="mt-0.5 truncate text-[11px] text-muted-foreground">
                    {session.user_agent ?? 'Unknown client'}
                  </p>
                </div>
                <p className="text-[11px] text-muted-foreground">
                  Last used {formatRelative(session.last_used_at)} · expires{' '}
                  {formatDateTime(session.expires_at)}
                </p>
              </li>
            ))}
          </ul>
        )}
      </section>

      <ConfirmDialog
        open={confirm !== null}
        title={confirm ? confirmCopy[confirm].title : ''}
        description={confirm ? confirmCopy[confirm].description : ''}
        confirmLabel={confirm ? confirmCopy[confirm].label : ''}
        pending={actionPending}
        tone={confirm === 'enable' || confirm === 'promote' ? 'default' : 'danger'}
        onConfirm={() => void runConfirmedAction()}
        onClose={() => setConfirm(null)}
      />
    </div>
  );
}
