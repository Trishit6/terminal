#!/usr/bin/env node
/**
 * KERNEXT build-time SEO generator.
 *
 * Runs after `vite build` (see `apps/web/package.json`) and produces the
 * deployment SEO artifacts from the single source of truth in
 * `apps/web/src/lib/seo-routes.ts`:
 *
 * - `dist/routes.json`   — route manifest consumed by `apps/shell-server`
 *                          to patch SPA head tags per route and answer
 *                          unknown paths with a real HTTP 404.
 * - `dist/sitemap.xml`   — indexable public routes (only when a production
 *                          origin is configured; never a placeholder domain).
 * - `dist/robots.txt`    — crawl rules; references the sitemap when an
 *                          origin is configured.
 * - `dist/llms.txt`      — concise, accurate product overview for LLM agents.
 * - `dist/index.html`    — origin-dependent fallback tags (`og:url`,
 *                          `og:image`, `twitter:image`, canonical) are
 *                          resolved to absolute URLs, or removed entirely
 *                          when no origin is configured.
 *
 * The production origin comes from `VITE_PUBLIC_SITE_URL` (environment
 * variable or the workspace `.env`); see `.env.example`.
 */

import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { fileURLToPath, URL } from 'node:url';

const ROOT = fileURLToPath(new URL('..', import.meta.url));
const WEB_DIST = fileURLToPath(new URL('../apps/web/dist', import.meta.url));
const ROUTES_MODULE = new URL('../apps/web/src/lib/seo-routes.ts', import.meta.url);

const REPOSITORY_URL = 'https://github.com/Trishit6/terminal';
const SITE_NAME = 'KERNEXT';
const TAGLINE = 'The Next Core of Computing.';
const DESCRIPTION =
  'KERNEXT is a native Rust shell engine with a real PTY, a local Axum runtime, and one terminal that runs on the CLI, web, and desktop.';

/** Reads VITE_PUBLIC_SITE_URL from the environment or the workspace .env. */
function resolveSiteUrl() {
  const fromEnv = process.env.VITE_PUBLIC_SITE_URL?.trim();
  if (fromEnv) {
    return fromEnv;
  }

  // The workspace `.env` is gitignored (it holds local secrets such as
  // DATABASE_URL), so it only exists on a developer machine and never in CI.
  // Reading it unconditionally made the production build throw `ENOENT` on a
  // clean checkout. Only consult it when it is actually present.
  if (existsSync(`${ROOT}/.env`)) {
    const envFile = readFileSync(`${ROOT}/.env`, 'utf8');
    for (const line of envFile.split(/\r?\n/)) {
      const match = line.match(/^\s*VITE_PUBLIC_SITE_URL\s*=\s*(.*)\s*$/);
      if (match && match[1] && !match[1].startsWith('"')) {
        return match[1].trim();
      }
    }
  }

  return null;
}

const rawSiteUrl = resolveSiteUrl();
const siteUrl = rawSiteUrl ? rawSiteUrl.replace(/\/+$/, '') : null;

if (siteUrl && !/^https?:\/\//.test(siteUrl)) {
  console.error(`[seo] VITE_PUBLIC_SITE_URL must be an absolute http(s) URL, got: ${rawSiteUrl}`);
  process.exit(1);
}

if (siteUrl?.startsWith('http://') && !siteUrl.startsWith('http://127.0.0.1') && !siteUrl.startsWith('http://localhost')) {
  console.warn(
    `[seo] VITE_PUBLIC_SITE_URL uses plain http (${siteUrl}); production domains should be https.`,
  );
}

const { ROUTE_SEO, NOT_FOUND_SEO, sitemapRoutes } = await import(ROUTES_MODULE.href);

// ---------------------------------------------------------------------------
// Validation — the manifest is contract, not decoration.
// ---------------------------------------------------------------------------

const failures = [];
const seenPaths = new Set();
const seenTitles = new Set();

for (const route of ROUTE_SEO) {
  if (!route.path.startsWith('/') || route.path !== route.path.toLowerCase()) {
    failures.push(`path must be lowercase and start with '/': ${route.path}`);
  }
  if (route.path.length > 1 && route.path.endsWith('/')) {
    failures.push(`path must not end with '/': ${route.path}`);
  }
  if (seenPaths.has(route.path)) {
    failures.push(`duplicate route path: ${route.path}`);
  }
  if (seenTitles.has(route.title)) {
    failures.push(`duplicate route title: ${route.title}`);
  }
  if (!route.title.includes(SITE_NAME)) {
    failures.push(`title must name the product: ${route.title}`);
  }
  if (route.description.length < 40 || route.description.length > 320) {
    failures.push(`description should be 40-320 characters: ${route.path}`);
  }
  seenPaths.add(route.path);
  seenTitles.add(route.title);
}

const REQUIRED_PUBLIC = ['/', '/features', '/docs', '/downloads', '/about', '/contact'];
for (const required of REQUIRED_PUBLIC) {
  const route = ROUTE_SEO.find((entry) => entry.path === required);
  if (!route || !route.indexable) {
    failures.push(`missing indexable public route: ${required}`);
  }
}

if (failures.length > 0) {
  console.error('[seo] route manifest validation failed:');
  for (const failure of failures) {
    console.error(`  - ${failure}`);
  }
  process.exit(1);
}

// ---------------------------------------------------------------------------
// routes.json — consumed by apps/shell-server (see src/spa.rs).
// ---------------------------------------------------------------------------

const manifest = {
  site_url: siteUrl,
  routes: ROUTE_SEO.map(({ path, title, description, indexable }) => ({
    path,
    title,
    description,
    indexable,
  })),
  not_found: NOT_FOUND_SEO,
};

writeFileSync(`${WEB_DIST}/routes.json`, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`[seo] wrote routes.json (${manifest.routes.length} routes)`);

// ---------------------------------------------------------------------------
// sitemap.xml — only with a real, configured origin.
// ---------------------------------------------------------------------------

if (siteUrl) {
  const urls = sitemapRoutes()
    .map((route) => `  <url><loc>${siteUrl}${route.path === '/' ? '/' : route.path}</loc></url>`)
    .join('\n');

  const sitemap = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>
`;

  writeFileSync(`${WEB_DIST}/sitemap.xml`, sitemap);
  console.log(`[seo] wrote sitemap.xml (${sitemapRoutes().length} URLs, origin ${siteUrl})`);
} else {
  console.log(
    '[seo] VITE_PUBLIC_SITE_URL is not configured: skipping sitemap.xml rather than emitting a placeholder domain',
  );
}

// ---------------------------------------------------------------------------
// robots.txt
// ---------------------------------------------------------------------------

const robots = `# robots.txt for KERNEXT${siteUrl ? ` (${siteUrl})` : ''}
# Crawling of the public website is allowed. Private application areas
# (the desktop webview routes and the render-test fixture) and API
# endpoints are excluded from crawling; account and terminal pages carry
# robots noindex metadata instead, so crawlers can see and honor it.
# robots.txt is not an authentication or privacy mechanism.

User-agent: *
Allow: /
Disallow: /api/
Disallow: /desktop/
Disallow: /terminal/render-test
${siteUrl ? `Sitemap: ${siteUrl}/sitemap.xml\n` : ''}`;

writeFileSync(`${WEB_DIST}/robots.txt`, robots);
console.log('[seo] wrote robots.txt');

// ---------------------------------------------------------------------------
// llms.txt — https://llmstxt.org
// ---------------------------------------------------------------------------

const link = (path, label) => `[${label}](${siteUrl ? `${siteUrl}${path}` : path})`;

const llms = `# ${SITE_NAME}

> ${TAGLINE}

${SITE_NAME} is a native Rust shell engine with a real pseudoterminal (PTY), a
local Axum runtime, and one terminal that runs on three surfaces: the CLI
executable \`kernext\`, the local web terminal, and a Tauri 2 desktop
application. One execution path — lexer, parser, AST evaluator, pipeline
engine — is shared by all three surfaces; nothing is simulated.

- Product: ${SITE_NAME}
- Tagline: ${TAGLINE}
- Values: OPEN • POWERFUL • INDEPENDENT
- Architecture: Rust workspace (shell-core, shell-builtins, shell-runtime,
  shell-line-editor, shell-config, shell-api) plus a React web client and a
  Tauri 2 desktop app
- Platforms: Windows, Linux, macOS — built and tested on all three
- CLI executable: \`kernext\`
- Prompt: \`kernext ~ $ \`

## Website

- ${link('/', 'Home')}
- ${link('/features', 'Features')}
- ${link('/docs', 'Documentation')}
- ${link('/downloads', 'Downloads')}
- ${link('/about', 'About')}
- ${link('/contact', 'Contact')}

## Repository

- ${REPOSITORY_URL}
`;

writeFileSync(`${WEB_DIST}/llms.txt`, llms);
console.log('[seo] wrote llms.txt');

// ---------------------------------------------------------------------------
// index.html — resolve or remove origin-dependent fallback tags.
// ---------------------------------------------------------------------------

const indexPath = `${WEB_DIST}/index.html`;

if (!existsSync(indexPath)) {
  console.error('[seo] dist/index.html is missing; run vite build first');
  process.exit(1);
}

let indexHtml = readFileSync(indexPath, 'utf8');

/** Absolute URL for a site path when an origin is configured. */
const absolute = (path) => (siteUrl ? `${siteUrl}${path}` : null);

// Rewrite `<meta>`/`<link>` tags marked data-kernext-origin. Without a
// configured origin the whole tag is dropped — a placeholder canonical or
// og:image would be worse than none.
indexHtml = indexHtml.replace(/<(?:meta|link)\b[^>]*data-kernext-origin[^>]*>/gs, (tag) => {
  if (!siteUrl) {
    return '';
  }

  let next = tag;

  const replaceAttr = (attr, value) => {
    next = next.replace(new RegExp(`${attr}="[^"]*"`), `${attr}="${value}"`);
  };

  if (tag.includes('property="og:url"')) {
    replaceAttr('content', absolute('/') ?? '/');
  } else if (tag.includes('property="og:image"') || tag.includes('name="twitter:image"')) {
    replaceAttr('content', absolute('/og-image.png') ?? '/og-image.png');
  } else if (tag.includes('rel="canonical"')) {
    replaceAttr('href', absolute('/') ?? '/');
  }

  return next;
});

writeFileSync(indexPath, indexHtml);

if (siteUrl) {
  console.log(`[seo] resolved origin-dependent fallback tags to ${siteUrl}`);
} else {
  console.log('[seo] removed origin-dependent fallback tags (no production origin configured)');
}

console.log('[seo] done');
