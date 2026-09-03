# Folder for Github Page

This is the Folder used for the Github Page.

It uses [VitePress](https://vitepress.dev/) to generate a static site from md files stored here.

You only need to edit this files and Github Actions will build and deploy it itself [github-page.yml](..\..\.github\workflows\github-page.yml)

Nav and Header Links are set in [config.js](.vitepress\config.js).


## Local Test

```
npm install
npx vitepress dev
```