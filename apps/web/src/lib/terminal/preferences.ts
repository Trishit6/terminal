export type TerminalThemeId =
  | 'kernext-dark'
  | 'midnight-blue'
  | 'classic-terminal'
  | 'light-terminal';

export type TerminalCursorStyle = 'block' | 'bar' | 'underline';

export interface TerminalPreferences {
  width: number | null;
  height: number;
  fontSize: number;
  fontFamily: string;
  lineHeight: number;
  padding: number;
  theme: TerminalThemeId;
  cursorStyle: TerminalCursorStyle;
  opacity: number;
  showToolbar: boolean;
}

export const TERMINAL_PREFERENCES_STORAGE_KEY =
  'kernext.web-terminal.preferences';
export const TERMINAL_PREFERENCES_VERSION = 1;

export const DEFAULT_TERMINAL_PREFERENCES: TerminalPreferences = {
  width: null,
  height: 520,
  fontSize: 13,
  fontFamily: 'JetBrains Mono',
  lineHeight: 1.45,
  padding: 12,
  theme: 'kernext-dark',
  cursorStyle: 'block',
  opacity: 1,
  showToolbar: true,
};

export const TERMINAL_FONT_FAMILIES = [
  'JetBrains Mono',
  'Cascadia Mono',
  'Consolas',
  'SF Mono',
  'monospace',
] as const;

export const TERMINAL_THEMES: Record<
  TerminalThemeId,
  {
    label: string;
    background: string;
    foreground: string;
    cursor: string;
    selection: string;
    accent: string;
  }
> = {
  'kernext-dark': {
    label: 'KERNEXT Dark',
    background: '#080d14',
    foreground: '#dbe5ef',
    cursor: '#23c6df',
    selection: 'rgba(35, 198, 223, 0.22)',
    accent: '#23c6df',
  },
  'midnight-blue': {
    label: 'Midnight Blue',
    background: '#08111e',
    foreground: '#d9e8fb',
    cursor: '#6aaeff',
    selection: 'rgba(74, 143, 232, 0.25)',
    accent: '#6aaeff',
  },
  'classic-terminal': {
    label: 'Classic Terminal',
    background: '#07130b',
    foreground: '#c7f5cf',
    cursor: '#75e28b',
    selection: 'rgba(117, 226, 139, 0.22)',
    accent: '#75e28b',
  },
  'light-terminal': {
    label: 'Light Terminal',
    background: '#f4f6f8',
    foreground: '#202a35',
    cursor: '#087e8b',
    selection: 'rgba(8, 126, 139, 0.2)',
    accent: '#087e8b',
  },
};

const THEME_IDS = Object.keys(TERMINAL_THEMES) as TerminalThemeId[];
const CURSOR_STYLES: TerminalCursorStyle[] = [
  'block',
  'bar',
  'underline',
];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function clampNumber(value: unknown, fallback: number, min: number, max: number) {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return fallback;
  }

  return Math.min(max, Math.max(min, value));
}

function parsePreferences(value: unknown): TerminalPreferences | null {
  if (!isRecord(value)) {
    return null;
  }

  const width =
    value.width === null
      ? null
      : typeof value.width === 'number' && Number.isFinite(value.width)
        ? Math.round(Math.min(10000, Math.max(320, value.width)))
        : DEFAULT_TERMINAL_PREFERENCES.width;
  const fontFamily =
    typeof value.fontFamily === 'string' &&
    TERMINAL_FONT_FAMILIES.some((font) => font === value.fontFamily)
      ? value.fontFamily
      : DEFAULT_TERMINAL_PREFERENCES.fontFamily;
  const theme =
    typeof value.theme === 'string' &&
    THEME_IDS.includes(value.theme as TerminalThemeId)
      ? (value.theme as TerminalThemeId)
      : DEFAULT_TERMINAL_PREFERENCES.theme;
  const cursorStyle =
    typeof value.cursorStyle === 'string' &&
    CURSOR_STYLES.includes(value.cursorStyle as TerminalCursorStyle)
      ? (value.cursorStyle as TerminalCursorStyle)
      : DEFAULT_TERMINAL_PREFERENCES.cursorStyle;

  return {
    width,
    height: Math.round(
      clampNumber(value.height, DEFAULT_TERMINAL_PREFERENCES.height, 280, 1200),
    ),
    fontSize: clampNumber(
      value.fontSize,
      DEFAULT_TERMINAL_PREFERENCES.fontSize,
      11,
      20,
    ),
    fontFamily,
    lineHeight: clampNumber(
      value.lineHeight,
      DEFAULT_TERMINAL_PREFERENCES.lineHeight,
      1.1,
      1.8,
    ),
    padding: Math.round(
      clampNumber(value.padding, DEFAULT_TERMINAL_PREFERENCES.padding, 8, 28),
    ),
    theme,
    cursorStyle,
    opacity: clampNumber(value.opacity, 1, 0.75, 1),
    showToolbar: value.showToolbar !== false,
  };
}

export function parseTerminalPreferences(
  value: unknown,
): TerminalPreferences | null {
  if (!isRecord(value)) {
    return null;
  }

  if ('version' in value || 'preferences' in value) {
    if (
      value.version !== TERMINAL_PREFERENCES_VERSION ||
      !isRecord(value.preferences)
    ) {
      return null;
    }

    return parsePreferences(value.preferences);
  }

  return parsePreferences(value);
}

export function loadTerminalPreferences(): TerminalPreferences | null {
  try {
    const stored = window.localStorage.getItem(
      TERMINAL_PREFERENCES_STORAGE_KEY,
    );

    if (stored === null) {
      return null;
    }

    const parsed: unknown = JSON.parse(stored);
    const preferences = parseTerminalPreferences(parsed);

    if (!preferences) {
      window.localStorage.removeItem(TERMINAL_PREFERENCES_STORAGE_KEY);
    }

    return preferences;
  } catch {
    try {
      window.localStorage.removeItem(TERMINAL_PREFERENCES_STORAGE_KEY);
    } catch {
      return null;
    }

    return null;
  }
}

export function saveTerminalPreferences(
  preferences: TerminalPreferences,
): boolean {
  try {
    window.localStorage.setItem(
      TERMINAL_PREFERENCES_STORAGE_KEY,
      JSON.stringify({
        version: TERMINAL_PREFERENCES_VERSION,
        preferences,
      }),
    );
    return true;
  } catch {
    return false;
  }
}
