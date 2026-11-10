/**
 * Thin helpers around the Tauri 2 APIs.
 *
 * The web build imports these helpers only when it is actually running inside
 * a Tauri webview, so the browser bundle never depends on the desktop runtime.
 */

export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

export async function listenEvent<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<() => void> {
  const { listen } = await import('@tauri-apps/api/event');
  return listen<T>(event, (payload) => handler(payload.payload));
}
