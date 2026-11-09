const FUNCTION_KEYS: Record<string, string> = {
  F1: '\u001bOP',
  F2: '\u001bOQ',
  F3: '\u001bOR',
  F4: '\u001bOS',
  F5: '\u001b[15~',
  F6: '\u001b[17~',
  F7: '\u001b[18~',
  F8: '\u001b[19~',
  F9: '\u001b[20~',
  F10: '\u001b[21~',
  F11: '\u001b[23~',
  F12: '\u001b[24~',
};

const ARROW_SUFFIX: Record<string, string> = {
  ArrowUp: 'A',
  ArrowDown: 'B',
  ArrowRight: 'C',
  ArrowLeft: 'D',
  Home: 'H',
  End: 'F',
};

/**
 * Translates a DOM keyboard event into the byte sequence a terminal program
 * expects. Returns `null` when the browser should keep the key (for example
 * Ctrl+Shift+C for copying).
 */
export function keyToTerminalData(
  event: Pick<
    KeyboardEvent,
    | 'key'
    | 'ctrlKey'
    | 'altKey'
    | 'metaKey'
    | 'shiftKey'
    | 'isComposing'
    | 'keyCode'
  >,
): string | null {
  if (event.isComposing || event.keyCode === 229) {
    return null;
  }

  // Cmd/Meta belongs to the browser and operating system.
  if (event.metaKey) {
    return null;
  }

  // Ctrl+Shift+C/V are the terminal-friendly copy/paste shortcuts. Let the
  // browser fire the copy and paste events instead of sending control bytes.
  if (event.ctrlKey && event.shiftKey) {
    const key = event.key.toLowerCase();

    if (key === 'c' || key === 'v') {
      return null;
    }
  }

  if (event.ctrlKey) {
    const modification = modifierSequence(event);

    if (modification !== null) {
      return modification;
    }

    if (event.key === 'Enter') {
      return '\r';
    }

    if (event.key === 'Backspace') {
      return '\u0008';
    }

    if (event.key.length === 1) {
      const upper = event.key.toUpperCase();
      const code = upper.charCodeAt(0);

      if (code >= 64 && code <= 95) {
        return String.fromCharCode(code - 64);
      }
    }

    return null;
  }

  if (event.altKey) {
    const modification = modifierSequence(event);

    if (modification !== null) {
      return modification;
    }

    if (event.key === 'Enter') {
      return '\u001b\r';
    }

    if (event.key === 'Backspace') {
      return '\u001b\u007f';
    }

    if (event.key.length === 1) {
      return `\u001b${event.key}`;
    }

    return null;
  }

  if (event.shiftKey && event.key === 'Tab') {
    return '\u001b[Z';
  }

  switch (event.key) {
    case 'Enter':
      return '\r';
    case 'Backspace':
      return '\u007f';
    case 'Tab':
      return '\t';
    case 'Escape':
      return '\u001b';
    case 'ArrowUp':
      return '\u001b[A';
    case 'ArrowDown':
      return '\u001b[B';
    case 'ArrowRight':
      return '\u001b[C';
    case 'ArrowLeft':
      return '\u001b[D';
    case 'Home':
      return '\u001b[H';
    case 'End':
      return '\u001b[F';
    case 'Insert':
      return '\u001b[2~';
    case 'Delete':
      return '\u001b[3~';
    case 'PageUp':
      return '\u001b[5~';
    case 'PageDown':
      return '\u001b[6~';
    case 'Dead':
    case 'Process':
    case 'Unidentified':
      return null;
    default:
      break;
  }

  if (FUNCTION_KEYS[event.key]) {
    return FUNCTION_KEYS[event.key];
  }

  if (event.key.length === 1) {
    return event.key;
  }

  return null;
}

function modifierSequence(
  event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'altKey' | 'shiftKey'>,
): string | null {
  const suffix = ARROW_SUFFIX[event.key];

  if (suffix) {
    return `\u001b[1;${modifierNumber(event)}${suffix}`;
  }

  if (event.key === 'Delete') {
    return `\u001b[3;${modifierNumber(event)}~`;
  }

  if (event.key === 'Insert') {
    return `\u001b[2;${modifierNumber(event)}~`;
  }

  if (event.key === 'PageUp') {
    return `\u001b[5;${modifierNumber(event)}~`;
  }

  if (event.key === 'PageDown') {
    return `\u001b[6;${modifierNumber(event)}~`;
  }

  return null;
}

function modifierNumber(
  event: Pick<KeyboardEvent, 'ctrlKey' | 'altKey' | 'shiftKey'>,
): number {
  let modifier = 1;

  if (event.shiftKey) {
    modifier += 1;
  }

  if (event.altKey) {
    modifier += 2;
  }

  if (event.ctrlKey) {
    modifier += 4;
  }

  return modifier;
}
