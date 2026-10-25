// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical preparation of pending selected Player aura deltas.

use super::aura_publication::player_aura_info_with_provenance;
use wow_data::character_progression::ChrRacesStore;
use wow_entities::{AuraApplicationLikeCpp as AuraApplication, AuraCastProvenanceLikeCpp};
use wow_map::{
    MapKey, MapManager, MapTickCoordinationStateLikeCpp, MapTickPlanLikeCpp, PlayerHandle,
};
use wow_world_core::session::{PreparedSelectedAuraUpdate, prepare_selected_aura_update};

#[cfg(test)]
#[path = "selected_publication_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/selected_publication.rs"]
mod player_tests;

#[derive(Debug)]
struct PendingPlayerAuras {
    guid: wow_core::ObjectGuid,
    handle: PlayerHandle,
    map_key: MapKey,
    map_incarnation: u64,
    residence_revision: u64,
    level: u8,
    slots: Vec<(u8, AuraApplication, AuraCastProvenanceLikeCpp)>,
}

/// Prepare this Player's selected visible updates under the caller's manager
/// guard. C++ `Unit::_UpdateSpells` visits visible applications in slot order
/// before clearing its pending set (`Unit.cpp:2872-2875`, `Unit.h:638-639`).
/// This operation freezes every frame before consuming slots for later delivery.
pub fn collect_player_pending_selected_aura_updates(
    manager: &mut MapManager,
    handle: PlayerHandle,
    races: Option<&ChrRacesStore>,
) -> Result<Vec<PreparedSelectedAuraUpdate>, &'static str> {
    let (map_key, residence_revision) =
        manager
            .player_active_residence_revision_like_cpp(handle)
            .ok_or("selected aura source has no current active residence")?;
    let map_incarnation = manager
        .map_incarnation_like_cpp(map_key)
        .ok_or("selected aura source map is unavailable")?;
    let pending = manager
        .with_player_like_cpp(handle, |player| {
            let world = player.unit().world();
            if player.guid() != handle.guid()
                || !world.object().is_in_world()
                || !world.has_current_map()
                || MapKey::new(world.map_id(), world.instance_id()) != map_key
            {
                return Err("selected aura source identity is not current");
            }
            let auras = &player.unit().subsystems().auras;
            if auras.visible_auras_to_update.is_empty() {
                return Ok(None);
            }
            let sickness = races
                .ok_or("selected aura source race catalog is unavailable")?
                .get(u32::from(player.race_like_cpp()))
                .ok_or("selected aura source race is unavailable")?
                .res_sickness_spell_id;
            if sickness < 0 {
                return Err("selected aura source sickness spell is invalid");
            }
            let mut slots = Vec::new();
            for slot in auras.visible_auras_to_update.iter().copied() {
                let aura = auras
                    .runtime_application_like_cpp(slot)
                    .ok_or("pending aura application is unavailable")?;
                if matches!(aura.spell_id, 8326 | 20584)
                    || (sickness != 0 && aura.spell_id == sickness)
                {
                    if aura.slot != slot {
                        return Err("pending selected aura slot is inconsistent");
                    }
                    slots.push((
                        slot,
                        aura.clone(),
                        auras.aura_cast_provenance_like_cpp(slot),
                    ));
                }
            }
            if slots.is_empty() {
                return Ok(None);
            }
            slots.sort_unstable_by_key(|(slot, _, _)| *slot);
            let level = u8::try_from(player.unit().data().level)
                .map_err(|_| "selected aura source level is invalid")?;
            Ok(Some(PendingPlayerAuras {
                guid: handle.guid(),
                handle,
                map_key,
                map_incarnation,
                residence_revision,
                level,
                slots,
            }))
        })
        .ok_or("selected aura source is unavailable")??;
    match pending {
        Some(pending) => prepare_and_clear_pending_player_auras(manager, &pending),
        None => Ok(Vec::new()),
    }
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
            let Some((handle, admitted_revision)) = manager.current_player_admission_like_cpp(guid)
            else {
                continue;
            };
            if manager.player_active_residence_revision_like_cpp(handle)
                != Some((participant.key, admitted_revision))
            {
                continue;
            }
            let Some(snapshot) = manager
                .with_player_like_cpp(handle, |player| {
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
                        map_incarnation: participant.incarnation,
                        residence_revision: admitted_revision,
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
        if let Ok(updates) = prepare_and_clear_pending_player_auras(manager, &pending) {
            prepared.extend(updates);
        }
    }
    prepared
}

fn prepare_and_clear_pending_player_auras(
    manager: &mut MapManager,
    pending: &PendingPlayerAuras,
) -> Result<Vec<PreparedSelectedAuraUpdate>, &'static str> {
    let mut prepared = Vec::with_capacity(pending.slots.len());
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
        let update = prepare_selected_aura_update(
            manager,
            pending.handle,
            wow_packet::ServerPacket::to_bytes(&packet),
        )
        .ok_or("selected aura publication preparation failed")?;
        prepared.push(update);
    }

    if manager.map_incarnation_like_cpp(pending.map_key) != Some(pending.map_incarnation)
        || manager.player_active_residence_revision_like_cpp(pending.handle)
            != Some((pending.map_key, pending.residence_revision))
    {
        return Err("selected aura source binding changed before pending clear");
    }
    let cleared = manager.with_player_mut_like_cpp(pending.handle, |player| {
        let world = player.unit().world();
        if player.guid() != pending.guid
            || !world.object().is_in_world()
            || !world.has_current_map()
            || MapKey::new(world.map_id(), world.instance_id()) != pending.map_key
        {
            return false;
        }
        let auras = &mut player.unit_mut().subsystems_mut().auras;
        if pending.slots.iter().any(|(slot, expected, provenance)| {
            !auras.visible_auras_to_update.contains(slot)
                || auras.runtime_application_like_cpp(*slot) != Some(expected)
                || auras.aura_cast_provenance_like_cpp(*slot) != *provenance
        }) {
            return false;
        }
        for (slot, _, _) in &pending.slots {
            auras.visible_auras_to_update.remove(slot);
        }
        true
    });
    if cleared != Some(true) {
        return Err("selected aura application changed before pending clear");
    }
    Ok(prepared)
}
