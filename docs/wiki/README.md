# RustyCore documentation site

This directory is the source for RustyCore's VitePress site. It replaces the old
`docs/project-wiki` seed: published guides live here, while detailed migration,
architecture, and operational records remain in their existing repository directories
and are linked from the site.

The repository remains the documentation source of truth. Follow the
[documentation map](../README.md) for current state, plans and operating rules; link to
those maintained sources instead of copying mutable status tables or creating a second plan.

GitHub Actions builds pull requests that change this directory, except those authored by
exactly `alseif0x`, which use a local build. `validation-v2` does not run VitePress: run the
site build below explicitly for site changes. A matching push to `3.4.3` (including a merge)
or a manual workflow run on that branch publishes through GitHub Pages. Local edits and
builds do not publish; push and merge authority remain separate.

Navigation is configured in [`.vitepress/config.js`](.vitepress/config.js).

## Local development

Use Node 22.22.0 and npm 10.9.4, then run:

```bash
cd docs/wiki
npm ci --ignore-scripts
npm run docs:dev
```

Verify the production build from `docs/wiki` before publishing:

```bash
npm run docs:build
```
