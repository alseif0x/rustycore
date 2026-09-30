//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn spell_acquisition_catalog_arc_is_shared_with_session() {
    let (mut session, _, _) = make_session();
    let catalog = Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            std::iter::empty(),
            wow_data::EffectiveSpellAcquisitionRowsLikeCpp::default(),
            wow_data::SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    );

    session.set_spell_acquisition_catalog(Arc::clone(&catalog));

    let catalogs = &session.spell_catalogs;
    let installed = catalogs
        .spell_acquisition_catalog()
        .expect("the process-wide catalog must be installed");
    assert!(Arc::ptr_eq(&catalog, installed));
}
#[test]
fn spell_proc_entry_prefers_exact_difficulty_like_cpp() {
    let (mut session, _, _) = make_session();
    let mut entries = BTreeMap::new();
    entries.insert(
        wow_data::SpellProcKeyLikeCpp {
            spell_id: 42,
            difficulty: 1,
        },
        test_spell_proc_entry_like_cpp(25.0),
    );
    entries.insert(
        wow_data::SpellProcKeyLikeCpp {
            spell_id: 42,
            difficulty: 2,
        },
        test_spell_proc_entry_like_cpp(75.0),
    );
    session.set_spell_proc_store(Arc::new(wow_data::SpellProcStoreLikeCpp {
        proc_entries_by_spell_and_difficulty: entries,
    }));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        DifficultyEntry {
            id: 2,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
        DifficultyEntry {
            id: 1,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ])));

    let entry = session
        .spell_proc_entry_like_cpp(42, 2)
        .expect("exact proc entry");

    assert_eq!(entry.chance, 75.0);
}
#[test]
fn spell_proc_entry_walks_difficulty_fallback_like_cpp() {
    let (mut session, _, _) = make_session();
    let mut entries = BTreeMap::new();
    entries.insert(
        wow_data::SpellProcKeyLikeCpp {
            spell_id: 42,
            difficulty: 1,
        },
        test_spell_proc_entry_like_cpp(25.0),
    );
    session.set_spell_proc_store(Arc::new(wow_data::SpellProcStoreLikeCpp {
        proc_entries_by_spell_and_difficulty: entries,
    }));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        DifficultyEntry {
            id: 3,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 2,
            toggle_difficulty_id: 0,
        },
        DifficultyEntry {
            id: 2,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
        DifficultyEntry {
            id: 1,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ])));

    let entry = session
        .spell_proc_entry_like_cpp(42, 3)
        .expect("fallback proc entry");

    assert_eq!(entry.chance, 25.0);
}
#[test]
fn next_spell_in_chain_returns_zero_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(session.next_spell_in_chain_like_cpp(10), 0);
}
#[test]
fn prev_spell_in_chain_returns_zero_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(session.prev_spell_in_chain_like_cpp(10), 0);
}
#[test]
fn first_spell_in_chain_returns_input_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(session.first_spell_in_chain_like_cpp(10), 10);
}

#[test]
fn spell_required_queries_return_empty_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.spells_required_for_spell_like_cpp(100).is_empty());
    assert!(session.spells_requiring_spell_like_cpp(10).is_empty());
    assert!(!session.is_spell_requiring_spell_like_cpp(100, 10));
}
#[test]
fn spell_required_queries_match_forward_reverse_maps_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_required_store(Arc::new(test_spell_required_store_like_cpp()));

    assert_eq!(session.spells_required_for_spell_like_cpp(100), &[10, 11]);
    assert_eq!(session.spells_requiring_spell_like_cpp(10), &[100, 101]);
    assert!(session.is_spell_requiring_spell_like_cpp(100, 10));
    assert!(!session.is_spell_requiring_spell_like_cpp(11, 100));
}





#[test]
fn remove_known_spell_sends_unlearned_spells_with_suppress_flag_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_known_spells_like_cpp(vec![20, 40]);

    session.remove_known_spell_with_suppress_messaging_like_cpp(20, true);

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::UnlearnedSpells as u16
    );
    assert_eq!(packet.read_uint32().expect("count"), 1);
    assert_eq!(packet.read_uint32().expect("spell id"), 20);
    assert!(packet.read_bit().expect("SuppressMessaging"));
    packet.flush_bits();
    assert!(packet.is_empty());
}
#[test]
fn remove_known_spell_sends_superceded_packet_when_previous_rank_reactivates_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 20,
                supercedes_spell_id: 10,
            }],
            |_| true,
        ),
    ));
    session.set_known_spells_like_cpp(vec![10, 20]);

    session.remove_known_spell_with_suppress_messaging_like_cpp(20, true);

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::SupercededSpells as u16
    );
    assert_eq!(packet.read_uint32().expect("count"), 1);
    assert_eq!(packet.read_int32().expect("new spell id"), 10);
    assert!(!packet.read_bit().expect("IsFavorite"));
    assert!(!packet.read_bit().expect("field_8.HasValue"));
    assert!(packet.read_bit().expect("Superceded.HasValue"));
    assert!(!packet.read_bit().expect("TraitDefinitionID.HasValue"));
    packet.flush_bits();
    assert_eq!(packet.read_int32().expect("old spell id"), 20);
    assert!(
        packet.is_empty(),
        "C++ sends SendSupercededSpell instead of UnlearnedSpells when the lower rank reactivates"
    );
}

#[test]
fn spell_unlearn_adapter_preserves_complete_rows_and_trait_cleanup_before_notice() {
    let (mut session, send_rx, _) = make_session();
    session.set_known_spells_like_cpp(vec![10]);
    assert!(session.set_complete_represented_player_spell_rows_like_cpp([
        RepresentedPlayerSpellLikeCpp {
            spell_id: 10, active: true, disabled: false, dependent: false,
            favorite: true, state: RepresentedPlayerSpellStateLikeCpp::New,
        },
    ]));
    session.set_trait_definition_store(Arc::new(TraitDefinitionStore::from_entries([
        wow_data::trait_tree::TraitDefinitionEntry {
            id: 7, override_name: String::new(), override_subtext: String::new(),
            override_description: String::new(), spell_id: 10, override_icon: 0,
            overrides_spell_id: 100, visible_spell_id: 0,
        },
    ])));
    session.add_represented_override_spell_like_cpp(100, 10);
    session.set_represented_spell_trait_definition_id_like_cpp(10, 7);

    session.remove_known_spell_with_suppress_messaging_like_cpp(10, true);

    let rows = session.complete_represented_player_spell_rows_like_cpp().unwrap();
    assert_eq!(rows.get(&10), Some(&RepresentedPlayerSpellLikeCpp {
        spell_id: 10, active: false, disabled: false, dependent: false,
        favorite: false, state: RepresentedPlayerSpellStateLikeCpp::Removed,
    }));
    assert!(!session.represented_override_spells_like_cpp().contains_key(&100));
    assert!(!session.represented_spell_trait_definition_ids_like_cpp().contains_key(&10));
    assert_eq!(send_rx.try_recv().unwrap(),
        wow_packet::packets::trainer::UnlearnedSpells::single(10, true).to_bytes());
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn spell_unlearn_adapter_reactivation_retains_learning_invalidation_and_dependency_writer() {
    let (mut session, send_rx, _) = make_session();
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp { spell_id: 20, supercedes_spell_id: 10 }],
            |_| true,
        ),
    ));
    session.set_known_spells_like_cpp(vec![10, 20]);
    assert!(session.set_complete_represented_player_spell_rows_like_cpp([
        RepresentedPlayerSpellLikeCpp {
            spell_id: 10, active: true, disabled: false, dependent: true,
            favorite: false, state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
        },
        RepresentedPlayerSpellLikeCpp {
            spell_id: 20, active: true, disabled: false, dependent: false,
            favorite: false, state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
        },
    ]));

    session.remove_known_spell_like_cpp(20);

    assert_eq!(session.known_spells_like_cpp(), &[10]);
    assert!(session.complete_represented_player_spell_rows_like_cpp().is_none());
    assert!(!session.represented_player_spell_rows_loaded_like_cpp());
    assert!(!session.represented_dependent_known_spells_like_cpp().contains(&10));
    assert_eq!(send_rx.try_recv().unwrap(),
        wow_packet::packets::trainer::SupercededSpells::single(20, 10).to_bytes());
    assert!(send_rx.try_recv().is_err(), "reactivation suppresses the UnlearnedSpells packet");
}

#[test]
fn loaded_skill_gain_preserves_none_guid_coverage_priority() {
    let (mut session, _, _) = make_session();
    assert_eq!(session.player_guid(), None);
    assert!(!session.apply_loaded_spell_learn_skills_like_cpp(&[10]));
    session.set_spell_learn_skill_store(Arc::new(wow_data::SpellLearnSkillStoreLikeCpp {
        covered_spell_ids: BTreeSet::from([10]),
        ..Default::default()
    }));
    assert!(session.apply_loaded_spell_learn_skills_like_cpp(&[10]), "CoveredWithoutNode skips all Player reads");
    assert!(!session.apply_loaded_spell_learn_skills_like_cpp(&[-1, 10]), "signed conversion precedes coverage");
}

#[test]
fn loaded_skill_gain_uses_canonical_owner_before_fixture_rows() {
    let (mut session, _, _) = make_session();
    session.set_player_skill_records_like_cpp(HashMap::from([(755, RepresentedPlayerSkillLikeCpp {
        skill_id: 755, step: 1, value: 12, max: 75, profession_slot: -1,
        state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
    })]));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.with_owned_player_mut_like_cpp(|player| {
        player.replace_skill_records_like_cpp(vec![wow_entities::PlayerSkillRecord {
            skill_line_id: 755, step: 1, current_value: 100, max_value: 150,
            profession_slot: -1, state: wow_entities::PlayerSkillLoadState::Unchanged,
        }], true, true, Some(1), BTreeSet::new());
    }).unwrap();
    session.set_spell_learn_skill_store(Arc::new(wow_data::SpellLearnSkillStoreLikeCpp {
        skill_by_spell_id: BTreeMap::from([(10, wow_data::SpellLearnSkillNodeLikeCpp {
            skill: 755, step: 2, value: 75, maxvalue: 150,
        })]),
        ..Default::default()
    }));
    assert!(session.apply_loaded_spell_learn_skills_like_cpp(&[10]));
    assert_eq!(session.resolved_player_skill_value_like_cpp(755), Some(100));
    assert_eq!(session.complete_player_skill_occupied_slots_like_cpp(), Some(1));
    assert_eq!(session.player_skill_test_fixture_like_cpp.player_skill_records_like_cpp[&755].value, 12, "fixture rows cannot replace canonical authority");
}

#[test]
fn loaded_spell_reconstruction_ignores_stale_flags_and_override_writers() {
    let (mut session, _, _) = make_session();
    session.set_spell_learn_spell_store(Arc::new(wow_data::SpellLearnSpellStoreLikeCpp {
        learned_by_spell_id: BTreeMap::from([(10, vec![wow_data::SpellLearnSpellNodeLikeCpp {
            spell: 20, overrides_spell: 100, active: true, auto_learned: false,
        }])]),
    }));
    session.set_spell_learn_skill_store(Arc::new(wow_data::SpellLearnSkillStoreLikeCpp {
        skill_by_spell_id: BTreeMap::from([(20, wow_data::SpellLearnSkillNodeLikeCpp {
            skill: 755, step: 2, value: 75, maxvalue: 150,
        })]),
        ..Default::default()
    }));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(wow_map::MapManager::default())));
    assert!(session.player_handle_like_cpp.is_some());
    let mut known = vec![10];
    assert_eq!(session.apply_loaded_known_spell_dependencies_like_cpp(&mut known), 1);
    assert_eq!(known, vec![10, 20], "failed owner writers do not undo append/enqueue/count");
    assert_eq!(session.resolved_player_skill_value_like_cpp(755), None);
    assert!(!session.apply_loaded_spell_learn_skills_like_cpp(&[20]));
    assert_eq!(session.player_skill_test_fixture_like_cpp.player_skill_records_like_cpp.len(), 0, "Some stale handle never enables ownerless fixtures");
}
