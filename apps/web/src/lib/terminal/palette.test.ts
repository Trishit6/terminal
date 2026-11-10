import { describe, expect, it } from 'vitest';
import { ANSI_PALETTE, colorToCss, indexedColor, styleKey } from './palette';

describe('palette', () => {
  it('exposes sixteen base colours as hex strings', () => {
    expect(ANSI_PALETTE).toHaveLength(16);

    for (const entry of ANSI_PALETTE) {
      expect(entry).toMatch(/^#[0-9a-f]{6}$/u);
    }
  });

  it('maps the first sixteen indexes to the base palette', () => {
    expect(indexedColor(0)).toBe(ANSI_PALETTE[0]);
    expect(indexedColor(6)).toBe(ANSI_PALETTE[6]);
    expect(indexedColor(15)).toBe(ANSI_PALETTE[15]);
  });

  it('builds 256-color cube entries from the standard levels', () => {
    // Index 16 is the first cube entry: 0, 0, 0.
    expect(indexedColor(16)).toBe('rgb(0 0 0)');

    // Index 21 is the first row end: 0, 0, 255.
    expect(indexedColor(21)).toBe('rgb(0 0 255)');

    // Index 196 is pure red in the cube.
    expect(indexedColor(196)).toBe('rgb(255 0 0)');
  });

  it('maps the grayscale ramp above index 231', () => {
    expect(indexedColor(232)).toBe('rgb(8 8 8)');
    expect(indexedColor(255)).toBe('rgb(238 238 238)');
  });

  it('clamps out-of-range indexes safely', () => {
    expect(indexedColor(-1)).toBe(ANSI_PALETTE[0]);
    expect(indexedColor(300)).toMatch(/^rgb\(/u);
  });

  it('converts colors to CSS values with the right fallback', () => {
    expect(colorToCss({ kind: 'default' }, 'foreground')).toBe('var(--term-fg)');
    expect(colorToCss({ kind: 'default' }, 'background')).toBeUndefined();
    expect(colorToCss({ kind: 'indexed', index: 4 }, 'foreground')).toBe(ANSI_PALETTE[4]);
    expect(colorToCss({ kind: 'rgb', r: 1, g: 2, b: 3 }, 'foreground')).toBe('rgb(1 2 3)');
  });

  it('groups identical styles under one key and separates different ones', () => {
    expect(styleKey({ kind: 'default' })).toBe(styleKey({ kind: 'default' }));
    expect(styleKey({ kind: 'indexed', index: 3 })).not.toBe(
      styleKey({ kind: 'indexed', index: 4 }),
    );
    expect(styleKey({ kind: 'rgb', r: 0, g: 0, b: 0 })).not.toBe(styleKey({ kind: 'default' }));
  });
});
