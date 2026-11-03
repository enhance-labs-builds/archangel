import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';
import starlightLinksValidator from 'starlight-links-validator';
import starlightLlmsTxt from 'starlight-llms-txt';

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
            softwareVersion: '0.3.1',
            license: 'https://github.com/enhance-labs-builds/archangel#license',
            offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
          }),
        },
      ],
      // llms.txt gives a coding agent the whole site as plain text.
      plugins: [starlightLinksValidator({ errorOnLocalLinks: true }), starlightLlmsTxt()],
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
