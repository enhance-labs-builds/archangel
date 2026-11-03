// The Markdown that a coding agent reads: one file for each section and one for each page, which
// starlight-llms-txt writes from the same pages that the site shows. The site config, the head of
// each page and its visible link all name a file by this module, so they cannot disagree.
import GithubSlugger from 'github-slugger';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

// The site's folder, where the build runs; the bundled page code has no path of its own to read from.
const docs = join(process.cwd(), 'src', 'content', 'docs');

/** The sections of the site, in sidebar order. */
const SECTIONS = [
  { folder: 'start', label: 'Start', description: 'what archangel is, how it works, and a first table' },
  { folder: 'guides', label: 'Guides', description: 'one task each, from adding archangel to a repository to migrating from another tool' },
  { folder: 'concepts', label: 'Concepts', description: 'parts, groups, captures, kinds, tables, rules and examples' },
  { folder: 'reference', label: 'Reference', description: 'every config key, condition, option, command and output format' },
  { folder: 'reference/diagnostics', label: 'Diagnostic codes', description: 'what each code means, an example, and what to do' },
];

/** A value of a page's front matter. */
function field(text, name) {
  const line = text.match(new RegExp(`^${name}: (.*)$`, 'm'))?.[1]?.trim() ?? '';
  return line.startsWith('"') ? JSON.parse(line) : line;
}

/** Each page of a folder, without the pages of its subfolders, by its id. */
function pagesIn(folder) {
  return readdirSync(join(docs, folder), { withFileTypes: true })
    .filter((entry) => entry.isFile() && /\.mdx?$/.test(entry.name))
    .map((entry) => {
      const text = readFileSync(join(docs, folder, entry.name), 'utf8');
      return { id: `${folder}/${entry.name.replace(/\.mdx?$/, '')}`, title: field(text, 'title'), description: field(text, 'description') };
    })
    .sort((a, b) => a.id.localeCompare(b.id));
}

/**
 * The sets that starlight-llms-txt writes, sections first and then each page. The plugin names a
 * set's file by the slug of its label, in this order, so the slugs here are its file names.
 */
export function agentSets() {
  const slugger = new GithubSlugger();
  const sections = SECTIONS.map(({ folder, label, description }) => ({
    label,
    description,
    paths: folder === 'reference' ? ['reference/*'] : [`${folder}/**`],
  }));
  const pages = SECTIONS.flatMap(({ folder, label }) =>
    pagesIn(folder).map((page) => ({ label: `${label}: ${page.title}`, description: page.description, paths: [page.id], id: page.id })),
  );
  return [...sections, ...pages].map((set) => ({ ...set, slug: slugger.slug(set.label) }));
}

/** The path, under the site's base, of the Markdown of a page, by its id. */
export function markdownOf(id) {
  const set = agentSets().find((each) => each.id === id);
  return set ? `_llms-txt/${set.slug}.txt` : null;
}
