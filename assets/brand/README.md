# RustyCore brand assets

The visual theme pairs an icy portal and crystal core with a restrained copper
forge accent. Navy backgrounds and pale cyan highlights connect the README and wiki.

- `rustycore-banner.png`: illustrated README and wiki banner.
- `rustycore-mark.svg`: compact, editable portal symbol.
- `rustycore-logo.svg`: editable wordmark for wide placements.
- `../../docs/wiki/public/logo.svg`: copy of the mark for the VitePress navigation and favicon.
- `../../docs/wiki/public/rustycore-banner.png`: copy of the banner for the standalone Pages build.

Keep public copies in sync when replacing an asset. GitHub Pages builds from a
sparse checkout of `docs/wiki`, so it needs its own copies.

The banner was generated with the built-in image generation tool. The SVG mark
and wordmark are authored vector assets. Banner prompt:

> Create a finished premium wide cinematic banner for RustyCore, an open-source Rust World of Warcraft Wrath Classic server project. Aspect ratio 3:1. Dark icy fantasy meets a forge: monumental angular circular runic portal / forged core emblem glowing glacial cyan with subtle molten copper seams, on the RIGHT third, distant frozen mountain silhouettes and fine snow particles, deep midnight navy background. LEFT two thirds spacious and clean with exquisitely crisp large custom serif fantasy title exact text 'RUSTYCORE', smaller understated uppercase text below exact 'BRINGING AZEROTH TO RUST'. Tasteful editorial videogame art direction, sophisticated and original, restrained icy silver typography, cinematic painterly 3D material detail, dramatic edge lighting, strong silhouette. No cartoon crab, no cogwheel generic Rust logo, no official Warcraft logos or characters, no busy UI, no extra text, no watermark. Banner must read at Github README width 900px. Generate high quality landscape image.
