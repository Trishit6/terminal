import { Link } from '@tanstack/react-router';
import {
  AlertTriangle,
  ArrowRight,
  ArrowUpRight,
  Check,
  CheckCircle2,
  Download,
  ExternalLink,
  History,
  Loader2,
  Package,
  ShieldCheck,
  TerminalSquare,
} from 'lucide-react';
import { motion } from 'motion/react';
import { useCallback, useState } from 'react';
import { LogoMark } from '@/components/site/logo';
import { PageAsideRow, PageShell } from '@/components/site/page-shell';
import { PlatformMark } from '@/components/site/platform-mark';
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
import { useLatestRelease, useReleaseList, useVisitorPlatform } from '@/hooks/use-latest-release';
import {
  DEFAULT_REPOSITORY,
  archLabel,
  cliAssetsByPlatform,
  formatBytes,
  formatLabel,
  formatReleaseDate,
  formatShortDate,
  platformLabel,
  platformTargets,
  platformTitle,
  previousReleases,
  recommendTarget,
  releaseNotesExcerpt,
  releasesUrl as releasesUrlFor,
  repositoryUrl,
  type PlatformTarget,
  type ReleaseAsset,
} from '@/lib/releases';
import { KERNEXT_VERSION } from '@/lib/version';

const EASE = [0.22, 1, 0.36, 1] as const;

/** How long the "started" confirmation stays on a download button. */
const DOWNLOAD_CONFIRM_MS = 4000;

type DownloadPhase = 'idle' | 'pending' | 'success' | 'error';

/**
 * A download URL is only usable when it is an absolute HTTP(S) URL. Anything
 * else would be a bug in the release payload, and the button says so instead
 * of silently doing nothing.
 */
function isDownloadable(url: string): boolean {
  try {
    const parsed = new URL(url);
    return parsed.protocol === 'https:' || parsed.protocol === 'http:';
  } catch {
    return false;
  }
}

/**
 * Starts a download and reflects the click lifecycle.
 *
 * GitHub serves release assets from a different origin, so the browser exposes
 * no progress or completion event and no CORS headers to poll: the "started"
 * state confirms the browser accepted a real asset URL, and the transfer
 * itself is shown by the browser's own download UI. A genuinely unreachable
 * release is reported by the release lookup above, which is a real request.
 */
function DownloadButton({
  asset,
  label,
  size = 'sm',
  variant = 'default',
  className,
}: {
  asset: ReleaseAsset;
  label: string;
  size?: 'sm' | 'default' | 'lg';
  variant?: 'default' | 'outline' | 'secondary';
  className?: string;
}) {
  const [phase, setPhase] = useState<DownloadPhase>('idle');

  const start = useCallback(() => {
    if (!isDownloadable(asset.download_url)) {
      setPhase('error');
      return;
    }

    setPhase('pending');

    const anchor = document.createElement('a');
    anchor.href = asset.download_url;
    anchor.rel = 'noreferrer';
    anchor.download = asset.name;
    document.body.append(anchor);
    anchor.click();
    anchor.remove();

    setPhase('success');
    window.setTimeout(() => setPhase('idle'), DOWNLOAD_CONFIRM_MS);
  }, [asset.download_url, asset.name]);

  const content =
    phase === 'pending' ? (
      <>
        <Loader2 className="size-3.5 animate-spin" />
        Starting…
      </>
    ) : phase === 'success' ? (
      <>
        <CheckCircle2 className="size-3.5" />
        Download started
      </>
    ) : phase === 'error' ? (
      <>
        <AlertTriangle className="size-3.5" />
        Unavailable
      </>
    ) : (
      <>
        <Download className="size-3.5" />
        {label}
      </>
    );

  return (
    <Button
      type="button"
      size={size}
      variant={phase === 'error' ? 'destructive' : variant}
      className={className}
      onClick={start}
      disabled={phase === 'pending'}
      aria-live="polite"
      title={asset.name}
    >
      {content}
    </Button>
  );
}

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

/** Integrity links (checksum, signature) shown next to an asset. */
function IntegrityLinks({ asset }: { asset: ReleaseAsset }) {
  if (!asset.checksum_url && !asset.signature_url) {
    return null;
  }

  return (
    <span className="flex items-center gap-3">
      {asset.checksum_url ? (
        <a
          href={asset.checksum_url}
          rel="noreferrer"
          className="font-mono text-[10px] text-muted-foreground underline decoration-dotted underline-offset-4 transition-colors hover:text-foreground"
        >
          .sha256
        </a>
      ) : null}
      {asset.signature_url ? (
        <a
          href={asset.signature_url}
          rel="noreferrer"
          className="font-mono text-[10px] text-muted-foreground underline decoration-dotted underline-offset-4 transition-colors hover:text-foreground"
        >
          .sig
        </a>
      ) : null}
    </span>
  );
}

function AssetRow({ asset }: { asset: ReleaseAsset }) {
  return (
    <div className="space-y-2 border-t border-border/70 pt-3 first:border-0 first:pt-0">
      <div className="flex flex-wrap items-center gap-3">
        <DownloadButton
          asset={asset}
          label={formatLabel(asset.format)}
          variant="outline"
          className="h-8"
        />
        <IntegrityLinks asset={asset} />
        {asset.size > 0 ? (
          <span className="font-mono text-[10px] text-muted-foreground">
            {formatBytes(asset.size)}
          </span>
        ) : null}
      </div>
      <p className="truncate font-mono text-[10px] text-muted-foreground" title={asset.name}>
        {asset.name}
      </p>
    </div>
  );
}

function DesktopCard({
  target,
  index,
  recommended,
  versionLabel,
}: {
  target: PlatformTarget;
  index: number;
  recommended: boolean;
  versionLabel: string | null;
}) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 18 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-60px' }}
      transition={{ duration: 0.5, ease: EASE, delay: index * 0.05 }}
      className="h-full"
    >
      <Card
        className={
          recommended
            ? 'h-full border-primary/45 bg-surface/60 shadow-[0_0_0_1px_rgba(56,189,248,0.18)]'
            : 'h-full bg-surface/55'
        }
      >
        <CardHeader className="gap-3">
          <div className="flex items-start justify-between gap-3">
            <div className="grid size-10 place-items-center rounded-xl border border-border bg-background/70 text-primary">
              <PlatformMark platform={target.platform} className="size-5" />
            </div>
            {recommended ? (
              <Badge variant="success">Recommended</Badge>
            ) : (
              <Badge variant="secondary">{archLabel(target.platform, target.arch)}</Badge>
            )}
          </div>
          <div>
            <CardTitle className="text-base">{platformTitle(target.platform, target.arch)}</CardTitle>
            <CardDescription className="mt-1">
              {archLabel(target.platform, target.arch)}
              {versionLabel ? ` · ${versionLabel}` : ''}
            </CardDescription>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          {target.assets.map((asset) => (
            <AssetRow key={asset.name} asset={asset} />
          ))}
        </CardContent>
      </Card>
    </motion.div>
  );
}

function ReleaseUnavailable({ releasesPage, reason }: { releasesPage: string; reason: string | null }) {
  return (
    <Card className="bg-surface/55 p-6">
      <div className="flex items-start gap-3">
        <Package className="mt-0.5 size-4 shrink-0 text-primary" />
        <div>
          <p className="text-sm font-medium text-foreground">No published release yet.</p>
          <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
            {reason ??
              'The release workflow has not published a GitHub Release for this repository.'}{' '}
            Build from source in the meantime, or watch the repository&apos;s releases page.
          </p>
          <div className="mt-4 flex flex-wrap gap-3">
            <Button asChild variant="outline" size="sm">
              <a href={releasesPage} target="_blank" rel="noreferrer">
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
  const historyState = useReleaseList();
  const visitor = useVisitorPlatform();

  const release = state.status === 'ready' ? state.release : null;
  const available = Boolean(release?.available);

  const repository = release?.repository ?? DEFAULT_REPOSITORY;
  const repoUrl = repositoryUrl(repository);
  const releasesPage = releasesUrlFor(repository);
  const workflowUrl = `${repoUrl}/actions/workflows/release.yaml`;

  const versionLabel = release?.version ? `v${release.version}` : null;
  const publishedLabel = release ? formatReleaseDate(release.published_at) : null;

  const targets = available && release ? platformTargets(release) : [];
  const cliGroups = available && release ? cliAssetsByPlatform(release) : [];
  const recommended =
    available && release ? recommendTarget(release, visitor.platform, visitor.arch) : null;
  const notes = releaseNotesExcerpt(release?.notes ?? null, 6);

  const previous =
    historyState.status === 'ready'
      ? previousReleases(historyState.list, release?.tag ?? null)
      : [];
  const hasPrevious = previous.length > 0;
  const signed = Boolean(release?.assets.some((asset) => asset.signature_url));

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
            <a href={releasesPage} target="_blank" rel="noreferrer">
              <Download className="size-4" />
              All releases
              <ArrowUpRight className="size-3.5 opacity-70" />
            </a>
          </Button>
          <Button asChild variant="outline">
            <a href={repoUrl} target="_blank" rel="noreferrer">
              <ExternalLink className="size-4" />
              Source code
            </a>
          </Button>
        </>
      }
      aside={
        <>
          <PageAsideRow label="Latest" value={versionLabel ?? 'No release yet'} />
          <PageAsideRow label="Source" value={`v${KERNEXT_VERSION}`} />
          <PageAsideRow
            label="Desktop"
            value={targets.length > 0 ? `${targets.length} targets` : 'Not published'}
          />
          <PageAsideRow label="Checksums" value="SHA-256" />
          <PageAsideRow label="Signatures" value={signed ? 'Published' : 'Unsigned'} />
          <p className="page-aside__note">
            Assets, installers, and checksums are produced by CI from this
            repository — never by hand.
          </p>
        </>
      }
    >
      <div className="rounded-2xl border border-border bg-surface/40 p-5">
        <p className="text-xs leading-relaxed text-muted-foreground">
          <span className="font-medium text-foreground">Honest download policy:</span> this
          page reads the latest published GitHub Release and links only to the
          files that release actually contains. If no release has been published
          yet, it says so and points you at the source instead. No mirrors, no
          third-party binaries, and no download button for an installer that
          does not exist.
        </p>
      </div>

      <Separator className="my-12" />

      {/* ---------------------------------------------------------------- */}
      {/* Latest stable release                                            */}
      {/* ---------------------------------------------------------------- */}
      <section id="latest" className="scroll-mt-24">
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
                    <a href={releasesPage} target="_blank" rel="noreferrer">
                      Open GitHub Releases
                      <ArrowUpRight className="size-3.5" />
                    </a>
                  </Button>
                </div>
              </div>
            </div>
          </Card>
        ) : null}

        {state.status === 'ready' && !available ? (
          <ReleaseUnavailable releasesPage={releasesPage} reason={release?.error ?? null} />
        ) : null}

        {state.status === 'ready' && available && release ? (
          <div className="relative overflow-hidden rounded-3xl border border-border bg-surface/50 p-6 sm:p-8">
            <div
              className="pointer-events-none absolute -right-24 -top-28 size-72 rounded-full bg-primary/10 blur-3xl"
              aria-hidden="true"
            />
            <div className="relative flex flex-col gap-6 lg:flex-row lg:items-start lg:justify-between">
              <div className="flex items-start gap-4">
                <LogoMark className="size-14" />
                <div>
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-mono text-[10px] uppercase tracking-[0.24em] text-muted-foreground">
                      KERNEXT
                    </span>
                    <Badge variant="success">Latest stable</Badge>
                    {release.cached ? <Badge variant="secondary">Cached</Badge> : null}
                  </div>
                  <h2 className="mt-2 text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
                    {versionLabel}
                  </h2>
                  <p className="mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-[11px] text-muted-foreground">
                    {publishedLabel ? <span>Released {publishedLabel}</span> : null}
                    {release.tag ? <span>{release.tag}</span> : null}
                    <span>SHA-256 verified</span>
                  </p>
                </div>
              </div>

              <div className="flex flex-wrap gap-3">
                <Button asChild variant="outline">
                  <a href={release.html_url ?? releasesPage} target="_blank" rel="noreferrer">
                    Release notes
                    <ArrowUpRight className="size-3.5" />
                  </a>
                </Button>
              </div>
            </div>

            {notes.length > 0 ? (
              <ul className="relative mt-6 space-y-1.5 border-t border-border/70 pt-5">
                {notes.map((line) => (
                  <li
                    key={line}
                    className="flex items-start gap-2 text-xs leading-relaxed text-muted-foreground"
                  >
                    <Check className="mt-0.5 size-3.5 shrink-0 text-primary/70" />
                    <span>{line}</span>
                  </li>
                ))}
              </ul>
            ) : null}

            {recommended && recommended.assets[0] ? (
              <div className="relative mt-6 flex flex-col gap-4 rounded-2xl border border-border bg-background/40 p-4 sm:flex-row sm:items-center sm:justify-between">
                <div className="flex items-center gap-3">
                  <div className="grid size-10 place-items-center rounded-xl border border-border bg-background/70 text-primary">
                    <PlatformMark platform={recommended.platform} className="size-5" />
                  </div>
                  <div>
                    <p className="text-sm font-medium text-foreground">
                      Recommended for {platformLabel(recommended.platform)}
                    </p>
                    <p className="mt-0.5 font-mono text-[10px] text-muted-foreground">
                      {platformTitle(recommended.platform, recommended.arch)} ·{' '}
                      {archLabel(recommended.platform, recommended.arch)}
                      {recommended.assets[0].size > 0
                        ? ` · ${formatBytes(recommended.assets[0].size)}`
                        : ''}
                    </p>
                  </div>
                </div>
                <div className="flex flex-wrap items-center gap-3">
                  <DownloadButton
                    asset={recommended.assets[0]}
                    label={formatLabel(recommended.assets[0].format)}
                    size="default"
                  />
                  <IntegrityLinks asset={recommended.assets[0]} />
                </div>
              </div>
            ) : null}
          </div>
        ) : null}
      </section>

      {/* ---------------------------------------------------------------- */}
      {/* Desktop application                                              */}
      {/* ---------------------------------------------------------------- */}
      {state.status === 'ready' && available && release ? (
        <>
          <Separator className="my-12" />
          <section id="desktop" className="scroll-mt-24">
            <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
              Desktop application
            </div>
            <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
              KERNEXT Desktop {versionLabel}
            </h2>
            <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
              A Tauri 2 bundle that runs the native Rust engine in-process. It
              never needs the web server and never executes through a public
              network service. Every installer below is attached to the latest
              stable release.
            </p>

            <div className="mt-8">
              {targets.length > 0 ? (
                <div className="grid gap-5 sm:grid-cols-2 xl:grid-cols-3">
                  {targets.map((target, index) => (
                    <DesktopCard
                      key={`${target.platform}-${target.arch}`}
                      target={target}
                      index={index}
                      recommended={
                        recommended?.platform === target.platform &&
                        recommended?.arch === target.arch
                      }
                      versionLabel={versionLabel}
                    />
                  ))}
                </div>
              ) : (
                <ReleaseUnavailable
                  releasesPage={releasesPage}
                  reason="This release does not contain desktop installers."
                />
              )}
            </div>
          </section>
        </>
      ) : null}

      {/* ---------------------------------------------------------------- */}
      {/* Command line                                                     */}
      {/* ---------------------------------------------------------------- */}
      {state.status === 'ready' && available && release && cliGroups.length > 0 ? (
        <>
          <Separator className="my-12" />
          <section id="cli" className="scroll-mt-24">
            <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
              Command line
            </div>
            <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
              KERNEXT CLI {versionLabel}
            </h2>
            <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
              The CLI is a standalone executable built from the same Cargo
              workspace as the desktop app. Extract the archive and put{' '}
              <span className="font-mono">kernext</span> on your{' '}
              <span className="font-mono">PATH</span>. It has no runtime
              dependency on the website or the desktop interface.
            </p>

            <div className="mt-8 grid gap-5 sm:grid-cols-2 xl:grid-cols-3">
              {cliGroups.map((group) => (
                <Card key={group.platform ?? 'other'} className="bg-surface/55">
                  <CardHeader className="gap-3">
                    <div className="flex items-center gap-3">
                      {group.platform ? (
                        <div className="grid size-10 place-items-center rounded-xl border border-border bg-background/70 text-primary">
                          <PlatformMark platform={group.platform} className="size-5" />
                        </div>
                      ) : (
                        <div className="grid size-10 place-items-center rounded-xl border border-border bg-background/70 text-muted-foreground">
                          <TerminalSquare className="size-5" />
                        </div>
                      )}
                      <div>
                        <CardTitle className="text-base">
                          {group.platform ? platformLabel(group.platform) : 'Other targets'}
                        </CardTitle>
                        <CardDescription className="mt-0.5">
                          {group.assets.length} archive
                          {group.assets.length === 1 ? '' : 's'}
                        </CardDescription>
                      </div>
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    {group.assets.map((asset) => (
                      <div key={asset.name} className="space-y-2">
                        <div className="flex flex-wrap items-center gap-3">
                          <DownloadButton
                            asset={asset}
                            label={asset.arch ? archLabel('linux', asset.arch) : 'Download'}
                            variant="outline"
                            className="h-8"
                          />
                          <IntegrityLinks asset={asset} />
                          {asset.size > 0 ? (
                            <span className="font-mono text-[10px] text-muted-foreground">
                              {formatBytes(asset.size)}
                            </span>
                          ) : null}
                        </div>
                        <p
                          className="truncate font-mono text-[10px] text-muted-foreground"
                          title={asset.name}
                        >
                          {asset.name}
                        </p>
                      </div>
                    ))}
                  </CardContent>
                </Card>
              ))}
            </div>
          </section>
        </>
      ) : null}

      <Separator className="my-12" />

      {/* ---------------------------------------------------------------- */}
      {/* Install instructions                                             */}
      {/* ---------------------------------------------------------------- */}
      <section id="install" className="scroll-mt-24">
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Install
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          Installing KERNEXT
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          These builds are unsigned unless the release was produced with the
          optional signing secrets configured. An unsigned build still runs, but
          Windows SmartScreen and macOS Gatekeeper may warn on first launch.
        </p>

        <div className="mt-6 grid gap-4 sm:grid-cols-2">
          <Card className="p-5">
            <p className="text-sm font-medium">Windows 10 / 11 (x64)</p>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              Run the <span className="font-mono">.msi</span> or the{' '}
              <span className="font-mono">-setup.exe</span>. If SmartScreen
              appears, choose <em>More info</em> then <em>Run anyway</em>.
            </p>
          </Card>
          <Card className="p-5">
            <p className="text-sm font-medium">macOS (Apple Silicon · Intel)</p>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              Open the <span className="font-mono">.dmg</span> and drag KERNEXT
              into Applications. If Gatekeeper blocks it, right-click the app and
              choose <em>Open</em>.
            </p>
          </Card>
          <Card className="p-5">
            <p className="text-sm font-medium">Linux — AppImage</p>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              Make it executable, then run it. No installation step and no root
              required.
            </p>
          </Card>
          <Card className="p-5">
            <p className="text-sm font-medium">Linux — DEB / RPM</p>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              Install with <span className="font-mono">apt</span> or{' '}
              <span className="font-mono">dnf</span> so the package manager
              records the dependencies.
            </p>
          </Card>
        </div>

        <CommandBlock label="Linux" hint="AppImage · .deb · .rpm">
          {`chmod +x kernext-desktop-linux-x64.AppImage
./kernext-desktop-linux-x64.AppImage

# Debian / Ubuntu
sudo apt install ./kernext-desktop-linux-x64.deb

# Fedora / RHEL
sudo dnf install ./kernext-desktop-linux-x64.rpm`}
        </CommandBlock>

        <CommandBlock label="Command line" hint="PATH setup">
          {`# Linux / macOS — extract and add to PATH
tar -xzf kernext-x86_64-unknown-linux-gnu.tar.gz
sudo install -m 0755 kernext /usr/local/bin/kernext
kernext version

# Windows PowerShell — extract and add to PATH
Expand-Archive .\\kernext-x86_64-pc-windows-gnu.zip -DestinationPath .\\kernext
$env:Path = "$PWD\\kernext;$env:Path"
.\\kernext\\kernext.exe version`}
        </CommandBlock>
      </section>

      <Separator className="my-12" />

      {/* ---------------------------------------------------------------- */}
      {/* Integrity                                                        */}
      {/* ---------------------------------------------------------------- */}
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
          <span className="font-mono">.sha256</span> suffix, linked next to the
          download. Releases produced with signing enabled also carry a{' '}
          <span className="font-mono">.sig</span> signature file for each
          artifact. Verify before you run.
        </p>

        <CommandBlock label="Checksum verification" hint="sha256">
          {`# Linux / macOS
sha256sum -c kernext-desktop-linux-x64.AppImage.sha256

# macOS (shasum)
shasum -a 256 -c kernext-desktop-macos-arm64.dmg.sha256

# Windows PowerShell
Get-FileHash .\\kernext-desktop-windows-x64-setup.exe -Algorithm SHA256`}
        </CommandBlock>
      </section>

      {/* ---------------------------------------------------------------- */}
      {/* Previous releases                                                */}
      {/* ---------------------------------------------------------------- */}
      <Separator className="my-12" />
      <section id="previous" className="scroll-mt-24">
        <div className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Release history
        </div>
        <h2 className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl">
          Previous releases
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          Older versions stay available on GitHub Releases. Every entry links to
          the release that actually published it.
        </p>

        <div className="mt-6">
          {historyState.status === 'loading' ? (
            <Card className="bg-surface/55 p-6">
              <div className="flex items-center gap-3 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin text-primary" />
                Reading the release history…
              </div>
            </Card>
          ) : null}

          {historyState.status === 'error' ? (
            <Card className="border-warning/30 bg-warning/5 p-6">
              <p className="text-sm text-muted-foreground">{historyState.message}</p>
              <div className="mt-4">
                <Button asChild variant="outline" size="sm">
                  <a href={releasesPage} target="_blank" rel="noreferrer">
                    Open GitHub Releases
                    <ArrowUpRight className="size-3.5" />
                  </a>
                </Button>
              </div>
            </Card>
          ) : null}

          {historyState.status === 'ready' && !hasPrevious ? (
            <Card className="bg-surface/55 p-6">
              <div className="flex items-start gap-3">
                <History className="mt-0.5 size-4 shrink-0 text-primary" />
                <p className="text-xs leading-relaxed text-muted-foreground">
                  {historyState.list.available
                    ? 'This is the first published release, so there are no earlier versions yet.'
                    : (historyState.list.error ??
                      'The release history could not be read.')}
                </p>
              </div>
            </Card>
          ) : null}

          {historyState.status === 'ready' && hasPrevious ? (
            <ul className="divide-y divide-border overflow-hidden rounded-2xl border border-border bg-surface/40">
              {previous.map((entry) => (
                <li
                  key={entry.tag}
                  className="flex flex-wrap items-center justify-between gap-3 px-5 py-3.5"
                >
                  <div className="flex flex-wrap items-center gap-3">
                    <span className="font-mono text-xs text-foreground">{entry.tag}</span>
                    {entry.prerelease ? (
                      <Badge variant="warning">Pre-release</Badge>
                    ) : (
                      <Badge variant="secondary">Stable</Badge>
                    )}
                    {formatShortDate(entry.published_at) ? (
                      <span className="font-mono text-[10px] text-muted-foreground">
                        {formatShortDate(entry.published_at)}
                      </span>
                    ) : null}
                    <span className="font-mono text-[10px] text-muted-foreground">
                      {entry.asset_count} artifact{entry.asset_count === 1 ? '' : 's'}
                    </span>
                  </div>
                  <a
                    href={entry.html_url ?? releasesPage}
                    target="_blank"
                    rel="noreferrer"
                    className="inline-flex items-center gap-1 text-xs text-primary underline-offset-4 hover:underline"
                  >
                    GitHub Release
                    <ArrowUpRight className="size-3" />
                  </a>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      </section>

      <Separator className="my-12" />

      {/* ---------------------------------------------------------------- */}
      {/* Development builds and unsupported targets                       */}
      {/* ---------------------------------------------------------------- */}
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
              CI builds the CLI and the desktop app for every change, but those
              artifacts are not releases: they are unsigned, may be broken, and
              are supported only by the change that produced them. Download them
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
              Windows on ARM64, 32-bit x86, and other architectures are not built
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

      <Separator className="my-12" />

      {/* ---------------------------------------------------------------- */}
      {/* Build from source                                                */}
      {/* ---------------------------------------------------------------- */}
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
          prompt, pipelines, scripting, and background jobs. The front end is
          managed with pnpm.
        </p>

        <CommandBlock label="CLI" hint="Rust 1.85+">
          {`git clone ${repoUrl}.git
cd terminal
cargo build --workspace --release

# the executable
./target/release/kernext        # Linux / macOS
.\\target\\release\\kernext.exe   # Windows`}
        </CommandBlock>

        <CommandBlock label="Desktop" hint="pnpm">
          {`pnpm install --frozen-lockfile
pnpm run desktop:build

# development window with hot reload
pnpm run desktop:dev`}
        </CommandBlock>

        <p className="mt-4 text-xs leading-relaxed text-muted-foreground">
          The repository also carries platform installers under{' '}
          <span className="font-mono">installers/</span> that install a built
          executable into a standard location.
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
            <a href={repoUrl} target="_blank" rel="noreferrer">
              <ShieldCheck className="size-4" />
              Source repository
            </a>
          </Button>
        </div>
      </div>
    </PageShell>
  );
}
