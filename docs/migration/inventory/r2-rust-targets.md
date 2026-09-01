# R2 Rust target coverage

> Generado: 2026-05-07
> Alcance: docs dueno de `docs/migration/inventory/cpp-files-by-module.md`.
> Regla: C++ sigue siendo el oraculo; esta auditoria solo comprueba si el target Rust declarado existe y si tiene lineas activas.

## Summary

| Status | Rows |
|---|---:|
| `declared_pattern` | 6 |
| `exists_active` | 433 |
| `exists_empty` | 79 |
| `exists_manifest` | 9 |
| `missing_declared_path` | 64 |

## Docs without active Rust target

- `movement-pathgen.md`
- `scenarios.md`
- `scripts-ulduar.md`

## Full target table

| Doc | Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---|---:|---:|---|---|
| `accounts.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `accounts.md` | `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `accounts.md` | `crates/bnet-server` | `crate_dir` | 13 | 2831 | `exists_active` | crate exists |
| `accounts.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `accounts.md` | `crates/wow-database/src/statements/login.rs` | `file` | 1 | 327 | `exists_active` | file exists |
| `accounts.md` | `crates/wow-database/src/statements/character.rs` | `file` | 1 | 284 | `exists_active` | file exists |
| `accounts.md` | `crates/wow-network/src/world_socket.rs` | `file` | 1 | 1023 | `exists_active` | file exists |
| `accounts.md` | `crates/bnet-server/src/rest/handlers.rs` | `file` | 1 | 573 | `exists_active` | file exists |
| `accounts.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `accounts.md` | `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |
| `achievements.md` | `crates/wow-achievement` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `achievements.md` | `crates/wow-world/src/handlers` | `module_dir` | 14 | 8843 | `exists_active` | directory exists |
| `achievements.md` | `crates/wow-packet/src/packets/misc.rs` | `file` | 1 | 2613 | `exists_active` | file exists |
| `achievements.md` | `crates/wow-achievement/Cargo.toml` | `file` | 1 | 10 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `achievements.md` | `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `achievements.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `achievements.md` | `crates/wow-achievement/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `achievements.md` | `crates/wow-world/src/handlers/character.rs` | `file` | 1 | 4612 | `exists_active` | file exists |
| `achievements.md` | `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `achievements.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `ai-base.md` | `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `ai-base.md` | `crates/wow-ai/src/lib.rs` | `file` | 1 | 346 | `exists_active` | file exists |
| `ai-base.md` | `crates/wow-ai/Cargo.toml` | `file` | 1 | 11 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `ai-base.md` | `crates/wow-script/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `ai-base.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `ai-base.md` | `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `ai-smartscripts.md` | `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `ai-smartscripts.md` | `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `ai-smartscripts.md` | `crates/wow-script/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `ai-smartscripts.md` | `crates/wow-script/Cargo.toml` | `file` | 1 | 11 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `ai-smartscripts.md` | `crates/wow-ai/src/lib.rs` | `file` | 1 | 346 | `exists_active` | file exists |
| `ai-smartscripts.md` | `crates/wow-database/src/statements/world.rs` | `file` | 1 | 371 | `exists_active` | file exists |
| `ai-smartscripts.md` | `crates/wow-conditions` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `ai.md` | `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `ai.md` | `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `ai.md` | `crates/wow-scripts` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `ai.md` | `crates/wow-ai/src/lib.rs` | `file` | 1 | 346 | `exists_active` | file exists |
| `ai.md` | `crates/wow-script/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `ai.md` | `crates/wow-scripts/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `ai.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `ai.md` | `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `auctionhouse.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `auctionhouse.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `auctionhouse.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `auctionhouse.md` | `crates/wow-packet/src/packets/misc.rs` | `file` | 1 | 2613 | `exists_active` | file exists |
| `auctionhouse.md` | `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `auctionhouse.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `auctionhousebot.md` | `crates/wow-world/src/auctionhousebot` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `auctionhousebot.md` | `crates/wow-ahbot` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `auctionhousebot.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `auctionhousebot.md` | `crates/wow-config` | `crate_dir` | 1 | 397 | `exists_active` | crate exists |
| `battlefield.md` | `crates/wow-battlefield` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `battlefield.md` | `crates/wow-world/src/handlers` | `module_dir` | 14 | 8843 | `exists_active` | directory exists |
| `battlefield.md` | `crates/wow-areatrigger` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `battlefield.md` | `crates/wow-gameobject` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `battlegrounds.md` | `crates/wow-pvp` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `battlegrounds.md` | `crates/wow-pvp/Cargo.toml` | `file` | 1 | 10 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `battlegrounds.md` | `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `battlegrounds.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `battlegrounds.md` | `crates/wow-pvp/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `battlegrounds.md` | `crates/wow-world/src/handlers` | `module_dir` | 14 | 8843 | `exists_active` | directory exists |
| `battlegrounds.md` | `crates/wow-maps` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `battlepets.md` | `crates/wow-packet/src/packets/misc.rs` | `file` | 1 | 2613 | `exists_active` | file exists |
| `battlepets.md` | `crates/wow-world/src/handlers/character.rs` | `file` | 1 | 4612 | `exists_active` | file exists |
| `battlepets.md` | `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `battlepets.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `battlepets.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `blackmarket.md` | `crates/wow-world/src/blackmarket` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `blackmarket.md` | `crates/wow-blackmarket` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `blackmarket.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `blackmarket.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `blackmarket.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `blackmarket.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `blackmarket.md` | `crates/wow-constants/src/item.rs` | `file` | 1 | 1239 | `exists_active` | file exists |
| `bnetserver.md` | `crates/bnet-server` | `crate_dir` | 13 | 2831 | `exists_active` | crate exists |
| `bnetserver.md` | `crates/bnet-server/src/main.rs` | `file` | 1 | 245 | `exists_active` | file exists |
| `bnetserver.md` | `crates/bnet-server/src/state.rs` | `file` | 1 | 112 | `exists_active` | file exists |
| `bnetserver.md` | `crates/bnet-server/src/rest/mod.rs` | `file` | 1 | 198 | `exists_active` | file exists |
| `bnetserver.md` | `crates/bnet-server/src/rest/handlers.rs` | `file` | 1 | 573 | `exists_active` | file exists |
| `bnetserver.md` | `crates/bnet-server/src/rest/types.rs` | `file` | 1 | 86 | `exists_active` | file exists |
| `bnetserver.md` | `crates/bnet-server/src/rpc/mod.rs` | `file` | 1 | 42 | `exists_active` | file exists |
| `bnetserver.md` | `crates/bnet-server/src/rpc/session.rs` | `file` | 1 | 263 | `exists_active` | file exists |
| `bnetserver.md` | `crates/bnet-server/src/rpc/services/{account,authentication,connection,game_utilities}.rs` | `declared_pattern` | 0 | 0 | `declared_pattern` | pattern/proposed path; not resolvable as one file or directory |
| `bnetserver.md` | `crates/bnet-server/src/realm/mod.rs` | `file` | 1 | 392 | `exists_active` | file exists |
| `cache.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `cache.md` | `crates/wow-cache` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `cache.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `cache.md` | `crates/wow-social` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `calendar.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `calendar.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `calendar.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `calendar.md` | `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `calendar.md` | `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `calendar.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `chat.md` | `crates/wow-chat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `chat.md` | `crates/wow-world/src/handlers/chat.rs` | `file` | 1 | 413 | `exists_active` | file exists |
| `chat.md` | `crates/wow-packet/src/packets/chat.rs` | `file` | 1 | 351 | `exists_active` | file exists |
| `chat.md` | `crates/wow-chat/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `chat.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `chat.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `combat-manager.md` | `crates/wow-combat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `combat-manager.md` | `crates/wow-combat/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `combat-manager.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `combat-manager.md` | `crates/wow-ai/src/lib.rs` | `file` | 1 | 346 | `exists_active` | file exists |
| `combat-manager.md` | `crates/wow-world/src/handlers/combat.rs` | `file` | 1 | 152 | `exists_active` | file exists |
| `combat-manager.md` | `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `combat-manager.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `combat-threat.md` | `crates/wow-combat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `combat-threat.md` | `crates/wow-combat/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `combat-threat.md` | `crates/wow-ai/src/lib.rs` | `file` | 1 | 346 | `exists_active` | file exists |
| `combat-threat.md` | `crates/wow-world/src/handlers/combat.rs` | `file` | 1 | 152 | `exists_active` | file exists |
| `commands.md` | `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `commands.md` | `crates/wow-scripts` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `commands.md` | `crates/wow-script/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `commands.md` | `crates/wow-scripts/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `commands.md` | `crates/wow-handler/src/lib.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `commands.md` | `crates/wow-chat/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `commands.md` | `crates/wow-world/src/handlers/chat.rs` | `file` | 1 | 413 | `exists_active` | file exists |
| `commands.md` | `crates/world-server/src/main.rs` | `file` | 1 | 818 | `exists_active` | file exists |
| `commands.md` | `crates/bnet-server/src` | `module_dir` | 13 | 2831 | `exists_active` | directory exists |
| `common-collision.md` | `crates/wow-recastdetour` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `common-collision.md` | `crates/wow-collision` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `common-collision.md` | `crates/wow-recastdetour/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `common-collision.md` | `crates/wow-map/src/lib.rs` | `file` | 1 | 70 | `exists_active` | file exists |
| `common-collision.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `common-collision.md` | `crates/wow-combat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `common-collision.md` | `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `conditions.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `conditions.md` | `crates/wow-world/src/conditions` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `conditions.md` | `crates/wow-logging` | `crate_dir` | 1 | 464 | `exists_active` | crate exists |
| `conditions.md` | `crates/wow-logging/src/lib.rs` | `file` | 1 | 464 | `exists_active` | file exists |
| `conditions.md` | `crates/wow-packet/src/packets/character.rs` | `file` | 1 | 550 | `exists_active` | file exists |
| `database-framework.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `database-framework.md` | `crates/wow-database/src/lib.rs` | `file` | 1 | 58 | `exists_active` | file exists |
| `database-framework.md` | `crates/wow-database/src/database.rs` | `file` | 1 | 178 | `exists_active` | file exists |
| `database-framework.md` | `crates/wow-database/src/error.rs` | `file` | 1 | 21 | `exists_active` | file exists |
| `database-framework.md` | `crates/wow-database/src/params.rs` | `file` | 1 | 208 | `exists_active` | file exists |
| `database-framework.md` | `crates/wow-database/src/result.rs` | `file` | 1 | 198 | `exists_active` | file exists |
| `database-framework.md` | `crates/wow-database/src/transaction.rs` | `file` | 1 | 108 | `exists_active` | file exists |
| `database-framework.md` | `crates/wow-database/src/migration.rs` | `file` | 1 | 391 | `exists_active` | explicit manifest-driven migration engine; startup updater retired by #256 |
| `database-framework.md` | `crates/wow-database/src/statements/mod.rs` | `file` | 1 | 93 | `exists_active` | file exists |
| `database-framework.md` | `crates/wow-database/src/statements/login.rs` | `file` | 1 | 327 | `exists_active` | file exists |
| `database-framework.md` | `crates/wow-database/src/statements/character.rs` | `file` | 1 | 284 | `exists_active` | file exists |
| `database-framework.md` | `crates/wow-database/src/statements/world.rs` | `file` | 1 | 371 | `exists_active` | file exists |
| `database-framework.md` | `crates/wow-database/src/statements/hotfix.rs` | `file` | 1 | 25 | `exists_active` | file exists |
| `database-framework.md` | `crates/world-server/src/main.rs` | `file` | 1 | 818 | `exists_active` | file exists |
| `database-framework.md` | `crates/wow-data/src/hotfix_blob_cache.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `database-framework.md` | `crates/wow-database/src` | `module_dir` | 12 | 2262 | `exists_active` | directory exists |
| `datastores.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `datastores.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `datastores.md` | `crates/wow-data/src/wdc4.rs` | `file` | 1 | 915 | `exists_active` | file exists |
| `datastores.md` | `crates/wow-data/src/hotfix_cache.rs` | `file` | 1 | 111 | `exists_active` | file exists |
| `datastores.md` | `crates/wow-data/src/item.rs` | `file` | 1 | 123 | `exists_active` | file exists |
| `datastores.md` | `crates/wow-data/src/item_stats.rs` | `file` | 1 | 424 | `exists_active` | file exists |
| `datastores.md` | `crates/wow-data/src/player_stats.rs` | `file` | 1 | 307 | `exists_active` | file exists |
| `datastores.md` | `crates/wow-data/src/skill.rs` | `file` | 1 | 608 | `exists_active` | file exists |
| `datastores.md` | `crates/wow-data/src/area_trigger.rs` | `file` | 1 | 312 | `exists_active` | file exists |
| `datastores.md` | `crates/wow-data/src/spell.rs` | `file` | 1 | 225 | `exists_active` | file exists |
| `datastores.md` | `crates/wow-data/src/quest.rs` | `file` | 1 | 337 | `exists_active` | file exists |
| `datastores.md` | `crates/wow-data/src/quest_xp.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `datastores.md` | `crates/wow-database/src/statements/hotfix.rs` | `file` | 1 | 25 | `exists_active` | file exists |
| `datastores.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `dungeonfinding.md` | `crates/wow-world/src/lfg` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `dungeonfinding.md` | `crates/wow-world/src/handlers` | `module_dir` | 14 | 8843 | `exists_active` | directory exists |
| `dungeonfinding.md` | `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `dungeonfinding.md` | `crates/wow-world/src/handlers/mod.rs` | `file` | 1 | 20 | `exists_active` | file exists |
| `entities-areatrigger.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-areatrigger.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `entities-areatrigger.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `entities-areatrigger.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `entities-areatrigger.md` | `crates/wow-data/src/area_trigger.rs` | `file` | 1 | 312 | `exists_active` | file exists |
| `entities-areatrigger.md` | `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `entities-areatrigger.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `entities-areatrigger.md` | `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `entities-areatrigger.md` | `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `entities-areatrigger.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `entities-conversation.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-conversation.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `entities-conversation.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `entities-conversation.md` | `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `entities-conversation.md` | `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `entities-conversation.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `entities-conversation.md` | `crates/wow-packet/src/packets/update.rs` | `file` | 1 | 3072 | `exists_active` | file exists |
| `entities-corpse.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-corpse.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `entities-corpse.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `entities-corpse.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `entities-corpse.md` | `crates/wow-constants/src/object.rs:24,75` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `entities-corpse.md` | `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `entities-corpse.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `entities-corpse.md` | `crates/wow-constants/src/spell.rs` | `file` | 1 | 569 | `exists_active` | file exists |
| `entities-corpse.md` | `crates/wow-constants/src/unit.rs` | `file` | 1 | 599 | `exists_active` | file exists |
| `entities-corpse.md` | `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `entities-corpse.md` | `crates/wow-constants/src/shared.rs` | `file` | 1 | 464 | `exists_active` | file exists |
| `entities-corpse.md` | `crates/wow-constants/src/item.rs` | `file` | 1 | 1239 | `exists_active` | file exists |
| `entities-corpse.md` | `crates/wow-packet/src/packets/update_stubs.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `entities-corpse.md` | `crates/wow-world/src/handlers/loot.rs:172,199-218` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `entities-corpse.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `entities-creature.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-creature.md` | `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `entities-creature.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `entities-creature.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `entities-creature.md` | `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `entities-creature.md` | `crates/wow-world/src/handlers/character.rs` | `file` | 1 | 4612 | `exists_active` | file exists |
| `entities-creature.md` | `crates/wow-world/src/handlers/loot.rs` | `file` | 1 | 247 | `exists_active` | file exists |
| `entities-creature.md` | `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `entities-creature.md` | `crates/wow-packet/src/packets/update.rs` | `file` | 1 | 3072 | `exists_active` | file exists |
| `entities-creature.md` | `crates/wow-database/src/world_ext.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `entities-dynamicobject.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-dynamicobject.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `entities-dynamicobject.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `entities-dynamicobject.md` | `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `entities-dynamicobject.md` | `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `entities-gameobject.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-gameobject.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `entities-gameobject.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `entities-gameobject.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `entities-gameobject.md` | `crates/wow-packet/src/packets/update.rs` | `file` | 1 | 3072 | `exists_active` | file exists |
| `entities-gameobject.md` | `crates/wow-world/src/handlers/character.rs:2109,2454` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `entities-gameobject.md` | `crates/wow-world/src/session.rs:264,444` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `entities-gameobject.md` | `crates/wow-world/src/session.rs:996,1196` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `entities-gameobject.md` | `crates/wow-database/src/statements/world.rs:79,253` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `entities-gameobject.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `entities-gameobject.md` | `crates/wow-constants/src/object.rs:22,72` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `entities-gameobject.md` | `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `entities-gameobject.md` | `crates/wow-packet/src/packets/query.rs` | `file` | 1 | 616 | `exists_active` | file exists |
| `entities-object.md` | `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `entities-object.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-object.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `entities-object.md` | `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `entities-object.md` | `crates/wow-core/src/position.rs` | `file` | 1 | 190 | `exists_active` | file exists |
| `entities-object.md` | `crates/wow-core/src/lib.rs` | `file` | 1 | 7 | `exists_active` | file exists |
| `entities-object.md` | `crates/wow-packet/src/world_packet.rs` | `file` | 1 | 673 | `exists_active` | file exists |
| `entities-object.md` | `crates/wow-packet/src/update.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `entities-object.md` | `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `entities-player.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-player.md` | `crates/wow-entities` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `entities-player.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `entities-player.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `entities-player.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `entities-player.md` | `crates/wow-loot` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `entities-player.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/handlers/character.rs` | `file` | 1 | 4612 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/handlers/quest.rs` | `file` | 1 | 851 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/handlers/spell.rs` | `file` | 1 | 288 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/handlers/loot.rs` | `file` | 1 | 247 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/handlers/group.rs` | `file` | 1 | 467 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/handlers/social.rs` | `file` | 1 | 360 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/handlers/movement.rs` | `file` | 1 | 204 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/handlers/combat.rs` | `file` | 1 | 152 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/handlers/chat.rs` | `file` | 1 | 413 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/handlers/trainer.rs` | `file` | 1 | 432 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/handlers/inspect.rs` | `file` | 1 | 82 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-database/src/statements/character.rs` | `file` | 1 | 284 | `exists_active` | file exists |
| `entities-player.md` | `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `entities-sceneobject.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-sceneobject.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `entities-sceneobject.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `entities-sceneobject.md` | `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `entities-sceneobject.md` | `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `entities-sceneobject.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `entities-sceneobject.md` | `crates/wow-packet/src/packets/update.rs` | `file` | 1 | 3072 | `exists_active` | file exists |
| `entities-totem.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-totem.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `entities-totem.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `entities-totem.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `entities-totem.md` | `crates/wow-constants/src/creature.rs` | `file` | 1 | 623 | `exists_active` | file exists |
| `entities-totem.md` | `crates/wow-constants/src/item.rs` | `file` | 1 | 1239 | `exists_active` | file exists |
| `entities-transport.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-transport.md` | `crates/wow-map` | `crate_dir` | 3 | 558 | `exists_active` | crate exists |
| `entities-transport.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `entities-transport.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `entities-transport.md` | `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `entities-transport.md` | `crates/wow-packet/src/packets/movement.rs` | `file` | 1 | 461 | `exists_active` | file exists |
| `entities-transport.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `entities-unit.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-unit.md` | `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `entities-unit.md` | `crates/wow-combat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `entities-unit.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `entities-unit.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `entities-unit.md` | `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `entities-unit.md` | `crates/wow-ai/src/lib.rs` | `file` | 1 | 346 | `exists_active` | file exists |
| `entities-unit.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `entities-unit.md` | `crates/wow-world/src/handlers/spell.rs` | `file` | 1 | 288 | `exists_active` | file exists |
| `entities-vehicle.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `entities-vehicle.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `entities-vehicle.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `entities-vehicle.md` | `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `entities-vehicle.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `entities-vehicle.md` | `crates/wow-packet/src/packets/movement.rs` | `file` | 1 | 461 | `exists_active` | file exists |
| `events.md` | `crates/wow-gameevents` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `events.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `events.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `events.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `events.md` | `crates/wow-database/src/statements/world.rs` | `file` | 1 | 371 | `exists_active` | file exists |
| `globals.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `globals.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `globals.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `globals.md` | `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `globals.md` | `crates/wow-data/src/quest.rs` | `file` | 1 | 337 | `exists_active` | file exists |
| `globals.md` | `crates/wow-data/src/quest_xp.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `globals.md` | `crates/wow-data/src/item.rs` | `file` | 1 | 123 | `exists_active` | file exists |
| `globals.md` | `crates/wow-data/src/spell.rs` | `file` | 1 | 225 | `exists_active` | file exists |
| `globals.md` | `crates/wow-data/src/skill.rs` | `file` | 1 | 608 | `exists_active` | file exists |
| `globals.md` | `crates/wow-data/src/area_trigger.rs` | `file` | 1 | 312 | `exists_active` | file exists |
| `globals.md` | `crates/wow-data/src/player_stats.rs` | `file` | 1 | 307 | `exists_active` | file exists |
| `globals.md` | `crates/wow-data/src/hotfix_cache.rs` | `file` | 1 | 111 | `exists_active` | file exists |
| `globals.md` | `crates/wow-data/src/wdc4.rs` | `file` | 1 | 915 | `exists_active` | file exists |
| `globals.md` | `crates/wow-database/src/statements/{world.rs,character.rs,login.rs,hotfix.rs}` | `declared_pattern` | 0 | 0 | `declared_pattern` | pattern/proposed path; not resolvable as one file or directory |
| `globals.md` | `crates/wow-network/src/player_registry.rs` | `file` | 1 | 47 | `exists_active` | file exists |
| `globals.md` | `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `globals.md` | `crates/wow-world/src/handlers/character.rs` | `file` | 1 | 4612 | `exists_active` | file exists |
| `globals.md` | `crates/wow-data/src/{quest,item,player_stats}.rs` | `declared_pattern` | 0 | 0 | `declared_pattern` | pattern/proposed path; not resolvable as one file or directory |
| `grids.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `grids.md` | `crates/wow-map/src/coords.rs` | `file` | 1 | 254 | `exists_active` | file exists |
| `grids.md` | `crates/wow-map/src/cell.rs` | `file` | 1 | 234 | `exists_active` | file exists |
| `grids.md` | `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `grids.md` | `crates/wow-map` | `crate_dir` | 3 | 558 | `exists_active` | crate exists |
| `groups.md` | `crates/wow-network/src/group_registry.rs` | `file` | 1 | 53 | `exists_active` | file exists |
| `groups.md` | `crates/wow-world/src/handlers/group.rs` | `file` | 1 | 467 | `exists_active` | file exists |
| `groups.md` | `crates/wow-packet/src/packets/party.rs` | `file` | 1 | 302 | `exists_active` | file exists |
| `guilds.md` | `crates/wow-guild` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `guilds.md` | `crates/wow-world/src/handlers/guild.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `guilds.md` | `crates/wow-packet/src/packets/guild.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `guilds.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `guilds.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `handlers.md` | `crates/wow-world/src/handlers` | `module_dir` | 14 | 8843 | `exists_active` | directory exists |
| `handlers.md` | `crates/wow-handler` | `crate_dir` | 1 | 116 | `exists_active` | crate exists |
| `handlers.md` | `crates/wow-world/tests` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `instances.md` | `crates/wow-instances` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `instances.md` | `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `inventory.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `inventory.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `inventory.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `inventory.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `inventory.md` | `crates/wow-data/src/item.rs` | `file` | 1 | 123 | `exists_active` | file exists |
| `inventory.md` | `crates/wow-data/src/item_stats.rs` | `file` | 1 | 424 | `exists_active` | file exists |
| `inventory.md` | `crates/wow-packet/src/packets/item.rs` | `file` | 1 | 395 | `exists_active` | file exists |
| `inventory.md` | `crates/wow-world/src/handlers/character.rs` | `file` | 1 | 4612 | `exists_active` | file exists |
| `inventory.md` | `crates/wow-database/src/statements/character.rs` | `file` | 1 | 284 | `exists_active` | file exists |
| `logging.md` | `crates/wow-logging` | `crate_dir` | 1 | 464 | `exists_active` | crate exists |
| `logging.md` | `crates/wow-logging/Cargo.toml` | `file` | 1 | 15 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `logging.md` | `crates/wow-config` | `crate_dir` | 1 | 397 | `exists_active` | crate exists |
| `logging.md` | `crates/wow-logging/src/lib.rs` | `file` | 1 | 464 | `exists_active` | file exists |
| `loot.md` | `crates/wow-loot` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `loot.md` | `crates/wow-packet/src/packets/loot.rs` | `file` | 1 | 210 | `exists_active` | file exists |
| `loot.md` | `crates/wow-world/src/handlers/loot.rs` | `file` | 1 | 247 | `exists_active` | file exists |
| `loot.md` | `crates/wow-loot/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `loot.md` | `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `loot.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `loot.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `mails.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `mails.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `mails.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `mails.md` | `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `maps.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `maps.md` | `crates/wow-map` | `crate_dir` | 3 | 558 | `exists_active` | crate exists |
| `maps.md` | `crates/wow-map/src/coords.rs` | `file` | 1 | 254 | `exists_active` | file exists |
| `maps.md` | `crates/wow-map/src/cell.rs` | `file` | 1 | 234 | `exists_active` | file exists |
| `maps.md` | `crates/wow-map/src/lib.rs` | `file` | 1 | 70 | `exists_active` | file exists |
| `maps.md` | `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `maps.md` | `crates/wow-world/src/map.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `maps.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `miscellaneous.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `miscellaneous.md` | `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `miscellaneous.md` | `crates/wow-combat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `miscellaneous.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `miscellaneous.md` | `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `miscellaneous.md` | `crates/wow-constants/src/lib.rs` | `file` | 1 | 30 | `exists_active` | file exists |
| `miscellaneous.md` | `crates/wow-constants/src/shared.rs` | `file` | 1 | 464 | `exists_active` | file exists |
| `miscellaneous.md` | `crates/wow-data/src/quest_xp.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `miscellaneous.md` | `crates/wow-data/src/player_stats.rs` | `file` | 1 | 307 | `exists_active` | file exists |
| `movement-generators.md` | `crates/wow-movement/src/motion_master.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `movement-generators.md` | `crates/wow-movement/src/generators` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `movement-generators.md` | `crates/wow-movement` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `movement-generators.md` | `crates/wow-ai/src` | `module_dir` | 1 | 346 | `exists_active` | directory exists |
| `movement-generators.md` | `crates/wow-world/src/handlers/movement.rs` | `file` | 1 | 204 | `exists_active` | file exists |
| `movement-generators.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `movement-generators.md` | `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `movement-generators.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `movement-pathgen.md` | `crates/wow-recastdetour` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `movement-pathgen.md` | `crates/wow-movement/src/path_generator.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `movement-pathgen.md` | `crates/wow-recastdetour/Cargo.toml` | `file` | 1 | 9 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `movement-pathgen.md` | `crates/wow-recastdetour/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `movement-spline.md` | `crates/wow-movement/src/spline` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `movement-spline.md` | `crates/wow-movement` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `movement-spline.md` | `crates/wow-packet/src/packets/movement.rs` | `file` | 1 | 461 | `exists_active` | file exists |
| `movement.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `movement.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `movement.md` | `crates/wow-recastdetour` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `movement.md` | `crates/wow-movement` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `movement.md` | `crates/wow-packet/src/packets/movement.rs` | `file` | 1 | 461 | `exists_active` | file exists |
| `movement.md` | `crates/wow-world/src/handlers/movement.rs` | `file` | 1 | 204 | `exists_active` | file exists |
| `movement.md` | `crates/wow-recastdetour/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `movement.md` | `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `outdoorpvp.md` | `crates/wow-pvp` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `outdoorpvp.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `petitions.md` | `crates/wow-world/src/petitions` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `petitions.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `petitions.md` | `crates/wow-packet/src/packets/petitions.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `petitions.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `petitions.md` | `crates/wow-world/src/handlers/petitions.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `petitions.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `petitions.md` | `crates/wow-world/src/handlers` | `module_dir` | 14 | 8843 | `exists_active` | directory exists |
| `pets.md` | `crates/wow-world/src/pets` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `pets.md` | `crates/wow-world/src/handlers/pets.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `pets.md` | `crates/wow-packet/src/packets/pets.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `pets.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `pets.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `pets.md` | `crates/wow-world/src/handlers/character.rs:3040–3045` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `pets.md` | `crates/wow-database/src/statements` | `module_dir` | 5 | 1100 | `exists_active` | directory exists |
| `pets.md` | `crates/wow-ai/src` | `module_dir` | 1 | 346 | `exists_active` | directory exists |
| `pets.md` | `crates/wow-handler/src/lib.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `phasing.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `phasing.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `phasing.md` | `crates/wow-packet/src/packets/misc.rs` | `file` | 1 | 2613 | `exists_active` | file exists |
| `phasing.md` | `crates/wow-phasing` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `pools.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `pools.md` | `crates/wow-world/src/pools` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `pools.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `pools.md` | `crates/wow-pools` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `proto.md` | `crates/wow-proto` | `crate_dir` | 2 | 254 | `exists_active` | crate exists |
| `proto.md` | `crates/wow-proto/Cargo.toml` | `file` | 1 | 14 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `proto.md` | `crates/wow-proto/build.rs` | `file` | 1 | 30 | `exists_active` | file exists |
| `proto.md` | `crates/wow-proto/src/lib.rs` | `file` | 1 | 224 | `exists_active` | file exists |
| `proto.md` | `crates/wow-proto/proto/bgs/low/pb/client` | `module_dir` | 0 | 0 | `exists_empty` | directory exists; no active Rust source lines |
| `proto.md` | `crates/bnet-server` | `crate_dir` | 13 | 2831 | `exists_active` | crate exists |
| `proto.md` | `crates/wow-handler` | `crate_dir` | 1 | 116 | `exists_active` | crate exists |
| `quests.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `quests.md` | `crates/wow-packet/src/packets/quest.rs` | `file` | 1 | 603 | `exists_active` | file exists |
| `quests.md` | `crates/wow-world/src/handlers/quest.rs` | `file` | 1 | 851 | `exists_active` | file exists |
| `quests.md` | `crates/wow-quest` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `quests.md` | `crates/wow-data/src/quest.rs` | `file` | 1 | 337 | `exists_active` | file exists |
| `quests.md` | `crates/wow-data/src/quest_xp.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `quests.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `reputation.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `reputation.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `reputation.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `reputation.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `scenarios.md` | `crates/wow-world/src/scenarios` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `scripting.md` | `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `scripting.md` | `crates/wow-scripts` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `scripting.md` | `crates/wow-script/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `scripting.md` | `crates/wow-script/Cargo.toml` | `file` | 1 | 11 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `scripting.md` | `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `scripting.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `scripting.md` | `crates/wow-scripts/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `scripting.md` | `crates/wow-handler` | `crate_dir` | 1 | 116 | `exists_active` | crate exists |
| `scripting.md` | `crates/wow-handler/src/lib.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `scripts-icc.md` | `crates/wow-scripts` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `scripts-icc.md` | `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `scripts-icc.md` | `crates/wow-scripts/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `scripts-icc.md` | `crates/wow-script/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `scripts-icc.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `scripts-icc.md` | `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |
| `scripts-ulduar.md` | `crates/wow-scripts` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `scripts-ulduar.md` | `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `scripts-ulduar.md` | `crates/wow-scripts/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `scripts-ulduar.md` | `crates/wow-script/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `scripts-ulduar.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `scripts.md` | `crates/wow-scripts` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `scripts.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `scripts.md` | `crates/wow-pvp` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `scripts.md` | `crates/wow-battleground` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `scripts.md` | `crates/wow-pet` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `scripts.md` | `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `scripts.md` | `crates/wow-scripts/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `scripts.md` | `crates/wow-scripts/Cargo.toml` | `file` | 1 | 11 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `scripts.md` | `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `scripts.md` | `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `server.md` | `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `server.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `server.md` | `crates/wow-network/src/world_socket.rs` | `file` | 1 | 1023 | `exists_active` | file exists |
| `server.md` | `crates/wow-network/src/session_mgr.rs` | `file` | 1 | 188 | `exists_active` | file exists |
| `server.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `server.md` | `crates/world-server/src/main.rs` | `file` | 1 | 818 | `exists_active` | file exists |
| `services.md` | `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `services.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `services.md` | `crates/wow-proto` | `crate_dir` | 2 | 254 | `exists_active` | crate exists |
| `services.md` | `crates/bnet-server/src/rpc/services/{account,authentication,connection,game_utilities}.rs` | `declared_pattern` | 0 | 0 | `declared_pattern` | pattern/proposed path; not resolvable as one file or directory |
| `services.md` | `crates/wow-proto/proto` | `module_dir` | 0 | 0 | `exists_empty` | directory exists; no active Rust source lines |
| `services.md` | `crates/wow-world/src` | `module_dir` | 17 | 12778 | `exists_active` | directory exists |
| `services.md` | `crates/wow-social` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `services.md` | `crates/wow-chat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `shared-datastores.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `shared-datastores.md` | `crates/wow-data/src/wdc4.rs` | `file` | 1 | 915 | `exists_active` | file exists |
| `shared-datastores.md` | `crates/wow-data/src/hotfix_cache.rs` | `file` | 1 | 111 | `exists_active` | file exists |
| `shared-datastores.md` | `crates/wow-data/src/{item,item_stats,player_stats,skill,area_trigger,spell,quest,quest_xp}.rs` | `declared_pattern` | 0 | 0 | `declared_pattern` | pattern/proposed path; not resolvable as one file or directory |
| `shared-datastores.md` | `crates/wow-database/src/statements/hotfix.rs` | `file` | 1 | 25 | `exists_active` | file exists |
| `shared-datastores.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `shared-dynamic.md` | `crates/wow-handler/src/lib.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `shared-dynamic.md` | `crates/wow-handler` | `crate_dir` | 1 | 116 | `exists_active` | crate exists |
| `shared-dynamic.md` | `crates/wow-combat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `shared-json.md` | `crates/bnet-server/src/realm/mod.rs` | `file` | 1 | 392 | `exists_active` | file exists |
| `shared-json.md` | `crates/wow-proto` | `crate_dir` | 2 | 254 | `exists_active` | crate exists |
| `shared-networking.md` | `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `shared-networking.md` | `crates/wow-network/src/world_socket.rs` | `file` | 1 | 1023 | `exists_active` | file exists |
| `shared-networking.md` | `crates/wow-network/src/accept.rs` | `file` | 1 | 386 | `exists_active` | file exists |
| `shared-networking.md` | `crates/wow-network/src/session_mgr.rs` | `file` | 1 | 188 | `exists_active` | file exists |
| `shared-networking.md` | `crates/wow-network/src/lib.rs` | `file` | 1 | 19 | `exists_active` | file exists |
| `shared-networking.md` | `crates/wow-network/src/player_registry.rs` | `file` | 1 | 47 | `exists_active` | file exists |
| `shared-networking.md` | `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |
| `shared-packets.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `shared-packets.md` | `crates/wow-packet/src/world_packet.rs` | `file` | 1 | 673 | `exists_active` | file exists |
| `shared-packets.md` | `crates/wow-packet/src/header.rs` | `file` | 1 | 100 | `exists_active` | file exists |
| `shared-packets.md` | `crates/wow-packet/src/compression.rs` | `file` | 1 | 424 | `exists_active` | file exists |
| `shared-packets.md` | `crates/wow-packet/src/lib.rs` | `file` | 1 | 48 | `exists_active` | file exists |
| `shared-packets.md` | `crates/wow-packet/src/packets/mod.rs` | `file` | 1 | 26 | `exists_active` | file exists |
| `shared-packets.md` | `crates/wow-packet/tests` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `shared-realm.md` | `crates/bnet-server` | `crate_dir` | 13 | 2831 | `exists_active` | crate exists |
| `shared-realm.md` | `crates/bnet-server/src/realm/mod.rs` | `file` | 1 | 392 | `exists_active` | file exists |
| `shared-realm.md` | `crates/wow-database/src/statements/login.rs` | `file` | 1 | 327 | `exists_active` | file exists |
| `shared-secrets.md` | `crates/wow-crypto` | `crate_dir` | 9 | 2327 | `exists_active` | crate exists |
| `shared-secrets.md` | `crates/bnet-server` | `crate_dir` | 13 | 2831 | `exists_active` | crate exists |
| `shared-secrets.md` | `crates/wow-database/src/statements/login.rs` | `file` | 1 | 327 | `exists_active` | file exists |
| `shared-secrets.md` | `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |
| `skills.md` | `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `skills.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `skills.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `skills.md` | `crates/wow-data/src/skill.rs` | `file` | 1 | 608 | `exists_active` | file exists |
| `spells-aura.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `spells-aura.md` | `crates/wow-packet/src/packets/aura.rs` | `file` | 1 | 123 | `exists_active` | file exists |
| `spells-aura.md` | `crates/wow-spell/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `spells-aura.md` | `crates/wow-world/src/handlers/spell.rs` | `file` | 1 | 288 | `exists_active` | file exists |
| `spells-aura.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `spells-effects.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `spells-effects.md` | `crates/wow-spell/src/effects/dispatch.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `spells-effects.md` | `crates/wow-spell/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `spells-effects.md` | `crates/wow-world/src/handlers/spell.rs` | `file` | 1 | 288 | `exists_active` | file exists |
| `spells-effects.md` | `crates/wow-packet/src/packets/spell.rs` | `file` | 1 | 466 | `exists_active` | file exists |
| `spells-effects.md` | `crates/wow-data/src/spell_info.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `spells-effects.md` | `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `spells-info.md` | `crates/wow-spell/src/spell_info.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `spells-info.md` | `crates/wow-data/src/spell.rs` | `file` | 1 | 225 | `exists_active` | file exists |
| `spells-info.md` | `crates/wow-spell/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `spells-info.md` | `crates/wow-world/src/handlers/spell.rs` | `file` | 1 | 288 | `exists_active` | file exists |
| `spells-info.md` | `crates/wow-data/src/wdc4.rs` | `file` | 1 | 915 | `exists_active` | file exists |
| `spells-info.md` | `crates/wow-data/src/hotfix_cache.rs` | `file` | 1 | 111 | `exists_active` | file exists |
| `spells-info.md` | `crates/wow-spell/src/diminish` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `spells-info.md` | `crates/wow-spell/src/immunity` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `spells-info.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `spells-mgr.md` | `crates/wow-spell/src/spell_mgr.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `spells-mgr.md` | `crates/wow-data/src/spell.rs` | `file` | 1 | 225 | `exists_active` | file exists |
| `spells-mgr.md` | `crates/wow-spell/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `spells-mgr.md` | `crates/wow-data/src/wdc4.rs` | `file` | 1 | 915 | `exists_active` | file exists |
| `spells-mgr.md` | `crates/wow-data/src/hotfix_cache.rs` | `file` | 1 | 111 | `exists_active` | file exists |
| `spells-mgr.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `spells.md` | `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `spells.md` | `crates/wow-world/src/handlers/spell.rs` | `file` | 1 | 288 | `exists_active` | file exists |
| `spells.md` | `crates/wow-packet/src/packets/{spell,aura}.rs` | `declared_pattern` | 0 | 0 | `declared_pattern` | pattern/proposed path; not resolvable as one file or directory |
| `spells.md` | `crates/wow-spell/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `spells.md` | `crates/wow-packet/src/packets/spell.rs` | `file` | 1 | 466 | `exists_active` | file exists |
| `spells.md` | `crates/wow-packet/src/packets/aura.rs` | `file` | 1 | 123 | `exists_active` | file exists |
| `spells.md` | `crates/wow-data/src/spell_info.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `spells.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `storages.md` | `crates/wow-social` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `storages.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `storages.md` | `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `support.md` | `crates/wow-support` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `support.md` | `crates/wow-social` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `support.md` | `crates/wow-handler` | `crate_dir` | 1 | 116 | `exists_active` | crate exists |
| `support.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `support.md` | `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `support.md` | `crates/wow-packet/src/packets/misc.rs` | `file` | 1 | 2613 | `exists_active` | file exists |
| `support.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `support.md` | `crates/wow-database/src/statements` | `module_dir` | 5 | 1100 | `exists_active` | directory exists |
| `texts.md` | `crates/wow-chat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `texts.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `texts.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `texts.md` | `crates/wow-database/src/statements/world.rs` | `file` | 1 | 371 | `exists_active` | file exists |
| `time.md` | `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `time.md` | `crates/wow-core/src/time.rs` | `file` | 1 | 166 | `exists_active` | file exists |
| `time.md` | `crates/wow-packet/src/packets/misc.rs` | `file` | 1 | 2613 | `exists_active` | file exists |
| `tools.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `tools.md` | `crates/wow-tools` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `tools.md` | `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |
| `warden.md` | `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `warden.md` | `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `weather.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `weather.md` | `crates/wow-weather` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `world.md` | `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |
| `world.md` | `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `world.md` | `crates/world-server/src/main.rs` | `file` | 1 | 818 | `exists_active` | file exists |
| `world.md` | `crates/wow-world/src/lib.rs` | `file` | 1 | 13 | `exists_active` | file exists |
| `world.md` | `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |
| `worldserver.md` | `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |
| `worldserver.md` | `crates/world-server/src/main.rs` | `file` | 1 | 818 | `exists_active` | file exists |
| `worldserver.md` | `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `worldserver.md` | `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `worldserver.md` | `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
