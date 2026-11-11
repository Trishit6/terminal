import { Link, useSearch } from '@tanstack/react-router';
import { useEffect, useRef, useState } from 'react';
import {
  AuthShell,
  FormError,
  FormNotice,
} from '@/components/auth/auth-shell';
import { describeAuthError, useAuth } from '@/components/auth/auth-provider';

type VerificationState =
  | { kind: 'idle' }
  | { kind: 'pending' }
  | { kind: 'verified'; message: string }
  | { kind: 'failed'; message: string };

export function VerifyEmailPage() {
  const search = useSearch({ strict: false }) as { token?: string };
  const { verifyEmail } = useAuth();
  const token = search.token ?? '';
  const [state, setState] = useState<VerificationState>({ kind: 'idle' });
  const attempted = useRef(false);

  useEffect(() => {
    if (!token || attempted.current) {
      return;
    }

    attempted.current = true;
    setState({ kind: 'pending' });

    void verifyEmail(token)
      .then((message) => setState({ kind: 'verified', message }))
      .catch((cause) =>
        setState({ kind: 'failed', message: describeAuthError(cause) }),
      );
  }, [token, verifyEmail]);

  return (
    <AuthShell
      title={
        state.kind === 'verified'
          ? 'Email verified.'
          : state.kind === 'failed'
            ? 'Verification did not complete.'
            : 'Verifying your email.'
      }
      description={
        state.kind === 'verified'
          ? 'Your account now has a confirmed email address.'
          : 'This link is single-use and expires after 24 hours.'
      }
      footer={
        <Link to="/account" className="text-primary hover:underline">
          Go to your account
        </Link>
      }
    >
      {!token ? (
        <FormNotice tone="warning">
          This page needs a verification token. Open the link from your
          verification email, or resend it from your account.
        </FormNotice>
      ) : state.kind === 'failed' ? (
        <FormError>{state.message}</FormError>
      ) : state.kind === 'verified' ? (
        <FormNotice tone="success">{state.message}</FormNotice>
      ) : (
        <div className="space-y-3" aria-busy="true">
          <div className="h-11 animate-pulse rounded-xl bg-surface-soft" />
          <p className="text-center text-xs text-muted-foreground">
            Checking the verification token…
          </p>
        </div>
      )}
    </AuthShell>
  );
}
