use super::*;

#[tokio::test]
async fn spell_damage_skips_dead_creature_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_010);
    session.player_guid = Some(ObjectGuid::create_player(1, 53));
    session.client_visible_guids_like_cpp.insert(guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.take_damage(20);
            creature
                .creature
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Corpse);
            creature.creature.clear_data_changes();
        })
        .unwrap();

    session.apply_damage(None, guid, 7).await.unwrap();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(world_creature.current_hp(), 20);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn spell_instakill_effect_row_kills_creature_and_logs_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let spell_id = 728_i32;
    let guid = test_creature_guid(18_013);
    let player_guid = ObjectGuid::create_player(1, 56);
    session.player_guid = Some(player_guid);
    session.set_player_level_like_cpp(80);
    register_test_creature(&mut session, manager.clone(), guid, 40);

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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_INSTAKILL,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, guid)
        .await
        .expect("represented instakill effect row should execute");

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(world_creature.current_hp(), 0);
    assert!(
        !world_creature.creature.is_alive(),
        "C++ EffectInstaKill delegates to Unit::Kill after logging"
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes: Vec<_> = packets
        .iter()
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
        .collect();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellInstakillLog,
            ServerOpcodes::SpellNonMeleeDamageLog,
            ServerOpcodes::CooldownEvent
        ]
    );
    let mut instakill = wow_packet::WorldPacket::from_bytes(&packets[1]);
    assert_eq!(
        instakill.read_uint16().expect("opcode"),
        ServerOpcodes::SpellInstakillLog as u16
    );
    assert_eq!(instakill.read_packed_guid().expect("target"), guid);
    assert_eq!(instakill.read_packed_guid().expect("caster"), player_guid);
    assert_eq!(instakill.read_int32().expect("spell id"), spell_id);
    assert!(instakill.is_empty());
}
