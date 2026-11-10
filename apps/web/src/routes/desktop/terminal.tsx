import { useEffect, useState } from 'react';
import { TerminalWorkspace } from '@/components/terminal/terminal-workspace';
import { desktopSettingsGet, type DesktopSettings } from '@/lib/desktop';

export function DesktopTerminalPage() {
  const [settings, setSettings] = useState<DesktopSettings | null>(null);

  useEffect(() => {
    let cancelled = false;

    void desktopSettingsGet()
      .then((next) => {
        if (!cancelled) {
          setSettings(next);
        }
      })
      .catch(() => {
        // The terminal still works with presentation defaults.
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="mx-auto flex h-full w-full max-w-[1600px] flex-col gap-4 px-4 py-6 sm:px-6 lg:px-8">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="font-mono text-[10px] uppercase tracking-[0.22em] text-primary">
            Terminal
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
            Interactive sessions
          </h1>
          <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
            Real KERNEXT processes and engine sessions driven by the local
            Rust runtime through Tauri IPC.
          </p>
        </div>
      </div>

      <div className="min-h-0 flex-1">
        <TerminalWorkspace
          transport="tauri"
          defaultMode="pty"
          fontSize={settings?.terminal_font_size ?? 14}
          fontFamily={settings?.terminal_font_family ?? 'JetBrains Mono'}
          cursorStyle={settings?.terminal_cursor_style ?? 'block'}
        />
      </div>
    </div>
  );
}
