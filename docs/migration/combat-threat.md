# Migration: Combat — ThreatManager

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Combat/ThreatManager.{h,cpp}` + `src/server/game/Combat/ThreatReference` (declared in same headers; no separate file). `HostileReference` is the legacy-name predecessor (3.3.5a) — in 3.4.3 it was replaced by `ThreatReference`.
> **Rust target crate(s):** `crates/wow-combat/` (empty — see §13)
> **Layer:** L5 sub-module of `combat.md`. Depends on `entities-unit.md` (L4 — Unit, Creature), `spells-effects.md` (L5 — taunt/threat-mod auras, redirect spells), `ai.md` (L5 — `CreatureAI::AttackStart`, victim selection). Sibling of `combat-dealdamage.md`, `combat-manager.md`.
> **Status:** ❌ not started — entire subsystem absent.
> **Audited vs C++:** ✅ complete 2026-05-01 (see §13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Per-Creature threat list. Tracks every Unit that has dealt damage to (or healed against) the owning creature, sorted by computed threat amount with tie-break rules around online/suppressed/offline state and taunt/none/detaunt state. Selects the `current victim` (top-of-heap online entry, taunt-honoring) which `CreatureAI` reads to drive `AttackStart`. Supports threat modifiers (Vengeance-style auras, Defensive Stance +30% physical/holy/nature, Tricks-of-the-Trade redirect, Misdirection redirect, Vigilance 100% transfer), taunt mechanics (`SPELL_AURA_MOD_TAUNT` flips taunt-state and calls `MatchUnitThreatToHighestThreat`), fixate (lock victim ignoring threat), threat decay on out-of-combat (drop to zero on `EnterEvadeMode`), and broadcasting threat updates to the player UI via `SMSG_THREAT_UPDATE` family packets.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Combat/ThreatManager.cpp` | 913 | `prefix` |
| `game/Combat/ThreatManager.h` | 321 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Combat/ThreatManager.h` | 321 | Public interface: `ThreatReference` class, `ThreatManager` class, `OnlineState` / `TauntState` enums, `CompareThreatLessThan` functor, redirect map types |
| `src/server/game/Combat/ThreatManager.cpp` | 913 | Implementation: heap maintenance, victim selection, modifier table, redirect logic, broadcast to clients |
| `src/server/game/Spells/Auras/SpellAuraEffects.cpp` | (HandleAuraModTaunt) | Aura handler that calls `ThreatManager::TauntUpdate` |
| `src/server/game/Spells/Auras/SpellAuraEffects.cpp` | (HandleAuraModThreat) | Aura handler that mutates threat-modifier-by-school |
| `src/server/game/Entities/Creature/Creature.cpp` | (Update tick) | Calls `_threatManager.Update(diff)` and reads `GetCurrentVictim()` |
| `src/server/game/Server/Packets/CombatPackets.cpp` | (SendThreatUpdate / SendThreatRemove / SendThreatClear / SendHighestThreatUpdate) | Wire-format writers for threat broadcast packets |
| `src/server/game/AI/CreatureAI/CreatureAI.cpp` | (SelectTarget, JustEnteredCombat) | Consumers of threat list |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `ThreatManager` | class (one per `Unit`, but only Creatures are "threat-bearing") | Owns the threat list; provides AddThreat/Update/GetCurrentVictim/Taunt/Fixate/Redirect API |
| `ThreatReference` | class | Edge in the list: owner ↔ victim, threat amount, online state, taunt state, temp modifier |
| `ThreatManager::Heap` | nested class | Min-heap-with-inverted-compare = max-heap by threat. Backed by `std::vector<ThreatReference*>` plus `std::push_heap` / `std::pop_heap` |
| `ThreatManager::ThreatListIterator` | nested class | Stable iterator over heap (caches snapshot at construct time to survive AddThreat during iteration) |
| `OnlineState` | `enum class : uint8` | `ONLINE` (top), `SUPPRESSED` (CC'd / immune / OOR), `OFFLINE` (target despawned / out-of-map) |
| `TauntState` | `enum class : uint8` | `DETAUNT` (debuff active, target ignored), `NONE` (default), `TAUNT` (forced victim) |
| `CompareThreatLessThan` | functor (operator()) | Heap comparator: order by `(OnlineState, TauntState, amount)` lexicographic |
| `ThreatManager::ThreatenedByMeList` | `std::unordered_set<ThreatManager*>` | Reverse index: every ThreatManager whose list *includes us as victim* |
| `RedirectThreatInfo` | struct | `{ targetGuid, pct }` keyed by spell id |
| `RedirectInfoMap` | `std::unordered_map<uint32, RedirectThreatInfo>` | Per-owner table: spell id → redirect target |
| `ThreatManager::ThreatModifier` | per-school float[7] | Modifier multiplier per `SpellSchool` from `SPELL_AURA_MOD_THREAT` auras |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `ThreatManager::Initialize()` | Sets `_ownerCanHaveThreatList` based on owner type (Creature non-pet) | `CanHaveThreatList(owner)` |
| `ThreatManager::Update(uint32 tdiff)` | Tick: decay, online-state re-eval, victim re-selection, broadcast queue | `UpdateVictim`, `EvaluateSuppressed`, `SendThreatListToClients` |
| `ThreatManager::AddThreat(Unit* target, float amount, SpellInfo const* spell, bool ignoreModifiers, bool ignoreRedirects)` | Insert/increment threat ref; auto-creates `CombatReference` via `CombatManager::SetInCombatWith` | `CalculateModifiedThreat`, `PutThreatListRef`, `CombatManager::SetInCombatWith`, `RegisterRedirectThreat` |
| `ThreatManager::ScaleThreat(Unit* target, float factor)` | Multiply current threat by factor (0 = reset to 0). Triggers heap re-sort | heap re-sort |
| `ThreatManager::ModifyThreatByPercent(Unit*, int32 pct)` | Wrapper: `ScaleThreat(0.01 * (100 + pct))` | id |
| `ThreatManager::MatchUnitThreatToHighestThreat(Unit* target)` | Set target's threat = current max(list). Called on Taunt apply to "level up" the taunter | heap peek + assign |
| `ThreatManager::TauntUpdate()` | Re-eval current victim across all taunt-state ThreatReferences. Called when `SPELL_AURA_MOD_TAUNT` apply/remove | iter + `UpdateVictim` |
| `ThreatManager::ResetAllThreat()` | Reset all amounts to zero, keep refs in list | iter |
| `ThreatManager::ClearAllThreat()` | Wipe entire list (evade / death) | iter + `SendThreatClear` |
| `ThreatManager::FixateTarget(Unit* target)` | Lock current victim until cleared; ignores threat-amount sort | direct field |
| `ThreatManager::ClearFixate()` | Release fixate lock | id |
| `ThreatManager::GetCurrentVictim()` | Top-of-heap honoring taunt + fixate | heap peek |
| `ThreatManager::GetLastVictim()` | Previous tick's current victim (for stickiness / target-lost detection) | cached field |
| `ThreatManager::GetThreat(Unit*, bool includeOffline)` | Lookup threat amount in list | hashmap find |
| `ThreatManager::GetSortedThreatList()` (const) | Snapshot copy of refs sorted by threat — safe for iteration during mutation | std::sort copy |
| `ThreatManager::GetModifiableThreatList()` | Mutable snapshot for callers who modify during iteration | std::sort copy |
| `ThreatManager::IsThreatenedBy(ObjectGuid, bool includeOffline)` | Lookup membership | hashmap find |
| `ThreatManager::ForwardThreatForAssistingMe(Unit* assistant, float baseThreat, SpellInfo* spell)` | Assistance: every ref in our list also gains `assistant→ref.victim` threat at `baseThreat` × modifier | iter + AddThreat on each |
| `ThreatManager::RemoveMeFromThreatLists()` | Reverse cleanup: walk `_threatenedByMe` and remove ourselves as victim from each | iter `_threatenedByMe` |
| `ThreatManager::RegisterRedirectThreat(uint32 spellId, ObjectGuid victim, uint32 pct)` | Register Misdirection / Tricks of the Trade redirect | redirect map |
| `ThreatManager::UnregisterRedirectThreat(uint32 spellId)` / `UnregisterRedirectThreat(uint32 spellId, ObjectGuid victim)` | Cleanup on aura expire | redirect map |
| `ThreatManager::EvaluateSuppressed(bool canExpire)` | Walk list, flip `ONLINE↔SUPPRESSED` based on victim aura immunity / range / CC | iter + state flip |
| `ThreatManager::PutThreatListRef(ObjectGuid, ThreatReference*)` | Internal insert with heap push | heap push |
| `ThreatManager::PurgeThreatListRef(ObjectGuid, bool sendToClients)` | Internal remove + heap fix | heap remove + `SendThreatRemove` |
| `ThreatManager::SendThreatListToClients(bool isFullUpdate)` | Broadcast to all observing players (`SMSG_THREAT_UPDATE` or `SMSG_HIGHEST_THREAT_UPDATE`) | packet write + send |
| `ThreatManager::ProcessThreatEvent(ObjectGuid victim, uint32 baseAmount, SpellInfo*)` | Public entry from `Spell::EffectThreat` | `AddThreat` |
| `ThreatReference::AddThreat(float amount)` | Increment + parent heap re-sort | heap fix |
| `ThreatReference::ScaleThreat(float factor)` | Multiply + heap re-sort | id |
| `ThreatReference::ClearThreat()` | Set to 0 + heap re-sort | id |
| `ThreatReference::SetTaunt(TauntState)` / `GetTauntState()` | Mutate taunt state + parent heap re-sort | id |
| `ThreatReference::IsOnline()` / `IsOffline()` / `IsSuppressed()` | State accessors | — |
| `ThreatReference::UpdateOnlineState()` | Re-evaluate based on current target conditions | aura/range checks |
| `ThreatReference::ShouldBeOffline()` (private) | Distance check + isAlive check + map check | — |
| `ThreatReference::ShouldBeSuppressed()` (private) | Aura-CC check + immunity check | aura iter |

---

## 5. Module dependencies

**Depends on:**
- `entities-unit.md` — `Unit::IsAlive`, `Unit::IsValidAttackTarget`, `GetMap()`, `GetDistance`, `GetAuraEffectsByType` (for taunt/threat-mod/CC), `Creature::CanHaveThreatList`.
- `spells-effects.md` — `SPELL_AURA_MOD_TAUNT` (TauntUpdate driver), `SPELL_AURA_MOD_THREAT` (per-school multiplier table), `SpellInfo->ThreatMultiplier`, `SpellInfo->StackThreatModifier`, `SpellInfo->SchoolMask` (for school-modifier lookup), redirect spells (Misdirection, Tricks of the Trade, Vigilance).
- `combat-manager.md` — every `AddThreat` auto-calls `CombatManager::SetInCombatWith` to ensure combat state is established.
- `ai.md` — `CreatureAI::JustEnteredCombat` fires when first ref added; `CreatureAI::AttackStart` reads `GetCurrentVictim`; `CreatureAI::EnterEvadeMode` calls `ClearAllThreat`.
- `pets.md` — pet damage threat is attributed to owner (pet damage → owner threat ref on target).
- `entities.md` (Creature) — `Creature::Update` calls `_threatManager.Update(diff)`.
- `maps.md` / `grids.md` — visibility / range checks for `EvaluateSuppressed` (target out-of-grid → SUPPRESSED).
- `combat-dealdamage.md` — every `DealDamage` ends with an implicit `ThreatManager::AddThreat(attacker, damage_value × spell_threat_coeff)`.

**Depended on by:**
- `combat.md` — parent module summary.
- `ai.md` — every `SelectTarget(SELECT_TARGET_TOPAGGRO)` reads `GetSortedThreatList` or `GetCurrentVictim`.
- `entities.md` (Creature) — `Update` reads `GetCurrentVictim`; cache for `Creature::AttackedBy`, `Creature::SetCurrentVictim`.
- `combat-dealdamage.md` — kill cascade may call `RemoveMeFromThreatLists()` to clean reverse refs.

---

## 6. SQL / DB queries (if any)

ThreatManager itself emits no SQL. Inputs read at unit-load time:

| Statement / Source | Purpose | DB |
|---|---|---|
| `creature_template.faction` | Faction-friendly check (assistance threat) | world |
| `creature_template.unit_flags` (UNIT_FLAG_NON_ATTACKABLE) | `CanHaveThreatList` filter | world |
| `creature_threat_modifier` (custom override table, rarely populated) | Per-creature threat multiplier | world |
| `spell_dbc.ThreatMultiplier` (DB2 SpellMisc) | Per-spell threat coefficient | hotfixes |
| `spell_threat` (custom spell-script overrides) | Server-side threat coefficient overrides | world |

DBC/DB2 stores consumed:

| Store | What it loads | Read by |
|---|---|---|
| `SpellStore` (via `SpellInfo`) | School mask, threat coefficient, attributes | `CalculateModifiedThreat`, `AddThreat` |
| `SpellEffectStore` | Threat-effect amounts (for `SPELL_EFFECT_THREAT` direct-threat spells) | `Spell::EffectThreat` |
| `FactionStore` | Faction relations for assistance | `ForwardThreatForAssistingMe` |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_THREAT_UPDATE` (0x2A30) | S→C | `ThreatManager::SendThreatUpdate` — incremental update, list of (guid, threat) pairs for one creature's threat list. Sent to every Player on group/raid that has the creature visible. |
| `SMSG_HIGHEST_THREAT_UPDATE` | S→C | `ThreatManager::SendHighestThreatUpdate` — sent when the top-of-heap changes (new tank, "sticky-target" hand-off) |
| `SMSG_THREAT_REMOVE` | S→C | `ThreatManager::SendThreatRemove` — single-target removal (target died, despawned, fixate-cleared) |
| `SMSG_THREAT_CLEAR` | S→C | `ThreatManager::SendThreatClear` — wipe (creature evade, creature death) |
| `SMSG_AI_REACTION` (0x26B5) | S→C | First-threat-add fires `SMSG_AI_REACTION` (mob aggro'd) — actually emitted by `CombatManager::SetInCombatWith` but caused by ThreatManager |

There are no CMSG opcodes — threat is pure server-side state inferred from damage events.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-combat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-combat/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-ai/src/lib.rs` | `file` | 1 | 346 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/combat.rs` | `file` | 1 | 152 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-combat/src/lib.rs` — **0 lines** (empty crate; see §13). No `ThreatManager`, no `ThreatReference`, no heap, no taunt/fixate state, no redirect table.
- `crates/wow-ai/src/lib.rs` — `CreatureState::InCombat` boolean; `enter_combat(player_guid)` stores a single attacker GUID. No threat amount, no list, no sort.
- `crates/wow-world/src/handlers/combat.rs` — does not interact with any threat system.

**What's implemented:**
- `creature.attacker = Some(player_guid)` — single-attacker model. Equivalent to a 1-entry threat list with no amount and no tie-break.
- AI loop reads `creature.attacker` to know who to swing at.

**What's missing vs C++:**
- **`ThreatManager` struct** — does not exist.
- **`ThreatReference` struct** — does not exist.
- **Heap structure** — no `BinaryHeap`, no sort, no comparator.
- **`OnlineState` enum** — does not exist. Cannot distinguish ONLINE / SUPPRESSED / OFFLINE.
- **`TauntState` enum** — does not exist. Taunt aura is a no-op.
- **`AddThreat`** — no API. Damage events do not register threat.
- **`ScaleThreat` / `ModifyThreatByPercent`** — no API. Vengeance, Soothe Animal, etc. cannot reduce threat.
- **`MatchUnitThreatToHighestThreat`** — no API. Cannot honor taunt's "ramp to top" semantic.
- **`TauntUpdate`** — no aura handler integration.
- **`FixateTarget` / `ClearFixate`** — no API. Boss mechanics that rely on fixate (Sapphiron's Frost Breath, Heigan's Eruption, etc.) cannot be scripted correctly.
- **`GetCurrentVictim` / `GetLastVictim`** — no API. AI uses `creature.attacker` directly which is single-target only.
- **`ForwardThreatForAssistingMe`** — no API. NPC assistance (faction-friendly nearby creatures aggroing on heal/help) does not work.
- **`RemoveMeFromThreatLists`** — no reverse cleanup. Player logout / death does not properly evict from creature threat lists.
- **`RegisterRedirectThreat` / `UnregisterRedirectThreat`** — no API. Misdirection (hunter), Tricks of the Trade (rogue), Vigilance (warrior) all silent.
- **`EvaluateSuppressed`** — no API. Iceblock / Vanish / Feign Death do not flip threat state to SUPPRESSED.
- **`SMSG_THREAT_UPDATE` / `THREAT_REMOVE` / `THREAT_CLEAR` / `HIGHEST_THREAT_UPDATE`** — packet writers do not exist. Client threat meter (Omen, base UI threat bar) shows nothing.
- **Per-school threat modifiers** — no `[f32; 7]` table for `MOD_THREAT` aura summation. Defensive Stance's +30% physical threat is silent.
- **`ThreatModifier` per-spell table** — no integration with `SpellInfo::ThreatMultiplier`. Threat-bombs (Mocking Blow, Taunt) and threat-reducers (Soothing Kiss, Tranquilizing Shot) do nothing.
- **Pet threat attribution** — pet damage does not credit owner's threat ref on target.
- **Combat-ref auto-creation** — adding threat does not auto-create `CombatReference`.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `creature.attacker: Option<ObjectGuid>` is a single-attacker assumption — cannot model the "two players DPSing the same boss" case at all (whoever attacked second overwrites the first).
- The legacy `WorldSession.in_combat` boolean is a per-player flag, not a per-creature threat ref — combat exit logic cannot work correctly because there's no ref-counted "combat with whom" state.
- AI selects victim by reading `creature.attacker` → no way to pick the highest-threat target → tank/DPS mechanic is effectively random.
- Without `RemoveMeFromThreatLists`, a player who logs out mid-fight stays in `creature.attacker` until creature evades — looks like the creature is fighting a ghost.

**Tests existing:**
- 0 tests for ThreatManager.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#COMBAT_THREAT.WBS.001** Partir y cerrar la migracion auditada de `game/Combat/ThreatManager.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.cpp`
  Rust target: `crates/wow-combat`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 913 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#COMBAT_THREAT.WBS.002** Cerrar la migracion auditada de `game/Combat/ThreatManager.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.h`
  Rust target: `crates/wow-combat`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for `MIGRATION_ROADMAP.md` §5 reference.
Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h).

- [ ] **#COMBAT-THR.1** Define `enum OnlineState { Online, Suppressed, Offline }` with `Online > Suppressed > Offline` ordering for tie-break. (L)
- [ ] **#COMBAT-THR.2** Define `enum TauntState { Detaunt, None, Taunt }` with `Detaunt < None < Taunt` ordering. (L)
- [ ] **#COMBAT-THR.3** Define `struct ThreatReference { owner: ObjectGuid, victim: ObjectGuid, amount: f32, online_state: OnlineState, taunt_state: TauntState, temp_modifier: i32 }`. (L)
- [ ] **#COMBAT-THR.4** Define `struct HeapEntry { neg_amount: ordered_float, online_state, taunt_state, victim: ObjectGuid }` + custom `Ord` impl: order by `online_state desc`, `taunt_state desc`, `neg_amount asc` (max-heap by amount). (M)
- [ ] **#COMBAT-THR.5** Define `struct ThreatManager { owner: ObjectGuid, refs: HashMap<ObjectGuid, ThreatReference>, heap: BinaryHeap<HeapEntry>, threatened_by_me: HashSet<ObjectGuid>, current_victim: Option<ObjectGuid>, last_victim: Option<ObjectGuid>, fixate: Option<ObjectGuid>, redirects: HashMap<u32, RedirectThreatInfo>, modifier_by_school: [f32; 7], update_timer_ms: u32, can_have_threat_list: bool }`. (M)
- [ ] **#COMBAT-THR.6** `fn initialize(&mut self, owner: &Unit)` — set `can_have_threat_list` from owner type (Creature non-pet non-totem). (L)
- [ ] **#COMBAT-THR.7** `fn add_threat(&mut self, world, target_guid, amount, spell: Option<&SpellInfo>, ignore_modifiers, ignore_redirects)` — apply per-school modifier × spell coefficient × redirect, insert/update ref, push to heap, auto-create CombatReference via `CombatManager::set_in_combat_with`. (H)
- [ ] **#COMBAT-THR.8** `fn calculate_modified_threat(&self, base_amount, spell, target) -> f32` — apply `modifier_by_school[school]` × `spell.threat_multiplier` × `target.threat_taken_modifier`. (M)
- [ ] **#COMBAT-THR.9** `fn scale_threat(&mut self, target, factor)` + `fn modify_threat_by_percent(&mut self, target, pct)` — multiply existing amount, re-heapify. (M)
- [ ] **#COMBAT-THR.10** `fn match_unit_threat_to_highest_threat(&mut self, target)` — set target.amount = current heap top.amount. Called from `TauntUpdate`. (M)
- [ ] **#COMBAT-THR.11** `fn taunt_update(&mut self, world)` — re-evaluate `taunt_state` for every ref by checking taunt aura presence; call `match_unit_threat_to_highest_threat` for newly-tainted; re-heapify. (H)
- [ ] **#COMBAT-THR.12** `fn fixate_target(&mut self, target)` + `fn clear_fixate(&mut self)` — direct field mutation; `get_current_victim` honors fixate first. (L)
- [ ] **#COMBAT-THR.13** `fn get_current_victim(&self) -> Option<ObjectGuid>` — returns `fixate` if set, else `heap.peek().victim` filtered to ONLINE state. (M)
- [ ] **#COMBAT-THR.14** `fn evaluate_suppressed(&mut self, world, can_expire)` — walk refs; flip ONLINE↔SUPPRESSED based on victim's aura immunity / out-of-range / CC. (H)
- [ ] **#COMBAT-THR.15** `fn update(&mut self, world, diff_ms)` — tick: decay temp modifiers, run `evaluate_suppressed`, run `update_victim` (re-pick top-of-heap), run `send_threat_list_to_clients` if delta. (H)
- [ ] **#COMBAT-THR.16** `fn forward_threat_for_assisting_me(&self, world, assistant, base_threat, spell)` — iterate refs, for each victim call `add_threat(assistant→victim, base_threat × modifier)`. (M)
- [ ] **#COMBAT-THR.17** `fn remove_me_from_threat_lists(&mut self, world)` — walk `threatened_by_me`, for each owner_guid call `world.creature(owner_guid).threat_manager.purge_threat_list_ref(self.owner)`. (M)
- [ ] **#COMBAT-THR.18** `fn register_redirect_threat(&mut self, spell_id, victim, pct)` + `fn unregister_redirect_threat(&mut self, spell_id, victim)` — used by Misdirection, Tricks of the Trade, Vigilance. (M)
- [ ] **#COMBAT-THR.19** `fn reset_all_threat(&mut self)` (set amounts to 0, keep refs) + `fn clear_all_threat(&mut self)` (wipe + send `SMSG_THREAT_CLEAR`). (M)
- [ ] **#COMBAT-THR.20** `fn get_threat(&self, target, include_offline) -> f32`, `fn get_sorted_threat_list(&self) -> Vec<ThreatReference>`, `fn get_modifiable_threat_list(&self) -> Vec<ThreatReference>`. (L)
- [ ] **#COMBAT-THR.21** `fn process_threat_event(&mut self, world, victim, base_amount, spell)` — public entry from `Spell::EffectThreat`. (L)
- [ ] **#COMBAT-THR.22** SMSG_THREAT_UPDATE writer — pack `(target_guid: ObjectGuid, count: u32, [(victim_guid: PackedGuid, amount: u32); count])`. (M)
- [ ] **#COMBAT-THR.23** SMSG_HIGHEST_THREAT_UPDATE writer — `(target_guid, new_top_guid, count, list...)`. (M)
- [ ] **#COMBAT-THR.24** SMSG_THREAT_REMOVE writer + SMSG_THREAT_CLEAR writer. (L)
- [ ] **#COMBAT-THR.25** `fn send_threat_list_to_clients(&self, world, is_full_update)` — broadcast to all observing Players on grid. (M)
- [ ] **#COMBAT-THR.26** Aura handler integration: `SPELL_AURA_MOD_TAUNT` apply/remove → call `taunt_update`; `SPELL_AURA_MOD_THREAT` apply/remove → mutate `modifier_by_school[mask]`. (M)
- [ ] **#COMBAT-THR.27** Pet threat attribution: `add_threat` with `attacker` flag set forwards to owner-side ThreatManager. (M)
- [ ] **#COMBAT-THR.28** AI integration: `Creature::Update` calls `threat_manager.update(diff)`; AI `select_target(TopAggro)` reads `get_current_victim`. (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#COMBAT_THREAT.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 1234 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.h`. Rust target: `crates/wow-combat`. | `cargo test -p wow-combat` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#COMBAT_THREAT.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 1234 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.h`. Rust target: `crates/wow-combat`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#COMBAT_THREAT.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 1234 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.h`. Rust target: `crates/wow-combat`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#COMBAT_THREAT.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 1234 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.h`. Rust target: `crates/wow-combat`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `add_threat(B, 100)` + `add_threat(C, 50)` → `get_current_victim() == Some(B)`.
- [ ] Test: `add_threat(B, 100)` + `add_threat(C, 200)` → `get_current_victim() == Some(C)`.
- [ ] Test: `scale_threat(B, 0.0)` after `add_threat(B, 100)` → `get_threat(B) == 0`; `get_current_victim()` falls through to next-highest.
- [ ] Test: `modify_threat_by_percent(B, -50)` after `add_threat(B, 100)` → `get_threat(B) == 50`.
- [ ] Test: `match_unit_threat_to_highest_threat(C)` when B has 200 threat → `get_threat(C) == 200`.
- [ ] Test: Taunt aura on B (`taunt_state = Taunt`) when C has 500 threat and B has 100 → `get_current_victim() == Some(B)` (taunt overrides amount).
- [ ] Test: Taunt aura expires → `taunt_update` resets `taunt_state = None` → highest-amount target resumes (matches `match_unit_threat_to_highest_threat` outcome).
- [ ] Test: `fixate_target(B)` when C has more threat → `get_current_victim() == Some(B)` indefinitely; `clear_fixate()` releases.
- [ ] Test: `online_state` flow — Iceblock aura applied on B → `evaluate_suppressed` flips B to `Suppressed`; B not selected as victim. Aura expires → flips back to `Online`.
- [ ] Test: B logs out → `evaluate_suppressed(can_expire=false)` flips B to `Offline`; never selected.
- [ ] Test: `add_threat` automatically calls `CombatManager::set_in_combat_with(owner, target)` (regla "threat ⇒ combat").
- [ ] Test: `clear_all_threat()` empties heap, sends `SMSG_THREAT_CLEAR` to all observing players, fires `EnterEvadeMode`.
- [ ] Test: Misdirection — `register_redirect_threat(spell_id=34477, victim=hunter_target, pct=100)` + `add_threat(hunter, 1000)` → 100% routed to `hunter_target`; hunter's own ref unchanged.
- [ ] Test: Tricks of the Trade — `register_redirect_threat(spell_id=57934, victim=tank, pct=100)` for 6 seconds, then expires.
- [ ] Test: Vigilance — `register_redirect_threat(spell_id=50720, victim=warrior, pct=100)` permanent (until aura removed).
- [ ] Test: `forward_threat_for_assisting_me` — friendly NPC heal on assisted target → assistant gains threat on every ref of helped creature.
- [ ] Test: `remove_me_from_threat_lists` — player despawns mid-fight → cleared from all threatening creatures' lists.
- [ ] Test: Pet threat attribution — pet hits target → owner gains threat ref on target (zero on pet→target).
- [ ] Test: Per-school modifier — `modifier_by_school[Physical] = 1.30` (Defensive Stance) → physical threat multiplied by 1.30; holy/fire/etc. unchanged.
- [ ] Test: Heap iteration during mutation — `get_sorted_threat_list()` returns snapshot copy; concurrent `add_threat` does not invalidate.
- [ ] Test: SMSG_THREAT_UPDATE byte-exact vs captured client packet (3-target list).
- [ ] Test: SMSG_HIGHEST_THREAT_UPDATE fires when top-of-heap changes; does NOT fire when sub-leader changes.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 1234 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.h` | `crates/wow-combat/` (empty — see §13) \| ❌ not started — entire subsystem absent. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#COMBAT_THREAT.DIV.001` | `crates/wow-combat` (`exists_empty`, 0 Rust lines) | 2 C++ files / 1234 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#COMBAT_THREAT.DIV.002` | `crates/wow-combat/src/lib.rs` (`exists_empty`, 0 Rust lines) | 2 C++ files / 1234 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |

<!-- REFINE.023:END known-divergences -->

- **Tie-break order is canonical**: `OnlineState` first (Online > Suppressed > Offline), then `TauntState` (Taunt > None > Detaunt), then `amount` (descending). Reordering breaks taunt — a taunt on a low-threat target would lose to a high-threat target without the aura, which is the opposite of game expectation.
- **`MatchUnitThreatToHighestThreat` is the taunt's "level up" mechanic**: applying taunt does NOT just set `taunt_state = Taunt` — it also bumps the target's threat amount to match the current top-of-heap so that when the taunt expires, the target stays the tank. Implementing taunt as "set amount = infinity" breaks the post-expire transition.
- **`ScaleThreat(target, 0.0)` is the standard "reset" recipe**: NOT `RemoveThreat` — the ref stays in the list with amount=0 so that the target still counts as in-combat. `ClearAllThreat` is for evade only.
- **Heap iteration during AddThreat is invariant-breaking**: C++ uses `GetSortedThreatList()` (copy-on-snapshot) for iteration when the loop body may call `AddThreat`. In Rust this maps cleanly to `.clone()` / `.iter()` on the snapshot. Don't try to iterate `&self.heap` while `&mut self.add_threat` — borrow check will tell you anyway.
- **`ThreatenedByMe` reverse index is essential for cleanup**: when a Unit dies / despawns / logs out, you MUST walk `_threatenedByMe` and call `purge_threat_list_ref` on each owner. Skipping = "ghost target" bug where creature continues to swing at empty space.
- **`CanHaveThreatList` filter**: only Creatures (not Pets, not Totems, not Vehicles, not GM-mode players, not non-attackable NPCs) get a real threat list. Pets ride on owner's CombatManager but contribute threat to `owner.threat_manager` on every pet hit. Totems aggro using a simpler "highest damage in last 5s" heuristic.
- **Per-school modifier table**: `modifier_by_school: [f32; 7]` — auras with `SPELL_AURA_MOD_THREAT` and a school mask add to *all* schools they cover. Defensive Stance has school mask Physical|Holy|Nature → bumps three slots simultaneously. Removing the aura must subtract from the same slots, not zero them (multiple stacking auras possible).
- **PvP combat does NOT engage threat**: PvP combat uses `CombatManager.PvPCombatReference` only. ThreatManager is creature-only. Adding threat between two players is a no-op.
- **Misdirection/Tricks redirect % is multiplicative against modifier**: `final_threat = base_threat × (1 - redirect_pct/100)` for source, `redirect_target_threat = base_threat × (redirect_pct/100) × source.modifier` (some servers get the modifier-application-order wrong → Tricks gives more threat to the tank than the rogue would have generated, which is actually correct per-Blizzard 3.4.3).
- **`EvaluateSuppressed` runs every tick**: which is expensive on a heap of 40 raid members. C++ caches "next eval timestamp" per ref to skip evals when nothing changed. Mirror this in Rust to avoid O(n) every frame.
- **`SMSG_THREAT_UPDATE` is broadcast to observers, not just party**: every Player on the same map who has the creature in visibility range gets the packet. Filter by visibility cell membership.
- **`SMSG_HIGHEST_THREAT_UPDATE` fires when top-of-heap changes, NOT every tick**: spamming it every frame breaks Omen ThreatBar smoothing.
- **`temp_modifier` field**: short-duration threat amp/reduce auras (e.g. "Shield Slam: deals X threat for 10s next attack only") use temp_modifier rather than mutating amount directly. Decays each tick.
- **Vigilance is a 100% redirect that ALSO copies threat retroactively**: when Vigilance is applied, the warrior's threat on every ref in protector's list is cloned to the warrior's threat manager. Implementation pitfall — easy to forget the retroactive copy.
- **`ForwardThreatForAssistingMe` is the "heal-aggro" engine**: when a friendly NPC heals the creature owner, the heal triggers a `ForwardThreat` event that adds threat from the healer to every entry in owner's list. This is how heal-aggro works in PvE — without it, healers don't pull when off-tank dies.
- **Heap re-sort cost**: every `add_threat` is O(log n). For a 40-target boss raid, AddThreat called every spell hit = ~40 ops/sec × log40 = ~6 ops. Fine. The expensive op is `evaluate_suppressed` which is O(n × aura_check).

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class ThreatManager` | `struct ThreatManager { owner_guid, refs: HashMap<ObjectGuid, ThreatReference>, heap: BinaryHeap<HeapEntry>, threatened_by_me: HashSet<ObjectGuid>, current_victim, last_victim, fixate: Option<ObjectGuid>, redirects: HashMap<u32, RedirectThreatInfo>, modifier_by_school: [f32; 7], update_timer_ms: u32, can_have_threat_list: bool }` | Composition in Creature struct (no inheritance) |
| `class ThreatReference` | `struct ThreatReference { owner: ObjectGuid, victim: ObjectGuid, amount: f32, online_state: OnlineState, taunt_state: TauntState, temp_modifier: i32 }` | — |
| `struct ThreatManager::Heap` (private) | `BinaryHeap<HeapEntry>` from std | Custom Ord on HeapEntry inverts amount for max-heap |
| `enum class OnlineState` | `enum OnlineState { Online, Suppressed, Offline }` | Order: `Online > Suppressed > Offline` for tie-break (use derive Ord with reverse if needed) |
| `enum class TauntState` | `enum TauntState { Detaunt, None, Taunt }` | Order: `Taunt > None > Detaunt` |
| `struct CompareThreatLessThan` (functor) | `impl Ord for HeapEntry` | Lexicographic on `(online_state, taunt_state, amount)` |
| `std::unordered_set<ThreatManager*> ThreatenedByMeList` | `HashSet<ObjectGuid>` | Identifier, not pointer |
| `std::unordered_map<uint32, RedirectThreatInfo> RedirectInfoMap` | `HashMap<u32, RedirectThreatInfo>` | — |
| `struct RedirectThreatInfo` | `struct RedirectThreatInfo { target: ObjectGuid, pct: u32 }` | — |
| `void Update(uint32 tdiff)` | `fn update(&mut self, world: &mut WorldState, diff_ms: u32)` | — |
| `void AddThreat(Unit*, float, SpellInfo const*, bool, bool)` | `fn add_threat(&mut self, world, target_guid: ObjectGuid, amount: f32, spell: Option<&SpellInfo>, ignore_modifiers: bool, ignore_redirects: bool)` | World ref needed for redirect lookup + auto-CombatRef |
| `Unit* GetCurrentVictim()` | `fn get_current_victim(&self) -> Option<ObjectGuid>` | Return GUID, caller dereferences via world |
| `std::vector<ThreatReference*> GetSortedThreatList()` | `fn get_sorted_threat_list(&self) -> Vec<ThreatReference>` | Snapshot copy |
| `SendThreatListToClients(bool)` | `fn send_threat_list_to_clients(&self, world, is_full: bool)` | World ref to enumerate observers |

---

*Template version: 1.0 (2026-05-01).* Last updated: 2026-05-01.

---

## 13. Audit (2026-05-01)

**Scope.** Audited the C++ ThreatManager at `/home/server/woltk-trinity-legacy/src/server/game/Combat/ThreatManager.h` (321 lines, ThreatReference + ThreatManager class declarations + OnlineState / TauntState enums + CompareThreatLessThan functor) and `ThreatManager.cpp` (913 lines, full heap / redirect / taunt / fixate / evaluate-suppressed / send-to-clients implementation), against the Rust workspace at `/home/server/rustycore/crates/wow-combat/`.

**Empty-crate finding — CONFIRMED.** `/home/server/rustycore/crates/wow-combat/src/lib.rs` is **0 lines**. None of the ThreatManager subsystem exists in any form: no `ThreatManager` struct, no `ThreatReference`, no `BinaryHeap` for sorting, no `OnlineState` enum, no `TauntState` enum, no `CompareThreatLessThan` equivalent, no `RedirectInfoMap`, no `[f32; 7]` per-school modifier table, no SMSG_THREAT_* packet writers.

**Surrogate path.** What runs today is one field in `crates/wow-ai/src/lib.rs`: `creature.attacker: Option<ObjectGuid>` — a single-attacker model holding one GUID with no amount, no online state, no taunt state, no tie-break. `enter_combat(player_guid)` overwrites unconditionally. `reset_combat()` clears unconditionally. The AI loop reads `creature.attacker` to pick the victim — there is no way to model "tank holds aggro because they have 50000 threat while DPS at 30000 stays second on threat list".

**Kill pipeline (threat-side) is broken.** When a Unit dies, C++ chains:
1. `RemoveMeFromThreatLists()` walks `_threatenedByMe` reverse index, calling `PurgeThreatListRef(self.owner)` on every threatening creature.
2. `ClearAllThreat()` is invoked on the victim's own ThreatManager (if creature) to wipe its list.
3. `SMSG_THREAT_REMOVE` is broadcast to all observers per-creature.

**In RustyCore, none of this runs.** A dead player's GUID stays in `creature.attacker` until the creature evades by some other path. A dead creature's `attacker` field is never read again, but no `SMSG_THREAT_REMOVE` ever goes out. Client threat UI is permanently dark — Omen and the base WoW threat meter receive zero packets.

**Redirect / taunt mechanics.** Misdirection (hunter spell 34477), Tricks of the Trade (rogue spell 57934), Vigilance (warrior spell 50720) — all three depend on `RegisterRedirectThreat` and the redirect map. **Zero are implemented.** Taunt aura (`SPELL_AURA_MOD_TAUNT`) has no handler integration with a TauntUpdate function — applying taunt does not change victim selection. PvP trinkets that grant mechanic immunity rely on `EvaluateSuppressed` flipping `OnlineState` — also unimplemented.

**Worst hidden divergence.** Per-school threat modifiers (`SPELL_AURA_MOD_THREAT` aura with school mask) drive class identities like Defensive Stance (+30% physical/holy/nature for warriors) and Salvation (-30% all schools for blessed targets). Without the `[f32; 7]` table and its per-school addition/subtraction logic on aura apply/remove, **tank vs DPS threat ratio is identical to a baseline player swinging a stick**. Tanking is effectively impossible. PvE encounters with multi-target threat juggling (Naxxramas Four Horsemen, Black Knight) are unscriptable.

**Cross-references.** See `combat.md` §8 / §13 for the parent-doc audit. See `combat-dealdamage.md` §13 for the damage-pipeline companion gap (every `DealDamage` should call `AddThreat` — neither exists). See `combat-manager.md` §13 for the CombatManager companion gap (every `AddThreat` should auto-create a `CombatReference` via `SetInCombatWith` — neither exists). See `ai.md` (when authored) for the consumer-side gap (`CreatureAI::SelectTarget` reads `GetCurrentVictim` which doesn't exist).
