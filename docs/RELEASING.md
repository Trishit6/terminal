# Releasing KERNEXT

This document is the maintainer runbook for publishing KERNEXT. It covers the
desktop installers, the release workflow, checksum verification, and the
optional signing configuration.

The authoritative artifact producer is
[`.github/workflows/desktop-release.yaml`](../.github/workflows/desktop-release.yaml).
Nothing in this document requires a secret: unsigned builds are the default and
they work.

## What a release contains

`desktop-release.yaml` publishes these files, each with a matching `.sha256`
checksum (sha256sum format, so `sha256sum -c` verifies them):

| Platform | Architecture | Installer | Canonical file name |
| --- | --- | --- | --- |
| Windows | x64 | WiX MSI | `kernext-desktop-windows-x64.msi` |
| Windows | x64 | NSIS setup | `kernext-desktop-windows-x64-setup.exe` |
| Linux | x64 | AppImage | `kernext-desktop-linux-x64.AppImage` |
| Linux | x64 | Debian package | `kernext-desktop-linux-x64.deb` |
| macOS | Apple Silicon | Disk image | `kernext-desktop-macos-arm64.dmg` |
| macOS | Intel | Disk image | `kernext-desktop-macos-x64.dmg` |

Names are produced by [`scripts/collect-bundles.mjs`](../scripts/collect-bundles.mjs),
which locates the Tauri bundle output in the Cargo target directory and renames
each installer to the canonical name above. It exits non-zero when a platform
produced no installer, so a silent empty artifact cannot reach the release.

The CLI archives (`kernext-<target>.tar.gz` / `.zip`) are produced by the
separate, manual, draft-only
[`release.yaml`](../.github/workflows/release.yaml) workflow. The public
downloads page renders CLI archives only when the published release actually
contains them, so a desktop-only release is a valid release.

## Package managers: npm and pnpm

Both are supported and both lockfiles are tracked:

- `package-lock.json` — used by `ci.yml` and by local `npm ci`.
- `pnpm-lock.yaml` — used by `desktop-release.yaml` with
  `pnpm install --frozen-lockfile`.

When you add, remove, or upgrade a dependency, update **both** lockfiles and
commit them together:

```sh
npm install
pnpm install
```

The Tauri build hook does not assume either package manager. `tauri.conf.json`
runs `node ../../scripts/build-web.mjs`, which picks the manager in this order:

1. `KERNEXT_PACKAGE_MANAGER` when set (`npm` or `pnpm`).
2. pnpm, when `pnpm-lock.yaml` exists and `pnpm` runs.
3. npm.

The release workflow sets `KERNEXT_PACKAGE_MANAGER: pnpm`, so CI uses the
lockfile it installed with.

## Prerequisites

- Write access to the repository and permission to create releases.
- A checkout with `git` configured for pushes.
- Locally (optional, for a dry run): Rust 1.85+, Node.js 22+, and either npm 10+
  or pnpm 12+.

macOS and Linux bundles cannot be produced on Windows and vice versa. The
workflow uses GitHub's hosted runners for each platform; you do not need a
macOS or Linux machine to release.

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
npm ci
npm run typecheck
npm test
npm run build

# 2. Confirm the tag matches every manifest.
node scripts/check-release-version.mjs v0.2.0

# 3. Tag and push. This starts the release workflow.
git tag v0.2.0
git push origin v0.2.0
```

Watch the run at
`https://github.com/Trishit6/terminal/actions/workflows/desktop-release.yaml`.

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
2. **`desktop`** (matrix, `fail-fast: false`): Windows x64 (`msi,nsis`),
   Linux x64 (`appimage,deb`), macOS Apple Silicon (`dmg`, `aarch64-apple-darwin`),
   and macOS Intel (`dmg`, `x86_64-apple-darwin`). Each leg builds the front end
   through `scripts/build-web.mjs`, runs `tauri build` with the platform's
   bundles, collects canonical installers plus checksums, and uploads them as an
   artifact. `fail-fast: false` means one broken platform does not hide the
   others' logs — but every leg must still succeed.
3. **`release`** (`ubuntu-latest`, `contents: write`): downloads all artifacts,
   refuses to continue when no installer is present, verifies every `.sha256`,
   writes release notes, and creates the GitHub Release with
   `gh release create <tag> --latest`.

No step uses `continue-on-error`. A failed platform build fails the `desktop`
job, which skips `release`, so a partially built release is never published.

## Verifying a published release

```sh
TAG=v0.2.0
gh release view "$TAG"
gh release download "$TAG" --pattern 'kernext-desktop-*'

# Verify each installer against its checksum (Linux/macOS, or Git Bash).
for f in kernext-desktop-*; do
  case "$f" in *.sha256) continue;; esac
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
`GET /api/v1/releases/latest`, so the downloads page reflects the new release
within its 10-minute cache window (restart the server to refresh immediately).

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
applied when the secrets are non-empty.

No signing secret is required for a normal release, and no secret is stored in
the repository.

## Troubleshooting

- **`release tag vX expects version Y`** — bump the five manifests listed above,
  or retag with the correct version.
- **`No desktop installers were downloaded`** — every platform leg failed.
  Open the failed `desktop` job; the `Collect installers` step names the
  platform that produced nothing.
- **`sha256sum -c` fails in `release`** — an artifact was corrupted in transit
  between jobs. Re-run the workflow; the checksum step exists precisely to
  catch this before publishing.
- **`pnpm install --frozen-lockfile` fails** — the lockfile is out of date.
  Run `pnpm install` locally and commit `pnpm-lock.yaml`.
- **The downloads page still shows the old release** — the endpoint caches for
  10 minutes. Wait, or restart the server.

## Files that define a release

- [`.github/workflows/desktop-release.yaml`](../.github/workflows/desktop-release.yaml) — the public release path.
- [`scripts/check-release-version.mjs`](../scripts/check-release-version.mjs) — tag/manifest version guard.
- [`scripts/collect-bundles.mjs`](../scripts/collect-bundles.mjs) — canonical installer names and SHA-256 files.
- [`scripts/build-web.mjs`](../scripts/build-web.mjs) — package-manager-neutral front-end build hook.
- [`apps/desktop/src-tauri/tauri.conf.json`](../apps/desktop/src-tauri/tauri.conf.json) — bundle targets and the build hook.
- [`apps/shell-server/src/releases.rs`](../apps/shell-server/src/releases.rs) — the `/api/v1/releases/latest` feed behind the downloads page.
- [`apps/web/src/routes/downloads.tsx`](../apps/web/src/routes/downloads.tsx) — the downloads page that renders this data.
