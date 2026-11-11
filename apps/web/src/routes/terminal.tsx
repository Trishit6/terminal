import { Link } from '@tanstack/react-router';
import { ArrowUpRight, Circle, Loader2, ShieldCheck } from 'lucide-react';
import { TerminalWorkspace } from '@/components/terminal/terminal-workspace';
import { useAuth } from '@/components/auth/auth-provider';
import { Button } from '@/components/ui/button';

export function TerminalPage() {
  const { ready, signedIn, user, accountsEnabled } = useAuth();

  if (!ready) {
    return (
      <div className="mx-auto flex w-full max-w-site items-center justify-center px-4 py-24">
        <Loader2 className="size-5 animate-spin text-primary" />
      </div>
    );
  }

  // When the server runs with accounts, terminal access is account-gated:
  // every session is owned by the authenticated user. A server without a
  // database keeps its local-only behavior.
  if (accountsEnabled && !signedIn) {
    return (
      <div className="mx-auto flex w-full max-w-lg flex-col items-center px-4 py-24 text-center">
        <div className="grid size-12 place-items-center rounded-2xl border border-border bg-surface">
          <ShieldCheck className="size-5 text-primary" />
        </div>
        <h1 className="mt-5 text-2xl font-semibold tracking-[-0.02em]">
          Sign in to open the terminal.
        </h1>
        <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
          Terminal sessions run against this machine, so they are bound to your
          KERNEXT account. Sign in to start one.
        </p>
        <div className="mt-6 flex flex-wrap items-center justify-center gap-3">
          <Button asChild>
            <Link to="/sign-in" search={{ redirect: '/terminal' }}>
              Sign in
            </Link>
          </Button>
          <Button asChild variant="outline">
            <Link to="/create-account">Create an account</Link>
          </Button>
        </div>
        <p className="mt-5 text-[11px] text-muted-foreground">
          The KERNEXT CLI and desktop terminal keep working offline without an
          account.
        </p>
      </div>
    );
  }

  // Synthetic accounts are created with a one-time password. The server
  // rejects terminal sockets with `403 password_change_required` until that
  // password is replaced, so do not open a WebSocket that is guaranteed to
  // fail; show the required step and link straight to the security settings.
  if (accountsEnabled && user?.must_change_password) {
    return (
      <div className="mx-auto flex w-full max-w-lg flex-col items-center px-4 py-24 text-center">
        <div className="grid size-12 place-items-center rounded-2xl border border-warning/30 bg-warning/5">
          <ShieldCheck className="size-5 text-warning" />
        </div>
        <h1 className="mt-5 text-2xl font-semibold tracking-[-0.02em]">
          Change your initial password first.
        </h1>
        <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
          This account still uses the password it was created with. Terminal
          sessions stay locked until you choose a new one, so no terminal
          connection was attempted.
        </p>
        <Button asChild className="mt-6">
          <Link to="/account">Change password</Link>
        </Button>
        <p className="mt-5 text-[11px] text-muted-foreground">
          The KERNEXT CLI and desktop terminal keep working offline without an
          account.
        </p>
      </div>
    );
  }

  return (
    <div className="mx-auto flex w-full max-w-site flex-col px-4 py-6 sm:px-6 lg:px-8">
      <div className="mb-5 flex flex-wrap items-end justify-between gap-4">
        <div className="max-w-2xl">
          <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
            KERNEXT Web
          </p>
          <h1 className="mt-1.5 text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
            Terminal
          </h1>
          <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
            A real shell on this machine, not a browser simulation. PTY mode
            runs the native <code className="rounded bg-surface-soft px-1 font-mono text-[11px]">kernext</code>{' '}
            process; Engine mode sends each line through the same Rust runtime
            in-process. Commands run against your local filesystem, so treat
            this like any other terminal you open.
          </p>
        </div>

        <div className="flex flex-col gap-2 rounded-xl border border-border bg-surface/50 px-3.5 py-3 text-xs">
          <span className="inline-flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
            <Circle className="size-2 fill-primary text-primary" />
            {accountsEnabled ? 'Account-scoped session' : 'Local runtime'}
          </span>
          <code className="font-mono text-[11px] text-foreground">
            http://127.0.0.1:7878
          </code>
          <Link
            to="/docs"
            hash="quick-start"
            className="inline-flex items-center gap-1 text-[11px] text-muted-foreground transition-colors hover:text-primary"
          >
            Setup notes
            <ArrowUpRight className="size-3" />
          </Link>
        </div>
      </div>

      <div className="w-full min-w-0">
        <TerminalWorkspace
          transport="ws"
          defaultMode="pty"
          customizable
        />
      </div>
    </div>
  );
}
