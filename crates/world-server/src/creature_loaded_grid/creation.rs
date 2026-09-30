//! Single loaded-grid Creature creator and its Record projections.

use super::*;

struct CreatedLoadedGridCreature {
    lifecycle_record: CreatureLoadFromDbLifecycleRecord,
    creature: Creature,
    map_insertion_requested: bool,
}

impl CreatureLoadedGridLifecycleResolverLikeCpp {
    fn resolve_creature(
        &self,
        spawn_id: u64,
        map_object_guid: ObjectGuid,
    ) -> Result<CreatedLoadedGridCreature, CreatureLoadedGridResolveErrorLikeCpp> {
        let spawn = self
            .spawns
            .get(&spawn_id)
            .ok_or(CreatureLoadedGridResolveErrorLikeCpp::MissingSpawnData { spawn_id })?;
        let template = self
            .templates
            .get(&spawn.entry)
            .ok_or(CreatureLoadedGridResolveErrorLikeCpp::MissingTemplate { entry: spawn.entry })?;
        let selection = self.runtime_selections.get(&spawn.entry).ok_or(
            CreatureLoadedGridResolveErrorLikeCpp::MissingRuntimeSelection { entry: spawn.entry },
        )?;
        validate_map_object_guid_like_cpp(spawn, template, map_object_guid)?;

        let lifecycle_record = CreatureLoadFromDbLifecycleRecord {
            create: CreatureCreateLifecycleRecord {
                guid: map_object_guid,
                entry: template.entry,
                map_id: spawn.map_id,
                instance_id: spawn.instance_id,
                position: spawn.position,
                dynamic: false,
                vehicle_id: template.vehicle_id,
                vehicle_kit_create_input: template.vehicle_kit_create_input.clone(),
                add_to_world_vehicle_reset_context: template
                    .add_to_world_vehicle_reset_context
                    .clone(),
                template: template_lifecycle_record(template),
                spawn: Some(spawn_lifecycle_record(spawn)),
                selected_level: selection.selected_level,
                stats: selection.stats,
                selected_display_id: selection.selected_display_id,
                selected_model_dimensions: selection.selected_model_dimensions,
                selected_equipment_id: selection.selected_equipment_id,
                selected_original_equipment_id: selection.selected_original_equipment_id,
                selected_virtual_items: selection.selected_virtual_items,
                corpse_delay: template.corpse_delay,
                ignore_corpse_decay_ratio: template.ignore_corpse_decay_ratio,
                addon: template.addon.clone(),
            },
            spawn: spawn_lifecycle_record(spawn),
        };

        let mut creature = Creature::load_from_db_lifecycle(lifecycle_record.clone());
        creature.set_formation_info_like_cpp(spawn.formation_info);
        if let Some(sparring_health_pct) = template.sparring_health_pct {
            creature.set_sparring_health_pct_like_cpp(sparring_health_pct);
        }
        // This is the DB-backed boundary that has resolved and applied the
        // selected creature_addon/template_addon source. Empty local aura
        // containers can therefore be accredited as inert for the stat and
        // power-cost fields in `SpellCastLogData`; any represented aura keeps
        // the completeness check closed and every later mutation revokes it.
        creature
            .unit_mut()
            .subsystems_mut()
            .auras
            .set_spell_cast_log_aura_authority_inert_like_cpp(true);
        let map_insertion_requested = spawn.add_to_map;
        Ok(CreatedLoadedGridCreature {
            lifecycle_record,
            creature,
            map_insertion_requested,
        })
    }

    pub fn resolve_loaded_grid_creature_like_cpp(
        &self,
        spawn_id: u64,
        map_object_guid: ObjectGuid,
    ) -> Result<CreatureLoadedGridResolvedLikeCpp, CreatureLoadedGridResolveErrorLikeCpp> {
        let CreatedLoadedGridCreature { lifecycle_record, creature, map_insertion_requested } =
            self.resolve_creature(spawn_id, map_object_guid)?;
        let map_object_record = if map_insertion_requested {
            Some(
                MapObjectRecord::new_creature(creature.clone()).map_err(|error| {
                    CreatureLoadedGridResolveErrorLikeCpp::MapObjectRecord(format!("{error:?}"))
                })?,
            )
        } else {
            None
        };

        Ok(CreatureLoadedGridResolvedLikeCpp {
            lifecycle_record,
            creature,
            map_object_record,
            map_insertion_requested,
        })
    }

    pub(crate) fn resolve_creature_record(
        &self,
        spawn_id: u64,
        map_object_guid: ObjectGuid,
    ) -> Result<Option<MapObjectRecord>, CreatureLoadedGridResolveErrorLikeCpp> {
        let created = self.resolve_creature(spawn_id, map_object_guid)?;
        project_creature_record(created.creature, created.map_insertion_requested)
    }
}

fn project_creature_record(
    creature: Creature,
    map_insertion_requested: bool,
) -> Result<Option<MapObjectRecord>, CreatureLoadedGridResolveErrorLikeCpp> {
    if map_insertion_requested {
        Some(MapObjectRecord::new_creature(creature).map_err(|error| {
            CreatureLoadedGridResolveErrorLikeCpp::MapObjectRecord(format!("{error:?}"))
        })).transpose()
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests;
