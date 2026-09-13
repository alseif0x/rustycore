//! Login hydration and CREATE projection share one Player-owned configuration map.
use super::WorldSession;
use tracing::warn;
use wow_core::ObjectGuid;
use wow_entities::{PlayerTraitConfigDetails, PlayerTraitEntry};
use wow_packet::packets::update::{TraitConfigCreateData, TraitEntryCreateData};

impl WorldSession {
    pub(crate) fn trait_authority_complete_like_cpp(
        &self,
        configs: &[TraitConfigCreateData],
        player_guid: ObjectGuid,
    ) -> bool {
        let complete = self.trait_tree_skill_line_index().is_none_or(|index| {
            configs.iter().all(|config| match config.config_type {
                2 => u32::try_from(config.skill_line_id)
                    .ok()
                    .is_some_and(|skill_line_id| index.has_skill_line_like_cpp(skill_line_id)),
                3 => u32::try_from(config.trait_system_id)
                    .ok()
                    .is_some_and(|trait_system_id| {
                        index.has_trait_system_like_cpp(trait_system_id)
                    }),
                _ => true,
            })
        });
        if !complete {
            warn!(
                player_guid = player_guid.counter(),
                "Keeping profession trait-config authority incomplete: no linked TraitMgr tree"
            );
        }
        complete
    }

    pub(crate) fn retain_loaded_trait_configs_like_cpp(
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

        self.with_owned_player_mut_like_cpp(|player| {
            player
                .gameplay_state_mut()
                .spells
                .install_loaded_trait_config_details_like_cpp(&hydration)
        })
        .unwrap_or(false)
    }

    pub(crate) fn owned_trait_configs_for_create_like_cpp(
        &self,
    ) -> Option<Vec<TraitConfigCreateData>> {
        self.with_owned_player_like_cpp(|player| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wow_data::skill_talent::SkillLineXTraitTreeStore;
    use wow_data::trait_tree::{TraitTreeEntry, TraitTreeSkillLineIndexLikeCpp, TraitTreeStore};
    use wow_packet::packets::update::TraitConfigCreateData;

    #[test]
    fn generic_trait_configs_require_a_loaded_trait_system_like_cpp() {
        let (_packet_tx, packet_rx) = flume::unbounded();
        let (send_tx, _send_rx) = flume::unbounded();
        let mut session = WorldSession::new(
            1,
            "traits".into(),
            0,
            2,
            2,
            12340,
            vec![],
            "enUS".into(),
            packet_rx,
            send_tx,
        );
        let index = TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
            &SkillLineXTraitTreeStore::from_entries([]),
            &TraitTreeStore::from_entries([TraitTreeEntry {
                id: 10,
                trait_system_id: 7,
                unused1000_1: 0,
                first_trait_node_id: 0,
                player_condition_id: 0,
                flags: 0,
                unused1000_2: 0.0,
                unused1000_3: 0.0,
            }]),
            |_| false,
        );
        session.set_trait_tree_skill_line_index(Arc::new(index));

        let config = |trait_system_id| TraitConfigCreateData {
            id: 1,
            config_type: 3,
            chr_specialization_id: 0,
            combat_config_flags: 0,
            local_identifier: 0,
            skill_line_id: 0,
            trait_system_id,
            name: "generic".into(),
            entries: vec![],
        };
        let player = ObjectGuid::create_player(1, 1);
        assert!(session.trait_authority_complete_like_cpp(&[config(7)], player));
        assert!(!session.trait_authority_complete_like_cpp(&[config(8)], player));
    }
}
