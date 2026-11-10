import { Link } from '@tanstack/react-router';
import {
  Boxes,
  Cpu,
  Heart,
  Layers,
  LockKeyhole,
  Monitor,
  Server,
  ShieldCheck,
  Terminal,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { LogoMark } from '@/components/site/logo';

const VALUES = [
  {
    icon: ShieldCheck,
    title: 'Open',
    description:
      'The engine, terminal, and desktop shell are built from the same local Rust workspace.',
  },
  {
    icon: Cpu,
    title: 'Powerful',
    description:
      'Native pipelines, real PTY sessions, and a 53-command built-in registry.',
  },
  {
    icon: LockKeyhole,
    title: 'Independent',
    description:
      'No wrapper shell. Commands run through the KERNEXT dispatcher and native process APIs.',
  },
] as const;

const SURFACES = [
  {
    icon: Terminal,
    title: 'CLI',
    description:
      'The reference surface with Reedline editing, cinematic startup, and the full shell engine.',
  },
  {
    icon: Server,
    title: 'Web',
    description:
      'The same engine exposed through a local Axum server and a custom React terminal.',
  },
  {
    icon: Monitor,
    title: 'Desktop',
    description:
      'A Tauri 2 shell that reuses the web UI and drives the local runtime directly.',
  },
] as const;

export function DesktopAboutPage() {
  return (
    <div className="mx-auto w-full max-w-[1100px] px-6 py-10 lg:px-10">
      <div className="flex flex-col gap-6 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-4">
          <LogoMark className="size-14" />
          <div>
            <h1 className="text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
              KERNEXT
            </h1>
            <p className="mt-1 text-sm text-muted-foreground">
              The Next Core of Computing.
            </p>
            <div className="mt-2 flex flex-wrap gap-2">
              <Badge variant="outline">OPEN</Badge>
              <Badge variant="outline">POWERFUL</Badge>
              <Badge variant="outline">INDEPENDENT</Badge>
            </div>
          </div>
        </div>
        <div className="text-right text-xs text-muted-foreground">
          <div className="font-mono">Build. Execute. Innovate.</div>
          <div className="mt-1 font-mono">Local Rust runtime</div>
        </div>
      </div>

      <div className="mt-10 grid gap-4 md:grid-cols-3">
        {VALUES.map((value) => (
          <Card key={value.title}>
            <CardHeader className="gap-2">
              <div className="grid size-9 place-items-center rounded-xl border border-border bg-background/70 text-primary">
                <value.icon className="size-4" />
              </div>
              <CardTitle className="text-base">{value.title}</CardTitle>
            </CardHeader>
            <CardContent>
              <CardDescription>{value.description}</CardDescription>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="mt-10">
        <h2 className="text-xl font-semibold tracking-[-0.02em]">Surfaces</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          One engine, three ways in. The CLI is the reference; the web and
          desktop shells reuse the same Rust runtime.
        </p>
        <div className="mt-4 grid gap-4 md:grid-cols-3">
          {SURFACES.map((surface) => (
            <Card key={surface.title} className="bg-surface/60">
              <CardHeader className="gap-2">
                <div className="flex items-center gap-2">
                  <surface.icon className="size-4 text-primary" />
                  <CardTitle className="text-base">{surface.title}</CardTitle>
                </div>
              </CardHeader>
              <CardContent>
                <CardDescription>{surface.description}</CardDescription>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>

      <div className="mt-10 grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Layers className="size-4 text-primary" />
              Runtime composition
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2 text-sm text-muted-foreground">
              <li>Native lexer, parser, AST, and dispatcher.</li>
              <li>Pipelines and redirections stay in-process.</li>
              <li>External commands launch through native process APIs.</li>
              <li>PTY sessions use ConPTY on Windows and openpty on Unix.</li>
            </ul>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Boxes className="size-4 text-primary" />
              Desktop composition
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2 text-sm text-muted-foreground">
              <li>Tauri 2 host with restrictive capabilities.</li>
              <li>React and TypeScript frontend shared with the web app.</li>
              <li>Local IPC commands; no public server round trip.</li>
              <li>Cinematic startup and a frameless custom title bar.</li>
            </ul>
          </CardContent>
        </Card>
      </div>

      <div className="mt-10 flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-border bg-surface/50 p-6">
        <div className="flex items-center gap-3 text-sm text-muted-foreground">
          <Heart className="size-4 text-danger" />
          Built as a local-first shell.
        </div>
        <div className="flex gap-2">
          <Link
            to="/desktop"
            className="text-xs font-medium text-primary hover:text-primary-strong"
          >
            Back to dashboard
          </Link>
          <span className="text-xs text-muted-foreground">·</span>
          <Link
            to="/desktop/settings"
            className="text-xs font-medium text-primary hover:text-primary-strong"
          >
            Settings
          </Link>
        </div>
      </div>
    </div>
  );
}
