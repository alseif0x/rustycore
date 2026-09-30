use super::*;

#[test]
fn remove_known_spell_clears_titan_grip_and_penalty_aura_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let spell_id = 774_i32;
    let penalty_spell_id = 49152_i32;
    let player_guid = ObjectGuid::create_player(1, 154);
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
        "RemoveTitanGrip".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.set_can_titan_grip(true, penalty_spell_id as u32);
    });
    session
        .apply_aura(penalty_spell_id, player_guid, 0, 0)
        .expect("represented penalty aura");

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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_TITAN_GRIP,
                effect_misc_value_1: penalty_spell_id,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0_u32; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    session.set_spell_store(Arc::new(spell_store));
    session.set_known_spells_like_cpp(vec![spell_id]);

    session.remove_known_spell_like_cpp(spell_id);

    assert_eq!(
        session.mutate_canonical_player_like_cpp(|player| {
            (
                player.can_titan_grip(),
                player.titan_grip_penalty_spell_id(),
            )
        }),
        Some((false, 0)),
        "C++ Player::RemoveSpell clears m_canTitanGrip when the removed spell is passive and has SPELL_EFFECT_TITAN_GRIP"
    );
    assert!(
        !session
            .visible_auras
            .values()
            .any(|aura| aura.spell_id == penalty_spell_id),
        "C++ RemoveSpell removes m_titanGripPenaltySpellId auras before SetCanTitanGrip(false)"
    );
}
#[test]
fn remove_known_spell_clears_dual_wield_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let spell_id = 775_i32;
    let player_guid = ObjectGuid::create_player(1, 155);
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
        "RemoveDualWield".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
    });

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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0_u32; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    session.set_spell_store(Arc::new(spell_store));
    session.set_known_spells_like_cpp(vec![spell_id]);

    session.remove_known_spell_like_cpp(spell_id);

    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| { player.unit().can_dual_wield_like_cpp() }),
        Some(false),
        "C++ Player::RemoveSpell clears m_canDualWield when the removed spell is passive and has SPELL_EFFECT_DUAL_WIELD"
    );
}
