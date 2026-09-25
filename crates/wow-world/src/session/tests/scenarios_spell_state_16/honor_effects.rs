use super::*;

#[tokio::test]
async fn spell_give_honor_effect_row_sends_pvp_credit_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 747_i32;
    let player_guid = ObjectGuid::create_player(1, 64);
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_GIVE_HONOR,
                effect_base_points: 25,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented give-honor spell row should execute");

    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes: Vec<_> = packets
        .iter()
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
        .collect();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::PvpCredit,
            ServerOpcodes::CooldownEvent,
        ]
    );
    let mut credit = wow_packet::WorldPacket::from_bytes(&packets[1]);
    assert_eq!(
        credit.read_uint16().expect("opcode"),
        ServerOpcodes::PvpCredit as u16
    );
    assert_eq!(credit.read_int32().expect("OriginalHonor"), 25);
    assert_eq!(credit.read_int32().expect("Honor"), 25);
    assert_eq!(
        credit.read_packed_guid().expect("Target"),
        ObjectGuid::EMPTY,
        "C++ EffectGiveHonor leaves PvPCredit.Target default-initialized"
    );
    assert_eq!(credit.read_int32().expect("Rank"), 0);
    assert!(credit.is_empty());
}

#[tokio::test]
async fn spell_give_honor_effect_row_adds_honor_xp_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 749_i32;
    let player_guid = ObjectGuid::create_player(1, 67);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Honor".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        10,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_GIVE_HONOR,
                effect_base_points: 8_825,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented give-honor spell row should add honor XP");

    assert_eq!(
        session.mutate_canonical_player_like_cpp(|player| {
            (
                player.data().honor_level,
                player.active_data().honor,
                player.active_data().honor_next_level,
            )
        }),
        Some((1, 25, 8_800))
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::PvpCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}

#[tokio::test]
async fn spell_give_honor_effect_row_requires_current_player_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 748_i32;
    let player_guid = ObjectGuid::create_player(1, 65);
    let other_player_guid = ObjectGuid::create_player(1, 66);
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_GIVE_HONOR,
                effect_base_points: 25,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, other_player_guid)
        .await
        .expect("represented give-honor non-current player target should no-op");

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
