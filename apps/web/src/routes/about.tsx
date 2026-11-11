import { Link } from '@tanstack/react-router';
import {
  ArrowRight,
  Boxes,
  Cpu,
  Github,
  Globe2,
  Monitor,
  Scale,
  Sparkles,
  Terminal,
  TerminalSquare,
} from 'lucide-react';
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
import { KERNEXT_VERSION } from '@/lib/version';

const VALUES = [
  {
    title: 'Open',
    description:
      'One engine, one source tree, every surface reading the same code. What the CLI does, the web terminal and the desktop app do.',
  },
  {
    title: 'Powerful',
    description:
      'A real lexer, parser, and evaluator. Native pipelines, redirections, scripting, and background jobs — implemented, not emulated.',
  },
  {
    title: 'Independent',
    description:
      'No wrapper around Bash, CMD, or PowerShell. KERNEXT owns its syntax and launches programs through OS process APIs.',
  },
] as const;

const FACTS = [
  { label: 'Product', value: 'KERNEXT' },
  { label: 'Version', value: `v${KERNEXT_VERSION}` },
  { label: 'Engine', value: 'Rust workspace' },
  { label: 'Surfaces', value: 'CLI · Web · Desktop' },
  { label: 'Platforms', value: 'Windows · Linux · macOS' },
  { label: 'License', value: 'Not yet selected' },
] as const;

const STACK = [
  { icon: Cpu, name: 'Rust', detail: 'Engine, runtime, built-ins, PTY backends, server, Tauri backend.' },
  { icon: Terminal, name: 'Reedline', detail: 'The CLI line editor: history, completion, key bindings.' },
  { icon: Globe2, name: 'Axum + Tokio', detail: 'The local web runtime behind the web terminal.' },
  { icon: Monitor, name: 'Tauri 2', detail: 'The desktop host wrapping the same engine and terminal UI.' },
  { icon: Boxes, name: 'React 19 · Vite', detail: 'The shared front end for web and desktop.' },
  { icon: Sparkles, name: 'Tailwind CSS v4', detail: 'A restrained, engineering-led interface system.' },
] as const;

export function AboutPage() {
  return (
    <div className="mx-auto w-full max-w-[1100px] px-4 py-14 sm:px-6 lg:px-8 lg:py-20">
      <div className="grid gap-10 lg:grid-cols-[1.2fr_0.8fr] lg:items-center">
        <div>
          <Badge variant="outline">About</Badge>
          <h1 className="mt-4 text-balance text-4xl font-semibold tracking-[-0.04em] sm:text-5xl">
            The next core of computing.
          </h1>
          <p className="mt-5 text-sm leading-relaxed text-muted-foreground sm:text-base">
            KERNEXT is a native Rust shell engine with one execution path and
            three surfaces: a CLI, a web terminal served by a local Axum
            runtime, and a Tauri 2 desktop application. It is built as a
            single product where every surface answers to the same dispatcher,
            the same built-in registry, and the same configuration.
          </p>
          <p className="mt-4 font-mono text-xs uppercase tracking-[0.2em] text-primary">
            Independent. Native. Engineered.
          </p>
        </div>

        <div className="about-core" aria-hidden="true">
          <div className="about-core__face">
            <LogoMark className="size-16" />
          </div>
          <span className="about-core__caption">KERNEXT / RUST ENGINE</span>
        </div>
      </div>

      {/* Section heading keeps the outline valid: h1 → h2 → card h3s. */}
      <section>
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Values
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          What KERNEXT stands for
        </h2>
        <div className="mt-6 grid gap-4 sm:grid-cols-3">
          {VALUES.map((value) => (
            <Card key={value.title} className="bg-surface/55">
              <CardHeader>
                <CardTitle className="text-base">{value.title}</CardTitle>
                <CardDescription>{value.description}</CardDescription>
              </CardHeader>
            </Card>
          ))}
        </div>
      </section>

      <Separator className="my-12" />

      <section>
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Identity
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          Project facts
        </h2>

        <div className="mt-6 overflow-hidden rounded-2xl border border-border">
          <table className="w-full text-left text-xs sm:text-sm">
            <tbody className="divide-y divide-border">
              {FACTS.map((fact) => (
                <tr key={fact.label} className="bg-surface/40">
                  <th className="w-40 px-4 py-3 font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                    {fact.label}
                  </th>
                  <td className="px-4 py-3 font-medium">{fact.value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <p className="mt-4 text-xs leading-relaxed text-muted-foreground">
          <span className="inline-flex items-center gap-1.5">
            <Scale className="size-3.5" />
            The license has not been chosen yet, so no licensing claim is made
            on this site. The repository is the source of truth.
          </span>
        </p>
      </section>

      <Separator className="my-12" />

      <section>
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Technology
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          What it is built with
        </h2>

        <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {STACK.map((item) => (
            <Card key={item.name} className="p-5">
              <div className="flex items-center gap-2 text-sm font-medium">
                <item.icon className="size-4 text-primary" />
                {item.name}
              </div>
              <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
                {item.detail}
              </p>
            </Card>
          ))}
        </div>
      </section>

      <Separator className="my-12" />

      <section>
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Direction
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          What &quot;complete&quot; means here
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          Status claims on this site are conservative on purpose. A surface is
          only described as working when the repository&apos;s automated tests
          or an explicit build verify it, and the project status document
          separates what is implemented, what is tested, and what remains
          unverified per platform.
        </p>

        <div className="mt-6 grid gap-4 sm:grid-cols-3">
          <Card className="bg-surface/55">
            <CardContent className="p-5">
              <div className="flex items-center gap-2 text-sm font-medium">
                <TerminalSquare className="size-4 text-success" />
                CLI
              </div>
              <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
                The reference surface. Verified by the Rust test suite on all
                three platforms.
              </p>
            </CardContent>
          </Card>
          <Card className="bg-surface/55">
            <CardContent className="p-5">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Globe2 className="size-4 text-success" />
                Web
              </div>
              <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
                REST and WebSocket integration tests run against the real
                Axum router and the real engine.
              </p>
            </CardContent>
          </Card>
          <Card className="bg-surface/55">
            <CardContent className="p-5">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Monitor className="size-4 text-success" />
                Desktop
              </div>
              <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
                Tauri 2 app with unit tests and a debug bundle build verified
                on Windows; other platforms pending hosts.
              </p>
            </CardContent>
          </Card>
        </div>
      </section>

      <div className="mt-12 flex flex-col items-start justify-between gap-4 rounded-2xl border border-border-strong bg-surface p-6 sm:flex-row sm:items-center">
        <div>
          <p className="text-sm font-medium">Build. Execute. Innovate.</p>
          <p className="mt-1 text-xs text-muted-foreground">
            The tagline is also the development loop.
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
            <a href="https://github.com/Trishit6/terminal" target="_blank" rel="noreferrer">
              <Github className="size-4" />
              Repository
              <ArrowRight className="size-3.5 opacity-70" />
            </a>
          </Button>
        </div>
      </div>

      <div className="mt-10 flex items-center justify-center gap-2 text-xs text-muted-foreground">
        <Cpu className="size-3.5" />
        KERNEXT · The Next Core of Computing.
      </div>
    </div>
  );
}
