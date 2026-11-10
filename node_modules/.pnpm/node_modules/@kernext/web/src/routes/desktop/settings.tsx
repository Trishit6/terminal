import { useEffect, useState } from 'react';
import {
  Check,
  Monitor,
  Palette,
  RotateCcw,
  Save,
  Terminal,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import {
  desktopSettingsGet,
  desktopSettingsSave,
  type DesktopSettings,
} from '@/lib/desktop';

const DEFAULT_SETTINGS: DesktopSettings = {
  theme: 'midnight',
  terminal_font_size: 14,
  terminal_font_family: 'JetBrains Mono',
  terminal_cursor_style: 'block',
  default_working_directory: '',
  history_enabled: true,
  history_size: 500,
  startup_animation: true,
  reduce_motion: false,
  accent_color: 'cyan',
};

export function DesktopSettingsPage() {
  const [settings, setSettings] = useState<DesktopSettings>(DEFAULT_SETTINGS);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    let cancelled = false;

    void desktopSettingsGet()
      .then((next) => {
        if (!cancelled) {
          setSettings(next);
          setLoaded(true);
        }
      })
      .catch((caught: unknown) => {
        if (!cancelled) {
          setError(
            caught instanceof Error ? caught.message : String(caught),
          );
          setLoaded(true);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const update = <K extends keyof DesktopSettings>(
    key: K,
    value: DesktopSettings[K],
  ) => {
    setSettings((current) => ({ ...current, [key]: value }));
    setSaved(false);
  };

  const save = async () => {
    setSaving(true);
    setError(null);

    try {
      const next = await desktopSettingsSave(settings);
      setSettings(next);
      setSaved(true);
      window.setTimeout(() => setSaved(false), 1800);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setSaving(false);
    }
  };

  const reset = () => {
    setSettings(DEFAULT_SETTINGS);
    setSaved(false);
  };

  return (
    <div className="mx-auto w-full max-w-[1100px] px-6 py-8 lg:px-10">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="font-mono text-[10px] uppercase tracking-[0.22em] text-primary">
            Settings
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
            Preferences
          </h1>
          <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
            Desktop-only presentation settings. They are validated by the Rust
            runtime before they are stored.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={reset} className="gap-2">
            <RotateCcw className="size-3.5" />
            Reset
          </Button>
          <Button size="sm" onClick={() => void save()} disabled={saving} className="gap-2">
            {saving ? <Save className="size-3.5 animate-pulse" /> : <Save className="size-3.5" />}
            Save
          </Button>
        </div>
      </div>

      {error ? (
        <div className="mt-6 rounded-lg border border-danger/30 bg-danger/10 px-4 py-3 text-sm text-danger">
          {error}
        </div>
      ) : null}

      {saved ? (
        <div className="mt-6 flex items-center gap-2 rounded-lg border border-success/30 bg-success/10 px-4 py-3 text-sm text-success">
          <Check className="size-4" />
          Settings saved.
        </div>
      ) : null}

      <div className="mt-8 space-y-4">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Palette className="size-4 text-primary" />
              Appearance
            </CardTitle>
            <CardDescription>
              Theme, accent color, and motion preferences.
            </CardDescription>
          </CardHeader>
          <CardContent className="grid gap-4 sm:grid-cols-2">
            <label className="space-y-1.5">
              <span className="text-sm font-medium">Theme</span>
              <select
                value={settings.theme}
                onChange={(event) =>
                  update('theme', event.target.value as DesktopSettings['theme'])
                }
                className="h-10 w-full rounded-lg border border-border bg-surface/70 px-3 text-sm outline-none focus:border-primary/50"
              >
                <option value="midnight">Midnight</option>
                <option value="light">Light</option>
                <option value="system">System</option>
              </select>
            </label>

            <label className="space-y-1.5">
              <span className="text-sm font-medium">Accent color</span>
              <select
                value={settings.accent_color}
                onChange={(event) =>
                  update(
                    'accent_color',
                    event.target.value as DesktopSettings['accent_color'],
                  )
                }
                className="h-10 w-full rounded-lg border border-border bg-surface/70 px-3 text-sm outline-none focus:border-primary/50"
              >
                <option value="cyan">Cyan</option>
                <option value="blue">Blue</option>
                <option value="violet">Violet</option>
              </select>
            </label>

            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={settings.startup_animation}
                onChange={(event) =>
                  update('startup_animation', event.target.checked)
                }
                className="size-4 rounded border-border bg-surface accent-primary"
              />
              Show the startup animation
            </label>

            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={settings.reduce_motion}
                onChange={(event) => update('reduce_motion', event.target.checked)}
                className="size-4 rounded border-border bg-surface accent-primary"
              />
              Reduce motion across the interface
            </label>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Terminal className="size-4 text-primary" />
              Terminal
            </CardTitle>
            <CardDescription>
              Font, cursor, and default working directory for new sessions.
            </CardDescription>
          </CardHeader>
          <CardContent className="grid gap-4 sm:grid-cols-2">
            <label className="space-y-1.5">
              <span className="text-sm font-medium">Font family</span>
              <input
                type="text"
                value={settings.terminal_font_family}
                onChange={(event) =>
                  update('terminal_font_family', event.target.value)
                }
                className="h-10 w-full rounded-lg border border-border bg-surface/70 px-3 font-mono text-sm outline-none focus:border-primary/50"
              />
            </label>

            <label className="space-y-1.5">
              <span className="text-sm font-medium">Font size</span>
              <input
                type="number"
                min={8}
                max={32}
                value={settings.terminal_font_size}
                onChange={(event) =>
                  update('terminal_font_size', Number(event.target.value))
                }
                className="h-10 w-full rounded-lg border border-border bg-surface/70 px-3 font-mono text-sm outline-none focus:border-primary/50"
              />
            </label>

            <label className="space-y-1.5">
              <span className="text-sm font-medium">Cursor style</span>
              <select
                value={settings.terminal_cursor_style}
                onChange={(event) =>
                  update(
                    'terminal_cursor_style',
                    event.target.value as DesktopSettings['terminal_cursor_style'],
                  )
                }
                className="h-10 w-full rounded-lg border border-border bg-surface/70 px-3 text-sm outline-none focus:border-primary/50"
              >
                <option value="block">Block</option>
                <option value="bar">Bar</option>
                <option value="underline">Underline</option>
              </select>
            </label>

            <label className="space-y-1.5">
              <span className="text-sm font-medium">Default working directory</span>
              <input
                type="text"
                value={settings.default_working_directory}
                onChange={(event) =>
                  update('default_working_directory', event.target.value)
                }
                placeholder="Leave empty for the user home directory"
                className="h-10 w-full rounded-lg border border-border bg-surface/70 px-3 font-mono text-xs outline-none focus:border-primary/50"
              />
            </label>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Monitor className="size-4 text-primary" />
              History
            </CardTitle>
            <CardDescription>
              Terminal history retention for desktop sessions.
            </CardDescription>
          </CardHeader>
          <CardContent className="grid gap-4 sm:grid-cols-2">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={settings.history_enabled}
                onChange={(event) => update('history_enabled', event.target.checked)}
                className="size-4 rounded border-border bg-surface accent-primary"
              />
              Keep command history
            </label>

            <label className="space-y-1.5">
              <span className="text-sm font-medium">History size</span>
              <input
                type="number"
                min={1}
                max={10000}
                value={settings.history_size}
                onChange={(event) =>
                  update('history_size', Number(event.target.value))
                }
                className="h-10 w-full rounded-lg border border-border bg-surface/70 px-3 font-mono text-sm outline-none focus:border-primary/50"
              />
            </label>
          </CardContent>
        </Card>
      </div>

      <Separator className="my-8" />

      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span>
          {loaded ? 'Settings are stored in the desktop configuration directory.' : 'Loading settings…'}
        </span>
        <Badge variant="outline">KERNEXT</Badge>
      </div>
    </div>
  );
}
