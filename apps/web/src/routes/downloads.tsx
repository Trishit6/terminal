import { Link } from '@tanstack/react-router';
import {
  ArrowRight,
  ArrowUpRight,
  Check,
  Download,
  ExternalLink,
  FileArchive,
  Package,
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
import { Separator } from '@/components/ui/separator';
import { PlatformMark } from '@/components/site/platform-mark';
import { KERNEXT_VERSION } from '@/lib/version';

const EASE = [0.22, 1, 0.36, 1] as const;

/** The repository that hosts the source and its releases. */
const REPOSITORY_URL = 'https://github.com/Trishit6/terminal';

const PLATFORMS = [
  {
    brand: 'windows',
    name: 'Windows',
    targets: ['x86_64', 'Windows 10 or later'],
    artifacts: [
      'CLI executable: `kernext.exe` (portable archive)',
      'Desktop installer produced by Tauri (`npm run desktop:build`)',
    ],
  },
  {
    brand: 'linux',
    name: 'Linux',
    targets: ['x86_64', 'glibc and musl targets'],
    artifacts: [
      'CLI executable: `kernext` (tar.gz archives)',
      'Desktop packages produced by Tauri on Linux runners',
      'Shell installer: `installers/install-linux.sh`',
    ],
  },
  {
    brand: 'macos',
    name: 'macOS',
    targets: ['Apple Silicon', 'Intel'],
    artifacts: [
      'CLI executable: `kernext` (tar.gz archives)',
      'Desktop `.app` bundle and `.dmg` produced by Tauri',
      'Shell installer: `installers/install-macos.sh`',
    ],
  },
] as const;

export function DownloadsPage() {
  return (
    <div className="mx-auto w-full max-w-[1100px] px-4 py-14 sm:px-6 lg:px-8 lg:py-20">
      <div className="max-w-3xl">
        <Badge variant="outline">Downloads</Badge>
        <h1 className="mt-4 text-balance text-4xl font-semibold tracking-[-0.04em] sm:text-5xl">
          Get the engine.
          <span className="text-gradient"> From source or release.</span>
        </h1>
        <p className="mt-5 text-sm leading-relaxed text-muted-foreground sm:text-base">
          KERNEXT is built by the repository&apos;s CI on Windows, Linux, and
          macOS. Release archives carry the CLI executable and desktop
          packages, each published with SHA-256 checksums. The current source
          version is{' '}
          <span className="font-mono text-foreground">v{KERNEXT_VERSION}</span>.
        </p>
      </div>

      <div className="mt-8 flex flex-wrap gap-3">
        <Button asChild>
          <a href={`${REPOSITORY_URL}/releases`} target="_blank" rel="noreferrer">
            <Download className="size-4" />
            Releases on GitHub
            <ArrowUpRight className="size-3.5 opacity-70" />
          </a>
        </Button>
        <Button asChild variant="outline">
          <Link to="/docs">
            <TerminalSquare className="size-4" />
            Build from source
          </Link>
        </Button>
      </div>

      <div className="mt-6 rounded-2xl border border-border bg-surface/40 p-5">
        <p className="text-xs leading-relaxed text-muted-foreground">
          <span className="font-medium text-foreground">Honest download policy:</span>{' '}
          this page links only to the repository&apos;s own releases and build
          instructions. If no release has been published yet, the releases
          page will be empty — build from source instead. No mirrors, no
          third-party binaries, and no version numbers that were not produced
          by the release workflow.
        </p>
      </div>

      <Separator className="my-12" />

      <section id="platforms" className="scroll-mt-24">
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Platforms
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          What each platform ships
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          The CLI is a single static-friendly executable. The desktop app is a
          Tauri 2 bundle produced per platform. Both come from the same Cargo
          workspace, so the version reported by{' '}
          <span className="font-mono">kernext version</span> always matches the
          release tag.
        </p>

        <div className="mt-8 grid gap-5 lg:grid-cols-3">
          {PLATFORMS.map((platform, index) => (
            <motion.div
              key={platform.name}
              initial={{ opacity: 0, y: 18 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: '-60px' }}
              transition={{ duration: 0.5, ease: EASE, delay: index * 0.05 }}
            >
              <Card className="h-full bg-surface/55">
                <CardHeader className="gap-3">
                  <div className="flex items-center justify-between">
                    <div className="grid size-10 place-items-center rounded-xl border border-border bg-background/70 text-primary">
                      <PlatformMark platform={platform.brand} className="size-5" />
                    </div>
                    <Badge variant="secondary">CLI + Desktop</Badge>
                  </div>
                  <div>
                    <CardTitle className="text-base">{platform.name}</CardTitle>
                    <CardDescription className="mt-1">
                      {platform.targets.join(' · ')}
                    </CardDescription>
                  </div>
                </CardHeader>
                <CardContent className="space-y-2.5">
                  {platform.artifacts.map((artifact) => (
                    <div
                      key={artifact}
                      className="flex items-start gap-2 text-xs text-muted-foreground"
                    >
                      <Check className="mt-0.5 size-3.5 shrink-0 text-success" />
                      {artifact}
                    </div>
                  ))}
                </CardContent>
              </Card>
            </motion.div>
          ))}
        </div>
      </section>

      <Separator className="my-12" />

      <section id="cli" className="scroll-mt-24">
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          CLI
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          Build the CLI executable
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          The CLI is the reference surface. A release build produces the{' '}
          <span className="font-mono">kernext</span> executable used by the
          prompt, pipelines, scripting, and background jobs.
        </p>

        <div className="mt-6 overflow-hidden rounded-2xl border border-border bg-[#04070c] text-[#d8e6f5]">
          <div className="flex items-center justify-between border-b border-white/10 px-4 py-2.5">
            <span className="font-mono text-[10px] uppercase tracking-[0.16em] text-white/45">
              From a checkout
            </span>
            <span className="font-mono text-[10px] text-white/30">
              Rust 1.85+
            </span>
          </div>
          <pre className="overflow-x-auto p-4 font-mono text-[11px] leading-relaxed sm:text-xs">
            <code>{`git clone ${REPOSITORY_URL}.git
cd terminal
cargo build --workspace --release

# the executable
./target/release/kernext   # Linux / macOS
.\\target\\release\\kernext.exe  # Windows`}</code>
          </pre>
        </div>

        <p className="mt-4 text-xs leading-relaxed text-muted-foreground">
          The repository also carries platform installers under{' '}
          <span className="font-mono">installers/</span> that install a built
          executable into a standard location and verify the checksum of what
          they copy.
        </p>
      </section>

      <Separator className="my-12" />

      <section id="desktop" className="scroll-mt-24">
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Desktop
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          Build the desktop application
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          The Tauri 2 app wraps the same engine. Building it produces the
          platform bundle (installer on Windows, `.app` and `.dmg` on macOS,
          Linux packages) from one command.
        </p>

        <div className="mt-6 overflow-hidden rounded-2xl border border-border bg-[#04070c] text-[#d8e6f5]">
          <div className="flex items-center justify-between border-b border-white/10 px-4 py-2.5">
            <span className="font-mono text-[10px] uppercase tracking-[0.16em] text-white/45">
              Prerequisites
            </span>
            <span className="font-mono text-[10px] text-white/30">
              npm · Rust · platform toolchain
            </span>
          </div>
          <pre className="overflow-x-auto p-4 font-mono text-[11px] leading-relaxed sm:text-xs">
            <code>{`npm ci
npm run web:build
npm run desktop:build

# development window with hot reload
npm run desktop:dev`}</code>
          </pre>
        </div>

        <div className="mt-4 grid gap-4 sm:grid-cols-2">
          <Card className="p-5">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Package className="size-4 text-primary" />
              Bundles per platform
            </div>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              Linux needs the Tauri system libraries (webkit2gtk and friends);
              Windows needs WebView2; macOS needs Xcode command line tools.
              See <span className="font-mono">docs/DESKTOP.md</span>.
            </p>
          </Card>
          <Card className="p-5">
            <div className="flex items-center gap-2 text-sm font-medium">
              <ShieldCheck className="size-4 text-success" />
              Local execution only
            </div>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              The desktop app runs commands through its embedded Rust runtime.
              It never requires the Axum web server and never executes
              through a public network service.
            </p>
          </Card>
        </div>
      </section>

      <Separator className="my-12" />

      <section id="checksums" className="scroll-mt-24">
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Integrity
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          Verifying a download
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          Every artifact published by the release workflow is accompanied by a
          SHA-256 checksum file with the same name plus a{' '}
          <span className="font-mono">.sha256</span> suffix.
        </p>

        <div className="mt-6 overflow-hidden rounded-2xl border border-border bg-[#04070c] text-[#d8e6f5]">
          <div className="flex items-center justify-between border-b border-white/10 px-4 py-2.5">
            <span className="font-mono text-[10px] uppercase tracking-[0.16em] text-white/45">
              Checksum verification
            </span>
            <span className="font-mono text-[10px] text-white/30">sha256</span>
          </div>
          <pre className="overflow-x-auto p-4 font-mono text-[11px] leading-relaxed sm:text-xs">
            <code>{`sha256sum -c kernext-x86_64-unknown-linux-gnu.tar.gz.sha256

# PowerShell
Get-FileHash kernext-x86_64-pc-windows-msvc.zip -Algorithm SHA256`}</code>
          </pre>
        </div>

        <p className="mt-4 text-xs leading-relaxed text-muted-foreground">
          <span className="font-medium text-foreground">Releases are
          manual:</span> the workflow produces a draft GitHub release with
          generated notes; nothing is published automatically. See{' '}
          <span className="font-mono">docs/PROJECT_STATUS.md</span> for the
          current release-artifact state.
        </p>
      </section>

      <div className="mt-12 flex flex-col items-start justify-between gap-4 rounded-2xl border border-border-strong bg-surface p-6 sm:flex-row sm:items-center">
        <div>
          <p className="text-sm font-medium">Questions before installing?</p>
          <p className="mt-1 text-xs text-muted-foreground">
            The documentation covers configuration, security, and
            troubleshooting.
          </p>
        </div>
        <div className="flex flex-wrap gap-3">
          <Button asChild variant="outline">
            <Link to="/docs">
              Documentation
              <ArrowRight className="size-4" />
            </Link>
          </Button>
          <Button asChild variant="outline">
            <a href={REPOSITORY_URL} target="_blank" rel="noreferrer">
              Source repository
              <ExternalLink className="size-4" />
            </a>
          </Button>
        </div>
      </div>

      <div className="mt-8 flex items-center justify-center gap-2 text-xs text-muted-foreground">
        <FileArchive className="size-3.5" />
        Archives, installers, and checksums are produced by CI from this
        repository — never by hand.
      </div>
    </div>
  );
}
