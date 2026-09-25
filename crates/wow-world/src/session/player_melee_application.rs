// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player melee application: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::combat;
use super::{ATTACK_DISPLAY_DELAY_LIKE_CPP_MS, HashMap, MIN_MELEE_REACH_LIKE_CPP};
use super::{NOMINAL_MELEE_RANGE_LIKE_CPP, ObjectGuid, Position, UnitState, WeaponAttackType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerAttackStartLikeCppResult {
    Rejected,
    Accepted { send_attack_start: bool },
}

/// One C++ `Unit::DoMeleeAttackIfReady` pass over a canonical player.
///
/// Lifted out of `impl WorldSession` by #28: the body was already exactly one
/// closure over `&mut Player`, and the global legacy loop reaches the same
/// player through the canonical map rather than through a session. Behaviour,
/// argument order and C++ anchors are unchanged.
pub(in crate::session) fn take_canonical_player_attack_swings_like_cpp(
    player: &mut wow_entities::Player,
    diff_ms: u32,
    in_melee_range: bool,
    facing_target: bool,
    within_los: bool,
    melee_damage_bonus: [RepresentedMeleeDamageBonusLikeCpp; 2],
    armor_mitigation: combat::RepresentedArmorMitigationLikeCpp,
    outcome_facts: (
        crate::session_rules::RepresentedMeleeAttackerFactsLikeCpp,
        crate::session_rules::RepresentedMeleeVictimFactsLikeCpp,
    ),
    damage_taken: crate::session_rules::RepresentedMeleeDamageTakenLikeCpp,
) -> Option<(
    Vec<combat::RepresentedMeleeSwingLikeCpp>,
    Option<Option<u8>>,
)> {
    // C++ `Unit::MeleeDamageBonusDone`'s `SPELL_AURA_MOD_AUTOATTACK_DAMAGE`
    // product, written by the owning session and read here by both owners.
    let autoattack_damage_multiplier = player.unit().mod_autoattack_damage_pct_like_cpp();
    // C++ `DoMeleeAttackIfReady` reads the `UnitData` ranges recalculated by
    // `UpdateDamagePhysical`; the Player-owned snapshot is the Rust equivalent.
    let base_weapon_damage = player.weapon_damage_like_cpp(WeaponAttackType::BaseAttack);
    let offhand_weapon_damage = player.weapon_damage_like_cpp(WeaponAttackType::OffAttack);
    // C++ `Unit::DoMeleeAttackIfReady` admits the offhand branch only when
    // `!IsInFeralForm() && haveOffhandWeapon()` (Unit.cpp:2140). Resolve both
    // predicates before borrowing the mutable Unit; dual-wield capability by
    // itself is not an equipped weapon.
    let has_offhand_weapon = player.has_offhand_weapon_for_attack_like_cpp();
    let is_in_feral_form = player.is_in_feral_form_like_cpp();
    let unit = player.unit_mut();
    let spell_pauses_combat_timer = [
        wow_entities::CurrentSpellSlot::Generic,
        wow_entities::CurrentSpellSlot::Channeled,
    ]
    .into_iter()
    .any(|slot| {
        unit.current_spell(slot)
            .is_some_and(|spell| spell.delay_combat_timer_during_cast)
    });
    if !spell_pauses_combat_timer {
        unit.update_attack_timers_like_cpp(diff_ms);
    }
    let mut swings = Vec::new();
    let mut processed_ready_attack = false;
    let mut base_attack_error_update = None;
    // C++: Unit::DoMeleeAttackIfReady, Unit.cpp:2087 exits before
    // processing swings unless UNIT_STATE_MELEE_ATTACKING is present.
    if !unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()) {
        return None;
    }
    // C++: Unit::DoMeleeAttackIfReady, Unit.cpp:2090 exits while charging.
    if unit.has_unit_state(UnitState::CHARGING.bits()) {
        return None;
    }
    // C++: Unit::DoMeleeAttackIfReady returns while casting unless
    // the active channeled spell explicitly allows actions.
    if unit.has_unit_state(UnitState::CASTING.bits()) {
        let channeled = unit.current_spell(wow_entities::CurrentSpellSlot::Channeled);
        if !channeled.is_some_and(|spell| spell.allow_actions_during_channel) {
            return None;
        }
    }
    let has_auto_attack_error = !in_melee_range || !facing_target;
    let melee_state_update_allowed =
        within_los && unit.can_attacker_state_update_melee_like_cpp(false);

    if unit.is_attack_ready_like_cpp(WeaponAttackType::BaseAttack) {
        processed_ready_attack = true;
        if has_auto_attack_error {
            base_attack_error_update = Some(Some(if !in_melee_range { 0 } else { 1 }));
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 100);
        } else {
            base_attack_error_update = Some(None);
            if has_offhand_weapon
                && unit.attack_timer(WeaponAttackType::OffAttack) < ATTACK_DISPLAY_DELAY_LIKE_CPP_MS
            {
                unit.set_attack_timer(
                    WeaponAttackType::OffAttack,
                    ATTACK_DISPLAY_DELAY_LIKE_CPP_MS,
                );
            }
            if melee_state_update_allowed {
                unit.remove_attacking_interrupt_auras_like_cpp();
                if unit
                    .current_spell(wow_entities::CurrentSpellSlot::Melee)
                    .is_some()
                {
                    let _ = unit.finish_spell(wow_entities::CurrentSpellSlot::Melee);
                } else {
                    let [min_damage, max_damage] = base_weapon_damage;
                    swings.push(represented_white_swing_damage_like_cpp(
                        min_damage,
                        max_damage,
                        autoattack_damage_multiplier,
                        melee_damage_bonus[0],
                        armor_mitigation,
                        outcome_facts,
                        damage_taken,
                        false,
                    ));
                }
            }
            unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        }
    }

    if !is_in_feral_form
        && has_offhand_weapon
        && unit.is_attack_ready_like_cpp(WeaponAttackType::OffAttack)
    {
        processed_ready_attack = true;
        if has_auto_attack_error {
            unit.set_attack_timer(WeaponAttackType::OffAttack, 100);
        } else {
            if unit.attack_timer(WeaponAttackType::BaseAttack) < ATTACK_DISPLAY_DELAY_LIKE_CPP_MS {
                unit.set_attack_timer(
                    WeaponAttackType::BaseAttack,
                    ATTACK_DISPLAY_DELAY_LIKE_CPP_MS,
                );
            }
            if melee_state_update_allowed {
                unit.remove_attacking_interrupt_auras_like_cpp();
                let [min_damage, max_damage] = offhand_weapon_damage;
                swings.push(represented_white_swing_damage_like_cpp(
                    min_damage,
                    max_damage,
                    autoattack_damage_multiplier,
                    melee_damage_bonus[1],
                    armor_mitigation,
                    outcome_facts,
                    damage_taken,
                    true,
                ));
            }
            unit.reset_attack_timer_like_cpp(WeaponAttackType::OffAttack);
        }
    }

    processed_ready_attack.then_some((swings, base_attack_error_update))
}

/// C++ `CombatManager::SetInCombatWith` for a player attacker, on an already
/// locked map.
///
/// Lifted by #28 so the global loop can begin a combat reference without a
/// session. The session variant keeps the lock acquisition and delegates here.
pub(in crate::session) fn begin_combat_ref_on_map_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    attacker_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    relation_represented: bool,
    attacker_is_friendly_to_victim: bool,
    victim_is_friendly_to_attacker: bool,
) -> bool {
    let Some(attacker) = map.get_typed_player(attacker_guid) else {
        return false;
    };
    let attacker_unit = attacker.unit();
    let attacker_world = attacker_unit.world();
    let attacker_combat = &attacker_unit.subsystems().combat;

    let (context, both_player_controlled) = if let Some(victim) = map.get_typed_player(victim_guid)
    {
        let victim_unit = victim.unit();
        let victim_world = victim_unit.world();
        let victim_combat = &victim_unit.subsystems().combat;
        (
            wow_entities::CombatBeginContextLikeCpp {
                same_unit: attacker_guid == victim_guid,
                attacker_in_world: attacker_world.object().is_in_world(),
                victim_in_world: victim_world.object().is_in_world(),
                attacker_alive: attacker_unit.is_alive(),
                victim_alive: victim_unit.is_alive(),
                same_map: attacker_world.is_in_map(victim_world),
                same_phase: attacker_world.in_same_phase(victim_world),
                attacker_unit_state: attacker_unit.unit_state(),
                victim_unit_state: victim_unit.unit_state(),
                attacker_combat_disallowed: attacker_combat.combat_disallowed,
                victim_combat_disallowed: victim_combat.combat_disallowed,
                relation_represented,
                attacker_is_friendly_to_victim,
                victim_is_friendly_to_attacker,
                attacker_or_owner_player_is_game_master: attacker.is_game_master_like_cpp(),
                victim_or_owner_player_is_game_master: victim.is_game_master_like_cpp(),
            },
            true,
        )
    } else if let Some(result) = map.with_creature_like_cpp(victim_guid, |victim| {
        let victim_unit = victim.unit();
        let victim_world = victim_unit.world();
        let victim_combat = &victim_unit.subsystems().combat;
        (
            wow_entities::CombatBeginContextLikeCpp {
                same_unit: false,
                attacker_in_world: attacker_world.object().is_in_world(),
                victim_in_world: victim_world.object().is_in_world(),
                attacker_alive: attacker_unit.is_alive(),
                victim_alive: victim_unit.is_alive(),
                same_map: attacker_world.is_in_map(victim_world),
                same_phase: attacker_world.in_same_phase(victim_world),
                attacker_unit_state: attacker_unit.unit_state(),
                victim_unit_state: victim_unit.unit_state(),
                attacker_combat_disallowed: attacker_combat.combat_disallowed,
                victim_combat_disallowed: victim_combat.combat_disallowed,
                relation_represented,
                attacker_is_friendly_to_victim,
                victim_is_friendly_to_attacker,
                attacker_or_owner_player_is_game_master: attacker.is_game_master_like_cpp(),
                victim_or_owner_player_is_game_master: false,
            },
            false,
        )
    }) {
        result
    } else {
        return false;
    };

    if !wow_entities::CombatSubsystem::can_begin_combat_like_cpp(context) {
        return false;
    }

    let Some(attacker) = map.get_typed_player_mut(attacker_guid) else {
        return false;
    };
    let attacker_started = attacker
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(victim_guid, both_player_controlled, false);

    let victim_started = if let Some(victim) = map.get_typed_player_mut(victim_guid) {
        victim
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_in_combat_with(attacker_guid, both_player_controlled, false)
    } else if let Some(victim) = map.get_typed_creature_mut(victim_guid) {
        victim
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_in_combat_with(attacker_guid, both_player_controlled, false)
    } else {
        false
    };

    attacker_started && victim_started
}

/// Apply one player's melee swings to a canonical player victim.
///
/// Lifted out of `run_combat_tick` by #28: the body was already one closure over
/// `&mut Player`, and whoever owns the tick resolves the same transition. The
/// arithmetic — `max(1)` per swing, saturating health, `-1` unless the swing
/// overkills — is unchanged.
pub(in crate::session) fn apply_player_melee_to_canonical_player_like_cpp(
    victim: &mut wow_entities::Player,
    swings: &[combat::RepresentedMeleeSwingLikeCpp],
) -> Option<(Vec<(u32, i32)>, u8)> {
    if !victim.unit().is_alive() {
        return None;
    }
    let target_level = victim.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8;
    let mut sent_swings = Vec::new();
    for swing in swings {
        // C++ `DealMeleeDamage` applies nothing for a missed or avoided swing.
        if swing.damage == 0 {
            sent_swings.push((0, -1));
            continue;
        }
        let damage = swing.damage;
        let health_before = victim.unit().data().health;
        let health_after = health_before.saturating_sub(u64::from(damage));
        victim.unit_mut().set_health(health_after);
        let over_damage = if health_after == 0 {
            u64::from(damage).saturating_sub(health_before) as i32
        } else {
            -1
        };
        sent_swings.push((damage, over_damage));
    }
    Some((sent_swings, target_level))
}

/// What one player's melee pass did to a legacy creature.
///
/// `move_stop` carries the stop position and spline id rather than serialised
/// bytes: whoever owns the tick applies the transition, and the session that
/// owns the receiver builds the packet. Keeping construction at the session is
/// what makes the bytes identical — `MonsterMoveStop` is viewer-independent,
/// but the values update beside it is not (#28).
#[derive(Clone, Debug)]
pub(crate) struct PlayerMeleeCreatureHitLikeCpp {
    /// `(damage, killed, over_damage)` per swing, in swing order.
    pub swings: Vec<(u32, bool, i32)>,
    /// `(hit_info, victim_state, blocked, original_damage)` per swing,
    /// index-aligned with `swings`.
    pub swing_presentations: Vec<(u32, u8, u32, u32)>,
    pub entry: u32,
    pub level: u8,
    pub died: bool,
    pub move_stop: Option<(Position, u32)>,
    pub values_update: wow_entities::UnitValuesUpdate,
}

/// Module-level so the global legacy loop, which has no session, shares the
/// same arithmetic as the session owner (#28): C++ `CalculateMeleeDamage`
/// (`Unit.cpp:1326-1334`) rolls `CalculateDamage`, passes it through
/// `MeleeDamageBonusDone` and applies `CalcArmorReducedDamage`.
#[allow(clippy::too_many_arguments)]
pub(in crate::session) fn represented_white_swing_damage_like_cpp(
    min_damage: f32,
    max_damage: f32,
    autoattack_damage_multiplier: f32,
    melee_damage_bonus: RepresentedMeleeDamageBonusLikeCpp,
    armor_mitigation: combat::RepresentedArmorMitigationLikeCpp,
    outcome_facts: (
        crate::session_rules::RepresentedMeleeAttackerFactsLikeCpp,
        crate::session_rules::RepresentedMeleeVictimFactsLikeCpp,
    ),
    damage_taken: crate::session_rules::RepresentedMeleeDamageTakenLikeCpp,
    offhand: bool,
) -> combat::RepresentedMeleeSwingLikeCpp {
    let rolled = crate::session_rules::white_swing_roll_like_cpp(min_damage, max_damage) as f32;
    let damage = (rolled + melee_damage_bonus.flat as f32)
        * melee_damage_bonus.pct
        * autoattack_damage_multiplier;
    // C++ `CalculateMeleeDamage` runs `MeleeDamageBonusTaken` between the done
    // bonus and the armour reduction (`Unit.cpp:1326-1341`).
    let damage = crate::session_rules::melee_damage_taken_apply_like_cpp(
        damage_taken,
        damage.max(1.0).round() as u32,
    );
    let damage = crate::session_rules::armor_reduced_damage_like_cpp(
        damage,
        armor_mitigation.attacker_level,
        armor_mitigation.victim_level,
        armor_mitigation.victim_armor,
        armor_mitigation.armor_penetration_pct,
        armor_mitigation.target_resistance_normal_aura,
        armor_mitigation.ignore_target_resist_normal_pct,
        armor_mitigation.bypass_armor_pct_by_caster,
    );
    // C++ assigns `OriginalDamage` inside the outcome switch, so the shared
    // arithmetic returns it with the dealt damage (`Unit.cpp:1343-1440`).
    // C++ rolls the attack table after mitigation and before the outcome
    // switch (`Unit.cpp:1341-1343`).
    let outcome_inputs =
        crate::session_rules::melee_outcome_inputs_like_cpp(&outcome_facts.0, &outcome_facts.1);
    let outcome =
        crate::session_rules::rolled_melee_outcome_like_cpp(&outcome_inputs[usize::from(offhand)]);
    let (damage, blocked, original_damage) = crate::session_rules::melee_outcome_damage_like_cpp(
        outcome,
        damage,
        armor_mitigation.attacker_level,
        armor_mitigation.victim_level,
        outcome_facts.0.crit_damage_multiplier,
        crate::session_rules::CREATURE_BLOCK_PERCENT_LIKE_CPP,
    );
    let (hit_info, victim_state) =
        crate::session_rules::melee_outcome_presentation_like_cpp(outcome, offhand);
    combat::RepresentedMeleeSwingLikeCpp {
        damage,
        original_damage,
        blocked,
        hit_info,
        victim_state,
    }
}

/// C++ `Unit::GetAPMultiplier(attType, normalized = false)` clamped by
/// `Player::CalculateMinMaxDamage`: the equipped delay in seconds, or the
/// two-second unarmed default, never below the 0.25 floor.
pub(in crate::session) fn legacy_attack_power_multiplier_like_cpp(
    base_attack_speed_ms: u32,
) -> f32 {
    if base_attack_speed_ms > 0 {
        (base_attack_speed_ms as f32 / 1000.0).max(0.25)
    } else {
        2.0
    }
}

/// C++ `Unit::MeleeDamageBonusDone`'s `(DoneFlatBenefit, DoneTotalMod)` pair the
/// swing owner computes for one attack type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::session) struct RepresentedMeleeDamageBonusLikeCpp {
    pub flat: i32,
    pub pct: f32,
}

impl RepresentedMeleeDamageBonusLikeCpp {
    pub(in crate::session) const NONE: Self = Self { flat: 0, pct: 1.0 };
}

pub(in crate::session) fn is_within_melee_range_like_cpp(
    attacker_position: Position,
    attacker_combat_reach: f32,
    target_position: Position,
    target_combat_reach: f32,
) -> bool {
    let melee_range = (attacker_combat_reach.max(0.0) + target_combat_reach.max(0.0) + 4.0 / 3.0)
        .max(NOMINAL_MELEE_RANGE_LIKE_CPP);
    attacker_position.distance(&target_position) <= melee_range
}

pub(in crate::session) fn is_within_target_boundary_radius_like_cpp(
    attacker_position: Position,
    attacker_combat_reach: f32,
    target_position: Position,
    target_combat_reach: f32,
    target_bounding_radius: f32,
) -> bool {
    let boundary_radius = target_bounding_radius.max(MIN_MELEE_REACH_LIKE_CPP)
        + attacker_combat_reach.max(0.0)
        + target_combat_reach.max(0.0);
    attacker_position.distance(&target_position) < boundary_radius
}

pub(in crate::session) fn is_unit_facing_target_for_melee_like_cpp(
    unit_position: Position,
    target_position: Position,
) -> bool {
    let dx = target_position.x - unit_position.x;
    let dy = target_position.y - unit_position.y;
    if dx.abs() <= f32::EPSILON && dy.abs() <= f32::EPSILON {
        return true;
    }

    let target_angle = dy.atan2(dx);
    let mut diff = (target_angle - unit_position.orientation).rem_euclid(std::f32::consts::TAU);
    if diff > std::f32::consts::PI {
        diff = std::f32::consts::TAU - diff;
    }
    diff <= std::f32::consts::PI / 3.0
}

/// How often the map-wide combat-reference sweep runs, per map, in the player
/// melee phase.
///
/// The session did this every combat tick — roughly every 100 ms
/// (`driver/mod.rs`, every second pass). The loop runs at the map update
/// interval, so calling it per tick would promote an O(combat units) map-wide
/// sweep from 10 Hz to 100 Hz. #28 preserves the cadence instead of the call
/// site.
pub(in crate::session) const PLAYER_MELEE_COMBAT_REF_REVALIDATE_INTERVAL_MS: u32 = 100;

/// Accumulated time per map key, so the sweep above keeps its cadence across
/// ticks. Owned by the loop task, not by any map guard.
#[derive(Debug, Default)]
pub struct PlayerMeleePhaseStateLikeCpp {
    pub(in crate::session) revalidate_accumulated_ms: HashMap<(u16, u32), u32>,
}

/// One attacker the tick owner will resolve this frame.
///
/// Built from the player registry before any map lock is taken, so the phase
/// never needs a session to know who is swinging.
#[derive(Clone, Debug)]
pub struct PlayerMeleeAttackerSnapshotLikeCpp {
    pub registration: crate::session::directory::PlayerRegistration,
    pub player_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    /// The attacker's published combat mirror, used to notice that a session
    /// still believes it is fighting a victim the map resolved away.
    pub in_combat_mirror: bool,
    pub tap_group_guids: Vec<ObjectGuid>,
}

/// One resolved victim, carried between the collect and execute phases with no
/// guard held.
#[derive(Clone, Debug)]
pub(in crate::session) struct PendingPlayerSwingLikeCpp {
    pub(in crate::session) attacker: PlayerMeleeAttackerSnapshotLikeCpp,
    pub(in crate::session) victim_guid: ObjectGuid,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyPlayerMeleeTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub attackers_seen: usize,
    pub maps_seen: usize,
    pub combat_ref_revalidations: usize,
    pub victims_resolved: usize,
    pub swings_ready: usize,
    pub creature_hits: usize,
    pub player_hits: usize,
    pub creature_kills: usize,
    pub victim_missing: usize,
    pub victim_not_alive: usize,
    pub attacker_unavailable: usize,
    pub canonical_mirror_rejections: usize,
    pub in_combat_reconciles: usize,
    pub commands: Vec<crate::session::mailbox::ApplyPlayerMeleeResultLikeCppCommand>,
}
