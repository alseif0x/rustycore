# RustyCore documentation site

This directory is the source for RustyCore's VitePress site. It replaces the old
`docs/project-wiki` seed: published guides live here, while detailed migration,
architecture, and operational records remain in their existing repository directories
and are linked from the site.

GitHub Actions builds every pull request that changes this directory. A merge into
`3.4.3` publishes the resulting static site through GitHub Pages.

Navigation is configured in [`.vitepress/config.js`](.vitepress/config.js).

## Local development

Use Node 22.22.0 and npm 10.9.4, then run:

```bash
npm ci
npm run docs:dev
```

Verify the production build before publishing:

```bash
npm run docs:build
```
