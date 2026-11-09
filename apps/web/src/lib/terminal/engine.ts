import { useEffect, useRef, useSyncExternalStore } from 'react';
import { AnsiParser } from './ansi';
import { TerminalBuffer } from './buffer';

/**
 * React-facing terminal engine.
 *
 * Parsing is batched into an animation frame so a fast PTY stream cannot force
 * one React render per WebSocket frame. The buffer remains the single source of
 * truth; `version` is the external-store snapshot React subscribes to.
 */
export class TerminalEngine {
  readonly buffer: TerminalBuffer;
  private readonly parser: AnsiParser;
  private version = 0;
  private pending = '';
  private frame: number | null = null;
  private lastTitle = '';
  private listeners = new Set<() => void>();
  private responseListeners = new Set<(data: string) => void>();
  private titleListeners = new Set<(title: string) => void>();

  constructor(rows = 24, cols = 80) {
    this.buffer = new TerminalBuffer(rows, cols);
    this.parser = new AnsiParser(this.buffer, (data) => {
      for (const listener of this.responseListeners) {
        listener(data);
      }
    });
  }

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  getSnapshot = (): number => this.version;

  onResponse = (listener: (data: string) => void): (() => void) => {
    this.responseListeners.add(listener);
    return () => {
      this.responseListeners.delete(listener);
    };
  };

  onTitle = (listener: (title: string) => void): (() => void) => {
    this.titleListeners.add(listener);
    return () => {
      this.titleListeners.delete(listener);
    };
  };

  write(data: string): void {
    if (!data) {
      return;
    }

    this.pending += data;

    if (this.frame !== null) {
      return;
    }

    this.frame = window.requestAnimationFrame(() => {
      this.frame = null;
      this.flush();
    });
  }

  writeNow(data: string): void {
    if (!data) {
      return;
    }

    this.pending += data;
    this.cancelFrame();
    this.flush();
  }

  resize(rows: number, cols: number): void {
    this.buffer.resize(rows, cols);
    this.emit();
  }

  clear(): void {
    this.pending = '';
    this.cancelFrame();
    this.buffer.clear();
    this.emit();
  }

  reset(): void {
    this.pending = '';
    this.cancelFrame();
    this.parser.reset();
    this.buffer.reset();
    this.emit();
  }

  getText(): string {
    this.flushNow();
    return this.buffer.dumpText();
  }

  flushNow(): void {
    this.cancelFrame();
    this.flush();
  }

  private flush(): void {
    if (!this.pending) {
      return;
    }

    const data = this.pending;
    this.pending = '';
    this.parser.write(data);
    this.emit();

    if (this.buffer.title !== this.lastTitle) {
      this.lastTitle = this.buffer.title;

      for (const listener of this.titleListeners) {
        listener(this.buffer.title);
      }
    }
  }

  private cancelFrame(): void {
    if (this.frame !== null) {
      window.cancelAnimationFrame(this.frame);
      this.frame = null;
    }
  }

  private emit(): void {
    this.version += 1;

    for (const listener of this.listeners) {
      listener();
    }
  }
}

export interface UseTerminalEngineResult {
  engine: TerminalEngine;
  version: number;
}

export function useTerminalEngine(rows = 24, cols = 80): UseTerminalEngineResult {
  const engineRef = useRef<TerminalEngine | null>(null);

  if (engineRef.current === null) {
    engineRef.current = new TerminalEngine(rows, cols);
  }

  const engine = engineRef.current;
  const version = useSyncExternalStore(
    engine.subscribe,
    engine.getSnapshot,
    engine.getSnapshot,
  );

  useEffect(
    () => () => {
      engine.flushNow();
    },
    [engine],
  );

  return { engine, version };
}
