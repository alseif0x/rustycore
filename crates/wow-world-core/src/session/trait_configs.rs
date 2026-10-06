//! Login hydration and CREATE projection share one Player-owned configuration map.

use std::collections::{BTreeMap, BTreeSet};

use tracing::warn;
use wow_core::ObjectGuid;
use wow_data::trait_tree::{
    TraitConfigEntryLikeCpp, TraitConfigValidationResultLikeCpp, TraitNodeEntryStore,
    TraitPlayerFactsLikeCpp,
};
use wow_entities::{PlayerTraitConfigDetails, PlayerTraitEntry};
use wow_packet::packets::update::{TraitConfigCreateData, TraitEntryCreateData};

impl crate::session::HubMut<'_> {
    pub fn retain_loaded_trait_configs_like_cpp(
        &mut self,
        configs: &[TraitConfigCreateData],
    ) -> bool {
        // The owner enforces that this hydration describes exactly the rows it
        // loaded; the session only shapes the packet payload into the details
        // it stores.
        let hydration = configs
            .iter()
            .enumerate()
            .map(|(create_index, config)| {
                (
                    config.id,
                    (
                        config.config_type,
                        config.chr_specialization_id,
                        config.combat_config_flags,
                    ),
                    PlayerTraitConfigDetails {
                        create_index,
                        local_identifier: config.local_identifier,
                        skill_line_id: config.skill_line_id,
                        trait_system_id: config.trait_system_id,
                        name: config.name.clone(),
                        entries: config
                            .entries
                            .iter()
                            .map(|entry| PlayerTraitEntry {
                                trait_node_id: entry.trait_node_id,
                                trait_node_entry_id: entry.trait_node_entry_id,
                                rank: entry.rank,
                                granted_ranks: entry.granted_ranks,
                            })
                            .collect(),
                    },
                )
            })
            .collect::<Vec<_>>();

        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .gameplay_state_mut()
                    .spells
                    .install_loaded_trait_config_details_like_cpp(&hydration)
            })
            .unwrap_or(false)
    }
}

impl crate::session::HubRef<'_> {
    fn trait_tree_ids_for_config_like_cpp(
        &self,
        config: &TraitConfigCreateData,
    ) -> Option<Vec<u32>> {
        let index = self.catalogs.trait_tree_skill_line_index()?.as_ref();
        let tree_ids = match config.config_type {
            1 => {
                let specialization_id = u32::try_from(config.chr_specialization_id).ok()?;
                let specialization = self
                    .catalogs
                    .chr_specialization_store()?
                    .get(specialization_id)?;
                if !index.has_class_like_cpp(specialization.class_id) {
                    return None;
                }
                index.trees_for_class_like_cpp(specialization.class_id)
            }
            2 => {
                let skill_line_id = u32::try_from(config.skill_line_id).ok()?;
                if !index.has_skill_line_like_cpp(skill_line_id) {
                    return None;
                }
                index.trees_for_skill_line_like_cpp(skill_line_id)
            }
            3 => {
                let trait_system_id = u32::try_from(config.trait_system_id).ok()?;
                if !index.has_trait_system_like_cpp(trait_system_id) {
                    return None;
                }
                index.trees_for_trait_system_like_cpp(trait_system_id)
            }
            _ => return None,
        };
        (!tree_ids.is_empty()).then(|| tree_ids.to_vec())
    }

    /// Snapshot only canonical Player facts while the Player handle is held,
    /// then run the immutable TraitMgr semantic operation without lending any
    /// mutable Player state to the catalog.
    fn trait_player_facts_like_cpp(
        &self,
    ) -> Option<(
        i32,
        u32,
        u64,
        BTreeMap<u32, i32>,
        BTreeSet<u32>,
        BTreeSet<u32>,
    )> {
        self.core.with_owned_player_like_cpp(|player| {
            let currencies = player
                .gameplay_state()
                .currencies
                .iter()
                .map(|(&currency_id, currency)| (currency_id, currency.quantity as i32))
                .collect();
            let rewarded_quests = player
                .gameplay_state()
                .quests
                .rewarded_quest_ids_like_cpp()
                .clone();
            let achievements = player
                .gameplay_state()
                .achievements
                .iter()
                .map(|record| record.achievement_id)
                .collect();
            (
                player.unit().data().level,
                player.primary_specialization_id_like_cpp(),
                player.money(),
                currencies,
                rewarded_quests,
                achievements,
            )
        })
    }

    /// Apply C++ `ValidateConfig` and replace invalid persisted entries with
    /// `GetGrantedTraitEntriesForConfig` before Player publication.
    pub fn normalize_trait_configs_like_cpp(
        &self,
        configs: &[TraitConfigCreateData],
    ) -> Option<Vec<TraitConfigCreateData>> {
        let index = self.catalogs.trait_tree_skill_line_index()?.as_ref();
        if !index.graph_loaded_like_cpp() {
            return None;
        }
        let (
            level,
            primary_specialization_id,
            money,
            currency_quantities,
            rewarded_quest_ids,
            achievement_ids,
        ) = self.trait_player_facts_like_cpp()?;
        let facts = TraitPlayerFactsLikeCpp {
            level,
            primary_specialization_id,
            money,
            currency_quantities: &currency_quantities,
            rewarded_quest_ids: &rewarded_quest_ids,
            achievement_ids: &achievement_ids,
        };
        configs
            .iter()
            .map(|config| {
                let entries = config
                    .entries
                    .iter()
                    .filter_map(|entry| {
                        let candidate = TraitConfigEntryLikeCpp {
                            trait_node_id: entry.trait_node_id,
                            trait_node_entry_id: entry.trait_node_entry_id,
                            rank: entry.rank,
                            granted_ranks: entry.granted_ranks,
                        };
                        index
                            .is_valid_entry_like_cpp(&candidate)
                            .then_some(candidate)
                    })
                    .collect::<Vec<_>>();
                let Some(tree_ids) = self.trait_tree_ids_for_config_like_cpp(config) else {
                    let mut normalized = config.clone();
                    // C++ `ValidateConfig` returns `Unknown` when no tree
                    // resolves for the config, and its subsequent granted
                    // lookup also returns an empty vector. Do not retain
                    // individually valid rows for an unknown config scope.
                    normalized.entries = Vec::new();
                    return Some(normalized);
                };
                let result = index.validate_config_like_cpp(
                    &tree_ids,
                    config.config_type,
                    config.chr_specialization_id,
                    &entries,
                    &facts,
                );
                let effective_entries = if result == TraitConfigValidationResultLikeCpp::Ok {
                    entries
                } else {
                    index.granted_entries_for_config_like_cpp(
                        &tree_ids,
                        config.config_type,
                        config.chr_specialization_id,
                        &entries,
                        &facts,
                    )
                };
                let mut normalized = config.clone();
                normalized.entries = effective_entries
                    .into_iter()
                    .map(|entry| TraitEntryCreateData {
                        trait_node_id: entry.trait_node_id,
                        trait_node_entry_id: entry.trait_node_entry_id,
                        rank: entry.rank,
                        granted_ranks: entry.granted_ranks,
                    })
                    .collect();
                Some(normalized)
            })
            .collect()
    }

    pub fn trait_authority_complete_like_cpp(
        &self,
        configs: &[TraitConfigCreateData],
        node_entries: &TraitNodeEntryStore,
        player_guid: ObjectGuid,
    ) -> bool {
        let entries_complete = configs
            .iter()
            .flat_map(|config| &config.entries)
            .all(|entry| {
                let Some(node_entry_id) = u32::try_from(entry.trait_node_entry_id).ok() else {
                    return false;
                };
                let Some(node_entry) = node_entries.get(node_entry_id) else {
                    return false;
                };
                entry.trait_node_id > 0
                    && entry.rank >= 0
                    && entry.granted_ranks >= 0
                    && i64::from(entry.rank) + i64::from(entry.granted_ranks)
                        <= i64::from(node_entry.max_ranks)
            });
        let complete = entries_complete
            && self
                .catalogs
                .trait_tree_skill_line_index()
                .is_none_or(|index| {
                    configs.iter().all(|config| {
                        let tree_ids = match config.config_type {
                            1 => u32::try_from(config.chr_specialization_id)
                                .ok()
                                .and_then(|specialization_id| {
                                    self.catalogs
                                        .chr_specialization_store()
                                        .and_then(|store| store.get(specialization_id))
                                        .map(|specialization| {
                                            (
                                                index.has_class_like_cpp(specialization.class_id),
                                                index.trees_for_class_like_cpp(
                                                    specialization.class_id,
                                                ),
                                            )
                                        })
                                })
                                .and_then(|(present, ids)| present.then_some(ids)),
                            2 => u32::try_from(config.skill_line_id)
                                .ok()
                                .map(|skill_line_id| {
                                    (
                                        index.has_skill_line_like_cpp(skill_line_id),
                                        index.trees_for_skill_line_like_cpp(skill_line_id),
                                    )
                                })
                                .and_then(|(present, ids)| present.then_some(ids)),
                            3 => u32::try_from(config.trait_system_id)
                                .ok()
                                .map(|trait_system_id| {
                                    (
                                        index.has_trait_system_like_cpp(trait_system_id),
                                        index.trees_for_trait_system_like_cpp(trait_system_id),
                                    )
                                })
                                .and_then(|(present, ids)| present.then_some(ids)),
                            _ => Some(&[] as &[u32]),
                        };
                        let Some(tree_ids) = tree_ids else {
                            return false;
                        };
                        if !index.graph_loaded_like_cpp() {
                            return true;
                        }
                        config.entries.iter().all(|entry| {
                            let Some(node_id) = u32::try_from(entry.trait_node_id).ok() else {
                                return false;
                            };
                            let Some(entry_id) = u32::try_from(entry.trait_node_entry_id).ok()
                            else {
                                return false;
                            };
                            index.entry_belongs_to_tree_set_like_cpp(tree_ids, node_id, entry_id)
                        })
                    })
                });
        if !complete {
            warn!(
                player_guid = player_guid.counter(),
                "Keeping trait-config authority incomplete: TraitMgr validation failed"
            );
        }
        complete
    }

    pub fn owned_trait_configs_for_create_like_cpp(&self) -> Option<Vec<TraitConfigCreateData>> {
        self.core.with_owned_player_like_cpp(|player| {
            let runtime = &player.gameplay_state().spells;
            if !runtime.trait_config_rows_complete_like_cpp()
                || !runtime.trait_entry_rows_complete_like_cpp()
            {
                return None;
            }
            let mut configs = runtime
                .trait_config_rows_like_cpp()
                .iter()
                .map(|(&id, state)| {
                    let details = state.details.as_ref()?;
                    Some((
                        details.create_index,
                        TraitConfigCreateData {
                            id,
                            config_type: state.header.0,
                            chr_specialization_id: state.header.1,
                            combat_config_flags: state.header.2,
                            local_identifier: details.local_identifier,
                            skill_line_id: details.skill_line_id,
                            trait_system_id: details.trait_system_id,
                            name: details.name.clone(),
                            entries: details
                                .entries
                                .iter()
                                .map(|entry| TraitEntryCreateData {
                                    trait_node_id: entry.trait_node_id,
                                    trait_node_entry_id: entry.trait_node_entry_id,
                                    rank: entry.rank,
                                    granted_ranks: entry.granted_ranks,
                                })
                                .collect(),
                        },
                    ))
                })
                .collect::<Option<Vec<_>>>()?;
            configs.sort_by_key(|(index, _)| *index);
            if configs
                .iter()
                .enumerate()
                .any(|(expected, (actual, _))| expected != *actual)
            {
                return None;
            }
            Some(configs.into_iter().map(|(_, config)| config).collect())
        })?
    }
}
