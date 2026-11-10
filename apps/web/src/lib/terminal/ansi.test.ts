import { describe, expect, it } from 'vitest';
import { AnsiParser } from './ansi';
import { TerminalBuffer } from './buffer';

function setup(rows = 6, cols = 20) {
  const buffer = new TerminalBuffer(rows, cols);
  const responses: string[] = [];
  const parser = new AnsiParser(buffer, (data) => responses.push(data));
  return { buffer, parser, responses };
}

/** Visible screen text, one string per row, trailing blanks trimmed. */
function lines(buffer: TerminalBuffer): string[] {
  return buffer.dumpText().split('\n');
}

describe('AnsiParser', () => {
  it('writes plain text at the cursor', () => {
    const { buffer, parser } = setup();

    parser.write('kernext');

    expect(lines(buffer)[0]).toBe('kernext');
  });

  it('handles carriage return and line feed', () => {
    const { buffer, parser } = setup();

    parser.write('one\r\ntwo');

    expect(lines(buffer)[0]).toBe('one');
    expect(lines(buffer)[1]).toBe('two');
    expect(buffer.cursor).toEqual({ row: 1, col: 3 });
  });

  it('moves the cursor with CSI sequences', () => {
    const { buffer, parser } = setup();

    parser.write('\u001b[3;5H');
    expect(buffer.cursor).toEqual({ row: 2, col: 4 });

    parser.write('\u001b[2A');
    expect(buffer.cursor.row).toBe(0);

    parser.write('\u001b[4B');
    expect(buffer.cursor.row).toBe(4);

    parser.write('\u001b[2C');
    expect(buffer.cursor.col).toBe(6);

    parser.write('\u001b[2D');
    expect(buffer.cursor.col).toBe(4);

    parser.write('\u001b[G');
    expect(buffer.cursor.col).toBe(0);
  });

  it('applies SGR colours including 256-color and truecolor', () => {
    const { buffer, parser } = setup();

    parser.write('\u001b[31m');
    expect(buffer.style.fg).toEqual({ kind: 'indexed', index: 1 });

    parser.write('\u001b[38;5;45m');
    expect(buffer.style.fg).toEqual({ kind: 'indexed', index: 45 });

    parser.write('\u001b[38;2;10;20;30m');
    expect(buffer.style.fg).toEqual({ kind: 'rgb', r: 10, g: 20, b: 30 });

    parser.write('\u001b[48;2;1;2;3m');
    expect(buffer.style.bg).toEqual({ kind: 'rgb', r: 1, g: 2, b: 3 });

    parser.write('\u001b[1m');
    expect(buffer.style.bold).toBe(true);

    parser.write('\u001b[0m');
    expect(buffer.style.fg).toEqual({ kind: 'default' });
    expect(buffer.style.bg).toEqual({ kind: 'default' });
    expect(buffer.style.bold).toBe(false);
  });

  it('erases the display and the current line', () => {
    const { buffer, parser } = setup();

    parser.write('line one\r\nline two');
    expect(lines(buffer)[0]).toBe('line one');

    parser.write('\u001b[2J');
    expect(buffer.dumpText()).toBe('');

    parser.write('partial');
    parser.write('\u001b[G');
    parser.write('\u001b[K');
    expect(lines(buffer)[0]).toBe('');
    expect(buffer.cursor.col).toBe(0);
  });

  it('consumes unknown escape sequences instead of printing them', () => {
    const { buffer, parser } = setup();

    parser.write('a\u001b[?25lb\u001b]0;title\u0007c');

    const text = buffer.dumpText();
    expect(text).toContain('a');
    expect(text).toContain('b');
    expect(text).toContain('c');
    expect(text).not.toContain('[');
    expect(text).not.toContain('title');
  });

  it('records OSC window titles', () => {
    const { buffer, parser } = setup();

    parser.write('\u001b]0;kernext — session\u0007');
    expect(buffer.title).toBe('kernext — session');

    parser.write('\u001b]2;kernext — other\u001b\\');
    expect(buffer.title).toBe('kernext — other');
  });

  it('answers a cursor position report request', () => {
    const { buffer, parser, responses } = setup();

    parser.write('\u001b[2;4H');
    parser.write('\u001b[6n');

    expect(responses).toEqual(['\u001b[2;4R']);
    expect(buffer.cursor).toEqual({ row: 1, col: 3 });
  });

  it('enters and leaves the alternate screen with mode 1049', () => {
    const { buffer, parser } = setup();

    parser.write('main');
    parser.write('\u001b[?1049h');
    expect(buffer.altScreen).toBe(true);
    expect(buffer.dumpText()).toBe('');

    parser.write('alternate');
    expect(lines(buffer)[0]).toBe('alternate');

    parser.write('\u001b[?1049l');
    expect(buffer.altScreen).toBe(false);
    expect(lines(buffer)[0]).toBe('main');
  });

  it('scrolls the display with SU', () => {
    const { buffer, parser } = setup(4, 10);

    parser.write('a\r\nb\r\nc\r\nd');
    parser.write('\u001b[1S');

    // The top row moved into scrollback and the grid scrolled up.
    expect(buffer.scrollback).toHaveLength(1);
    expect(buffer.dumpText().split('\n')).toEqual(['a', 'b', 'c', 'd']);

    const gridText = buffer.grid
      .map((row) =>
        row.map((cell) => cell.ch || ' ').join('').replace(/\s+$/u, ''),
      )
      .join('|');
    expect(gridText).toBe('b|c|d|');
  });

  it('repeats the last character with REP', () => {
    const { buffer, parser } = setup();

    parser.write('x\u001b[3b');

    expect(lines(buffer)[0]).toBe('xxxx');
  });
});
