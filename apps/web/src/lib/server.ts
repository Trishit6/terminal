import type {
  ApiErrorBody,
  CreateSessionResponse,
  ExecuteCommandResponse,
  HealthResponse,
  SessionInfo,
  SessionListResponse,
} from './protocol';

const TOKEN_KEY = 'kernext.server.token';

export class ApiError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
  }
}

export function getServerToken(): string {
  try {
    return window.localStorage.getItem(TOKEN_KEY) ?? '';
  } catch {
    return '';
  }
}

export function setServerToken(token: string): void {
  try {
    if (token) {
      window.localStorage.setItem(TOKEN_KEY, token);
    } else {
      window.localStorage.removeItem(TOKEN_KEY);
    }
  } catch {
    // Storage can be blocked in hardened browser contexts. The terminal still
    // works with an unauthenticated local server.
  }
}

/**
 * Base URL for HTTP requests.
 *
 * - Vite dev/preview uses relative URLs and the configured `/api` proxy.
 * - Tauri loads the UI from `tauri://localhost`, so it must reach the loopback
 *   server explicitly.
 * - `VITE_KERNEXT_SERVER` or `window.__KERNEXT_SERVER__` overrides both.
 */
export function serverHttpBase(): string {
  const configured =
    import.meta.env.VITE_KERNEXT_SERVER ?? window.__KERNEXT_SERVER__ ?? '';

  if (configured) {
    return configured.replace(/\/+$/, '');
  }

  if (window.__TAURI_INTERNALS__) {
    return 'http://127.0.0.1:7878';
  }

  return '';
}

export function serverWsBase(): string {
  const base = serverHttpBase();

  if (base) {
    return base.replace(/^http/i, 'ws');
  }

  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${window.location.host}`;
}

export function apiUrl(path: string): string {
  return `${serverHttpBase()}${path}`;
}

export function wsUrl(path: string): string {
  const token = getServerToken();
  const url = `${serverWsBase()}${path}`;

  if (!token) {
    return url;
  }

  const separator = url.includes('?') ? '&' : '?';
  return `${url}${separator}token=${encodeURIComponent(token)}`;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = getServerToken();
  const headers = new Headers(init?.headers);

  if (init?.body) {
    headers.set('content-type', 'application/json');
  }

  if (token) {
    headers.set('authorization', `Bearer ${token}`);
  }

  const response = await fetch(apiUrl(path), {
    ...init,
    // Account mode authenticates terminal requests with the HttpOnly session
    // cookie. Bearer tokens remain supported for local-only deployments.
    credentials: 'include',
    headers,
  });

  if (!response.ok) {
    let body: ApiErrorBody | null = null;
    let parsed = true;

    try {
      body = (await response.json()) as ApiErrorBody;
    } catch {
      body = null;
      parsed = false;
    }

    // KERNEXT error responses always carry a JSON body with a stable code.
    // An error response without one did not come from the KERNEXT server:
    // when the site is served by the Vite dev server, its `/api` proxy
    // answers HTTP 500 with an empty body whenever the backend is not
    // running. Report that honestly and actionably instead of a bare
    // "HTTP 500".
    if (!parsed || !body) {
      throw new ApiError(
        response.status,
        'backend_unreachable',
        response.status >= 500
          ? 'The request did not reach the KERNEXT server. When the site runs through the Vite dev server this usually means the backend is not running yet — start it with `cargo run -p shell-server` or `npm run dev`, then connect again.'
          : `The server answered HTTP ${response.status} without a KERNEXT error body.`,
      );
    }

    throw new ApiError(
      response.status,
      body.code ?? 'request_failed',
      body.message ?? `The server answered with HTTP ${response.status}.`,
    );
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

export function fetchHealth(): Promise<HealthResponse> {
  return request<HealthResponse>('/api/v1/health');
}

export function createSession(workingDirectory?: string): Promise<CreateSessionResponse> {
  return request<CreateSessionResponse>('/api/v1/sessions', {
    method: 'POST',
    body: JSON.stringify(workingDirectory ? { working_directory: workingDirectory } : {}),
  });
}

export function listSessions(): Promise<SessionListResponse> {
  return request<SessionListResponse>('/api/v1/sessions');
}

export function getSession(id: number): Promise<SessionInfo> {
  return request<SessionInfo>(`/api/v1/sessions/${id}`);
}

export function deleteSession(id: number): Promise<void> {
  return request<void>(`/api/v1/sessions/${id}`, { method: 'DELETE' });
}

export function executeCommand(
  id: number,
  command: string,
  options?: { workingDirectory?: string },
): Promise<ExecuteCommandResponse> {
  return request<ExecuteCommandResponse>(`/api/v1/sessions/${id}/execute`, {
    method: 'POST',
    body: JSON.stringify({
      command,
      ...(options?.workingDirectory
        ? { working_directory: options.workingDirectory }
        : {}),
    }),
  });
}

export function describeConnectionError(error: unknown): string {
  if (error instanceof ApiError) {
    return `${error.message} (${error.code})`;
  }

  if (error instanceof TypeError) {
    return 'The KERNEXT server is not reachable. Start it with `cargo run -p shell-server` and try again.';
  }

  if (error instanceof Error) {
    return error.message;
  }

  return 'The connection failed for an unknown reason.';
}
