import { TerminalWorkspace } from '@/components/terminal/terminal-workspace';

export function TerminalPage() {
  return (
    <div className="mx-auto w-full max-w-site px-4 py-8 sm:px-6 lg:px-8">
      <div className="mb-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
            KERNEXT Web
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
            Terminal
          </h1>
          <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
            A real PTY or engine session served by the local Axum runtime.
            Start the server, then connect.
          </p>
        </div>
      </div>

      <div className="h-[calc(100vh-220px)] min-h-[480px]">
        <TerminalWorkspace transport="ws" defaultMode="pty" />
      </div>
    </div>
  );
}
