use super::*;

#[tokio::test]
async fn spell_dismiss_pet_effect_row_clears_represented_pet_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 753_i32;
    let player_guid = ObjectGuid::create_player(1, 72);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 0, 0, 500, 73);
    session.set_player_guid(Some(player_guid));
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_PASSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_STAY_LIKE_CPP,
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DISMISS_PET,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, pet_guid)
        .await
        .expect("represented dismiss-pet spell row should execute");

    assert_eq!(session.represented_pet_guid_like_cpp, None);
    assert_eq!(
        session.represented_pet_react_state_like_cpp,
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP
    );
    assert_eq!(
        session.represented_pet_command_state_like_cpp,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}

#[tokio::test]
async fn spell_dismiss_pet_effect_row_requires_represented_pet_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 754_i32;
    let player_guid = ObjectGuid::create_player(1, 74);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 0, 0, 500, 75);
    let other_pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 0, 0, 500, 76);
    session.set_player_guid(Some(player_guid));
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_PASSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_STAY_LIKE_CPP,
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DISMISS_PET,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, other_pet_guid)
        .await
        .expect("represented dismiss-pet non-active pet target should no-op");

    assert_eq!(session.represented_pet_guid_like_cpp, Some(pet_guid));
    assert_eq!(
        session.represented_pet_react_state_like_cpp,
        wow_packet::packets::pet::REACT_PASSIVE_LIKE_CPP
    );
    assert_eq!(
        session.represented_pet_command_state_like_cpp,
        wow_packet::packets::pet::COMMAND_STAY_LIKE_CPP
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
