# Contributing to KERNEXT

KERNEXT — **The Next Core of Computing.** OPEN • POWERFUL • INDEPENDENT.

Thanks for your interest. This document covers the ground rules that keep the
shell consistent; [`AGENTS.md`](AGENTS.md) carries the deeper invariants and is
the authoritative engineering contract.

## Ground rules

1. **One engine, three surfaces.** The CLI (`apps/shell-cli`), the web server
   and front end (`apps/shell-server`, `apps/web`), and the desktop
   application (`apps/desktop`) all execute through the same Rust engine
   (`shell-core` → `shell-runtime` → `shell-builtins`). Never duplicate shell
   semantics in another layer, and never simulate the shell in JavaScript.
2. **No wrapper shells.** External processes are launched with
   `std::process::Command` and separate arguments. The only exception is
   Windows `.cmd`/`.bat` scripts, which go through `%COMSPEC%` as a launcher —
   never as a parser.
3. **Built-ins win.** The registry is checked before functions, and functions
   before external resolution. Keep that order.
4. **Honesty in output.** Never print progress that did not happen, claim a
   command exists that is not installed, or document a feature that is not
   implemented. Tests must not be weakened to make a build pass.
5. **Branding** comes from the constants in `crates/shell-core/src/lib.rs`.
   Do not hardcode versions or product names; use `VERSION` /
   `Session::version()`. Internal crate names stay `shell-*`; user-facing
   surfaces say KERNEXT. See [`docs/BRANDING.md`](docs/BRANDING.md).

## Development setup

- Rust 1.85+ (Rust 2024 edition).
- Node.js + pnpm 11 (`package.json` pins `pnpm@11.25.0` via `packageManager`;
  Corepack handles it).
- A real terminal for interactive verification.

On the VFAT/`showexec` checkout, cargo artifacts automatically route to an
ext4 target directory via `.cargo/config.toml` — never create a repo-local
`target/` directory.

## Workflow

```sh
# Rust: all of these must pass before you open a PR
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Web
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test
pnpm build
```

Focused suites are listed in [`README.md`](README.md#development). Filesystem
tests use isolated temp directories and `Session::set_working_directory` —
never mutate the process working directory in tests. External-process and
pipeline tests use the test binary itself as the helper, so they never depend
on a particular external program.

## Pull requests

- Keep changes focused; one logical change per PR.
- Add or update tests next to the behavior they cover.
- Keep documentation in sync: `README.md`, `AGENTS.md`,
  `docs/COMMANDS.md` (built-in list and registry count), and
  `docs/PROJECT_STATUS.md` must reflect reality, including the built-in count
  in `crates/shell-builtins/src/registry.rs`.
- Desktop changes must keep `apps/desktop/src-tauri/build.rs` MinGW manifest
  shadowing intact (see [`docs/DESKTOP.md`](docs/DESKTOP.md)).
- Do not commit build artifacts (`apps/web/dist`, any `target/` directory) or
  modify `pnpm-lock.yaml` by hand.

## Reporting issues

Open an issue at <https://github.com/Trishit6/terminal/issues> with the
surface (CLI/Web/Desktop), platform, and a minimal reproduction. For
security reports follow [`docs/SECURITY.md`](docs/SECURITY.md) instead of
opening a public issue.

## License

No license has been selected for this project yet; contributions are accepted
into the repository as-is until one is chosen.
