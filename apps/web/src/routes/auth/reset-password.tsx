import { Link, useSearch } from '@tanstack/react-router';
import { useState } from 'react';
import {
  AuthShell,
  FormError,
  FormNotice,
  PasswordField,
  SubmitButton,
} from '@/components/auth/auth-shell';
import { describeAuthError, useAuth } from '@/components/auth/auth-provider';
import { passwordProblem } from '@/lib/validation';

export function ResetPasswordPage() {
  const search = useSearch({ strict: false }) as { token?: string };
  const { resetPassword } = useAuth();
  const token = search.token ?? '';

  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [done, setDone] = useState<string | null>(null);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    const next: Record<string, string> = {};
    const problem = passwordProblem(password);
    if (problem) {
      next.password = problem;
    }
    if (confirm !== password) {
      next.confirm = 'The passwords do not match.';
    }
    setFieldErrors(next);

    if (Object.keys(next).length > 0 || !token) {
      return;
    }

    setPending(true);
    try {
      setDone(await resetPassword(token, password));
    } catch (cause) {
      setError(describeAuthError(cause));
    } finally {
      setPending(false);
    }
  }

  if (!token) {
    return (
      <AuthShell
        title="This reset link is incomplete."
        description="The link is missing its single-use token. Request a new one."
        footer={
          <Link to="/forgot-password" className="text-primary hover:underline">
            Request a new link
          </Link>
        }
      >
        <Link
          to="/forgot-password"
          className="flex h-11 w-full items-center justify-center rounded-xl bg-primary text-sm font-medium text-[#03131a] hover:bg-primary-strong"
        >
          Request a password reset
        </Link>
      </AuthShell>
    );
  }

  return (
    <AuthShell
      title={done ? 'Password updated.' : 'Choose a new password.'}
      description={
        done
          ? 'Every other session for this account has been signed out.'
          : 'Resetting your password signs out all existing sessions.'
      }
      footer={
        <Link to="/sign-in" className="text-primary hover:underline">
          Back to sign in
        </Link>
      }
    >
      {done ? (
        <div className="space-y-4">
          <FormNotice tone="success">{done}</FormNotice>
          <Link
            to="/sign-in"
            className="flex h-11 w-full items-center justify-center rounded-xl bg-primary text-sm font-medium text-[#03131a] hover:bg-primary-strong"
          >
            Sign in with the new password
          </Link>
        </div>
      ) : (
        <form onSubmit={handleSubmit} className="space-y-4" noValidate>
          <FormError>{error}</FormError>

          <PasswordField
            label="New password"
            name="new_password"
            value={password}
            onChange={setPassword}
            autoComplete="new-password"
            placeholder="At least 10 characters"
            error={fieldErrors.password}
            hint="10+ characters, a letter and a number"
            disabled={pending}
          />

          <PasswordField
            label="Confirm new password"
            name="confirm_password"
            value={confirm}
            onChange={setConfirm}
            autoComplete="new-password"
            placeholder="Repeat the new password"
            error={fieldErrors.confirm}
            disabled={pending}
          />

          <SubmitButton pending={pending}>Set new password</SubmitButton>
        </form>
      )}
    </AuthShell>
  );
}
