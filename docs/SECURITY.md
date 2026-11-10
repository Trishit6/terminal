# KERNEXT security model

KERNEXT is a shell. Every surface that can execute a command can run any
program the host user can run. This document describes the safeguards that
exist, their limits, and the deployment rules that follow from them.

## Threat model summary

| Surface | Executes commands | Where | Intended exposure |
| --- | --- | --- | --- |
| CLI (`kernext`) | Yes | Local terminal | The local user, directly |
| Desktop (Tauri) | Yes | Local Rust runtime (`shell-desktop`) | The local user, directly |
| Web server (`shell-server`) | Yes | Local Axum server | **Loopback only** by design |

The web server is a **local companion** for the bundled web front end. It is
not, and must not be, a publicly reachable remote-shell service. Anyone who
can reach the HTTP port can execute commands with the server process's
permissions. Treat `KERNEXT_SERVER_BIND` as the security boundary.

## Web server safeguards

The safeguards are layered and on by default (`apps/shell-server/src/`):

1. **Loopback bind by default.** `KERNEXT_SERVER_BIND` defaults to
   `127.0.0.1:7878`. Binding to `0.0.0.0` or another interface is an explicit
   operator choice and exposes command execution to the network.
2. **Bearer-token authentication.** When `KERNEXT_SERVER_TOKEN` is set, every
   mutating endpoint requires `Authorization: Bearer <token>`. The comparison
   is constant-time. Without the variable the server logs a loud warning and
   trusts any client that can reach it.
3. **WebSocket authentication.** Browsers cannot attach `Authorization`
   headers to WebSocket upgrades, so the token may also arrive as a `?token=`
   query parameter; the header wins when both are present
   (`auth::authorize_ws`).
4. **Origin allowlist.** Browser `Origin` headers are validated against a
   fixed list — the Vite dev servers, the bundled server origin, and Tauri's
   custom origins. A foreign web page cannot drive the local server from a
   browser. Requests without an `Origin` header (curl, desktop runtime,
   integration tests) are allowed, since the check exists to stop browsers.
5. **Session caps.** `KERNEXT_MAX_SESSIONS` (default 16) caps concurrent
   sessions, so a client cannot exhaust memory with unbounded sessions.
6. **Idle cleanup.** A background sweeper runs every 60 seconds and removes
   sessions idle for more than 30 minutes, freeing their state and PTY
   processes.
7. **Command size limit.** Input larger than 64 KiB
   (`MAX_COMMAND_BYTES`) is rejected before evaluation.
8. **Session isolation.** Every session owns its working directory, variables,
   and history. One session cannot read another session's shell state.

These are local-development controls, not a multi-user authorization system.
There is no per-user identity, RBAC, audit log, or rate limiting beyond the
session cap.

## What the safeguards do not cover

- **No network isolation of commands.** Commands executed through the server
  run with full local permissions of the server process, including network
  access. A public deployment turns KERNEXT into an unauthenticated (or
  single-token) remote code execution endpoint.
- **No sandboxing.** External processes are not placed in containers, chroots,
  or OS sandboxes.
- **No TLS.** The server speaks plain HTTP/WS. The token travels in cleartext
  unless a reverse proxy terminates TLS in front — which is still not a
  recommended topology.
- **Desktop trust.** The desktop application runs commands through its own
  local Rust runtime and never through the web server; its security posture
  is that of any locally installed developer tool.

## Deployment rules

1. Keep the default loopback bind unless you have a specific reason.
2. Always set `KERNEXT_SERVER_TOKEN` when the server runs anywhere other than
   an isolated developer machine.
3. Never expose the web server directly to the internet (no `0.0.0.0` binds,
   no port forwarding, no public reverse proxy to the API routes). If a
   remote terminal is genuinely needed, put it behind an authenticating proxy
   and accept that the token is the only barrier — and that TLS must be
   terminated upstream.
4. Run the server as a regular user, not root/administrator.

## Responsible disclosure

The repository lives at <https://github.com/Trishit6/terminal>. If private
vulnerability reporting is enabled there, report through
<https://github.com/Trishit6/terminal/security/advisories/new>; otherwise
contact the maintainer privately. Please do not open a public issue that
describes a vulnerability.
