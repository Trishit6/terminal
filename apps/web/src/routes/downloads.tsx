import { Link } from '@tanstack/react-router';
import {
  AlertTriangle,
  ArrowRight,
  ArrowUpRight,
  Check,
  Download,
  ExternalLink,
  Loader2,
  Package,
  ShieldCheck,
  TerminalSquare,
} from 'lucide-react';
import { motion } from 'motion/react';
import { PageAsideRow, PageShell } from '@/components/site/page-shell';
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
import {
  DEFAULT_REPOSITORY,
  archLabel,
  cliAssets,
  formatBytes,
  formatLabel,
  formatReleaseDate,
  platformLabel,
  platformTargets,
  repositoryUrl,
  type PlatformTarget,
} from '@/lib/releases';
import { useLatestRelease } from '@/hooks/use-latest-release';

const EASE = [0.22, 1, 0.36, 1] as const;

const PLATFORMS = [
  {
    brand: 'windows',
    name: 'Windows',
    targets: 'x64',
    artifacts: ['`.msi` and `-setup.exe` desktop installers', 'CLI executable `kernext.exe`'],
  },
  {
    brand: 'linux',
    name: 'Linux',
    targets: 'x64',
    artifacts: ['`.AppImage` and `.deb` desktop packages', 'CLI executable `kernext`'],
  },
  {
    brand: 'macos',
    name: 'macOS',
    targets: 'Apple Silicon · Intel',
    artifacts: ['`.dmg` desktop images for both architectures', 'CLI executable `kernext`'],
  },
] as const;

/** Terminal-style command block used across the page. */
function CommandBlock({
  label,
  hint,
  children,
}: {
  label: string;
  hint: string;
  children: string;
}) {
  return (
    <div className="mt-6 overflow-hidden rounded-2xl border border-border bg-[#04070c] text-[#d8e6f5]">
      <div className="flex items-center justify-between border-b border-white/10 px-4 py-2.5">
        <span className="font-mono text-[10px] uppercase tracking-[0.16em] text-white/45">
          {label}
        </span>
        <span className="font-mono text-[10px] text-white/30">{hint}</span>
      </div>
      <pre className="overflow-x-auto p-4 font-mono text-[11px] leading-relaxed sm:text-xs">
        <code>{children}</code>
      </pre>
    </div>
  );
}

function DownloadCard({ target, index }: { target: PlatformTarget; index: number }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 18 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-60px' }}
      transition={{ duration: 0.5, ease: EASE, delay: index * 0.05 }}
    >
      <Card className="h-full bg-surface/55">
        <CardHeader className="gap-3">
          <div className="flex items-center justify-between">
            <div className="grid size-10 place-items-center rounded-xl border border-border bg-background/70 text-primary">
              <PlatformMark platform={target.platform} className="size-5" />
            </div>
            <Badge variant="success">Stable</Badge>
          </div>
          <div>
            <CardTitle className="text-base">{platformLabel(target.platform)}</CardTitle>
            <CardDescription className="mt-1">
              {archLabel(target.platform, target.arch)}
            </CardDescription>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {target.assets.map((asset) => (
            <div key={asset.name} className="space-y-2">
              <div className="flex flex-wrap items-center gap-3">
                <Button asChild size="sm">
                  <a href={asset.download_url} rel="noreferrer">
                    <Download className="size-3.5" />
                    {formatLabel(asset.format)}
                  </a>
                </Button>
                {asset.size > 0 ? (
                  <span className="font-mono text-[10px] text-muted-foreground">
                    {formatBytes(asset.size)}
                  </span>
                ) : null}
                {asset.checksum_url ? (
                  <a
                    href={asset.checksum_url}
                    rel="noreferrer"
                    className="font-mono text-[10px] text-muted-foreground underline decoration-dotted underline-offset-4 transition-colors hover:text-foreground"
                  >
                    .sha256
                  </a>
                ) : null}
              </div>
              <p className="truncate font-mono text-[10px] text-muted-foreground" title={asset.name}>
                {asset.name}
              </p>
            </div>
          ))}
        </CardContent>
      </Card>
    </motion.div>
  );
}

function ReleaseUnavailable({
  releasesUrl,
  reason,
}: {
  releasesUrl: string;
  reason: string | null;
}) {
  return (
    <Card className="bg-surface/55 p-6">
      <div className="flex items-start gap-3">
        <Package className="mt-0.5 size-4 shrink-0 text-primary" />
        <div>
          <p className="text-sm font-medium text-foreground">
            No published release yet.
          </p>
          <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
            {reason ??
              'The release workflow has not published a GitHub Release for this repository.'}{' '}
            Build from source in the meantime, or watch the repository&apos;s
            releases page.
          </p>
          <div className="mt-4 flex flex-wrap gap-3">
            <Button asChild variant="outline" size="sm">
              <a href={releasesUrl} target="_blank" rel="noreferrer">
                Open GitHub Releases
                <ArrowUpRight className="size-3.5" />
              </a>
            </Button>
            <Button asChild variant="ghost" size="sm">
              <Link to="/docs">
                Build from source
                <ArrowRight className="size-3.5" />
              </Link>
            </Button>
          </div>
        </div>
      </div>
    </Card>
  );
}

export function DownloadsPage() {
  const state = useLatestRelease();
  const release = state.status === 'ready' ? state.release : null;

  const repository = release?.repository ?? DEFAULT_REPOSITORY;
  const repoUrl = repositoryUrl(repository);
  const releasesUrl = `${repoUrl}/releases`;
  const workflowUrl = `${repoUrl}/actions/workflows/desktop-release.yaml`;

  const versionLabel = release?.version ? `v${release.version}` : null;
  const publishedLabel = release ? formatReleaseDate(release.published_at) : null;
  const targets = release?.available ? platformTargets(release) : [];
  const cli = release?.available ? cliAssets(release) : [];
  const notesUrl = release?.html_url ?? releasesUrl;

  return (
    <PageShell
      kicker="04 / Downloads"
      title={
        <>
          Download KERNEXT. <em>Native on every desktop.</em>
        </>
      }
      description={
        <>
          Installers are built by the repository&apos;s release workflow for
          Windows, Linux, and macOS, published on GitHub Releases with SHA-256
          checksums, and listed here from that release data — never invented.{' '}
          {versionLabel ? (
            <>
              Latest stable:{' '}
              <span className="font-mono text-foreground">{versionLabel}</span>
              {publishedLabel ? <> ({publishedLabel})</> : null}.
            </>
          ) : (
            <>
              The source version is{' '}
              <span className="font-mono text-foreground">v{KERNEXT_VERSION}</span>.
            </>
          )}
        </>
      }
      actions={
        <>
          <Button asChild>
            <a href={releasesUrl} target="_blank" rel="noreferrer">
              <Download className="size-4" />
              All releases
              <ArrowUpRight className="size-3.5 opacity-70" />
            </a>
          </Button>
          <Button asChild variant="outline">
            <Link to="/docs">
              <TerminalSquare className="size-4" />
              Build from source
            </Link>
          </Button>
        </>
      }
      aside={
        <>
          <PageAsideRow label="Latest" value={versionLabel ?? 'No release yet'} />
          <PageAsideRow label="Source" value={`v${KERNEXT_VERSION}`} />
          <PageAsideRow label="Checksums" value="SHA-256" />
          <PageAsideRow label="Source" value="GitHub Releases" />
          <p className="page-aside__note">
            Assets, installers, and checksums are produced by CI from this
            repository — never by hand.
          </p>
        </>
      }
    >
      <div className="mt-6 rounded-2xl border border-border bg-surface/40 p-5">
        <p className="text-xs leading-relaxed text-muted-foreground">
          <span className="font-medium text-foreground">Honest download policy:</span>{' '}
          this page reads the latest published GitHub Release and links only to
          the files that release actually contains. If no release has been
          published yet, it says so and points you at the source instead. No
          mirrors, no third-party binaries, and no download button for an
          installer that does not exist.
        </p>
      </div>

      <Separator className="my-12" />

      <section id="latest" className="scroll-mt-24">
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Latest stable release
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          {versionLabel ? `KERNEXT ${versionLabel}` : 'Published installers'}
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          Every installer below is attached to the latest stable GitHub
          Release, with a matching <span className="font-mono">.sha256</span>{' '}
          file.
        </p>

        <div className="mt-8">
          {state.status === 'loading' ? (
            <Card className="bg-surface/55 p-6">
              <div className="flex items-center gap-3 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin text-primary" />
                Reading the latest release from GitHub…
              </div>
            </Card>
          ) : null}

          {state.status === 'error' ? (
            <Card className="border-warning/30 bg-warning/5 p-6">
              <div className="flex items-start gap-3">
                <AlertTriangle className="mt-0.5 size-4 shrink-0 text-warning" />
                <div>
                  <p className="text-sm font-medium text-foreground">
                    Release information is unavailable.
                  </p>
                  <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                    {state.message} Every release stays reachable on GitHub.
                  </p>
                  <div className="mt-4 flex flex-wrap gap-3">
                    <Button asChild variant="outline" size="sm">
                      <a href={releasesUrl} target="_blank" rel="noreferrer">
                        Open GitHub Releases
                        <ArrowUpRight className="size-3.5" />
                      </a>
                    </Button>
                  </div>
                </div>
              </div>
            </Card>
          ) : null}

          {state.status === 'ready' && !release?.available ? (
            <ReleaseUnavailable releasesUrl={releasesUrl} reason={release?.error ?? null} />
          ) : null}

          {state.status === 'ready' && release?.available ? (
            <div className="space-y-6">
              <div className="flex flex-wrap items-center gap-3">
                {release.tag ? <Badge>{release.tag}</Badge> : null}
                {release.name ? (
                  <span className="text-sm font-medium text-foreground">{release.name}</span>
                ) : null}
                {publishedLabel ? (
                  <span className="font-mono text-[11px] text-muted-foreground">
                    {publishedLabel}
                  </span>
                ) : null}
                <a
                  href={notesUrl}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1 text-xs text-primary underline-offset-4 hover:underline"
                >
                  Release notes
                  <ArrowUpRight className="size-3" />
                </a>
              </div>

              {targets.length > 0 ? (
                <div className="grid gap-5 lg:grid-cols-3">
                  {targets.map((target, index) => (
                    <DownloadCard
                      key={`${target.platform}-${target.arch}`}
                      target={target}
                      index={index}
                    />
                  ))}
                </div>
              ) : (
                <ReleaseUnavailable
                  releasesUrl={releasesUrl}
                  reason="This release does not contain desktop installers."
                />
              )}

              {cli.length > 0 ? (
                <div className="rounded-2xl border border-border bg-surface/40 p-5">
                  <p className="text-xs font-medium text-foreground">
                    Command-line archives
                  </p>
                  <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                    The CLI is a standalone executable. Extract it and add the
                    directory to <span className="font-mono">PATH</span>.
                  </p>
                  <ul className="mt-3 space-y-2">
                    {cli.map((asset) => (
                      <li
                        key={asset.name}
                        className="flex flex-wrap items-center gap-3 text-xs"
                      >
                        <a
                          href={asset.download_url}
                          rel="noreferrer"
                          className="font-mono text-primary underline-offset-4 hover:underline"
                        >
                          {asset.name}
                        </a>
                        {asset.size > 0 ? (
                          <span className="font-mono text-[10px] text-muted-foreground">
                            {formatBytes(asset.size)}
                          </span>
                        ) : null}
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
            </div>
          ) : null}
        </div>
      </section>

      <Separator className="my-12" />

      <section id="platforms" className="scroll-mt-24">
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Platforms
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          What each platform ships
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          The desktop app is a Tauri 2 bundle produced per platform. The CLI is
          a single executable from the same Cargo workspace, so the version
          reported by <span className="font-mono">kernext version</span> always
          matches the release tag.
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
                    <CardDescription className="mt-1">{platform.targets}</CardDescription>
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

        <div className="mt-6 grid gap-4 sm:grid-cols-2">
          <Card className="p-5">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Package className="size-4 text-primary" />
              Verified build targets
            </div>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              Windows x64, Linux x64, and macOS on both Apple Silicon and Intel
              are built by CI. Runtime requirements follow the Tauri 2 platform
              baseline — see <span className="font-mono">docs/DESKTOP.md</span>.
            </p>
          </Card>
          <Card className="p-5">
            <div className="flex items-center gap-2 text-sm font-medium">
              <ShieldCheck className="size-4 text-success" />
              Local execution only
            </div>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              The desktop app runs commands through its embedded Rust runtime.
              It never requires the Axum web server and never executes through
              a public network service.
            </p>
          </Card>
        </div>
      </section>

      <Separator className="my-12" />

      <section id="install" className="scroll-mt-24">
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Install
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          Installing KERNEXT
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          These builds are unsigned unless the release was produced with the
          optional signing secrets configured. An unsigned build still runs,
          but Windows SmartScreen and macOS Gatekeeper may warn on first launch.
        </p>

        <div className="mt-6 grid gap-4 sm:grid-cols-2">
          <Card className="p-5">
            <p className="text-sm font-medium">Windows</p>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              Run the <span className="font-mono">.msi</span> or the{' '}
              <span className="font-mono">-setup.exe</span>. If SmartScreen
              appears, choose <em>More info</em> then <em>Run anyway</em>.
            </p>
          </Card>
          <Card className="p-5">
            <p className="text-sm font-medium">macOS</p>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              Open the <span className="font-mono">.dmg</span> and drag KERNEXT
              into Applications. If Gatekeeper blocks it, right-click the app
              and choose <em>Open</em>.
            </p>
          </Card>
          <Card className="p-5">
            <p className="text-sm font-medium">Linux — AppImage</p>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              Make it executable, then run it. No installation step.
            </p>
          </Card>
          <Card className="p-5">
            <p className="text-sm font-medium">Linux — Debian package</p>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              Install with <span className="font-mono">apt</span> so the
              package manager records the dependency.
            </p>
          </Card>
        </div>

        <CommandBlock label="Linux" hint="AppImage + .deb">
          {`chmod +x kernext-desktop-linux-x64.AppImage
./kernext-desktop-linux-x64.AppImage

# or the Debian package
sudo apt install ./kernext-desktop-linux-x64.deb`}
        </CommandBlock>
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
          Every installer published by the release workflow is accompanied by a
          SHA-256 checksum file with the same name plus a{' '}
          <span className="font-mono">.sha256</span> suffix. Verify before you
          run.
        </p>

        <CommandBlock label="Checksum verification" hint="sha256">
          {`# Linux / macOS
sha256sum -c kernext-desktop-linux-x64.AppImage.sha256

# Windows PowerShell
Get-FileHash .\\kernext-desktop-windows-x64-setup.exe -Algorithm SHA256`}
        </CommandBlock>
      </section>

      <Separator className="my-12" />

      <section id="source" className="scroll-mt-24">
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          From source
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          Build it yourself
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          The CLI is the reference surface. A release build produces the{' '}
          <span className="font-mono">kernext</span> executable used by the
          prompt, pipelines, scripting, and background jobs. The repository
          supports both npm and pnpm for the front end.
        </p>

        <CommandBlock label="CLI" hint="Rust 1.85+">
          {`git clone ${repoUrl}.git
cd terminal
cargo build --workspace --release

# the executable
./target/release/kernext        # Linux / macOS
.\\target\\release\\kernext.exe   # Windows`}
        </CommandBlock>

        <CommandBlock label="Desktop" hint="npm or pnpm">
          {`npm ci            # or: pnpm install --frozen-lockfile
npm run desktop:build

# development window with hot reload
npm run desktop:dev`}
        </CommandBlock>

        <p className="mt-4 text-xs leading-relaxed text-muted-foreground">
          The repository also carries platform installers under{' '}
          <span className="font-mono">installers/</span> that install a built
          executable into a standard location.
        </p>
      </section>

      <Separator className="my-12" />

      <section id="development" className="scroll-mt-24">
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Not a stable release
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          Development builds and unsupported targets
        </h2>

        <div className="mt-6 grid gap-4 sm:grid-cols-2">
          <Card className="p-5">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Package className="size-4 text-primary" />
              Development builds
            </div>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              CI builds the desktop app for every change, but those artifacts
              are not releases: they are unsigned, may be broken, and are
              supported only by the change that produced them. Download them
              from the workflow runs, never from this page.
            </p>
            <div className="mt-4">
              <Button asChild variant="outline" size="sm">
                <a href={workflowUrl} target="_blank" rel="noreferrer">
                  Workflow runs
                  <ArrowUpRight className="size-3.5" />
                </a>
              </Button>
            </div>
          </Card>
          <Card className="p-5">
            <div className="flex items-center gap-2 text-sm font-medium">
              <TerminalSquare className="size-4 text-muted-foreground" />
              Unsupported targets
            </div>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              Windows arm64, Linux arm64, and other architectures are not built
              or published. They are not advertised here. Build from source if
              you need one.
            </p>
            <div className="mt-4">
              <Button asChild variant="ghost" size="sm">
                <Link to="/docs">
                  Build from source
                  <ArrowRight className="size-3.5" />
                </Link>
              </Button>
            </div>
          </Card>
        </div>
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
            <a href={repoUrl} target="_blank" rel="noreferrer">
              Source repository
              <ExternalLink className="size-4" />
            </a>
          </Button>
        </div>
      </div>
    </PageShell>
  );
}
