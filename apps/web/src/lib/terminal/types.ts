export type Color =
  | { kind: 'default' }
  | { kind: 'indexed'; index: number }
  | { kind: 'rgb'; r: number; g: number; b: number };

export const DEFAULT_COLOR: Color = { kind: 'default' };

export interface Cell {
  ch: string;
  fg: Color;
  bg: Color;
  bold: boolean;
  dim: boolean;
  italic: boolean;
  underline: boolean;
  inverse: boolean;
  hidden: boolean;
  strikethrough: boolean;
}

export interface StyleState {
  fg: Color;
  bg: Color;
  bold: boolean;
  dim: boolean;
  italic: boolean;
  underline: boolean;
  inverse: boolean;
  hidden: boolean;
  strikethrough: boolean;
}

export interface CursorPosition {
  row: number;
  col: number;
}

export function defaultStyle(): StyleState {
  return {
    fg: DEFAULT_COLOR,
    bg: DEFAULT_COLOR,
    bold: false,
    dim: false,
    italic: false,
    underline: false,
    inverse: false,
    hidden: false,
    strikethrough: false,
  };
}

export function blankCell(): Cell {
  return {
    ch: ' ',
    fg: DEFAULT_COLOR,
    bg: DEFAULT_COLOR,
    bold: false,
    dim: false,
    italic: false,
    underline: false,
    inverse: false,
    hidden: false,
    strikethrough: false,
  };
}

export function cloneCell(cell: Cell): Cell {
  return {
    ...cell,
    // Colors are immutable value objects; sharing them is safe and avoids
    // allocating a new object for every cell of scrollback.
    fg: cell.fg,
    bg: cell.bg,
  };
}

export function blankRow(columns: number): Cell[] {
  return Array.from({ length: columns }, blankCell);
}

export function applyStyle(cell: Cell, style: StyleState): void {
  cell.fg = style.fg;
  cell.bg = style.bg;
  cell.bold = style.bold;
  cell.dim = style.dim;
  cell.italic = style.italic;
  cell.underline = style.underline;
  cell.inverse = style.inverse;
  cell.hidden = style.hidden;
  cell.strikethrough = style.strikethrough;
}

export function isCombining(codePoint: number): boolean {
  return (
    (codePoint >= 0x0300 && codePoint <= 0x036f) ||
    (codePoint >= 0x0483 && codePoint <= 0x0489) ||
    (codePoint >= 0x0591 && codePoint <= 0x05bd) ||
    (codePoint >= 0x0610 && codePoint <= 0x061a) ||
    (codePoint >= 0x064b && codePoint <= 0x065f) ||
    (codePoint >= 0x0670 && codePoint <= 0x0670) ||
    (codePoint >= 0x06d6 && codePoint <= 0x06ed) ||
    (codePoint >= 0x0711 && codePoint <= 0x0711) ||
    (codePoint >= 0x0730 && codePoint <= 0x074a) ||
    (codePoint >= 0x07a6 && codePoint <= 0x07b0) ||
    (codePoint >= 0x07eb && codePoint <= 0x07f3) ||
    (codePoint >= 0x0816 && codePoint <= 0x082d) ||
    (codePoint >= 0x0859 && codePoint <= 0x085b) ||
    (codePoint >= 0x08d3 && codePoint <= 0x08e1) ||
    (codePoint >= 0x08e3 && codePoint <= 0x0902) ||
    (codePoint >= 0x093a && codePoint <= 0x093c) ||
    (codePoint >= 0x0941 && codePoint <= 0x0948) ||
    (codePoint >= 0x094d && codePoint <= 0x094d) ||
    (codePoint >= 0x0951 && codePoint <= 0x0957) ||
    (codePoint >= 0x0962 && codePoint <= 0x0963) ||
    (codePoint >= 0x1ab0 && codePoint <= 0x1aff) ||
    (codePoint >= 0x1dc0 && codePoint <= 0x1dff) ||
    (codePoint >= 0x20d0 && codePoint <= 0x20ff) ||
    (codePoint >= 0xfe00 && codePoint <= 0xfe0f) ||
    (codePoint >= 0xfe20 && codePoint <= 0xfe2f) ||
    (codePoint >= 0xe0100 && codePoint <= 0xe01ef)
  );
}

export function isWide(codePoint: number): boolean {
  return (
    (codePoint >= 0x1100 && codePoint <= 0x115f) ||
    (codePoint >= 0x2329 && codePoint <= 0x232a) ||
    (codePoint >= 0x2e80 && codePoint <= 0x303e) ||
    (codePoint >= 0x3041 && codePoint <= 0x33ff) ||
    (codePoint >= 0x3400 && codePoint <= 0x4dbf) ||
    (codePoint >= 0x4e00 && codePoint <= 0x9fff) ||
    (codePoint >= 0xa000 && codePoint <= 0xa4cf) ||
    (codePoint >= 0xa960 && codePoint <= 0xa97f) ||
    (codePoint >= 0xac00 && codePoint <= 0xd7a3) ||
    (codePoint >= 0xf900 && codePoint <= 0xfaff) ||
    (codePoint >= 0xfe10 && codePoint <= 0xfe19) ||
    (codePoint >= 0xfe30 && codePoint <= 0xfe6f) ||
    (codePoint >= 0xff00 && codePoint <= 0xff60) ||
    (codePoint >= 0xffe0 && codePoint <= 0xffe6) ||
    (codePoint >= 0x1f000 && codePoint <= 0x1f9ff) ||
    (codePoint >= 0x1fa70 && codePoint <= 0x1faff) ||
    (codePoint >= 0x20000 && codePoint <= 0x3fffd)
  );
}

export function charWidth(character: string): number {
  if (!character) {
    return 0;
  }

  const codePoint = character.codePointAt(0);

  if (codePoint === undefined) {
    return 1;
  }

  if (codePoint === 0 || (codePoint < 32 && codePoint !== 9)) {
    return 0;
  }

  if (codePoint >= 0x7f && codePoint < 0xa0) {
    return 0;
  }

  if (isCombining(codePoint)) {
    return 0;
  }

  return isWide(codePoint) ? 2 : 1;
}
