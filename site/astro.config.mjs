import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';
import starlightLinksValidator from 'starlight-links-validator';
import starlightLlmsTxt from 'starlight-llms-txt';
import { agentSets } from './src/agent-pages.mjs';

// The site lives at enhancelabs.ai/archangel. CloudFront sends that path to GitHub Pages, which
// serves the same build at enhance-labs-builds.github.io/archangel, so the base is the same in both.
const site = 'https://enhancelabs.ai';
const base = '/archangel';
const card = `${site}${base}/social-card.png`;

export default defineConfig({
  site,
  base,
  integrations: [
    starlight({
      title: 'Archangel',
      description: 'Architecture tests for TypeScript. Make architecture verifiable.',
      logo: { light: './src/assets/logo-light.svg', dark: './src/assets/logo-dark.svg' },
      favicon: '/favicon.svg',
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/enhance-labs-builds/archangel' }],
      editLink: { baseUrl: 'https://github.com/enhance-labs-builds/archangel/edit/main/site/' },
      customCss: ['@fontsource-variable/inter', '@fontsource-variable/jetbrains-mono', './src/styles/theme.css'],
      expressiveCode: {
        themes: ['github-dark-default'],
        styleOverrides: {
          borderRadius: '10px',
          borderColor: '#2a2a2a',
          codeBackground: '#171717',
          codeFontFamily: "'JetBrains Mono Variable', ui-monospace, monospace",
          frames: { editorTabBarBackground: '#0f0f0f', editorActiveTabBackground: '#171717', terminalTitlebarBackground: '#0f0f0f', terminalBackground: '#171717', editorActiveTabIndicatorTopColor: '#fc622d', editorActiveTabIndicatorBottomColor: 'transparent' },
        },
      },
      // A link preview shows the card, and search engines read what archangel is from the JSON-LD.
      head: [
        { tag: 'meta', attrs: { property: 'og:image', content: card } },
        { tag: 'meta', attrs: { property: 'og:image:width', content: '1200' } },
        { tag: 'meta', attrs: { property: 'og:image:height', content: '630' } },
        { tag: 'meta', attrs: { property: 'og:image:alt', content: 'Archangel: make architecture verifiable. Architecture tests for TypeScript.' } },
        { tag: 'meta', attrs: { name: 'twitter:image', content: card } },
        {
          tag: 'script',
          attrs: { type: 'application/ld+json' },
          content: JSON.stringify({
            '@context': 'https://schema.org',
            '@type': 'SoftwareApplication',
            name: 'Archangel',
            description: 'Architecture tests for TypeScript: which code may import which code, cycles, names, folder structure and exports, checked in milliseconds.',
            applicationCategory: 'DeveloperApplication',
            operatingSystem: 'macOS, Linux, Windows',
            url: `${site}${base}/`,
            downloadUrl: 'https://www.npmjs.com/package/@enhance-eng/archangel',
            softwareVersion: '0.3.3',
            license: 'https://github.com/enhance-labs-builds/archangel#license',
            offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
          }),
        },
      ],
      // A page's head links to its Markdown, and its title shows the same link.
      routeMiddleware: './src/route-data.ts',
      components: { PageTitle: './src/components/PageTitle.astro' },
      plugins: [
        starlightLinksValidator({ errorOnLocalLinks: true }),
        // llms.txt lists a Markdown file for each section and each page, so a coding agent reads
        // only what it needs.
        starlightLlmsTxt({
          details: [
            'Archangel tests the structure of TypeScript code: which code may import which code, cycles, file and folder names, and exports. Its config is `archangel.config.ts`.',
            '',
            'To answer a question, read one page below, not the complete documentation. Start with "Start: Getting started" for a first config, "Concepts" for what the words of a config mean, and "Reference" for every key and option. For a diagnostic code such as `may-import`, read "Diagnostic codes: may-import", or run `npx archangel docs may-import`, which prints the same text with no network.',
            '',
            'Every config and command output in these pages comes from a small repository that a test runs on the published package `@enhance-eng/archangel`, so the outputs are what archangel prints.',
          ].join('\n'),
          // Keep the prose, file trees and asides whole: a page for an agent is short already.
          minify: { whitespace: false, details: false, note: false, tip: false },
          customSelectors: { all: ['.sl-anchor-link'] },
          customSets: agentSets().map(({ label, description, paths }) => ({ label, description, paths })),
          optionalLinks: [
            { label: 'Source', url: 'https://github.com/enhance-labs-builds/archangel', description: 'the Rust core, the npm package and the editor extension' },
            { label: 'npm package', url: 'https://www.npmjs.com/package/@enhance-eng/archangel' },
          ],
        }),
      ],
      sidebar: [
        {
          label: 'Start',
          items: ['start/introduction', 'start/getting-started', 'start/how-archangel-works'],
        },
        {
          label: 'Guides',
          items: [
            'guides/existing-repository',
            'guides/backend-layers',
            'guides/feature-sliced-design',
            'guides/nextjs',
            'guides/monorepos',
            'guides/names',
            'guides/plugins',
            'guides/ci',
            'guides/editors-and-agents',
            'guides/docs-for-agents',
            'guides/exceptions-and-baselines',
            {
              label: 'Migrate from other tools',
              items: [
                'guides/from-dependency-cruiser',
                'guides/from-eslint-plugin-boundaries',
                'guides/from-steiger',
                'guides/from-eslint-plugin-check-file',
              ],
            },
          ],
        },
        {
          label: 'Concepts',
          items: [
            'concepts/parts-groups-and-externals',
            'concepts/captures-and-instances',
            'concepts/tables-rows-and-cells',
            'concepts/kinds',
            'concepts/rules-and-conditions',
            'concepts/examples',
            'concepts/what-a-rule-judges',
          ],
        },
        {
          label: 'Reference',
          items: [
            'reference/config',
            'reference/sets-of-files',
            'reference/tables',
            'reference/conditions',
            'reference/import-options',
            'reference/rule-options',
            'reference/shorthands',
            'reference/examples',
            'reference/presets',
            'reference/plugin-api',
            'reference/commands',
            'reference/output-and-exit-codes',
            {
              label: 'Diagnostic codes',
              collapsed: true,
              items: ['reference/diagnostics', { autogenerate: { directory: 'reference/diagnostics' } }],
            },
          ],
        },
      ],
    }),
  ],
});
