// Spec: openspec/changes/public-cleanup-and-agent-docs, "A coding agent reads each page as Markdown".
// The test reads the built site, so `npm run build` runs first, as scripts/check does.
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { site } from './support.mjs';

const dist = join(site, 'dist');

/** Each built HTML page of the docs, by its path in dist, without the home page and the 404 page. */
function builtPages() {
  return readdirSync(dist, { recursive: true })
    .filter((path) => path.endsWith('index.html') && path !== 'index.html' && !path.startsWith('_'))
    .map((path) => path.replaceAll('\\', '/'));
}

test('each page links to its Markdown, and llms.txt lists it', () => {
  assert.ok(existsSync(join(dist, 'llms.txt')), 'the site is built; run npm run build first');
  const index = readFileSync(join(dist, 'llms.txt'), 'utf8');
  assert.ok(builtPages().length > 80, 'the test reads every page of the site');
  const problems = builtPages().flatMap((page) => {
    const html = readFileSync(join(dist, page), 'utf8');
    const link = html.match(/<link rel="alternate" type="text\/markdown"[^>]* href="\/archangel\/([^"]+)"/)?.[1];
    if (!link) return [`${page} names no Markdown in its head`];
    const found = [];
    if (!existsSync(join(dist, link))) found.push(`${page} names ${link}, which the build did not write`);
    if (!index.includes(`https://enhancelabs.ai/archangel/${link})`)) found.push(`llms.txt does not list ${link}, the Markdown of ${page}`);
    if (!html.includes(`class="agent-markdown" href="/archangel/${link}"`)) found.push(`${page} shows no link to ${link}`);
    const markdown = existsSync(join(dist, link)) ? readFileSync(join(dist, link), 'utf8') : '';
    const title = html.match(/<h1 id="_top">([^<]+)<\/h1>/)?.[1];
    if (title && !markdown.includes(`# ${title.replaceAll('&#39;', "'").replaceAll('&amp;', '&')}`)) found.push(`${link} is not the page ${page}`);
    return found;
  });
  assert.deepEqual(problems, [], problems.join('\n'));
});
