// Each page names its Markdown in its head, for a coding agent that reads the HTML first.
import { defineRouteMiddleware } from '@astrojs/starlight/route-data';
import { markdownOf } from './agent-pages.mjs';

export const onRequest = defineRouteMiddleware((context) => {
  const route = context.locals.starlightRoute;
  const markdown = markdownOf(route.entry.id);
  if (markdown === null) return;
  route.head.push({
    tag: 'link',
    attrs: { rel: 'alternate', type: 'text/markdown', title: 'Markdown for agents', href: `${import.meta.env.BASE_URL}/${markdown}` },
  });
});
