---
layout: home

hero:
  name: RustyCore
  text: WoW Wrath of the Lich King Classic server emulator written in Rust
  tagline: A C++-contrasted port targeting client 3.4.3.54261
  image:
    src: /logo.svg
    alt: RustyCore
  actions:
    - theme: brand
      text: Server setup
      link: /server/setup
    - theme: alt
      text: Development guide
      link: /develop/
    - theme: alt
      text: Current migration state
      link: https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/STATE.md

features:
  - icon: ⚔️
    title: WotLK Classic target
    details: Protocol and gameplay work target client 3.4.3.54261. The currently tested game-build path is 51943.
  - icon: 🦀
    title: Rust implementation
    details: Rust 1.98, edition 2024, Tokio networking, MariaDB persistence, and typed packet handling.
  - icon: 🔎
    title: C++-first parity
    details: The legacy TrinityCore-derived C++ server is the behavioral source of truth for the port.
  - icon: 🚧
    title: Active migration
    details: Login and world-entry paths are represented, while full gameplay and live map-runtime parity remain in progress.
---

RustyCore aims for full functional parity with its legacy C++ reference. It is not yet a
production-ready game server; represented logic and a successful build do not by themselves
prove complete live-runtime behavior.
