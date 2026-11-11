import { Link, useNavigate, useSearch } from '@tanstack/react-router';
import { useState } from 'react';
import {
  AuthDivider,
  AuthShell,
  FormError,
  FormField,
  FormNotice,
  PasswordField,
  SubmitButton,
} from '@/components/auth/auth-shell';
import { describeAuthError, useAuth } from '@/components/auth/auth-provider';
import { Checkbox } from '@/components/auth/checkbox';

export function SignInPage() {
  const navigate = useNavigate();
  const search = useSearch({ strict: false }) as { redirect?: string };
  const { signIn, accountsEnabled, ready, signedIn, serverReachable } =
    useAuth();

  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [remember, setRemember] = useState(true);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<{
    email?: string;
    password?: string;
  }>({});

  const redirectTo =
    search.redirect && search.redirect.startsWith('/')
      ? search.redirect
      : '/account';

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    const next: { email?: string; password?: string } = {};
    if (!email.trim()) {
      next.email = 'Enter the email address for your account.';
    } else if (!email.includes('@')) {
      next.email = 'Enter a valid email address.';
    }
    if (!password) {
      next.password = 'Enter your password.';
    }
    setFieldErrors(next);

    if (Object.keys(next).length > 0) {
      return;
    }

    setPending(true);
    try {
      await signIn(email.trim(), password, remember);
      await navigate({ to: redirectTo });
    } catch (cause) {
      setError(describeAuthError(cause));
    } finally {
      setPending(false);
    }
  }

  if (ready && signedIn) {
    return (
      <AuthShell
        title="You're already signed in."
        description="This browser already has an active KERNEXT account session."
        footer={
          <Link to="/account" className="text-primary hover:underline">
            Go to your account
          </Link>
        }
      >
        <Link
          to="/account"
          className="flex h-11 w-full items-center justify-center rounded-xl bg-primary text-sm font-medium text-[#03131a] transition-colors hover:bg-primary-strong"
        >
          Open account
        </Link>
      </AuthShell>
    );
  }

  return (
    <AuthShell
      title="Welcome back."
      description="Sign in to continue to your KERNEXT workspace."
      footer={
        <>
          New to KERNEXT?{' '}
          <Link
            to="/create-account"
            className="text-foreground underline-offset-4 hover:text-primary hover:underline"
          >
            Create an account
          </Link>
        </>
      }
    >
      {!ready ? (
        <div className="space-y-4" aria-busy="true">
          <div className="h-11 animate-pulse rounded-xl bg-surface-soft" />
          <div className="h-11 animate-pulse rounded-xl bg-surface-soft" />
          <div className="h-11 animate-pulse rounded-xl bg-surface-soft" />
        </div>
      ) : (
        <form onSubmit={handleSubmit} className="space-y-4" noValidate>
          {!serverReachable ? (
            <FormNotice tone="warning">
              The KERNEXT server is not reachable. Start it with
              <code className="mx-1 rounded bg-surface-soft px-1 font-mono text-[11px]">
                npm run dev
              </code>
              and reload this page.
            </FormNotice>
          ) : !accountsEnabled ? (
            <FormNotice tone="warning">
              Account sign-in is not configured on this server. Set
              <code className="mx-1 rounded bg-surface-soft px-1 font-mono text-[11px]">
                DATABASE_URL
              </code>
              to a MariaDB connection in the workspace <code>.env</code>, then
              restart the backend.
            </FormNotice>
          ) : null}

          <FormError>{error}</FormError>

          <FormField
            label="Email address"
            name="email"
            type="email"
            value={email}
            onChange={setEmail}
            autoComplete="email"
            placeholder="you@example.com"
            error={fieldErrors.email}
            disabled={!accountsEnabled || pending}
          />

          <PasswordField
            name="password"
            value={password}
            onChange={setPassword}
            autoComplete="current-password"
            placeholder="Your password"
            error={fieldErrors.password}
            disabled={!accountsEnabled || pending}
          />

          <div className="flex items-center justify-between gap-3">
            <Checkbox
              checked={remember}
              onChange={setRemember}
              label="Keep me signed in"
              disabled={!accountsEnabled || pending}
            />
            <Link
              to="/forgot-password"
              className="text-xs text-muted-foreground underline-offset-4 transition-colors hover:text-primary hover:underline"
            >
              Forgot password?
            </Link>
          </div>

          <SubmitButton pending={pending}>Sign in</SubmitButton>

          <AuthDivider>Local shell</AuthDivider>

          <p className="text-center text-[11px] leading-relaxed text-muted-foreground">
            The local KERNEXT shell keeps working without an account. Signing in
            only protects web sessions and account features.
          </p>
        </form>
      )}
    </AuthShell>
  );
}
