import { Link } from '@tanstack/react-router';
import { ArrowUpRight, TerminalSquare } from 'lucide-react';
import { Logo } from '@/components/site/logo';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';

export function SiteFooter() {
  return (
    <footer className="border-t border-border bg-surface/40">
      <div className="mx-auto w-full max-w-[1500px] px-4 py-12 sm:px-6 lg:px-8">
        <div className="grid gap-10 lg:grid-cols-[1.5fr_1fr_1fr_1.2fr]">
          <div className="space-y-4">
            <Logo />
            <p className="max-w-xs text-sm leading-relaxed text-muted-foreground">
              A native Rust shell engine with a real PTY, a local Axum runtime,
              and one terminal that runs on the CLI, web, and desktop.
            </p>
            <div className="flex flex-wrap gap-2">
              <Badge variant="success">Open</Badge>
              <Badge variant="default">Powerful</Badge>
              <Badge variant="secondary">Independent</Badge>
            </div>
          </div>

          <div>
            <h3 className="font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              Product
            </h3>
            <ul className="mt-4 space-y-2.5 text-sm">
              <li>
                <Link to="/terminal" className="text-muted-foreground transition-colors hover:text-foreground">
                  Web terminal
                </Link>
              </li>
              <li>
                <Link to="/" hash="platform" className="text-muted-foreground transition-colors hover:text-foreground">
                  Platform
                </Link>
              </li>
              <li>
                <Link to="/" hash="surfaces" className="text-muted-foreground transition-colors hover:text-foreground">
                  Surfaces
                </Link>
              </li>
              <li>
                <Link to="/docs" className="text-muted-foreground transition-colors hover:text-foreground">
                  Documentation
                </Link>
              </li>
            </ul>
          </div>

          <div>
            <h3 className="font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              Runtime
            </h3>
            <ul className="mt-4 space-y-2.5 text-sm">
              <li>
                <Link to="/docs" hash="api" className="text-muted-foreground transition-colors hover:text-foreground">
                  HTTP API
                </Link>
              </li>
              <li>
                <Link to="/docs" hash="websocket" className="text-muted-foreground transition-colors hover:text-foreground">
                  WebSocket protocol
                </Link>
              </li>
              <li>
                <Link to="/docs" hash="security" className="text-muted-foreground transition-colors hover:text-foreground">
                  Security model
                </Link>
              </li>
              <li>
                <Link to="/docs" hash="status" className="text-muted-foreground transition-colors hover:text-foreground">
                  Project status
                </Link>
              </li>
            </ul>
          </div>

          <div className="rounded-2xl border border-border bg-background/60 p-5">
            <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              Current build
            </p>
            <div className="mt-3 space-y-2.5 text-sm">
              <div className="flex items-center justify-between gap-3">
                <span className="text-muted-foreground">CLI</span>
                <Badge variant="success">Stable</Badge>
              </div>
              <div className="flex items-center justify-between gap-3">
                <span className="text-muted-foreground">Web</span>
                <Badge variant="default">New</Badge>
              </div>
              <div className="flex items-center justify-between gap-3">
                <span className="text-muted-foreground">Desktop</span>
                <Badge variant="warning">In progress</Badge>
              </div>
            </div>
            <Separator className="my-4" />
            <Link
              to="/terminal"
              className="inline-flex items-center gap-1.5 text-sm text-primary transition-colors hover:text-primary-strong"
            >
              <TerminalSquare className="size-4" />
              Run a live command
              <ArrowUpRight className="size-3.5" />
            </Link>
          </div>
        </div>

        <Separator className="my-8" />

        <div className="flex flex-col gap-4 text-xs text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
          <p>
            © {new Date().getFullYear()} KERNEXT. Build. Execute. Innovate.
          </p>
          <div className="flex flex-wrap items-center gap-4">
            <span className="font-mono uppercase tracking-[0.18em]">
              Open · Powerful · Independent
            </span>
            <span className="hidden h-3 w-px bg-border sm:block" />
            <span className="font-mono uppercase tracking-[0.18em]">
              Local-first runtime
            </span>
          </div>
        </div>
      </div>
    </footer>
  );
}
