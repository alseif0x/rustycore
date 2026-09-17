# RustyCore — Master port and delivery plan

**Reconciled 2026-09-15 under #584 / #787 / #748 / #63 / [master index #49](https://github.com/alseif0x/rustycore/issues/49), with PR #955, docs-only PR #954, #953, #950, #901, #902, #904, #906, #907, #909, #911, #913, #915, #916, #917, #919, #921, #922, #923, #924, #925, #926, #927, #929, #931, #933, #935, #936, #938, #940, #942, #944, #946, #947 and #948 integrated.**
Source baseline for this reconciliation: `3.4.3` at
`581eb19e8602d02ff593f89997fc495eb0f36e34` (PR #955, following docs-only PR #954 and PR #953/#948/#935/#933/#931/#929/#927/#926/#925/#924, PR #922/#921, PR #919, PR #917, PR #916, PR #915, #913, #909/#907/#906/#904/#902/#901/#899/#897/#895/#893/#891/#889/#887/#885/#876/#873/#871/#869/#866/#864/#862/#860/#859/#855/#854/#853 and #851; the earlier `179fd5d4`, `93fa95a9`, `6f42782f`, `995cd77f`, `cc055998`, `4e3ad8f0`, `1143ed41`, `a9623787`, `276e3981`, `d934451a`, `7bb9a911`, `16303cc7`, `62c1369f`, `db125076`, `a3e97063`, `a96ee548`, `76a05081`,
`886e13ad`,
`5d8c079a` and `ebc3b3eb` references remain historical evidence for the issue inventory).
Initial inventory: **46 open issues**, all given a disposition below; #748 is this
bounded planning delivery. Administrative consolidation does not count as implementation.

Current exact architecture inventory after PR #948: 649 WorldSession fields (219 production, 430 test fixtures). The locked-encounter authority, Player `m_seer` visibility projection, canonical Pet visibility CREATE discovery and unified directed object DESTROY publication for Creature/Pet/Corpse are integrated; no unresolved production WorldSession residual remains in this audited slice. PR #931 also ratifies the P4 physical split of loaded-grid creature tests: the production facade is 797 lines, with 28 regressions in two responsibility-scoped modules plus a shared fixture facade. PR #933 splits the 2,643-line game-event runtime into seven responsibility modules behind a compact facade; all 181 game-event regressions remain registered. Session retains only a one-field publication fence for the explicit FAR_SIGHT clear packet. PR #935 also splits the spawn loader catalog models into two private responsibility modules while preserving startup ownership and public paths. PR #942 continues the same P4 boundary by moving PoolMgr/member validation and spawn-group startup loading into a private child; PR #944 moves the object spawn-row and linked-respawn family; PR #946 moves waypoint and formation startup metadata without changing the 3,427-line aggregate ceiling. PR #948 moves the linked-respawn row adapter, GameEvent world prefix/suffix adapters and spawn-group member adapter into the existing object, GameEvent and pool children; the composition facade is 730 lines, the object child 661, the pool child 358 and the GameEvent loader child 663, while the aggregate remains exactly 3,427 lines.

The target remains **full functional parity with the TrinityCore-derived WoW 3.4.3
server**, with the approved native/Wasm module product. A playable milestone is an
intermediate acceptance point, not a smaller replacement target.

**Fresh Creature runtime audit, 2026-09-15:** the remaining #584 C0/C3 boundary is
recorded in [creature-runtime-audit.md](../architecture/creature-runtime-audit.md).
Production still has a mutable `WorldCreature` runtime owner and a canonical
`wow-map::Map::entity_world` representation used by visibility and target
consumers. `RuntimeTickOwner::GlobalLegacy` plus the canonical `ExternalRuntime`
no-op prevents a second timer writer, but the cloned sync path and the
`Creature::runtime_update_plan` seam do not constitute a complete C++
`Map::Update`/`Creature::Update` consumer. The structural **C3.1 — one
map-owned Creature runtime outcome boundary** is implemented by commit
`e2ca3df9`: production captures one typed tick input, routes the six existing
phases through an explicit boundary, records authority/incarnation and
publication/DB counts, and has positive plus stale-incarnation regressions.
This does not retire the legacy owner or claim full Creature parity. The next
implementation vertical is selected by dependency from #29/#31, then
#32/#33/#34, with each consumer required to use the C3.1 envelope and no
speculative AI or crate split.

## 1. Direction from here

**#61 narrow values-update negative spell field — 2026-09-17, implementation
`a7c53527`:** `PlayerStatChanges` gains `mod_damage_done_neg[7]` and the narrow
values writer sets the C++ mask bits 284-290 and emits the per-school negative
value, closing the last gap of the spell-field publication; `ModDamageDonePercent`
(291-297) remains outside this runtime writer. wow-packet --lib 742/0,
wow-world --lib 3926/0/1; no live DB/restart/relogin QA.

**#61 spell damage and healing field wire publication — 2026-09-17,
implementation `9847962e`:** `PlayerCombatStats`, `PlayerCreateData` and
`PlayerStatChanges` replace the scalar `spell_power` with
`mod_damage_done_pos[7]`, `mod_damage_done_neg[7]` (create data) and
`mod_healing_done_pos`, and the create/values writers emit them from those
arrays; the `ModDamageDoneNeg` bits of the narrow values update stay unset, as
before. wow-packet --lib 742/0, wow-world --lib 3926/0/1; no live
DB/restart/relogin QA.

**#61 spell damage and healing done producers — 2026-09-17, implementation
`92f0f550`:** `Player::UpdateSpellDamageAndHealingBonus` (`StatSystem.cpp:171-197`)
is produced from the C++ `Unit::SpellBaseDamageBonusDone`/`SpellBaseHealingBonusDone`
terms (`Unit.cpp:6860-6890`, `7282-7315`, auras 13/135/174/175) plus the
`SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` (366) pass, so
`SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` (404) now reads the real
`min(ModHealingDonePos, ModDamageDonePos[HOLY..MAX])`. The canonical Player
publishes `mod_damage_done_pos`/`neg` and `mod_healing_done_pos`; the packet
adapters still send the scalar `spell_power`, which is the next publication
unit. `wow-data --lib` 751/0, `wow-entities --lib` 940/0, `wow-world --lib`
3926/0/1; no live DB/restart/relogin QA.

**#61 override attack power by spell power — 2026-09-17, implementation
`8ee397c5`:** `Player::UpdateAttackPowerAndDamage` (`StatSystem.cpp:333-403`)
now applies the `SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT`
(`SpellAuraDefines.h:499`) branch: while the aura is active, both attack mods
replace the strength/agility/level base with
`CalculatePct(min(ModHealingDonePos, ModDamageDonePos[HOLY..MAX]), percent)`
(`StatSystem.cpp:341-379`), with the amount accumulated like
`HandleOverrideAttackPowerBySpellPower` (`SpellAuraEffects.cpp:3785-3796`). The
existing threat and `CalculateMinMaxDamage` consumers are reused unchanged;
`wow-data --lib` 750/0 and `wow-world --lib` 3925/0/1. The
`SPELL_AURA_MOD_DAMAGE_DONE`/`MOD_HEALING_DONE` producers remain the next gate,
and there is no live DB/restart/relogin QA.

**Seven stale `wow-world --lib` expectations — 2026-09-17, implementation
`2534b97e`:** the seven scenarios left red by PR #1003 were reproduced on clean
base `6b46be23` and corrected as harness/expectation defects, not production
changes: the void-storage fixture now seeds the learned one-handed-sword
proficiency required by `CollectionMgr::CanAddAppearance`
(`CollectionMgr.cpp:649-726`; first-login `playercreateinfo_cast_spell` →
`SPELL_EFFECT_PROFICIENCY`, `CharacterHandler.cpp:1284-1288`,
`SpellEffects.cpp:1785-1804`); the reputation-retention fixture publishes the
loaded identity before the rows load (`Player::LoadFromDB`,
`CharacterHandler.cpp:1070`); the logout snapshot, durable creature rail, fall
death and two quest-giver queries match the owned C++ order and canonical level
readers. `wow-world --lib` is **3924 passed/0 failed/1 ignored**. No gameplay
parity and no live DB/restart/relogin QA.

**Represented quest-share party fixture identity — 2026-09-17, implementation
`05d0d5e4`:** the party fixture adopted an identity-less canonical Player, so the
registry level snapshot was overwritten with 0 and 21 `push_quest_to_party`
scenarios failed on `SatisfyQuestMinLevel` before their intended branch. The
fixture now mirrors `Player::LoadFromDB`; test-harness correctness only, and the
remaining 7 suite failures stay a separate defect track.

**Save-snapshot manager-lock re-entry — 2026-09-17, implementation `f55d9ef3`:**
the fixture save-snapshot path resolved `player_level_like_cpp` inside a
manager-locked `do_for_all_maps` closure and self-deadlocked; the level is now
hoisted, which removed the last `wow-world --lib` hang. The suite completes
(3896 passed, 28 pre-existing failures) and those failures are recorded as a
separate defect track, not parity work.

**Session reputation-closure lock re-entry deadlock — 2026-09-17, implementation
`299fe975`:** `with_reputation_mgr_like_cpp` holds the canonical manager lock while
its closure runs, so resolving `player_race_like_cpp`/`player_class_like_cpp`
inside it self-deadlocked. The identity is hoisted before the closure at all six
call sites, which removes the reputation-family `wow-world --lib` hangs. The same
unit adds the `CanUseItem` learning-effect and reputation gates to
`CollectionMgr::CanAddAppearance`. This is a runtime-correctness repair, not a
parity claim; the 28 pre-existing `wow-world --lib` failures and the two
`save_snapshot_owner` hangs remain separate tracks.

**Collection appearance `CanUseItem` template gates — 2026-09-17, implementation
`a2c8c3bb`:** `CollectionMgr::CanAddAppearance` now also applies the represented
`Player::CanUseItem(ItemTemplate const*)` gates (`Player.cpp:11069-11125`):
internal and faction flags, allowable race, required level, required skill/rank
and required ability. Holiday, reputation, the learning-effect pair and artifact
specialization remain separate. This keeps the collection admission on the same
template contract as C++ without changing the #584 → #583 → #153 architecture
gate.

**Collection appearance weapon-proficiency gate — 2026-09-17, implementation
`4ba42678`:** `CollectionMgr::CanAddAppearance` now reads the learned
`Player::GetWeaponProficiency` mask (`Player.cpp:649-699`) instead of the class
default, so collected weapon appearances follow the same proficiency authority as
C++ and the `!GetPlayer()` guard fails closed. The collection acceptance fixtures
install the canonical Player and seed the mask. The armor-proficiency half of
`CanUseItem` and the broader class/race/level gates remain separate. This does not
change the #584 → #583 → #153 architecture gate.

**#61 attack power aura producers — 2026-09-17, implementation `408725a5`:** the
attack power projection now consumes `SPELL_AURA_MOD_ATTACK_POWER`/
`MOD_RANGED_ATTACK_POWER` flats and the `..._PCT` multipliers in the C++
`Player::UpdateAttackPowerAndDamage` order (`StatSystem.cpp:333-403`), with the
`CLASSMASK_WAND_USERS` ranged skip, and `Unit::GetTotalAttackPowerValue`'s zero
clamp. The multipliers are published through the effective stats, the stat-update
packet and the create block instead of a hardcoded zero, so the already-published
attack power fields and damage ranges become aura-aware without a new writer.
`MOD_ATTACK_POWER_OF_ARMOR` and the spell-power override remain separate gates,
continuing the F1 aura-route work without changing the #584 → #583 → #153
architecture gate.

**#61 school resistances — 2026-09-17, implementation `67d23891`:** the six magic
schools now run C++ `Unit::UpdateResistances` (`Unit.cpp:9148-9163`) with the
item `BASE_VALUE`, the `MOD_BASE_RESISTANCE_PCT` `BASE_PCT`, the
`MOD_RESISTANCE`/`MOD_BASE_RESISTANCE` `TOTAL_VALUE` and the
`MOD_RESISTANCE_PCT` `TOTAL_PCT`, and the login create block publishes the seven
`UnitData::Resistances` values instead of a hardcoded zero. The shared
mask-parameterized helpers keep one resistance-aura authority for both the armor
and school routes. Post-login resistance deltas remain a separate gate. This
continues the F1 resistance route without changing the
#584 → #583 → #153 architecture gate.

**#61 critical-strike aura percentages — 2026-09-17, implementation
`7412dac5`:** the critical-strike projection now consumes
`SPELL_AURA_MOD_WEAPON_CRIT_PERCENT` filtered per attack by the weapon
requirement plus the global `SPELL_AURA_MOD_CRIT_PCT`, and
`SPELL_AURA_MOD_SPELL_CRIT_CHANCE` plus `MOD_CRIT_PCT` for every school, in the
C++ `UpdateWeaponDependentCritAuras`/`UpdateCritPercentage`/
`UpdateSpellCritChance` order (`Player.cpp:8079-8107`, `StatSystem.cpp:502-538`,
`718-731`). The offhand group no longer shares the mainhand value and the
already-published critical fields become aura-aware without a new writer,
continuing the F1 aura-route work without changing the
#584 → #583 → #153 architecture gate.

**#61 avoidance aura percentages — 2026-09-17, implementation `255c888f`:** the
represented avoidance projection now consumes the flat
`SPELL_AURA_MOD_BLOCK_PERCENT`/`MOD_PARRY_PERCENT`/`MOD_DODGE_PERCENT` sums in
the C++ `Player::UpdateBlockPercentage`/`UpdateParryPercentage`/
`UpdateDodgePercentage` order (`StatSystem.cpp:483-499`, `659-679`, `700-717`),
with the class parry caps honoured. The already-published
`block_pct`/`dodge_pct`/`parry_pct` fields become aura-aware without a new
writer, continuing the F1 aura-route work without changing the
#584 → #583 → #153 architecture gate.

**#61 armor aura producers — 2026-09-17, implementation `6b7334a0`:** the stat
system now runs the complete C++ `Player::UpdateArmor` producer chain
(`StatSystem.cpp:251-276`): `MOD_BASE_RESISTANCE_PCT` `BASE_PCT`,
`MOD_RESISTANCE`/`MOD_BASE_RESISTANCE` normal-mask `TOTAL_VALUE`,
`MOD_RESISTANCE_OF_STAT_PERCENT`, `MOD_RESISTANCE_PCT` `TOTAL_PCT` and
`MOD_BONUS_ARMOR_PCT`, in C++ order with `int32(value)` truncation. The session
resolves the aura inputs from the canonical applications and feeds the existing
stat-system input, so the already-published `armor` field becomes aura-aware
without a new writer. The school (1-6) resistance publication stays a separate
gate. This advances the F1 resistance route without changing the
#584 → #583 → #153 architecture gate.

**#61 `Unit::m_transformSpell` and `IsPolymorphed` — 2026-09-17, implementation
`e8994e95`:** the canonical Unit aura subsystem now owns C++
`Unit::m_transformSpell` with the `AuraEffect::HandleAuraTransform` apply/remove
rules (`SpellAuraEffects.cpp:1944-1951`, `2129-2131`), written only by the session
aura insert/remove funnels. `Unit::IsPolymorphed` (`Unit.cpp:9993-10004`) is
resolved through the `SpellClassOptions` MAGE family plus effect 0 applying
`SPELL_AURA_MOD_CONFUSE` (`SpellInfo.cpp:2665-2671`) and drives the C++
`Player::RegenerateHealth` polymorph branch (`Player.cpp:1857-1859`);
`Unit::IsInDisallowedMountForm` (`Unit.cpp:8813-8820`) now reads the canonical
transform spell. This closes the hardcoded polymorph gate and the visible-aura
transform heuristic without a new mutable mirror, lock or clock, and without
changing the #584 → #583 → #153 architecture gate.

**#61 aura-backed per-attack expertise — 2026-09-16, implementation `a3345bc9`:**
`Player::UpdateExpertise` (`StatSystem.cpp:759-786`) now runs in the character
stat projection: the truncated combat-rating bonus plus the
`SPELL_AURA_MOD_EXPERTISE` sum filtered by
`SpellInfo::IsItemFitToSpellRequirements` (`SpellInfo.cpp:1757-1768`) for the
weapon of each attack, clamped at zero and written to
`MainhandExpertise`/`OffhandExpertise` separately. Same-effect stack groups are
folded like `Unit::GetTotalAuraModifier` (`Unit.cpp:4818-4850`), and the
unwritten `RangedExpertise`/`CombatRatingExpertise` fields keep their zero
create value. This completes the equipment/aura half of the F1 stat projection
without a new Session field or clock and without changing the
#584 → #583 → #153 architecture gate.

**#61 food/drink regeneration emote visual — 2026-09-16, implementation `73c67a9a`:**
the regeneration tick now completes C++ `Player::RegenerateAll`'s tail
(`Player.cpp:1609-1678`): `m_foodEmoteTimerCount` accumulates `m_regenTimer` on
its own five-second timer and, on crossing 5000ms, publishes
`SPELL_VISUAL_KIT_FOOD` (406) for a Standing `SPELL_AURA_MOD_REGEN` or
`SPELL_VISUAL_KIT_DRINK` (438) for a Standing `SPELL_AURA_MOD_POWER_REGEN`
through `Unit::SendPlaySpellVisualKit` (`Unit.cpp:11566-11574`). The accumulator
is a canonical `Unit` field and the session tick keeps the single writer, so the
regeneration vertical now covers the timer/health/power/emote phases without a
new Session field or clock. This continues the F1 regeneration vertical without
changing the #584 → #583 → #153 architecture gate.

**#61 observer `SMSG_POWER_UPDATE` fan-out — 2026-09-16, implementation `06ef4fd2`:**
the owner power publication now mirrors `Unit::SetPower`'s
`SendMessageToSet(packet, true)` (`Unit.cpp:9287-9312`): the owner gets its own
packet and the same bytes are queued for nearby observers through the existing
realm visibility rail, excluding the source. This closes the observer
packet-type parity gate for the represented regeneration publication without a
new Session field or clock.

**#61 creature-kill durability loss — 2026-09-16, implementation `7d33d742`:**
the creature-melee victim handler now runs the C++ `Unit::Kill` player-victim
durability branch (`Unit.cpp:10639-10648`): the PvE gate
`durabilityLoss && !player && !InBattleground()` applies
`DurabilityLossAll(baseLoss, false)` and publishes the C++
`baseLoss - baseLoss * MOD_DURABILITY_LOSS multiplier` message before the melee
result presentation. This completes the general creature producer of the F1
durability chain; the player-killer PvP branch and `SetPvPDeath` remain explicit
#61 gates.

**#61 durability-damage spell effects — 2026-09-16, implementation `5759b474`:**
the represented direct spell-effect dispatch now handles
`SPELL_EFFECT_DURABILITY_DAMAGE` (111) and
`SPELL_EFFECT_DURABILITY_DAMAGE_PCT` (115) (`SpellEffects.cpp:4316-4373`),
reusing the integrated `DurabilityPointsLossAll`/`DurabilityLoss` functions, the
`INVENTORY_SLOT_BAG_0` slot branch and the `TYPEID_PLAYER` target guard. This
completes the spell producer of the F1 durability chain; the general
`Unit::Kill` PvE/PvP producer remains an explicit #61 gate.

**#61 fall-death item durability loss — 2026-09-16, implementation `c8059a85`:**
the represented `Player::DurabilityLossAll`/`DurabilityLoss`/`DurabilityPointsLoss`
chain (`Player.cpp:4522-4620`) now runs on the fall-to-death branch of
`Player::EnvironmentalDamage` (`Player.cpp:655-670`): equipment loses
`DurabilityLoss.OnDeath` percent with the `MOD_DURABILITY_LOSS` multiplier and
the `PREVENT_DURABILITY_LOSS` gate, a 0-durability item loses its mods before
the durability write, and `SMSG_DURABILITY_DAMAGE_DEATH` is published. The rate
is a config-registry row installed through the runtime policy bundle. The
general `Unit::Kill` PvE/PvP producer and durability-damage spells remain
explicit #61 gates. This advances the F1 item/stat vertical without changing the
#584 → #583 → #153 architecture gate.

**#61 C++ regeneration rates — 2026-09-16, implementation `bf3794f0`:** the
regeneration tick now consumes the C++ `World::setRegenRate` values
(`World.cpp:615-623`) through `PlayerRegenerationRatesLikeCpp` instead of
hardcoding 1.0: `Rate.Health` scales `RegenerateHealth` and the
`RatesForPower`-mapped `Rate.Mana`/`Rate.Rage.Loss`/`Rate.Focus`/`Rate.Energy`/
`Rate.RunicPower.Loss` scale `Regenerate`. The six keys are registry rows
resolved by the composition root and attached to
`SessionHandlerCatalogsLikeCpp` (no new Session field). The `RatesForPower`
entries for unrepresented powers remain an explicit boundary. This continues the
F1 regeneration vertical without changing the #584 → #583 → #153 architecture
gate.

**#61 non-mana power-regeneration loop — 2026-09-16, implementation `2387c04b`:**
the canonical session tick now walks the complete C++
`for (power = POWER_MANA; power < MAX_POWERS; ...)` loop
(`Player.cpp:1614,1681-1747`), so the represented primary rage/energy/focus/
runic-power is regenerated from its DB2 `PowerTypeEntry` with the
`MOD_POWER_REGEN_PERCENT`/`MOD_POWER_REGEN` aura producers and the
`m_regenTimerCount`-vs-`m_regenTimer` energy split. Mana keeps the published
`UpdateManaRegen` inputs and skips those producers. One `SMSG_POWER_UPDATE` is
published per changed power on the two-second boundary. This continues the F1
effected-stat/regeneration vertical without changing the #584 → #583 → #153
architecture gate. Rate overrides, alternate powers, rune regeneration,
`IsPolymorphed`/`m_transformSpell`, observer `SendMessageToSet` parity and live
QA remain open #61 gates.

**#61 health-regeneration tick — 2026-09-16, implementation `fbd8755e`:** the
canonical session tick now runs the complete C++ `Player::RegenerateAll`
step, including the two-second `RegenerateHealth` branch. `wow-data` owns the
`RegenMPPerSpt`/`RegenHPPerSpt`/`OCTRegenHP` tables behind
`RegenGameTablesLikeCpp`; `Unit::regenerate_health_like_cpp` reproduces the
formula, aura producers, `int32` truncation and `ModifyHealth` clamp from
`Player.cpp:1842-1882,5162-5180` and `Unit.cpp:8115-8155`. This advances the F1
effected-stat/regeneration vertical without changing the #584 → #583 → #153
architecture gate. `IsPolymorphed`/`m_transformSpell`, alternate powers,
rate overrides, observer `SendMessageToSet` parity and live QA remain open #61
gates.

**#743 (group state application/reconciliation), #735 (reputation encapsulation) and
#787 (World/Map session-phase coordination) are delivered and accepted within their
recorded scopes.** P3.8, P3.9, P3.10, P3.11, P3.12 and P3.13 are integrated bounded visibility corrections
after P3.1–P3.7. P3.7 closes the measured Creature relocation
fanout gap, while P3.8 marks recipients for object admission/removal through the same
deferred Player-session rail. P3.1 retired the discarded canonical Creature writer,
P3.2 delivered canonical `Map::SendObjectUpdates` publication, and P3.3 now moves
`ProcessRespawns`/`UpdateSpawnGroupConditions` before object visitation while retaining
the legacy Creature owner. P3.3 and P3.4 are integrated: the
production map tick consumes one nearby-cell/source plan for the represented
`ObjectUpdater` families instead of scanning whole typed stores. The legacy Creature
writer and its complete effect consumer remain outside that structural cut. P3.5 now
uses each source's C++ activation radius, including Creature/Pet `m_SightDistance`
for inactive sources; P3.6 now applies the Player cinematic instance-distance override
once its represented camera cursor is active. P3.7 is the bounded fanout delivery from
`CreatureRelocationVisibilityPlan.player_visibility_updates` into the existing deferred
Player-session rail; P3.8 extends that rail to `Map::AddToMap`/`RemoveFromMap` lifecycle
recipient marking without delivering under a map mutation; P3.9 publishes directed
ordinary Creature DESTROY after map removal through that rail with map-incarnation
and `HaveAtClient` fences. P3.10 publishes receiver-filtered Player/Unit VALUES after
`Map::SendObjectUpdates`, with Unit enqueue fixes and no packet delivery under a map
guard. PR #873 keeps Player/Unit out of the generic map rail to prevent duplicate
delivery and owner-only bytes reaching observers, and rechecks the admitted `MapKey`
before publication. Neither slice migrates the legacy Creature owner. P3.10 is integrated;
P3.11 now includes canonical Pets in the Creature CREATE discovery path after the map
cell query, preserving phase/range/detection gates and the canonical Pet owner. P3.12
extends the already-audited directed DESTROY rail to Pets after `Map::RemoveFromMap`,
preserving map-incarnation and `HaveAtClient` fences. It does not move Pet AI, movement,
summon lifecycle, persistence or vehicle/transport ownership. P3.13 generalizes the
same directed DESTROY rail to in-world Corpses, retaining the C++
`WorldObject::RemoveFromWorld`/`UpdateObjectVisibilityOnDestroy` order; corpse
reclaim, persistence, loot and live QA remain separate gates.
PR #876 adds the bounded Transport VALUES visibility projection without broadening the
CREATE/lifecycle claim. The fresh 2026-09-14 audit selected **Transport
CREATE/DESTROY and phase-visibility intents**, delivered by PR #901 and integrated at
`bf460aa7a8ccec0269eea1771a094ef12f0c6109`. C++ creates and removes transport blocks
through `Map::AddToMap` and `Map::SendUpdateTransportVisibility`
(`Map.cpp:574-610,1853-1915`); the Rust slice carries typed map snapshots through
the deferred Session visibility rail and updates `m_visibleTransports` atomically
with CREATE/OUT-OF-RANGE packets. This candidate deliberately leaves
`Transport::TeleportPassengersAndHideTransport` map relocation, seat/offset admission,
passenger movement, AI/scripts and live capture/DB acceptance as separate #63/#584
gates. It does not reopen closed issues or start Creature-owner migration.

The fresh post-#931 audit selected the next P4 navigability slice in
`world-server/src/runtime/game_events.rs`. PR #933 preserves its complete public
operation set while separating despawn, loaded-grid, spawn, bootstrap, scheduler,
live-update and consumer responsibilities. The facade is 9 lines and the child
modules total 2,632 lines; no runtime owner, dependency, packet or persistence
boundary changed. The focused game-event suite passes 181/181 and the architecture
ratchets remain at baseline.

The fresh post-#933 audit selected the 5,125-line
`world-server/src/spawn_store_loader.rs` hotspot for a responsibility split. PR #935
keeps startup composition, DB ownership and spawn conversion in the facade while
moving the represented GameEventMgr catalog models to
`spawn_store_loader/game_event_catalog.rs` and WorldStateMgr startup/index rules to
`spawn_store_loader/world_state_catalog.rs`. The facade is now 3,427 lines; the
children are 1,355 and 345 lines, and the public `spawn_store_loader` paths remain
unchanged through deliberate reexports. The runtime child visibility correction also
makes the published #933 split compile from the parent reexports. Focused game-event
coverage remains 181/181; compile, format/diff and architecture check/self-test pass.
The final local profile still reports the three pre-existing GameObject respawn-save
failures in `scenarios_9` (587 passed, 3 failed); PR #936 then tightened both physical
policy rows to the exact scanner counts at the current integration head. No packet, SQL,
clock, lock, persistence order or runtime behavior changed in this physical slice.

The follow-up post-#935 audit selected the remaining game-event state family inside
the canonical spawn metadata owner. PR #938 (`c5967026`) moves the complete
game-event transition/cache implementation into the private
`spawn_store_loader/game_event_runtime.rs` module while keeping every operation as
an inherent method on `CanonicalSpawnMetadataLikeCpp`. The facade is now 2,627
lines and the runtime child is 800 lines; the aggregate hotspot remains exactly
3,427 lines, so no policy ceiling grows. The focused game-event suite remains
181/181, architecture check/self-test and compile/format/diff gates pass, and the
full local profile retains only the three pre-existing `scenarios_9` GameObject
respawn-save failures (587 passed, 3 failed). No owner, packet, SQL, clock, lock,
persistence order or runtime behavior changed. This closes one navigability family;
creature/gameobject/addon/spawn-group loading and the remaining #584 C0-C4,
runtime, capture, DB/relogin and live-QA gates remain open.

The next post-#938 audit selected the complete GameEventMgr startup loader family.
PR #940 (`2959e245`) moves condition, prerequisite, pool, spawn-GUID,
quest-relation, NPC-flag/vendor and model/equipment loading and validation into
the private `spawn_store_loader/game_event_loader.rs` module. The composition
facade is now 1,990 lines and the loader child is 637; together with the existing
800-line runtime child this keeps the measured spawn-loader aggregate at exactly
3,427 lines. The parent reimports all `pub(super)` helpers so the existing private
fixtures remain valid without widening public API. Game-event tests remain 181/181;
architecture, compile, format/diff and final gates pass, while the full profile
retains the same three pre-existing `scenarios_9` GameObject respawn-save failures
(587 passed, 3 failed). No startup order, owner, packet, SQL, clock, lock,
persistence or runtime behavior changed. Remaining creature/gameobject/addon/
spawn-group loading and the other #584 gates require a fresh audit.

The post-#940 audit selected the complete pool and spawn-group startup family.
PR #942 (`77ca1c64`) moves pool templates, creature/gameobject/pool members,
relation and map validation, autospawn candidates and spawn-group template
construction into private `spawn_store_loader/pool_loader.rs`. The composition
facade is now 1,652 lines and the pool child 338; the measured spawn-loader
aggregate remains exactly the existing 3,427-line ceiling and the public helper
path remains unchanged. Focused pool tests pass 29/29; architecture, compile,
format/diff and structural gates pass, while the full profile retains the same
three pre-existing `scenarios_9` GameObject respawn-save failures (587 passed,
3 failed). No startup order, owner, packet, SQL, clock, lock, persistence or
runtime behavior changed. Remaining creature/gameobject/addon loader families
and the other #584 C0–C4/runtime/capture/DB/relogin/live-QA gates require a
fresh audit.

The post-#942 audit selected the complete object-spawn startup family.
PR #944 (`e0a8fae0`) moves Creature, GameObject and AreaTrigger row loading,
conversion/validation, runtime-row capture and linked-respawn admission into
private `spawn_store_loader/spawn_object_loader.rs`. The composition facade is
now 999 lines and the object child 653; with the existing pool and GameEvent
children, the measured spawn-loader aggregate remains exactly the 3,427-line
ceiling. Existing startup calls and parent-private test paths remain available
through scoped `pub(super)` imports. Focused spawn-loader tests pass 135/135;
architecture, compile, format/diff and structural gates pass, while the full
profile retains the same three pre-existing `scenarios_9` GameObject
respawn-save failures (587 passed, 3 failed). No startup order, owner, packet,
SQL, clock, lock, persistence or runtime behavior changed. The remaining
addon loader family and other #584 C0–C4/runtime/capture/DB/relogin/live-QA
gates require a fresh audit.

The post-#944 audit selected the complete creature movement metadata family.
PR #946 (`9698a656`) moves waypoint path/report construction, coordinate and
delay normalization, default waypoint lookup, creature formation validation and
their persistence loaders into private
`spawn_store_loader/creature_movement_loader.rs`. The composition facade is now
784 lines and the movement child 215; with the existing pool, object and
GameEvent children, the measured spawn-loader aggregate remains exactly the
3,427-line ceiling. Public waypoint/formation types and helper paths stay
available through explicit re-exports, and startup order is unchanged. Focused
spawn-loader tests pass 135/135; architecture, compile, format/diff and
structural gates pass, while the full profile retains the same three
pre-existing `scenarios_9` GameObject respawn-save failures (587 passed, 3
failed). No gameplay, packet, SQL, clock, lock, persistence or runtime behavior
changed. The remaining addon loader family and residual composition readers
require a fresh audit.

The post-#946 residual audit selected the remaining startup readers that still lived
physically in the spawn-loader parent. PR #948 (`861fbc70`) moves the linked-respawn
row conversion into `spawn_object_loader.rs`, the GameEvent world prefix/suffix
adapters into `game_event_loader.rs`, and spawn-group member persistence adaptation
into `pool_loader.rs`. The composition facade is now 730 lines; the object, pool,
GameEvent loader, movement and runtime children are 661, 358, 663, 215 and 800
lines respectively, and the measured aggregate remains exactly the 3,427-line
ceiling. Existing `pub(super)` paths preserve startup order, public APIs and
parent-private fixtures. Focused spawn-loader tests pass 135/135; architecture
check/self-test, compile, format/diff and final physical/hotspot gates pass. The
full profile retains the same three pre-existing `scenarios_9` GameObject
respawn-save failures (587 passed, 3 failed). No gameplay, packet, SQL, clock,
lock, persistence or runtime behavior changed. PR #950 reconciles the stale
Session ownership anchor and the exact syntax, registry and bridge baselines
after #929/#933; the checker-only suite passes 364/364, syntax-only ownership
passes, and architecture check/self-test remain green. The next step is a fresh
audit of the remaining #584 C0–C4/runtime/capture/DB/relogin/live-QA work.

The fresh post-#929 audit selected a P4 navigability slice in
`world-server/src/creature_loaded_grid.rs`. PR #931 keeps its production
construction/resolution contract in a 797-line facade and moves all 28 tests into
`creature_loaded_grid_tests/{builder,resolver}.rs`, preserving registration and
`cfg(test)` attribution. No runtime owner, dependency, packet or persistence
boundary changes. The next #584 macro again requires a fresh C0–C4 audit; the
remaining runtime/lifecycle/capture gates are not implied to be complete.

PR #897 closes the next measured P2 owner surface: the remaining production mount-presentation write now uses a named `Player` transition that applies `MountDisplayID` and `UNIT_FLAG_MOUNT` together, following TrinityCore `Unit::Mount` / `Unit::Dismount` (`Entities/Unit/Unit.cpp:7822-7865`). Session retains aura, collision, vehicle-kit and packet side effects; its broad unit-presentation closure is test-only for detached scale fixtures. The Player owner regression, mount spell-state scenarios, package checks, formatting/diff and architecture ratchet pass. This is an ownership closure: full mount gameplay, persistence, captures and live QA remain separate #63/#584 gates. The next #584 macro still comes from a fresh C0–C4 responsibility and consumer audit.

The fresh audit selected **Player identity ownership** as the bounded #584
macro, integrated by PR #904 (`4ad36d42`). Name, race, class, level
and gender resolve from canonical `wow_entities::Player`/`Unit`/`WorldObject` after
the login transition, following `Player::LoadFromDB` (`Player.cpp:17060-17089,
17247-17283`) and `Unit::GetLevel/GetRace/GetClass/GetGender` (`Unit.h:733-745`).
The Session keeps only a one-way login bootstrap DTO, and detached identity fields
are test fixtures. Consumer migration covers module login, registry, character
query/entry, group and chat; stale handles fail closed. The C++ swing-error member
(`Player.h:3023`) is deliberately a separate melee macro. Its focused
checks and architecture evidence are recorded in `session-578-checkpoint.md`;
durable save/reload, captures and live DB/relogin remain open acceptance gates.

PR #906 integrates the bounded **Player swing-error owner** macro at merge
`9a35ba0f` (implementation `67409023`). It follows `Player::SetAttackSwingError`
(`Player.cpp:20625-20631`) and its `Unit::DoMeleeAttackIfReady` caller
(`Unit.cpp:2087-2150`): nullable duplicate suppression and clear state live on
the canonical `wow_entities::Player`, while Session remains the packet
encoding/delivery adapter. Missing or stale owners fail closed. This removes the
Session mirror, updates the ownership ledger to 647 total fields (220 production
and 427 test fixtures) and leaves 11 production residual fields for later audited macros. The
slice does not claim a packet-layout change, persistence, exact captures or live
DB/relogin QA; those remain explicit #584 acceptance gates.

PR #907 integrates the pending-bind confirmation evidence boundary at merge
`33141494b4410a26e01ac5d72ea9f82132b2e522`. TrinityCore confirms pending binds on
`Player`/`InstanceMap` from `MiscHandler.cpp:1063-1075`; Rust's
`represented_confirmed_pending_binds` vector had no production counterpart and is
now `cfg(test)` diagnostic evidence only. The production field count is 220 while
total membership stays 647; the final profile paths 01–07 pass and the full
`wow-world` library suite reaches the documented 900-second timeout, so it is not
claimed green. No packet, persistence, map or Player behavior changes.

The same fresh C0–C4 audit classifies `recent_player_guid_low_like_cpp` as
cohesive Session identity state. TrinityCore's `WorldSession::m_GUIDLow`
(`Server/WorldSession.h:1881`, `WorldSession.cpp:877-888,980-985`) survives
recent logout for character-scoped account-data attribution and social admission;
Rust's `set_player_guid` and persistence plan have the same writer, readers and
Session lifetime. The field moves from the unresolved residual into
`session_identity_account_and_realm_policy` in the ledger without a code or
behavior change. The residual is now 10 exact production fields; total membership
remains 647 (220 production, 427 test fixtures).

PR #909 integrates the audited `player_guid` classification into a dedicated
selected-player Session binding family at merge
`1c8b5577badccb6f74d3b049a7e231494b9fa792`. TrinityCore keeps `_player` on `WorldSession`
(`Server/WorldSession.h:1882`) and installs/clears it through `SetPlayer` during
login and logout (`WorldSession.cpp:672-694,978-985`); Rust's `set_player_guid`
is the generation-checked GUID key used by Session admission, lifecycle,
addressing and stale-owner guards. It never reconstructs gameplay state when the
canonical Player is absent. This ledger-only classification reduces the exact
unresolved production residual to 9 while keeping 647 total fields (220 production,
427 test fixtures). Architecture check, self-test (20/20) and diff validation pass.

PR #911 integrates the audited `vmap_indoor_check_like_cpp` classification into
the immutable world-configuration family at merge
`0dbf768419338ac7d3bf3974cf20fa7e7fa4c2ad`. TrinityCore reads `CONFIG_VMAP_INDOOR_CHECK`
from process `World` (`World.h:119`, `World.cpp:1116`); Rust loads the same key in
`world-server/app.rs:5064`, injects it through
`SessionRuntimePolicyCapabilitiesLikeCpp` (`session_resources.rs:295,505`) and
only reads the resulting switch in the aura indoor-check branch. This ledger-only
classification leaves 8 exact unresolved production fields and keeps the 647-field
membership unchanged (220 production, 427 test fixtures). Architecture check,
self-test (20/20) and diff validation pass.

PR #913 integrates the audited classification of `represented_cast_unstuck_enabled_like_cpp`
to the immutable world-configuration family. TrinityCore reads
`CONFIG_CAST_UNSTUCK` from `World` (`World.h:119`, `World.cpp:1116`) before
`Spell::EffectStuck` (`Spells/SpellEffects.cpp:3265-3269`); Rust now loads the key
through `world-server/app.rs`, injects it in
`SessionRuntimePolicyCapabilitiesLikeCpp`, and the spell consumer only reads the
policy. The named disabled-policy scenario and world-server config regression
cover this boundary. The code-backed classification leaves 7 exact unresolved
production fields and keeps the 647-field membership unchanged (220 production,
427 test fixtures).

## P2 Player base-stat catalog classification — candidate, 2026-09-14

El siguiente C0–C4 audit clasifica `player_stats` como un catálogo inmutable de
configuración/servicios, no como autoridad de estado de `WorldSession`. TrinityCore
mantiene las filas de raza/clase/nivel en `ObjectMgr::_playerInfo` y expone
`ObjectMgr::GetPlayerLevelInfo` (`Globals/ObjectMgr.h:628-673,1155,1866`,
`ObjectMgr.cpp:4429-4445`); `Player` solo consulta esa autoridad durante el cálculo
de sus estadísticas (`Player.cpp:2256-2260,2370-2374`). Rust ya carga el catálogo una
vez en `world-server/app.rs:2222-2240`, lo compone en la capacidad `SessionInventoryCapabilitiesLikeCpp`
(`session_resources.rs:65,107`) y lo conserva como `Arc<PlayerStatsStore>` para los
lectores de estadísticas (`handlers/character/stats.rs:117`, `session_state.rs:1103`).
El ledger registra la clasificación en `immutable_catalogs_configuration_and_services`;
no se mueve gameplay, no se crea una autoridad duplicada y el residual exacto queda en
6 campos hasta la integración.

## P3 Creature query duplicate-response correction — integrated PR #916, 2026-09-14

PR #916 integrates the `creature_query_cache` correction: the audit found a behavior mismatch, not a valid Session
authority. TrinityCore keeps serialized bytes in `CreatureTemplate::QueryData` and
`WorldSession::HandleCreatureQuery` responds to every `CMSG_QUERY_CREATURE`, using the
cache only to build the payload (`Handlers/QueryHandler.cpp:71-99`,
`World.cpp:1706-1707`). Rust kept a per-Session `HashSet<u32>` and silently suppressed
repeated requests; the field is removed and the handler now responds to every request.
The focused regression requires two responses for two identical queries. The syntax
snapshot was regenerated with the official checker, and the existing
`represented_player_unit_values_updates_delivered_like_cpp` classification is recorded
under the Map/visibility publication family. The ledger remains 647 fields (219
production, 428 fixtures) and the exact residual falls to 5; the integrated boundary keeps
the `CacheDataQueries` configuration difference explicit for future work.

## P4 Map publication delivery guards — integrated PR #918, 2026-09-14

The fresh C0–C4 audit classifies `represented_capture_point_removed_delivered_like_cpp` and
`represented_dynamic_object_values_updates_delivered_like_cpp` under
`map_runtime_creature_gameobject_and_visibility`. TrinityCore emits
`CapturePointRemoved` from `GameObject::Delete` (`Entities/GameObject/GameObject.cpp:1746-1756`)
and builds DynamicObject field updates through `WorldObjectChangeAccumulator`
(`Entities/Object/Object.cpp:3654-3717`) before `Map::SendObjectUpdates` publishes the
per-player update map (`Maps/Map.cpp:1929-1948`). Rust consumes only the canonical
`ManagedMap` summaries in `session/movement/movement_publication.rs:175-208` and
`session/instances/map_key.rs:453-628`; the generation/GUID/fingerprint sets are
receiver-local publication fences, not GameObject or DynamicObject authorities. The
classification leaves three exact production fields unresolved without changing wire bytes,
map clocks or persistence.

The remaining fields are intentionally separate responsibilities: `represented_instance_reset_times_like_cpp`
tracks C++ `Player::_instanceResetTimes` and its load/save paths (`Player.cpp:1116-1125,
19190-19198,27937-28010`) and needs a Player-owned persistence cut;
`represented_locked_dungeon_encounters` stands in for `Player::IsLockedToDungeonEncounter`
(`Player.cpp:20725-20748`) but has no production writer while the loaded
`DungeonEncounterStore` is not injected into sessions (`world-server/app.rs:522-528`);
and `represented_seer_guid_like_cpp` is C++ `Player::m_seer` (`Player.h:2417,2423`,
`Player.cpp:298-300,25344-25395`) and requires a complete Player/visibility owner move.

## P2 Player instance-reset owner — integrated PR #919, 2026-09-14

PR #919 moves C++ `Player::_instanceResetTimes` onto the canonical
`wow_entities::PlayerGameplayState` (`crates/wow-entities/src/player_gameplay_state.rs:60`)
with named Player operations in `player/recent_instances.rs`. Session admission now
materializes the canonical Player before farm-limit checks and entry accounting, so
`MapManager::CreateMap` observes one owner as in TrinityCore (`Player.cpp:1116-1125`).
Login hydration and save projection use that same owner
(`session/persistence/load.rs:101-140`, `session/lifecycle/persistence/projection.rs:380-388`);
the former Session map remains only as a `cfg(test)` fixture fallback and is transferred
once when a fixture owner is materialized. The focused owner regression, 16 instance
scenarios, 5 instance-count scenarios, 3 teleport scenarios, package check and syntax
ownership check pass. At the #919 checkpoint the exact production residual was two
fields; PR #921 closes the encounter query, leaving the Player `m_seer` visibility seam
as the only exact production residual.

## P2 Player dungeon-encounter lock query — integrated PR #921, 2026-09-14

PR #921 moves encounter-lock resolution off the mutable Session fixture set and
onto the C++-shaped query boundary. `DungeonEncounter.db2` is injected once through
`SessionWorldCatalogCapabilitiesLikeCpp`; the Session binding locates the player's
unique canonical map/difficulty and reads the shared `InstanceLockMgr` completed
encounter mask, matching `Player::IsLockedToDungeonEncounter`
(`Player.cpp:20725-20748`). Unknown encounter rows and absent active locks are known
unlocked states; missing or ambiguous authority fails closed for production loot.
`represented_locked_dungeon_encounters` remains only as `cfg(test)` input for
handle-less legacy fixtures. Creature and GameObject encounter-loot filters use the
canonical query. Focused canonical, loot and composition tests, cargo checks, the
ownership syntax check and architecture self-tests pass at merge
`d25cbc9161f8affb8c5201a1ad6a653870938969`. The only exact production residual is
the Player `m_seer` visibility seam.

## P2 Player::m_seer canonical visibility projection — integrated PR #923, 2026-09-14

TrinityCore initializes `Player::m_seer` to the Player itself
(`Player.cpp:298-300`, `Player.h:2417-2425`) and changes it through
`SetViewpoint` (`Player.cpp:25338-25395`); map and visibility code consume that
pointer (`Map.cpp:716-718`, `GridNotifiers.cpp:95-222`). Rust production code now
derives the seer GUID from the canonical map-owned Player's
`ActivePlayerData::FarsightObject`; empty means the Player itself. Deferred
visibility, movement, aggro and GameObject/DynamicObject consumers use this
projection. The former Session field is a `cfg(test)` fixture only.
`last_observed_farsight_object_like_cpp` is a receiver-local publication fence for
the one explicit FAR_SIGHT clear VALUES packet after viewpoint removal, not gameplay
authority. FAR_SIGHT (14), GameObject despawn (22), DynamicObject VALUES (15),
package, ownership syntax, architecture and self-test evidence pass at merge
`5f6b1ad8`. Full captures, DB/relogin durability and live QA remain gameplay/runtime
gates under #41/#63/#584; #584 retains other C0-C4 and runtime work.

## P3.13 directed Corpse DESTROY — integrated PR #929, 2026-09-14

The C++ corpse removal path is `Corpse::RemoveFromWorld` →
`WorldObject::RemoveFromWorld` (`Corpse.cpp:56`), with
`UpdateObjectVisibilityOnDestroy` reached before the source leaves the map
(`Object.cpp:1023-1029`, `Map.cpp:934-951`). Rust now uses the same generic
post-map-guard DESTROY rail for in-world Corpses as for Creature/Pet, capturing
recipients before erasure and rechecking map incarnation and `HaveAtClient` in the
receiving Session. Corpse reclaim, persistence, loot, transport/vehicle lifecycle,
exact captures and live QA remain separate #584/#63 gates.

PR #895 closes the next measured P2 owner surface: production rest-flag, deferred-publication and rest-clock writes now use named transitions on `wow-entities::Player` over `PlayerRestState`, following `RestMgr::SetRestFlag` / `RemoveRestFlag` (`RestMgr.cpp:95-122`), `RestMgr::_restTime` (`RestMgr.h:86`) and `Player::SetRestState` (`Player.h:2652`). Session retains packet/application ordering and its generic rest-state mutator is detached-fixture-only under `cfg(test)`. The Player owner regression, rest-owner scenarios, affected chat/area-trigger/zone scenarios, package checks, formatting/diff and architecture ratchet pass. This is an ownership closure: quest objective progress, durable persistence, captures and live QA remain separate #41/#584 gates. The next #584 macro still comes from a fresh C0–C4 responsibility and consumer audit.

PR #878 closes the next measured P2 owner surface: the Player-owned mount VehicleKit
operations now have named transitions in `wow-entities::Player`, following
`Unit::CreateVehicleKit`, `Unit::RemoveVehicleKit` and `Unit::GetVehicleKit`
(`Unit.cpp:11304-11323`). Session keeps template admission and represented aura,
packet and presentation effects; detached kit fixtures remain test-only. This slice
does not claim complete vehicle seat/offset admission, passenger lifecycle,
CREATE/DESTROY, Pet/corpse/Transport publication or live/DB acceptance. The next
#584 macro still comes from a fresh C0–C4 responsibility and consumer audit.
No old issue is reopened and no residual is promoted to implementation merely from a
textual inventory.

PR #881 closes the adjacent P2 Player-aura owner surface. Named `Player` operations
now mutate the Unit-owned `AuraSubsystem` for visible aura publication/removal,
aura-authority completion/tombstone/reset and threat-aura install/apply/remove;
Session keeps the protocol/catalog adapter and the handle-less closure is retained
only for `cfg(test)` fixtures. The C++ owner evidence is `Unit.h:620-640,
1226-1260,1825-1844` and `Unit.cpp:680-690`. This is an ownership closure, not
full aura gameplay or persistence acceptance; captures, DB/restart/relogin and live
QA remain explicit later gates. The next #584 macro still requires a fresh C0-C4
responsibility/consumer audit.

PR #883 closes the bounded P2 owner surface for represented `TradeData` in
`wow-entities::Player`. Named open/clear, state-index, acceptance, gold, item-slot
and trade-spell operations follow `Player::m_trade`, anchored to `Player.h:2998`,
`TradeHandler.cpp:694-695`, `Player.cpp:12864-12879` and `TradeData.cpp:58-150`.
Session remains the protocol/application adapter for money and inventory admission,
packet encoding and partner mailbox delivery; its generic whole-state mutator is
test-fixture-only. The delivery does not claim trade settlement durability, captures
or live QA, and the next #584 macro still requires a fresh C0-C4 audit.

PR #885 closes the bounded P2 owner surface for represented guild membership and
invitation transitions in `wow-entities::Player` at `19dea8e078f5b7bec827532cefb8f46911e332e7` (implementation
`f0d32675`). Named membership, invitation, rank and clear-invitation operations
follow `Player::SetInGuild` (`Player.cpp:7216`), `SetGuildIdInvited` and `SetGuildRank`
(`Player.h:1939,1943`); the old composite guild install is retired. Session retains
GuildMgr/cache, protocol and application effects, while its broad whole-state guild
mutator remains fixture-only under `cfg(test)`. Owner, social-scenario,
guild-handler, package, architecture and formatting/diff checks pass. The delivery
does not claim guild-manager/database durability, captures or live QA, and the next
#584 macro still requires a fresh C0-C4 audit.

PR #887 closes the bounded P2 Player Battleground owner surface at
`72f6a3fa87d00f9319c1cfa626f7a10345fc9654` (implementation `4bd82511`). Named
Player operations now own the represented `m_bgData` type/map/status/queue
transitions and adjacent arena invitation, following `Player.h:976,2335-2338,2821`,
`Player.cpp:24258-24262` and `Player.h:1956`. Session retains queue admission,
matchmaking/lifecycle coordination, packets and application effects; the broad
Battleground mutator remains only for handle-less `cfg(test)` fixtures. The owner,
canonical ownership, PVP handler, package, formatting/diff and architecture checks
pass. Queue/matchmaking/lifecycle, persistence, captures and live QA remain separate
gameplay gates, so #584 stays open for its other C0-C4 responsibilities.

PR #889 closes the next bounded P2 Player owner surface: at-login flags and
weapon/armor proficiency masks now use named canonical `Player` transitions in
`player/persistent_capabilities.rs`, following `Player.h:1433-1434,2474`.
Session keeps persistence and packet/application effects; the generic whole-state
adapter is retained only for handle-less `cfg(test)` fixtures. Owner, persistence
and spell-state regressions plus the architecture ratchet pass. Durable save/reload,
captures, live QA and the remaining #584 C0-C4 responsibilities are still open.

PR #891 closes the next measured P2 Player owner surface: post-teleport taxi route
advancement and taxi-flight cleanup now use named Player transitions over
`PlayerTaxi`, following `PlayerTaxi::NextTaxiDestination` (`PlayerTaxi.h:74`) and
`Player::CleanupAfterTaxiFlight` (`Player.cpp:22019`). Session keeps map admission,
spline completion and packet/application effects; the handle-less generic taxi
mutator is fixture-only under `cfg(test)`. The owner regression, 14 taxi tests,
11 canonical-access scenarios, 13 movement scenarios and architecture ratchet pass.
Route generation/node admission, teleport ordering, persistence, captures and live
QA remain explicit later gameplay gates; #584 stays open.

PR #893 closes the next bounded P2 Player owner surface: production zone/area,
terrain-authority, PvP-hostility/timer and outdoors writes now use named
transitions over `PlayerWorldLocalState`, following `Player::UpdateZone`,
`Player::UpdateArea`, `Player::UpdatePvPState`, `Player::UpdateContestedPvP`
and `WorldObject::IsOutdoors`. Session retains terrain/catalog resolution,
rest/aura/packet effects and application ordering; the generic world-local
mutator is fixture-only under `cfg(test)`. Owner, chat/zone, world-state,
outdoors/spell-state, package, formatting/diff and architecture checks pass.
Full terrain admission, aura/quest/rest side effects, persistence, captures and
live QA remain explicit #584 gameplay gates.

The P2 item-modifier writer residual is integrated by PR #816: the original
`&mut PlayerItemBonusStateLikeCpp` closure was retired behind a named Player-owned
resolved-effect operation. The follow-up owner closure was delivered in implementation
`ecc67603`: each item-set, level-cap, reset and resolved-enchantment mutation now names a
`Player` operation, while the fallback exists only for `cfg(test)` fixtures.
The follow-up named Player item-modifier closure is integrated by PR #839 (merge
`cc0559980a4232ab5743affaaa2babfedffdfcf3`, implementation `ecc67603`) and passed
`validation-v2 final` on 2026-09-13 (manifest
`target/validation-v2/manifests/20260913T165720.613650Z-3851477-final.json`).
Catalog/effect consumers remain in `wow-world`; effective statistics and auras remain
allocated to gameplay work such as #61. The P2 item-object residual is integrated in
`3.4.3` as `ef82beeb`: `PlayerInventoryRuntime` owns a closed
`ItemObjectUpdateLikeCpp` command set, every production caller describes updates without
receiving `&mut Item`, and wrapped-gift transformation is an owner operation. The old
closure remains only behind `cfg(test)` for fixtures. #584 remains open for its other
measured C0-C4 residuals; this delivery does not activate #583 or close the core gate.

The P2 Void Storage owner residual is integrated by PR #844 (`6f42782f`, implementation
`77e2c4b2`). `Player` now owns the fixed 160-slot state and the complete clear/load/mark,
lookup, free-slot, add, delete and swap transitions anchored to TrinityCore's `Player.cpp`
load/save and accessors. Session remains the protocol/application boundary for template
validation, persistence ordering and packet encoding. The focused owner tests and 29 Void
Storage world tests pass; DB/restart/relogin durability remains explicitly open.

The first F1 equipment/stat projection is integrated by PR #851 (`b26ce713`, implementation
`fb33111c`). A private character stat module now derives the represented equipment inputs
and publishes one runtime-only `PlayerEffectiveCombatStatsLikeCpp` snapshot owned by
`Player` at login and equipment recalculation. PR #857 (`fd302c35`) makes initial spell
threat consume the Player-owned AP snapshot with C++ clamp/multiplier order
(`Spell.cpp:5558-5575`, `Unit.cpp:9165-9180`). PR #858 (`cda7f8a0`) makes melee
consume Player-owned base/offhand weapon ranges derived by the C++-shaped `wow-data`
projection. PR #869 (`abd396a0`) closes the exact offhand admission used by
`Unit::DoMeleeAttackIfReady` (`Unit.cpp:2140`): the canonical Player slot and Item
object must resolve a weapon inventory type and a non-broken item, and
`Unit::IsInFeralForm` (`Unit.cpp:8807-8812`) suppresses the offhand branch. The packet
VALUES adapter split is retained; these slices do not make the snapshot a persistence
record or claim full combat parity. Keep #61 open for aura-backed producers, exact
AP/damage/aura/regen/expertise/penetration behavior, reversible equipment lifecycle
and capture/live DB/relogin evidence. F2
(#29/#31) may consume the snapshot only after those missing participants are integrated
and acceptance is recorded.

The next #61 audit found that the first projection still had two item-stat inputs:
`handlers/character/stats.rs` walked equipped inventory rows while the canonical
`PlayerItemBonusStateLikeCpp` accumulator also supplied `_ApplyItemBonuses`. Commit
`a43f51cf` removes that double-writer path. `_ApplyAllItemMods` now seeds the canonical
accumulator for each loaded, non-broken equipment item; equip, unequip, swap, equipment-set,
scaling and repair transitions publish the same complete effective-stat projection used by
combat. The change follows TrinityCore `Player::_ApplyItemBonuses` and
`Player::_ApplyAllItemMods` (`Player.cpp:7654, 7688`) and the equipment repair path
(`Player.cpp:8575`). The regression
`equipment_stats_use_one_canonical_contribution_path_like_cpp` proves one contribution on
equip, removal on break, reapplication on repair and equality between login and post-login
equip. A `world-server` check passed in 6m52s; the affected item suites pass 7/7, 5/5,
10/10 and 16/16. The architecture checker still reports the pre-existing
`session/mod.rs` ratchet drift (+21 production and +142 test lines) with no change in that
file; its baseline remains untouched. #61 stays open for aura-backed producers, complete
AP/damage/regen/expertise/penetration formulas, live item-wear break transitions and exact
capture/DB/relogin acceptance. #29/#31 remain gated on those participants.

The next bounded #61 slice is implementation `48378050`: the canonical Player
effective-stat publisher now derives equipment/rating expertise once from the C++
`GetRatingBonusValue(CR_EXPERTISE)` path and its level-specific CombatRatings
multiplier (`Player.cpp:5189-5209`, `StatSystem.cpp:759-783`). Mainhand, offhand and
aggregate expertise in the Player snapshot are consumed by the VALUES adapter, while
the raw fallback remains limited to handle-less fixtures. The canonical equipment
regression plus item suites pass (1, 7 and 5 tests), and the production world-server
check passes in 7m01s. Aura-backed expertise, complete formulas, wear-to-broken
production and capture/DB/relogin acceptance remain explicit #61 gates.

The following bounded #61 slice is implementation `9004f5cf`: world-server startup
loads the required C++ `RegenMPPerSpt.txt` table once, and the Session capability graph
installs that immutable authority on each session. `Player::OCTRegenMPPerSpirit`
(`Player.cpp:5182-5190`) and `Player::UpdateManaRegen` (`StatSystem.cpp:799-827`) are
represented by `sqrt(Intellect) * Spirit * ratio[level,class]`; the canonical Player
snapshot is the sole producer consumed by the VALUES adapter. Three `wow-data` table
tests and six focused character stat/persistence tests pass, and the production
`world-server` check passes in 3m02s. This slice removes the incorrect hand-written
class coefficients and extra constant but does not claim aura percentage/stat-derived
MP5, complete regen-tick publication, wear-to-broken production or live capture/DB/
relogin parity; those remain #61 acceptance gates.

PR #959 (`4f7ce25b`, implementation `48218460`) integrated the aura-backed percentage
producers from
`Player::UpdateManaRegen` (`StatSystem.cpp:809-812`). The Player-owned effective-stat
publisher resolves active visible applications through canonical aura state plus
immutable `SpellInfo`, filters both `SPELL_AURA_MOD_POWER_REGEN_PERCENT` and
`SPELL_AURA_MOD_MANA_REGEN_PCT` to `POWER_MANA`, and multiplies the spirit producer by
each C++ `1 + amount / 100` contribution. The focused aura regression and the six
existing stat-update regressions pass, as does the affected test-aware check for
`world-server`, `wow-data` and `wow-world`. This is only the percentage producer:
flat `MOD_POWER_REGEN`, stat-derived MP5, complete tick/publication semantics,
wear-to-broken production and live capture/DB/relogin parity remain #61 gates.
Same-effect stack-policy parity needs an explicit aura-application contract before
it can be claimed. The architecture physical-source check passes, while the runtime
hotspot ratchet retains the pre-existing Session/character/world-server/Player drift;
the baseline remains untouched.

PR #960 (`96b8ffe3`, implementation `bfe49b2b`) completes the local flat and interrupted portions of
`Player::UpdateManaRegen` (`StatSystem.cpp:815-826`). The canonical Player publisher
adds `MOD_POWER_REGEN` to the MP5-equivalent rate, projects each
`MOD_MANA_REGEN_FROM_STAT` effect with `stat * amount / 500`, and applies the capped
`MOD_MANA_REGEN_INTERRUPT` percentage to the spirit component for combat interruption.
The aura resolver retains one canonical visible-application/SpellInfo authority; the
combined aura regression and six existing stat-update regressions pass, as does the
affected test-aware check for `world-server`, `wow-data` and `wow-world`. This does not
populate the legacy `ModPowerRegen` packet field because the C++ path writes the two
`PowerRegen*` fields; full tick/publication, wear-to-broken production and live
capture/DB/relogin parity remain #61 gates.

The next #61 slice (implementation `5bc59ddb`, integrated as `d74381ec` by PR #962)
gives the published regen snapshot a real
consumer. A session-owned tick with the canonical world/map diff runs the C++
`Player::Update → RegenerateAll → Regenerate(POWER_MANA)` chain
(`Player.cpp:1047-1051,1609-1681,1681-1827`): it accumulates
`m_regenTimer`/`m_regenTimerCount`, reads the canonical
`PlayerEffectiveCombatStatsLikeCpp::mana_regen`/`mana_regen_combat` plus the DB2
`PowerTypeEntry` scalars, carries `m_powerFraction`, clamps at min/max and publishes at
the `m_regenTimerCount >= 2000 || forcesSetPower` boundary. The throttled branch writes
current power under `DoWithSuppressingObjectUpdates`/`ClearChanged`, while the boundary
sends the new `SMSG_POWER_UPDATE` (`CombatPackets.cpp:104-116`). `Spell::TakePower`
(`Spell.cpp:5444-5445`) arms the five-second MP5 rule and
`SPELL_AURA_PREVENT_REGENERATE_POWER` is resolved from canonical visible auras. Regen
runtime state lives on the canonical `Unit` to preserve the reviewed `player/mod.rs`
physical ceiling. Six `wow-entities` regen tests, the `wow-packet` layout test and three
production-shaped `wow-world` tests pass; the physical-source ratchet passes and the
pre-existing runtime hotspot drift is not regenerated. Health regeneration, non-mana
powers, `Rate.Mana` config override, observer `SendMessageToSet` packet-type parity,
wear-to-broken production and live capture/DB/relogin parity remain #61 gates.

The first F1 movement-admission slice is integrated by PR #853
(`7c3add2f`, implementation `3af90ec2`). It enforces the C++ early returns for
pending player teleport and unfinished controlled-mover MoveSpline before any
movement side effect, while preserving the existing deferred visibility bridge
from #588. This is a bounded admission correction: #63 remains open for transport
passenger/reset behavior, vehicle and non-creature movers, death/BG/taxi branches,
ack/order, and live client/server/DB capture QA. Do not treat the focused regressions
or local final profile as full movement acceptance.

PR #855 extends this same bounded #63 operation to canonical transport membership
(`10528d45`). The handler removes stale passenger membership before a switch or detach,
adds only an in-world typed Transport owned by the current Map, and clears the
movement transport state when the requested target is absent. PR #859
(`95444be0`) then matches the C++ vehicle passenger turning early return
(`MovementHandler.cpp:408-421`), and PR #860 (`704dc4cb`) applies the stale
transport distance guard to controlled Creature/Pet movers (`MovementHandler.cpp:345-350`).
Both slices pass focused regressions and `validation-v2 final` with 3,881 tests and
zero failures. #63 remains open for complete transport seat/offset admission, other
mover kinds, death/BG/taxi, ACK/order and live capture/QA; preserve the deferred
visibility bridge from #588.

PR #862 (`28762f16`) closes the next measured ACK boundary from
`MovementHandler.cpp:721-739`: `MoveTimeSkipped` now admits the active controlled
mover, advances that mover's own uint32 movement clock and routes `MoveSkipTime`
from its position/GUID, matching `mover->SendMessageToSet`. The controlled-mover
regression, 49-test movement suite, architecture checks, world-server check and
`validation-v2 quick` pass at the recorded manifest. #63 remains open for the
remaining ACK/force/knockback/taxi/death/BG operations, complete transport
seat/offset admission, broader mover kinds and live capture/QA.

PR #864 (`2d375ef1`) closes the following measured force-ACK boundary from
`MovementHandler.cpp:581-663`. Apply, Remove and mod-magnitude ACKs validate the
status against the active `m_unitMovedByMe`, read the expected magnitude from the
active Unit, adjust accepted client time and publish from the mover's position/GUID;
wrong-GUID ACKs are rejected without publication. The controlled-mover regression,
50 movement-handler tests, architecture checks, world-server check and
`validation-v2 quick` pass at the recorded manifest. #63 remains open for ordinary
speed ACKs, knockback, remaining transport/death/BG/taxi operations, broader mover
kinds, complete transport offsets/seats and live capture/QA.

PR #866 (`0079daa8`) closes the next measured knockback-ACK admission boundary
from `MovementHandler.cpp:548-559`. The handler validates through the Player but
accepts the active `m_unitMovedByMe` GUID, then retains the Player-owned movement
info write and Player-source `MoveUpdateKnockBack` publication. A controlled-mover
regression covers acceptance and source identity; the focused test, 50-test
movement suite, architecture checks, `world-server` check and `validation-v2 quick`
pass at the recorded manifest. #63 remains open for ordinary speed ACKs, remaining
transport/death/BG/taxi operations, broader mover kinds, complete seat/offset
admission, captures and live QA.

PR #869 (`abd396a0`) closes the measured offhand admission boundary under #61.
`Unit::DoMeleeAttackIfReady` now requires `!IsInFeralForm()` and
`haveOffhandWeapon()` (`Unit.cpp:2140`); the latter resolves the canonical Player
slot and Item object through `GetWeaponForAttack` (`Unit.cpp:496`,
`Player.cpp:9243-9270`) and rejects absent, non-weapon or broken items. Focused
owner/world regressions, the production world-server check and architecture
ratchets pass. This is a bounded consumer correction; aura-backed combat math,
reversible equipment lifecycle and live/capture acceptance remain in #61.

Continue the remaining core under #584 by complete operations, execution/lifetime
boundaries and physical organization. #582 is closed after its decoder-only delivery;
#486 has its implementation integrated by PR #807 and remains open only for the
action-specific capture/live gate and administration mutations not represented by the
current Rust surface. #524's relation-query order correction is integrated by PR #803;
PR #822/#824/#826/#828/#830/#832/#834 now makes the production WDC4/SQL sequence table-granular through
`SkillLineAbility`, `SkillRaceClassInfo` and `TraitTree`/`SkillLineXTraitTree`, and indexes generic
`TraitSystemID` plus combat class trees for the production session authority, and rejects
invalid persisted node-entry ranks at login. The bounded projection, official/custom overlays,
WDC4 table-hash ownership and final `RecordRemoved`
filtering are implemented through PR #826/#828; Generic and Combat config validation are
integrated by PR #830/#832. PR #836 (merge `0fca1020`, implementation `d9770755`) adds the immutable node/group/edge and
cost/condition/loadout relation projection and makes persisted node/entry topology fail
closed at login. PR #842 (`995cd77f`, implementation `add6650a`) now validates persisted
conditions, costs, parent/rank rules and granted-entry fallback before login publication
from canonical Player facts. PR #846 (`93fa95a9`, implementation `57116f75`) now composes
the 24 C++-projected base Trait/`SpecSetMember` SQL hotfix tables in official-then-custom
order, retains WDC4 table hashes, applies final `RecordRemoved` tombstones and fails
before publication on malformed rows. PR #848 (`179fd5d4`, implementation `95274da1`)
now composes the `trait_definition_locale` and `trait_currency_source_locale` overlays
with exact locale SQL projections, official-then-custom precedence and immutable Player
capability retention. The issue remains open for explicit cross-store coverage,
startup/live DB/relogin evidence, production locale/EffectPoints consumers and later
spending, mutation persistence and starter-build application. Physically reconciled by
PR #811, the issue remains open for complete `TraitMgr` authority acceptance and these
explicit residuals. In parallel with safe
independent work, prepare
a playable circuit: effective equipment/stats → combat and death/recovery →
quests/loot/interactions → complete class kit, travel and durable services → soak.

The core/module acceptance chain remains **required #584 core → #583 → #153**.
The former umbrella #133 is already closed. Its closure is neither evidence that this
chain passed nor an extra future task. The module product need not block every
independent gameplay delivery; its production activation still requires its actual
core prerequisites. Complete the approved architecture/module acceptance before
declaring the whole Part-1 program accepted at #47.

The M0–M6 headings remain milestone identifiers; obsolete `[NN]` priority prefixes
are retired. Neither is an unconditional execution order. Issue numbers, crate names and file counts
do not define dependencies. The tables below distinguish preferred order from
capabilities that actually block a consumer.

## 2. Authority, evidence and limits

- This file and #49 own overall direction and issue allocation.
  [STATE.md](STATE.md) owns dated implementation/evidence status.
- [The refactor completion plan](../architecture/refactor-completion-plan.md)
  owns the detailed P0–P6 continuation; [modularity/ECS design](../architecture/modularity-and-ecs-plan.md)
  and [module design](../architecture/module-design-guidelines.md) own contracts and budgets.
  Issue bodies define bounded delivery and acceptance, not competing master plans.
- Required base behavior comes from target C++ at
  `/home/server/woltk-trinity-legacy`, SHA
  `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, and appropriate target-build captures.
  The pinned complementary AzerothCore reference and its limits are in
  [docs/README.md](../README.md); 3.3.5 wire/SQL are not substitutes for 3.4.3.
- This review read every open issue and traced representative current production
  paths, relevant C++ and existing evidence. It is **not** a full opcode/effect/DB2
  census, a fresh live scenario, a durability proof or a new whole-port parity base.
  Partial source findings and hypotheses are distinguished in the updated issues.
- No historical percentage, count of implemented enums, passing mock or closed
  umbrella proves present capability. Preserve the exact source/runtime identity
  of previous tests and captures; do not relabel them as testing this plan.

## 3. Integrated foundations to reuse

These predecessor issues are closed. Reuse their integrated responsibilities and
inspect their recorded limits when changing a consumer; do not repeat them from an
old unchecked checklist.

| Foundation | Closed issue scope and retained boundary |
| --- | --- |
| Data, terrain, LOS, respawn, persistence and integrity | #14–#20, #52, #60, #62, #64; their bounded delivery does not prove every store/stat/save path. |
| World entry, packet handling and movement/AI | #7–#11, #21–#26, #50, #53, #57, #66. #26 proves its bounded creature cast wire/lifecycle, not full effects or combat AI. |
| Runtime clock cuts and homebind | #28/#371 and #44. They do not complete runtime convergence, all transfers or item-use. |
| Persistence/migration and module foundation | #169/#574/#256 and #228–#231. Reuse SQLx-free contracts, the migration authority and the narrow external login API. |
| Canonical Player, finalization, acquisition, visibility and cast | #578/#585/#587/#588/#589. Preserve residence/incarnation, save fences and metadata; broader gameplay remains open. |
| Recent architecture repair | #716 analyzer; #718 represented quest-reward transaction; #722 named Player operations; #737 item-runtime ownership. Real reward recovery evidence and generic mutation residuals are not discharged by these closures. |

The old high/medium findings in [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md)
are leads with dated evidence. Contrast them before making them current blockers;
retain their owning capability even if a diagnosis proves obsolete.

## 4. Execution lanes and dependencies

A lane describes a result and its acceptance. It does not create one PR per row,
force unrelated work to wait, or permit a partially completed macro to be closed.

| Lane | Preferred work | Entry / exit contract |
| --- | --- | --- |
| **A — Core architecture** | Measured P2/P3/P4 residuals under #584; #743, #735 and #787 delivered | One canonical authority and execution owner, complete consumers, explicit lifetime/persistence/publication and terminal physical/dependency dispositions. See §6. |
| **F1 — Character foundations** | #61, #63; relevant #12, #486 and #524 corrections | Equipment reaches effective stats; movement and entry consume the integrated owners. Weather or all vehicles do not block unrelated combat. |
| **F2 — Combat and recovery** | #29, residual #30, #31; consolidated #43 and #54 | A real fight can reach death, release/recovery and safe logout. Accept numeric combat only with effective stats and the necessary aura/absorb participants. |
| **F3 — Progression and interaction** | consolidated #41; #55, #56, #13, #36 and #51 | Accept → progress → complete/reward, real loot and GO/item interactions. Shared consumers must compose; queues or recorded requests alone do not pass. |
| **F4 — Complete class/simulation kit** | #32, #33, #34, #35, #27 | Executable effect/aura/proc/AI chains with lifecycle and observable outcomes. A participant needed by F2/F3 is delivered there, not delayed merely because its family is listed here. |
| **F5 — Travel and durable services** | #40/#45, remaining #12; #37, #38, #39 | Flight/instance/reconnect coherence; mail, trade and AH complete their transactional lifecycles. These may advance when their real prerequisites are ready. |
| **X — Stateful extension product** | #583 under #99, then #153 audit | Required core accepted first; real native/Rust-Wasm/C-Wasm/mixed behavior, durable module state/reward and author/operator lifecycle. Wider #99 expansion is separate. |
| **F6 — Part-1 terminal acceptance** | #46, then #47 | Integrated functional scenarios, multi-client soak and real save/restart/relogin under load; no known integrity failures deferred to soak. |
| **O — Environment and QA support** | #255, #351; #279/#352 when applicable | Reproducible setup and usable guarded QA. A specific broken fixture/DNS target blocks its live scenario, not every local implementation. |
| **B — Bounded side delivery** | #582 | Reconcile and integrate the existing decoder branch; it does not activate or complete matchmaking. |
| **D — Optional developer experience** | #260 | Typed configuration foundation while preserving current .conf behavior; not a prerequisite for #231, #583 or gameplay. |
| **L — Full-parity continuation** | #48 and source-evidence index #65 | Preserve all remaining target behavior. Refresh the whole-port plan at #47 before creating a detailed Part-2 child tree. |

### Hard capability relationships

- Equipment → effective Unit/Player stats (#61) precedes accepting damage numbers
  in #29/#31. Analysis and preservation of existing combat paths may proceed earlier.
- #13 and #36 use the shared cast contract delivered by #589 and extended by #30;
  each owns its GO/item admission and side effects. They do not clone cast ownership.
- #31 supplies damage/heal application to periodic/proc consumers; required aura,
  absorb or trigger participants are integrated with the affected operation.
  These are shared capability contracts, not a circular demand that all of #31
  and all of #32 must each close before the other can start.
- #43 supplies the complete death/recovery cycle used by combat, instance and
  resurrection scenarios. #54 extends logout admission and reuses #585 finalization.
- #41/#55/#56 share objective-credit integration, with one owner for each
  transition. #743 supplies group-state consistency; #51 retains protocol/lifecycle
  acceptance. Neither relation justifies merging unrelated responsibilities.
- The reward-by-mail criterion of #41 and auction delivery in #39 require the
  relevant durable mail capability of #37. Quests without mail may be developed
  and tested earlier; **#41 must not close while its required mail criterion is
  missing**. Schedule that producer before final quest closeout. Trade #38 does
  not require AH or mail.
- #583 waits for required #584 core; #153 waits for accepted #584 and #583.
  #99 is a product umbrella, not an implementation prerequisite that must close.
- #46 starts after the selected functional flows work; #47 requires their complete
  Part-1 acceptance and #46. Failures found while building an operation are repaired
  there rather than being stored until F6.

## 5. Complete disposition of the initial open issues

“Revalidate” means a scoped current-source/behavior check at the start of delivery,
not another project-wide audit. A consolidation is closed as superseded, with all
acceptance retained by the recipient; it does not mark functionality complete.

| Issue | Primary owner / lane | Disposition and concrete next scope |
| --- | --- | --- |
| [#12](https://github.com/alseif0x/rustycore/issues/12) | F1/F5, entry/transitions | Revalidate transport attachment/re-seat and weather/world-state presentation; transport columns are already loaded. Track remaining entry/cinematic/hotfix presentation findings here when reproduced. |
| [#13](https://github.com/alseif0x/rustycore/issues/13) | F3, GO-use | Integrate the real spell/portal operation from existing GO dispatch and effective templates/scripts. |
| [#27](https://github.com/alseif0x/rustycore/issues/27) | F4, creature reactions | Trace Talk/text loading and AI/script consumers; deliver event, locale/range and multi-recipient behavior. |
| [#29](https://github.com/alseif0x/rustycore/issues/29) | F2, melee | Complete calculation/outcomes with equipment stats and both Player/Creature consumers; reuse current timers/owners. |
| [#30](https://github.com/alseif0x/rustycore/issues/30) | F2, cast prerequisites | Complete residual CheckCast, resources/reagents and history/cooldowns around integrated #589. |
| [#31](https://github.com/alseif0x/rustycore/issues/31) | F2, damage/heal | Calculation → canonical application → threat/death/publication, with exact modifier and failure behavior. |
| [#32](https://github.com/alseif0x/rustycore/issues/32) | F4, aura lifecycle | Apply/update/periodic/remove, real modifiers and required persistence; no historical aura-count target. |
| [#33](https://github.com/alseif0x/rustycore/issues/33) | F4, procs | Trace real events/trigger consumers, filters, RNG/chance/PPM/charges and recursion/failure. |
| [#34](https://github.com/alseif0x/rustycore/issues/34) | F4, spell effects | Complete selected class chains with their targets, auras, movement and summons; preserve full-port coverage separately. |
| [#35](https://github.com/alseif0x/rustycore/issues/35) | F4, channels/missiles/areas | Runtime lifecycle and effects, beyond the existing packet shapes; reuse the execution clock. |
| [#36](https://github.com/alseif0x/rustycore/issues/36) | F3, item-use | ItemEffect is loaded; deliver registered item admission, cast, charges/consumption and real effects. |
| [#37](https://github.com/alseif0x/rustycore/issues/37) | F5, mail | Complete online/offline mail, attachments/money/COD and durable recovery; supply reward/AH consumers. |
| [#38](https://github.com/alseif0x/rustycore/issues/38) | F5, trade | Complete two-Player item/gold exchange beyond the existing accepted-state protocol. |
| [#39](https://github.com/alseif0x/rustycore/issues/39) | F5, AH | Complete target-version auction lifecycle and mail delivery, not a response invented for a legacy no-op opcode. |
| [#40](https://github.com/alseif0x/rustycore/issues/40) | F5, taxi | Discovery/route/payment → flight/map transitions → landing/recovery; existing taxi state is not flight execution. |
| [#41](https://github.com/alseif0x/rustycore/issues/41) | F3, quest lifecycle | Receives #58/#59: admission, acceptance/sharing/source items, objectives, completion and full in-scope reward; reuse #718. |
| [#42](https://github.com/alseif0x/rustycore/issues/42) | Superseded by #43 | Transfer death, durability and ghost restrictions into the complete recovery macro; close administratively, not as implemented. |
| [#43](https://github.com/alseif0x/rustycore/issues/43) | F2, death/recovery | Receives #42: death/CORPSE → release/ghost → corpse/graveyard → reclaim/healer/resurrection and relog. |
| [#45](https://github.com/alseif0x/rustycore/issues/45) | F5, instances | Revalidate bind/save/difficulty/reset and actual admission; finish enter/leave/reconnect using existing transfer/finalization. |
| [#46](https://github.com/alseif0x/rustycore/issues/46) | F6, soak | Measured multi-client stability on an identified installed build, after incremental operation acceptance. |
| [#47](https://github.com/alseif0x/rustycore/issues/47) | F6, playable exit | Real save/recovery under load and all required functional flows; triggers the next full-port planning pass. |
| [#48](https://github.com/alseif0x/rustycore/issues/48) | L, full-parity umbrella | Retain complete coverage, remove stale percentages; add explicit social/LFG and authentication/network coverage. |
| [#49](https://github.com/alseif0x/rustycore/issues/49) | Master index | Maintain this direction and complete allocation; not an implementation PR or a future prerequisite to its children. |
| [#51](https://github.com/alseif0x/rustycore/issues/51) | F3, group lifecycle | Revalidate current invite/accept/decline/cancel/leave/disband/category behavior; separate from #743 delivery guarantees. |
| [#54](https://github.com/alseif0x/rustycore/issues/54) | F2, logout admission | Deny/delay/instant/cancel/countdown using the existing durable finalizer. |
| [#55](https://github.com/alseif0x/rustycore/issues/55) | F3, loot | Existing gates are not globally missing; finish actual modifiers/grants/credit and multi-client consistency. |
| [#56](https://github.com/alseif0x/rustycore/issues/56) | F3, area-trigger | Bits, conditions, scripts and tavern paths exist; trace residual explore/BG/corpse/transfer semantics. |
| [#58](https://github.com/alseif0x/rustycore/issues/58) | Superseded by #41 | Preserve timed-active exclusivity and recursive breadcrumb admission as explicit quest criteria. |
| [#59](https://github.com/alseif0x/rustycore/issues/59) | Superseded by #41 | Preserve acceptance/completion/reward participants and evidence, without redoing #718. |
| [#61](https://github.com/alseif0x/rustycore/issues/61) | F1, equipment/stats | **Projection and first consumers integrated by PR #851 (`b26ce713`), #857 (`fd302c35`), #858 (`cda7f8a0`) and #869 (`abd396a0`); issue remains open.** Exact offhand admission now follows `Unit.cpp:496,2140,8807-8812` and `Player.cpp:9243-9270`, including broken-item and feral-form gates. Finish aura-backed producers, exact C++ AP/damage/aura/regen/expertise/penetration paths, reversible equip/unequip and broken/repair lifecycle, then capture/live DB/relogin acceptance before accepting combat numbers. |
| [#63](https://github.com/alseif0x/rustycore/issues/63) | F1, movement | **Admission + transport/ACK slices integrated by PR #853/#855/#859/#860/#862/#864/#866:** pending teleport and unfinished controlled-mover spline fail closed before side effects; canonical Map-owned transport membership switches/detaches safely and resets missing targets; vehicle passenger turning follows the C++ early return; stale transport distance validation covers controlled Creature/Pet movers; `MoveTimeSkipped`, movement-force Apply/Remove/mod-magnitude, ordinary force-speed and knockback ACKs validate, advance/check and publish from the active controlled mover/Player source. Continue complete vehicle/transport seat-offset admission, runtime branches whose mover/consumer is not represented, exact packet-order captures and live client/server/DB QA; preserve #588 deferred visibility. |
| [#65](https://github.com/alseif0x/rustycore/issues/65) | L, source-evidence index | Correct finding/issue status and retain exact C++ provenance; not a separate implementation queue or fresh count. |
| [#99](https://github.com/alseif0x/rustycore/issues/99) | X, module ecosystem | #583 is the selected stateful product; wider language/WIT/hot-reload proposals remain later capability-led planning. |
| [#153](https://github.com/alseif0x/rustycore/issues/153) | X/A, terminal audit | Audit accepted #584/#583 and their evidence; do not absorb known implementation work or await #133 reopening. |
| [#255](https://github.com/alseif0x/rustycore/issues/255) | O, reproducible bootstrap | Reuse integrated migration/status authority; deliver pinned artifact, cache/offline import and setup diagnostics. |
| [#260](https://github.com/alseif0x/rustycore/issues/260) | D, typed JSON config | Optional bounded foundation on the current toolchain; keep .conf/overlays/environment semantics and offline validation. |
| [#279](https://github.com/alseif0x/rustycore/issues/279) | O, QA credential rotation | Explicit disposable-fixture recovery with DB/file failure handling; create-only provisioning remains the default. |
| [#351](https://github.com/alseif0x/rustycore/issues/351) | O, guarded loot QA | Reconcile current runtime/capture orchestration and chest fixture ownership; preserve restore guarantees and prove the actual smoke. |
| [#352](https://github.com/alseif0x/rustycore/issues/352) | O, realm address operations | Revalidate configured DNS/IP and restart diagnostics. No silent fallback or code change inferred from the historical incident. |
| [#486](https://github.com/alseif0x/rustycore/issues/486) | F1, target identity query | **Implementation integrated by PR #807 (`86a0eb97`).** The canonical cache/connected-target path returns target game/BNet identities; keep open for action-specific packet/live evidence and unrepresented undelete/barber mutation coverage. |
| [#524](https://github.com/alseif0x/rustycore/issues/524) | F1, TraitMgr functional authority | PR #803 fixed the relation-query order; PR #822/#824 (`7bb9a911`) separates the WDC4/SQL stages, PR #826 (`d934451a`) applies official/custom `SkillLineXTraitTree` overlays, PR #828 (`276e3981`) retains the WDC4 table hash and applies final table-scoped removals, PR #830 (`a9623787`) indexes `TraitSystemID` trees and validates Generic configs, PR #832 (`1143ed41`) indexes `ChrSpecialization` class masks and validates Combat configs, and PR #834 (`4e3ad8f0`) rejects missing or over-ranked persisted node entries, all in production order `SkillLine` → `SkillLineAbility` → `SkillRaceClassInfo` → `TraitTree`/`SkillLineXTraitTree`, preserving fail-before-publication boundaries. PR #836 (`0fca1020`) adds the immutable node/group/edge/cost/condition/loadout graph. PR #842 (`995cd77f`, implementation `add6650a`) adds C++-aligned semantic condition/cost/parent/rank validation and deterministic granted-entry fallback before login publication from canonical Player facts. PR #846 (`93fa95a9`, implementation `57116f75`) composes all 24 C++-projected base Trait/`SpecSetMember` SQL hotfix tables with official-then-custom precedence, WDC4-hash-scoped removals and fail-before-publication checks. PR #848 (`179fd5d4`, implementation `95274da1`) adds both locale overlays with exact SQL projections and immutable Player capability retention. Remaining gates are explicit cross-store fail-before-publication coverage, startup/live DB/relogin evidence, production locale/EffectPoints consumers, real spending/mutation persistence and starter-build application. |
| [#582](https://github.com/alseif0x/rustycore/issues/582) | B, existing LFG decoders | **Closed/integrated as `21686375` (PR #797).** Six C++-faithful client decoders; no handler, queue or matchmaking claim. |
| [#583](https://github.com/alseif0x/rustycore/issues/583) | X, stateful native/Wasm | Deliver the preserved M0–M4 product after required core; the external login API and laboratory are insufficient. |
| [#584](https://github.com/alseif0x/rustycore/issues/584) | A, core coordinator | Own remaining P2/P3/P4 and C0–C4 dispositions. PR #816 integrates the P2 item-bonus writer retirement: resolved state application is a named Player-owned operation and the generic `&mut PlayerItemBonusStateLikeCpp` bridge is gone. Candidate `23a7fe16` adds the item-object P2 closure retirement: all production item-object mutations use the Player-owned closed command set, with fixture-only closure access retained. P3.4–P3.10 are integrated in the current delivery, including directed ordinary Creature DESTROY after map removal (PR #820, `62c1369f`) and receiver-filtered Player/Unit VALUES fanout after `Map::SendObjectUpdates` (PR #871, `304f482b`), corrected by PR #873 (`bd5b13d4`) to exclude those families from the generic map rail and fence the post-lock MapKey. P3.11 (PR #925, merge `a1f66c33`) includes indexed canonical Pets in the Creature CREATE discovery path while retaining the common phase/range/detection filters; P3.12 extends the directed DESTROY rail to Pets with the same map-incarnation and `HaveAtClient` fences. Neither slice migrates Pet AI, movement, summon lifecycle, persistence or vehicle/corpse/transport ownership. PR #876 (`ed92d14f`) projects `Player::m_visibleTransports` through the registry and uses it for Transport VALUES fanout, with focused producer/consumer regressions. PR #878 (`d8cb0594`, implementation `ceb58c9a`) gives the Player-owned mount VehicleKit named install/snapshot/clear/remove/eject transitions, while Session retains admission and publication effects. PR #881 (`2a916c1c`, implementation `3e1bdd9d`) gives Player named aura-authority, visible-aura and threat-aura transitions over the Unit-owned `AuraSubsystem`; the generic Session adapter is fixture-only. PR #883 (`0f79ca83`, implementation `50e98f43`) closes the represented TradeData owner surface; Session keeps admission, packet and partner-mailbox adapters. PR #885 (`19dea8e`, implementation `f0d32675`) closes the represented guild membership/invitation/rank owner surface, following `Player.cpp:7216` and `Player.h:1939,1943`; its broad composite install is retired and fixture mutation remains `cfg(test)` only. PR #887 (`72f6a3fa`, implementation `4bd82511`) closes the represented Battleground type/map/status/queue and arena-invitation Player owner surface, following `Player.h:976,2335-2338,2821`, `Player.cpp:24258-24262` and `Player.h:1956`; Session retains queue admission, matchmaking/lifecycle, packets and application effects, and the whole-state mutator remains fixture-only. PR #891 (`faa5964b`, implementation `a2a32586`) closes the Player taxi post-teleport advance and flight-cleanup owner transitions; Session retains map/spline/application effects and route generation, persistence, captures and live QA remain open. PR #893 (`cd054d5a`, implementation `89c66ada`) closes the Player world-local zone/area, terrain-authority, PvP-hostility/timer and outdoors writer transitions; Session retains terrain/catalog resolution and application effects, while full admission, persistence, captures and live QA remain open. The legacy Creature writer, AI/combat, scripts, FlyByCamera, shared-raid field flags, remaining Pet/corpse/transport lifecycle parity and exact capture/live gates remain explicit later boundaries. |
| [#735](https://github.com/alseif0x/rustycore/issues/735) | A, reputation boundary | **Closed/delivered.** Player owns reputation state and named transitions; catalogs, packets and persistence consumers remain outside the domain boundary. |
| [#743](https://github.com/alseif0x/rustycore/issues/743) | A, group consistency | **Closed/delivered.** GroupRegistry remains authoritative and dropped state-bearing commands converge through the session boundary. |
| [#787](https://github.com/alseif0x/rustycore/issues/787) | A, session-phase coordination | **Integrated as `d14a9a67` (PR #792; accepted at `76369bda`).** World runs before Map, phase permits remain live through finalization/retirement, and shutdown/replacement barriers are covered by production-linked tests and guarded login/save/relogin QA. |

**#584 Transport visibility implementation integrated (2026-09-14, PR #901):** the typed
transport owner now feeds CREATE/OUT-OF-RANGE snapshots from canonical map state into
the deferred Session rail. Add/remove marks same-phase map-reference recipients;
refresh publishes the packet and the separate `m_visibleTransports` membership in one
transition, with map/instance/incarnation checks retained by the existing consumer.
The focused map and bridge regressions plus the `wow-world` check pass at merge
`bf460aa7a8ccec0269eea1771a094ef12f0c6109`. Map relocation via
`Transport::TeleportPassengersAndHideTransport`, passenger seat/offset admission,
AI/scripts, taxi routing and live capture/DB acceptance remain explicit #63/#584
gates; no Creature-owner migration is included.

**Planning delivery:** [#748](https://github.com/alseif0x/rustycore/issues/748) owns
this documentation/issue reconciliation and its validation. Closing it does not close #49
or any gameplay acceptance. #42/#58/#59 retain their history and redirects to recipients.

## 6. Finish architecture without another endless rewrite

### A1 — Group consistency and reputation (delivered)

#743 and #735 are closed in their bounded scopes. Their retained contracts are the
single GroupRegistry authority with convergence for dropped state-bearing commands,
and Player-owned reputation state with catalog/packet resolution outside the domain.
Do not reopen either as a generic mailbox rewrite or a whole-manager move.

### A2 — Remaining application and persistence boundaries

Use the actual generic Player/item mutation callers in the ownership ledger.
For each complete operation, migrate its rules, readers, writers, persistence and
publication; retire broad access or justify a bounded stable seam under the existing
policy. Replacing a closure with a differently named generic closure is not retirement.
A short canonical access adapter is not automatically an independent gameplay owner.

Preserve #718's represented quest transaction and #585's finalization. Complete the
remaining account/character, save/acknowledgement, unknown-COMMIT, cancellation and
recovery obligations where their promised contract is not discharged. Real DB/restart/
relogin evidence remains necessary; source guards and controlled futures alone are
not a durability claim. Unrelated missing gameplay is allocated to its functional macro.

### A3 — Production execution, lifetime and private storage

Trace startup and the current Session/map/legacy calls, using the dated
[clock/phase trace](../architecture/runtime-clock-phase-trace.md) and the fresh
[Creature runtime audit](../architecture/creature-runtime-audit.md). The current
source still selects GlobalLegacy for its creature path and starts the canonical
map loop; the `ExternalRuntime` branch prevents a second timer writer, but this is
not by itself C++ phase convergence or a demonstrated complete Creature consumer.
The #787 coordination contract is accepted. The P2 item-modifier writer is also
closed as an ownership residual: `PlayerItemModifierRuntimeStateLikeCpp` applies
resolved state through a named operation and Session no longer lends its bonus
record to a generic closure. The item-object residual is likewise closed in `3.4.3` by
PR #838 (`ef82beeb`): `PlayerInventoryRuntime::apply_item_object_updates_like_cpp` is the only
production mutation entry and receives data commands rather than lending an item. Reconcile admission, phases/barriers,
one resolution, backpressure and transfer/detach/unload/shutdown before removing
another bridge. P3.3 integrated the finite phase correction: after the admitted map
session pass, run `ProcessRespawns` and `UpdateSpawnGroupConditions` before object
visitors, using map incarnations from the tick plan so a replacement map cannot
inherit work. P3.4 now carries canonical player/viewpoint, represented
far-combat/aura/summon and active-non-player sources into one deduplicated nearby
selection, invokes existing per-object consumers only for selected in-world objects,
preserves the all-transport loop and C++ ordering, and keeps delivery outside map
guards. P3.5 resolves `WorldObject::GetGridActivationRange` from the canonical source:
Players and active objects use the map visibility range, inactive Creatures/Pets use
their `m_SightDistance`, and unsupported or missing records fail closed. P3.6 adds
the Player cinematic override as `max(DEFAULT_VISIBILITY_INSTANCE,
Map::GetVisibilityRange())` once the represented camera cursor is active. The Rust
selector still does not own FlyByCamera lookup or cinematic movement. P3.7 reuses the
same source list and per-source activation radius for relocation marking, then turns
affected Players from `CreatureRelocationVisibilityPlan.player_visibility_updates`
into one coalesced deferred visibility intent per Player. The legacy Creature writer
and its complete effect consumer remain a separate migration boundary. P3.8 now marks
nearby in-world Players from `Map::AddToMap` and `Map::RemoveFromMap` before/while the
source is attached, reusing the same deferred intent rail; this supplies recipient
selection without packet delivery under a map mutation. P3.9 now captures ordinary
Creature DESTROY recipients at removal and publishes one typed command per current
Session after the map guard, fenced by map incarnation and `HaveAtClient`; P3.11 adds
canonical Pet CREATE discovery through the same Creature snapshot path, and P3.12
captures Pet DESTROY recipients before erasure and publishes the existing directed
command after the guard. P3.13 now uses the same typed object rail for Corpse
removal. Pet runtime/owner lifecycle, corpse reclaim/persistence/loot, transport
fanout and live capture remain separate acceptance gates.

Keep the selected private hecs direction and finite V2 conformance evidence.
Integrate it only with real owners/consumers and the lifetime/reentry contract;
no global ECS, public raw storage API, extra runtime clock or dependency-only “migration.”
P3.1 and P3.2 are integrated bounded contracts: the Creature owner is explicit and
the discarded canonical Creature plan is skipped under the legacy/session owner;
canonical `SendObjectUpdates` snapshots are published after guards are released.
P3.3 preserves those owners while correcting respawn/condition phase order and the
admitted-incarnation boundary. P3.4 is a delivered selection/phase correction, not
Creature AI migration: it retires the map-wide typed-store scans in the production
path, covers nearby inclusion and out-of-cell exclusion, and leaves a measurable
retirement path for the remaining unrepresented C++ sources. P3.5 closes the
source-specific Creature/Pet activation-radius mismatch without moving the AI owner.
P3.6 is a selection correction only; P3.7 is a visibility-intent fanout correction
only. Neither promotes cinematic movement, script dispatch, directed CREATE/DESTROY
publication, or Creature AI into the current macro. The fresh audit now defines
C3.1 as the next finite migration contract: typed map-tick input/outcome,
authority/incarnation fences, one execution order and a production-linked failure
suite. It must land before retiring the clone bridge or claiming
`runtime_update_plan` is consumed. Define later behavior contracts from traced
consumers under #584/#29/#31/#32/#33/#34, never as a speculative issue per bridge
or crate.

### A4 — Physical and dependency closeout

Apply the same semantic/physical policy to production, tests, fixtures, adapters,
composition and tools. Replace arbitrary numerical file partitions with cohesive
responsibilities when closing their family; do not conduct a blind global rename.
Private modules precede earned crates. Preserve visibility, exact registration sets
and persistence inventories while retiring the real legacy accesses.

Migration non-growth is not terminal acceptance. Measure remaining oversized files,
logical owners, permitted dependency exceptions and production bridges at closeout;
every residual needs the policy's specific accepted disposition. #153 audits that
result; it is not the implementation owner of known splits.

## 7. Part 1 acceptance — a complete playable circuit

The M0–M6 names remain milestone identifiers. Their broad exits cannot be inferred
from a child issue's closure or from a single working spell/class.

| Milestone | Required integrated outcome |
| --- | --- |
| M0 foundations | Effective data/stats, terrain/LOS/pathing, respawn and persistence/integrity prerequisites used by the selected scenarios. Preserve accepted predecessor coverage and identify residuals. |
| M1 entry | Create/login, bags/UI and correct identity/initial publication, with no client/Lua failures and scoped capture-clean entry flows; #12/#486 own relevant remaining paths. |
| M2 world | Movement, visibility, patrol/path/aggro/evade, abilities, reactions and respawn; multiple clients agree. Bounded #26/#28 acceptance is not this full exit. |
| M3 combat/classes | Normal class rotations, melee/spell outcomes, resources/cooldowns, periodic effects/CC/procs and required channels/areas, with effective stats and correct death/recovery consequences. |
| M4 interaction | GO/item-use, common quest objective and reward types, groups/loot, mail/trade/AH and flights work end-to-end. Earlier quest development does not remove commercial services from this exit. |
| M5 lifecycle | Death/release/corpse/resurrection, homebind, transport/weather/world state, instances and deny/delay/cancel logout; reconnect/relogin preserve the correct outcome. |
| M6 stability | Multi-session/map soak and actual periodic-save/recovery under load; no lost acknowledged state, duplicate grant, known unresolved integrity defect or unbounded queue failure. |

At #47, record a concrete scenario matrix with target client/build/data, representative
class/race/level/map coverage and every required milestone exit. One successful class
or login is evidence for that case, not all Part 1. Preserve wider target coverage for
Part 2; any scope adjustment needs an explicit retained owner, not a smaller headline.

The environment for required QA must be reproducible and its addresses/fixtures usable.
That requirement is not a claim that all optional authoring tooling (#260), the full
#99 ecosystem or a production LFG queue must be finished to call Part 1 playable.

## 8. Part 2 — full target parity, with an explicit coverage map

#48 retains the complete target beyond the playable cases. The old numeric
baselines (385/631 handlers, 42/150 effects, 5/255 auras, 110/325 stores and script LOC)
are historical and are retired as current status or fixed acceptance denominators.
Measure the target-version inventory and real consumers when the relevant domain is
audited. A registered opcode, represented request or loaded table alone is not coverage.

| Ledger | Required target coverage |
| --- | --- |
| L1 | Client packet handlers, metadata/admission, operation and errors. |
| L2 | Server packet layouts, values, recipients, connection and observable order. |
| L3 | Spell effects and their complete application chains. |
| L4 | Aura types, periodicity, stacking/removal, modifiers and proc interactions. |
| L5 | Required DB2/DBC/GameTable catalogs, effective overlays/removals and consumers. |
| L6 | Creature AI families and lifecycle, beyond bounded combat slices. |
| L7 | SmartAI event/action/target execution and script integration. |
| L8 | Movement generators and transport/taxi/vehicle integration. |
| L9 | Terrain, pathfinding, LOS, collision and required extraction/data support. |
| L10 | Conditions and their actual operation consumers. |
| L11 | Phasing and visibility refresh across state/lifecycle changes. |
| L12 | Complete item/inventory/bank/equipment/durability/buyback/gift behavior. |
| L13 | Complete quest rules, objective types, rewards and persistent state. |
| L14 | Mail, calendar, petitions and their delivery/lifecycle. |
| L15 | Target-version auction house and associated behaviors. |
| L16 | Battlegrounds, arenas, battlefields and outdoor PvP. |
| L17 | Instances, save/bind/difficulty/lockouts/resets and raids. |
| L18 | Achievements, reputation, skills, talents/glyphs, titles and progression. |
| L19 | Pets, vehicles, totems and related AI/state/lifetime. |
| L20 | Required first-party content scripts by family, independently of optional modules. |
| L21 | Warden/anticheat and target-supported enforcement. |
| L22 | UpdateField values and derived state, not only layouts. |
| L23 | Supported server configuration and effective runtime consumers. |
| L24 | Database statements/loaders, schemas, transactions and recovery. |
| L25 | Remaining production runtime, grids, visibility/object updates and legacy retirement. |
| L26 | Source-reference verification, including the historical #65 findings. |
| **L27** | **Explicit social coverage:** groups/raids, guilds, friends/ignores/channels and automatic LFG, beyond #51/#582. |
| **L28** | **Explicit authentication/network coverage:** bnet/world account and realm flows, session/connection lifecycle and target-required security/protocol behavior. |

L27/L28 make previously implicit coverage visible; they do not create child issue
trees or promise unrelated platform features. The map is a planning coverage aid,
not proof that every target operation has already been inventoried.

### Part 2 transition gate

This review establishes the full direction **now**. After #47/M6.2, refresh the
whole-port inventory against that later integration and versioned C++/data/captures,
then decompose the remaining L-ledgers into complete implementation macros.
Do not create hundreds of speculative child issues today. Existing justified
functional work may proceed earlier through its actual dependencies.

The owner closing #47 records the new review base and hands the remaining scope
to #48/#49. Reconcile delivered behavior and evidence before calculating any coverage
denominator or proposing a first Part-2 implementation.

## 9. Delivery, validation and maintenance

1. Select a ready complete responsibility from this plan. Reuse its issue/branch;
   declare exact operation, current consumers, source/data contract, dependencies,
   retirement list and acceptance. Distinguish unknowns from established gaps.
2. Finish the implementation, consumers and tests with coherent internal commits.
   Keep behavioral repair separate from structural movement. No issue/PR per helper,
   routine approval round or partially implemented macro passed off as complete.
3. At delivery acceptance, run affected unit/production-linked/failure tests,
   architecture/metadata checks and the applicable final profile. The parent or one
   assigned executor schedules heavyweight validation sequentially on the shared host.
4. Changed bytes/metadata/connection/order need scoped packet/capture evidence;
   new action-specific live scenarios differ from regression goldens. Randomized
   combat needs controlled inputs/RNG or justified distribution/causal checks.
   Real durability claims need real DB/restart/relogin evidence.
5. Update the owning issue/checkpoint and current status with exact tested SHA,
   command, host, result and limits. Remove superseded accesses/baselines only from
   reviewed semantic evidence. Preserve prior evidence identities and user work.
6. Close an implementation issue only after integrated scoped acceptance. Close
   superseded tracking issues administratively with full scope transferred and
   backlinks. Do not close #49 merely because a planning PR lands.
7. Reconcile this index after a macro lands, a dependency changes or a concrete new
   defect alters priority. Do not append another contradictory “current plan.”
   Urgent integrity faults move ahead of preferences; an unverified historical
   diagnosis does not silently become a new global blocker.

Use [AGENTS.md](../../AGENTS.md) and [validation-v2](../operations/validation-v2.md)
for the actual commands, authority and evidence reuse. Planning changes create no
new runtime result. Report **implemented**, **integrated** and **parity-proven**
separately; do not replace acceptance with percentages of files, fields or closed issues.
