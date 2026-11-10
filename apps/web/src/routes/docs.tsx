import { Link } from '@tanstack/react-router';
import {
  ArrowUpRight,
  Check,
  Clipboard,
  ClipboardCheck,
  ExternalLink,
  LockKeyhole,
  Server,
  ShieldCheck,
  TerminalSquare,
} from 'lucide-react';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui/tabs';
import { cn } from '@/lib/utils';

export function DocsPage() {
  return (
    <div className="mx-auto w-full max-w-[1100px] px-4 py-14 sm:px-6 lg:px-8 lg:py-20">
      <div className="max-w-3xl">
        <Badge variant="outline">Documentation</Badge>
        <h1 className="mt-4 text-balance text-4xl font-semibold tracking-[-0.04em] sm:text-5xl">
          One runtime. Three surfaces.
          <span className="text-gradient"> No simulated shell.</span>
        </h1>
        <p className="mt-5 text-sm leading-relaxed text-muted-foreground sm:text-base">
          KERNEXT runs a native Rust engine on your machine. This page covers
          the local server, both WebSocket transports, the HTTP API, the
          security model, and the current verification status.
        </p>
      </div>

      <div className="mt-10 flex flex-wrap gap-3">
        <Button asChild>
          <Link to="/terminal">
            <TerminalSquare className="size-4" />
            Open the web terminal
          </Link>
        </Button>
        <Button asChild variant="outline">
          <Link to="/" hash="architecture">
            Architecture overview
            <ArrowUpRight className="size-4" />
          </Link>
        </Button>
      </div>

      <Separator className="my-12" />

      <section id="quickstart" className="scroll-mt-24">
        <SectionTitle
          eyebrow="Quick start"
          title="Run the engine locally"
          description="The web terminal needs two processes: the Rust server and the Vite client. The PTY transport also needs a built kernext executable."
        />

        <Tabs defaultValue="web" className="mt-6">
          <TabsList>
            <TabsTrigger value="web">Web terminal</TabsTrigger>
            <TabsTrigger value="cli">CLI</TabsTrigger>
            <TabsTrigger value="server">Server only</TabsTrigger>
          </TabsList>

          <TabsContent value="web" className="space-y-4">
            <CodeBlock
              title="Terminal 1 — build and run the local server"
              code={`cargo build -p shell-cli -p shell-server
cargo run -p shell-server`}
            />
            <CodeBlock
              title="Terminal 2 — start the Vite client"
              code={`pnpm install
pnpm web:dev`}
            />
            <p className="text-xs leading-relaxed text-muted-foreground">
              Open <span className="font-mono text-foreground">http://127.0.0.1:5173/terminal</span>.
              The Vite dev server proxies <span className="font-mono">/api</span> to
              the Axum server on <span className="font-mono">127.0.0.1:7878</span>.
            </p>
          </TabsContent>

          <TabsContent value="cli">
            <CodeBlock
              title="Run the reference surface"
              code={`cargo build -p shell-cli
cargo run -p shell-cli

# or launch the built binary directly
./target/debug/kernext`}
            />
            <p className="mt-4 text-xs leading-relaxed text-muted-foreground">
              The CLI requires an interactive terminal. Piped input is rejected
              by design because KERNEXT is an interactive shell.
            </p>
          </TabsContent>

          <TabsContent value="server">
            <CodeBlock
              title="Run the Axum backend"
              code={`cargo run -p shell-server

# optional
KERNEXT_SERVER_BIND=127.0.0.1:7878
KERNEXT_SERVER_TOKEN=local-dev-token
KERNEXT_MAX_SESSIONS=16
KERNEXT_BIN=/absolute/path/to/kernext`}
            />
            <p className="mt-4 text-xs leading-relaxed text-muted-foreground">
              The server reads these environment variables at startup. The token
              is a local control, not a multi-user authorization system.
            </p>
          </TabsContent>
        </Tabs>
      </section>

      <Separator className="my-12" />

      <section id="api" className="scroll-mt-24">
        <SectionTitle
          eyebrow="HTTP API"
          title="Sessions over REST"
          description="Every endpoint below is served by the local Axum runtime. Mutating routes require a bearer token when KERNEXT_SERVER_TOKEN is configured."
        />

        <div className="mt-6 overflow-hidden rounded-2xl border border-border">
          <table className="w-full text-left text-xs sm:text-sm">
            <thead className="bg-surface-soft text-muted-foreground">
              <tr>
                <th className="px-4 py-3 font-medium">Method</th>
                <th className="px-4 py-3 font-medium">Path</th>
                <th className="px-4 py-3 font-medium">Purpose</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border font-mono text-[11px] sm:text-xs">
              {[
                ['GET', '/api/v1/health', 'Runtime status, version, platform, PTY availability'],
                ['GET', '/api/v1/sessions', 'List live sessions'],
                ['POST', '/api/v1/sessions', 'Create a Rust session'],
                ['GET', '/api/v1/sessions/{id}', 'Read one session snapshot'],
                ['DELETE', '/api/v1/sessions/{id}', 'Stop and remove a session'],
                ['POST', '/api/v1/sessions/{id}/execute', 'Execute one command line'],
                ['GET', '/api/v1/sessions/{id}/ws', 'Upgrade to the engine WebSocket'],
                ['GET', '/api/v1/pty', 'Upgrade to the native PTY WebSocket'],
              ].map(([method, path, purpose]) => (
                <tr key={`${method}-${path}`} className="bg-surface/40">
                  <td className="px-4 py-3 text-primary">{method}</td>
                  <td className="px-4 py-3 text-foreground">{path}</td>
                  <td className="px-4 py-3 font-sans text-muted-foreground">{purpose}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div className="mt-6 grid gap-4 lg:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Server className="size-4 text-primary" />
                Create a session
              </CardTitle>
              <CardDescription>
                An empty body starts in the server process directory with the
                default KERNEXT configuration.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <CodeBlock
                compact
                code={`curl -s http://127.0.0.1:7878/api/v1/sessions \\
  -H 'content-type: application/json' \\
  -d '{"working_directory":"/absolute/path"}'`}
              />
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <TerminalSquare className="size-4 text-primary" />
                Execute a command
              </CardTitle>
              <CardDescription>
                The response includes the combined output, exit code, action,
                and the updated session snapshot.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <CodeBlock
                compact
                code={`curl -s http://127.0.0.1:7878/api/v1/sessions/1/execute \\
  -H 'content-type: application/json' \\
  -d '{"command":"echo hello-from-kernext"}'`}
              />
            </CardContent>
          </Card>
        </div>
      </section>

      <Separator className="my-12" />

      <section id="websocket" className="scroll-mt-24">
        <SectionTitle
          eyebrow="WebSocket protocol"
          title="Two transports, one vocabulary"
          description="The engine socket executes command lines. The PTY socket relays raw terminal bytes. Control messages stay JSON in both directions."
        />

        <div className="mt-6 grid gap-4 lg:grid-cols-2">
          <Card className="border-primary/20 bg-surface/55">
            <CardHeader>
              <div className="flex items-center justify-between gap-3">
                <CardTitle>Engine transport</CardTitle>
                <Badge variant="default">Line-oriented</Badge>
              </div>
              <CardDescription>
                Best for command execution, session inspection, and scripting.
                Output is combined text, not a full-screen emulator.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <CodeBlock
                compact
                code={`// client
{ "type": "execute", "command": "echo hello" }

// server
{ "type": "ready", "session": { ... }, "prompt": "kernext ~ $ " }
{ "type": "output", "stream": "stdout", "data": "hello\\n" }
{ "type": "result", "success": true, "exit_code": 0,
  "action": "continue", "session": { ... } }`}
              />
            </CardContent>
          </Card>

          <Card className="border-accent/20 bg-surface/55">
            <CardHeader>
              <div className="flex items-center justify-between gap-3">
                <CardTitle>PTY transport</CardTitle>
                <Badge variant="warning">Raw bytes</Badge>
              </div>
              <CardDescription>
                Best for the real interactive shell: Reedline, colours, resize,
                Ctrl+C, and full-screen programs.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <CodeBlock
                compact
                code={`// client control messages
{ "type": "input", "data": "ls\\r" }
{ "type": "resize", "rows": 36, "cols": 120 }
{ "type": "interrupt" }
{ "type": "clear" }

// server
{ "type": "pty_ready", "prompt": "kernext ~ $ " }
<binary frames: raw UTF-8 terminal output>`}
              />
            </CardContent>
          </Card>
        </div>

        <div className="mt-6 rounded-2xl border border-border bg-surface/40 p-5">
          <p className="text-xs leading-relaxed text-muted-foreground">
            <span className="font-medium text-foreground">Authentication note:</span>{' '}
            browsers cannot set an <span className="font-mono">Authorization</span>{' '}
            header on a WebSocket. When a token is enabled, the web client appends
            it as <span className="font-mono">?token=...</span>. The server accepts
            that query parameter only for the two WebSocket upgrade routes.
          </p>
        </div>
      </section>

      <Separator className="my-12" />

      <section id="terminal" className="scroll-mt-24">
        <SectionTitle
          eyebrow="Web terminal"
          title="A custom terminal, not an embedded one"
          description="The web terminal is a React screen buffer with a VT-style parser. It is intentionally not xterm.js."
        />

        <div className="mt-6 grid gap-4 sm:grid-cols-2">
          {[
            {
              title: 'ANSI screen buffer',
              body: 'Cursor addressing, erase/insert/delete, scroll regions, 16/256/24-bit colours, alternate screen, and scrollback.',
            },
            {
              title: 'Keyboard mapping',
              body: 'Printable input, arrows, Home/End, Insert/Delete, Page Up/Down, F1–F12, Ctrl combinations, Alt sequences, and paste.',
            },
            {
              title: 'Resize',
              body: 'Cell metrics are measured from the live font. The PTY receives rows and columns on every meaningful size change.',
            },
            {
              title: 'Batched rendering',
              body: 'Parser writes are collected in an animation frame and published through an external store, so a fast stream does not force one React render per frame.',
            },
          ].map((item) => (
            <Card key={item.title} className="p-5">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Check className="size-4 text-success" />
                {item.title}
              </div>
              <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
                {item.body}
              </p>
            </Card>
          ))}
        </div>
      </section>

      <Separator className="my-12" />

      <section id="security" className="scroll-mt-24">
        <SectionTitle
          eyebrow="Security"
          title="Local by default, explicit by design"
          description="KERNEXT executes real commands. The server is a local development control surface, not a public multi-tenant service."
        />

        <div className="mt-6 grid gap-4 lg:grid-cols-2">
          <Card className="p-5">
            <div className="flex items-center gap-2 text-sm font-medium">
              <ShieldCheck className="size-4 text-success" />
              What is enforced
            </div>
            <ul className="mt-3 space-y-2 text-xs leading-relaxed text-muted-foreground">
              <li>Loopback bind by default (127.0.0.1:7878).</li>
              <li>Explicit browser origin allowlist for CORS and upgrades.</li>
              <li>Optional constant-time bearer token check.</li>
              <li>Session cap, idle cleanup, and a 64 KiB command limit.</li>
              <li>Output capture uses a bounded in-process session.</li>
            </ul>
          </Card>
          <Card className="p-5">
            <div className="flex items-center gap-2 text-sm font-medium">
              <LockKeyhole className="size-4 text-warning" />
              What is not claimed
            </div>
            <ul className="mt-3 space-y-2 text-xs leading-relaxed text-muted-foreground">
              <li>No multi-user authorization or tenant isolation.</li>
              <li>No sandbox around external programs you launch.</li>
              <li>No TLS termination in the Rust server.</li>
              <li>No guarantee that a public deployment is safe without a proxy.</li>
              <li>The local token is a control, not a security boundary.</li>
            </ul>
          </Card>
        </div>
      </section>

      <Separator className="my-12" />

      <section id="status" className="scroll-mt-24">
        <SectionTitle
          eyebrow="Status"
          title="What is actually verified"
          description="Status labels are deliberately conservative. A feature is only marked verified after a real build or test run."
        />

        <div className="mt-6 overflow-hidden rounded-2xl border border-border">
          <table className="w-full text-left text-xs sm:text-sm">
            <thead className="bg-surface-soft text-muted-foreground">
              <tr>
                <th className="px-4 py-3 font-medium">Surface</th>
                <th className="px-4 py-3 font-medium">Status</th>
                <th className="px-4 py-3 font-medium">Notes</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {[
                ['CLI engine', 'Tested', 'Existing Rust test suite and platform PTY tests.'],
                ['Axum server', 'Tested', 'REST and WebSocket integration tests in apps/shell-server/tests.'],
                ['Web terminal', 'Tested', 'Build, typecheck, unit tests, and consistency suite over the real engine.'],
                ['Desktop (Tauri 2)', 'Implemented', 'Debug bundle build and unit tests verified on Windows; Linux/macOS builds pending hosts.'],
              ].map(([surface, status, notes]) => (
                <tr key={surface} className="bg-surface/40">
                  <td className="px-4 py-3 font-medium">{surface}</td>
                  <td className="px-4 py-3">
                    <Badge
                      variant={
                        status === 'Tested' || status === 'Implemented'
                          ? 'success'
                          : status === 'In progress'
                            ? 'warning'
                            : 'secondary'
                      }
                    >
                      {status}
                    </Badge>
                  </td>
                  <td className="px-4 py-3 text-muted-foreground">{notes}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div className="mt-6 rounded-2xl border border-border bg-surface/40 p-5">
          <p className="text-xs leading-relaxed text-muted-foreground">
            Known limits: the engine transport cannot interrupt a running
            external command because the session socket awaits each execution.
            Use the PTY transport for full control. The PTY endpoint needs a
            built <span className="font-mono">kernext</span> binary beside the
            server executable or a valid{' '}
            <span className="font-mono">KERNEXT_BIN</span>. The desktop app
            bundles its own engine and does not need either server.
          </p>
        </div>
      </section>

      <Separator className="my-12" />

      <div className="flex flex-col items-start justify-between gap-4 rounded-2xl border border-border bg-surface/40 p-6 sm:flex-row sm:items-center">
        <div>
          <p className="text-sm font-medium">Ready to run a real command?</p>
          <p className="mt-1 text-xs text-muted-foreground">
            Start the local server and open the PTY terminal.
          </p>
        </div>
        <Button asChild>
          <Link to="/terminal">
            Open terminal
            <ExternalLink className="size-4" />
          </Link>
        </Button>
      </div>
    </div>
  );
}

function SectionTitle({
  eyebrow,
  title,
  description,
}: {
  eyebrow: string;
  title: string;
  description: string;
}) {
  return (
    <div className="max-w-3xl">
      <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
        {eyebrow}
      </div>
      <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
        {title}
      </h2>
      <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
        {description}
      </p>
    </div>
  );
}

function CodeBlock({
  code,
  title,
  compact = false,
}: {
  code: string;
  title?: string;
  compact?: boolean;
}) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    } catch {
      setCopied(false);
    }
  };

  return (
    <div className="overflow-hidden rounded-xl border border-border bg-[#04070c] text-[#d8e6f5]">
      {title ? (
        <div className="flex items-center justify-between gap-3 border-b border-white/10 px-4 py-2.5">
          <span className="font-mono text-[10px] uppercase tracking-[0.16em] text-white/45">
            {title}
          </span>
          <CopyButton copied={copied} onClick={copy} />
        </div>
      ) : (
        <div className="flex justify-end border-b border-white/10 px-3 py-2">
          <CopyButton copied={copied} onClick={copy} />
        </div>
      )}
      <pre className={cn('overflow-x-auto p-4 font-mono leading-relaxed', compact ? 'text-[10px]' : 'text-[11px] sm:text-xs')}>
        <code>{code}</code>
      </pre>
    </div>
  );
}

function CopyButton({
  copied,
  onClick,
}: {
  copied: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="inline-flex items-center gap-1.5 rounded-md border border-white/10 px-2 py-1 font-mono text-[9px] uppercase tracking-[0.14em] text-white/45 transition-colors hover:border-white/25 hover:text-white/75"
    >
      {copied ? <ClipboardCheck className="size-3" /> : <Clipboard className="size-3" />}
      {copied ? 'Copied' : 'Copy'}
    </button>
  );
}
