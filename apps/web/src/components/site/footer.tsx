import { Link } from '@tanstack/react-router';
import { ArrowUpRight, TerminalSquare } from 'lucide-react';
import { Logo } from '@/components/site/logo';
import { KERNEXT_VERSION } from '@/lib/version';

export function SiteFooter() {
  return (
    <footer className="site-footer">
      <div className="site-footer__inner">
        <div className="site-footer__masthead">
          <div>
            <Logo />
            <p>
              An independent Rust shell engine with a real PTY, a local Axum
              runtime, and one terminal across the CLI, web, and desktop.
              Built. Executed. Verified.
            </p>
            <p className="mt-3 font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
              Open · Powerful · Independent
            </p>
          </div>

          <div className="site-footer__status">
            <span>
              Release <b>v{KERNEXT_VERSION}</b>
            </span>
            <span>
              CLI <b>Reference</b>
            </span>
            <span>
              Web <b>Live</b>
            </span>
            <span>
              Desktop <b>Implemented</b>
            </span>
            <Link
              to="/terminal"
              className="mt-1 inline-flex items-center gap-1.5 text-primary normal-case tracking-normal transition-colors hover:text-foreground"
            >
              <TerminalSquare className="size-3.5" aria-hidden="true" />
              Run a live command
              <ArrowUpRight className="size-3" aria-hidden="true" />
            </Link>
          </div>
        </div>

        <div className="site-footer__grid">
          <div className="site-footer__column">
            {/* Footer column headings are `h2` so pages that go straight from
                the page `h1` into the footer keep a valid heading outline. */}
            <h2>Product</h2>
            <ul>
              <li>
                <Link to="/terminal">Web terminal</Link>
              </li>
              <li>
                <Link to="/features">Features</Link>
              </li>
              <li>
                <Link to="/downloads">Downloads</Link>
              </li>
              <li>
                <Link to="/docs">Documentation</Link>
              </li>
            </ul>
          </div>

          <div className="site-footer__column">
            <h2>Project</h2>
            <ul>
              <li>
                <Link to="/about">About</Link>
              </li>
              <li>
                <Link to="/contact">Contact and community</Link>
              </li>
              <li>
                <Link to="/docs" hash="status">
                  Project status
                </Link>
              </li>
              <li>
                <Link to="/docs" hash="security">
                  Security model
                </Link>
              </li>
            </ul>
          </div>

          <div className="site-footer__column">
            <h2>Build</h2>
            <ul>
              <li>
                <Link to="/docs" hash="quickstart">
                  Quick start
                </Link>
              </li>
              <li>
                <Link to="/docs" hash="api">
                  HTTP API
                </Link>
              </li>
              <li>
                <Link to="/downloads" hash="platforms">
                  Platform targets
                </Link>
              </li>
              <li>
                <Link to="/docs" hash="websocket">
                  WebSocket transports
                </Link>
              </li>
            </ul>
          </div>

          <div className="site-footer__column">
            <h2>Surfaces</h2>
            <ul>
              <li>
                <Link to="/">CLI runtime</Link>
              </li>
              <li>
                <Link to="/terminal">Browser terminal</Link>
              </li>
              <li>
                <Link to="/downloads">Tauri desktop</Link>
              </li>
              <li>
                <Link to="/sign-in">Account</Link>
              </li>
            </ul>
          </div>
        </div>

        <div className="site-footer__legal">
          <p>© {new Date().getFullYear()} KERNEXT. Build. Execute. Innovate.</p>
          <div>
            <span>Independent · Native · Engineered</span>
            <span>Local-first runtime</span>
          </div>
        </div>
      </div>
    </footer>
  );
}
