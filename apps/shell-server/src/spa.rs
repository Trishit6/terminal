//! SPA shell serving with honest HTTP statuses and per-route head patching.
//!
//! The built web client (`apps/web/dist`) is a client-rendered SPA: every
//! navigation needs `index.html`. Serving it with `200` for *any* path — the
//! classic `ServeDir::fallback(ServeFile::new(index))` setup — makes every
//! typo look like a real page to crawlers.
//!
//! Instead, the shell is served through [`SpaShell`], which loads the route
//! manifest generated at build time (`scripts/seo.mjs` writes
//! `dist/routes.json` from `apps/web/src/lib/seo-routes.ts`):
//!
//! - a known route gets the shell with `200` and the route's title,
//!   description, Open Graph tags, and robots directive patched into the
//!   fallback head tags (so non-JS clients see correct metadata),
//! - an unknown path gets the shell with a real `404` status, the
//!   not-found metadata, and `robots: noindex`,
//! - `/features/` redirects to `/features` (clean URLs, no trailing slash),
//! - unknown `/api/*` paths keep their JSON 404 instead of an HTML shell.
//!
//! Private application routes (`indexable: false`) are served with
//! `robots: noindex` so crawlers that do not run JavaScript still see the
//! correct directive. When the manifest was built with
//! `VITE_PUBLIC_SITE_URL`, canonical and `og:url` are patched per route too.
//!
//! When `routes.json` is missing (a hand-copied dist without the SEO step),
//! `SpaShell::load` returns `None` and the router keeps the previous
//! behaviour of serving the shell with `200` for every path.

use std::path::Path;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use tower_http::services::ServeDir;

/// One route's metadata, mirrored from `apps/web/src/lib/seo-routes.ts`.
#[derive(Debug, Clone, Deserialize)]
pub struct RouteSeoEntry {
    /// Route path exactly as registered in the web router.
    pub path: String,
    /// Full document title.
    pub title: String,
    /// Meta description.
    pub description: String,
    /// Whether the page may be indexed.
    pub indexable: bool,
}

/// Metadata for paths that match no route.
#[derive(Debug, Clone, Deserialize)]
pub struct NotFoundSeo {
    /// Title for the not-found response.
    pub title: String,
    /// Description for the not-found response.
    pub description: String,
}

/// The route manifest written to `dist/routes.json` by `scripts/seo.mjs`.
#[derive(Debug, Clone, Deserialize)]
pub struct RouteManifest {
    /// Configured production origin (`VITE_PUBLIC_SITE_URL`), if any.
    pub site_url: Option<String>,
    /// All known routes.
    pub routes: Vec<RouteSeoEntry>,
    /// Not-found metadata.
    pub not_found: NotFoundSeo,
}

/// The SPA shell plus its route manifest.
pub struct SpaShell {
    /// Contents of `index.html`.
    html: String,
    manifest: RouteManifest,
}

impl SpaShell {
    /// Loads `index.html` and `routes.json` from a built web directory.
    ///
    /// Returns `None` when either file is missing or unreadable; the router
    /// then falls back to plain static serving.
    #[must_use]
    pub fn load(web_dir: &Path) -> Option<Self> {
        let html = std::fs::read_to_string(web_dir.join("index.html")).ok()?;
        let raw = std::fs::read_to_string(web_dir.join("routes.json")).ok()?;
        let manifest = serde_json::from_str(&raw).ok()?;

        Some(Self { html, manifest })
    }

    /// Finds the manifest entry for a request path.
    #[must_use]
    pub fn lookup(&self, path: &str) -> Option<&RouteSeoEntry> {
        let normalized = normalize_path(path);
        self.manifest
            .routes
            .iter()
            .find(|route| route.path == normalized)
    }

    /// Renders the shell for a known route with patched head tags.
    #[must_use]
    pub fn render(&self, route: &RouteSeoEntry) -> String {
        self.patch(
            &route.title,
            &route.description,
            route.indexable,
            &route.path,
        )
    }

    /// Renders the shell for an unknown path: not-found metadata and 404.
    #[must_use]
    pub fn render_not_found(&self) -> String {
        self.patch(
            &self.manifest.not_found.title,
            &self.manifest.not_found.description,
            false,
            "/",
        )
    }

    /// Patches the fallback head tags in the shell HTML.
    fn patch(&self, title: &str, description: &str, indexable: bool, path: &str) -> String {
        let mut html = patch_title(&self.html, &escape_html(title));

        html = patch_meta(html, "name=\"description\"", &escape_html(description));
        html = patch_meta(html, "property=\"og:title\"", &escape_html(title));
        html = patch_meta(
            html,
            "property=\"og:description\"",
            &escape_html(description),
        );
        html = patch_meta(
            html,
            "name=\"robots\"",
            if indexable { "index,follow" } else { "noindex" },
        );

        if let Some(origin) = self
            .manifest
            .site_url
            .as_deref()
            .map(|origin| origin.trim_end_matches('/'))
        {
            let canonical = format!("{origin}{}", normalize_path(path));
            html = patch_meta(html, "property=\"og:url\"", &escape_html(&canonical));
            html = patch_link(html, "rel=\"canonical\"", &escape_html(&canonical));
        }

        html
    }
}

/// Strips one trailing slash from non-root paths (`/features/` → `/features`).
fn normalize_path(path: &str) -> &str {
    if path.len() > 1 {
        path.strip_suffix('/').unwrap_or(path)
    } else {
        path
    }
}

/// Escapes a value for safe inclusion in HTML text and quoted attributes.
fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Replaces the text of the `<title>` element.
fn patch_title(html: &str, title: &str) -> String {
    let Some(open) = html.find("<title") else {
        return html.to_owned();
    };
    let Some(tag_end) = html[open..].find('>') else {
        return html.to_owned();
    };
    let tag_end = open + tag_end;
    let Some(close) = html[tag_end..].find("</title>") else {
        return html.to_owned();
    };
    let close = tag_end + close;

    format!("{}{}{}", &html[..tag_end + 1], title, &html[close..])
}

/// Replaces the `content` attribute of the `<meta>` tag that contains `key`.
///
/// Both the `data-kernext-static` marker and `key` must appear in the same
/// tag; without the marker the patch leaves React-managed tags untouched.
fn patch_meta(html: String, key: &str, content: &str) -> String {
    patch_tag_attribute(
        html,
        "meta",
        &format!("data-kernext-static {key}"),
        "content",
        content,
    )
}

/// Replaces the `href` attribute of the `<link>` tag that contains `key`.
fn patch_link(html: String, key: &str, href: &str) -> String {
    patch_tag_attribute(
        html,
        "link",
        &format!("data-kernext-static {key}"),
        "href",
        href,
    )
}

/// Replaces `attribute` inside the first tag of `tag` kind containing `keys`.
///
/// The scan is intentionally small and literal: the shell HTML is generated
/// by this repository's build, so attribute order and quoting are known.
fn patch_tag_attribute(
    html: String,
    tag: &str,
    keys: &str,
    attribute: &str,
    value: &str,
) -> String {
    let opener = format!("<{tag}");
    let mut search_from = 0;

    while let Some(found) = html[search_from..].find(&opener) {
        let start = search_from + found;
        let Some(tag_end_rel) = html[start..].find('>') else {
            break;
        };
        let tag_end = start + tag_end_rel;
        let tag_text = &html[start..=tag_end];

        if tag_text.contains(keys) {
            let needle = format!("{attribute}=\"");
            if let Some(attribute_start) = tag_text.rfind(&needle) {
                let content_start = start + attribute_start + needle.len();
                let Some(content_end) = html[content_start..].find('"') else {
                    break;
                };
                let content_end = content_start + content_end;

                let mut next = String::with_capacity(html.len() + value.len());
                next.push_str(&html[..content_start]);
                next.push_str(value);
                next.push_str(&html[content_end..]);
                return next;
            }
        }

        search_from = tag_end;
    }

    html
}

/// Axum handler serving the SPA shell for any non-asset path.
pub async fn spa_shell(State(shell): State<Arc<SpaShell>>, uri: Uri) -> Response {
    let path = uri.path();

    // Unknown API routes keep their JSON 404 instead of an HTML shell.
    if path == "/api" || path.starts_with("/api/") {
        return crate::routes::api_not_found().await;
    }

    // Clean URLs: `/features/` redirects to `/features`.
    if path.len() > 1 && path.ends_with('/') {
        let target = normalize_path(path);
        let query = uri
            .query()
            .map_or(String::new(), |query| format!("?{query}"));

        return Response::builder()
            .status(StatusCode::PERMANENT_REDIRECT)
            .header(header::LOCATION, format!("{target}{query}"))
            .body(axum::body::Body::empty())
            .expect("redirect response should build");
    }

    let (status, html) = match shell.lookup(path) {
        Some(route) => (StatusCode::OK, shell.render(route)),
        None => (StatusCode::NOT_FOUND, shell.render_not_found()),
    };

    let mut response = (
        status,
        [(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))],
        axum::response::Html(html),
    )
        .into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response
}

/// Builds the static file service with the SPA shell fallback.
///
/// Serves real files from `web_dir` (hashed Vite assets, favicon, manifest)
/// and hands every other path to [`spa_shell`].
#[must_use]
pub fn static_service(web_dir: &Path, shell: Arc<SpaShell>) -> ServeDir<axum::Router> {
    let spa_router = axum::Router::new().fallback(spa_shell).with_state(shell);

    ServeDir::new(web_dir).fallback(spa_router)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHELL: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
    <title data-kernext-static>KERNEXT — The Next Core of Computing.</title>
    <meta data-kernext-static name="description" content="fallback description" />
    <meta data-kernext-static name="robots" content="index,follow" />
    <meta data-kernext-static property="og:title" content="fallback title" />
    <meta data-kernext-static property="og:description" content="fallback description" />
    <meta data-kernext-static property="og:url" content="https://kernext.example.org/" />
    <link data-kernext-static rel="canonical" href="https://kernext.example.org/" />
  </head>
  <body><div id="root"></div></body>
</html>
"#;

    fn manifest() -> RouteManifest {
        serde_json::from_str(
            r#"{
                "site_url": "https://kernext.example.org",
                "routes": [
                    {"path": "/", "title": "KERNEXT — Home", "description": "home description", "indexable": true},
                    {"path": "/features", "title": "KERNEXT Features", "description": "features description", "indexable": true},
                    {"path": "/terminal", "title": "Web terminal — KERNEXT", "description": "terminal description", "indexable": false}
                ],
                "not_found": {"title": "Page not found — KERNEXT", "description": "nothing here"}
            }"#,
        )
        .expect("manifest should parse")
    }

    #[test]
    fn patch_replaces_title_and_metadata_for_a_known_route() {
        let shell = SpaShell {
            html: SHELL.to_owned(),
            manifest: manifest(),
        };
        let route = shell.lookup("/features").expect("route should exist");
        let html = shell.render(route);

        assert!(html.contains("<title data-kernext-static>KERNEXT Features</title>"));
        assert!(html.contains(
            r#"<meta data-kernext-static name="description" content="features description""#
        ));
        assert!(html.contains(
            r#"<meta data-kernext-static property="og:title" content="KERNEXT Features""#
        ));
        assert!(html.contains(r#"<meta data-kernext-static name="robots" content="index,follow""#));
        assert!(html.contains(r#"href="https://kernext.example.org/features""#));
    }

    #[test]
    fn patch_marks_private_routes_noindex() {
        let shell = SpaShell {
            html: SHELL.to_owned(),
            manifest: manifest(),
        };
        let route = shell.lookup("/terminal").expect("route should exist");
        let html = shell.render(route);

        assert!(html.contains(r#"<meta data-kernext-static name="robots" content="noindex""#));
    }

    #[test]
    fn patch_uses_not_found_metadata_for_unknown_paths() {
        let shell = SpaShell {
            html: SHELL.to_owned(),
            manifest: manifest(),
        };
        let html = shell.render_not_found();

        assert!(html.contains("<title data-kernext-static>Page not found — KERNEXT</title>"));
        assert!(html.contains(r#"<meta data-kernext-static name="robots" content="noindex""#));
    }

    #[test]
    fn patch_escapes_html_in_values() {
        let shell = SpaShell {
            html: SHELL.to_owned(),
            manifest: RouteManifest {
                site_url: None,
                routes: vec![RouteSeoEntry {
                    path: "/x".to_owned(),
                    title: "A & B \"quoted\"".to_owned(),
                    description: "less < than".to_owned(),
                    indexable: true,
                }],
                not_found: NotFoundSeo {
                    title: "None".to_owned(),
                    description: "None".to_owned(),
                },
            },
        };
        let html = shell.render(&shell.manifest.routes[0]);

        assert!(html.contains("A &amp; B &quot;quoted&quot;"));
        assert!(html.contains("less &lt; than"));
    }

    #[test]
    fn lookup_normalizes_trailing_slashes() {
        let shell = SpaShell {
            html: SHELL.to_owned(),
            manifest: manifest(),
        };

        assert!(shell.lookup("/features/").is_some());
        assert!(shell.lookup("/").is_some());
        assert!(shell.lookup("/nope").is_none());
    }

    #[test]
    fn patch_without_site_url_leaves_origin_tags_alone() {
        let shell = SpaShell {
            html: SHELL.to_owned(),
            manifest: RouteManifest {
                site_url: None,
                routes: vec![],
                not_found: NotFoundSeo {
                    title: "None".to_owned(),
                    description: "None".to_owned(),
                },
            },
        };
        let html = shell.render_not_found();

        // The example origin in the shell stays untouched rather than being
        // rewritten to a fabricated value.
        assert!(html.contains(r#"href="https://kernext.example.org/""#));
    }
}
