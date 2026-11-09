/**
 * Wire types shared with `crates/shell-api`.
 *
 * Keep these in sync with the Rust serde representation. The server is the
 * source of truth; these types exist so the web terminal never guesses at the
 * shape of a command result.
 */

export type SessionId = number;

export interface SessionInfo {
  id: SessionId;
  working_directory: string;
  state: 'running' | 'stopped' | string;
  variables: Record<string, string>;
  job_count: number;
  platform: string;
  version: string;
  prompt: string;
}

export interface CreateSessionResponse {
  success: boolean;
  session?: SessionInfo;
  error?: string;
}

export interface ExecuteCommandResponse {
  success: boolean;
  output: string;
  exit_code?: number;
  action: 'continue' | 'clear' | 'clear_history' | 'exit' | string;
  session: SessionInfo;
}

export interface SessionListResponse {
  sessions: SessionInfo[];
}

export interface HealthResponse {
  status: string;
  product: string;
  version: string;
  api_version: string;
  sessions: number;
  platform?: string;
  pty_available?: boolean;
}

export interface ApiErrorBody {
  code: string;
  message: string;
}

export type OutputStream = 'stdout' | 'stderr' | 'system';

export type ClientMessage =
  | { type: 'execute'; command: string }
  | { type: 'input'; data: string }
  | { type: 'resize'; rows: number; cols: number }
  | { type: 'interrupt' }
  | { type: 'clear' }
  | { type: 'ping' };

export type ServerMessage =
  | { type: 'ready'; session: SessionInfo; prompt: string }
  | { type: 'pty_ready'; prompt: string }
  | { type: 'output'; stream: OutputStream; data: string }
  | {
      type: 'result';
      success: boolean;
      exit_code?: number;
      action: string;
      session: SessionInfo;
    }
  | { type: 'error'; message: string }
  | { type: 'pong' }
  | { type: 'session_closed' };

export const OUTPUT_STREAM = {
  stdout: 'stdout',
  stderr: 'stderr',
  system: 'system',
} as const;
