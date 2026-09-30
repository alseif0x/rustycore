//! Trait tree indexing, semantic projection and DB2 loader regressions.

use super::*;
use crate::skill_talent::{SkillLineXTraitTreeEntry, SkillLineXTraitTreeStore};
use std::path::Path;

#[test]
fn trait_node_store_uses_cpp_tree_parent_relationship() {
    let store = TraitNodeStore::from_entries([TraitNodeEntry {
        id: 10,
        trait_tree_id: 20,
        pos_x: 1,
        pos_y: 2,
        node_type: 3,
        flags: 4,
    }]);

    assert_eq!(store.get(10).unwrap().trait_tree_id, 20);
}

#[test]
fn trait_tree_skill_line_index_matches_valid_links_and_order() {
    let links = SkillLineXTraitTreeStore::from_entries([
        SkillLineXTraitTreeEntry {
            id: 1,
            skill_line_id: 164,
            trait_tree_id: 20,
            order_index: 2,
        },
        SkillLineXTraitTreeEntry {
            id: 2,
            skill_line_id: 164,
            trait_tree_id: 10,
            order_index: 1,
        },
        SkillLineXTraitTreeEntry {
            id: 3,
            skill_line_id: 999,
            trait_tree_id: 20,
            order_index: 0,
        },
        SkillLineXTraitTreeEntry {
            id: 4,
            skill_line_id: 164,
            trait_tree_id: -1,
            order_index: 0,
        },
    ]);
    let trees = TraitTreeStore::from_entries([
        TraitTreeEntry {
            id: 10,
            trait_system_id: 0,
            unused1000_1: 0,
            first_trait_node_id: 0,
            player_condition_id: 0,
            flags: 0,
            unused1000_2: 0.0,
            unused1000_3: 0.0,
        },
        TraitTreeEntry {
            id: 20,
            trait_system_id: 0,
            unused1000_1: 0,
            first_trait_node_id: 0,
            player_condition_id: 0,
            flags: 0,
            unused1000_2: 0.0,
            unused1000_3: 0.0,
        },
    ]);

    let index = TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
        &links,
        &trees,
        |skill_line_id| skill_line_id == 164,
        |_| Vec::new(),
    );
    assert_eq!(index.trees_for_skill_line_like_cpp(164), &[10, 20]);
    assert!(index.has_skill_line_like_cpp(164));
    assert!(!index.has_skill_line_like_cpp(999));
    assert_eq!(index.len(), 2);
}

#[test]
fn trait_tree_index_exposes_generic_trait_systems_like_cpp() {
    let links = SkillLineXTraitTreeStore::from_entries([]);
    let trees = TraitTreeStore::from_entries([
        TraitTreeEntry {
            id: 10,
            trait_system_id: 7,
            unused1000_1: 0,
            first_trait_node_id: 0,
            player_condition_id: 0,
            flags: 0,
            unused1000_2: 0.0,
            unused1000_3: 0.0,
        },
        TraitTreeEntry {
            id: 20,
            trait_system_id: 7,
            unused1000_1: 0,
            first_trait_node_id: 0,
            player_condition_id: 0,
            flags: 0,
            unused1000_2: 0.0,
            unused1000_3: 0.0,
        },
        TraitTreeEntry {
            id: 30,
            trait_system_id: 0,
            unused1000_1: 0,
            first_trait_node_id: 0,
            player_condition_id: 0,
            flags: 0,
            unused1000_2: 0.0,
            unused1000_3: 0.0,
        },
    ]);

    let index = TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
        &links,
        &trees,
        |_| false,
        |_| Vec::new(),
    );
    assert_eq!(index.trees_for_trait_system_like_cpp(7), &[10, 20]);
    assert!(index.has_trait_system_like_cpp(7));
    assert!(!index.has_trait_system_like_cpp(30));
}

#[test]
fn trait_mgr_graph_indexes_nodes_relations_costs_conditions_edges_and_loadouts() {
    let links = SkillLineXTraitTreeStore::from_entries([]);
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
    let nodes = TraitNodeStore::from_entries([
        TraitNodeEntry {
            id: 100,
            trait_tree_id: 10,
            pos_x: 0,
            pos_y: 0,
            node_type: 0,
            flags: 0,
        },
        TraitNodeEntry {
            id: 101,
            trait_tree_id: 10,
            pos_x: 1,
            pos_y: 0,
            node_type: 0,
            flags: 0,
        },
    ]);
    let node_entries = TraitNodeEntryStore::from_entries([TraitNodeEntryEntry {
        id: 1000,
        trait_definition_id: 5,
        max_ranks: 2,
        node_entry_type: 0,
    }]);
    let groups = TraitNodeGroupStore::from_entries([TraitNodeGroupEntry {
        id: 200,
        trait_tree_id: 10,
        flags: 0,
    }]);
    let costs = TraitCostStore::from_entries([TraitCostEntry {
        id: 300,
        internal_name: "cost".into(),
        amount: 1,
        trait_currency_id: 4,
    }]);
    let conditions = TraitCondStore::from_entries([TraitCondEntry {
        id: 400,
        cond_type: 1,
        trait_tree_id: 10,
        granted_ranks: 1,
        quest_id: 0,
        achievement_id: 0,
        spec_set_id: 0,
        trait_node_group_id: 0,
        trait_node_id: 0,
        trait_currency_id: 0,
        spent_amount_required: 0,
        flags: 0,
        required_level: 0,
        free_shared_string_id: 0,
        spend_more_shared_string_id: 0,
    }]);
    let index = TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
        &links,
        &trees,
        |_| false,
        |_| Vec::new(),
    )
    .with_trait_graph_like_cpp(
        &trees,
        &nodes,
        &node_entries,
        &TraitNodeEntryXTraitCondStore::from_entries([TraitNodeEntryXTraitCondEntry {
            id: 500,
            trait_cond_id: 400,
            trait_node_entry_id: 1000,
        }]),
        &TraitNodeEntryXTraitCostStore::from_entries([TraitNodeEntryXTraitCostEntry {
            id: 501,
            trait_node_entry_id: 1000,
            trait_cost_id: 300,
        }]),
        &groups,
        &TraitNodeGroupXTraitCondStore::from_entries([]),
        &TraitNodeGroupXTraitCostStore::from_entries([]),
        &TraitNodeGroupXTraitNodeStore::from_entries([TraitNodeGroupXTraitNodeEntry {
            id: 600,
            trait_node_group_id: 200,
            trait_node_id: 100,
            index: 0,
        }]),
        &TraitNodeXTraitCondStore::from_entries([]),
        &TraitNodeXTraitCostStore::from_entries([]),
        &TraitNodeXTraitNodeEntryStore::from_entries([TraitNodeXTraitNodeEntryEntry {
            id: 601,
            trait_node_id: 100,
            trait_node_entry_id: 1000,
            index: 0,
        }]),
        &TraitEdgeStore::from_entries([TraitEdgeEntry {
            id: 700,
            visual_style: 0,
            left_trait_node_id: 100,
            right_trait_node_id: 101,
            edge_type: 2,
        }]),
        &costs,
        &conditions,
        &TraitTreeLoadoutStore::from_entries([TraitTreeLoadoutEntry {
            id: 800,
            trait_tree_id: 10,
            chr_specialization_id: 71,
        }]),
        &TraitTreeLoadoutEntryStore::from_entries([TraitTreeLoadoutEntryEntry {
            id: 801,
            trait_tree_loadout_id: 800,
            selected_trait_node_id: 100,
            selected_trait_node_entry_id: 1000,
            num_points: 1,
            order_index: 0,
        }]),
        &TraitTreeXTraitCostStore::from_entries([TraitTreeXTraitCostEntry {
            id: 900,
            trait_tree_id: 10,
            trait_cost_id: 300,
        }]),
    );

    assert!(index.graph_loaded_like_cpp());
    assert_eq!(index.nodes_for_tree_like_cpp(10), &[100, 101]);
    assert_eq!(index.entries_for_node_like_cpp(100), &[1000]);
    assert_eq!(index.groups_for_node_like_cpp(100), &[200]);
    assert_eq!(index.parent_nodes_for_node_like_cpp(101), &[(100, 2)]);
    assert_eq!(index.cost_ids_for_tree_like_cpp(10), &[300]);
    assert_eq!(index.cost_ids_for_entry_like_cpp(1000), &[300]);
    assert_eq!(index.condition_ids_for_entry_like_cpp(1000), &[400]);
    assert_eq!(
        index.loadout_for_specialization_like_cpp(71)[0].num_points,
        1
    );
    assert!(
        index
            .loadout_selection_is_valid_like_cpp(&index.loadout_for_specialization_like_cpp(71)[0])
    );
    assert!(index.entry_belongs_to_tree_set_like_cpp(&[10], 100, 1000));
    assert!(!index.entry_belongs_to_tree_set_like_cpp(&[10], 101, 1000));
}

#[test]
fn trait_mgr_semantic_validation_matches_cpp_conditions_costs_and_grants() {
    let links = SkillLineXTraitTreeStore::from_entries([]);
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
    let nodes = TraitNodeStore::from_entries([
        TraitNodeEntry {
            id: 100,
            trait_tree_id: 10,
            pos_x: 0,
            pos_y: 0,
            node_type: 0,
            flags: 0,
        },
        TraitNodeEntry {
            id: 101,
            trait_tree_id: 10,
            pos_x: 1,
            pos_y: 0,
            node_type: 2,
            flags: 0,
        },
    ]);
    let node_entries = TraitNodeEntryStore::from_entries([
        TraitNodeEntryEntry {
            id: 1000,
            trait_definition_id: 0,
            max_ranks: 2,
            node_entry_type: 0,
        },
        TraitNodeEntryEntry {
            id: 1001,
            trait_definition_id: 0,
            max_ranks: 1,
            node_entry_type: 0,
        },
        TraitNodeEntryEntry {
            id: 1002,
            trait_definition_id: 0,
            max_ranks: 1,
            node_entry_type: 0,
        },
    ]);
    let costs = TraitCostStore::from_entries([TraitCostEntry {
        id: 300,
        internal_name: "point".into(),
        amount: 1,
        trait_currency_id: 4,
    }]);
    let conditions = TraitCondStore::from_entries([
        TraitCondEntry {
            id: 400,
            cond_type: 0,
            trait_tree_id: 10,
            granted_ranks: 0,
            quest_id: 42,
            achievement_id: 0,
            spec_set_id: 0,
            trait_node_group_id: 0,
            trait_node_id: 0,
            trait_currency_id: 0,
            spent_amount_required: 0,
            flags: 0,
            required_level: 0,
            free_shared_string_id: 0,
            spend_more_shared_string_id: 0,
        },
        TraitCondEntry {
            id: 401,
            cond_type: 2,
            trait_tree_id: 10,
            granted_ranks: 1,
            quest_id: 0,
            achievement_id: 7,
            spec_set_id: 0,
            trait_node_group_id: 0,
            trait_node_id: 0,
            trait_currency_id: 0,
            spent_amount_required: 0,
            flags: 0,
            required_level: 0,
            free_shared_string_id: 0,
            spend_more_shared_string_id: 0,
        },
        TraitCondEntry {
            id: 402,
            cond_type: 0,
            trait_tree_id: 10,
            granted_ranks: 0,
            quest_id: 0,
            achievement_id: 0,
            spec_set_id: 0,
            trait_node_group_id: 0,
            trait_node_id: 0,
            trait_currency_id: 0,
            spent_amount_required: 0,
            flags: 0,
            required_level: 60,
            free_shared_string_id: 0,
            spend_more_shared_string_id: 0,
        },
        TraitCondEntry {
            id: 403,
            cond_type: 0,
            trait_tree_id: 10,
            granted_ranks: 0,
            quest_id: 0,
            achievement_id: 0,
            spec_set_id: 9,
            trait_node_group_id: 0,
            trait_node_id: 0,
            trait_currency_id: 0,
            spent_amount_required: 0,
            flags: 0,
            required_level: 0,
            free_shared_string_id: 0,
            spend_more_shared_string_id: 0,
        },
        TraitCondEntry {
            id: 404,
            cond_type: 0,
            trait_tree_id: 10,
            granted_ranks: 0,
            quest_id: 0,
            achievement_id: 7,
            spec_set_id: 0,
            trait_node_group_id: 0,
            trait_node_id: 0,
            trait_currency_id: 0,
            spent_amount_required: 0,
            flags: 0,
            required_level: 0,
            free_shared_string_id: 0,
            spend_more_shared_string_id: 0,
        },
    ]);
    let index = TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
        &links,
        &trees,
        |_| false,
        |_| Vec::new(),
    )
    .with_trait_graph_like_cpp(
        &trees,
        &nodes,
        &node_entries,
        &TraitNodeEntryXTraitCondStore::from_entries([
            TraitNodeEntryXTraitCondEntry {
                id: 500,
                trait_cond_id: 400,
                trait_node_entry_id: 1001,
            },
            TraitNodeEntryXTraitCondEntry {
                id: 501,
                trait_cond_id: 401,
                trait_node_entry_id: 1002,
            },
            TraitNodeEntryXTraitCondEntry {
                id: 503,
                trait_cond_id: 404,
                trait_node_entry_id: 1000,
            },
        ]),
        &TraitNodeEntryXTraitCostStore::from_entries([TraitNodeEntryXTraitCostEntry {
            id: 502,
            trait_node_entry_id: 1000,
            trait_cost_id: 300,
        }]),
        &TraitNodeGroupStore::from_entries([TraitNodeGroupEntry {
            id: 200,
            trait_tree_id: 10,
            flags: 0,
        }]),
        &TraitNodeGroupXTraitCondStore::from_entries([TraitNodeGroupXTraitCondEntry {
            id: 504,
            trait_cond_id: 403,
            trait_node_group_id: 200,
        }]),
        &TraitNodeGroupXTraitCostStore::from_entries([]),
        &TraitNodeGroupXTraitNodeStore::from_entries([TraitNodeGroupXTraitNodeEntry {
            id: 505,
            trait_node_group_id: 200,
            trait_node_id: 101,
            index: 0,
        }]),
        &TraitNodeXTraitCondStore::from_entries([TraitNodeXTraitCondEntry {
            id: 506,
            trait_cond_id: 402,
            trait_node_id: 100,
        }]),
        &TraitNodeXTraitCostStore::from_entries([]),
        &TraitNodeXTraitNodeEntryStore::from_entries([
            TraitNodeXTraitNodeEntryEntry {
                id: 600,
                trait_node_id: 100,
                trait_node_entry_id: 1000,
                index: 0,
            },
            TraitNodeXTraitNodeEntryEntry {
                id: 601,
                trait_node_id: 101,
                trait_node_entry_id: 1001,
                index: 0,
            },
            TraitNodeXTraitNodeEntryEntry {
                id: 602,
                trait_node_id: 101,
                trait_node_entry_id: 1002,
                index: 1,
            },
        ]),
        &TraitEdgeStore::from_entries([TraitEdgeEntry {
            id: 700,
            visual_style: 0,
            left_trait_node_id: 100,
            right_trait_node_id: 101,
            edge_type: 2,
        }]),
        &costs,
        &conditions,
        &TraitTreeLoadoutStore::from_entries([]),
        &TraitTreeLoadoutEntryStore::from_entries([]),
        &TraitTreeXTraitCostStore::from_entries([]),
    )
    .with_trait_currency_data_like_cpp(
        &TraitCurrencyStore::from_entries([TraitCurrencyEntry {
            id: 4,
            currency_type: 2,
            currency_types_id: 0,
            flags: 0,
            icon: 0,
        }]),
        &TraitCurrencySourceStore::from_entries([TraitCurrencySourceEntry {
            id: 800,
            requirement: String::new(),
            trait_currency_id: 4,
            amount: 2,
            quest_id: 0,
            achievement_id: 0,
            player_level: 0,
            trait_node_entry_id: 1000,
            order_index: 0,
        }]),
        &TraitTreeXTraitCurrencyStore::from_entries([TraitTreeXTraitCurrencyEntry {
            id: 801,
            index: 0,
            trait_tree_id: 10,
            trait_currency_id: 4,
        }]),
        &crate::SpecSetMemberStore::from_entries([crate::SpecSetMemberEntry {
            id: 802,
            chr_specialization_id: 71,
            spec_set_id: 9,
        }]),
    );
    let currency_quantities = BTreeMap::new();
    let rewarded_quest_ids = BTreeSet::from([42]);
    let achievement_ids = BTreeSet::from([7]);
    let facts = TraitPlayerFactsLikeCpp {
        level: 80,
        primary_specialization_id: 71,
        money: 0,
        currency_quantities: &currency_quantities,
        rewarded_quest_ids: &rewarded_quest_ids,
        achievement_ids: &achievement_ids,
    };
    let valid = [
        TraitConfigEntryLikeCpp {
            trait_node_id: 100,
            trait_node_entry_id: 1000,
            rank: 2,
            granted_ranks: 0,
        },
        TraitConfigEntryLikeCpp {
            trait_node_id: 101,
            trait_node_entry_id: 1001,
            rank: 1,
            granted_ranks: 0,
        },
    ];
    assert_eq!(
        index.validate_config_like_cpp(&[10], 3, 0, &valid, &facts),
        TraitConfigValidationResultLikeCpp::Ok
    );
    let invalid_selection = [
        valid[0],
        valid[1],
        TraitConfigEntryLikeCpp {
            trait_node_id: 101,
            trait_node_entry_id: 1002,
            rank: 1,
            granted_ranks: 0,
        },
    ];
    assert_eq!(
        index.validate_config_like_cpp(&[10], 3, 0, &invalid_selection, &facts),
        TraitConfigValidationResultLikeCpp::Unknown
    );
    let granted =
        index.granted_entries_for_config_like_cpp(&[10], 3, 0, &invalid_selection, &facts);
    assert_eq!(
        granted,
        vec![TraitConfigEntryLikeCpp {
            trait_node_id: 101,
            trait_node_entry_id: 1002,
            rank: 0,
            granted_ranks: 1,
        }]
    );
    let no_quest = TraitPlayerFactsLikeCpp {
        rewarded_quest_ids: &BTreeSet::new(),
        ..facts
    };
    assert_eq!(
        index.validate_config_like_cpp(&[10], 3, 0, &valid, &no_quest),
        TraitConfigValidationResultLikeCpp::Unknown
    );
    let no_achievement = TraitPlayerFactsLikeCpp {
        achievement_ids: &BTreeSet::new(),
        ..facts
    };
    assert_eq!(
        index.validate_config_like_cpp(&[10], 3, 0, &valid, &no_achievement),
        TraitConfigValidationResultLikeCpp::Unknown
    );
    let no_level = TraitPlayerFactsLikeCpp { level: 59, ..facts };
    assert_eq!(
        index.validate_config_like_cpp(&[10], 3, 0, &valid, &no_level),
        TraitConfigValidationResultLikeCpp::Unknown
    );
    let no_specialization = TraitPlayerFactsLikeCpp {
        primary_specialization_id: 72,
        ..facts
    };
    assert_eq!(
        index.validate_config_like_cpp(&[10], 3, 0, &valid, &no_specialization),
        TraitConfigValidationResultLikeCpp::Unknown
    );
}

#[test]
fn load_trait_tree_db2_subbatch_when_fixtures_exist() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "esES";
    let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
    if !dbc_dir.exists() {
        eprintln!(
            "Skipping test: DB2 fixture directory not found at {}",
            dbc_dir.display()
        );
        return;
    }

    macro_rules! load_if_exists {
        ($file:literal, $store:ty) => {
            if dbc_dir.join($file).exists() {
                let _store = <$store>::load(data_dir, locale)
                    .unwrap_or_else(|error| panic!("failed to load {}: {error:#}", $file));
            }
        };
    }

    load_if_exists!("TraitCond.db2", TraitCondStore);
    load_if_exists!("TraitCost.db2", TraitCostStore);
    load_if_exists!("TraitCurrency.db2", TraitCurrencyStore);
    load_if_exists!("TraitCurrencySource.db2", TraitCurrencySourceStore);
    load_if_exists!("TraitDefinition.db2", TraitDefinitionStore);
    load_if_exists!(
        "TraitDefinitionEffectPoints.db2",
        TraitDefinitionEffectPointsStore
    );
    load_if_exists!("TraitEdge.db2", TraitEdgeStore);
    load_if_exists!("TraitNode.db2", TraitNodeStore);
    load_if_exists!("TraitNodeEntry.db2", TraitNodeEntryStore);
    load_if_exists!(
        "TraitNodeEntryXTraitCond.db2",
        TraitNodeEntryXTraitCondStore
    );
    load_if_exists!(
        "TraitNodeEntryXTraitCost.db2",
        TraitNodeEntryXTraitCostStore
    );
    load_if_exists!("TraitNodeGroup.db2", TraitNodeGroupStore);
    load_if_exists!(
        "TraitNodeGroupXTraitCond.db2",
        TraitNodeGroupXTraitCondStore
    );
    load_if_exists!(
        "TraitNodeGroupXTraitCost.db2",
        TraitNodeGroupXTraitCostStore
    );
    load_if_exists!(
        "TraitNodeGroupXTraitNode.db2",
        TraitNodeGroupXTraitNodeStore
    );
    load_if_exists!("TraitNodeXTraitCond.db2", TraitNodeXTraitCondStore);
    load_if_exists!("TraitNodeXTraitCost.db2", TraitNodeXTraitCostStore);
    load_if_exists!(
        "TraitNodeXTraitNodeEntry.db2",
        TraitNodeXTraitNodeEntryStore
    );
    load_if_exists!("TraitTree.db2", TraitTreeStore);
    load_if_exists!("TraitTreeLoadout.db2", TraitTreeLoadoutStore);
    load_if_exists!("TraitTreeLoadoutEntry.db2", TraitTreeLoadoutEntryStore);
    load_if_exists!("TraitTreeXTraitCost.db2", TraitTreeXTraitCostStore);
    load_if_exists!("TraitTreeXTraitCurrency.db2", TraitTreeXTraitCurrencyStore);
}
