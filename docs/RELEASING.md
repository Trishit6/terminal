# Releasing KERNEXT

This document is the maintainer runbook for publishing KERNEXT. It covers the
CLI archives, the desktop installers, the release workflow, checksum
verification, and the optional signing configuration.

The authoritative artifact producer is
[`.github/workflows/release.yaml`](../.github/workflows/release.yaml). Nothing
in this document requires a secret: unsigned builds are the default and they
work.

## What a release contains

`release.yaml` publishes two families of files, each with a matching `.sha256`
checksum (sha256sum format, so `sha256sum -c` verifies them).

### Desktop application

| Platform | Architecture | Installer | Canonical file name |
| --- | --- | --- | --- |
| Windows 10/11 | x64 | WiX MSI | `kernext-desktop-windows-x64.msi` |
| Windows 10/11 | x64 | NSIS setup | `kernext-desktop-windows-x64-setup.exe` |
| Linux | x64 | AppImage | `kernext-desktop-linux-x64.AppImage` |
| Linux | x64 | Debian package | `kernext-desktop-linux-x64.deb` |
| Linux | x64 | RPM package | `kernext-desktop-linux-x64.rpm` |
| Linux | arm64 | AppImage | `kernext-desktop-linux-arm64.AppImage` |
| Linux | arm64 | Debian package | `kernext-desktop-linux-arm64.deb` |
| Linux | arm64 | RPM package | `kernext-desktop-linux-arm64.rpm` |
| macOS | Apple Silicon | Disk image | `kernext-desktop-macos-arm64.dmg` |
| macOS | Intel | Disk image | `kernext-desktop-macos-x64.dmg` |

Names are produced by [`scripts/collect-bundles.mjs`](../scripts/collect-bundles.mjs),
which locates the Tauri bundle output in the Cargo target directory and renames
each installer to the canonical name above. It exits non-zero when a platform
produced no installer, so a silent empty artifact cannot reach the release. When
a bundle was signed, the matching minisign `.sig` file is published beside it
under the same base name.

### Command-line shell

| Platform | Architecture | Archive |
| --- | --- | --- |
| Windows | x64 (GNU) | `kernext-x86_64-pc-windows-gnu.zip` |
| Linux | x64 (glibc) | `kernext-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | x64 (musl) | `kernext-x86_64-unknown-linux-musl.tar.gz` |
| Linux | arm64 | `kernext-aarch64-unknown-linux-gnu.tar.gz` |
| macOS | Apple Silicon | `kernext-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `kernext-x86_64-apple-darwin.tar.gz` |

Every archive contains both `kernext` (the shell) and `kernext-server` (the
local web API server). The public downloads page renders CLI archives only when
the published release actually contains them, so a desktop-only release is a
valid release — but `release.yaml` always builds both families.

## Package manager: pnpm only

`pnpm-lock.yaml` is the single authoritative lockfile. `package-lock.json` was
removed; npm, yarn, and bun are not supported.

```sh
pnpm install --frozen-lockfile
```

When you add, remove, or upgrade a dependency, run `pnpm install` and commit the
updated `pnpm-lock.yaml`. pnpm settings live in `pnpm-workspace.yaml` (pnpm 12
no longer reads a `pnpm` field from `package.json`).

The Tauri hooks run `node ../../scripts/build-web.mjs` (`beforeBuildCommand`)
and `node ../../scripts/build-web.mjs --dev` (`beforeDevCommand`). That helper
is pnpm-only: it runs the web package's own `build`/`dev` script and fails with
a clear message when pnpm is missing.

## Prerequisites

- Write access to the repository and permission to create releases.
- A checkout with `git` configured for pushes.
- Locally (optional, for a dry run): Rust 1.85+, Node.js 22+, and pnpm 10+.

macOS and Linux bundles cannot be produced on Windows and vice versa. The
workflow uses GitHub's hosted runners for each platform; you do not need a
macOS, Linux, or arm64 machine to release.

## Version bumping

A release tag must agree with every version declaration in the tree, or the
workflow fails before it builds anything. Update all five:

1. `Cargo.toml` — `[workspace.package] version`.
2. `apps/desktop/src-tauri/tauri.conf.json` — `version` (this is the version
   embedded in the installer file names and the app's About screen).
3. `package.json` — `version`.
4. `apps/web/package.json` — `version`.
5. `apps/desktop/package.json` — `version`.

Then verify locally before tagging:

```sh
node scripts/check-release-version.mjs v0.2.0
```

The script exits `1` and names every mismatching file when the tag and the
manifests disagree, and `2` when it is called without a tag. It runs in the
workflow's `verify` job, so a mistyped tag cannot publish installers whose
embedded version disagrees with the release.

Also update `RELEASE_NOTES.md` and `docs/PROJECT_STATUS.md` for the version.

## Release checklist

From a clean checkout of the default branch, with the version bumped and
committed:

```sh
# 1. Verify locally (the same gates the workflow runs).
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm install --frozen-lockfile
pnpm run typecheck
pnpm test
pnpm run build

# 2. Confirm the tag matches every manifest.
node scripts/check-release-version.mjs v0.2.0

# 3. Tag and push. This starts the release workflow.
git tag v0.2.0
git push origin v0.2.0
```

Watch the run at
`https://github.com/Trishit6/terminal/actions/workflows/release.yaml`.

A manual run against an existing tag is available from the same page
(**Run workflow** → enter the tag), which is useful when a platform runner
failed for an infrastructure reason rather than a code reason.

## What the workflow does

1. **`verify`** (`ubuntu-22.04`): installs the Linux desktop build
   dependencies, Rust (with `clippy` and `rustfmt`), pnpm, and Node; installs
   front-end dependencies with `--frozen-lockfile`; checks the tag against the
   manifests; runs `cargo fmt --check`, `cargo clippy -D warnings`,
   `cargo test --workspace`, and the web typecheck, tests, and production build.
   Any failure stops the release here.
2. **`cli`** (matrix, `fail-fast: false`): Windows x64 (GNU), Linux x64
   (glibc), Linux x64 (musl), Linux arm64, macOS Apple Silicon, and macOS Intel.
   Each leg builds `-p shell-cli -p shell-server` for its Rust target triple
   with `--locked` and uploads the two binaries.
3. **`desktop`** (matrix, `fail-fast: false`): Windows x64 (`msi,nsis`),
   Linux x64 (`appimage,deb,rpm`), Linux arm64 (`appimage,deb,rpm`),
   macOS Apple Silicon (`dmg`, `aarch64-apple-darwin`), and macOS Intel (`dmg`,
   `x86_64-apple-darwin`). Each leg builds the front end through
   `scripts/build-web.mjs`, runs `tauri build` with the platform's bundles,
   collects canonical installers plus checksums, and uploads them as an
   artifact. `fail-fast: false` means one broken platform does not hide the
   others' logs — but every leg must still succeed.
4. **`release`** (`ubuntu-latest`, `contents: write`): downloads all artifacts,
   assembles the CLI archives, copies the installers, generates a `.sha256` for
   every file, verifies each checksum, writes release notes, and creates the
   GitHub Release with `gh release create <tag> --latest`.

The Linux arm64 legs run on GitHub's `ubuntu-24.04-arm` runner. They are newer
than the x64 legs and are recorded as unverified in `docs/PROJECT_STATUS.md`
until a real run has completed.

No step uses `continue-on-error`. A failed platform build fails its matrix job,
which skips `release`, so a partially built release is never published.

## Verifying a published release

```sh
TAG=v0.2.0
gh release view "$TAG"
gh release download "$TAG" --pattern 'kernext-*'

# Verify each artifact against its checksum (Linux/macOS, or Git Bash).
for f in kernext-*; do
  case "$f" in *.sha256|*.sig) continue;; esac
  sha256sum -c "$f.sha256"
done
```

On Windows PowerShell, compare manually:

```powershell
Get-FileHash .\kernext-desktop-windows-x64-setup.exe -Algorithm SHA256
Get-Content .\kernext-desktop-windows-x64-setup.exe.sha256
```

Then confirm the release is marked **Latest** on
`https://github.com/Trishit6/terminal/releases`. The website reads it through
`GET /api/v1/releases/latest` (and the history through `GET /api/v1/releases`),
so the downloads page reflects the new release within its 10-minute cache window
(restart the server to refresh immediately).

## Signing (optional)

The workflow ships **unsigned** builds. That is a complete, working product:

- Windows shows a SmartScreen warning until the publisher builds reputation.
- macOS shows the “unidentified developer” Gatekeeper prompt; users open the app
  with **Control-click → Open** or **System Settings → Privacy & Security → Open
  Anyway**.
- Linux package managers may warn about an unsigned package.

To sign, add repository secrets and enable them in the workflow. **Do not pass
an empty secret through to the build**: Tauri treats a present-but-empty
`APPLE_CERTIFICATE` as a signing request and fails. Enable signing only after
the secrets exist.

Windows (code signing certificate in `tauri.conf.json` `bundle.windows`):
set `certificateThumbprint`, `digestAlgorithm`, and `timestampUrl`, and make the
certificate available to the Windows runner.

macOS (Developer ID + notarization): the macOS legs need these secrets wired
into the `Build desktop bundles` step as environment variables —
`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`,
`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`. Guard the wiring so it is only
applied when the secrets are non-empty. Tauri then emits a minisign `.sig`
beside each bundle, which `collect-bundles.mjs` publishes and the downloads page
links next to the installer.

No signing secret is required for a normal release, and no secret is stored in
the repository.

## Troubleshooting

- **`release tag vX expects version Y`** — bump the five manifests listed above,
  or retag with the correct version.
- **`No release assets were assembled`** — every build leg failed, or an
  expected artifact was missing. Open the failed `cli`/`desktop` job; the
  `Collect installers and checksums` step names the platform that produced
  nothing.
- **`sha256sum -c` fails in `release`** — an artifact was corrupted in transit
  between jobs. Re-run the workflow; the checksum step exists precisely to
  catch this before publishing.
- **`pnpm install --frozen-lockfile` fails** — the lockfile is out of date.
  Run `pnpm install` locally and commit `pnpm-lock.yaml`.
- **`pnpm is required to build the web front end`** — the Tauri build hook could
  not find pnpm. Install it with `npm install -g pnpm` or `corepack enable`.
- **The downloads page still shows the old release** — the endpoint caches for
  10 minutes. Wait, or restart the server.

## Files that define a release

- [`.github/workflows/release.yaml`](../.github/workflows/release.yaml) — the release path (CLI + desktop + publish).
- [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) — the pull-request pipeline; never publishes.
- [`scripts/check-release-version.mjs`](../scripts/check-release-version.mjs) — tag/manifest version guard.
- [`scripts/collect-bundles.mjs`](../scripts/collect-bundles.mjs) — canonical installer names, signatures, and SHA-256 files.
- [`scripts/build-web.mjs`](../scripts/build-web.mjs) — pnpm front-end build hook for Tauri.
- [`apps/desktop/src-tauri/tauri.conf.json`](../apps/desktop/src-tauri/tauri.conf.json) — bundle targets and the build hooks.
- [`apps/shell-server/src/releases.rs`](../apps/shell-server/src/releases.rs) — the `/api/v1/releases` feeds behind the downloads page.
- [`apps/web/src/routes/downloads.tsx`](../apps/web/src/routes/downloads.tsx) — the downloads page that renders this data.
