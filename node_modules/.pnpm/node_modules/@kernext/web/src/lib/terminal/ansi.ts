import type { TerminalBuffer } from './buffer';
import { DEFAULT_COLOR, defaultStyle, type Color } from './types';

type ParserState =
  | 'ground'
  | 'escape'
  | 'csi'
  | 'osc'
  | 'osc_escape'
  | 'string'
  | 'string_escape'
  | 'charset';

function clampByte(value: number): number {
  return Math.min(255, Math.max(0, Math.trunc(value)));
}

function colorIndex(index: number): Color {
  return { kind: 'indexed', index: clampByte(index) };
}

function rgbColor(red: number, green: number, blue: number): Color {
  return {
    kind: 'rgb',
    r: clampByte(red),
    g: clampByte(green),
    b: clampByte(blue),
  };
}

/**
 * Incremental ANSI/VT parser.
 *
 * It targets the subset emitted by KERNEXT, Reedline, crossterm, and common
 * full-screen programs. Unsupported sequences are consumed safely rather than
 * printed as literal escape text.
 */
export class AnsiParser {
  private state: ParserState = 'ground';
  private params = '';
  private intermediates = '';
  private osc = '';

  constructor(
    private readonly buffer: TerminalBuffer,
    private readonly respond: (data: string) => void = () => {},
  ) {}

  write(text: string): void {
    for (const character of text) {
      this.consume(character);
    }
  }

  reset(): void {
    this.state = 'ground';
    this.params = '';
    this.intermediates = '';
    this.osc = '';
  }

  private consume(character: string): void {
    const codePoint = character.codePointAt(0) ?? 0;

    switch (this.state) {
      case 'ground':
        this.consumeGround(character, codePoint);
        break;
      case 'escape':
        this.consumeEscape(character);
        break;
      case 'csi':
        this.consumeCsi(character, codePoint);
        break;
      case 'osc':
        this.consumeOsc(character, codePoint);
        break;
      case 'osc_escape':
        if (character === '\\') {
          this.finishOsc();
        } else {
          this.osc += `\u001b${character}`;
          this.state = 'osc';
        }
        break;
      case 'string':
        if (codePoint === 0x1b) {
          this.state = 'string_escape';
        }
        break;
      case 'string_escape':
        this.state = character === '\\' ? 'ground' : 'string';
        break;
      case 'charset':
        this.state = 'ground';
        break;
    }
  }

  private consumeGround(character: string, codePoint: number): void {
    if (codePoint === 0x1b) {
      this.state = 'escape';
      return;
    }

    switch (codePoint) {
      case 0x0a:
        this.buffer.lineFeed();
        return;
      case 0x0d:
        this.buffer.carriageReturn();
        return;
      case 0x08:
        this.buffer.backspace();
        return;
      case 0x09:
        this.buffer.tab();
        return;
      case 0x07:
      case 0x0e:
      case 0x0f:
        return;
      default:
        break;
    }

    if (codePoint < 0x20 || codePoint === 0x7f) {
      return;
    }

    this.buffer.write(character);
  }

  private consumeEscape(character: string): void {
    switch (character) {
      case '[':
        this.state = 'csi';
        this.params = '';
        this.intermediates = '';
        return;
      case ']':
        this.state = 'osc';
        this.osc = '';
        return;
      case 'P':
      case '^':
      case '_':
        this.state = 'string';
        return;
      case '(':
      case ')':
      case '*':
      case '+':
      case '-':
      case '.':
      case '/':
        this.state = 'charset';
        return;
      case '7':
        this.buffer.saveCursor();
        this.state = 'ground';
        return;
      case '8':
        this.buffer.restoreCursor();
        this.state = 'ground';
        return;
      case 'D':
        this.buffer.lineFeed();
        this.state = 'ground';
        return;
      case 'E':
        this.buffer.carriageReturn();
        this.buffer.lineFeed();
        this.state = 'ground';
        return;
      case 'M':
        this.buffer.reverseIndex();
        this.state = 'ground';
        return;
      case 'c':
        this.buffer.reset();
        this.state = 'ground';
        return;
      case '=':
      case '>':
      case '\\':
      case '~':
        this.state = 'ground';
        return;
      default:
        this.state = 'ground';
    }
  }

  private consumeCsi(character: string, codePoint: number): void {
    if (codePoint >= 0x30 && codePoint <= 0x3f) {
      if (this.params.length < 256) {
        this.params += character;
      }
      return;
    }

    if (codePoint >= 0x20 && codePoint <= 0x2f) {
      if (this.intermediates.length < 8) {
        this.intermediates += character;
      }
      return;
    }

    if (codePoint >= 0x40 && codePoint <= 0x7e) {
      this.dispatchCsi(character);
      this.state = 'ground';
      this.params = '';
      this.intermediates = '';
      return;
    }

    if (codePoint === 0x1b) {
      this.state = 'escape';
      this.params = '';
      this.intermediates = '';
    }
  }

  private consumeOsc(character: string, codePoint: number): void {
    if (codePoint === 0x07) {
      this.finishOsc();
      return;
    }

    if (codePoint === 0x1b) {
      this.state = 'osc_escape';
      return;
    }

    if (this.osc.length < 2048) {
      this.osc += character;
    }
  }

  private finishOsc(): void {
    const separator = this.osc.indexOf(';');
    const command = separator === -1 ? this.osc : this.osc.slice(0, separator);
    const value = separator === -1 ? '' : this.osc.slice(separator + 1);

    if ((command === '0' || command === '2') && value.length > 0) {
      this.buffer.title = value;
    }

    this.osc = '';
    this.state = 'ground';
  }

  private dispatchCsi(final: string): void {
    let raw = this.params;
    let privateMode = false;

    if (raw.startsWith('?')) {
      privateMode = true;
      raw = raw.slice(1);
    } else if (raw.startsWith('>') || raw.startsWith('<') || raw.startsWith('=')) {
      raw = raw.slice(1);
    }

    const numbers = raw
      .replace(/:/g, ';')
      .split(';')
      .map((part) => {
        if (part === '') {
          return 0;
        }

        const parsed = Number.parseInt(part, 10);
        return Number.isFinite(parsed) ? parsed : 0;
      });

    const first = numbers[0] ?? 0;
    const amount = Math.max(1, first || 1);

    switch (final) {
      case 'A':
        this.buffer.moveCursor(0, -amount);
        break;
      case 'B':
        this.buffer.moveCursor(0, amount);
        break;
      case 'C':
        this.buffer.moveCursor(amount, 0);
        break;
      case 'D':
        this.buffer.moveCursor(-amount, 0);
        break;
      case 'E':
        this.buffer.moveCursor(0, amount);
        this.buffer.carriageReturn();
        break;
      case 'F':
        this.buffer.moveCursor(0, -amount);
        this.buffer.carriageReturn();
        break;
      case 'G':
        this.buffer.setCursorColumn(amount - 1);
        break;
      case 'H':
      case 'f':
        this.buffer.setCursor((numbers[0] || 1) - 1, (numbers[1] || 1) - 1);
        break;
      case 'd':
        this.buffer.setCursor((numbers[0] || 1) - 1, this.buffer.cursor.col);
        break;
      case 'J':
        this.buffer.eraseInDisplay(first);
        break;
      case 'K':
        this.buffer.eraseInLine(first);
        break;
      case 'L':
        this.buffer.insertLines(amount);
        break;
      case 'M':
        this.buffer.deleteLines(amount);
        break;
      case 'P':
        this.buffer.deleteChars(amount);
        break;
      case '@':
        this.buffer.insertChars(amount);
        break;
      case 'X':
        this.buffer.eraseChars(amount);
        break;
      case 'S':
        this.buffer.scrollUp(amount);
        break;
      case 'T':
        this.buffer.scrollDown(amount);
        break;
      case 'b':
        this.buffer.repeatLastCharacter(amount);
        break;
      case 'm':
        this.applySgr(numbers);
        break;
      case 'r':
        this.buffer.setScrollRegion(
          (numbers[0] || 1) - 1,
          (numbers[1] || this.buffer.rows) - 1,
        );
        break;
      case 's':
        this.buffer.saveCursor();
        break;
      case 'u':
        this.buffer.restoreCursor();
        break;
      case 'h':
      case 'l':
        this.applyMode(privateMode, numbers, final === 'h');
        break;
      case 'n':
        if (first === 6) {
          this.respond(
            `\u001b[${this.buffer.cursor.row + 1};${this.buffer.cursor.col + 1}R`,
          );
        } else if (first === 5) {
          this.respond('\u001b[0n');
        }
        break;
      case 'c':
        this.respond('\u001b[?1;2c');
        break;
      case 'I':
        this.buffer.tab(amount);
        break;
      case 'Z':
        this.buffer.backTab(amount);
        break;
      case 't':
      case 'q':
      case 'g':
      case 'i':
      case 'p':
      case 'x':
      case 'e':
      case 'a':
      case '`':
      case '|':
      case '}':
      case '~':
        break;
      default:
        break;
    }
  }

  private applyMode(
    privateMode: boolean,
    numbers: number[],
    enabled: boolean,
  ): void {
    if (!privateMode) {
      return;
    }

    for (const mode of numbers) {
      switch (mode) {
        case 25:
          this.buffer.cursorVisible = enabled;
          break;
        case 47:
        case 1047:
        case 1049:
          if (enabled) {
            if (mode === 1049) {
              this.buffer.saveCursor();
            }

            this.buffer.enterAltScreen();
          } else {
            this.buffer.exitAltScreen();

            if (mode === 1049) {
              this.buffer.restoreCursor();
            }
          }
          break;
        case 2004:
          this.buffer.bracketedPaste = enabled;
          break;
        default:
          // Focus reporting, mouse reporting, and application cursor modes are
          // intentionally ignored. The custom terminal does not emulate them
          // yet, and consuming them prevents stray escape text.
          break;
      }
    }
  }

  private applySgr(params: number[]): void {
    const style = { ...this.buffer.style, fg: this.buffer.style.fg, bg: this.buffer.style.bg };

    for (let index = 0; index < params.length; index += 1) {
      const code = params[index] ?? 0;

      switch (code) {
        case 0:
          Object.assign(style, defaultStyle());
          break;
        case 1:
          style.bold = true;
          break;
        case 2:
          style.dim = true;
          break;
        case 3:
          style.italic = true;
          break;
        case 4:
          style.underline = true;
          break;
        case 7:
          style.inverse = true;
          break;
        case 8:
          style.hidden = true;
          break;
        case 9:
          style.strikethrough = true;
          break;
        case 21:
          style.underline = true;
          break;
        case 22:
          style.bold = false;
          style.dim = false;
          break;
        case 23:
          style.italic = false;
          break;
        case 24:
          style.underline = false;
          break;
        case 27:
          style.inverse = false;
          break;
        case 28:
          style.hidden = false;
          break;
        case 29:
          style.strikethrough = false;
          break;
        case 39:
          style.fg = DEFAULT_COLOR;
          break;
        case 49:
          style.bg = DEFAULT_COLOR;
          break;
        case 38:
          if (params[index + 1] === 5) {
            style.fg = colorIndex(params[index + 2] ?? 0);
            index += 2;
          } else if (params[index + 1] === 2) {
            style.fg = rgbColor(
              params[index + 2] ?? 0,
              params[index + 3] ?? 0,
              params[index + 4] ?? 0,
            );
            index += 4;
          }
          break;
        case 48:
          if (params[index + 1] === 5) {
            style.bg = colorIndex(params[index + 2] ?? 0);
            index += 2;
          } else if (params[index + 1] === 2) {
            style.bg = rgbColor(
              params[index + 2] ?? 0,
              params[index + 3] ?? 0,
              params[index + 4] ?? 0,
            );
            index += 4;
          }
          break;
        case 58:
          if (params[index + 1] === 5) {
            index += 2;
          } else if (params[index + 1] === 2) {
            index += 4;
          }
          break;
        case 59:
          break;
        default:
          if (code >= 30 && code <= 37) {
            style.fg = colorIndex(code - 30);
          } else if (code >= 40 && code <= 47) {
            style.bg = colorIndex(code - 40);
          } else if (code >= 90 && code <= 97) {
            style.fg = colorIndex(code - 90 + 8);
          } else if (code >= 100 && code <= 107) {
            style.bg = colorIndex(code - 100 + 8);
          }
          break;
      }
    }

    this.buffer.style = style;
  }
}
