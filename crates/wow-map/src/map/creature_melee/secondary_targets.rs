//! Secondary-target split and share damage for the legacy creature melee tick.
//!
//! Split out of `creature_melee_tick` as behavior-preserving structure: the
//! caller keeps the C++ map-update serialization, the established
//! canonical -> legacy lock order and the delivery of the terms resolved here.

use super::source::CreatureMeleeSource;
use super::*;
use crate::manager::MeleeKillCollector;

/// The split/share terms the tick's loop needs after the secondary-target
/// phase: the damage the primary apply commits and the publications its
/// delivery owns.
#[derive(Default)]
pub(super) struct MeleeSecondaryTargetsOutcomeLikeCpp {
    pub(super) damage: u32,
    /// C++ sends `AttackerStateUpdate` before the share loop's secondary
    /// mutation, so the primary keeps its pre-share health for wire overkill
    /// when the share aura's caster is the primary victim itself.
    pub(super) primary_wire_health_before: Option<u64>,
    pub(super) represented_damage_done: u32,
    pub(super) split_mutation_events: Vec<MeleeEffect>,
    pub(super) split_combat_log_packets: Vec<MeleeEffect>,
    pub(super) share_mutation_events: Vec<MeleeEffect>,
    pub(super) primary_was_share_target: bool,
    pub(super) primary_player_share_health_updates: Vec<u64>,
}

/// C++ `Unit::CalcAbsorbResist`'s split tail (`Unit.cpp:1958-2015`) and
/// `Unit::DealDamage`'s share loop (`Unit.cpp:833-856`) for one represented
/// white swing, followed by the primary's pre-share wire health and the
/// `damageDone` the primary apply commits.
///
/// Runs against the canonical map the caller holds locked in its established
/// canonical -> legacy order and appends the secondary compatibility syncs to
/// the caller's list, so the replay at the end of the tick keeps C++'s order.
pub(super) fn apply_secondary_targets_damage_like_cpp(
    canonical_manager: &mut MapManager,
    source: &CreatureMeleeSource<'_>,
    swing: &PendingCreatureSwingLikeCpp,
    catalogs: &impl CreatureMeleeCatalogsLikeCpp,
    damage: u32,
    state: &mut MeleeSwingStateLikeCpp,
    creature_victim_syncs: &mut Vec<CreatureVictimCompatibilitySyncLikeCpp>,
    kills: &mut Option<MeleeKillCollector<'_>>,
) -> MeleeSecondaryTargetsOutcomeLikeCpp {
    let mut split_mutation_events = Vec::new();
    let mut split_combat_log_packets = Vec::new();
    let mut share_mutation_events = Vec::new();
    let mut primary_was_share_target = false;
    let mut primary_player_share_health_updates = Vec::new();
    let damage = if damage > 0 {
        match catalogs.represented() {
            true => {
                let map_difficulty_id = canonical_manager
                    .find_map(u32::from(swing.map_id), swing.instance_id)
                    .map(|managed| managed.difficulty())
                    .unwrap_or(0);
                let attacker_is_player_controlled = source.is_player_controlled(canonical_manager);
                let split = super::split::apply_melee_split_damage_like_cpp(
                    canonical_manager,
                    swing.map_id,
                    swing.instance_id,
                    swing.attacker_guid,
                    swing.victim_guid,
                    damage,
                    0x01,
                    attacker_is_player_controlled,
                    catalogs,
                    map_difficulty_id,
                    kills,
                );
                if split.absorbed > 0 {
                    state.absorbed_damage = state.absorbed_damage.saturating_add(split.absorbed);
                    state.hit_info.replace_absorb(split.damage);
                    if let Some((info, _)) = state.creature_victim_presentation.as_mut() {
                        info.replace_absorb(split.damage);
                    }
                }
                split_mutation_events = split.mutation_events;
                split_combat_log_packets = split.combat_log_packets;
                for (split_victim_guid, state) in split.creature_syncs {
                    let mut split_swing = *swing;
                    split_swing.victim_guid = split_victim_guid;
                    creature_victim_syncs.push(CreatureVictimCompatibilitySyncLikeCpp {
                        swing: split_swing,
                        state,
                    });
                }
                split.damage
            }
            false => damage,
        }
    } else {
        damage
    };
    // C++ sends `AttackerStateUpdate` before entering `DealDamage`, whose
    // share loop mutates secondary targets before the primary health write.
    // Preserve the pre-share health for wire overkill if the aura caster
    // is the primary victim itself.
    let primary_wire_health_before = canonical_manager
        .find_map(u32::from(swing.map_id), swing.instance_id)
        .and_then(|managed| {
            if swing.victim_guid.is_player() {
                managed
                    .map()
                    .get_typed_player(swing.victim_guid)
                    .map(|victim| victim.unit().data().health)
            } else {
                managed
                    .map()
                    .with_creature_like_cpp(swing.victim_guid, |victim| victim.unit().data().health)
            }
        });
    let represented_damage_done = if damage > 0 && !swing.victim_guid.is_player() {
        canonical_manager
            .find_map(u32::from(swing.map_id), swing.instance_id)
            .and_then(|managed| {
                managed
                    .map()
                    .with_creature_like_cpp(swing.victim_guid, |victim| {
                        victim.calculate_damage_for_sparring_like_cpp(
                            true,
                            source.is_player_controlled(canonical_manager),
                            damage,
                        )
                    })
            })
            .unwrap_or(damage)
    } else {
        damage
    };
    if represented_damage_done > 0 && catalogs.represented() {
        let map_difficulty_id = canonical_manager
            .find_map(u32::from(swing.map_id), swing.instance_id)
            .map(|managed| managed.difficulty())
            .unwrap_or(0);
        let attacker_is_player_controlled = source.is_player_controlled(canonical_manager);
        let share = super::share::apply_melee_share_damage_like_cpp(
            canonical_manager,
            swing.map_id,
            swing.instance_id,
            swing.attacker_guid,
            swing.victim_guid,
            represented_damage_done,
            0x01,
            attacker_is_player_controlled,
            catalogs,
            map_difficulty_id,
            kills,
        );
        share_mutation_events = share.mutation_events;
        primary_was_share_target = share.primary_was_share_target;
        primary_player_share_health_updates = share.primary_player_share_health_updates;
        for (share_victim_guid, state) in share.creature_syncs {
            let mut share_swing = *swing;
            share_swing.victim_guid = share_victim_guid;
            creature_victim_syncs.push(CreatureVictimCompatibilitySyncLikeCpp {
                swing: share_swing,
                state,
            });
        }
    }
    MeleeSecondaryTargetsOutcomeLikeCpp {
        damage,
        primary_wire_health_before,
        represented_damage_done,
        split_mutation_events,
        split_combat_log_packets,
        share_mutation_events,
        primary_was_share_target,
        primary_player_share_health_updates,
    }
}
