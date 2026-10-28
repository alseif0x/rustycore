//! Player inebriate effect scenarios.

use super::*;

const FAKE_INEBRIATE_AURA_TYPE: i32 = 304;

fn mark_player_aura_authority_complete(session: &mut crate::session::WorldSession) {
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
}

fn install_realm_output(session: &mut crate::session::WorldSession) -> flume::Receiver<Vec<u8>> {
    let (sender, receiver) = flume::unbounded();
    session.core.install_realm_send_channel_for_test(sender);
    receiver
}

fn install_fake_inebriation_aura(
    session: &mut crate::session::WorldSession,
    spell_id: i32,
    slot: u8,
    current_amount: Option<i32>,
) {
    let mut spell_store =
        crate::selected_spell_fixtures::canonical_spell_store([(spell_id as u32, 0)]);
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
                effect_aura: FAKE_INEBRIATE_AURA_TYPE,
                effect_base_points: 99,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_spell_group_store(Arc::new(wow_data::SpellGroupStoreLikeCpp::default()));
    session.set_spell_group_stack_rule_store(Arc::new(
        wow_data::SpellGroupStackRuleStoreLikeCpp::default(),
    ));

    let caster_guid = ObjectGuid::create_player(1, i64::from(spell_id));
    session
        .mutate_canonical_player_like_cpp(|player| {
            let application = wow_entities::AuraApplicationLikeCpp {
                spell_id,
                difficulty_id: 0,
                caster_guid,
                slot,
                duration_total: 10_000,
                duration_remaining: 10_000,
                stack_count: 1,
                aura_flags: 0,
                effect_mask: 1,
                aura_interrupt_flags: 0,
                aura_interrupt_flags2: 0,
                represented_effect: None,
                represented_amount: current_amount.unwrap_or_default(),
                represented_effect_amounts: current_amount
                    .map(|amount| {
                        vec![wow_entities::RepresentedAuraEffectAmountLikeCpp {
                            effect_index: 0,
                            amount,
                        }]
                    })
                    .unwrap_or_default(),
                represented_misc_value: None,
                represented_multiplier: 1.0,
                applied_at: std::time::Instant::now(),
            };
            let applied = wow_entities::AppliedAuraRef::new(
                u32::try_from(spell_id).expect("positive fake Aura spell ID"),
                caster_guid,
                slot,
                1,
            );
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            auras.insert_runtime_application_like_cpp(application);
            // This legacy amount exists, but the native query must read the
            // current AuraEffect amount from the indexed application fields.
            auras.register_applied_aura_effect_like_cpp(
                applied,
                FAKE_INEBRIATE_AURA_TYPE,
                current_amount.unwrap_or(77),
                0,
            );
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
}

#[tokio::test]
async fn spell_inebriate_effect_increases_current_player_drunk_value_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let realm_rx = install_realm_output(&mut session);
    let spell_id = 802_i32;
    let player_guid = ObjectGuid::create_player(1, 802);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().world_mut().object_mut().add_to_world();
            player.clear_data_changes();
            player.set_inebriation_like_cpp(15);
        })
        .unwrap();

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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE,
                effect_base_points: 35,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    mark_player_aura_authority_complete(&mut session);

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectInebriate should execute");

    let inebriation = session
        .mutate_canonical_player_like_cpp(|player| player.inebriation_like_cpp())
        .unwrap();
    assert_eq!(inebriation, 50);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
    assert_eq!(
        realm_rx.try_recv().unwrap(),
        wow_packet::packets::misc::CrossedInebriationThreshold {
            guid: player_guid,
            threshold: 2,
            item_id: 0,
        }
        .to_bytes()
    );

    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .expect("canonical MapManager")
        .clone();
    {
        let mut manager = manager.lock().unwrap();
        let player = manager
            .find_map_mut(0, 0)
            .expect("current World map")
            .map_mut()
            .get_typed_player_mut(player_guid)
            .expect("native Player is owned by the current Map");
        let object = player.unit().world().object();
        assert!(
            object.is_object_updated(),
            "SetDrunkValue dirties native PlayerData"
        );
        assert!(object.is_need_notify(wow_entities::ObjectNotifyFlags::VISIBILITY_CHANGED));

        let _ = manager.update(1);
        let map = manager.find_map(0, 0).expect("current World map");
        let updates = map
            .last_send_object_updates_summary_like_cpp()
            .player_values_updates;
        assert_eq!(
            updates.len(),
            1,
            "Map captures one final Player values update"
        );
        assert_eq!(updates[0].guid, player_guid);
        let player_data = updates[0]
            .values_update
            .player_data
            .as_ref()
            .expect("native Map snapshot contains PlayerData");
        assert_eq!(player_data.values.inebriation, 50);
        assert!(
            player_data
                .mask
                .is_set(wow_entities::PLAYER_DATA_INEBRIATION_BIT)
        );
        assert!(
            !map.map()
                .get_typed_player(player_guid)
                .unwrap()
                .unit()
                .world()
                .object()
                .is_object_updated(),
            "Map consumes native Player update masks once"
        );
    }
    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        1,
        "the World consumer sends the Map-owned snapshot once"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        0,
        "a second consumer pass cannot duplicate the consumed snapshot"
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn spell_inebriate_effect_clamps_to_hundred_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let realm_rx = install_realm_output(&mut session);
    let spell_id = 803_i32;
    let player_guid = ObjectGuid::create_player(1, 803);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_inebriation_like_cpp(90);
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE,
            effect_base_points: 30,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    mark_player_aura_authority_complete(&mut session);

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented primary EffectInebriate should execute");

    let inebriation = session
        .mutate_canonical_player_like_cpp(|player| player.inebriation_like_cpp())
        .unwrap();
    assert_eq!(inebriation, 100);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
    assert!(realm_rx.is_empty(), "90→100 stays inside Smashed");
}
#[tokio::test]
async fn spell_inebriate_effect_negative_amount_sobers_and_clamps_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let realm_rx = install_realm_output(&mut session);
    let spell_id = 804_i32;
    let player_guid = ObjectGuid::create_player(1, 804);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_inebriation_like_cpp(20);
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE,
            effect_base_points: -30,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    mark_player_aura_authority_complete(&mut session);

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("negative represented EffectInebriate should execute");

    let inebriation = session
        .mutate_canonical_player_like_cpp(|player| player.inebriation_like_cpp())
        .unwrap();
    assert_eq!(inebriation, 0);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
    assert_eq!(
        realm_rx.try_recv().unwrap(),
        wow_packet::packets::misc::CrossedInebriationThreshold {
            guid: player_guid,
            threshold: 0,
            item_id: 0,
        }
        .to_bytes()
    );
}
#[tokio::test]
async fn spell_inebriate_effect_ignores_non_player_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let realm_rx = install_realm_output(&mut session);
    let spell_id = 805_i32;
    let player_guid = ObjectGuid::create_player(1, 805);
    let creature_guid = test_creature_guid(18_805);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    register_test_creature(&mut session, shared_map_manager(), creature_guid, 40);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_inebriation_like_cpp(10);
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE,
            effect_base_points: 40,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    mark_player_aura_authority_complete(&mut session);

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("non-player target represented EffectInebriate should execute as C++ no-op");

    let inebriation = session
        .mutate_canonical_player_like_cpp(|player| player.inebriation_like_cpp())
        .unwrap();
    assert_eq!(inebriation, 10);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
    assert!(
        realm_rx.is_empty(),
        "non-player target has no inebriation packet"
    );
}

#[test]
fn direct_native_drunk_set_uses_owned_fake_aura_facts_and_u32_item_bits() {
    let (mut session, _, send_rx) = make_session();
    let realm_rx = install_realm_output(&mut session);
    let player_guid = ObjectGuid::create_player(1, 806);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_inebriation_like_cpp(25);
            player.unit_mut().set_invisibility_detect_like_cpp(6, 25);
        })
        .unwrap();
    install_fake_inebriation_aura(&mut session, 18_006, 7, Some(0));

    let changed = wow_world_application::set_player_drunk_value_like_cpp(
        &session,
        player_guid,
        wow_world_application::PlayerDrunkValueInput::Set(0),
        Some(ObjectGuid::create_item(1, 806)),
        Some(u32::MAX),
    )
    .expect("native SetDrunkValue should use the owned Player and complete Aura view");

    assert!(changed, "the return value reports thresholdChanged");
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.inebriation_like_cpp())
            .unwrap(),
        0
    );
    assert_eq!(
        realm_rx.try_recv().unwrap(),
        wow_packet::packets::misc::CrossedInebriationThreshold {
            guid: player_guid,
            threshold: 0,
            item_id: -1,
        }
        .to_bytes()
    );

    let changed = wow_world_application::set_player_drunk_value_like_cpp(
        &session,
        player_guid,
        wow_world_application::PlayerDrunkValueInput::Set(50),
        None,
        Some(u32::MAX),
    )
    .expect("an entry without a physical cast item is not an item ID");
    assert!(changed);
    assert_eq!(
        realm_rx.try_recv().unwrap(),
        wow_packet::packets::misc::CrossedInebriationThreshold {
            guid: player_guid,
            threshold: 2,
            item_id: 0,
        }
        .to_bytes()
    );
    assert!(
        send_rx.is_empty(),
        "native threshold uses the realm connection"
    );
}

#[test]
fn unavailable_native_fake_aura_amount_and_item_metadata_do_not_write_player() {
    let (mut session, _, send_rx) = make_session();
    let realm_rx = install_realm_output(&mut session);
    let player_guid = ObjectGuid::create_player(1, 807);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_inebriation_like_cpp(31);
            player.unit_mut().set_invisibility_detect_like_cpp(6, 31);
        })
        .unwrap();

    let unknown_aura_source = wow_world_application::set_player_drunk_value_like_cpp(
        &session,
        player_guid,
        wow_world_application::PlayerDrunkValueInput::Set(0),
        None,
        None,
    );
    assert!(unknown_aura_source.is_err());
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.inebriation_like_cpp())
            .unwrap(),
        31
    );

    install_fake_inebriation_aura(&mut session, 18_007, 8, None);

    let missing_item_entry = wow_world_application::set_player_drunk_value_like_cpp(
        &session,
        player_guid,
        wow_world_application::PlayerDrunkValueInput::Set(0),
        Some(ObjectGuid::create_item(1, 807)),
        None,
    );
    assert!(missing_item_entry.is_err());

    let missing_current_amount = wow_world_application::set_player_drunk_value_like_cpp(
        &session,
        player_guid,
        wow_world_application::PlayerDrunkValueInput::Set(0),
        None,
        None,
    );
    assert!(missing_current_amount.is_err());
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.inebriation_like_cpp())
            .unwrap(),
        31
    );
    assert!(realm_rx.is_empty());
    assert!(send_rx.is_empty());
}

#[test]
fn effect_delta_overflow_is_rejected_before_native_player_write() {
    let (mut session, _, send_rx) = make_session();
    let realm_rx = install_realm_output(&mut session);
    let player_guid = ObjectGuid::create_player(1, 808);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_inebriation_like_cpp(1);
        })
        .unwrap();
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_spell_group_store(Arc::new(wow_data::SpellGroupStoreLikeCpp::default()));
    session.set_spell_group_stack_rule_store(Arc::new(
        wow_data::SpellGroupStackRuleStoreLikeCpp::default(),
    ));
    mark_player_aura_authority_complete(&mut session);

    let overflow = wow_world_application::set_player_drunk_value_like_cpp(
        &session,
        player_guid,
        wow_world_application::PlayerDrunkValueInput::EffectDelta(i32::MAX),
        None,
        None,
    );
    assert!(overflow.is_err());
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.inebriation_like_cpp())
            .unwrap(),
        1
    );
    assert!(realm_rx.is_empty());
    assert!(send_rx.is_empty());
}
