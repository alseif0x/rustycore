# RustyCore ⚔️🦀

**WoW Wrath of the Lich King Classic (3.4.3.54261) server emulator written in Rust.**

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org)

---

## What is RustyCore?

RustyCore is a high-performance WoW WotLK 3.4.3 private server emulator built from scratch in Rust.
Built using WoW protocol research from the TrinityCore/MaNGOS community.

### Why Rust?

| | RustyCore (Rust) | Traditional (C#/.NET) |
|---|---|---|
| Release binary | ~10 MB | ~200 MB+ |
| Idle RAM | ~50–80 MB | ~300–500 MB |
| GC pauses | None | Yes |
| Concurrency | Tokio async | Thread-based |

Runs comfortably on a Raspberry Pi 5.

---

## Stack

- **Rust 1.85** (edition 2024)
- **Tokio** — async runtime
- **Axum** — BNet REST API
- **SQLx + MariaDB** — 4 databases (auth, characters, world, hotfixes)
- **hecs** — ECS (Entity Component System)
- **prost** — protobuf (BNet protocol)

---

## Workspace Crates

```
crates/
  bnet-server/     — Battle.net authentication server
  world-server/    — World (game) server
  wow-core/        — Core types and primitives
  wow-constants/   — Game constants
  wow-crypto/      — Cryptography (SRP6, AES-GCM)
  wow-network/     — Networking (Tokio)
  wow-packet/      — Packet serialization/deserialization
  wow-handler/     — Packet handler dispatch
  wow-world/       — World session and game logic
  wow-data/        — DBC/DB2 data loading
  wow-database/    — Database layer (SQLx)
  wow-ecs/         — Entity Component System (hecs)
  wow-spell/       — Spell system
  wow-combat/      — Combat system
  wow-ai/          — Creature AI
  wow-chat/        — Chat system
  wow-social/      — Friends, ignore, inspect
  wow-loot/        — Loot system
  wow-map/         — Maps and instances
  ...and more
```

---

## Building

```bash
# Debug build
cargo build --workspace

# Release build (~10 MB binaries)
cargo build --workspace --release

# Tests
cargo test --workspace
```

---

## Requirements

- Rust 1.85+
- MariaDB 10.6+
- protoc (for BNet protobuf)

---

## License

GPL v3 — see [LICENSE](LICENSE)

WoW protocol research based on [TrinityCore](https://github.com/TrinityCore/TrinityCore) and [MaNGOS](https://github.com/mangos/MaNGOS).

World of Warcraft © Blizzard Entertainment. This project is not affiliated with or endorsed by Blizzard Entertainment.
