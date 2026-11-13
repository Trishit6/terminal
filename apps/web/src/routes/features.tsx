import { Link } from '@tanstack/react-router';
import {
  ArrowRight,
  Braces,
  BriefcaseBusiness,
  Check,
  Cpu,
  Gauge,
  History,
  Layers3,
  LockKeyhole,
  Network,
  Server,
  Settings2,
  ShieldCheck,
  Split,
  TerminalSquare,
} from 'lucide-react';
import { motion } from 'motion/react';
import { PageAsideRow, PageShell } from '@/components/site/page-shell';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';

const EASE = [0.22, 1, 0.36, 1] as const;

const GROUPS = [
  {
    id: 'engine',
    eyebrow: 'Shell engine',
    title: 'A real lexer, parser, and evaluator',
    icon: Cpu,
    summary:
      'Every line is lexed with spans, parsed into an AST, and evaluated in Rust. KERNEXT never splits input on whitespace and never delegates parsing to another shell.',
    items: [
      'Pipelines, redirections, groups, lists, and `&&`/`||` are AST constructs with defined precedence.',
      'Built-ins win over functions, and functions win over external programs.',
      'Syntax errors report cleanly with status 2 instead of crashing the session.',
    ],
  },
  {
    id: 'terminal',
    eyebrow: 'Terminal',
    title: 'A genuine pseudoterminal, everywhere',
    icon: TerminalSquare,
    summary:
      'ConPTY on Windows and openpty on Unix give full-screen programs a real TTY with size, resize, and interrupt support.',
    items: [
      'The CLI prompt runs on Reedline with history, completion menu, and genuine clear handling.',
      'The web terminal renders its own ANSI screen buffer — cursor addressing, erase, scroll regions, 24-bit colour, scrollback, alternate screen.',
      'Keyboard mapping covers arrows, Home/End, Insert/Delete, Page Up/Down, F1–F12, Ctrl, Alt, and paste.',
    ],
  },
  {
    id: 'processes',
    eyebrow: 'Processes',
    title: 'External programs, launched natively',
    icon: Split,
    summary:
      'Resolved executables are started with OS process APIs and an argument vector — one argument per word, boundaries preserved.',
    items: [
      'PATH resolution with PATHEXT handling on Windows; `.cmd`/`.bat` shims go through the system interpreter as a launcher, never as a parser.',
      'Exit codes, signal termination, and stdin/stdout/stderr are reported through one CommandOutcome model.',
      'Interactive commands run inside a PTY with streaming output and Ctrl+C.',
    ],
  },
  {
    id: 'scripting',
    eyebrow: 'Scripting',
    title: 'A native scripting runtime',
    icon: Braces,
    summary:
      'Variables, functions, loops, and conditionals are evaluated in-process with explicit bounds on recursion and iteration.',
    items: [
      '`$name` and `${name}` expansion, `$?` and `$$` special variables, exported and local layers.',
      '`if`/`else`, `while`, `for` over word lists and numeric ranges, functions with scoped parameters.',
      'Assignments, `export`, `unset`, `env`, and `set` built-ins with Windows-aware environment layering.',
    ],
  },
  {
    id: 'jobs',
    eyebrow: 'Jobs',
    title: 'Background jobs with a shared table',
    icon: BriefcaseBusiness,
    summary:
      'A job table shared across pipeline clones tracks running, done, failed, and killed jobs.',
    items: [
      '`jobs`, `wait`, `kill`, `fg`, and `bg` built-ins manage the table.',
      'Background evaluation is isolated: `exit` or `clear` in a job cannot control the foreground shell.',
      'Job IDs are shell job numbers, reported honestly — not OS process IDs.',
    ],
  },
  {
    id: 'config',
    eyebrow: 'Configuration',
    title: 'One TOML configuration across surfaces',
    icon: Settings2,
    summary:
      'Prompt, history, colors, animation, and fonts live in a validated TOML file with the same defaults everywhere.',
    items: [
      'Colors accept ANSI names, 256-color indices, and `#RRGGBB`.',
      'History size, secret filtering, suggestions, and hidden-file completion are configurable.',
      'Unknown fields are rejected; invalid files fall back to defaults with a warning.',
    ],
  },
  {
    id: 'safety',
    eyebrow: 'Safety',
    title: 'Local-first by design',
    icon: ShieldCheck,
    summary:
      'The runtime executes real commands, so the boundaries are explicit and enforced in code, not documentation.',
    items: [
      'The web server binds to loopback by default with an origin allowlist and optional bearer tokens.',
      'Session caps, idle cleanup, and a 64 KiB command limit bound the local API.',
      'The desktop app executes through its embedded Rust runtime — never through a public server.',
    ],
  },
] as const;

export function FeaturesPage() {
  return (
    <PageShell
      kicker="02 / Features"
      title={
        <>
          Everything the engine does. <em>Once, in Rust.</em>
        </>
      }
      description="KERNEXT ships one engine behind three surfaces. Every feature below is implemented in the shared crates and exercised by the automated test suite that runs on Windows, Linux, and macOS."
      actions={
        <>
          <Button asChild>
            <Link to="/terminal">
              <TerminalSquare className="size-4" />
              Open the web terminal
            </Link>
          </Button>
          <Button asChild variant="outline">
            <Link to="/docs">
              Read the documentation
              <ArrowRight className="size-4" />
            </Link>
          </Button>
        </>
      }
      aside={
        <>
          <PageAsideRow label="Engine" value="Rust workspace" />
          <PageAsideRow label="Surfaces" value="3" />
          <PageAsideRow label="Built-ins" value="56" />
          <PageAsideRow label="Status" value="Implemented" />
          <p className="page-aside__note">
            Claims map to <span className="font-mono">crates/shell-*</span> and
            the repository test suite.
          </p>
        </>
      }
    >

      <nav className="section-index" aria-label="Feature categories">
        <span className="section-index__label">Categories</span>
        <div className="section-index__scroller">
          {GROUPS.map((group) => (
            <a key={group.id} href={`#${group.id}`}>
              {group.eyebrow}
            </a>
          ))}
        </div>
      </nav>

      <div className="mt-14 space-y-16">
        {GROUPS.map((group, index) => (
          <motion.section
            key={group.id}
            id={group.id}
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: '-60px' }}
            transition={{ duration: 0.55, ease: EASE, delay: index * 0.03 }}
            className="scroll-mt-24"
          >
            <div className="flex items-start gap-4">
              <div className="grid size-11 shrink-0 place-items-center rounded-xl border border-border bg-background/70 text-primary">
                <group.icon className="size-5" />
              </div>
              <div className="max-w-3xl">
                <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
                  {group.eyebrow}
                </div>
                <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
                  {group.title}
                </h2>
                <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
                  {group.summary}
                </p>
              </div>
            </div>

            <ul className="mt-6 grid gap-x-8 gap-y-4 border-t border-border pt-5 md:grid-cols-3">
              {group.items.map((item) => (
                <li key={item} className="flex items-start gap-2.5">
                  <Check className="mt-0.5 size-3.5 shrink-0 text-success" />
                  <p className="text-xs leading-relaxed text-muted-foreground">
                    {item}
                  </p>
                </li>
              ))}
            </ul>
          </motion.section>
        ))}
      </div>

      <Separator className="my-16" />

      <div className="grid gap-6 border-t border-border pt-6 lg:grid-cols-3">
        {[
          {
            icon: Layers3,
            title: 'One engine',
            detail:
              'The CLI, web terminal, and desktop app share the dispatcher, the built-in registry, the runtime, and the configuration model.',
          },
          {
            icon: History,
            title: 'Consistent history',
            detail:
              'History, prompt, and exit statuses behave identically on every surface because they come from the same session runtime.',
          },
          {
            icon: LockKeyhole,
            title: 'No hidden shells',
            detail:
              'No Bash, CMD, or PowerShell is invoked to parse KERNEXT syntax. The only interpreter use is launching Windows command scripts.',
          },
        ].map((item) => (
          <div key={item.title} className="max-w-sm">
            <h3 className="flex items-center gap-2 text-sm font-semibold">
              <item.icon className="size-4 text-primary" />
              {item.title}
            </h3>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              {item.detail}
            </p>
          </div>
        ))}
      </div>

      <div className="mt-14 grid gap-4 rounded-2xl border border-border bg-surface/40 p-6 sm:grid-cols-3">
        {[
          { icon: Server, title: 'Local Axum runtime', detail: 'REST sessions and two WebSocket transports.' },
          { icon: Network, title: 'Streaming relay', detail: 'Binary PTY frames with JSON control messages.' },
          { icon: Gauge, title: 'Bounded runtime', detail: 'Session caps, idle cleanup, bounded recursion.' },
        ].map((item) => (
          <div key={item.title} className="flex items-start gap-3">
            <item.icon className="mt-0.5 size-4 shrink-0 text-primary" />
            <div>
              <p className="text-sm font-medium">{item.title}</p>
              <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                {item.detail}
              </p>
            </div>
          </div>
        ))}
      </div>

      <div className="mt-14 rounded-2xl border border-border bg-surface/40 p-6">
        <p className="text-xs leading-relaxed text-muted-foreground">
          <span className="font-medium text-foreground">Find something missing?</span>{' '}
          Built-ins keep a precise registry: <span className="font-mono">help</span> lists
          every command your build ships, and{' '}
          <span className="font-mono">which</span> reports whether a name is a
          built-in, a function, or a program on your PATH. Feature claims on
          this page map to code in{' '}
          <span className="font-mono">crates/shell-*</span> and{' '}
          <span className="font-mono">apps/</span>, verified by the
          repository&apos;s test suite.
        </p>
      </div>

      <div className="mt-14 overflow-hidden rounded-2xl border border-border bg-[#04070c] p-6">
        <div className="space-y-1 font-mono text-[11px] leading-relaxed text-[#d8e6f5]">
          <p>
            <span className="text-[#67e8f9]">kernext ~ $</span> which echo git
          </p>
          <p className="text-[#c7d2fe]">echo: shell builtin</p>
          <p className="text-[#c7d2fe]">C:\Program Files\Git\cmd\git.exe</p>
          <p className="pt-1">
            <span className="text-[#67e8f9]">kernext ~ $</span> sysinfo
          </p>
          <p className="text-[#9fb3c8]">product    KERNEXT</p>
          <p className="text-[#9fb3c8]">platform   windows</p>
          <p className="text-[#9fb3c8]">arch       x86_64</p>
        </div>
      </div>

      <div className="mt-14 flex flex-col items-start justify-between gap-4 rounded-2xl border border-border-strong bg-surface p-6 sm:flex-row sm:items-center">
        <div>
          <p className="text-sm font-medium">See it run for real.</p>
          <p className="mt-1 text-xs text-muted-foreground">
            The web terminal connects to the same engine this page describes.
          </p>
        </div>
        <div className="flex flex-wrap gap-3">
          <Button asChild>
            <Link to="/terminal">
              <TerminalSquare className="size-4" />
              Open terminal
            </Link>
          </Button>
          <Button asChild variant="outline">
            <Link to="/downloads">
              Get KERNEXT
              <ArrowRight className="size-4" />
            </Link>
          </Button>
        </div>
      </div>
    </PageShell>
  );
}
