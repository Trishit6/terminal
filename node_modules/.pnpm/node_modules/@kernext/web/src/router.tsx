import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
} from '@tanstack/react-router';
import { AboutPage } from './routes/about';
import { ContactPage } from './routes/contact';
import { DocsPage } from './routes/docs';
import { DownloadsPage } from './routes/downloads';
import { FeaturesPage } from './routes/features';
import { HomePage } from './routes/home';
import { NotFoundPage } from './routes/not-found';
import { TerminalPage } from './routes/terminal';
import { DesktopAboutPage } from './routes/desktop/about';
import { DesktopCommandExplorerPage } from './routes/desktop/command-explorer';
import { DesktopDashboardPage } from './routes/desktop/dashboard';
import { DesktopFileExplorerPage } from './routes/desktop/file-explorer';
import { DesktopSessionsPage } from './routes/desktop/sessions';
import { DesktopSettingsPage } from './routes/desktop/settings';
import { DesktopTerminalPage } from './routes/desktop/terminal';
import { DesktopShell } from './components/desktop/desktop-shell';
import { SiteLayout, TerminalLayout } from './components/site/site-layout';

const SITE_DESCRIPTION =
  'KERNEXT is a native Rust shell engine with a real PTY, a local Axum runtime, and one terminal that runs on the CLI, web, and desktop.';

const rootRoute = createRootRoute({
  component: () => <Outlet />,
  notFoundComponent: NotFoundPage,
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
  head: () => ({
    title: 'KERNEXT — The Next Core of Computing.',
    meta: [{ name: 'description', content: SITE_DESCRIPTION }],
  }),
});

const featuresRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/features',
  component: FeaturesPage,
  head: () => ({
    title: 'Features — KERNEXT',
    meta: [
      {
        name: 'description',
        content:
          'A real lexer, parser, and evaluator in Rust: native pipelines, redirections, scripting, background jobs, a genuine PTY, and one engine behind the CLI, web, and desktop surfaces.',
      },
    ],
  }),
});

const docsRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/docs',
  component: DocsPage,
  head: () => ({
    title: 'Documentation — KERNEXT',
    meta: [
      {
        name: 'description',
        content:
          'KERNEXT documentation: local quick start, HTTP API, WebSocket transports, security model, and verification status for the CLI, web terminal, and desktop app.',
      },
    ],
  }),
});

const downloadsRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/downloads',
  component: DownloadsPage,
  head: () => ({
    title: 'Downloads — KERNEXT',
    meta: [
      {
        name: 'description',
        content:
          'Download KERNEXT from its GitHub releases or build it from source: CLI executable, desktop installers, and SHA-256 checksums for Windows, Linux, and macOS.',
      },
    ],
  }),
});

const aboutRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/about',
  component: AboutPage,
  head: () => ({
    title: 'About — KERNEXT',
    meta: [
      {
        name: 'description',
        content:
          'KERNEXT is an open, powerful, independent Rust shell engine. One execution path, three surfaces, honest verification claims.',
      },
    ],
  }),
});

const contactRoute = createRoute({
  getParentRoute: () => publicRoute,
  path: '/contact',
  component: ContactPage,
  head: () => ({
    title: 'Community — KERNEXT',
    meta: [
      {
        name: 'description',
        content:
          'The KERNEXT repository is the single official channel for issues, contributions, and security reports.',
      },
    ],
  }),
});

const terminalRoute = createRoute({
  getParentRoute: () => terminalLayoutRoute,
  path: '/terminal',
  component: TerminalPage,
  head: () => ({
    title: 'Web terminal — KERNEXT',
    meta: [
      {
        name: 'description',
        content:
          'A real PTY or engine session served by the local KERNEXT Axum runtime — streaming output, interactive input, and genuine command execution.',
      },
    ],
  }),
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
});

const desktopTerminalRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/terminal',
  component: DesktopTerminalPage,
});

const desktopCommandExplorerRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/commands',
  component: DesktopCommandExplorerPage,
});

const desktopFileExplorerRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/files',
  component: DesktopFileExplorerPage,
});

const desktopSessionsRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/sessions',
  component: DesktopSessionsPage,
});

const desktopSettingsRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/settings',
  component: DesktopSettingsPage,
});

const desktopAboutRoute = createRoute({
  getParentRoute: () => desktopRoute,
  path: '/about',
  component: DesktopAboutPage,
});

const routeTree = rootRoute.addChildren([
  publicRoute.addChildren([
    homeRoute,
    featuresRoute,
    docsRoute,
    downloadsRoute,
    aboutRoute,
    contactRoute,
  ]),
  terminalLayoutRoute.addChildren([terminalRoute]),
  desktopRoute.addChildren([
    desktopIndexRoute,
    desktopTerminalRoute,
    desktopCommandExplorerRoute,
    desktopFileExplorerRoute,
    desktopSessionsRoute,
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
