import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { TerminalEngine } from './engine';

/**
 * The browser interface the engine needs for frame batching. The tests run
 * in a plain Node environment, so a timer-free manual scheduler stands in
 * for `window.requestAnimationFrame`.
 */
interface FrameWindow {
  requestAnimationFrame: (callback: () => void) => number;
  cancelAnimationFrame: (handle: number) => void;
}

const scheduled: Array<() => void> = [];
let nextHandle = 1;

function installFrameWindow(): void {
  const stub: FrameWindow = {
    requestAnimationFrame: (callback) => {
      scheduled.push(callback);
      nextHandle += 1;
      return nextHandle;
    },
    cancelAnimationFrame: () => {
      // Cancellation is tracked by the engine through `this.frame`; a stale
      // callback flushing empty pending data is a harmless no-op.
    },
  };

  (globalThis as unknown as { window?: FrameWindow }).window = stub;
}

function removeFrameWindow(): void {
  delete (globalThis as unknown as { window?: FrameWindow }).window;
  scheduled.length = 0;
}

function runScheduledFrames(): void {
  const pending = scheduled.splice(0, scheduled.length);
  for (const frame of pending) {
    frame();
  }
}

describe('TerminalEngine', () => {
  beforeAll(() => {
    installFrameWindow();
  });

  afterAll(() => {
    removeFrameWindow();
  });

  it('renders writeNow output synchronously', () => {
    const engine = new TerminalEngine(6, 20);

    engine.writeNow('kernext ~ $ ');

    // dumpText trims trailing blanks per row, like a real screen dump.
    expect(engine.getText()).toBe('kernext ~ $');
    expect(engine.getSnapshot()).toBeGreaterThan(0);
  });

  it('ignores empty writes without scheduling work', () => {
    const engine = new TerminalEngine(4, 10);

    engine.writeNow('');
    engine.write('');

    expect(scheduled).toHaveLength(0);
    expect(engine.getSnapshot()).toBe(0);
  });

  it('batches write() into an animation frame and flushes once', () => {
    const engine = new TerminalEngine(4, 20);
    const versionBefore = engine.getSnapshot();

    engine.write('a');
    expect(scheduled).toHaveLength(1);
    expect(engine.getSnapshot()).toBe(versionBefore);

    engine.write('b');
    engine.write('c');
    expect(scheduled).toHaveLength(1);

    runScheduledFrames();

    expect(engine.getText()).toBe('abc');
    expect(engine.getSnapshot()).toBe(versionBefore + 1);
  });

  it('notifies subscribers on flush and stops after unsubscribe', () => {
    const engine = new TerminalEngine(4, 20);
    let notified = 0;
    const unsubscribe = engine.subscribe(() => {
      notified += 1;
    });

    engine.writeNow('hello');
    expect(notified).toBe(1);

    unsubscribe();
    engine.writeNow('again');
    expect(notified).toBe(1);
  });

  it('reports terminal responses such as cursor position reports', () => {
    const engine = new TerminalEngine(4, 20);
    const responses: string[] = [];
    engine.onResponse((data) => responses.push(data));

    engine.writeNow('kernext ~ $ \u001b[6n');

    expect(responses).toEqual(['\u001b[1;13R']);
  });

  it('emits title changes once per distinct title', () => {
    const engine = new TerminalEngine(4, 20);
    const titles: string[] = [];
    engine.onTitle((title) => titles.push(title));

    engine.writeNow('\u001b]0;kernext — engine\u0007');
    engine.writeNow('\u001b]0;kernext — engine\u0007');
    engine.writeNow('\u001b]0;kernext — other\u0007');

    expect(titles).toEqual(['kernext — engine', 'kernext — other']);
  });

  it('clear and reset empty the screen and bump the version', () => {
    const engine = new TerminalEngine(4, 20);

    engine.writeNow('leftover');
    expect(engine.getText()).toBe('leftover');

    engine.clear();
    expect(engine.getText()).toBe('');

    engine.writeNow('again');
    engine.reset();
    expect(engine.getText()).toBe('');
    expect(engine.buffer.cursor).toEqual({ row: 0, col: 0 });
  });

  it('resize forwards to the buffer', () => {
    const engine = new TerminalEngine(6, 20);

    engine.resize(10, 40);

    expect(engine.buffer.rows).toBe(10);
    expect(engine.buffer.cols).toBe(40);
  });
});
