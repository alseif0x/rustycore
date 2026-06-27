# Migration: game/Handlers

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Handlers/`
> **Rust target crate(s):** `crates/wow-world/src/handlers/` + `crates/wow-handler/`
> **Layer:** L2
> **Status:** ⚠️ partial (~22% coverage by handler count, ~23% by registered opcodes)
> **Audited vs C++:** ⚠️ audited 2026-05-01 — see §13. 121/~560 C++ HandleXxx covered, 19 of 45 C++ handler families have ZERO Rust counterpart, multiple silent read-order bugs in implemented handlers.
> **Last updated:** 2026-05-01

---

## 1. Purpose

Procesa packets CMSG del cliente y produce SMSG. ~560 métodos `HandleXxx` distribuidos en 45 archivos C++ (~21.8K líneas). Los handlers son métodos de `WorldSession` en C++; en Rust son funciones libres organizadas por dominio (`character.rs`, `chat.rs`, `movement.rs`, etc.) registradas vía `inventory::submit!` para mapeo opcode → handler.

---

## 2. C++ canonical files (45 handlers)

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Handlers/AdventureJournalHandler.cpp` | 67 | `prefix` |
| `game/Handlers/AdventureMapHandler.cpp` | 40 | `prefix` |
| `game/Handlers/AuctionHouseHandler.cpp` | 1124 | `prefix` |
| `game/Handlers/AuthHandler.cpp` | 126 | `prefix` |
| `game/Handlers/BankHandler.cpp` | 324 | `prefix` |
| `game/Handlers/BattleGroundHandler.cpp` | 1346 | `prefix` |
| `game/Handlers/BattlePetHandler.cpp` | 134 | `prefix` |
| `game/Handlers/BattlenetHandler.cpp` | 88 | `prefix` |
| `game/Handlers/BlackMarketHandler.cpp` | 158 | `prefix` |
| `game/Handlers/CalendarHandler.cpp` | 575 | `prefix` |
| `game/Handlers/ChannelHandler.cpp` | 219 | `prefix` |
| `game/Handlers/CharacterHandler.cpp` | 2895 | `prefix` |
| `game/Handlers/ChatHandler.cpp` | 830 | `prefix` |
| `game/Handlers/CollectionsHandler.cpp` | 43 | `prefix` |
| `game/Handlers/CombatHandler.cpp` | 82 | `prefix` |
| `game/Handlers/DuelHandler.cpp` | 106 | `prefix` |
| `game/Handlers/GarrisonHandler.cpp` | 44 | `prefix` |
| `game/Handlers/GroupHandler.cpp` | 783 | `prefix` |
| `game/Handlers/GuildHandler.cpp` | 813 | `prefix` |
| `game/Handlers/HotfixHandler.cpp` | 126 | `prefix` |
| `game/Handlers/InspectHandler.cpp` | 152 | `prefix` |
| `game/Handlers/ItemHandler.cpp` | 1220 | `prefix` |
| `game/Handlers/LFGHandler.cpp` | 971 | `prefix` |
| `game/Handlers/LootHandler.cpp` | 508 | `prefix` |
| `game/Handlers/MailHandler.cpp` | 676 | `prefix` |
| `game/Handlers/MiscHandler.cpp` | 1440 | `prefix` |
| `game/Handlers/MovementHandler.cpp` | 816 | `prefix` |
| `game/Handlers/NPCHandler.cpp` | 577 | `prefix` |
| `game/Handlers/NPCHandler.h` | 39 | `prefix` |
| `game/Handlers/PetHandler.cpp` | 810 | `prefix` |
| `game/Handlers/PetitionsHandler.cpp` | 464 | `prefix` |
| `game/Handlers/QueryHandler.cpp` | 333 | `prefix` |
| `game/Handlers/QuestHandler.cpp` | 849 | `prefix` |
| `game/Handlers/ScenarioHandler.cpp` | 43 | `prefix` |
| `game/Handlers/SceneHandler.cpp` | 43 | `prefix` |
| `game/Handlers/SkillHandler.cpp` | 117 | `prefix` |
| `game/Handlers/SocialHandler.cpp` | 184 | `prefix` |
| `game/Handlers/SpellHandler.cpp` | 569 | `prefix` |
| `game/Handlers/TaxiHandler.cpp` | 239 | `prefix` |
| `game/Handlers/TicketHandler.cpp` | 152 | `prefix` |
| `game/Handlers/TokenHandler.cpp` | 43 | `prefix` |
| `game/Handlers/ToyHandler.cpp` | 102 | `prefix` |
| `game/Handlers/TradeHandler.cpp` | 814 | `prefix` |
| `game/Handlers/TransmogrificationHandler.cpp` | 314 | `prefix` |
| `game/Handlers/VehicleHandler.cpp` | 196 | `prefix` |
| `game/Handlers/VoidStorageHandler.cpp` | 249 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Opcodes (~) | Purpose |
|---|---|---|---|
| `AdventureJournalHandler.cpp` | 67 | 1-2 | Adventure journal (legion+, stub) |
| `AdventureMapHandler.cpp` | 40 | 1 | Adventure map (legion+, stub) |
| `AuctionHouseHandler.cpp` | 1124 | 10-12 | Browse, bid, sell |
| `AuthHandler.cpp` | 126 | 3-5 | Session auth, version, addon list |
| `BankHandler.cpp` | 324 | 8-10 | Bank/vault access |
| `BattleGroundHandler.cpp` | 1346 | 15-20 | BG queue, leave, port |
| `BattlenetHandler.cpp` | 88 | 2-3 | BNet roster (legion+) |
| `BattlePetHandler.cpp` | 134 | 5-7 | Pet journal (MoP+) |
| `BlackMarketHandler.cpp` | 158 | 3-5 | Black market (MoP+) |
| `CalendarHandler.cpp` | 575 | 10-15 | Calendar events, RSVP |
| `ChannelHandler.cpp` | 219 | 8-10 | Chat channels join/leave/mod |
| `CharacterHandler.cpp` | 2884 | 25-30 | Char enum/create/delete/login/customize/rename |
| `ChatHandler.cpp` | 830 | 18-22 | Say/yell/whisper/emote/addon |
| `CollectionsHandler.cpp` | 43 | 1-2 | Toy/heirloom (WoD+, stub) |
| `CombatHandler.cpp` | 82 | 3-5 | Attack swing/stop, duel legacy |
| `DuelHandler.cpp` | 106 | 3-5 | Duel accept/decline/forfeit |
| `GarrisonHandler.cpp` | 44 | 1-2 | Garrison (WoD+, stub) |
| `GroupHandler.cpp` | 783 | 15-18 | Party invite/leave, loot, leader |
| `GuildHandler.cpp` | 813 | 18-22 | Guild create/invite/roster/rank |
| `HotfixHandler.cpp` | 126 | 2-3 | DB2 hotfix request (MoP+) |
| `InspectHandler.cpp` | 152 | 3-5 | Inspect player/talents/glyphs |
| `ItemHandler.cpp` | 1220 | 15-18 | Swap, burn, sell, container |
| `LFGHandler.cpp` | 971 | 15-20 | LFG queue/cancel/role/teleport |
| `LootHandler.cpp` | 508 | 8-12 | Loot item/pass/disenchant/money |
| `MailHandler.cpp` | 676 | 10-12 | Mail list/body/send/delete |
| `MiscHandler.cpp` | 1440 | 25-35 | Repop, tutorial, reset, taxi |
| `MovementHandler.cpp` | 816 | 18-25 | Move/jump/fall/teleport/transport |
| `NPCHandler.cpp` | 577 | 10-15 | NPC quest list, gossip, vendor, stable, trainer |
| `PetHandler.cpp` | 810 | 12-18 | Pet command/summon/control |
| `PetitionsHandler.cpp` | 464 | 6-8 | Guild petition sign |
| `QueryHandler.cpp` | 333 | 4-6 | Query creature/GO/quest/NPC text |
| `QuestHandler.cpp` | 849 | 12-16 | Quest accept/complete/abandon |
| `ScenarioHandler.cpp` | 43 | 1-2 | Scenario (MoP+, stub) |
| `SceneHandler.cpp` | 43 | 1-2 | Scene (WoD+, stub) |
| `SkillHandler.cpp` | 117 | 3-5 | Skill change/initial |
| `SocialHandler.cpp` | 184 | 5-8 | Friend/ignore list |
| `SpellHandler.cpp` | 569 | 10-15 | Cast/cancel cast/skill learn |
| `TaxiHandler.cpp` | 239 | 5-7 | Taxi node list, activate |
| `TicketHandler.cpp` | 152 | 3-5 | Bug report, GM ticket |
| `TokenHandler.cpp` | 43 | 1-2 | Token redeem (WoD+, stub) |
| `ToyHandler.cpp` | 102 | 2-3 | Toy list/use (WoD+) |
| `TradeHandler.cpp` | 814 | 12-15 | Trade begin/items/accept/money |
| `TransmogrificationHandler.cpp` | 314 | 4-6 | Transmog appearance |
| `VehicleHandler.cpp` | 196 | 5-7 | Vehicle enter/exit/seat |
| `VoidStorageHandler.cpp` | 249 | 5-7 | Void storage (Cata+) |
| **TOTAL** | **~21,793** | **~260** | **45 archivos, ~560 métodos `HandleXxx`** |

---

## 3. Grouped handler summary (high-level domains)

- **Character / Session (CharacterHandler):** 25-30 opcodes — login, create, customize, rename, faction change, equipment sets
- **Chat (ChatHandler + ChannelHandler):** 25-28 opcodes — say/yell/whisper/channel/emote/addon
- **Movement (MovementHandler):** 18-25 opcodes — move, jump, fall, teleport, transport, heartbeat
- **Combat / Duel (CombatHandler + DuelHandler):** 6-10 opcodes
- **Social (SocialHandler):** 5-8 opcodes — friends, ignore
- **Group / Raid (GroupHandler):** 15-18 opcodes — invite, leave, role, loot
- **Guild (GuildHandler):** 18-22 opcodes — create, roster, bank
- **Quest (QuestHandler):** 12-16 opcodes
- **Items (ItemHandler + BankHandler + VoidStorageHandler):** 35-42 opcodes — swap, equip, bank, void
- **Spell / Skill (SpellHandler + SkillHandler):** 13-20 opcodes
- **NPC / Vendor / Trainer (NPCHandler):** 10-15 opcodes — gossip, list inventory, train
- **Auction (AuctionHouseHandler):** 10-12 opcodes
- **Loot (LootHandler):** 8-12 opcodes
- **Mail (MailHandler):** 10-12 opcodes
- **Battlegrounds (BattleGroundHandler):** 15-20 opcodes
- **LFG (LFGHandler):** 15-20 opcodes
- **Calendar (CalendarHandler):** 10-15 opcodes
- **Misc (MiscHandler):** 25-35 opcodes (taxi, tutorial, reset, time sync, etc.)

---

## 4. Top-level methods of `WorldSession::HandleXxx`

(Demasiados para listar todos; los más críticos):

| Class::Method | Purpose | Calls into |
|---|---|---|
| `WorldSession::HandlePlayerLoginOpcode` | Carga jugador en mundo | DB queries (~10), `Map::AddPlayerToMap`, init packets |
| `WorldSession::HandleCharEnumOpcode` | Lista personajes | `CHAR_SEL_ENUM` |
| `WorldSession::HandleCharCreateOpcode` | Crea personaje | `CharacterDatabase::Insert`, validations |
| `WorldSession::HandleCharDeleteOpcode` | Elimina (soft) | `CHAR_DEL_CHARACTER` |
| `WorldSession::HandleMovementOpcodes` | Multi-opcode movement | `Map::PlayerRelocation`, anti-cheat |
| `WorldSession::HandleCastSpellOpcode` | Cast spell | `Spell::prepare`, `SpellMgr::GetSpellInfo` |
| `WorldSession::HandleQuestgiverAcceptQuestOpcode` | Accept quest | `Player::AddQuestAndCheckCompletion` |
| `WorldSession::HandleAuctionListItems` | Browse AH | `AuctionMgr::BuildListAuctionItems` |
| `WorldSession::HandleSendMail` | Mail send | `MailMgr::Send` |
| `WorldSession::HandleGuildCreate` | Create guild | `GuildMgr::CreateGuild` |
| `WorldSession::HandleAttackSwing` | Auto-attack | `Unit::AttackerStateUpdate` |
| `WorldSession::HandleLoginOpcode` (auth) | CMSG_AUTH_SESSION | SRP6 verify, derive session key |

---

## 5. Module dependencies

**Depends on:**
- `shared/Packets/ByteBuffer.h` — deserialización
- `game/Server/Protocol/Opcodes.h` — enums
- `game/Entities/Player/Player.h` — `m_player` ptr en cada handler
- `game/Entities/Object/Object.h` — `ObjectGuid`, `ObjectAccessor`
- Numerosos managers (`AuctionMgr`, `MailMgr`, `GuildMgr`, `SpellMgr`, `LootMgr`, `QuestMgr`, etc.)
- `DatabaseEnv.h` — prepared statements

**Depended on by:**
- `game/World/WorldSession.h` — todos son métodos de `WorldSession`
- `game/Scripting/ScriptMgr.h` — scripts pueden hookear pre/post handler

---

## 6. SQL / DB queries

100+ prepared statements emitidos por handlers. Ejemplos:

| Handler | Statement | DB |
|---|---|---|
| HandlePlayerLogin | `CHAR_SEL_CHARACTER`, `CHAR_SEL_CHARACTER_AURAS`, etc. | character |
| HandleCharDelete | `CHAR_DEL_CHARACTER` | character |
| HandleGuildCreate | `GUILD_CREATE` | character |
| HandleGuildRoster | `GUILD_SEL_MEMBERS` | character |
| HandleAutoEquipItem | `CHAR_UPD_CHARACTER_ITEM_INSTANCE` | character |
| HandleSendMail | `MAIL_INSERT` | character |
| HandleQuestAccept | `CHAR_UPD_QUEST_STATUS` | character |
| HandleCastSpell | `SPELL_SEL_COOLDOWN` | world |

Sin SQL inline; todos prepared.

---

## 7. Wire-protocol opcodes mapping (C++ vs Rust)

**Resumen ejecutivo:** TrinityCore implementa ~260 opcodes únicos repartidos en 45 handlers. RustyCore tiene ~101 opcodes registrados en 13 handler files = **~39% coverage**.

### Character (30 opcodes C++)

| Opcode | C++ | Rust | Status |
|---|---|---|---|
| `CMSG_ENUM_CHARACTERS` | HandleCharEnum | character::handle_enum_characters | ✅ |
| `CMSG_CREATE_CHARACTER` | HandleCharCreate | character::handle_create_character | ✅ |
| `CMSG_CHAR_DELETE` | HandleCharDelete | character::handle_char_delete | ✅ |
| `CMSG_PLAYER_LOGIN` | HandlePlayerLogin | character::handle_player_login | ✅ |
| `CMSG_LOADING_SCREEN_NOTIFY` | HandleLoadScreen | (stub) | ⚠️ |
| `CMSG_CHARACTER_RENAME_REQUEST` | HandleCharRename | (missing) | ❌ |
| `CMSG_CHAR_CUSTOMIZE` | HandleCharCustomize | (missing) | ❌ |
| `CMSG_CHAR_RACE_OR_FACTION_CHANGE` | HandleCharRaceOrFactionChange | (missing) | ❌ |
| `CMSG_ALTER_APPEARANCE` | HandleAlterAppearance | (missing) | ❌ |
| `CMSG_EQUIPMENT_SET_SAVE` | HandleEquipmentSetSave | (missing) | ❌ |
| ... (~20 más) | ... | (missing) | ❌ |

### Chat (25 opcodes C++)

Mostly OK — Rust cubre say/yell/whisper/channel/guild/party/raid/emote, falta channel mod/owner/password/banlist.

### Movement (20 opcodes C++)

Mostly partial — Rust cubre move forward/back/jump/fall/teleport/heartbeat/ack. Faltan opcodes especializados (transport, vehicle, swim, etc.).

### Combat (8 opcodes)

Rust: `attack_swing`, `attack_stop`. Faltan duel-related (5).

### Guilds (20 opcodes) — **TODO MISSING (0%)**

### Auctions (10 opcodes) — **TODO MISSING (0%)**

### Mail (10 opcodes) — **TODO MISSING (0%)**

### Bank (8 opcodes) — **TODO MISSING (0%)**

### LFG (15 opcodes) — **TODO MISSING (0%)**

### Calendar (10 opcodes) — **TODO MISSING (0%)**

### Battlegrounds (15 opcodes) — **TODO MISSING (0%)**

### Items extras (swap/split/transmog/void) — **TODO MISSING**

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world/src/handlers` | `module_dir` | 14 | 8843 | `exists_active` | directory exists |
| `crates/wow-handler` | `crate_dir` | 1 | 116 | `exists_active` | crate exists |
| `crates/wow-world/tests` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |

<!-- REFINE.021:END rust-target-coverage -->

**Handler files in `/home/server/rustycore/crates/wow-world/src/handlers/`:**

| File | Opcodes | Coverage of corresponding C++ |
|---|---|---|
| `character.rs` | 13 | ~40% CharacterHandler |
| `chat.rs` | 15 | ~60% ChatHandler |
| `movement.rs` | 12 | ~60% MovementHandler |
| `combat.rs` | 3 | ~40% CombatHandler |
| `spell.rs` | 8 | ~50% SpellHandler |
| `quest.rs` | 7 | ~50% QuestHandler |
| `group.rs` | 6 | ~40% GroupHandler |
| `loot.rs` | 5 | ~50% LootHandler |
| `social.rs` | 4 | ~65% SocialHandler |
| `inspect.rs` | 2 | ~40% InspectHandler |
| `trainer.rs` | 3 | ~30% NPCHandler (trainer subset) |
| `battlenet.rs` | 2 | ~10% BattlenetHandler |
| `misc.rs` | 15 | ~40% MiscHandler |
| **TOTAL** | **~101** | **~39%** |

**What's implemented well:**
- Character login pipeline (enum → create/delete → login)
- Chat messaging (5 main types)
- Basic movement
- Quest basic flow (accept/complete/cancel/progress)
- Social (friends/ignore)
- Combat auto-attack

**What's missing vs C++:**
- **CharacterHandler extras**: rename, customize, race/faction change, equipment sets, faction at war, tutorial flags, alter appearance
- **GuildHandler**: COMPLETO missing (0/20 opcodes)
- **AuctionHouseHandler**: COMPLETO missing (0/10)
- **MailHandler**: COMPLETO missing (0/10)
- **BankHandler**: COMPLETO missing (0/8)
- **ItemHandler extras**: swap, split, transmog, void storage (faltan ~15 opcodes)
- **LFGHandler**: COMPLETO missing (0/15)
- **CalendarHandler**: COMPLETO missing (0/10)
- **BattleGroundHandler**: COMPLETO missing (0/15)
- **PetHandler**: COMPLETO missing (0/12)
- **TradeHandler**: COMPLETO missing (0/12)
- **VoidStorageHandler**: COMPLETO missing
- **TaxiHandler**: COMPLETO missing
- **CombatHandler extras**: duel handlers
- Specialized: BattlePets, Achievements, Scenarios, Instances, Petitions

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- **Handler signature:** C++ `WorldSession::Handle(Packet&)` vs Rust `fn handle(&mut session, &mut packet)` — verificar paso correcto de session_id/player_guid.
- **Async dispatch:** ¿Handlers Rust son async? ¿Cómo se manejan operaciones DB que en C++ son sincrónicas blocking?
- **Error handling:** C++ frecuentemente silencia (log + return). Rust `Result`. ¿Política consistente?

**Tests existing:**
- ~5-10 integration tests en `crates/wow-world/tests/`
- Necesitan fixtures (mock Session, Player, Items)

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#HANDLERS.WBS.001** Cerrar la migracion auditada de `game/Handlers/AdventureJournalHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/AdventureJournalHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.002** Cerrar la migracion auditada de `game/Handlers/AdventureMapHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/AdventureMapHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.003** Partir y cerrar la migracion auditada de `game/Handlers/AuctionHouseHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/AuctionHouseHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1124 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.004** Cerrar la migracion auditada de `game/Handlers/AuthHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/AuthHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.005** Cerrar la migracion auditada de `game/Handlers/BankHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/BankHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.006** Partir y cerrar la migracion auditada de `game/Handlers/BattleGroundHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/BattleGroundHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1346 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.007** Cerrar la migracion auditada de `game/Handlers/BattlePetHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/BattlePetHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.008** Cerrar la migracion auditada de `game/Handlers/BattlenetHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/BattlenetHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.009** Cerrar la migracion auditada de `game/Handlers/BlackMarketHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/BlackMarketHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.010** Partir y cerrar la migracion auditada de `game/Handlers/CalendarHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/CalendarHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 575 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.011** Cerrar la migracion auditada de `game/Handlers/ChannelHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/ChannelHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.012** Partir y cerrar la migracion auditada de `game/Handlers/CharacterHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/CharacterHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2895 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.013** Partir y cerrar la migracion auditada de `game/Handlers/ChatHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/ChatHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 830 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.014** Cerrar la migracion auditada de `game/Handlers/CollectionsHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/CollectionsHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.015** Cerrar la migracion auditada de `game/Handlers/CombatHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/CombatHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.016** Cerrar la migracion auditada de `game/Handlers/DuelHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/DuelHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.017** Cerrar la migracion auditada de `game/Handlers/GarrisonHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/GarrisonHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.018** Partir y cerrar la migracion auditada de `game/Handlers/GroupHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/GroupHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 783 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.019** Partir y cerrar la migracion auditada de `game/Handlers/GuildHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/GuildHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 813 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [x] **#HANDLERS.WBS.020a** Migrar flujo inicial de `game/Handlers/HotfixHandler.cpp`: `HandleDBQueryBulk`, `SendAvailableHotfixes`, `HandleHotfixRequest`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/HotfixHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Server/Packets/HotfixPackets.cpp`
  Rust target: `crates/wow-world/src/handlers/character.rs`, `crates/wow-packet/src/packets/misc.rs`, `crates/wow-data/src/hotfix_cache.rs`
  Acceptance: `cargo fmt --check`; `cargo test -p wow-packet hotfix -- --nocapture`; `cargo test -p wow-data hotfix -- --nocapture`; `cargo test -p wow-world dispatch_table -- --nocapture`; `cargo build -p world-server --release`
  Notes: closed as interim blob-cache parity for live client startup. Remaining full parity is tracked in `datastores.md` as typed `DB2Manager`, per-table hotfix overlay, optional-data allowlist enforcement, and runtime `SMSG_HOTFIX_PUSH`.
- [ ] **#HANDLERS.WBS.020** Cerrar la migracion auditada completa de `game/Handlers/HotfixHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/HotfixHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.021** Cerrar la migracion auditada de `game/Handlers/InspectHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/InspectHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.022** Partir y cerrar la migracion auditada de `game/Handlers/ItemHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/ItemHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1220 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.023** Partir y cerrar la migracion auditada de `game/Handlers/LFGHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/LFGHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 971 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.024** Partir y cerrar la migracion auditada de `game/Handlers/LootHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/LootHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 508 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.025** Partir y cerrar la migracion auditada de `game/Handlers/MailHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/MailHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 676 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.026** Partir y cerrar la migracion auditada de `game/Handlers/MiscHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/MiscHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1440 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.027** Partir y cerrar la migracion auditada de `game/Handlers/MovementHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/MovementHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 816 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.028** Partir y cerrar la migracion auditada de `game/Handlers/NPCHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/NPCHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 577 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.029** Cerrar la migracion auditada de `game/Handlers/NPCHandler.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/NPCHandler.h`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.030** Partir y cerrar la migracion auditada de `game/Handlers/PetHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/PetHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 810 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.031** Cerrar la migracion auditada de `game/Handlers/PetitionsHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/PetitionsHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.032** Cerrar la migracion auditada de `game/Handlers/QueryHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/QueryHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.033** Partir y cerrar la migracion auditada de `game/Handlers/QuestHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/QuestHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 849 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.034** Cerrar la migracion auditada de `game/Handlers/ScenarioHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/ScenarioHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.035** Cerrar la migracion auditada de `game/Handlers/SceneHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/SceneHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.036** Cerrar la migracion auditada de `game/Handlers/SkillHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/SkillHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.037** Cerrar la migracion auditada de `game/Handlers/SocialHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/SocialHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.038** Partir y cerrar la migracion auditada de `game/Handlers/SpellHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/SpellHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 569 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.039** Cerrar la migracion auditada de `game/Handlers/TaxiHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/TaxiHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.040** Cerrar la migracion auditada de `game/Handlers/TicketHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/TicketHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.041** Cerrar la migracion auditada de `game/Handlers/TokenHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/TokenHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.042** Cerrar la migracion auditada de `game/Handlers/ToyHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/ToyHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.043** Partir y cerrar la migracion auditada de `game/Handlers/TradeHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/TradeHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 814 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.044** Cerrar la migracion auditada de `game/Handlers/TransmogrificationHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/TransmogrificationHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.045** Cerrar la migracion auditada de `game/Handlers/VehicleHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/VehicleHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#HANDLERS.WBS.046** Cerrar la migracion auditada de `game/Handlers/VoidStorageHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/VoidStorageHandler.cpp`
  Rust target: `crates/wow-world/src/handlers`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

**Character handlers (10 missing)**
- [ ] **#HANDLERS.1** CharacterRenameRequest (M)
- [ ] **#HANDLERS.2** CharCustomize (M)
- [ ] **#HANDLERS.3** CharRaceOrFactionChange (H)
- [ ] **#HANDLERS.4** EquipmentSet save/delete/use (M)
- [ ] **#HANDLERS.5** SetFactionAtWar/Watched/Inactive (L)
- [ ] **#HANDLERS.6** AlterAppearance (barber shop) (M)

**Guild handlers (20 missing)**
- [ ] **#HANDLERS.7** GuildCreate/Invite/Leave/Delete (H)
- [ ] **#HANDLERS.8** GuildRoster/EventLog/Permissions (H)
- [ ] **#HANDLERS.9** GuildBank query/deposit/withdraw/swap (XL — split)

**Auction House (10 missing)**
- [ ] **#HANDLERS.10** AuctionList items/bidder/owner/pending (H)
- [ ] **#HANDLERS.11** AuctionPlaceBid/RemoveItem/SellItem (H)

**Mail (10 missing)**
- [ ] **#HANDLERS.12** GetMailList/Body/SendMail (H)
- [ ] **#HANDLERS.13** MailTakeMoney/TakeItem/Delete/Return (M)

**Bank (8 missing)**
- [ ] **#HANDLERS.14** BankAccessRequest/BankerActivate/BuyBackItem (L)
- [ ] **#HANDLERS.15** BuyBankSlot (L)

**Item extras (10 missing)**
- [ ] **#HANDLERS.16** SwapItem/SwapInvItem/SplitItem (M)
- [ ] **#HANDLERS.17** WrapItem/UnwrapItem (L)
- [ ] **#HANDLERS.18** VoidStorageTransfer/Query (M)
- [ ] **#HANDLERS.19** Transmogrification (M)

**Group extras (6 missing)**
- [ ] **#HANDLERS.20** DisbandGroup/PartySwap/ChangeSubGroup (L)
- [ ] **#HANDLERS.21** SetLootMethod/SetRole (M)

**Spell/Skill (5 missing)**
- [ ] **#HANDLERS.22** SkillLearn/LevelUp (L)
- [ ] **#HANDLERS.23** CancelAutoRepeatSpell (L)

**Duel (4 missing)**
- [ ] **#HANDLERS.24** DuelAccept/Cancel/Decline/Forfeit (M)

**NPC/Vendor extras (8 missing)**
- [ ] **#HANDLERS.25** GossipSelectMultipleOption, SpiritHealerActivate, etc. (L-M)

**Misc (20+ missing)**
- [ ] **#HANDLERS.26** Calendar opcodes completos (H, ~6h)
- [ ] **#HANDLERS.27** TaxiActivate/QueryNodes (L-M)
- [ ] **#HANDLERS.28** TutorialSetFlag/RepopRequest/CorpseMoved (L)
- [ ] **#HANDLERS.29** Battleground queue/leave/port (H)
- [ ] **#HANDLERS.30** BlackMarket/Petition extras (L-M)

**Cross-cutting**
- [ ] **#HANDLERS.31** Audit completo opcodes C++ vs Rust; tabla exhaustiva (H, ~3h investigación)
- [ ] **#HANDLERS.32** Documentar error policy (cuándo silenciar vs propagar) (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#HANDLERS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 46 files / 21843 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/CharacterHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Handlers/MiscHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Handlers/BattleGroundHandler.cpp`. Rust target: `crates/wow-handler`. | `cargo test -p wow-handler` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#HANDLERS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 46 files / 21843 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/CharacterHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Handlers/MiscHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Handlers/BattleGroundHandler.cpp`. Rust target: `crates/wow-handler`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#HANDLERS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 46 files / 21843 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/CharacterHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Handlers/MiscHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Handlers/BattleGroundHandler.cpp`. Rust target: `crates/wow-handler`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#HANDLERS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 46 files / 21843 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/CharacterHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Handlers/MiscHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Handlers/BattleGroundHandler.cpp`. Rust target: `crates/wow-handler`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Character enum → lista coincide con DB
- [ ] Create character → insertado, enumerable
- [ ] Player login → auras cargadas, equipo correcto, posición persistente
- [ ] Chat broadcast → recibido por nearby players, logged si aplica
- [ ] Party invite → invitee puede aceptar/rechazar, queue correcto
- [ ] Quest accept → marca active, progress=0
- [ ] Item swap → slots cambian, stats jugador actualizados
- [ ] Spell cast → cast time, mana deducido, effects aplicados
- [ ] Duel → ambos en arena, winner/loser correctos
- [ ] Guild create → guild en DB, creator es leader, puede invitar

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 46 files / 21843 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/CharacterHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Handlers/MiscHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Handlers/BattleGroundHandler.cpp` | `crates/wow-world/src/handlers/` + `crates/wow-handler/` \| ⚠️ partial (~22% coverage by handler count, ~23% by registered opcodes) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#HANDLERS.DIV.001` | `crates/wow-world/tests` (`missing_declared_path`, 0 Rust lines) | 46 C++ files / 21843 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Handlers/CharacterHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Handlers/MiscHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Handlers/BattleGroundHandler.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **`WorldSession` stateful:** mantiene `Player*`, cache, pending tx. En Rust `struct Session` con sync. Verificar locks.
- **Async ops:** `HandlePlayerLogin` hace ~10 queries seq. ¿Rust async? Cuidado executor blocking.
- **Opcodes vary by expansion:** WoLK 3.4.3 != Cata != MoP. Mantener `wow_constants::ClientOpcodes` sync con cliente target.
- **Bit-packing en CMSGs:** `CMSG_CAST_SPELL` puede mezclar flags bit-packed + bytes planos. Ver `shared-packets.md`.
- **Performance hotspot:** `HandlePlayerLogin` blocking en C++ (cientos ms). Async en Rust requiere cuidado.
- **Security:** Todos los handlers DEBEN chequear `session.player()` antes de tocar player state. Missing check = exploitable.
- **`ASSERT(m_player)`:** C++ a menudo asume non-null sin check. Rust `Option<&Player>` — explicit.
- **Script callbacks:** `ScriptMgr::OnBeforePlayerLogout` etc. son hooks. Rust necesita registry equivalente.

---

## 12. C++ → Rust mapping

| C++ Pattern | Rust Pattern | Notas |
|---|---|---|
| `void WorldSession::HandleXxx(Packet&)` | `async fn handle_xxx(session: &mut Session, pkt: &mut WorldPacket) -> Result<(), HandlerError>` | Registro vía `inventory::submit!` |
| `m_player` member | `session.player()` / `player_mut()` | Posiblemente `Option` |
| `GetPlayer()` null check | `session.player().ok_or(HandlerError::NoPlayer)?` | Early return `?` |
| `SendPacket(packet.Write())` | `session.send_packet(&packet)` | Encapsula serialización |
| `sObjectAccessor->FindPlayer(guid)` | `world.find_player(guid)` | World lookup |
| `CharacterDatabase.PreparedStatement()` | `db.prepare(stmt).execute(params).await?` | Async DB |
| Exception | `Result<T, Error>` con `?` | Match en lugar de try-catch |
| `if (!m_player) return` | `let player = session.player()?` | Idiom Rust |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

Mechanical comparison between C++ TC wotlk_classic
(`/home/server/woltk-trinity-legacy/src/server/game/Handlers/`) and Rust
(`/home/server/rustycore/crates/wow-world/src/handlers/`). C++ method count is
`grep -cE '^void WorldSession::Handle' *.cpp`; Rust function count is
`grep -cE '^\s*(pub )?(async )?fn handle_'` per file. The earlier server.md
audit (2026-05-01) measured **145/621 = ~23%** of C++ active handlers
registered via `inventory::submit!`; this audit confirms the same gap from the
opposite direction (counting `HandleXxx` symbols vs Rust `handle_*` fns).

### 13.1 Coverage matrix — C++ family → Rust module → method count

| C++ Handler family | C++ `HandleXxx` | Rust module | Rust `handle_*` | Inventory entries | % |
|---|---|---|---|---|---|
| MiscHandler.cpp | 63 | misc.rs | 43 | 43 | ~68% |
| GuildHandler.cpp | 60 | — | 0 | 0 | **0%** |
| CharacterHandler.cpp | 36 | character.rs | 40 (incl. NPC/item shims merged into the file) | 43 | ~50% real char-only |
| BattleGroundHandler.cpp | 34 | — | 0 | 0 | **0%** |
| GroupHandler.cpp | 31 | group.rs | 3 | 3 | ~10% |
| AuctionHouseHandler.cpp | 23 | — | 0 | 0 | **0%** |
| SpellHandler.cpp | 22 | spell.rs | 3 | 3 | ~14% |
| LFGHandler.cpp | 22 | — | 0 | 0 | **0%** |
| ItemHandler.cpp | 22 | (folded into character.rs) | ~6 item shims | — | ~27% |
| MovementHandler.cpp | 19 | movement.rs | 3 | 3 | ~16% |
| QuestHandler.cpp | 17 | quest.rs | 9 | 9 | ~53% |
| ChatHandler.cpp | 17 | chat.rs | 5 | 11 (1 fn covers many opcodes) | ~65% by opcode |
| CalendarHandler.cpp | 16 | — | 0 | 0 | **0%** |
| PetHandler.cpp | 13 | — | 0 | 0 | **0%** |
| NPCHandler.cpp | 13 | trainer.rs (+ misc.rs gossip/vendor) | 2 + ~5 in misc | 2 | ~30% |
| TradeHandler.cpp | 12 | — | 0 | 0 | **0%** |
| QueryHandler.cpp | 12 | (in character.rs / misc.rs) | ~5 | 5 | ~42% |
| BattlePetHandler.cpp | 11 | — | 0 | 0 | **0%** (legion+, OK) |
| VehicleHandler.cpp | 9 | — | 0 | 0 | **0%** |
| TicketHandler.cpp | 9 | — | 0 | 0 | **0%** |
| PetitionsHandler.cpp | 9 | — | 0 | 0 | **0%** |
| MailHandler.cpp | 9 | — | 0 | 0 | **0%** |
| BankHandler.cpp | 9 | — | 0 | 0 | **0%** |
| SocialHandler.cpp | 7 | social.rs | 3 | 3 | ~43% |
| SkillHandler.cpp | 7 | — | 0 | 0 | **0%** |
| LootHandler.cpp | 7 | loot.rs | 3 | 3 | ~43% |
| TaxiHandler.cpp | 5 | — | 0 | 0 | **0%** |
| GarrisonHandler.cpp | 5 | — | 0 | 0 | (WoD+, skip) |
| ChannelHandler.cpp | 5 | — | 0 | 0 | **0%** |
| VoidStorageHandler.cpp | 4 | — | 0 | 0 | **0%** |
| InspectHandler.cpp | 4 | inspect.rs | 1 | 1 | ~25% |
| DuelHandler.cpp | 4 | — | 0 | 0 | **0%** |
| CombatHandler.cpp | 3 | combat.rs | 3 | 3 | ✅ 100% |
| ToyHandler / Scene / BlackMarket / Adventure* / Token / Hotfix / Transmog / Collections / Auth / Bnet / Scenario | 1–3 each | (battlenet.rs has 1) | 0–1 | — | mostly **0%** (legion+/MoP+) |
| **TOTALS** | **~560** (~498 active for WoLK 3.4.3) | 13 modules | **121** `handle_*` | **128** registered | **~22% by fn count, ~23% by opcode (matches server.md)** |

### 13.2 Critical missing handler families (zero Rust coverage, WoLK-relevant)

19 C++ handler families have **no Rust file**:

1. **GuildHandler** (60 fns) — no guild create/invite/roster/bank/rank ops
2. **BattleGroundHandler** (34) — no BG queue/leave/port/score
3. **AuctionHouseHandler** (23) — confirmed by `auctionhouse.md`
4. **LFGHandler** (22) — no dungeon finder
5. **CalendarHandler** (16) — no events/RSVP
6. **PetHandler** (13) — no pet command/control/spell
7. **TradeHandler** (12) — no player trade (begin/items/accept)
8. **VehicleHandler** (9) — no vehicle enter/exit/seat
9. **TicketHandler** (9) — no GM ticket/bug report
10. **PetitionsHandler** (9) — no guild petition flow
11. **MailHandler** (9) — confirmed; mail entirely absent
12. **BankHandler** (9) — no bank/buyback slots
13. **SkillHandler** (7) — no skill learn/level
14. **TaxiHandler** (5) — no flight masters / activate taxi
15. **ChannelHandler** (5) — chat channels (only chat.rs covers core msg types)
16. **VoidStorageHandler** (4) — Cata+, lower priority
17. **DuelHandler** (4) — no duel accept/decline/forfeit
18. **TransmogrificationHandler** (1) — Cata+, lower priority
19. Stubs (Adventure*, BattlePet, Garrison, Scene, Token, Toy, BlackMarket, Collections) — most are post-WoLK and acceptable to defer.

### 13.3 Cross-reference with server.md audit

`server.md` measured **145 of 621 active C++ opcodes** registered in
`inventory::iter::<PacketHandlerEntry>` = **23.3%**. This audit counts
**121 Rust `handle_*` functions** vs **~498 WoLK-relevant `HandleXxx`** =
**~24%**. The two angles agree; the gap is the same gap. The ~24 delta
between 121 fns and 145 registered opcodes is explained by `chat.rs` and
`misc.rs` reusing one Rust function for multiple opcode variants
(`handle_chat_message` covers SAY/PARTY/RAID/INSTANCE_CHAT etc.).

### 13.4 Architectural pattern verification

C++ uses `OpcodeTable::Initialize()` (`Opcodes.cpp`) with a `DEFINE_HANDLER`
macro that records `(opcode, status, processing, &WorldSession::HandleXxx)`
into a static array indexed by opcode. Dispatch is direct virtual call
through a function pointer.

Rust uses `inventory::submit!` to push `PacketHandlerEntry { opcode, status,
processing, handler_name }` into a static collection assembled at startup.
**Two-step dispatch (match arm + submit) means forgetting `submit!` silently
drops the handler** — covered in AGENTS.md.

**SessionStatus parity:** ✅ Rust models all 4 C++ values
(`STATUS_AUTHED`, `STATUS_LOGGEDIN`, `STATUS_TRANSFER`,
`STATUS_LOGGEDIN_OR_RECENTLY_LOGGOUT`) one-for-one. server.md flags that
several `CMSG_QUERY_*` Rust handlers use `Authed` where C++ requires
`LoggedIn` — a behavioural divergence, not a missing axis.

**PacketProcessing parity:** ❌ **Rust drops one mode.** C++ has three:
`PROCESS_INPLACE`, `PROCESS_THREADUNSAFE`, `PROCESS_THREADSAFE`. Rust has
**two**: `Inplace`, `ThreadUnsafe`. C++ uses `PROCESS_THREADSAFE` for ~30
opcodes that are dispatched on `Map::Update()` rather than
`World::UpdateSessions()` — e.g. `CMSG_ACTIVATE_TAXI`,
`CMSG_AREA_TRIGGER` (when running thread-safe), several movement ack
variants. Rust currently funnels these to `ThreadUnsafe`, losing the map
locality. Mark `wow-handler::PacketProcessing` as needing a third variant
before reviving the world-tick scheduler.

### 13.5 Silent bugs in implemented handlers (5-handler spot-check)

Sampled `handle_quest_giver_accept_quest`, `handle_quest_giver_choose_reward`,
`handle_party_invite`, `handle_add_friend`, `handle_attack_swing`. C++ ground
truth from `Server/Packets/{Quest,Party,Social,Combat}Packets.cpp`.

| Handler | C++ Read order | Rust Read order | Verdict |
|---|---|---|---|
| `handle_attack_swing` | `>> Victim` (single GUID) | `AttackSwing::read` (single GUID via typed `ClientPacket`) | ✅ |
| `handle_add_friend` | `ReadBits(9) name; ReadBits(9) notes; ReadString; ReadString` | identical | ✅ |
| `handle_party_invite` | `ReadBit hasPartyIndex; ResetBitPos; ReadBits(9)x2; >>ProposedRoles; >>TargetGUID; ReadString x2; if hasPartyIndex >>PartyIndex` | identical (uses `read_packed_guid` which `ObjectGuid::operator>>` already implies) | ✅ |
| `handle_quest_giver_accept_quest` | `>> QuestGiverGUID; >> QuestID; ReadBit StartCheat` | `read_packed_guid; read_uint32; read_uint8` | **❌ SILENT BUG** — `StartCheat` is a single bit in C++, Rust reads a full byte. Misaligns nothing here because it is the last field, but the `read_uint8` will pull the next packet byte if the wire format actually encodes 1 bit + flush. Real-world client may send 1 padded byte, so it limps; it is still wrong against the protocol spec. |
| `handle_quest_giver_choose_reward` | `>> QuestGiverGUID; >> QuestID; >> Choice` where `Choice = QuestChoiceItem { ReadBits(2) LootItemType; >> ItemInstance (multi-field); >> Quantity (i32) }` | `read_packed_guid; read_uint32 quest_id; read_uint32 choice_item_id; read_uint32 loot_item_type` | **❌ MAJOR SILENT BUG** — Rust treats `Choice` as a flat `(item_id u32, loot_type u32)` pair. The C++ struct is `LootItemType (2 bits) + full ItemInstance (item_id + bonuses + modifications + ...) + Quantity (i32)`. Rust will misparse every quest reward selection where the chosen item has bonuses/mods, and the `loot_item_type` int Rust reads is actually the start of `ItemInstance::ItemID`. |

Two bugs in five handlers (~40% sampling defect rate) suggests the rest of
the implemented surface needs the same line-by-line audit. **Recommend a
follow-up sweep** of every `handle_*` against `Server/Packets/*.cpp` `Read()`
methods before declaring even the 22% covered handlers correct.

### 13.6 Recommended sub-task priority shuffle

Original §9 ordered sub-tasks by C++ family. Re-ordered by impact (player-facing
+ already partially-tested code paths first):

1. **HANDLERS.PRIO-A — fix existing read-order bugs** (NEW). Sweep all 121
   `handle_*` against C++ `::Read()` definitions in `Server/Packets/`. Two
   confirmed bugs in 5 spot-checks; expect ~30–40 silent bugs total.
   Owner: anyone touching an existing handler.
2. **HANDLERS.PRIO-B — Mail (#HANDLERS.12-13)** (was M priority). Mail is
   the blocking dependency for AH, calendar invitations, and refunded
   purchases. 9 C++ fns, all backed by existing `mail` DB schema → fast win.
3. **HANDLERS.PRIO-C — GroupHandler full coverage (#HANDLERS.20-21)**. We
   have 3/31 fns and many systems (loot rolls, BG queue, raid markers)
   silently depend on group state.
4. **HANDLERS.PRIO-D — DuelHandler (#HANDLERS.24)** — 4 fns, low complexity,
   unblocks PvP testing without needing BG infra.
5. **HANDLERS.PRIO-E — TaxiHandler + BankHandler** (5 + 9 fns) — both small,
   both unlock common gameplay loops.
6. **HANDLERS.PRIO-F — `PacketProcessing::ThreadSafe` variant** in
   `wow-handler::lib.rs` before any Map-tick scheduler refactor.
7. Defer Auction/LFG/Calendar/BG/Pet/Trade until after PRIO-A–F land — they
   are large (≥12 fns each) and currently have no DB-side scaffolding.
8. Stubs for legion+/MoP+ handlers (Adventure*, BattlePet, BlackMarket,
   Garrison, Scene, Token, Transmog, VoidStorage) stay at the bottom; not
   relevant for WoLK 3.4.3 client.
