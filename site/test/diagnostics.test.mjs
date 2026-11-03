// Spec: openspec/changes/documentation-site, "Each diagnostic code has a page from the release".
import assert from 'node:assert/strict';
import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { docsOf, groups } from '../scripts/diagnostics.mjs';
import { site } from './support.mjs';

test('each documented code has a page', () => {
  const listed = groups.flatMap((group) => group.codes);
  const documented = readdirSync(join(site, '..', 'crates', 'archangel_rules', 'docs'))
    .filter((file) => file.endsWith('.md') && file !== 'README.md')
    .map((file) => file.replace(/\.md$/, ''));
  // A code that a later release adds has no text in the pinned release, so it waits for the pin.
  const released = documented.filter((code) => docsOf(code) !== null);
  assert.deepEqual(released.filter((code) => !listed.includes(code)), [], 'the site lists each code of the release');
  assert.deepEqual(listed.filter((code) => docsOf(code) === null), [], 'the release prints a page for each listed code');
  assert.equal(new Set(listed).size, listed.length, 'the site lists each code once');
});
