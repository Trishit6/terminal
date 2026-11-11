#!/usr/bin/env node
/**
 * Vite is configured with `src/main.tsx` as its rollup input (a workaround for
 * the Vite 8 HTML-entry EISDIR bug), so it does not emit `index.html` itself.
 * The old build copied the development index, which kept pointing at
 * `/src/main.tsx` and produced a blank production page.
 *
 * This script writes the real production shell: the hashed entry module and
 * the extracted stylesheet, while preserving every other head tag. It runs
 * before `scripts/seo.mjs`, which then patches route metadata.
 */
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath, URL } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const source = readFileSync(`${root}apps/web/index.html`, 'utf8');
const assetsDirectory = `${root}apps/web/dist/assets`;

function findAsset(prefixRegex) {
  const match = readdirSync(assetsDirectory).find((name) => prefixRegex.test(name));
  if (!match) {
    throw new Error(
      `build-index: no emitted asset matching ${prefixRegex} in apps/web/dist/assets`,
    );
  }
  return `/assets/${match}`;
}

const entry = findAsset(/^main-.*\.js$/);
const stylesheet = findAsset(/^main-.*\.css$/);

let html = source.replace(
  /<script type="module" src="\/src\/main\.tsx"><\/script>/,
  `<script type="module" crossorigin src="${entry}"></script>`,
);

if (html === source) {
  throw new Error('build-index: could not find the development module entry in index.html');
}

html = html.replace(
  '</head>',
  `    <link rel="stylesheet" href="${stylesheet}" />\n  </head>`,
);

writeFileSync(`${root}apps/web/dist/index.html`, html);
console.log(`[build-index] entry ${entry}, stylesheet ${stylesheet}`);
