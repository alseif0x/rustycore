//! Secondary-target split and share damage for the legacy creature melee tick.
//!
//! Split out of `creature_melee_tick` as behavior-preserving structure: the
//! caller keeps the C++ map-update serialization, the established
//! canonical -> legacy lock order and the delivery of the terms resolved here.

use super::*;

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
    pub(super) split_mutation_events: Vec<RuntimeEvent>,
    pub(super) split_combat_log_packets: Vec<Vec<u8>>,
    pub(super) share_mutation_events: Vec<RuntimeEvent>,
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
    canonical_manager: &mut wow_map::MapManager,
    attacker: &crate::map_manager::WorldCreature,
    swing: &PendingCreatureSwingLikeCpp,
    config: &crate::session::LegacyCreatureAggroConfigLikeCpp,
    damage: u32,
    state: &mut MeleeSwingStateLikeCpp,
    creature_victim_syncs: &mut Vec<CreatureVictimCompatibilitySyncLikeCpp>,
) -> MeleeSecondaryTargetsOutcomeLikeCpp {
    let mut split_mutation_events = Vec::new();
    let mut split_combat_log_packets = Vec::new();
    let mut share_mutation_events = Vec::new();
    let mut primary_was_share_target = false;
    let mut primary_player_share_health_updates = Vec::new();
    let damage = if damage > 0 {
        match config.spell_store.as_deref() {
            Some(spell_store) => {
                let map_difficulty_id = canonical_manager
                    .find_map(u32::from(swing.map_id), swing.instance_id)
                    .map(|managed| managed.difficulty())
                    .unwrap_or(0);
                let split = super::super::creature_melee_split::apply_melee_split_damage_like_cpp(
                    canonical_manager,
                    swing.map_id,
                    swing.instance_id,
                    swing.attacker_guid,
                    swing.victim_guid,
                    damage,
                    0x01,
                    attacker
                        .creature
                        .is_charmed_owned_by_player_or_player_like_cpp(),
                    spell_store,
                    config.spell_misc_store.as_deref(),
                    config.spell_threat_store.as_deref(),
                    config.spell_chain_store.as_deref(),
                    map_difficulty_id,
                    config.difficulty_store.as_deref(),
                );
                if split.absorbed > 0 {
                    state.absorbed_damage = state.absorbed_damage.saturating_add(split.absorbed);
                    state.hit_info &= !(wow_packet::packets::combat::HIT_INFO_FULL_ABSORB
                        | wow_packet::packets::combat::HIT_INFO_PARTIAL_ABSORB);
                    state.hit_info |= if split.damage == 0 {
                        wow_packet::packets::combat::HIT_INFO_FULL_ABSORB
                    } else {
                        wow_packet::packets::combat::HIT_INFO_PARTIAL_ABSORB
                    };
                    if let Some((info, _, _)) = state.creature_victim_presentation.as_mut() {
                        *info &= !(wow_packet::packets::combat::HIT_INFO_FULL_ABSORB
                            | wow_packet::packets::combat::HIT_INFO_PARTIAL_ABSORB);
                        *info |= if split.damage == 0 {
                            wow_packet::packets::combat::HIT_INFO_FULL_ABSORB
                        } else {
                            wow_packet::packets::combat::HIT_INFO_PARTIAL_ABSORB
                        };
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
            None => damage,
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
                            attacker
                                .creature
                                .is_charmed_owned_by_player_or_player_like_cpp(),
                            damage,
                        )
                    })
            })
            .unwrap_or(damage)
    } else {
        damage
    };
    if represented_damage_done > 0
        && let Some(spell_store) = config.spell_store.as_deref()
    {
        let map_difficulty_id = canonical_manager
            .find_map(u32::from(swing.map_id), swing.instance_id)
            .map(|managed| managed.difficulty())
            .unwrap_or(0);
        let share = super::super::creature_melee_share::apply_melee_share_damage_like_cpp(
            canonical_manager,
            swing.map_id,
            swing.instance_id,
            swing.attacker_guid,
            swing.victim_guid,
            represented_damage_done,
            0x01,
            attacker
                .creature
                .is_charmed_owned_by_player_or_player_like_cpp(),
            spell_store,
            config.spell_misc_store.as_deref(),
            config.spell_threat_store.as_deref(),
            config.spell_chain_store.as_deref(),
            map_difficulty_id,
            config.difficulty_store.as_deref(),
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
