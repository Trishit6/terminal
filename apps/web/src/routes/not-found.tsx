import { useEffect } from 'react';
import { Link } from '@tanstack/react-router';
import { motion } from 'motion/react';
import { ArrowRight, Compass, TerminalSquare } from 'lucide-react';
import { Button } from '@/components/ui/button';

const EASE = [0.22, 1, 0.36, 1] as const;

const SITEMAP_LINKS: Array<[label: string, to: string]> = [
  ['Home', '/'],
  ['Features', '/features'],
  ['Documentation', '/docs'],
  ['Downloads', '/downloads'],
  ['About', '/about'],
  ['Contact', '/contact'],
];

export function NotFoundPage() {
  // The router has no `head()` for a failed match, so the 404 page sets its
  // own title directly and renders its robots directive; React 19 hoists the
  // meta tag into the document head.
  useEffect(() => {
    document.title = 'Page not found — KERNEXT';
  }, []);

  return (
    <div className="page-shell">
      {/* Crawlers must not index a page that does not exist. */}
      <meta name="robots" content="noindex" />

      <motion.div
        className="not-found"
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, ease: EASE }}
      >
        <div>
          <p className="page-kicker">Error / 404</p>
          <h1>
            No such <em>route.</em>
          </h1>
          <p className="page-lede">
            The page you requested does not exist in the KERNEXT website. The
            index below covers every section that does.
          </p>

          <div className="page-actions">
            <Button asChild>
              <Link to="/">Back to the home page</Link>
            </Button>
            <Button asChild variant="outline">
              <Link to="/docs">
                <Compass className="size-4" />
                Documentation
                <ArrowRight className="size-3.5 opacity-70" />
              </Link>
            </Button>
            <Button asChild variant="outline">
              <Link to="/terminal">
                <TerminalSquare className="size-4" />
                Open the terminal
              </Link>
            </Button>
          </div>

          <nav className="not-found__links" aria-label="Site index">
            {SITEMAP_LINKS.map(([label, to]) => (
              <Link key={to} to={to}>
                {label}
              </Link>
            ))}
          </nav>
        </div>

        <div className="not-found__code" aria-hidden="true">
          <b>404</b>
          <span>Shell path not found</span>
        </div>
      </motion.div>
    </div>
  );
}
