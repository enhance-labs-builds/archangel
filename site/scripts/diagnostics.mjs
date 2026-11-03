// Writes one page for each diagnostic code, from what `archangel docs <code>` of the pinned release
// prints. The pages are not in Git, so a page always says what the terminal says.
import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const site = join(dirname(fileURLToPath(import.meta.url)), '..');
const bin = join(site, 'node_modules', '@enhance-eng', 'archangel', 'bin', 'archangel.js');
export const folder = join(site, 'src', 'content', 'docs', 'reference', 'diagnostics');

/** The groups of codes, in the order that the site lists them. */
export const groups = JSON.parse(readFileSync(join(site, 'src', 'diagnostics.json'), 'utf8'));

/** What `archangel docs <code>` prints, or null when the release does not know the code. */
export function docsOf(code) {
  const found = spawnSync(process.execPath, [bin, 'docs', code, '--color', 'never'], { encoding: 'utf8' });
  return found.status === 0 ? found.stdout : null;
}

/** The page for one code: the release's text, under a title and a description. */
export function pageOf(code, group, text) {
  // The release links one code page to another by its file name, as GitHub shows them.
  const body = text
    .replace(/^# .*\n+/, '')
    .replace(/\]\(([a-z-]+)\.md\)/g, '](/archangel/reference/diagnostics/$1/)');
  const description = body.split('\n\n')[0].replace(/\s+/g, ' ').trim();
  return [
    '---',
    `title: ${code}`,
    `description: ${JSON.stringify(description)}`,
    'editUrl: false',
    'sidebar:',
    `  label: ${code}`,
    '---',
    '',
    `<p class="diagnostic-group">${group}. This text is what <code>archangel docs ${code}</code> prints.</p>`,
    '',
    body,
  ].join('\n');
}

/** Writes every page, and fails on a code that the release does not know. */
export function writePages() {
  rmSync(folder, { recursive: true, force: true });
  mkdirSync(folder, { recursive: true });
  for (const { group, codes } of groups) {
    for (const code of codes) {
      const text = docsOf(code);
      if (text === null) throw new Error(`archangel docs ${code} printed no page: remove the code from src/diagnostics.json, or pin a release that has it`);
      writeFileSync(join(folder, `${code}.md`), pageOf(code, group, text));
    }
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) writePages();
