---
layout: page
sidebar: false
title: Bringing Azeroth to Rust
description: An open-source WotLK Classic 3.4.3 server emulator written in Rust. Explore the project, set up a development realm and help build it.
---

<script setup>
import { withBase } from 'vitepress'
</script>

<div class="rc-home">
  <div class="rc-banner">
    <img :src="withBase('/rustycore-banner.png')" alt="RustyCore — Bringing Azeroth to Rust. A glowing frozen portal overlooking an icy mountain fortress." width="2172" height="724" fetchpriority="high">
  </div>
  <section class="rc-intro" aria-labelledby="welcome">
    <div>
      <p class="rc-eyebrow">Open source · WotLK Classic 3.4.3</p>
      <h1 id="welcome">An old world.<br>A new core.</h1>
    </div>
    <div>
      <p>We're bringing the world of Azeroth to Rust. RustyCore is a community-driven port of a TrinityCore-derived server, built around faithful behavior, clear ownership and evidence you can inspect.</p>
      <div class="rc-actions">
        <a class="rc-button primary" :href="withBase('/server/setup')">Get started →</a>
        <a class="rc-button" href="https://github.com/alseif0x/rustycore">Explore the source</a>
      </div>
    </div>
  </section>
  <aside class="rc-status" aria-label="Project status">
    <strong>Forging ahead</strong>
    <p>Active development. Login and world-entry paths have scoped test evidence; full gameplay parity is still in progress. <a href="https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/STATE.md">Read the current state →</a></p>
  </aside>
  <section aria-labelledby="path">
    <h2 class="rc-section-title" id="path">Choose your path</h2>
    <div class="rc-paths">
      <a class="rc-path" :href="withBase('/server/setup')">
        <span class="number">01 / BUILD</span>
        <h3>Set up a realm</h3>
        <p>Build the authentication and world servers. Find the database, game data and configuration requirements for your development environment.</p>
        <span class="link">Server guide →</span>
      </a>
      <a class="rc-path" :href="withBase('/client/')">
        <span class="number">02 / EXPLORE</span>
        <h3>Connect &amp; test</h3>
        <p>Check the supported client target, understand the connection flow and learn what a successful login verifies.</p>
        <span class="link">Client guide →</span>
      </a>
      <a class="rc-path" :href="withBase('/develop/')">
        <span class="number">03 / CONTRIBUTE</span>
        <h3>Help shape the core</h3>
        <p>Bring your Rust skills, investigate a bug, improve a guide or help verify behavior. Small, thoughtful contributions matter.</p>
        <span class="link">Contributor guide →</span>
      </a>
    </div>
  </section>
  <section class="rc-principles" aria-label="Project approach">
    <div>
      <h2>Familiar world. Faithful behavior.</h2>
      <p>The goal is full functional parity with the 3.4.3 reference server. Protocol, gameplay and persistence changes are checked against versioned source and relevant client/server evidence. Follow the <a href="https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/PORT_PLAN.md">roadmap</a>.</p>
    </div>
    <div>
      <h2>Built in Rust. Built in the open.</h2>
      <p>Rust, Tokio and MariaDB underpin the port. Explore the <a :href="withBase('/reference/')">technical references</a> for architecture, configuration and the decisions behind the systems.</p>
    </div>
  </section>
  <section class="rc-community" aria-labelledby="community">
    <h2 id="community">There's a place for you here.</h2>
    <p>You don't have to know every packet or write a game system to help. Ask a question on <a href="https://github.com/alseif0x/rustycore/discussions">GitHub Discussions</a>, meet the community on <a href="https://discord.gg/mH6ACpGPb2">Discord</a>, or <a href="https://github.com/alseif0x/rustycore/issues/new/choose">report a reproducible bug</a>.</p>
  </section>
</div>
