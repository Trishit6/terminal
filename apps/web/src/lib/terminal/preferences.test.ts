import { describe, expect, it } from 'vitest';
import {
  DEFAULT_TERMINAL_PREFERENCES,
  parseTerminalPreferences,
} from './preferences';

describe('parseTerminalPreferences', () => {
  it('rejects malformed and outdated preference records', () => {
    expect(parseTerminalPreferences(null)).toBeNull();
    expect(
      parseTerminalPreferences({
        version: 2,
        preferences: DEFAULT_TERMINAL_PREFERENCES,
      }),
    ).toBeNull();
  });

  it('clamps numeric settings and falls back from unknown enum values', () => {
    expect(
      parseTerminalPreferences({
        width: 20000,
        height: 10,
        fontSize: 100,
        fontFamily: 'not-a-font',
        lineHeight: 4,
        padding: 2,
        theme: 'not-a-theme',
        cursorStyle: 'not-a-cursor',
        opacity: 0,
      }),
    ).toEqual({
      ...DEFAULT_TERMINAL_PREFERENCES,
      width: 10000,
      height: 280,
      fontSize: 20,
      lineHeight: 1.8,
      padding: 8,
      opacity: 0.75,
    });
  });

  it('reads the versioned account preference envelope', () => {
    const saved = {
      ...DEFAULT_TERMINAL_PREFERENCES,
      theme: 'midnight-blue',
      height: 720,
    };

    expect(
      parseTerminalPreferences({ version: 1, preferences: saved }),
    ).toEqual(saved);
  });
});
