import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  useSyncExternalStore,
} from 'react';
import {
  Circle,
  CircleSlash,
  Copy,
  Cpu,
  Loader2,
  Monitor,
  Plus,
  Power,
  RefreshCw,
  Square,
  Terminal as TerminalIcon,
  X,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { TerminalScreen } from '@/components/terminal/terminal-screen';
import { useTerminalConnection, type ConnectionMode, type ConnectionTransport } from '@/hooks/use-terminal-connection';
import { TerminalEngine } from '@/lib/terminal/engine';
import { cn } from '@/lib/utils';

export interface TerminalWorkspaceProps {
  transport?: ConnectionTransport;
  defaultMode?: ConnectionMode;
  fontSize?: number;
  fontFamily?: string;
  cursorStyle?: 'block' | 'bar' | 'underline';
  showModeToggle?: boolean;
  showTabBar?: boolean;
  className?: string;
}

interface TerminalTab {
  id: string;
  label: string;
  mode: ConnectionMode;
}

let nextTabId = 1;

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
  className,
}: TerminalWorkspaceProps) {
  const [tabs, setTabs] = useState<TerminalTab[]>(() => [createTab(defaultMode)]);
  const [activeId, setActiveId] = useState(() => tabs[0]?.id ?? '');

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

  return (
    <div
      className={cn(
        'flex h-full min-h-0 flex-col gap-3 rounded-2xl border border-border bg-surface/60 p-3 backdrop-blur',
        className,
      )}
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

      {activeTab ? (
        <TerminalPanel
          key={activeTab.id}
          mode={activeTab.mode}
          transport={transport}
          fontSize={fontSize}
          fontFamily={fontFamily}
          cursorStyle={cursorStyle}
          showModeToggle={showModeToggle}
          onModeChange={(mode) => setTabMode(activeTab.id, mode)}
        />
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
  showModeToggle: boolean;
  onModeChange: (mode: ConnectionMode) => void;
}

function TerminalPanel({
  mode,
  transport,
  fontSize,
  fontFamily,
  cursorStyle,
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
  const [terminalFontSize, setTerminalFontSize] = useState(fontSize);

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
    setTerminalFontSize((current) =>
      Math.min(18, Math.max(11, current + delta)),
    );
  }, []);

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
