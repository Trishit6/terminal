import { describe, expect, it } from 'vitest';
import { TerminalBuffer } from './buffer';

/** Feed text one character at a time, the way the parser does. */
function type(buffer: TerminalBuffer, text: string): void {
  for (const character of text) {
    buffer.write(character);
  }
}

/** Move to the start of the next line, like `\r\n` from a program. */
function newline(buffer: TerminalBuffer): void {
  buffer.lineFeed();
  buffer.carriageReturn();
}

/** The visible grid only, without scrollback, trailing blank rows removed. */
function screen(buffer: TerminalBuffer): string[] {
  const rows = buffer.grid.map((row) =>
    row
      .map((cell) => (cell.hidden ? ' ' : cell.ch || ' '))
      .join('')
      .replace(/\s+$/u, ''),
  );

  while (rows.length > 0 && rows.at(-1) === '') {
    rows.pop();
  }

  return rows;
}

describe('TerminalBuffer', () => {
  it('starts with the requested geometry and a blank screen', () => {
    const buffer = new TerminalBuffer(10, 24);

    expect(buffer.rows).toBe(10);
    expect(buffer.cols).toBe(24);
    expect(buffer.dumpText()).toBe('');
    expect(buffer.cursor).toEqual({ row: 0, col: 0 });
  });

  it('never accepts degenerate geometry', () => {
    const buffer = new TerminalBuffer(0, 0);

    expect(buffer.rows).toBeGreaterThanOrEqual(2);
    expect(buffer.cols).toBeGreaterThanOrEqual(2);
  });

  it('writes characters and advances the cursor one cell each', () => {
    const buffer = new TerminalBuffer(4, 10);

    type(buffer, 'abc');

    expect(buffer.cursor.col).toBe(3);
    expect(buffer.dumpText()).toBe('abc');
  });

  it('line feed keeps the column, like a bare LF', () => {
    const buffer = new TerminalBuffer(4, 10);

    type(buffer, 'ab');
    buffer.lineFeed();
    type(buffer, 'cd');

    expect(buffer.cursor).toEqual({ row: 1, col: 4 });
    expect(screen(buffer)).toEqual(['ab', '  cd']);
  });

  it('scrolling at the bottom keeps the scrolled row in scrollback', () => {
    const buffer = new TerminalBuffer(2, 10);

    type(buffer, 'one');
    newline(buffer);
    type(buffer, 'two');
    newline(buffer);
    type(buffer, 'three');

    // The screen holds only the last two lines …
    expect(screen(buffer)).toEqual(['two', 'three']);

    // … but dumpText includes scrollback, so nothing is silently lost.
    expect(buffer.dumpText().split('\n')).toEqual(['one', 'two', 'three']);
    expect(buffer.scrollback).toHaveLength(1);
    expect(buffer.scrollbackVersion).toBeGreaterThan(0);
  });

  it('carriage return returns to column zero without changing rows', () => {
    const buffer = new TerminalBuffer(4, 10);

    type(buffer, 'abc');
    buffer.carriageReturn();

    expect(buffer.cursor).toEqual({ row: 0, col: 0 });

    type(buffer, 'x');
    expect(buffer.dumpText()).toBe('xbc');
  });

  it('backspace moves the cursor left without erasing, as VT100 does', () => {
    const buffer = new TerminalBuffer(4, 10);

    type(buffer, 'abc');
    buffer.backspace();

    expect(buffer.cursor.col).toBe(2);
    expect(buffer.dumpText()).toBe('abc');

    // The classic erase idiom: overwrite with a space, then step back.
    buffer.write(' ');
    buffer.backspace();
    expect(buffer.dumpText()).toBe('ab');
  });

  it('tabs stop at the next multiple of eight', () => {
    const buffer = new TerminalBuffer(4, 20);

    type(buffer, 'ab');
    buffer.tab();

    expect(buffer.cursor.col).toBe(8);
  });

  it('erases only from the cursor to the end of the line', () => {
    const buffer = new TerminalBuffer(4, 10);

    type(buffer, 'abcdef');
    buffer.setCursorColumn(3);
    buffer.eraseInLine(0);

    expect(buffer.dumpText()).toBe('abc');
  });

  it('inserts and deletes characters shifting the rest of the row', () => {
    const buffer = new TerminalBuffer(4, 10);

    type(buffer, 'ace');
    buffer.setCursorColumn(1);
    buffer.insertChars(1);
    type(buffer, 'b');

    expect(buffer.dumpText()).toBe('abce');

    buffer.setCursorColumn(1);
    buffer.deleteChars(1);

    expect(buffer.dumpText()).toBe('ace');
  });

  it('resizes while preserving content', () => {
    const buffer = new TerminalBuffer(4, 10);

    type(buffer, 'hello');

    buffer.resize(6, 12);

    expect(buffer.rows).toBe(6);
    expect(buffer.cols).toBe(12);
    expect(buffer.dumpText()).toBe('hello');
  });

  it('shrinking rows pushes removed lines into scrollback', () => {
    const buffer = new TerminalBuffer(4, 10);

    type(buffer, 'a');
    newline(buffer);
    type(buffer, 'b');
    newline(buffer);
    type(buffer, 'c');
    newline(buffer);
    type(buffer, 'd');

    buffer.resize(2, 10);

    expect(buffer.scrollback).toHaveLength(2);
    expect(screen(buffer)).toEqual(['c', 'd']);
    expect(buffer.dumpText().split('\n').at(-1)).toBe('d');
  });

  it('scroll regions confine scrolling and leave outside rows alone', () => {
    const buffer = new TerminalBuffer(5, 10);

    for (const [index, line] of ['top', 'a', 'b', 'c', 'bot'].entries()) {
      type(buffer, line);

      // No newline after the last line: an extra feed would scroll `top`
      // into scrollback before the region is even set.
      if (index < 4) {
        newline(buffer);
      }
    }

    // Rows 1..3 form the region; `top` (row 0) and `bot` (row 4) stay put.
    buffer.setScrollRegion(1, 3);
    expect(buffer.cursor.row).toBe(1);

    // setCursor addresses rows relative to the region top.
    buffer.setCursor(2, 0);
    expect(buffer.cursor.row).toBe(3);

    buffer.lineFeed();

    const text = screen(buffer);
    expect(text[0]).toBe('top');
    expect(text[1]).toBe('b');
    expect(text[2]).toBe('c');
    expect(text[3]).toBe('');
    expect(text[4]).toBe('bot');
    expect(buffer.scrollback).toHaveLength(0);
  });

  it('save and restore cursor round-trips position', () => {
    const buffer = new TerminalBuffer(6, 10);

    buffer.setCursor(2, 5);
    buffer.saveCursor();
    buffer.setCursor(4, 1);
    buffer.restoreCursor();

    expect(buffer.cursor).toEqual({ row: 2, col: 5 });
  });

  it('reset clears content, scrollback, and cursor state', () => {
    const buffer = new TerminalBuffer(4, 10);

    type(buffer, 'data');
    newline(buffer);
    buffer.reset();

    expect(buffer.dumpText()).toBe('');
    expect(buffer.scrollback).toHaveLength(0);
    expect(buffer.cursor).toEqual({ row: 0, col: 0 });
  });
});
