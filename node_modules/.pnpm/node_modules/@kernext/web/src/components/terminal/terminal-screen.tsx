import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from 'react';
import { colorToCss, styleKey } from '@/lib/terminal/palette';
import { keyToTerminalData } from '@/lib/terminal/input';
import type { TerminalEngine } from '@/lib/terminal/engine';
import type { Cell } from '@/lib/terminal/types';
import { cn } from '@/lib/utils';

export interface TerminalScreenHandle {
  focus: () => void;
}

export interface TerminalScreenProps {
  engine: TerminalEngine;
  version: number;
  onData: (data: string) => void;
  onResize: (rows: number, cols: number) => void;
  fontSize?: number;
  fontFamily?: string;
  cursorStyle?: 'block' | 'bar' | 'underline';
  autoFocus?: boolean;
  className?: string;
}

const MAX_RENDERED_SCROLLBACK = 500;

export const TerminalScreen = forwardRef<TerminalScreenHandle, TerminalScreenProps>(
  function TerminalScreen(
    {
      engine,
      version,
      onData,
      onResize,
      fontSize = 13,
      fontFamily,
      cursorStyle = 'block',
      autoFocus = false,
      className,
    },
    ref,
  ) {
    const shellRef = useRef<HTMLDivElement>(null);
    const scrollRef = useRef<HTMLDivElement>(null);
    const inputRef = useRef<HTMLTextAreaElement>(null);
    const probeRef = useRef<HTMLSpanElement>(null);
    const stuckRef = useRef(true);
    const onResizeRef = useRef(onResize);
    const composingRef = useRef(false);
    const [cellHeight, setCellHeight] = useState(22);
    const buffer = engine.buffer;

    onResizeRef.current = onResize;

    const measure = useCallback(() => {
      const probe = probeRef.current;
      const scroll = scrollRef.current;

      if (!probe || !scroll) {
        return;
      }

      const probeRect = probe.getBoundingClientRect();
      const computed = window.getComputedStyle(probe);
      const cellWidth = probeRect.width / 20;
      const lineHeight = Number.parseFloat(computed.lineHeight);
      const height =
        Number.isFinite(lineHeight) && lineHeight > 0
          ? lineHeight
          : probeRect.height;

      if (cellWidth <= 0 || height <= 0) {
        return;
      }

      const scrollStyle = window.getComputedStyle(scroll);
      const paddingX =
        (Number.parseFloat(scrollStyle.paddingLeft) || 0) +
        (Number.parseFloat(scrollStyle.paddingRight) || 0);
      const paddingY =
        (Number.parseFloat(scrollStyle.paddingTop) || 0) +
        (Number.parseFloat(scrollStyle.paddingBottom) || 0);
      const availableWidth = Math.max(0, scroll.clientWidth - paddingX);
      const availableHeight = Math.max(0, scroll.clientHeight - paddingY);
      const rows = Math.max(8, Math.floor(availableHeight / height));
      const cols = Math.max(24, Math.floor(availableWidth / cellWidth));

      setCellHeight(height);
      onResizeRef.current(rows, cols);
    }, []);

    useEffect(() => {
      measure();
      const scroll = scrollRef.current;

      if (!scroll) {
        return;
      }

      const observer = new ResizeObserver(() => measure());
      observer.observe(scroll);
      const frame = window.requestAnimationFrame(measure);

      return () => {
        observer.disconnect();
        window.cancelAnimationFrame(frame);
      };
    }, [measure, fontSize, fontFamily]);

    useEffect(() => {
      if (autoFocus) {
        const frame = window.requestAnimationFrame(() => {
          inputRef.current?.focus();
        });

        return () => window.cancelAnimationFrame(frame);
      }

      return undefined;
    }, [autoFocus]);

    useImperativeHandle(
      ref,
      () => ({
        focus: () => inputRef.current?.focus(),
      }),
      [],
    );

    useLayoutEffect(() => {
      const scroll = scrollRef.current;

      if (!scroll || !stuckRef.current) {
        return;
      }

      scroll.scrollTop = scroll.scrollHeight;
    }, [version, buffer.altScreen]);

    const scrollbackRows = useMemo(() => {
      if (buffer.altScreen) {
        return null;
      }

      const start = Math.max(0, buffer.scrollback.length - MAX_RENDERED_SCROLLBACK);
      const rows: ReactNode[] = [];

      for (let index = start; index < buffer.scrollback.length; index += 1) {
        rows.push(
          <TerminalRow key={`s-${index}`} cells={buffer.scrollback[index]} />,
        );
      }

      return rows;
      // Scrollback cells are cloned when they enter the buffer and never
      // mutated afterwards, so the version counter is the correct dependency.
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [buffer.scrollbackVersion, buffer.altScreen]);

    const screenRows = buffer.grid.map((cells, row) => (
      <TerminalRow
        key={`r-${row}`}
        cells={cells}
        cursorCol={
          row === buffer.cursor.row && buffer.cursorVisible
            ? buffer.cursor.col
            : -1
        }
        cursorStyle={cursorStyle}
      />
    ));

    const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      const data = keyToTerminalData(event.nativeEvent);

      if (data === null) {
        return;
      }

      event.preventDefault();
      onData(data);
    };

    const handleInput = (event: React.FormEvent<HTMLTextAreaElement>) => {
      if (composingRef.current) {
        return;
      }

      const value = event.currentTarget.value;

      if (value) {
        onData(value);
        event.currentTarget.value = '';
      }
    };

    const handlePaste = (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
      const text = event.clipboardData.getData('text');

      if (text) {
        event.preventDefault();
        onData(text);
      }
    };

    const handleScroll = () => {
      const scroll = scrollRef.current;

      if (!scroll) {
        return;
      }

      stuckRef.current =
        scroll.scrollHeight - scroll.scrollTop - scroll.clientHeight < 28;
    };

    const focus = () => {
      inputRef.current?.focus();
    };

    return (
      <div
        ref={shellRef}
        className={cn(
          'terminal-shell relative flex h-full min-h-0 flex-col rounded-2xl border border-border-strong',
          className,
        )}
        style={
          {
            '--terminal-line-height': `${cellHeight}px`,
            fontSize: `${fontSize}px`,
            fontFamily: fontFamily ?? undefined,
          } as CSSProperties
        }
        onClick={focus}
        role="application"
        aria-label="KERNEXT terminal"
      >
        <div className="terminal-scan absolute inset-0 z-0" aria-hidden="true" />
        <div
          ref={scrollRef}
          className="terminal-scroll relative z-[1] min-h-0 flex-1 px-4 py-3"
          onScroll={handleScroll}
        >
          <span
            ref={probeRef}
            aria-hidden="true"
            className="pointer-events-none invisible absolute whitespace-pre font-mono"
            style={{ fontSize: `${fontSize}px`, fontFamily: fontFamily ?? undefined }}
          >
            {'W'.repeat(20)}
          </span>
          <div role="log" aria-live="off">
            {scrollbackRows}
            {screenRows}
          </div>
        </div>
        <textarea
          ref={inputRef}
          className="terminal-keyboard"
          aria-label="Terminal keyboard input"
          autoCapitalize="off"
          autoComplete="off"
          autoCorrect="off"
          spellCheck={false}
          wrap="off"
          onKeyDown={handleKeyDown}
          onInput={handleInput}
          onPaste={handlePaste}
          onCompositionStart={() => {
            composingRef.current = true;
          }}
          onCompositionEnd={(event) => {
            composingRef.current = false;
            const value = event.currentTarget.value;

            if (value) {
              onData(value);
              event.currentTarget.value = '';
            }
          }}
        />
      </div>
    );
  },
);

function TerminalRow({
  cells,
  cursorCol = -1,
  cursorStyle = 'block',
}: {
  cells: Cell[];
  cursorCol?: number;
  cursorStyle?: 'block' | 'bar' | 'underline';
}) {
  const spans: ReactNode[] = [];
  let text = '';
  let signature = '';
  let style: CSSProperties | undefined;
  let cursor = false;
  let start = 0;

  const flush = () => {
    if (!text && !cursor) {
      return;
    }

    spans.push(
      <span
        key={`span-${start}`}
        className={
          cursor
            ? cursorStyle === 'bar'
              ? 'terminal-cursor terminal-cursor-bar'
              : cursorStyle === 'underline'
                ? 'terminal-cursor terminal-cursor-underline'
                : 'terminal-cursor'
            : undefined
        }
        style={style}
      >
        {text || ' '}
      </span>,
    );

    text = '';
    cursor = false;
  };

  for (let col = 0; col < cells.length; col += 1) {
    const cell = cells[col];
    const isCursor = col === cursorCol;
    const character = cell.hidden ? ' ' : cell.ch;

    if (!character && !isCursor) {
      // The second cell of a wide character has no text of its own. The first
      // cell already renders across both columns.
      continue;
    }

    const nextSignature = cellSignature(cell) + (isCursor ? '|c' : '');

    if (nextSignature !== signature) {
      flush();
      signature = nextSignature;
      style = cellStyle(cell);
      start = col;
    }

    cursor = isCursor;
    text += character || ' ';
  }

  flush();

  return <div className="terminal-row">{spans}</div>;
}

function cellSignature(cell: Cell): string {
  return [
    styleKey(cell.fg),
    styleKey(cell.bg),
    cell.bold ? 'b' : '',
    cell.dim ? 'd' : '',
    cell.italic ? 'i' : '',
    cell.underline ? 'u' : '',
    cell.inverse ? 'v' : '',
    cell.hidden ? 'h' : '',
    cell.strikethrough ? 's' : '',
  ].join('|');
}

function cellStyle(cell: Cell): CSSProperties {
  const decorations: string[] = [];

  if (cell.underline) {
    decorations.push('underline');
  }

  if (cell.strikethrough) {
    decorations.push('line-through');
  }

  const base: CSSProperties = {
    whiteSpace: 'pre',
    fontWeight: cell.bold ? 700 : undefined,
    fontStyle: cell.italic ? 'italic' : undefined,
    textDecoration: decorations.length > 0 ? decorations.join(' ') : undefined,
    opacity: cell.dim ? 0.68 : undefined,
  };

  if (cell.inverse) {
    return {
      ...base,
      color: colorToCss(cell.bg, 'background') ?? 'var(--term-bg)',
      backgroundColor: colorToCss(cell.fg, 'foreground'),
    };
  }

  return {
    ...base,
    color: colorToCss(cell.fg, 'foreground'),
    backgroundColor: colorToCss(cell.bg, 'background'),
  };
}
