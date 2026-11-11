import { useEffect } from 'react';
import { Link } from '@tanstack/react-router';
import { motion } from 'motion/react';
import { ArrowRight, Compass, TerminalSquare } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';

const EASE = [0.22, 1, 0.36, 1] as const;

const SITEMAP_LINKS: Array<[label: string, to: string]> = [  ['Home', '/'],
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
    <div className="mx-auto flex min-h-[70vh] w-full max-w-[1100px] flex-col items-center justify-center px-4 py-16 text-center sm:px-6">
      {/* Crawlers must not index a page that does not exist. */}
      <meta name="robots" content="noindex" />

      <motion.div
        initial={{ opacity: 0, y: 18 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, ease: EASE }}
        className="flex flex-col items-center"
      >
        <Badge variant="outline">404</Badge>
        <h1 className="mt-4 text-balance text-4xl font-semibold tracking-[-0.04em] sm:text-5xl">
          No such route.
        </h1>
        <p className="mt-4 max-w-md text-sm leading-relaxed text-muted-foreground">
          The page you requested does not exist in the KERNEXT website. The
          links below cover every section that does.
        </p>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 14 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, ease: EASE, delay: 0.08 }}
        className="mt-8 flex flex-wrap justify-center gap-3"
      >
        <Button asChild>
          <Link to="/">
            Back to the home page
          </Link>
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
      </motion.div>

      <Card className="mt-10 w-full max-w-2xl bg-surface/55">
        <CardContent className="grid gap-2 p-5 text-left sm:grid-cols-3">
          {SITEMAP_LINKS.map(([label, to]) => (
            <Link
              key={to}
              to={to}
              className="rounded-lg px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-surface-soft hover:text-foreground"
            >
              {label}
            </Link>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}
