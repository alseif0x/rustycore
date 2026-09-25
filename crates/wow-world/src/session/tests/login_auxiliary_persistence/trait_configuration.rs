use super::*;

#[tokio::test]
async fn trait_config_load_does_not_authorize_stale_or_missing_player_owner() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    let projection = |player: &Player| player.spell_runtime_like_cpp().clone();
    let expected = manager
        .lock()
        .unwrap()
        .with_player_like_cpp(handle, projection);
    for missing in [false, true] {
        if missing {
            session.canonical_map_manager = None;
        }
        session.set_player_lifecycle_port_like_cpp(AuxiliaryLoadPortLikeCpp::new([
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                PlayerLoginAuxiliaryLoadedLikeCpp::TraitEntries(vec![]),
            ),
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                PlayerLoginAuxiliaryLoadedLikeCpp::TraitConfigs(vec![]),
            ),
        ]));
        assert!(
            session
                .load_active_player_trait_configs_like_cpp(
                    &wow_data::trait_tree::TraitNodeEntryStore::from_entries([]),
                    guid
                )
                .await
                .is_empty()
        );
        assert!(
            session
                .complete_represented_spell_trait_definition_ids_like_cpp()
                .is_none()
        );
        assert_eq!(
            manager
                .lock()
                .unwrap()
                .with_player_like_cpp(handle, projection),
            expected
        );
    }
}

#[tokio::test]
async fn trait_config_load_borrows_catalog_and_preserves_fail_closed_active_and_detached() {
    use wow_data::trait_tree::{
        TraitDefinitionEntry, TraitDefinitionStore, TraitNodeEntryEntry, TraitNodeEntryStore,
    };

    for detached in [false, true] {
        for valid_node in [false, true] {
            for valid_definition in [false, true] {
                let (mut session, _, _) = make_session();
                let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
                if detached {
                    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
                }
                let nodes = TraitNodeEntryStore::from_entries(if valid_node {
                    vec![TraitNodeEntryEntry {
                        id: 300,
                        trait_definition_id: 400,
                        max_ranks: 3,
                        node_entry_type: 0,
                    }]
                } else {
                    vec![]
                });
                if valid_definition {
                    session.set_trait_definition_store(Arc::new(
                        TraitDefinitionStore::from_entries([TraitDefinitionEntry {
                            id: 400,
                            override_name: String::new(),
                            override_subtext: String::new(),
                            override_description: String::new(),
                            spell_id: 500,
                            override_icon: 0,
                            overrides_spell_id: 0,
                            visible_spell_id: 0,
                        }]),
                    ));
                }
                let port = AuxiliaryLoadPortLikeCpp::new([
                    PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                        PlayerLoginAuxiliaryLoadedLikeCpp::TraitEntries(vec![
                            PlayerTraitEntryLoadRowLikeCpp {
                                trait_config_id: Some(100),
                                trait_node_id: Some(200),
                                trait_node_entry_id: Some(300),
                                rank: Some(2),
                                granted_ranks: Some(1),
                            },
                        ]),
                    ),
                    PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                        PlayerLoginAuxiliaryLoadedLikeCpp::TraitConfigs(vec![
                            PlayerTraitConfigLoadRowLikeCpp {
                                id: Some(100),
                                config_type: Some(1),
                                chr_specialization_id: Some(62),
                                combat_config_flags: Some(4),
                                local_identifier: Some(2),
                                skill_line_id: None,
                                trait_system_id: None,
                                name: Some("Arcane".to_owned()),
                            },
                        ]),
                    ),
                ]);
                session.set_player_lifecycle_port_like_cpp(port.clone());
                assert!(
                    session
                        .set_complete_represented_spell_trait_definition_ids_like_cpp([(999, 998)])
                );
                let configs = session
                    .load_active_player_trait_configs_like_cpp(&nodes, guid)
                    .await;
                assert_eq!(configs.len(), 1);
                assert_eq!(configs[0].entries[0].trait_node_entry_id, 300);
                assert_eq!(configs[0].entries[0].rank, 2);
                assert_eq!(configs[0].entries[0].granted_ranks, 1);
                assert_eq!(
                    session.complete_represented_spell_trait_definition_ids_like_cpp(),
                    (valid_node && valid_definition).then(|| HashMap::from([(500, 400)]))
                );
                assert_eq!(
                    port.requests(),
                    vec![
                        PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitEntries {
                            player_guid: guid.counter() as u64
                        },
                        PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitConfigs {
                            player_guid: guid.counter() as u64
                        },
                    ]
                );
            }
        }
    }
}

#[tokio::test]
async fn trait_config_loads_preserve_cpp_entry_then_config_order_and_raw_values() {
    let nodes = wow_data::trait_tree::TraitNodeEntryStore::from_entries([
        wow_data::trait_tree::TraitNodeEntryEntry {
            id: 300,
            trait_definition_id: 0,
            max_ranks: 3,
            node_entry_type: 0,
        },
    ]);
    let port = AuxiliaryLoadPortLikeCpp::new([
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::TraitEntries(vec![PlayerTraitEntryLoadRowLikeCpp {
                trait_config_id: Some(100),
                trait_node_id: Some(200),
                trait_node_entry_id: Some(300),
                rank: Some(2),
                granted_ranks: Some(1),
            }]),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::TraitConfigs(vec![
                PlayerTraitConfigLoadRowLikeCpp {
                    id: Some(100),
                    config_type: Some(1),
                    chr_specialization_id: Some(62),
                    combat_config_flags: Some(4),
                    local_identifier: Some(2),
                    skill_line_id: None,
                    trait_system_id: None,
                    name: Some("Arcane".to_owned()),
                },
            ]),
        ),
    ]);
    let (mut session, _, _) = make_session();
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let guid = ObjectGuid::create_player(1, 48);
    session.set_player_guid(Some(guid));
    install_canonical_player_owner_for_test(&mut session, 0, 0);

    let configs = session
        .load_active_player_trait_configs_like_cpp(&nodes, guid)
        .await;

    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].id, 100);
    assert_eq!(configs[0].config_type, 1);
    assert_eq!(configs[0].chr_specialization_id, 62);
    assert_eq!(configs[0].combat_config_flags, 4);
    assert_eq!(configs[0].local_identifier, 2);
    assert_eq!(configs[0].name, "Arcane");
    assert_eq!(configs[0].entries.len(), 1);
    assert_eq!(configs[0].entries[0].trait_node_id, 200);
    assert_eq!(configs[0].entries[0].trait_node_entry_id, 300);
    assert_eq!(configs[0].entries[0].rank, 2);
    assert_eq!(configs[0].entries[0].granted_ranks, 1);
    assert_eq!(
        session.owned_trait_configs_for_create_like_cpp(),
        Some(configs)
    );
    assert_eq!(
        port.requests(),
        vec![
            PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitEntries { player_guid: 48 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitConfigs { player_guid: 48 },
        ]
    );
}

#[tokio::test]
async fn trait_config_login_replaces_invalid_rows_with_granted_entries_like_cpp() {
    use wow_data::trait_tree::{
        TraitCondEntry, TraitCondStore, TraitNodeEntry, TraitNodeEntryEntry, TraitNodeEntryStore,
        TraitNodeEntryXTraitCondEntry, TraitNodeEntryXTraitCondStore, TraitNodeStore,
        TraitNodeXTraitNodeEntryEntry, TraitNodeXTraitNodeEntryStore, TraitTreeEntry,
        TraitTreeLoadoutEntryStore, TraitTreeLoadoutStore, TraitTreeSkillLineIndexLikeCpp,
        TraitTreeStore, TraitTreeXTraitCostStore,
    };

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
    let nodes = TraitNodeStore::from_entries([TraitNodeEntry {
        id: 100,
        trait_tree_id: 10,
        pos_x: 0,
        pos_y: 0,
        node_type: 0,
        flags: 0,
    }]);
    let node_entries = TraitNodeEntryStore::from_entries([TraitNodeEntryEntry {
        id: 1000,
        trait_definition_id: 0,
        max_ranks: 1,
        node_entry_type: 0,
    }]);
    let index = TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
        &wow_data::skill_talent::SkillLineXTraitTreeStore::from_entries([]),
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
                trait_node_entry_id: 1000,
            },
            TraitNodeEntryXTraitCondEntry {
                id: 502,
                trait_cond_id: 401,
                trait_node_entry_id: 1000,
            },
        ]),
        &wow_data::trait_tree::TraitNodeEntryXTraitCostStore::from_entries([]),
        &wow_data::trait_tree::TraitNodeGroupStore::from_entries([]),
        &wow_data::trait_tree::TraitNodeGroupXTraitCondStore::from_entries([]),
        &wow_data::trait_tree::TraitNodeGroupXTraitCostStore::from_entries([]),
        &wow_data::trait_tree::TraitNodeGroupXTraitNodeStore::from_entries([]),
        &wow_data::trait_tree::TraitNodeXTraitCondStore::from_entries([]),
        &wow_data::trait_tree::TraitNodeXTraitCostStore::from_entries([]),
        &TraitNodeXTraitNodeEntryStore::from_entries([TraitNodeXTraitNodeEntryEntry {
            id: 501,
            trait_node_id: 100,
            trait_node_entry_id: 1000,
            index: 0,
        }]),
        &wow_data::trait_tree::TraitEdgeStore::from_entries([]),
        &wow_data::trait_tree::TraitCostStore::from_entries([]),
        &TraitCondStore::from_entries([
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
        ]),
        &TraitTreeLoadoutStore::from_entries([]),
        &TraitTreeLoadoutEntryStore::from_entries([]),
        &TraitTreeXTraitCostStore::from_entries([]),
    );

    let port = AuxiliaryLoadPortLikeCpp::new([
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::TraitEntries(vec![PlayerTraitEntryLoadRowLikeCpp {
                trait_config_id: Some(100),
                trait_node_id: Some(100),
                trait_node_entry_id: Some(1000),
                rank: Some(1),
                granted_ranks: Some(0),
            }]),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::TraitConfigs(vec![
                PlayerTraitConfigLoadRowLikeCpp {
                    id: Some(100),
                    config_type: Some(3),
                    chr_specialization_id: None,
                    combat_config_flags: None,
                    local_identifier: None,
                    skill_line_id: None,
                    trait_system_id: Some(7),
                    name: Some("Generic".to_owned()),
                },
            ]),
        ),
    ]);
    let (mut session, _, _) = make_session();
    session.set_trait_tree_skill_line_index(Arc::new(index));
    session.set_player_lifecycle_port_like_cpp(port);
    let guid = install_canonical_player_owner_for_test(&mut session, 0, 0);

    let configs = session
        .load_active_player_trait_configs_like_cpp(&node_entries, guid)
        .await;

    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].entries.len(), 1);
    assert_eq!(configs[0].entries[0].trait_node_id, 100);
    assert_eq!(configs[0].entries[0].trait_node_entry_id, 1000);
    assert_eq!(configs[0].entries[0].rank, 0);
    assert_eq!(configs[0].entries[0].granted_ranks, 1);
    let owned_configs = session
        .owned_trait_configs_for_create_like_cpp()
        .expect("canonical Player must retain the complete normalized trait rows");
    assert_eq!(owned_configs, configs);
}

#[tokio::test]
async fn malformed_trait_entry_keeps_authority_incomplete_without_suppressing_configs() {
    let port = AuxiliaryLoadPortLikeCpp::new([
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::TraitEntries(vec![PlayerTraitEntryLoadRowLikeCpp {
                trait_config_id: Some(100),
                trait_node_id: None,
                trait_node_entry_id: Some(300),
                rank: Some(2),
                granted_ranks: Some(1),
            }]),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::TraitConfigs(vec![
                PlayerTraitConfigLoadRowLikeCpp {
                    id: Some(100),
                    config_type: Some(2),
                    chr_specialization_id: None,
                    combat_config_flags: None,
                    local_identifier: None,
                    skill_line_id: Some(164),
                    trait_system_id: None,
                    name: Some("Blacksmithing".to_owned()),
                },
            ]),
        ),
    ]);
    let (mut session, _, _) = make_session();
    session.set_player_lifecycle_port_like_cpp(port);

    let configs = session
        .load_active_player_trait_configs_like_cpp(
            &wow_data::trait_tree::TraitNodeEntryStore::from_entries([]),
            ObjectGuid::create_player(1, 49),
        )
        .await;

    assert_eq!(configs.len(), 1);
    assert!(configs[0].entries.is_empty());
    assert!(
        !session
            .player_spell_test_fixture_like_cpp
            .represented_trait_config_rows_complete_like_cpp
    );
    assert!(
        !session
            .player_spell_test_fixture_like_cpp
            .represented_trait_entry_rows_complete_like_cpp
    );
}

#[tokio::test]
async fn failed_trait_entries_do_not_suppress_the_independent_config_query_like_cpp() {
    let port = AuxiliaryLoadPortLikeCpp::new([
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
            reason: "entry query failed".to_owned(),
        },
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::TraitConfigs(vec![
                PlayerTraitConfigLoadRowLikeCpp {
                    id: Some(101),
                    config_type: Some(3),
                    chr_specialization_id: None,
                    combat_config_flags: None,
                    local_identifier: None,
                    skill_line_id: None,
                    trait_system_id: Some(7),
                    name: Some("Generic".to_owned()),
                },
            ]),
        ),
    ]);
    let (mut session, _, _) = make_session();
    session.set_player_lifecycle_port_like_cpp(port.clone());

    let configs = session
        .load_active_player_trait_configs_like_cpp(
            &wow_data::trait_tree::TraitNodeEntryStore::from_entries([]),
            ObjectGuid::create_player(1, 50),
        )
        .await;

    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].trait_system_id, 7);
    assert!(
        !session
            .player_spell_test_fixture_like_cpp
            .represented_trait_config_rows_complete_like_cpp
    );
    assert_eq!(port.requests().len(), 2);
}
