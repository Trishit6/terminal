/**
 * Transport adapters for the terminal connection hook.
 *
 * The hook only cares about a tiny socket-like surface: send a client message,
 * observe open/message/error/close, and shut down cleanly. The WebSocket
 * adapter talks to the Axum server; the Tauri adapter talks to the desktop
 * runtime through IPC and never touches the network.
 */

import {
  desktopExecuteSession,
  desktopPtyClose,
  desktopPtyInterrupt,
  desktopPtyResize,
  desktopPtyWrite,
} from '../desktop';
import { listenEvent } from '../tauri';
import type { ClientMessage } from '../protocol';

export const SOCKET_CONNECTING = 0;
export const SOCKET_OPEN = 1;
export const SOCKET_CLOSING = 2;
export const SOCKET_CLOSED = 3;

export interface TerminalSocketEvent {
  data: unknown;
}

export interface TerminalSocket {
  readonly readyState: number;
  open(): void;
  send(message: ClientMessage): void;
  close(code?: number, reason?: string): void;
  onopen: (() => void) | null;
  onmessage: ((event: TerminalSocketEvent) => void) | null;
  onerror: (() => void) | null;
  onclose: (() => void) | null;
}

interface PtyOutputEvent {
  session_id: number;
  data: string;
}

interface PtyClosedEvent {
  session_id: number;
}

/** Adapter around the Axum WebSocket used by the web terminal. */
export class WebSocketTerminalSocket implements TerminalSocket {
  private socket: WebSocket | null = null;
  private state = SOCKET_CONNECTING;

  onopen: (() => void) | null = null;
  onmessage: ((event: TerminalSocketEvent) => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;

  constructor(private readonly url: string) {}

  get readyState(): number {
    return this.socket ? this.socket.readyState : this.state;
  }

  open(): void {
    if (this.socket) {
      return;
    }

    const socket = new WebSocket(this.url);
    socket.binaryType = 'arraybuffer';
    this.socket = socket;
    this.state = SOCKET_CONNECTING;

    socket.onopen = () => {
      this.state = SOCKET_OPEN;
      this.onopen?.();
    };

    socket.onmessage = (event) => {
      this.onmessage?.({ data: event.data });
    };

    socket.onerror = () => {
      this.onerror?.();
    };

    socket.onclose = () => {
      this.state = SOCKET_CLOSED;
      this.onclose?.();
    };
  }

  send(message: ClientMessage): void {
    this.socket?.send(JSON.stringify(message));
  }

  close(code?: number, reason?: string): void {
    const socket = this.socket;
    this.socket = null;
    this.state = SOCKET_CLOSED;

    if (socket && socket.readyState <= WebSocket.OPEN) {
      socket.close(code, reason);
    }
  }
}

/** Adapter around the desktop runtime's IPC commands and events. */
export class TauriTerminalSocket implements TerminalSocket {
  private state = SOCKET_CONNECTING;
  private listeners: Array<() => void> = [];

  onopen: (() => void) | null = null;
  onmessage: ((event: TerminalSocketEvent) => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;

  constructor(
    private readonly mode: 'engine' | 'pty',
    private readonly sessionId: number,
  ) {}

  get readyState(): number {
    return this.state;
  }

  async open(): Promise<void> {
    if (this.state !== SOCKET_CONNECTING) {
      return;
    }

    if (this.mode === 'pty') {
      await this.attachPtyListeners();
    }

    this.state = SOCKET_OPEN;
    this.onopen?.();
  }

  async send(message: ClientMessage): Promise<void> {
    if (this.state !== SOCKET_OPEN) {
      return;
    }

    try {
      if (this.mode === 'engine') {
        await this.sendEngineMessage(message);
      } else {
        await this.sendPtyMessage(message);
      }
    } catch (error) {
      this.onerror?.();
      throw error;
    }
  }

  async close(): Promise<void> {
    if (this.state === SOCKET_CLOSED) {
      return;
    }

    this.state = SOCKET_CLOSING;
    this.detachListeners();

    if (this.mode === 'pty') {
      await desktopPtyClose(this.sessionId).catch(() => undefined);
    }

    this.state = SOCKET_CLOSED;
    this.onclose?.();
  }

  private async attachPtyListeners(): Promise<void> {
    const output = await listenEvent<PtyOutputEvent>(
      'desktop-pty-output',
      (payload) => {
        if (payload.session_id !== this.sessionId) {
          return;
        }

        this.onmessage?.({ data: payload.data });
      },
    );

    const closed = await listenEvent<PtyClosedEvent>(
      'desktop-pty-closed',
      (payload) => {
        if (payload.session_id !== this.sessionId) {
          return;
        }

        this.state = SOCKET_CLOSED;
        this.detachListeners();
        this.onclose?.();
      },
    );

    this.listeners.push(output, closed);
  }

  private detachListeners(): void {
    const listeners = this.listeners;
    this.listeners = [];

    for (const unlisten of listeners) {
      unlisten();
    }
  }

  private async sendEngineMessage(message: ClientMessage): Promise<void> {
    if (message.type !== 'execute') {
      return;
    }

    const response = await desktopExecuteSession(
      this.sessionId,
      message.command,
    );

    if (response.output) {
      this.emitJson({
        type: 'output',
        stream: 'stdout',
        data: response.output,
      });
    }

    this.emitJson({
      type: 'result',
      success: response.success,
      exit_code: response.exit_code,
      action: response.action,
      session: response.session,
    });
  }

  private async sendPtyMessage(message: ClientMessage): Promise<void> {
    switch (message.type) {
      case 'input':
        await desktopPtyWrite(this.sessionId, message.data);
        break;
      case 'resize':
        await desktopPtyResize(this.sessionId, message.rows, message.cols);
        break;
      case 'interrupt':
        await desktopPtyInterrupt(this.sessionId);
        break;
      case 'clear':
        await desktopPtyWrite(this.sessionId, 'clear\r');
        break;
      case 'execute':
        await desktopPtyWrite(this.sessionId, `${message.command}\r`);
        break;
      case 'ping':
      default:
        break;
    }
  }

  private emitJson(message: unknown): void {
    this.onmessage?.({ data: JSON.stringify(message) });
  }
}
