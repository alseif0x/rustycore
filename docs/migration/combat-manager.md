# Migration: Combat — CombatManager

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Combat/CombatManager.{h,cpp}` + `src/server/game/Server/Packets/CombatPackets.cpp` (CombatReference / PvPCombatReference broadcast packets).
> **Rust target crate(s):** `crates/wow-combat/` (empty — see §13)
> **Layer:** L5 sub-module of `combat.md`. Depends on `entities-unit.md` (L4 — Unit, IsValidAttackTarget), `combat-threat.md` (L5 sibling — every AddThreat auto-creates CombatRef), `combat-dealdamage.md` (L5 sibling — every DealDamage triggers SetInCombatWith), `ai.md` (L5 — JustEnteredCombat / JustExitedCombat hooks). Sibling of `combat-dealdamage.md`, `combat-threat.md`.
> **Status:** ❌ not started — entire subsystem absent. Surrogate is `WorldSession.in_combat: bool` per-player + `WorldSession.combat_target: Option<ObjectGuid>`.
> **Audited vs C++:** ✅ complete 2026-05-01 (see §13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Per-Unit combat state ledger. Tracks the bidirectional set of *combat partners* (every Unit currently in mutual combat with the owner), separated into PvE refs (no timeout — combat persists until evade or death) and PvP refs (5-second sliding timer that refreshes on every offensive action). Drives the in-combat state machine: ENTER (`SetInCombatWith` creates ref + fires `CreatureAI::JustEnteredCombat` + first-time-aggro `SMSG_AI_REACTION`), HOLD (`Update` ticks PvP timers, `RevalidateCombat` re-validates each ref against faction/charm changes), SUPPRESS (vanish, feign death, launched-but-not-landed missile mark a side as suppressed without ending the ref), EXIT (`EndCombat` removes both sides + fires `JustExitedCombat`), and EVADE (`EndCombatBeyondRange` clears all PvE refs when owner moves outside leash range, typically 25-50 yards). Owns the "is this Unit in combat?" predicate that gates regen, mounting, talent swapping, log-out timer, durability decay, AFK detection, and so on. Broadcasts `UNIT_FLAG_IN_COMBAT` to clients via UPDATE_OBJECT field updates and emits `SMSG_PVP_FLAGS_CHANGED` when PvP flag toggles inside the combat-state transition.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Combat/CombatManager.cpp` | 406 | `prefix` |
| `game/Combat/CombatManager.h` | 146 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Combat/CombatManager.h` | 146 | Public interface: `CombatReference` struct, `PvPCombatReference` (extends), `CombatManager` class |
| `src/server/game/Combat/CombatManager.cpp` | 406 | Implementation: SetInCombatWith, EndCombat, suppress, PvP timer, RevalidateCombat, NotifyAICombat, CanBeginCombat |
| `src/server/game/Server/Packets/CombatPackets.h` | 240 | Wire-format types (cf parent `combat.md` §2) |
| `src/server/game/Server/Packets/CombatPackets.cpp` | 166 | Packet impls including SMSG_PVP_FLAGS_CHANGED |
| `src/server/game/Entities/Unit/Unit.cpp` | (`SetInCombatState` ~lines 8500-8650) | Bridge: `Unit::SetInCombatWith` → `CombatManager::SetInCombatWith` |
| `src/server/game/Entities/Unit/Unit.cpp` | (`ClearInCombat` ~lines 8650-8720) | Bridge: clears all refs via `CombatManager::EndAllCombat` |
| `src/server/game/Entities/Creature/Creature.cpp` | (Update tick) | Calls `_combatManager.Update(diff)` per tick |
| `src/server/game/AI/CreatureAI/CreatureAI.cpp` | (`JustEnteredCombat`, `JustExitedCombat`, `EnterEvadeMode`) | Consumers of CombatManager events |
| `src/server/game/Spells/Auras/SpellAuraEffects.cpp` | (HandleAuraModInvisibility — vanish path) | Calls `SuppressPvPCombat` on aura apply |
| `src/server/game/Spells/Auras/SpellAuraEffects.cpp` | (HandleFeignDeath) | Calls `SuppressPvPCombat` + sets PvE refs to evading |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `CombatReference` | struct | Edge between two Units in combat. Fields: `first: Unit*`, `second: Unit*`, `_isPvP: bool`, `_suppressFirst: bool`, `_suppressSecond: bool`. Heap-allocated; both Units' `CombatManager` hold raw pointers to the same object |
| `PvPCombatReference` | struct (extends `CombatReference`) | Adds `expiry: TimePoint` (5-second sliding timer); `RefreshTimer` resets to now+5s on action |
| `CombatManager` | class (one per `Unit`) | Owns two `unordered_map<ObjectGuid, CombatReference*>`: `_pveRefs` and `_pvpRefs`; provides SetInCombatWith / EndAllCombat / Update / RevalidateCombat / EndCombatBeyondRange / SuppressPvPCombat |
| `CombatManager::PutReference` | private helper | Internal insert that handles existing-ref update |
| `CombatManager::PurgeReference` | private helper | Internal remove from one side's map only (called by `CombatReference::EndCombat` for both sides) |

There are no separate enums — all state is encoded in struct fields (booleans + GUIDs + timer).

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `CombatManager::CombatManager(Unit* owner)` | Bind owner pointer; init empty maps | — |
| `CombatManager::~CombatManager()` | Walk both maps; call `EndCombat` on each ref to clean both sides | `EndCombat` per ref |
| `CombatManager::HasCombat() const` | True if either map non-empty AND has at least one ref where this side is non-suppressed | iter |
| `CombatManager::HasPvECombat() const` / `HasPvPCombat() const` | Type-specific variant | iter |
| `CombatManager::IsInCombatWith(ObjectGuid) const` | Single-ref lookup | hashmap find both maps |
| `CombatManager::IsInCombatWith(Unit const*) const` | overload via `GetGUID()` | id |
| `CombatManager::SetInCombatWith(Unit* who, bool addSecondHalf = false)` | Create ref between owner and `who` (PvE or PvP based on `IsValidAttackTarget`); fire AI hook on first-time entry | `PutReference`, `NotifyAICombat`, `Unit::SetInCombatState` |
| `CombatManager::EndCombat(Unit* who)` | Remove ref between owner and `who` (both sides); fire `JustExitedCombat` if last | `CombatReference::EndCombat` → 2× `PurgeReference` |
| `CombatManager::EndAllPvECombat()` | Walk `_pveRefs`, call EndCombat on each | iter+EndCombat |
| `CombatManager::EndAllPvPCombat()` | Walk `_pvpRefs`, call EndCombat on each | id |
| `CombatManager::EndAllCombat()` | Both | id |
| `CombatManager::EndCombatBeyondRange(float range, bool includingPvP = false)` | Walk PvE refs, EndCombat any whose other side is beyond `range`; PvP refs ignored unless flag | distance check |
| `CombatManager::Update(uint32 tdiff)` | Tick PvP timers; expired refs are EndCombat'd; revalidate every Nth tick | `PvPCombatReference::Update`, `EndCombat` |
| `CombatManager::RevalidateCombat()` | Re-evaluate every ref against current faction/charm/flags; remove invalidated | per-ref `IsValidAttackTarget` |
| `CombatManager::SuppressPvPCombat()` | Mark all PvP refs as suppressed-on-our-side (vanish/feign death) | iter+suppress |
| `CombatManager::EndAllPvECombat()` (with reason) | Cleanup with optional AI evade reason | id |
| `static CombatManager::CanBeginCombat(Unit const* a, Unit const* b)` | Validation pre-check: faction, GM-mode, immune, dead, friendly, charmed, in-flight | `Unit::IsValidAttackTarget`, `Unit::HasUnitFlag`, `Unit::IsImmunedToDamage` |
| `static CombatManager::NotifyAICombat(Unit* who, Unit* target)` | Fire `JustEnteredCombat` on target's AI if first ref | `CreatureAI::JustEnteredCombat`, `Unit::SetInCombatState` |
| `CombatReference::EndCombat()` | Remove from both `first._combatManager` and `second._combatManager` | 2× `PurgeReference` |
| `CombatReference::SuppressFor(Unit*)` | Set the appropriate suppress flag (first or second based on which side is `who`) | direct field |
| `CombatReference::IsSuppressedFor(Unit const*) const` | Read flag for the requested side | direct field |
| `PvPCombatReference::Update(uint32 tdiff)` | Decrement timer; return true if expired | timer math |
| `PvPCombatReference::RefreshTimer()` | Reset timer to 5000 ms on any offensive action | direct field |

---

## 5. Module dependencies

**Depends on:**
- `entities-unit.md` — `Unit::IsValidAttackTarget`, `Unit::SetInCombatState`, `Unit::HasUnitFlag`, `Unit::IsAlive`, `Unit::IsCharmed`, `Unit::GetGUID`, `Unit::GetDistance`.
- `combat-threat.md` — every `ThreatManager::AddThreat` calls `CombatManager::SetInCombatWith` to ensure combat ref exists.
- `combat-dealdamage.md` — every `DealDamage` indirectly causes `SetInCombatWith` (via the threat add inside it).
- `ai.md` — fires `CreatureAI::JustEnteredCombat`, `CreatureAI::JustExitedCombat`, `CreatureAI::EnterEvadeMode`, `CreatureAI::AttackedBy`. PvE evade flow depends on `EndCombatBeyondRange` → `EnterEvadeMode`.
- `entities.md` (Creature) — `Creature::Update` calls `_combatManager.Update(diff)`.
- `entities.md` (Player) — Player log-out timer, `Player::CanFreeMove`, `Player::HasInCombatPvP`.
- `pvp.md` — PvP flag transitions trigger `SMSG_PVP_FLAGS_CHANGED`; PvP refs feed honor-kill detection.
- `spells-effects.md` — vanish (`SPELL_AURA_MOD_INVISIBILITY` with stealth flag) calls `SuppressPvPCombat`; feign death (`SPELL_AURA_FEIGN_DEATH`) suppresses + drops PvE; immunity auras may invalidate refs via `RevalidateCombat`.
- `maps.md` / `grids.md` — distance check for `EndCombatBeyondRange`; visibility for SMSG broadcast.
- `groups.md` — group invite blocked while in combat (predicate via `HasCombat`).
- `mounts.md` (in `entities.md`) — mount blocked while in combat.
- `chat.md` (whisper / log-out timer) — log-out 20s combat timer reads `HasCombat`.

**Depended on by:**
- `combat.md` — parent module summary.
- `ai.md` — `CreatureAI::JustEnteredCombat` / `JustExitedCombat` consumers.
- `combat-threat.md` — `ClearAllThreat` calls `EndAllCombat`; `AddThreat` calls `SetInCombatWith`.
- `combat-dealdamage.md` — `Unit::Kill` calls `EndAllCombat` for the dying side.
- `entities.md` (Creature) — `Creature::Update` reads `HasCombat` to decide tick path.
- `regen.md` (in `entities-unit.md`) — out-of-combat regen gated on `!HasCombat`.

---

## 6. SQL / DB queries (if any)

CombatManager emits no SQL. Inputs read at unit-load time:

| Statement / Source | Purpose | DB |
|---|---|---|
| `creature_template.faction` | Faction validation in `IsValidAttackTarget` | world |
| `creature_template.unit_flags` | UNIT_FLAG_NON_ATTACKABLE / UNIT_FLAG_IMMUNE_TO_PC / UNIT_FLAG_IN_COMBAT | world |
| `creature_template.unit_flags2` | UNIT_FLAG2_DISABLE_TURN | world |
| `creature_template.npcflag` | NPC flag immunities | world |
| `faction_template` (DBC) | Faction relations for `CanBeginCombat` | DBC |

DBC/DB2 stores consumed:

| Store | What it loads | Read by |
|---|---|---|
| `FactionTemplateStore` | Faction-vs-faction matrix | `CanBeginCombat` (via `Unit::IsValidAttackTarget`) |
| `FactionStore` | Faction identity | id |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_AI_REACTION` (0x26B5) | S→C | `CombatManager::SetInCombatWith` first-aggro broadcast: target GUID + `AI_REACTION_HOSTILE` |
| `SMSG_PVP_FLAGS_CHANGED` (3.4.3 specific) | S→C | `CombatManager` PvP-flag transitions — emitted when entering PvP combat or exiting (5s timer expires) |
| `UPDATE_OBJECT` field `UNIT_FIELD_FLAGS` (UNIT_FLAG_IN_COMBAT bit) | S→C | Set/cleared when `HasCombat` transitions; broadcast via grid update queue |
| `SMSG_DUEL_REQUESTED` / `SMSG_DUEL_COMPLETE` | S→C | Duel state transitions also pass through CombatManager (duel = special PvP combat ref) |
| `SMSG_LFG_PROPOSAL_UPDATE` (indirect) | S→C | LFG ready-check rejected while `HasCombat` |

There are no CMSG opcodes specific to CombatManager — combat state is purely server-derived.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-combat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-combat/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `crates/wow-ai/src/lib.rs` | `file` | 1 | 346 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/combat.rs` | `file` | 1 | 152 | `exists_active` | file exists |
| `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-combat/src/lib.rs` — **0 lines** (empty crate; see §13). No `CombatManager`, no `CombatReference`, no `PvPCombatReference`, no PvP timer, no suppress flags, no `RevalidateCombat`, no `EndCombatBeyondRange`, no `CanBeginCombat`.
- `crates/wow-world/src/session.rs` (and `WorldSession` definition) — `WorldSession.in_combat: bool` (per-player flag), `WorldSession.combat_target: Option<ObjectGuid>` (single-target).
- `crates/wow-ai/src/lib.rs` — `CreatureState::InCombat` boolean inside the creature struct. Methods: `enter_combat(player_guid)`, `reset_combat()`.
- `crates/wow-world/src/handlers/combat.rs` — sets `WorldSession.in_combat = true` and `creature.enter_combat(player_guid)` on CMSG_ATTACK_SWING; clears both on CMSG_ATTACK_STOP.

**What's implemented:**
- Player-side `in_combat: bool` toggle (one boolean per session).
- Creature-side `is_in_combat: bool` toggle (one boolean per creature in `wow-ai`).
- Single combat target attribution: `creature.attacker = Some(player_guid)` (overwritten unconditionally on each `enter_combat`).
- Combat exit on CMSG_ATTACK_STOP: `in_combat = false`, `attacker = None`.
- Combat exit on creature death: implicit — creature flagged not alive, attacker stays in `creature.attacker` field but never read.

**What's missing vs C++:**
- **`CombatManager` struct** — does not exist.
- **`CombatReference` struct** — does not exist; combat is not modeled as a ref-counted edge.
- **`PvPCombatReference` extension** — no PvP timer; PvP combat exits the moment last `CMSG_ATTACK_STOP` arrives (instant) instead of 5-second sliding window.
- **PvE vs PvP separation** — single `in_combat` boolean cannot distinguish; PvE evade rules cannot diverge from PvP timeout rules.
- **Multiple combat partners** — `WorldSession.combat_target: Option<ObjectGuid>` is single-valued; cannot model a player in combat with 3 mobs at once.
- **`SetInCombatWith`** — no proper API. Combat entry happens implicitly via boolean assignment with no validation, no AI hook, no SMSG_AI_REACTION, no faction/immune/charm pre-check.
- **`EndCombat`** — no per-ref removal API. Cannot "exit combat with mob A while staying in combat with mob B".
- **`EndAllPvECombat` / `EndAllPvPCombat` / `EndAllCombat`** — only "clear `in_combat` boolean" exists; no per-ref iteration, no AI hooks fired.
- **`EndCombatBeyondRange`** — no leash mechanism. Creatures do not evade when player runs out of range; they remain "in combat" until killed or stopped.
- **`SuppressPvPCombat`** — no suppress flag concept. Vanish (rogue), Feign Death (hunter), Camouflage do not suppress combat refs — they cannot work correctly.
- **`RevalidateCombat`** — no re-validation. Faction-change / charm-break / flagging changes mid-fight do not propagate to the combat ledger.
- **`CanBeginCombat`** — no static validation. Combat can be entered against immune / dead / GM-mode / friendly targets without rejection.
- **`Update(tdiff)`** — no tick. PvP timer cannot expire; combat states are purely event-driven without time evolution.
- **`NotifyAICombat`** — `CreatureAI::JustEnteredCombat` is not fired on aggro; `JustExitedCombat` is not fired on combat exit. Most PvE encounter scripts (boss enrage timers, phase transitions, ability cycling) depend on these hooks.
- **`SMSG_AI_REACTION`** — packet exists in `wow-packet` but is not emitted by combat entry path.
- **`SMSG_PVP_FLAGS_CHANGED`** — not emitted on combat-driven PvP flag transitions.
- **`UNIT_FLAG_IN_COMBAT` field update** — the flag bit is not toggled on the Unit's `UNIT_FIELD_FLAGS` field via UPDATE_OBJECT broadcast; clients do not see "in combat" indicators on other players/creatures.
- **First-time-aggro emit** — there is no "first add" detection that emits `SMSG_AI_REACTION` once per encounter.
- **Suppress flag dual-side** — `_suppressFirst` and `_suppressSecond` separately track whether each side is suppressed; this dual-flag model is absent.
- **Duel as combat ref** — no special-cased duel CombatReference; duels are not in a combat partnership state.
- **Combat broadcast** — observer Players nearby do not receive any combat-state SMSG_AI_REACTION broadcasts.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `WorldSession.in_combat: bool` is a *per-session* flag, not a *per-Unit* flag — the player's character may lack the in-combat state at the Unit level (which is what other systems test against).
- `creature.is_in_combat: bool` flips on the first attacker, but never flips back to `false` if the attacker dies or logs out (no listener).
- The PvP 5-second timer on log-out is not implemented — players can `/logout` mid-PvP and skip the 20-second combat-logout timer that should apply.
- Mounting is not gated on combat state in code, meaning players can mount mid-combat (rule violation).
- Talent-tree swapping (when implemented) will need `HasCombat` predicate — currently unsourceable.
- HP/mana regen ticks are gated on `WorldSession.in_combat` — but that's per-session, not per-unit, so creatures regen during combat (incorrect).

**Tests existing:**
- 0 tests for CombatManager.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#COMBAT_MANAGER.WBS.001** Cerrar la migracion auditada de `game/Combat/CombatManager.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.cpp`
  Rust target: `crates/wow-combat`, `crates/wow-ai`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#COMBAT_MANAGER.WBS.002** Cerrar la migracion auditada de `game/Combat/CombatManager.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.h`
  Rust target: `crates/wow-combat`, `crates/wow-ai`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for `MIGRATION_ROADMAP.md` §5 reference.
Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h).

- [ ] **#COMBAT-MGR.1** Define `struct CombatReference { first: ObjectGuid, second: ObjectGuid, is_pvp: bool, suppress_first: bool, suppress_second: bool }`. (L)
- [ ] **#COMBAT-MGR.2** Define `struct PvPCombatReference { base: CombatReference, expiry_ms: u64 }` + `fn refresh_timer(&mut self, now_ms)` + `fn update(&mut self, now_ms) -> bool`. (M)
- [ ] **#COMBAT-MGR.3** Define `struct CombatManager { owner: ObjectGuid, pve_refs: HashMap<ObjectGuid, Box<CombatReference>>, pvp_refs: HashMap<ObjectGuid, Box<PvPCombatReference>>, last_revalidate_tick: u32 }`. (M)
- [ ] **#COMBAT-MGR.4** `fn has_combat(&self) -> bool` + `fn has_pve_combat(&self) -> bool` + `fn has_pvp_combat(&self) -> bool` — count non-suppressed refs from owner's side. (L)
- [ ] **#COMBAT-MGR.5** `fn is_in_combat_with(&self, target: ObjectGuid) -> bool` — both maps lookup. (L)
- [ ] **#COMBAT-MGR.6** `fn set_in_combat_with(&mut self, world: &mut WorldState, who: ObjectGuid, add_second_half: bool)` — validate via `can_begin_combat`, decide PvP vs PvE via `is_valid_attack_target`, insert ref in both directions, set `Unit::set_in_combat_state(true)` on both sides, fire `notify_ai_combat`, broadcast `SMSG_AI_REACTION` if first PvE entry. (XL)
- [ ] **#COMBAT-MGR.7** `fn end_combat(&mut self, world, who: ObjectGuid)` — remove ref from both sides; fire `CreatureAI::just_exited_combat` if last ref; clear `UNIT_FLAG_IN_COMBAT` on owner if both maps empty. (H)
- [ ] **#COMBAT-MGR.8** `fn end_all_pve_combat(&mut self, world)` + `fn end_all_pvp_combat(&mut self, world)` + `fn end_all_combat(&mut self, world)`. (M)
- [ ] **#COMBAT-MGR.9** `fn end_combat_beyond_range(&mut self, world, range_yards: f32, including_pvp: bool)` — walk PvE refs; for each, compute distance via owner & second's positions; if > range, end combat. Fires evade flow. (H)
- [ ] **#COMBAT-MGR.10** `fn update(&mut self, world, diff_ms: u32, now_ms: u64)` — tick PvP timers (expired → end_combat); revalidate every Nth tick (e.g. 250ms). (M)
- [ ] **#COMBAT-MGR.11** `fn revalidate_combat(&mut self, world)` — walk both maps; for each ref, re-eval `is_valid_attack_target` (faction change, charm break, flag toggle); EndCombat invalidated refs. (M)
- [ ] **#COMBAT-MGR.12** `fn suppress_pvp_combat(&mut self)` — set `suppress_first` (or `suppress_second` based on which side owner is) on every PvP ref. Used by vanish / feign death. (M)
- [ ] **#COMBAT-MGR.13** `static fn can_begin_combat(world: &WorldState, a: ObjectGuid, b: ObjectGuid) -> bool` — faction, GM-mode, dead, immune, friendly, charmed, in-flight, NON_ATTACKABLE flag checks. (M)
- [ ] **#COMBAT-MGR.14** `static fn notify_ai_combat(world, who: ObjectGuid, target: ObjectGuid)` — fire `CreatureAI::just_entered_combat(target)` if `target` is the first ref; fire `CreatureAI::attacked_by(who)`. (M)
- [ ] **#COMBAT-MGR.15** SMSG_AI_REACTION emit from `set_in_combat_with` first-aggro path: `(target_guid: ObjectGuid, reaction: u32 = 2 /* HOSTILE */)`. (L)
- [ ] **#COMBAT-MGR.16** SMSG_PVP_FLAGS_CHANGED writer + emit from PvP entry/exit (`pvp_refs.is_empty() = false → true` and vice versa). (M)
- [ ] **#COMBAT-MGR.17** `UNIT_FIELD_FLAGS` `UNIT_FLAG_IN_COMBAT` bit toggle on combat enter/exit; broadcast via UPDATE_OBJECT field-update queue to grid observers. (M)
- [ ] **#COMBAT-MGR.18** Bridge to `Unit::set_in_combat_state(in_combat: bool)` — sets unit-side flag; `Unit::clear_in_combat()` calls `combat_manager.end_all_combat()`. (M)
- [ ] **#COMBAT-MGR.19** Player log-out timer integration: `HasCombat` true → 20-second log-out delay; PvP combat extends the timer. (M)
- [ ] **#COMBAT-MGR.20** Mount / Ground-mount / Flight gating on `HasCombat`. (L)
- [ ] **#COMBAT-MGR.21** OOC regen integration: `Unit::ModifyHealthRegenInCombat` reads `combat_manager.has_combat()`. (L)
- [ ] **#COMBAT-MGR.22** Vanish / Feign Death aura handlers call `suppress_pvp_combat()` + `end_all_pve_combat()` (with AI evade reason). (M)
- [ ] **#COMBAT-MGR.23** Duel as special-cased PvP CombatReference: `Player::Duel(target)` creates a PvPCombatReference flagged `is_duel = true`; on duel-end, the ref is purged regardless of timer. (M)
- [ ] **#COMBAT-MGR.24** Pet combat partnership: pet hits add owner+target to `set_in_combat_with`; pet damage routes both threat (to owner) AND combat ref (to owner). (M)
- [ ] **#COMBAT-MGR.25** Group "in combat" predicate: `Group::IsInCombat() → any member.combat_manager.has_combat()`. (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#COMBAT_MANAGER.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 552 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.h`. Rust target: `crates/wow-ai`, `crates/wow-combat`, `crates/wow-packet`. | `cargo test -p wow-ai && cargo test -p wow-combat && cargo test -p wow-packet` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#COMBAT_MANAGER.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 552 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.h`. Rust target: `crates/wow-ai`, `crates/wow-combat`, `crates/wow-packet`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#COMBAT_MANAGER.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 552 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.h`. Rust target: `crates/wow-ai`, `crates/wow-combat`, `crates/wow-packet`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#COMBAT_MANAGER.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 552 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.h`. Rust target: `crates/wow-ai`, `crates/wow-combat`, `crates/wow-packet`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `set_in_combat_with(A, B)` creates refs in both directions; `is_in_combat_with(B)` true on A's manager, `is_in_combat_with(A)` true on B's manager.
- [ ] Test: `end_combat(B)` from A's manager removes ref from BOTH A's and B's managers (no leak).
- [ ] Test: `set_in_combat_with(A, B)` in PvP context creates `PvPCombatReference` not `CombatReference`; A's `pvp_refs` and B's `pvp_refs` both populated.
- [ ] Test: `PvPCombatReference::update(now_ms = creation_ms + 5001)` returns `true` (expired); call `end_combat` cleanups both sides.
- [ ] Test: `refresh_timer()` on existing PvP ref resets expiry to now+5000ms.
- [ ] Test: `suppress_for(A)` makes `has_combat()` ignore that ref *from A's perspective*; B's `has_combat()` still counts the ref.
- [ ] Test: `end_combat_beyond_range(25.0, including_pvp=false)` on A's manager — B is at 30 yards → ref ends; PvP ref to C at 100 yards stays.
- [ ] Test: `end_combat_beyond_range(50.0, including_pvp=true)` — PvP ref to C at 100 yards ends.
- [ ] Test: `revalidate_combat()` — A's faction is changed mid-fight; previously hostile B now friendly → ref auto-removed.
- [ ] Test: `can_begin_combat` rejects: GM-mode target, NON_ATTACKABLE flag, dead target, friendly faction, in-flight target.
- [ ] Test: `set_in_combat_with(A, B)` with B already in combat with C → second-half-add does NOT clear B's ref to C.
- [ ] Test: `end_all_pve_combat()` ends only PvE refs; `pvp_refs` survive.
- [ ] Test: `notify_ai_combat` fires `JustEnteredCombat(B)` once on B's CreatureAI when B's first ref is added; does NOT re-fire on second/third add.
- [ ] Test: SMSG_AI_REACTION emitted to all observing Players when B's first ref to A is added.
- [ ] Test: SMSG_PVP_FLAGS_CHANGED emitted when transition `pvp_refs.is_empty() → false`; a second PvP ref does NOT re-emit; transition `→ empty` re-emits "exit PvP" flag.
- [ ] Test: `UNIT_FIELD_FLAGS` bit `UNIT_FLAG_IN_COMBAT` set on entering combat; cleared on `has_combat() == false`; broadcast in next UPDATE_OBJECT.
- [ ] Test: `set_in_combat_with(A, B)` validates: A and B alive, A and B not GM, no immunity blocking, returns false silently if `can_begin_combat == false`.
- [ ] Test: Vanish (rogue): `suppress_pvp_combat()` sets `suppress_first` on every PvP ref where rogue is `first`; rogue's `has_combat()` == false; opponents still have the ref but cannot target the rogue.
- [ ] Test: Player log-out timer: `WorldSession::log_out_request` while `has_combat()` == true → 20-second delay; mid-PvP → 5s additional after PvP timer.
- [ ] Test: Mount attempt while `has_combat()` == true → SMSG_MOUNT_RESULT denial.
- [ ] Test: Group ready-check: `Group::is_in_combat()` true if any member's `combat_manager.has_combat()` true.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 552 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.h` | `crates/wow-combat/` (empty — see §13) \| ❌ not started — entire subsystem absent. Surrogate is `WorldSession.in_combat: bool` per-player + `WorldSession.combat_target: Option<ObjectGuid>`. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#COMBAT_MANAGER.DIV.001` | `crates/wow-combat` (`exists_empty`, 0 Rust lines) | 2 C++ files / 552 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#COMBAT_MANAGER.DIV.002` | `crates/wow-combat/src/lib.rs` (`exists_empty`, 0 Rust lines) | 2 C++ files / 552 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |

<!-- REFINE.023:END known-divergences -->

- **PvE vs PvP refs are physically separate maps**: a Unit can hold both `_pveRefs[mob_guid]` and `_pvpRefs[player_guid]` simultaneously. Don't merge them — the eviction rules differ (PvE = leash distance, PvP = 5-second timer).
- **Suppress flags are dual-sided**: `_suppressFirst` and `_suppressSecond` track each side independently. A vanished rogue is `suppressed` on rogue's side; the opponent is still actively in combat from their side until *their* condition triggers (combat end, target cleared). Implementing as a single `suppressed: bool` is wrong.
- **Combat ref is heap-allocated and shared**: in C++, a single `CombatReference*` is held by both `_combatManager.pveRefs[other_guid]` and `other._combatManager.pveRefs[self_guid]`. Both pointers point to the same struct. EndCombat purges from both sides via `PurgeReference` × 2. In Rust, use `Box<CombatReference>` owned by *one* side, `&CombatReference` reference held by other (or rebuild lookups as `HashMap<(ObjectGuid, ObjectGuid), CombatReference>` keyed by sorted pair — simpler).
- **PvP timer 5000ms is hard-coded in Blizzlike 3.4.3**: do not parameterize it. Resets on EVERY offensive action: spell cast, damage dealt, taunt, debuff applied. Idle 5s = exit PvP combat. Raid wipe detection is "all players exit combat" → trips on this exact timer.
- **`SetInCombatWith` is symmetric but the AI hook is one-way**: if A is a Player and B is a Creature, `JustEnteredCombat` fires on B's AI only (Players have no CreatureAI). Mutual creature-vs-creature combat fires both AI hooks.
- **First-aggro `SMSG_AI_REACTION` fires once per encounter**: the second Unit attacking the same target should NOT re-emit. C++ tracks this via "first ref add" in `SetInCombatWith`. Implementing as "emit on every add" causes the client to play the aggro grunt repeatedly.
- **`UNIT_FLAG_IN_COMBAT` is a UPDATE_OBJECT field bit**: changing it requires re-broadcasting the unit's UNIT_FIELD_FLAGS field to all observers. Don't try to send a one-off packet for it.
- **`EndCombatBeyondRange` is the leash**: typical PvE leash distance is 25-50 yards (mob-specific via `creature_template.leash_radius` if set; default 25). PvP refs ignore range — only the 5s timer ends them.
- **`RevalidateCombat` cadence**: C++ runs every ~250ms in `Update`. Doing it every tick is expensive; doing it never causes faction-change exploits. 4Hz is the canonical rate.
- **`CanBeginCombat` is mandatory pre-validation**: skipping it allows GM characters to enter combat, dead targets to be aggro'd, friendly NPCs to be attacked. Always check before `SetInCombatWith` even from internal callers.
- **Charm break**: when charm aura on B breaks while A is in combat with B, the ref must be re-validated (A and B may now be friendly/hostile differently). Without `RevalidateCombat`, the combat ref persists incorrectly.
- **Duel completion**: duels end via timeout (3 min idle) or explicit yield. The PvP combat ref must be force-purged regardless of 5s timer, AND `UNIT_FIELD_FLAGS` PvP-flag bit must be cleared.
- **Pet combat partnerships**: a pet entering combat causes the *owner* to enter combat too. Pet's CombatReference is held both for pet AND for owner. When pet dies, owner's ref to the same target survives (owner is still attacking).
- **Vanish (rogue) details**: not just suppress — it ALSO ends all current PvE refs immediately (mobs evade) AND suppresses PvP refs (5s window for the rogue to fade out). Mis-implementing as just "suppress all" leaves PvE mobs swinging at empty space.
- **Feign Death (hunter) details**: similar to vanish but on a 1.5s GCD; suppresses all refs; mobs evade if FD succeeds the resist roll. Failed FD does nothing (no suppress).
- **Group invite / mount / talent-swap gating**: all rely on `HasCombat()`. If any of them call the per-session boolean instead, behavior diverges from C++ (e.g., player mounts mid-combat because `WorldSession.in_combat` got cleared but Unit-level combat is still active).
- **Log-out timer**: 20s normal, 5s if "rest area" (inn/city), instant if not in combat. The 20s timer is gated on `HasCombat()`. PvP-combat exit triggers a fresh 5s timer overlay (so "/logout while PvP-flagged" → wait full 5s + 20s if PvP combat refs present).
- **AFK auto-logout**: 30 minutes idle in PvP areas auto-logs the player; gated on `HasPvPCombat()` to prevent kicking mid-fight.
- **Combat broadcasting**: SMSG_AI_REACTION + UNIT_FIELD_FLAGS update go to everyone in visibility range. Don't restrict to party-only. Conversely, SMSG_PVP_FLAGS_CHANGED is broadcast to everyone too.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class CombatManager` (per-Unit) | `struct CombatManager { owner: ObjectGuid, pve_refs: HashMap<ObjectGuid, Box<CombatReference>>, pvp_refs: HashMap<ObjectGuid, Box<PvPCombatReference>>, last_revalidate_tick: u32 }` | Composition in Unit (no inheritance) |
| `struct CombatReference` | `struct CombatReference { first: ObjectGuid, second: ObjectGuid, is_pvp: bool, suppress_first: bool, suppress_second: bool }` | GUIDs not pointers |
| `struct PvPCombatReference : CombatReference` | `struct PvPCombatReference { base: CombatReference, expiry_ms: u64 }` | Composition not inheritance |
| `void SetInCombatWith(Unit*, bool)` | `fn set_in_combat_with(&mut self, world: &mut WorldState, who: ObjectGuid, add_second_half: bool)` | Needs world access for second-half-add |
| `void EndCombat(Unit*)` | `fn end_combat(&mut self, world: &mut WorldState, who: ObjectGuid)` | World access for AI hook + bidirectional purge |
| `void EndCombatBeyondRange(float, bool)` | `fn end_combat_beyond_range(&mut self, world: &mut WorldState, range_yards: f32, including_pvp: bool)` | World access for distance lookup |
| `void Update(uint32 tdiff)` | `fn update(&mut self, world: &mut WorldState, diff_ms: u32, now_ms: u64)` | now_ms for PvP timer evaluation |
| `void RevalidateCombat()` | `fn revalidate_combat(&mut self, world: &mut WorldState)` | World for `IsValidAttackTarget` |
| `void SuppressPvPCombat()` | `fn suppress_pvp_combat(&mut self)` | Mutates only own state |
| `bool HasCombat() const` | `fn has_combat(&self) -> bool` | — |
| `bool IsInCombatWith(ObjectGuid) const` | `fn is_in_combat_with(&self, target: ObjectGuid) -> bool` | — |
| `static bool CanBeginCombat(Unit const*, Unit const*)` | `pub fn can_begin_combat(world: &WorldState, a: ObjectGuid, b: ObjectGuid) -> bool` | Free function in `wow-combat::manager` |
| `static void NotifyAICombat(Unit*, Unit*)` | `pub fn notify_ai_combat(world: &mut WorldState, who: ObjectGuid, target: ObjectGuid)` | Free function |
| `std::unordered_map<ObjectGuid, CombatReference*> _pveRefs` | `HashMap<ObjectGuid, Box<CombatReference>>` | Box-owned not raw-ptr |
| `void CombatReference::EndCombat()` | `fn end_combat(self, world: &mut WorldState)` (consuming `self`) | Self-consumed because purges from both sides → no danglers |
| `void CombatReference::SuppressFor(Unit*)` | `fn suppress_for(&mut self, who: ObjectGuid)` | — |
| `bool PvPCombatReference::Update(uint32 tdiff)` | `fn update(&mut self, now_ms: u64) -> bool` | True = expired |
| `void PvPCombatReference::RefreshTimer()` | `fn refresh_timer(&mut self, now_ms: u64)` | — |
| `WorldPackets::Combat::AIReaction` | `struct AIReaction { target: ObjectGuid, reaction: u32 }` (existe en `wow-packet`) | — |
| `SMSG_PVP_FLAGS_CHANGED` (3.4.3) | new packet writer needed | — |

---

*Template version: 1.0 (2026-05-01).* Last updated: 2026-05-01.

---

## 13. Audit (2026-05-01)

**Scope.** Audited the C++ CombatManager at `/home/server/woltk-trinity-legacy/src/server/game/Combat/CombatManager.h` (146 lines, CombatReference / PvPCombatReference structs + CombatManager class declaration) and `CombatManager.cpp` (406 lines, full SetInCombatWith / EndCombat / EndCombatBeyondRange / RevalidateCombat / SuppressPvPCombat / NotifyAICombat / CanBeginCombat implementation), against the Rust workspace at `/home/server/rustycore/crates/wow-combat/`.

**Empty-crate finding — CONFIRMED.** `/home/server/rustycore/crates/wow-combat/src/lib.rs` is **0 lines**. None of the CombatManager subsystem exists in any form: no `CombatManager` struct, no `CombatReference` struct, no `PvPCombatReference` extension, no `_pveRefs` / `_pvpRefs` maps, no PvP 5-second timer, no `SetInCombatWith`, no `EndCombatBeyondRange`, no `RevalidateCombat`, no `SuppressPvPCombat`, no `CanBeginCombat`, no `NotifyAICombat`, no SMSG_PVP_FLAGS_CHANGED writer.

**Surrogate path.** What runs today is two booleans:
1. `WorldSession.in_combat: bool` — per-session player-combat flag, toggled by `handle_attack_swing` / `handle_attack_stop` only. Not per-Unit; not propagated to other systems.
2. `CreatureState::InCombat` (boolean inside creature struct in `wow-ai`) — set on `enter_combat(player_guid)` (which also stores a single attacker GUID), cleared on `reset_combat()`.

This collapses the entire C++ CombatManager subsystem (per-Unit dual-map ref-counted with PvE/PvP separation, suppress flags, PvP timer, leash range, revalidation, AI hooks) into two booleans plus a single-attacker GUID. Multiple combat partners cannot be modeled. PvP combat exits instantly on `/stopattack` instead of after 5 seconds. Vanish, Feign Death, Camouflage cannot work. Mount-mid-combat is unblocked (no per-Unit `HasCombat` predicate). Log-out timer is not gated on combat state. Group invite / talent swap / talent-tree change cannot check `HasCombat`.

**Kill pipeline (combat-side) is broken.** When a Unit dies, C++ chains:
1. `CombatManager::EndAllCombat()` walks both maps, calling `EndCombat` on each ref.
2. `EndCombat` propagates to BOTH sides (`PurgeReference` × 2) — the dying side's ref is removed from its partner's map.
3. Each `EndCombat` fires `CreatureAI::JustExitedCombat` on the partner if the partner's last ref ended.
4. `UNIT_FIELD_FLAGS` UNIT_FLAG_IN_COMBAT bit is cleared on the dying side; broadcast to grid observers via UPDATE_OBJECT.

**In RustyCore, none of this runs.** The dying creature's `is_in_combat` boolean stays `true` (no listener ever resets it). The killing player's `WorldSession.in_combat` stays `true` (only cleared by `CMSG_ATTACK_STOP`). `CreatureAI::just_exited_combat` is never fired. UNIT_FLAG_IN_COMBAT bit is not toggled — clients still see "in combat" indicator on dead bodies. The PvP flag does not transition off after the timer (timer doesn't exist).

**Vanish / Feign Death / Camouflage divergence.** All three abilities depend on `SuppressPvPCombat()` to mark PvP refs as suppressed-on-self-side without clearing the partner's side. **Zero are implemented.** Casting Vanish today does nothing combat-state-wise — the rogue stays in combat, mobs still target them, the ability is functionally broken.

**Worst hidden divergence.** `CreatureAI::JustEnteredCombat` and `JustExitedCombat` are the foundational hooks for **every PvE encounter script** in the game. Boss enrage timers, phase transitions, ability cycle init, P1→P2 transition triggers, sub-add spawn timers, soft-enrage cooldowns, all initiate from `JustEnteredCombat`. Ending-combat reset (loot ready, despawn schedule, evade animation, return-to-spawn pathing) all initiate from `JustExitedCombat`. **Neither hook is fired in RustyCore today** — making boss script development impossible until `set_in_combat_with` and `end_combat` exist with proper AI hook integration. This is not just a missing API; it is a missing prerequisite for all encounter content (Naxxramas, Ulduar, ToC, Icecrown Citadel — every raid).

**Cross-references.** See `combat.md` §8 / §13 for the parent-doc audit (covers the same `wow-combat` 0-line confirmation). See `combat-dealdamage.md` §13 for the damage-pipeline companion gap (every `DealDamage` should call `SetInCombatWith` at the top — neither exists). See `combat-threat.md` §13 for the ThreatManager companion gap (every `AddThreat` should call `SetInCombatWith` to ensure combat ref — neither exists). See `ai.md` (when authored) for the consumer-side gap (`CreatureAI::JustEnteredCombat`/`JustExitedCombat` hooks unimplemented). See `pvp.md` (when authored) for SMSG_PVP_FLAGS_CHANGED + 5-second PvP timer gap details.
