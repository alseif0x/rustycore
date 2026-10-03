use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use tracing::warn;
use wow_core::Position;
use wow_entities::{CorpseCustomizationChoice, CorpseType};
use wow_world_core::map_manager::{
    LoadedMapCorpseRowLikeCpp, MapCorpseLoadOutcomeLikeCpp,
    materialize_loaded_map_corpses_like_cpp, parse_corpse_items_like_cpp,
};
use wow_world_core::session::HubRef;

use super::SessionLifecycleState;

impl SessionLifecycleState {
    /// Load the map's persisted corpses once, including the two auxiliary
    /// tables consumed by C++ `Map::LoadCorpseData` before `AddCorpse`.
    pub async fn load_map_corpse_data_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_id: u16,
        instance_id: u32,
    ) -> MapCorpseLoadOutcomeLikeCpp {
        let Some(manager) = hub.core.canonical_map_manager.as_ref().map(Arc::clone) else {
            return MapCorpseLoadOutcomeLikeCpp::default();
        };
        {
            let Ok(manager) = manager.lock() else {
                warn!(
                    map_id,
                    instance_id, "Cannot inspect canonical map corpse-load state: lock poisoned"
                );
                return MapCorpseLoadOutcomeLikeCpp::default();
            };
            let Some(map) = manager.find_map(u32::from(map_id), instance_id) else {
                warn!(
                    map_id,
                    instance_id, "Cannot load C++ map corpses: canonical map is unavailable"
                );
                return MapCorpseLoadOutcomeLikeCpp::default();
            };
            if map.map().corpse_data_loaded_like_cpp() {
                return MapCorpseLoadOutcomeLikeCpp {
                    already_loaded: true,
                    ..Default::default()
                };
            }
        }

        let Some(port) = self.map_corpse_persistence_port_like_cpp().map(Arc::clone) else {
            return MapCorpseLoadOutcomeLikeCpp::default();
        };
        let (corpse_rows, phase_rows, customization_rows) = match port
            .load_map_corpses_like_cpp(wow_persistence::MapCorpseLoadRequestLikeCpp {
                map_id: u32::from(map_id),
                instance_id,
            })
            .await
        {
            wow_persistence::MapCorpseLoadOutcomeLikeCpp::Loaded {
                corpses,
                phases,
                customizations,
            } => (corpses, phases, customizations),
            wow_persistence::MapCorpseLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    map_id,
                    instance_id, "C++ Map::LoadCorpseData base query failed: {reason}"
                );
                return MapCorpseLoadOutcomeLikeCpp::default();
            }
        };

        let mut rows = Vec::with_capacity(corpse_rows.len());
        let mut invalid_type_rows = 0u32;
        for row in corpse_rows {
            let corpse_type = match row.corpse_type {
                1 => Some(CorpseType::ResurrectablePve),
                2 => Some(CorpseType::ResurrectablePvp),
                // C++ rejects bones and values >= MAX_CORPSE_TYPE.
                _ => None,
            };
            if let Some(corpse_type) = corpse_type {
                rows.push(LoadedMapCorpseRowLikeCpp {
                    position: Position::new(row.pos_x, row.pos_y, row.pos_z, row.orientation),
                    map_id: row.map_id,
                    display_id: row.display_id,
                    items: parse_corpse_items_like_cpp(&row.item_cache),
                    race: row.race,
                    class: row.class,
                    sex: row.sex,
                    flags: u32::from(row.flags),
                    dynamic_flags: u32::from(row.dynamic_flags),
                    ghost_time: i64::from(row.ghost_time),
                    corpse_type,
                    instance_id: row.instance_id,
                    owner_db_guid: row.owner_guid,
                });
            } else {
                invalid_type_rows = invalid_type_rows.saturating_add(1);
            }
        }

        let mut phases = HashMap::<u64, BTreeSet<u32>>::new();
        let mut customizations = HashMap::<u64, Vec<CorpseCustomizationChoice>>::new();
        match phase_rows {
            wow_persistence::MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(phase_rows) => {
                for row in phase_rows {
                    phases
                        .entry(row.owner_guid)
                        .or_default()
                        .insert(row.phase_id);
                }
            }
            wow_persistence::MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    map_id,
                    instance_id,
                    "C++ Map::LoadCorpseData phase query failed; continuing without phases: {reason}"
                );
            }
        }
        match customization_rows {
            wow_persistence::MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(customization_rows) => {
                for row in customization_rows {
                    customizations.entry(row.owner_guid).or_default().push(
                        CorpseCustomizationChoice {
                            option_id: row.option_id,
                            choice_id: row.choice_id,
                        },
                    );
                }
            }
            wow_persistence::MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    map_id,
                    instance_id,
                    "C++ Map::LoadCorpseData customization query failed; continuing without customizations: {reason}"
                );
            }
        }

        let faction_templates_by_race = rows
            .iter()
            .filter_map(|row| {
                hub.catalogs
                    .faction_template_for_race_like_cpp(row.race)
                    .map(|faction| (row.race, faction))
            })
            .collect::<HashMap<_, _>>();
        let Ok(mut manager) = manager.lock() else {
            warn!(
                map_id,
                instance_id, "Cannot materialize canonical map corpses: lock poisoned"
            );
            return MapCorpseLoadOutcomeLikeCpp {
                invalid_type_rows,
                ..Default::default()
            };
        };
        let Some(map) = manager.find_map_mut(u32::from(map_id), instance_id) else {
            warn!(
                map_id,
                instance_id, "Cannot materialize C++ map corpses: canonical map disappeared"
            );
            return MapCorpseLoadOutcomeLikeCpp {
                invalid_type_rows,
                ..Default::default()
            };
        };
        let mut outcome = materialize_loaded_map_corpses_like_cpp(
            map.map_mut(),
            hub.core.realm_id(),
            rows,
            &phases,
            &customizations,
            &faction_templates_by_race,
        );
        outcome.invalid_type_rows = outcome.invalid_type_rows.saturating_add(invalid_type_rows);
        outcome
    }
}
