import { useEffect, useState } from 'react';

const QUERY = '(prefers-reduced-motion: reduce)';

function currentPreference(): boolean {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
    return false;
  }

  return window.matchMedia(QUERY).matches;
}

/**
 * Tracks the operating system's Reduced Motion preference.
 *
 * This deliberately reads `matchMedia` directly instead of using Motion's
 * `useReducedMotion` hook: that hook logs a development warning whenever the
 * preference is enabled, even inside a `MotionConfig` that sets
 * `reducedMotion="never"`. Components that need to branch on the preference
 * use this hook so the warning stays out of the console.
 */
export function usePrefersReducedMotion(): boolean {
  const [reduced, setReduced] = useState(currentPreference);

  useEffect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
      return;
    }

    const media = window.matchMedia(QUERY);
    const update = () => setReduced(media.matches);

    update();
    media.addEventListener('change', update);

    return () => {
      media.removeEventListener('change', update);
    };
  }, []);

  return reduced;
}
