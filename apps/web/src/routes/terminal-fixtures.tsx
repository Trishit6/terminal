import { useCallback, useEffect, useRef, useState } from 'react';
import { Button } from '@/components/ui/button';
import { TerminalScreen } from '@/components/terminal/terminal-screen';
import { useTerminalEngine } from '@/lib/terminal/engine';

/**
 * Terminal rendering fixtures.
 *
 * A backend-free page that drives the same `TerminalScreen` used by the web
 * and desktop terminals with scripted output: basic text, multi-line output,
 * ANSI colour, Unicode, long streams, wrapped lines, unbroken lines, and dense
 * block art (the vertical-overlap regression case). It exists for visual
 * regression checks of the DOM renderer, font metrics, and resize behaviour
 * without a shell server or PTY.
 */

const CRLF = '\r\n';
const PROMPT = 'kernext ~ $ ';

interface Fixture {
  id: string;
  label: string;
  build: () => string;
}

const LONG_UNBROKEN = 'K'.repeat(400);

const WRAPPED_PARAGRAPH =
  'KERNEXT pipelines connect built-in commands and external processes with ' +
  'real operating-system pipes. Output streams stage to stage without being ' +
  'collected into memory, and a redirection applies to its own stage only. ' +
  'This sentence is long enough to wrap across several terminal columns, ' +
  'which exercises the line-wrapping path of the screen buffer.';

const BLOCK_ART = [
  '  ████████████████████████████████████████████████████',
  '  ██                                              ██',
  '  ██   ██████   ██████   ██████   ██████         ██',
  '  ██   ██       ██       ██       ██              ██',
  '  ██   ██████   ██████   ██████   ██████         ██',
  '  ██       ██       ██       ██       ██          ██',
  '  ██   ██████   ██████   ██████   ██████   ███   ██',
  '  ██                                              ██',
  '  ████████████████████████████████████████████████████',
].join(CRLF);

function numberedLines(count: number): string {
  const lines: string[] = [];

  for (let index = 1; index <= count; index += 1) {
    const padded = String(index).padStart(4, ' ');
    lines.push(`${padded}  KERNEXT streaming output line`);
  }

  return lines.join(CRLF);
}

const FIXTURES: Fixture[] = [
  {
    id: 'basic',
    label: 'Basic text',
    build: () =>
      ['KERNEXT Shell v1.0.0', 'The Next Core of Computing.', 'Independent. Native. Engineered.', PROMPT].join(CRLF),
  },
  {
    id: 'lines',
    label: 'Multiple lines',
    build: () =>
      ['First line', 'Second line', 'Third line', 'Fourth line', 'Fifth line'].join(CRLF),
  },
  {
    id: 'colors',
    label: 'ANSI colours',
    build: () =>
      [
        'Normal text',
        '\u001b[32mGreen success message',
        '\u001b[33mYellow warning message',
        '\u001b[31mRed error message',
        '\u001b[36mCyan information message',
        '\u001b[1;94mBright bold blue message',
        '\u001b[38;2;34;211;238mTruecolour cyan message',
        '\u001b[7mInverse video message\u001b[0m still normal',
        '\u001b[0m',
      ].join(CRLF),
  },
  {
    id: 'unicode',
    label: 'Unicode',
    build: () =>
      [
        'KERNEXT ✓',
        'Unicode: café, naïve, 日本語',
        'Symbols: → ← ↑ ↓',
        'Emoji: 🔧 ⚡ 🧠 (wide characters)',
      ].join(CRLF),
  },
  {
    id: 'block-art',
    label: 'Block art',
    build: () => BLOCK_ART,
  },
  {
    id: 'wrapped',
    label: 'Wrapped text',
    build: () => WRAPPED_PARAGRAPH,
  },
  {
    id: 'unbroken',
    label: 'Unbroken text',
    build: () => LONG_UNBROKEN,
  },
  {
    id: 'lines-100',
    label: '100 lines',
    build: () => numberedLines(100),
  },
  {
    id: 'lines-1000',
    label: '1000 lines',
    build: () => numberedLines(1000),
  },
];

export function TerminalFixturesPage() {
  const { engine, version } = useTerminalEngine();
  const [size, setSize] = useState({ rows: 0, cols: 0 });
  const [height, setHeight] = useState<'tall' | 'short'>('tall');
  const sizeRef = useRef({ rows: 0, cols: 0 });

  const handleData = useCallback(
    (data: string) => {
      // A tiny local line editor so typing, backspace, Enter, Ctrl+C, and
      // Ctrl+L can be exercised without a shell behind the screen.
      for (const character of data) {
        switch (character) {
          case '\r':
            engine.writeNow(CRLF + PROMPT);
            break;
          case '\u0003':
            engine.writeNow('^C' + CRLF + PROMPT);
            break;
          case '\u000c':
            engine.clear();
            engine.writeNow(PROMPT);
            break;
          case '\u007f':
          case '\u0008':
            engine.writeNow('\u0008 \u0008');
            break;
          case '\u001b':
            engine.writeNow('^[');
            break;
          default:
            engine.writeNow(character);
        }
      }
    },
    [engine],
  );

  // Mirror the production resize wiring from `useTerminalConnection`: the
  // visible grid must track the viewport, or output that fits the screen gets
  // clamped to the 24x80 buffer default and rows overwrite each other.
  const handleResize = useCallback(
    (rows: number, cols: number) => {
      const previous = sizeRef.current;
      sizeRef.current = { rows, cols };
      setSize((current) =>
        current.rows === rows && current.cols === cols
          ? current
          : { rows, cols },
      );

      if (previous.rows === rows && previous.cols === cols) {
        return;
      }

      engine.resize(rows, cols);
    },
    [engine],
  );

  // Load a representative showcase on mount so a fresh page (or a headless
  // screenshot) immediately shows text, colour, Unicode, and block art.
  useEffect(() => {
    const showcase = FIXTURES.filter((fixture) =>
      ['basic', 'colors', 'unicode'].includes(fixture.id),
    );
    engine.reset();
    engine.writeNow(
      showcase.map((fixture) => fixture.build()).join(CRLF + CRLF) +
        CRLF +
        CRLF +
        BLOCK_ART +
        CRLF +
        PROMPT,
    );
  }, [engine]);

  const run = useCallback(
    (fixture: Fixture) => {
      engine.writeNow(fixture.build() + CRLF + PROMPT);
    },
    [engine],
  );

  const reset = useCallback(() => {
    engine.reset();
    engine.writeNow('KERNEXT render fixtures ready.' + CRLF + PROMPT);
  }, [engine]);

  return (
    <div className="mx-auto flex w-full max-w-site flex-col px-4 py-6 sm:px-6 lg:px-8">
      <div className="mb-5 flex flex-wrap items-end justify-between gap-4">
        <div className="max-w-2xl">
          <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
            Rendering fixtures
          </p>
          <h1 className="mt-1.5 text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
            Terminal render test
          </h1>
          <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
            The same <code className="rounded bg-surface-soft px-1 font-mono text-[11px]">TerminalScreen</code>{' '}
            the web and desktop terminals use, driven by scripted output. Use it
            to check font metrics, row spacing, ANSI colours, Unicode, wrapping,
            streaming output, and resize behaviour without a backend.
          </p>
        </div>

        <div className="flex flex-col gap-2 rounded-xl border border-border bg-surface/50 px-3.5 py-3 text-xs">
          <span className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
            Measured grid
          </span>
          <code className="font-mono text-[11px] text-foreground">
            {size.rows} rows × {size.cols} cols
          </code>
          <span className="font-mono text-[10px] text-muted-foreground">
            cell height follows the font
          </span>
        </div>
      </div>

      <div className="mb-4 flex flex-wrap items-center gap-2">
        {FIXTURES.map((fixture) => (
          <Button key={fixture.id} size="sm" variant="secondary" onClick={() => run(fixture)}>
            {fixture.label}
          </Button>
        ))}
        <Button size="sm" variant="outline" onClick={reset}>
          Reset
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => setHeight((current) => (current === 'tall' ? 'short' : 'tall'))}
        >
          {height === 'tall' ? 'Short viewport' : 'Tall viewport'}
        </Button>
      </div>

      <div className={height === 'tall' ? 'h-[calc(100dvh-16.5rem)] min-h-[520px]' : 'h-80 min-h-0'}>
        <TerminalScreen
          engine={engine}
          version={version}
          onData={handleData}
          onResize={handleResize}
          autoFocus
        />
      </div>

      <p className="mt-3 text-xs leading-relaxed text-muted-foreground">
        Interactive checks: click the terminal and type, use Backspace, Enter,
        Ctrl+C, Ctrl+L, and the arrow keys; scroll through long output; resize
        the browser window or zoom the page. Rows must never overlap, the block
        art must stay a solid frame, and every column must stay aligned.
      </p>
    </div>
  );
}
