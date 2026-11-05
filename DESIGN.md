---
name: "Custom Shell"
description: "A cross-platform command-line shell built in Rust"
colors:
  primary: "#4F46E5"
  primary-dim: "#4338CA"
  success: "#2D6A4F"
  success-dim: "#1B4332"
  warning: "#C4634A"
  warning-dim: "#8B4513"
  error: "#8B1A1A"
  error-dim: "#5C0D0D"
  muted: "#78716C"
  muted-dim: "#57534E"
  background: "#000000"
  surface: "#1C1917"
  text: "#E7E5E4"
  text-dim: "#A8A29E"
typography:
  banner-title:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 700
    lineHeight: 1
    letterSpacing: "0.05em"
    textTransform: "uppercase"
  banner-subtitle:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 400
    lineHeight: 1
    letterSpacing: "0.02em"
  prompt:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 400
    lineHeight: 1
  prompt-context:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 500
    lineHeight: 1
  command-output:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 400
    lineHeight: 1.4
  error:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 700
    lineHeight: 1
  error-detail:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 400
    lineHeight: 1
  help-header:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 700
    lineHeight: 1
    textTransform: "uppercase"
  help-command:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 400
    lineHeight: 1
  help-summary:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 400
    lineHeight: 1
  completion-item:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 400
    lineHeight: 1
  completion-type:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 500
    lineHeight: 1
  muted:
    fontFamily: "monospace"
    fontSize: "1em"
    fontWeight: 400
    lineHeight: 1
rounded:
  sm: "0"
  md: "0"
  lg: "0"
spacing:
  xs: "1"
  sm: "2"
  md: "4"
  lg: "8"
  xl: "16"
components:
  prompt-default:
    backgroundColor: "transparent"
    textColor: "{colors.primary}"
    typography: "{typography.prompt}"
    padding: "0 1"
  prompt-context:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    typography: "{typography.prompt-context}"
    padding: "0 1"
    rounded: "{rounded.sm}"
  prompt-continuation:
    backgroundColor: "transparent"
    textColor: "{colors.muted}"
    typography: "{typography.prompt}"
    padding: "0 1"
  banner:
    backgroundColor: "transparent"
    textColor: "{colors.text}"
    typography: "{typography.banner-title}"
    padding: "1 0"
    borderColor: "{colors.muted}"
    borderWidth: "1"
  banner-subtitle:
    backgroundColor: "transparent"
    textColor: "{colors.muted}"
    typography: "{typography.banner-subtitle}"
    padding: "0 0"
  help-header:
    backgroundColor: "transparent"
    textColor: "{colors.text}"
    typography: "{typography.help-header}"
    padding: "1 0 0"
  help-row:
    backgroundColor: "transparent"
    textColor: "{colors.text}"
    typography: "{typography.help-command}"
    padding: "0 2"
  help-summary:
    backgroundColor: "transparent"
    textColor: "{colors.text-dim}"
    typography: "{typography.help-summary}"
    padding: "0 0"
  help-external-note:
    backgroundColor: "transparent"
    textColor: "{colors.muted}"
    typography: "{typography.muted}"
    padding: "1 0 0"
  error-message:
    backgroundColor: "transparent"
    textColor: "{colors.error}"
    typography: "{typography.error}"
    padding: "0 0"
  error-detail:
    backgroundColor: "transparent"
    textColor: "{colors.text-dim}"
    typography: "{typography.error-detail}"
    padding: "0 2"
  command-output:
    backgroundColor: "transparent"
    textColor: "{colors.text}"
    typography: "{typography.command-output}"
    padding: "0 0"
  completion-menu:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    typography: "{typography.completion-item}"
    padding: "1 2"
    rounded: "{rounded.sm}"
    borderColor: "{colors.muted}"
    borderWidth: "1"
  completion-item-selected:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.background}"
    typography: "{typography.completion-item}"
    padding: "0 1"
  completion-type-badge:
    backgroundColor: "{colors.muted}"
    textColor: "{colors.background}"
    typography: "{typography.completion-type}"
    padding: "0 1"
    rounded: "{rounded.sm}"
  history-search:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.primary}"
    typography: "{typography.prompt}"
    padding: "0 1"
    rounded: "{rounded.sm}"
  multiline-indicator:
    backgroundColor: "transparent"
    textColor: "{colors.muted}"
    typography: "{typography.prompt}"
    padding: "0 1"
---

# Design System: Custom Shell

## Overview

**Creative North Star: "The Quiet Professional"**

A polished but restrained terminal interface that feels like a well-configured developer tool. Every visual decision serves clarity and density of information without crowding. The shell speaks through structure — box-drawing, aligned columns, meaningful prefixes — not decoration. Color is a signal, not ornament: a single accent for primary actions, semantic hues for state, muted grays for metadata. The default experience is monochrome and fully functional; color enhances but never gatekeeps.

**Key Characteristics:**
- **Structural over decorative**: Box-drawing, arrows, brackets, prefixes convey hierarchy
- **Density with purpose**: Information-rich but scannable; context appears when relevant
- **Restrained palette**: One primary accent, semantic state colors, stepped neutrals
- **Graceful degradation**: Full function without color; color only amplifies signal
- **Terminal-native**: Embraces the character grid, monospace rhythm, instant transitions

## Colors

A restrained ANSI palette optimized for 16-color, 256-color, and truecolor terminals. All values are truecolor hex; 256-color and 16-color fallbacks map to nearest semantic neighbors.

### Primary
- **Deep Indigo** (#4F46E5): Primary accent — prompt symbol, active completion, history search, version banner. The shell's signature hue.
- **Deep Indigo Dim** (#4338CA): Hover/active states for interactive elements (completion selection).

### Success
- **Forest Green** (#2D6A4F): Successful operations, confirmed actions, positive status indicators.
- **Forest Green Dim** (#1B4332): Subtle success contexts (e.g., zero exit code in prompt).

### Warning
- **Burnt Sienna** (#C4634A): Non-fatal issues, deprecation notices, destructive confirmations.
- **Burnt Sienna Dim** (#8B4513): Muted warning contexts.

### Error
- **Oxblood** (#8B1A1A): Errors, failures, critical messages, failed command prefix.
- **Oxblood Dim** (#5C0D0D): Error detail text, secondary error context.

### Neutral
- **Stone** (#78716C): Secondary text, metadata, disabled states, banner rules, completion borders.
- **Stone Dim** (#57534E): Deeply muted text, placeholder hints, history timestamps.
- **Background** (#000000): Terminal default background (assumed).
- **Surface** (#1C1917): Elevated surfaces — completion menu, history search, context blocks.
- **Text** (#E7E5E4): Primary command output, help text, prompt context.
- **Text Dim** (#A8A29E): Help summaries, command descriptions, non-critical output.

### Named Rules
**The One Accent Rule.** Deep Indigo appears on ≤10% of any screen. Its rarity is the signal.

**The Semantic Color Rule.** Success/Warning/Error colors convey state only. Never used for decoration or brand.

**The Monochrome-First Rule.** Every screen is fully legible and functional in 16-color monochrome. Color is progressive enhancement.

## Typography

A single monospace font stack — the terminal's configured font — with weight, case, and spacing creating hierarchy.

**Display Font:** Terminal monospace (user-configured)
**Body Font:** Terminal monospace (user-configured)
**Label/Mono Font:** Terminal monospace (user-configured)

**Character:** Uniform character grid. Hierarchy through weight (400/500/700), case (UPPERCASE for headers), letter-spacing (0.02–0.05em for display), and structural characters — never through proportional fonts or size variation.

### Hierarchy
- **Banner Title** (700, 1em, 1.0, +0.05em, UPPERCASE): Shell identity. Centered within box-drawing frame.
- **Banner Subtitle** (400, 1em, 1.0, +0.02em): Tagline. Centered, muted.
- **Prompt** (400, 1em, 1.0): Default input line. Primary color for symbol (`>`), text color for path/context.
- **Prompt Context** (500, 1em, 1.0): Git branch, venv, exit code — inline, surface background, padded.
- **Continuation Prompt** (400, 1em, 1.0, muted): Multiline (`::: `), history search (`(reverse-search: )`).
- **Command Output** (400, 1em, 1.4): Standard output. 1.4 line-height for readability.
- **Error** (700, 1em, 1.0, Oxblood): Error prefix (`command: `). Bold for immediate recognition.
- **Error Detail** (400, 1em, 1.0, Text Dim): Error explanation, indented 2 chars.
- **Help Header** (700, 1em, 1.0, UPPERCASE): "AVAILABLE COMMANDS". Section divider.
- **Help Command** (400, 1em, 1.0): Left column, 10-char fixed width, Primary color when colored.
- **Help Summary** (400, 1em, 1.0, Text Dim): Right column, wrapped at terminal width.
- **Completion Item** (400, 1em, 1.0): Menu entries. Selected = inverted (Primary bg, Background fg).
- **Completion Type Badge** (500, 1em, 1.0, Stone bg/Background fg): File type, command kind — padded, rounded.
- **Muted** (400, 1em, 1.0, Stone Dim): External command note, timestamps, hints.

### Named Rules
**The Fixed Grid Rule.** No proportional fonts. No variable font sizes. The terminal character cell is the atomic unit.

**The Weight Hierarchy Rule.** 400 = body, 500 = contextual emphasis, 700 = structural headers and errors. No other weights.

## Layout

Terminal character grid. Single-column flow with aligned sub-structures.

### Spatial Model
- **Banner**: 40-char wide box-drawing frame (`=`), centered content, 1-line vertical padding inside frame.
- **Prompt**: Single line by default. `custom-shell > ` (15 chars) + working directory (truncated at 50% terminal width).
- **Prompt with Context**: Multi-line. Line 1: context badges (git, venv, $?) in surface boxes. Line 2: prompt symbol + path.
- **Help Table**: Two columns. Column 1: command name, 10 chars fixed, left-aligned. Column 2: summary, wraps at terminal width minus 12 chars.
- **Completion Menu**: Popup below cursor. Max 10 rows, scrollable. Item: `[type-badge] name`. Selected row highlighted.
- **Error Block**: Error prefix on own line. Detail indented 2 spaces on following line(s).
- **Command Output**: Raw stdout/stderr passthrough. No framing unless command provides it.

### Density
- **Compact**: Default. Single-line prompt, no blank lines between commands.
- **Breathing**: When context badges appear, 1 blank line before prompt.
- **Banner**: 1 blank line above and below the framed block.

### Responsive Behavior
- **< 60 cols**: Prompt truncates path aggressively (`…/dir`). Help wraps summary at 40 chars. Completion menu narrows.
- **60–100 cols**: Standard layout. Path shows ~30 chars.
- **> 100 cols**: Full path. Help column gap = 2 chars. Completion shows type badges.

### Named Rules
**The 10-Char Command Column Rule.** Help command names occupy exactly 10 chars. Longer names truncate with `…`; shorter pad with spaces. Alignment is inviolate.

**The 50% Path Rule.** Working directory in prompt never exceeds 50% of terminal width. Truncates left with `…/`.

## Elevation & Depth

No shadows. No blur. Depth conveyed through **structural characters** and **surface contrast**.

### Structural Vocabulary
- **Box-drawing frame** (`=`, `│`, `┌`, `┐`, `└`, `┘`): Banner. Highest visual weight. Encloses identity.
- **Surface boxes** (`[`, `]`, `▌`, `▐`): Prompt context badges (git, venv, exit code). Inline, padded.
- **Continuation arrow** (`::: `): Multiline input. Visual continuation, not elevation.
- **Search bracket** (`(reverse-search: )`): History search mode. Encloses mode + term.
- **Completion popup**: Surface background (`#1C1917`), Stone border (`│`, `─`). Appears below cursor.
- **Error prefix** (`command: `): Bold Oxblood. Structural marker, not color-dependent.
- **Indent** (`  `): Two-space continuation for error detail, wrapped help summaries.

### Named Rules
**The Frame Rule.** Only the startup banner uses a full box-drawing frame. No other element frames itself.

**The Surface Rule.** Elevated UI (completion, history search, context badges) uses `Surface` background + `Stone` border. Base UI (prompt, output, help) uses transparent background.

**The Instant Transition Rule.** Terminal UI has no animation. State changes are immediate. Reduce-motion is inherent.

## Shapes

**Corner Language:** Sharp (0 radius). Terminal character grid has no curves. Box-drawing characters create perceived corners.

**Border Language:** Single-line box-drawing (`│`, `─`, `┌`, `┐`, `└`, `┘`) for banner. Single-character brackets (`[`, `]`) for inline surfaces. No double-line, no rounded.

**Clipping:** Text truncates with `…` (U+2026) when exceeding allocated columns. No ellipsis in middle — only left-truncate paths, right-truncate command names.

**Recurring Silhouettes:**
- **Banner frame**: 40-char wide, 5-line tall (top rule, title, subtitle, bottom rule, blank).
- **Context badge**: `[main]` `[(venv)]` `[0]` — variable width, fixed height (1 line), surface bg.
- **Completion row**: `[F] filename` ` [C] command` — type badge + name, 1 line each.
- **Error block**: `command: error message` + `  detail line` — 2+ lines, left-aligned.

## Components

### Prompt
**Character:** Compact, informative, color-accented symbol.
- **Default**: `custom-shell > ` (Primary color `>`, Text color path).
- **With Context**: Multi-line. Line 1: `[git:main] [(venv)] [0]` (Surface bg, Text fg, Stone border). Line 2: prompt symbol + truncated path.
- **Continuation**: `::: ` (Muted).
- **States**: Default / Context / Continuation / History Search.
- **Color**: `>` in Primary. Context badges: git=Primary, venv=Success, $!=Error (when non-zero).

### Banner
**Character:** Framed identity block. Appears once at startup.
- **Frame**: 40-char `=` rules top/bottom. `│` sides implicit via centered content.
- **Title**: "CUSTOM SHELL" (Banner Title style, centered).
- **Subtitle**: "Independent Rust CLI" (Banner Subtitle style, centered, Muted).
- **Info lines**: Version, Platform, Working directory (left-aligned inside frame, Muted label + Text value).
- **Hints**: "Type 'help'…", "Type 'exit'…" (Muted, centered).

### Help
**Character:** Scannable two-column reference. Dense but aligned.
- **Header**: "AVAILABLE COMMANDS" (Help Header, UPPERCASE, 1 blank line above).
- **Rows**: Command (Help Command, 10-char fixed, Primary when colored) + Summary (Help Summary, Text Dim, wraps).
- **External Note**: "Commands not listed above are executed as external programs." (Help External Note, Muted, 1 blank line above).

### Command Output
**Character:** Transparent passthrough. No framing, no prefix. Commands own their output format.
- **Stdout**: Direct write, Text color.
- **Stderr**: Direct write, Text color (commands may use Error color internally).
- **Pipeline**: Final stage output only. Intermediate stages silent unless error.

### Error Messages
**Character:** Immediate recognition. Bold prefix + indented detail.
- **Prefix**: `command: ` (Error style, Bold, Oxblood). Always includes command name.
- **Detail**: Indented 2 spaces (Error Detail, Text Dim). Wraps at terminal width.
- **No color fallback**: Bold + prefix sufficient for recognition in monochrome.

### Completions
**Character:** Dense menu with type signal. Appears below cursor.
- **Menu**: Surface bg, Stone border, max 10 rows, scroll indicator when overflow.
- **Row**: `[TYPE] name` — Type badge (Completion Type Badge, Stone bg) + name (Completion Item).
- **Selection**: Inverted (Primary bg, Background fg). No cursor — whole row highlights.
- **Types**: `F` file, `D` directory, `C` command, `V` variable, `B` builtin, `H` history.

### History Search
**Character:** Inline mode indicator. Replaces prompt during Ctrl+R.
- **Format**: `(reverse-search: term)` or `(failing reverse-search: term)` (History Search style, Surface bg, Primary fg).
- **Behavior**: Updates live. Enter accepts, Ctrl+G cancels, Ctrl+R cycles.

### Multiline Indicator
**Character:** Quiet continuation signal.
- **Format**: `::: ` (Multiline Indicator, Muted). No box, no frame. Pure structural character.

## Do's and Don'ts

### Do:
- **Do** use box-drawing (`=`, `│`, `┌`, `┐`, `└`, `┘`) only for the startup banner frame.
- **Do** use square brackets (`[`, `]`) for inline context badges (git, venv, exit code).
- **Do** align help command names to exactly 10 characters. Truncate with `…` or pad with spaces.
- **Do** truncate prompt path at 50% terminal width, left-truncating with `…/`.
- **Do** prefix every error with `command: ` in bold (Oxblood when colored).
- **Do** indent error detail lines by exactly 2 spaces.
- **Do** use Primary (Deep Indigo) only for: prompt symbol `>`, active completion row, history search, version in banner.
- **Do** use semantic colors (Success/Warning/Error) only for state — never for decoration.
- **Do** render all UI fully functional in 16-color monochrome. Color is progressive enhancement.
- **Do** keep completion menu to max 10 rows with scroll indicator.
- **Do** use Surface background + Stone border for all elevated UI (completions, history search, context badges).

### Don't:
- **Don't** use double-line box-drawing (`═`, `║`, `╔`, `╗`, `╚`, `╝`) anywhere.
- **Don't** use rounded corners or Unicode box-drawing with curves — terminal grid is sharp.
- **Don't** add decorative colors, gradients, or background washes.
- **Don't** frame command output, help sections, or error blocks — only the banner frames.
- **Don't** exceed 10% Primary color coverage on any screen.
- **Don't** use Success/Warning/Error colors for non-state purposes (e.g., don't color command names in help).
- **Don't** animate, fade, or transition any UI element. Terminal changes are instant.
- **Don't** show a cursor/pointer in completion menu — highlight the whole row.
- **Don't** wrap command names in help — truncate at 10 chars with `…`.
- **Don't** use proportional fonts, variable font sizes, or letter-spacing outside defined scale.