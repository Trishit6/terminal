/**
 * KERNEXT administration API client.
 *
 * Every endpoint is authorized by the Rust server. This module only shapes
 * requests and responses; hiding a button is never treated as protection.
 * Sessions use the same HttpOnly cookie and CSRF double-submit as the account
 * API, so the shared `apiRequest` transport from `lib/auth.ts` is reused.
 */
import { apiRequest } from '@/lib/auth';

export type ServiceStatus =
  | 'healthy'
  | 'degraded'
  | 'unavailable'
  | 'unknown';

export interface AdminUserSummary {
  id: string;
  full_name: string;
  email: string;
  email_verified: boolean;
  created_at: string;
  account_status: 'active' | 'disabled' | string;
  role: 'user' | 'admin' | string;
  last_login_at: string | null;
  active_sessions: number;
}

export interface UserPage {
  users: AdminUserSummary[];
  total: number;
  page: number;
  per_page: number;
}

export interface AdminSessionSummary {
  id: string;
  user_agent: string | null;
  created_at: string;
  last_used_at: string;
  expires_at: string;
}

export interface UserDetailResponse {
  user: AdminUserSummary;
  sessions: AdminSessionSummary[];
}

export interface AuditEntry {
  id: string;
  event_type: string;
  actor_id: string | null;
  actor_email: string | null;
  target_user_id: string | null;
  target_label: string | null;
  metadata: Record<string, unknown> | null;
  ip_address: string | null;
  created_at: string;
}

export interface AuditPage {
  events: AuditEntry[];
  total: number;
  page: number;
  per_page: number;
}

export interface ServiceSummary {
  id: string;
  label: string;
  status: ServiceStatus;
}

export interface OverviewResponse {
  totals: {
    total_users: number;
    active_users: number;
    administrators: number;
    active_sessions: number;
    active_terminal_sessions: number;
  };
  services: ServiceSummary[];
  latest_registrations: AdminUserSummary[];
  recent_security_events: AuditEntry[];
  recent_system_events: AuditEntry[];
  server: ServerInfo;
  generated_at: string;
}

export interface ServerInfo {
  product: string;
  version: string;
  os: string;
  arch: string;
  uptime_seconds: number;
  started_at?: string;
}

export interface HealthCheck {
  id: string;
  label: string;
  status: ServiceStatus;
  detail: string;
  latency_ms?: number;
}

export interface HealthResponse {
  status: ServiceStatus;
  checks: HealthCheck[];
  server: ServerInfo;
  database: {
    configured: boolean;
    latency_ms?: number;
    migrations_applied: number | null;
    migrations_expected: number;
    migrations_latest?: number;
  };
  generated_at: string;
}

export interface TerminalSession {
  id: string;
  owner: string | null;
  mode: 'engine' | 'pty' | string;
  created_at: string;
  last_active_at: string;
  status: 'active' | 'stopped' | string;
  platform: string;
  owner_name: string | null;
  owner_email: string | null;
}

export interface UsersParams {
  search?: string;
  role?: string;
  status?: string;
  page?: number;
  per_page?: number;
}

export interface AuditParams {
  category?: string;
  event_type?: string;
  page?: number;
  per_page?: number;
}

/** Builds a query string from non-empty, defined parameters. */
export function buildAdminQuery(
  params: Record<string, string | number | undefined | null>,
): string {
  const search = new URLSearchParams();

  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === null) {
      continue;
    }
    const text = String(value).trim();
    if (text === '' || text === 'all') {
      continue;
    }
    search.set(key, text);
  }

  const query = search.toString();
  return query ? `?${query}` : '';
}

export function adminOverview(): Promise<OverviewResponse> {
  return apiRequest<OverviewResponse>('/api/admin/overview');
}

export function adminUsers(params: UsersParams = {}): Promise<UserPage> {
  return apiRequest<UserPage>(
    `/api/admin/users${buildAdminQuery({
      search: params.search,
      role: params.role,
      status: params.status,
      page: params.page,
      per_page: params.per_page,
    })}`,
  );
}

export function adminUser(id: string): Promise<UserDetailResponse> {
  return apiRequest<UserDetailResponse>(
    `/api/admin/users/${encodeURIComponent(id)}`,
  );
}

export function adminUpdateUser(
  id: string,
  input: { full_name: string; email: string },
): Promise<{ user: AdminUserSummary }> {
  return apiRequest(`/api/admin/users/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(input),
  });
}

export function adminSetStatus(
  id: string,
  status: 'active' | 'disabled',
): Promise<{ user: AdminUserSummary }> {
  return apiRequest(`/api/admin/users/${encodeURIComponent(id)}/status`, {
    method: 'POST',
    body: JSON.stringify({ status }),
  });
}

export function adminSetRole(
  id: string,
  role: 'user' | 'admin',
): Promise<{ user: AdminUserSummary }> {
  return apiRequest(`/api/admin/users/${encodeURIComponent(id)}/role`, {
    method: 'POST',
    body: JSON.stringify({ role }),
  });
}

export function adminUserSessions(
  id: string,
): Promise<{ sessions: AdminSessionSummary[] }> {
  return apiRequest(`/api/admin/users/${encodeURIComponent(id)}/sessions`);
}

export function adminRevokeSessions(
  id: string,
): Promise<{ revoked: number }> {
  return apiRequest(`/api/admin/users/${encodeURIComponent(id)}/sessions`, {
    method: 'DELETE',
  });
}

export function adminTerminalSessions(): Promise<{
  sessions: TerminalSession[];
}> {
  return apiRequest('/api/admin/terminal-sessions');
}

export function adminTerminateTerminal(id: string): Promise<void> {
  return apiRequest(
    `/api/admin/terminal-sessions/${encodeURIComponent(id)}`,
    { method: 'DELETE' },
  );
}

export function adminHealth(): Promise<HealthResponse> {
  return apiRequest<HealthResponse>('/api/admin/health');
}

export function adminAuditLogs(params: AuditParams = {}): Promise<AuditPage> {
  return apiRequest<AuditPage>(
    `/api/admin/audit-logs${buildAdminQuery({
      category: params.category,
      event_type: params.event_type,
      page: params.page,
      per_page: params.per_page,
    })}`,
  );
}
