import { Link } from '@tanstack/react-router';
import {
  ArrowRight,
  Bug,
  Code2,
  ExternalLink,
  GitBranch,
  Github,
  MessageSquare,
  ScrollText,
  ShieldCheck,
  TerminalSquare,
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

const EASE = [0.22, 1, 0.36, 1] as const;

const REPOSITORY_URL = 'https://github.com/Trishit6/terminal';

const CHANNELS = [
  {
    icon: Bug,
    title: 'Report an issue',
    description:
      'Bugs, cross-platform differences, and feature requests go to the repository issue tracker. Include the surface (CLI, web, desktop), the platform, and the exact command line.',
    action: 'Open the issue tracker',
    href: `${REPOSITORY_URL}/issues`,
  },
  {
    icon: Code2,
    title: 'Contribute code',
    description:
      'Read CONTRIBUTING.md in the repository root first: it explains the workspace layout, the command flow invariants, and the required checks (fmt, clippy, tests) before any change can be accepted.',
    action: 'View the repository',
    href: REPOSITORY_URL,
  },
  {
    icon: ScrollText,
    title: 'Read the status',
    description:
      'docs/PROJECT_STATUS.md separates implemented, tested, partially tested, and blocked work per surface and platform — start there before assuming a feature exists.',
    action: 'Project status document',
    href: `${REPOSITORY_URL}#implementation-status`,
  },
] as const;

export function ContactPage() {
  return (
    <div className="mx-auto w-full max-w-[1100px] px-4 py-14 sm:px-6 lg:px-8 lg:py-20">
      <div className="max-w-3xl">
        <Badge variant="outline">Community</Badge>
        <h1 className="mt-4 text-balance text-4xl font-semibold tracking-[-0.04em] sm:text-5xl">
          One repository.
          <span className="text-gradient"> Every conversation.</span>
        </h1>
        <p className="mt-5 text-sm leading-relaxed text-muted-foreground sm:text-base">
          KERNEXT is developed in the open. The repository is the single
          channel for issues, contributions, and release discussion — there
          are no other official accounts, chat rooms, or mailing lists to
          chase.
        </p>
      </div>

      <div className="mt-10 grid gap-5 lg:grid-cols-3">
        {CHANNELS.map((channel, index) => (
          <motion.div
            key={channel.title}
            initial={{ opacity: 0, y: 18 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: '-60px' }}
            transition={{ duration: 0.5, ease: EASE, delay: index * 0.05 }}
          >
            <Card className="flex h-full flex-col bg-surface/55">
              <CardHeader className="gap-3">
                <div className="grid size-10 place-items-center rounded-xl border border-border bg-background/70 text-primary">
                  <channel.icon className="size-5" />
                </div>
                <div>
                  <CardTitle className="text-base">{channel.title}</CardTitle>
                  <CardDescription className="mt-2">
                    {channel.description}
                  </CardDescription>
                </div>
              </CardHeader>
              <CardContent className="mt-auto">
                <a
                  href={channel.href}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1.5 text-xs font-medium text-primary transition-colors hover:text-primary-strong"
                >
                  {channel.action}
                  <ArrowRight className="size-3.5" />
                </a>
              </CardContent>
            </Card>
          </motion.div>
        ))}
      </div>

      <div className="mt-10 grid gap-4 lg:grid-cols-2">
        <Card className="p-6">
          <div className="flex items-center gap-2 text-sm font-medium">
            <MessageSquare className="size-4 text-primary" />
            What makes a good issue
          </div>
          <ul className="mt-3 space-y-2 text-xs leading-relaxed text-muted-foreground">
            <li>· The exact command line and its observed output.</li>
            <li>· Which surface: CLI, web terminal, or desktop app.</li>
            <li>· Platform and version — `version` prints both.</li>
            <li>· Whether it is reproducible with the test suite.</li>
          </ul>
        </Card>
        <Card className="p-6">
          <div className="flex items-center gap-2 text-sm font-medium">
            <ShieldCheck className="size-4 text-success" />
            Security reports
          </div>
          <p className="mt-3 text-xs leading-relaxed text-muted-foreground">
            Suspected vulnerabilities in the server or the desktop app belong
            in a private repository advisory rather than a public issue. Do
            not publish working exploits for the local runtime.
          </p>
        </Card>
      </div>

      <div className="mt-12 flex flex-col items-start justify-between gap-4 rounded-2xl border border-border-strong bg-surface p-6 sm:flex-row sm:items-center">
        <div>
          <p className="text-sm font-medium">Open • Powerful • Independent</p>
          <p className="mt-1 text-xs text-muted-foreground">
            The same values that shaped the engine shape the community rules.
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
            <a href={REPOSITORY_URL} target="_blank" rel="noreferrer">
              <Github className="size-4" />
              Repository
              <ExternalLink className="size-3.5 opacity-70" />
            </a>
          </Button>
        </div>
      </div>

      <div className="mt-10 flex items-center justify-center gap-2 text-xs text-muted-foreground">
        <GitBranch className="size-3.5" />
        No fake channels: if it is not in the repository, it is not KERNEXT.
      </div>
    </div>
  );
}
