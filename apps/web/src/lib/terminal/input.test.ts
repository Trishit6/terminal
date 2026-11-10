import { describe, expect, it } from 'vitest';
import { keyToTerminalData } from './input';

type EventInit = Pick<
  KeyboardEvent,
  'key' | 'ctrlKey' | 'altKey' | 'metaKey' | 'shiftKey' | 'isComposing' | 'keyCode'
>;

function event(init: Partial<EventInit>): EventInit {
  return {
    key: '',
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    shiftKey: false,
    isComposing: false,
    keyCode: 0,
    ...init,
  };
}

describe('keyToTerminalData', () => {
  it('maps printable characters to themselves', () => {
    expect(keyToTerminalData(event({ key: 'a' }))).toBe('a');
    expect(keyToTerminalData(event({ key: 'Z' }))).toBe('Z');
    expect(keyToTerminalData(event({ key: ' ' }))).toBe(' ');
  });

  it('maps Enter, Backspace, Tab, and Escape', () => {
    expect(keyToTerminalData(event({ key: 'Enter' }))).toBe('\r');
    expect(keyToTerminalData(event({ key: 'Backspace' }))).toBe('\u007f');
    expect(keyToTerminalData(event({ key: 'Tab' }))).toBe('\t');
    expect(keyToTerminalData(event({ key: 'Escape' }))).toBe('\u001b');
  });

  it('maps arrow keys to their escape sequences', () => {
    expect(keyToTerminalData(event({ key: 'ArrowUp' }))).toBe('\u001b[A');
    expect(keyToTerminalData(event({ key: 'ArrowDown' }))).toBe('\u001b[B');
    expect(keyToTerminalData(event({ key: 'ArrowRight' }))).toBe('\u001b[C');
    expect(keyToTerminalData(event({ key: 'ArrowLeft' }))).toBe('\u001b[D');
  });

  it('maps Home, End, Insert, Delete, Page Up, and Page Down', () => {
    expect(keyToTerminalData(event({ key: 'Home' }))).toBe('\u001b[H');
    expect(keyToTerminalData(event({ key: 'End' }))).toBe('\u001b[F');
    expect(keyToTerminalData(event({ key: 'Insert' }))).toBe('\u001b[2~');
    expect(keyToTerminalData(event({ key: 'Delete' }))).toBe('\u001b[3~');
    expect(keyToTerminalData(event({ key: 'PageUp' }))).toBe('\u001b[5~');
    expect(keyToTerminalData(event({ key: 'PageDown' }))).toBe('\u001b[6~');
  });

  it('maps function keys', () => {
    expect(keyToTerminalData(event({ key: 'F1' }))).toBe('\u001bOP');
    expect(keyToTerminalData(event({ key: 'F5' }))).toBe('\u001b[15~');
    expect(keyToTerminalData(event({ key: 'F12' }))).toBe('\u001b[24~');
  });

  it('maps Ctrl combinations to control bytes', () => {
    expect(keyToTerminalData(event({ key: 'c', ctrlKey: true }))).toBe('\u0003');
    expect(keyToTerminalData(event({ key: 'l', ctrlKey: true }))).toBe('\u000c');
    expect(keyToTerminalData(event({ key: 'd', ctrlKey: true }))).toBe('\u0004');
    expect(keyToTerminalData(event({ key: 'Enter', ctrlKey: true }))).toBe('\r');
    expect(keyToTerminalData(event({ key: 'Backspace', ctrlKey: true }))).toBe('\u0008');
  });

  it('keeps Ctrl+Shift+C and Ctrl+Shift+V for the browser', () => {
    expect(keyToTerminalData(event({ key: 'c', ctrlKey: true, shiftKey: true }))).toBeNull();
    expect(keyToTerminalData(event({ key: 'v', ctrlKey: true, shiftKey: true }))).toBeNull();
  });

  it('ignores Meta keys and composition events', () => {
    expect(keyToTerminalData(event({ key: 'r', metaKey: true }))).toBeNull();
    expect(keyToTerminalData(event({ key: 'a', isComposing: true }))).toBeNull();
    expect(keyToTerminalData(event({ key: 'a', keyCode: 229 }))).toBeNull();
  });

  it('maps Alt-modified single characters with an escape prefix', () => {
    expect(keyToTerminalData(event({ key: 'a', altKey: true }))).toBe('\u001ba');
    expect(keyToTerminalData(event({ key: 'Enter', altKey: true }))).toBe('\u001b\r');
    expect(keyToTerminalData(event({ key: 'Backspace', altKey: true }))).toBe('\u001b\u007f');
  });

  it('maps Shift+Tab to the reverse tab sequence', () => {
    expect(keyToTerminalData(event({ key: 'Tab', shiftKey: true }))).toBe('\u001b[Z');
  });

  it('returns null for keys the terminal does not own', () => {
    expect(keyToTerminalData(event({ key: 'Shift' }))).toBeNull();
    expect(keyToTerminalData(event({ key: 'Control' }))).toBeNull();
    expect(keyToTerminalData(event({ key: 'Unidentified' }))).toBeNull();
  });
});
