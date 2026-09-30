//! Shared loaded-grid Creature creation with owned postmaterialization rejection.

use super::*;

pub(crate) fn build_creature_respawn_records(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Result<
    Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp>,
    LoadedGridCreaturePreparationError,
> {
    let Some(respawn_time) = map
        .get_respawn_info_like_cpp(object_type, spawn_id)
        .map(|info| info.respawn_time)
    else {
        debug!(
            spawn_id,
            respawn_type = object_type as u8,
            "C++ loaded-grid Creature DoRespawn blocked: missing map-owned respawn timer before LoadFromDB"
        );
        return Ok(None);
    };
    build_creature_records_with_respawn_time(
        map,
        object_type,
        spawn_id,
        canonical_spawn_metadata,
        caches,
        respawn_time,
    )
}

pub(crate) fn build_creature_spawn_records(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Result<
    Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp>,
    LoadedGridCreaturePreparationError,
> {
    build_creature_records_with_respawn_time(
        map,
        object_type,
        spawn_id,
        canonical_spawn_metadata,
        caches,
        0,
    )
}

pub(crate) fn build_creature_records_with_respawn_time(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    respawn_time: i64,
) -> Result<
    Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp>,
    LoadedGridCreaturePreparationError,
> {
    if object_type != wow_map::SpawnObjectType::Creature {
        return Ok(None);
    }

    let Some(spawn) = canonical_spawn_metadata
        .spawn_store()
        .spawn_data(object_type, spawn_id)
    else {
        debug!(
            respawn_type = object_type as u8,
            spawn_id, "C++ loaded-grid Creature DoRespawn blocked: missing canonical SpawnData"
        );
        return Ok(None);
    };
    let Some(runtime_row) = canonical_spawn_metadata.creature_runtime_row_like_cpp(spawn_id) else {
        debug!(
            spawn_id,
            entry = spawn.id,
            "C++ loaded-grid Creature DoRespawn blocked: missing DB-backed creature runtime row"
        );
        return Ok(None);
    };
    let Ok(map_id) = u16::try_from(map.map_id()) else {
        warn!(
            map_id = map.map_id(),
            spawn_id,
            entry = spawn.id,
            "C++ loaded-grid Creature DoRespawn blocked: map id does not fit ObjectGuid world-object map field"
        );
        return Ok(None);
    };
    let difficulty_id = map.spawn_mode();
    let instance_id = map.instance_id();
    let formation_info = canonical_spawn_metadata
        .creature_formation_info_like_cpp(spawn_id)
        .copied();
    let mut random = MapCreatureModelSelectionRandomLikeCpp { map };
    let inputs =
        creature_loaded_grid::build_loaded_grid_creature_inputs_with_power_stores_from_db_like_cpp(
            spawn,
            runtime_row,
            caches.template_store.as_ref(),
            caches.difficulty_store.as_ref(),
            caches.base_stats_store.as_ref(),
            &caches.health_rates,
            caches.display_store.as_ref(),
            caches.model_store.as_ref(),
            caches.model_info_store.as_ref(),
            Some(caches.creature_equipment_store.as_ref()),
            caches.creature_addon_store.as_ref(),
            Some(caches.chr_classes_store.as_ref()),
            Some(caches.power_type_store.as_ref()),
            difficulty_id,
            instance_id,
            respawn_time,
            true,
            formation_info,
            &mut random,
        );
    let (template, resolved_spawn, runtime_selection) = match inputs {
        Ok(inputs) => inputs,
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                "C++ loaded-grid Creature DoRespawn blocked: failed to compose DB-backed LoadFromDB inputs"
            );
            return Ok(None);
        }
    };

    let low = match map.generate_low_guid_like_cpp(HighGuid::Creature) {
        Ok(low) => low,
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                "C++ loaded-grid Creature DoRespawn blocked: map-owned Creature low-guid generation failed"
            );
            return Ok(None);
        }
    };
    let mut template = template;
    creature_addon_provenance::resolve_addon_visuals_like_cpp(
        template.addon.as_mut(),
        caches.spell_x_spell_visual_store.as_ref(),
        difficulty_id,
    );
    template.sparring_health_pct = caches
        .sparring_store
        .values_for_entry_like_cpp(template.entry)
        .and_then(|values| {
            if values.is_empty() {
                None
            } else {
                let max = u32::try_from(values.len().saturating_sub(1)).unwrap_or(0);
                let index = map.urand_inclusive_like_cpp(0, max) as usize;
                values.get(index).copied()
            }
        });
    if let Some(vehicle_id) = template.vehicle_id {
        if let Some(vehicle_entry) = caches.vehicle_store.get(vehicle_id) {
            template.vehicle_kit_create_input = Some(wow_entities::VehicleKitCreateInputLikeCpp {
                vehicle_id,
                creature_entry: template.entry,
                loading: true,
                seat_defs: caches
                    .vehicle_seat_store
                    .seat_defs_for_vehicle_like_cpp(vehicle_entry),
            });
            template.add_to_world_vehicle_reset_context =
                Some(wow_entities::CreatureAddToWorldVehicleResetContextLikeCpp {
                    is_mechanical_creature: template.creature_type
                        == CREATURE_TYPE_MECHANICAL_LIKE_CPP,
                    is_world_boss: template.type_flags & CREATURE_TYPE_FLAG_BOSS_MOB_LIKE_CPP != 0,
                    accessories: caches
                        .vehicle_accessory_store
                        .accessories_for_vehicle_like_cpp(Some(spawn_id), template.entry)
                        .map(ToOwned::to_owned)
                        .unwrap_or_default(),
                });
        }
    }

    let map_object_high = if template.vehicle_id.is_some() {
        HighGuid::Vehicle
    } else {
        HighGuid::Creature
    };
    let map_object_guid = match map_object_high {
        HighGuid::Vehicle => {
            ObjectGuid::create_vehicle_like_cpp(caches.realm_id, map_id, template.entry, low)
        }
        HighGuid::Creature => {
            ObjectGuid::create_creature_like_cpp(caches.realm_id, map_id, template.entry, low)
        }
        _ => unreachable!("loaded-grid creature records only create Creature or Vehicle GUIDs"),
    };
    let resolver = creature_loaded_grid::CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template],
        [resolved_spawn],
        [(spawn.id, runtime_selection)],
    );
    creature_addon_provenance::settle_creature_record(
        map,
        resolver.resolve_creature_record(spawn_id, map_object_guid),
        spawn_id,
        spawn.id,
        map_object_guid,
    )
}

#[cfg(test)]
mod tests;
