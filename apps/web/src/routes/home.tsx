import { Link } from '@tanstack/react-router';
import {
  ArrowRight,
  ArrowUpRight,
  Braces,
  Check,
  Cpu,
  Gauge,
  Globe2,
  Layers3,
  LockKeyhole,
  Monitor,
  Network,
  Orbit,
  Play,
  Server,
  ShieldCheck,
  Sparkles,
  Terminal,
  TerminalSquare,
  Waves,
  Zap,
} from 'lucide-react';
import { motion } from 'motion/react';
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
import { LogoMark } from '@/components/site/logo';
import { cn } from '@/lib/utils';

const EASE = [0.22, 1, 0.36, 1] as const;

const fadeUp = {
  initial: { opacity: 0, y: 18 },
  whileInView: { opacity: 1, y: 0 },
  viewport: { once: true, margin: '-80px' },
  transition: { duration: 0.6, ease: EASE },
};

const FEATURES = [
  {
    icon: Cpu,
    title: 'Native lexer and parser',
    description:
      'Input becomes a real AST. Pipelines, redirections, groups, and assignments are parsed by KERNEXT, never split with whitespace or handed to another shell.',
    label: 'Engine',
  },
  {
    icon: TerminalSquare,
    title: 'A real pseudoterminal',
    description:
      'ConPTY on Windows and openpty on Unix. Full-screen programs, resize, Ctrl+C, and raw terminal bytes stream over a dedicated WebSocket.',
    label: 'PTY',
  },
  {
    icon: Server,
    title: 'Local Axum runtime',
    description:
      'Sessions live in Rust. REST creates and inspects them; WebSockets execute commands or attach to the native shell process.',
    label: 'Server',
  },
  {
    icon: Layers3,
    title: 'One engine, three surfaces',
    description:
      'The CLI, web terminal, and desktop app all resolve to the same Dispatcher and the same built-in registry. No behavior forks per UI.',
    label: 'Shared core',
  },
  {
    icon: Braces,
    title: 'Scripting that stays native',
    description:
      'Variables, exports, functions, loops, conditionals, pipelines, and redirections are evaluated in-process with bounded recursion and iteration.',
    label: 'Language',
  },
  {
    icon: LockKeyhole,
    title: 'Local-first by default',
    description:
      'Loopback binding, an explicit origin allowlist, session caps, idle cleanup, command-size limits, and optional bearer tokens for local use.',
    label: 'Safety',
  },
] as const;

const SURFACES = [
  {
    icon: Terminal,
    title: 'KERNEXT CLI',
    status: 'Implemented',
    tone: 'success' as const,
    description:
      'The original surface. A Reedline prompt, cinematic startup, native pipelines, background jobs, and 53 built-ins.',
    points: ['`kernext` executable', 'Animated startup + config', 'Native process execution'],
    to: '/docs',
    action: 'Read the CLI notes',
  },
  {
    icon: Globe2,
    title: 'KERNEXT Web',
    status: 'Implemented',
    tone: 'default' as const,
    description:
      'A React terminal with its own ANSI renderer, cursor, scrollback, keyboard mapping, and resize handling. No xterm.js.',
    points: ['PTY and engine transports', 'Custom ANSI screen buffer', 'Local Axum session API'],
    to: '/terminal',
    action: 'Open the terminal',
  },
  {
    icon: Monitor,
    title: 'KERNEXT Desktop',
    status: 'In progress',
    tone: 'warning' as const,
    description:
      'A Tauri 2 shell around the same local runtime. The desktop surface is being prepared; do not treat it as shipped.',
    points: ['Tauri 2 host planned', 'Same web terminal UI', 'Native process bridge'],
    to: '/docs',
    action: 'See the status',
  },
] as const;

const FLOW = [
  {
    icon: TerminalSquare,
    title: 'Custom terminal',
    detail: 'Keyboard → ANSI buffer → WebSocket frames',
  },
  {
    icon: Network,
    title: 'WebSocket bridge',
    detail: 'Raw PTY bytes or one command line at a time',
  },
  {
    icon: Server,
    title: 'Axum + Tokio',
    detail: 'Sessions, auth, limits, and upgrade handling',
  },
  {
    icon: Cpu,
    title: 'KERNEXT engine',
    detail: 'Lex → parse → AST → dispatcher → outcome',
  },
] as const;

export function HomePage() {
  return (
    <div className="overflow-hidden">
      <Hero />
      <ValueStrip />
      <PlatformSection />
      <ArchitectureSection />
      <SurfacesSection />
      <RuntimeSection />
      <CallToAction />
    </div>
  );
}

function Hero() {
  return (
    <section className="relative isolate overflow-hidden pb-20 pt-16 sm:pt-24 lg:pb-28">
      <div className="absolute inset-0 -z-10 aurora" />
      <div className="absolute inset-x-0 top-0 -z-10 h-[560px] shell-grid opacity-60" />
      <div className="pointer-events-none absolute left-1/2 top-0 -z-10 h-[480px] w-[860px] -translate-x-1/2 rounded-full bg-primary/10 blur-[120px]" />

      <div className="mx-auto grid w-full max-w-[1500px] items-center gap-14 px-4 sm:px-6 lg:grid-cols-[1.05fr_0.95fr] lg:px-8">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.7, ease: EASE }}
          className="max-w-2xl"
        >
          <div className="mb-6 flex flex-wrap items-center gap-2">
            <Badge variant="outline" className="gap-2 pl-2">
              <span className="relative flex size-1.5">
                <span className="absolute inline-flex size-full animate-ping rounded-full bg-primary opacity-70" />
                <span className="relative inline-flex size-1.5 rounded-full bg-primary" />
              </span>
              Local Rust runtime
            </Badge>
            <Badge variant="secondary">OPEN • POWERFUL • INDEPENDENT</Badge>
          </div>

          <h1 className="text-balance text-5xl font-semibold leading-[0.98] tracking-[-0.045em] text-foreground sm:text-6xl lg:text-7xl">
            The next core of{' '}
            <span className="text-gradient">computing.</span>
          </h1>

          <p className="mt-6 max-w-xl text-base leading-relaxed text-muted-foreground sm:text-lg">
            KERNEXT is a native Rust shell engine with a real PTY, a local Axum
            runtime, and a custom terminal that runs the same code in your
            browser. No simulated shell. No Bash, CMD, or PowerShell wrapper.
          </p>

          <div className="mt-8 flex flex-col gap-3 sm:flex-row sm:items-center">
            <Button asChild size="lg" className="group">
              <Link to="/terminal">
                <Play className="size-4 fill-current" />
                Open the terminal
                <ArrowRight className="size-4 transition-transform group-hover:translate-x-0.5" />
              </Link>
            </Button>
            <Button asChild variant="outline" size="lg">
              <Link to="/docs">
                Read the architecture
                <ArrowUpRight className="size-4" />
              </Link>
            </Button>
          </div>

          <div className="mt-8 flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-muted-foreground">
            <span className="inline-flex items-center gap-1.5">
              <Check className="size-3.5 text-success" />
              Same Rust engine as the CLI
            </span>
            <span className="inline-flex items-center gap-1.5">
              <Check className="size-3.5 text-success" />
              Real ConPTY / openpty
            </span>
            <span className="inline-flex items-center gap-1.5">
              <Check className="size-3.5 text-success" />
              Local-only defaults
            </span>
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, scale: 0.96, y: 24 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          transition={{ duration: 0.85, ease: EASE, delay: 0.08 }}
          className="relative mx-auto w-full max-w-[580px]"
        >
          <div className="pointer-events-none absolute -inset-10 -z-10 rounded-full bg-primary/10 blur-3xl" />
          <div className="relative mx-auto aspect-square w-[72%] max-w-[360px]">
            <div className="orbital h-full w-full">
              <div className="flex size-[46%] items-center justify-center rounded-full border border-primary/20 bg-background/70 shadow-[0_0_80px_rgba(34,211,238,0.18)] backdrop-blur-xl">
                <LogoMark className="size-20 sm:size-24" />
              </div>
              <span className="orbital-dot" />
            </div>
            <FloatingChip className="-left-6 top-10" icon={Cpu} label="Rust engine" />
            <FloatingChip className="-right-4 top-24" icon={Waves} label="ConPTY" />
            <FloatingChip className="bottom-4 left-2" icon={Network} label="WebSocket" />
          </div>

          <div className="relative mt-[-12%] overflow-hidden rounded-2xl border border-border-strong bg-[#04070c] shadow-[0_40px_120px_-40px_rgba(2,8,23,0.95)]">
            <div className="flex items-center justify-between border-b border-white/10 px-4 py-2.5">
              <div className="flex items-center gap-1.5">
                <span className="size-2 rounded-full bg-[#ff6b6b]/80" />
                <span className="size-2 rounded-full bg-[#f5d76e]/80" />
                <span className="size-2 rounded-full bg-[#5ee6a8]/80" />
              </div>
              <span className="font-mono text-[10px] uppercase tracking-[0.2em] text-white/40">
                Example session
              </span>
              <span className="font-mono text-[10px] text-white/30">local</span>
            </div>
            <div className="space-y-1 p-4 font-mono text-[11px] leading-relaxed text-[#d8e6f5] sm:text-xs">
              <TerminalLine prompt="kernext ~ $" command="version" />
              <div className="text-[#67e8f9]">KERNEXT v0.1.0</div>
              <TerminalLine prompt="kernext ~ $" command="echo hello-from-kernext" />
              <div className="text-[#c7d2fe]">hello-from-kernext</div>
              <TerminalLine prompt="kernext ~ $" command="ls" />
              <div className="text-[#9fb3c8]">apps&nbsp;&nbsp;crates&nbsp;&nbsp;docs&nbsp;&nbsp;installers</div>
              <div className="flex items-center gap-1 pt-1">
                <span className="text-[#67e8f9]">kernext ~ $</span>
                <span className="inline-block h-4 w-2 animate-pulse bg-[#67e8f9]" />
              </div>
            </div>
          </div>
        </motion.div>
      </div>
    </section>
  );
}

function FloatingChip({
  className,
  icon: Icon,
  label,
}: {
  className?: string;
  icon: typeof Cpu;
  label: string;
}) {
  return (
    <div
      className={cn(
        'absolute z-10 hidden items-center gap-2 rounded-full border border-border-strong bg-surface/85 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground shadow-xl backdrop-blur-md sm:flex',
        className,
      )}
    >
      <Icon className="size-3.5 text-primary" />
      {label}
    </div>
  );
}

function TerminalLine({ prompt, command }: { prompt: string; command: string }) {
  return (
    <div>
      <span className="text-[#67e8f9]">{prompt}</span>{' '}
      <span className="text-white/90">{command}</span>
    </div>
  );
}

function ValueStrip() {
  const values = [
    { value: '53', label: 'native built-ins' },
    { value: '1', label: 'shared Rust engine' },
    { value: '3', label: 'runtime surfaces' },
    { value: '0', label: 'wrapper shells' },
  ];

  return (
    <section className="border-y border-border bg-surface/30">
      <div className="mx-auto grid w-full max-w-[1500px] grid-cols-2 divide-x divide-border px-4 sm:px-6 lg:grid-cols-4 lg:px-8">
        {values.map((item) => (
          <div key={item.label} className="px-4 py-6 text-center sm:py-8">
            <div className="font-mono text-3xl font-semibold tracking-[-0.04em] text-foreground">
              {item.value}
            </div>
            <div className="mt-1 font-mono text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
              {item.label}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

function PlatformSection() {
  return (
    <section id="platform" className="relative py-20 sm:py-28">
      <div className="mx-auto w-full max-w-[1500px] px-4 sm:px-6 lg:px-8">
        <motion.div {...fadeUp} className="max-w-3xl">
          <Badge variant="outline">Platform</Badge>
          <h2 className="mt-4 text-balance text-3xl font-semibold tracking-[-0.03em] sm:text-4xl lg:text-5xl">
            A shell engine, not a shell wrapper.
          </h2>
          <p className="mt-4 max-w-2xl text-sm leading-relaxed text-muted-foreground sm:text-base">
            Every command crosses the same native pipeline: a lexer, a parser,
            an AST, and a dispatcher built in Rust. Pipelines and redirections
            stay in-process. External programs launch through native process
            APIs, never through another shell.
          </p>
        </motion.div>

        <div className="mt-12 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {FEATURES.map((feature, index) => (
            <motion.div
              key={feature.title}
              initial={{ opacity: 0, y: 18 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: '-60px' }}
              transition={{ duration: 0.5, ease: EASE, delay: index * 0.05 }}
            >
              <Card className="group h-full overflow-hidden border-border/80 bg-surface/55 transition-colors hover:border-primary/30 hover:bg-surface">
                <div className="h-px w-full bg-gradient-to-r from-transparent via-primary/40 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />
                <CardHeader className="gap-3">
                  <div className="flex items-center justify-between">
                    <div className="grid size-9 place-items-center rounded-xl border border-border bg-background/70 text-primary">
                      <feature.icon className="size-4" />
                    </div>
                    <span className="font-mono text-[9px] uppercase tracking-[0.18em] text-muted-foreground">
                      {feature.label}
                    </span>
                  </div>
                  <CardTitle>{feature.title}</CardTitle>
                </CardHeader>
                <CardContent>
                  <CardDescription>{feature.description}</CardDescription>
                </CardContent>
              </Card>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}

function ArchitectureSection() {
  return (
    <section id="architecture" className="relative overflow-hidden border-y border-border bg-surface/25 py-20 sm:py-28">
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_20%_20%,rgba(34,211,238,0.08),transparent_35%),radial-gradient(circle_at_80%_60%,rgba(139,92,246,0.08),transparent_35%)]" />
      <div className="relative mx-auto grid w-full max-w-[1500px] gap-12 px-4 sm:px-6 lg:grid-cols-[0.9fr_1.1fr] lg:items-center lg:px-8">
        <motion.div {...fadeUp}>
          <Badge variant="outline">Architecture</Badge>
          <h2 className="mt-4 text-balance text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
            From keystroke to kernel.
          </h2>
          <p className="mt-4 max-w-lg text-sm leading-relaxed text-muted-foreground sm:text-base">
            The browser owns presentation. The server owns the session. The
            engine owns semantics. A raw PTY transport is available when a
            program needs the whole screen &mdash; not just a line of output.
          </p>

          <div className="mt-8 space-y-3">
            {FLOW.map((step, index) => (
              <div key={step.title} className="flex gap-3">
                <div className="flex flex-col items-center">
                  <div className="grid size-9 shrink-0 place-items-center rounded-xl border border-border bg-background/70 text-primary">
                    <step.icon className="size-4" />
                  </div>
                  {index < FLOW.length - 1 ? (
                    <div className="mt-1 h-6 w-px bg-border" />
                  ) : null}
                </div>
                <div className="pt-1">
                  <div className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
                    Step {index + 1}
                  </div>
                  <div className="mt-0.5 text-sm font-medium">{step.title}</div>
                  <div className="mt-0.5 text-xs leading-relaxed text-muted-foreground">
                    {step.detail}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, x: 20 }}
          whileInView={{ opacity: 1, x: 0 }}
          viewport={{ once: true, margin: '-80px' }}
          transition={{ duration: 0.7, ease: EASE }}
          className="space-y-4"
        >
          <Card className="overflow-hidden border-border-strong bg-[#04070c] text-[#d8e6f5]">
            <div className="flex items-center justify-between border-b border-white/10 px-4 py-3">
              <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.18em] text-white/45">
                <ShieldCheck className="size-3.5 text-[#5ee6a8]" />
                Transport map
              </div>
              <span className="font-mono text-[10px] text-white/30">v1</span>
            </div>
            <pre className="overflow-x-auto p-4 font-mono text-[11px] leading-relaxed sm:text-xs">
              <code>{`POST /api/v1/sessions        create a Rust session
GET  /api/v1/sessions        list live sessions
POST /api/v1/sessions/:id/execute
WS   /api/v1/sessions/:id/ws engine transport
WS   /api/v1/pty             native PTY transport

client { type: "execute", command }
client { type: "input", data }
client { type: "resize", rows, cols }
server { type: "output", stream, data }
server { type: "result", action, session }`}</code>
            </pre>
          </Card>

          <div className="grid gap-4 sm:grid-cols-2">
            <Card className="p-5">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Gauge className="size-4 text-primary" />
                Bounded by design
              </div>
              <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
                Session caps, idle cleanup, a 64 KiB command limit, and bounded
                recursion and loop iterations keep the local runtime honest.
              </p>
            </Card>
            <Card className="p-5">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Orbit className="size-4 text-accent" />
                No shell inside
              </div>
              <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
                KERNEXT passes argument vectors to native process APIs. It never
                builds a command string for Bash, CMD, or PowerShell to parse.
              </p>
            </Card>
          </div>
        </motion.div>
      </div>
    </section>
  );
}

function SurfacesSection() {
  return (
    <section id="surfaces" className="py-20 sm:py-28">
      <div className="mx-auto w-full max-w-[1500px] px-4 sm:px-6 lg:px-8">
        <motion.div {...fadeUp} className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
          <div className="max-w-2xl">
            <Badge variant="outline">Surfaces</Badge>
            <h2 className="mt-4 text-balance text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
              One engine. Three ways in.
            </h2>
            <p className="mt-4 text-sm leading-relaxed text-muted-foreground sm:text-base">
              The CLI is the reference surface. The web terminal is live in this
              repository. The desktop host is the next surface, not a shipped
              claim.
            </p>
          </div>
          <Button asChild variant="outline" size="sm">
            <Link to="/docs" hash="status">
              View verified status
              <ArrowUpRight className="size-3.5" />
            </Link>
          </Button>
        </motion.div>

        <div className="mt-12 grid gap-5 lg:grid-cols-3">
          {SURFACES.map((surface, index) => (
            <motion.div
              key={surface.title}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: '-60px' }}
              transition={{ duration: 0.55, ease: EASE, delay: index * 0.07 }}
            >
              <Card className="flex h-full flex-col bg-surface/55">
                <CardHeader className="gap-4">
                  <div className="flex items-center justify-between">
                    <div className="grid size-10 place-items-center rounded-xl border border-border bg-background/70 text-primary">
                      <surface.icon className="size-5" />
                    </div>
                    <Badge variant={surface.tone}>{surface.status}</Badge>
                  </div>
                  <div>
                    <CardTitle className="text-base">{surface.title}</CardTitle>
                    <CardDescription className="mt-2">
                      {surface.description}
                    </CardDescription>
                  </div>
                </CardHeader>
                <CardContent className="mt-auto space-y-2.5">
                  {surface.points.map((point) => (
                    <div key={point} className="flex items-start gap-2 text-xs text-muted-foreground">
                      <Check className="mt-0.5 size-3.5 shrink-0 text-success" />
                      {point}
                    </div>
                  ))}
                  <Separator className="my-4" />
                  <Link
                    to={surface.to}
                    className="inline-flex items-center gap-1.5 text-xs font-medium text-primary transition-colors hover:text-primary-strong"
                  >
                    {surface.action}
                    <ArrowRight className="size-3.5" />
                  </Link>
                </CardContent>
              </Card>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}

function RuntimeSection() {
  const items = [
    { icon: Zap, title: 'Native pipelines', detail: 'OS pipes between external stages, buffered bytes between built-ins.' },
    { icon: Globe2, title: 'Real WebSockets', detail: 'Backpressure-aware relay with binary PTY frames and JSON control frames.' },
    { icon: Cpu, title: 'Background jobs', detail: 'A shared job table with jobs, wait, kill, fg, and bg built-ins.' },
    { icon: Sparkles, title: 'Cinematic startup', detail: 'A palette-driven emblem and wordmark that stays text-only in the CLI.' },
  ];

  return (
    <section className="border-y border-border bg-surface/25 py-20 sm:py-24">
      <div className="mx-auto grid w-full max-w-[1500px] gap-10 px-4 sm:px-6 lg:grid-cols-[1fr_1.1fr] lg:items-center lg:px-8">
        <motion.div {...fadeUp}>
          <Badge variant="outline">Runtime</Badge>
          <h2 className="mt-4 text-balance text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
            Built for the terminal, not around it.
          </h2>
          <p className="mt-4 text-sm leading-relaxed text-muted-foreground sm:text-base">
            The web app does not pretend to be a terminal. It renders a real
            screen buffer, maps keys to terminal sequences, resizes the PTY, and
            shows exactly what the Rust process sends back.
          </p>
          <div className="mt-6 flex flex-wrap gap-2">
            <Badge variant="secondary">React 19</Badge>
            <Badge variant="secondary">Vite</Badge>
            <Badge variant="secondary">Tailwind v4</Badge>
            <Badge variant="secondary">TanStack Router</Badge>
            <Badge variant="secondary">Motion</Badge>
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-80px' }}
          transition={{ duration: 0.7, ease: EASE }}
          className="grid gap-3 sm:grid-cols-2"
        >
          {items.map((item) => (
            <Card key={item.title} className="p-5">
              <div className="flex items-center gap-2 text-sm font-medium">
                <item.icon className="size-4 text-primary" />
                {item.title}
              </div>
              <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
                {item.detail}
              </p>
            </Card>
          ))}
        </motion.div>
      </div>
    </section>
  );
}

function CallToAction() {
  return (
    <section className="py-20 sm:py-24">
      <div className="mx-auto w-full max-w-[1500px] px-4 sm:px-6 lg:px-8">
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-80px' }}
          transition={{ duration: 0.65, ease: EASE }}
          className="relative overflow-hidden rounded-3xl border border-border-strong bg-surface p-8 sm:p-12 lg:p-16"
        >
          <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_15%_20%,rgba(34,211,238,0.18),transparent_38%),radial-gradient(circle_at_85%_80%,rgba(139,92,246,0.18),transparent_38%)]" />
          <div className="relative flex flex-col gap-8 lg:flex-row lg:items-center lg:justify-between">
            <div className="max-w-2xl">
              <Badge variant="outline">Build. Execute. Innovate.</Badge>
              <h2 className="mt-4 text-balance text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
                The next core is local.
              </h2>
              <p className="mt-3 max-w-xl text-sm leading-relaxed text-muted-foreground sm:text-base">
                Start the server, open the terminal, and run a command through
                the same Rust engine that powers the CLI.
              </p>
            </div>
            <div className="flex flex-col gap-3 sm:flex-row">
              <Button asChild size="lg">
                <Link to="/terminal">
                  <TerminalSquare className="size-4" />
                  Open terminal
                </Link>
              </Button>
              <Button asChild variant="outline" size="lg">
                <Link to="/docs">
                  Read the docs
                  <ArrowUpRight className="size-4" />
                </Link>
              </Button>
            </div>
          </div>
        </motion.div>
      </div>
    </section>
  );
}
