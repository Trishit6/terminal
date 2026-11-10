import { Link } from '@tanstack/react-router';
import { ArrowUpRight, Circle } from 'lucide-react';
import { TerminalWorkspace } from '@/components/terminal/terminal-workspace';

export function TerminalPage() {
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
            Local runtime
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

      <div className="h-[calc(100dvh-15.5rem)] min-h-[520px]">
        <TerminalWorkspace transport="ws" defaultMode="pty" />
      </div>
    </div>
  );
}
