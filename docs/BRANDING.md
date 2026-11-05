# KERNEXT Branding

## Identity

| Element | Value |
| --- | --- |
| Product name | **KERNEXT** |
| Tagline | **The Next Core of Computing.** |
| Values | **OPEN • POWERFUL • INDEPENDENT** |
| Support phrase | **Build. Execute. Innovate.** |
| Prompt | `kernext ~ $ ` |
| Executable | `kernext` |
| Internal package prefix | `shell-*` (unchanged for stability) |

KERNEXT is always written in uppercase when used as a wordmark or product name.
The command typed by users is lowercase: `kernext`.

## Wordmark

The CLI uses a monospace block wordmark rendered from six rows at 80+ columns:

```text
██╗  ██╗ ███████╗ ██████╗  ███╗   ██╗ ███████╗ ██╗  ██╗ ████████╗
██║ ██╔╝ ██╔════╝ ██╔══██╗ ████╗  ██║ ██╔════╝ ╚██╗██╔╝ ╚══██╔══╝
█████╔╝  █████╗   ██████╔╝ ██╔██╗ ██║ █████╗    ╚███╔╝     ██║
██╔═██╗  ██╔══╝   ██╔══██╗ ██║╚██╗██║ ██╔══╝    ██╔██╗     ██║
██║  ██╗ ███████╗ ██║  ██║ ██║ ╚████║ ███████╗ ██╔╝ ██╗    ██║
╚═╝  ╚═╝ ╚══════╝ ╚═╝  ╚═╝ ╚═╝  ╚═══╝ ╚══════╝ ╚═╝  ╚═╝    ╚═╝
```

At 24–79 columns the CLI uses a compact three-row mark:

```text
╦╔═╔═╗╦═╗╔╗╔╔═╗═╗╔╦
╠╩╗║╣ ╠╦╝║║║║╣  ╠╩╗║
╩ ╩╚═╝╩╚═╝╚╝╚═╝═╝ ╩
```

Below 24 columns it falls back to the plain `KERNEXT` wordmark.

The logo is never stretched, recolored arbitrarily, or placed on a background
that reduces contrast.

## Color

The CLI uses ANSI colors gated by terminal capability and user preference.

| Role | ANSI | Intent |
| --- | --- | --- |
| Primary | Bright blue `\x1b[94m` | Structure, core geometry |
| Accent | Bright cyan `\x1b[96m` | Wordmark highlight, focus |
| Secondary | Bright violet `\x1b[95m` | Tagline and future-facing energy |
| Success | Bright green `\x1b[92m` | Completed initialization stages |
| Failure | Bright red `\x1b[91m` | Failed stages only |
| Muted | Dim `\x1b[2m` | Metadata, version, paths |

Rules:

- `NO_COLOR` disables color completely.
- `TERM=dumb` disables color completely.
- Piped or redirected output is never colored or animated.
- Color is an enhancement; every message must remain readable as plain text.
- Red is reserved for genuine failure, never decoration.

## Voice

KERNEXT is direct, technical, and confident without hype.

- Prefer concrete verbs: **Build. Execute. Innovate.**
- State what the software does, not what it aspires to be.
- Never claim a feature works when it is partial. Say "parses" versus "runs".
- Use sentence case for prose and uppercase only for the wordmark.
- Keep the tagline punctuated: **The Next Core of Computing.**

## Surface boundaries

| Surface | Status | Visual direction |
| --- | --- | --- |
| KERNEXT CLI | Implemented | Text-only, monospace wordmark, restrained ANSI palette |
| KERNEXT Web | Planned | Graphical splash, mountain wallpaper, browser terminal |
| KERNEXT Desktop | Planned | Native shell surface on the same core engine |

The CLI does not render the graphical wallpaper or splash. Those belong to the
future Web and Desktop surfaces, which must not be presented as implemented.

## Configuration defaults

```toml
prompt = "kernext ~ $ "
color = true
startup_animation = true
quiet_startup = false
accent_color = "bright-cyan"
secondary_color = "bright-blue"
highlight_color = "bright-magenta"
success_color = "bright-green"
error_color = "bright-red"
prompt_color = "bright-cyan"
```

The palette drives the animated wordmark gradient, checklist markers, welcome
text, and prompt. `terminal_font` and `apply_terminal_font` are optional OSC 50
hints; the CLI never requires a particular font to be installed.

The prompt is intentionally short enough to leave room for command output and
uses `~` as the default working-directory shorthand rather than a full path.
