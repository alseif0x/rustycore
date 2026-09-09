//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn summon_object_live_spell_requires_focus_waits_for_search_spell_focus_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 704_i32;
    let template_entry = 9005_u32;
    let player_guid = ObjectGuid::create_player(1, 7006);
    let player_position = Position::new(140.0, 240.0, 34.0, 0.0);
    let canonical = shared_canonical_map_manager();
    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        gameobject_summon_spell_info_like_cpp(
            spell_id,
            181,
            vec![
                summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap()),
                summon_object_slot_effect_like_cpp(i32::try_from(template_entry).unwrap(), 0),
            ],
        ),
    );

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 704,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("represented CheckCast should send CastFailed without failing the transport path");

    assert!(
        session
            .client_visible_guids_like_cpp
            .snapshot_like_cpp()
            .into_iter()
            .all(|guid| !guid.is_game_object()),
        "focus-required summons must not fabricate a caster-based GO without SearchSpellFocus or aura bypass"
    );
    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    assert_eq!(
        managed.map().map_object_count(),
        1,
        "only the canonical player should exist while focusObject is unrepresented"
    );
    drop(manager);
    let packets = drain_server_packet_bytes(&send_rx);
    let expected_cast_failed = wow_packet::packets::spell::CastFailed {
        cast_id: ObjectGuid::EMPTY,
        spell_id,
        // C++ SendCastResult carries m_SpellVisual (set in the Spell ctor) even on an
        // early REQUIRES_SPELL_FOCUS CheckCast failure — not a default/empty visual.
        visual: wow_packet::packets::spell::SpellCastVisual {
            spell_visual_id: 704,
            script_visual_id: 0,
        },
        reason: SpellCastResult::RequiresSpellFocus as i32,
        fail_arg1: 181,
        fail_arg2: 0,
    }
    .to_bytes();
    assert_eq!(
        packets,
        vec![expected_cast_failed],
        "C++ Spell::CheckCast sends SPELL_FAILED_REQUIRES_SPELL_FOCUS with FailedArg1 set to the required SpellFocusObject id before SpellGo"
    );
    assert_eq!(
        update_object_packet_count_like_cpp(&packets),
        0,
        "focus-required guarded summon must not send a fabricated GameObject create"
    );
}
#[tokio::test]
async fn summon_object_live_spell_requires_focus_allows_matching_focus_aura_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let focus_aura_spell_id = 707_i32;
    let spell_id = 706_i32;
    let template_entry = 9008_u32;
    let player_guid = ObjectGuid::create_player(1, 7009);
    let player_position = Position::new(170.0, 270.0, 37.0, 0.5);
    let canonical = shared_canonical_map_manager();
    let summon_spell_info = gameobject_summon_spell_info_like_cpp(
        spell_id,
        181,
        vec![summon_object_wild_effect_like_cpp(
            i32::try_from(template_entry).unwrap(),
        )],
    );
    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        summon_spell_info.clone(),
    );
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        focus_aura_spell_id,
        wow_data::SpellInfo {
            spell_id: focus_aura_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS,
                effect_misc_value_1: 181,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(spell_id, summon_spell_info);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            focus_aura_spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 707,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("represented provide-spell-focus aura should apply");
    assert!(
        session.has_represented_aura_effect_with_misc_value_like_cpp(
            RepresentedAuraEffectLikeCpp::ProvideSpellFocus,
            181
        )
    );

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 706,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("focus aura should satisfy represented CheckCast focus gate");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("aura-bypassed focus-required summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        Position::new(
            player_position.x
                + wow_map::map::DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP
                    * player_position.orientation.cos(),
            player_position.y
                + wow_map::map::DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP
                    * player_position.orientation.sin(),
            player_position.z,
            player_position.orientation,
        )
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "focus aura bypass should still trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn represented_provide_spell_focus_aura_uses_spell_effect_row_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 725_i32;
    let player_guid = ObjectGuid::create_player(1, 7030);
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
                effect_index: 1,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS,
                effect_misc_value_1: 181,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 725,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("represented apply-aura effect row should execute");

    assert!(
        session.has_represented_aura_effect_with_misc_value_like_cpp(
            RepresentedAuraEffectLikeCpp::ProvideSpellFocus,
            181
        ),
        "C++ HandleEffects dispatches SPELL_EFFECT_APPLY_AURA from SpellEffectInfo rows"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::CooldownEvent
        ]
    );
}
#[tokio::test]
async fn generic_apply_aura_single_effect_row_applies_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 726_i32;
    let player_guid = ObjectGuid::create_player(1, 7031);
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
                effect_index: 2,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_DUMMY,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 726,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("single represented generic apply-aura effect row should execute");

    let aura = session
        .visible_auras
        .values()
        .find(|aura| aura.spell_id == spell_id)
        .expect("C++ EffectApplyAura registers the current SpellEffectInfo row");
    assert_eq!(aura.represented_effect, None);
    assert_eq!(aura.caster_guid, player_guid);
    assert_eq!(aura.duration_total, 30_000);
    assert_eq!(aura.aura_flags, 0x0000_0001);
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::CooldownEvent
        ]
    );
}
#[tokio::test]
async fn generic_owned_aura_cancel_removes_single_effect_row_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 728_i32;
    let player_guid = ObjectGuid::create_player(1, 7033);
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_DUMMY,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: spell_id as u32,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("single represented generic apply-aura effect row should execute");
    let _ = drain_server_opcodes(&send_rx);

    let removed = session.remove_represented_cancelable_owned_aura_like_cpp(spell_id, player_guid);

    assert_eq!(removed, 1);
    assert!(
        !session
            .visible_auras
            .values()
            .any(|aura| aura.spell_id == spell_id)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate]
    );
}
#[tokio::test]
async fn generic_owned_aura_cancel_preserves_passive_spell_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 729_i32;
    let player_guid = ObjectGuid::create_player(1, 7034);
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_DUMMY,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: spell_id as u32,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("single represented generic apply-aura effect row should execute");
    let _ = drain_server_opcodes(&send_rx);

    let removed = session.remove_represented_cancelable_owned_aura_like_cpp(spell_id, player_guid);

    assert_eq!(removed, 0);
    assert!(
        session
            .visible_auras
            .values()
            .any(|aura| aura.spell_id == spell_id)
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn generic_apply_aura_multi_row_waits_for_effect_mask_grouping_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 727_i32;
    let player_guid = ObjectGuid::create_player(1, 7032);
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
            effects: vec![
                wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS,
                    effect_misc_value_1: 182,
                    ..Default::default()
                },
                wow_data::SpellEffectInfo {
                    effect_index: 2,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_DUMMY,
                    ..Default::default()
                },
            ],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 727,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("represented apply-aura rows should not fabricate extra generic aura slots");

    assert!(
        session.has_represented_aura_effect_with_misc_value_like_cpp(
            RepresentedAuraEffectLikeCpp::ProvideSpellFocus,
            182
        ),
        "special represented aura row still executes"
    );
    assert_eq!(
        session.visible_auras.len(),
        1,
        "C++ groups apply-aura rows through one AuraApplication effect mask; Rust must not fabricate a second generic slot"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::CooldownEvent
        ]
    );
}
#[tokio::test]
async fn battle_pet_xp_pct_aura_registers_cpp_multiplier_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 728_i32;
    let player_guid = ObjectGuid::create_player(1, 7033);
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_BATTLE_PET_XP_PCT,
                effect_base_points: 50,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 728,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("represented battle-pet XP aura should execute");

    let aura = session
        .visible_auras
        .values()
        .find(|aura| aura.spell_id == spell_id)
        .expect("battle-pet XP aura");
    assert_eq!(
        aura.represented_effect,
        Some(RepresentedAuraEffectLikeCpp::ModBattlePetXpPct)
    );
    assert_eq!(aura.represented_amount, 50);
    assert_eq!(aura.represented_multiplier, 1.5);
    assert_eq!(
        session.total_represented_aura_multiplier_like_cpp(
            RepresentedAuraEffectLikeCpp::ModBattlePetXpPct
        ),
        1.5
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::CooldownEvent
        ]
    );
}
#[tokio::test]
async fn detected_range_aura_registers_cpp_modifier_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 729_i32;
    let player_guid = ObjectGuid::create_player(1, 7034);
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DETECTED_RANGE,
                effect_base_points: 4,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 729,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("represented detected-range aura should execute");

    assert_eq!(
        session.total_represented_aura_modifier_like_cpp(
            RepresentedAuraEffectLikeCpp::ModDetectedRange
        ),
        4
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::CooldownEvent
        ]
    );
}
