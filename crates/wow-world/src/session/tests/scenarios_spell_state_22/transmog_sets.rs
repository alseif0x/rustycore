use super::*;

#[tokio::test]
async fn spell_learn_transmog_set_adds_valid_appearances_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 73);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let spell_id = 73_100;
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogSpellTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries([
        TransmogSetItemEntry {
            id: 1,
            transmog_set_id: 700,
            item_modified_appearance_id: 65,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 2,
            transmog_set_id: 700,
            item_modified_appearance_id: 999,
            flags: 0,
        },
    ])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([ItemModifiedAppearanceEntry {
            id: 65,
            item_id: 777,
            item_appearance_modifier_id: 0,
            item_appearance_id: 9_000,
            order_index: 0,
            transmog_source_type_enum: 0,
        }]),
    ));
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET,
                effect_misc_value_1: 700,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented learn transmog set should execute");

    assert!(session.represented_has_item_appearance_like_cpp(65));
    assert!(!session.represented_has_item_appearance_like_cpp(999));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_learn_transmog_set_skips_missing_set_or_non_player_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 74);
    let other_player_guid = ObjectGuid::create_player(1, 75);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let spell_id = 73_101;
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogSpellSkipTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries([])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([]),
    ));
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET,
                effect_misc_value_1: 700,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    for target_guid in [player_guid, other_player_guid] {
        session
            .execute_spell(spell_id, target_guid)
            .await
            .expect("represented learn transmog set guard should no-op");
        assert!(!session.represented_has_item_appearance_like_cpp(65));
        assert_eq!(
            drain_server_opcodes(&send_rx),
            vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
        );
    }
}
