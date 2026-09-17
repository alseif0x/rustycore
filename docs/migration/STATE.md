# RustyCore — Honest Current State (single source of truth)

**Integration head — 2026-09-17:** `3.4.3` is at
`d3583610968219ce18e22cf933045f62899d3832` (PR #1095, the #29 victim conditional
critical-chance auras, following PR #1093, the #29 victim avoidance
auras, PR #1091, the #29 victim-side melee
damage-taken chain, PR #1089, the #29 melee block band,
PR #1087, the #29 melee attack
table, PR #1085, the #29 white-swing armour
mitigation, PR #1083, the #29 white-swing
damage roll, PR #1081, the #65 GameObject
respawn-save test fidelity fix, PR #1079, the #61 victim
aurastate/aura-mechanic melee bonuses, PR #1077, the #61 melee
creature-type damage bonus, PR #1075, the #61 cast-speed auras, PR #1073, the #61 white-swing auto-attack damage aura, PR #1071, the #61 displayed-power ownership, PR #1069, the #61 `HandleShapeshiftBoosts`, PR #1067, the #61 form-change item effect refresh, PR #1065, the #61 shapeshift form ownership and `CombatRoundTime`, PR #1063, the #61 attack-speed aura application, PR #1061, the #61 `CONFIG_STATS_LIMITS_*` caps, PR #1059, the #61 `BonusCoefficientFromAP` table term, PR #1057, the #61 `SpellHealingPctDone` completion, PR #1055, the #61 remaining `SpellDamagePctDone` terms, PR #1053, the #61 health-derived unit aura states, PR #1051, the #61 mechanic-based damage multipliers, PR #1049, the #61 versus-aurastate damage multiplier, following PR #1047, the creature missing-health heal scaling, PR #1045, the versus-creature-type damage multiplier, PR #1043, the missing-health healing scaling, PR #1041, the `SpellHealingBonusTaken`, PR #1039, the victim `ModHealing` term, PR #1037, the direct-heal spell-power bonus, PR #1035, the school damage percentage, PR #1033, the caster spell-power damage bonus, PR #1031, the weapon-enchantment damage term, PR #1029, the ranged weapon fit, PR #1027, the `Unit::UpdateDamageDoneMods` representation, PR #1025, the `UpdateDamagePctDoneMods` representation, PR #1023, the `VersatilityBonus` publication, PR #1021, the override percentage publication, PR #1019, the `ModTargetResistance`/spell-penetration publication, PR #1017, the `ModHealingDonePercent` publication, PR #1015, the `ModDamageDonePercent` publication, PR #1013, the narrow values-update negative spell field, PR #1011, the spell field wire publication, PR #1009, the spell damage/healing done producers, PR #1007, the override-attack-power-by-spell-power aura, PR #1005, the seven stale `wow-world --lib` expectations, PR #1003, the quest party fixture identity fix, PR #1001, the save-snapshot manager-lock re-entry fix, PR #999, the session reputation-closure lock re-entry deadlock fix, PR #997, the collection appearance `CanUseItem` template gates, PR #995, the collection appearance weapon-proficiency gate, PR #993, the #61 attack power aura producers, PR #991, the #61 school resistances, PR #989, the #61 critical-strike aura percentages, PR #987, the #61 avoidance aura percentages, PR #985, the #61 armor aura producers, PR #983, the #61 `Unit::m_transformSpell`/`IsPolymorphed` owner, PR #980, the #61 aura-backed per-attack expertise, PR #978, the #61 food/drink regeneration emote visual, PR #976, the #61 observer `SMSG_POWER_UPDATE` fan-out, PR #974, the #61 creature-kill durability loss, PR #972, the #61 durability-damage spell effects, PR #970, the #61 fall-death item durability loss, PR #968, the #61 C++ regeneration rates, PR #966, the #61 non-mana power-regeneration loop, PR #964, the #61 health-regeneration tick, PR #962, the #61 mana-regeneration docs sync, PR #960, PR #959/#958, docs-only PR #956, and PR #957/#955/#954/#953/#950/#948/#935/#933/#931/#929/#926/#925/#924/#923/#922/#921/#904/#902/#901/#899/#897/#895/#893/#891/#889/#887/#885/#876/#873/#871/#869/#866/#864/#862/#860/#859/#855/#854/#853, PR #851, PR #848, PR #846, PR #844 and PR #842). The entries below preserve
dated evidence and limits; they do not select an already integrated macro again.
The active architecture sequence is the remaining measured work in #584, followed
by the stateful module product #583 and the independent audit #153. #582 and
#587–#589 are closed in their bounded scopes; #486 and #524 remain open only for
the residual acceptance explicitly stated below.

**#29 victim conditional critical-chance auras — 2026-09-17, implementation
`accffc17`, integrated as `d3583610` by PR #1095:** C++
`Unit::GetUnitCriticalChanceTaken` (`Unit.cpp:2424-2452`) adds the victim's
`SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH` (183) while
`!HealthBelowPct(MiscValueB)` holds and its
`SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER` (306) while the attacker is the caster,
on top of the melee-crit sums delivered by PR #1093.
`AppliedAuraEffectLikeCpp` now carries the effect's `MiscValueB`, and
`RepresentedMeleeVictimFactsLikeCpp` gained
`crit_chance_vs_target_health_pct` and `crit_chance_for_caster_pct`, folded into
the crit band by `melee_outcome_inputs_like_cpp`; both owners apply the C++
predicates from the creature's health percentage and the attacker GUID. Boundary:
the `SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER_PET` branch needs a `TempSummon`
attacker, which the represented player swing is not. Evidence: the fact test pins
crit 25 from the two conditional sums,
`white_swing_applies_victim_critical_chance_auras_like_cpp` proves a foreign
caster's aura is ignored and the attacker's own applies (7 damage normal, 14 with
`HITINFO_CRITICALHIT` from the 100% target-health aura), and the production owner
`map_owned_player_melee_applies_victim_critical_chance_auras_like_cpp` holds the
map-owned tick to the same crit; wow-packet 744/0, wow-data 753/0, wow-entities
940/0, wow-world 3982/0/1 and world-server 594/0/0 pass; format,
`git diff --check` and the physical ratchet pass without new ceiling growth, and
`validation-v2 quick` (manifest `20260917T150133.551794Z-2926042-quick.json`)
passes. `validation-v2 final` stops only at the pre-existing `hotspot-ratchet`
baseline failure (manifest `20260917T150216.515054Z-2926250-final.json`). No live
DB/restart/relogin QA. The victim-side attack-table terms are now complete apart
from the casting/control avoidance gate; #29 remains open for the remaining
spell/melee math.

**#29 victim avoidance auras — 2026-09-17, implementation `28622fb0`, integrated
as `4cbacecc` by PR #1093:** C++
`Unit::GetUnitDodgeChance`/`GetUnitParryChance`/`GetUnitBlockChance`
(`Unit.cpp:2313-2360`) add the victim's `SPELL_AURA_MOD_DODGE_PERCENT` (49),
`SPELL_AURA_MOD_PARRY_PERCENT` (47) and `SPELL_AURA_MOD_BLOCK_PERCENT` (51);
`MeleeSpellMissChance` subtracts the victim's
`SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE` (184);
`GetUnitCriticalChanceTaken` adds `SPELL_AURA_MOD_ATTACKER_MELEE_CRIT_CHANCE`
(187) plus `SPELL_AURA_MOD_ATTACKER_SPELL_AND_WEAPON_CRIT_CHANCE` (197); and
`GetUnitDodgeChance` adds the attacker's `SPELL_AURA_MOD_COMBAT_RESULT_CHANCE`
(248) `VICTIMSTATE_DODGE` row and `SPELL_AURA_MOD_ENEMY_DODGE` (251). Those were
the attack table's documented "no represented producer" boundary, which round
57's `creature_aura_effects_like_cpp` now retires:
`RepresentedMeleeAttackerFactsLikeCpp` gained `dodge_reduction_pct`,
`RepresentedMeleeVictimFactsLikeCpp` the five victim aura sums, and
`melee_outcome_inputs_like_cpp` folds them into the same bands. Both owners
resolve them — the session from the target creature's applied auras plus the
attacker's aura sums, the map-owned `GlobalLegacy` runtime from its creature
snapshot and the attacker's auras. Boundary: the victim-side conditional crit
modifiers (`MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH`,
`MOD_CRIT_CHANCE_FOR_CASTER`/`_PET`) and the casting/control avoidance gate still
have no represented producer. Evidence: the fact test pins
`MOD_DODGE_PERCENT` +100, `MOD_PARRY_PERCENT` -4, `MOD_BLOCK_PERCENT` +2, a -1
attacker dodge reduction and the hit/crit aura sums (dodge 101.5, parry 1.5,
block 5, crit 15); `white_swing_applies_victim_avoidance_auras_like_cpp` forces a
guaranteed dodge in the session owner and
`map_owned_player_melee_applies_victim_avoidance_auras_like_cpp` holds the
map-owned tick to the same outcome with unchanged creature health; wow-data
753/0, wow-world 3980/0/1 (two consecutive runs) and world-server 594/0/0 pass;
format, `git diff --check` and the physical ratchet pass without new ceiling
growth, and `validation-v2 quick` (manifest
`20260917T145024.092238Z-2913639-quick.json`) passes. `validation-v2 final` stops
only at the pre-existing `hotspot-ratchet` baseline failure (manifest
`20260917T145121.294308Z-2913958-final.json`). No live DB/restart/relogin QA. #29
remains open for the remaining spell/melee math.

**#29 victim-side melee damage-taken chain — 2026-09-17, implementation
`45b7571c`, integrated as `9bcb8bef` by PR #1091:** C++
`Unit::CalculateMeleeDamage` (`Unit.cpp:1326-1334`) runs the victim's
`Unit::MeleeDamageBonusTaken` (`Unit.cpp:1687-1759`) between the done bonus and
the armour reduction; the represented white swing skipped the whole victim-side
chain because no rule could read a creature's applied auras.
`session_rules::creature_aura_effects_like_cpp` now resolves every active effect
of a creature's `AppliedAuraRef` list at the caller's difficulty (the player
projection's counterpart, and the first creature-aura-effect consumer beyond the
mechanic mask), and `session_rules::melee_damage_taken_flat_pct_like_cpp`
assembles `MeleeDamageBonusTaken` for a white swing: the normal-school
`SPELL_AURA_MOD_DAMAGE_TAKEN` (14) and `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN` (125)
flat sums, the `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` (87),
`SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER` (343) and
`SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT` (126) multipliers, and the Sanctified
Wrath bypass from the attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` (269);
`melee_damage_taken_apply_like_cpp` applies the C++ tail, including the early
return when the negative flat benefit absorbs the hit. Both owners resolve the
inputs — the session from the target creature's applied auras plus the attacker's
ignore-resist sum, the map-owned `GlobalLegacy` runtime from its creature
snapshot and the attacker's auras. Boundary: the fixed cheat-death aura (45182),
the ranged variants and every `spellProto` branch cannot apply to a represented
white swing, the versatility term is commented out in the 3.4.3 source itself,
and a canonical-player victim keeps `NONE` like the other creature-only victim
terms. Evidence: `creature_aura_effects_resolve_the_applied_mask_like_cpp` pins
the effect-mask projection, `melee_damage_taken_matches_cpp_like_cpp` pins the
flat sums, the school/caster/pct multipliers, the negative-flat clamp and the
ignore-resist bypass,
`white_swing_applies_victim_melee_damage_taken_like_cpp` drives the session owner
100 -> 50 (flat) -> 25 (pct) -> 50 (bypass), and the production owner
`map_owned_player_melee_applies_victim_melee_damage_taken_like_cpp` holds the
map-owned tick to the flat half; wow-packet 744/0, wow-data 753/0, wow-entities
940/0, wow-world 3978/0/1 (three consecutive runs) and world-server 594/0/0 pass;
format, `git diff --check` and the physical ratchet pass (one recorded ten-line
ceiling growth), and `validation-v2 quick` (manifest
`20260917T143743.670941Z-2898243-quick.json`) passes. `validation-v2 final` stops
only at the pre-existing `hotspot-ratchet` baseline failure (manifest
`20260917T143842.381692Z-2898605-final.json`). No live DB/restart/relogin QA. The
represented melee attacker and victim chains of `CalculateMeleeDamage` are now
complete for creature victims; #29 remains open for the remaining spell/melee
math (melee haste/cooldown consumers, ranged auto-attack, proc/scripted terms).

**#29 melee block band — 2026-09-17, implementation `91c3df70`, integrated as
`ede81837` by PR #1089:** C++ `Unit::CalculateMeleeDamage`'s `MELEE_HIT_BLOCK`
branch (`Unit.cpp:1399-1407`) keeps `VICTIMSTATE_HIT`, sets `HITINFO_BLOCK`,
subtracts `CalculatePct(damage, GetBlockPercent(attackerLevel))` and doubles the
blocked amount when the block is critical; `AttackerStateUpdate::Write`
(`CombatLogPackets.cpp:373-397`) then appends the blocked amount and the trailing
`float Unk`. The represented table skipped the band because neither wire field
was ported. `session_rules/rules_4.rs` now rolls `MELEE_HIT_BLOCK` between
GLANCING and CRIT from the victim's `GetUnitBlockChance` base plus the
victim-level bonus (no expertise reduction, exactly like C++), and the outcome
switch returns `(damage, blocked)` with `CalculatePct(damage, 30)` for the flat
creature block percent (`Unit.h:947`); `wow-packet` gained `blocked: i32` on
`AttackerStateUpdate`, the conditional `int32(blocked)`/`float(Unk)` write and
`HIT_INFO_BLOCK`; and `RepresentedMeleeSwingLikeCpp`, the mailbox DTO and the
`PlayerMeleeCreatureHitLikeCpp` presentation vector carry the blocked amount to
the publishing session in both owners. Boundary: `IsBlockCritical`'s victim aura
sum and `Player::GetBlockPercent`'s shield-block formula have no represented
producer, so the flat 30% creature block stays. Evidence: the band-edge test
covers the block boundaries, the damage switch pins `(70, 30)` and `(5, 2)`, the
presentation pins `HITINFO_BLOCK | HITINFO_AFFECTS_VICTIM` with
`VICTIMSTATE_HIT`, the packet byte test parses the blocked amount and the
trailing float, and the new `white_swing_publishes_a_block_like_cpp` (session
owner) and `map_owned_player_melee_publishes_a_block_like_cpp` (production
map-owned tick) force a 100% block band and hold the 70/30 split plus the
creature's health loss. wow-packet 744/0, wow-data 753/0, wow-entities 940/0,
wow-world 3974/0/1 (three consecutive runs) and world-server 594/0/0 pass;
format, `git diff --check` and the physical ratchet pass (three recorded ceiling
growths of 1-2 lines), and `validation-v2 quick` (manifest
`20260917T141411.755780Z-2873193-quick.json`) passes. `validation-v2 final` stops
only at the pre-existing `hotspot-ratchet` baseline failure (manifest
`20260917T141516.534662Z-2873575-final.json`). No live DB/restart/relogin QA. #29
remains open for the `MeleeDamageBonusTaken` victim chain and the remaining
spell/melee math.

**#29 melee attack table — 2026-09-17, implementation `e478a2b7`, integrated as
`08e27329` by PR #1087:** C++ `Unit::CalculateMeleeDamage`
(`Unit.cpp:1341-1343`) rolls `Unit::RollMeleeOutcomeAgainst`
(`Unit.cpp:2272-2378`) after mitigation and applies the outcome switch
(`1345-1440`); the represented white swing always landed as a normal hit, so
nothing could miss, be dodged, parried, glance or crit and the published
`SMSG_ATTACKERSTATEUPDATE` always carried the plain hit flags. The new
`session_rules/rules_4.rs` owns the represented table — the band order
`MISS > DODGE > PARRY > GLANCING > CRIT > HIT` in C++'s 1/10000 units with its
`int32(chance * 100.0f)` truncation and gated bands, the outcome damage switch
(avoids to zero, glancing `1 - min(leveldiff, 3) * 0.1`, crit double) and the
`HitInfo`/`TargetState` presentation — while `melee_outcome_inputs_like_cpp`
assembles every chance from attacker/victim facts so both owners share one rule.
`Creature` gained `CreatureAvoidanceLikeCpp` (its C++
`GetUnitDodgeChance`/`GetUnitParryChance`/`GetUnitBlockChance` bases), seeded by
`apply_lifecycle_record` from the template and its `NO_PARRY`/`NO_BLOCK` flags,
and `PlayerEffectiveCombatStatsLikeCpp` gained `melee_hit_chance_pct`
(`m_modMeleeHitChance`), so the map runtime resolves the miss band without a
combat-ratings table. `take_canonical_player_attack_swings_like_cpp` draws the
roll per landed swing and returns `RepresentedMeleeSwingLikeCpp { damage,
hit_info, victim_state }`; both owners resolve the facts, the avoid path deals no
damage, no tap and no threat, and the mailbox DTO carries the outcome to the
publishing session. `wow-packet` gained the correctly named 3.4.3 constants
(`HIT_INFO_AFFECTS_VICTIM`, `HIT_INFO_OFFHAND`, `HIT_INFO_MISS`,
`HIT_INFO_CRITICAL_HIT`, `HIT_INFO_GLANCING`, `VICTIM_STATE_INTACT`,
`VICTIM_STATE_DODGE`, `VICTIM_STATE_PARRY`), replacing the misnamed
`HIT_INFO_NORMAL_SWING` (`C++ HITINFO_NORMALSWING` is zero) and the stale
`victim_state` doc. Boundaries: the block band stayed out of the roll because the
represented `AttackerStateUpdate` did not yet port the conditional
`blocked`/`unk` fields C++ appends for `HITINFO_BLOCK` (delivered by the entry
above); victim
`MOD_DODGE_PERCENT`/`MOD_PARRY_PERCENT` auras,
`CREATURE_FLAG_EXTRA_NO_CRUSHING_BLOWS`, the critical-damage-bonus aura and the
casting/control avoidance gate have no represented producer; a canonical-player
victim keeps the pre-table behaviour. Evidence:
`melee_attack_table_matches_roll_melee_outcome_against_like_cpp` pins every band
edge plus the gated and truncated cases,
`melee_attack_table_outcome_effects_match_calculate_melee_damage_like_cpp` pins
the damage/presentation switch (including `HITINFO_OFFHAND`),
`melee_attack_table_inputs_resolve_cpp_chances_like_cpp` pins the fact assembly
(dual-wield `+19`, `+1.5` per level, glancing at `+4` levels, totem, player
victim), `white_swing_publishes_the_attack_table_outcome_like_cpp` drives the
session owner (400 landed swings with both hits and avoids, then a `-200%`
hit-chance aura forcing `HIT_INFO_MISS`/`VICTIM_STATE_INTACT` at zero damage) and
`map_owned_player_melee_publishes_the_attack_table_outcome_like_cpp` holds the
production map-owned tick to the same miss; fixtures isolate the table through
`CreatureAvoidanceLikeCpp` (zero for a directly constructed creature) plus a
trained hit chance, so the existing exact-damage regressions stay deterministic.
wow-packet 742/0, wow-entities 940/0, wow-world 3972/0/1 and world-server 594/0/0
pass; format, `git diff --check` and the physical ratchet pass (two recorded
ceiling growths; the melee-math tests live in the bounded
`session/tests/scenarios_combat_4.rs`), and `validation-v2 quick` (manifest
`20260917T135143.177788Z-2830540-quick.json`) passes. `validation-v2 final` stops
only at the pre-existing `hotspot-ratchet` baseline failure (manifest
`20260917T135303.990925Z-2831019-final.json`). No live DB/restart/relogin QA. #29
remains open for the `MeleeDamageBonusTaken` victim chain and the remaining
spell/melee math.

**#29 white-swing armour mitigation — 2026-09-17, implementation `a739c8a4`,
integrated as `9cc0ea20` by PR #1085:** C++ `Unit::CalculateMeleeDamage`
(`Unit.cpp:1326-1339`) runs the rolled and bonus-adjusted damage through
`Unit::CalcArmorReducedDamage` (`Unit.cpp:1623-1685`) before the hit table; the
represented white swing skipped armour, so every auto-attack landed at full value
against armoured victims. `session_rules::armor_reduced_damage_like_cpp`
implements the formula — the victim's `GetArmor()` (a creature's
`CreatureBaseStats::GenerateArmor`), the attacker's live
`CR_ARMOR_PENETRATION` percentage with C++'s `maxArmorPen` cap, the attacker's
`SPELL_AURA_MOD_TARGET_RESISTANCE` (123) sum covering
`SPELL_SCHOOL_MASK_NORMAL`, the level-59 extension and the 75% clamp — while
`GetArmorMultiplierForTarget` is `1.0` for every 3.4.3 unit.
`PlayerEffectiveCombatStatsLikeCpp` gained `armor_penetration_pct`, written once
by the stat projection as `GetRatingBonusValue(CR_ARMOR_PENETRATION)` clamped to
100, so both swing owners read one canonical value and the map runtime needs no
combat-ratings table; `RepresentedArmorMitigationLikeCpp` carries the resolved
inputs into the shared `take_canonical_player_attack_swings_like_cpp`, with the
session resolving them in `represented_melee_armor_mitigation_like_cpp` and the
map-owned `GlobalLegacy` runtime resolving the creature's armour/level plus the
attacker's snapshot and aura sum. Boundary:
`SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` (345) and
`SPELL_AURA_MOD_IGNORE_TARGET_RESIST` (269) have no represented producer, the
`SpellModOp::TargetResistance` adjustment cannot apply to an auto-attack, a
canonical-player victim contributes no armour in either owner (that snapshot
belongs to the victim's session), the represented Player has no
`SetMeleeDamageSchool` override so the swing is physical, and the final
`max(1.0).round()` conversion is unchanged. Evidence:
`armor_reduction_matches_calc_armor_reduced_damage_like_cpp` pins the formula
(armour 0/5,000/10,000,000, 25%/100% penetration, a cancelled armour value and
the sub-60 cap), `white_swing_applies_victim_armor_mitigation_like_cpp` holds the
session owner to 753/1,000 damage and proves a non-normal
`MOD_TARGET_RESISTANCE` row is ignored while the normal row cancels the armour,
and `map_owned_player_melee_applies_victim_armor_mitigation_like_cpp` holds the
production map-owned tick to the same 753/1,000; wow-world --lib is 3967/0/1 and
world-server --lib 594/0/0; format, `git diff --check` and the physical ratchet
pass, and `validation-v2 quick` (manifest
`20260917T124035.688694Z-2724130-quick.json`) passes. `validation-v2 final` stops
only at the pre-existing `hotspot-ratchet` baseline failure (manifest
`20260917T124051.127247Z-2724225-final.json`). No live DB/restart/relogin QA. #29
remains open for the `RollMeleeOutcomeAgainst` hit table and the
`MeleeDamageBonusTaken` victim chain (both delivered by the entries above) and the
remaining spell/melee math.

**#29 white-swing damage roll — 2026-09-17, implementation `cdc6ea1a`, integrated
as `eae76cf2` by PR #1083:** C++ `Unit::CalculateMeleeDamage`
(`Unit.cpp:1326-1334`) rolls `Unit::CalculateDamage` and passes the rolled value
into `MeleeDamageBonusDone`; `CalculateDamage` (`Unit.cpp:2384-2435`) clamps both
`UnitData` bounds at zero, orders them, truncates to `uint32` and returns
`urand(min, max)`. The represented white swing instead always used the lower
bound, so every auto-attack dealt the weapon's minimum damage. The new
receiver-free rule `session_rules::white_swing_roll_like_cpp` implements that
sequence and `represented_white_swing_damage_like_cpp` calls it for the base and
offhand swings, so the session owner and the map-owned `GlobalLegacy` owner share
one implementation; the draw sits inside the landed-swing branch, so a timer that
is not ready consumes nothing. The draw mirrors C++'s process-global
`RandomEngine::Instance()` through the established `wow_core::urand_like_cpp`
wrapper (the mapping `docs/migration/common.md` records for `urand`); the
creature-owned RNG stream is a separate later design for creature runtime
authority and is untouched. Boundary: the rest of `CalculateMeleeDamage` (the
`RollMeleeOutcomeAgainst` hit table and the `CalcArmorReducedDamage` mitigation,
both delivered by the entries above), the `MeleeDamageBonusTaken` victim chain
and the final `max(1.0).round()` conversion remain as they were. Evidence:
`white_swing_roll_bounds_follow_calculate_damage_like_cpp` pins the
clamp/order/truncate bounds (zero and negative ranges, inverted fractional
`9.9`/`5.2` -> `[5, 9]`), `white_swing_damage_rolls_the_published_range_like_cpp`
lands 400 swings from a `[5, 9]` weapon inside the range and requires both bounds
plus at least four distinct values, and every equal-range exact-damage white-swing
regression stays exact; wow-world --lib is 3964/0/1 and world-server --lib
594/0/0; format, `git diff --check` and the physical ratchet pass, and
`validation-v2 quick` (manifest `20260917T120229.664585Z-2698392-quick.json`)
passes. `validation-v2 final` stops only at the pre-existing `hotspot-ratchet`
baseline failure (manifest `20260917T120326.090074Z-2698686-final.json`). No live
DB/restart/relogin QA. #29 remains open for the hit table, mitigation, the
victim-side chain and the remaining spell/melee math.

**#65 GameObject respawn-save test fidelity — 2026-09-17, implementation
`192da738`, integrated as `22423d96` by PR #1081:** the three `scenarios_9`
GameObject respawn-save regressions that this document carried as pre-existing
failures for many rounds are stale tests, not a production defect. C++
`Map::Update` reaches an object only through a player's nearby cells or
`m_activeNonPlayers` (`Map.cpp:701-767`), and the represented production
`MapObjectUpdateSelectionLikeCpp::NearbyCells` plan uses that same source set,
while the tests inserted the GameObject through the test helper's
`already_in_world` shortcut, which skips both the cell insert and
`Map::AddToMap`'s `AddToActive` tail (`Map.cpp:555-570`), leaving the map with no
visitor source at all; they were written against the pre-selection whole-store
visitor. The new local fixture
`insert_active_live_gameobject_for_spawn_like_cpp` marks the object active and
inserts it through `add_map_object_record_to_map_like_cpp`, so `Map::AddToMap`
inserts it into its cell and `AddToActive` puts it in the active non-player set
`Map::Update` visits every tick; the replaced-timer scenario additionally keeps a
non-active sibling with the identical deactivated state as a negative control, so
exactly one save is still required. No production code changed. Evidence:
`world-server --lib` is now 594/0/0 (previously 591 passed with these three
failures), format, `git diff --check` and the physical ratchet pass, and
`validation-v2 quick` (manifest `20260917T114619.975721Z-2683096-quick.json`)
passes; `validation-v2 final` still stops only at the pre-existing
`hotspot-ratchet` baseline failure (manifest
`20260917T114814.937686Z-2683594-final.json`), which this tests-only change does
not introduce. This supersedes the "three pre-existing `scenarios_9` failures"
statements in the older dated entries below.

**#61 victim aurastate/aura-mechanic melee bonuses — 2026-09-17, implementation
`1d33f078`, integrated as `44525aae` by PR #1079:** C++
`Unit::MeleeDamageBonusDone` (`Unit.cpp:7631-7648`) multiplies `DoneTotalMod` by
`SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE` (303) when
`pVictim->HasAuraState(miscValue)` holds, and by
`SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC` (249) when
`pVictim->HasAuraWithMechanic(1 << miscValue)` holds; the represented white swing
applied neither in either owner. `session_rules::melee_damage_bonus_done_like_cpp`
(renamed from `..._creature_type_like_cpp`) now takes the victim's aura-state and
mechanic masks and folds both multipliers, and its early return on an empty
creature-type mask is gone because these two terms apply without a creature type.
`Unit::HasAuraWithMechanic` now has a single receiver-free implementation in
`session_rules` (`aura_application_mechanic_mask_like_cpp` for a canonical
Player's `AuraApplication` map, `applied_aura_mechanic_mask_like_cpp` for a
creature's `AppliedAuraRef` list), replacing the session-local copy the
spell-damage path used; the session target helper delegates to it. The session
owner reads `represented_target_aura_state_mask_like_cpp` (already shared with the
spell path) and `represented_target_mechanic_mask_like_cpp`; the map-owned
`GlobalLegacy` production phase derives the creature victim's aurastate mask from
its aura subsystem plus `WorldCreature::health_aura_state_like_cpp` and the
mechanic mask from its applied auras at the map's own difficulty, collected during
the canonical collect phase so the execute phase keeps the established
canonical-then-legacy lock order. Boundary: a canonical-player victim resolved
through the runtime's non-creature branch still contributes no masks (that state
is owned by the victim's session), and the victim
`SPELL_AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS`/`RANGED_...` term still has no
represented creature-aura producer. Evidence:
`white_swing_applies_victim_aurastate_and_mechanic_melee_bonus_like_cpp` holds the
session owner to `pct` 1.0/2.0/4.0 and a 28-damage swing from a 7-damage weapon,
`map_owned_player_melee_applies_victim_aurastate_and_mechanic_bonus_like_cpp`
holds the map-owned production tick to 5/10/20 damage, and wow-world --lib is
3962/0/1; format, `git diff --check`, the physical ratchet and `validation-v2
quick` (manifest `20260917T112226.278946Z-2668271-quick.json`) pass. No live
DB/restart/relogin QA. #61 stays open for the Shadow Bite per-DoT term, the
blocked victim-side `SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains
(creature aura producers are absent), the ranged/offhand attack-type selection for
the AP term, the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 melee creature-type damage bonus — 2026-09-17, implementation `db3c2db6`,
integrated as `6d9b91bc` by PR #1077:** C++ `Unit::MeleeDamageBonusDone`
(`Unit.cpp:7558-7650`) adds the flat `SPELL_AURA_MOD_DAMAGE_DONE_CREATURE` (59)
benefit, converts the attacker's `SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS` (102)
sum with `GetAPMultiplier`, and multiplies by
`SPELL_AURA_MOD_DAMAGE_DONE_VERSUS` (168), all keyed on the victim's
`GetCreatureTypeMask`; the represented white swing applied none of them in either
owner. `session_rules::melee_damage_bonus_done_creature_type_like_cpp` is the
receiver-free rule both owners call (built on the shared aura projection from PR
#1073, so they cannot drift), `RepresentedMeleeDamageBonusLikeCpp` carries the
`(DoneFlatBenefit, DoneTotalMod)` pair into the shared swing function which now
computes `max((rolled + flat) * pct * autoattack_pct, 1.0)`, the session resolves
the victim from the canonical `attacking()` target with the existing represented
target helpers, and the map-owned runtime gained the immutable
`creature_template_lifecycle_store` on `LegacyCreatureAggroConfigLikeCpp`
(populated in `app.rs` beside the spell stores) with the config now threaded into
`run_legacy_player_melee_tick_once_like_cpp`. Boundary: the victim's
`SPELL_AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS`/`RANGED_...` term has no
represented creature-aura producer; the versus-aurastate (303) and
target-aura-mechanic (249) multipliers were the next unit, delivered by the entry
above. Evidence: the new
scenario registers a type-7 creature template and asserts `flat = 5` from the
creature-type aura alone, `pct = 2.0` after adding the versus aura, and a
24-damage swing from 7 (`(7 + 5) * 2`); wow-data --lib 753/0, wow-entities --lib
940/0 and wow-world --lib 3960/0/1 pass, as do format, `git diff --check`, the
physical ratchet (two recorded ceiling growths) and `validation-v2 quick`
(manifest `20260917T105857.497150Z-2661037-quick.json`). No live DB/restart/
relogin QA. #61 stays open for the Shadow Bite per-DoT term, the blocked
victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 cast-speed auras — 2026-09-17, implementation `fa7abbfd`, integrated as
`a7b1faf0` by PR #1075:** C++ `Spell::prepare` stores `m_casttime` after
`Unit::ApplyCastTimePercentMod` (`Unit.cpp:10229-10252`), driven by
`AuraEffect::HandleModCastingSpeed` (`SpellAuraEffects.cpp:4272-4315`) and
`HandleModCombatSpeedPct` (`4330-4351`). The represented cast state copied the
raw DB2 `CastTime`, so haste auras never shortened a cast.
`represented_cast_speed_multiplier_like_cpp` folds
`SPELL_AURA_MOD_CASTING_SPEED_NOT_STACK` (65), `SPELL_AURA_HASTE_SPELLS` (216),
`SPELL_AURA_MELEE_SLOW` (193) and `SPELL_AURA_MOD_SPEED_SLOW_ALL` (252) with the
C++ conversion (positive `100/(100+amount)`, negative `(100-amount)/100`), and a
total of `1000` is C++'s `SetInstantCast`, represented as a zero multiplier. The
`player_cast::Runtime` seam gained `cast_time_multiplier` (default `1.0`) and
`prepare` applies it once when building `SpellCastState`, so the stored state,
the `timed` gate and the published `SMSG_SPELL_START` agree; the session
implements it from its canonical auras. A mislabeled constant found while wiring
this was fixed: the file declared both `SPELL_AURA_HASTE_SPELLS = 73` and
`SPELL_AURA_MOD_POWER_COST_SCHOOL = 73`, while C++ has
`MOD_POWER_COST_SCHOOL = 73` and `HASTE_SPELLS = 216`, so the duplicate was
removed and its only (test) consumer updated. Boundary: the spell-group
de-duplication and the `ModHasteRegen` cooldown-recovery consumer remain
unrepresented. Evidence: the new scenario prepares a 1500 ms cast with and
without a +50% aura and asserts the stored cast time is 1500 then 1000, with the
multiplier reading `100/150`; wow-data --lib 753/0, wow-entities --lib 940/0 and
wow-world --lib 3959/0/1 pass, as do format, `git diff --check`, the physical
ratchet and `validation-v2 quick` (manifest
`20260917T095808.364454Z-2630347-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the remaining `MeleeDamageBonusDone` terms (which need the
victim's creature type and mechanic masks, so the map-owned runtime needs the
creature-template store plumbed into `LegacyCreatureAggroConfigLikeCpp`), the
Shadow Bite per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 white-swing auto-attack damage aura — 2026-09-17, implementation
`02f1affc`, integrated as `ccab6018` by PR #1073:** C++
`Unit::MeleeDamageBonusDone` (`Unit.cpp:7620-7627`, tail `7666-7667`) adds every
active `SPELL_AURA_MOD_AUTOATTACK_DAMAGE` percentage to `DoneTotalMod`, which
multiplies the swung damage; the represented white swing ignored it in both the
session and the production map-owned runtime. New receiver-free
`session_rules/rules_3.rs` hosts the aura-effect projection
`player_aura_effects_by_spell_aura_type_like_cpp` (deterministic ascending slot
order) and `represented_autoattack_damage_multiplier_like_cpp`; the session's
`resolved_aura_effects_by_spell_aura_type_like_cpp` now delegates to it, so both
owners resolve the canonical Player auras with one implementation instead of two
drifting copies. `Unit::mod_autoattack_damage_pct_like_cpp` carries the product
on the canonical Player and the owning session recomputes it inside the existing
aura-mutation sync (the same place that writes the attack-time multipliers), so
the map-owned swing path only reads it and there is one writer per transition;
`represented_white_swing_damage_like_cpp` applies it in the shared swing function
both owners call. Boundary: the remaining `MeleeDamageBonusDone` terms (flat
creature-type damage, `MOD_MELEE_ATTACK_POWER_VERSUS` with `GetAPMultiplier`,
versus-creature-type, versus-aurastate and target-aura-mechanic multipliers) need
the victim's creature type and mechanic masks and were the next unit, delivered by
the melee creature-type and victim aurastate/aura-mechanic entries above. Evidence:
the new scenario asserts a 7-damage white swing, 14 once a +100%
`SPELL_AURA_MOD_AUTOATTACK_DAMAGE` aura is applied (with the canonical multiplier
reading 2.0) and 7 again after removal; wow-data --lib 753/0, wow-entities --lib
940/0 and wow-world --lib 3958/0/1 pass, as do format, `git diff --check`, the
physical ratchet and `validation-v2 quick` (manifest
`20260917T094729.144549Z-2619377-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the remaining `MeleeDamageBonusDone` terms, the Shadow Bite
per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 displayed-power ownership and the shapeshift Unit field — 2026-09-17,
implementation `b48b7204`, integrated as `dd9b138b` by PR #1071:** C++
`Unit::CalculateDisplayPowerType` (`Unit.cpp:5550-5600`) selects
`UNIT_FIELD_DISPLAYPOWER` from the active form, then an active
`SPELL_AURA_MOD_POWER_DISPLAY`, then `ChrClasses`; `Unit::UpdateDisplayPower`
(`Unit.cpp:5600-5603`) is reached from `Player::InitDataForForm`
(`Player.cpp:22090`) and `AuraEffect::HandleAuraModPowerDisplay`
(`SpellAuraEffects.cpp:4027-4039`). The represented runtime never recomputed it,
so a druid in cat form kept the mana bar.
`Unit::calculate_display_power_type_like_cpp` and
`represented_power_type_from_u8_like_cpp` implement the selection;
`sync_represented_display_power_like_cpp` writes the canonical field and
publishes the changed values delta from the form-change sync and from any aura
mutation carrying `SPELL_AURA_MOD_POWER_DISPLAY`. Wiring it exposed a
second-authority defect: the session only wrote the transitional `Player`
gameplay-state projection and never `Unit::data.shapeshift_form` (the published
`UNIT_FIELD_BYTES_2` byte C++ `Unit::SetShapeshiftForm` writes), so the reader
never saw the canonical value. The setter now writes the Unit field and keeps the
projection in sync, the reader prefers the Unit field with the projection as
fallback, and `Unit::set/get_shapeshift_form_id_like_cpp` expose the raw byte so
ids the represented enum does not name (`FORM_GHOUL`, epic flight) still select
their power type. Boundary: the vehicle and hunter/warlock pet branches of
`CalculateDisplayPowerType`, and the skill-boost/collision-height half of
`HandleAuraModShapeshift`, remain unrepresented. Evidence: one scenario asserts
the mana default, `POWER_ENERGY` in cat form with a published UPDATE_OBJECT, the
mana restore on removal and `POWER_RAGE` in bear form; another asserts a
`SPELL_AURA_MOD_POWER_DISPLAY` aura selects `POWER_RUNIC_POWER` outside a form;
wow-data --lib 753/0, wow-entities --lib 940/0 and wow-world --lib 3957/0/1 pass,
as do format, `git diff --check`, the physical ratchet and `validation-v2 quick`
(manifest `20260917T092920.288970Z-2602350-quick.json`). No live DB/restart/
relogin QA. #61 stays open for the Shadow Bite per-DoT term, the blocked
victim-side `SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura
producers are absent), the ranged/offhand attack-type selection for the AP term,
the offhand-damage aura scale, the melee auto-attack `MeleeDamageBonusDone`
chain (the production swing owner is the global legacy runtime, so it needs the
shared Player+SpellStore resolver and the creature-template plumbing), the
player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers,
rune regeneration and live DB/restart/relogin QA.

**#61 `HandleShapeshiftBoosts` — 2026-09-17, implementation `9cb30cd8`,
integrated as `74f9efac` by PR #1069:** C++
`AuraEffect::HandleShapeshiftBoosts` (`SpellAuraEffects.cpp:1325-1464`) owns the
form's bonus spells and the stance-gated self-aura sweep; the represented form
change installed only the attack times, so a druid entered cat or bear form
without the form's passive boosts and kept auras a shapeshift should drop. New
`session/spell_state/shapeshift.rs` implements both branches:
`apply_represented_shapeshift_boosts_like_cpp` casts the form's hardcoded boost
ids (including the glyph-gated Shadowform and Spectral Wolf choices, and
reproducing C++'s absent `FORM_DIRE_BEAR_FORM` entry) plus every known passive or
`SPELL_ATTR0_DO_NOT_DISPLAY_SPELLBOOK_AURA_ICON_COMBAT_LOG` spell whose `Stances`
mask admits the form; `remove_represented_shapeshift_boosts_like_cpp` drops the
removed form's owned boost auras and sweeps every self-cast aura whose `Stances`
the resulting form no longer admits, reproducing `Aura::IsRemovedOnShapeLost`.
`sync_represented_shapeshift_form_ownership_like_cpp` now returns the mutation
direction and resulting form so the sync runs the matching branch before the item
refresh and the stat republish. Boundary: the skill-boost, `UpdateDisplayPower`
and collision-height half of `HandleAuraModShapeshift` remains unrepresented.
Evidence: one scenario asserts the cat-form boost spell 3025 appears on entry and
is dropped by `RemoveOwnedAura` on exit, another asserts a known passive with a
cat-form `Stances` mask is applied on entry and swept when the form is lost;
wow-data --lib 753/0, wow-entities --lib 940/0 and wow-world --lib 3955/0/1 pass,
as do format, `git diff --check`, the physical ratchet (one new bounded
`spell_state/shapeshift.rs`) and `validation-v2 quick` (manifest
`20260917T091150.444546Z-2580017-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the Shadow Bite per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 form-change item effect refresh — 2026-09-17, implementation
`5c427e5c`, integrated as `14b07630` by PR #1067:** C++
`Player::InitDataForForm` (`Player.cpp:22093-22094`) calls
`Player::UpdateEquipSpellsAtFormChange` on every shapeshift apply and removal:
each equipped item runs `ApplyItemEquipSpell(item, false, true)` and
`(item, true, true)` so spells whose `Stances` no longer fit are dropped and the
now-fitting ones are applied, and the item-set auras replay under the new form.
The represented `apply_represented_item_set_aura_refresh_events_like_cpp`
planner existed but nothing outside tests ever called it, so item-set and equip
spells gated by a form were inert.
`refresh_represented_item_effects_at_form_change_like_cpp` now removes the
equipped items' spell auras the new form rejects (matching on the item caster
GUID), re-applies each item's equip spells through the existing form-aware
`apply_initial_item_equip_auras_like_cpp`, and replays the item-set auras with
`form_change = true`; `sync_represented_shapeshift_form_like_cpp` runs it
between the attack-time update and the stat republish. Boundary:
`HandleShapeshiftBoosts` (the form's bonus spells) and the form display/
power-type switch remain unrepresented. Evidence: two new scenarios equip an
item carrying a cat-form-gated `ItemEffect` equip spell and a two-piece set
bonus gated the same way, then assert both auras appear when the form aura is
applied and disappear when it is removed; wow-data --lib 753/0, wow-entities
--lib 940/0 and wow-world --lib 3953/0/1 pass, as do format, `git diff --check`,
the physical ratchet and `validation-v2 quick` (manifest
`20260917T085935.668738Z-2566293-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the Shadow Bite per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, `HandleShapeshiftBoosts`, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 shapeshift form ownership and `CombatRoundTime` — 2026-09-17,
implementation `eddc62de`, integrated as `08f0bd3c` by PR #1065:** C++
`AuraEffect::HandleAuraModShapeshift` (`SpellAuraEffects.cpp:1838-1866`) owns
`Unit::SetShapeshiftForm`, and `Player::InitDataForForm`
(`Player.cpp:22076-22098`) installs the form's
`SpellShapeshiftFormEntry::CombatRoundTime`, consumed by
`Player::CalculateMinMaxDamage` (`StatSystem.cpp:461-467`) and
`Player::_ApplyWeaponDamage` (`Player.cpp:8018-8020`). The represented form
mirror had no producer, so a feral form never changed the attack time or the
weapon damage. `sync_represented_shapeshift_form_ownership_like_cpp` now sets
the form from an applied `SPELL_AURA_MOD_SHAPESHIFT` effect and clears it on
removal only when no other active aura still applies one;
`represented_shapeshift_combat_round_time_like_cpp` and
`apply_represented_shapeshift_base_attack_time_like_cpp` mirror
`InitDataForForm`/`SetRegularAttackTime` (writing only an attack whose equipped
weapon declares a delay) and `sync_represented_shapeshift_form_like_cpp` runs
both plus the stat republish; `effective_weapon_damage_ranges_like_cpp` applies
the `CombatRoundTime / 1000 / GetAPMultiplier` weapon rescale; and
`represented_can_use_attack_type_like_cpp` (extracted from the enchantment gate)
returns `None` for an unavailable canonical owner so the enchantment path stays
fail-closed while `_ApplyWeaponDamage` treats an unknown unit as unflagged.
Boundary: `HandleShapeshiftBoosts` (the form's bonus spells), the form display
and power-type switch, and `UpdateEquipSpellsAtFormChange` remain
unrepresented. Evidence: the new scenario asserts no round time before the aura,
the resolved 1000 ms form after it, main/off-hand base attack times at the form
value with ranged at 2000, and the form clearing on removal; the pure-function
test asserts the feral rescale and the zero-round-time branch; wow-data --lib
753/0, wow-entities --lib 940/0 and wow-world --lib 3951/0/1 pass, as do format,
`git diff --check`, the physical ratchet (two recorded one-line call-site
growths on `items.rs` and `world_entry.rs`) and `validation-v2 quick` (manifest
`20260917T084828.044088Z-2556349-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the Shadow Bite per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 attack-speed aura application — 2026-09-17, implementation
`dc8dac1b`, integrated as `920587ff` by PR #1063:** C++
`Unit::ApplyAttackTimePercentMod` (`Unit.cpp:10201-10227`) maintains
`m_modAttackSpeedPct` from the attack-speed aura handlers
(`SpellAuraEffects.cpp:4353-4393`), and the represented Unit owned the field
with no writer, so every swing used the unhasted base attack time.
`Unit::apply_attack_time_multipliers_like_cpp` installs the per-attack
multipliers while preserving the remaining fraction of the current swing;
`represented_attack_speed_multipliers_like_cpp` reproduces the handler mapping
(9 main hand; 138/217/319 main+off; 140 ranged; 192/342 all three; 193/252 all
three) and the exact C++ amount conversion (positive uses
`100 / (100 + amount)`, negative uses `(100 - amount) / 100`, which is not the
inverse); and `sync_represented_attack_speed_like_cpp` rewrites the canonical
Player values after every aura apply and removal, so the session that owns the
auras stays the only writer while the map-owned swing path only reads. Boundary:
the C++ `GetHighestExclusiveSameEffectSpellGroupValue` de-duplication of
`MOD_MELEE_HASTE`/`MELEE_SLOW` needs spell-group tables the represented session
does not load, so every active effect multiplies, and
`ApplyCastTimePercentMod` has no represented consumer yet. Evidence: the new
scenario asserts the `[1.0; 3]` sentinel, `100/130` on main and off hand from a
+30 `SPELL_AURA_MOD_MELEE_HASTE`, the swing-timer reset using the hasted
duration, the `1.3` factor from a -30 `SPELL_AURA_MOD_SPEED_SLOW_ALL` on all
three attacks, and the restore after each removal; wow-data --lib 753/0,
wow-entities --lib 940/0 and wow-world --lib 3950/0/1 pass, as do format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260917T082722.684280Z-2531109-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the Shadow Bite per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the shapeshift `CombatRoundTime` weapon-damage path,
the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate
powers, rune regeneration and live DB/restart/relogin QA.

**#61 `CONFIG_STATS_LIMITS_*` caps — 2026-09-17, implementation
`5eef7a77`, integrated as `604381f9` by PR #1061:** C++
`Player::UpdateBlockPercentage`, `UpdateDodgePercentage`,
`UpdateParryPercentage` and `UpdateCritPercentage` cap their published
percentages with `value > limit ? limit : value` while
`CONFIG_STATS_LIMITS_ENABLE` is set (`StatSystem.cpp:496-497`, `506-507`,
`675-676`, `712-713`; `World.cpp:1664-1668` defaults `false`/`95.0`), which the
represented projection ignored. `wow_data::StatsLimitsLikeCpp` now carries the
five values with `clamp_block`/`clamp_dodge`/`clamp_parry`/`clamp_crit`
helpers; `world-server` resolves the already-registered
`CONFIG_STATS_LIMITS_*` world-config keys and hands the immutable value to the
session through the existing `SessionRuntimePolicyCapabilitiesLikeCpp` beside
the durability rate; and `player_stat_system_projection_like_cpp` applies the
caps once, so the login create snapshot and the canonical effective-stats
snapshot cannot diverge. Boundary: the caps cover exactly the six published
fields C++ writes; the represented model still has no per-caster avoidance aura
state or diminishing-returns difference. Evidence: the three new/extended
tests cover the enabled/disabled, boundary (`>`, not `>=`) and negative cases,
the world-config file keys and defaults, and a login projection whose
parry/block/crit read 5.0 by default and 1.0 once the limits are enabled;
wow-data --lib 753/0, wow-entities --lib 940/0 and wow-world --lib 3949/0/1 pass,
and `world-server --lib` reports 591 passed with the three `scenarios_9`
gameobject-compatibility failures that reproduce unchanged on the untouched base
(verified with the change stashed), so they are pre-existing and outside this
unit; they were later resolved as stale test setups by the #65 GameObject
respawn-save test fidelity entry above. Format, `git diff --check`, the physical ratchet (the one-line composition
wiring is recorded on `crates/world-server/src/app.rs`, 5658 -> 5659) and
`validation-v2 quick` (manifest `20260917T080332.494026Z-2517083-quick.json`)
pass. No live DB/restart/relogin QA. #61 stays open for the Shadow Bite per-DoT
term, the blocked victim-side `SpellDamageBonusTaken`/`MeleeDamageBonusTaken`
chains (creature aura producers are absent), the ranged/offhand attack-type
selection for the AP term, the offhand-damage aura scale, the player-killer
(PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 `BonusCoefficientFromAP` table term — 2026-09-17, implementation
`de597571`, integrated as `a00f9efe` by PR #1059:** the C++ "Check for table
values" block of `Unit::SpellDamageBonusDone`/`SpellHealingBonusDone`
(`Unit.cpp:6633-6649`, `7132-7140`) adds
`int32(stack * BonusCoefficientFromAP * APbonus)` to the flat done benefit. The
DB2 field was already loaded into `SpellEffectDb2Entry` but dropped by the
runtime `SpellEffectInfo`, so represented direct damage and heals ignored it.
`wow_data::SpellEffectInfo` now carries `effect_bonus_coefficient_from_ap`
(`DB2Structure.h:3525`, `SpellInfo.cpp:436`), populated by
`spell_effect_from_db2_like_cpp`; the new
`represented_spell_bonus_coefficient_from_ap_like_cpp` resolves
`Unit::GetTotalAttackPowerValue(BASE_ATTACK)` through the canonical Player
accessor, and both bonus helpers add the term.
`represented_spell_has_attribute_like_cpp` centralises the difficulty-resolved
attribute read now shared by the three attribute gates, and
`SpellDamageBonusDone`'s `SPELL_ATTR3_IGNORE_CASTER_MODIFIERS` early-out also
skips the flat advertised benefit, matching the C++ order
(`Unit.cpp:6607-6612`). Boundary: `stack` is always one, the
`SpellModOp::BonusCoefficient` adjustment is not represented, and the attack
type is always `BASE_ATTACK` because the represented `SpellInfo` carries no
`SpellFamilyName`/`EquippedItemSubClassMask`, so the C++
`RANGED_ATTACK`/`OFF_ATTACK` selection and the victim's
`SPELL_AURA_*_ATTACK_POWER_ATTACKER_BONUS` term remain unrepresented. Evidence:
three new scenarios in `session/tests/scenarios_spell_state_25.rs` assert
`int32(3.0 * 100)` extra damage with 100 attack power, `int32(2.0 * 100)` extra
healing, and that an attr3 spell skips the flat `BonusCoefficient` benefit while
the un-gated one keeps it; wow-data --lib 752/0, wow-entities --lib 940/0 and
wow-world --lib 3949/0/1 pass, as do format, `git diff --check`, the physical
ratchet and `validation-v2 quick` (manifest
`20260917T072428.572336Z-2496250-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the Shadow Bite per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP`
branch and `SetPvPDeath`, alternate powers, rune regeneration and live
DB/restart/relogin QA.

**#61 `SpellHealingPctDone` completion — 2026-09-17, implementation
`1eab52dc`, integrated as `003e1bb9` by PR #1057:** the represented direct heal
applied only the published `ModHealingDonePercent` and the missing-health aura,
so C++ `Unit::SpellHealingPctDone` (`Unit.cpp:7185-7229`) was incomplete in two
ways. `represented_healing_pct_done_gated_like_cpp` now returns `1.0f` for
`SPELL_ATTR3_IGNORE_CASTER_MODIFIERS` (`SharedDefines.h:571`) and
`SPELL_ATTR6_IGNORE_HEALING_MODIFIERS` (`SharedDefines.h:680`) while the flat
`SpellBaseHealingBonusDone` benefit stays in place, exactly as C++ returns from
`SpellHealingPctDone` alone; the un-gated chain multiplies by
`SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE` (303) for every active victim aura
state before the missing-health term. `represented_aura_state_bit_like_cpp`
fails closed on an out-of-range aura state instead of shifting out of range, and
the damage path uses the same helper. Boundary: the `SPELLFAMILY_POTION`
early-out (the represented `SpellInfo` has no family name), the 354
`IsAffectingSpell` family/flag gate, the totem redirect and the scripted handlers
remain unrepresented. Evidence: the two new scenarios in
`session/tests/scenarios_spell_state_25.rs` assert a 100 heal becomes 200 against
a defensive-aurastate creature, and that a coefficient-0.5 heal with 100 spell
power heals 225 without the attribute and 150 with
`SPELL_ATTR6_IGNORE_HEALING_MODIFIERS` (proving the flat benefit survives the
gate); wow-data --lib 752/0, wow-entities --lib 940/0 and wow-world --lib
3946/0/1 pass, as do format, `git diff --check`, the physical ratchet (the
three-line `SPELL_ATTR6_IGNORE_HEALING_MODIFIERS` constant is recorded on
`crates/wow-data/src/spell/mod.rs`, 127 -> 130) and `validation-v2 quick`
(manifest `20260917T071254.960525Z-2480943-quick.json`). No live DB/restart/
relogin QA. #61 stays open for the Shadow Bite per-DoT term, the blocked
victim-side `SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura
producers are absent), the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 remaining `SpellDamagePctDone` terms — 2026-09-17, implementation
`04dd26c3`, integrated as `150664e5` by PR #1055:** the last three pieces of
C++ `Unit::SpellDamagePctDone` (`Unit.cpp:6683-6772`) were missing from the
represented direct-damage path. `SPELL_ATTR3_IGNORE_CASTER_MODIFIERS`
(`SharedDefines.h:571`) and `SPELL_ATTR6_IGNORE_CASTER_DAMAGE_MODIFIERS`
(`SharedDefines.h:682`) now return `1.0f` before any done term is read; Mage Ice
Lance (228598) multiplies by three on a victim carrying `AURA_STATE_FROZEN`
(`Unit.cpp:6750-6755`); and Warlock Drain Soul (198590) doubles while the caster
carries `AURA_STATE_WOUNDED_20_PERCENT` (`Unit.cpp:6762-6765`). Both scripted
terms read the represented `Unit::HasAuraState` union added by PR #1053;
`AURA_STATE_FROZEN` (=4, `SharedDefines.h:2510`) is added. Boundary: the Warlock
Shadow Bite per-DoT term, the `SPELL_AURA_ABILITY_IGNORE_AURASTATE` shortcut of
`Unit::HasAuraState`, and the `SpellFamilyName` switch guard (the represented
`SpellInfo` has no family name, so the globally unique spell id keys the branch)
remain unrepresented. Evidence: the three new scenarios in
`session/tests/scenarios_spell_state_25.rs` assert the versus-aurastate +100%
aura applies to a plain spell (200 damage) but not to an attr6- or attr3-gated
one (100 each), Ice Lance deals 100 unfrozen and 300 frozen, and Drain Soul deals
100 from a healthy caster and 200 from a wounded one; wow-data --lib 752/0,
wow-entities --lib 940/0 and wow-world --lib 3944/0/1 pass, as do format,
`git diff --check`, the physical ratchet (the six-line inline `attributes`
addition is recorded on `crates/wow-data/src/spell/mod.rs`, 121 -> 127) and
`validation-v2 quick` (manifest `20260917T070107.585649Z-2470766-quick.json`).
No live DB/restart/relogin QA. #61 stays open for the Shadow Bite per-DoT term,
`SpellDamageBonusTaken` and the remaining victim-side gates, the offhand-damage
aura scale, the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and
`SetPvPDeath`, alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 health-derived unit aura states — 2026-09-17, implementation
`4555be48`, integrated as `2db5424d` by PR #1053:** C++ `Unit::Update`
(`Unit.cpp:469-476`) maintains the health-derived bits of
`Unit::m_unitData->AuraState` (WOUNDED_20/25/35, WOUND_HEALTH_20_80,
HEALTHY_75, WOUND_HEALTH_35_80) for every alive Unit, and
`Unit::HasAuraState` reads that union. The represented runtime split the union:
the damage path read only the aura subsystem's aura-driven bits, so
`SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE` (303) never matched a health state,
and `represented_has_aura_state_like_cpp` was a hardcoded `false` stub that
denied every `Spell::CheckCast` caster-aura-state restriction.
`represented_player_aura_state_mask_like_cpp` now ORs the canonical Player's
aura-driven mask with `WorldCreature::health_aura_state_like_cpp` over the
resolved vitals, `represented_unit_aura_state_mask_like_cpp` extends the union to
a world creature, `represented_target_aura_state_mask_like_cpp` feeds the damage
path the same union, and `represented_has_aura_state_like_cpp` answers the cast
restriction with checked shifting (a positive state outside the 32-bit mask
denies rather than panics). `AURA_STATE_WOUNDED_20_PERCENT` (=2,
`SharedDefines.h:2508`) is added. Boundary: the per-caster aura-state branch of
`Unit::HasAuraState` (`PER_CASTER_AURA_STATE_MASK`, RAID_ENCOUNTER_2 and
ROGUE_POISONED) and the `SPELL_AURA_ABILITY_IGNORE_AURASTATE` shortcut remain
unrepresented; a unit that cannot be resolved still yields mask zero. Evidence:
the three new scenarios in the bounded `session/tests/scenarios_spell_state_25.rs`
(creature below 20% health receives the aura-303 multiplier while a healthy one
does not; the player mask is empty at 50% health and carries WOUNDED_20 at 10%
while HEALTHY_75 clears; the login passive cast gate follows the health-derived
caster aura state) pass, as do wow-entities --lib 940/0 and wow-world --lib
3941/0/1, format, `git diff --check`, the physical ratchet (after
scenarios_spell_state_11.rs reached the 2,000-line test-file budget and the split
registration needed the recorded session_tests.rs ceiling move 5766 -> 5768) and
`validation-v2 quick` (manifest `20260917T065115.813498Z-2459846-quick.json`).
No live DB/restart/relogin QA. #61 stays open for the family-scripted and
attribute-gated damage terms, `SpellDamageBonusTaken` and the remaining
victim-side gates, the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 mechanic-based damage multipliers — 2026-09-17, implementation
`f190dd97`, integrated as `d23bfd17` by PR #1051:** `Unit::SpellDamagePctDone`
(`Unit.cpp:6734-6746`) still omitted the two mechanic terms of the represented
direct-damage path. `represented_target_mechanic_mask_like_cpp` now mirrors
`Unit::HasAuraWithMechanic` (`Unit.cpp:4714-4729`), unioning each applied aura's
`SpellInfo::Mechanic` and the mechanics of its applied `IsEffect()` slots, for
the canonical player's visible applications (using the application's stored
difficulty) and for world-creature `AppliedAuraRef`s (which retain none, so the
current map difficulty is used). `represented_spell_damage_pct_done_like_cpp`
multiplies the damage by every caster aura-249 effect whose misc mechanic the
victim carries and applies the summed aura-276 percentage for the cast effect's
`SpellEffectInfo::Mechanic`, falling back to `SpellInfo::Mechanic`, resolved
through `SpellHitMetadataLikeCpp`; the bonus helper now receives the C++ effect
index for that lookup. Boundary: the family-scripted terms and the
`SPELL_ATTR3_IGNORE_CASTER_MODIFIERS` /
`SPELL_ATTR6_IGNORE_CASTER_DAMAGE_MODIFIERS` early-outs remain unrepresented.
Evidence: the two new session tests assert the matching and non-matching mechanic
cases for auras 249 and 276 (`spell_school_damage_applies_damage_done_for_mechanic_like_cpp`,
`spell_school_damage_applies_damage_percent_done_by_target_aura_mechanic_like_cpp`);
wow-data --lib 752/0 and wow-world --lib 3938/0/1; format, `git diff --check`,
the physical ratchet and `validation-v2 quick` (manifest
`20260917T062936.129884Z-2436096-quick.json`) pass. `validation-v2 final` was
also run on this candidate: it fails only at `check_architecture.py
hotspot-ratchet`, which already fails identically on the untouched base
`35535748` (`crates/wow-world/src/session/mod.rs` +5591 total lines against the
stored baseline, `handlers/character/mod.rs` +1550, and others); the drift is
pre-existing repository debt outside this unit's scope and was not re-baselined.
No live DB/restart/relogin QA. #61 stays open for the family-scripted and
attribute-gated damage terms, `SpellDamageBonusTaken` and the remaining
victim-side gates, the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 versus-aurastate damage multiplier — 2026-09-17, implementation
`cf8190d5`, integrated as `93f20092` by PR #1049:** `Unit::SpellDamagePctDone`
(`Unit.cpp:6728-6734`) multiplies spell damage by
`GetTotalAuraMultiplier(SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE)` for every
active effect whose misc aura state the victim carries, which the represented
damage path omitted. `represented_target_aura_state_mask_like_cpp` reads the
target's unit aura state mask (the canonical player's or the world creature's),
and `represented_spell_damage_pct_done_like_cpp` multiplies its school maximum by
each matching aura-303 effect. Boundary: the target-aura-mechanic (249) and
`MOD_DAMAGE_DONE_FOR_MECHANIC` terms (resolved next by the 2026-09-17
mechanic-based multipliers entry) plus the family scripts remained
unrepresented, and a target whose aura state cannot be resolved keeps only the
previously applied multipliers. Evidence: the new session test sets the
defensive aura state on a registered creature, asserts an unrelated-state aura
(misc 2) leaves 100 damage and the matching aura (misc 1, +100%) doubles it to
200; wow-data --lib 752/0 and wow-world --lib 3936/0/1; format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260917T061932.575648Z-2427345-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the mechanic/family damage terms, `SpellDamageBonusTaken`
and the remaining victim-side gates, the offhand-damage aura scale, the
player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 creature missing-health heal scaling — 2026-09-17, implementation
`5011ad4a`, integrated as `3afbd42b` by PR #1047:** the missing-health healing
term added in PR #1043 only applied when the victim was the session player, so
heals landing on creatures ignored
`SPELL_AURA_MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH` (354).
`represented_target_health_pct_like_cpp` now resolves the target's health
percentage from the canonical player vitals or the world creature's runtime
health (`WorldCreature::current_hp`/`max_hp`), and
`represented_spell_healing_bonus_done_like_cpp` applies the scaling for any
resolvable target. Boundary: the aura's `AuraEffect::IsAffectingSpell`
family/flag gate is still not represented, and a target that cannot be resolved
keeps the unscaled percentage. Evidence: the new session test damages a
registered creature to 500/1000 health, applies aura 354 with amount 100 and
casts a 100 base heal, asserting 150 healing (`int32(100 * 1.5)`) and 650
remaining health; the player-victim heal scenario stays green; wow-data --lib
752/0 and wow-world --lib 3935/0/1; format, `git diff --check`, the physical
ratchet and `validation-v2 quick` (manifest
`20260917T061323.951908Z-2419707-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the remaining aurastate/mechanic and victim-side terms,
the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 versus-creature-type damage multiplier — 2026-09-17, implementation
`7f2aa52c`, integrated as `5b0c9954` by PR #1045:** `Unit::SpellDamagePctDone`
(`Unit.cpp:6724-6726`) multiplies spell damage by
`GetTotalAuraMultiplierByMiscMask(SPELL_AURA_MOD_DAMAGE_DONE_VERSUS, victim->
GetCreatureTypeMask())`, which the represented damage path omitted.
`represented_target_creature_type_mask_like_cpp` implements C++
`Unit::GetCreatureTypeMask` (`Unit.cpp:8796-8800`) from the world creature's
template `creature_type` through the already-present
`CreatureTemplateLifecycleStoreLikeCpp` (players and unknown templates return 0),
and `represented_spell_damage_pct_done_like_cpp` multiplies its school maximum by
every active aura-168 effect whose misc intersects the victim's type mask.
`CreatureTemplateLifecycleRecordLikeCpp` derives `Default` so fixtures can build
a template with only the fields they exercise. Boundary: the aurastate (303),
target-aura-mechanic (249) and `MOD_DAMAGE_DONE_FOR_MECHANIC` terms plus the
family scripts remained unrepresented; the first two were resolved by the
versus-aurastate and mechanic-based multiplier entries above, leaving the family
scripts, and a target whose creature type is unavailable keeps only the school
percentage. Evidence: the new session test
registers a creature whose template type is 7 (bit 6), applies a versus aura for
an unrelated type (bit 1) and asserts 100 damage, then the matching aura (+100%)
and asserts 200 damage; wow-data --lib 752/0 and wow-world --lib 3934/0/1;
format, `git diff --check`, the physical ratchet and `validation-v2 quick`
(manifest `20260917T060718.732178Z-2412155-quick.json`) pass. No live
DB/restart/relogin QA. #61 stays open for the remaining aurastate/mechanic and
victim-side terms, the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 missing-health healing scaling — 2026-09-17, implementation `c4453097`,
integrated as `1043da08` by PR #1043:** `Unit::SpellHealingPctDone`
(`Unit.cpp:7204-7229`) scales healing done by the target's missing health for
`SPELL_AURA_MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH` (354), which the
represented heal path omitted.
`represented_spell_healing_bonus_done_like_cpp` now applies
`AddPct(DoneTotalMod, CalculatePct(amount, healthPctDiff))` for every active
aura-354 effect when the victim is the session player, using
`resolved_player_vitals_like_cpp` for the missing-health percentage. Boundary:
the aura's `AuraEffect::IsAffectingSpell` family/flag gate is not represented, so
the term applies to any represented heal the aura owner casts; creature victims
keep it unapplied because their health and auras are not represented. Evidence:
the heal scenario removes the taken auras, applies aura 354 with amount 100 and
sets the player to 500/1000 health; the fourth cast asserts 382 healing
(`int32((100 + int32(140 * 0.5)) * (1.5 * 1.5))`) taking the player to 882;
wow-data --lib 752/0 and wow-world --lib 3933/0/1; format, `git diff --check`,
the physical ratchet and `validation-v2 quick` (manifest
`20260917T055914.037802Z-2404133-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the remaining caster/victim modifier gates, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 `SpellHealingBonusTaken` — 2026-09-17, implementation `b5a2f7a7`,
integrated as `490b1f92` by PR #1041:** `Unit::SpellHealingBonusTaken`
(`Unit.cpp:7231-7239`) multiplies healing a unit receives by the most positive
and most negative active `SPELL_AURA_MOD_HEALING_PCT` amounts, which the
represented heal path omitted.
`represented_spell_healing_bonus_taken_like_cpp`
(`session/spell_effects/effect_combat.rs`) mirrors
`GetMaxNegativeAuraModifier`/`GetMaxPositiveAuraModifier` and applies each with
`AddPct`, truncating the product at zero; `apply_heal_from_caster_like_cpp`
applies it to spell-driven heals when the target is the session player, the only
represented aura owner. Boundary: creature targets keep the raw amount because
their auras are not represented, the Nourish druid case is not modelled, and
non-spell heals are unchanged. Evidence: the heal scenario applies
`SPELL_AURA_MOD_HEALING_PCT` -50 and +25 after the second cast and asserts the
third heal is `int32(255 * (1 - 0.5) * (1 + 0.25)) = 159`, taking the player from
580 to 739 health; wow-data --lib 752/0 and wow-world --lib 3933/0/1; format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260917T055228.342758Z-2396282-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the remaining caster/victim modifier gates, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 victim `ModHealing` term — 2026-09-17, implementation `2d158052`,
integrated as `5924714b` by PR #1039:** `Unit::SpellHealingBonusDone`
(`Unit.cpp:7123-7124`) also adds the victim's
`GetTotalAuraModifierByMiscMask(SPELL_AURA_MOD_HEALING, schoolMask)` to the
advertised benefit, which the direct-heal path of PR #1037 omitted.
`represented_spell_healing_bonus_done_like_cpp` now takes the `target_guid` and
adds the `SPELL_AURA_MOD_HEALING` (115) sum whose misc intersects the spell's
`SpellMisc.SchoolMask` when the victim is the session player, the only
represented aura owner; the `SPELL_EFFECT_HEAL` branch passes the target guid
through. Boundary: creature victims keep the term unapplied because their auras
are not represented, and the `BonusCoefficientFromAP` table, the periodic-leech
suppression, the spell-mod coefficient adjustment and the scripted handlers
remain open. Evidence: the heal scenario applies a holy
`SPELL_AURA_MOD_HEALING` (+40) after the first cast and asserts the second heal
is `int32((100 + int32(140 * 0.5)) * 1.5) = 255`, taking the player from 325 to
580 health; wow-data --lib 752/0 and wow-world --lib 3933/0/1; format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260917T054507.143715Z-2388072-quick.json`) pass. No live DB/restart/relogin QA.
#61 stays open for the remaining victim/caster modifier gates, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 direct-heal spell-power bonus — 2026-09-17, implementation `f7416510`,
integrated as `1a19dbcd` by PR #1037:** `Unit::SpellHealingBonusDone`
(`Unit.cpp:7100-7183`) was not represented, so `SPELL_EFFECT_HEAL` applied the
raw `EffectBasePoints`. `represented_spell_healing_bonus_done_like_cpp`
(`session/spell_effects/effect_combat.rs`) now implements the player-caster
`int32(max(float(healamount + int32(SpellBaseHealingBonusDone(schoolMask) *
BonusCoefficient)) * DoneTotalMod, 0.0f))` with `DoneTotalMod` read from the
published `mod_healing_done_percent`, and
`represented_spell_base_healing_bonus_done_like_cpp` mirrors
`Unit::SpellBaseHealingBonusDone` (`Unit.cpp:7282-7315`): the
`SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` short circuit, otherwise the
`SPELL_AURA_MOD_HEALING_DONE` sum whose misc is zero or intersects the school
mask, plus the published `GetBaseSpellPowerBonus()`, the mana-class intellect
term (represented by the base-mana row) and the
`SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT` percentages. The
`SPELL_EFFECT_HEAL` branch in `execution.rs` routes the base through the helper
using the same `SpellMisc.SchoolMask` resolution as the damage path. Boundaries:
the victim `SPELL_AURA_MOD_HEALING` term, the `BonusCoefficientFromAP` table, the
periodic-leech suppression, the spell-mod coefficient adjustment and the scripted
handlers remain unrepresented; creature casters keep the raw value and a missing
`SpellMisc` row or canonical snapshot fails closed. Evidence: the new session
test installs a canonical player with `spell_power = 100`,
`mod_healing_done_percent = 1.5`, a holy `SpellMisc` row and a heal spell with
`effect_bonus_coefficient = 0.5` and 100 base healing, casts it at itself and
asserts 325 health (`(100 + 50) * 1.5`); wow-data --lib 752/0 and wow-world --lib
3933/0/1; format, `git diff --check`, the physical ratchet and
`validation-v2 quick` (manifest `20260917T053806.484070Z-2379570-quick.json`)
pass. No live DB/restart/relogin QA. #61 stays open for the versus/mechanic and
victim-modifier gates, the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 school damage percentage — 2026-09-17, implementation `32572914`,
integrated as `d94a13d4` by PR #1035:** `Unit::SpellDamagePctDone`'s player
branch was missing from the school-damage path, so the
`ActivePlayerData::ModDamageDonePercent` values published by PR #1015 did not
affect damage. `represented_spell_damage_pct_done_like_cpp`
(`session/spell_effects/effect_combat.rs`) implements
`maxModDamagePercentSchool` (`Unit.cpp:6683-6772`): the highest published
`mod_damage_done_percent` among the spell's `SpellMisc.SchoolMask` schools, and
`represented_spell_damage_bonus_done_like_cpp` now multiplies
`(pdamage + DoneTotal)` by it before the final
`int32(max(float(...), 0.0f))` truncation. Boundaries: the versus-creature-type
(168), versus-aurastate (303), target-aura-mechanic (249) and
`MOD_DAMAGE_DONE_FOR_MECHANIC` multipliers plus the spell-family scripted cases
remain unrepresented. Evidence: the new session test installs a canonical player
with holy `mod_damage_done_percent = 1.5` and fire `2.5`, a holy|fire `SpellMisc`
row and a 100 base-damage spell, casts it at a 1000 HP creature and asserts 750
HP remaining (`100 * max(1.5, 2.5)`); the flat-term test stays green; wow-data
--lib 752/0 and wow-world --lib 3932/0/1; format, `git diff --check`, the
physical ratchet and `validation-v2 quick` (manifest
`20260917T053142.856354Z-2372583-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the versus/mechanic damage multipliers, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 caster spell-power damage bonus — 2026-09-17, implementation `067a6f91`,
integrated as `ec20d472` by PR #1033:** `Unit::SpellDamageBonusDone`
(`Unit.cpp:6623-6680`) was not represented, so `SPELL_EFFECT_SCHOOL_DAMAGE`
applied the raw `EffectBasePoints` and ignored the caster's spell power and
damage coefficient. `represented_spell_damage_bonus_done_like_cpp`
(`session/spell_effects/effect_combat.rs`) now implements the player-caster
`SPELL_DIRECT_DAMAGE` flat term `int32(max((pdamage +
int32(SpellBaseDamageBonusDone(schoolMask) * BonusCoefficient)) * DoneTotalMod,
0))` with `DoneTotalMod` currently 1.0, and
`represented_spell_base_damage_bonus_done_like_cpp` mirrors
`Unit::SpellBaseDamageBonusDone` (`Unit.cpp:6860-6890`): the
`SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` short circuit, otherwise
`GetTotalAuraModifierByMiscMask(SPELL_AURA_MOD_DAMAGE_DONE, schoolMask)` plus
the published `GetBaseSpellPowerBonus()` plus the
`SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT` terms read from the canonical
snapshot's stats. The school mask comes from the session's existing
`SpellMiscStore` (`SpellMisc.SchoolMask`), so no data-model change was needed,
and the school-damage branch in `execution.rs` routes the base through the
helper before `apply_damage_from_caster_like_cpp`. Boundaries:
`SpellDamagePctDone` (the school `max ModDamageDonePercent` and the
versus/aurastate/mechanic multipliers) is not modelled yet, the represented model
stores one `BonusCoefficient` per spell rather than per `SpellEffectInfo`,
creature casters keep the raw value, and a missing `SpellMisc` row or canonical
snapshot fails closed to the raw value. Evidence: the new session test installs a
canonical player with `spell_power = 100`, a holy `SpellMisc` row and a spell
with `effect_bonus_coefficient = 0.5` and 100 base damage, casts it at a 1000 HP
creature and asserts 850 HP remaining (`(100 + 50) * 1.0`); wow-data --lib 752/0
and wow-world --lib 3931/0/1; format, `git diff --check`, the physical ratchet
and `validation-v2 quick` (manifest
`20260917T052557.573032Z-2365779-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for `SpellDamagePctDone`, the offhand-damage aura scale, the
player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 weapon-enchantment damage term — 2026-09-17, implementation `54c1f8ec`,
integrated as `1e4bb9d4` by PR #1031:** `Player::UpdateDamageDoneMods`
(`Player.cpp:4991-5015`) adds `ITEM_ENCHANTMENT_TYPE_DAMAGE` (2)
`EffectScalingPoints` for the attack's enchanted weapon, plus the same scaled by
the weapon delay for `ITEM_ENCHANTMENT_TYPE_TOTEM` (6) on shamans. The Rust
`SpellItemEnchantmentEntry` never loaded `EffectScalingPoints`, so enchanted
weapon damage was missing. The entry now carries
`effect_scaling_points: [f32; 3]` read from DB2 field 3
(`DB2Structure.h:3573`); `Eq` is no longer derived because the field is `f32`,
and the twenty struct literals across the data/world fixtures gained the field.
`represented_weapon_enchant_damage_like_cpp`
(`session/player_items/equipment_slots.rs`) resolves the attack's usable weapon,
walks every enchantment slot and sums the damage/totem points;
`represented_weapon_damage_flat_like_cpp` now returns `[f32; 3]` (aura sum plus
the enchantment term), and `PlayerSpellBonusInputLikeCpp.weapon_damage_flat`,
`PlayerStatSystemProjectionLikeCpp` and `PlayerEffectiveCombatStatsLikeCpp` move
to `f32`, matching the C++ `SetStatFlatModifier` float storage. Evidence: the new
session test runs the same enchanted sword fixture for a shaman (class 7,
permanent enchant `effect = [DAMAGE, TOTEM]`,
`effect_scaling_points = [25.0, 10.0]`, 2.0 s weapon delay) and a warrior
(class 1), asserting `weapon_damage_flat == [45.0, 0.0, 0.0]` and `[25.0, 0.0,
0.0]`; the wow-data flat/percentage test moves to the `f32` values; wow-data
--lib 752/0 and wow-world --lib 3930/0/1; format, `git diff --check`, the
physical ratchet and `validation-v2 quick` (manifest
`20260917T051430.821411Z-2355689-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 ranged weapon fit — 2026-09-17, implementation `9dd5c131`, integrated as
`616d0163` by PR #1029:** the two preceding entries recorded that the
weapon-fit producers excluded ranged weapon requirements because the
represented `GetWeaponForAttack` helper covered the melee slots only.
`Player::GetWeaponForAttack(RANGED_ATTACK, true)` (`Player.cpp:9243-9270`) reads
`EQUIPMENT_SLOT_RANGED`, so bow/gun/crossbow restricted auras were wrongly
rejected for the ranged attack. `represented_usable_weapon_item_id_like_cpp`
now resolves the ranged slot; the callers that mirror
`Player::UpdateExpertise` (`StatSystem.cpp:759-786`) still skip `RANGED_ATTACK`
themselves, because C++ returns early there, while
`UpdateWeaponDependentCritAuras`, `UpdateDamageDoneMods` and
`UpdateDamagePctDoneMods` now fit against the equipped ranged weapon. The
helper's doc comment is corrected accordingly: the ranged early return belongs to
`UpdateExpertise`, not to `GetWeaponForAttack`. Evidence: the new session test
equips a bow, applies a bow-restricted `SPELL_AURA_MOD_WEAPON_CRIT_PERCENT` (+5)
and `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE` (+100), and asserts `ranged_crit_pct ==
10`, `crit_pct == 5`, `offhand_crit_pct == 5` and `weapon_damage_pct == [1.0,
0.5, 2.0]`; wow-world --lib 3929/0/1; format, `git diff --check`, the physical
ratchet and `validation-v2 quick` (manifest
`20260917T044535.708128Z-2334728-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the weapon-enchantment damage term, the offhand-damage
aura scale, the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and
`SetPvPDeath`, alternate powers, rune regeneration and live DB/restart/relogin
QA.

**#61 `Unit::UpdateDamageDoneMods` representation — 2026-09-17,
implementation `7c0232da`, integrated as `cca7331a` by PR #1027:**
`Unit::UpdateDamageDoneMods` (`Unit.cpp:8997-9027`) sets the
`UNIT_MOD_DAMAGE_*` `TOTAL_VALUE` from the physical
`SPELL_AURA_MOD_DAMAGE_DONE` (13) sum filtered by
`Player::CheckAttackFitToAuraRequirement`, and `Unit::CalculateMinMaxDamage`
adds it inside the total percentage. RustyCore never modelled the flat term, so
melee damage bonuses from that aura were missing from every represented weapon
range. `represented_weapon_damage_flat_like_cpp`
(`session/player_items/equipment_slots.rs`) reuses the represented weapon-fit
helper also used by the crit, expertise and damage-percentage producers;
`PlayerSpellBonusInputLikeCpp.weapon_damage_flat` (default `[0; 3]`) flows
through `PlayerStatSystemProjectionLikeCpp` into
`effective_weapon_damage_ranges_like_cpp` as
`((weapon + ap) + flat) * pct` (C++ `((weapon + baseValue) * basePct +
totalValue) * totalPct`), and `PlayerEffectiveCombatStatsLikeCpp` publishes the
per-attack values. Boundary: the weapon-enchantment
`ITEM_ENCHANTMENT_TYPE_DAMAGE`/`TOTEM` term that
`Player::UpdateDamageDoneMods` (`Player.cpp:4965-5015`) adds for an enchanted
weapon is not represented and remains a separate gate. Evidence: the wow-data
test pins the flat-before-percentage arithmetic and the offhand factor; the
session test extends the weapon-damage scenario with an item-neutral +20 physical
aura, asserting `weapon_damage_flat == [20, 20, 20]` and a mainhand range growth
of `20 * 3.0`; wow-data --lib 752/0, wow-entities --lib 940/0 and wow-world --lib
3928/0/1; format, `git diff --check`, the physical ratchet and
`validation-v2 quick` (manifest `20260917T043815.435806Z-2327221-quick.json`)
pass. No live DB/restart/relogin QA. #61 stays open for the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers,
rune regeneration and live DB/restart/relogin QA.

**#61 `Unit::UpdateDamagePctDoneMods` representation — 2026-09-17,
implementation `720bcb0a`, integrated as `8a55e6c5` by PR #1025:** the
`UNIT_MOD_DAMAGE_*` `TOTAL_PCT` that `Unit::CalculateMinMaxDamage` applies to
both weapon range bounds was never represented, so the offhand 50% penalty and
the physical damage-percentage auras were missing. `represented_weapon_damage_pct_like_cpp`
(`session/player_items/equipment_slots.rs`) now applies the C++ base factor
(mainhand and ranged 1.0, offhand 0.5) times every active
`SPELL_AURA_MOD_DAMAGE_PERCENT_DONE` (79) effect covering
`SPELL_SCHOOL_MASK_NORMAL` and fitting the attack's weapon through
`Player::CheckAttackFitToAuraRequirement` (`Player.cpp:8145-8156`);
`resolved_aura_effects_with_spell_and_misc_like_cpp`
(`session/spell_state/aura.rs`) exposes the owning spell id the fit needs.
`PlayerSpellBonusInputLikeCpp.weapon_damage_pct` (default `[1.0, 0.5, 1.0]`)
flows through `PlayerStatSystemProjectionLikeCpp` into
`effective_weapon_damage_ranges_like_cpp` and is published on
`PlayerEffectiveCombatStatsLikeCpp`. **Deliberate departure, recorded:** the
source multiplies the offhand factor by
`GetTotalAuraModifier(SPELL_AURA_MOD_OFFHAND_DAMAGE_PCT, ...)`, a raw sum that is
0 when no such aura is active and therefore zeroes offhand damage on every
equip/login; RustyCore keeps the evident intent (0.5 base times the physical
multiplier) and does not apply that term, leaving
`SPELL_AURA_MOD_OFFHAND_DAMAGE_PCT` as a separate gate that needs capture
evidence for its scale. Ranged weapon requirements are excluded rather than
resolved because the represented `GetWeaponForAttack` helper covers the melee
slots only. Evidence: the wow-data test pins the projection copy; the session
test equips a sword mainhand and a dagger offhand and asserts `[1.0, 0.5, 1.0]`
with the offhand range at half the mainhand one, `[1.5, 0.75, 1.5]` after an
item-neutral +50% aura and `[3.0, 0.75, 1.5]` after a sword-restricted +100%
aura; wow-data --lib 751/0, wow-entities --lib 940/0 and wow-world --lib
3928/0/1; format, `git diff --check`, the physical ratchet and
`validation-v2 quick` (manifest `20260917T042933.856352Z-2316020-quick.json`)
pass. No live DB/restart/relogin QA. #61 stays open for the offhand-damage aura
scale, ranged weapon requirements, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 `VersatilityBonus` publication — 2026-09-17, implementation `a065cd97`,
integrated as `3bff1bf7` by PR #1023:** the create block and the narrow values
update wrote `ActivePlayerData::VersatilityBonus` (bit 56) as a constant 0.0.
`AuraEffect::HandleModVersatilityByPct` (`SpellAuraEffects.cpp:3797-3808`) now
supplies it: the sum of every active `SPELL_AURA_MOD_VERSATILITY` (471) amount,
stored through `Player::SetVersatilityBonus` (`Player.h:2753`) with the
`SetUpdateFieldStatValue` zero clamp. The same handler's
`UpdateHealingDonePercentMod` versatility term is commented out in C++ and stays
absent, and `UpdateVersatilityDamageDone` writes no field, so only the bonus is
produced. `represented_spell_bonus_like_cpp` sums the aura amounts,
`PlayerSpellBonusInputLikeCpp` carries `versatility_bonus_aura`,
`PlayerStatSystemProjectionLikeCpp` exposes the clamped `versatility_bonus`, and
`PlayerEffectiveCombatStatsLikeCpp` publishes it; `PlayerCombatStats`,
`PlayerCreateData` and `PlayerStatChanges` carry it so both writers emit the
real value (bit 56, already masked). Evidence: the wow-data test pins the sum,
the zero clamp and the default; the extended session test applies aura 471 with
amount 200 and asserts 200.0; `scenarios_spell_state_22` asserts the 0.0 default;
the `scenarios_3` byte test locates parent-38 field 16 (bit 56) and reads the
value back; wow-data --lib 751/0, wow-entities --lib 940/0, wow-world --lib
3927/0/1 and wow-packet --lib 742/0; format, `git diff --check`, the physical
ratchet (three explained fixture ceilings: `update_tests/mod.rs` 216→218,
`items.rs` 3902→3903 and `world_entry.rs` 2780→2781) and `validation-v2 quick`
(manifest `20260917T041731.698070Z-2298918-quick.json`) pass. No live
DB/restart/relogin QA. #61 stays open for the melee damage percentage consumer,
the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 override percentage field publication — 2026-09-17, implementation
`204958b6`, integrated as `b75fb666` by PR #1021:** the create block and the
narrow values update wrote `OverrideSpellPowerByAPPercent` and
`OverrideAPBySpellPowerPercent` as an unconditional `-1.0` that has no C++ or
capture source in this tree. C++ accumulates the aura amounts into
zero-initialized fields: `AuraEffect::HandleOverrideSpellPowerByAttackPower`
(`SpellAuraEffects.cpp:3770-3781`) for
`SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` (366) and
`HandleOverrideAttackPowerBySpellPower` (`3783-3796`) for
`SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` (404). The projection now exposes
`override_spell_power_by_ap_percent` and `override_ap_by_spell_power_percent`
(0.0 when absent), `PlayerEffectiveCombatStatsLikeCpp` publishes them, and
`PlayerCombatStats`/`PlayerCreateData`/`PlayerStatChanges` carry them so both
writers emit the accumulated values (bits 65-66, already masked). The
attack-power branch keeps the C++ `HasAuraType` presence rule, now distinct from
the published value. Evidence: the wow-data test pins the defaults and the 366
value; the session test asserts 0.0 before any override aura and 50.0 after the
404 aura, and a dedicated level-80 warrior test drives 366 to
`int32(CalculatePct(220, 50) + 0.5) = 110` on both flat spell fields with attack
power unchanged; the `scenarios_3` byte test locates parent-38 fields 25-26
(bits 65-66) and reads both floats; `scenarios_spell_state_22` pins the 0.0
defaults; wow-data --lib 751/0, wow-entities --lib 940/0, wow-world --lib
3927/0/1 and wow-packet --lib 742/0; format, `git diff --check`, the physical
ratchet (three explained fixture ceilings: `update_tests/mod.rs` 212→216,
`items.rs` 3900→3902 and `world_entry.rs` 2778→2780) and `validation-v2 quick`
(manifest `20260917T040804.079374Z-2286951-quick.json`) pass. No live
DB/restart/relogin QA. #61 stays open for the melee damage percentage consumer,
the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 `ModTargetResistance` and item spell penetration publication —
2026-09-17, implementation `2715525d`, integrated as `5fd96c75` by PR #1019:**
the create block and the narrow values update wrote both target resistance
fields as constants. `AuraEffect::HandleModTargetResistance`
(`SpellAuraEffects.cpp:3507-3530`) now feeds them: an effect covering
`SPELL_SCHOOL_MASK_NORMAL` adds to `ModTargetPhysicalResistance`, and one
covering the whole `SPELL_SCHOOL_MASK_SPELL` (`SharedDefines.h:340-343`) adds to
`ModTargetResistance`; `Player::ApplySpellPenetrationBonus`
(`StatSystem.cpp:231-235`) subtracts the item/enchant
`ITEM_MOD_SPELL_PENETRATION`. `represented_spell_bonus_like_cpp` sums the two
aura sets and passes the `spell_penetration_bonus` accumulator;
`PlayerSpellBonusInputLikeCpp` carries `target_resistance_aura`,
`item_spell_penetration` and `target_physical_resistance_aura`;
`PlayerStatSystemProjectionLikeCpp` exposes `mod_target_resistance`
(`aura − item`) and `mod_target_physical_resistance`; and
`PlayerEffectiveCombatStatsLikeCpp` publishes both. `PlayerCombatStats`,
`PlayerCreateData` and `PlayerStatChanges` carry the fields so the create block
and the narrow values update write them instead of the hardcoded zeros (bits
67-68, already masked). Boundary: `ModHealingPercent`,
`ModPeriodicHealingDonePercent` and `ModSpellPowerPercent` keep their 1.0 create
value because C++ has no writer for them, and the
`OverrideSpellPowerByAPPercent`/`OverrideAPBySpellPowerPercent` create sentinels
are unchanged and remain a separate contract decision. Evidence: the wow-data
test pins the projection arithmetic; the extended session test applies aura 123
with the magic mask (20) and armor mask (30) plus an item penetration bonus (15)
and asserts 5/30; `scenarios_spell_state_22` asserts the item path leaves both at
zero; the `scenarios_3` byte test locates parent-38 fields 27 and 28 (bits 67-68)
and reads back both values; wow-data --lib 751/0, wow-entities --lib 940/0,
wow-world --lib 3926/0/1 and wow-packet --lib 742/0; format, `git diff --check`,
the physical ratchet (two explained fixture ceilings: `update_tests/mod.rs`
208→212 and `world_entry.rs` 2776→2778) and `validation-v2 quick` (manifest
`20260917T035358.228914Z-2263464-quick.json`) pass. No live DB/restart/relogin QA.
#61 stays open for the override percentage contract, the melee damage percentage
consumer, the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and
`SetPvPDeath`, alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 `ModHealingDonePercent` publication — 2026-09-17, implementation
`cbbb3a49`, integrated as `98a45d75` by PR #1017:** `Player::UpdateHealingDonePercentMod`
(`StatSystem.cpp:588-599`) is now produced: the value starts at 1.0, multiplies
`1 + amount/100` over every active `SPELL_AURA_MOD_HEALING_DONE_PERCENT` (136)
effect and is clamped at zero, matching the re-run from
`AuraEffect::HandleModHealingDonePct` (`SpellAuraEffects.cpp:3645-3654`).
`represented_spell_bonus_like_cpp` computes it, `PlayerSpellBonusInputLikeCpp`
carries `healing_done_percent` with a manual `Default` of 1.0,
`PlayerStatSystemProjectionLikeCpp` exposes `mod_healing_done_percent` and
`PlayerEffectiveCombatStatsLikeCpp` publishes it. `PlayerCombatStats` and
`PlayerCreateData` carry `mod_healing_done_pct` so the create block writes the
real multiplier instead of 1.0, and `PlayerStatChanges`'s existing field is fed
from the projection for the narrow values update (bit 61, already emitted).
Boundary: `ModHealingPercent`, `ModPeriodicHealingDonePercent` and
`ModSpellPowerPercent` have no C++ writer and keep their 1.0 create value, while
`ModTargetResistance`/`ModTargetPhysicalResistance` (aura 123 plus the item
spell-penetration term) remain hardcoded and are the next gate. Evidence: the
wow-data test pins the projection copy; the extended session test drives aura 136
twice on a level-80 priest to `(1+0.5)*(1+1.0) = 3.0`; `scenarios_spell_state_22`
pins the 1.0 create value; the `scenarios_3` byte test locates parent-38 field 21
(bit 61) and asserts the emitted multiplier; wow-data --lib 751/0, wow-entities
--lib 940/0, wow-world --lib 3926/0/1 and wow-packet --lib 742/0; format,
`git diff --check`, the physical ratchet (two explained fixture ceilings:
`update_tests/mod.rs` 207→208 and `world_entry.rs` 2775→2776) and
`validation-v2 quick` (manifest `20260917T034615.374132Z-2251853-quick.json`)
pass. No live DB/restart/relogin QA. #61 stays open for the target
resistance/spell-penetration fields, the melee damage percentage consumer, the
player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 `ModDamageDonePercent` publication — 2026-09-17, implementation
`b11a04cb`, integrated as `c3645a79` by PR #1015:** the last per-school
`ActivePlayerData` spell field is now produced and published.
`AuraEffect::HandleModDamagePercentDone` (`SpellAuraEffects.cpp:4525-4548`) sets
`ModDamageDonePercent[i]` to
`GetTotalAuraMultiplierByMiscMask(SPELL_AURA_MOD_DAMAGE_PERCENT_DONE, 1 << i)` —
the product of `1 + amount/100` over the effects intersecting that school, with
the 1.0 create value kept when none does. `represented_spell_bonus_like_cpp`
computes that array, `PlayerSpellBonusInputLikeCpp` carries it with a manual
`Default` that starts at 1.0, and `PlayerEffectiveCombatStatsLikeCpp` publishes
`mod_damage_done_percent`. `PlayerCombatStats`, `PlayerCreateData` and
`PlayerStatChanges` carry it so the create block and the narrow values update
emit the real multiplier; the narrow writer sets the `ModDamageDonePercent` bits
(291-297 → block 9 bits 3-9) and writes the value after `ModDamageDoneNeg`,
completing the `SpellCrit/ModDmgDonePos/ModDmgDoneNeg/ModDmgDonePct` interleave.
Boundary: the physical-school `Unit::UpdateAllDamagePctDoneMods()`
(`Unit.cpp:9074-9078`) melee damage percentage remains unrepresented and open.
Evidence: the wow-data test pins the projection copy; the extended session test
drives aura 79 on a level-80 priest to holy `(1+0.5)*(1+1.0) = 3.0`, fire 1.25
and 1.0 elsewhere; `scenarios_spell_state_22` pins the 1.0 create value; the
`scenarios_3` byte test pins block 9 `0x000003FF` and every school's percent
value; wow-data --lib 751/0, wow-entities --lib 940/0, wow-world --lib 3926/0/1
and wow-packet --lib 742/0; format, `git diff --check`, the physical ratchet (two
explained fixture ceilings: `update_tests/mod.rs` 205→207 and `world_entry.rs`
2774→2775) and `validation-v2 quick` (manifest
`20260917T033705.720788Z-2239874-quick.json`) pass. No live DB/restart/relogin QA.
#61 stays open for the melee
damage percentage consumer, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 narrow values-update negative spell field — 2026-09-17, implementation
`a7c53527`, integrated as `a584faf0` by PR #1013:** the boundary recorded by the
wire-publication entry above is closed. `PlayerStatChanges` gains
`mod_damage_done_neg: [i32; 7]`, the last C++ `ActivePlayerData` spell field the
narrow path was missing; the equipment projection (`items.rs`) and the shared
`session/mod.rs` fixture populate it. The narrow writer (`state_5.rs`) now sets
the `ModDamageDoneNeg` mask bits (284-290 → block 8 bits 28-31 and block 9 bits
0-2) and emits each school's negative value after the positive one, matching the
`SpellCrit/ModDmgDonePos/ModDmgDoneNeg/ModDmgDonePct` interleave from
`Player::UpdateSpellDamageAndHealingBonus` (`StatSystem.cpp:171-197`).
`ModDamageDonePercent` (291-297) stays outside this runtime writer and remains
open. Evidence: the `scenarios_3` byte test pins group 0 with block 9, block 8
`0xFFFFE000`, block 9 `0x00000007` and reads back every school's negative value;
`scenarios_spell_state_22` asserts the item path leaves the negative array zero;
wow-packet --lib 742/0 and wow-world --lib 3926/0/1; format, `git diff --check`,
the physical ratchet (one explained fixture ceiling: `update_tests/mod.rs`
204→205) and `validation-v2 quick` (manifest
`20260917T032401.703353Z-2226427-quick.json`) pass. No live DB/restart/relogin QA.
#61 stayed open for `ModDamageDonePercent`, delivered by the entry above, and
remains open for the melee damage percentage consumer, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 spell damage and healing field wire publication — 2026-09-17,
implementation `9847962e`, integrated as `2307fb68` by PR #1011:** the packet
adapters no longer send the item `spell_power` scalar to every magic school and
to healing. `PlayerCombatStats`, `PlayerCreateData` and `PlayerStatChanges`
replace `spell_power` with the C++ field set — `mod_damage_done_pos[7]`,
`mod_damage_done_neg[7]` (create data) and `mod_healing_done_pos` — so each
struct has one authority per value. The create block (`state_2.rs`,
`StatSystem.cpp:171-197`) writes the per-school positive and negative entries plus
the healing field from those arrays, and the narrow values update
(`state_5.rs`) writes `ModDamageDonePos[i]` and `ModHealingDonePos` from the same
arrays; the wire layout is unchanged because the scalar occupied exactly those
slots. The login `PlayerCombatStats` snapshot (`world_entry.rs`) and the equip
`PlayerStatChanges` (`items.rs`) copy the projection fields, and the
`session/mod.rs` fixture helper states that it has no aura/stat producers.
Boundary: the narrow values-update path still leaves the `ModDamageDoneNeg` bits
(284-290) unset, exactly as before this unit; the create block publishes the
negative entries and the generic `state_4.rs` writer already supports them once
their mask bits are set. Evidence: `scenarios_spell_state_22` asserts the item
spell power reaches `mod_healing_done_pos` and every magic school; wow-packet
--lib 742/0 and wow-world --lib 3926/0/1; format, `git diff --check`, the
physical ratchet (two explained fixture ceilings: `update_tests/mod.rs` 201→204
and `world_entry.rs` 2772→2774) and `validation-v2 quick` (manifest
`20260917T031859.286082Z-2219004-quick.json`) pass. No live DB/restart/relogin QA.
#61 stayed open for the narrow values-update negative bits, delivered by the
narrow values-update entry above, and remains open for
`ModDamageDonePercent`, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers,
rune regeneration and live DB/restart/relogin QA.

**#61 spell damage and healing done producers — 2026-09-17, implementation
`92f0f550`, integrated as `bd449980` by PR #1009:** `Player::UpdateSpellDamageAndHealingBonus`
(`StatSystem.cpp:171-197`) is now produced. `PlayerSpellBonusInputLikeCpp` carries
the C++ terms: `GetBaseSpellPowerBonus()`, the `SPELL_AURA_MOD_DAMAGE_DONE` (13)
per-school net sum and its negative `ModDamageDoneNeg` part,
`SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT` (174, school mask plus stat) and
`SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT` (175, stat),
`SPELL_AURA_MOD_HEALING_DONE` (135), and
`ActivePlayerData::OverrideSpellPowerByAPPercent` (366).
`calculate_player_stat_system_like_cpp` computes the fields in C++
`UpdateAllStats` order — spell fields first, then `UpdateAttackPowerAndDamage`,
then the 366 pass that replaces both bonuses with
`int32(CalculatePct(GetTotalAttackPowerValue(BASE_ATTACK), pct) + 0.5f)` — so the
404 branch now derives `min(ModHealingDonePos, ModDamageDonePos[HOLY..MAX])` from
the real fields instead of the item scalar, and `SetUpdateFieldStatValue` clamps
both published fields at zero. The healing path adds `max(0, Intellect)` when the
class base-mana row is nonzero (`GetPowerIndex(POWER_MANA) != MAX_POWERS`), and
`Player::ApplySpellPowerBonus`'s early return while 366 is present is represented
by zeroing `GetBaseSpellPowerBonus()`. `PlayerEffectiveCombatStatsLikeCpp`
publishes `mod_damage_done_pos`, `mod_damage_done_neg` and
`mod_healing_done_pos`. Boundary: the create-block and values-update adapters
published only the scalar `spell_power` at that point; the per-school arrays
are published by the spell-field wire entry above. Evidence: a pure test pins the per-school
positive/negative split, the mana intellect term and the 366 override, and a
session test drives 13/135/174/175 on a level-80 priest to holy 150, fire 120
with `ModDamageDoneNeg[fire] = -50` and healing 187, then 404 to both attack mods
50; `wow-data --lib` 751/0, `wow-entities --lib` 940/0, `wow-world --lib`
3926/0/1, format, `git diff --check`, the physical ratchet and `validation-v2
quick` (manifest `20260917T030821.891532Z-2209220-quick.json`) pass. No live
DB/restart/relogin QA. #61 stayed open for the packet publication of those fields,
delivered by the spell-field wire entry above, and remains open for the
player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 override attack power by spell power — 2026-09-17, implementation
`8ee397c5`, integrated as `12efaf8f` by PR #1007:** the last unrepresented branch
of `Player::UpdateAttackPowerAndDamage` (`StatSystem.cpp:333-403`) is applied:
while `SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` (`SpellAuraDefines.h:499`) is
active, C++ skips the strength/agility/level base and sets `BASE_VALUE` for both
the melee and the ranged unit mod to `CalculatePct(float(minSpellPower),
OverrideAPBySpellPowerPercent)` (`StatSystem.cpp:341-379`), where `minSpellPower`
is `min(ModHealingDonePos, ModDamageDonePos[HOLY..MAX])`.
`PlayerStatSystemInputLikeCpp` now carries
`attack_power_override_by_spell_power: Option<(i32, f32)>`; the pure stat system
replaces both bases when it is `Some`, preserving the `int32(base_attPower)`
truncation, and otherwise keeps the previous arithmetic. The producer sums every
active effect amount, mirroring the `ApplyModUpdateFieldValue` accumulator in
`AuraEffect::HandleOverrideAttackPowerBySpellPower`
(`SpellAuraEffects.cpp:3785-3796`); C++ tests `HasAuraType`, so an active effect
whose summed percent is zero still overrides. The minimum spell power is the
represented item spell power that `Player::ApplySpellPowerBonus`
(`StatSystem.cpp:153-168`) publishes to `ModHealingDonePos` and the magic
`ModDamageDonePos` entries; the `SPELL_AURA_MOD_DAMAGE_DONE`/`MOD_HEALING_DONE`
producers that would widen those fields are implemented by the spell
damage/healing done entry above. The override
reaches the existing consumers unchanged: `Unit::GetTotalAttackPowerValue`
(threat) and `effective_weapon_damage_ranges_like_cpp`
(`Unit::CalculateMinMaxDamage`). Evidence: a pure stat-system test pins the
replacement, truncation and zero-percent presence rule, an end-to-end session
test moves the baseline 220/-10 to 200/200 with 1,000 spell power and two
stacking 15+5 percent effects; `wow-data --lib` 750/0, `wow-world --lib`
3925/0/1, format, `git diff --check`, the physical ratchet and `validation-v2
quick` (manifest `20260917T025325.633908Z-2197184-quick.json`) pass. No live
DB/restart/relogin QA. #61 stays open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration, the packet publication of the spell fields and live
DB/restart/relogin QA.

**Seven stale `wow-world --lib` expectations — 2026-09-17, implementation
`2534b97e`, integrated as `343be766` by PR #1005 (test harness, not gameplay
progress):** the seven scenarios left red by PR #1003 were each reproduced on
clean base `6b46be23` before correction; none was a production regression and no
production code changed. (1) `login_load_adds_default_void_item_appearance_like_cpp`
installed a canonical Player with no learned proficiencies and then asserted
`CollectionMgr::CanAddAppearance` (`CollectionMgr.cpp:649-726`), which requires
`Player::GetWeaponProficiency() & (1 << subclass)`; a real character learns its
class proficiency spells on first login (`CharacterHandler.cpp:1284-1288` casting
`playercreateinfo_cast_spell`), whose `SPELL_EFFECT_PROFICIENCY` runs
`Player::AddWeaponProficiency` (`SpellEffects.cpp:1785-1804`, `Player.h:1433`), so
the fixture now seeds the one-handed-sword mask on the Player it installs.
(2) `repeated_login_attachment_preserves_loaded_reputation_for_final_save`
installed the canonical Player without the loaded identity, so the second
`send_login_sequence` ensure read a race change and reinitialized the manager;
the fixture now publishes the loaded race/class/level first, matching
`Player::LoadFromDB` (`CharacterHandler.cpp:1070`). (3) The logout save-snapshot
scenario still expected the stale session level 10 while `Player::SaveToDB` reads
canonical `Unit::GetLevel` (the snapshot itself already expected 42).
(4) The durable creature rail drain now starts with the `Unit::Kill`
creature-killer durability loss (`Unit.cpp:10639-10648`) before the victim health
presentation, and the fall-land lethal scenario adds
`Player::EnvironmentalDamage`'s durability loss (`Player.cpp:667-669`). (5) The
two `scenarios_world_entities_16` quest-giver queries set the level before
canonical adoption, which overwrote it. Evidence: `wow-world --lib` moves from
3917 passed/7 failed to **3924 passed/0 failed/1 ignored**; format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260917T024356.370256Z-2188249-quick.json`) pass. This restores the suite as
usable evidence; it does not add gameplay parity and no live DB/restart/relogin
QA.

**Represented quest-share party fixture identity — 2026-09-17, implementation
`05d0d5e4`, integrated as `1bc1f9c3` by PR #1003 (test harness, not gameplay progress):** `install_represented_party`
registered the receiver and only then adopted the synthetic canonical Player,
whose identity had never been set. Since the registry movement publication
snapshots `player_level_like_cpp()` (the canonical owner's level, C++
`Unit::GetLevel`), every later `sync_player_registry_state_like_cpp()` overwrote
the registry level with 0 and the represented `SatisfyQuestMinLevel` gate rejected
the receiver with `QUEST_PUSH_REASON_LOW_LEVEL` (22) before the intended
prerequisite, reputation, level or expansion branches ran; 21
`push_quest_to_party` scenarios were red for that artificial reason. The fixture
now re-applies the loaded identity after adoption, matching
`Player::LoadFromDB`, which builds the canonical Player with its identity before
the session registers; production behavior is unchanged. Evidence:
`wow-world --lib` moves from 3896 passed/28 failed to **3917 passed/7 failed**,
`push_quest_to_party` 49/49 green; format, `git diff --check`, the physical
ratchet and `validation-v2 quick` (manifest
`20260917T022350.040983Z-2176206-quick.json`, 37.9 s) pass. The 7 remaining
`wow-world --lib` failures — `reputation_retention`,
`movement_fall_land_lethal`, `void_storage` login appearance,
`scenarios_persistence_2` logout snapshot, two `scenarios_world_entities_16`
quest-giver queries and `scenarios_world_entities_1` durable creature rail —
each need their own reproduction and C++ contrast and remained an open defect track,
resolved by the seven-stale-expectations entry above. No live DB/restart/relogin QA.

**Save-snapshot manager-lock re-entry — 2026-09-17, implementation `f55d9ef3`, integrated as `ac592c5f` by PR #1001:**
`fixture_player_save_to_db_snapshot_like_cpp` held the canonical map-manager lock
for its `do_for_all_maps` scan and resolved `self.player_level_like_cpp()` inside
that closure; the accessor re-enters the same non-reentrant lock, so both
`save_snapshot_owner` scenarios hung forever (they also hung on the clean base
`b3f36c1b`). The level is now resolved before the lock, with the snapshot fields
unchanged and no production path affected (`current_player_save_to_db_snapshot_like_cpp`'s
header helper only reads the borrowed Player). This was the last known hang:
**`wow-world --lib` now runs to completion with no skips — 3896 passed, 28 failed,
1 ignored, 0 filtered.** The 28 failures are pre-existing (their filters fail
identically on the clean base, re-checked here): 21 `handlers::quest::tests`
`push_quest_to_party` fan-out cases, two `scenarios_world_entities_16`
quest-giver queries, `reputation_retention`, `movement_fall_land_lethal`,
`void_storage` login appearance, `scenarios_persistence_2` logout snapshot and
`scenarios_world_entities_1` durable creature rail; they form a separate defect
track and are not claimed fixed. Focused: `save_snapshot_owner` (5). Format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260917T021722.275999Z-2169415-quick.json`, 38.3 s) pass. No live
DB/restart/relogin QA.

**Session reputation-closure lock re-entry deadlock — 2026-09-17, implementation
`299fe975`, integrated as `3c3f2ce4` by PR #999:** six call sites ran `self.player_race_like_cpp()`/
`self.player_class_like_cpp()` (and the friendship store) inside
`with_reputation_mgr_like_cpp`/`mutate_reputation_mgr_like_cpp`, whose closure
runs while the canonical map-manager `Mutex` is held; those session accessors take
the same non-reentrant lock, so any session with a canonical Player deadlocked.
The identity is now resolved before the closure in
`session/progression/reputation.rs`, `session/quest/objectives.rs`,
`session/world_entities/creature_kill.rs`, `handlers/quest/eligibility.rs` and
the new item-admission helper, with the same values passed to the same manager
calls and no other behavior change. This removes the reputation-family hangs: the
previously stuck
`scenarios_player_items_8::repair_item_handler_requires_repair_npc_and_repairs_single_item_like_cpp`
(1) and the `scenarios_progression` reputation-objective tests (13) now pass, and
`wow-world --lib` (skipping two unrelated pre-existing `save_snapshot_owner`
hangs) completes with 3894 passed and 28 failed whose filters fail identically on
the clean base `b3f36c1b`, so no regression is introduced. The same unit
completes two `CollectionMgr::CanAddAppearance` gates that C++
`Player::CanUseItem(ItemTemplate const*)` applies: the `483`/`55884`
learning-effect pair and the required-faction reputation rank, sharing the new
`represented_item_reputation_rank_like_cpp` and
`represented_item_effect_spell_ids_like_cpp` helpers with the login inventory
admission. Focused: `can_add_item_appearance` (5). Format, `git diff --check`,
the physical ratchet and `validation-v2 quick` (manifest
`20260917T021045.633991Z-2163059-quick.json`, 62.2 s) pass. The two
`save_snapshot_owner` tests still hang on both the clean base and this head and
need their own investigation; the 28 pre-existing `wow-world --lib` failures
(quest party fan-out, fall-lethal values, void-storage appearance, logout
snapshot, quest-giver query, durable creature rail) are unchanged by this unit and
remain a separate defect track. No live DB/restart/relogin QA.

**Collection appearance `CanUseItem` template gates — 2026-09-17, implementation
`a2c8c3bb`, integrated as `a8ba2c3c` by PR #997:** `can_add_item_appearance_represented_like_cpp` now runs the
`Player::CanUseItem(ItemTemplate const*)` template admission
(`Player.cpp:11069-11125`) that C++ `CollectionMgr::CanAddAppearance` applies
before its own branches: the `ITEM_FLAG2_INTERNAL_ITEM` and Faction
Horde/Alliance flags, the allowable race mask, the required level, the required
skill and rank against the canonical skill value, and the required ability
against the known spells. The transmog-specific gates (source type,
`NoSourceForItemVisual`, quality/artifact, item class/subclass/inventory, learned
weapon proficiency and duplicate) are unchanged. The holiday, reputation, the
483/55884 learning-effect pair and the artifact specialization gates remain
separate slices, documented in place. Focused coverage: a new scenario matrix
asserts a plain usable weapon passes while the internal flag, an opposite-faction
item, an above-level item, an unknown required ability and a race-restricted item
are rejected, and a known required ability passes; `scenarios_player_items_5`
(15), `scenarios_player_items_6` (13), `transmog` (15), `collection` (27),
`heirloom` (9), `scenarios_player_items_1` (53) and `persistence::` (105) stay
green; format, `git diff --check`, the physical ratchet and `validation-v2 quick`
(manifest `20260917T012737.474242Z-2142222-quick.json`, 39.5 s) pass. This is the
collection/transmog vertical, not the F1 equipment-stat vertical of #61. The full
`wow-world --lib` suite stays unusable on this host because several unrelated
pre-existing async tests hang, and `validation-v2 final` stops at the
pre-existing runtime hotspot LOC ratchet, which keeps its drift and was not
regenerated. No live DB/restart/relogin QA.

**Collection appearance weapon-proficiency gate — 2026-09-17, implementation
`4ba42678`, integrated as `978d8ee9` by PR #995:** the represented `CollectionMgr::CanAddAppearance`
(`CollectionMgr.cpp:649-726`) now reads the learned `Player::GetWeaponProficiency`
mask (`Player.h:1432`) instead of the class default
`SetProficiency::default_weapons`, matching the C++ `ITEM_CLASS_WEAPON` branch and
the `!item || !GetPlayer()` guards. `Player::weapon_proficiency_like_cpp` and
`represented_player_weapon_proficiency_like_cpp` are the new read path; the
collection scenarios install the canonical Player that `_owner->GetPlayer()`
requires, seed the mask through the new `grant_learned_weapon_proficiency_like_cpp`
fixture, and drive the already-collected case through the canonical collection
state. A new scenario proves the mask decides admission: a warrior with only Mace
learned collects a mace appearance and is rejected for a sword even though the
warrior class default includes swords. The physical policy records the reviewed
`session_tests.rs` fixture delta (5762→5766). Focused coverage:
`scenarios_player_items_5` (14), `scenarios_player_items_6` (13), `transmog`
(15), `collection` (27) and `heirloom` (9) stay green; format, `git diff --check`,
the physical ratchet and `validation-v2 quick` (manifest
`20260917T011753.474734Z-2137458-quick.json`, 69.7 s) pass. This is the
collection/transmog vertical, not the F1 equipment-stat vertical of #61; the
armor-proficiency half of `CanUseItem`, the broader `CanUseItem` class/race/level
gates and the `ItemSpecStats` fallback remain separate gates. The full
`wow-world --lib` suite stays unusable on this host because several unrelated
pre-existing async tests hang, and `validation-v2 final` stops at the
pre-existing runtime hotspot LOC ratchet, which keeps its drift and was not
regenerated. No live DB/restart/relogin QA.

**#61 attack power aura producers — 2026-09-17, implementation `408725a5`, integrated as `9c7d8ff8` by PR #993:** the
attack power route now runs C++ `Player::UpdateAttackPowerAndDamage`
(`StatSystem.cpp:333-403`): `SPELL_AURA_MOD_ATTACK_POWER` (99) and
`SPELL_AURA_MOD_RANGED_ATTACK_POWER` (124) supply the flat `TOTAL_VALUE`
published as `AttackPowerModPos`/`RangedAttackPowerModPos`,
`SPELL_AURA_MOD_ATTACK_POWER_PCT` (166) and
`SPELL_AURA_MOD_RANGED_ATTACK_POWER_PCT` (167) supply `TOTAL_PCT` published as
the `TOTAL_PCT - 1.0` multiplier, and the ranged producers keep the
`CLASSMASK_WAND_USERS` skip (`SharedDefines.h:190`). `Unit::GetTotalAttackPowerValue`
now applies its zero clamp before the multiplier, replacing the previous
unclamped `total_attack_power` (the damage-range consumer therefore uses the
aura-aware total). The projection returns both multipliers, the canonical
effective stats and `PlayerStatChanges` carry them instead of a hardcoded zero,
and the create block writes `AttackPowerMultiplier`/`RangedAttackPowerMultiplier`
from `PlayerCombatStats`/`PlayerCreateData`. The two new aura constants carry
their `SpellAuraDefines.h` anchors and the physical policy records the reviewed
one-line `world_entry.rs` (2771→2772) and two-line fixture
(`update_tests/mod.rs` 199→201) deltas. Focused coverage: a pure stat-system test
pins the flat/percentage/multiplier arithmetic and the clamp, an end-to-end
session test asserts the melee and ranged producers plus
`GetTotalAttackPowerValue` (405), a second asserts the wand-user skip, and a
packet test pins the create-block multipliers; `wow-data stat_system` (8),
`wow-packet --lib` (742), `scenarios_player_items_12` (12),
`scenarios_player_items_1` (53), `persistence::` (105),
`handlers::character::tests::login` (11), `worldport` (9),
`scenarios_world_entities_24` (11) and `scenarios_spell_state_22` (9) stay green;
format, `git diff --check`, the physical ratchet and `validation-v2 quick`
(manifest `20260917T010221.921007Z-2129573-quick.json`, 76.3 s) pass.
`SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` is implemented by the
override-attack-power entry above (it has no `SPELL_AURA_MOD_ATTACK_POWER_OF_ARMOR`
counterpart in this 3.4.3 source), and the same-effect stack-rule question
remains, the `GetTotalAuraMultiplier` same-effect stack-rule
grouping is not applied (consistent with the older aura helpers), the full
`wow-world --lib` suite stays unusable on this host because several unrelated
pre-existing async tests hang, and `validation-v2 final` stops at the
pre-existing runtime hotspot LOC ratchet, which keeps its drift and was not
regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 school resistances (`Unit::UpdateResistances`) — 2026-09-17, implementation
`67d23891`, integrated as `0300850f` by PR #991:** the six magic school resistances are now represented and published.
`represented_school_resistances_like_cpp` resolves each school as the item
`BASE_VALUE` (`gear.resistances[school]`) scaled by the
`SPELL_AURA_MOD_BASE_RESISTANCE_PCT` `BASE_PCT`, plus the flat
`SPELL_AURA_MOD_RESISTANCE`/`MOD_BASE_RESISTANCE` `TOTAL_VALUE`, then the
`SPELL_AURA_MOD_RESISTANCE_PCT` `TOTAL_PCT`, with the C++ `int32(value)`
truncation (`Unit.cpp:9148-9163`); the mask-parameterized aura helpers are shared
with the `Player::UpdateArmor` physical route from PR #985. The canonical
effective stats now carry the aura-aware schools and the login create path
threads them into `PlayerCombatStats`/`PlayerCreateData.school_resistances[6]`,
so `PlayerCreateData::write_create` writes the seven `UnitData::Resistances`
values that were previously a hardcoded zero. The packet layout is unchanged and
no new mutable state, mirror, lock or clock appears; the physical policy records
the reviewed one-line fixture delta (`update_tests/mod.rs` 198→199). Focused
coverage: a packet test pins the seven create values in order and an end-to-end
session test asserts holy/fire item+aura resistances, the per-school percentage
isolation and the removal path; `wow-packet --lib` (741),
`scenarios_player_items_12` (10), `scenarios_player_items_1` (51),
`persistence::` (105), `handlers::character::tests::login` (11), `worldport` (9)
and `scenarios_world_entities_24` (11) stay green; format, `git diff --check`,
the physical ratchet and `validation-v2 quick` (manifest
`20260917T005300.823629Z-2123777-quick.json`, 69.5 s) pass. The
`BASE_PCT_EXCLUDE_CREATE` modifier has no represented producer and is implicitly
1.0 (C++ default 100.0), post-login resistance *deltas* remain unrepresented
because no resistance delta writer exists yet, the `GetTotalAuraMultiplier`
same-effect stack-rule grouping is not applied (consistent with the older aura
helpers), the full `wow-world --lib` suite stays unusable on this host because
several unrelated pre-existing async tests hang, and `validation-v2 final` stops
at the pre-existing runtime hotspot LOC ratchet, which keeps its drift and was
not regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration, class-talent bonuses and live DB/restart/relogin QA.

**#61 critical-strike aura percentages — 2026-09-17, implementation
`7412dac5`, integrated as `81ee68b9` by PR #989:** the represented critical-strike projection now consumes the C++
aura producers. `Player::UpdateWeaponDependentCritAuras`
(`Player.cpp:8079-8107`) resolves `SPELL_AURA_MOD_WEAPON_CRIT_PERCENT` with the
`CheckAttackFitToAuraRequirement` filter (`Player.cpp:8145-8156`, reusing the
represented weapon admitted by `IsItemFitToSpellRequirements`) plus the
unfiltered `SPELL_AURA_MOD_CRIT_PCT` (290) sum into
`PlayerStatSystemInputLikeCpp.crit_mainhand_aura_pct`/`crit_offhand_aura_pct`/
`crit_ranged_aura_pct`; `Player::UpdateCritPercentage`
(`StatSystem.cpp:502-538`) then applies `5% + FLAT_MOD + rating` per group, so
the offhand no longer shares the mainhand value and the per-attack weapon
requirement can select different auras. `Player::UpdateSpellCritChance`
(`StatSystem.cpp:718-731`) adds `SPELL_AURA_MOD_SPELL_CRIT_CHANCE` (57) plus
`MOD_CRIT_PCT` before the spell rating for all seven schools. The new
`represented_weapon_crit_aura_modifier_like_cpp` reuses the expertise unit's
weapon/item-fit admission, so no second ownership path appears; the four
published critical fields already flow through the create block and the
post-login stat update, so no packet, mirror, lock or clock change is needed.
The `SPELL_AURA_MOD_CRIT_PCT` constant carries its `SpellAuraDefines.h` anchor.
Focused coverage: a pure stat-system test pins the per-group base plus aura sums,
and an end-to-end session test asserts that the sword-only aura is rejected
unarmed, applies to the mainhand after equipping the sword, and never leaks to
the offhand/ranged or spell groups; `wow-data stat_system` (7),
`scenarios_player_items_12` (9), `scenarios_player_items_1` (50),
`persistence::` (105), `handlers::character::tests::login` (11),
`scenarios_spell_state_22` (9) and `scenarios_world_entities_24` (11) stay green;
format, `git diff --check`, the physical ratchet and `validation-v2 quick`
(manifest `20260917T004453.285261Z-2117991-quick.json`, 63.9 s) pass. The
`GetTotalAuraModifier` same-effect stack-rule grouping is not applied (consistent
with the older aura modifier helpers), class-talent critical bonuses and the
school (1-6) resistance publication remain separate gates, the full
`wow-world --lib` suite stays unusable on this host because several unrelated
pre-existing async tests hang, and `validation-v2 final` stops at the
pre-existing runtime hotspot LOC ratchet, which keeps its drift and was not
regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 avoidance aura percentages — 2026-09-17, implementation `255c888f`, integrated as `25d6c0b9` by PR #987:** the
represented avoidance projection now consumes the C++ aura producers of
`Player::UpdateBlockPercentage` (`StatSystem.cpp:483-499`),
`Player::UpdateParryPercentage` (`659-679`) and `Player::UpdateDodgePercentage`
(`700-717`): the flat `GetTotalAuraModifier(SPELL_AURA_MOD_BLOCK_PERCENT)` /
`MOD_PARRY_PERCENT` / `MOD_DODGE_PERCENT` sums feed
`PlayerStatSystemInputLikeCpp.spell_block_pct`/`spell_parry_pct`/
`spell_dodge_pct`, added after the 5% base and on the non-diminishing side before
the class diminishing-returns formula, with the class parry caps honoured (a
zero cap keeps parry at zero). The session resolves them from the canonical
visible applications through the shared `GetTotalAuraModifier` helper;
`block_pct`/`dodge_pct`/`parry_pct` were already published through the create
block and the post-login stat update, so no packet, mirror, lock or clock change
is needed. The two new aura type constants carry their `SpellAuraDefines.h`
anchors. Focused coverage: a pure stat-system test pins the aura terms plus the
zero-cap class branch, and an end-to-end session test applies the three avoidance
auras and the removal path; `wow-data stat_system` (6),
`scenarios_player_items_12` (8), `scenarios_player_items_1` (49),
`persistence::` (105), `handlers::character::tests::login` (11) and
`scenarios_spell_state_22` (9) stay green; format, `git diff --check`, the
physical ratchet and `validation-v2 quick` (manifest
`20260917T003635.249782Z-2114404-quick.json`, 84.2 s) pass. The
`GetTotalAuraModifier` same-effect stack-rule grouping is not applied (consistent
with the older aura modifier helpers), melee/ranged/spell crit aura producers,
the school (1-6) resistance publication and `GetDodgeFromAgility` (empty in this
3.4.3 fork) remain separate gates, the full `wow-world --lib` suite stays
unusable on this host because several unrelated pre-existing async tests hang,
and `validation-v2 final` stops at the pre-existing runtime hotspot LOC ratchet,
which keeps its drift and was not regenerated. #61 remains open for the
player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 armor aura producers (`Player::UpdateArmor`) — 2026-09-17, implementation
`6b7334a0`, integrated as `487cd1d3` by PR #985:** the pure stat system now applies every C++ `Player::UpdateArmor`
producer (`StatSystem.cpp:251-276`): `SPELL_AURA_MOD_BASE_RESISTANCE_PCT` (142)
supplies the `BASE_PCT` factor scaling the item `BASE_VALUE`,
`SPELL_AURA_MOD_RESISTANCE` (22) plus `SPELL_AURA_MOD_BASE_RESISTANCE` (83) with
the normal school mask supply the flat `TOTAL_VALUE`,
`SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT` (182) adds `CalculatePct` of its
`MiscValueB` stat before the multipliers, `SPELL_AURA_MOD_RESISTANCE_PCT` (101)
supplies `TOTAL_PCT` and `SPELL_AURA_MOD_BONUS_ARMOR_PCT` (466) the final
multiplier, with the C++ `int32(value)` truncation. The session resolves those
inputs from the canonical visible applications through the new
`resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp` (which keeps
`MiscValueB` available) and feeds the existing
`PlayerStatSystemInputLikeCpp`; `armor` is already published through the create
block and the post-login stat update, so no packet or mirror change is needed and
no second writer appears. The five aura type constants carry their
`SpellAuraDefines.h` anchors. Focused coverage: a pure stat-system test pins the
producer order (`((100*1.5)+24+40+6)*1.25*1.1` → 302) and an end-to-end session
test applies the normal-mask flat aura, a fire-mask aura, the percentage pair and
the of-stat aura, asserting each step plus the removal path; `wow-data
stat_system` (5), `scenarios_player_items_12` (7), `scenarios_player_items_1`
(48), `persistence::` (105), `scenarios_spell_state_22` (9) and
`scenarios_world_entities_24` (11) stay green; format, `git diff --check`, the
physical ratchet and `validation-v2 quick` (manifest
`20260917T002634.532691Z-2110432-quick.json`, 113.8 s) pass. The same-effect
stack-rule grouping of `GetTotalAuraMultiplier` is not applied to the
`MOD_BONUS_ARMOR_PCT` product (consistent with the older mana/stat aura
multiplier helpers), the school (1-6) resistance `BASE_VALUE`/`TOTAL_VALUE`
publication remains a separate gate because no resistance delta writer exists
yet, and the full `wow-world --lib` suite stays unusable on this host because
several unrelated pre-existing async tests hang; `validation-v2 final` stops at
the pre-existing runtime hotspot LOC ratchet, which keeps its drift and was not
regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 `Unit::m_transformSpell` and `IsPolymorphed` — 2026-09-17, implementation
`e8994e95`, integrated as `72c1fd9b` by PR #983:** the canonical Unit aura subsystem now owns C++
`Unit::m_transformSpell` (`AuraSubsystem::transform_spell_like_cpp`) with the two
writers from `AuraEffect::HandleAuraTransform`: the apply rule overwrites when
there is no current transform spell info, when the new spell is not positive, or
when the current transform spell is positive (`SpellAuraEffects.cpp:1944-1951`),
and the remove rule clears only the aura that owns the current transform
(`SpellAuraEffects.cpp:2129-2131`). The session aura insert/remove funnels are the
only production writers and the character-identity bulk clear drops a stale
transform. `Unit::IsPolymorphed` (`Unit.cpp:9993-10004`) is now resolved from the
MAGE `SpellClassOptions` family (`SPELLFAMILY_MAGE`, family flag `0x1000000`)
plus effect 0 applying `SPELL_AURA_MOD_CONFUSE` (`SpellInfo.cpp:2665-2671`), which
replaces the hardcoded `false` in the C++ `Player::RegenerateHealth` gate
(`Player.cpp:1857-1859`); `Unit::IsInDisallowedMountForm`
(`Unit.cpp:8813-8820`) reads the canonical transform spell instead of scanning
visible auras. `SpellClassOptionsStore`, already loaded at startup, is attached
to the existing `SessionSpellCatalogCapabilitiesLikeCpp` bundle, so no new
mutable state, lock or clock appears; the physical policy records the reviewed
one-line `app.rs` composition delta (5657→5658). Focused coverage: a new
wow-entities precedence test for the transform write rules and an end-to-end
regeneration test where a represented Polymorph (118) bypasses the in-combat gate
with `GetMaxHealth() / 3.0` and removal restores suppression; the mount suite
(14), `scenarios_spell_state_11` (12), `persistence::` (105) and
`wow-entities unit_subsystems` (60) stay green; format, `git diff --check`, the
physical ratchet and `validation-v2 quick` (manifest
`20260917T000637.522552Z-2103079-quick.json`, 495.0 s, of which 485.2 s is the
full workspace rebuild) pass. Handle-less test fixtures do not persist the
derived transform field; acceptance cases install a canonical Player owner, as
production does. The full `wow-world --lib` suite remains unusable on this host
because several unrelated pre-existing async tests hang, and `validation-v2 final`
stops at the pre-existing runtime hotspot LOC ratchet, which keeps its drift and
was not regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 aura-backed per-attack expertise (`Player::UpdateExpertise`) — 2026-09-16,
implementation `a3345bc9`, integrated as `a9a2f2aa` by PR #980:** the character stat projection now derives
`MainhandExpertise`/`OffhandExpertise` like C++
`Player::UpdateExpertise` (`StatSystem.cpp:759-786`): the combat-rating bonus is
truncated to `int32`, each attack adds the `SPELL_AURA_MOD_EXPERTISE` sum whose
spell is fit for that attack's weapon, and the result is clamped at zero.
`represented_expertise_aura_modifier_like_cpp` mirrors
`GetTotalAuraModifier(SPELL_AURA_MOD_EXPERTISE, predicate)` including the
`SPELL_GROUP_STACK_RULE_EXCLUSIVE_SAME_EFFECT` grouping, resolves the weapon
through `Player::GetWeaponForAttack(attack, true)` (`Player.cpp:9243-9270`,
equipped and not broken) and filters each aura through
`SpellInfo::IsItemFitToSpellRequirements` (`SpellInfo.cpp:1757-1768`) against the
represented `SpellEquippedItems` row, so the two weapons can select different
auras. Two adjacent C++ corrections land with the same owner: the rating bonus
is truncated like `int32(GetRatingBonusValue(CR_EXPERTISE))`, and
`ActivePlayerData::RangedExpertise`/`CombatRatingExpertise` keep their zero
create value because C++ never writes them (`UpdateFields.cpp:2889-2893`,
`UpdateFields.h:655-658`). A new
`resolved_aura_effect_amounts_by_spell_like_cpp` exposes the owning spell id
next to each aura amount for the filter predicate. No new `WorldSession` field,
lock or clock. Focused coverage: a new end-to-end test equips a mainhand axe and
applies item-neutral, axe-fit, sword-only and armor-only expertise auras,
asserting the per-attack filtering and the removal path; the existing
equipment-projection test asserts the zero `RangedExpertise`/
`CombatRatingExpertise` create values; `wow-world --lib persistence::` (104),
`scenarios_player_items_1` (47) and `scenarios_player_items_12` (6) stay green;
format, `git diff --check`, the physical ratchet and `validation-v2 quick`
(manifest `20260916T233915.554585Z-2091574-quick.json`, 57.6 s) pass. The full
`wow-world --lib` suite is not usable on this host because several unrelated
pre-existing async tests hang (for example
`scenarios_player_items_8::repair_item_handler_requires_repair_npc_and_repairs_single_item_like_cpp`,
reproduced on unmodified `3.4.3`), and `validation-v2 final` stops at the
pre-existing runtime hotspot LOC ratchet, which keeps its drift and was not
regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and `IsPolymorphed`/`m_transformSpell` (blocked on representing
the spell-family flags that classify `SPELL_SPECIFIC_MAGE_POLYMORPH`), plus live
DB/restart/relogin QA.

**#61 food/drink regeneration emote visual — 2026-09-16, implementation
`73c67a9a`, integrated as `c2608254` by PR #978:** the regeneration tick now completes the tail of C++
`Player::RegenerateAll` (`Player.cpp:1609-1678`). `m_foodEmoteTimerCount`
accumulates `m_regenTimer` beside the two-second health window and stays
independent from it — it is never reset when the aura applies, matching the C++
comment and sniff behaviour. Crossing 5000ms publishes `SPELL_VISUAL_KIT_FOOD`
(406, `SharedDefines.h:397`) when an active `SPELL_AURA_MOD_REGEN` spell carries
`SpellAuraInterruptFlags::Standing`, otherwise `SPELL_VISUAL_KIT_DRINK` (438,
`SharedDefines.h:398`) for a Standing `SPELL_AURA_MOD_POWER_REGEN`, then
subtracts one five-second window; food wins over drink when both apply. The
accumulator lives on the canonical `UnitPowerRegenStateLikeCpp`, the session tick
keeps the single writer, and the kit selection reads the canonical visible
applications plus the difficulty-resolved interrupt word used by the StandState
interrupt path. The packet is `Unit::SendPlaySpellVisualKit`
(`Unit.cpp:11566-11574`) with `KitType = 0`/`Duration = 0`, delivered as one realm
copy for the owner plus the existing position-based visibility fan-out, the same
`SendMessageToSet(packet, true)` split as `SMSG_POWER_UPDATE`. No new
`WorldSession` field, lock or clock. Focused coverage: a new wow-entities
five-second-window timer test and three end-to-end regeneration tests (food
preferred over drink, drink-only, and suppression when no Standing flag is
present); `wow-world --lib persistence::` (104), the pre-existing regeneration
suite (7) and `wow-entities --lib unit::ops_3` (19) stay green; format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260916T221447.629046Z-2067328-quick.json`, 74.9 s) pass. `validation-v2 final`
stops at the pre-existing runtime hotspot LOC ratchet, which keeps its drift and
was not regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, the remaining
`RatesForPower` entries, alternate powers, rune regeneration,
`IsPolymorphed`/`m_transformSpell`, and live DB/restart/relogin QA.

**#61 observer `SMSG_POWER_UPDATE` fan-out — 2026-09-16, implementation
`06ef4fd2`, integrated as `93532597` by PR #976:** the owner power publication now mirrors C++
`Unit::SetPower`'s `SendMessageToSet(packet, true)` (`Unit.cpp:9287-9312`). The
owner session still receives its own `SMSG_POWER_UPDATE`; the same serialized
bytes are queued for the nearby observers through the existing position-based
realm visibility rail via
`broadcast_player_packet_to_visible_set_realm_like_cpp`, which excludes the
source so each session gets exactly one packet. No new `WorldSession` field,
lock, clock or public API. Focused coverage: a new player-publication test
registers a visible observer and asserts the realm-visible command carries the
identical bytes, while the cast-lifecycle (13), regeneration (7) and durability
(14) suites stay green; format, `git diff --check`, the physical ratchet and
`validation-v2 quick` (manifest
`20260916T220112.859778Z-2062121-quick.json`, 72.7 s) pass. #61 remains open for
the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
the remaining `RatesForPower` entries, alternate powers, rune regeneration,
`IsPolymorphed`/`m_transformSpell`, aura-backed expertise and live
DB/restart/relogin QA.

**#61 creature-kill durability loss (`Unit::Kill`) — 2026-09-16, implementation
`7d33d742`, integrated as `1dd3ba5b` by PR #974:** the creature-melee victim handler now runs the C++ `Unit::Kill`
player-victim durability branch (`Unit.cpp:10639-10648`). `over_damage >= 0` is
the represented kill signal (the map already committed the lethal swing), and the
PvE condition `durabilityLoss && !player && !victim->InBattleground()` skips a
battleground victim. `apply_represented_durability_loss_on_death_like_cpp` calls
`Player::DurabilityLossAll(baseLoss, false)` and returns the
`SMSG_DURABILITY_DAMAGE_DEATH` percent C++ derives as
`baseLoss - baseLoss * GetTotalAuraMultiplier(MOD_DURABILITY_LOSS)`; the C++
`uint32` truncation (0 with no aura) is reproduced rather than repaired. The loss
message is published before the melee result presentation, matching `Unit::Kill`
running inside `DealMeleeDamage`. Focused coverage: two new creature-melee
command tests (lethal publishes the loss, battleground skips it) plus the four
existing command regressions; the durability and durability-spell suites stay
green (`wow-world --lib durability`: 14 passed); format, `git diff --check`, the
physical ratchet plus its 20-test suite and `validation-v2 quick` (manifest
`20260916T214822.691938Z-2057566-quick.json`, 77.2 s) pass. #61 remains open for
the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
the remaining `RatesForPower` entries, alternate powers, rune regeneration,
`IsPolymorphed`/`m_transformSpell`, observer `SendMessageToSet` parity and live
DB/restart/relogin QA.

**#61 durability-damage spell effects — 2026-09-16, implementation `5759b474`,
integrated as `93d91726` by PR #972:**
the represented direct spell-effect dispatch now handles
`SPELL_EFFECT_DURABILITY_DAMAGE` (111) and
`SPELL_EFFECT_DURABILITY_DAMAGE_PCT` (115) for the represented player target
(`SpellEffects.cpp:4316-4373`). Effect 111 calls
`Player::DurabilityPointsLossAll(damage, misc < -1)` when `MiscValue < 0` and
`Player::DurabilityPointsLoss` on the `INVENTORY_SLOT_BAG_0` slot otherwise;
effect 115 calls `Player::DurabilityLossAll(damage / 100, misc < -1)` /
`Player::DurabilityLoss`. Both reuse the integrated broken-item mod-removal
ordering and the `SPELL_AURA_PREVENT_DURABILITY_LOSS` gate, and only the
represented player target is processed like C++'s `TYPEID_PLAYER` guard. Focused
coverage: two new end-to-end `execute_spell` tests plus the existing
break/mod-removal and fall-producer regressions (`cargo test -p wow-world --lib
durability`: 12 passed); format, `git diff --check`, the physical ratchet and
`validation-v2 quick` (manifest
`20260916T213346.613171Z-2052499-quick.json`, 150.2 s) pass. #61 remains open for
the general `Unit::Kill` PvE/PvP producer, the remaining `RatesForPower` entries,
alternate powers, rune regeneration, `IsPolymorphed`/`m_transformSpell`, observer
`SendMessageToSet` parity and live DB/restart/relogin QA.

**#61 fall-death item durability loss (`Player::EnvironmentalDamage` →
`DurabilityPointsLoss`) — 2026-09-16, implementation `c8059a85`, integrated as
`68bfce47` by PR #970:** the
represented durability chain now runs on a lethal fall.
`apply_represented_durability_loss_all_like_cpp` walks the equipment slots (and,
when `inventory`, the backpack and bag contents) exactly like
`Player::DurabilityLossAll` (`Player.cpp:4522-4544`), applies the
`SPELL_AURA_MOD_DURABILITY_LOSS` multiplier and the `max(1, ...)` floor per
item, and honours `SPELL_AURA_PREVENT_DURABILITY_LOSS`. It reproduces
`Player::DurabilityPointsLoss` (`Player.cpp:4590-4620`): reaching 0 durability
removes the equipped item's mods *before* the durability write so the
represented `_ApplyItemMods` gate still sees the item as unbroken, and a repair
restores them. The fall-to-death branch of `Player::EnvironmentalDamage`
(`Player.cpp:655-670`) now publishes `SMSG_DURABILITY_DAMAGE_DEATH` through the
new `DurabilityDamageDeath` packet (`MiscPackets.cpp:481-486`).
`DurabilityLoss.OnDeath`/`RATE_DURABILITY_LOSS_ON_DEATH` (`World.cpp:710-721`)
is a registry row loaded by the composition root and installed through
`SessionRuntimePolicyCapabilitiesLikeCpp`; C++'s out-of-range 0.0 behaviour is
reproduced rather than repaired. Focused coverage: the wow-packet layout test,
the wow-config registry count/key test (353 rows, 50 Float), the world-server
`durability_loss_on_death_rate_uses_cpp_world_config_key_like_cpp` config test,
the `durability_points_loss_breaks_and_removes_equipped_item_mods_like_cpp`
break/mod-removal test and the `fall_death_applies_item_durability_loss_message_like_cpp`
producer test; format, `git diff --check`, the physical ratchet plus its 20-test
suite and `validation-v2 quick` (manifest
`20260916T211452.247049Z-2046069-quick.json`, 553.7 s) pass. The physical policy
records the reviewed one-line `app.rs` composition delta. The runtime hotspot and
Session syntax-ownership ratchets keep their pre-existing drift and were not
regenerated. #61 remains open for the general `Unit::Kill` PvE/PvP producer,
`SPELL_EFFECT_DURABILITY_DAMAGE`/`DURABILITY_DAMAGE_PCT`, the remaining
`RatesForPower` entries, alternate powers, rune regeneration,
`IsPolymorphed`/`m_transformSpell`, observer `SendMessageToSet` parity and live
DB/restart/relogin QA.

**#61 C++ regeneration rates — 2026-09-16, implementation `bf3794f0`,
integrated as `fbed40ea` by PR #968:** the
regeneration tick no longer hardcodes `rate: 1.0`/`rate_health: 1.0`; it
consumes the C++ `World::setRegenRate` values (`World.cpp:615-623`).
`PlayerRegenerationRatesLikeCpp` (`session_policy.rs`) carries `RATE_HEALTH`,
`RATE_POWER_MANA`, `RATE_POWER_RAGE_LOSS`, `RATE_POWER_FOCUS`,
`RATE_POWER_ENERGY` and `RATE_POWER_RUNIC_POWER_LOSS`, maps them through the
C++ `RatesForPower` table (`Player.cpp:1681-1747`) and defaults every field to
1.0 like `sWorld->getRate`. The six keys are now rows of
`cpp-world-config-registry.tsv`; the composition root resolves them from the
already-loaded `WorldConfigSet` and attaches the immutable value to
`SessionHandlerCatalogsLikeCpp`, so no new `WorldSession` field or second
authority appears. `Rate.Health` scales the `RegenerateHealth` spirit component
and the per-power rate scales `Regenerate` before the aura producers, exactly as
C++. The physical policy records the reviewed 3-line composition delta for
`app.rs`. Focused coverage: the wow-config registry count/key test (352 rows, 49
Float), the world-server `player_regeneration_rates_use_cpp_world_config_keys`
config test and the `regeneration_rates_scale_mana_and_health_like_cpp` tick
test; format, `git diff --check`, the physical ratchet plus its 20-test unit
suite and `validation-v2 quick` (manifest
`20260916T204201.787414Z-2033943-quick.json`, 523.7 s) pass. The runtime hotspot
and Session syntax-ownership ratchets keep their pre-existing drift and were not
regenerated. #61 remains open for the remaining `RatesForPower` entries of
unrepresented powers (soul shards, lunar power, combo points, ...), alternate
powers beyond the represented primary, `UpdateAllRunesRegen`/rune cooldowns,
`IsPolymorphed`/`m_transformSpell`, observer `SendMessageToSet` parity and live
DB/restart/relogin QA.

**#61 non-mana power-regeneration loop — 2026-09-16, implementation `2387c04b`,
integrated as `4206ab00` by PR #966:**
the session tick now walks the complete C++ `RegenerateAll` power loop
(`Player.cpp:1614`): it iterates `POWER_MANA..MAX_POWERS`, skips powers without a
represented index, and applies one `Player::Regenerate` per power, so a
rage/energy/focus/runic-power primary is regenerated or decayed from its DB2
`PowerTypeEntry` instead of being ignored. `Unit::regenerate_power_like_cpp` now
applies the non-mana aura producers from `Player.cpp:1745-1747`:
`GetTotalAuraMultiplierByMiscValue(SPELL_AURA_MOD_POWER_REGEN_PERCENT, power)`
and the flat `SPELL_AURA_MOD_POWER_REGEN` modifier scaled by `m_regenTimerCount`
for every power except energy, which uses `m_regenTimer`; mana keeps reading the
published `UpdateManaRegen` fields and skips those producers exactly like C++. The
tick resolves the per-power DB2 entry, the `SPELL_AURA_PREVENT_REGENERATE_POWER`
value check and both misc-value aura families once, then publishes one
`SMSG_POWER_UPDATE` per changed power on the two-second boundary. Focused
coverage: 4 new `wow-entities` tests (percent/flat aura, energy timer selection,
accumulated window for other powers, mana exclusion) and 2 production-shaped
`wow-world` tick tests (rage decay, energy throttle); format, `git diff --check`,
the physical-source ratchet and `cargo check -p world-server` pass. The runtime
hotspot and Session syntax-ownership ratchets keep their pre-existing drift and
were not regenerated. #61 remains open for `RatesForPower`/`sWorld->getRate`
config overrides, alternate powers beyond the represented primary,
`UpdateAllRunesRegen`/rune cooldowns, `IsPolymorphed`/`m_transformSpell`, the
observer `SendMessageToSet` `SetPower` parity, productive wear-to-broken and live
DB/restart/relogin QA.

**#61 health-regeneration tick (`Player::RegenerateAll` → `RegenerateHealth`) —
2026-09-16, implementation `fbd8755e`, integrated as `381038dd` by PR #964:** the
session-owned tick now runs the
complete C++ `Player::Update → RegenerateAll` step. It accumulates
`m_regenTimer`/`m_regenTimerCount` for every living in-world player regardless
of mana prevention or power representation, regenerates the represented primary
power, and runs the two-second `RegenerateHealth` branch on the canonical
`Unit`. `Unit::regenerate_health_like_cpp` reproduces the C++ formula
(`Player.cpp:1842-1882`): `OCTRegenHPPerSpirit` with the 50-Spirit split and the
`OCTRegenHP.txt`/`RegenHPPerSpt.txt` level/class ratios
(`Player.cpp:5162-5180`), the `RATE_HEALTH` under-15 level multiplier, the
out-of-combat `SPELL_AURA_MOD_HEALTH_REGEN_PERCENT` multiplier and
`SPELL_AURA_MOD_REGEN * 0.4` flat term, the in-combat
`SPELL_AURA_MOD_REGEN_DURING_COMBAT` `ApplyPct`, the non-stand `1.5x` factor,
`SPELL_AURA_MOD_HEALTH_REGEN_IN_COMBAT`, `m_baseHealthRegen / 2.5`, the `int32`
truncation and the `Unit::ModifyHealth` clamp (`Unit.cpp:8115-8155`). Positive
regen marks `UnitData::Health`, which the canonical map's `SendObjectUpdates`
publishes; C++ sends no explicit packet for a positive delta. `wow-data` now
owns the moved `RegenMPPerSpt` family plus the new `RegenHPPerSpt` and
`OCTRegenHP` tables in `game_tables/regen.rs` behind `RegenGameTablesLikeCpp`
(`GameTables.h:124-165,302-330`, `GameTables.cpp:128,130`); the composition root
loads the bundle once and installs it on the Session, replacing the single MP
table field. Focused coverage: 7 `wow-entities` formula tests, 9 `wow-data`
regen-table tests and 2 production-shaped `wow-world` tick tests (window gating
+ in-combat suppression); format, `git diff --check` and `cargo check -p
world-server` pass. The architecture syntax ownership baseline remains the
pre-existing stale set from #958/#962 and was not regenerated; this slice
renames the already-unreviewed mana-regen Session field/setter and adds the
health tick method, so `session-ownership-check --syntax-only` stays red on the
same drift. #61 remains open for `IsPolymorphed`/`m_transformSpell`, alternate
powers, `Rate.Health`/`Rate.Mana` config overrides, observer
`SendMessageToSet` packet-type parity, productive wear-to-broken, aura-backed
expertise and live DB/restart/relogin QA.

**#61 mana-regeneration tick and `SMSG_POWER_UPDATE` publication — 2026-09-16,
implementation `5bc59ddb`, integrated as `d74381ec` by PR #962:** the canonical
Player snapshot now drives the C++
`Player::Update → RegenerateAll → Regenerate(POWER_MANA)` chain instead of only
publishing the regen fields. A session-owned tick (the same `Player::Update`
boundary already used for `DoMeleeAttackIfReady`) runs with the canonical
world/map diff, accumulates `m_regenTimer`/`m_regenTimerCount`, and consumes the
published `PlayerEffectiveCombatStatsLikeCpp::mana_regen`/`mana_regen_combat`,
the DB2 `PowerTypeEntry` (`RegenPeace`, `RegenCombat`, `MinPower`, `CenterPower`,
`Flags`, `RegenInterruptTimeMS`) and the five-second MP5 rule
(`_regenMP5InterruptStartTime`). The operation reproduces the `m_powerFraction`
carry, the min/max clamp, the `m_regenTimerCount >= 2000 || forcesSetPower`
publication boundary, the throttled `ClearChanged` write (the suppressed setter
does not queue a packet) and the new `PowerUpdate`
(`SMSG_POWER_UPDATE`, `CombatPackets.cpp:104-116`) on the boundary.
`Spell::TakePower` (`Spell.cpp:5444-5445`) now arms the five-second rule, and
`SPELL_AURA_PREVENT_REGENERATE_POWER` (`SpellAuraDefines.h:389`) is resolved from
the canonical visible auras. The regen runtime state lives on the canonical
`Unit` (C++ keeps the timers on `Player` and the MP5 mark on `Unit`) so the
reviewed `player/mod.rs` physical ceiling is preserved. Focused coverage: 6
`wow-entities` regeneration tests, the `wow-packet` `PowerUpdate` layout test and
3 production-shaped `wow-world` tests (throttle/publish, interrupted rate,
mana-cost MP5 producer); format, `git diff --check` and the affected
entity/packet/world checks pass. The architecture physical-source ratchet passes;
the runtime hotspot ratchet keeps its pre-existing `session/mod.rs`,
`character/mod.rs`, `world-server/lib.rs` and `player/mod.rs` growth and was not
regenerated. #61 remains open for health regeneration, non-mana powers, the
`Rate.Mana` config override, observer `SendMessageToSet` packet-type parity,
productive wear-to-broken and live DB/restart/relogin QA.

**#61 canonical equipment contribution closure — 2026-09-15, implementation
`a43f51cf`:** the effective-stat projection no longer sums equipped inventory rows
alongside the canonical `PlayerItemBonusStateLikeCpp` accumulator. Login now seeds the
accumulator through the same non-broken-item gate as `_ApplyAllItemMods`, and all represented
equipment transitions publish the complete Player-owned effective snapshot consumed by
combat. The focused regression covers equip, unequip, break, repair and login equivalence;
affected inventory suites pass 7/7, 5/5, 10/10 and 16/16, and `cargo check -p world-server`
passes. The architecture check remains blocked by the pre-existing `session/mod.rs`
ratchet drift and was not masked by baseline regeneration. #61 remains open for the
unimplemented wear-to-broken producer, aura-backed and complete combat formulas, and live
capture/DB/relogin proof.

**#61 canonical equipment expertise publication — 2026-09-15, implementation
`48378050`:** the complete Player snapshot now derives the equipment/rating portion
of mainhand, offhand and aggregate expertise from the canonical expertise rating and
the level-specific `CombatRatings` multiplier. The VALUES adapter consumes that
snapshot, retaining its raw calculation only for handle-less test/early-login
fixtures. This follows `Player::GetRatingMultiplier` / `GetRatingBonusValue`
(`Player.cpp:5189-5209`) and `Player::UpdateExpertise`
(`StatSystem.cpp:759-783`); aura expertise remains a separate producer gate.
The focused canonical equipment regression and item suites pass (1, 7 and 5 tests),
format/diff checks pass and `cargo check -p world-server` passes in 7m01s. The
architecture ratchet still has the previously recorded `session/mod.rs` drift;
no baseline was regenerated. #61 remains open for aura-backed expertise, full
combat/regen/penetration formulas, wear-to-broken production and capture/DB/relogin
acceptance.

**#61 C++ mana-regeneration table projection — 2026-09-15, implementation
`9004f5cf`:** startup now loads `gt/RegenMPPerSpt.txt` once through the world-server
composition root and installs the immutable table in each `WorldSession`. The canonical
Player effective-stat snapshot computes spirit regeneration as
`sqrt(Intellect) * Spirit * ratio[level,class]`, matching `Player::OCTRegenMPPerSpirit`
(`Player.cpp:5182-5190`) and the spirit branch of `Player::UpdateManaRegen`
(`StatSystem.cpp:799-827`); the packet adapter consumes that snapshot instead of keeping
the former hard-coded class coefficients and extra constant. The table parser, class-column
mapping and level-80 fixture are covered by 3 `wow-data` tests; the focused character
stat/persistence set passes 6/6, `cargo check -p world-server` passes in 3m02s, and format/
diff checks pass. Aura percentage modifiers, stat-derived MP5, full regen tick publication,
wear-to-broken production and live capture/DB/relogin acceptance remain explicit #61 gates.
The architecture physical-source check passes; its runtime hotspot ratchet still reports
the pre-existing drift in `session/mod.rs`, character handlers, `world-server/lib.rs` and
`wow-entities/player/mod.rs`, so no baseline was regenerated.

**#61 aura-backed mana-regeneration percentage projection — 2026-09-15, implementation
`48218460`, integrated by PR #959 (`4f7ce25b`):** the canonical Player stat publisher now applies the
two C++ percentage producers from `Player::UpdateManaRegen` (`StatSystem.cpp:809-812`):
`SPELL_AURA_MOD_POWER_REGEN_PERCENT` and `SPELL_AURA_MOD_MANA_REGEN_PCT`, both filtered
to `POWER_MANA`. The producer resolves canonical visible aura applications against
immutable `SpellInfo` effect metadata and multiplies each active effect as
`1 + amount / 100`, preserving loaded applications whose represented-effect shortcut
is absent. The focused aura regression and the six existing stat-update regressions
pass; the affected `world-server`, `wow-data` and `wow-world` test-aware check also
passes. Flat `MOD_POWER_REGEN`, stat-derived MP5, complete tick/publication semantics,
wear-to-broken production and live capture/DB/relogin evidence remain explicit #61
gates. Same-effect C++ stack-policy details stay outside this bounded producer until
the aura application contract carries them explicitly. The architecture physical-source
check passes; its runtime hotspot ratchet still reports the pre-existing growth in the
Session, character-handler, world-server and Player roots, so no baseline was regenerated.

**#61 flat and interrupted mana-regeneration aura projection — 2026-09-15, implementation
`bfe49b2b`, integrated by PR #960 (`96b8ffe3`):** the same Player-owned producer now completes the
remaining local `UpdateManaRegen` arithmetic. It adds `MOD_POWER_REGEN` to the flat
MP5-equivalent rate, adds each `MOD_MANA_REGEN_FROM_STAT` effect as
`stat * amount / 500`, and applies the capped `MOD_MANA_REGEN_INTERRUPT` percentage
to the spirit component for the interrupted-combat field (`StatSystem.cpp:815-826`).
The canonical aura resolver sums modifiers by MiscValue and retains the same immutable
SpellInfo/visible-application authority used by the percentage path. The combined aura
regression and six existing stat-update regressions pass; the affected test-aware
world-server/data/world check, format and diff checks pass. The old `ModPowerRegen`
packet field remains zero because this C++ path writes `PowerRegenFlatModifier` and
`PowerRegenInterruptedFlatModifier`; full tick/publication, wear-to-broken production
and live capture/DB/relogin evidence remain #61 gates. The physical-source architecture
check passes; the pre-existing runtime hotspot ratchet remains unaltered.

**Fresh Creature runtime audit — 2026-09-15:** the current boundary and next
macro are recorded in
[`creature-runtime-audit.md`](../architecture/creature-runtime-audit.md).
`WorldCreature` remains the mutable runtime/AI owner while
`wow-map::Map::entity_world` supplies visibility and target reads. Production
uses `GlobalLegacy` and the canonical `ExternalRuntime` no-op to avoid a second
timer writer; cloned sync fences protect stale health/loot/incarnation writes,
but the canonical `runtime_update_plan` still has no production effect
consumer. The structural **C3.1 — one map-owned Creature runtime outcome
boundary** is implemented by `e2ca3df9`: typed tick input/outcome, admitted
ObjectUpdater Creature stamps, authority/incarnation validation and
production-linked phase-order, death/respawn and stale-incarnation tests. The
full C++ `Map::Update`/`Creature::Update` operation is therefore not converged;
the next behavior vertical must consume this envelope. No mass owner migration
or timer-only plan consumer is selected.

**#63 movement audit — 2026-09-14:** the current code and TrinityCore anchors show
that ordinary force-speed ACKs are already implemented in
`session/movement/speed.rs` with the C++ forced-change counter, transport exemption,
correction and kick branches (`MovementHandler.cpp:468-546`), and knockback ACK
admission/publication is integrated by PR #866 (`MovementHandler.cpp:548-559`). They
are no longer pending implementation items. The open boundary is complete
vehicle/transport seat-offset admission, runtime branches whose mover or consumer is
not represented, exact packet-order captures and live client/server/DB QA.

**Player `m_seer` visibility projection integrated — 2026-09-14, PR #923, merge
`5f6b1ad83d6ea6adf81368a36384593808632737`:** TrinityCore initializes
`Player::m_seer` to self (`Player.cpp:298-300`, `Player.h:2417-2425`) and mutates
it through `SetViewpoint` (`Player.cpp:25338-25395`), while map and visibility
read that pointer (`Map.cpp:716-718`, `GridNotifiers.cpp:95-222`). Rust production
now derives the seer GUID from the canonical map-owned Player's
`ActivePlayerData::FarsightObject`; empty means the Player itself. Deferred
visibility, movement, aggro and GameObject/DynamicObject consumers use the
projection, and the former Session field is a `cfg(test)` fixture only.
`last_observed_farsight_object_like_cpp` is a receiver-local publication fence for
the explicit FAR_SIGHT clear VALUES packet, not gameplay authority. Focused
FAR_SIGHT (14), GameObject despawn (22), DynamicObject VALUES (15), package,
ownership syntax, architecture and 20 self-tests pass. Full captures,
DB/relogin durability and live QA remain open gameplay/runtime gates under
#41/#63/#584; #584 still owns other C0-C4 and runtime work.

**Canonical Pet visibility CREATE integrated — 2026-09-14, #584 / PR #925, merge
`a1f66c33903ee76b6bb0675177d33e3b283806ed`:** the canonical map already places
Pets in the Creature cell family, matching TrinityCore's generic
`Map::AddToMap`/`UpdateObjectVisibilityOnCreate` path (`Map.cpp:530-610`) and
`Pet::AddToWorld` (`Pet.cpp:69-88`). The Session visibility query now reads
Creature/Pet records through `with_creature_or_pet_like_cpp`, so an in-world Pet
passes the existing map, phase, range and detection gates and reaches the common
Creature CREATE snapshot. The focused regression
`visible_creatures_skip_not_in_world_canonical_objects_like_cpp`
passes, as do the affected package check and formatting/diff checks. This closes
the projection gap only; Pet AI/movement, summon ownership/persistence,
corpse/transport lifecycle, exact captures and live DB/restart/relogin QA remain
explicit #584/#63 gates. Directed Pet DESTROY is covered by the integrated PR #927
slice recorded below.

**Directed object DESTROY integrated — 2026-09-14, #584 / PR #929, merge `028185d8`:**
TrinityCore removes ordinary units through `Map::RemoveFromMap` while the source is
still attached, so `WorldObject::DestroyForNearbyPlayers` can walk nearby Players
(`Map.cpp:934-951`, `Object.cpp:3617-3648`); `Pet::RemoveFromWorld` follows that
Unit path (`Pet.cpp:94-101`). For Corpses, `Corpse::RemoveFromWorld` delegates to
the same world-object removal path (`Corpse.cpp:56`, `Object.cpp:1023-1029`). Rust
now captures one generic object recipient list before canonical erasure and sends
one directed DESTROY command for Creature, Pet or Corpse, retaining map-incarnation
and `HaveAtClient` fences. The `wow-map` visibility suite (47 tests), `wow-world`
deferred-visibility suite (12 tests), mailbox suite (16 tests), package check and
architecture checks pass. Pet AI, summon ownership, corpse reclaim/persistence/loot,
vehicle/transport lifecycle and live QA remain separate gates.

**P4 loaded-grid test split integrated — 2026-09-14, #584 / PR #931, merge `b2545d50`:**
`crates/world-server/src/creature_loaded_grid.rs` now keeps 797 production lines and
mounts `creature_loaded_grid_tests/mod.rs`; its builder/resolver children retain all
28 regressions under explicit `cfg(test)`. This is a physical navigability closure:
no runtime owner, dependency, packet, persistence or behavior changed. Focused
world-server tests pass 28/28; format/diff and architecture check/self-test pass.
The global physical terminal remains open for other #584 entries.

**P4 game-event runtime split integrated — 2026-09-15, #584 / PR #933, merge `ef30bb3e`:**
`crates/world-server/src/runtime/game_events.rs` is now a compact facade over seven
responsibility modules (unspawn, grid, spawn, bootstrap, scheduler, live update and
consume). The complete 181-test game-event suite remains green; `cargo check`,
format/diff and architecture check/self-test pass. No runtime owner, dependency,
packet, persistence or behavior changed. The global physical terminal and remaining
C0–C4/runtime/capture/DB/relogin/live-QA gates remain open.

**P4 spawn catalog navigability integrated — 2026-09-15, #584 / PR #935, merge
`c922fd60d71018a0502954a6ddffe151401e48bf`:** the post-#933 audit split the
5,125-line `world-server/src/spawn_store_loader.rs` by responsibility. Startup
composition, DB ownership and spawn conversion remain in the 3,427-line facade;
GameEventMgr catalog models and WorldStateMgr startup/index rules now live in
`spawn_store_loader/game_event_catalog.rs` (1,355 lines) and
`world_state_catalog.rs` (345 lines), with the existing public paths reexported.
The published #933 runtime child modules are crate-visible to their parent reexports,
restoring the integration compile contract. The focused game-event suite passes
181/181; compile, format/diff and architecture check/self-test pass. The full local
profile still has three pre-existing `scenarios_9` GameObject respawn-save failures
(587 passed, 3 failed). Follow-up PR #936 tightens the two physical policy rows to the
exact scanner counts at the current head. This is physical organization only; remaining #584 C0–C4,
runtime, capture, DB/relogin and live-QA gates stay open.

**P4 game-event runtime/cache split integrated — 2026-09-15, #584 / PR #938, merge
`c5967026730b8c95802839f3d48ef083cae4dfac`:** the post-#935 audit extracted the
complete game-event state-transition and cache family from
`spawn_store_loader.rs` into private `spawn_store_loader/game_event_runtime.rs`.
The implementation remains an inherent impl on `CanonicalSpawnMetadataLikeCpp`,
so its canonical ownership and public method paths are unchanged. The current
facade is 2,627 lines and the child is 800; the aggregate hotspot remains at the
validated 3,427-line ceiling. The 181 game-event tests pass; compile,
format/diff, architecture check and self-test pass. The full local profile still
reports the three pre-existing `scenarios_9` GameObject respawn-save failures
(587 passed, 3 failed). No packet, SQL, clock, lock, persistence order or runtime
behavior changed; the remaining loader families and #584 C0–C4/runtime/capture/
DB-relogin/live-QA gates stay open.

**P4 GameEventMgr startup-loader split integrated — 2026-09-15, #584 / PR #940,
merge `2959e24565ea786fa507105eb28a519f52b1fa7e`:** the post-#938 audit moved
the complete GameEventMgr startup loading/validation family into private
`spawn_store_loader/game_event_loader.rs`. Conditions, prerequisites, pools,
spawn GUIDs, quest relations, NPC flags/vendors and model/equipment validation
remain the same operations and order; only their physical module changed. The
composition facade is 1,990 lines and the loader child 637; with the 800-line
runtime child, the measured spawn-loader aggregate remains at the exact 3,427-line
ceiling. All 181 game-event tests pass, as do compile, format/diff, architecture
check/self-test and final structural gates. The full local profile still has the
three pre-existing `scenarios_9` GameObject respawn-save failures (587 passed, 3
failed). Parent-private fixture helper paths remain available through `pub(super)`;
no public API, owner, startup behavior, packet, SQL, clock, lock or persistence
order changed. Remaining loader families and #584 C0–C4/runtime/capture/DB-relogin/
live-QA gates remain open.

**P4 pool and spawn-group startup-loader split integrated — 2026-09-15, #584 / PR #942,
merge `77ca1c64fdca504172bf47c07b43a3a6a9d2aaf8`:** the post-#940 audit moved
PoolMgr templates and member loading, relation/map/final validation, autospawn
candidates and spawn-group template construction to private
`spawn_store_loader/pool_loader.rs`. The composition facade is 1,652 lines and
the child 338; the measured aggregate remains exactly the 3,427-line ceiling.
The public spawn-group helper path and parent-private fixture access remain
unchanged. Focused pool tests pass 29/29, with architecture, compile,
format/diff and structural gates passing; the full profile retains the same
three pre-existing `scenarios_9` GameObject respawn-save failures (587 passed,
3 failed). No startup order, owner, packet, SQL, clock, lock, persistence or
runtime behavior changed; remaining loader families and #584 C0–C4/runtime/
capture/DB-relogin/live-QA gates remain open.

**P4 object-spawn startup-loader split integrated — 2026-09-15, #584 / PR #944,
merge `e0a8fae08aa0f2d220330495eb8303125969bd18`:** the post-#942 audit moved
Creature, GameObject and AreaTrigger spawn-row loading, conversion/validation,
runtime-row capture and linked-respawn admission to private
`spawn_store_loader/spawn_object_loader.rs`. The composition facade is 999
lines and the child 653; with the existing pool and GameEvent children, the
measured aggregate remains exactly the 3,427-line ceiling. Existing startup
calls and parent-private fixture paths remain unchanged. Focused spawn-loader
tests pass 135/135, with architecture, compile, format/diff and structural
gates passing; the full profile retains the same three pre-existing
`scenarios_9` GameObject respawn-save failures (587 passed, 3 failed). No
startup order, owner, packet, SQL, clock, lock, persistence or runtime behavior
changed; the remaining addon loader family and other #584 gates remain open.

**P4 creature movement metadata split integrated — 2026-09-15, #584 / PR #946,
merge `9698a656b2a3dad320a6f4a3d737d7698a326f79`:** the post-#944 audit moved
waypoint path/report construction, coordinate and delay normalization, default
waypoint lookup, creature formation validation and their persistence loaders to
private `spawn_store_loader/creature_movement_loader.rs`. The composition facade
is 784 lines and the child 215; with the existing pool, object and GameEvent
children, the measured aggregate remains exactly the 3,427-line ceiling. Public
waypoint/formation types and helper paths remain available through explicit
re-exports and startup order is unchanged. Focused spawn-loader tests pass
135/135, with architecture, compile, format/diff and structural gates passing;
the full profile retains the same three pre-existing `scenarios_9` GameObject
respawn-save failures (587 passed, 3 failed). No gameplay, packet, SQL, clock,
lock, persistence or runtime behavior changed; the remaining addon loader family
and residual composition readers require a fresh audit.

**P4 spawn-loader residual adapter closure integrated — 2026-09-15, #584 / PR
#948, merge `861fbc701a6e556f4988ad149e90faa23b3c53f1`:** the post-#946 audit
moved the linked-respawn row conversion into `spawn_object_loader.rs`, the
GameEvent world prefix/suffix adapters into `game_event_loader.rs`, and the
spawn-group member persistence adapter into `pool_loader.rs`. The composition
facade is 730 lines; the object, pool, GameEvent loader, movement and runtime
children are 661, 358, 663, 215 and 800 lines, with the measured aggregate still
exactly 3,427 lines. Existing scoped imports preserve startup order, public paths
and parent-private fixtures. Focused spawn-loader tests pass 135/135; architecture
check/self-test, compile, format/diff and final physical/hotspot gates pass. The
full profile retains the same three pre-existing `scenarios_9` GameObject
respawn-save failures (587 passed, 3 failed). No gameplay, packet, SQL, clock,
lock, persistence or runtime behavior changed. PR #950 reconciles the stale
Session ownership anchor and exact syntax, registry and bridge baselines after
#929/#933; the checker-only suite passes 364/364, syntax-only ownership passes,
and architecture check/self-test remain green. The next step is a fresh audit
of the remaining #584 C0–C4/runtime/capture/DB/relogin/live-QA work.

**Transport C0/C3 lifecycle integrated — 2026-09-14, PR #901, merge
`bf460aa7a8ccec0269eea1771a094ef12f0c6109` (implementation `82b2d8d9`):** the bounded
CREATE/DESTROY and phase-visibility slice is integrated into `3.4.3`. `wow-map` marks same-phase Players
through the Transport map-reference walk (`Map.cpp:574-610,1853-1915`) on add/remove;
`wow-world` snapshots typed in-world transports into owned `GameObjectCreateData`,
creates `CreateTransport`/OUT-OF-RANGE blocks during the existing deferred visibility
refresh, and publishes `m_visibleTransports` atomically with the packet. The map guard
is released before Session delivery, and the separate Transport VALUES membership
rail remains intact. Focused map visibility and entity-bridge regressions pass, as do
the `wow-world` package check and formatting/diff checks on the merged implementation. The slice
does not claim `Transport::TeleportPassengersAndHideTransport` map relocation,
passenger seat/offset admission, AI/scripts, taxi routing, exact C++ captures, or live
DB/restart/relogin QA; those remain explicit #63/#584 gates. No legacy Creature writer
migration is implied.

**P2 Player identity owner — integrated PR #904, merge `4ad36d420a0f56297390262f3667be1b5fc4ae6f`,
2026-09-14:** the #584 macro moves name, race, class, level and gender
authority to the canonical `wow_entities::Player`/`Unit`/`WorldObject`. The change
follows TrinityCore `Player::LoadFromDB` (`Player.cpp:17060-17089`, `17247-17283`)
and `Unit::GetLevel/GetRace/GetClass/GetGender` (`Unit.h:733-745`). Session retains
only a login bootstrap DTO until owner installation; the old identity fields are
fixture-only under `cfg(test)`, stale handles fail closed, and module/registry/
character/group/chat consumers resolve through the canonical owner. `Player::m_swingErrorMsg`
remains a separate melee residual. The candidate passes the two crate checks, 919
entity tests, login 17/17, persistence 25/25, identity 23/23, formatting/diff and
architecture check plus 20 self-tests. Save/reload durability, captures and live
DB/relogin QA remain open gates; the next #584 implementation must follow a fresh
C0–C4 responsibility and consumer audit rather than reselecting this slice.

**P2 Player-owned mount presentation transition — 2026-09-14, #584 / PR #897, integration `90e58c7358d03340fb9ce10461a14dd33b94ceed` (implementation `bc58334f`):** the remaining production mount-presentation write now uses a named `Player` transition that applies `MountDisplayID` and `UNIT_FLAG_MOUNT` together, following TrinityCore `Unit::Mount` / `Unit::Dismount` (`Entities/Unit/Unit.cpp:7822-7865`). Session retains aura, collision, vehicle-kit and packet side effects; its broad unit-presentation closure is test-only for detached scale fixtures. The Player owner regression, mount spell-state scenarios, package, formatting/diff and architecture checks pass. Full mount gameplay, persistence, captures and live QA remain separate gates under #63/#584.

**P2 Player-owned RestMgr transitions — 2026-09-14, #584 / PR #895, integration `94c21521a6d215f8f9fb086ddbef3267ee08c10f` (implementation `ea18126e`):** the remaining production Player rest-flag, deferred-publication and rest-clock writes now use named transitions on `wow-entities::Player` over its Player-owned `PlayerRestState`. The boundary follows `RestMgr::SetRestFlag` / `RemoveRestFlag` (`RestMgr.cpp:95-122`), `RestMgr::_restTime` (`RestMgr.h:86`) and `Player::SetRestState` (`Player.h:2652`). Session retains zone/catalog resolution, packet publication and application ordering; its generic PlayerRestState mutator is now a detached-fixture seam under `cfg(test)` only. The Player owner regression, rest-owner scenarios, chat, area-trigger, zone, package, formatting/diff and architecture checks pass. Quest objective progress, durable persistence, captures and live QA remain separate gameplay gates under #41/#584.

**P2 Player-owned mount VehicleKit operations — 2026-09-14, #584 / PR #878,
integration `d8cb0594093c25ae618ab2603896fd3e2f7de26d` (implementation
`ceb58c9a`):** the remaining production Session mutation surface for a Player's
mount vehicle kit now routes through named `Player` operations in
`crates/wow-entities/src/player/vehicle.rs`. This follows TrinityCore's
`Unit::CreateVehicleKit`, `Unit::RemoveVehicleKit` and `Unit::GetVehicleKit`
(`src/server/game/Entities/Unit/Unit.cpp:11304-11323`): install, snapshot,
clear, uninstall/removal and ejectable-passenger removal are owner transitions.
Session retains vehicle-template admission, aura/packet/presentation effects and
the detached fixture fallback under `cfg(test)`; no second production authority,
lock or clock is added. The owner lifecycle test, three represented-eject tests,
both package checks, formatting/diff checks and the architecture ratchet pass.
This is a structural owner closure only: complete vehicle seat/offset admission,
passenger lifecycle, CREATE/DESTROY, Pet/corpse/Transport publication, captures,
DB/restart/relogin and live QA remain separate #584/#63 gates.

**P2 Player-owned aura mutation operations — 2026-09-14, #584 / PR #881,
integration `2a916c1c429456085e9274a60fcf57138ce12b43` (implementation
`3e1bdd9d`):** the remaining production Session mutation surface for Player aura
state now routes through named operations on `Player` over its Unit-owned
`AuraSubsystem`: visible-aura insert/remove, persisted aura-authority completion,
spell-hit authority tombstone/reset, and threat-aura install/apply/remove. This
matches TrinityCore's Unit-owned aura maps and transitions (`Unit.h:620-640,
1226-1260,1825-1844`; `Unit.cpp:680-690`). Session retains packet/catalog
adaptation and the handle-less generic mutator is `cfg(test)` only. The focused
canonical/detached/replacement aura-authority regression, both package checks,
formatting/diff checks and architecture ratchet pass. Full aura gameplay, exact
packet captures, durable DB/restart/relogin and live QA remain separate #584 gates.

**P2 Player-owned trade transitions — 2026-09-14, #584 / PR #883, integration
`0f79ca837a7417876986cf1803f715a8512b7503` (implementation `50e98f43`):** the represented `TradeData` state now has a private
`wow-entities::Player` owner module with named open/clear, state-index,
acceptance, gold, item-slot and trade-spell transitions. This follows
`Player::m_trade` (`Player.h:2998`), creation in `TradeHandler.cpp:694-695`,
cleanup in `Player.cpp:12864-12879` and `TradeData.cpp:58-150`. Session retains
money/inventory/catalog admission, packet encoding and partner mailbox delivery;
its broad trade mutator is available only to handle-less `cfg(test)` fixtures.
The focused Player owner regressions and the 15-test social scenario suite pass,
as do the world check, architecture ratchet and formatting/diff checks. This is
an ownership closure: full trade settlement durability, captures and live QA stay
with the gameplay acceptance boundary.

**P2 Player-owned guild membership transitions — 2026-09-14, #584 / PR #885,
integration `19dea8e078f5b7bec827532cefb8f46911e332e7` (implementation
`f0d32675`):** the represented guild membership and pending invitation state now
have named operations on `wow-entities::Player`,
following `Player::SetInGuild` (`Player.cpp:7216`), `SetGuildIdInvited` and
`SetGuildRank` (`Player.h:1939,1943`). Session remains the GuildMgr/cache, protocol
and application adapter; its whole-state guild mutator is retained only for
handle-less `cfg(test)` fixtures. The composite guild install is retired. Owner,
social scenario, guild handler, package, architecture and formatting/diff checks
pass. This is an ownership closure, not guild-manager/database, packet-capture or
live-QA acceptance.

**P2 Player-owned Battleground transitions — 2026-09-14, #584 / PR #887,
integration `72f6a3fa87d00f9319c1cfa626f7a10345fc9654` (implementation
`4bd82511`):** represented Battleground type/map, status, queue-slot and arena-team
invitation transitions now resolve named operations on `wow-entities::Player`. The
owner follows `Player::m_bgData`/`BGData` (`Player.h:976,2821`),
`InBattleground`/`GetBattlegroundTypeId` (`Player.h:2335-2338`), the represented
Battleground id write (`Player.cpp:24258-24262`) and
`SetArenaTeamIdInvited` (`Player.h:1956`). Session keeps queue admission,
matchmaking/lifecycle coordination, packets and application effects; its
whole-state Battleground mutator is retained only for handle-less `cfg(test)`
fixtures. Owner tests, the 11-test canonical ownership scenario, the 23-test PVP
handler suite, both package checks, formatting/diff checks and the architecture
ratchet pass. Functional queue/matchmaking/lifecycle, persistence, captures and
live QA remain separate #584 gameplay gates.

**P2 Player-owned persistent capability transitions — 2026-09-14, #584 / PR #889,
integration `e37570c4e1e9feee04aadac6d485f5d1f314ced1` (implementation
`4066e261`):** the character-loaded at-login flags and weapon/armor proficiency
masks now have named transitions on `wow-entities::Player` in
`player/persistent_capabilities.rs`. The operations follow
`Player::SetAtLoginFlag` (`Player.h:2474`) and
`Player::AddWeaponProficiency`/`AddArmorProficiency` (`Player.h:1433-1434`),
including idempotent mask OR and removal reporting. Session retains persistence
projection and packet/application orchestration; its whole-state mutator is
fixture-only under `cfg(test)`. Owner tests, the 25-test persistence scenario,
spell-state scenarios, package checks, formatting/diff checks and the architecture
ratchet pass. Durable save/reload, captures and live QA remain separate gameplay
gates; #584 stays open.

**P2 Player-owned taxi flight transitions — 2026-09-14, #584 / PR #891,
integration `faa5964bbdb360adc12e58279ab17b76c76b66b9` (implementation
`a2a32586`):** post-teleport route advancement and taxi-flight cleanup now use
named transitions on `wow-entities::Player` over its `PlayerTaxi` state. The
boundary follows `PlayerTaxi::NextTaxiDestination` (`PlayerTaxi.h:74`) and
`Player::CleanupAfterTaxiFlight` (`Player.cpp:22019`); Session retains map
admission, spline completion and packet/application effects, while the generic
handle-less taxi mutator is fixture-only under `cfg(test)`. The Player owner
regression, 14 taxi tests, 11 canonical-access scenarios, 13 movement scenarios,
package checks, formatting/diff checks and architecture ratchet pass. Route
creation/node admission, teleport ordering, persistence, captures and live QA
remain separate #584 gameplay gates.

**P2 Player-owned world-local transitions — 2026-09-14, #584 / PR #893,
integration `cd054d5a3b19f9f75bfef726de1dc99368b3227b` (implementation
`89c66ada`):** all production zone/area, terrain-authority, PvP-hostility/timer
and outdoors writes now use named transitions on `wow-entities::Player` over
`PlayerWorldLocalState`. The boundary follows `Player::UpdateZone`,
`Player::UpdateArea`, `Player::UpdatePvPState`, `Player::UpdateContestedPvP`
and `WorldObject::IsOutdoors`; Session retains terrain/catalog resolution,
rest/aura/packet effects and application ordering, while the generic world-local
mutator is fixture-only under `cfg(test)`. The Player owner test, 11 owner tests,
chat/zone, world-state and outdoors/spell-state scenarios, package checks,
formatting/diff checks and architecture ratchet/self-test pass. Full terrain
admission, aura/quest/rest side effects, persistence, captures and live QA remain
separate #584 gameplay gates.

**F1 movement knockback-ACK slice — 2026-09-14, #63 / PR #866, merge
`0079daa81c4955e38031009a24b17b8dbabc7d9b`:** `HandleMoveKnockBackAck` now
admits a status whose GUID is the active `_player->m_unitMovedByMe`, after the
Player-owned validation, matching `MovementHandler.cpp:548-559`. The accepted
status remains written to the Player-owned movement-info state and the
`MoveUpdateKnockBack` publication keeps the Player as its source, as in C++.
A controlled-mover regression proves acceptance and Player-source publication;
the focused test, all 50 movement-handler tests, architecture checks,
`world-server` check and `validation-v2 quick` pass at
`target/validation-v2/manifests/20260914T052813.807323Z-281858-quick.json`.
#63 remains open for ordinary speed ACKs, remaining transport/death/BG/taxi
branches, broader mover coverage, exact captures and live QA.

**F1 movement force-ACK slice — 2026-09-14, #63 / PR #864, merge
`2d375ef164a292f10f265a73e2086785161e95f8`:** `HandleMoveApplyMovementForceAck`,
`HandleMoveRemoveMovementForceAck` and `HandleMoveSetModMovementForceMagnitudeAck`
now validate against the active `m_unitMovedByMe`, read the expected magnitude from
that Unit (including controlled Creature/Pet map state), adjust accepted client time
and publish from the mover's position/GUID. This matches
`MovementHandler.cpp:581-663`, including `mover->SendMessageToSet`; wrong-GUID
ACKs are rejected without publication. The focused controlled-mover regression, 50
movement-handler tests, architecture checks, `world-server` check and
`validation-v2 quick` pass at
`target/validation-v2/manifests/20260914T050653.422270Z-262358-quick.json`.
#63 remains open for ordinary speed ACKs, knockback, remaining transport/death/BG/
taxi branches, broader mover coverage, captures and live QA.

**F1 movement MoveTimeSkipped controlled-mover slice — 2026-09-14, #63 / PR #862, merge
`28762f166d39d69e844b99978bbd72e4085a2739`:** `HandleMoveTimeSkippedOpcode` now
validates the packet against the active `m_unitMovedByMe`, advances the Player or
represented controlled Creature/Pet's own uint32 movement clock, and publishes
`MoveSkipTime` from that mover's position/GUID. This matches
`MovementHandler.cpp:721-739`, including `mover->SendMessageToSet`; a focused
regression proves the controlled mover clock and observer routing. The focused
slice and movement suite pass (2 and 49 tests), architecture checks and
`world-server` compilation pass, and `validation-v2 quick` passes at
`target/validation-v2/manifests/20260914T043604.661090Z-236189-quick.json`.
#63 remains open for the remaining transport seat/offset admission, other movement
ACK/force/knockback/taxi/death/BG branches, broader mover coverage and live
client/server/DB capture QA.

**F1 movement stale-mover transport guard — 2026-09-14, #63 / PR #860, merge
`704dc4cb55652bd8a74974146a843dc3edc4a506`:** the stale transport distance
check now resolves the active mover position for Player, legacy controlled
Creature/Pet and the canonical Creature/Pet fallback before comparing against
`SIZE_OF_GRIDS`, matching `MovementHandler.cpp:345-350` for every represented
`Unit`. A focused controlled-mover regression rejects a large stale packet
without relocation or a second `MoveUpdate`; final validation passes with 3,881
tests and zero failures. #63 remains open for the remaining transport seat/
offset admission, death/BG/taxi branches, ACK/order and live client/server/DB
capture QA.

**F1 movement vehicle-turning slice — 2026-09-14, #63 / PR #859, merge
`95444be06bd7213529a0656d37c4aaeaa3410489`:** a Player passenger in a seat
with `VEHICLE_SEAT_FLAG_ALLOW_TURNING` now updates only facing, removes turning
interrupt auras and returns before relocation or `MoveUpdate` publication,
matching `MovementHandler.cpp:408-421`. Pre-return fall/landing/flight side
effects stay before the branch, and the focused regression proves unchanged
coordinates and no broadcast. Final validation passes with 3,881 tests and
zero failures. #63 remains open for complete transport offset/seat admission,
other mover kinds, death/BG/taxi branches, ACK/order and live capture QA.

**F1 movement transport membership slice — 2026-09-14, #63 / PR #855, merge
`10528d454f8e2b1504e6ed87ee5c1eb0a0d38524`:** the accepted Player movement path now reconciles the canonical Map-owned Transport passenger set before side effects and publication. A transport switch removes the previous passenger first; a valid in-world target is added; a missing or not-in-world target resets transport state. Focused attach/switch/detach/missing-target regressions and the 47-test movement suite pass; `validation-v2 final` passes with 3,880 `wow-world` tests and 0 failures. #63 remains open for vehicle seat/turning, complete offset validation, non-Creature/death/BG/taxi branches, ACK/order and live client/server/DB capture QA.

**F1 movement admission slice — 2026-09-14, #63 / PR #853, merge
`7c3add2fd5a1df791fcc793295028de553f9346a` (implementation `3af90ec2`):**
`HandleMovementOpcode` now rejects player movement while near or far teleport is
pending and rejects controlled-mover movement until its canonical MoveSpline is
finalized. The guards run before emote, position, state or packet-publication side
effects, matching `MovementHandler.cpp` and `Player.h::IsBeingTeleported`; the
active legacy `MapManager` spline authority remains used while that runtime is
live, with the canonical map fallback and fail-closed unknown state. Focused
positive/negative regressions, the full `wow-world` library suite (3,878 passed,
0 failed, 1 ignored), architecture checks and `validation-v2 final` passed at the
integration candidate. #63 remains open for transport passenger enter/exit and
reset, vehicle turning, non-creature movers, death/BG/taxi branches,
acknowledgement/order, and live client/server/DB capture evidence. The deferred
visibility bridge for `MoveInitActiveMoverComplete` is already integrated under
#588 and is not duplicated here.

**F1 equipment/stat projection — 2026-09-13, #61 / PR #851, merge
`b26ce713b844f1146a7b2952aade4dd532f1a16e` (implementation `fb33111c`):** the
character stat projection now lives in a private `handlers/character/stats.rs`
module and publishes one runtime-only `PlayerEffectiveCombatStatsLikeCpp` snapshot
owned by the canonical `Player`. Login and equipment recalculation publish the
represented item/stat projection; packet VALUES publication keeps its existing
adapter split. The bounded projection includes represented item stats, AP/ranged AP,
health/mana, ratings, spell power, armor, static school resistances and the Player
item-bonus fields for resistance, regen, penetration and shield. `PlayerGameplayState::is_empty`
now treats preallocated all-empty Void Storage capacity as empty. The final profile
passed at this merge SHA (`target/validation-v2/manifests/20260913T232737.961399Z-4127376-final.json`):
architecture/format checks, one-job workspace test compilation and 4,778
`wow-entities`/`wow-world` library tests passed (one ignored). #61 remains open for
production combat/melee/spell consumers, exact AP/damage/aura/regen/expertise/
penetration behavior, reversible equipment lifecycle and capture/live DB/relogin
acceptance where observable; this projection does not close the issue.

**F1 canonical AP threat consumer — 2026-09-14, #61 / PR #857, merge
`fd302c350ce56cae4841c04165e50c23ae0b01e8`:** initial spell threat now reads
the Player-owned effective attack-power snapshot, preserving C++ modifier sum,
non-negative clamp and multiplier order (`Spell.cpp:5558-5575`,
`Unit.cpp:9165-9180`). The snapshot retains AP and ranged-AP multipliers;
the focused harmful-spell and Player-owner tests pass. Aura-backed producers,
the remaining combat/stat consumers and live parity remain open in #61.

**F1 canonical weapon-range consumer — 2026-09-14, #61 / PR #858, merge
`cda7f8a0dcd2129c45d83982eabdb1ae7c3a6f14`:** the melee pass now snapshots
Player-owned base/offhand weapon ranges before the mutable Unit timer update;
item/AP/delay inputs come from one C++-shaped `wow-data` projection. The focused
world/entity/data tests and final validation pass at
`target/validation-v2/manifests/20260914T024623.613803Z-127420-final.json`.
Exact aura-backed damage modifiers, complete weapon admission, full spell/melee
parity and live capture/QA remain open in #61.

**F1 exact offhand admission — 2026-09-14, #61 / PR #869, merge
`abd396a0afcb247b411acbaf0d05d8713b747daf`:** the melee timer and swing path now
require a canonical Player offhand slot with a weapon inventory type and a
non-broken Item object, and reject the offhand branch in feral forms. This follows
`Unit::haveOffhandWeapon` (`Unit.cpp:496`), `Unit::DoMeleeAttackIfReady`
(`Unit.cpp:2140`), `Unit::IsInFeralForm` (`Unit.cpp:8807-8812`) and
`Player::GetWeaponForAttack` (`Player.cpp:9243-9270`). Focused owner admission/
feral regressions, all 26 `combat_tick_` world tests, production `world-server`
check and architecture checks pass at the pre-merge candidate. Aura-backed damage
modifiers, captures and live combat parity remain open in #61.

**Latest bounded data delivery — 2026-09-13, PR #848, merge
`179fd5d40491e4ded2a8c25b3261263330855cd5` (implementation `95274da1`):** the
effective Trait catalog now composes the C++ locale projections for
`TraitDefinition` and `TraitCurrencySource` after their effective base stores. The
adapter uses the exact locale SQL statements, official-then-custom precedence and
the same fail-before-publication field validation as the base hotfix set. Locale
entries are retained in the immutable Player catalog capability group for future
packet/runtime consumers. Focused locale composition and production batch tests pass;
this does not claim a live MariaDB startup/restart/relogin run, cross-store failure
coverage beyond the exercised batches, a production consumer for
`TraitDefinitionEffectPoints`, or later spending, mutation and starter-build
behavior. #524 remains open for those explicit gates.

**Previous bounded data delivery — 2026-09-13, PR #846, merge
`93fa95a9f4c803ff04c68253910766738f3b31be` (implementation `57116f75`):** the
effective Trait catalog now composes all 24 C++-projected base hotfix tables,
including `SpecSetMember`, in official-then-custom order before publication. Each
WDC4-backed store retains its table hash and applies final hash-scoped
`RecordRemoved` tombstones; malformed rows fail before the effective catalog is
published. The exact SQL projections are anchored to `HotfixDatabase.cpp:1409,
1690-1803`; focused adapter, statement and overlay tests pass. This delivery does
not claim locale-specific overlays (`trait_definition_locale` and
`trait_currency_source_locale`), a production consumer for
`TraitDefinitionEffectPoints`, complete cross-store/startup DB/restart/relogin
evidence, or later spending, mutation and starter-build behavior; #524 remains open
for those gates.

**Latest bounded architecture delivery — 2026-09-13, PR #844, merge
`6f42782fedb1eb77d7896fd139c195fbfbb9c43b` (implementation `77e2c4b2`):** the
fixed `Player::_voidStorageItems` state and its clear/load/mark/free-slot/lookup/
add/delete/swap transitions now have one canonical owner in `wow-entities::Player`.
Session keeps item-template admission, persistence orchestration, packet encoding and
detached test fixtures. The C++ anchors are `Player.cpp:18334`, `20002` and
`28025-28098`; no second production authority, lock or clock was added. The focused
owner invariants and all 29 existing Void Storage world tests pass. This slice does
not claim durable DB/restart/relogin evidence or close #584.

**Previous bounded architecture delivery — 2026-09-13, PR #839, merge
`cc0559980a4232ab5743affaaa2babfedffdfcf3` (implementation `ecc67603`):** a fresh
audit of the integrated head selected and delivered the remaining P2 item-modifier
owner closure. The previous generic
`mutate_player_item_modifier_runtime_like_cpp` Session surface is retired from
production; named Player operations now own item-set transitions, level caps, bonus
reset and resolved enchantment actions. Consumers and fixtures were migrated without
changing catalog, aura, packet or publication ordering. The canonical detached/stale
ownership regression, the Player owner regression and the existing item-threat
regression pass; `cargo check -p wow-world` with one job plus format/diff and the
ownership-policy check pass. `validation-v2 final` then passed on 2026-09-13; the
manifest is `target/validation-v2/manifests/20260913T165720.613650Z-3851477-final.json`
and the `wow-entities`/`wow-world` suite completed 3875 tests with zero failures
(one ignored). The delivery is integrated; #584 remains open for its remaining
boundaries. This evidence does not claim #61 effective statistics, TraitMgr consumers
(#524), Creature migration, live QA or DB durability.

The latest bounded functional delivery is #524's **semantic TraitMgr configuration
validation and deterministic login fallback**, integrated by PR #842 as
`995cd77fb48566b972741520986bc41d106470ea` (implementation `add6650a`). The immutable
projection now evaluates C++-aligned conditions, costs, parent/rank rules and granted
entries before publishing Player state, using canonical Player facts. PR #846 now
composes the effective base Trait/`SpecSetMember` SQL hotfix set with WDC4-hash-scoped
removals and fail-before-publication semantics. PR #848 now adds the two locale
overlays (`trait_definition_locale` and `trait_currency_source_locale`) with the
same official/custom composition and immutable capability retention. The issue
remains open for explicit cross-store fail-before-publication coverage,
startup/live DB/relogin evidence, production consumers for locale/effect-point data,
and later spending, mutation persistence and starter-build application. The
legacy Creature writer remains deferred until all AI, combat, movement, script,
persistence and visibility consumers have one owner.

**#486 target-account identity correction — merged in PR #807, integration
`86a0eb97` (2026-09-13):**
`CMSG_QUERY_PLAYER_NAMES` now reads a startup-warmed `CharacterCache` projection
(`CharacterCache.cpp:69+`) containing the target game-account, race, gender, class,
level and delete state, plus the login `account.battlenet_account` relation. The
handler no longer constructs account GUIDs from the querying session. It preserves
the C++ cache-presence gate and overlays the connected target's PlayerDirectory
identity when available. Character create/delete/rename/customize commits update the
same projection after their database result, so no second mutable Player owner or
packet-time full `CHAR_SEL_CHARACTER` query exists. The three focused query cases
(offline/mixed result, missing/failure ordering and connected override) and the two
cache projection tests pass; `cargo check --locked -p wow-database -p wow-persistence
-p wow-world -p world-server`, formatting/diff checks and architecture check/self-test
pass on aarch64 with one Cargo job. Declined-name fields and live undelete/barber
cache updates remain outside the currently represented Rust administration surface;
the implementation is integrated, while the issue remains open until the required
action-specific capture/live acceptance is completed.

**Integrated delivery — 2026-09-12, #787 / PR #792, integration SHA
`d14a9a67194e8013241e5dc837d9e72589aabac0`:** World/Map phase coordination is
implemented and locally accepted on `787-p3-map-driven-session-pass` at `76369bda`.
The correction retains the World
completion acknowledgement through task-owned finalization, session destruction,
BattlePet attachment release and registration retirement, preserves the shutdown
handover, and keeps interrupted effects behind the cross-step barrier.

The clean final gate passed at manifest
`20260912T225625.408381Z-3135200-final.json` (`status: passed`, 1,107.844 s,
one Cargo job): `wow-map` 735, `wow-world` 3,867 and `world-server` 585 tests,
with zero failures, plus the architecture and ownership checks. Guarded live
save/relogin QA also passed with `outcome: passed-restored`, `bot_status: 0` and
`login_save_relog_verified: true` for `TESTBOT1@bot.local`; the original live
SHA `c2a3b461…` is serving again. Two earlier attempts exceeded the guard's
startup timeout because the unoptimized debug candidate needs about 205 s to
load the full data set; they were restored cleanly and are not functional
failures. [The session checkpoint](../architecture/session-578-checkpoint.md#787-resumption-finalization-is-inside-the-world-completion-boundary--2026-09-12)
owns the contract and evidence. This is the selected #584 delivery; older
next-issue instructions below are dated history.

**Architecture plan reconciliation — 2026-09-11, #748, base `5d8c079a`:**
All 46 initially open issues were reviewed and their bodies synchronized with the
master plan. #42 was consolidated into #43, and #58/#59 into #41, retaining their
acceptance before closing them as superseded (`not_planned`). The resulting inventory
is 43 open program issues plus planning delivery #748; no functionality was marked
implemented by these administrative closures. #748 records this delivery's local
validation and publication status separately from historical gameplay evidence.

`PORT_PLAN.md` and GitHub #49 remain the general direction; the architecture documents
below hold the technical ownership and acceptance detail. #133 was closed on 2026-09-09.
#578/#585/#587/#588/#589/#716/#718/#722/#737 are integrated and closed in their bounded
scopes. Do not wait for or reopen #133, and do not use older checkpoint ordering as a
current instruction.

The technical gate remains **#584 core → #583 native/Wasm product → #153 independent
audit**. #584 retains unfinished C0–C4 core work; #583 owns the preserved M0–M4
native/Wasm product and does not block unrelated gameplay macros, while production module
integration waits for required #584 work. The Rust/Wasm/C mixed product is mandatory even
though operator activation is optional. No new micro-issues are planned:
each macro includes its consumers and validation, with file-specific exceptions allowed by
the module policy.

**Persistence ratchet reconciled — 2026-09-13, #584 follow-up:** the first remote check
after P3.1 found a valid out-of-line module declaration unsupported by the inventory
parser and stale snapshot/workflow paths left behind by the #695/#703/#718 file moves.
The parser now mounts such declarations as child source boundaries, with a focused
regression. A fresh inventory of the current `3.4.3` tree records 9,837 exact rows
(7,619 production and 2,218 fixtures), 1,024 production workflows and 1,027 semantic
groups. The reconciliation preserves 986 reviewed workflow contracts under their new
paths, explicitly annotates 38 current identities (including all six #718 quest-reward
adapter identities), and removes 40 obsolete pre-split identities. The full
`session-ownership-check check`, the checked snapshot/policy consistency test and
architecture `check --self-test` pass. This is architecture evidence maintenance only;
it does not claim gameplay progress or close any core gate.

**P3.2 map VALUES publication is implemented — 2026-09-13, #584, `0290ba79`:** the canonical
`Map::SendObjectUpdates` producer now snapshots and clears Player, Unit, GameObject/
Transport, Corpse, AreaTrigger, SceneObject and Conversation values. The world-server
map tick converts those snapshots to owned `UpdateObject` packets and delivers them
after releasing map, metadata and persistence guards, filtering current sessions by
map, instance, in-world state and committed visibility. DynamicObject retains its
existing dedicated session consumer, so no duplicate writer was introduced. The
focused map and world-server publication tests pass. Architecture `check` and
`self-test`, formatting, diff checks and validation-v2 `quick` pass; the manifest is
`target/validation-v2/manifests/20260913T032906.489145Z-3320295-quick.json`. This is
a bounded runtime publication delivery, not client capture/live QA or completion of #584.

**LFG decoder slice integrated — 2026-09-13, #582 / PR #797, integration SHA
`21686375d1fae81876b91f662069a25781ef10b2`:** six C++-anchored client packet
decoders are present in `wow-packet` and issue #582 is closed. The ten focused
decoder cases, the full 738-test `wow-packet` library suite, formatting/diff checks
and validation-v2 quick passed on the aarch64 host. This is wire decoding only:
there is no production handler registration, queue, matchmaking, teleport, reward,
two-socket live QA or full LFG parity claim. The detailed boundary and C++ anchors
are in [the LFG audit](../architecture/lfg-343-audit.md).

**P3.3 respawn/condition phase order — 2026-09-13, #584 / PR #798, integration SHA
`b912c95e33073090a1ef9ea0e09a2b260856389e` (implementation `c7daa069`):**
the canonical map tick now runs `ProcessRespawns` and
`UpdateSpawnGroupConditions` after the admitted session pass and before object
visitation, matching `Map.cpp:682-693` within the split-tick design. The phase is
restricted to the `MapTickParticipantLikeCpp` key/incarnation list so a map
recreated during the released session phase cannot receive its predecessor's work;
the `MapManager.cpp:287-318` delayed-update barrier and existing asynchronous DB
queue/persistence fence remain unchanged. A focused replacement-incarnation
regression plus the six existing spawn-condition tests, world-server check and
format/diff checks pass. Validation-v2 `quick` passed with one Cargo job in 7m03s;
manifest: `target/validation-v2/manifests/20260913T042148.632508Z-3355103-quick.json`.
The legacy Creature writer, nearby-cell visitation and other unrepresented
`Map::Update` phases remain outside this bounded delivery. The integrated delivery
has no live client, capture or DB/restart/relogin QA evidence.

**P3.4 delivered under #584 — nearby-cell ObjectUpdater production wiring, 2026-09-13:**
the production map tick now consumes one incarnation-scoped, deduplicated nearby plan
from in-world Players, viewpoints, represented far combat/aura/summon references and
active non-Players before selected `ObjectUpdater` family consumers. This follows the
C++ source order and visitor boundary (`Map.cpp:695-754`,
`GridNotifiers.cpp:258-264`): Players and Corpses stay out of the visitor set and
transports retain their separate full loop. The selection is implemented in
`wow-map/src/map/object_update_selection.rs`, wired through
`ManagedMap::update_after_sessions...`, and selected by
`world-server/src/runtime/map_tick.rs`; direct callers retain the whole-store seam.
Focused positive/negative tests prove nearby inclusion, out-of-cell exclusion and
split-tick consumption; architecture, formatting, `wow-map` tests and
`world-server` composition checks pass. Creature/Pet source activation uses the
canonical per-source `m_SightDistance` for inactive Creatures/Pets, while active
objects and Players use the map visibility range, matching
`WorldObject::GetGridActivationRange` (`Entities/Object/Object.cpp:1433-1450`).
Unsupported or missing source records fail closed. Creature AI/combat, scripts,
transport passengers, relocation fanout and live client/DB parity remain separate
#584 boundaries.

**P3.5 activation range — 2026-09-13, #584, implementation `7214fb68`:** the
production nearby-cell selection now resolves each center's C++ activation radius
from its canonical map record. Inactive Creatures/Pets use their existing
`sight_distance` field; active objects and Players retain `Map::GetVisibilityRange`.
The focused regression covers both branches, and the existing P3.4 nearby-selection
regression remains green. This is a finite phase-fidelity correction; it does not
claim the still-unmodeled Player cinematic activation override or any live client,
capture, DB/restart/relogin or Creature AI/combat acceptance.

**P3.6 Player cinematic activation radius — 2026-09-13, #584 / PR #804, integration
SHA `6157a0916899bb341b3f4a271ea69333a7cf61f6`:** the nearby-cell
selector now matches `WorldObject::GetGridActivationRange` for an active Player
cinematic: after the represented camera cursor is selected, it uses
`max(DEFAULT_VISIBILITY_INSTANCE, Map::GetVisibilityRange)`; beginning a sequence
before its first camera and ending it retain the map range. This follows
`Entities/Object/Object.cpp:1433-1450` and `Entities/Player/CinematicMgr.h:38-45`.
The implementation reads the canonical Player-owned cinematic state and does not
create a second owner or clock. The current Rust state has no FlyByCamera store, so
camera-row lookup and cinematic movement remain outside this selector. The focused
positive/negative regression is `grid_activation_range_uses_instance_distance_for_active_player_cinematic_like_cpp`;
the existing inactive Creature/active-object regression remains green. No live
client, capture or DB/restart/relogin evidence is claimed, and Creature AI/combat,
scripts, fanout and relocation notification effects remain separate #584 work.
The integration candidate passed `VALIDATION_V2_CARGO_JOBS=1 ./tools/validation-v2
quick --base origin/3.4.3` with one Cargo job in 11.519 seconds; manifest:
`target/validation-v2/manifests/20260913T061658.691366Z-3417098-quick.json`.

**P3.7 Creature relocation visibility fanout — 2026-09-13, #584 / PR #814,
integration `a3e970635c2df891f284fa6ac0b083b4c2659473` (implementation
`a130d9da`):** the canonical map now reuses the same Player source construction for
`ObjectUpdater` and `ProcessRelocationNotifies`, including viewpoints, far PvE
combat creatures, aura casters and summons. Relocation marking resolves each center
through `grid_activation_range_for_guid_like_cpp`, preserving the C++ source-specific
radius. `MapManager` consumes `CreatureRelocationVisibilityPlan.player_visibility_updates`
and coalesces one `PlayerVisibilityRefreshIntentLikeCpp` per affected Player before
the existing deferred Session rail. Residence revision, incarnation, viewpoint,
backpressure and delivery outside the map guard remain enforced by the existing
owner/Session path. The focused manager, map and relocation suites plus
`validation-v2 quick` passed on the aarch64 host in 36.36 seconds with one Cargo job
(`target/validation-v2/manifests/20260913T094251.016189Z-3544489-quick.json`). The
architecture check's physical ratchet passes, while the unchanged hotspot ceilings
still report their pre-existing drift; no baseline was regenerated. This slice does
not claim directed creature CREATE/DESTROY packets, AI/combat, scripts, FlyByCamera,
live client capture or DB/restart/relogin evidence.

**P3.8 map object lifecycle visibility intents — 2026-09-13, #584 / PR #818, integration
`6ef133437fc26e4e27182be084e50246ef48d375` (implementation `162c1a9d`):** the
canonical `Map::AddToMap` and `Map::RemoveFromMap` paths now mark nearby in-world
Players with `ObjectNotifyFlags::VISIBILITY_CHANGED` while the source is still
attached. This supplies the missing recipient selection for the existing deferred
visibility rail: the map tick consumes the flag, `MapManager` coalesces a
residence/incarnation-checked intent and the Session recomputes its real client
ledger after map guards are released. The helper covers already-in-world, Creature,
GameObject and generic map-object admission paths and removal before record erasure;
it sorts/deduplicates recipients, excludes a Player from its own walk and never
delivers a packet or awaits under the map mutation.

The focused add/remove map tests pass and prove the canonical Player flag mutation
and source removal. This closes the unmarked-recipient gap only: exact directed
CREATE/DESTROY bytes, transport-specific fanout, client capture and live
DB/restart/relogin acceptance remain separate #584 gates. The outcome telemetry
still names the direct synchronous C++ visibility call as a runtime gap because the
Rust equivalent is intentionally deferred through the existing phase boundary.

**P3.9 directed Creature DESTROY — 2026-09-13, #584 / PR #820, integration
`62c1369f4e49200b6f6d7605b0bc7de5caeace7c` (implementation `8ab62574`):** an ordinary
Creature removed from an active map now produces a typed directed-destroy result
while it is still attached. The canonical map captures nearby in-world Players,
excludes the direct charmer and binds the result to the map incarnation; the
world loop publishes it after releasing map guards through the durable Session
mailbox. The Session validates login, map/instance, incarnation and its
`client_visible_guids_like_cpp` ledger before atomically removing the GUID and
emitting one `SMSG_UPDATE_OBJECT` destroy. Durable ordering keeps this destroy
ahead of a coalesced visibility refresh. The focused map test, two positive/negative
Session tests, mailbox fence test, `cargo check` for `wow-map`/`world-server`,
formatting and diff checks pass with one Cargo job. CREATE, Pet, corpse/transport,
capture and live DB/restart/relogin acceptance remain separate gates; #584 stays
open for its remaining measured macrodeliverables.

**P3.10 Player/Unit VALUES fanout — 2026-09-14, #584 / PR #871 + #873, integration
`bd5b13d4b885f1887b95c654e2ebdb13f6c70c49` (implementation `30157c1f`, correction
`5586997d`):** the
Session now consumes the Player and Unit snapshots produced by
`Map::SendObjectUpdates` after map guards are released. Unit field mutations enqueue
their canonical in-world object, self updates retain owner/active-player fields, and
observers receive receiver-filtered Player/Unit values only when map, instance,
incarnation, phase, range and `HaveAtClient` fences hold. Creature/Pet updates use
the same committed visibility path; packet bytes are built before delivery and no
map lock crosses a send. The generic map rail excludes Player/Unit so the filtered
Session path is the sole publisher for those families, and the Session rechecks
the admitted `MapKey` before publishing after the map lock is released. Three
`wow-world` fanout regressions, 27 `wow-entities`
unit regressions, the production `world-server` check, architecture checks and
`validation-v2 quick` pass at
`target/validation-v2/manifests/20260914T072651.033982Z-371881-quick.json`.
Shared-raid field flags, exact packet captures, complete CREATE/Pet/corpse/transport
coverage and live DB/restart/relogin acceptance remain separate #584 gates; #584
stays open and the next macro still requires a fresh responsibility audit.

**Transport VALUES visibility projection — 2026-09-14, #584 / PR #876, integration
`ed92d14f173ee2be2332b242576eb603aaff58f4` (implementation `0c8e0f69`):** the
map `Transport` VALUES route now uses the C++ `Player::m_visibleTransports`
membership separately from ordinary `m_clientGUIDs`. The Session publishes that
generation-safe set through `PlayerSessionRegistrationLikeCpp` and
`PlayerRuntimeRecipient`; the world-server delivery adapter selects it only for
MO transport GUIDs, while all other object families retain ordinary committed
visibility. The Session command consumer applies the same gate, so a stale or
undeliverable Transport update is dropped before packet publication. The focused
world-server fanout regression proves visible and non-visible recipients, and the
wow-world regression proves absent, present and cleared Session membership. The
world-server check, focused tests, formatting/diff checks and full architecture
check pass. This closes only Transport VALUES fanout; Transport CREATE/DESTROY,
passenger lifecycle, exact captures and live DB/restart/relogin evidence remain
separate #584/#63 gates.

**P2 item-bonus writer retirement — 2026-09-13, #584 / PR #816, integration
`db1250767090a5c951dae96ad6c2a2d5b24873ff` (implementation `948b7ea9`, ledger
ratchet `d5d21a60`):** the last generic `with_bonuses_mut_like_cpp` closure no
longer exposes `&mut PlayerItemBonusStateLikeCpp` from Session. The state-only
resolved-effect tail of TrinityCore `Player::_ApplyItemBonuses`
(`Player.cpp:7688-7975`) and `Player::ApplyEnchantment`
(`Player.cpp:13058-13389`) is now
`PlayerItemModifierRuntimeStateLikeCpp::apply_enchantment_effect_action_like_cpp`
in `wow-entities`. Catalog lookup, item admission, spell casts, aura publication
and packets remain in `wow-world`; unsupported/deferred actions remain explicit
and are not falsely claimed as applied by the owner. The Player hotspot ceiling was
updated to the reviewed live aggregate (`15468` production, `13122` test lines);
the physical ratchet passes and the four unrelated logical hotspot drifts remain
unchanged. Evidence: 14 `wow-entities` modifier tests, 16 world-entity tests, 129
player-item tests, `cargo check --locked --tests -p wow-world`, formatting/diff
checks and validation-v2 quick in 8.53 s with one Cargo job
(`target/validation-v2/manifests/20260913T100604.990747Z-3562620-quick.json`).
No live runtime, capture, DB/restart/relogin or full item-stat parity claim is made.

**#524 skill catalog and TraitMgr projection — 2026-09-13, sequence implementation
`0192bac3` plus hotspot follow-up `de4e114e`, TraitTree overlay `9cca8f2b`, final-removal delivery `68fe676d`, generic TraitMgr index `eee72efd`, combat/class index `4ee7391e` and persisted-entry validation `c09a1f42`, integration `4e3ad8f0`:** the production bootstrap now keeps the C++ table stages independently observable: `SkillLine` → WDC4/official/custom `SkillLineAbility` → WDC4/official/custom `SkillRaceClassInfo` → WDC4 `TraitTree` plus WDC4/official/custom `SkillLineXTraitTree` projection. Official-then-custom order and fail-before-publication behavior remain explicit. The link store retains its WDC4 table hash and applies final table-scoped `RecordRemoved` statuses before projection. PR #830 adds the immutable `TraitSystemID -> TraitTreeID` projection and Generic config validation; PR #832 adds the C++ `ChrSpecialization -> ClassID -> _skillLinesByClass` combat projection and moves TraitMgr construction after effective `SkillRaceClassInfo` composition; PR #834 rejects missing or over-ranked persisted node entries at the existing Session authority boundary. Focused `wow-data`, `wow-world` and `world-server` regressions plus the physical architecture check pass. This remains a bounded startup correction, not complete TraitMgr parity: node topology, cost/condition/loadout consumers, final cross-store orchestration and startup/live evidence remain open.

**#524 TraitMgr node-graph projection — PR #836, merge `0fca1020`, implementation `d9770755`:**
the catalog owner now loads the C++ node, group, edge, cost, condition and loadout DB2
relations once, builds deterministic immutable indexes for tree nodes, node entries,
groups, parent edges, costs, conditions and specialization loadouts, and returns the
single `TraitNodeEntryStore` capability used by the world bootstrap. When the graph is
available, the existing Player trait-config authority rejects a persisted node/entry pair
that is not linked to one of the config's resolved trees. Focused `wow-data` and
`wow-world` regressions, `world-server` check, formatting/diff and physical-source checks
pass. This is still a bounded projection: the index does not yet implement currency
ownership, condition evaluation, spending, starter-build application, complete
cross-store failure orchestration or live startup/DB/relogin evidence; #524 remains open.

**Group state application is integrated — 2026-09-11, #743 / PR #750,
`77df8194` (implementation `9e6767bb`):**
`GroupRegistry` stays the single authority and every state-bearing group command now
either reaches its member or records a delivery obligation that the member converges
on in a dedicated driver phase. C++ never needs this fence because
`Group::RemoveMember` (`Group.cpp:550`) and `Group::Disband` (`Group.cpp:713`) call
`Player::SetGroup(nullptr)` (`Player.cpp:23440`) inside the operation itself. The
delivered contract, its exact code targets and its retained boundary are in
[the refactor completion plan](../architecture/refactor-completion-plan.md#entrega-local-aceptada--9e6767bb).
Evidence on this aarch64 development host with one Cargo job: `cargo test -p wow-world
--lib` 3,841 passed, `cargo test -p wow-social --lib` 80 passed, the three
production-linked `wow-world` integration targets passed, and `session-ownership-check
check --syntax-only`, `check_architecture.py check`/`self-test`, `cargo fmt --all --
--check` and `git diff --check` passed. The reviewed inventory deltas are exactly the
new surface and the `handlers/loot/handlers.rs` → `handlers/group/commands.rs`
relocation. **No live runtime, capture or DB/restart/relogin evidence exists for this
delivery**; queue saturation is exercised on the real bounded channel in the session
composition, not on a running server.

**Reputation encapsulation is integrated — 2026-09-11, #735 / PR #751,
`9ec0e855` (implementation `c68f8e16`):** the canonical Player
owns `PlayerReputationStateLikeCpp`, the equivalent of C++ `Player::m_reputationMgr`
(`Player.h:3116`), together with the state-changing invariants that need no catalog.
`ReputationMgrLikeCpp` became a borrow of that state, so the per-operation reconstruction
and the whole-aggregate write-back through `gameplay_state_mut()` are removed from the tree
rather than renamed; catalog resolution, rank/spillover rules and packet construction stayed
in `wow-world`, and the shared value types moved to `wow-constants` with a
`wow_data::reputation` re-export so no forbidden dependency was added. Every recorded
consumer migrated: load/hydration, login publication, reputation flags, quest rewards, spell
effects, kill rewards, the save projection and the save acknowledgement, with `need_send`
and `need_save` still independent. The delivered contract is in
[the refactor completion plan](../architecture/refactor-completion-plan.md). **No live
runtime, capture or DB/restart/relogin evidence exists for this delivery.**

**Talent encapsulation is locally accepted — 2026-09-11, #752:** the canonical Player
owns `PlayerTalentRuntimeState` with private fields and the named transitions C++
performs on `_talents`/`_specializationInfo`, each bounding its group index and glyph
slot. The session's generic talent-runtime closure is private to its owner module:
persistence load and the talent-reset COMMIT call named transitions instead. The
catalog, spell/aura and packet work stayed in `wow-world`. C++'s per-row talent
`State` remains unrepresented in Rust; that is a pre-existing boundary this delivery
records rather than changes. **No live runtime, capture or DB/restart/relogin
evidence exists for this delivery.**

**Spell-runtime encapsulation is locally accepted — 2026-09-11, #754:** the canonical
Player owns `PlayerSpellRuntimeState` with its fields closed to the Player module and
the transitions C++ performs on `m_spells` and `m_overrideSpells`. The owner now
enforces the rules callers wrote by hand, and the session's generic closure is scoped
to `session::spell_state`. The completeness flags keep their authoritative-empty versus
unhydrated meaning. **No live runtime, capture or DB/restart/relogin evidence exists
for this delivery.**

**Quest-state encapsulation is locally accepted — 2026-09-12, #756:** the canonical
Player owns `PlayerQuestGameplayState` with its fields closed to the Player module and
the transitions C++ performs on `m_QuestStatus`, `m_RewardedQuests`, `m_DFQuests` and
`m_seasonalquests`. Fourteen single-transition callers use named session operations; the
five catalog-driven objective walks and the quest-slot compaction keep the borrow as a
recorded projection whose exit condition is #41's objective-progress contract. **No live
runtime, capture or DB/restart/relogin evidence exists for this delivery.**

The remaining P2 operations — taxi, collections, item equipment sets, cast state,
difficulty, trait configs and the two canonical access helpers — then lead into P3
runtime/lifetime/private-hecs and P4 semantic/physical work under #584. Item-modifier
state ownership is delivered; remaining item work is limited to its catalog/effect
consumers and any separately reproduced gameplay gap.

Finite hecs V2 conformance passed within its recorded laboratory limits. Production `hecs`
and Wasmtime are not installed in this base. The dated six-clock trace and the 31 oversized
file measurement are evidence boundaries, not new exhaustive measurements. The historical
363-test analyzer result is not a new test run.

**Quest reward is integrated — 2026-09-11, #718:** P1 established one durable character
transaction for the represented reward participants. C++ `Player::RewardQuest`
(`Player.cpp:14625`) applies grants in memory and reaches the database through its closing
`SaveToDB(false)` (`Player.cpp:14867`); the Rust contract and departures are in the
[quest reward operation contract](../architecture/quest-reward-operation-contract.md).
Implementation is `78276ddf57463fc0f568c1c4bcf84d619af68cad`, integrated with its recorded
local evidence. Evidence on aarch64, Rust 1.98.0 and one Cargo job: `cargo test -p wow-world --lib quest`
427 passed; `./tools/validation-v2 final --base origin/3.4.3` passed at that SHA with a clean
tree and a manifest verified green, covering the 363 analyzer tests, the exact ownership
surface, the physical and hotspot ratchets and the server build. The three moved ceilings
and the reviewed ownership surface each carry their measurement. No real database write,
restart or relogin QA was performed; live durability and the unimplemented participants
remain open port work.

The records dated before this reconciliation are retained below as historical evidence.
Their former next-step, issue-state and test-count language does not override this block.

**Historical #716 delivery record — 2026-09-10, reviewed integration `aff42a51`:**
the user approved starting the architecture repair program. #716 restores exact
ownership-provenance acceptance after the mechanical module passes. At that base,
the general architecture check and self-tests pass; the syntax-only Session check
fails on two relocated fixture scopes and one still-live canonical/legacy bridge
whose parent import is lost by the analyzer. Physical terminal acceptance also fails:
31 files exceed 2,000 lines, while migration ceilings still pass. These results do not
reopen the integrated, closed #587/#588/#589 scoped gameplay deliveries.
Implementation and acceptance of #716 are tracked in the
[architecture plan](../architecture/modularity-and-ecs-plan.md#architecture-program-state--2026-09-11).
**#716 is locally accepted**, with implementation commit
`6ae62d73a9f910ebd664d82e331520b836a73c77`: 363 analyzer library tests, syntax-only
ownership, architecture check/self-test and validation-v2 quick pass. The 65 earlier
bridge records are preserved after the reviewed relocation; seven existing dual
references are now inventoried, giving 72 exact records. Test commands ran on the
working candidate at `aff42a51` and its verified V2 manifest records `dirty: true`;
the tested code/policies were committed unchanged. The
[technical refactor plan](../architecture/refactor-completion-plan.md#7-registro-historico-conservado)
records commands, source identity and the remaining boundaries. No push, merge,
server build, deployment, live QA or exhaustive persistence rescan was performed
for this tooling delivery. Physical terminal acceptance still fails at the same
31 files. The former ordering text is historical; current #584 sequencing is #743,
then #735 without a hard dependency, followed by remaining P2/P3/P4 work.

**Second physical pass closed — 2026-09-10, integration `a4a9073e`:** fourteen
deliveries between #685 and #711 finished the physical track outside the curated
hotspots. #685 and #687 moved the last inline `mod tests` blocks out of oversized
production files; #689, #691, #693, #701, #705, #707, #709 and #711 divided 27 files by
item family, by method group or by protocol family; #695, #697, #699 and #703 grouped
flat sibling module families into directories, by shared prefix and then by module kind,
taking the `wow-persistence` root from 53 entries to 26, `wow-database` from 64 to 40 and
`world-server` from 46 to 25. The physical ratchet's cohesion reviews fell from 111 to
63. Every delivery proved its item or method multiset identical before and after, kept
each crate's test count unchanged, and passed
`./tools/validation-v2 final --base origin/3.4.3` at the committed SHA.

That pass retained four files outside the hotspots above the 1,000-line review
budget: `session_tests.rs` (5,756) and its 197 shared helpers, plus
`statements/character/identities.rs` (1,789), `statement_def.rs` (1,627) and
`player/lifecycle_adapter.rs` (1,609). Forty-four more sat inside the eight curated
hotspot aggregates. #716 corrects the earlier claim that a trait impl or match must
change its type to delegate to private modules; those shapes and fixture sharing do
not establish terminal exceptions. This corrects
the earlier claim that relocation was exhausted for "the `map_manager` pair": #705 divided
its 2,069-line `impl WorldCreature` into seven submodules and #711 divided the 2,191-line
root into five, so both now sit at 130 and 255 lines. The measured limits behind the rest
are recorded in
[the module-design guide](../architecture/module-design-guidelines.md#what-a-physical-division-cannot-reach),
including #713's measurement that dividing the loot and quest fixture roots costs their
hotspot aggregates +63 and +43 test lines at the cheapest wiring available. What remains
includes both semantic extraction and justified private decomposition under #584
C0-C4. This dated record predates #133's closure and does not close the technical #584 gate.

**Historical physical decomposition pass — 2026-09-09, integration `df94f231`:**
#589 and its sub-issues #590-#595 are merged. Sixteen deliveries between #634 and
#664, following the Session-root separations of #603-#632, then completed the
physical half of the module-design policy: 184,295 lines of oversized roots
reduced to 4,972 across the QA bot, the architecture checker, `wow-entities`,
`wow-data`, `wow-packet`, `wow-loot`, `wow-map`, `wow-database`, `wow-world`,
`capture-diff`, `wow-instances`, `wow-ai`, `wow-network` and `wow-recastdetour`. Every delivery proved its top-level item
surface identical before and after, kept its crate's test count unchanged, and
passed `./tools/validation-v2 final --base origin/3.4.3` at the committed SHA;
sixty-seven of the hundred reviewed ceilings are now at or below 2,000 lines.

At that revision, 116,693 lines remained above 2,000 lines in 33 files: 29 sat
inside the eight curated runtime-ownership hotspots or the `map_manager` pair,
one was a vendored C++ translation unit and three were capture fixture shell
scripts with their own runtime-authorization gates. The earlier conclusion that
all relocation was exhausted is superseded by the 2026-09-10 review above. The
hotspot ratchet measures the module aggregate, so splitting a file inside an
owner at its baseline makes the aggregate grow; and splitting a bridge
function's context removed a row from the 65-row bridge inventory while the
build and all 3,822 `wow-world` tests stayed green (#662, reverted). That count
loss did not prove retirement or make file separation invalid; #716 repairs the
parent-import provenance gap. What remains includes semantic extraction and
justified physical decomposition under #584 C0-C4, with the per-owner entry
conditions recorded in
[the modularity plan](../architecture/modularity-and-ecs-plan.md#historical-physical-decomposition-pass--2026-09-09).
This dated record closes neither the technical #584 gate nor any current gameplay macro.

**Historical delivery — 2026-09-08, integration `8c47af95`:** #586 is merged and
#585 closed. #587's represented spell-acquisition boundary is implemented locally
on its own branch (original `43c4e801`, subsequent QA/evidence through `c88603ee`).
The branch is now rebased onto #588 commit `328b721f`; paired trainer and controlled
ordinary-effect acquisition/save/relogin pass on runtime source `3a634630`.
Combined final at `b48a1cfb` passes 14 commands, 5,824 Rust tests (two ignored) and
20 physical-policy tests, with zero failures. Local scoped acceptance is complete;
publication is pending. The earlier stationary-login failure remains recorded.
The user approved **#588 — deferred player visibility publication** as that
concrete prerequisite. It connects the existing map notify phase to retained
Session delivery, with exact incarnation/residence admission and the actual
client visibility ledger. No new clock or whole-map rewrite is selected.
Implementation and acceptance are tracked in
[the #588 checkpoint](../architecture/deferred-visibility-588-checkpoint.md).
#588 and #587 have local scoped acceptance. Their content is carried in full by
the #589 branch, which stacks on both, so a single PR integrates the three
deliveries; the user authorised that integration on 2026-09-08.
#589, the represented Player cast-request lifecycle, is **implemented and
locally accepted**, described in
[the existing architecture plan](../architecture/modularity-and-ecs-plan.md#historical-delivered-design--589-2026-09-08)
and recorded in [its checkpoint](../architecture/player-cast-589.md), which owns
the executed evidence, the represented value limits of the publication payload
and the deviations deliberately left open. Its six sub-issues #590-#595 cover the
functional contract and consumers, the 3.4.3 payload and publication, ownership
and physical organization, the acceptance bot, the integral regression campaign,
and closure. The earlier "paused, uncommitted working tree" state is superseded.
The current technical gate is #584 core → #583 → #153; #133's tracker closure is
already recorded above. #589 does not complete all spell gameplay.
The dated pre-merge account below remains historical
evidence, not an instruction to reopen #585.

Derived C++ and Rust trainer purchase/rejection/save/relogin pass, as does the
explicitly synthetic30798→6197 cast fixture with unchanged stock data and verified
effective SQL. Full trainer/cast windows retain documented order/metadata differences;
scoped learning and logout bytes match. Current #587 evidence:
[spell acquisition](../architecture/spell-acquisition-587.md).
#585's scoped acceptance and retained parity limits
are recorded below and in its checkpoint; do not reopen it from pre-merge status text.
The following dated candidate narrative is historical, not an instruction to repeat
completed work or a new whole-port completion/parity claim.

Local #585 candidate `ccf5f84d` passed final validation on 2026-09-07. After renewed
explicit authorization, guarded normal save/relogin QA passed and the original
world-server executable was restored and serving. The earlier permission blocker
is resolved. The subsequent orderly transport-disconnect/save/relogin scenario
also passed with QA tooling `55ec9a8b`, and the original server was restored.
Candidate `bf884aec` subsequently passed final validation and guarded pending-transfer
disconnect/save/relogin QA, after two separately committed fixes for realm routing
and retained-destination normalization. Original character location and server binary
were restored and verified. Applicable fresh C++ capture comparison remains outstanding.
A temporary derived C++ executable has now been exercised on an isolated database copy;
its four explicit reference corrections are documented in the checkpoint.
Candidate `07698639` passed final validation and 34 release production-integration tests.
Its installed LogoutComplete now matches C++ in bytes and realm connection in a fresh
strict one-packet comparison. The full paired normal-logout capture remains divergent:
instant-logout response/order and missing side effects. The same starting fixture exposes
a reputation flag mismatch; neither failed comparison is waived. The old capture wrappers
target PM2, not the current systemd deployment. These bounded passes are not full parity
or issue closure; details and private evidence identities are in the checkpoint below.
Subsequent final QA corrected compressed faction arrays and repeated map-entry reputation
initialization; installed `48b3729b` preserves the five affected faction rows. Its next
failure exposed persisted bows being rejected by a new-acquisition ambiguity policy.
That bounded load correction and fresh-C++-driven portal reason/orientation corrections
are implemented and installed candidate `5f5e225f` passed all three paired live
scenarios (normal logout, EOF disconnect and pending-transfer disconnect, each with
relogin and six retained persistence projections). Release production integration
passed 34 tests. Scoped LogoutComplete, InitializeFactions, TransferPending and
NewWorld comparisons match C++; full logout/portal windows remain divergent.
Final validation first exposed a concurrent ambient-trace test; the test-only
isolation repair passed 358 database tests and 20 concurrent trace-suite repetitions.
Candidate `b8895373` then passed final validation; the reviewed exhaustive ownership
check also passed with all 10,106 persistence references and 1,029 semantic groups.
This is bounded local acceptance evidence, not publication, full action parity or
closure of #585/#584.
The subsequent bounded scope review classifies inherited admission and broader
combat/aura/group/object cleanup differences as retained #584 work, not additional
#585 implementation prerequisites. Included-path blockers were corrected and tested.
The local delivery is ready for publication/review with those limits disclosed;
no full-action parity, push, merge or next implementation family is claimed.
PR #586 is published. Its two cross-socket ordering findings are corrected. Immediate
and delayed transfer use the existing realm writer fence; LogoutComplete uses the
instance writer fence. The existing session update becomes async without a new
owner/task. Renewed final validation at `37ef2a93` passed 6,162 tests with zero
failures. Release executable `5e48162a` passed paired normal logout/relogin and
pending-portal disconnect/relogin on the isolated database; the four scoped
logout/transfer packet comparisons passed. The original runtime stayed intact.
The user authorized integration without another review request; #585 remains open
until PR #586 merges. Further #584 work and the next-family selection remain separate.
Fresh C++ EOF and pending-transfer bot scenarios passed; C++ then crashed on stopping
after the portal scenario. That reference shutdown is not recorded as successful.
Current #585 contract, evidence and remaining boundaries:
[session finalization](../architecture/session-finalization-585.md).

**Historical capability-audit base:** 2026-08-09 · `3.4.3` @ `42977e9a`, including issue #26's
bounded creature-spell P1 wire/lifecycle acceptance and login faction hydration.
**Architecture/plan review:** 2026-09-05 · local #578 branch @ `93e4002a`.
The latter is not a new whole-port parity audit or a deployment claim. Undated subsystem tables,
counts and source locations below belong to the historical audit unless a later bounded note
explicitly updates them; recheck them against current code before selecting implementation work.

The guarded C++ and Rust `15691` evidence was recaptured from clean harness HEAD
`42977e9accb24fc3921af075f4122e1f0180f4a2`; strict diff is CLEAN at 2/2 packets and
`verify-required creature-spell-casting` is CLEAN.

This document replaces the drifting status snapshots in `_INDEX.md` (2026-05-01, "5–15%"),
the `MIGRATION_ROADMAP.md` §3 inherited table (which tells you not to trust it), and the
old append-log, now referenced through `current-session-handoff.md`'s Git-history pointer.
Its historical capability matrix is **grounded in
the named code audit down to subsystem/subdependency level**, not in what prior docs or the
inventory TSV claim. Architecture decisions: [adr-runtime-tick-ownership.md](adr-runtime-tick-ownership.md).
Forward plan: [PORT_PLAN.md](PORT_PLAN.md). Bugs found in already-shipped code:
[EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md). Source-verification issues #50–#64
and index #65 retain traceability under plan ledger L26, not authority for their original
diagnoses or finding counts. Recheck selected residuals against current Rust and exact C++
sources; preserve only independently supported behavior and evidence in the
[C++ findings](../audits/cpp-parity-findings.md).

Repository refactors are governed by
[`docs/architecture/ownership-and-boundaries.md`](../architecture/ownership-and-boundaries.md):
one mutable owner per concept, private modules before crates, explicit mirror retirement, and
executable Cargo/handler-contract guardrails.

The approved [module design guidelines](../architecture/module-design-guidelines.md) now require
both semantic boundaries and physical source/test navigability. Remaining monolith decomposition
belongs to #584 C0–C4; #583 applies the same policy to its own product, including Rust/Wasm/C.
The physical ratchet implemented above `8f5caedc` now covers repository source/tests/tooling,
with 103 initial legacy non-growth ceilings and an independent terminal mode. The first Rust
split above `d3f5c20c` reduces the persistence facade from 4,513 to 544 lines, preserving root
public contracts in private operation modules and retiring its legacy ceiling (102 remain).
The following adapter split above `1e6b7c40` reduces its 4,957-line root to 1,608 lines,
with private statement/row modules and responsibility-specific tests (101 legacy ceilings
remain). Its reviewed exhaustive inventory and matching policy are reconciled in the owning
checkpoint, including stale pre-catalog records and preservation of currency caller provenance.
These are physical decompositions, not closure of broad lifecycle capability cohesion. The existing
logical-owner guards remain. Migration PASS is not terminal acceptance: the legacy files
still need their stated splits or concrete bounded exceptions before #578 closes.

The following local C4 guard repair above published `9cd1da41` preserves scoped callable
reexport/import-chain provenance without exporting private aliases across packages. Unresolved
generic outputs conservatively retain known argument provenance instead of silently losing pools;
329 checker tests pass. Its reviewed exhaustive snapshot contains 10,084 rows: 53 explained
false aliases removed, 21 references added (20 deliberately conservative), with no direct SQL
operation removed and a byte-identical policy. Exact deltas and limitations live in the
owning checkpoint. The parser root's ceiling shrinks from 21,248 to 21,030 lines, with small
private provenance/test modules; this is not terminal physical or semantic closure.

The subsequent C4 repair above `bca3885f` corrects implicit child resolution beneath
explicit `#[path]` file mounts in both source walkers. Five compiler-backed fixtures
and the full 334-test checker suite pass. The exhaustive inventory comparison also
passes with the same 10,084 persistence rows and byte-identical snapshot/policy;
no ceiling or exception is widened. This closes the bounded module-path defect,
not C4 or production C0/C3 phase coordination; details live in the owning checkpoint.

The C0 cut above `36d0ccbf` adds the exact C++ world/map packet-filter contract to
`wow-handler`, tested across all processing classes and Player residences in dev/release.
It corrects the misleading Inplace/socket-thread description, without changing dispatch.
The current independent Session and map loops still lack the required phase coordination;
the integration cut and queue/incarnation/barrier obligations are recorded in the owning
checkpoint. A passing pure filter test is not production scheduling or C0 acceptance.

The C0/C1 residence cut above `590b93f0` checks canonical index, generation, backing
Player identity/container and world/map binding together. Existing residence queries
now fail closed on inconsistent state; the checked API distinguishes those errors
from missing and replaced owners. Public production-library lifecycle and Session
login/save regressions exercise the change. This is invalid-state admission hardening,
not a new scheduler, storage migration or complete lifecycle/durability acceptance;
exact tests and remaining boundaries live in the Session checkpoint.

The following C0 queue cut above `bdae6204` preserves unselected packets in one
bounded Session FIFO when a handler future is cancelled, without replaying the
selected handler's partial effects. C++ LockedQueue filters only the head and stops
when it is ineligible; this constrains the pending world/map integration. Current
normal dispatch order and registry remain unchanged; phase filtering and coordination
are still not enabled. The checkpoint records focused tests and the reviewed small
logical-owner growth, not a physical monolith waiver or C0 completion.

The subsequent C0/C3 rename cut above `ab1cdab3` connects owned read/commit stages
to the production Session driver: the handler submits and returns, Session admits ready
reads and presents confirmed commit results, and composition drains submitted commits
before disconnect save. Workers cannot mutate Session/Player or send packets. Cancellation
of the drain retains its handles; worker failure or a shutdown timeout is not clean
quiescence or rollback proof. This changes asynchronous scheduling while preserving Rust's
commit-result-before-response fence. Global World/Map phase coordination, the first complete
Player lifecycle/save vertical, resource/backpressure and live/durability acceptance remain
open. Exact validation and source anchors live in the owning Session checkpoint.

The following cut above `b35bba96` reproduces and removes blocking capacity waits
from ready rename callbacks. Session now retains owned channel-send futures for the
entire ready batch, preserving FIFO against later packets without repeating commits.
Callback coordination/transport lifetime moves into the private Session driver; read/
commit application stages stay transport-free. Worker loss stops read admission and
retires Session rather than pretending completion was a normal DB rejection. General
synchronous sends and immediate admission-error responses remain explicit C0/C3 work;
this is bounded callback evidence, not whole-runtime backpressure or durability acceptance.

## Historical architecture and execution checkpoint — 2026-09-05

The former approved implementation unit was **#578 with draft PR #579**, under #133. Internal
commits/checkpoints did not create micro-issues, micro-PRs or a new approval gate. The current
contract-led plan, exact inventories, acceptance evidence and remaining boundaries now live in
[`session-578-checkpoint.md`](../architecture/session-578-checkpoint.md). #153 verifies the
complete result; it is not an implementation owner for already-known cuts.

The [explicit reanalysis cadence](../architecture/modularity-and-ecs-plan.md#reanalysis-checkpoints--evidence-before-replication)
is conformance before production storage migration, then review of the first real C1/C2 vertical
with C0 execution evidence before replicating its design. C4 checks the complete #578 balance
before #583 production integration; #153 audits both merged macros. After architecture, review
each selected gameplay macro just in time and perform the fresh whole-port planning pass at
#47/M6.2. No checkpoint introduced another routine approval, issue or PR. The tracker closure
of #133 is recorded in the current block above; the technical gate remains #584 → #583 → #153.

At the reviewed local HEAD, canonical `wow_entities::Player` owns the migrated gameplay families
and `wow_map::MapManager` coordinates its generation-checked active/detached lifetime. The former
whole-Player Session write-back and ObjectAccessor Player-copy paths are retired. This is real
ownership progress, not proof that Session is already a thin shell: gameplay orchestration,
catalog/service retention, broad mutable Map access and runtime bridges were then #578 work;
the remaining technical ownership now belongs to #584.
SQLx isolation under #169 is closed; terminal capability cohesion and the complete Session/runtime
boundary still need evidence. Closing #252/#297/#378 proved their stated directory, transport and
classification cuts; those dated results do not alter the current #133 tracker closure.

The next cuts are selected by complete operation contracts and their deletion conditions, not
field counts. Distinguish implementation, production-path integration and parity evidence. Use
focused checks during a cut, bounded integration/failure tests at the affected owner boundary,
and the exhaustive/final stack at terminal acceptance. Retain required live/capture evidence and
explicit publication/deployment approvals; a green fixture suite does not replace them.

After the architecture deliverable, re-audit the next port macro against current Rust and exact
C++ anchors before implementing its residual work. For example, #26 closed a bounded wire/lifecycle
slice, not general creature spell execution; #30's original claim of no power deduction is stale
(`handlers/spell.rs` already checks and deducts canonical Player power and tests rejection without
deduction). Existing #30–#35 and the full-parity ledgers retain their broader contracts. Do not
restart completed work from an old issue diagnosis or silently narrow a milestone to one capture.

The latest [modularity and ECS plan](../architecture/modularity-and-ecs-plan.md), reviewed above
laboratory HEAD `ee9a0128`, **selects private selective `hecs` now**, retaining cohesive domain
aggregates. This is a design choice, not an installed backend or proof it beats every alternative.
The finite independent-state/third-module checkpoint precedes production migration,
not another open-ended backend selection. Its two-module pre-freeze stage passes
at `118171c1`; the post-freeze independent third module also passes all four producer/lifecycle
tests at `c67acbfd`, without host/ABI/oracle edits. The 320-sample aarch64 campaign also passes
all preregistered gates: [result, costs and retained evidence](../architecture/modularity-conformance-results.md).
This completes finite pre-migration conformance, not production acceptance or a 10 ms frame
budget; the next checkpoint is the first real C1/C2 vertical with C0 admission/phase evidence.
The first production C1 cut now captures one coherent full-save projection and acknowledges
only its saved incarnation/row values, retaining changes made during pending I/O. The old
group-wide ACK is test-only. Production-linked controlled-persistence tests cover late change,
replacement, rollback, Unknown and cancellation; they do not establish real DB/relogin or
scheduler parity. All C0–C4 acceptance remains open to the extent recorded in the
[checkpoint](../architecture/session-578-checkpoint.md), including far-transfer save semantics.
An authorized live run on 2026-09-05 now adds bounded **real normal-save/relogin** evidence
for runtime `68fb338b` with QA tooling `04d54074`: two confirmed MariaDB save transactions,
two fresh logins, 13 skill/207 reputation rows retained, and identical 42-known-spell packets.
Persisted spell/favorite/equipment/transmog tables were empty, so their nonempty mutation
branches are not claimed live-proven. The original world executable was restored and verified
serving; BNet was not restarted. No crash, injected failure, scheduler/transfer parity,
publication or macro completion is implied. Exact hashes, scope and reports are in the checkpoint.
The C1 lifetime cut rejects occupied-map destruction/bulk unload and preserves the old
incarnation when replacement cannot allocate a generation. Controlled production-linked map
tests reproduce the old failure; automatic evacuation and complete shutdown QA remain open.
Map occupancy now comes solely from canonical Players: the manual count field/setter/fallback
is retired, and instance-full/GM fixtures use real occupants with unchanged packet assertions.

Native Rust is the default for first-party/custom extensions; Wasmtime/Core Wasm is the selected
operator-optional executor of shared hooks/state/lifecycle contracts. **Scope expansion:** #583
now delivers that bounded adapter and Rust/C guest evidence as well as external stateful modules,
composition and durable operator lifecycle after #231/#578. #153 audits both macros before #133
closes, including Wasm acceptance. The bounded delivery no longer waits for M6; the wider #99
ecosystem retains a fresh planning gate. Current login-message modules do not prove this product.

The completed [V1 laboratory](../architecture/modularity-lab-results.md) supplies 34 contract
checks and 120 corrected-campaign samples on aarch64, all within its pre-registered budgets.
It demonstrates the modeled contracts/costs, not arbitrary module state, a non-Rust guest, real
save durability or production integration. Its first campaign is retained as superseded after
three test/adapter defects were corrected. The separate V2 result above adds independent-module
evidence; neither experiment installs a production ECS/Wasm dependency, deploys code, advances
gameplay completion or establishes a whole-port capability-audit base.

Database migration boundary (issue #256): the daemon-owned permissive `DbUpdater` has been
retired. The `rustycore-db` composition binary is the sole schema migration authority, using a
source-controlled immutable SHA-256 manifest, per-database/component chains, advisory locks and a
durable incomplete marker that makes no false MariaDB DDL rollback claim. `world-server` validates
auth/characters/world/hotfixes and `bnet-server` validates auth through bounded read-only queries
before runtime writes or listeners; neither scans SQL paths, creates schemas, invokes a SQL client
or downloads artifacts. Exact legacy hashes or explicit schema fingerprints provide the
TDB343.24081 transition without reapplying already-materialized RustyCore DDL. Baseline artifact
acquisition remains #255 and the terminal persistence audit remains #153.

Trainer architecture note (issues #157/#158/#159, later dispatched by #142): list and the buy
adapter share one immutable offer decision. Normal trainer teaching revalidates that decision
under the exclusive money owner, commits effective money plus the exact #164 spell/skill result in
one Character DB transaction, attributes unknown COMMIT outcomes with a durable 128-bit operation
token, installs runtime state, and then publishes money, visual kits 179/362 and acquisition actions
in C++ success order. Non-packet acquisition effects install immediately after commit so a later
cross-socket fence failure cannot discard them; a valid cast fully suppressed by immunity or
rejected by the dynamic spell-disable gate still pays and emits both trainer visuals like C++,
while channeled wrappers remain outside the reduced projection. A process-wide pre-ConnectTo
character claim rejects a second live session for the same GUID and is released on a failed
instance handoff or late login packet-ordering fence, preserving C++'s single `Player*` save
authority; normal logout retains that claim through the account-wide offline write and old Player
identity teardown. Effective equipped-item and target-restriction duplicates follow C++'s deterministic
highest-record-ID assignment; ordinary pending spell/skill
changes are saved before trainer preparation instead of making the trainer unavailable until the
next autosave. Trainer failures and visuals use the Realm connection; creature visual fanout
retains the already validated canonical-or-legacy source position. Castable wrappers
require both a startup audit of effective/world-table blockers and a fresh player effect-mask proof;
the startup audit intentionally omits shapeshift metadata because C++ trainer wrappers use
`TRIGGERED_FULL_MASK`, including `TRIGGERED_IGNORE_SHAPESHIFT`;
the active proof rejects unsupported pet-aura hooks before mutation, replays definite self-target
and aura-spell cast failures after the C++-ordered fee/visuals, and resolves retained immunity auras
from their creation difficulty instead of the player's current map difficulty;
active auras now match covered `EffectAura`/`EffectAttributes` and negative aura-link immunity to the exact wrapper
effect/spell while startup excludes unsupported mechanic/state shapes; full C++ immunity-map parity
remains deferred until canonical Unit ownership.
Aura restrictions, equipped-item restrictions and craft reagents compose DB2,
official/custom hotfix overlays and final removals. Craft startup authority rejects a craft when
its created item or any positive reagent
effective sparse item template is absent, matching
`SpellMgr::IsSpellValid`.
Deterministic player `EffectLearnSpell` retains its distinct immediate-runtime/deferred-save timing.
Issue #142 later activated the `TrainerBuySpell` dispatcher arm and reconciled the
PartyUninvite/Vehicle registrations to exact equality with zero drift exceptions.

Battle-pet trainer purchase note (issue #161): a confirmed battle-pet species is now a purchasable
offer product (`Trainer.cpp:127-146` resolves `IsCastable()` before the `AddPet` branch, so only
direct-learn trainer spells reach it) and the list renders it available because C++ `GetSpellState`
has no cap gate. The purchase itself closes the legacy crash window — C++ charged in memory and
committed Character DB first and Login DB second at the next `Player::SaveToDB`
(`Player.cpp:19336-19344`), and `BattlePetMgr::SaveToDB` cleared `SaveInfo` at statement-append
time (`BattlePetMgr.cpp:377`), so a crash between commits kept the charge and silently lost the
pet — with a durable saga keyed by a 128-bit request key shared with the #160 Login DB receipt:
guarded charge + pending command in one Character DB transaction, exactly one pet through the #160
account owner (fence, journal lease and per-species capacity rechecked inside it), success packets
queued only after pet durability and recorded afterward by a durable `published` marker,
exactly-once refund for terminal failures with absolute durable-money reconciliation, and bounded login recovery that
converges interrupted commands without background tasks. Publication keeps the C++ battle-pet
order (money update, `SMSG_BATTLE_PET_UPDATES` petAdded, dependent runtime learn +
`SMSG_LEARNED_SPELLS`, trainer visual kits suppressed, silent cap). Pet, charge and refund are
exactly-once; packet enqueue attempts are recoverable and may repeat because enqueue has no client
ACK and cannot be atomic with the marker, while actual network delivery remains best-effort. A
crash may cause a recovery re-send without consuming the sole durable recovery signal first;
admission-time capacity/journal-lock failures return a structured result
while the wire stays silent like C++. Full design, transition table and fault matrix:
[battlepets.md](battlepets.md) (2026-08-03, #161). #142 activated the dispatcher arm.

### Fidelity policy for proven legacy defects

The legacy C++ server is the behavioral baseline, not an instruction to reproduce undefined
behavior or a demonstrated logic bug. An intentional Rust deviation is acceptable only when the
C++ behavior and defect are both pinned to exact source, the replacement is the smallest bounded
repair, focused tests distinguish it from both the legacy failure and a speculative rewrite, and
the deviation is recorded in the owning migration item. Client-visible changes additionally need
the corresponding C++/Rust capture decision recorded and, when deliberately different, a
re-pinned golden approved as a compatibility change. Suspicious literals, cleanup opportunities
and merely plausible optimizations do not meet that bar.

---

## 0. Historical capability audit: the "represented" pattern

The historical audit found many **`represented_*_like_cpp` paths** where a handler decoded
and validated a request but recorded intent without the required live mutation. This explained
why represented breadth was not playable parity. It is not a current rule that every function
with that suffix is inert: later paths directly mutate canonical owners. Trace the selected
operation from admission through mutation, persistence and publication before diagnosing it.

- Where the mutation path *was* wired, the feature genuinely **WORKS** (melee combat,
  bounded creature aggro/threat and spell-wire publication, inventory move/equip/destroy,
  loot, quest accept/turn-in, vendor, trainer, groups — durable state paths persist to DB).
- Where only the represented layer exists, the feature **looks handled but does nothing**
  observable (mail, auction, trade, taxi, resurrection, hearthstone bind, GO-use/portals,
  most spell effects and creature AI families beyond the bounded aggro/threat/melee/template-
  spell slices).

This is why the old "98% represented" metric and "bags don't open" coexist without
contradiction: ~98% of logic is *represented*, a much smaller fraction is *live*. The plan's
job is to convert represented→live for the playable path, then for everything.

Initial bridge convention: [represented-live-bridge.md](represented-live-bridge.md) documents
the handler → represented intent → live application boundary. The first converted example is
client stand-state change because it has one canonical owner and a deterministic, capturable
realm response. Applied outcomes emit bounded telemetry only after the canonical mutation
succeeds; client-controlled intent history is retained only in tests.

Live evidence (2026-07-12): the stand-state bot passed a Sit request with distinct realm and
instance sockets, and the bounded C++/Rust capture matched four packets exactly:
`CMSG_STAND_STATE_CHANGE 0x318C` on connection 1, `SMSG_STAND_STATE_UPDATE 0x271C` on
connection 0, `SMSG_UPDATE_OBJECT 0x27CB` on connection 1, and the `CMSG_PING 0x3768`
fence on connection 1. Strict capture-diff reported CLEAN after symmetrically excluding only
ambient `s2c:0x2DD4` creature movement. The slice remains represented-partial for the
documented full `Spell::cancel`, original cast-difficulty metadata, cross-Unit aura-application
lifecycle, and canonical `Map::SendObjectUpdates` ownership gaps. Missing masks now resolve from
an effective table composed in C++ load order from `SpellInterrupts.db2`, official/custom SQL
overlays by DB2 record ID, world `serverside_spell` masks, and the interrupt-mask subset of
`LoadSpellInfoCorrections`; this does not claim full server-side `SpellInfo` or correction parity.

Creature-movement evidence (2026-07-24, issues #21–#24): M2.1's production implementation had already
landed after the issue was opened. The global legacy tick launches random/waypoint `MoveSpline`s,
serializes `SMSG_ON_MONSTER_MOVE`, and fans them to nearby sessions through the final
`HaveAtClient` gate; PR #77 installed/restarted that runtime and manually verified visible
creature movement in the client. The issue closeout pins a real 117-byte C++ compressed-waypoint
packet from the accredited capture artifact
`a25f2c2bbf60de6cda7e32f305d732733017e711eb474dd5dbf6e007690143a8`, and Rust reproduces
it byte-for-byte; the complete 717-test packet suite is clean with that regression.

Issue #24's guarded live Detour capture is now strict-clean on three isolated packets
(heartbeat, one compressed chase spline, ping fence). The capture proved C++ falls from the
elevated fixture to the lower `.map` plane when static VMap height is unavailable. Rust preserves
the elevation only because Detour itself returned a connected elevated polygon corridor; it does
not lift a lower route from equal endpoint heights, which cannot distinguish disconnected
platforms. Identity, options, flags and transport fields remain strict. The same capture exposed
and fixed a separate lifecycle omission: Rust now mirrors
`Creature::AtEngage`/home-finalize by temporarily adding `UNIT_FLAG_CAN_SWIM` when the movement
template permits water, so `MoveSplineInit` publishes the same `CAN_SWIM` flag as C++ and restores
the out-of-combat flag afterward. Player chase snapshots are keyed by map, instance and GUID, so a
live target that teleports elsewhere cannot feed foreign coordinates to the chaser's Detour map.
M2.2 adds one persistent `wow_movement::MotionMaster` to each legacy `WorldCreature` and advances
it once from the globally owned creature frame, after spline position advancement as in
`Unit::Update`. Random and waypoint execution is now gated by the selected stack entry; active
combat chase has normal priority, interrupts an in-flight wander spline with the existing
C++-shape stop packet in the same global aggro frame, but remains below a represented
highest-priority point/charge generator. The represented source lifecycle is advanced in that
same frame and popped finalizers are applied before resynchronizing, so finite spline/timer
generators release their selector proxy and expose chase/default naturally. Combat reset also
exposes the default generator again. This is a bounded runtime bridge: the owner-dependent
random/waypoint work still runs in the existing concrete generators after stack selection, and
real chase target pathing remains with M2.5.

M2.3 re-audited the existing world startup loader against `WaypointManager::LoadPaths`: the
issue's “no waypoint path loader” diagnosis was stale. The current DB loads 7,698 parent paths
and 142,185 ordered nodes, and 5,419 waypoint spawns resolve a nonzero addon path. The remaining
live defect was cadence: the spline advanced against elapsed wall time while random/waypoint
timers received a fixed configured 10 ms, so scheduler delay could finalize the spline before
the generator timer and stretch patrol re-arming in proportion to runtime lag. The single existing Tokio owner now measures and
clamps the real elapsed frame `diff`, matching the C++ `World::Update(diff)` →
`MapManager::Update(diff)` contract, without adding tasks or holding a lock across an await.
Positive, negative and long-horizon regressions prove random re-arming and multi-node waypoint
progress. The installed release (`3c210cdc…`) passed a connected bot login/stand smoke: the bot
received two `SMSG_ON_MONSTER_MOVE` packets and the server published 627 movement packets over
327 visible-work ticks in the observed window. This closes the bounded live wander/patrol item,
not Detour path exactness, formation/transport behavior, SmartAI callbacks or chase/threat.

M2.4's opening diagnosis — "navmesh loaded but `find_path()` never invoked" — was **stale**.
`wow-recastdetour` is a real vendored Detour build with `findPath`/`findStraightPath`/
`moveAlongSurface`/`raycast` plus a ported `FindSmoothPath`/`FixupCorridor`/`GetSteerTarget`, the
runtime pathfinder thread is created whenever `CONFIG_ENABLE_MMAPS` is on (C++ default `true`),
and both live generators already resolved corridors and launched them through
`MoveSplineInit::MovebyPath`. The real defects were in **what the query returned and when it was
even attempted**, all four contrasted against `PathGenerator.cpp`:

1. `PathGenerator::CreateFilter`/`UpdateFilter` (`PathGenerator.cpp:648-698`) derives the Detour
   filter from the owner; both live call sites passed a hardcoded ground-only creature filter, so
   amphibious owners could never cross `NAV_WATER`/`NAV_MAGMA_SLIME` polygons and combat/evade
   owners never got `NAV_GROUND_STEEP`. The filter is now sampled per creature from
   `CanWalk()`/`CanEnterWater()`/`IsInCombat()`/`IsInEvadeMode()`.
2. `CalculatePath` answers a missing navmesh/tile with `BuildShortcut()` and
   `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH` (`PathGenerator.cpp:79-86`), which
   `RandomMovementGenerator::SetRandomLocation` happily launches. RustyCore mapped that same case
   to a path *failure*, so wander on unmeshed terrain retried every 100 ms forever instead of
   moving. Only a genuine Detour error stays a failure now.
3. `BuildPolyPath` calls `BuildPointPath` exactly once (`PathGenerator.cpp:287`, `:527`). The Rust
   corridor builder also ran a full `findStraightPath` pass whose result was then discarded — one
   wasted Detour query per request, and its `PATHFIND_SHORTCUT`/`PATHFIND_SHORT` bits leaked into
   the surviving smooth path, which made the callers reject a perfectly good navmesh route. The
   point path is now built once, in the mode `_useStraightPath`/`_useRaycast` selects, and the
   possibly clamped `endPoint` is threaded through while `GetEndPosition()` stays the requested
   destination for the `_forceDestination` comparisons.
4. `CalculatePath` requires `HaveTile(start)` **and** `HaveTile(dest)`; C++ satisfies both because
   `TerrainInfo::LoadMMap` loads each grid's `.mmtile` as the grid loads. RustyCore demand-loads
   from the path request and only consulted the start position, so any destination one tile over
   reported "no navmesh" while the mesh sat on disk. Both endpoints are demand-loaded now.

The three shortcut/failure `PathType` values are also bit-exact again: C++ `BuildShortcut()`
*assigns* `PATHFIND_SHORTCUT` before the caller ORs onto it, so the no-poly and empty-`findPath`
branches are plain `PATHFIND_NOPATH` and the point-path failures are exactly
`SHORTCUT|NOPATH` / `SHORTCUT|SHORT`. A deterministic navmesh fixture (a walkable ring around an
unwalkable centre cell) proves the live waypoint tick walks *around* the hole with real
intermediate points; the same test degrades to the four-point straight line when pathfinding is
disabled, so it is a genuine regression guard.

Four further C++ branches were then closed in the same slice:

5. **The mesh-hole exceptions.** `BuildPolyPath` grants `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`
   when the owner `CanFly()` (`PathGenerator.cpp:180,198-202`), and the far-from-poly branch also
   shortcuts for a flying owner or one that `IsFalling()` towards a lower destination — the charge
   case (`:221-240`). Those owner facts are now threaded into the Detour layer. The `CanSwim()`
   halves still need `Map::GetLiquidStatus`; treating "no liquid data" as "not submerged" can only
   withhold a shortcut C++ would grant, never invent one.
6. **`UNIT_STATE_IGNORE_PATHFINDING`** is set from `CREATURE_FLAG_EXTRA_IGNORE_PATHFINDING`
   (0x20000000) as `Creature::Create` does (`Creature.cpp:1154-1155`), through both the create
   lifecycle and the runtime `flags_extra` seam the legacy registration path uses.
7. **Corridor reuse.** The previous `_pathPolyRefs` now travel in the path request, so
   `BuildPolyPath`'s subpath and 80%-prefix-plus-suffix branches (`:291-413`) are reachable, and
   `GetPathPolyByPosition` (`:94-123`) is ported so `GetPolyByLocation` consults that corridor
   before paying for `findNearestPoly` — which also changes the `distToStartPoly` feeding the
   7.0-yard test. Random and chase keep their corridor for the generator's lifetime like C++;
   waypoint and home get a fresh one per call because `MoveSplineInit::MoveTo` constructs a new
   `PathGenerator`. Two edge decisions are explicit: the C++ lookup really uses the full 3D
   `dtVdistSqr` and its literal squared `< 3.0f` threshold (effective radius `sqrt(3)`), so Rust
   does not replace it with the reviewer's 2D metric or a speculative `< 9.0`. A failed
   80%-suffix keeps the complete valid prefix: with no suffix there is no overlap polygon to
   subtract, although C++ unconditionally computes `prefix + 0 - 1`. Rust clamps the point path
   to that retained corridor boundary, recalculates a singleton prefix instead of reproducing
   C++'s zero-length tail underflow, and clamps a successful singleton partial corridor to its
   reachable polygon rather than appending a straight segment across a disconnected gap.
   `BuildPointPath` branches that call C++ `BuildShortcut()` now also clear the retained polygon
   corridor, so a later update cannot reuse path state that `PathGenerator::Clear()` destroyed.
8. **Chase and home now path.** `ChaseMovementGenerator::Update`
   (`ChaseMovementGenerator.cpp:94-240`) and `HomeMovementGenerator::SetTargetLocation`
   (`HomeMovementGenerator.cpp:60-82`) were already faithful Rust ports with **no caller**. Chase
   drives the live tick with the victim's facts snapshotted by the tick driver (players from the
   registry, creature victims from the map, both before the mutable borrow) because the creature
   step has no object accessor; it applies the C++ destination choice, `forceDest = CanFly()`,
   `ShortenPathUntilDist` against the victim, `SetFacing(target)` and the chase walk template.
   Rust deliberately stores the computed move-toward/move-away direction: C++ compares
   `moveToward` with `_movingTowards` but never assigns the field, leaving move-away range checks
   on the wrong bound and able to stop immediately. The direction is committed only after the
   corresponding spline launches; a direction flip discards the old `PathGenerator` corridor
   before querying, while a failed query cannot publish movement that never began. Home
   replaces a *teleport*: the old `Returning` arm assigned `move_target` onto the creature position
   with no spline and no packet. Both have around-obstacle tests against the real navmesh fixture
   that fail when pathfinding is disabled.

The connected gate is now represented by the fail-closed
`detour-chase-around-obstacle` capture flow. It pins a generated MMap tile, disposable
character/spawn identity, exact heartbeat → compressed `SMSG_ON_MONSTER_MOVE` → ping window,
full MonsterMove decoding (normalizing only the process-global spline ID), source/binary
revision provenance, and guarded private-DataDir/database restoration. Its reviewed C++/Rust
pair is strict-CLEAN across all three selected packets with an empty divergence baseline.

Still absent, and explicitly **not** claimed: **point/charge, fleeing and confused** generators are
complete, unit-tested ports with no live trigger — nothing sets `UNIT_STATE_FLEEING`/`CONFUSED`
(there are no fear/confuse aura handlers) and no live caller installs a Point generator, so wiring
tick arms for them would only be exercisable from tests. Also open: mutual chase (needs the
victim's `MotionMaster`), `IsWithinLOS` and `ShortenPathUntilDist`'s LOS test (VMap stub), the
`CanSwim()` mesh-hole halves and `Map::IsUnderWater` (liquid), `Map::GetForceEnabled/Disabled
NavMeshFilterFlags`, `_useRaycast`/`_useStraightPath` (no live caller),
`MovePositionToFirstCollision` + `IsWithinLOS` gating the wander roll, VMAP/liquid-aware
`NormalizePath`, off-mesh links/transports/formation, the reached-home addon/sparring-health reloads,
and C++'s per-instance pathfinder concurrency — every map still serializes
through one pathfinder thread.

M2.5 / issue #25 replaces spawn-frozen threat with the live C++ ownership path. Nonlethal hostile
spell damage now engages the creature and adds effective-damage threat; direct healing forwards
half of effective healing, divided across eligible threatening creatures. `EffectTaunt` matches
the caster to the available highest threat and `SPELL_AURA_MOD_TAUNT` gives the newest active
taunt priority until its DB2 duration expires, restoring an older still-active taunt afterward.
The global creature tick reselects at C++'s 110% melee / 130% ranged thresholds, broadcasts
`SMSG_ATTACK_STOP` on evade, clears threat/tap state, blocks re-aggro while returning, and restores
spawn health only when home movement finalizes. Threat references now also preserve C++'s distinct
`Suppressed` state for targets immune to the attacker's melee school, confused, or held by a
damage-breakable stun; they remain in the threat list but cannot be selected until online again,
and clearing the aura alone does not expire suppression: only new threat from that target or
C++'s explicit `TauntUpdate` reevaluation can reactivate it. An active taunt explicitly bypasses
suppression. `CallAssistance` is once per engagement, delayed by the configured family-assistance
delay, stored on the caller with assistant GUIDs like C++ `AssistDelayEvent`, re-resolved and
restricted by C++ `CanAssistTo` gates when due, and cannot chain from an assistant. Focused
positive/negative regressions also pin ordered, lossless delivery of committed creature combat
events; its per-session backlog is bounded and disconnects a stalled/desynchronized consumer
instead of dropping events or growing without limit. The complete `wow-world`
3119/0 and `wow-entities` 667/0 library suites are clean. A guarded live
`detour-chase-around-obstacle` recapture from `4535a25a` proves the attack-accepted → target
acquisition → chase slice byte/opcode-clean against the retained C++ golden (3/3 packets, no
value/routing/missing/extra differences); the fixture also restored its character, respawn, world
DB, and private DataDir snapshot exactly. That wire window does not exercise heal, taunt,
assistance, or evade; those branches remain covered by focused C++-anchored regressions rather
than dedicated live captures.

M2.6 / issue #26 closes the bounded stock `CombatAI`/`TurretAI` template-spell publication
slice. The globally owned creature frame reads the hydrated template slots and active-difficulty
spell metadata, schedules only the represented instant target shapes with the C++ raw cooldown
rules, and commits an atomic adjacent `SMSG_SPELL_START`/`SMSG_SPELL_GO` pair before the
same-frame melee phase. The final P1 hardening removes GO's unconditional-hit assumption.
It permits publication only for the bounded C++ resolution of a physical
`DmgClass=MELEE` Creature spell whose sole target is a Player attacked from behind, whose spell
and represented effect mechanics are all zero, and whose complete Creature/Player source
authority proves every omitted spell-hit source hit-inert. Completeness does not require every
external source to be empty: persistence/login sources may be nonempty when every exact effect is neutral
to this bounded result, but the reduced runtime's canonical local aura
application/modifier/visible containers still must be empty until their full C++ semantics are
owned. Player authority fails closed across persistence and login/zone reconciliation, map/area
ancestry, guild, skills, active/rewarded/auto-push quests, glyphs, active-specialization traits,
pets and battle-pet slots, FFA/PvP/war-mode state, SpellArea/outdoor/battlefield sources, and
script, legacy/all-rank and SpellLinked hooks. A valid SpellLinked hook blocks its candidate, as
does the absolute trigger ID retained from a rejected loader row.

A Creature-owned uniform roll in `0..=9_999` resolves base `MISS` below `500` (5%) and `HIT`
otherwise. C++ causal order is preserved locally: cast before repeat schedule and hit roll before
the cooldown draw; `NO_ATTACK_MISS` consumes exactly one hit roll before forcing `HIT`. The due
EventMap slot is cleared before cast, so a blocked post-cast schedule cannot create an immediate
retry loop. After publishing an accepted HIT, Rust tombstones before scheduling: C++'s launch
phase next consumes an unconditional critical roll and possible effect-value draws outside this
wire slice. MISS does not enter those target-effect draws and may retain authority for its repeat
delay. Spell, melee and movement randomness share a fail-closed Creature tombstone. A valid
melee swing that reaches the still-unrepresented C++ damage/outcome/proc calculation sets it and
publishes no invented damage or wire. C++ has one process-global RNG whereas Rust has one RNG per
Creature; P1 therefore claims equal distributions and local causal draw order, not the exact
global stream or cross-Creature interleaving. Missing or unsupported metadata, authority, power,
aura, projectile, target, visual, difficulty and optional-payload shapes fail closed with neither
START nor GO. Already-performed deterministic event/reset work remains, while a tombstone blocks
subsequent random-dependent work. This is a **wire/lifecycle subset only**: GO's resolved hit/miss
topology does not apply spell effects, damage, health, aura, or power mutation, and it is not
evidence of the full `Spell` pipeline or M3 combat math.

The first Rust live attempt exposed a real login bridge defect rather than a casting mismatch:
`player_faction_template_like_cpp` remained unset, the player registry published faction template
`0`, and the creature-hostility gate correctly rejected that unrepresented identity before
`AttackStart`. The final HEAD derives the player's faction template from `ChrRacesStore`
when the loaded identity is installed and mirrors it into the canonical `Player`; the regression
covers identity → registry → canonical player without manually seeding a faction. The accredited C++ source
derivation starts at base HEAD `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, applies the reviewed
one-file patch SHA-256
`ef8b3c29f46fe537e1ae4e826b5610afcd534999f900ec9554ee0534e7847262`, and yields patched HEAD
`8cfed90bf1720dbf8b9dc109113c8d7d9173ff6c`. That patch only corrects the
`ChrSpecialization` index-container bound needed to load the installed DB2 dataset; it does not
touch creature AI, spell selection, casting, or packet serialization.

The final fixture guard uses contract `creature-spell-casting-shell-fixture-v2`. It verifies the
installed stock `AIName=SmartAI` and difficulty-0 `StaticFlags1=0`, CAS-switches the capture
window to `CombatAI` and `0x00100000` (`CREATURE_STATIC_FLAG_NO_MELEE`), then CAS-restores the
exact `SmartAI`/`0` pair and verifies cleanup. Suppressing auto-melee prevents that unrelated path
from consuming melee damage RNG before CombatAI's due spell without disabling its EventMap cast.
The final live authority also loads effective `ChrSpecialization` hotfix rows and corrects
external-ID `AreaTable` WDC4 offsets, so Shattrath area `3697` resolves to map `530` and Terokkar
zone `3519` like C++. OutdoorPvPTF source spell `33377` is admitted only after its effective XP
and outgoing-damage auras plus exact runtime-hook authority prove it hit-inert; the four
Auchindoun dungeon IDs remain fail-closed without C++'s `(Map*, zone)` registration authority.

Both C++ and Rust were recaptured from clean harness HEAD
`42977e9accb24fc3921af075f4122e1f0180f4a2`. The final guarded Cabal Interrogator `22378` /
Eviscerate `15691` import contains exactly the adjacent START/GO pair on both sides with no
accepted divergence. It records one observed **HIT** branch and does not prove that `15691`
always hits. Its current review identities are:

- C++ RAW PKT:
  `b52cc8ba962160be63286e72eb7611c6282b0cdc3a1cee0082fc6d6d7bf2c7b9`;
- Rust RAW tree:
  `9aee309d9ffb2e2e1e5a33167c228ccaa8d1634d917efd026e1a525f2a5db94a`;
- C++ / Rust capture manifests:
  `d40e3615b3337a26a3c4d4e380dc665c23719133ec0b3c7a05febdfd640e849d` /
  `3c942209db52f9f36b3d661477f0cad766e7e2ee49cf1fc97d68a74d996f0da0`;
- filtered C++ PKT:
  `a6b32206e3277e455e25f6aa8e491606aa5cd9449e2bf24245ea9dd5db79d932`;
- normalized Rust tree:
  `cc8d53b06c2727c95990eda80fd095a1a7e390da0af16981429d07176e0c003b`;
- `capture-lineage.json` file:
  `f443539e7857ac27dfb2029012f1e889d92ed27a224f89f7a6247f9510f0479d`.

`diff creature-spell-casting --strict` reports 2 matched packets with zero value, routing,
missing, or extra differences. `verify-required creature-spell-casting` is CLEAN with exact
topology/order and correlated payload semantics.

---

## 1. Historical progress picture (three axes, not one number)

Historical capability snapshot at the audit base above, not measured percentages for #578 or
current HEAD. The current architecture checkpoint reports acceptance boundaries separately.

| Axis | Estimate | Meaning |
|---|---:|---|
| **A. Breadth represented** | ~98% of R8 rows touched | Logic exists/contrasted in the represented model. **Retired as a headline** — rewards breadth over working features. |
| **B. Live-playable core** | **partial, holed, buggy** | Core loop (login→move→melee→loot→quest→vendor→group) is live. The scoped D-C1…D-C9 CRIT integrity track is closed, but combat math and multiple HIGH/MED gameplay/runtime gaps remain. Spells and creature AI have bounded live subsets; most spell effects/AI families, world-interaction and death remain represented-only or absent. |
| **C. Full 1:1 parity** | **low** | Long tail absent: ~108 spell effects, ~255 aura types, content scripts (0/294k LOC), mail/AH/calendar live, BG/arena/instances, ~215 stat/data stores. |

Part 1 of the plan drives **B** to complete; Part 2 drives **C** to complete.

---

## 2. Historical capability matrix and bounded additions

This matrix records the historical subsystem audit and its explicitly dated additions. Its old
paths and statuses are discovery pointers, not proof that the same gap still exists at HEAD.

**WORKS** = mutates live state + persists/broadcasts like C++ · **WORKS⚠** = live + persists
**but has correctness/integrity bugs** (see [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md)) ·
**PARTIAL** = subset only · **STUB / REPRESENTED-ONLY** = validates/records intent, no
observable mutation · **ABSENT**.

> ⚠ The scoped D-C1…D-C9 integrity defects are closed, but the adversarial audit still lists
> substantial HIGH/MED gaps in [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md). "WORKS"
> means the named path is live; it is not a claim of full 1:1 gameplay parity.

### Core gameplay loop — live, with remaining non-CRIT gaps
| Capability | Status | Evidence / defects |
|---|---|---|
| Auth/BNet SRP6 + world-enter handshake | WORKS | recent `fix(bnet)` commits; played live |
| Player base/stat projection | **PARTIAL (live)** | issue #60 replaces the incorrect `player_levelstats` path with C++-anchored `player_racestats` + `player_classlevelstats`, `GtBaseMP`, `CreateHealth=0` and a shared create/login/equipment/level-up StatSystem projection. Login seeds passive parry/block capability before projection and defers its saved-health clamp until persisted stat auras and represented item/enchantment modifiers are active; covered total-stat aura recalculations retain those item bonuses and emit no pre-CreateObject VALUES delta. The recorded paired C++/Rust login captures match the scoped max-health/mana, five primary stats, armor, base mana, AP and damage fields, including the 3% total-stat racial passive. That evidence does not establish complete login `UpdateObject` or wider unit-mod/aura/item-stat parity, which remain open. |
| Starting skills and skill-rewarded login spells | **WORKS (scoped issue #62)** | `SkillRaceClassInfo` now follows C++ `Availability`/`MinLevel`; default skill rank/max/step follows language, level, mono, tier, always-max and DK rules; loaded rows are normalized like `_LoadSkills`; and the live no-DB-spell login path applies `LearnSkillRewardedSpells` with real spell levels, quest fallback, Riding, masks and actual skill values. Correct WDC4 inline IDs restore Common-compressed `SkillLineAbility` fields. A live Blood Elf Hunter C++/Rust pair yields the same exact 43-spell set under the reviewed unordered-map comparator; bit/count/list/favorites integrity remains strict. Wider skill gain/update/discovery/unlearn runtime remains in L18. |
| Player movement + broadcast to nearby | WORKS⚠ | `movement.rs:310`; trust-client position (D-H10), creature destroy deferred (D-H15), async CREATE race (D-H14) |
| Melee combat (deals damage→death→loot) | **PARTIAL** | `session.rs:47635`; **no damage formula / hit table / armor mitigation** (D-H1, D-H2) — numbers are wrong |
| Global creature runtime (aggro/melee/move→packets) | WORKS (default on) | `world-server/src/main.rs:12682+` |
| Inventory equip/swap/move/destroy (+DB) | WORKS | D-C1/C2 relog metadata and D-C4 atomic swaps are closed; issue #52 adds C++ position/bank/unequip/store/equip gates, container-aware move/merge/swap/destroy, realm-routed errors, and paired installed C++/Rust invalid-gate plus occupied-swap/relog evidence with a strict clean action capture |
| Loot items + money (+DB) | WORKS⚠ | D-C5/C6 concurrent-claim duplication is closed; quest-credit gap (D-H6) and process-abort recovery of detached post-COMMIT continuations remain separate boundaries |
| Quests accept/turn-in core rewards (+DB) | **PARTIAL** | `quest.rs:1359,5569`; kill/explore/item objectives may not auto-advance (D-H4/H5/H6) |
| Vendor buy/sell, Trainer learn, Groups (+DB) | WORKS⚠ | vendor D-C8 and group-capacity D-C9 atomicity are closed; finite-stock oversell (D-H11), trainer validation (D-H9), buyback/refund and wider group parity remain |
| Gossip menus + quest-giver status icons | WORKS | `handlers/quest.rs:1248`; gossip conditions evaluated |
| Item enchant/gem/socket, durability repair, binding | **PARTIAL** | D-C1/D-C2 relog now reloads and serializes the exact persisted 13-slot enchant/random-property state, including the paired exact create-block proof. Broader gem/socket/durability/binding runtime parity remains unproven and belongs to later parity work; issue #52 is the separate move/equip/store validation closure. |
| Bank / equipment-sets / void-storage persistence | **WORKS** | D-C3 is closed: PRs #103, #113 and #115 merged with required CI/review gates. Installed bank, equipment/transmog and void-storage relog QA passed; the committed equipment-set ACK and void-storage query captures are strict C++/Rust CLEAN. Issue #114's documented failure-only all-or-nothing divergence remains intentional and bounded to its stronger transaction contract. |
| UpdateFields / CREATE-block serialization | WORKS | `wow-packet/src/packets/update.rs`; issue #10 re-audited rows 1212–1220 and closed M1.4's bounded value gaps, including selected non-mana creature power and runtime GO ArtKit; canonical per-spawn ParentRotation remains in its documented architecture follow-up |
| Rested XP / offline rest state (XP slice) | **PARTIAL (live)** | issue #81: live accrual, consumption, DB persistence/relog and `SMSG_LOG_XP_GAIN` are capture-clean under the reviewed runtime-counter comparator; full `RestMgr` ownership and rest-area wire remain open |

#### 2026-07-18 bounded rested-XP evidence

The same guarded bot workflow passed against the primary C++ reference and
RustyCore. A DB-controlled 86,400-second offline interval produced wilderness
and resting bonuses of about `14.88` and `60.00`; a live Mana Wyrm kill
(`entry=15274`, map `530`) emitted realm-routed `SMSG_LOG_XP_GAIN` with
`Original=100`, `Reason=Kill`, `Amount=50`, and `GroupBonus=1.0`. XP persisted
`0 -> 100`, rest bonus persisted `300 -> 250`, a full relog observed the saved
state, the disposable fixture was restored, and the persisted 300-second
creature respawn row cleared naturally.

The committed one-packet C++/Rust flow is strict-CLEAN only under the narrowly
reviewed comparator that omits the nonzero lower 40-bit runtime counter of a
Creature Kill victim GUID. It still requires realm routing and exact high type,
realm, map, entry, subtype, server id, and every XP field; malformed or
zero-counter bodies fail. This does not prove a real tavern/city AreaTrigger
walk, nested `RestInfo` update-field wire, honor rest, full group/KillRewarder
fanout, or a per-Player `RestMgr`/`Player::Update` owner. The aggregate
`#PLAYER.12` therefore remains open.

### Engines — PARTIAL / STUB (the gameplay-quality gap)
| Capability | Status | Evidence |
|---|---|---|
| Spell cast → SPELL_START / SPELL_GO | **WORKS (bounded paths; issue #26 P1 verified)** | the represented player-cast handler publishes its existing START/GO path; issue #26 adds one map-owned creature path. P1 no longer presumes HIT: only a physical melee Creature spell against a rear-attacked Player, with zero mechanics and complete source authority proving omitted sources hit-inert, may publish the atomic pair; canonical local aura containers remain empty. Player persistence/login/zone/map/guild/skill/quest/glyph/trait/pet/FFA/SpellArea/hook evidence fails closed, including valid and rejected-trigger SpellLinked rows. The Creature roll maps `<500` to base `MISS` and the rest to `HIT`; local order is cast→schedule and hit→cooldown, with `NO_ATTACK_MISS` still consuming a hit roll. An accepted HIT publishes then tombstones before scheduling because subsequent launch/effect RNG is omitted; MISS may retain authority for its repeat delay. Spell/melee/movement share the tombstone. C++ global-vs-Rust per-Creature RNG parity is distribution/local-order only. Guard v2 recaptured C++ and Rust from clean `42977e9a`, temporarily switching stock `SmartAI`/`0` to `CombatAI`/`NO_MELEE` and restoring it; strict 2/2 and `verify-required` are CLEAN. This does not claim effect execution or the full Spell pipeline. |
| Spell effects dispatched | PARTIAL **~42/150** | `session.rs:48774-49383` |
| Spell cast cost/timing/cooldown | **PARTIAL** | SpellCastTimes/SpellCooldowns/SpellPower DB2 hydrate represented `SpellInfo`; the player path checks/deducts flat + mana-pct costs, while creature `CombatAI` uses raw `RecoveryTime`, floors missing/short values at C++'s 5-second AI default, and re-arms every repeat attempt in `[cooldown, 2×cooldown]`. Full C++ SpellHistory/modifiers and general cast lifecycle remain open. |
| Spell damage calc (coeff/crit/resist/absorb) | **ABSENT** | `SpellEffectDb2Entry` parsed but unused |
| Spell LOS/range/facing/reagent checks | **PARTIAL** | issue #26 revalidates the live canonical creature/victim, C++ min/max range with combat reaches and movement allowance, and map LOS immediately before publication; the final bounded P1 slice additionally requires that the Creature caster is behind its Player target. General player-cast range/facing/reagent coverage and real VMap-backed LOS remain incomplete; the shared VMap foundation is still a stub. |
| Aura apply + client update | PARTIAL | `session.rs:26431` |
| Aura periodic tick (DoT/HoT) + ~255 aura types | **STUB** (~5 types) | `session.rs:27810` expiry only; `unit_subsystems.rs:12-14` |
| Proc system | **ABSENT** | `SpellAuraOptionsEntry` fields unused |
| Channeled / missiles / ground-AOE / DynamicObject | **ABSENT** | no lifecycle state machine |
| Creature AI (threat gen / spell cast / waypoints / SmartAI / text) | **PARTIAL** | random wander, DB waypoint patrol, threat/taunt/assistance/evade, and bounded CombatAI/TurretAI template-spell START/GO publication are live; spell effects, other AI families, text and SmartAI interpretation remain open |
| Creature movement: spline broadcast (SMSG_MONSTER_MOVE) | **PARTIAL** | global legacy tick continuously launches and broadcasts random/waypoint splines on measured elapsed cadence; exact captured compressed-waypoint body is byte-clean |
| MotionMaster tick | **PARTIAL** | persistent per-creature stack is ticked once by the global frame and selects random/waypoint/chase priority; concrete owner-bound bodies and broader generator callbacks remain |
| Pathfinding (Detour navmesh query) | **PARTIAL** (random/waypoint/chase/home live) | real vendored Detour + ported `FindSmoothPath`; owner-derived filter, single `BuildPointPath`, both endpoint tiles demand-loaded, C++-exact no-navmesh shortcut, fly/falling mesh-hole exceptions, `IGNORE_PATHFINDING`, corridor reuse + `GetPathPolyByPosition`. Point/flee/confused have no live trigger; raycast/straight-path unreachable; mutual chase, VMAP LOS and liquid-aware `NormalizePath` absent |
| Canonical map tick (`wow_map::MapManager`) | **STUB** | `wow-map/src/manager.rs:520` no AI/combat side effects |

### World-interaction & death — REPRESENTED-ONLY (handlers record intent, no mutation)
| Capability | Status | Evidence |
|---|---|---|
| GameObject use (doors/chests/**portals**/quest objs) | REPRESENTED-ONLY | `handlers/misc.rs:6378`; portal/chest/transport fall to "not ported"; **#13** |
| Mail send/list/take/delete | **ABSENT** (no handler) | only `QueryNextMailTime` exists |
| Auction list/bid/buy/sell | REPRESENTED-ONLY | `handlers/misc.rs:1862` record-only |
| Player trade | REPRESENTED-ONLY | `handlers/misc.rs:1466` record-only |
| Taxi/flight path | REPRESENTED-ONLY | `handlers/misc.rs:117` no flight movement |
| Death/resurrection/corpse/graveyard | REPRESENTED-ONLY | `handlers/misc.rs:2443,2495`; "graveyard data not implemented" |
| Durability loss on death, ghost flags | **ABSENT** | no KillPlayer hook |
| Hearthstone bind / set home | **STUB** | `character.rs:8926` "TODO: set bind in DB" |
| Item-use → cast spell (`ItemEffect.db2`) | **ABSENT** | store not loaded; no `CMSG_USE_ITEM` executor |

### Simulation foundations — silent subdependencies (block correctness everywhere)
| Foundation | Status | Evidence / impact |
|---|---|---|
| Terrain height (GetHeightZ / ground correction) | **ABSENT** | creatures spawn at DB Z; no ground snap |
| VMap line-of-sight | **STUB** (returns `true`) | `world_object.rs:1551`; spells/pathing tunnel walls |
| Stat-formula GameTables (Gt* crit/dodge/HP) | **PARTIAL** | `CombatRatings.txt` now drives represented crit/dodge/parry/block rating scaling; HP/MP/regen/class stat tables still incomplete |
| `ChrClasses`, `ChrRaces`, `FactionTemplate`, `CharBaseInfo` stores | **PARTIAL (live)** | the first three stores are loaded and login now publishes the race-derived player faction to the registry/canonical Player, closing the faction-0 creature-hostility failure; full `CharBaseInfo` and wider create/stat parity remain open |
| `SpellPower`/`SpellCastTimes`/`SpellCooldowns` stores | **PARTIAL** | loaded into represented spell metadata for normal difficulty; full C++ modifier and runtime integration still incomplete |
| DBC/DB2 store coverage | **~110 / ~325 (34%)** | `cpp-db2-stores.tsv` |
| Player periodic save | **REPRESENTED-PARTIAL** | session timer now uses `CONFIG_INTERVAL_SAVE` / `PlayerSaveInterval` and queues represented `Player::SaveToDB`; installed runtime passed bot login/logout and action/travel/quest-objective preservation QA, while first-save randomization, capture diff, and manual live-client QA remain pending |
| Multi-statement save transactions | **REPRESENTED-PARTIAL** | represented character save now commits the Rust-covered `Player::SaveToDB` statement set in one `SqlTransaction`; full C++ save surface, login/account transaction coupling, capture diff, and manual live-client QA remain pending |
| Respawn DB persistence | **ABSENT** | in-memory queue; respawns lost on restart |
| Quest objective auto-credit (loot item / explore) | **ABSENT** | not hooked to loot/area-trigger |

### Absent systems (whole-domain)
Content **Scripts** (`wow-scripts` 40 ln vs ~294k LOC C++) · Battlegrounds/Arena/Battlefield/
OutdoorPvP · Instances lock/save/difficulty toggle · full Conditions eval · full Phasing
refresh · Weather · Warden · Calendar · Petitions · Pet/Totem AI. Empty crates:
`wow-combat`/`wow-spell`/`wow-achievement`/`wow-social`/`wow-pvp` (1 line each; `wow-ecs` was removed by #298).

---

## 3. Historical playable-blocker notes and bounded fixes

Issue #7's CUF login crash is fixed on its branch: Rust now matches the exact C++
post-add order, and a paired live capture with one non-empty profile pins the
four-packet sequence. The CUF and final phase-shift packets are byte-identical;
the broader M1 exit and manual-client UI validation remain open.

Issue #8 closes the compression-stream lifetime defect: the active C++ `0x400`
threshold now has one persistent deflate owner for the complete physical socket,
including across `WorldSocket::split_for_io`. Installed login QA decoded four
successive compressed packets through one persistent inflater before completing
the stand-state round trip. Original-client/manual UI validation remains part of
the broader M1 exit.

Issue #9 closes the first eleven login-burst audit rows. Seven live gaps are corrected:
global account-data and tutorial resends, battle-pet-lock placement, configured MOTD,
cross-socket packet ordering, contact-list publication, and PlayerCondition-filtered
account-mount partials. The other five rows were already fixed, request-driven, or
explicitly bounded after C++ contrast. An accredited 81-packet two-socket Rust capture
proves the corrected physical order; the installed C++ runtime could not complete the
same bot's second-socket login, so this is intentionally not described as a full
byte-clean login capture.

| # | Bug | Effect | Status |
|---|---|---|---|
| #13 | `CMSG_GAME_OBJ_USE` doesn't cast GO use-spell | **portals do nothing** | open |
| #9–#12 | 33 login-burst divergences (`world-load-audit.md`) | ordering/value parity | #9–#10 merged; #11 implemented and locally validated; #12 open |

---

## Historical architecture reality

At the 2026-09-05 reviewed local HEAD, the legacy creature runtime still supplies behavior while
canonical Map storage owns active Players and the staged `MapRuntime` applies typed transitions.
It is no longer accurate to call canonical Map an empty storage/runtime skeleton. Nor does that
make it the sole production simulation driver: shared manager synchronization, legacy/canonical
bridges and the remaining Session-driven work are explicit convergence boundaries.

The target remains a modular monolith with one mutable owner per concept, explicit C++ phases and
generation-checked Player identity. `MapRuntime` currently uses staged synchronous commands under
the existing manager synchronization; a dedicated task per map and a production ECS are not
claimed. See [the accepted entity-world ADR](adr-map-runtime-entity-world.md) and
[the current checkpoint](../architecture/session-578-checkpoint.md) for actual cuts and evidence.
Old LOC, warning and test counts elsewhere in this document must not be reused as current metrics.

---

## 5. Definition of "done" going forward (replaces `represented-complete`)

A capability is **done** only when: (1) it runs in the **live runtime**, (2) its wire output
is **capture-clean vs a C++ capture** of the same action — byte/opcode exact unless a narrowly
reviewed comparator omits only an intrinsically runtime-allocated identifier or canonicalizes
only proven unordered C++ collection order, while retaining every stable value/count and failing
malformed input — (3) it has been **exercised on a running server/client**, and (4) C++ refs +
the validating capture/test are cited.
`represented-complete` is no longer a closure state — it means "logic drafted, not yet live".

This gameplay-capability definition does not require a fresh capture for every behavior-preserving
structural commit. Refactors follow the proportional gates in AGENTS.md and their explicit issue
acceptance: preserve bytes, metadata, connection and order with focused evidence; obtain the
required action-specific capture/live evidence when the change or acceptance calls for it.
Report a bounded proof as bounded, and distinguish code tests, historical golden regression,
fresh runtime evidence and full functional parity. Architecture completion requires the complete
technical gate: #584 core, the #583 product (including the mandatory Rust/Wasm/C mixed delivery
with optional operator activation), and the independent #153 audit. #133 is already closed
administratively and is not a future gate. Do not substitute a favorable field count or test
total. Neither architecture closure nor playable M6 closes the full Part-2 parity ledgers.
