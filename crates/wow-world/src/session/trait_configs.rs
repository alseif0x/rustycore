//! Login hydration and CREATE projection share one Player-owned configuration map.
use super::WorldSession;
use wow_entities::{PlayerTraitConfigDetails, PlayerTraitEntry};
use wow_packet::packets::update::{TraitConfigCreateData, TraitEntryCreateData};

impl WorldSession {
    pub(crate) fn retain_loaded_trait_configs_like_cpp(
        &mut self,
        configs: &[TraitConfigCreateData],
    ) -> bool {
        self.with_owned_player_mut_like_cpp(|player| {
            let runtime = &mut player.gameplay_state_mut().spells;
            if !runtime.trait_config_rows_complete
                || !runtime.trait_entry_rows_complete
                || runtime.trait_config_rows.len() != configs.len()
                || configs
                    .iter()
                    .map(|config| config.id)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != configs.len()
                || configs.iter().any(|config| {
                    runtime
                        .trait_config_rows
                        .get(&config.id)
                        .is_none_or(|state| {
                            state.header
                                != (
                                    config.config_type,
                                    config.chr_specialization_id,
                                    config.combat_config_flags,
                                )
                        })
                })
            {
                return false;
            }
            for (create_index, config) in configs.iter().enumerate() {
                runtime
                    .trait_config_rows
                    .get_mut(&config.id)
                    .unwrap()
                    .details = Some(PlayerTraitConfigDetails {
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
                });
            }
            true
        })
        .unwrap_or(false)
    }

    pub(crate) fn owned_trait_configs_for_create_like_cpp(
        &self,
    ) -> Option<Vec<TraitConfigCreateData>> {
        self.with_owned_player_like_cpp(|player| {
            let runtime = &player.gameplay_state().spells;
            if !runtime.trait_config_rows_complete || !runtime.trait_entry_rows_complete {
                return None;
            }
            let mut configs = runtime
                .trait_config_rows
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
