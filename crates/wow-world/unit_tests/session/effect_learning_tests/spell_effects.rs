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
        .player_spell_test_fixture_like_cpp
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
    assert!(
        session
            .player_spell_test_fixture_like_cpp
            .represented_player_spell_rows_like_cpp
            .is_empty()
    );
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
            .player_spell_test_fixture_like_cpp
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
            .player_spell_test_fixture_like_cpp
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
            .player_spell_test_fixture_like_cpp
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
