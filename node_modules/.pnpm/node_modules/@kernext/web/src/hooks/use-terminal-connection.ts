import { useCallback, useEffect, useRef, useState } from 'react';
import type { TerminalEngine } from '@/lib/terminal/engine';
import {
  createSession,
  deleteSession,
  describeConnectionError,
  fetchHealth,
  wsUrl,
} from '@/lib/server';
import type {
  ClientMessage,
  HealthResponse,
  ServerMessage,
  SessionInfo,
} from '@/lib/protocol';

export type ConnectionMode = 'pty' | 'engine';
export type ConnectionStatus =
  | 'idle'
  | 'connecting'
  | 'open'
  | 'closed'
  | 'error';

export interface TerminalSize {
  rows: number;
  cols: number;
}

export interface UseTerminalConnectionOptions {
  engine: TerminalEngine;
  mode: ConnectionMode;
  enabled?: boolean;
}

export interface TerminalConnection {
  status: ConnectionStatus;
  error: string | null;
  session: SessionInfo | null;
  health: HealthResponse | null;
  size: TerminalSize;
  connect: () => void;
  disconnect: () => void;
  sendInput: (data: string) => void;
  resize: (rows: number, cols: number) => void;
  clear: () => void;
  interrupt: () => void;
  copyOutput: () => Promise<boolean>;
  refreshHealth: () => Promise<void>;
}

const DEFAULT_PROMPT = 'kernext ~ $ ';
const HISTORY_LIMIT = 200;

/**
 * Owns the WebSocket life cycle for both terminal transports.
 *
 * - `pty` relays raw terminal bytes to the real `kernext` process.
 * - `engine` executes command lines through the session WebSocket and keeps a
 *   small local line editor so typed text is echoed before the server answers.
 */
export function useTerminalConnection({
  engine,
  mode,
  enabled = true,
}: UseTerminalConnectionOptions): TerminalConnection {
  const [status, setStatus] = useState<ConnectionStatus>('idle');
  const [error, setError] = useState<string | null>(null);
  const [session, setSession] = useState<SessionInfo | null>(null);
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [size, setSize] = useState<TerminalSize>({ rows: 24, cols: 80 });

  const socketRef = useRef<WebSocket | null>(null);
  const sessionIdRef = useRef<number | null>(null);
  const decoderRef = useRef<TextDecoder | null>(null);
  const sizeRef = useRef<TerminalSize>(size);
  const modeRef = useRef<ConnectionMode>(mode);
  const generationRef = useRef(0);
  const lineRef = useRef('');
  const promptRef = useRef(DEFAULT_PROMPT);
  const historyRef = useRef<string[]>([]);
  const historyIndexRef = useRef(0);
  const lastOutputRef = useRef('');

  modeRef.current = mode;

  const sendClient = useCallback((message: ClientMessage): boolean => {
    const socket = socketRef.current;

    if (!socket || socket.readyState !== WebSocket.OPEN) {
      return false;
    }

    socket.send(JSON.stringify(message));
    return true;
  }, []);

  const closeSocket = useCallback((deleteRemote: boolean) => {
    const socket = socketRef.current;
    socketRef.current = null;

    if (socket && socket.readyState <= WebSocket.OPEN) {
      socket.close(1000, 'client closing');
    }

    if (deleteRemote && sessionIdRef.current !== null) {
      const id = sessionIdRef.current;
      sessionIdRef.current = null;
      void deleteSession(id).catch(() => {
        // The server may already have stopped the session. Cleanup is best
        // effort and must never break the UI.
      });
    }
  }, []);

  const openPty = useCallback(
    (nextHealth: HealthResponse) => {
      if (nextHealth.pty_available === false) {
        throw new Error(
          'The server does not expose a PTY endpoint. Build the CLI with `cargo build -p shell-cli` and restart the server.',
        );
      }

      const socket = new WebSocket(wsUrl('/api/v1/pty'));
      socket.binaryType = 'arraybuffer';
      socketRef.current = socket;
      decoderRef.current = new TextDecoder('utf-8', { fatal: false });

      socket.onopen = () => {
        if (socketRef.current !== socket) {
          return;
        }

        setStatus('open');
        setError(null);
        sendClient({
          type: 'resize',
          rows: sizeRef.current.rows,
          cols: sizeRef.current.cols,
        });
      };

      socket.onmessage = (event) => {
        if (socketRef.current !== socket) {
          return;
        }

        if (typeof event.data === 'string') {
          let message: ServerMessage;

          try {
            message = JSON.parse(event.data) as ServerMessage;
          } catch {
            return;
          }

          switch (message.type) {
            case 'pty_ready':
              promptRef.current = message.prompt;
              break;
            case 'error':
              setError(message.message);
              engine.write(
                `\r\n\u001b[31m${message.message}\u001b[0m\r\n`,
              );
              break;
            case 'session_closed':
              setStatus('closed');
              break;
            default:
              break;
          }

          return;
        }

        void decodeBinary(decoderRef.current, event.data, engine);
      };

      socket.onerror = () => {
        if (socketRef.current !== socket) {
          return;
        }

        setError(
          'The PTY connection failed. Check that the KERNEXT server is running and that `kernext` was built.',
        );
        setStatus('error');
      };

      socket.onclose = () => {
        if (socketRef.current !== socket) {
          return;
        }

        socketRef.current = null;
        setStatus((previous) => (previous === 'error' ? previous : 'closed'));
      };
    },
    [engine, sendClient],
  );

  const openEngine = useCallback(
    async (generation: number) => {
      const response = await createSession();

      if (generation !== generationRef.current) {
        if (response.session) {
          void deleteSession(response.session.id).catch(() => {});
        }
        return;
      }

      if (!response.success || !response.session) {
        throw new Error(response.error ?? 'The server did not create a session.');
      }

      const created = response.session;
      sessionIdRef.current = created.id;
      setSession(created);
      promptRef.current = created.prompt;
      lineRef.current = '';
      historyIndexRef.current = historyRef.current.length;
      lastOutputRef.current = '';

      const socket = new WebSocket(
        wsUrl(`/api/v1/sessions/${created.id}/ws`),
      );
      socketRef.current = socket;

      let finished = false;

      const finish = (message: string) => {
        if (finished) {
          return;
        }

        finished = true;
        setStatus('closed');
        setSession(null);

        if (sessionIdRef.current !== null) {
          const id = sessionIdRef.current;
          sessionIdRef.current = null;
          void deleteSession(id).catch(() => {});
        }

        engine.write(`\r\n\u001b[2m${message}\u001b[0m\r\n`);
      };

      const handleText = (text: string) => {
        let message: ServerMessage;

        try {
          message = JSON.parse(text) as ServerMessage;
        } catch {
          return;
        }

        switch (message.type) {
          case 'ready': {
            setSession(message.session);
            promptRef.current = message.prompt;
            engine.write(message.prompt);
            break;
          }
          case 'output': {
            if (message.stream === 'system' && message.data.includes('\u001b[2J')) {
              engine.clear();
            }

            engine.write(message.data);
            lastOutputRef.current = (lastOutputRef.current + message.data).slice(-4096);
            break;
          }
          case 'result': {
            setSession(message.session);
            promptRef.current = message.session.prompt;

            if (message.action === 'clear') {
              engine.clear();
              engine.write(message.session.prompt);
              lastOutputRef.current = '';
              break;
            }

            if (message.action === 'exit') {
              finish('KERNEXT session closed.');
              break;
            }

            if (lastOutputRef.current && !lastOutputRef.current.endsWith('\n')) {
              engine.write('\r\n');
            }

            engine.write(message.session.prompt);
            lastOutputRef.current = '';
            break;
          }
          case 'error': {
            setError(message.message);
            engine.write(`\r\n\u001b[31m${message.message}\u001b[0m\r\n`);
            engine.write(promptRef.current);
            break;
          }
          case 'session_closed': {
            finish('KERNEXT session closed.');
            break;
          }
          case 'pong':
          case 'pty_ready':
          default:
            break;
        }
      };

      socket.onopen = () => {
        if (socketRef.current !== socket) {
          return;
        }

        setStatus('open');
        setError(null);
      };

      socket.onmessage = (event) => {
        if (socketRef.current !== socket) {
          return;
        }

        if (typeof event.data === 'string') {
          handleText(event.data);
        }
      };

      socket.onerror = () => {
        if (socketRef.current !== socket) {
          return;
        }

        setError(
          'The session WebSocket failed. Check that the KERNEXT server is still running.',
        );
        setStatus('error');
      };

      socket.onclose = () => {
        if (socketRef.current !== socket) {
          return;
        }

        socketRef.current = null;
        setStatus((previous) => (previous === 'error' ? previous : 'closed'));

        if (sessionIdRef.current !== null) {
          const id = sessionIdRef.current;
          sessionIdRef.current = null;
          void deleteSession(id).catch(() => {});
        }
      };
    },
    [engine],
  );

  const connect = useCallback(() => {
    generationRef.current += 1;
    const generation = generationRef.current;

    closeSocket(true);
    engine.reset();
    setStatus('connecting');
    setError(null);
    setSession(null);
    lineRef.current = '';
    historyIndexRef.current = historyRef.current.length;
    lastOutputRef.current = '';

    void (async () => {
      try {
        const nextHealth = await fetchHealth();

        if (generation !== generationRef.current) {
          return;
        }

        setHealth(nextHealth);

        if (modeRef.current === 'pty') {
          openPty(nextHealth);
        } else {
          await openEngine(generation);
        }
      } catch (caught) {
        if (generation !== generationRef.current) {
          return;
        }

        setError(describeConnectionError(caught));
        setStatus('error');
      }
    })();
  }, [closeSocket, engine, openEngine, openPty]);

  const disconnect = useCallback(() => {
    generationRef.current += 1;
    closeSocket(true);
    setStatus('closed');
    setSession(null);
  }, [closeSocket]);

  useEffect(() => {
    if (!enabled) {
      closeSocket(true);
      setStatus('idle');
      return undefined;
    }

    connect();

    return () => {
      generationRef.current += 1;
      closeSocket(true);
    };
  }, [enabled, mode, connect, closeSocket]);

  const submitLine = useCallback(
    (command: string) => {
      lineRef.current = '';
      engine.write('\r\n');

      if (command.trim() === '') {
        engine.write(promptRef.current);
        return;
      }

      const history = historyRef.current;

      if (history[history.length - 1] !== command) {
        history.push(command);

        if (history.length > HISTORY_LIMIT) {
          history.shift();
        }
      }

      historyIndexRef.current = history.length;
      lastOutputRef.current = '';

      if (!sendClient({ type: 'execute', command })) {
        engine.write('\u001b[31m[not connected]\u001b[0m\r\n');
        engine.write(promptRef.current);
      }
    },
    [engine, sendClient],
  );

  const redrawLine = useCallback(
    (next: string) => {
      lineRef.current = next;
      engine.write(`\r\u001b[2K${promptRef.current}${next}`);
    },
    [engine],
  );

  const handleEngineInput = useCallback(
    (data: string) => {
      let index = 0;

      while (index < data.length) {
        if (data.startsWith('\u001b[A', index)) {
          const history = historyRef.current;

          if (history.length > 0) {
            const next = Math.max(0, historyIndexRef.current - 1);
            historyIndexRef.current = next;
            redrawLine(history[next] ?? '');
          }

          index += 3;
          continue;
        }

        if (data.startsWith('\u001b[B', index)) {
          const history = historyRef.current;

          if (history.length > 0) {
            const next = Math.min(history.length, historyIndexRef.current + 1);
            historyIndexRef.current = next;
            redrawLine(next === history.length ? '' : (history[next] ?? ''));
          }

          index += 3;
          continue;
        }

        const escape = /^\u001b(?:\[[0-9;?]*[A-Za-z~]|O[A-Za-z])/.exec(
          data.slice(index),
        );

        if (escape) {
          index += escape[0].length;
          continue;
        }

        const codePoint = data.codePointAt(index) ?? 0;
        const character = String.fromCodePoint(codePoint);
        index += character.length;

        if (character === '\r' || character === '\n') {
          submitLine(lineRef.current);
          continue;
        }

        if (character === '\u007f' || character === '\u0008') {
          const line = lineRef.current;

          if (!line) {
            continue;
          }

          const characters = Array.from(line);
          characters.pop();
          lineRef.current = characters.join('');
          engine.write('\b \b');
          continue;
        }

        if (character === '\u0003') {
          lineRef.current = '';
          engine.write('^C\r\n');
          engine.write(promptRef.current);
          continue;
        }

        if (character === '\u000c') {
          engine.clear();
          engine.write(promptRef.current);
          continue;
        }

        if (character === '\t') {
          lineRef.current += '\t';
          engine.write('\t');
          continue;
        }

        if (codePoint < 0x20) {
          continue;
        }

        lineRef.current += character;
        engine.write(character);
      }
    },
    [engine, redrawLine, submitLine],
  );

  const sendInput = useCallback(
    (data: string) => {
      if (modeRef.current === 'pty') {
        sendClient({ type: 'input', data });
        return;
      }

      handleEngineInput(data);
    },
    [handleEngineInput, sendClient],
  );

  const resize = useCallback(
    (rows: number, cols: number) => {
      const previous = sizeRef.current;
      sizeRef.current = { rows, cols };
      setSize((current) =>
        current.rows === rows && current.cols === cols
          ? current
          : { rows, cols },
      );

      if (
        previous.rows === rows &&
        previous.cols === cols
      ) {
        return;
      }

      if (modeRef.current === 'pty') {
        sendClient({ type: 'resize', rows, cols });
      }
    },
    [sendClient],
  );

  const clear = useCallback(() => {
    if (modeRef.current === 'pty') {
      engine.clear();
      sendClient({ type: 'clear' });
      return;
    }

    engine.clear();
    engine.write(promptRef.current);
  }, [engine, sendClient]);

  const interrupt = useCallback(() => {
    if (modeRef.current === 'pty') {
      sendClient({ type: 'interrupt' });
      return;
    }

    lineRef.current = '';
    engine.write('^C\r\n');
    engine.write(promptRef.current);
  }, [engine, sendClient]);

  const copyOutput = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(engine.getText());
      return true;
    } catch {
      return false;
    }
  }, [engine]);

  const refreshHealth = useCallback(async () => {
    try {
      const nextHealth = await fetchHealth();
      setHealth(nextHealth);
      setError(null);
    } catch (caught) {
      setHealth(null);
      setError(describeConnectionError(caught));
    }
  }, []);

  return {
    status,
    error,
    session,
    health,
    size,
    connect,
    disconnect,
    sendInput,
    resize,
    clear,
    interrupt,
    copyOutput,
    refreshHealth,
  };
}

async function decodeBinary(
  fallback: TextDecoder | null,
  data: ArrayBuffer | Blob | string,
  engine: TerminalEngine,
): Promise<void> {
  if (typeof data === 'string') {
    engine.write(data);
    return;
  }

  const buffer = data instanceof Blob ? await data.arrayBuffer() : data;
  const decoder = fallback ?? new TextDecoder('utf-8', { fatal: false });
  const text = decoder.decode(new Uint8Array(buffer), { stream: true });

  if (text) {
    engine.write(text);
  }
}
