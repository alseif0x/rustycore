//! Creature-addon Aura visual resolution and Map-owned Cast provenance.

use super::*;

/// Resolve the bounded C++ `SpellInfo::GetSpellXSpellVisualId` domain used by
/// production Creature addon auras. The audited 3.4.3 effective data has at
/// most one row per addon spell, all at difficulty 0 and without caster
/// conditions. If future data leaves that proven domain, return 0 instead of
/// guessing through a missing UnitCondition/visual-override authority.
pub(crate) fn creature_addon_spell_x_spell_visual_id_like_cpp(
    store: &wow_data::SpellXSpellVisualStore,
    spell_id: u32,
    difficulty_id: u8,
) -> i32 {
    let rows_for = |selected_difficulty| {
        store
            .entries_like_cpp()
            .filter(|entry| {
                entry.spell_id == spell_id && entry.difficulty_id == selected_difficulty
            })
            .collect::<Vec<_>>()
    };
    let mut rows = rows_for(difficulty_id);
    if rows.is_empty() && difficulty_id != 0 {
        rows = rows_for(0);
    }
    let [row] = rows.as_slice() else {
        return 0;
    };
    if row.caster_player_condition_id != 0 || row.caster_unit_condition_id != 0 {
        return 0;
    }
    i32::try_from(row.id).unwrap_or(0)
}

pub(super) fn resolve_addon_visuals_like_cpp(
    addon: Option<&mut wow_entities::CreatureAddonLifecycleRecordLikeCpp>,
    store: &wow_data::SpellXSpellVisualStore,
    difficulty_id: u8,
) {
    let Some(addon) = addon else {
        return;
    };
    for aura in &mut addon.aura_applications {
        aura.spell_visual_id =
            creature_addon_spell_x_spell_visual_id_like_cpp(store, aura.spell_id, difficulty_id);
    }
}

pub(super) fn settle_resolved_loaded_grid_creature_like_cpp(
    map: &mut wow_map::Map,
    resolved: Result<
        creature_loaded_grid::CreatureLoadedGridResolvedLikeCpp,
        creature_loaded_grid::CreatureLoadedGridResolveErrorLikeCpp,
    >,
    spawn_id: wow_map::SpawnId,
    entry: u32,
    map_object_guid: ObjectGuid,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    match resolved {
        Ok(resolved) => {
            let mut primary_record = resolved.map_object_record?;
            let Some(creature) = primary_record.creature_mut() else {
                warn!(
                    spawn_id,
                    entry,
                    guid = ?map_object_guid,
                    "C++ loaded-grid Creature DoRespawn blocked: resolver returned a non-Creature primary record"
                );
                return None;
            };
            if let Err(error) = map.settle_creature_addon_aura_provenance_like_cpp(creature) {
                warn!(
                    ?error,
                    spawn_id,
                    entry,
                    guid = ?map_object_guid,
                    "C++ loaded-grid Creature DoRespawn blocked: map-owned addon Aura CastGUID allocation failed"
                );
                return None;
            }
            Some(wow_map::map::LoadedGridRespawnRecordsLikeCpp::primary_only(
                primary_record,
            ))
        }
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry,
                guid = ?map_object_guid,
                "C++ loaded-grid Creature DoRespawn blocked: resolver rejected loaded Creature record"
            );
            None
        }
    }
}
