# RustyCore

**A Rust port of a WoW Wrath of the Lich King Classic server.**

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![Target](https://img.shields.io/badge/WotLK%20Classic-3.4.3.54261-blueviolet.svg)](#)

RustyCore is an attempt to bring a TrinityCore-style WotLK Classic server into Rust without losing the behavior that makes the original server work.

This is not a rewrite that guesses how the game should behave. The C++ server is the reference. Every serious piece of logic is being checked against the legacy C++ implementation first, then ported, tested, and documented in Rust.

The project is still in active migration. Some systems are already represented in Rust, some are partially ported, and some runtime pieces are still being wired into a live server path. If you like protocol work, game-server internals, Rust, databases, packet formats, or old-school MMO systems, there is plenty here to dig into.

## Why This Exists

The WoW emulator ecosystem has decades of hard-earned C++ knowledge. RustyCore is about preserving that behavior while exploring what a safer, more modular, more testable server can look like in Rust.

The current priority is simple:

1. Match the C++ behavior.
2. Keep the port honest: no fake "done" labels, no hidden gaps.
3. Add tests and documentation as each subsystem moves.
4. Improve the architecture only when parity is understood.

Rust is useful here because a game server is full of shared state, packet boundaries, async IO, ownership problems, and long-lived runtime systems. That is exactly where Rust's type system starts to pay rent.

## Current Focus

RustyCore currently targets:

- **Client:** WotLK Classic `3.4.3.54261`
- **Reference implementation:** TrinityCore/WotLK-style C++ code
- **Primary goal:** complete C++ -> Rust port with behavioral parity
- **Secondary goal:** Rust-native cleanup where it does not change gameplay semantics

Important note: modern systems that are not part of the WotLK target, such as Battle Pets or Black Market features, may exist in notes or partial code, but they are not the current migration priority.

## What Is Inside

```text
crates/
  bnet-server/     Battle.net authentication server
  world-server/    World/game server entry point
  wow-core/        Core types, GUIDs, time, networking helpers
  wow-constants/   Opcodes and game constants
  wow-crypto/      SRP6, AES-GCM and auth crypto
  wow-network/     Tokio networking and session plumbing
  wow-packet/      Packet read/write code
  wow-handler/     Packet handler registry and dispatch metadata
  wow-world/       WorldSession, handlers and represented game logic
  wow-map/         Canonical map/runtime structures
  wow-data/        DBC/DB2 data loading
  wow-database/    SQLx database layer and prepared statements
  wow-ai/          Creature AI work
  wow-chat/        Chat validation and routing
  wow-loot/        Loot logic
  wow-script/      Script integration foundation
```

The migration notes live under `docs/migration/`. They are not decoration: they are part of the porting process and track what has been verified, what is partial, and what still needs runtime validation.

## Build

Requirements:

- Rust `1.85+`
- MariaDB `10.6+`
- `protoc` for Battle.net protobuf generation

```bash
cargo build --workspace
cargo test --workspace
```

For checks that need protobuf:

```bash
PROTOC=/path/to/protoc cargo check -p world-server
```

In the main development environment for this repo, `PROTOC` is usually:

```bash
PROTOC=/home/cdmonio/.local/protoc/bin/protoc
```

## How To Help

RustyCore needs people who enjoy careful work. The useful contributions are not only giant features.

Good ways to contribute:

- Compare Rust behavior against the C++ reference.
- Port one bounded handler or subsystem at a time.
- Add packet shape tests.
- Add database statement tests.
- Improve docs when an inventory entry is stale.
- Run the server and report exact client behavior.
- Help with bot/client simulation tests.
- Review code for places where Rust accidentally "works" but does not match C++.

Before changing gameplay logic, please check the C++ source first. If the C++ behavior looks wrong, document that too; sometimes the right answer is to understand the legacy bug before deciding whether Rust should preserve or fix it.

## Support The Project

This project takes a lot of time: protocol research, porting, testing, docs, DB work, and long debugging sessions with real clients.

If RustyCore is useful to you, or you simply want to help keep the work moving, donations are welcome.

| Network | Wallet |
|---|---|
| ₿ BTC | `bc1qeggjcl5guwmqr0aa4emufyzyh7nu5rkfrytqy8` |
| Ξ ETH / BNB | `0xfec63e014e0bd36d77b094ff27f7e7f5d7ab67aa` |
| ◎ Solana | `9ktt1zinmwwsZXGx9x1BM995FwbAfdNWe65v1mdPgDhn` |
| XRP | `rBVvKPrQAmd5uDZ89nDgz5HbSWVD6sTbg2` |

Even small support helps buy time to keep the port moving.

## License

RustyCore is licensed under GPL v3. See [LICENSE](LICENSE).

WoW protocol research and server behavior are based on the public work of the TrinityCore and MaNGOS communities.

World of Warcraft is owned by Blizzard Entertainment. This project is not affiliated with, endorsed by, or sponsored by Blizzard Entertainment.
