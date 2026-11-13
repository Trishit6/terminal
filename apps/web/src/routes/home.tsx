import { Suspense, lazy } from 'react';
import { Link } from '@tanstack/react-router';
import {
  ArrowRight,
  ArrowUpRight,
  Cpu,
  Globe2,
  Monitor,
  Play,
  Terminal,
  TerminalSquare,
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

const CAPABILITIES = [
  {
    number: '01',
    title: 'Native command engine',
    detail:
      'A hand-written lexer, parser, AST, evaluator, and 56 native built-ins give KERNEXT ownership of its command language.',
  },
  {
    number: '02',
    title: 'Real process execution',
    detail:
      'External programs launch through operating-system process APIs with argument boundaries preserved.',
  },
  {
    number: '03',
    title: 'Interactive terminal',
    detail:
      'ConPTY on Windows and openpty on Unix give full-screen programs a real terminal with live resize.',
  },
  {
    number: '04',
    title: 'Pipelines and jobs',
    detail:
      'Native pipelines, redirections, and background jobs all run through the same shared Rust engine.',
  },
] as const;

const SURFACES = [
  {
    number: '01',
    name: 'CLI',
    icon: Terminal,
    title: 'KERNEXT CLI',
    description:
      'The reference surface. Reedline prompt, history, completion, native built-ins, and real process execution.',
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
      'A browser terminal with its own ANSI screen buffer and a real PTY WebSocket transport to the local server.',
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
      'A Tauri 2 application that executes through its embedded Rust runtime, never a public server.',
    tags: ['Tauri 2', 'local runtime', 'shared UI'],
    to: '/downloads',
    cta: 'Desktop builds',
    preview: 'desktop',
  },
] as const;

export function HomePage() {
  return (
    <div className="kernext-home">
      <Hero />
      <SurfacesSection />
      <EngineSection />
      <PlatformSection />
      <ArchitectureSection />
      <TerminalSection />
      <DocumentationSection />
      <CallToAction />
    </div>
  );
}

function Hero() {
  return (
    <section className="hero">
      <div className="hero__grid">
        <motion.div
          className="hero__copy"
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.65, ease: EASE }}
        >
          <div className="hero__status">
            <span className="hero__pulse" aria-hidden="true" />
            <b>System online</b>
            <span>v{KERNEXT_VERSION}</span>
          </div>

          <h1>
            The shell, <span>re-engineered</span>
            <em>from the core.</em>
          </h1>

          <p className="hero__lede">
            KERNEXT is an independent, Rust-powered computing environment: one
            native engine behind a terminal, a browser workspace, and a desktop
            application. Real PTYs. Real processes. No simulated shell.
          </p>

          <div className="hero__actions">
            <Button asChild size="lg">
              <Link to="/terminal">
                <Play className="size-4 fill-current" />
                Open the terminal
              </Link>
            </Button>
            <Button asChild variant="outline" size="lg">
              <Link to="/features">
                Explore the engine
                <ArrowRight className="size-4" />
              </Link>
            </Button>
          </div>

          <ul className="hero__meta">
            <li>Rust engine</li>
            <li>Windows · Linux · macOS</li>
            <li>100% native parsing</li>
          </ul>
        </motion.div>

        <motion.div
          className="hero__stage"
          initial={{ opacity: 0, scale: 0.97 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.8, ease: EASE, delay: 0.08 }}
        >
          <div className="hero__stage-grid" aria-hidden="true" />
          <Suspense
            fallback={
              <div className="orbital-hero">
                <CoreSceneFallback />
              </div>
            }
          >
            <OrbitalHero />
          </Suspense>
          <span className="hero__tag hero__tag--a" aria-hidden="true">
            <i /> Rust engine
          </span>
          <span className="hero__tag hero__tag--b" aria-hidden="true">
            <i /> Native PTY
          </span>
          <span className="hero__tag hero__tag--c" aria-hidden="true">
            <i /> Cross-platform
          </span>
          <div className="hero__readout" aria-hidden="true">
            <span>
              <b>Core</b> shell-engine
            </span>
            <span>
              <b>State</b> Ready
            </span>
          </div>
        </motion.div>
      </div>

      <nav className="hero__rail" aria-label="KERNEXT surfaces">
        <Link to="/docs" className="hero__rail-item">
          <span>01</span>
          <div>
            <b>CLI</b>
            <p>Native prompt and execution</p>
          </div>
          <ArrowUpRight className="size-4" aria-hidden="true" />
        </Link>
        <Link to="/terminal" className="hero__rail-item">
          <span>02</span>
          <div>
            <b>Web Terminal</b>
            <p>Real PTY in the browser</p>
          </div>
          <ArrowUpRight className="size-4" aria-hidden="true" />
        </Link>
        <Link to="/downloads" className="hero__rail-item">
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
      initial={{ opacity: 0, y: 14 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-80px' }}
      transition={{ duration: 0.55, ease: EASE }}
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

function EngineSection() {
  return (
    <section id="engine" className="home-section home-section--ruled">
      <SectionHead
        index="02 / Engine"
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

        <ol className="spec-list">
          {CAPABILITIES.map((item) => (
            <li key={item.number} className="spec-row">
              <span className="spec-row__number">{item.number}</span>
              <div>
                <h3>{item.title}</h3>
                <p>{item.detail}</p>
              </div>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}

function PlatformSection() {
  return (
    <section className="platform-band">
      <div className="platform-band__inner">
        <div className="grid gap-4 px-4 py-10 sm:px-6 md:grid-cols-[0.7fr_1.3fr] md:items-end lg:px-8 lg:py-12">
          <div>
            <p className="section-kicker">
              <span>03 / Platforms</span>
              Cross-platform by design
            </p>
            <h2 className="mt-3 text-3xl font-[545] tracking-[-0.05em] sm:text-4xl">
              Built for the systems you use.
            </h2>
          </div>
          <p className="max-w-xl text-sm leading-relaxed text-muted-foreground md:pb-1">
            The workspace targets Windows, Linux, and macOS with
            platform-specific process resolution and terminal backends. The
            same engine and the same tests run on every target.
          </p>
        </div>

        <div className="platform-band__grid">
          <div className="platform-band__item">
            <PlatformMark platform="windows" />
            <div>
              <span className="platform-band__label">Target 01</span>
              <h3>Windows</h3>
              <p>Native process resolution · ConPTY · Tauri 2</p>
            </div>
            <span className="platform-band__state">Supported</span>
          </div>
          <div className="platform-band__item">
            <PlatformMark platform="linux" />
            <div>
              <span className="platform-band__label">Target 02</span>
              <h3>Linux</h3>
              <p>Native process execution · openpty · Tauri 2</p>
            </div>
            <span className="platform-band__state">Supported</span>
          </div>
          <div className="platform-band__item">
            <PlatformMark platform="macos" />
            <div>
              <span className="platform-band__label">Target 03</span>
              <h3>macOS</h3>
              <p>Apple Silicon and Intel · openpty · Tauri 2</p>
            </div>
            <span className="platform-band__state">Supported</span>
          </div>
        </div>

        <div className="platform-band__note">
          <span>CI matrix · Windows x64 · Linux x64 · macOS arm64 / x64</span>
          <Link to="/downloads">
            Build and release details
            <ArrowUpRight className="size-3.5" aria-hidden="true" />
          </Link>
        </div>
      </div>
    </section>
  );
}

function ArchitectureSection() {
  return (
    <section id="architecture" className="home-section home-section--ruled">
      <SectionHead
        index="04 / Architecture"
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

function TerminalSection() {
  return (
    <section className="home-section home-section--ruled">
      <div className="terminal-split">
        <div>
          <p className="section-kicker">
            <span>05 / Experience</span>
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
              <span>Appearance</span>
              <b>Saved per browser or account</b>
            </li>
          </ul>

          <div className="terminal-split__actions">
            <Button asChild size="lg">
              <Link to="/terminal">
                <TerminalSquare className="size-4" />
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
                echo {"open the terminal to run your own"}
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
        index="06 / Documentation"
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
            <Link to="/downloads">
              Download KERNEXT
            </Link>
          </Button>
        </div>
      </div>
      <div className="cta-band__wordmark" aria-hidden="true">
        KERNEXT
      </div>
    </section>
  );
}