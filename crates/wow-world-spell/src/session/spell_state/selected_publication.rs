// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical preparation of pending selected Player aura deltas.

use super::selected_catalog::{
    SelectedAuraPublicationCatalogs, is_selected_aura_spell, selected_aura_wire_facts,
    sickness_spell_for_race,
};
use super::selected_wire::selected_aura_info;
use wow_entities::{
    AuraApplicationLikeCpp as AuraApplication, AuraCastProvenanceLikeCpp, AuraPersistenceMetadata,
    AuraThreatSnapshotLikeCpp,
};
use wow_map::{MapKey, MapManager, PlayerHandle};
use wow_world_core::session::{PreparedSelectedAuraUpdate, prepare_selected_aura_update};

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
    slots: Vec<(u8, FrozenSelectedAura)>,
}

#[derive(Debug)]
struct FrozenSelectedAura {
    application: AuraApplication,
    provenance: AuraCastProvenanceLikeCpp,
    snapshot: AuraThreatSnapshotLikeCpp,
    persistence: Option<AuraPersistenceMetadata>,
}

/// Prepare this Player's selected visible updates under the caller's manager
/// guard. C++ `Unit::_UpdateSpells` visits visible applications in slot order
/// before clearing its pending set (`Unit.cpp:2872-2875`, `Unit.h:638-639`).
/// This operation freezes every frame before consuming slots for later delivery.
pub fn collect_player_pending_selected_aura_updates(
    manager: &mut MapManager,
    handle: PlayerHandle,
    catalogs: SelectedAuraPublicationCatalogs<'_>,
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
            let sickness = sickness_spell_for_race(catalogs, player.race_like_cpp())?;
            let sickness_spell = (sickness > 0).then_some(sickness);
            let mut slots = Vec::new();
            for slot in auras.visible_auras_to_update.iter().copied() {
                let application = auras
                    .runtime_application_like_cpp(slot)
                    .ok_or("pending aura application is unavailable")?;
                if is_selected_aura_spell(application.spell_id, sickness) {
                    if application.slot != slot {
                        return Err("pending selected aura slot is inconsistent");
                    }
                    crate::validate_selected_aura_application(
                        auras,
                        application.spell_id,
                        slot,
                        sickness_spell,
                    )?;
                    let snapshot = auras
                        .threat_snapshot_like_cpp(slot)
                        .cloned()
                        .ok_or("selected aura effect snapshot is unavailable")?;
                    slots.push((
                        slot,
                        FrozenSelectedAura {
                            application: application.clone(),
                            provenance: auras.aura_cast_provenance_like_cpp(slot),
                            snapshot,
                            persistence: auras.aura_persistence_metadata(slot).cloned(),
                        },
                    ));
                }
            }
            if slots.is_empty() {
                return Ok(None);
            }
            slots.sort_unstable_by_key(|(slot, _)| *slot);
            Ok(Some(PendingPlayerAuras {
                guid: handle.guid(),
                handle,
                map_key,
                map_incarnation,
                residence_revision,
                slots,
            }))
        })
        .ok_or("selected aura source is unavailable")??;
    match pending {
        Some(pending) => prepare_and_clear_pending_player_auras(manager, &pending, catalogs),
        None => Ok(Vec::new()),
    }
}

fn prepare_and_clear_pending_player_auras(
    manager: &mut MapManager,
    pending: &PendingPlayerAuras,
    catalogs: SelectedAuraPublicationCatalogs<'_>,
) -> Result<Vec<PreparedSelectedAuraUpdate>, &'static str> {
    let mut prepared = Vec::with_capacity(pending.slots.len());
    for (_, frozen) in &pending.slots {
        let facts = selected_aura_wire_facts(
            &frozen.application,
            &frozen.provenance,
            frozen.persistence.as_ref(),
            catalogs,
        )?;
        let aura_info = selected_aura_info(
            &frozen.application,
            &frozen.provenance,
            &frozen.snapshot,
            facts,
        )?;
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
        if pending.slots.iter().any(|(slot, expected)| {
            !auras.visible_auras_to_update.contains(slot)
                || auras.runtime_application_like_cpp(*slot) != Some(&expected.application)
                || auras.aura_cast_provenance_like_cpp(*slot) != expected.provenance
                || auras.threat_snapshot_like_cpp(*slot) != Some(&expected.snapshot)
                || auras.aura_persistence_metadata(*slot) != expected.persistence.as_ref()
        }) {
            return false;
        }
        for (slot, _) in &pending.slots {
            auras.visible_auras_to_update.remove(slot);
        }
        true
    });
    if cleared != Some(true) {
        return Err("selected aura application changed before pending clear");
    }
    Ok(prepared)
}
