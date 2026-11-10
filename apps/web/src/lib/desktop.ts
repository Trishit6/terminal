/**
 * Typed wrappers around the desktop-only Tauri commands.
 *
 * These calls never leave the local Rust runtime. They exist so the shared
 * frontend can drive the desktop shell without depending on the Axum server.
 */

import { invokeCommand } from './tauri';
import type {
  ExecuteCommandResponse,
  HealthResponse,
  SessionInfo,
} from './protocol';

export type SessionKind = 'engine' | 'pty';

export interface DesktopSessionSummary {
  id: number;
  kind: SessionKind;
  working_directory: string;
  state: string;
  platform: string;
  version: string;
  prompt: string;
  job_count: number;
}

export interface RecentCommand {
  command: string;
  exit_code?: number | null;
  timestamp: string;
}

export interface DesktopDashboard {
  version: string;
  platform: string;
  working_directory: string;
  sessions: DesktopSessionSummary[];
  recent_commands: RecentCommand[];
  pty_available: boolean;
  uptime_seconds: number;
}

export interface DirectoryEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size?: number | null;
  modified?: string | null;
}

export interface DirectoryListing {
  path: string;
  entries: DirectoryEntry[];
  total: number;
}

export type CommandCategory =
  | 'core_shell'
  | 'filesystem'
  | 'text_processing'
  | 'system'
  | 'development'
  | 'networking';

export type CommandKind = 'builtin' | 'external';

export interface CommandCatalogEntry {
  name: string;
  description: string;
  category: CommandCategory;
  kind: CommandKind;
  available: boolean;
  usage: string[];
  platforms: string[];
}

export interface DesktopSettings {
  theme: 'midnight' | 'light' | 'system';
  terminal_font_size: number;
  terminal_font_family: string;
  terminal_cursor_style: 'block' | 'bar' | 'underline';
  default_working_directory: string;
  history_enabled: boolean;
  history_size: number;
  startup_animation: boolean;
  reduce_motion: boolean;
  accent_color: 'cyan' | 'blue' | 'violet';
}

export function desktopHealth(): Promise<HealthResponse> {
  return invokeCommand<HealthResponse>('desktop_health');
}

export function desktopDashboard(): Promise<DesktopDashboard> {
  return invokeCommand<DesktopDashboard>('desktop_dashboard');
}

export function desktopListSessions(): Promise<DesktopSessionSummary[]> {
  return invokeCommand<DesktopSessionSummary[]>('desktop_list_sessions');
}

export function desktopCreateSession(
  workingDirectory?: string,
): Promise<SessionInfo> {
  return invokeCommand<SessionInfo>('desktop_create_session', {
    request: workingDirectory ? { working_directory: workingDirectory } : {},
  });
}

export function desktopExecuteSession(
  sessionId: number,
  command: string,
): Promise<ExecuteCommandResponse> {
  return invokeCommand<ExecuteCommandResponse>('desktop_execute_session', {
    sessionId,
    command,
  });
}

export function desktopDeleteSession(sessionId: number): Promise<boolean> {
  return invokeCommand<boolean>('desktop_delete_session', { sessionId });
}

export function desktopCreatePtySession(
  rows: number,
  cols: number,
): Promise<DesktopSessionSummary> {
  return invokeCommand<DesktopSessionSummary>('desktop_create_pty_session', {
    rows,
    cols,
  });
}

export function desktopPtyWrite(
  sessionId: number,
  data: string,
): Promise<void> {
  return invokeCommand<void>('desktop_pty_write', { sessionId, data });
}

export function desktopPtyResize(
  sessionId: number,
  rows: number,
  cols: number,
): Promise<void> {
  return invokeCommand<void>('desktop_pty_resize', {
    sessionId,
    rows,
    cols,
  });
}

export function desktopPtyInterrupt(sessionId: number): Promise<void> {
  return invokeCommand<void>('desktop_pty_interrupt', { sessionId });
}

export function desktopPtyClose(sessionId: number): Promise<boolean> {
  return invokeCommand<boolean>('desktop_pty_close', { sessionId });
}

export function desktopFsList(path?: string): Promise<DirectoryListing> {
  return invokeCommand<DirectoryListing>('desktop_fs_list', {
    path: path ?? null,
  });
}

export function desktopFsCreateDirectory(path: string): Promise<void> {
  return invokeCommand<void>('desktop_fs_create_directory', { path });
}

export function desktopFsRename(from: string, to: string): Promise<void> {
  return invokeCommand<void>('desktop_fs_rename', { from, to });
}

export function desktopFsDelete(path: string, recursive: boolean): Promise<void> {
  return invokeCommand<void>('desktop_fs_delete', { path, recursive });
}

export function desktopFsOpen(path: string): Promise<void> {
  return invokeCommand<void>('desktop_fs_open', { path });
}

export function desktopFsSessionDirectory(
  sessionId: number,
  kind: SessionKind,
): Promise<string> {
  return invokeCommand<string>('desktop_fs_session_directory', {
    sessionId,
    kind,
  });
}

export function desktopCommandCatalog(): Promise<CommandCatalogEntry[]> {
  return invokeCommand<CommandCatalogEntry[]>('desktop_command_catalog');
}

export function desktopSettingsGet(): Promise<DesktopSettings> {
  return invokeCommand<DesktopSettings>('desktop_settings_get');
}

export function desktopSettingsSave(
  settings: DesktopSettings,
): Promise<DesktopSettings> {
  return invokeCommand<DesktopSettings>('desktop_settings_save', {
    settings,
  });
}

export function sessionSummaryToSessionInfo(
  summary: DesktopSessionSummary,
): SessionInfo {
  return {
    id: summary.id,
    working_directory: summary.working_directory,
    state: summary.state,
    variables: {},
    job_count: summary.job_count,
    platform: summary.platform,
    version: summary.version,
    prompt: summary.prompt,
  };
}
