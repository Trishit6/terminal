import { Link } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { AuthShell, FormNotice } from '@/components/auth/auth-shell';
import { describeAuthError, useAuth } from '@/components/auth/auth-provider';

export function SignOutPage() {
  const { signOut, signedIn, ready } = useAuth();
  const [error, setError] = useState<string | null>(null);
  const [settled, setSettled] = useState(false);

  useEffect(() => {
    if (!ready) {
      return;
    }

    let cancelled = false;

    void signOut()
      .then(() => {
        if (!cancelled) {
          setSettled(true);
        }
      })
      .catch((cause) => {
        if (!cancelled) {
          setError(describeAuthError(cause));
          setSettled(true);
        }
      });

    return () => {
      cancelled = true;
    };
    // `signOut` is stable and must run once when the page is ready.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready]);

  return (
    <AuthShell
      title={settled && !signedIn ? "You're signed out." : 'Signing out.'}
      description={
        error
          ? 'The server could not confirm the sign-out, but this browser has dropped the account view.'
          : 'Your server-side session has been revoked and its cookie cleared.'
      }
      footer={
        <>
          Return to the{' '}
          <Link to="/" className="text-primary hover:underline">
            KERNEXT home page
          </Link>{' '}
          or{' '}
          <Link to="/sign-in" className="text-primary hover:underline">
            sign in again
          </Link>
          .
        </>
      }
    >
      {error ? <FormNotice tone="warning">{error}</FormNotice> : null}
      {!error ? (
        <div className="flex h-11 items-center justify-center rounded-xl border border-border bg-surface-soft/40 font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
          Session closed
        </div>
      ) : null}
    </AuthShell>
  );
}
