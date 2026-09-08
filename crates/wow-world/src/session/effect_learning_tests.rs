use super::*;
use crate::spell_acquisition::*;

#[tokio::test]
async fn spell_learn_spell_effect_row_preserves_base_grant_without_richer_authority_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let (mut observer, _, _) = make_session();
    let spell_id = 750_i32;
    let learned_spell_id = 13_337_i32;
    let player_guid = ObjectGuid::create_player(1, 68);
    let observer_guid = ObjectGuid::create_player(1, 69);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    session.set_player_guid(Some(player_guid));
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([])));
    assert!(session.replace_complete_spell_acquisition_runtime_like_cpp(
        [],
        [],
        [],
        HashMap::new(),
        0,
        BTreeSet::new(),
    ));
    player_registry.register_or_replace(
        player_guid,
        broadcast_info_with_command(player_guid, registry_send_tx, session.session_command_tx()),
        Default::default(),
    );
    observer.set_player_guid(Some(observer_guid));
    observer.set_player_registry(Arc::clone(&player_registry));
    assert!(
        !observer
            .player_registry()
            .expect("observer shares the player registry")
            .loot_player_context(player_guid)
            .expect("source player is registered")
            .known_spells
            .contains(&learned_spell_id),
        "another session must not observe the fallback grant before it happens"
    );
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL,
                effect_trigger_spell: learned_spell_id,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(
        learned_spell_id,
        wow_data::SpellInfo {
            spell_id: learned_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented learn-spell row should execute");

    assert!(session.known_spells_like_cpp().contains(&learned_spell_id));
    assert!(
        observer
            .player_registry()
            .expect("observer shares the player registry")
            .loot_player_context(player_guid)
            .expect("source player remains registered")
            .known_spells
            .contains(&learned_spell_id),
        "the fallback grant must be visible to other sessions through the shared registry"
    );
    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes: Vec<_> = packets
        .iter()
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
        .collect();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::LearnedSpells,
            ServerOpcodes::CooldownEvent,
        ]
    );
    assert_eq!(packets.len(), 3);
    assert_eq!(
        session.represented_spell_acquisition_post_commit_actions_like_cpp(),
        &[
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id: learned_spell_id as u32,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: learned_spell_id as u32,
                favorite: false,
                suppress_messaging: false,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id: learned_spell_id as u32,
            },
        ]
    );
    let spell_rows = session
        .represented_player_spell_rows_like_cpp
        .values()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(
        spell_rows,
        vec![RepresentedPlayerSpellLikeCpp {
            spell_id: learned_spell_id,
            active: true,
            disabled: false,
            dependent: false,
            favorite: false,
            state: RepresentedPlayerSpellStateLikeCpp::New,
        }],
        "the shallow C++-faithful grant remains dirty in the complete in-world spell map"
    );
}

#[tokio::test]
async fn spell_learn_spell_fallback_rejects_mount_source_before_character_grant() {
    let (mut session, _, send_rx) = make_session();
    let learned_spell_id = 13_358_i32;
    let player_guid = ObjectGuid::create_player(1, 79);
    session.set_player_guid(Some(player_guid));
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 1,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: learned_spell_id,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    assert!(session.replace_complete_spell_acquisition_runtime_like_cpp(
        [],
        [],
        [],
        HashMap::new(),
        0,
        BTreeSet::new(),
    ));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        learned_spell_id,
        wow_data::SpellInfo {
            spell_id: learned_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    assert!(
        !session
            .apply_learn_spell_effect_like_cpp(learned_spell_id, player_guid)
            .await
    );

    assert!(!session.known_spells_like_cpp().contains(&learned_spell_id));
    assert!(session.represented_player_spell_rows_like_cpp.is_empty());
    assert!(
        session
            .represented_spell_acquisition_post_commit_actions_like_cpp()
            .is_empty()
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[tokio::test]
async fn spell_learn_spell_effect_row_rejects_missing_base_spell_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 755_i32;
    let missing_spell_id = 13_340_i32;
    let player_guid = ObjectGuid::create_player(1, 74);
    session.set_player_guid(Some(player_guid));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL,
                effect_trigger_spell: missing_spell_id,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("the parent spell still executes when its broken learn effect no-ops");

    assert!(!session.known_spells_like_cpp().contains(&missing_spell_id));
    assert!(
        session
            .represented_fallback_player_spell_rows_like_cpp
            .is_empty(),
        "C++ AddSpell does not leave a dirty row for a missing SpellInfo"
    );
    assert!(
        session
            .represented_spell_acquisition_post_commit_actions_like_cpp()
            .is_empty(),
        "invalid base spell metadata must not publish acquisition actions"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}

#[tokio::test]
async fn spell_learn_spell_fallback_preserves_disabled_inactive_state_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let learned_spell_id = 13_341_i32;
    let player_guid = ObjectGuid::create_player(1, 75);
    session.set_player_guid(Some(player_guid));
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_spell_required_store(Arc::new(wow_data::SpellRequiredStoreLikeCpp::default()));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([])));
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            RepresentedPlayerSpellLikeCpp {
                spell_id: learned_spell_id,
                active: false,
                disabled: true,
                dependent: false,
                favorite: true,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ])
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.set_complete_represented_override_spells_like_cpp([]));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        learned_spell_id,
        wow_data::SpellInfo {
            spell_id: learned_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    assert!(
        session
            .apply_learn_spell_effect_like_cpp(learned_spell_id, player_guid)
            .await
    );

    assert!(session.known_spells_like_cpp().contains(&learned_spell_id));
    assert_eq!(
        session
            .represented_player_spell_rows_like_cpp
            .get(&learned_spell_id),
        Some(&RepresentedPlayerSpellLikeCpp {
            spell_id: learned_spell_id,
            active: false,
            disabled: false,
            dependent: false,
            favorite: true,
            state: RepresentedPlayerSpellStateLikeCpp::Changed,
        }),
        "C++ Player::LearnSpell preserves active only when re-enabling a disabled row"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        Vec::<ServerOpcodes>::new(),
        "C++ AddSpell returns false after enabling a row whose preserved active bit is false"
    );
    assert_eq!(
        session.represented_spell_acquisition_post_commit_actions_like_cpp(),
        &[crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
            spell_id: learned_spell_id as u32,
        }],
        "C++ AddSpell reaches LearnOrKnow after enabling the row, but disabled LearnSpell skips the quest objective"
    );
}

#[tokio::test]
async fn spell_learn_spell_fallback_reactivates_inactive_known_spell_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let learned_spell_id = 13_343_i32;
    let player_guid = ObjectGuid::create_player(1, 77);
    session.set_player_guid(Some(player_guid));
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([])));
    assert!(session.replace_complete_spell_acquisition_runtime_like_cpp(
        [RepresentedPlayerSpellLikeCpp {
            spell_id: learned_spell_id,
            active: false,
            disabled: false,
            dependent: false,
            favorite: true,
            state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
        }],
        [],
        [],
        HashMap::new(),
        0,
        BTreeSet::new(),
    ));
    assert!(session.known_spells_like_cpp().contains(&learned_spell_id));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        learned_spell_id,
        wow_data::SpellInfo {
            spell_id: learned_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    assert!(
        session
            .apply_learn_spell_effect_like_cpp(learned_spell_id, player_guid)
            .await
    );

    assert_eq!(
        session
            .represented_player_spell_rows_like_cpp
            .get(&learned_spell_id),
        Some(&RepresentedPlayerSpellLikeCpp {
            spell_id: learned_spell_id,
            active: true,
            disabled: false,
            dependent: false,
            favorite: true,
            state: RepresentedPlayerSpellStateLikeCpp::Changed,
        }),
        "C++ Player::LearnSpell calls AddSpell(active=true) for an inactive non-disabled row"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::LearnedSpells],
        "C++ AddSpell returns true when it reactivates the row"
    );
    assert_eq!(
        session.represented_spell_acquisition_post_commit_actions_like_cpp(),
        &[
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: learned_spell_id as u32,
                favorite: true,
                suppress_messaging: false,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id: learned_spell_id as u32,
            },
        ],
        "the active-state early return skips AddSpell's LearnOrKnow tail, while non-disabled LearnSpell still advances its quest objective"
    );
}

#[test]
fn base_learn_spell_fallback_keeps_known_lower_rank_inactive_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let lower_spell_id = 13_344_i32;
    let higher_spell_id = 13_345_i32;
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: higher_spell_id as u32,
                supercedes_spell_id: lower_spell_id as u32,
            }],
            |_| true,
        ),
    ));
    assert!(session.replace_complete_spell_acquisition_runtime_like_cpp(
        [
            RepresentedPlayerSpellLikeCpp {
                spell_id: lower_spell_id,
                active: false,
                disabled: false,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
            RepresentedPlayerSpellLikeCpp {
                spell_id: higher_spell_id,
                active: true,
                disabled: false,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ],
        [],
        [],
        HashMap::new(),
        0,
        BTreeSet::new(),
    ));

    assert!(apply_base_learning_like_cpp(&mut session, lower_spell_id));

    assert_eq!(
        session
            .represented_player_spell_rows_like_cpp
            .get(&lower_spell_id)
            .map(|row| (row.active, row.state)),
        Some((false, RepresentedPlayerSpellStateLikeCpp::Unchanged)),
        "C++ AddSpell keeps a lower rank inactive when its next rank is known"
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
    assert_eq!(
        session.represented_spell_acquisition_post_commit_actions_like_cpp(),
        &[crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
            spell_id: lower_spell_id as u32,
        }],
        "unchanged AddSpell returns before LearnOrKnow, but non-disabled LearnSpell still advances the quest objective"
    );
}

#[test]
fn base_learn_spell_fallback_rejects_ranked_insertion_before_partial_mutation() {
    let (mut session, _, send_rx) = make_session();
    let lower_spell_id = 13_352_i32;
    let higher_spell_id = 13_353_i32;
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: higher_spell_id as u32,
                supercedes_spell_id: lower_spell_id as u32,
            }],
            |_| true,
        ),
    ));
    let lower_row = RepresentedPlayerSpellLikeCpp {
        spell_id: lower_spell_id,
        active: true,
        disabled: false,
        dependent: false,
        favorite: false,
        state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
    };
    assert!(session.set_complete_represented_player_spell_rows_like_cpp([lower_row]));

    assert!(!apply_base_learning_like_cpp(&mut session, higher_spell_id));

    assert_eq!(
        session
            .represented_player_spell_rows_like_cpp
            .get(&lower_spell_id),
        Some(&lower_row)
    );
    assert!(
        !session
            .represented_player_spell_rows_like_cpp
            .contains_key(&higher_spell_id)
    );
    assert!(
        session
            .represented_spell_acquisition_post_commit_actions_like_cpp()
            .is_empty(),
        "rank insertion must stop before C++ previous-rank and supersession work becomes partial"
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn base_learn_spell_fallback_allows_first_rank_insertion_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let first_rank = 13_354_i32;
    let second_rank = 13_355_i32;
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: second_rank as u32,
                supercedes_spell_id: first_rank as u32,
            }],
            |_| true,
        ),
    ));
    assert!(session.replace_complete_spell_acquisition_runtime_like_cpp(
        [],
        [],
        [],
        HashMap::new(),
        0,
        BTreeSet::new(),
    ));

    assert!(apply_base_learning_like_cpp(&mut session, first_rank));

    assert_eq!(
        session.represented_player_spell_rows_like_cpp[&first_rank],
        RepresentedPlayerSpellLikeCpp {
            spell_id: first_rank,
            active: true,
            disabled: false,
            dependent: false,
            favorite: false,
            state: RepresentedPlayerSpellStateLikeCpp::New,
        }
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::LearnedSpells]
    );
}

#[test]
fn base_learn_spell_fallback_reactivates_disabled_cpp_closure_in_order() {
    let (mut session, _, send_rx) = make_session();
    let root_spell = 13_346_i32;
    let next_spell = 13_347_i32;
    let requiring_spell = 13_348_i32;
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: next_spell as u32,
                supercedes_spell_id: root_spell as u32,
            }],
            |_| true,
        ),
    ));
    let mut required_store = wow_data::SpellRequiredStoreLikeCpp::default();
    required_store
        .required_by_spell_id
        .insert(requiring_spell as u32, vec![root_spell as u32]);
    required_store
        .requiring_by_required_spell_id
        .insert(root_spell as u32, vec![requiring_spell as u32]);
    session.set_spell_required_store(Arc::new(required_store));
    assert!(session.replace_complete_spell_acquisition_runtime_like_cpp(
        [
            RepresentedPlayerSpellLikeCpp {
                spell_id: root_spell,
                active: true,
                disabled: true,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
            RepresentedPlayerSpellLikeCpp {
                spell_id: next_spell,
                active: false,
                disabled: true,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
            RepresentedPlayerSpellLikeCpp {
                spell_id: requiring_spell,
                active: true,
                disabled: true,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ],
        [],
        [],
        HashMap::new(),
        0,
        BTreeSet::new(),
    ));

    assert!(apply_base_learning_like_cpp(&mut session, root_spell));

    let rows = &session.represented_player_spell_rows_like_cpp;
    assert!(rows.values().all(|row| !row.disabled));
    assert!(rows[&root_spell].active);
    assert!(!rows[&next_spell].active);
    assert!(rows[&requiring_spell].active);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::LearnedSpells, ServerOpcodes::LearnedSpells]
    );
    assert_eq!(
        session.represented_spell_acquisition_post_commit_actions_like_cpp(),
        &[
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id: root_spell as u32,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: root_spell as u32,
                favorite: false,
                suppress_messaging: false,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id: next_spell as u32,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id: requiring_spell as u32,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: requiring_spell as u32,
                favorite: false,
                suppress_messaging: false,
            },
        ]
    );
}

#[test]
fn base_learn_spell_fallback_clears_trait_override_before_reactivation_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 13_356_i32;
    let overridden_spell_id = 13_357_i32;
    let trait_definition_id = 77_i32;
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_spell_required_store(Arc::new(wow_data::SpellRequiredStoreLikeCpp::default()));
    session.set_trait_definition_store(Arc::new(
        wow_data::trait_tree::TraitDefinitionStore::from_entries([
            wow_data::trait_tree::TraitDefinitionEntry {
                id: trait_definition_id as u32,
                override_name: String::new(),
                override_subtext: String::new(),
                override_description: String::new(),
                spell_id: 0,
                override_icon: 0,
                overrides_spell_id: overridden_spell_id,
                visible_spell_id: 0,
            },
        ]),
    ));
    assert!(session.replace_complete_spell_acquisition_runtime_like_cpp(
        [RepresentedPlayerSpellLikeCpp {
            spell_id,
            active: true,
            disabled: true,
            dependent: false,
            favorite: false,
            state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
        }],
        [(spell_id, trait_definition_id)],
        [(overridden_spell_id, spell_id)],
        HashMap::new(),
        0,
        BTreeSet::new(),
    ));

    assert!(apply_base_learning_like_cpp(&mut session, spell_id));

    assert!(
        !session
            .represented_spell_trait_definition_ids_like_cpp
            .contains_key(&spell_id)
    );
    assert!(
        !session
            .represented_override_spells_like_cpp
            .get(&overridden_spell_id)
            .is_some_and(|spells| spells.contains(&spell_id))
    );
    assert!(!session.represented_player_spell_rows_like_cpp[&spell_id].disabled);
}

#[test]
fn learn_spell_capacity_rejection_cannot_enter_shallow_fallback() {
    assert!(!may_shallow_fallback_after_profession_plan_error_like_cpp(
        crate::profession::PrimaryProfessionCapacityPlanErrorLikeCpp::CapacityExceeded {
            configured_max: 2,
            used: 2,
            requested_new: 1,
        },
    ));
    assert!(may_shallow_fallback_after_profession_plan_error_like_cpp(
        crate::profession::PrimaryProfessionCapacityPlanErrorLikeCpp::MissingPlayerSkillSnapshot,
    ));
    assert!(!may_shallow_fallback_after_profession_plan_error_like_cpp(
        crate::profession::PrimaryProfessionCapacityPlanErrorLikeCpp::InvalidConfiguredMaximum {
            configured: 3,
        },
    ));
}

#[test]
fn base_learn_spell_fallback_replaces_temporary_row_with_durable_new_row_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 13_350_i32;
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            RepresentedPlayerSpellLikeCpp {
                spell_id,
                active: true,
                disabled: false,
                dependent: false,
                favorite: true,
                state: RepresentedPlayerSpellStateLikeCpp::Temporary,
            },
        ])
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.set_complete_represented_override_spells_like_cpp([]));

    assert!(apply_base_learning_like_cpp(&mut session, spell_id));

    assert_eq!(
        session
            .represented_player_spell_rows_like_cpp
            .get(&spell_id),
        Some(&RepresentedPlayerSpellLikeCpp {
            spell_id,
            active: true,
            disabled: false,
            dependent: false,
            favorite: true,
            state: RepresentedPlayerSpellStateLikeCpp::New,
        }),
        "C++ AddSpell removes PLAYERSPELL_TEMPORARY before inserting a durable new row"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::LearnedSpells]
    );
    assert_eq!(
        session.represented_spell_acquisition_post_commit_actions_like_cpp(),
        &[
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id: spell_id as u32,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: spell_id as u32,
                favorite: true,
                suppress_messaging: false,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id: spell_id as u32,
            },
        ]
    );
}

#[test]
fn base_learn_spell_fallback_rejects_disabled_row_without_dependency_authority() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 13_349_i32;
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    let original_row = RepresentedPlayerSpellLikeCpp {
        spell_id,
        active: true,
        disabled: true,
        dependent: false,
        favorite: false,
        state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
    };
    assert!(session.set_complete_represented_player_spell_rows_like_cpp([original_row]));
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.set_complete_represented_override_spells_like_cpp([]));

    assert!(!apply_base_learning_like_cpp(&mut session, spell_id));

    assert_eq!(
        session
            .represented_player_spell_rows_like_cpp
            .get(&spell_id),
        Some(&original_row),
        "without both C++ dependency stores the fallback must fail before enabling the row"
    );
    assert!(
        session
            .represented_spell_acquisition_post_commit_actions_like_cpp()
            .is_empty()
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[tokio::test]
async fn spell_learn_spell_fallback_defers_without_complete_spell_rows_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let learned_spell_id = 13_342_i32;
    let player_guid = ObjectGuid::create_player(1, 76);
    session.set_player_guid(Some(player_guid));
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        learned_spell_id,
        wow_data::SpellInfo {
            spell_id: learned_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    assert!(
        !session
            .apply_learn_spell_effect_like_cpp(learned_spell_id, player_guid)
            .await
    );
    assert!(
        session
            .represented_fallback_player_spell_rows_like_cpp
            .is_empty(),
        "an unknown durable row must not be guessed into a targeted UPSERT overlay"
    );
    assert!(
        !session.known_spells_like_cpp().contains(&learned_spell_id),
        "runtime publication must also wait for the complete C++ PlayerSpellMap authority"
    );
    assert!(
        session
            .represented_spell_acquisition_post_commit_actions_like_cpp()
            .is_empty()
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn fallback_reconciliation_preserves_dependent_promotion_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 13_351_i32;
    session
        .represented_fallback_player_spell_rows_like_cpp
        .insert(
            spell_id,
            RepresentedPlayerSpellLikeCpp {
                spell_id,
                active: true,
                disabled: false,
                dependent: true,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::New,
            },
        );

    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            RepresentedPlayerSpellLikeCpp {
                spell_id,
                active: true,
                disabled: false,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ])
    );

    let reconciled = session.represented_player_spell_rows_like_cpp[&spell_id];
    assert!(reconciled.dependent);
    assert_eq!(
        reconciled.state,
        RepresentedPlayerSpellStateLikeCpp::Changed,
        "C++ AddSpell promotes an existing independent row and marks it changed"
    );
}

#[tokio::test]
async fn spell_learn_spell_effect_row_does_not_duplicate_known_spell_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 751_i32;
    let learned_spell_id = 13_338_i32;
    let player_guid = ObjectGuid::create_player(1, 69);
    session.set_player_guid(Some(player_guid));
    session.set_known_spells_like_cpp(vec![learned_spell_id]);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL,
                effect_trigger_spell: learned_spell_id,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("duplicate represented learn-spell row should execute as no-op");

    assert_eq!(session.known_spells_like_cpp(), &[learned_spell_id]);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent],
        "C++ Player::LearnSpell sends LearnedSpells only when AddSpell reports a new learned entry"
    );
}

#[tokio::test]
async fn spell_learn_spell_effect_row_requires_current_player_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 752_i32;
    let learned_spell_id = 13_339_i32;
    let player_guid = ObjectGuid::create_player(1, 70);
    let other_player_guid = ObjectGuid::create_player(1, 71);
    session.set_player_guid(Some(player_guid));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL,
                effect_trigger_spell: learned_spell_id,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, other_player_guid)
        .await
        .expect("represented learn-spell non-current player target should no-op");

    assert!(!session.known_spells_like_cpp().contains(&learned_spell_id));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
