# Deploying the KERNEXT web server

The web server (`apps/shell-server`, binary `kernext-server`) serves the React
front end and the engine/PTY WebSocket APIs. It is designed as a **local
companion**, not a public service. Read
[`SECURITY.md`](SECURITY.md) before changing anything below.

## Build

```sh
# 1. Build the web front end (produces apps/web/dist)
npm ci
npm run web:build

# 2. Build and run the server
cargo run -p shell-server        # debug
cargo build --release -p shell-server
```

The executable is `kernext-server`. Note that the repository's
`.cargo/config.toml` pins a shared `build.target-dir` **outside the source
tree** (a workstation-specific setting for a removable-drive checkout), so the
binary does not appear under `./target/`. When building on another machine,
override it and run the binary from the directory you chose:

```sh
CARGO_TARGET_DIR="$HOME/kernext-target" cargo build --release -p shell-server
"$HOME/kernext-target/release/kernext-server"
```

The release archives published on GitHub ship `kernext` and `kernext-server`
together with no build step required.

The server serves `apps/web/dist` when it exists, or the directory named by
`KERNEXT_WEB_DIR`. If neither is present the API still works, but the site
routes return nothing.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `KERNEXT_SERVER_BIND` | `127.0.0.1:7878` | Listen address. Keep it loopback unless you have read [`SECURITY.md`](SECURITY.md). |
| `KERNEXT_SERVER_TOKEN` | unset | Bearer token required by every mutating endpoint and WebSocket. **Set this** whenever the server is reachable by anything other than your own user session. |
| `KERNEXT_MAX_SESSIONS` | `16` | Concurrent session cap. |
| `KERNEXT_WEB_DIR` | `apps/web/dist` | Directory of static front-end files to serve. |
| `KERNEXT_BIN` | sibling of the server executable | Path to the `kernext` CLI binary used for PTY sessions. |

Sessions idle for more than 30 minutes are removed by a background sweeper
that runs every 60 seconds.

## Endpoints

| Route | Purpose |
| --- | --- |
| `GET /` | Serves the built marketing site and web terminal |
| `GET /api/v1/health` | Server health, version, session count, PTY availability |
| `GET /api/v1/sessions` | Lists sessions |
| `POST /api/v1/sessions` | Creates an engine session |
| `GET /api/v1/sessions/{id}` | Session metadata |
| `DELETE /api/v1/sessions/{id}` | Removes a session |
| `POST /api/v1/sessions/{id}/execute` | Runs one command through the engine |
| `GET /api/v1/sessions/{id}/ws` | Engine session WebSocket |
| `GET /api/v1/pty` | Pseudoterminal WebSocket (ConPTY/openpty) |

Unmatched non-API paths fall back to `index.html`, so client-side route URLs
survive a refresh. API routes always take precedence over the static fallback.

Authentication, when a token is set, is `Authorization: Bearer <token>` on the
HTTP routes and either the same header or `?token=<token>` on WebSockets.

## Front-end development

```sh
npm run web:dev          # Vite dev server at http://127.0.0.1:5173
cargo run -p shell-server
```

The dev-server origins (`:5173`, `:4173`) and the bundled origin (`:7878`) are
the only browser origins the server accepts. The desktop application talks to
its own local runtime instead — it never calls this server.

## Reverse proxy (advanced, not recommended)

If you must place KERNEXT behind a proxy, understand what you are exposing:
an authenticated HTTP route to arbitrary command execution on the host. The
minimum viable hardening is:

1. Terminate TLS on the proxy.
2. Require real user authentication at the proxy **in addition** to
   `KERNEXT_SERVER_TOKEN`.
3. Keep `KERNEXT_SERVER_BIND` on loopback and proxy from the same host.
4. Run the server under an unprivileged, dedicated user account.

There is no multi-user model, audit log, or command allowlist in the server.
Do not expose it to untrusted networks.

## Desktop application

The desktop application is self-contained: `npm run desktop:build` produces
platform bundles (installers/packages) through Tauri, embedding the built
front end. It never contacts the web server. See
[`DESKTOP.md`](DESKTOP.md).
