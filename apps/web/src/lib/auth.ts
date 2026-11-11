/**
 * Web account API for KERNEXT.
 *
 * Sessions live in HttpOnly cookies, so every request uses
 * `credentials: 'include'`. Mutating requests echo the readable CSRF cookie in
 * the `X-CSRF-Token` header (double-submit). No tokens are kept in
 * localStorage.
 */

export interface AuthUser {
  id: string;
  full_name: string;
  email: string;
  email_verified: boolean;
  created_at: string;
  account_status: string;
}

export interface Preferences {
  theme: string;
  terminal_preferences: Record<string, unknown> | null;
}

export interface AuthStatus {
  accounts_enabled: boolean;
  email_configured: boolean;
  dev_mail: boolean;
}

export interface AuthSessionSummary {
  id: string;
  created_at: string;
  last_used_at: string;
  expires_at: string;
  current: boolean;
}

export interface MeResponse {
  user: AuthUser;
  preferences: Preferences;
  session_expires_at: string;
  csrf_token: string | null;
}

export interface RegisterResult {
  user: AuthUser;
  email_verification_required: boolean;
  email_delivery: string;
  dev_verification_token?: string;
}

export interface ForgotPasswordResult {
  message: string;
  email_delivery: string;
  dev_reset_token?: string;
}

export interface MessageResult {
  message: string;
}

export class AuthError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = 'AuthError';
    this.status = status;
    this.code = code;
  }
}

function apiBase(): string {
  if (import.meta.env.VITE_KERNEXT_SERVER) {
    return import.meta.env.VITE_KERNEXT_SERVER.replace(/\/+$/, '');
  }

  return window.__KERNEXT_SERVER__?.replace(/\/+$/, '') ?? '';
}

function csrfToken(): string {
  for (const cookie of document.cookie.split(';')) {
    const [name, ...rest] = cookie.trim().split('=');
    if (name === 'kernext_csrf') {
      return decodeURIComponent(rest.join('='));
    }
  }

  return '';
}

/**
 * The CSRF cookie is set and cleared together with the HttpOnly session
 * cookie (see `session_cookies` in `shell-server/src/db.rs`), so its presence
 * is a reliable hint that a session exists. Anonymous visitors can skip the
 * `/api/auth/me` probe entirely, which keeps the expected 401 out of the
 * browser console on every page load.
 */
export function hasSessionCookie(): boolean {
  return csrfToken() !== '';
}

async function request<T>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers = new Headers(init.headers);
  const method = (init.method ?? 'GET').toUpperCase();

  if (init.body) {
    headers.set('content-type', 'application/json');
  }

  if (method !== 'GET' && method !== 'HEAD') {
    headers.set('x-csrf-token', csrfToken());
  }

  const response = await fetch(`${apiBase()}${path}`, {
    ...init,
    credentials: 'include',
    headers,
  });

  if (!response.ok) {
    let body: { code?: string; message?: string } | null = null;

    try {
      body = (await response.json()) as { code?: string; message?: string };
    } catch {
      body = null;
    }

    if (!body || !body.code) {
      throw new AuthError(
        response.status,
        'backend_unreachable',
        response.status >= 500
          ? 'The account service did not answer. Start the KERNEXT server (cargo run -p shell-server) and try again.'
          : `The account service answered HTTP ${response.status}.`,
      );
    }

    throw new AuthError(
      response.status,
      body.code,
      body.message ?? 'The request failed.',
    );
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

export function fetchAuthStatus(): Promise<AuthStatus> {
  return request<AuthStatus>('/api/auth/status');
}

export function fetchMe(): Promise<MeResponse> {
  return request<MeResponse>('/api/auth/me');
}

export function signInRequest(
  email: string,
  password: string,
  remember: boolean,
): Promise<{ user: AuthUser; preferences: Preferences; expires_at: string }> {
  return request('/api/auth/login', {
    method: 'POST',
    body: JSON.stringify({ email, password, remember }),
  });
}

export function registerRequest(input: {
  fullName: string;
  email: string;
  password: string;
  acceptTerms: boolean;
}): Promise<RegisterResult> {
  return request<RegisterResult>('/api/auth/register', {
    method: 'POST',
    body: JSON.stringify({
      full_name: input.fullName,
      email: input.email,
      password: input.password,
      accept_terms: input.acceptTerms,
    }),
  });
}

export function signOutRequest(): Promise<void> {
  return request<void>('/api/auth/logout', { method: 'POST' });
}

export function updateProfileRequest(
  fullName: string,
  email: string,
): Promise<{
  user: AuthUser;
  email_verification_required: boolean;
  email_delivery: string;
  dev_verification_token?: string;
}> {
  return request('/api/auth/profile', {
    method: 'PATCH',
    body: JSON.stringify({ full_name: fullName, email }),
  });
}

export function updatePreferencesRequest(
  theme: string,
  terminalPreferences: Record<string, unknown> | null,
): Promise<Preferences> {
  return request<Preferences>('/api/auth/preferences', {
    method: 'PUT',
    body: JSON.stringify({
      theme,
      terminal_preferences: terminalPreferences,
    }),
  });
}

export function forgotPasswordRequest(
  email: string,
): Promise<ForgotPasswordResult> {
  return request<ForgotPasswordResult>('/api/auth/forgot-password', {
    method: 'POST',
    body: JSON.stringify({ email }),
  });
}

export function resetPasswordRequest(
  token: string,
  password: string,
): Promise<MessageResult> {
  return request<MessageResult>('/api/auth/reset-password', {
    method: 'POST',
    body: JSON.stringify({ token, password }),
  });
}

export function verifyEmailRequest(token: string): Promise<MessageResult> {
  return request<MessageResult>('/api/auth/verify-email', {
    method: 'POST',
    body: JSON.stringify({ token }),
  });
}

export function resendVerificationRequest(): Promise<{
  email_delivery: string;
  dev_verification_token?: string;
}> {
  return request('/api/auth/resend-verification', { method: 'POST' });
}

export function changePasswordRequest(
  currentPassword: string,
  newPassword: string,
): Promise<MessageResult> {
  return request<MessageResult>('/api/auth/change-password', {
    method: 'POST',
    body: JSON.stringify({
      current_password: currentPassword,
      new_password: newPassword,
    }),
  });
}

export async function listSessionsRequest(): Promise<AuthSessionSummary[]> {
  const body = await request<{ sessions: AuthSessionSummary[] }>(
    '/api/auth/sessions',
  );
  return body.sessions;
}

export function revokeSessionRequest(id: string): Promise<void> {
  return request<void>(`/api/auth/sessions/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}

export function isAuthError(error: unknown, code?: string): boolean {
  return (
    error instanceof AuthError && (code === undefined || error.code === code)
  );
}

export function describeAuthError(error: unknown): string {
  if (error instanceof AuthError) {
    return error.message;
  }

  if (error instanceof TypeError) {
    return 'The account service is not reachable. Start the KERNEXT server and try again.';
  }

  if (error instanceof Error) {
    return error.message;
  }

  return 'The request failed for an unknown reason.';
}
