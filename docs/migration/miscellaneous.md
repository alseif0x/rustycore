# Migration: Miscellaneous

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Miscellaneous/`
> **Rust target crate(s):** `wow-constants`, `wow-core`, `wow-combat`, `wow-data`, `wow-script` (scattered)
> **Layer:** L0 (foundation — pulled in by every layer above)
> **Status:** ⚠️ partial — only `SharedDefines` constants and a few `Formulas::XP::*` curves are partially ported
> **Audited vs C++:** ✅ complete (audit 2026-05-01)
> **Last updated:** 2026-05-01

---

## 1. Purpose

`Miscellaneous/` is TrinityCore's grab-bag of headers that don't cleanly belong to any single subsystem. It hosts (a) the master `SharedDefines.h` constant catalogue (8000+ lines of game enums: races, classes, powers, factions, spell families, item flags, expansion ids, etc.), (b) closed-form gameplay formulas (`Formulas.h`: XP curves, honor scaling, gray-level rules, rest XP, gain reductions), (c) the localised `Language.h` text-id table, (d) the `RaceMask` template and races enum, (e) lightweight predicate functors (`CommonPredicates.h` — sorting/filtering for unit lists), and (f) `enuminfo_*.cpp` reflection scaffolding (auto-generated from `// TITLE/DESCRIPTION/SKIP` markers in the enums; used by GM commands and admin tooling).

Because every other module includes one of these headers, this directory is effectively a foundation layer that every higher layer transitively depends on.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Miscellaneous/CommonPredicates.cpp` | 53 | `prefix` |
| `game/Miscellaneous/CommonPredicates.h` | 98 | `prefix` |
| `game/Miscellaneous/Formulas.h` | 290 | `prefix` |
| `game/Miscellaneous/Language.h` | 1245 | `prefix` |
| `game/Miscellaneous/RaceMask.h` | 200 | `prefix` |
| `game/Miscellaneous/SharedDefines.h` | 8184 | `prefix` |
| `game/Miscellaneous/enuminfo_RaceMask.cpp` | 139 | `prefix` |
| `game/Miscellaneous/enuminfo_SharedDefines.cpp` | 5199 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Miscellaneous/SharedDefines.h` | **8184** | The master enum catalogue: classes, races, powers, weapon types, item subclasses, faction templates, ~hundreds of `MAX_*`, expansion ids, spell families, all the magic constants the rest of the server reads |
| `src/server/game/Miscellaneous/Formulas.h` | 290 | Inline namespaces `Trinity::Honor`, `Trinity::XP`, `Trinity::Currency`, `Trinity::Honor::*` containing closed-form rules: gray level, XP needed for level, XP gain modifiers, rest-bonus XP, honor at level, kill-honor, etc. Hot path on every kill / quest turn-in |
| `src/server/game/Miscellaneous/Language.h` | 1245 | `enum Language` — every `LANG_*` server-string identifier (BG announcements, GM command output, system messages). Resolved through `ObjectMgr::GetTrinityString` |
| `src/server/game/Miscellaneous/RaceMask.h` | 200 | `enum Races` (with TITLE/DESCRIPTION markers for codegen) + `template<typename T> struct RaceMask` packing race ids into a bitfield |
| `src/server/game/Miscellaneous/CommonPredicates.h` | 98 | Functors used by `std::sort` / `std::find_if` over unit lists: `IsVictimOf`, `HealthPctOrderPred`, `PowerPctOrderPred`, `Inverter` |
| `src/server/game/Miscellaneous/CommonPredicates.cpp` | 53 | Implementations of the above |
| `src/server/game/Miscellaneous/enuminfo_RaceMask.cpp` | 139 | Auto-generated reflection: `Races` enum → name/title/description tables (used by GM commands `.lookup`/`.npc info`) |
| `src/server/game/Miscellaneous/enuminfo_SharedDefines.cpp` | **5199** | Auto-generated reflection for every enum in `SharedDefines.h`. Build-time code-gen output |

---

## 3. Classes / Structs / Enums

### From `SharedDefines.h` (selected — out of dozens)

| Symbol | Kind | Purpose |
|---|---|---|
| `Classes` | enum | `CLASS_WARRIOR=1 .. CLASS_MONK=10 .. CLASS_DEMONHUNTER=12` |
| `Powers` | enum int8 | `POWER_MANA=0`, `POWER_RAGE`, `POWER_FOCUS`, `POWER_ENERGY`, `POWER_HAPPINESS`, `POWER_RUNIC_POWER` (3.4.3) |
| `WeaponAttackType` | enum | `BASE_ATTACK / OFF_ATTACK / RANGED_ATTACK` |
| `SpellSchool`, `SpellSchoolMask` | enum | Damage school taxonomy (physical/holy/fire/nature/frost/shadow/arcane) |
| `Stats` | enum | `STAT_STRENGTH .. STAT_SPIRIT` |
| `ItemQualities` | enum | Poor/Common/Uncommon/Rare/Epic/Legendary/Artifact |
| `ReputationRank` | enum | Hated → Exalted (8 ranks) |
| `ContentLevels`, `Expansions` | enum | `EXPANSION_CLASSIC=0 .. EXPANSION_WRATH_OF_THE_LICH_KING=2 .. CURRENT_EXPANSION` |
| `LootMethod`, `LootType` | enum | Group loot rules |
| `MAX_*` | const | Hundreds of compile-time bounds: `MAX_CLASSES`, `MAX_RACES`, `MAX_POWERS`, `MAX_SPELL_EFFECTS`, `MAX_QUEST_LOG_SIZE`, etc. |

### From `Formulas.h`

| Symbol | Kind | Purpose |
|---|---|---|
| `Trinity::GetExpansionForLevel(level)` | inline fn | Maps `1..120 → EXPANSION_*` |
| `Trinity::Honor::hk_honor_at_level_f / hk_honor_at_level` | inline fn | Honor reward for player kill |
| `Trinity::XP::GetGrayLevel(plLevel)` | inline fn | Threshold below which mob gives no XP |
| `Trinity::XP::Gain(player, victim, isBattleground)` | inline fn | Final XP awarded for a kill |
| `Trinity::XP::xp_to_level(level)` | inline fn | XP curve to next level |
| `Trinity::XP::ConQuestXPRate(level)` / various rate hooks | inline fn | Group bonuses, rest bonus calculation |
| `Trinity::Currency::ConquestRatingCalculator(rating)` | inline fn | PVP currency formula |

### From `RaceMask.h`

| Symbol | Kind | Purpose |
|---|---|---|
| `Races` | enum | `RACE_HUMAN=1 .. RACE_PANDAREN_HORDE=26 .. RACE_KUL_TIRAN=32` etc. (Wrath subset is 1–11 + 22) |
| `RaceMask<T>` | struct template | Packs race ids into `T` (uint32 / uint64) bitfield. Has `HasRace(race)` |

### From `CommonPredicates.h`

| Symbol | Kind | Purpose |
|---|---|---|
| `Trinity::Predicates::IsVictimOf` | functor class | True iff `obj` is the attacker's current victim |
| `Trinity::Predicates::HealthPctOrderPred(asc)` | functor class | Sort by health % |
| `Trinity::Predicates::PowerPctOrderPred(power, asc)` | functor class | Sort by power % |
| `Trinity::Predicates::Inverter<PRED>` | template | Logical NOT wrapper |

### From `Language.h`

| Symbol | Kind | Purpose |
|---|---|---|
| `Language` | enum | ~1200 entries: `LANG_*` ids, each maps via `trinity_string` SQL table to localised text |

---

## 4. Critical public methods / functions

This module is mostly **header-only**: inline `constexpr` / `inline` free functions in namespaces. There are no real classes with state.

| Symbol | Purpose | Calls into |
|---|---|---|
| `Trinity::GetExpansionForLevel(uint32 level) -> uint32` | Determines expansion bracket for a level — used by zone gating, item availability | — (pure) |
| `Trinity::Honor::hk_honor_at_level(uint8 level, float multiplier=1.0f) -> uint32` | Computes honor for a player kill at level | `sScriptMgr->OnHonorCalculation` |
| `Trinity::XP::GetGrayLevel(uint8 plLevel) -> uint8` | Mob level below which XP is 0 (anti-twink) | — (pure) |
| `Trinity::XP::Gain(Player*, Unit*, bool isBattleground) -> uint32` | Total XP for a kill | `sScriptMgr->OnXPGain`, `BaseGain`, rest-bonus check |
| `Trinity::XP::xp_to_level(uint8 level) -> uint32` | XP needed to reach `level+1` | DBC `XpEntry` |
| `Trinity::Predicates::IsVictimOf(Unit const* attacker)` | Used as `std::find_if` predicate to filter target lists | — |
| `Trinity::Predicates::HealthPctOrderPred(asc)` | `std::sort` comparator over `Unit*` | `Unit::GetHealthPct` |
| `RaceMask<T>::HasRace(Races r)` | Bit-test for race membership | — |
| `EnumUtils::GetEnumName<E>(E val) -> char const*` | Reflection lookup (driven by `enuminfo_*.cpp`) | code-gen tables |

---

## 5. Module dependencies

**Depends on:**
- `Define.h` (TC's primitive typedefs `uint32`, `uint8`, etc.)
- `Player.h`, `Unit.h`, `Map.h`, `Creature.h`, `World.h` (`Formulas.h` includes them all — dragging the entire game header graph in via this single file is a known compile-time pain point)
- `ScriptMgr.h` (script hooks `OnXPGain`, `OnHonorCalculation`)
- DBC stores (XP curves, race/class data) — `Formulas` is "pure" but reads tables loaded elsewhere
- `EnumUtils` codegen (`SharedDefines.h` magic comments `// TITLE`, `// DESCRIPTION`, `// SKIP` are parsed by the build to produce `enuminfo_*.cpp`)

**Depended on by:**
- **Everything.** `SharedDefines.h` is the single most-included header in the server (it defines `Classes`, `Races`, `Powers`, `MAX_*` — touched by every gameplay file). `Formulas.h` is pulled in by `Player.cpp`, `Unit.cpp`, `Group.cpp`, `Battleground.cpp`, `KillRewarder` etc. `Language.h` is referenced anywhere a localised system message is sent. `CommonPredicates.h` is used in spell targeting, AI threat lists, and party utility code.

---

## 6. SQL / DB queries (if any)

Almost none — this directory holds compile-time constants and pure functions. The only DB-touching name is `Language` enum entries, which are resolved at runtime by `ObjectMgr::GetTrinityString(LANG_*, locale)` against the `trinity_string` table (loaded by `ObjectMgr::LoadTrinityStrings` — see `globals.md`).

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT entry, content_default, content_loc1..N FROM trinity_string` | Resolve `LANG_*` to localised text (loader lives in ObjectMgr, not here) | world |

No DBC/DB2 stores are owned by this module either; XP curves come from `XpEntry` (DBC), faction data from `FactionEntry`, etc. — read but not loaded here.

---

## 7. Wire-protocol packets (if any)

None directly. The constants in `SharedDefines.h` are referenced by every packet builder — e.g. `MAX_INVENTORY_ITEMS` is used in `SMSG_*_LIST_RESPONSE` sizing — but no packet originates in `Miscellaneous/`.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `crates/wow-combat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-constants/src/lib.rs` | `file` | 1 | 30 | `exists_active` | file exists |
| `crates/wow-constants/src/shared.rs` | `file` | 1 | 464 | `exists_active` | file exists |
| `crates/wow-data/src/quest_xp.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `crates/wow-data/src/player_stats.rs` | `file` | 1 | 307 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

This module is **mostly absent** as a coherent unit; bits and pieces are scattered across crates.

**Files in `/home/server/rustycore`:**
- `crates/wow-constants/src/lib.rs` + `unit.rs` + `creature.rs` + `item.rs` + `movement.rs` + `object.rs` + `opcodes.rs` + `shared.rs` + `spell.rs` + `update.rs` — partial port of `SharedDefines.h` enums (race/class/gender/power live in `unit.rs`)
- `crates/wow-constants/src/shared.rs` — items the C++ side calls "shared" constants (max sizes, `MAX_*` analogues)
- `crates/wow-data/src/quest_xp.rs` — partial port of `Trinity::XP::xp_to_level` curve
- `crates/wow-data/src/player_stats.rs` — partial port of formulas around base stats / level info
- No file maps to `Language.h`, no file maps to `CommonPredicates.h`, no file maps to `Formulas.h::Honor`, no file maps to `RaceMask.h`'s template.

**What's implemented:**
- Race / class / gender / power enums (in `wow-constants/unit.rs`)
- A partial XP-to-level curve (`wow-data/quest_xp.rs`)
- Some `MAX_*` constants and opcode enums (`wow-constants/opcodes.rs`, `shared.rs`)

**What's missing vs C++:**
- The vast majority of `SharedDefines.h` enums (probably 70%+ — full audit needed but on inspection many spell families, item flags, faction templates, vehicle flags, summon types, achievement criteria types, etc. are absent)
- All of `Formulas.h::Honor` (PVP honor scaling)
- `Trinity::XP::GetGrayLevel` (anti-twink rule) — anti-grind logic relies on it
- `Trinity::XP::Gain` full pipeline (kills currently award flat or no XP in the Rust handlers)
- `Trinity::XP::ConQuestXPRate` group bonus
- Rest XP calculation
- `Trinity::Currency::ConquestRatingCalculator`
- `Language` enum + i18n string lookup ⇒ all GM messages and BG announcements are currently English-only hardcoded
- `RaceMask<T>` template — used by `MailLevelReward`, talent prereqs, spell targeting; without it those structs cannot be ported faithfully
- `CommonPredicates` — spell targeting / AI sort code currently uses ad-hoc closures; behaviour is similar but lacks the named, tested predicates
- `EnumUtils` reflection — Rust has no equivalent of TC's enuminfo codegen; GM `.lookup` commands cannot list enum values

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- The XP curve in `wow-data/quest_xp.rs` may use kit XP tables rather than `Formulas::XP::xp_to_level` directly. Differences of 1–2 XP per level are easy to miss but break parity with retail clients that compute the same.
- Honor formulas: if PVP honor is awarded at all today, it almost certainly uses a placeholder constant rather than `level * 1.55 * multiplier`.
- Power enum ordering: TC has `POWER_HAPPINESS=4` and `POWER_RUNIC_POWER=6` (Wrath), but later expansions removed `POWER_HAPPINESS`. If the Rust port followed retail, `POWER_HAPPINESS` may be missing — breaks pet UI in 3.4.3.
- `Expansions::EXPANSION_WRATH_OF_THE_LICH_KING` constant: Rust may have `Expansion::Wrath = 2` vs TC's `EXPANSION_WRATH_OF_THE_LICH_KING = 2` — naming convention differs but the discriminant must match.

**Tests existing:**
- `wow-data/quest_xp.rs` has small unit tests for quest XP at known levels
- No tests for honor formulas, gray level, rest XP, race-mask membership

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#MISCELLANEOUS.WBS.001** Cerrar la migracion auditada de `game/Miscellaneous/CommonPredicates.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/CommonPredicates.cpp`
  Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-combat`, `crates/wow-data`, `crates/wow-script`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MISCELLANEOUS.WBS.002** Cerrar la migracion auditada de `game/Miscellaneous/CommonPredicates.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/CommonPredicates.h`
  Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-combat`, `crates/wow-data`, `crates/wow-script`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MISCELLANEOUS.WBS.003** Cerrar la migracion auditada de `game/Miscellaneous/Formulas.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/Formulas.h`
  Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-combat`, `crates/wow-data`, `crates/wow-script`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MISCELLANEOUS.WBS.004** Partir y cerrar la migracion auditada de `game/Miscellaneous/Language.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/Language.h`
  Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-combat`, `crates/wow-data`, `crates/wow-script`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1245 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#MISCELLANEOUS.WBS.005** Cerrar la migracion auditada de `game/Miscellaneous/RaceMask.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/RaceMask.h`
  Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-combat`, `crates/wow-data`, `crates/wow-script`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MISCELLANEOUS.WBS.006** Partir y cerrar la migracion auditada de `game/Miscellaneous/SharedDefines.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h`
  Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-combat`, `crates/wow-data`, `crates/wow-script`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 8184 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#MISCELLANEOUS.WBS.007** Cerrar la migracion auditada de `game/Miscellaneous/enuminfo_RaceMask.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/enuminfo_RaceMask.cpp`
  Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-combat`, `crates/wow-data`, `crates/wow-script`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MISCELLANEOUS.WBS.008** Partir y cerrar la migracion auditada de `game/Miscellaneous/enuminfo_SharedDefines.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/enuminfo_SharedDefines.cpp`
  Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-combat`, `crates/wow-data`, `crates/wow-script`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 5199 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#MISC.1** Audit `wow-constants` against `SharedDefines.h`: produce a checklist of every enum + `MAX_*` and tick those already ported. (M)
- [ ] **#MISC.2** Add missing enums in `wow-constants` (spell families, item flags, faction templates, summon types, achievement criteria, etc.). (H — wide surface)
- [ ] **#MISC.3** Port `Formulas::XP::GetGrayLevel(plLevel)` → `wow-combat/src/xp.rs`. (L)
- [ ] **#MISC.4** Port `Formulas::XP::Gain(player, victim, is_bg)` full pipeline including base XP, level-diff modifier, rest bonus consumption, BG penalty, ScriptMgr hook surface. (M)
- [ ] **#MISC.5** Port `Formulas::XP::xp_to_level(level)` and align with `wow-data/quest_xp.rs` (single source of truth). (L)
- [ ] **#MISC.6** Port `Formulas::Honor::hk_honor_at_level{,_f}` and integrate with `wow-pvp`. (M)
- [ ] **#MISC.7** Port `Formulas::GetExpansionForLevel(level)`. (L)
- [ ] **#MISC.8** Port `Formulas::Currency::ConquestRatingCalculator` and the Conquest/Honor cap logic. (M)
- [ ] **#MISC.9** Port `Language` enum to `wow-constants/src/language.rs` (~1200 entries). Generate from C++ via a one-off script. (M)
- [ ] **#MISC.10** Wire `language.rs` into chat/system-message senders to replace hardcoded English. (M)
- [ ] **#MISC.11** Port `RaceMask<T>` as `wow-core/src/race_mask.rs` — `pub struct RaceMask(u64); fn has_race(r: Race) -> bool`. (L)
- [ ] **#MISC.12** Refit `MailLevelReward` and other consumers to use `RaceMask`. (L)
- [ ] **#MISC.13** Port `CommonPredicates` as `wow-combat/src/predicates.rs` with `IsVictimOf`, `HealthPctOrder`, `PowerPctOrder`, `Inverter`. (L)
- [ ] **#MISC.14** Replace ad-hoc closures in spell targeting / AI threat sorting with the predicates. (M)
- [ ] **#MISC.15** Decide policy on enum reflection: skip (Rust + a `strum` derive macro) vs port `enuminfo_*` mapping tables. Recommended: `strum::Display` + `strum::EnumIter`, scoped to the GM-command crate. (M)
- [ ] **#MISC.16** Add `wowlang` integration test: load `trinity_string` rows for a known `LANG_*` and verify EN+ESES strings match TC capture. (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#MISCELLANEOUS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 8 files / 15408 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/enuminfo_SharedDefines.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/Language.h`. Rust target: `crates/wow-combat`, `crates/wow-constants`, `crates/wow-core`, `crates/wow-data`, `crates/wow-script`. | `cargo test -p wow-combat && cargo test -p wow-constants && cargo test -p wow-core` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#MISCELLANEOUS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 8 files / 15408 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/enuminfo_SharedDefines.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/Language.h`. Rust target: `crates/wow-combat`, `crates/wow-constants`, `crates/wow-core`, `crates/wow-data`, `crates/wow-script`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#MISCELLANEOUS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 8 files / 15408 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/enuminfo_SharedDefines.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/Language.h`. Rust target: `crates/wow-combat`, `crates/wow-constants`, `crates/wow-core`, `crates/wow-data`, `crates/wow-script`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#MISCELLANEOUS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 8 files / 15408 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/enuminfo_SharedDefines.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/Language.h`. Rust target: `crates/wow-combat`, `crates/wow-constants`, `crates/wow-core`, `crates/wow-data`, `crates/wow-script`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `xp_to_level(70)` matches TC value (4 977 600 XP at end of TBC content) — single golden number check
- [ ] Test: `get_gray_level(80) == 71` (Wrath cap rules)
- [ ] Test: `xp_gain(level 80 player, level 80 mob)` returns the same XP as TC's reference computation for a battlegroundless kill
- [ ] Test: `honor_at_level(80) == ceil(80 * 1.55)` — formula parity
- [ ] Test: `get_expansion_for_level(78) == EXPANSION_WRATH_OF_THE_LICH_KING`
- [ ] Test: `RaceMask::from(&[RACE_HUMAN, RACE_DWARF]).has_race(RACE_HUMAN) == true`, `.has_race(RACE_ORC) == false`
- [ ] Test: `health_pct_order(asc=true)` sorts a `[100%, 50%, 75%]` triple to `[50, 75, 100]`
- [ ] Test: every `Powers` enum value in Rust has the same numeric discriminant as the C++ `enum Powers`
- [ ] Test: every `Classes` enum value is in `1..=10` (Wrath set), no extras leaked from later expansions
- [ ] Test: language id `LANG_BG_AB_NODE_ASSAULTED` resolves to a non-empty string for at least `enUS` and `esES` locales

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 8 files / 15408 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/enuminfo_SharedDefines.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/Language.h` | `wow-constants`, `wow-core`, `wow-combat`, `wow-data`, `wow-script` (scattered) \| ⚠️ partial — only `SharedDefines` constants and a few `Formulas::XP::*` curves are partially ported |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#MISCELLANEOUS.DIV.001` | `crates/wow-combat` (`exists_empty`, 0 Rust lines) | 8 C++ files / 15408 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/enuminfo_SharedDefines.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/Language.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#MISCELLANEOUS.DIV.002` | `crates/wow-script` (`exists_empty`, 0 Rust lines) | 8 C++ files / 15408 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/enuminfo_SharedDefines.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/Language.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **`SharedDefines.h` is multi-expansion in the upstream branch.** Many enums extend beyond Wrath (Cataclysm classes, Mists pandaren races, BFA allied races). For 3.4.3 you only need the subset gated by expansion ≤ 2; copying everything is harmless but wasteful and risks introducing bogus race/class ids the client cannot recognise.
- **`Formulas.h` is `inline` + included from headers.** Don't replicate that pattern in Rust — put functions in regular modules (`wow-combat/src/xp.rs`, etc.) and let LTO handle inlining. The reason TC keeps these inline is C++ ODR / single-translation-unit constraints, not performance.
- **`enuminfo_SharedDefines.cpp` is generated.** It is 5 199 lines but contains zero hand-written logic — it's the auto-generated `// TITLE` / `// DESCRIPTION` reflection table. Don't port it manually; if you need enum reflection, use `strum` / `num_enum` / `derive_more::Display` macros.
- **`Trinity::XP::Gain` calls `ScriptMgr::OnXPGain`.** Without script hook integration the xp number is correct but mods/scripts that adjust XP cannot intercept. Decide if `wow-script` exposes this hook before declaring the migration done.
- **`Language.h` ids are referenced by integer in many places.** When porting, preserve the numeric discriminants — content saved in `trinity_string` keys off the int. Reordering = i18n breakage.
- **`RaceMask` template has `T = uint64_t` for masks that include 9th-expansion races; for Wrath you can fit in `u32`.** Don't downcast permanently — the upstream MailLevelReward struct uses `uint64_t`. Use `u64` for forward compatibility.
- **`CommonPredicates::HealthPctOrderPred` calls `Unit::GetHealthPct`.** That method must exist on the Rust unit type before the predicate is portable.
- **`GetExpansionForLevel`** is a stair-step function. The Wrath-only Rust port can hard-code `level → 2` for `level ≤ 80` and not worry about higher expansions, but document the assumption.
- **Honor formula `level * 1.55`** is the wrath value. Cataclysm/Mists changed it to `level * 1.55 * dampen` and BfA further. Keep the 1.55 constant for 3.4.3 fidelity.
- **The `Trinity::` namespace is both a project marker and a real C++ namespace.** In Rust we don't need it — ports go into `wow-combat::xp`, `wow-pvp::honor`, `wow-core::race_mask`, etc. The naming convention `Trinity::Foo::bar` should not survive translation.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `enum Classes`, `enum Races`, `enum Powers`, `enum WeaponAttackType`, … (in `SharedDefines.h`) | `enum` types in `wow-constants/{unit,creature,item,...}.rs` with explicit discriminants | Keep numeric discriminants identical to C++ |
| `MAX_CLASSES`, `MAX_POWERS`, etc. | `pub const MAX_CLASSES: u32 = 10;` | Group in `wow-constants/lib.rs` or per-domain submodule |
| `template<typename T> struct RaceMask` | `struct RaceMask(u64)` (single concrete type — no template needed) | `impl RaceMask { fn has_race(self, r: Race) -> bool }` |
| `enum Language` (1200+ entries) | `enum Language { ... }` in `wow-constants/src/language.rs` | Discriminants must match C++; codegen recommended |
| `inline uint32 Trinity::GetExpansionForLevel(level)` | `pub fn expansion_for_level(level: u32) -> Expansion` in `wow-combat::xp` | Returns enum, not raw u32 |
| `inline uint32 Trinity::Honor::hk_honor_at_level(level, mult)` | `pub fn honor_at_level(level: u8, mult: f32) -> u32` in `wow-pvp::honor` | — |
| `inline uint8 Trinity::XP::GetGrayLevel(plLevel)` | `pub fn gray_level(player_level: u8) -> u8` in `wow-combat::xp` | — |
| `inline uint32 Trinity::XP::Gain(Player*, Unit*, bool)` | `pub fn xp_gain(player: &Player, victim: &Unit, in_bg: bool) -> u32` in `wow-combat::xp` | — |
| `Trinity::Predicates::IsVictimOf(Unit*)` | `pub fn is_victim_of(attacker: &Unit) -> impl Fn(&dyn UnitLike) -> bool` | Or named struct with `Fn` impl |
| `Trinity::Predicates::HealthPctOrderPred` | `pub fn health_pct_cmp(asc: bool) -> impl Fn(&Unit, &Unit) -> Ordering` | — |
| `enuminfo_SharedDefines.cpp` (auto-gen) | `#[derive(strum::Display, strum::EnumIter)]` on each enum | No manual port |
| `BattlenetRpcErrorCode` | (lives in `wow-proto` — see `proto.md`) | Cross-cutting |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Constant volume in `wow-constants`** (CamelCase enum variant count via `grep -cE "^\s*[A-Z][A-Za-z0-9_]+\s*[,=]"`):

| File | Variants | Lines | Purpose |
|---|---|---|---|
| `opcodes.rs` | 1618 | 1642 | `Opcode` enum |
| `item.rs` | 806 | 1239 | item flags / classes / subclasses / qualities |
| `spell.rs` | 440 | 569 | spell families / mechanics / aura types |
| `unit.rs` | 231 | 599 | races / classes / powers / unit flags |
| `shared.rs` | 208 | 464 | language / chat / `MAX_*` |
| `creature.rs` | 144 | 623 | creature flags / family / type |
| `object.rs` | 90 | 173 | object types / dynamic flags |
| `update.rs` | 4 | 31 | UpdateField enum |
| `movement.rs` | 0 | 107 | movement flag bitfields (consts, not enum) |
| **Total** | **~3 541 variants** | 5 477 | (multi-domain) |

C++ `SharedDefines.h` has 8 184 lines and `enuminfo_SharedDefines.cpp` (the auto-generated reflection mirror) is 5 199 lines — broadly comparable in source volume to the 5 477-line Rust port. Coverage is roughly **40–50%** by surface (the doc's section 8 "70%+ missing" is too pessimistic; many spell families and item flags *are* present in `spell.rs`/`item.rs`, but pre-Wrath flag groups like `ItemFlags3+`, `SPELL_AURA_MOD_OVERRIDE_*` Cata additions, vehicle subtypes, achievement criteria types, and almost all `MAX_*` numeric bounds beyond the basics are absent). Update section 8 estimate to **~50% missing**, not 70%+.

**`Language.h` enum** (~1 200 entries): grep `LANG_` in `wow-constants/src/` finds only `AllLanguages = -1` in `shared.rs:32` (a different enum — chat language, not server-string id). **Confirmed absent.** Section 8 correct.

**`RaceMask<T>`**: `grep "RaceMask\|race_mask"` across `wow-core` and `wow-constants` returns **zero hits**. Confirmed absent. #MISC.11 still required.

**`CommonPredicates`**: no `is_victim_of`, `health_pct_order`, `power_pct_order`, `inverter` anywhere. Absent.

**`Formulas::Honor::*`**: zero hits for `honor_at_level`, `hk_honor`. Absent.

**`Formulas::XP`** partial port location confirmed: `crates/wow-world/src/session.rs:723` carries `gray_level(pl)` and `:734 zero_difference(pl)`, plus an inline XP-gain pipeline at `:700–720` that mirrors `Trinity::XP::Gain` (n_base_exp by content level, gray check, ZD-band scaling). It is **inlined into `WorldSession`**, not extracted to `wow-combat::xp` as section 8/9 anticipates — the formulas exist but in the wrong crate. Update #MISC.3/#MISC.4: refactor existing session-level code into `wow-combat`, don't write from scratch.

**`xp_to_level`**: `wow-data/src/quest_xp.rs:33` exists (loader); no closed-form fallback if the table is missing. Doc claim "partial" correct.

**`GetExpansionForLevel`**: not found. Absent.

**Verdict:** ⚠️ partial confirmed. Net adjustments to upstream sections: (a) constant coverage is ~50%, not 30%; (b) XP formulas are present but mis-located; (c) Language/RaceMask/CommonPredicates/Honor remain wholly absent — these four are the real gap.
