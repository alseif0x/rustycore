//! Login hydration and CREATE projection share one Player-owned configuration map.
use super::WorldSession;
#[cfg(test)]
use wow_core::ObjectGuid;
#[cfg(test)]
use wow_data::trait_tree::TraitNodeEntryStore;
#[cfg(test)]
use wow_packet::packets::update::{TraitConfigCreateData, TraitEntryCreateData};

impl WorldSession {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wow_data::skill_talent::SkillLineXTraitTreeStore;
    use wow_data::trait_tree::{TraitTreeEntry, TraitTreeSkillLineIndexLikeCpp, TraitTreeStore};

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
            |_| Vec::new(),
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
        let nodes = wow_data::trait_tree::TraitNodeEntryStore::from_entries([]);
        assert!(
            crate::session::hub_ref(&session).trait_authority_complete_like_cpp(
                &[config(7)],
                &nodes,
                player
            )
        );
        assert!(
            !crate::session::hub_ref(&session).trait_authority_complete_like_cpp(
                &[config(8)],
                &nodes,
                player
            )
        );
    }

    #[test]
    fn combat_trait_configs_resolve_specialization_class_like_cpp() {
        let (_packet_tx, packet_rx) = flume::unbounded();
        let (send_tx, _send_rx) = flume::unbounded();
        let mut session = WorldSession::new(
            1,
            "combat-traits".into(),
            0,
            2,
            2,
            12340,
            vec![],
            "enUS".into(),
            packet_rx,
            send_tx,
        );
        session.set_chr_specialization_store(Arc::new(
            wow_data::ChrSpecializationStore::from_entries([wow_data::ChrSpecializationEntry {
                id: 42,
                class_id: 3,
                order_index: 0,
                role: 0,
            }]),
        ));
        let index = TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
            &SkillLineXTraitTreeStore::from_entries([wow_data::SkillLineXTraitTreeEntry {
                id: 100,
                skill_line_id: 164,
                trait_tree_id: 10,
                order_index: 0,
            }]),
            &TraitTreeStore::from_entries([TraitTreeEntry {
                id: 10,
                trait_system_id: 0,
                unused1000_1: 0,
                first_trait_node_id: 0,
                player_condition_id: 0,
                flags: 0,
                unused1000_2: 0.0,
                unused1000_3: 0.0,
            }]),
            |_| true,
            |skill_line_id| {
                (skill_line_id == 164)
                    .then_some(vec![3])
                    .unwrap_or_default()
            },
        );
        session.set_trait_tree_skill_line_index(Arc::new(index));

        let config = |specialization_id| TraitConfigCreateData {
            id: 1,
            config_type: 1,
            chr_specialization_id: specialization_id,
            combat_config_flags: 0,
            local_identifier: 0,
            skill_line_id: 0,
            trait_system_id: 0,
            name: "combat".into(),
            entries: vec![],
        };
        let player = ObjectGuid::create_player(1, 1);
        let nodes = wow_data::trait_tree::TraitNodeEntryStore::from_entries([]);
        assert!(
            crate::session::hub_ref(&session).trait_authority_complete_like_cpp(
                &[config(42)],
                &nodes,
                player
            )
        );
        assert!(
            !crate::session::hub_ref(&session).trait_authority_complete_like_cpp(
                &[config(43)],
                &nodes,
                player
            )
        );
    }

    #[test]
    fn trait_entries_require_known_node_entry_and_fit_max_ranks_like_cpp() {
        let (_packet_tx, packet_rx) = flume::unbounded();
        let (send_tx, _send_rx) = flume::unbounded();
        let session = WorldSession::new(
            1,
            "trait-entry-authority".into(),
            0,
            2,
            2,
            12340,
            vec![],
            "enUS".into(),
            packet_rx,
            send_tx,
        );
        let nodes =
            TraitNodeEntryStore::from_entries([wow_data::trait_tree::TraitNodeEntryEntry {
                id: 10,
                trait_definition_id: 1,
                max_ranks: 3,
                node_entry_type: 0,
            }]);
        let mut config = TraitConfigCreateData {
            id: 1,
            config_type: 0,
            chr_specialization_id: 0,
            combat_config_flags: 0,
            local_identifier: 0,
            skill_line_id: 0,
            trait_system_id: 0,
            name: "entries".into(),
            entries: vec![TraitEntryCreateData {
                trait_node_id: 1,
                trait_node_entry_id: 10,
                rank: 2,
                granted_ranks: 1,
            }],
        };
        let player = ObjectGuid::create_player(1, 1);
        assert!(
            crate::session::hub_ref(&session).trait_authority_complete_like_cpp(
                &[config.clone()],
                &nodes,
                player
            )
        );

        config.entries[0].rank = 3;
        assert!(
            !crate::session::hub_ref(&session).trait_authority_complete_like_cpp(
                &[config],
                &nodes,
                player
            )
        );
    }

    #[test]
    fn production_trait_authority_rejects_node_entry_from_another_tree_like_cpp() {
        let (_packet_tx, packet_rx) = flume::unbounded();
        let (send_tx, _send_rx) = flume::unbounded();
        let mut session = WorldSession::new(
            1,
            "trait-topology".into(),
            0,
            2,
            2,
            12340,
            vec![],
            "enUS".into(),
            packet_rx,
            send_tx,
        );
        let trees = TraitTreeStore::from_entries([TraitTreeEntry {
            id: 10,
            trait_system_id: 7,
            unused1000_1: 0,
            first_trait_node_id: 100,
            player_condition_id: 0,
            flags: 0,
            unused1000_2: 0.0,
            unused1000_3: 0.0,
        }]);
        let nodes = wow_data::trait_tree::TraitNodeStore::from_entries([
            wow_data::trait_tree::TraitNodeEntry {
                id: 100,
                trait_tree_id: 10,
                pos_x: 0,
                pos_y: 0,
                node_type: 0,
                flags: 0,
            },
        ]);
        let node_entries =
            TraitNodeEntryStore::from_entries([wow_data::trait_tree::TraitNodeEntryEntry {
                id: 1000,
                trait_definition_id: 1,
                max_ranks: 1,
                node_entry_type: 0,
            }]);
        let index = TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
            &SkillLineXTraitTreeStore::from_entries([]),
            &trees,
            |_| false,
            |_| Vec::new(),
        )
        .with_trait_graph_like_cpp(
            &trees,
            &nodes,
            &node_entries,
            &wow_data::trait_tree::TraitNodeEntryXTraitCondStore::from_entries([]),
            &wow_data::trait_tree::TraitNodeEntryXTraitCostStore::from_entries([]),
            &wow_data::trait_tree::TraitNodeGroupStore::from_entries([]),
            &wow_data::trait_tree::TraitNodeGroupXTraitCondStore::from_entries([]),
            &wow_data::trait_tree::TraitNodeGroupXTraitCostStore::from_entries([]),
            &wow_data::trait_tree::TraitNodeGroupXTraitNodeStore::from_entries([]),
            &wow_data::trait_tree::TraitNodeXTraitCondStore::from_entries([]),
            &wow_data::trait_tree::TraitNodeXTraitCostStore::from_entries([]),
            &wow_data::trait_tree::TraitNodeXTraitNodeEntryStore::from_entries([
                wow_data::trait_tree::TraitNodeXTraitNodeEntryEntry {
                    id: 1,
                    trait_node_id: 100,
                    trait_node_entry_id: 1000,
                    index: 0,
                },
            ]),
            &wow_data::trait_tree::TraitEdgeStore::from_entries([]),
            &wow_data::trait_tree::TraitCostStore::from_entries([]),
            &wow_data::trait_tree::TraitCondStore::from_entries([]),
            &wow_data::trait_tree::TraitTreeLoadoutStore::from_entries([]),
            &wow_data::trait_tree::TraitTreeLoadoutEntryStore::from_entries([]),
            &wow_data::trait_tree::TraitTreeXTraitCostStore::from_entries([]),
        );
        session.set_trait_tree_skill_line_index(Arc::new(index));

        let config = |node_id| TraitConfigCreateData {
            id: 1,
            config_type: 3,
            chr_specialization_id: 0,
            combat_config_flags: 0,
            local_identifier: 0,
            skill_line_id: 0,
            trait_system_id: 7,
            name: "generic".into(),
            entries: vec![TraitEntryCreateData {
                trait_node_id: node_id,
                trait_node_entry_id: 1000,
                rank: 1,
                granted_ranks: 0,
            }],
        };
        let player = ObjectGuid::create_player(1, 1);
        assert!(
            crate::session::hub_ref(&session).trait_authority_complete_like_cpp(
                &[config(100)],
                &node_entries,
                player
            )
        );
        assert!(
            !crate::session::hub_ref(&session).trait_authority_complete_like_cpp(
                &[config(101)],
                &node_entries,
                player
            )
        );
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/trait_configs/f3_shims.rs"]
mod f3_shims;
