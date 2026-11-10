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
import { motion } from 'motion/react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { TerminalScreen } from '@/components/terminal/terminal-screen';
import { useTerminalConnection, type ConnectionMode, type ConnectionStatus, type ConnectionTransport } from '@/hooks/use-terminal-connection';
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

  const handleCopy = useCallback(async () => {
    const copied = await connection.copyOutput();
    setCopied(copied);

    if (copied) {
      window.setTimeout(() => setCopied(false), 1500);
    }
  }, [connection]);

  const status = connection.status;
  const statusLabel = status.charAt(0).toUpperCase() + status.slice(1);
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

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <div className="flex items-center gap-2 rounded-lg border border-border bg-surface/80 px-3 py-1.5 text-xs">
          <span className="inline-flex items-center gap-1.5">
            {statusIcon}
            <span className="font-medium">{statusLabel}</span>
          </span>
          <span className="text-muted-foreground">·</span>
          <span className="font-mono text-[11px] text-muted-foreground">
            {connection.session?.working_directory ?? '—'}
          </span>
        </div>

        {showModeToggle ? (
          <div className="inline-flex overflow-hidden rounded-lg border border-border">
            {(['pty', 'engine'] as const).map((value) => (
              <button
                key={value}
                type="button"
                onClick={() => onModeChange(value)}
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
              Connect
            </Button>
          )}
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={connection.clear}
            className="gap-1.5"
          >
            <Square className="size-3.5" />
            Clear
          </Button>
          <Button
            type="button"
            variant="outline"
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
            variant="outline"
            size="sm"
            onClick={connection.interrupt}
            className="gap-1.5"
          >
            <CircleSlash className="size-3.5" />
            Interrupt
          </Button>
        </div>
      </div>

      {connection.error ? (
        <div className="rounded-lg border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger">
          {connection.error}
        </div>
      ) : null}

      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.25, ease: 'easeOut' }}
        className="min-h-0 flex-1"
      >
        <TerminalScreen
          engine={engine}
          version={version}
          onData={connection.sendInput}
          onResize={connection.resize}
          fontSize={fontSize}
          fontFamily={fontFamily}
          cursorStyle={cursorStyle}
          autoFocus
          className="h-full"
        />
      </motion.div>

      <div className="flex items-center justify-between text-[11px] text-muted-foreground">
        <span className="inline-flex items-center gap-1.5 font-mono">
          {connection.size.rows} × {connection.size.cols}
        </span>
        <span className="inline-flex items-center gap-1.5 font-mono">
          {connection.health?.version ? `v${connection.health.version}` : ''}
          {connection.health?.platform ? ` · ${connection.health.platform}` : ''}
        </span>
      </div>
    </div>
  );
}

export function TerminalStatusBadge({ status }: { status: ConnectionStatus }) {
  return (
    <Badge variant={status === 'open' ? 'success' : status === 'error' ? 'danger' : 'secondary'}>
      {status}
    </Badge>
  );
}
