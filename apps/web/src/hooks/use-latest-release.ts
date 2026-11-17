import { useEffect, useState } from 'react';
import { fetchLatestRelease, type LatestRelease } from '@/lib/releases';

/** Lifecycle of the downloads page's release lookup. */
export type ReleaseState =
  | { status: 'loading' }
  | { status: 'ready'; release: LatestRelease }
  | { status: 'error'; message: string };

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
        if (!active) {
          return;
        }

        setState({
          status: 'error',
          message:
            error instanceof Error
              ? error.message
              : 'The release information could not be loaded.',
        });
      });

    return () => {
      active = false;
      controller.abort();
    };
  }, []);

  return state;
}
