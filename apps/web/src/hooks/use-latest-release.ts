import { useEffect, useState } from 'react';
import {
  detectVisitorPlatform,
  fetchLatestRelease,
  fetchReleaseList,
  type LatestRelease,
  type ReleaseList,
  type VisitorPlatform,
} from '@/lib/releases';

/** Lifecycle of the downloads page's release lookup. */
export type ReleaseState =
  | { status: 'loading' }
  | { status: 'ready'; release: LatestRelease }
  | { status: 'error'; message: string };

/** Lifecycle of the downloads page's release-history lookup. */
export type ReleaseListState =
  | { status: 'loading' }
  | { status: 'ready'; list: ReleaseList }
  | { status: 'error'; message: string };

function messageOf(error: unknown): string {
  return error instanceof Error
    ? error.message
    : 'The release information could not be loaded.';
}

/**
 * Loads the latest release once on mount.
 *
 * The request is aborted on unmount so a slow GitHub lookup cannot resolve
 * into a component that is already gone. Errors are surfaced as state, never
 * thrown, because the page must render a real message either way.
 */
export function useLatestRelease(): ReleaseState {
  const [state, setState] = useState<ReleaseState>({ status: 'loading' });

  useEffect(() => {
    const controller = new AbortController();
    let active = true;

    fetchLatestRelease(controller.signal)
      .then((release) => {
        if (active) {
          setState({ status: 'ready', release });
        }
      })
      .catch((error: unknown) => {
        if (active) {
          setState({ status: 'error', message: messageOf(error) });
        }
      });

    return () => {
      active = false;
      controller.abort();
    };
  }, []);

  return state;
}

/**
 * Loads the release history once on mount.
 *
 * The server serves both endpoints from one cached GitHub lookup, so this
 * second request does not double the API traffic.
 */
export function useReleaseList(): ReleaseListState {
  const [state, setState] = useState<ReleaseListState>({ status: 'loading' });

  useEffect(() => {
    const controller = new AbortController();
    let active = true;

    fetchReleaseList(controller.signal)
      .then((list) => {
        if (active) {
          setState({ status: 'ready', list });
        }
      })
      .catch((error: unknown) => {
        if (active) {
          setState({ status: 'error', message: messageOf(error) });
        }
      });

    return () => {
      active = false;
      controller.abort();
    };
  }, []);

  return state;
}

/**
 * Best-effort visitor platform, resolved in the browser only.
 *
 * Returns empty values while rendering outside a browser so the page never
 * depends on `navigator` existing.
 */
export function useVisitorPlatform(): VisitorPlatform {
  const [value, setValue] = useState<VisitorPlatform>({ platform: null, arch: null });

  useEffect(() => {
    setValue(detectVisitorPlatform());
  }, []);

  return value;
}
