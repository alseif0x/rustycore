// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map-owned collection of pending selected Ghost/Wisp aura deltas.

use super::aura_publication::player_aura_info_with_provenance;
use wow_entities::{AuraApplicationLikeCpp as AuraApplication, AuraCastProvenanceLikeCpp};
use wow_map::{MapKey, MapManager, MapTickCoordinationStateLikeCpp, MapTickPlanLikeCpp, PlayerHandle};
use wow_world_core::session::{PreparedSelectedAuraUpdate, prepare_selected_aura_update};

#[cfg(test)]
#[path = "selected_publication_tests.rs"]
mod tests;

#[derive(Debug)]
struct PendingPlayerAuras {
    guid: wow_core::ObjectGuid,
    handle: PlayerHandle,
    map_key: MapKey,
    level: u8,
    slots: Vec<(u8, AuraApplication, AuraCastProvenanceLikeCpp)>,
}

/// Prepare and consume selected aura deltas for the maps admitted by one
/// canonical map tick. The caller holds MapManager throughout; no directory
/// access, packet delivery or await occurs here.
pub fn collect_pending_selected_aura_updates(
    manager: &mut MapManager,
    plan: &MapTickPlanLikeCpp,
) -> Vec<PreparedSelectedAuraUpdate> {
    if manager.tick_coordination_like_cpp()
        != MapTickCoordinationStateLikeCpp::AwaitingSessions(plan.epoch_like_cpp())
    {
        return Vec::new();
    }

    let mut pending_players = Vec::new();
    for participant in plan.updated_maps_like_cpp() {
        if manager.map_incarnation_like_cpp(participant.key) != Some(participant.incarnation) {
            continue;
        }
        let Some(map) = manager
            .find_map(participant.key.map_id, participant.key.instance_id)
            .map(|managed| managed.map())
        else {
            continue;
        };
        let player_guids = map
            .object_update_visit_plan_for_current_tick(plan.effective_diff_ms())
            .player_update_guids;
        for guid in player_guids {
            let Some((handle, admitted_revision)) =
                manager.current_player_admission_like_cpp(guid)
            else {
                continue;
            };
            if manager.player_active_residence_revision_like_cpp(handle)
                != Some((participant.key, admitted_revision))
            {
                continue;
            }
            let Some(snapshot) = manager.with_player_like_cpp(handle, |player| {
                if player.guid() != guid
                    || !player.unit().world().object().is_in_world()
                    || player.unit().world().map_id() != participant.key.map_id
                    || player.unit().world().instance_id() != participant.key.instance_id
                {
                    return None;
                }
                let level = u8::try_from(player.unit().data().level).ok()?;
                let auras = &player.unit().subsystems().auras;
                let mut slots = auras
                    .visible_auras_to_update
                    .iter()
                    .copied()
                    .filter_map(|slot| {
                        let aura = auras.runtime_application_like_cpp(slot)?;
                        matches!(aura.spell_id, 8326 | 20584).then(|| {
                            (
                                slot,
                                aura.clone(),
                                auras.aura_cast_provenance_like_cpp(slot),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                slots.sort_unstable_by_key(|(slot, _, _)| *slot);
                (!slots.is_empty()).then_some(PendingPlayerAuras {
                    guid,
                    handle,
                    map_key: participant.key,
                    level,
                    slots,
                })
            })
            .flatten()
            else {
                continue;
            };
            pending_players.push(snapshot);
        }
    }

    let mut prepared = Vec::new();
    for pending in pending_players {
        let mut player_updates = Vec::with_capacity(pending.slots.len());
        let mut all_prepared = true;
        for (_, aura, provenance) in &pending.slots {
            let aura_info = player_aura_info_with_provenance(
                aura,
                pending.level,
                pending.map_key.map_id as u16,
                Some(*provenance),
            );
            let packet = wow_packet::packets::misc::AuraUpdate {
                unit_guid: pending.guid,
                update_all: false,
                auras: vec![aura_info],
            };
            let Some(update) = prepare_selected_aura_update(
                manager,
                pending.handle,
                wow_packet::ServerPacket::to_bytes(&packet),
            ) else {
                all_prepared = false;
                break;
            };
            player_updates.push(update);
        }
        if !all_prepared {
            continue;
        }

        let slots = &pending.slots;
        let cleared = manager.with_player_mut_like_cpp(pending.handle, |player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            if slots.iter().any(|(slot, expected, provenance)| {
                !auras.visible_auras_to_update.contains(slot)
                    || auras.runtime_application_like_cpp(*slot) != Some(expected)
                    || auras.aura_cast_provenance_like_cpp(*slot) != *provenance
            }) {
                return false;
            }
            for (slot, _, _) in slots {
                auras.visible_auras_to_update.remove(slot);
            }
            true
        });
        if cleared != Some(true) {
            continue;
        }
        prepared.extend(player_updates);
    }

    prepared
}
