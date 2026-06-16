# Migration: Entities / Totem

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/`
> **Rust target crate(s):** `crates/wow-world/`, `crates/wow-spell/`, `crates/wow-constants/`
> **Layer:** L4 (sub-modules)
> **Status:** ❌ not started
> **Audited vs C++:** ⚠️ partial (header-level audit only)
> **Last updated:** 2026-05-01

---

## 1. Purpose

`Totem` is a `Minion` subclass (which is itself a `TempSummon`/`Creature` chain) representing the shaman totem entities: the four element schools (Fire / Earth / Water / Air totems) plus the rare `TOTEM_STATUE` variant. Totems have a fixed duration, are pinned to one of the four `SUMMON_SLOT_TOTEM` slots on the owner Unit (so a new totem in the same slot replaces the old one), and disable all the standard creature stat-update machinery (their stats are spell-driven, not formula-driven). On unsummon they send `SMSG_TOTEM_DESTROYED` to the owner so the client clears the totem bar UI.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/Totem/Totem.cpp` | 168 | `prefix` |
| `game/Entities/Totem/Totem.h` | 59 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/Totem/Totem.h` | 59 | `Totem` class (final, inherits `Minion`) |
| `src/server/game/Entities/Totem/Totem.cpp` | 168 | ctor, Update (duration tick), InitStats (slot assignment + display id), InitSummon (cast spells), UnSummon (send destroyed pkt), spell-immunity overrides |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Totem` | class (final, `Minion`) | Shaman/druid totem entity |
| `TotemType` | enum | `TOTEM_PASSIVE=0`, `TOTEM_ACTIVE=1`, `TOTEM_STATUE=2` |
| `SUMMON_SLOT_TOTEM` / `SUMMON_SLOT_TOTEM_2..4` / `MAX_TOTEM_SLOT` | constants (`SharedDefines.h`) | The four-slot bar on `Unit::m_SummonSlot` |
| `SUMMON_SLOT_ANY_TOTEM` | constant | Sentinel meaning "first free totem slot" |

---

## 4. Critical public methods

| Symbol | Purpose | Calls into |
|---|---|---|
| `Totem(SummonPropertiesEntry const*, Unit* owner)` | Construct; mark `UNIT_MASK_TOTEM`, default `TOTEM_PASSIVE`, `m_duration=0` | `Minion` ctor |
| `Update(uint32 diff)` | Decrement duration; UnSummon on owner-dead/self-dead/expired | `Creature::Update` |
| `InitStats(summoner, duration)` | Pick slot via `FindUsableTotemSlot`; send `SMSG_TOTEM_CREATED`; pick race-specific display via `SpellMgr::GetModelForTotem` | `Minion::InitStats` |
| `InitSummon(summoner)` | Cast totem's `m_spells[0..]` self-buffs / on-summon spells | `Creature::CastSpell` |
| `UnSummon(uint32 msTime = 0)` | Schedule despawn; send `SMSG_TOTEM_DESTROYED` to owner | `TempSummon::UnSummon` |
| `GetSpell(slot=0)` | Return spell id stored at `m_spells[slot]` | — |
| `GetTotemDuration()` / `SetTotemDuration(Milliseconds)` | Lifetime control | — |
| `GetTotemType()` | Passive / Active / Statue | — |
| `IsImmunedToSpellEffect(...)` | Override: totems immune to most CC | `SpellInfo::HasAttribute` |
| `UpdateStats/UpdateAllStats/UpdateResistances/UpdateArmor/UpdateMaxHealth/UpdateMaxPower/UpdateAttackPowerAndDamage/UpdateDamagePhysical` | All overridden to no-op | — |

---

## 5. Module dependencies

**Depends on:**
- `Minion` / `TempSummon` / `Creature` / `Unit` (chain)
- `SummonPropertiesEntry` (DBC; defines `Slot`, control type)
- `SpellMgr::GetModelForTotem(spellId, race)` — race-specific totem visuals (Tauren vs Orc vs Draenei vs Troll, etc.)
- `SpellInfo` / `SpellEffectInfo` (immunity logic)
- `WorldPackets::Totem::TotemCreated` / `TotemDestroyed` (`TotemPackets.h`)
- `Group` (for totem-aura propagation to party in WoLK 3.4)

**Depended on by:**
- `Spell::EffectSummonType` (case `SUMMON_TYPE_TOTEM`) — creates the totem
- `WorldSession::HandleTotemDestroyed` — player explicitly destroys a totem (CMSG opcode)
- Shaman talents (Totemic Mastery, Totemic Focus) — modify `m_duration` and slots
- `Unit::m_SummonSlot[SUMMON_SLOT_TOTEM..MAX_TOTEM_SLOT]` (replace-old-on-cast logic)

---

## 6. SQL / DB queries

`Totem.cpp` does not query directly. Tables loaded by `SpellMgr` / `ObjectMgr`:

| Statement / Source | Purpose | DB |
|---|---|---|
| `spell_totem_model` | Per-(spellId, race) override of totem display id | world |
| `summon_properties` (DBC) | Slot assignment for the summoned creature | DBC |

DBC stores:

| Store | What it loads | Read by |
|---|---|---|
| `SummonPropertiesStore` | `SummonProperties.dbc` | `Totem` ctor (slot) |
| `CreatureDisplayInfoStore` | `CreatureDisplayInfo.dbc` | display id resolution |

---

## 7. Wire-protocol packets

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_TOTEM_CREATED` | S → C | `Totem::InitStats` (player owner only) | (`crates/wow-constants/src/opcodes.rs` — opcode `TotemCreated = 0x26c8`) |
| `SMSG_TOTEM_DESTROYED` | S → C | `Totem::UnSummon` | (`TotemDestroyed = 0x34f8`) |
| `SMSG_TOTEM_MOVED` | S → C | totem aura prop / slot move | (`TotemMoved = 0x26ca`) |
| `CMSG_TOTEM_DESTROYED` | C → S | player clicks the X on totem bar | needs handler in `wow-handler` |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-constants/src/creature.rs` | `file` | 1 | 623 | `exists_active` | file exists |
| `crates/wow-constants/src/item.rs` | `file` | 1 | 1239 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-constants/src/opcodes.rs` — `TotemDestroyed`, `TotemCreated`, `TotemMoved` enumerated
- `crates/wow-constants/src/creature.rs:408` — `CreatureFamily::Totem = 11` (or similar — not the totem entity, an unrelated family enum)
- `crates/wow-constants/src/item.rs` — `Totem` item-class / item-subclass entries (item-side)
- **0 lines** of `Totem` entity logic. No `m_SummonSlot` array on the Unit equivalent.

**What's implemented:** opcode constants only.

**What's missing vs C++:** entire 168-line `Totem.cpp` — slot assignment, duration tick, race-specific display, on-summon spell casts, totem-destroyed packet, no-op stat overrides, immunity overrides. Also missing: `m_SummonSlot` on Unit, `SUMMON_SLOT_TOTEM` constants, the four-slot bar replace-on-cast logic.

**Suspicious / likely divergent:** none — nothing exists.

**Tests existing:** 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_TOTEM.WBS.001** Cerrar la migracion auditada de `game/Entities/Totem/Totem.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.cpp`
  Rust target: `crates/wow-world`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_TOTEM.WBS.002** Cerrar la migracion auditada de `game/Entities/Totem/Totem.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.h`
  Rust target: `crates/wow-world`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#TOT.1** Add `SummonSlot` constants to `wow-constants` (`Pet=0`, `Totem1..4`, `Quest`, `MiniPet`, `MAX_TOTEM_SLOT=5`) (L)
- [ ] **#TOT.2** Add `m_SummonSlot: [ObjectGuid; 7]` (or analog) on the Unit equivalent in `wow-world` (L)
- [ ] **#TOT.3** Port `TotemType` enum to `wow-constants` (L)
- [ ] **#TOT.4** Define `Totem` struct in `wow-world/src/entities/totem.rs` (`type`, `duration`, `spell_ids[4]`) — wraps `Creature`/`TempSummon` (M)
- [ ] **#TOT.5** Implement `update_tick`: owner-dead OR self-dead OR expired → unsummon (L)
- [ ] **#TOT.6** Implement `init_stats`: slot assignment via `find_usable_totem_slot`; send `SMSG_TOTEM_CREATED`; race-specific display via `SpellMgr::get_model_for_totem` (M)
- [ ] **#TOT.7** Implement `init_summon`: cast `spell_ids` self-buffs (M)
- [ ] **#TOT.8** Implement `unsummon`: send `SMSG_TOTEM_DESTROYED` (L)
- [ ] **#TOT.9** Wire `CMSG_TOTEM_DESTROYED` handler (player clicks X) (L)
- [ ] **#TOT.10** Override stat updates to no-ops (totems do not recalc stats) (L)
- [ ] **#TOT.11** Implement `is_immune_to_spell_effect` overrides (knockback, fear, root, etc.) (M)
- [ ] **#TOT.12** `spell_totem_model` table loader in `wow-data`: represented query/store exists (`WorldStatements::SEL_SPELL_TOTEM_MODEL`, `SpellTotemModelStoreLikeCpp`); startup wiring and `Totem::InitStats` consumption are still pending (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_TOTEM.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 227 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.h`. Rust target: `crates/wow-constants`, `crates/wow-spell`, `crates/wow-world`. | `cargo test -p wow-constants && cargo test -p wow-spell && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_TOTEM.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 227 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.h`. Rust target: `crates/wow-constants`, `crates/wow-spell`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_TOTEM.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 227 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.h`. Rust target: `crates/wow-constants`, `crates/wow-spell`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_TOTEM.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 227 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.h`. Rust target: `crates/wow-constants`, `crates/wow-spell`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: summoning totem in slot N replaces the existing totem in slot N (old one unsummoned)
- [ ] Test: `SUMMON_SLOT_ANY_TOTEM` picks first empty slot (1..4)
- [ ] Test: totem duration of 60s expires at exactly `tick_count * tick_ms ≥ 60000`
- [ ] Test: owner death triggers immediate totem unsummon
- [ ] Test: `SMSG_TOTEM_CREATED` sent only if owner is a Player
- [ ] Test: race-specific display id picked when `spell_totem_model` row matches (Tauren/Orc/etc.)
- [ ] Test: totem ignores stat-update calls (UpdateMaxHealth no-op asserted)

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 227 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.h` | `crates/wow-world/`, `crates/wow-spell/`, `crates/wow-constants/` \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_TOTEM.DIV.001` | `crates/wow-spell` (`exists_empty`, 0 Rust lines) | 2 C++ files / 227 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Totem/Totem.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **`MAX_TOTEM_SLOT` is the count, not an index** — slot indices are `SUMMON_SLOT_TOTEM (=1)` through `SUMMON_SLOT_TOTEM+3`. Off-by-one is the canonical bug here.
- The four-slot bar on the client is **0-indexed** in `SMSG_TOTEM_CREATED` (Slot field is `slot - SUMMON_SLOT_TOTEM`). Do not send the absolute summon-slot index.
- `m_unitTypeMask |= UNIT_MASK_TOTEM` matters for spell targeting. Don't drop it.
- Race-specific totem displays exist for Tauren / Orc / Draenei / Troll / Goblin (not present for the rare alliance-shaman in 3.0+; double-check `spell_totem_model` rows).
- WoLK 3.4 has the **Totemic Recall** spell (mass-recall all totems with mana refund). Implement as a server-side effect that calls `unsummon` with `msTime=0` on each totem in `m_SummonSlot[1..4]`.
- Druid mushrooms (Wild Mushroom, Cataclysm-era) are NOT totems despite slot reuse in later patches; they don't exist in 3.4.
- `IsImmunedToSpellEffect` totem override is conservative — totems are immune to most CC but **not** to direct damage spells. Mirror the C++ branches exactly.
- `m_spells[0..3]` on Creature are the totem's on-summon spell IDs. Some totems (Searing Totem) have only one; others (Fire Elemental Totem) summon a separate creature mid-tick.

---

## 12. C++ → Rust mapping

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Totem : Minion` | `struct Totem` (composition over Creature) | no inheritance |
| `enum TotemType` | `enum TotemType { Passive, Active, Statue }` | direct |
| `Milliseconds m_duration` | `Duration` | std::time |
| `m_spells[0..3]` on Creature | `[u32; 4]` field on `Creature` | direct |
| `Unit::m_SummonSlot[7]` | `[ObjectGuid; 7]` field on `Unit` | direct |
| `SUMMON_SLOT_TOTEM` | `const SUMMON_SLOT_TOTEM: usize = 1;` | direct |
| `WorldPackets::Totem::TotemCreated` | `pub struct TotemCreatedPacket { totem, slot, duration_ms, spell_id }` | wow-packet |

---

## 13. §13 Audit (vs `/home/server/woltk-trinity-legacy/`)

| C++ symbol | Found in Rust | File | Verdict |
|---|---|---|---|
| `class Totem` | no | — | ❌ missing |
| `enum TotemType` | no | — | ❌ missing |
| `Totem::Update` (duration tick) | no | — | ❌ missing |
| `Totem::InitStats` (slot, display) | no | — | ❌ missing |
| `Totem::UnSummon` (TOTEM_DESTROYED) | no | — | ❌ missing |
| `Unit::m_SummonSlot[7]` | no | — | ❌ missing |
| `SUMMON_SLOT_TOTEM` constants | no | — | ❌ missing |
| `SpellMgr::GetModelForTotem` | partial | `crates/wow-data/src/spell.rs` (`SpellTotemModelStoreLikeCpp::get_model_for_totem_like_cpp`) | ⚠️ represented store/accessor exists, not loaded or consumed by totem runtime |
| `SMSG_TOTEM_CREATED` sender | no | — | ❌ missing |
| `SMSG_TOTEM_DESTROYED` sender | no | — | ❌ missing |
| `CMSG_TOTEM_DESTROYED` handler | no | — | ❌ missing |
| `TotemCreated`/`TotemDestroyed`/`TotemMoved` opcode constants | yes | `crates/wow-constants/src/opcodes.rs` | ⚠️ enumerated, no senders/handlers |

**Verdict:** ❌ not started. Surface coverage ≈ 0% (opcode constants only). No entity, no slot bar, no spell-driven summon path.
