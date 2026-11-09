import {
  applyStyle,
  blankCell,
  blankRow,
  charWidth,
  cloneCell,
  defaultStyle,
  type Cell,
  type CursorPosition,
  type StyleState,
} from './types';

interface MainScreenState {
  grid: Cell[][];
  scrollback: Cell[][];
  cursor: CursorPosition;
  savedCursor: CursorPosition;
  scrollTop: number;
  scrollBottom: number;
}

const MAX_SCROLLBACK = 1000;

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(Math.max(value, minimum), maximum);
}

/**
 * A small VT-style screen buffer.
 *
 * This is not a complete terminal emulator, but it implements the sequences a
 * modern line editor and full-screen program actually use: cursor addressing,
 * erase/insert/delete, scroll regions, SGR colours, alternate screen, and
 * scrollback. The renderer is intentionally separate so the parser stays
 * testable without a DOM.
 */
export class TerminalBuffer {
  rows: number;
  cols: number;
  grid: Cell[][];
  scrollback: Cell[][] = [];
  cursor: CursorPosition = { row: 0, col: 0 };
  savedCursor: CursorPosition = { row: 0, col: 0 };
  scrollTop = 0;
  scrollBottom: number;
  wrapPending = false;
  cursorVisible = true;
  bracketedPaste = false;
  altScreen = false;
  title = '';
  style: StyleState = defaultStyle();
  scrollbackVersion = 0;

  private lastChar = ' ';
  private mainScreen: MainScreenState | null = null;

  constructor(rows = 24, cols = 80) {
    this.rows = Math.max(2, rows);
    this.cols = Math.max(2, cols);
    this.scrollBottom = this.rows - 1;
    this.grid = Array.from({ length: this.rows }, () => blankRow(this.cols));
  }

  resize(rows: number, cols: number): void {
    rows = Math.max(2, rows);
    cols = Math.max(2, cols);

    if (rows === this.rows && cols === this.cols) {
      return;
    }

    const oldRows = this.rows;
    const oldCols = this.cols;
    const oldGrid = this.grid;
    const offset = rows < oldRows ? oldRows - rows : 0;

    if (offset > 0 && !this.altScreen) {
      const removed = oldGrid
        .slice(0, offset)
        .map((row) => row.map(cloneCell));
      this.pushScrollback(removed);
    }

    const next = Array.from({ length: rows }, () => blankRow(cols));

    for (let row = 0; row < rows; row += 1) {
      const sourceRow = oldGrid[row + offset];

      if (!sourceRow) {
        continue;
      }

      const copyColumns = Math.min(cols, oldCols);

      for (let col = 0; col < copyColumns; col += 1) {
        next[row][col] = cloneCell(sourceRow[col]);
      }
    }

    this.rows = rows;
    this.cols = cols;
    this.grid = next;
    this.scrollTop = 0;
    this.scrollBottom = rows - 1;
    this.cursor.row = clamp(this.cursor.row - offset, 0, rows - 1);
    this.cursor.col = clamp(this.cursor.col, 0, cols - 1);
    this.savedCursor.row = clamp(this.savedCursor.row - offset, 0, rows - 1);
    this.savedCursor.col = clamp(this.savedCursor.col, 0, cols - 1);
    this.wrapPending = false;
  }

  write(character: string): void {
    if (!character) {
      return;
    }

    const width = charWidth(character);

    if (width === 0) {
      this.appendCombining(character);
      return;
    }

    if (this.wrapPending) {
      this.cursor.col = 0;
      this.lineFeed();
    }

    if (this.cursor.col + width > this.cols) {
      this.cursor.col = 0;
      this.lineFeed();
    }

    const cell = this.grid[this.cursor.row]?.[this.cursor.col];

    if (!cell) {
      return;
    }

    cell.ch = character;
    applyStyle(cell, this.style);

    if (width === 2) {
      const second = this.grid[this.cursor.row]?.[this.cursor.col + 1];

      if (second) {
        second.ch = '';
        applyStyle(second, this.style);
      }
    }

    this.lastChar = character;
    this.cursor.col += width;

    if (this.cursor.col >= this.cols) {
      this.cursor.col = this.cols - 1;
      this.wrapPending = true;
    }
  }

  private appendCombining(character: string): void {
    let row = this.cursor.row;
    let col = this.cursor.col - 1;

    if (col < 0) {
      row -= 1;
      col = this.cols - 1;
    }

    if (row < 0) {
      return;
    }

    const cell = this.grid[row]?.[col];

    if (cell) {
      cell.ch += character;
    }
  }

  repeatLastCharacter(count: number): void {
    const total = Math.max(1, count);

    for (let index = 0; index < total; index += 1) {
      this.write(this.lastChar);
    }
  }

  carriageReturn(): void {
    this.cursor.col = 0;
    this.wrapPending = false;
  }

  lineFeed(): void {
    this.wrapPending = false;

    if (this.cursor.row === this.scrollBottom) {
      this.scrollUp(1);
      return;
    }

    if (this.cursor.row < this.rows - 1) {
      this.cursor.row += 1;
    }
  }

  reverseIndex(): void {
    this.wrapPending = false;

    if (this.cursor.row === this.scrollTop) {
      this.scrollDown(1);
      return;
    }

    if (this.cursor.row > 0) {
      this.cursor.row -= 1;
    }
  }

  backspace(): void {
    this.wrapPending = false;

    if (this.cursor.col > 0) {
      this.cursor.col -= 1;
    }
  }

  tab(count = 1): void {
    const total = Math.max(1, count);

    for (let index = 0; index < total; index += 1) {
      const nextStop = Math.min(
        this.cols - 1,
        (Math.floor(this.cursor.col / 8) + 1) * 8,
      );

      if (nextStop === this.cursor.col) {
        this.cursor.col = 0;
        this.lineFeed();
      } else {
        this.cursor.col = nextStop;
      }
    }

    this.wrapPending = false;
  }

  backTab(count = 1): void {
    const total = Math.max(1, count);

    for (let index = 0; index < total; index += 1) {
      this.cursor.col = Math.max(
        0,
        (Math.ceil(this.cursor.col / 8) - 1) * 8,
      );
    }

    this.wrapPending = false;
  }

  moveCursor(deltaX: number, deltaY: number): void {
    this.cursor.row = clamp(
      this.cursor.row + deltaY,
      this.scrollTop,
      this.scrollBottom,
    );
    this.cursor.col = clamp(this.cursor.col + deltaX, 0, this.cols - 1);
    this.wrapPending = false;
  }

  setCursor(row: number, col: number): void {
    this.cursor.row = clamp(
      this.scrollTop + row,
      this.scrollTop,
      this.scrollBottom,
    );
    this.cursor.col = clamp(col, 0, this.cols - 1);
    this.wrapPending = false;
  }

  setCursorColumn(col: number): void {
    this.cursor.col = clamp(col, 0, this.cols - 1);
    this.wrapPending = false;
  }

  saveCursor(): void {
    this.savedCursor = { ...this.cursor };
  }

  restoreCursor(): void {
    this.cursor = { ...this.savedCursor };
    this.wrapPending = false;
  }

  setScrollRegion(top: number, bottom: number): void {
    const nextTop = clamp(top, 0, this.rows - 1);
    const nextBottom = clamp(bottom, 0, this.rows - 1);

    if (nextTop >= nextBottom) {
      this.scrollTop = 0;
      this.scrollBottom = this.rows - 1;
    } else {
      this.scrollTop = nextTop;
      this.scrollBottom = nextBottom;
    }

    this.cursor.row = this.scrollTop;
    this.cursor.col = 0;
    this.wrapPending = false;
  }

  scrollUp(count = 1): void {
    const bottom = this.scrollBottom;
    const top = this.scrollTop;
    const total = Math.min(
      Math.max(1, count),
      Math.max(1, bottom - top + 1),
    );
    const removed: Cell[][] = [];

    for (let index = 0; index < total; index += 1) {
      const row = this.grid.splice(top, 1)[0];

      if (!row) {
        break;
      }

      if (!this.altScreen && top === 0) {
        removed.push(row.map(cloneCell));
      }

      this.grid.splice(bottom, 0, blankRow(this.cols));
    }

    if (removed.length > 0) {
      this.pushScrollback(removed);
    }
  }

  scrollDown(count = 1): void {
    const bottom = this.scrollBottom;
    const top = this.scrollTop;
    const total = Math.min(
      Math.max(1, count),
      Math.max(1, bottom - top + 1),
    );

    for (let index = 0; index < total; index += 1) {
      this.grid.splice(bottom, 1);
      this.grid.splice(top, 0, blankRow(this.cols));
    }
  }

  eraseInDisplay(mode: number): void {
    switch (mode) {
      case 0: {
        this.eraseInLine(0);

        for (let row = this.cursor.row + 1; row < this.rows; row += 1) {
          this.grid[row] = blankRow(this.cols);
        }
        break;
      }
      case 1: {
        for (let row = 0; row < this.cursor.row; row += 1) {
          this.grid[row] = blankRow(this.cols);
        }

        this.eraseInLine(1);
        break;
      }
      case 2: {
        for (let row = 0; row < this.rows; row += 1) {
          this.grid[row] = blankRow(this.cols);
        }
        break;
      }
      case 3: {
        this.scrollback = [];
        this.scrollbackVersion += 1;
        break;
      }
      default:
        break;
    }
  }

  eraseInLine(mode: number): void {
    const row = this.grid[this.cursor.row];

    if (!row) {
      return;
    }

    let start = 0;
    let end = this.cols - 1;

    if (mode === 0) {
      start = this.cursor.col;
    } else if (mode === 1) {
      end = this.cursor.col;
    } else if (mode !== 2) {
      return;
    }

    for (let col = start; col <= end; col += 1) {
      row[col] = blankCell();
    }
  }

  insertChars(count: number): void {
    const row = this.grid[this.cursor.row];

    if (!row) {
      return;
    }

    const total = clamp(count, 1, this.cols - this.cursor.col);
    row.splice(this.cursor.col, 0, ...Array.from({ length: total }, blankCell));
    row.length = this.cols;
    this.wrapPending = false;
  }

  deleteChars(count: number): void {
    const row = this.grid[this.cursor.row];

    if (!row) {
      return;
    }

    const total = clamp(count, 1, this.cols - this.cursor.col);
    row.splice(this.cursor.col, total);
    row.push(...Array.from({ length: total }, blankCell));
    this.wrapPending = false;
  }

  eraseChars(count: number): void {
    const row = this.grid[this.cursor.row];

    if (!row) {
      return;
    }

    const total = clamp(count, 1, this.cols - this.cursor.col);

    for (let col = this.cursor.col; col < this.cursor.col + total; col += 1) {
      row[col] = blankCell();
    }
  }

  insertLines(count: number): void {
    if (
      this.cursor.row < this.scrollTop ||
      this.cursor.row > this.scrollBottom
    ) {
      return;
    }

    const total = Math.min(
      Math.max(1, count),
      this.scrollBottom - this.cursor.row + 1,
    );

    for (let index = 0; index < total; index += 1) {
      this.grid.splice(this.scrollBottom, 1);
      this.grid.splice(this.cursor.row, 0, blankRow(this.cols));
    }

    this.cursor.col = 0;
    this.wrapPending = false;
  }

  deleteLines(count: number): void {
    if (
      this.cursor.row < this.scrollTop ||
      this.cursor.row > this.scrollBottom
    ) {
      return;
    }

    const total = Math.min(
      Math.max(1, count),
      this.scrollBottom - this.cursor.row + 1,
    );

    for (let index = 0; index < total; index += 1) {
      this.grid.splice(this.cursor.row, 1);
      this.grid.splice(this.scrollBottom, 0, blankRow(this.cols));
    }

    this.cursor.col = 0;
    this.wrapPending = false;
  }

  enterAltScreen(): void {
    if (this.altScreen) {
      return;
    }

    this.mainScreen = {
      grid: this.grid,
      scrollback: this.scrollback,
      cursor: { ...this.cursor },
      savedCursor: { ...this.savedCursor },
      scrollTop: this.scrollTop,
      scrollBottom: this.scrollBottom,
    };

    this.altScreen = true;
    this.grid = Array.from({ length: this.rows }, () => blankRow(this.cols));
    this.cursor = { row: 0, col: 0 };
    this.savedCursor = { row: 0, col: 0 };
    this.scrollTop = 0;
    this.scrollBottom = this.rows - 1;
    this.wrapPending = false;
  }

  exitAltScreen(): void {
    if (!this.altScreen) {
      return;
    }

    const main = this.mainScreen;
    this.altScreen = false;

    if (!main) {
      this.reset();
      return;
    }

    this.grid = main.grid;
    this.scrollback = main.scrollback;
    this.cursor = main.cursor;
    this.savedCursor = main.savedCursor;
    this.scrollTop = main.scrollTop;
    this.scrollBottom = main.scrollBottom;
    this.mainScreen = null;
    this.wrapPending = false;
  }

  pushScrollback(rows: Cell[][]): void {
    if (rows.length === 0 || this.altScreen) {
      return;
    }

    for (const row of rows) {
      this.scrollback.push(row);
    }

    if (this.scrollback.length > MAX_SCROLLBACK) {
      this.scrollback.splice(0, this.scrollback.length - MAX_SCROLLBACK);
    }

    this.scrollbackVersion += 1;
  }

  reset(): void {
    this.grid = Array.from({ length: this.rows }, () => blankRow(this.cols));
    this.scrollback = [];
    this.scrollbackVersion += 1;
    this.cursor = { row: 0, col: 0 };
    this.savedCursor = { row: 0, col: 0 };
    this.scrollTop = 0;
    this.scrollBottom = this.rows - 1;
    this.wrapPending = false;
    this.cursorVisible = true;
    this.bracketedPaste = false;
    this.altScreen = false;
    this.mainScreen = null;
    this.title = '';
    this.style = defaultStyle();
    this.lastChar = ' ';
  }

  clear(): void {
    this.grid = Array.from({ length: this.rows }, () => blankRow(this.cols));
    this.scrollback = [];
    this.scrollbackVersion += 1;
    this.cursor = { row: 0, col: 0 };
    this.savedCursor = { row: 0, col: 0 };
    this.scrollTop = 0;
    this.scrollBottom = this.rows - 1;
    this.wrapPending = false;
    this.style = defaultStyle();
    this.lastChar = ' ';
  }

  dumpText(): string {
    const rows = this.altScreen ? this.grid : [...this.scrollback, ...this.grid];

    return rows
      .map((row) =>
        row
          .map((cell) => (cell.hidden ? ' ' : cell.ch || ' '))
          .join('')
          .replace(/\s+$/u, ''),
      )
      .join('\n')
      .replace(/\n+$/u, '');
  }
}
