import { RefreshCw, ServerOff } from 'lucide-react';
import { useAuth } from '@/components/auth/auth-provider';

/**
 * Persistent, non-blocking status for the local API.
 *
 * It lives outside the router so every surface — marketing pages, account
 * screens, terminal, and admin — shows the same state. The terminal already
 * explains what to do inside its own workspace; this pill is the global
 * reminder that the API is unreachable, with a retry that re-probes it.
 */
export function BackendStatusIndicator() {
  const { ready, serverReachable, refresh } = useAuth();

  if (!ready || serverReachable) {
    return null;
  }

  return (
    <div
      role="status"
      aria-live="polite"
      className="fixed bottom-4 right-4 z-50 flex max-w-[calc(100vw-1.5rem)] items-center gap-2 rounded-full border border-danger/40 bg-surface/95 py-1 pl-3 pr-1 text-[11px] text-foreground shadow-2xl backdrop-blur"
    >
      <ServerOff className="size-3.5 shrink-0 text-danger" aria-hidden="true" />
      <span className="shrink-0 font-medium">Backend offline</span>
      {import.meta.env.DEV ? (
        <span className="hidden truncate text-muted-foreground sm:inline">
          · start it with <code className="font-mono">npm run dev</code>
        </span>
      ) : (
        <span className="hidden truncate text-muted-foreground sm:inline">
          · the API is unreachable
        </span>
      )}
      <button
        type="button"
        onClick={() => void refresh()}
        className="inline-flex h-7 shrink-0 items-center gap-1 rounded-full px-2 font-medium text-primary transition-colors hover:bg-primary/10 hover:text-primary-strong"
        aria-label="Retry backend connection"
      >
        <RefreshCw className="size-3" aria-hidden="true" />
        Retry
      </button>
    </div>
  );
}
