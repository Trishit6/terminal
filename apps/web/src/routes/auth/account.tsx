import { Link } from '@tanstack/react-router';
import {
  CheckCircle2,
  KeyRound,
  Laptop,
  Loader2,
  LogOut,
  MailWarning,
  ShieldCheck,
  UserRound,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import {
  AuthShell,
  FormError,
  FormField,
  FormNotice,
  PasswordField,
} from '@/components/auth/auth-shell';
import { describeAuthError, useAuth } from '@/components/auth/auth-provider';
import { Button } from '@/components/ui/button';
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui/tabs';
import { passwordProblem } from '@/lib/validation';
import type { AuthSessionSummary } from '@/lib/auth';

function formatDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(date);
}

export function AccountPage() {
  const {
    ready,
    signedIn,
    user,
    preferences,
    accountsEnabled,
    updateProfile,
    changePassword,
    listSessions,
    revokeSession,
    signOut,
    resendVerification,
    updatePreferences,
    refresh,
  } = useAuth();

  const [tab, setTab] = useState('profile');

  const [fullName, setFullName] = useState('');
  const [email, setEmail] = useState('');
  const [profilePending, setProfilePending] = useState(false);
  const [profileError, setProfileError] = useState<string | null>(null);
  const [profileNotice, setProfileNotice] = useState<string | null>(null);
  const [profileFieldErrors, setProfileFieldErrors] = useState<
    Record<string, string>
  >({});

  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [passwordPending, setPasswordPending] = useState(false);
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const [passwordNotice, setPasswordNotice] = useState<string | null>(null);

  const [sessions, setSessions] = useState<AuthSessionSummary[] | null>(null);
  const [sessionError, setSessionError] = useState<string | null>(null);
  const [sessionPending, setSessionPending] = useState(false);

  const [theme, setTheme] = useState(preferences?.theme ?? 'dark');
  const [preferencePending, setPreferencePending] = useState(false);
  const [preferenceNotice, setPreferenceNotice] = useState<string | null>(
    null,
  );

  const [verificationNotice, setVerificationNotice] = useState<string | null>(
    null,
  );
  const [verificationDevToken, setVerificationDevToken] = useState<
    string | null
  >(null);

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
    if (tab === 'sessions' && sessions === null && !sessionPending) {
      void loadSessions();
    }
  }, [tab, sessions, sessionPending, loadSessions]);

  async function handleProfile(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setProfileError(null);
    setProfileNotice(null);

    const next: Record<string, string> = {};
    if (fullName.trim().length < 2) {
      next.fullName = 'Enter your full name.';
    }
    if (!email.includes('@') || !email.includes('.')) {
      next.email = 'Enter a valid email address.';
    }
    setProfileFieldErrors(next);

    if (Object.keys(next).length > 0) {
      return;
    }

    setProfilePending(true);
    try {
      const result = await updateProfile(fullName.trim(), email.trim());
      setProfileNotice('Your profile has been updated.');
      setVerificationDevToken(result.dev_verification_token ?? null);
      if (result.email_verification_required) {
        setVerificationNotice(
          'Your email address changed. Confirm the new address to finish the update.',
        );
      }
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
      setPasswordNotice(
        await changePassword(currentPassword, newPassword),
      );
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
    } catch (cause) {
      setPasswordError(describeAuthError(cause));
    } finally {
      setPasswordPending(false);
    }
  }

  async function handleRevoke(id: string) {
    if (!window.confirm('Revoke this session? Any device using it will be signed out.')) {
      return;
    }

    setSessionError(null);
    try {
      await revokeSession(id);
      setSessions((current) =>
        current ? current.filter((session) => session.id !== id) : current,
      );
    } catch (cause) {
      setSessionError(describeAuthError(cause));
    }
  }

  async function handleResendVerification() {
    setVerificationNotice(null);
    try {
      const result = await resendVerification();
      if (result.email_delivery === 'already_verified') {
        setVerificationNotice('This address is already verified.');
        await refresh();
      } else if (result.email_delivery === 'sent') {
        setVerificationNotice('A new verification email has been sent.');
      } else {
        setVerificationNotice(
          'No email was sent because this server has no SMTP configuration.',
        );
      }
    } catch (cause) {
      setVerificationNotice(describeAuthError(cause));
    }
  }

  async function handlePreferences(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPreferencePending(true);
    setPreferenceNotice(null);
    try {
      await updatePreferences(theme);
      setPreferenceNotice('Account preferences saved.');
    } catch (cause) {
      setPreferenceNotice(describeAuthError(cause));
    } finally {
      setPreferencePending(false);
    }
  }

  if (!ready) {
    return (
      <AuthShell
        title="Loading your account."
        description="Checking the current KERNEXT account session."
      >
        <div className="flex h-24 items-center justify-center">
          <Loader2 className="size-5 animate-spin text-primary" />
        </div>
      </AuthShell>
    );
  }

  if (!accountsEnabled) {
    return (
      <AuthShell
        title="Accounts are not configured."
        description="This KERNEXT server is running without a MariaDB account database."
        footer={
          <Link to="/" className="text-primary hover:underline">
            Back to KERNEXT
          </Link>
        }
      >
        <FormNotice tone="warning">
          Set DATABASE_URL to a MariaDB connection and restart the server to
          enable accounts. The local shell and terminal still work without one.
        </FormNotice>
      </AuthShell>
    );
  }

  if (!signedIn || !user) {
    return (
      <AuthShell
        title="Sign in to view your account."
        description="Your profile, security settings, and active sessions live behind your KERNEXT account."
        footer={
          <Link to="/create-account" className="text-primary hover:underline">
            Create an account
          </Link>
        }
      >
        <Link
          to="/sign-in"
          className="flex h-11 w-full items-center justify-center rounded-xl bg-primary text-sm font-medium text-[#03131a] hover:bg-primary-strong"
        >
          Sign in
        </Link>
      </AuthShell>
    );
  }

  return (
    <div className="mx-auto w-full max-w-4xl px-4 py-10 sm:px-6">
      <header className="flex flex-wrap items-start justify-between gap-4 border-b border-border pb-6">
        <div className="flex items-start gap-4">
          <div className="grid size-12 place-items-center rounded-2xl border border-border bg-surface font-mono text-sm font-semibold text-primary">
            {user.full_name
              .split(/\s+/)
              .map((part) => part[0])
              .filter(Boolean)
              .slice(0, 2)
              .join('')
              .toUpperCase()}
          </div>
          <div>
            <h1 className="text-2xl font-semibold tracking-[-0.02em]">
              {user.full_name}
            </h1>
            <p className="mt-0.5 text-sm text-muted-foreground">{user.email}</p>
            <div className="mt-2 flex flex-wrap items-center gap-2 text-[11px]">
              <span className="inline-flex items-center gap-1.5 rounded-full border border-border px-2 py-0.5 text-muted-foreground">
                Member since {formatDate(user.created_at)}
              </span>
              <span
                className={
                  user.email_verified
                    ? 'inline-flex items-center gap-1.5 rounded-full border border-success/30 bg-success/10 px-2 py-0.5 text-success'
                    : 'inline-flex items-center gap-1.5 rounded-full border border-warning/30 bg-warning/10 px-2 py-0.5 text-warning'
                }
              >
                {user.email_verified ? (
                  <CheckCircle2 className="size-3" />
                ) : (
                  <MailWarning className="size-3" />
                )}
                {user.email_verified ? 'Email verified' : 'Email unverified'}
              </span>
            </div>
          </div>
        </div>

        <Button variant="outline" size="sm" asChild>
          <Link to="/sign-out">
            <LogOut className="size-3.5" />
            Sign out
          </Link>
        </Button>
      </header>

      {!user.email_verified ? (
        <div className="mt-5 rounded-xl border border-warning/25 bg-warning/5 px-4 py-3">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <p className="text-xs leading-relaxed text-warning">
              {verificationNotice ??
                'Confirm your email address to secure account recovery.'}
            </p>
            <div className="flex items-center gap-2">
              {verificationDevToken ? (
                <Link
                  to="/verify-email"
                  search={{ token: verificationDevToken }}
                  className="text-[11px] text-primary hover:underline"
                >
                  Development verification link
                </Link>
              ) : null}
              <Button
                type="button"
                variant="secondary"
                size="sm"
                onClick={() => void handleResendVerification()}
              >
                Resend email
              </Button>
            </div>
          </div>
        </div>
      ) : null}

      <Tabs value={tab} onValueChange={setTab} className="mt-6">
        <TabsList className="w-full justify-start overflow-x-auto">
          <TabsTrigger value="profile">
            <UserRound className="size-3.5" />
            Profile
          </TabsTrigger>
          <TabsTrigger value="security">
            <ShieldCheck className="size-3.5" />
            Security
          </TabsTrigger>
          <TabsTrigger value="sessions">
            <Laptop className="size-3.5" />
            Sessions
          </TabsTrigger>
          <TabsTrigger value="preferences">
            <KeyRound className="size-3.5" />
            Preferences
          </TabsTrigger>
        </TabsList>

        <TabsContent value="profile">
          <form
            onSubmit={handleProfile}
            className="space-y-4 rounded-2xl border border-border bg-surface/60 p-5"
            noValidate
          >
            <FormError>{profileError}</FormError>
            {profileNotice ? (
              <FormNotice tone="success">{profileNotice}</FormNotice>
            ) : null}

            <div className="grid gap-4 sm:grid-cols-2">
              <FormField
                label="Full name"
                name="full_name"
                value={fullName}
                onChange={setFullName}
                autoComplete="name"
                error={profileFieldErrors.fullName}
                disabled={profilePending}
              />
              <FormField
                label="Email address"
                name="email"
                type="email"
                value={email}
                onChange={setEmail}
                autoComplete="email"
                error={profileFieldErrors.email}
                disabled={profilePending}
              />
            </div>

            <div className="flex justify-end">
              <Button type="submit" disabled={profilePending}>
                {profilePending ? (
                  <Loader2 className="size-4 animate-spin" />
                ) : null}
                Save profile
              </Button>
            </div>
          </form>
        </TabsContent>

        <TabsContent value="security">
          <div className="space-y-4">
            <form
              onSubmit={handlePassword}
              className="space-y-4 rounded-2xl border border-border bg-surface/60 p-5"
              noValidate
            >
              <div>
                <h2 className="text-sm font-semibold">Change password</h2>
                <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                  Passwords are hashed with Argon2id. Your other sessions stay
                  valid unless you sign out of them below.
                </p>
              </div>

              <FormError>{passwordError}</FormError>
              {passwordNotice ? (
                <FormNotice tone="success">{passwordNotice}</FormNotice>
              ) : null}

              <PasswordField
                label="Current password"
                name="current_password"
                value={currentPassword}
                onChange={setCurrentPassword}
                autoComplete="current-password"
                disabled={passwordPending}
              />
              <PasswordField
                label="New password"
                name="new_password"
                value={newPassword}
                onChange={setNewPassword}
                autoComplete="new-password"
                hint="10+ characters, a letter and a number"
                disabled={passwordPending}
              />
              <PasswordField
                label="Confirm new password"
                name="confirm_password"
                value={confirmPassword}
                onChange={setConfirmPassword}
                autoComplete="new-password"
                disabled={passwordPending}
              />

              <div className="flex justify-end">
                <Button type="submit" disabled={passwordPending}>
                  {passwordPending ? (
                    <Loader2 className="size-4 animate-spin" />
                  ) : null}
                  Update password
                </Button>
              </div>
            </form>

            <div className="rounded-2xl border border-border bg-surface/60 p-5">
              <h2 className="text-sm font-semibold">Security details</h2>
              <dl className="mt-3 grid gap-3 text-xs sm:grid-cols-2">
                <div>
                  <dt className="text-muted-foreground">Password hashing</dt>
                  <dd className="mt-0.5 font-mono text-[11px] text-foreground">
                    Argon2id
                  </dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Session storage</dt>
                  <dd className="mt-0.5 font-mono text-[11px] text-foreground">
                    HttpOnly cookie · server-managed
                  </dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Account status</dt>
                  <dd className="mt-0.5 font-mono text-[11px] text-foreground">
                    {user.account_status}
                  </dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Account ID</dt>
                  <dd className="mt-0.5 break-all font-mono text-[11px] text-foreground">
                    {user.id}
                  </dd>
                </div>
              </dl>
            </div>
          </div>
        </TabsContent>

        <TabsContent value="sessions">
          <div className="rounded-2xl border border-border bg-surface/60">
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border p-5">
              <div>
                <h2 className="text-sm font-semibold">Active sessions</h2>
                <p className="mt-1 text-xs text-muted-foreground">
                  Only this account's sessions are listed. Revoking one signs
                  that device out immediately.
                </p>
              </div>
              <Button
                type="button"
                variant="secondary"
                size="sm"
                onClick={() => void loadSessions()}
                disabled={sessionPending}
              >
                {sessionPending ? (
                  <Loader2 className="size-3.5 animate-spin" />
                ) : null}
                Refresh
              </Button>
            </div>

            <div className="space-y-3 p-5">
              <FormError>{sessionError}</FormError>

              {sessions === null && sessionPending ? (
                <div className="h-16 animate-pulse rounded-xl bg-surface-soft" />
              ) : null}

              {sessions?.length === 0 ? (
                <p className="text-xs text-muted-foreground">
                  No active sessions were found.
                </p>
              ) : null}

              {sessions?.map((session) => (
                <div
                  key={session.id}
                  className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border bg-background/40 px-4 py-3"
                >
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-mono text-[11px] text-foreground">
                        {session.id.slice(0, 8)}…
                      </span>
                      {session.current ? (
                        <span className="rounded-full border border-primary/30 bg-primary/10 px-2 py-0.5 text-[10px] text-primary">
                          This device
                        </span>
                      ) : null}
                    </div>
                    <p className="mt-1 text-[11px] text-muted-foreground">
                      Signed in {formatDate(session.created_at)} · last used{' '}
                      {formatDate(session.last_used_at)} · expires{' '}
                      {formatDate(session.expires_at)}
                    </p>
                  </div>
                  <Button
                    type="button"
                    variant="destructive"
                    size="sm"
                    onClick={() => void handleRevoke(session.id)}
                    disabled={sessionPending}
                  >
                    {session.current ? 'Sign out' : 'Revoke'}
                  </Button>
                </div>
              ))}
            </div>
          </div>
        </TabsContent>

        <TabsContent value="preferences">
          <form
            onSubmit={handlePreferences}
            className="space-y-4 rounded-2xl border border-border bg-surface/60 p-5"
          >
            <div>
              <h2 className="text-sm font-semibold">Account preferences</h2>
              <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                These follow your account across browsers. Desktop terminal
                preferences stay local to that machine.
              </p>
            </div>

            {preferenceNotice ? (
              <FormNotice tone="success">{preferenceNotice}</FormNotice>
            ) : null}

            <label className="block max-w-xs space-y-1.5">
              <span className="text-xs font-medium">Theme</span>
              <select
                value={theme}
                onChange={(event) => setTheme(event.target.value)}
                className="h-11 w-full rounded-xl border border-border bg-background/60 px-3 text-sm outline-none focus:border-primary/60 focus:ring-2 focus:ring-primary/15"
              >
                <option value="dark">Midnight (dark)</option>
                <option value="light">Daylight (light)</option>
                <option value="system">Follow the operating system</option>
              </select>
            </label>

            <div className="flex justify-end">
              <Button type="submit" disabled={preferencePending}>
                {preferencePending ? (
                  <Loader2 className="size-4 animate-spin" />
                ) : null}
                Save preferences
              </Button>
            </div>
          </form>
        </TabsContent>
      </Tabs>

      <div className="mt-8 flex flex-wrap items-center justify-between gap-3 border-t border-border pt-5">
        <p className="text-[11px] text-muted-foreground">
          Local shell usage does not require this account.
        </p>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => {
            void signOut().then(() => {
              window.location.assign('/');
            });
          }}
        >
          <LogOut className="size-3.5" />
          Sign out
        </Button>
      </div>
    </div>
  );
}
