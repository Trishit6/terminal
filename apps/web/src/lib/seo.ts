/**
 * Site-wide SEO constants and head-tag builders.
 *
 * TanStack Router's `head()` route option feeds `<HeadContent />`, which
 * React 19 hoists into `document.head`. Every public route describes itself
 * through `seoHead()` here so titles, descriptions, canonical URLs, Open
 * Graph, Twitter cards, and JSON-LD stay consistent and truthful.
 *
 * The production origin comes from `VITE_PUBLIC_SITE_URL` (see
 * `.env.example`). Until a real domain is configured the value is unset and
 * origin-absolute tags (canonical, `og:url`, `og:image`) are simply not
 * emitted — a missing canonical is honest, a placeholder canonical is not.
 */

export const SITE_NAME = 'KERNEXT';
export const SITE_TAGLINE = 'The Next Core of Computing.';
export const SITE_DESCRIPTION =
  'KERNEXT is a native Rust shell engine with a real PTY, a local Axum runtime, and one terminal that runs on the CLI, web, and desktop.';
export const REPOSITORY_URL = 'https://github.com/Trishit6/terminal';
export const OG_IMAGE_PATH = '/og-image.png';
export const OG_IMAGE_WIDTH = 1200;
export const OG_IMAGE_HEIGHT = 630;

const configuredSiteUrl = (import.meta.env.VITE_PUBLIC_SITE_URL as
  | string
  | undefined)?.replace(/\/+$/, '');

/**
 * The configured production origin, or `null` when no domain is set yet.
 * Never fabricate a domain: everything origin-dependent keys off this.
 */
export const SITE_URL: string | null = configuredSiteUrl || null;

/** Absolute URL for a site path, or `null` when no origin is configured. */
export function siteUrl(path: string): string | null {
  if (!SITE_URL) {
    return null;
  }

  return `${SITE_URL}${path.startsWith('/') ? path : `/${path}`}`;
}

export interface SeoOptions {
  /** Full document title, e.g. `About KERNEXT — Independent. Native. Engineered.` */
  title: string;
  /** One or two sentences describing this page's actual content. */
  description: string;
  /** Canonical path of this page, e.g. `/features`. */
  path: string;
  /** Open Graph type. */
  type?: 'website';
  /** Twitter card layout. */
  twitterCard?: 'summary' | 'summary_large_image';
  /** Emit `robots: noindex` for private or app-only pages. */
  noindex?: boolean;
  /** JSON-LD structured data emitted with this page. */
  jsonLd?: Record<string, unknown> | Array<Record<string, unknown>>;
}

/**
 * TanStack Router head payloads. The route's `head()` option only consumes
 * `meta`, `links`, `scripts`, and `styles`; the document title is passed as a
 * `{ title }` meta descriptor (see `MetaDescriptor` in `@tanstack/router-core`),
 * and JSON-LD as a `{ 'script:ld+json': object }` descriptor.
 */
export interface HeadResult {
  meta: Array<Record<string, unknown>>;
  links: Array<Record<string, string>>;
}

/** Builds the `head()` payload for a route. */
export function seoHead(options: SeoOptions): HeadResult {
  const meta: Array<Record<string, unknown>> = [
    { title: options.title },
    { name: 'description', content: options.description },
    { property: 'og:title', content: options.title },
    { property: 'og:description', content: options.description },
    { property: 'og:type', content: options.type ?? 'website' },
    { property: 'og:site_name', content: SITE_NAME },
    { name: 'twitter:card', content: options.twitterCard ?? 'summary_large_image' },
    { name: 'twitter:title', content: options.title },
    { name: 'twitter:description', content: options.description },
  ];

  const links: Array<Record<string, string>> = [];

  if (!options.noindex) {
    const canonical = siteUrl(options.path);
    if (canonical) {
      links.push({ rel: 'canonical', href: canonical });
      meta.push({ property: 'og:url', content: canonical });
    }

    const image = siteUrl(OG_IMAGE_PATH);
    if (image) {
      meta.push(
        { property: 'og:image', content: image },
        { property: 'og:image:width', content: String(OG_IMAGE_WIDTH) },
        { property: 'og:image:height', content: String(OG_IMAGE_HEIGHT) },
        { name: 'twitter:image', content: image },
      );
    }
  }

  if (options.noindex) {
    meta.push({ name: 'robots', content: 'noindex' });
  }

  if (options.jsonLd) {
    const entries = Array.isArray(options.jsonLd) ? options.jsonLd : [options.jsonLd];
    for (const entry of entries) {
      meta.push({ 'script:ld+json': entry });
    }
  }

  return { meta, links };
}

// ---------------------------------------------------------------------------
// JSON-LD structured data
// ---------------------------------------------------------------------------

/** Organization schema for the home page. */
export function organizationSchema(): Record<string, unknown> {
  const url = siteUrl('/');
  const schema: Record<string, unknown> = {
    '@context': 'https://schema.org',
    '@type': 'Organization',
    name: SITE_NAME,
    description: SITE_DESCRIPTION,
    slogan: SITE_TAGLINE,
    url,
    logo: siteUrl('/favicon.svg'),
    sameAs: [REPOSITORY_URL],
  };

  return Object.fromEntries(Object.entries(schema).filter(([, value]) => value !== null));
}

/** WebSite schema for the home page. */
export function websiteSchema(): Record<string, unknown> {
  const schema: Record<string, unknown> = {
    '@context': 'https://schema.org',
    '@type': 'WebSite',
    name: SITE_NAME,
    url: siteUrl('/'),
    publisher: { '@type': 'Organization', name: SITE_NAME },
  };

  return Object.fromEntries(Object.entries(schema).filter(([, value]) => value !== null));
}

/**
 * SoftwareApplication schema.
 *
 * Only verifiable facts: name, description, category, supported operating
 * systems (built and tested on Windows, Linux, and macOS), and the official
 * repository. No ratings, reviews, prices, or invented download links.
 */
export function softwareApplicationSchema(version: string): Record<string, unknown> {
  const schema: Record<string, unknown> = {
    '@context': 'https://schema.org',
    '@type': 'SoftwareApplication',
    name: SITE_NAME,
    description: SITE_DESCRIPTION,
    url: siteUrl('/') ?? REPOSITORY_URL,
    applicationCategory: 'SystemSoftwareApplication',
    applicationSubCategory: 'Shell',
    operatingSystem: 'Windows, Linux, macOS',
    softwareVersion: version,
    programmingLanguage: 'Rust',
    author: { '@type': 'Organization', name: SITE_NAME, url: siteUrl('/') ?? REPOSITORY_URL },
    sameAs: [REPOSITORY_URL],
    offers: {
      '@type': 'Offer',
      price: '0',
      priceCurrency: 'USD',
    },
  };

  return Object.fromEntries(Object.entries(schema).filter(([, value]) => value !== null));
}

/** BreadcrumbList schema for pages that render a breadcrumb trail. */
export function breadcrumbSchema(
  trail: Array<{ name: string; path: string }>,
): Record<string, unknown> {
  return {
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: trail.map((item, index) => ({
      '@type': 'ListItem',
      position: index + 1,
      name: item.name,
      ...(siteUrl(item.path) ? { item: siteUrl(item.path) } : {}),
    })),
  };
}
