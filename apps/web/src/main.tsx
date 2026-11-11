import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { RouterProvider } from '@tanstack/react-router';
import { MotionConfig } from 'motion/react';
import { AuthProvider } from './components/auth/auth-provider';
import { router } from './router';
import './styles.css';

/**
 * `index.html` carries fallback SEO tags for non-JS clients and for the
 * instant first paint (the web server also patches them per route). Once
 * React mounts, `<HeadContent />` owns the head and renders the current
 * route's metadata, so the statically marked duplicates are removed to keep
 * exactly one set of tags.
 */
function removeStaticHeadTags(): void {
  document
    .querySelectorAll('head [data-kernext-static]')
    .forEach((element) => element.remove());
}

const container = document.getElementById('root');

if (!container) {
  throw new Error('KERNEXT frontend root element is missing');
}

removeStaticHeadTags();

createRoot(container).render(
  <StrictMode>
    {/* Motion accessibility configuration (motion.dev's documented fix for
        the "You have Reduced Motion enabled on your device" warning):
        production respects the OS-level Reduced Motion preference, while
        development keeps animations running so they can be reviewed locally.
        See https://motion.dev/troubleshooting/reduced-motion-disabled */}
    <MotionConfig reducedMotion={import.meta.env.PROD ? 'user' : 'never'}>
      <AuthProvider>
        <RouterProvider router={router} />
      </AuthProvider>
    </MotionConfig>
  </StrictMode>,
);
