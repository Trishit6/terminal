import { Link } from '@tanstack/react-router';
import {
  ArrowDown,
  ArrowRight,
  ArrowUpRight,
  Check,
  Cpu,
  Globe2,
  Monitor,
  Play,
  Server,
  Terminal,
  TerminalSquare,
} from 'lucide-react';
import { OrbitalHero } from '@/components/site/orbital-hero';
import { PlatformMark } from '@/components/site/platform-mark';
import { Button } from '@/components/ui/button';
import { KERNEXT_VERSION } from '@/lib/version';

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
      'External programs launch through operating-system process APIs with arguments kept intact.',
  },
  {
    number: '03',
    title: 'Interactive terminal',
    detail:
      'ConPTY on Windows and openpty on Unix provide full-screen programs with a real terminal.',
  },
  {
    number: '04',
    title: 'Pipelines and jobs',
    detail:
      'Native pipelines, redirections, and background jobs run through the shared Rust engine.',
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
      <CallToAction />
    </div>
  );
}

function Hero() {
  return (
    <section className="home-hero">
      <div className="home-hero__copy">
        <p className="home-eyebrow">
          <span className="home-eyebrow__rule" />
          Independent. Native. Engineered.
        </p>
        <h1>
          The Next Core
          <br />
          of Computing<span className="text-primary">.</span>
        </h1>
        <p className="home-hero__description">
          KERNEXT is an independent, Rust-powered computing environment bringing
          a consistent shell experience to your terminal, browser and desktop.
        </p>
        <div className="home-hero__actions">
          <Button asChild size="lg">
            <Link to="/terminal">
              <Play className="size-4 fill-current" />
              Open Terminal
            </Link>
          </Button>
          <Button asChild variant="outline" size="lg">
            <Link to="/features">
              Explore Features
              <ArrowRight className="size-4" />
            </Link>
          </Button>
        </div>
      </div>

      <div className="home-hero__object">
        <OrbitalHero />
        <div className="home-hero__annotation home-hero__annotation--left">
          <span>01</span>
          <span>Rust runtime</span>
        </div>
        <div className="home-hero__annotation home-hero__annotation--right">
          <span>02</span>
          <span>One engine · three surfaces</span>
        </div>
      </div>

      <a className="home-hero__scroll" href="#surfaces">
        Explore the system
        <ArrowDown className="size-3.5" />
      </a>
    </section>
  );
}

function SectionHeading({
  eyebrow,
  title,
  description,
}: {
  eyebrow: string;
  title: string;
  description?: string;
}) {
  return (
    <div className="home-section-heading">
      <p className="home-eyebrow">
        <span className="home-eyebrow__rule" />
        {eyebrow}
      </p>
      <h2>{title}</h2>
      {description ? <p>{description}</p> : null}
    </div>
  );
}

function SurfacesSection() {
  return (
    <section id="surfaces" className="home-section home-surfaces">
      <div className="home-section__inner">
        <SectionHeading
          eyebrow="One engine. Three surfaces."
          title="A consistent shell, wherever you work."
          description="Each surface is built around the same Rust engine—not a separate interpretation of it."
        />
        <div className="surface-showcase">
          <article className="surface-showcase__item surface-showcase__item--cli">
            <div className="surface-showcase__meta">
              <span>01 / Interface preview</span>
              <Terminal className="size-4" />
            </div>
            <div className="surface-cli-window">
              <div className="surface-cli-window__line">
                <span>kernext ~ $</span> kernext version
              </div>
              <p>KERNEXT v{KERNEXT_VERSION}</p>
              <div className="surface-cli-window__line">
                <span>kernext ~ $</span> echo &quot;ready&quot;
              </div>
              <p>ready</p>
            </div>
            <h3>KERNEXT CLI</h3>
            <p>Native prompt, command history and process execution.</p>
            <Link to="/docs" className="home-text-link">
              CLI documentation <ArrowUpRight className="size-3.5" />
            </Link>
          </article>

          <article className="surface-showcase__item surface-showcase__item--web">
            <div className="surface-showcase__meta">
              <span>02 / Browser</span>
              <Globe2 className="size-4" />
            </div>
            <div className="surface-web-window" aria-hidden="true">
              <div className="surface-web-window__bar">
                <span />
                <span />
                <span />
                <i>kernext / terminal</i>
              </div>
              <div className="surface-web-window__body">
                <span className="text-primary">kernext ~ $</span>
                <span> ls</span>
                <span className="surface-web-window__output">src&nbsp;&nbsp; Cargo.toml&nbsp;&nbsp; README.md</span>
              </div>
            </div>
            <h3>KERNEXT Web</h3>
            <p>A browser terminal with a custom renderer and real PTY transport.</p>
            <Link to="/terminal" className="home-text-link">
              Open Web Terminal <ArrowUpRight className="size-3.5" />
            </Link>
          </article>

          <article className="surface-showcase__item surface-showcase__item--desktop">
            <div className="surface-showcase__meta">
              <span>03 / Desktop</span>
              <Monitor className="size-4" />
            </div>
            <div className="surface-desktop-window" aria-hidden="true">
              <div className="surface-desktop-window__rail">
                <span className="surface-desktop-window__mark">K</span>
                <span />
                <span />
                <span />
              </div>
              <div className="surface-desktop-window__screen">
                <b>KERNEXT</b>
                <span>Local runtime</span>
                <i />
              </div>
            </div>
            <h3>KERNEXT Desktop</h3>
            <p>A Tauri application that runs through the local Rust runtime.</p>
            <Link to="/downloads" className="home-text-link">
              Desktop build notes <ArrowUpRight className="size-3.5" />
            </Link>
          </article>
        </div>
      </div>
    </section>
  );
}

function EngineSection() {
  return (
    <section className="home-section home-engine">
      <div className="home-section__inner home-engine__layout">
        <SectionHeading
          eyebrow="Engineered for real work"
          title="The shell owns the semantics."
          description="KERNEXT parses and evaluates its own command language. It does not hand shell syntax to another interpreter."
        />
        <div className="capability-list">
          {CAPABILITIES.map((item) => (
            <article key={item.number} className="capability-list__item">
              <span className="capability-list__number">{item.number}</span>
              <div>
                <h3>{item.title}</h3>
                <p>{item.detail}</p>
              </div>
              <Check className="size-4 text-primary" aria-hidden="true" />
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}

function PlatformSection() {
  return (
    <section className="home-section home-platforms">
      <div className="home-section__inner">
        <div className="home-platforms__heading">
          <SectionHeading
            eyebrow="Cross-platform by design"
            title="Built for the systems you use."
            description="The workspace targets Windows, Linux and macOS, with platform-specific process and terminal backends."
          />
          <Link to="/downloads" className="home-text-link">
            Platform build details <ArrowUpRight className="size-3.5" />
          </Link>
        </div>
        <div className="platform-list">
          <div className="platform-list__item">
            <PlatformMark platform="windows" />
            <div>
              <h3>Windows</h3>
              <p>Native process resolution · ConPTY · Tauri</p>
            </div>
            <span>Supported</span>
          </div>
          <div className="platform-list__item">
            <PlatformMark platform="linux" />
            <div>
              <h3>Linux</h3>
              <p>Native process execution · openpty · Tauri</p>
            </div>
            <span>Supported</span>
          </div>
          <div className="platform-list__item">
            <PlatformMark platform="macos" />
            <div>
              <h3>macOS</h3>
              <p>Native process execution · openpty · Tauri</p>
            </div>
            <span>Supported</span>
          </div>
        </div>
      </div>
    </section>
  );
}

function ArchitectureSection() {
  return (
    <section id="architecture" className="home-section home-architecture">
      <div className="home-section__inner">
        <SectionHeading
          eyebrow="A shared architecture"
          title="One engine. Clear boundaries."
          description="The same execution core serves the terminal, browser and desktop while each surface keeps its own interface."
        />
        <div className="architecture-map" aria-label="KERNEXT architecture">
          <div className="architecture-lane">
            <h3><Terminal className="size-4" /> CLI</h3>
            <div className="architecture-lane__step">Reedline interface</div>
            <ArrowDown className="architecture-lane__arrow" />
            <div className="architecture-lane__step">shell-cli</div>
          </div>
          <div className="architecture-lane">
            <h3><Globe2 className="size-4" /> Web</h3>
            <div className="architecture-lane__step">React terminal UI</div>
            <ArrowDown className="architecture-lane__arrow" />
            <div className="architecture-lane__step"><Server className="size-3.5" /> Axum API + PTY</div>
          </div>
          <div className="architecture-lane">
            <h3><Monitor className="size-4" /> Desktop</h3>
            <div className="architecture-lane__step">Shared React UI</div>
            <ArrowDown className="architecture-lane__arrow" />
            <div className="architecture-lane__step">Tauri + local worker</div>
          </div>
          <div className="architecture-map__shared">
            <Cpu className="size-5 text-primary" />
            <div>
              <strong>Shared Rust shell engine</strong>
              <span>shell-core · shell-builtins · shell-runtime · shell-api</span>
            </div>
            <span className="architecture-map__shared-detail">
              Lexer → parser → evaluator → process runtime
            </span>
          </div>
        </div>
      </div>
    </section>
  );
}

function TerminalSection() {
  return (
    <section className="home-section home-terminal">
      <div className="home-section__inner home-terminal__layout">
        <div>
          <p className="home-eyebrow">
            <span className="home-eyebrow__rule" />
            Terminal experience
          </p>
          <h2>A real shell, in your browser.</h2>
          <p className="home-terminal__description">
            The Web Terminal connects to the local Rust runtime. This short
            example is static; open the terminal to run commands yourself.
          </p>
          <Button asChild size="lg">
            <Link to="/terminal">
              <TerminalSquare className="size-4" />
              Open Web Terminal
              <ArrowRight className="size-4" />
            </Link>
          </Button>
        </div>
        <div className="home-terminal-window" data-terminal-preview="example">
          <div className="home-terminal-window__titlebar">
            <div className="home-terminal-window__controls" aria-hidden="true">
              <span />
              <span />
              <span />
            </div>
            <span>Example session · KERNEXT</span>
            <span className="home-terminal-window__local">STATIC</span>
          </div>
          <div className="home-terminal-window__content">
            <p><span>kernext ~ $</span> version</p>
            <p className="home-terminal-window__output">KERNEXT v{KERNEXT_VERSION}</p>
            <p><span>kernext ~ $</span> echo hello-from-kernext</p>
            <p className="home-terminal-window__output">hello-from-kernext</p>
            <p><span>kernext ~ $</span> pwd</p>
            <p className="home-terminal-window__output">/home/you/Projects/kernext</p>
          </div>
        </div>
      </div>
    </section>
  );
}

function CallToAction() {
  return (
    <section className="home-cta">
      <div className="home-cta__inner">
        <div>
          <p className="home-eyebrow">
            <span className="home-eyebrow__rule" />
            Build. Execute. Innovate.
          </p>
          <h2>KERNEXT</h2>
          <p>The Next Core of Computing.</p>
        </div>
        <Button asChild size="lg">
          <Link to="/terminal">
            Start with the terminal
            <ArrowRight className="size-4" />
          </Link>
        </Button>
      </div>
    </section>
  );
}
