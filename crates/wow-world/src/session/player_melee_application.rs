// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player melee application: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::combat;
use super::{ATTACK_DISPLAY_DELAY_LIKE_CPP_MS, HashMap, MIN_MELEE_REACH_LIKE_CPP};
use super::{NOMINAL_MELEE_RANGE_LIKE_CPP, ObjectGuid, Position, WeaponAttackType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerAttackStartLikeCppResult {
    Rejected,
    Accepted { send_attack_start: bool },
}

/// One C++ `Unit::DoMeleeAttackIfReady` pass over a canonical player.
///
/// Shared by the Session and global legacy loop on the same canonical Player.
/// Player owns readiness and timer mutations; this adapter resolves the
/// original damage/RNG pipeline at each callback before its timer reset.
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
    player.take_ready_melee_attacks(
        diff_ms,
        in_melee_range,
        facing_target,
        within_los,
        ATTACK_DISPLAY_DELAY_LIKE_CPP_MS,
        |attack_type, [min_damage, max_damage], autoattack_damage_multiplier| {
            let offhand = attack_type == WeaponAttackType::OffAttack;
            represented_white_swing_damage_like_cpp(
                min_damage,
                max_damage,
                autoattack_damage_multiplier,
                melee_damage_bonus[usize::from(offhand)],
                armor_mitigation,
                outcome_facts,
                damage_taken,
                offhand,
            )
        },
    )
}

/// C++ `CombatManager::SetInCombatWith` for a player attacker, on an already
/// locked map.
///
/// Shared by the Session and global legacy loop. Map owns admission and the
/// reciprocal writes; each caller retains its existing lock acquisition.
pub(in crate::session) fn begin_combat_ref_on_map_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    attacker_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    relation_represented: bool,
    attacker_is_friendly_to_victim: bool,
    victim_is_friendly_to_attacker: bool,
) -> bool {
    map.begin_player_combat_ref(
        attacker_guid,
        victim_guid,
        relation_represented,
        attacker_is_friendly_to_victim,
        victim_is_friendly_to_attacker,
    )
}

/// Apply one player's melee swings to a canonical player victim.
///
/// Player owns the ordered health mutations and represented over-damage result.
/// This adapter projects resolved damages lazily; both callers retain victim
/// resolution, their existing locks and publication after mutation. The domain
/// method documents the preserved C++ admission, death and stage-order gaps.
pub(in crate::session) fn apply_player_melee_to_canonical_player_like_cpp(
    victim: &mut wow_entities::Player,
    swings: &[combat::RepresentedMeleeSwingLikeCpp],
) -> Option<(Vec<(u32, i32)>, u8)> {
    victim.apply_melee_damage_batch(swings.iter().map(|swing| swing.damage))
}

/// What one player's melee pass did to a legacy creature.
///
/// `move_stop` carries the stop position and spline id rather than serialised
/// bytes: whoever owns the tick applies the transition, and the session that
/// owns the receiver builds the packet. Keeping construction at the session is
/// what makes the bytes identical — `MonsterMoveStop` is viewer-independent,
/// but the values update beside it is not (#28).
pub(crate) use wow_map::PlayerMeleeCreatureHit as PlayerMeleeCreatureHitLikeCpp;

/// Module-level so the global legacy loop, which has no session, shares the
/// same arithmetic as the session owner (#28). Combat owns the represented
/// `CalculateMeleeDamage` pipeline; this adapter supplies the original RNG at
/// each stage and retains packet outcome mapping and the World swing DTO.
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
    let (outcome, (damage, blocked, original_damage)) = wow_combat::calculate_white_swing(
        [min_damage, max_damage],
        autoattack_damage_multiplier,
        (melee_damage_bonus.flat, melee_damage_bonus.pct),
        armor_mitigation,
        (&outcome_facts.0, &outcome_facts.1),
        damage_taken,
        offhand,
        wow_core::urand_like_cpp,
        wow_core::urand_like_cpp,
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
