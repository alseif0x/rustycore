use super::*;

#[test]
fn toy_item_spell_effect_guard_uses_item_effect_parent_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_item_effect_store(Arc::new(ItemEffectStore::from_entries([ItemEffectEntry {
        id: 1,
        legacy_slot_index: 0,
        trigger_type: 0,
        charges: 0,
        cooldown_msec: 0,
        category_cooldown_msec: 0,
        spell_category_id: 0,
        spell_id: 12_345,
        chr_specialization_id: 0,
        parent_item_id: 30_000,
    }])));

    assert!(session.toy_item_has_spell_effect_like_cpp(30_000, 12_345));
    assert!(!session.toy_item_has_spell_effect_like_cpp(30_000, 54_321));
    assert!(!session.toy_item_has_spell_effect_like_cpp(30_001, 12_345));
}
#[test]
fn toy_item_spell_cooldown_uses_item_effect_override_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 12_345;
    let spell_info = wow_data::SpellInfo {
        recovery_time_ms: 1_500,
        cooldown_ms: 2_000,
        ..instant_toy_spell_info_like_cpp(spell_id)
    };

    session.set_item_effect_store(Arc::new(ItemEffectStore::from_entries([
        ItemEffectEntry {
            id: 1,
            legacy_slot_index: 0,
            trigger_type: 0,
            charges: 0,
            cooldown_msec: 5_000,
            category_cooldown_msec: 0,
            spell_category_id: 0,
            spell_id,
            chr_specialization_id: 0,
            parent_item_id: 30_000,
        },
        ItemEffectEntry {
            id: 2,
            legacy_slot_index: 0,
            trigger_type: 0,
            charges: 0,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id,
            chr_specialization_id: 0,
            parent_item_id: 30_001,
        },
    ])));

    assert_eq!(
        session.toy_item_spell_cooldown_ms_like_cpp(30_000, spell_id, &spell_info),
        5_000
    );
    assert_eq!(
        session.toy_item_spell_cooldown_ms_like_cpp(30_001, spell_id, &spell_info),
        2_000
    );
}
#[test]
fn handle_use_toy_sends_prepare_and_toy_cast_flags_like_cpp() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (mut session, _, send_rx) = make_session();
            let player_guid = ObjectGuid::create_player(1, 31_100);
            let client_cast_id = ObjectGuid::create_player(1, 31_101);
            let item_id = 30_000;
            let spell_id = 12_345;

            session.set_player_guid(Some(player_guid));
            session.set_player_map_position_like_cpp(571, Position::new(10.0, 10.0, 0.0, 0.0));
            install_stackable_test_item_template(&mut session, item_id, 1);
            session.load_represented_account_toys_like_cpp([(item_id, false, false)]);
            session.set_item_effect_store(Arc::new(ItemEffectStore::from_entries([
                ItemEffectEntry {
                    id: 1,
                    legacy_slot_index: 0,
                    trigger_type: 0,
                    charges: 0,
                    cooldown_msec: 0,
                    category_cooldown_msec: 0,
                    spell_category_id: 0,
                    spell_id,
                    chr_specialization_id: 0,
                    parent_item_id: item_id,
                },
            ])));
            let mut spell_store = SpellStore::new();
            spell_store.insert(spell_id, instant_toy_spell_info_like_cpp(spell_id));
            session.set_spell_store(Arc::new(spell_store));

            session
                .handle_use_toy(write_minimal_use_toy_packet_like_cpp(
                    item_id,
                    spell_id,
                    client_cast_id,
                ))
                .await;

            let prepare = send_rx.try_recv().expect("SpellPrepare packet");
            assert_eq!(
                wow_packet::WorldPacket::from_bytes(&prepare).server_opcode(),
                Some(ServerOpcodes::SpellPrepare)
            );
            let mut prepare_body = WorldPacket::from_bytes(&prepare[2..]);
            assert_eq!(prepare_body.read_packed_guid().unwrap(), client_cast_id);
            let server_cast_id = prepare_body.read_packed_guid().unwrap();
            assert_eq!(server_cast_id.high_type(), HighGuid::Cast);

            let go = send_rx.try_recv().expect("SpellGo packet");
            assert_eq!(
                wow_packet::WorldPacket::from_bytes(&go).server_opcode(),
                Some(ServerOpcodes::SpellGo)
            );
            let mut go_body = WorldPacket::from_bytes(&go[2..]);
            assert_eq!(go_body.read_packed_guid().unwrap(), player_guid);
            assert_eq!(go_body.read_packed_guid().unwrap(), player_guid);
            assert_eq!(go_body.read_packed_guid().unwrap(), server_cast_id);
            assert_eq!(go_body.read_packed_guid().unwrap(), client_cast_id);
            assert_eq!(go_body.read_int32().unwrap(), spell_id);
            let _visual = wow_packet::packets::spell::SpellCastVisual::read(&mut go_body)
                .expect("SpellVisual");
            assert_eq!(go_body.read_uint32().unwrap(), 0);
            assert_eq!(
                go_body.read_uint32().unwrap(),
                CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP
            );
        });
}
#[test]
fn handle_use_toy_rejects_second_cast_on_item_effect_cooldown_like_cpp() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (mut session, _, send_rx) = make_session();
            let player_guid = ObjectGuid::create_player(1, 31_200);
            let first_client_cast_id = ObjectGuid::create_player(1, 31_201);
            let second_client_cast_id = ObjectGuid::create_player(1, 31_202);
            let item_id = 30_000;
            let spell_id = 12_345;

            session.set_player_guid(Some(player_guid));
            session.set_player_map_position_like_cpp(571, Position::new(10.0, 10.0, 0.0, 0.0));
            install_stackable_test_item_template(&mut session, item_id, 1);
            session.load_represented_account_toys_like_cpp([(item_id, false, false)]);
            session.set_item_effect_store(Arc::new(ItemEffectStore::from_entries([
                ItemEffectEntry {
                    id: 1,
                    legacy_slot_index: 0,
                    trigger_type: 0,
                    charges: 0,
                    cooldown_msec: 5_000,
                    category_cooldown_msec: 0,
                    spell_category_id: 0,
                    spell_id,
                    chr_specialization_id: 0,
                    parent_item_id: item_id,
                },
            ])));
            let mut spell_store = SpellStore::new();
            spell_store.insert(spell_id, instant_toy_spell_info_like_cpp(spell_id));
            session.set_spell_store(Arc::new(spell_store));

            session
                .handle_use_toy(write_minimal_use_toy_packet_like_cpp(
                    item_id,
                    spell_id,
                    first_client_cast_id,
                ))
                .await;

            assert_eq!(
                drain_server_opcodes(&send_rx),
                vec![
                    ServerOpcodes::SpellPrepare,
                    ServerOpcodes::SpellGo,
                    ServerOpcodes::CooldownEvent
                ]
            );

            session
                .handle_use_toy(write_minimal_use_toy_packet_like_cpp(
                    item_id,
                    spell_id,
                    second_client_cast_id,
                ))
                .await;

            let failed = send_rx.try_recv().expect("CastFailed packet");
            assert_eq!(
                wow_packet::WorldPacket::from_bytes(&failed).server_opcode(),
                Some(ServerOpcodes::CastFailed)
            );
            let mut failed_body = WorldPacket::from_bytes(&failed[2..]);
            assert_eq!(
                failed_body.read_packed_guid().unwrap(),
                second_client_cast_id
            );
            assert_eq!(failed_body.read_int32().unwrap(), spell_id);
            let _ = wow_packet::packets::spell::SpellCastVisual::read(&mut failed_body)
                .expect("visual");
            assert_eq!(
                failed_body.read_int32().unwrap(),
                SpellCastResult::NotReady as i32
            );
            assert_eq!(failed_body.read_int32().unwrap(), 0);
            assert_eq!(failed_body.read_int32().unwrap(), 0);
            assert!(send_rx.try_recv().is_err());
        });
}
