// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented transmog-set, kill-credit, spell-charge and raid-marker effects
//! (#1363 slice 9d), from the World `session/spell_effects/{effects,effects_power,effects_progress}.rs`
//! and `session/spell_state/catalog.rs`.

use super::SpellEffectHostLikeCpp;
use crate::SpellbookHostLikeCpp;
use tracing::debug;
use wow_core::ObjectGuid;
use wow_packet::packets::spell::SpellTargetData;
use wow_world_core::entity_update_bridge::player_values_update_to_update_object;

pub fn apply_learn_transmog_set_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    target_guid: ObjectGuid,
    transmog_set_id: i32,
) {
    let Some(player_guid) = host.player_guid() else {
        return;
    };
    if target_guid != player_guid {
        return;
    }

    let Ok(transmog_set_id) = u32::try_from(transmog_set_id) else {
        return;
    };
    let Some(update) = host.add_transmog_set_like_cpp(transmog_set_id) else {
        return;
    };
    if let Some(packet) = player_values_update_to_update_object(
        player_guid,
        host.core().player_map_id_like_cpp(),
        &update,
    ) {
        host.send_packet(&packet);
    }
}

pub async fn apply_kill_credit_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    target_guid: ObjectGuid,
    creature_entry: i32,
    group_reward_like_cpp: bool,
) -> Result<(), &'static str> {
    let player_guid = host.player_guid().ok_or("No player GUID")?;
    if target_guid != player_guid {
        return Ok(());
    }

    let Ok(creature_entry) = u32::try_from(creature_entry) else {
        debug!(
            account = host.core().account_id,
            "Skipping represented kill-credit spell effect with negative MiscValue"
        );
        return Ok(());
    };
    if creature_entry == 0 {
        return Ok(());
    }

    if group_reward_like_cpp {
        debug!(
            account = host.core().account_id,
            creature_entry,
            "Represented SPELL_EFFECT_KILL_CREDIT2 applies current-session credit only; C++ group fanout remains unrepresented"
        );
    }
    host.on_creature_killed_with_generator_like_cpp(
        item_guid_generator,
        creature_entry,
        ObjectGuid::EMPTY,
    )
    .await;
    Ok(())
}

/// C++ `Spell::EffectModifySpellCharges`.
///
/// C++ restores one consumed charge per positive `damage` step on
/// `effectInfo->MiscValue`. Packet fanout for `SendSetSpellCharges` is
/// still outside this represented seam.
pub fn apply_modify_spell_charges_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    damage: i32,
    charge_category_id: i32,
    target_guid: ObjectGuid,
) -> u32 {
    let Some(player_guid) = host.player_guid() else {
        return 0;
    };
    if target_guid != player_guid || damage <= 0 {
        return 0;
    }
    let Ok(charge_category_id) = u32::try_from(charge_category_id) else {
        return 0;
    };
    if charge_category_id == 0 {
        return 0;
    }

    let restored = host
        .core()
        .mutate_canonical_player_like_cpp(|player| {
            let history = &mut player.unit_mut().subsystems_mut().spells.history;
            let mut restored = 0;
            for _ in 0..damage {
                if history.restore_charge(charge_category_id) {
                    restored += 1;
                }
            }
            restored
        })
        .unwrap_or(0);

    let mut represented_restored = 0;
    for _ in 0..damage {
        if restore_represented_character_spell_charge_like_cpp(host, charge_category_id) {
            represented_restored += 1;
        }
    }

    restored.max(represented_restored)
}

pub fn restore_represented_character_spell_charge_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    category_id: u32,
) -> bool {
    let (state, mut hub) = host.split_spell_state_mut();
    state.restore_represented_character_spell_charge_like_cpp(&mut hub, category_id)
}

/// C++ `Spell::EffectChangeRaidMarker`.
///
/// Represented boundary: current in-memory HOME group only. This mirrors
/// `Group::AddRaidMarker` storage and `SMSG_RAID_MARKERS_CHANGED` fanout to
/// connected represented members. Full DB persistence, instance/original
/// group category routing and `CMSG_CLEAR_RAID_MARKER` remain group-runtime
/// follow-ups.
pub fn apply_change_raid_marker_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    damage: i32,
    target_data: &SpellTargetData,
) -> bool {
    let Some(player_guid) = host.player_guid() else {
        return false;
    };
    let Some(destination) = target_data.dst_location.as_ref() else {
        return false;
    };
    let Some(group_guid) = host.resolved_group_guid_like_cpp() else {
        return false;
    };
    let Some(group_registry) = host.core().group_registry().cloned() else {
        return false;
    };

    let outcome = match group_registry.add_raid_marker_transition_like_cpp(
        group_guid,
        player_guid,
        damage as u8,
        u32::from(host.core().player_map_id_like_cpp()),
        destination.position,
        destination.transport,
    ) {
        Ok(outcome) => outcome,
        Err(_) => return false,
    };
    let packet_bytes_and_members = {
        let group = outcome.group;
        use wow_packet::ServerPacket;
        let packet = wow_packet::packets::party::RaidMarkersChanged {
            party_index: group.group_category_like_cpp(),
            active_markers: group.active_raid_markers_mask_like_cpp(),
            raid_markers: group
                .raid_marker_list_like_cpp()
                .into_iter()
                .map(|marker| wow_packet::packets::party::RaidMarker {
                    transport_guid: marker.transport_guid,
                    map_id: marker.map_id,
                    position: marker.position,
                })
                .collect(),
        }
        .to_bytes();
        (packet, group.members)
    };

    let (packet_bytes, members) = packet_bytes_and_members;
    let mut sent_to_self_via_registry = false;
    if let Some(registry) = host.core().player_registry().cloned() {
        for member_guid in members {
            let Some(member) = registry.group_presence(member_guid) else {
                continue;
            };
            if member_guid == player_guid {
                sent_to_self_via_registry = true;
            }
            let _ = registry.try_send_current_packet(member.registration, packet_bytes.clone());
        }
    }
    if !sent_to_self_via_registry {
        host.send_raw_packet(&packet_bytes);
    }
    true
}
