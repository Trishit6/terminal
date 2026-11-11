import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
} from 'react';
import {
  Circle,
  CircleSlash,
  Copy,
  Cpu,
  Maximize2,
  Minimize2,
  Loader2,
  Monitor,
  MoveDiagonal,
  Plus,
  Power,
  RefreshCw,
  RotateCcw,
  Settings2,
  Square,
  Terminal as TerminalIcon,
  X,
} from 'lucide-react';
import { useAuth } from '@/components/auth/auth-provider';
import { Button } from '@/components/ui/button';
import { TerminalScreen } from '@/components/terminal/terminal-screen';
import { useTerminalConnection, type ConnectionMode, type ConnectionTransport } from '@/hooks/use-terminal-connection';
import { TerminalEngine } from '@/lib/terminal/engine';
import {
  DEFAULT_TERMINAL_PREFERENCES,
  TERMINAL_FONT_FAMILIES,
  TERMINAL_PREFERENCES_VERSION,
  TERMINAL_THEMES,
  loadTerminalPreferences,
  parseTerminalPreferences,
  saveTerminalPreferences,
  type TerminalPreferences,
} from '@/lib/terminal/preferences';
import { cn } from '@/lib/utils';

export interface TerminalWorkspaceProps {
  transport?: ConnectionTransport;
  defaultMode?: ConnectionMode;
  fontSize?: number;
  fontFamily?: string;
  cursorStyle?: 'block' | 'bar' | 'underline';
  showModeToggle?: boolean;
  showTabBar?: boolean;
  customizable?: boolean;
  className?: string;
}

interface TerminalTab {
  id: string;
  label: string;
  mode: ConnectionMode;
}

let nextTabId = 1;

function getWorkspaceLimits(element: HTMLDivElement | null) {
  const parentWidth = element?.parentElement?.clientWidth ?? window.innerWidth;
  const top = element?.getBoundingClientRect().top ?? 0;

  return {
    width: Math.max(
      320,
      Math.min(Math.floor(window.innerWidth - 24), Math.floor(parentWidth)),
    ),
    height: Math.max(
      280,
      Math.floor(window.innerHeight - Math.max(0, top) - 24),
    ),
  };
}

function clampWorkspaceSize(
  preferences: TerminalPreferences,
  element: HTMLDivElement | null,
): TerminalPreferences {
  const limits = getWorkspaceLimits(element);

  return {
    ...preferences,
    width:
      preferences.width === null
        ? null
        : Math.min(limits.width, Math.max(320, Math.round(preferences.width))),
    height: Math.min(
      limits.height,
      Math.max(280, Math.round(preferences.height)),
    ),
  };
}

function createTab(mode: ConnectionMode): TerminalTab {
  const id = `terminal-${nextTabId}`;
  nextTabId += 1;

  return {
    id,
    label: `Session ${nextTabId - 1}`,
    mode,
  };
}

export function TerminalWorkspace({
  transport,
  defaultMode = 'pty',
  fontSize = 13,
  fontFamily = 'JetBrains Mono',
  cursorStyle = 'block',
  showModeToggle = true,
  showTabBar = true,
  customizable = false,
  className,
}: TerminalWorkspaceProps) {
  const {
    signedIn,
    preferences: accountPreferences,
    updatePreferences,
  } = useAuth();
  const [tabs, setTabs] = useState<TerminalTab[]>(() => [createTab(defaultMode)]);
  const [activeId, setActiveId] = useState(() => tabs[0]?.id ?? '');
  const [terminalPreferences, setTerminalPreferences] = useState(
    () => loadTerminalPreferences() ?? DEFAULT_TERMINAL_PREFERENCES,
  );
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);
  const [windowedFullscreen, setWindowedFullscreen] = useState(false);
  const [fullscreenError, setFullscreenError] = useState<string | null>(null);
  const [preferenceNotice, setPreferenceNotice] = useState<string | null>(null);
  const [displaySize, setDisplaySize] = useState({ width: 0, height: 0 });
  const [customWidth, setCustomWidth] = useState('');
  const [customHeight, setCustomHeight] = useState('');
  const workspaceRef = useRef<HTMLDivElement>(null);
  const toolsRef = useRef<HTMLDivElement>(null);
  const localEditsRef = useRef(false);
  const preferencesRef = useRef(terminalPreferences);
  const saveTimerRef = useRef<number | null>(null);
  const accountSyncTimerRef = useRef<number | null>(null);
  const resizeDragRef = useRef<{
    x: number;
    y: number;
    width: number;
    height: number;
  } | null>(null);
  preferencesRef.current = terminalPreferences;

  const changePreferences = useCallback(
    (update: (current: TerminalPreferences) => TerminalPreferences) => {
      localEditsRef.current = true;
      setPreferenceNotice(null);
      setTerminalPreferences(update);
    },
    [],
  );

  useEffect(() => {
    if (!customizable || !signedIn || localEditsRef.current) {
      return;
    }

    const remote = parseTerminalPreferences(
      accountPreferences?.terminal_preferences,
    );

    if (!remote) {
      return;
    }

    const next = clampWorkspaceSize(remote, workspaceRef.current);
    preferencesRef.current = next;
    setTerminalPreferences(next);
    setCustomWidth(String(next.width ?? (displaySize.width || 1200)));
    setCustomHeight(String(next.height));

    if (!saveTerminalPreferences(next)) {
      setPreferenceNotice('Browser storage is unavailable; account settings will still sync.');
    }
  }, [
    accountPreferences?.terminal_preferences,
    customizable,
    displaySize.width,
    signedIn,
  ]);

  useEffect(() => {
    if (!customizable || !localEditsRef.current) {
      return undefined;
    }

    if (saveTimerRef.current !== null) {
      window.clearTimeout(saveTimerRef.current);
    }

    saveTimerRef.current = window.setTimeout(() => {
      saveTimerRef.current = null;
      if (!saveTerminalPreferences(terminalPreferences)) {
        setPreferenceNotice('Could not save terminal preferences in this browser.');
      }
    }, 180);

    return () => {
      if (saveTimerRef.current !== null) {
        window.clearTimeout(saveTimerRef.current);
        saveTimerRef.current = null;
      }
    };
  }, [customizable, terminalPreferences]);

  useEffect(() => {
    if (!customizable || !signedIn || !localEditsRef.current) {
      return undefined;
    }

    if (accountSyncTimerRef.current !== null) {
      window.clearTimeout(accountSyncTimerRef.current);
    }

    const savedPreferences = terminalPreferences;
    const savedSnapshot = JSON.stringify(savedPreferences);
    accountSyncTimerRef.current = window.setTimeout(() => {
      accountSyncTimerRef.current = null;
      void updatePreferences(
        accountPreferences?.theme ?? 'dark',
        {
          version: TERMINAL_PREFERENCES_VERSION,
          preferences: savedPreferences,
        },
      )
        .then(() => {
          if (JSON.stringify(preferencesRef.current) === savedSnapshot) {
            localEditsRef.current = false;
            setPreferenceNotice('Terminal preferences synced to your account.');
          }
        })
        .catch(() => {
          setPreferenceNotice(
            'Preferences are saved in this browser, but could not sync to your account.',
          );
        });
    }, 900);

    return () => {
      if (accountSyncTimerRef.current !== null) {
        window.clearTimeout(accountSyncTimerRef.current);
        accountSyncTimerRef.current = null;
      }
    };
  }, [
    accountPreferences?.theme,
    customizable,
    signedIn,
    terminalPreferences,
    updatePreferences,
  ]);

  useEffect(() => {
    if (!customizable) {
      return undefined;
    }

    const element = workspaceRef.current;
    if (!element) {
      return undefined;
    }

    const measure = () => {
      const rect = element.getBoundingClientRect();
      setDisplaySize({
        width: Math.round(rect.width),
        height: Math.round(rect.height),
      });
    };
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    measure();

    const clampToViewport = () => {
      setTerminalPreferences((current) =>
        clampWorkspaceSize(current, workspaceRef.current),
      );
    };
    const frame = window.requestAnimationFrame(clampToViewport);
    window.addEventListener('resize', clampToViewport);

    return () => {
      observer.disconnect();
      window.cancelAnimationFrame(frame);
      window.removeEventListener('resize', clampToViewport);
    };
  }, [customizable]);

  useEffect(() => {
    if (!windowedFullscreen) {
      return undefined;
    }

    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setWindowedFullscreen(false);
        setFullscreenError(null);
      }
    };
    document.addEventListener('keydown', onKeyDown);

    return () => {
      document.body.style.overflow = previousOverflow;
      document.removeEventListener('keydown', onKeyDown);
    };
  }, [windowedFullscreen]);

  useEffect(() => {
    if (!customizable) {
      return undefined;
    }

    const onFullscreenChange = () => {
      setFullscreen(document.fullscreenElement === workspaceRef.current);
      setFullscreenError(null);
    };

    document.addEventListener('fullscreenchange', onFullscreenChange);
    return () => {
      document.removeEventListener('fullscreenchange', onFullscreenChange);
    };
  }, [customizable]);

  useEffect(
    () => () => {
      if (saveTimerRef.current !== null) {
        window.clearTimeout(saveTimerRef.current);
      }
      if (localEditsRef.current) {
        saveTerminalPreferences(preferencesRef.current);
      }
      if (accountSyncTimerRef.current !== null) {
        window.clearTimeout(accountSyncTimerRef.current);
      }
    },
    [],
  );

  useEffect(() => {
    if (tabs.length === 0) {
      const tab = createTab(defaultMode);
      setTabs([tab]);
      setActiveId(tab.id);
    }
  }, [defaultMode, tabs.length]);

  const addTab = useCallback(() => {
    const tab = createTab(defaultMode);
    setTabs((current) => [...current, tab]);
    setActiveId(tab.id);
  }, [defaultMode]);

  const closeTab = useCallback(
    (id: string) => {
      const index = tabs.findIndex((tab) => tab.id === id);

      if (index === -1) {
        return;
      }

      const next = tabs.filter((tab) => tab.id !== id);
      setTabs(next);

      if (id === activeId) {
        const fallback = next[Math.max(0, index - 1)] ?? next[0];
        setActiveId(fallback?.id ?? '');
      }
    },
    [activeId, tabs],
  );

  const setTabMode = useCallback((id: string, mode: ConnectionMode) => {
    setTabs((current) =>
      current.map((tab) => (tab.id === id ? { ...tab, mode } : tab)),
    );
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.ctrlKey && !event.metaKey) {
        return;
      }

      const key = event.key.toLowerCase();

      if (event.shiftKey && key === 't') {
        event.preventDefault();
        addTab();
        return;
      }

      if (key === 'w') {
        event.preventDefault();

        if (activeId) {
          closeTab(activeId);
        }
      }
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [activeId, addTab, closeTab]);

  const activeTab = tabs.find((tab) => tab.id === activeId) ?? tabs[0];
  const activePreset =
    terminalPreferences.width !== null
      ? 'custom'
      : terminalPreferences.height === 320
        ? 'compact'
        : terminalPreferences.height === 520
          ? 'standard'
          : terminalPreferences.height === 720
            ? 'large'
            : 'custom';
  const settingsMaxHeight = Math.max(
    120,
    Math.min(
      384,
      window.innerHeight -
        (toolsRef.current?.getBoundingClientRect().bottom ?? 0) -
        12,
    ),
  );

  const applyPreset = (preset: string) => {
    if (preset === 'custom') {
      setCustomWidth(String(displaySize.width || 1200));
      setCustomHeight(String(terminalPreferences.height));
      setSettingsOpen(true);
      return;
    }

    const height =
      preset === 'compact' ? 320 : preset === 'large' ? 720 : 520;
    changePreferences((current) =>
      clampWorkspaceSize({ ...current, width: null, height }, workspaceRef.current),
    );
  };

  const fitToWindow = () => {
    const limits = getWorkspaceLimits(workspaceRef.current);
    changePreferences((current) => ({
      ...current,
      width: null,
      height: limits.height,
    }));
  };

  const resetPreferences = () => {
    const next = clampWorkspaceSize(
      DEFAULT_TERMINAL_PREFERENCES,
      workspaceRef.current,
    );
    changePreferences(() => next);
    setCustomWidth(String(displaySize.width || 1200));
    setCustomHeight(String(next.height));
    setSettingsOpen(false);
  };

  const toggleFullscreen = async () => {
    setFullscreenError(null);

    if (
      document.fullscreenElement === workspaceRef.current ||
      windowedFullscreen
    ) {
      try {
        if (document.fullscreenElement === workspaceRef.current) {
          await document.exitFullscreen();
        } else {
          setWindowedFullscreen(false);
          setFullscreenError(null);
        }
      } catch {
        setFullscreenError('The browser could not exit fullscreen mode.');
      }
      return;
    }

    const element = workspaceRef.current;
    if (!element?.requestFullscreen) {
      setWindowedFullscreen(true);
      setFullscreenError('Fullscreen is not supported by this browser.');
      return;
    }

    try {
      await element.requestFullscreen();
    } catch {
      setWindowedFullscreen(true);
      setFullscreenError(
        'Browser fullscreen is unavailable; using a window-sized terminal. Press Escape to exit.',
      );
    }
  };

  const applyCustomSize = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const width = Number(customWidth);
    const height = Number(customHeight);

    if (
      !Number.isFinite(width) ||
      !Number.isFinite(height) ||
      width < 320 ||
      height < 280
    ) {
      setPreferenceNotice('Enter a width of at least 320px and a height of at least 280px.');
      return;
    }

    changePreferences((current) =>
      clampWorkspaceSize(
        { ...current, width: Math.round(width), height: Math.round(height) },
        workspaceRef.current,
      ),
    );
  };

  const handleResizePointerDown = (
    event: React.PointerEvent<HTMLButtonElement>,
  ) => {
    if (event.pointerType === 'mouse' && event.button !== 0) {
      return;
    }

    const rect = workspaceRef.current?.getBoundingClientRect();
    if (!rect) {
      return;
    }

    event.preventDefault();
    resizeDragRef.current = {
      x: event.clientX,
      y: event.clientY,
      width: rect.width,
      height: rect.height,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const handleResizePointerMove = (
    event: React.PointerEvent<HTMLButtonElement>,
  ) => {
    const drag = resizeDragRef.current;
    if (!drag) {
      return;
    }

    const limits = getWorkspaceLimits(workspaceRef.current);
    changePreferences((current) => ({
      ...current,
      width: Math.min(
        limits.width,
        Math.max(320, Math.round(drag.width + event.clientX - drag.x)),
      ),
      height: Math.min(
        limits.height,
        Math.max(280, Math.round(drag.height + event.clientY - drag.y)),
      ),
    }));
  };

  return (
    <div
      ref={workspaceRef}
      className={cn(
        'terminal-workspace-frame relative flex h-full min-h-0 min-w-0 flex-col gap-3 rounded-2xl border border-border bg-surface/60 p-3 backdrop-blur',
        windowedFullscreen && 'terminal-workspace-frame--windowed-fullscreen',
        customizable && 'h-auto max-w-full',
        className,
      )}
      style={
        customizable
          ? {
              width:
                terminalPreferences.width === null
                  ? '100%'
                  : terminalPreferences.width,
              height: terminalPreferences.height,
              maxWidth: '100%',
              minHeight: 280,
              flex: 'none',
              alignSelf: 'flex-start',
            }
          : undefined
      }
    >
      {showTabBar ? (
        <div className="flex items-center gap-2 overflow-x-auto no-scrollbar">
          <div className="flex items-center gap-1.5">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                type="button"
                onClick={() => setActiveId(tab.id)}
                className={cn(
                  'group inline-flex items-center gap-2 rounded-lg border px-3 py-1.5 text-xs font-medium transition-colors',
                  tab.id === activeId
                    ? 'border-primary/40 bg-primary/10 text-foreground'
                    : 'border-border bg-surface/70 text-muted-foreground hover:border-primary/30 hover:text-foreground',
                )}
              >
                <TerminalIcon className="size-3.5" />
                {tab.label}
                <span
                  role="button"
                  tabIndex={-1}
                  onClick={(event) => {
                    event.stopPropagation();
                    closeTab(tab.id);
                  }}
                  className="rounded p-0.5 opacity-0 transition-opacity group-hover:opacity-100 focus:opacity-100"
                  aria-label={`Close ${tab.label}`}
                >
                  <X className="size-3" />
                </span>
              </button>
            ))}
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={addTab}
            aria-label="New terminal session"
            className="size-7 shrink-0"
          >
            <Plus className="size-4" />
          </Button>
        </div>
      ) : null}

      {customizable ? (
        <>
          <div className="relative z-20">
          <div
            ref={toolsRef}
            className="flex min-w-0 flex-wrap items-center gap-2 border-b border-border/70 pb-2"
          >
            <label className="sr-only" htmlFor="terminal-size-preset">
              Terminal size preset
            </label>
            <select
              id="terminal-size-preset"
              value={activePreset}
              onChange={(event) => applyPreset(event.currentTarget.value)}
              className="h-11 min-w-32 rounded-lg border border-border bg-background px-2.5 text-xs text-foreground outline-none transition-colors focus:border-primary/60 focus:ring-2 focus:ring-primary/15 sm:h-9"
            >
              <option value="compact">Compact · 320px</option>
              <option value="standard">Standard · 520px</option>
              <option value="large">Large · 720px</option>
              <option value="custom">Custom size…</option>
            </select>

            <span className="whitespace-nowrap font-mono text-[10px] tabular-nums text-muted-foreground">
              {displaySize.width || '—'} × {displaySize.height || terminalPreferences.height}
            </span>

            <div className="ml-auto flex items-center gap-1">
              <Button
                type="button"
                variant={settingsOpen ? 'secondary' : 'ghost'}
                size="icon-sm"
                className="size-11 sm:size-8"
                aria-label={settingsOpen ? 'Close terminal settings' : 'Open terminal settings'}
                aria-expanded={settingsOpen}
                title="Terminal settings"
                onClick={() => {
                  setCustomWidth(String(displaySize.width || 1200));
                  setCustomHeight(String(terminalPreferences.height));
                  setSettingsOpen((open) => !open);
                }}
              >
                <Settings2 className="size-4" />
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                className="size-11 sm:size-8"
                aria-label="Fit terminal to available window space"
                title="Fit to window"
                onClick={fitToWindow}
              >
                <MoveDiagonal className="size-4" />
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                className="size-11 sm:size-8"
                aria-label={
                  fullscreen || windowedFullscreen
                    ? 'Exit fullscreen'
                    : 'Enter fullscreen'
                }
                title={
                  fullscreen || windowedFullscreen
                    ? 'Exit fullscreen'
                    : 'Fullscreen'
                }
                onClick={() => void toggleFullscreen()}
              >
                {fullscreen || windowedFullscreen ? (
                  <Minimize2 className="size-4" />
                ) : (
                  <Maximize2 className="size-4" />
                )}
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                className="size-11 sm:size-8"
                aria-label="Reset terminal preferences"
                title="Reset layout and appearance"
                onClick={resetPreferences}
              >
                <RotateCcw className="size-4" />
              </Button>
            </div>
          </div>

          {settingsOpen ? (
            <div
              className="terminal-settings-panel grid min-w-0 gap-4 rounded-xl border border-border bg-background p-3 shadow-2xl backdrop-blur-xl sm:p-4"
              style={{ maxHeight: settingsMaxHeight }}
            >
              <form
                className="grid grid-cols-2 items-end gap-3 sm:grid-cols-[minmax(7rem,1fr)_minmax(7rem,1fr)_auto]"
                onSubmit={applyCustomSize}
              >
                <label className="terminal-setting-label">
                  Width (px)
                  <input
                    type="number"
                    min={320}
                    max={getWorkspaceLimits(workspaceRef.current).width}
                    value={customWidth}
                    onChange={(event) => setCustomWidth(event.currentTarget.value)}
                    className="terminal-setting-control"
                  />
                </label>
                <label className="terminal-setting-label">
                  Height (px)
                  <input
                    type="number"
                    min={280}
                    max={getWorkspaceLimits(workspaceRef.current).height}
                    value={customHeight}
                    onChange={(event) => setCustomHeight(event.currentTarget.value)}
                    className="terminal-setting-control"
                  />
                </label>
                <Button type="submit" size="sm" className="col-span-2 sm:col-span-1">
                  Apply size
                </Button>
              </form>

              <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
                <label className="terminal-setting-label">
                  Font size · {terminalPreferences.fontSize}px
                  <input
                    type="range"
                    min={11}
                    max={20}
                    step={1}
                    value={terminalPreferences.fontSize}
                    onChange={(event) => {
                      const fontSize = Number(event.currentTarget.value);
                      changePreferences((current) => ({ ...current, fontSize }));
                    }}
                    className="terminal-setting-range"
                  />
                </label>
                <label className="terminal-setting-label">
                  Monospace font
                  <select
                    value={terminalPreferences.fontFamily}
                    onChange={(event) => {
                      const fontFamily = event.currentTarget.value;
                      changePreferences((current) => ({ ...current, fontFamily }));
                    }}
                    className="terminal-setting-control"
                  >
                    {TERMINAL_FONT_FAMILIES.map((font) => (
                      <option key={font} value={font}>
                        {font}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="terminal-setting-label">
                  Line height · {terminalPreferences.lineHeight.toFixed(2)}
                  <input
                    type="range"
                    min={1.1}
                    max={1.8}
                    step={0.05}
                    value={terminalPreferences.lineHeight}
                    onChange={(event) => {
                      const lineHeight = Number(event.currentTarget.value);
                      changePreferences((current) => ({ ...current, lineHeight }));
                    }}
                    className="terminal-setting-range"
                  />
                </label>
                <label className="terminal-setting-label">
                  Terminal padding · {terminalPreferences.padding}px
                  <input
                    type="range"
                    min={8}
                    max={28}
                    step={2}
                    value={terminalPreferences.padding}
                    onChange={(event) => {
                      const padding = Number(event.currentTarget.value);
                      changePreferences((current) => ({ ...current, padding }));
                    }}
                    className="terminal-setting-range"
                  />
                </label>
                <label className="terminal-setting-label">
                  Color theme
                  <select
                    value={terminalPreferences.theme}
                    onChange={(event) => {
                      const theme = event.currentTarget.value as TerminalPreferences['theme'];
                      changePreferences((current) => ({ ...current, theme }));
                    }}
                    className="terminal-setting-control"
                  >
                    {Object.entries(TERMINAL_THEMES).map(([id, theme]) => (
                      <option key={id} value={id}>
                        {theme.label}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="terminal-setting-label">
                  Cursor style
                  <select
                    value={terminalPreferences.cursorStyle}
                    onChange={(event) => {
                      const cursorStyle = event.currentTarget.value as TerminalPreferences['cursorStyle'];
                      changePreferences((current) => ({ ...current, cursorStyle }));
                    }}
                    className="terminal-setting-control"
                  >
                    <option value="block">Block</option>
                    <option value="bar">Bar</option>
                    <option value="underline">Underline</option>
                  </select>
                </label>
                <label className="terminal-setting-label col-span-2 sm:col-span-1">
                  Background opacity · {Math.round(terminalPreferences.opacity * 100)}%
                  <input
                    type="range"
                    min={75}
                    max={100}
                    step={1}
                    value={Math.round(terminalPreferences.opacity * 100)}
                    onChange={(event) => {
                      const opacity = Number(event.currentTarget.value) / 100;
                      changePreferences((current) => ({ ...current, opacity }));
                    }}
                    className="terminal-setting-range"
                  />
                </label>
              </div>
            </div>
          ) : null}
          </div>

          {preferenceNotice || fullscreenError ? (
            <p
              role={fullscreenError ? 'alert' : 'status'}
              className={cn(
                'text-[11px] leading-relaxed',
                fullscreenError ? 'text-danger' : 'text-muted-foreground',
              )}
            >
              {fullscreenError ?? preferenceNotice}
            </p>
          ) : null}
        </>
      ) : null}

      {activeTab ? (
        <TerminalPanel
          key={activeTab.id}
          mode={activeTab.mode}
          transport={transport}
          fontSize={customizable ? terminalPreferences.fontSize : fontSize}
          fontFamily={customizable ? terminalPreferences.fontFamily : fontFamily}
          cursorStyle={customizable ? terminalPreferences.cursorStyle : cursorStyle}
          appearance={customizable ? terminalPreferences : undefined}
          onFontSizeChange={
            customizable
              ? (size) =>
                  changePreferences((current) => ({
                    ...current,
                    fontSize: size,
                  }))
              : undefined
          }
          showModeToggle={showModeToggle}
          onModeChange={(mode) => setTabMode(activeTab.id, mode)}
        />
      ) : null}

      {customizable ? (
        <button
          type="button"
          className="terminal-resize-handle"
          aria-label="Resize terminal workspace. Use arrow keys to adjust its size."
          title="Drag to resize"
          onPointerDown={handleResizePointerDown}
          onPointerMove={handleResizePointerMove}
          onPointerUp={() => {
            resizeDragRef.current = null;
          }}
          onPointerCancel={() => {
            resizeDragRef.current = null;
          }}
          onLostPointerCapture={() => {
            resizeDragRef.current = null;
          }}
          onKeyDown={(event) => {
            if (!['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(event.key)) {
              return;
            }
            event.preventDefault();
            const step = event.shiftKey ? 48 : 16;
            const limits = getWorkspaceLimits(workspaceRef.current);
            changePreferences((current) => ({
              ...current,
              width:
                current.width === null
                  ? Math.min(
                      limits.width,
                      Math.max(320, displaySize.width + (event.key === 'ArrowRight' ? step : event.key === 'ArrowLeft' ? -step : 0)),
                    )
                  : Math.min(
                      limits.width,
                      Math.max(320, current.width + (event.key === 'ArrowRight' ? step : event.key === 'ArrowLeft' ? -step : 0)),
                    ),
              height: Math.min(
                limits.height,
                Math.max(
                  280,
                  current.height +
                    (event.key === 'ArrowDown'
                      ? step
                      : event.key === 'ArrowUp'
                        ? -step
                        : 0),
                ),
              ),
            }));
          }}
        >
          <span aria-hidden="true" />
        </button>
      ) : null}
    </div>
  );
}

interface TerminalPanelProps {
  mode: ConnectionMode;
  transport?: ConnectionTransport;
  fontSize: number;
  fontFamily: string;
  cursorStyle: 'block' | 'bar' | 'underline';
  appearance?: TerminalPreferences;
  onFontSizeChange?: (fontSize: number) => void;
  showModeToggle: boolean;
  onModeChange: (mode: ConnectionMode) => void;
}

function TerminalPanel({
  mode,
  transport,
  fontSize,
  fontFamily,
  cursorStyle,
  appearance,
  onFontSizeChange,
  showModeToggle,
  onModeChange,
}: TerminalPanelProps) {
  const engine = useMemo(() => new TerminalEngine(), []);
  const version = useSyncExternalStore(
    engine.subscribe,
    engine.getSnapshot,
    engine.getSnapshot,
  );
  const connection = useTerminalConnection({ engine, mode, transport });
  const [copied, setCopied] = useState(false);
  const [localTerminalFontSize, setLocalTerminalFontSize] = useState(fontSize);
  const terminalFontSize = onFontSizeChange
    ? fontSize
    : localTerminalFontSize;

  const handleCopy = useCallback(async () => {
    const copied = await connection.copyOutput();
    setCopied(copied);

    if (copied) {
      window.setTimeout(() => setCopied(false), 1500);
    }
  }, [connection]);

  const status = connection.status;
  const statusLabel =
    status === 'open'
      ? 'Connected'
      : status === 'connecting'
        ? 'Connecting'
        : status === 'error'
          ? 'Failed'
          : status === 'closed'
            ? 'Disconnected'
            : 'Ready';
  const statusIcon =
    status === 'open' ? (
      <Circle className="size-3 fill-success text-success" />
    ) : status === 'connecting' ? (
      <Loader2 className="size-3 animate-spin text-primary" />
    ) : status === 'error' ? (
      <CircleSlash className="size-3 text-danger" />
    ) : (
      <Circle className="size-3 text-muted-foreground" />
    );

  // The backend answered the reachability probe, but no terminal connection
  // is open yet. Everything between here and `open` is honest state, never a
  // fake prompt.
  const backendMissing = connection.backendState === 'unreachable';
  const showFirstRunNotice =
    backendMissing && (status === 'idle' || status === 'closed');
  const showConnectionError = status === 'error' && connection.error !== null;
  const ptyUnavailable =
    mode === 'pty' && connection.health?.pty_available === false;
  const showPtyNotice =
    ptyUnavailable && status !== 'open' && !showFirstRunNotice;
  const reconnect = status === 'closed' || status === 'error';

  const adjustFontSize = useCallback((delta: number) => {
    const max = onFontSizeChange ? 20 : 18;
    const next = Math.min(max, Math.max(11, terminalFontSize + delta));
    if (onFontSizeChange) {
      onFontSizeChange(next);
    } else {
      setLocalTerminalFontSize(next);
    }
  }, [onFontSizeChange, terminalFontSize]);

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      {/* Session toolbar: connection state, working directory, mode. */}
      <div className="flex flex-wrap items-center gap-2">
        <div className="flex items-center gap-2 rounded-lg border border-border bg-surface/80 px-3 py-1.5 text-xs">
          <span className="inline-flex items-center gap-1.5">
            {statusIcon}
            <span className="font-medium">{statusLabel}</span>
          </span>
          <span className="text-muted-foreground">·</span>
          <span className="font-mono text-[11px] text-muted-foreground">
            {mode === 'pty' ? 'interactive PTY' : 'engine sessions'}
          </span>
        </div>

        {showModeToggle ? (
          <div className="inline-flex overflow-hidden rounded-lg border border-border">
            {(['pty', 'engine'] as const).map((value) => (
              <button
                key={value}
                type="button"
                onClick={() => onModeChange(value)}
                title={
                  value === 'pty'
                    ? 'Full interactive terminal backed by a real kernext process'
                    : 'Line-oriented commands executed by the in-process Rust engine'
                }
                className={cn(
                  'inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium transition-colors',
                  mode === value
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-surface/70 text-muted-foreground hover:bg-surface hover:text-foreground',
                )}
              >
                {value === 'pty' ? (
                  <Monitor className="size-3.5" />
                ) : (
                  <Cpu className="size-3.5" />
                )}
                {value === 'pty' ? 'PTY' : 'Engine'}
              </button>
            ))}
          </div>
        ) : null}

        <div className="ml-auto flex items-center gap-1.5">
          {status === 'open' ? (
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={connection.disconnect}
              className="gap-1.5"
            >
              <Power className="size-3.5" />
              Disconnect
            </Button>
          ) : (
            <Button
              type="button"
              size="sm"
              onClick={connection.connect}
              className="gap-1.5"
            >
              <RefreshCw className="size-3.5" />
              {reconnect ? 'Reconnect' : 'Connect'}
            </Button>
          )}
        </div>
      </div>

      {/* Terminal toolbar: output controls and font settings. */}
      <div className="flex flex-wrap items-center gap-1.5">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={connection.clear}
          className="gap-1.5"
        >
          <Square className="size-3.5" />
          Clear
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={handleCopy}
          className="gap-1.5"
        >
          {copied ? (
            <Copy className="size-3.5 text-success" />
          ) : (
            <Copy className="size-3.5" />
          )}
          {copied ? 'Copied' : 'Copy'}
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={connection.interrupt}
          disabled={status !== 'open'}
          title={
            mode === 'pty'
              ? 'Send Ctrl+C to the running terminal process'
              : 'A running engine command cannot be interrupted yet'
          }
          className="gap-1.5"
        >
          <CircleSlash className="size-3.5" />
          Interrupt
        </Button>

        <div className="ml-auto flex items-center gap-1 rounded-lg border border-border bg-surface/70 px-1.5 py-1">
          <button
            type="button"
            aria-label="Decrease terminal font size"
            title="Smaller text"
            onClick={() => adjustFontSize(-1)}
            className="rounded px-1.5 py-0.5 text-xs text-muted-foreground transition-colors hover:bg-surface hover:text-foreground"
          >
            A−
          </button>
          <span className="font-mono text-[10px] tabular-nums text-muted-foreground">
            {terminalFontSize}px
          </span>
          <button
            type="button"
            aria-label="Increase terminal font size"
            title="Larger text"
            onClick={() => adjustFontSize(1)}
            className="rounded px-1.5 py-0.5 text-xs text-muted-foreground transition-colors hover:bg-surface hover:text-foreground"
          >
            A+
          </button>
        </div>
      </div>

      {showConnectionError ? (
        <div className="rounded-lg border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger">
          {connection.error}
        </div>
      ) : null}

      <div className="relative min-h-0 flex-1">
        <TerminalScreen
          engine={engine}
          version={version}
          onData={connection.sendInput}
          onResize={connection.resize}
          fontSize={terminalFontSize}
          fontFamily={fontFamily}
          cursorStyle={cursorStyle}
          lineHeight={appearance?.lineHeight}
          padding={appearance?.padding}
          theme={appearance?.theme}
          opacity={appearance?.opacity}
          autoFocus
          className={cn(
            'bg-[#04121a]',
            showFirstRunNotice && 'opacity-60',
          )}
        />

        {showPtyNotice ? (
          <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center p-6">
            <div className="pointer-events-auto w-full max-w-md rounded-2xl border border-border bg-surface/95 p-5 text-center shadow-2xl backdrop-blur">
              <div className="mx-auto mb-3 flex size-10 items-center justify-center rounded-xl border border-border bg-surface-soft">
                <Cpu className="size-5 text-primary" />
              </div>
              <h3 className="text-sm font-semibold text-foreground">
                PTY mode needs the built CLI
              </h3>
              <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
                The local server is running, but it could not find the{' '}
                <code className="font-mono text-[11px]">kernext</code>{' '}
                executable, so it cannot start an interactive terminal. Engine
                mode uses the same Rust runtime in-process and works right now.
              </p>
              <pre className="mt-3 overflow-x-auto rounded-lg border border-border bg-[#04121a] px-3 py-2 text-left font-mono text-[11px] text-primary">
                cargo build -p shell-cli
              </pre>
              <div className="mt-4 flex flex-wrap justify-center gap-2">
                <Button
                  type="button"
                  size="sm"
                  onClick={() => onModeChange('engine')}
                  className="gap-1.5"
                >
                  <Cpu className="size-3.5" />
                  Use Engine mode
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => void connection.refreshHealth()}
                  className="gap-1.5"
                >
                  <RefreshCw className="size-3.5" />
                  Check again
                </Button>
              </div>
            </div>
          </div>
        ) : null}

        {showFirstRunNotice ? (
          <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center p-6">
            <div className="pointer-events-auto w-full max-w-md rounded-2xl border border-border bg-surface/95 p-5 text-center shadow-2xl backdrop-blur">
              <div className="mx-auto mb-3 flex size-10 items-center justify-center rounded-xl border border-border bg-surface-soft">
                <TerminalIcon className="size-5 text-primary" />
              </div>
              <h3 className="text-sm font-semibold text-foreground">
                The KERNEXT server isn&rsquo;t running yet
              </h3>
              <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
                This terminal is a real shell — every command runs in the Rust
                engine on this machine, not in the browser. Start the local
                backend from the project directory, then connect:
              </p>
              <pre className="mt-3 overflow-x-auto rounded-lg border border-border bg-[#04121a] px-3 py-2 text-left font-mono text-[11px] text-primary">
                cargo run -p shell-server
              </pre>
              <p className="mt-2 text-[11px] leading-relaxed text-muted-foreground">
                The web app on port 5173 proxies to it on port 7878. The
                desktop app starts it automatically.
              </p>
              <Button
                type="button"
                size="sm"
                onClick={connection.connect}
                className="mt-4 gap-1.5"
              >
                <RefreshCw className="size-3.5" />
                Try again
              </Button>
            </div>
          </div>
        ) : null}
      </div>

      {/* Footer: live session facts only. */}
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-[11px] text-muted-foreground">
        <span className="font-mono">
          {connection.size.rows} × {connection.size.cols}
        </span>
        <span className="font-mono">
          {mode === 'pty' ? 'pty process' : 'engine session'}
        </span>
        {connection.session ? (
          <span className="font-mono">
            session {connection.session.id} · {connection.session.state}
          </span>
        ) : null}
        {connection.session ? (
          <span className="max-w-[420px] truncate font-mono">
            {connection.session.working_directory}
          </span>
        ) : null}
        {connection.health?.version ? (
          <span className="font-mono">v{connection.health.version}</span>
        ) : null}
        {connection.health?.platform ? (
          <span className="font-mono">{connection.health.platform}</span>
        ) : null}
      </div>
    </div>
  );
}
