# KERNEXT command reference

KERNEXT ships 56 native built-in commands. Built-ins always take precedence
over external programs and shell functions: a program named `echo` on `PATH`
never overrides the native `echo`. Anything that is not a built-in, function, or
path is resolved and launched as an external process (see
[`README.md`](../README.md#external-process-execution)).

Run `help` inside any KERNEXT surface for the one-line list. The registry lives
in `crates/shell-builtins/src/registry.rs`.

All commands below are available in every surface — CLI, Web, and Desktop —
because all three share the same Rust engine.

## Core shell

| Command | Usage | Description |
| --- | --- | --- |
| `help` | `help` | Lists every built-in command. |
| `version` | `version` | Prints `KERNEXT v<version>`. |
| `exit` | `exit [status]` | Prints a farewell and shuts the session down. The optional status becomes the process exit status. |
| `clear` | `clear` | Clears the terminal screen and scrollback. Acts only as a single command, never a pipeline stage. |
| `history` | `history [clear\|search\|limit]` | Shows numbered session history. `history clear` empties it, `history search <text>` filters it, `history limit <n>` caps it for the session. |
| `pwd` | `pwd` | Prints the session working directory. |
| `cd` | `cd [path]` | Changes and canonicalizes the session working directory. No argument goes to the home directory. |
| `run` | `run script.rsh` | Runs a KERNEXT script file natively with session state. |
| `sleep` | `sleep seconds` | Pauses for the given number of seconds. |
| `true` | `true` | Returns a successful status. |
| `false` | `false` | Returns a failing status. |

## Filesystem

| Command | Usage | Description |
| --- | --- | --- |
| `ls` | `ls [-a] [path]` | Lists directory contents; directories get a trailing separator. `-a`/`--all` includes hidden entries. |
| `tree` | `tree [-a] [-d] [-L n] [path]` | Displays a directory tree. `-a` shows hidden files, `-d` directories only, `-L n`/`--level n` limits depth. |
| `find` | `find [path] [filters]` | Searches files and directories. Filters: `-name`, `-iname`, `-path`, `-ipath` (glob patterns), `-type f\|d\|l`, `-maxdepth n`, `-mindepth n`. |
| `stat` | `stat [-L] file` | Shows file metadata (size, times, type, permissions). `-L`/`--dereference` follows symlinks. |
| `mkdir` | `mkdir [-p] dir` | Creates directories. `-p`/`--parents` creates missing parents. |
| `touch` | `touch file` | Creates a file or updates its modification time. |
| `cp` | `cp [-r] source... dest` | Copies files. `-r`/`-R`/`--recursive` copies directories. |
| `mv` | `mv source... dest` | Moves or renames files and directories. |
| `rm` | `rm [-r] [-f] path` | Removes entries. `-r`/`-R`/`--recursive` for directories, `-f`/`--force` ignores missing files. |

## Text processing

All of these read files or pipeline standard input and keep stdout and stderr
separate, so redirections can target them individually.

| Command | Usage | Description |
| --- | --- | --- |
| `cat` | `cat [file...]` | Prints file contents, or pipeline/`<` input when no file is given. |
| `head` | `head [-n n] [file...]` | Shows the first lines (default 10). Accepts `-n n`, `-nn`, `--lines n`, `--lines=n`, `-q`/`--quiet`. |
| `tail` | `tail [-n n] [file...]` | Shows the last lines (default 10). Same flags as `head`. |
| `wc` | `wc [-l] [-w] [-m] [-c] [file...]` | Counts lines, words, characters, and bytes; prints all four by default. |
| `grep` | `grep [-e] pattern [file...]` | Searches for a **literal** pattern (no regular expressions). Flags: `-e pattern`, `-i` ignore case, `-v` invert, `-n` line numbers, `-c` count only, `-q` quiet, `-F` accepted for compatibility. |
| `sort` | `sort [-r] [-n] [-u] [-f] [file...]` | Sorts lines. `-r` reverse, `-n` numeric, `-u` unique, `-f` fold case. |
| `uniq` | `uniq [-c] [-d] [-u] [file...]` | Reports or omits adjacent repeated lines. `-c` counts, `-d` duplicates only, `-u` unique only. |

## Scripting helpers

Value commands (`len`, `type`, `str`, `int`, `float`, `bool`, `expr`, `wc`)
print a bare value with no trailing newline, like POSIX `expr`.

| Command | Usage | Description |
| --- | --- | --- |
| `echo` | `echo [args...]` | Prints arguments separated by spaces. |
| `print` | `print [args...]` | Scripting form of `echo`; identical output. |
| `input` | `input variable` | Reads one line from pipeline or `<` input and assigns it. Never reads the terminal; interactive use without redirection reports a clear error. |
| `len` | `len value` | Reports the character length of a value. |
| `type` | `type value` | Reports a value's inferred type (`string`, `integer`, `float`, `boolean`, `empty`). |
| `str` | `str value` | Converts a value to a string. |
| `int` | `int value` | Converts to an integer, failing with a clear message when it cannot. |
| `float` | `float value` | Converts to a floating-point number. |
| `bool` | `bool value` | Converts to a boolean (`true`, `false`, `1`, `0`, case-insensitive). |
| `test` | `test expression` | Evaluates a conditional expression; sets `$?` to 0 or 1. |
| `[` | `[ expression ]` | Bracket alias of `test`; the closing `]` is required. |
| `expr` | `expr expression` | Evaluates arithmetic (`+ - * / %`) and comparisons; comparisons return 1 or 0. |

### `test` operators

Unary: `-e` exists, `-a` exists (alias), `-f` regular file, `-d` directory,
`-L`/`-h` symlink, `-s` non-empty, `-r` readable, `-w` writable, `-x`
executable, `-z` empty string, `-n` non-empty string.

Binary: `=` and `==` string equality, `!=` string inequality, and integer
comparisons `-eq`, `-ne`, `-lt`, `-le`, `-gt`, `-ge`.

Logical: `-a` and, `-o` or. Parenthesized grouping is not part of `test`; use
shell groups.

## Job control

Job IDs are shell job numbers, never operating-system PIDs.

| Command | Usage | Description |
| --- | --- | --- |
| `jobs` | `jobs` | Lists job number, state (`running`, `done`, `failed`, `killed`), and exit code. |
| `wait` | `wait [id]` | Waits for one job or every job. |
| `kill` | `kill id` | Terminates a running background job. |
| `fg` | `fg id` | Waits for a background job and prints its command (does not hand over the terminal yet). |
| `bg` | `bg id` | Reports a background job's state. |

Background work starts with a trailing `&`. External commands are spawned and
owned by the job table; other expressions run on a detached thread with a
cloned session, so their `cd`/variables cannot leak into the foreground shell.
There is no asynchronous completion notice yet — run `jobs` or `wait` to
inspect jobs.

## System information

| Command | Usage | Description |
| --- | --- | --- |
| `whoami` | `whoami` | Shows the current user name. |
| `date` | `date` | Shows the local date and time. |
| `host` | `host hostname` | Resolves a hostname through the system resolver and prints each address (IPv4 and IPv6), for example `localhost has address 127.0.0.1`. Uses standard-library resolution only — no network probes. |
| `sysinfo` | `sysinfo` | Prints product, version, platform, family, architecture, CPU core count, user, process ID, and working directory. |

## Variables and environment

| Command | Usage | Description |
| --- | --- | --- |
| `export` | `export NAME[=value]` | Marks variables for export to child processes. |
| `unset` | `unset NAME` | Removes shell variables (local and exported). |
| `env` | `env` | Shows the effective environment passed to child processes. |
| `set` | `set` | Shows local, exported, and inherited variables separately. |

See [`README.md`](../README.md#variable-expansion) for assignment syntax
(`name = value`), expansion (`$name`, `${name}`, `$?`, `$$`), and the
three-layer environment model.

## Path helpers

| Command | Usage | Description |
| --- | --- | --- |
| `which` | `which command` | Locates a built-in or resolves an executable through `PATH`. |
| `basename` | `basename path [suffix]` | Strips the directory (and optional suffix) from a path. |
| `dirname` | `dirname path` | Strips the last component from a path. |
| `realpath` | `realpath path` | Resolves a path to an absolute, canonical path. |

All paths are resolved against the session working directory, so relative
paths behave identically in the CLI, Web, and Desktop surfaces.
