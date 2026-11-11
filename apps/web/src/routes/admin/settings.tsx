import {
  KeyRound,
  Laptop,
  Loader2,
  LockKeyhole,
  Moon,
  Save,
  ShieldCheck,
  UserRound,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import {
  ConfirmDialog,
  EmptyState,
  ErrorNotice,
  PageHeader,
  RoleBadge,
  formatDateTime,
} from '@/components/admin/admin-ui';
import { FormError, FormNotice } from '@/components/auth/auth-shell';
import { useAuth } from '@/components/auth/auth-provider';
import { Button } from '@/components/ui/button';
import { describeAuthError } from '@/lib/auth';
import type { AuthSessionSummary } from '@/lib/auth';
import { isValidEmail, passwordProblem } from '@/lib/validation';

export function AdminSettingsPage() {
  const {
    user,
    preferences,
    updateProfile,
    changePassword,
    listSessions,
    revokeSession,
    updatePreferences,
    refresh,
  } = useAuth();

  const [fullName, setFullName] = useState('');
  const [email, setEmail] = useState('');
  const [profilePending, setProfilePending] = useState(false);
  const [profileError, setProfileError] = useState<string | null>(null);
  const [profileNotice, setProfileNotice] = useState<string | null>(null);

  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [passwordPending, setPasswordPending] = useState(false);
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const [passwordNotice, setPasswordNotice] = useState<string | null>(null);

  const [theme, setTheme] = useState(preferences?.theme ?? 'dark');
  const [themePending, setThemePending] = useState(false);
  const [themeNotice, setThemeNotice] = useState<string | null>(null);

  const [sessions, setSessions] = useState<AuthSessionSummary[] | null>(null);
  const [sessionError, setSessionError] = useState<string | null>(null);
  const [sessionPending, setSessionPending] = useState(false);
  const [revokeTarget, setRevokeTarget] = useState<AuthSessionSummary | null>(null);

  useEffect(() => {
    if (user) {
      setFullName(user.full_name);
      setEmail(user.email);
    }
  }, [user]);

  useEffect(() => {
    if (preferences) {
      setTheme(preferences.theme);
    }
  }, [preferences]);

  const loadSessions = useCallback(async () => {
    setSessionPending(true);
    setSessionError(null);
    try {
      setSessions(await listSessions());
    } catch (cause) {
      setSessionError(describeAuthError(cause));
    } finally {
      setSessionPending(false);
    }
  }, [listSessions]);

  useEffect(() => {
    void loadSessions();
  }, [loadSessions]);

  async function handleProfile(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setProfileError(null);
    setProfileNotice(null);

    if (fullName.trim().length < 2) {
      setProfileError('Enter your full name.');
      return;
    }
    if (!isValidEmail(email)) {
      setProfileError('Enter a valid email address.');
      return;
    }

    setProfilePending(true);
    try {
      await updateProfile(fullName.trim(), email.trim());
      setProfileNotice(
        'Profile updated. If you changed your email address, confirm the new address to finish the change.',
      );
    } catch (cause) {
      setProfileError(describeAuthError(cause));
    } finally {
      setProfilePending(false);
    }
  }

  async function handlePassword(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPasswordError(null);
    setPasswordNotice(null);

    if (!currentPassword) {
      setPasswordError('Enter your current password.');
      return;
    }

    const problem = passwordProblem(newPassword);
    if (problem) {
      setPasswordError(problem);
      return;
    }

    if (newPassword !== confirmPassword) {
      setPasswordError('The new passwords do not match.');
      return;
    }

    setPasswordPending(true);
    try {
      setPasswordNotice(await changePassword(currentPassword, newPassword));
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
      await refresh();
    } catch (cause) {
      setPasswordError(describeAuthError(cause));
    } finally {
      setPasswordPending(false);
    }
  }

  async function handleTheme(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setThemePending(true);
    setThemeNotice(null);
    try {
      await updatePreferences(theme);
      setThemeNotice('Theme preference saved to this account.');
    } catch (cause) {
      setThemeNotice(describeAuthError(cause));
    } finally {
      setThemePending(false);
    }
  }

  async function revoke() {
    if (!revokeTarget) {
      return;
    }

    setSessionPending(true);
    setSessionError(null);
    try {
      await revokeSession(revokeTarget.id);
      setSessions((current) =>
        current ? current.filter((session) => session.id !== revokeTarget.id) : current,
      );
      setRevokeTarget(null);
    } catch (cause) {
      setSessionError(describeAuthError(cause));
    } finally {
      setSessionPending(false);
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="Administrator settings"
        description="Your own profile, password, sessions, and theme preferences. These changes apply to your account, not to other users."
      />

      {user ? (
        <section className="flex flex-wrap items-center gap-3 rounded-2xl border border-border bg-surface/60 p-5">
          <span className="grid size-11 place-items-center rounded-xl border border-border bg-surface-soft font-mono text-xs font-semibold text-primary">
            {user.full_name
              .split(/\s+/)
              .map((part) => part[0])
              .filter(Boolean)
              .slice(0, 2)
              .join('')
              .toUpperCase()}
          </span>
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-semibold text-foreground">{user.full_name}</p>
            <p className="truncate text-xs text-muted-foreground">{user.email}</p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <RoleBadge role={user.role} />
            <span className="inline-flex items-center gap-1.5 rounded-full border border-success/30 bg-success/10 px-2 py-0.5 text-[10px] text-success">
              <ShieldCheck className="size-3" aria-hidden="true" />
              {user.email_verified ? 'Email verified' : 'Email unverified'}
            </span>
            <span className="rounded-full border border-border px-2 py-0.5 text-[10px] capitalize text-muted-foreground">
              {user.account_status}
            </span>
          </div>
        </section>
      ) : null}

      <div className="grid gap-4 xl:grid-cols-2">
        <section className="rounded-2xl border border-border bg-surface/60 p-5">
          <div className="flex items-center gap-2">
            <UserRound className="size-4 text-primary" aria-hidden="true" />
            <h2 className="text-sm font-semibold">Profile</h2>
          </div>

          <form onSubmit={handleProfile} className="mt-4 space-y-4" noValidate>
            <FormError>{profileError}</FormError>
            {profileNotice ? <FormNotice tone="success">{profileNotice}</FormNotice> : null}

            <label className="block space-y-1.5">
              <span className="text-xs font-medium">Full name</span>
              <input
                type="text"
                value={fullName}
                onChange={(event) => setFullName(event.target.value)}
                autoComplete="name"
                disabled={profilePending}
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
                disabled={profilePending}
                className="h-10 w-full rounded-xl border border-border bg-background/60 px-3 text-sm outline-none focus:border-primary/60 focus:ring-2 focus:ring-primary/15 disabled:opacity-60"
              />
            </label>

            <div className="flex justify-end">
              <Button type="submit" size="sm" disabled={profilePending}>
                {profilePending ? (
                  <Loader2 className="size-3.5 animate-spin" />
                ) : (
                  <Save className="size-3.5" />
                )}
                Save profile
              </Button>
            </div>
          </form>
        </section>

        <section className="rounded-2xl border border-border bg-surface/60 p-5">
          <div className="flex items-center gap-2">
            <LockKeyhole className="size-4 text-primary" aria-hidden="true" />
            <h2 className="text-sm font-semibold">Change password</h2>
          </div>

          <form onSubmit={handlePassword} className="mt-4 space-y-4" noValidate>
            <FormError>{passwordError}</FormError>
            {passwordNotice ? <FormNotice tone="success">{passwordNotice}</FormNotice> : null}

            <label className="block space-y-1.5">
              <span className="text-xs font-medium">Current password</span>
              <input
                type="password"
                value={currentPassword}
                onChange={(event) => setCurrentPassword(event.target.value)}
                autoComplete="current-password"
                disabled={passwordPending}
                className="h-10 w-full rounded-xl border border-border bg-background/60 px-3 text-sm outline-none focus:border-primary/60 focus:ring-2 focus:ring-primary/15 disabled:opacity-60"
              />
            </label>

            <label className="block space-y-1.5">
              <span className="text-xs font-medium">New password</span>
              <input
                type="password"
                value={newPassword}
                onChange={(event) => setNewPassword(event.target.value)}
                autoComplete="new-password"
                disabled={passwordPending}
                className="h-10 w-full rounded-xl border border-border bg-background/60 px-3 text-sm outline-none focus:border-primary/60 focus:ring-2 focus:ring-primary/15 disabled:opacity-60"
              />
              <span className="text-[10px] text-muted-foreground">
                10+ characters, a letter and a number
              </span>
            </label>

            <label className="block space-y-1.5">
              <span className="text-xs font-medium">Confirm new password</span>
              <input
                type="password"
                value={confirmPassword}
                onChange={(event) => setConfirmPassword(event.target.value)}
                autoComplete="new-password"
                disabled={passwordPending}
                className="h-10 w-full rounded-xl border border-border bg-background/60 px-3 text-sm outline-none focus:border-primary/60 focus:ring-2 focus:ring-primary/15 disabled:opacity-60"
              />
            </label>

            <div className="flex justify-end">
              <Button type="submit" size="sm" disabled={passwordPending}>
                {passwordPending ? (
                  <Loader2 className="size-3.5 animate-spin" />
                ) : (
                  <KeyRound className="size-3.5" />
                )}
                Update password
              </Button>
            </div>
          </form>
        </section>
      </div>

      <div className="grid gap-4 xl:grid-cols-2">
        <section className="rounded-2xl border border-border bg-surface/60 p-5">
          <div className="flex items-center gap-2">
            <Moon className="size-4 text-primary" aria-hidden="true" />
            <h2 className="text-sm font-semibold">Theme preference</h2>
          </div>
          <p className="mt-1 text-[11px] text-muted-foreground">
            Saved to your account so it follows you across browsers.
          </p>

          <form onSubmit={handleTheme} className="mt-4 space-y-4">
            {themeNotice ? <FormNotice tone="success">{themeNotice}</FormNotice> : null}
            <label className="block max-w-xs space-y-1.5">
              <span className="text-xs font-medium">Theme</span>
              <select
                value={theme}
                onChange={(event) => setTheme(event.target.value)}
                className="h-10 w-full rounded-xl border border-border bg-background/60 px-3 text-sm outline-none focus:border-primary/60 focus:ring-2 focus:ring-primary/15"
              >
                <option value="dark">Midnight (dark)</option>
                <option value="light">Daylight (light)</option>
                <option value="system">Follow the operating system</option>
              </select>
            </label>
            <div className="flex justify-end">
              <Button type="submit" size="sm" disabled={themePending}>
                {themePending ? <Loader2 className="size-3.5 animate-spin" /> : null}
                Save preference
              </Button>
            </div>
          </form>
        </section>

        <section className="overflow-hidden rounded-2xl border border-border bg-surface/60">
          <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border px-5 py-4">
            <div className="flex items-center gap-2">
              <Laptop className="size-4 text-primary" aria-hidden="true" />
              <div>
                <h2 className="text-sm font-semibold">Your active sessions</h2>
                <p className="mt-0.5 text-[11px] text-muted-foreground">
                  Revoking a session signs that device out.
                </p>
              </div>
            </div>
            <Button
              type="button"
              variant="secondary"
              size="sm"
              onClick={() => void loadSessions()}
              disabled={sessionPending}
            >
              {sessionPending ? <Loader2 className="size-3.5 animate-spin" /> : null}
              Refresh
            </Button>
          </div>

          <div className="space-y-3 p-4">
            {sessionError ? <ErrorNotice>{sessionError}</ErrorNotice> : null}

            {sessions === null ? (
              <div className="h-12 animate-pulse rounded-xl bg-surface-soft" />
            ) : sessions.length === 0 ? (
              <EmptyState
                title="No active sessions"
                description="This account has no live browser sessions."
              />
            ) : (
              <ul className="space-y-2">
                {sessions.map((session) => (
                  <li
                    key={session.id}
                    className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-border bg-background/40 px-3 py-2.5"
                  >
                    <div className="min-w-0">
                      <p className="flex items-center gap-2 font-mono text-[10px] text-foreground">
                        {session.id.slice(0, 8)}…
                        {session.current ? (
                          <span className="rounded-full border border-primary/30 bg-primary/10 px-1.5 py-0.5 text-[9px] text-primary">
                            This device
                          </span>
                        ) : null}
                      </p>
                      <p className="mt-0.5 text-[10px] text-muted-foreground">
                        Last used {formatDateTime(session.last_used_at)}
                      </p>
                    </div>
                    <Button
                      type="button"
                      variant="destructive"
                      size="sm"
                      onClick={() => setRevokeTarget(session)}
                      disabled={sessionPending}
                    >
                      {session.current ? 'Sign out' : 'Revoke'}
                    </Button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </section>
      </div>

      <ConfirmDialog
        open={revokeTarget !== null}
        title="Revoke this session?"
        description={
          revokeTarget?.current
            ? 'This will sign you out of this browser immediately.'
            : 'The device using this session will be signed out immediately.'
        }
        confirmLabel="Revoke session"
        pending={sessionPending}
        onConfirm={() => void revoke()}
        onClose={() => setRevokeTarget(null)}
      />
    </div>
  );
}
