import { Suspense, lazy, useState, useCallback } from 'react';
import { Link } from '@tanstack/react-router';
import {
  ArrowRight,
  ArrowUpRight,
  Braces,
  Cpu,
  GitBranch,
  Globe2,
  KeyRound,
  Layers,
  Monitor,
  ShieldCheck,
  Terminal,
  TerminalSquare,
  Workflow,
  Check,
  Copy,
  Code2,
  Globe,
  MonitorCheck,
  Cog,
} from 'lucide-react';
import { motion } from 'motion/react';
import { CoreSceneFallback } from '@/components/site/core-fallback';
import { PlatformMark } from '@/components/site/platform-mark';
import { Button } from '@/components/ui/button';
import { KERNEXT_VERSION } from '@/lib/version';

/**
 * The core scene pulls in `three`, `@react-three/fiber`, and `gsap` (~1 MB raw,
 * ~280 kB gzipped). Loading it lazily keeps that weight off the critical path:
 * the hero copy paints first, then the 3D chunk streams in behind a static
 * fallback that shares the scene's layout.
 */
const OrbitalHero = lazy(() =>
  import('@/components/site/orbital-hero').then((module) => ({
    default: module.OrbitalHero,
  })),
);

const EASE = [0.22, 1, 0.36, 1] as const;

const REPOSITORY_URL = 'https://github.com/Trishit6/terminal';

/**
 * Documentation entries. Each `hash` targets a real section id on `/docs`
 * (`quickstart`, `api`, `websocket`, `security`, `status`) — verified against
 * `routes/docs.tsx`.
 */
const DOC_ENTRIES = [
  {
    index: '01',
    title: 'Quick start',
    hash: 'quickstart',
    description: 'Install the CLI, launch the shell, and run the first command.',
  },
  {
    index: '02',
    title: 'HTTP session API',
    hash: 'api',
    description: 'Session lifecycle, request shapes, and error codes for the local server.',
  },
  {
    index: '03',
    title: 'WebSocket transports',
    hash: 'websocket',
    description: 'The engine and real-PTY streams the browser terminal connects to.',
  },
  {
    index: '04',
    title: 'Security model',
    hash: 'security',
    description: 'Accounts, HttpOnly sessions, CSRF, rate limits, and token hashing.',
  },
  {
    index: '05',
    title: 'Project status',
    hash: 'status',
    description: 'What is implemented, what is verified, and what is still open.',
  },
] as const;

const SURFACES = [
  {
    number: '01',
    name: 'CLI',
    icon: Terminal,
    title: 'KERNEXT CLI',
    description:
      'The reference surface. A Reedline prompt with history, completion, and key bindings in front of native built-ins and real process execution.',
    tags: ['kernext executable', 'Reedline', 'local runtime'],
    to: '/docs',
    cta: 'CLI documentation',
    preview: 'cli',
  },
  {
    number: '02',
    name: 'Web',
    icon: Globe2,
    title: 'KERNEXT Web',
    description:
      'A browser terminal with its own ANSI screen buffer and a real PTY WebSocket transport to the local Axum server.',
    tags: ['custom renderer', 'PTY transport', 'resize-aware'],
    to: '/terminal',
    cta: 'Open terminal',
    preview: 'web',
  },
  {
    number: '03',
    name: 'Desktop',
    icon: Monitor,
    title: 'KERNEXT Desktop',
    description:
      'A Tauri 2 application that executes through its embedded Rust runtime, never through a public server.',
    tags: ['Tauri 2', 'local runtime', 'shared UI'],
    to: '/downloads',
    cta: 'Desktop builds',
    preview: 'desktop',
  },
] as const;

const FEATURES = [
  {
    icon: Braces,
    title: 'Native command language',
    detail:
      'A hand-written lexer with spans, a precedence-aware parser, and an AST evaluator. Input is never split on whitespace.',
  },
  {
    icon: Layers,
    title: 'Pipelines and redirections',
    detail:
      'Real OS pipes between external programs, in-process built-in stages, and >, >>, 2>, 2>> resolved against the session directory.',
  },
  {
    icon: Workflow,
    title: 'Scripting runtime',
    detail:
      'Variables, quoting, expansion, if/else, while, for, functions, return, break, continue, and bounded recursion.',
  },
  {
    icon: Cpu,
    title: 'Genuine process execution',
    detail:
      'Programs launch through OS process APIs with argument boundaries preserved. Windows command scripts use a launcher, not a parser.',
  },
  {
    icon: KeyRound,
    title: 'Accounts and sessions',
    detail:
      'MariaDB-backed registration, Argon2id password hashing, HttpOnly cookies, CSRF double-submit, and hashed tokens at rest.',
  },
  {
    icon: GitBranch,
    title: 'Background jobs',
    detail:
      'jobs, wait, kill, fg, and bg over a shared job table, with background evaluation isolated from the foreground shell.',
  },
] as const;

const PLATFORMS = [
  {
    brand: 'windows' as const,
    name: 'Windows',
    detail: 'Native process resolution, ConPTY, and Tauri 2 bundling.',
    target: 'x86_64 · Windows 10 or later',
  },
  {
    brand: 'linux' as const,
    name: 'Linux',
    detail: 'Native process execution, openpty, and Tauri 2 packages.',
    target: 'x86_64 · glibc and musl',
  },
  {
    brand: 'macos' as const,
    name: 'macOS',
    detail: 'Apple Silicon and Intel, openpty, and a signed app bundle path.',
    target: 'arm64 and x64',
  },
] as const;

const CONTACT_CHANNELS = [
  {
    icon: GitBranch,
    title: 'Issues and proposals',
    description:
      'Bugs, platform differences, and feature requests go to the repository tracker with the surface, platform, and exact command line.',
    action: 'Open the tracker',
    href: `${REPOSITORY_URL}/issues`,
  },
  {
    icon: ShieldCheck,
    title: 'Security reports',
    description:
      'Suspected vulnerabilities in the server or desktop app belong in a private repository advisory, not a public issue.',
    action: 'Report privately',
    href: `${REPOSITORY_URL}/security/advisories/new`,
  },
  {
    icon: TerminalSquare,
    title: 'Contributions',
    description:
      'CONTRIBUTING.md explains the workspace layout, command-flow invariants, and the fmt, clippy, and test checks required before a change.',
    action: 'Read the guide',
    href: `${REPOSITORY_URL}/blob/main/CONTRIBUTING.md`,
  },
] as const;

// ── New data structures for the redesign ──

type Pillar = {
  icon: typeof Code2;
  title: string;
  description: string;
  tags: readonly string[];
  gradient: string;
  borderColor: string;
  hoverBorder: string;
};

const PILLARS: readonly Pillar[] = [
  {
    icon: Code2,
    title: 'Native Rust Engine',
    description: 'Hand-written lexer, parser, AST evaluator, and pipeline engine. Zero shell dependencies — pure Rust from keystroke to process.',
    tags: ['shell-core', 'shell-builtins', 'shell-runtime'] as const,
    gradient: 'from-cyan-500/20 to-blue-500/10',
    borderColor: 'border-cyan-500/30',
    hoverBorder: 'hover:border-cyan-500/60',
  },
  {
    icon: Globe,
    title: 'Browser Workspace',
    description: 'React + Vite frontend with custom ANSI terminal renderer. Real PTY WebSocket transport to local Axum server.',
    tags: ['shell-server', 'web terminal', 'PTY transport'] as const,
    gradient: 'from-blue-500/20 to-indigo-500/10',
    borderColor: 'border-blue-500/30',
    hoverBorder: 'hover:border-blue-500/60',
  },
  {
    icon: MonitorCheck,
    title: 'Desktop Application',
    description: 'Tauri 2 native app sharing the React UI. Executes through embedded Rust runtime — no public server, fully local.',
    tags: ['shell-desktop', 'Tauri 2', 'local execution'] as const,
    gradient: 'from-violet-500/20 to-purple-500/10',
    borderColor: 'border-violet-500/30',
    hoverBorder: 'hover:border-violet-500/60',
  },
];

const METADATA_BADGES = [
  { Icon: Cog, label: 'Rust Engine', color: 'text-orange-400', bgColor: 'bg-orange-400/10' },
  { Icon: Monitor, label: 'Win · Lin · macOS', color: 'text-cyan-400', bgColor: 'bg-cyan-400/10' },
  { Icon: TerminalSquare, label: 'Native PTY', color: 'text-violet-400', bgColor: 'bg-violet-400/10' },
] as const;

function SectionHead({
  index,
  eyebrow,
  title,
  description,
}: {
  index: string;
  eyebrow: string;
  title: string;
  description: string;
}) {
  return (
    <motion.div
      className="section-head"
      initial={{ opacity: 0, y: 12 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-80px' }}
      transition={{ duration: 0.5, ease: EASE }}
    >
      <div>
        <p className="section-kicker">
          <span>{index}</span>
          {eyebrow}
        </p>
        <h2>{title}</h2>
      </div>
      <p className="section-head__description">{description}</p>
    </motion.div>
  );
}

function SurfacesSection() {
  return (
    <section id="surfaces" className="home-section home-section--ruled">
      <SectionHead
        index="01 / Surfaces"
        eyebrow="One engine, three interfaces"
        title="The same runtime, wherever you work."
        description="Each surface is built around the shared Rust engine — not a separate interpretation of it. Start in the terminal, continue in the browser, ship on the desktop."
      />

      <div className="surface-index">
        {SURFACES.map((surface) => (
          <Link key={surface.name} to={surface.to} className="surface-row">
            <span className="surface-row__number">{surface.number}</span>
            <div className="surface-row__copy">
              <div className="surface-row__title">
                <surface.icon className="size-4" aria-hidden="true" />
                {surface.title}
              </div>
              <p>{surface.description}</p>
              <div className="surface-row__tags">
                {surface.tags.map((tag) => (
                  <span key={tag}>{tag}</span>
                ))}
              </div>
            </div>
            <div className="surface-row__visual" aria-hidden="true">
              {surface.preview === 'cli' ? <CliPreview /> : null}
              {surface.preview === 'web' ? <WebPreview /> : null}
              {surface.preview === 'desktop' ? <DesktopPreview /> : null}
            </div>
            <span className="surface-row__cta">
              {surface.cta}
              <ArrowUpRight className="size-3.5" aria-hidden="true" />
            </span>
          </Link>
        ))}
      </div>
    </section>
  );
}

function CliPreview() {
  return (
    <div className="surface-preview">
      <div className="surface-preview__bar">
        <i /> <i /> <i /><span>kernext / cli</span>
      </div>
      <div className="surface-preview__body">
        <p><span>kernext ~ $</span> version</p>
        <p className="output">KERNEXT v{KERNEXT_VERSION}</p>
        <p><span>kernext ~ $</span> which echo</p>
        <p className="output">echo: shell builtin</p>
      </div>
    </div>
  );
}

function WebPreview() {
  return (
    <div className="surface-preview">
      <div className="surface-preview__bar">
        <i /> <i /> <i /><span>kernext / terminal</span>
      </div>
      <div className="surface-preview__body">
        <p><span>kernext ~ $</span> ls</p>
        <p className="output">src&nbsp;&nbsp; Cargo.toml&nbsp;&nbsp; README.md</p>
        <p><span>kernext ~ $</span> sysinfo</p>
        <p className="output">product&nbsp;&nbsp; KERNEXT · platform&nbsp;&nbsp; local</p>
      </div>
    </div>
  );
}

function DesktopPreview() {
  return (
    <div className="surface-preview surface-preview--desktop">
      <div className="surface-preview__bar">
        <i /> <i /> <i /><span>kernext / desktop</span>
      </div>
      <div className="surface-preview__body">
        <div className="surface-preview__rail">
          <b>K</b><i /> <i /> <i />
        </div>
        <div className="surface-preview__desktop">
          <b>KERNEXT</b>
          <span>Local runtime · Ready</span>
        </div>
      </div>
    </div>
  );
}

function PlatformSection() {
  return (
    <section className="platform-band" aria-labelledby="platform-heading">
      <div className="platform-band__inner">
        <div className="platform-band__head">
          <div>
            <p className="section-kicker">
              <span>02 / Platforms</span>
              Cross-platform by design
            </p>
            <h2 id="platform-heading">Built for the systems you use.</h2>
          </div>
          <p>
            The workspace targets Windows, Linux, and macOS with
            platform-specific process resolution and terminal backends. The
            same engine and the same tests run on every target.
          </p>
        </div>

        <div className="platform-band__grid">
          {PLATFORMS.map((platform) => (
            <div key={platform.name} className="platform-band__item">
              <PlatformMark platform={platform.brand} />
              <div>
                <span className="platform-band__label">{platform.target}</span>
                <h3>{platform.name}</h3>
                <p>{platform.detail}</p>
              </div>
              <span className="platform-band__state">Supported</span>
            </div>
          ))}
        </div>

        <div className="platform-band__note">
          <span>CI matrix · Windows x64 · Linux x64 · macOS arm64 and x64</span>
          <Link to="/downloads">
            Build and release details
            <ArrowUpRight className="size-3.5" aria-hidden="true" />
          </Link>
        </div>
      </div>
    </section>
  );
}

function EngineSection() {
  return (
    <section id="engine" className="home-section home-section--ruled">
      <SectionHead
        index="03 / Engine"
        eyebrow="Ownership of the command language"
        title="The shell owns every semantic."
        description="KERNEXT lexes with spans, parses an AST, and evaluates in Rust. It never splits input on whitespace and never hands its syntax to Bash, CMD, or PowerShell."
      />

      <div className="engine-board">
        <div className="engine-board__copy">
          <p className="engine-board__lead">
            From keystroke to process, in one runtime.
          </p>
          <p>
            Built-ins win over functions; functions win over external programs.
            Pipelines and redirections are native constructs, and every stage
            reports one outcome with an honest exit status.
          </p>

          <div
            className="pipeline-figure"
            role="img"
            aria-label="A command is lexed, parsed into an AST, evaluated, sent through the pipeline engine, then executed."
          >
            <div className="pipeline-figure__input">
              <span>Input</span>
              <code>ls src | grep shell | sort</code>
            </div>
            <ol className="pipeline-figure__stages">
              <li>Lexer</li>
              <li>Parser</li>
              <li>AST</li>
              <li>Evaluator</li>
              <li>Pipeline</li>
              <li>Execute</li>
            </ol>
            <div className="pipeline-figure__status">
              <span>Output routed · exit status tracked</span>
              <b>Status 0</b>
            </div>
          </div>
        </div>

        <div className="engine-board__facts">
          <div>
            <b>56</b>
            <span>native built-ins</span>
          </div>
          <div>
            <b>1</b>
            <span>shared dispatcher</span>
          </div>
          <div>
            <b>3</b>
            <span>execution surfaces</span>
          </div>
          <div>
            <b>0</b>
            <span>shell wrappers</span>
          </div>
        </div>
      </div>
    </section>
  );
}

function FeaturesSection() {
  return (
    <section id="features" className="home-section home-section--ruled">
      <SectionHead
        index="04 / Capabilities"
        eyebrow="Implemented, not promised"
        title="What the engine actually does."
        description="Every item below exists in the repository and is exercised by its test suite. The documentation names the known gaps beside each feature."
      />

      <div className="feature-matrix">
        {FEATURES.map((feature) => (
          <motion.article
            key={feature.title}
            className="feature-matrix__item"
            initial={{ opacity: 0, y: 10 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: '-60px' }}
            transition={{ duration: 0.45, ease: EASE }}
          >
            <feature.icon className="size-5" aria-hidden="true" />
            <h3>{feature.title}</h3>
            <p>{feature.detail}</p>
          </motion.article>
        ))}
      </div>
    </section>
  );
}

function ArchitectureSection() {
  return (
    <section id="architecture" className="home-section home-section--ruled">
      <SectionHead
        index="05 / Architecture"
        eyebrow="One execution core"
        title="Clear boundaries between interface and engine."
        description="Every surface feeds the same lexer, parser, evaluator, and process runtime. The interfaces differ; the semantics do not."
      />

      <div className="flow" aria-label="KERNEXT architecture">
        <div className="flow__surfaces">
          <div className="flow__surface">
            <div className="flow__surface-top">
              <span>Surface 01</span>
              <Terminal className="size-4" aria-hidden="true" />
            </div>
            <h3>CLI</h3>
            <p>Reedline interface · shell-cli binary</p>
          </div>
          <div className="flow__surface">
            <div className="flow__surface-top">
              <span>Surface 02</span>
              <Globe2 className="size-4" aria-hidden="true" />
            </div>
            <h3>Web</h3>
            <p>React terminal · Axum API and PTY relay</p>
          </div>
          <div className="flow__surface">
            <div className="flow__surface-top">
              <span>Surface 03</span>
              <Monitor className="size-4" aria-hidden="true" />
            </div>
            <h3>Desktop</h3>
            <p>Shared React UI · Tauri and local runtime</p>
          </div>
        </div>

        <div className="flow__connector" aria-hidden="true">
          <i /> <i /> <i />
        </div>

        <div className="flow__engine">
          <Cpu className="size-5" aria-hidden="true" />
          <div>
            <b>Shared Rust shell engine</b>
            <span>shell-core · shell-builtins · shell-runtime · shell-api</span>
          </div>
          <code>→ one CommandOutcome</code>
        </div>

        <div className="flow__stages">
          <div className="flow__stage">
            Lex
            <span>Spans and tokens</span>
          </div>
          <div className="flow__stage">
            Parse
            <span>AST, precedence</span>
          </div>
          <div className="flow__stage">
            Evaluate
            <span>Built-ins, functions</span>
          </div>
          <div className="flow__stage">
            Execute
            <span>Pipelines, PTY, processes</span>
          </div>
        </div>
      </div>
    </section>
  );
}

function BenefitsSection() {
  return (
    <section id="benefits" className="home-section home-section--ruled">
      <div className="benefits">
        <div className="benefits__intro">
          <p className="section-kicker">
            <span>06 / Why KERNEXT</span>
            Built for people who live in a terminal
          </p>
          <h2>Built for the people who live in a terminal.</h2>
          <p className="benefits__lede">
            The design goal is not a prettier prompt. It is an execution model
            you can reason about: one parser, one dispatcher, one outcome — on
            every surface, on every platform.
          </p>

          <ol className="benefits__list">
            <li>
              <span>01</span>
              <div>
                <b>No hidden shells</b>
                <p>
                  KERNEXT parses its own syntax and launches programs with an
                  argument vector. Nothing is reinterpreted by Bash, CMD, or
                  PowerShell.
                </p>
              </div>
            </li>
            <li>
              <span>02</span>
              <div>
                <b>One runtime to learn</b>
                <p>
                  The CLI, browser workspace, and desktop app share the same
                  dispatcher, built-ins, scripting semantics, and configuration.
                </p>
              </div>
            </li>
            <li>
              <span>03</span>
              <div>
                <b>Local by default</b>
                <p>
                  Commands execute on your machine. The local server binds to
                  loopback, and the desktop app never routes execution through a
                  public service.
                </p>
              </div>
            </li>
            <li>
              <span>04</span>
              <div>
                <b>Tested across targets</b>
                <p>
                  The same Rust suite and browser checks run on Windows x64,
                  Linux x64, and macOS arm64 and x64.
                </p>
              </div>
            </li>
          </ol>
        </div>

        <aside className="benefits__panel" aria-label="KERNEXT development loop">
          <div className="benefits__panel-head">
            <span>The development loop</span>
            <span>Real commands</span>
          </div>
          <pre className="benefits__code">
            <code>{`# the same checks the project documents
kernext fmt --check
kernext check
kernext test

# run the full local stack
npm run dev`}</code>
          </pre>
          <dl className="benefits__facts">
            <div>
              <dt>Engine</dt>
              <dd>Rust workspace</dd>
            </div>
            <div>
              <dt>Surfaces</dt>
              <dd>CLI · Web · Desktop</dd>
            </div>
            <div>
              <dt>Boundaries</dt>
              <dd>Loopback and embedded</dd>
            </div>
          </dl>
          <Link to="/docs" hash="quickstart" className="home-link">
            Read the quick start
            <ArrowRight className="size-4" aria-hidden="true" />
          </Link>
        </aside>
      </div>
    </section>
  );
}

function TerminalSection() {
  return (
    <section className="home-section home-section--ruled">
      <div className="terminal-split">
        <div>
          <p className="section-kicker">
            <span>07 / Experience</span>
            Terminal workspace
          </p>
          <h2>A real shell, composed for the browser.</h2>
          <p className="terminal-split__lede">
            The web terminal renders its own ANSI screen buffer and connects to
            the local server over a real PTY transport. Resize the frame and
            the terminal rows and columns follow the backend.
          </p>

          <ul className="terminal-split__facts">
            <li>
              <span>Transport</span>
              <b>PTY WebSocket</b>
            </li>
            <li>
              <span>Rendering</span>
              <b>Custom ANSI buffer</b>
            </li>
            <li>
              <span>Layout</span>
              <b>Resizable · fullscreen</b>
            </li>
            <li>
              <span>Appearance</span>
              <b>Saved per browser or account</b>
            </li>
          </ul>

          <div className="terminal-split__actions">
            <Button asChild size="lg">
              <Link to="/terminal">
                <TerminalSquare className="size-4" aria-hidden="true" />
                Open Web Terminal
                <ArrowRight className="size-4" />
              </Link>
            </Button>
          </div>
        </div>

        <div className="home-terminal-window" data-terminal-preview="example">
          <div className="home-terminal-window__titlebar">
            <div className="home-terminal-window__controls" aria-hidden="true">
              <span /> <span /> <span />
            </div>
            <span>Example session · KERNEXT</span>
            <span className="home-terminal-window__local">STATIC</span>
          </div>
          <div className="home-terminal-window__content">
            <p>
              <span className="terminal-prompt">kernext ~ $</span>{' '}
              <span className="terminal-cmd">version</span>
            </p>
            <p className="home-terminal-window__output">
              KERNEXT v{KERNEXT_VERSION}
            </p>
            <p>
              <span className="terminal-prompt">kernext ~ $</span>{' '}
              <span className="terminal-cmd">help</span>
            </p>
            <p className="home-terminal-window__output">
              KERNEXT — Available commands: help, pwd, cd, ls, tree, find, stat, …
            </p>
            <p>
              <span className="terminal-prompt">kernext ~ $</span>{' '}
              <span className="terminal-cmd">
                echo {'"open the terminal to run your own"'}
              </span>
            </p>
            <p className="home-terminal-window__output terminal-ok">
              open the terminal to run your own
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}

function DocumentationSection() {
  return (
    <section id="documentation" className="home-section home-section--ruled">
      <SectionHead
        index="08 / Documentation"
        eyebrow="Read the engineering notes"
        title="The engine, documented from the source."
        description="Every page describes behaviour that exists in this repository: the command language, the HTTP session API, the WebSocket transports, the security model, and the current project status."
      />

      <div className="docs-band">
        {DOC_ENTRIES.map((entry) => (
          <Link key={entry.title} to="/docs" hash={entry.hash} className="docs-band__item">
            <span>{entry.index}</span>
            <b>{entry.title}</b>
            <ArrowUpRight className="size-4" aria-hidden="true" />
            <p>{entry.description}</p>
          </Link>
        ))}
      </div>

      <div className="docs-band__footer">
        <Button asChild variant="outline">
          <Link to="/docs">
            Read the documentation
            <ArrowRight className="size-4" />
          </Link>
        </Button>
        <p>Markdown sources live in the repository under <code>docs/</code>.</p>
      </div>
    </section>
  );
}

function DownloadsSection() {
  return (
    <section id="downloads" className="home-section home-section--ruled">
      <SectionHead
        index="09 / Downloads"
        eyebrow="Release artifacts, produced by CI"
        title="Install it where you work."
        description="The CLI is one executable; the desktop app is a Tauri 2 bundle from the same workspace. Releases are drafted by the repository workflow, so the releases page only ever lists artifacts that exist."
      />

      <div className="download-band">
        {PLATFORMS.map((platform) => (
          <Link
            key={platform.name}
            to="/downloads"
            className="download-band__item"
          >
            <PlatformMark platform={platform.brand} />
            <div>
              <h3>{platform.name}</h3>
              <span>{platform.target}</span>
              <p>{platform.detail}</p>
            </div>
            <ArrowUpRight className="size-4" aria-hidden="true" />
          </Link>
        ))}
      </div>

      <div className="download-band__footer">
        <p>
          Current source version <b>v{KERNEXT_VERSION}</b>. Archives, installers,
          and SHA-256 checksums come from the release workflow — never from a
          mirror.
        </p>
        <div>
          <Button asChild>
            <a href={`${REPOSITORY_URL}/releases`} target="_blank" rel="noreferrer">
              Releases on GitHub
              <ArrowUpRight className="size-3.5" aria-hidden="true" />
            </a>
          </Button>
          <Button asChild variant="outline">
            <Link to="/downloads">Download details</Link>
          </Button>
        </div>
      </div>
    </section>
  );
}

function AboutSection() {
  return (
    <section id="about" className="home-section home-section--ruled">
      <div className="home-about">
        <div>
          <p className="section-kicker">
            <span>10 / About</span>
            Independent · Native · Engineered
          </p>
          <h2>The next core of computing.</h2>
        </div>
        <div className="home-about__body">
          <p>
            KERNEXT began as an exercise in ownership: a shell that parses its
            own language, evaluates its own built-ins, and launches processes
            without borrowing another shell&apos;s interpretation of the command
            line. That engine now carries a CLI, a local web runtime, and a
            desktop application.
          </p>
          <p>
            The project is deliberately conservative about status. A surface is
            only described as working when automated tests or an explicit build
            verify it, and the project status document separates implemented,
            tested, and still-open work.
          </p>
          <div className="home-about__values">
            {['Open', 'Powerful', 'Independent'].map((value) => (
              <span key={value}>{value}</span>
            ))}
          </div>
          <Link to="/about" className="home-link">
            More about the project
            <ArrowRight className="size-4" aria-hidden="true" />
          </Link>
        </div>
      </div>
    </section>
  );
}

function ContactSection() {
  return (
    <section id="contact" className="home-section home-section--ruled">
      <SectionHead
        index="11 / Contact"
        eyebrow="One repository, every conversation"
        title="Developed in the open."
        description="There are no unofficial channels to chase. Issues, contributions, and security reports all live in the repository."
      />

      <div className="contact-band">
        {CONTACT_CHANNELS.map((channel) => (
          <a
            key={channel.title}
            href={channel.href}
            target="_blank"
            rel="noreferrer"
            className="contact-band__item"
          >
            <channel.icon className="size-5" aria-hidden="true" />
            <h3>{channel.title}</h3>
            <p>{channel.description}</p>
            <span>
              {channel.action}
              <ArrowUpRight className="size-3.5" aria-hidden="true" />
            </span>
          </a>
        ))}
      </div>
    </section>
  );
}

function CallToAction() {
  return (
    <section className="cta-band">
      <div className="cta-band__inner">
        <div className="cta-band__copy">
          <p className="section-kicker">Build. Execute. Innovate.</p>
          <h2>Build on the <em>next core.</em></h2>
          <p>
            Start with a live shell in the browser, then take the same engine
            to your terminal and desktop.
          </p>
        </div>
        <div className="cta-band__actions">
          <Button asChild size="lg">
            <Link to="/terminal">
              Open the terminal
              <ArrowRight className="size-4" />
            </Link>
          </Button>
          <Button asChild variant="outline" size="lg">
            <Link to="/downloads">Download KERNEXT</Link>
          </Button>
        </div>
      </div>
      <div className="cta-band__wordmark" aria-hidden="true">
        KERNEXT
      </div>
    </section>
  );
}

// ── New Components for the Redesigned Hero ──

function CopyCommandBox() {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    const command = 'curl -fsSL https://kernext.sh | sh';
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      // Fallback for non-secure contexts
      const textarea = document.createElement('textarea');
      textarea.value = command;
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand('copy');
      document.body.removeChild(textarea);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  }, []);

  return (
    <div className="copy-command-box">
      <div className="install-box">
        <span className="prompt-prefix">$</span>
        <code className="install-command">curl -fsSL https://kernext.sh | sh</code>
        <button
          onClick={handleCopy}
          className="copy-btn"
          aria-label={copied ? 'Copied to clipboard' : 'Copy install command'}
        >
          {copied ? (
            <Check className="size-4 text-emerald-400" aria-hidden="true" />
          ) : (
            <Copy className="size-4 text-muted-foreground hover:text-foreground transition-colors" aria-hidden="true" />
          )}
        </button>
      </div>
      {copied && (
        <span className="copy-toast">Copied!</span>
      )}
    </div>
  );
}

function MetadataBadges() {
  return (
    <div className="metadata-badges">
      {METADATA_BADGES.map((badge) => (
        <span key={badge.label} className="metadata-badge">
          <badge.Icon className={`size-3.5 ${badge.color}`} aria-hidden="true" />
          <span className="text-sm font-medium text-foreground">{badge.label}</span>
        </span>
      ))}
    </div>
  );
}

function PillarCard({ pillar, index }: { pillar: Pillar; index: number }) {
  return (
    <motion.article
      className="pillar-card"
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-100px' }}
      transition={{ duration: 0.5, ease: EASE, delay: index * 0.1 }}
    >
      <div className={`pillar-gradient ${pillar.gradient}`} aria-hidden="true" />
      <div className={`pillar-content ${pillar.borderColor} ${pillar.hoverBorder}`}>
        <div className="pillar-icon-wrapper">
          <pillar.icon className="size-6 text-foreground" aria-hidden="true" />
        </div>
        <h3 className="pillar-title">{pillar.title}</h3>
        <p className="pillar-description">{pillar.description}</p>
        <div className="pillar-tags">
          {pillar.tags.map((tag) => (
            <span key={tag} className="pillar-tag">{tag}</span>
          ))}
        </div>
      </div>
    </motion.article>
  );
}

function PillarsSection() {
  return (
    <section id="pillars" className="home-section home-section--ruled">
      <SectionHead
        index="03 / Core Pillars"
        eyebrow="Three surfaces, one engine"
        title="Every interface. Same execution core."
        description="Not wrappers, not simulations — each surface is a first-class entry point to the identical Rust engine."
      />

      <div className="pillars-grid">
        {PILLARS.map((pillar, i) => (
          <PillarCard key={pillar.title} pillar={pillar} index={i} />
        ))}
      </div>
    </section>
  );
}

function LiveTerminalPreview() {
  return (
    <div className="live-terminal-preview" role="img" aria-label="Live terminal preview showing KERNEXT commands executing">
      {/* macOS/Linux style window title bar */}
      <div className="terminal-titlebar">
        <div className="terminal-dots" aria-hidden="true">
          <span className="dot close" />
          <span className="dot minimize" />
          <span className="dot maximize" />
        </div>
        <div className="terminal-tab">
          <TerminalSquare className="size-3.5" aria-hidden="true" />
          <span>kernext — zsh</span>
        </div>
        <div className="terminal-badge-wrapper">
          <span className="pty-badge">
            <span className="pty-dot" aria-hidden="true" />
            PTY: active
          </span>
        </div>
      </div>
      {/* Terminal body with proper padding and content */}
      <div className="terminal-body">
        <div className="terminal-content">
          <div className="terminal-line">
            <span className="prompt">user@kernext:~$</span>
            <span className="command"> ls -la</span>
          </div>
          <div className="output">
            drwxr-xr-x  2 user user  4096 Jan 15 10:30 <span className="dir">src</span>
            <span className="dim">-rw-r--r--  1 user user   847 Jan 15 10:29 Cargo.toml</span>
            <span className="dim">-rw-r--r--  1 user user  1234 Jan 15 10:29 README.md</span>
          </div>
          <div className="terminal-line">
            <span className="prompt">user@kernext:~$</span>
            <span className="command"> cargo run --release</span>
          </div>
          <div className="output">
            <span className="info">   Compiling kernext v0.1.0</span>
            <span className="success">    Finished release [optimized] in 2.3s</span>
            <span className="info">    Running `target/release/kernext`</span>
          </div>
          <div className="terminal-line">
            <span className="prompt">user@kernext:~$</span>
            <span className="command"></span>
            <span className="cursor" aria-hidden="true">█</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function HeroVisual() {
  return (
    <div className="hero-visual">
      {/* Background orbital glow */}
      <div className="hero-glow" aria-hidden="true" />

      {/* Glowing KI core emblem behind top-right of terminal */}
      <div className="ki-core-emblem" aria-hidden="true" />

      {/* 3D K Core (OrbitalHero) - positioned behind terminal as subtle glow */}
      <div className="orbital-core-wrapper">
        <Suspense
          fallback={
            <div className="orbital-hero">
              <CoreSceneFallback />
            </div>
          }
        >
          <OrbitalHero />
        </Suspense>
      </div>

      {/* Live Terminal Preview - properly positioned floating window */}
      <div className="terminal-overlay-wrapper">
        <LiveTerminalPreview />
      </div>
    </div>
  );
}

function Hero() {
  return (
    <section className="core-hero">
      {/* Ambient background mesh/spotlight */}
      <div className="core-hero__ambient" aria-hidden="true" />

      {/* Radial mesh gradient backdrop */}
      <div className="core-hero__mesh" aria-hidden="true" />

      <div className="core-hero__grid">
        <motion.div
          className="core-hero__copy"
          initial={{ opacity: 0, y: 14 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, ease: EASE }}
        >
          {/* Enhanced status badge */}
          <div className="core-hero__status">
            <div className="core-hero__pulse-wrapper">
              <span className="core-hero__pulse" aria-hidden="true" />
            </div>
            <b>Engine online</b>
            <span>v{KERNEXT_VERSION}</span>
          </div>

          <h1 className="core-hero__headline">
            The shell, <em>{'re‑engineered'}</em> from the core.
          </h1>

          <p className="core-hero__lede">
            KERNEXT is an independent Rust computing environment: one native
            engine behind a terminal, a browser workspace, and a desktop
            application. Real PTYs. Real processes. No simulated shell.
          </p>

          <div className="core-hero__actions">
            <Button asChild size="lg">
              <Link to="/terminal">
                <TerminalSquare className="size-4" aria-hidden="true" />
                Open the terminal
              </Link>
            </Button>
            <Button asChild variant="outline" size="lg">
              <Link to="/features">
                Explore the engine
                <ArrowRight className="size-4" aria-hidden="true" />
              </Link>
            </Button>
          </div>

          {/* Copy command box */}
          <CopyCommandBox />

          <MetadataBadges />
        </motion.div>

        <motion.div
          className="core-hero__stage"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.9, ease: EASE, delay: 0.05 }}
        >
          <HeroVisual />
        </motion.div>
      </div>

      <nav className="hero-rail" aria-label="KERNEXT surfaces">
        <Link to="/docs" className="hero-rail__item">
          <span>01</span>
          <div>
            <b>CLI</b>
            <p>Native prompt and execution</p>
          </div>
          <ArrowUpRight className="size-4" aria-hidden="true" />
        </Link>
        <Link to="/terminal" className="hero-rail__item">
          <span>02</span>
          <div>
            <b>Web terminal</b>
            <p>A real PTY in the browser</p>
          </div>
          <ArrowUpRight className="size-4" aria-hidden="true" />
        </Link>
        <Link to="/downloads" className="hero-rail__item">
          <span>03</span>
          <div>
            <b>Desktop</b>
            <p>Local runtime, native window</p>
          </div>
          <ArrowUpRight className="size-4" aria-hidden="true" />
        </Link>
      </nav>
    </section>
  );
}

export function HomePage() {
  return (
    <div className="kernext-home">
      <Hero />
      <PillarsSection />
      <SurfacesSection />
      <PlatformSection />
      <EngineSection />
      <FeaturesSection />
      <ArchitectureSection />
      <BenefitsSection />
      <TerminalSection />
      <DocumentationSection />
      <DownloadsSection />
      <AboutSection />
      <ContactSection />
      <CallToAction />
    </div>
  );
}