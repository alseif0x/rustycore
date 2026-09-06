# Migration: Entities / Conversation

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/`
> **Rust target crate(s):** `crates/wow-world/`, `crates/wow-data/`, `crates/wow-constants/`
> **Layer:** L4 (sub-modules)
> **Status:** ❌ not started — **n/a for WoLK 3.4** (post-WoLK feature; carry stub only if needed)
> **Audited vs C++:** ⚠️ partial (header-level audit only)
> **Last updated:** 2026-05-01

---

## 1. Purpose

`Conversation` is the in-world cinematic-conversation entity introduced in **Battle for Azeroth (8.x)** and folded back into the legacy TC tree. It owns a list of "actors" (NPCs by GUID or by creature/display id), per-line start/end timing per locale, and a `_textureKitId` for client visuals. The client renders it as a popup-ish in-world dialogue between scripted speakers (think the cinematic conversations in BfA quests). **It does not exist in the WotLK 3.4.3.54261 retail client.** Including in TC's WoLK fork is a backport artifact — the high-guid type and TYPEID are reserved but no live data uses them.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/Conversation/Conversation.cpp` | 399 | `prefix` |
| `game/Entities/Conversation/Conversation.h` | 102 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/Conversation/Conversation.h` | 102 | `Conversation` class (final, WorldObject + GridObject) |
| `src/server/game/Entities/Conversation/Conversation.cpp` | 399 | Create, Update, Start, AddActor, line timing |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Conversation` | class (final) | Cinematic conversation entity |
| `ConversationActorType` | enum class (forward decl) | `WorldObject` vs `CreatureActor` typing |
| `UF::ConversationData` | UF struct | Wire data (lines, actors, lastLineEndTime, progress) |

---

## 4. Critical public methods

| Symbol | Purpose | Calls into |
|---|---|---|
| `CreateConversation(entry, creator, pos, privateOwner, spellInfo, autoStart)` | Static factory | `Create`, `Start` |
| `Start()` | Begin playback (server timer-driven) | broadcast UpdateObject |
| `Update(uint32 diff)` | Tick conversation playback; remove when last line ends | `Remove` |
| `AddActor(actorId, actorIdx, actorGuid)` | Bind world-object actor to slot | UpdateField mutation |
| `AddActor(actorId, actorIdx, type, creatureId, displayId)` | Bind static creature/display actor | — |
| `GetLineStartTime(locale, lineId)` / `GetLineEndTime` / `GetLastLineEndTime` | Per-locale timing queries | `_lineStartTimes`, `_lastLineEndTimes` |
| `GetActorUnit(actorIdx)` / `GetActorCreature(actorIdx)` | Resolve slot to entity | MapManager lookup |
| `GetTextureKitId()` / `GetDuration()` / `GetScriptId()` | Read-only accessors | — |
| `Remove()` | Despawn | `RemoveFromWorld` |

---

## 5. Module dependencies

**Depends on:**
- `WorldObject` / `GridObject` (entity base)
- `ConversationDataStore` (templates loaded from `conversation_template`, `conversation_actors`, `conversation_line` DB tables)
- `ConversationLine.dbc` / `ConversationLineStore` (per-line locale text + duration; **does not exist in 3.4 client DBCs**)
- `Map` (AddToMap)
- Scripts (per-id cinematic logic)

**Depended on by:**
- Quest scripts (post-WoLK)
- Spell effects with `EFFECT_CREATE_CONVERSATION` (post-WoLK)
- Private object visibility (`privateObjectOwner` GUID — only that player sees it)

---

## 6. SQL / DB queries

| Statement / Source | Purpose | DB |
|---|---|---|
| `conversation_template` | Per-id template (textureKit, duration) | world (post-WoLK schema) |
| `conversation_actors` | Actor slot definitions | world |
| `conversation_line_template` | Per-line scripted properties | world |

DBC stores (post-WoLK):

| Store | What it loads | Read by |
|---|---|---|
| `ConversationLineStore` | `ConversationLine.db2` | line duration / text |

**Note:** None of these tables/DB2 files exist in a stock WoLK 3.4.3 install. Including them requires a backport schema.

---

## 7. Wire-protocol packets

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_UPDATE_OBJECT` (with Conversation block) | S → C | spawn (post-WoLK clients) |
| `SMSG_CONVERSATION_LINE_STARTED` | S → C | per-line trigger | (`crates/wow-constants/src/opcodes.rs`: `ConversationLineStarted = 0x3546`) |

**Caveat:** the 3.4.3.54261 retail client does **not** know how to render `SMSG_CONVERSATION_LINE_STARTED`; sending it is a no-op or a desync risk. The opcode constant in `wow-constants` is enumerated for completeness only.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/update.rs` | `file` | 1 | 3072 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-constants/src/object.rs` — `TypeId::Conversation = 13`, `HighGuid::Conversation = 7`
- `crates/wow-core/src/guid.rs` — `HighGuid::Conversation`, `is_conversation`, type mapping
- `crates/wow-constants/src/opcodes.rs` — `ConversationLineStarted = 0x3546`
- `crates/wow-packet/src/packets/update.rs` — Conversation block bit hardcoded to `false` in object update mask
- **0 lines** of Conversation entity logic.

**What's implemented:** type-id, GUID type, opcode constant. Update bit reserved to `false`.

**What's missing vs C++:** entire 399-line `Conversation.cpp`. Not a priority for 3.4 — see status note.

**Suspicious / likely divergent:** none — feature does not exist on the target client.

**Tests existing:** 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_CONVERSATION.WBS.001** Cerrar la migracion auditada de `game/Entities/Conversation/Conversation.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.cpp`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_CONVERSATION.WBS.002** Cerrar la migracion auditada de `game/Entities/Conversation/Conversation.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.h`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#CONV.1** **Decision gate:** confirm whether any 3.4 backport content (custom server scripts) needs Conversation. If no — close as `n/a` and stop. If yes — proceed. (L)
- [ ] **#CONV.2** Port `ConversationActorType` enum to `wow-constants` (L)
- [ ] **#CONV.3** Define `Conversation` entity struct in `wow-world/src/entities/conversation.rs` (M)
- [ ] **#CONV.4** Implement `create` / `start` / `update_tick` / `remove` (M)
- [ ] **#CONV.5** Implement `add_actor` (both overloads) (L)
- [ ] **#CONV.6** Per-locale line-timing storage (`HashMap<(Locale, i32), Duration>`) (L)
- [ ] **#CONV.7** Conversation DB schema + loader in `wow-data` (M, post-WoLK schema design)
- [ ] **#CONV.8** `ConversationData` UF block + flip create/update bit (M)
- [ ] **#CONV.9** Wire `SMSG_CONVERSATION_LINE_STARTED` sender (only if client supports — verify in sniffs) (L)

**Recommendation:** mark this module **`n/a`** in the master roadmap until a concrete user need is identified for the 3.4 fork.

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_CONVERSATION.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 501 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-world`. | `cargo test -p wow-constants && cargo test -p wow-data && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_CONVERSATION.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 501 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_CONVERSATION.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 501 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_CONVERSATION.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 501 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

(Only relevant if the decision gate in #CONV.1 says yes.)

- [ ] Test: `add_actor` populates the correct slot index in the UF actor array
- [ ] Test: `update_tick` past `last_line_end_time` calls `remove`
- [ ] Test: per-locale line timing returns correct end-time for a given lineId
- [ ] Test: private-object owner GUID restricts visibility (only that player sees the entity in update broadcasts)

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `product_out_of_scope_or_post_wolk` | Keep C++ coverage assigned; implementation may be stubbed/disabled only with explicit client behavior and tests/n/a recorded. | 2 files / 501 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.h` | `crates/wow-world/`, `crates/wow-data/`, `crates/wow-constants/` \| ❌ not started — **n/a for WoLK 3.4** (post-WoLK feature; carry stub only if needed) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_CONVERSATION.DIV.001` | _none generated_ | 2 C++ files / 501 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Conversation/Conversation.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **WoLK 3.4.3 retail client cannot render conversations.** The structural code (TYPEID, HighGuid, opcode constant) is in TC's WoLK fork as dead infrastructure. Spawning a Conversation entity on a real 3.4 client will at best be ignored, at worst desync the world state.
- This module is the strongest "n/a" candidate of the seven entity sub-types in this batch. Recommend documenting and parking, not implementing.
- Private-object owner pattern is shared with SceneObject (single-player visibility). If both end up needed, factor that filter into a shared `PrivateObjectFilter` utility.
- `_lineStartTimes` is keyed by `(LocaleConstant, lineId)` — Rust would use `HashMap<(Locale, i32), Duration>`; locale is `enum Locale` (already exists somewhere in `wow-core` for chat).

---

## 12. C++ → Rust mapping

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Conversation : WorldObject` | `struct Conversation` (composition) | if implemented |
| `Milliseconds _duration` | `Duration` | `std::time::Duration` |
| `unordered_map<pair<Locale, i32>, Milliseconds>` | `HashMap<(Locale, i32), Duration>` | direct |
| `array<Milliseconds, TOTAL_LOCALES>` | `[Duration; TOTAL_LOCALES]` | direct |
| `enum class ConversationActorType` | `enum ConversationActorType { WorldObject, Creature }` | direct |
| `Position _stationaryPosition` | `Position` field | from `wow-core` |

---

## 13. §13 Audit (vs `/home/server/woltk-trinity-legacy/`)

| C++ symbol | Found in Rust | File | Verdict |
|---|---|---|---|
| `class Conversation` | no | — | ❌ missing (and likely n/a) |
| `enum class ConversationActorType` | no | — | ❌ missing |
| `Conversation::CreateConversation` | no | — | ❌ missing |
| `Conversation::AddActor` | no | — | ❌ missing |
| `Conversation::Update` / `Start` / `Remove` | no | — | ❌ missing |
| `_lineStartTimes` per-locale timing | no | — | ❌ missing |
| `UF::ConversationData` UF block | no (bit hardcoded false) | `crates/wow-packet/src/packets/update.rs` | ❌ missing |
| `TypeId::Conversation = 13` | yes | `crates/wow-constants/src/object.rs` | ✅ present (constant only) |
| `HighGuid::Conversation = 7` | yes | `crates/wow-core/src/guid.rs` | ✅ present (constant only) |
| `SMSG_CONVERSATION_LINE_STARTED` opcode | yes (constant) | `crates/wow-constants/src/opcodes.rs` | ⚠️ enumerated, no sender (and client likely won't render) |

**Verdict:** ❌ not started — **and recommended to stay that way for the 3.4 fork.** Surface coverage ≈ 0% (constants reserved). Mark `n/a` in master roadmap pending explicit user requirement.
