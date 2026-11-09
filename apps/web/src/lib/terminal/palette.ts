import type { Color } from './types';

/** ANSI 16-colour palette tuned to the KERNEXT cyan/indigo identity. */
export const ANSI_PALETTE = [
  '#0b111b',
  '#ff6b6b',
  '#5ee6a8',
  '#f5d76e',
  '#5bb8ff',
  '#c792ea',
  '#67e8f9',
  '#d8e6f5',
  '#58687e',
  '#ff8787',
  '#7cf0bc',
  '#ffe08a',
  '#82ccff',
  '#d8a8ff',
  '#9df3ff',
  '#ffffff',
] as const;

const CUBE_LEVELS = [0, 95, 135, 175, 215, 255] as const;

export function indexedColor(index: number): string {
  if (index < 0) {
    return ANSI_PALETTE[0];
  }

  if (index < 16) {
    return ANSI_PALETTE[index] ?? ANSI_PALETTE[0];
  }

  if (index < 232) {
    const offset = index - 16;
    const red = CUBE_LEVELS[Math.floor(offset / 36)] ?? 0;
    const green = CUBE_LEVELS[Math.floor((offset % 36) / 6)] ?? 0;
    const blue = CUBE_LEVELS[offset % 6] ?? 0;
    return `rgb(${red} ${green} ${blue})`;
  }

  const grey = 8 + (index - 232) * 10;
  return `rgb(${grey} ${grey} ${grey})`;
}

export function colorToCss(color: Color, fallback: 'foreground' | 'background'): string | undefined {
  switch (color.kind) {
    case 'default':
      return fallback === 'foreground' ? 'var(--term-fg)' : undefined;
    case 'indexed':
      return indexedColor(color.index);
    case 'rgb':
      return `rgb(${color.r} ${color.g} ${color.b})`;
  }
}

/** A stable key for grouping adjacent cells with identical styling. */
export function styleKey(color: Color): string {
  switch (color.kind) {
    case 'default':
      return 'd';
    case 'indexed':
      return `i${color.index}`;
    case 'rgb':
      return `r${color.r},${color.g},${color.b}`;
  }
}
