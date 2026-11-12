import {
  createRootRoute,
  createRoute,
  createRouter,
  HeadContent,
  lazyRouteComponent,
  Outlet,
} from '@tanstack/react-router';
import { AdminLayout } from './components/admin/admin-shell';
import { DesktopShell } from './components/desktop/desktop-shell';
import { SiteFrame, SiteLayout, TerminalLayout } from './components/site/site-layout';
import { NotFoundPage } from './routes/not-found';
import {
  breadcrumbSchema,
  organizationSchema,
  SITE_DESCRIPTION,
  SITE_NAME,
  SITE_TAGLINE,
  seoHead,
  softwareApplicationSchema,
  websiteSchema,
  type HeadResult,
} from './lib/seo';
import type { SeoOptions } from './lib/seo';
import { ROUTE_SEO } from './lib/seo-routes';
import { KERNEXT_VERSION } from './lib/version';

// Route-level code splitting: every page loads as its own chunk so the
// marketing site never downloads the terminal, desktop, or auth code.
const HomePage = lazyRouteComponent(() => import('./routes/home'), 'HomePage');
const FeaturesPage = lazyRouteComponent(() => import('./routes/features'), 'FeaturesPage');
const DocsPage = lazyRouteComponent(() => import('./routes/docs'), 'DocsPage');
const DownloadsPage = lazyRouteComponent(() => import('./routes/downloads'), 'DownloadsPage');
const AboutPage = lazyRouteComponent(() => import('./routes/about'), 'AboutPage');
const ContactPage = lazyRouteComponent(() => import('./routes/contact'), 'ContactPage');
const SignInPage = lazyRouteComponent(() => import('./routes/auth/sign-in'), 'SignInPage');
const CreateAccountPage = lazyRouteComponent(
  () => import('./routes/auth/create-account'),
  'CreateAccountPage',
);
const ForgotPasswordPage = lazyRouteComponent(
  () => import('./routes/auth/forgot-password'),
  'ForgotPasswordPage',
);
const ResetPasswordPage = lazyRouteComponent(
  () => import('./routes/auth/reset-password'),
  'ResetPasswordPage',
);
const VerifyEmailPage = lazyRouteComponent(
  () => import('./routes/auth/verify-email'),
  'VerifyEmailPage',
);
const AccountPage = lazyRouteComponent(() => import('./routes/auth/account'), 'AccountPage');
const SignOutPage = lazyRouteComponent(() => import('./routes/auth/sign-out'), 'SignOutPage');
const TerminalPage = lazyRouteComponent(() => import('./routes/terminal'), 'TerminalPage');
const TerminalFixturesPage = lazyRouteComponent(
  () => import('./routes/terminal-fixtures'),
  'TerminalFixturesPage',
);
const DesktopDashboardPage = lazyRouteComponent(
  () => import('./routes/desktop/dashboard'),
  'DesktopDashboardPage',
);
const DesktopTerminalPage = lazyRouteComponent(
  () => import('./routes/desktop/terminal'),
  'DesktopTerminalPage',
);
const DesktopCommandExplorerPage = lazyRouteComponent(
  () => import('./routes/desktop/command-explorer'),
  'DesktopCommandExplorerPage',
);
const DesktopFileExplorerPage = lazyRouteComponent(
  () => import('./routes/desktop/file-explorer'),
  'DesktopFileExplorerPage',
);
const DesktopSessionsPage = lazyRouteComponent(
  () => import('./routes/desktop/sessions'),
  'DesktopSessionsPage',
);
const DesktopAccountPage = lazyRouteComponent(
  () => import('./routes/desktop/account'),
  'DesktopAccountPage',
);
const DesktopSettingsPage = lazyRouteComponent(
  () => import('./routes/desktop/settings'),
  'DesktopSettingsPage',
);
const DesktopAboutPage = lazyRouteComponent(
  () => import('./routes/desktop/about'),
  'DesktopAboutPage',
);
const AdminOverviewPage = lazyRouteComponent(
  () => import('./routes/admin/overview'),
  'AdminOverviewPage',
);
const AdminUsersPage = lazyRouteComponent(
  () => import('./routes/admin/users'),
  'AdminUsersPage',
);
const AdminUserDetailPage = lazyRouteComponent(
  () => import('./routes/admin/user-detail'),
  'AdminUserDetailPage',
);
const AdminTerminalSessionsPage = lazyRouteComponent(
  () => import('./routes/admin/terminal-sessions'),
  'AdminTerminalSessionsPage',
);
const AdminHealthPage = lazyRouteComponent(
  () => import('./routes/admin/health'),
  'AdminHealthPage',
);
const AdminAuditPage = lazyRouteComponent(
  () => import('./routes/admin/audit'),
  'AdminAuditPage',
);
const AdminSettingsPage = lazyRouteComponent(
  () => import('./routes/admin/settings'),
  'AdminSettingsPage',
);

const SEO_BY_PATH = new Map(ROUTE_SEO.map((route) => [route.path, route]));

/**
 * Builds a route's `head()` payload from the shared route metadata in
 * `lib/seo-routes.ts` — the same data that drives `sitemap.xml`,
 * `robots.txt`, and the server-side SPA shell patching.
 */
function headFor(
  path: string,
  jsonLd?: SeoOptions['jsonLd'],
): HeadResult {
  const seo = SEO_BY_PATH.get(path);

  // Routes not registered in the SEO manifest (e.g. new routes added
  // without updating seo-routes.ts, or dynamic routes) fall back to a
  // generic KERNEXT title/description instead of crashing the page.
  if (!seo) {
    return seoHead({
      title: `${SITE_NAME} — ${SITE_TAGLINE}`,
      description: SITE_DESCRIPTION,
      path,
      noindex: true,
      jsonLd,
    });
  }

  return seoHead({
    title: seo.title,
    description: seo.description,
    path: seo.path,
    noindex: !seo.indexable,
    jsonLd,
  });
}

const rootRoute = createRootRoute({
  component: () => (
    <>
      {/* Renders the route `head()` payloads (title, meta, links, JSON-LD)
          into the document head; React 19 hoists these elements. */}
      <HeadContent />
      <Outlet />
    </>
  ),
  notFoundComponent: () => (
    <SiteFrame>
      <NotFoundPage />
    </SiteFrame>
  ),
  // Fallback title for anything that declares no head of its own.
  head: () => ({ meta: [{ title: 'KERNEXT — The Next Core of Computing.' }] }),
});

/** Pathless layout for marketing and documentation pages. */
const publicRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: 'public',
  component: SiteLayout,
});

/** Pathless layout for the full-height web terminal. */
const terminalLayoutRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: 'terminal-layout',
  component: TerminalLayout,
});

const homeRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/',
  component: HomePage,
  head: () =>
    headFor('/', [
      organizationSchema(),
      websiteSchema(),
      softwareApplicationSchema(KERNEXT_VERSION),
    ]),
});

const featuresRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/features',
  component: FeaturesPage,
  head: () => headFor('/features'),
});

const docsRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/docs',
  component: DocsPage,
  head: () =>
    headFor(
      '/docs',
      breadcrumbSchema([
        { name: 'Home', path: '/' },
        { name: 'Documentation', path: '/docs' },
      ]),
    ),
});

const downloadsRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/downloads',
  component: DownloadsPage,
  head: () => headFor('/downloads'),
});

const aboutRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/about',
  component: AboutPage,
  head: () => headFor('/about'),
});

const contactRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/contact',
  component: ContactPage,
  head: () => headFor('/contact'),
});

const signInRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/sign-in',
  component: SignInPage,
  head: () => headFor('/sign-in'),
});

const createAccountRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/create-account',
  component: CreateAccountPage,
  head: () => headFor('/create-account'),
});

const forgotPasswordRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/forgot-password',
  component: ForgotPasswordPage,
  head: () => headFor('/forgot-password'),
});

const resetPasswordRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/reset-password',
  component: ResetPasswordPage,
  head: () => headFor('/reset-password'),
});

const verifyEmailRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/verify-email',
  component: VerifyEmailPage,
  head: () => headFor('/verify-email'),
});

const accountRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/account',
  component: AccountPage,
  head: () => headFor('/account'),
});

const signOutRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/sign-out',
  component: SignOutPage,
  head: () => headFor('/sign-out'),
});

const terminalRoute = createRoute({
  getParentRoute: () => terminalLayoutRoute,
  path: '/terminal',
  component: TerminalPage,
  head: () => headFor('/terminal'),
});

/** Backend-free rendering fixtures for visual regression checks. */
const terminalFixturesRoute = createRoute({
  getParentRoute: () => terminalLayoutRoute,
  path: '/terminal/render-test',
  component: TerminalFixturesPage,
  head: () => headFor('/terminal/render-test'),
});

const adminRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/admin',
  component: AdminLayout,
});

const adminIndexRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: '/',
  component: AdminOverviewPage,
  head: () => headFor('/admin'),
});

const adminUsersRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'users',
  component: AdminUsersPage,
  head: () => headFor('/admin/users'),
});

const adminUserDetailRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'users/$userId',
  component: AdminUserDetailPage,
  head: () => headFor('/admin/users'),
});

const adminTerminalSessionsRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'terminal-sessions',
  component: AdminTerminalSessionsPage,
  head: () => headFor('/admin/terminal-sessions'),
});

const adminHealthRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'health',
  component: AdminHealthPage,
  head: () => headFor('/admin/health'),
});

const adminAuditRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'audit',
  component: AdminAuditPage,
  head: () => headFor('/admin/audit'),
});

const adminSettingsRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'settings',
  component: AdminSettingsPage,
  head: () => headFor('/admin/settings'),
});

const desktopRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/desktop',
  component: DesktopShell,
});

const desktopIndexRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/',
  component: DesktopDashboardPage,
  head: () => headFor('/desktop'),
});

const desktopTerminalRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/terminal',
  component: DesktopTerminalPage,
  head: () => headFor('/desktop/terminal'),
});

const desktopCommandExplorerRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/commands',
  component: DesktopCommandExplorerPage,
  head: () => headFor('/desktop/commands'),
});

const desktopFileExplorerRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/files',
  component: DesktopFileExplorerPage,
  head: () => headFor('/desktop/files'),
});

const desktopSessionsRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/sessions',
  component: DesktopSessionsPage,
  head: () => headFor('/desktop/sessions'),
});

const desktopAccountRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/account',
  component: DesktopAccountPage,
  head: () => headFor('/desktop/account'),
});

const desktopSettingsRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/settings',
  component: DesktopSettingsPage,
  head: () => headFor('/desktop/settings'),
});

const desktopAboutRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/about',
  component: DesktopAboutPage,
  head: () => headFor('/desktop/about'),
});

const routeTree = rootRoute.addChildren([
  publicRoute.addChildren([
    homeRoute,
    featuresRoute,
    docsRoute,
    downloadsRoute,
    aboutRoute,
    contactRoute,
    signInRoute,
    createAccountRoute,
    forgotPasswordRoute,
    resetPasswordRoute,
    verifyEmailRoute,
    accountRoute,
    signOutRoute,
  ]),
  terminalLayoutRoute.addChildren([terminalRoute, terminalFixturesRoute]),
  adminRoute.addChildren([
    adminIndexRoute,
    adminUsersRoute,
    adminUserDetailRoute,
    adminTerminalSessionsRoute,
    adminHealthRoute,
    adminAuditRoute,
    adminSettingsRoute,
  ]),
  desktopRoute.addChildren([
    desktopIndexRoute,
    desktopTerminalRoute,
    desktopCommandExplorerRoute,
    desktopFileExplorerRoute,
    desktopSessionsRoute,
    desktopAccountRoute,
    desktopSettingsRoute,
    desktopAboutRoute,
  ]),
]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
