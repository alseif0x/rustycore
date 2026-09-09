//! Session scenarios exercising the represented movement responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn represented_mounted_flight_speed_sets_can_fly_flags_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    let spell_id = 20_031;
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
                effect_aura:
                    wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED,
                effect_base_points: 60,
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
        .expect("mounted flight apply-aura row should execute");

    assert!(
        session
            .player_movement_flags_like_cpp()
            .contains(MovementFlag::CAN_FLY)
    );
    assert!(session.represented_can_swim_to_fly_transition_like_cpp());
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        opcodes.contains(&ServerOpcodes::MoveEnableTransitionBetweenSwimAndFly),
        "C++ mounted-flight aura enables swim-to-fly transition"
    );
    assert!(
        opcodes.contains(&ServerOpcodes::MoveSetCanFly),
        "C++ mounted-flight aura enables CanFly"
    );
}
#[test]
fn represented_mounted_flight_speed_removal_unsets_can_fly_when_last_source_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let mounted_flight = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED,
        effect_base_points: 60,
        effect_index: 0,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_032,
            caster,
            &mounted_flight,
            RepresentedAuraEffectLikeCpp::MountedFlightSpeed,
            30_000,
        )
        .unwrap();
    session.update_represented_flight_flags_for_flight_aura_like_cpp(true);
    let slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::MountedFlightSpeed))
                .then_some(slot)
        })
        .unwrap();
    let _ = drain_server_opcodes(&send_rx);

    session.remove_aura(slot).unwrap();

    assert!(
        !session
            .player_movement_flags_like_cpp()
            .contains(MovementFlag::CAN_FLY)
    );
    assert!(!session.represented_can_swim_to_fly_transition_like_cpp());
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        opcodes.contains(&ServerOpcodes::MoveDisableTransitionBetweenSwimAndFly),
        "C++ removes swim-to-fly transition when the last mounted-flight/fly aura is gone"
    );
    assert!(
        opcodes.contains(&ServerOpcodes::MoveUnsetCanFly),
        "C++ unsets CanFly when the last mounted-flight/fly aura is gone"
    );
}
#[test]
fn represented_swim_speed_increase_updates_move_swim_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let swim = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        effect_base_points: 50,
        effect_index: 0,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_029,
            caster,
            &swim,
            RepresentedAuraEffectLikeCpp::SwimSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_swim_speed_rate_like_cpp();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Swim) - 7.083333).abs()
            < 0.0001,
        "C++ MOVE_SWIM applies SPELL_AURA_MOD_INCREASE_SWIM_SPEED as AddPct on base swim speed"
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::MoveSetSwimSpeed),
        "swim speed changes are observable through Unit::SetSpeedRate"
    );
}
#[test]
fn represented_swim_speed_removal_recomputes_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let swim = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        effect_base_points: 50,
        effect_index: 0,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_030,
            caster,
            &swim,
            RepresentedAuraEffectLikeCpp::SwimSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_swim_speed_rate_like_cpp();
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Swim) - 7.083333).abs()
            < 0.0001
    );
    let swim_slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::SwimSpeed))
                .then_some(slot)
        })
        .unwrap();

    session.remove_aura(swim_slot).unwrap();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Swim) - 4.722222).abs()
            < 0.0001,
        "C++ aura removal recomputes MOVE_SWIM and drops the swim-speed modifier"
    );
}
#[test]
fn represented_forward_speed_slow_applies_to_swim_and_flight_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let swim = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        effect_base_points: 200,
        effect_index: 0,
        ..Default::default()
    };
    let flight = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_FLIGHT_SPEED,
        effect_base_points: 200,
        effect_index: 1,
        ..Default::default()
    };
    let slow = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED,
        effect_base_points: -50,
        effect_index: 2,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_031,
            caster,
            &swim,
            RepresentedAuraEffectLikeCpp::SwimSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_032,
            caster,
            &flight,
            RepresentedAuraEffectLikeCpp::FlightSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_033,
            caster,
            &slow,
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_forward_speed_rates_like_cpp();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Swim) - 7.083333).abs()
            < 0.0001,
        "C++ applies the strongest SPELL_AURA_MOD_DECREASE_SPEED after MOVE_SWIM positive speed math"
    );
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Flight) - 10.5).abs() < 0.0001,
        "C++ applies the strongest SPELL_AURA_MOD_DECREASE_SPEED after MOVE_FLIGHT positive speed math"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&ServerOpcodes::MoveSetSwimSpeed));
    assert!(opcodes.contains(&ServerOpcodes::MoveSetFlightSpeed));
}
#[test]
fn represented_forward_speed_cap_and_minimum_floor_apply_to_swim_and_flight_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let swim = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        effect_base_points: 200,
        effect_index: 0,
        ..Default::default()
    };
    let flight = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_FLIGHT_SPEED,
        effect_base_points: 200,
        effect_index: 1,
        ..Default::default()
    };
    let normal_cap = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED,
        effect_base_points: 1,
        effect_index: 2,
        ..Default::default()
    };
    let minimum = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_MINIMUM_SPEED,
        effect_base_points: 80,
        effect_index: 3,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_034,
            caster,
            &swim,
            RepresentedAuraEffectLikeCpp::SwimSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_035,
            caster,
            &flight,
            RepresentedAuraEffectLikeCpp::FlightSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_036,
            caster,
            &normal_cap,
            RepresentedAuraEffectLikeCpp::UseNormalMovementSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_037,
            caster,
            &minimum,
            RepresentedAuraEffectLikeCpp::MinimumSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_forward_speed_rates_like_cpp();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Swim) - 3.7777777).abs()
            < 0.0001,
        "C++ applies SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED before the final minimum-speed floor for MOVE_SWIM"
    );
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Flight) - 5.6).abs() < 0.0001,
        "C++ applies SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED before the final minimum-speed floor for MOVE_FLIGHT"
    );
}
#[test]
fn represented_backward_speed_slow_updates_all_backward_move_types_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let slow = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED,
        effect_base_points: -50,
        effect_index: 0,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_038,
            caster,
            &slow,
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_backward_speed_rates_like_cpp();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::RunBack) - 2.25).abs()
            < 0.0001,
        "C++ Unit::UpdateSpeed applies SPELL_AURA_MOD_DECREASE_SPEED to MOVE_RUN_BACK"
    );
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::SwimBack) - 1.25).abs()
            < 0.0001,
        "C++ Unit::UpdateSpeed applies SPELL_AURA_MOD_DECREASE_SPEED to MOVE_SWIM_BACK"
    );
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::FlightBack) - 2.25).abs()
            < 0.0001,
        "C++ Unit::UpdateSpeed applies SPELL_AURA_MOD_DECREASE_SPEED to MOVE_FLIGHT_BACK"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&ServerOpcodes::MoveSetRunBackSpeed));
    assert!(opcodes.contains(&ServerOpcodes::MoveSetSwimBackSpeed));
    assert!(opcodes.contains(&ServerOpcodes::MoveSetFlightBackSpeed));
}
#[test]
fn represented_backward_speed_slow_removal_restores_base_speeds_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let slow = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED,
        effect_base_points: -50,
        effect_index: 0,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_039,
            caster,
            &slow,
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_backward_speed_rates_like_cpp();
    let _ = drain_server_opcodes(&send_rx);
    let slow_slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::DecreaseSpeed))
                .then_some(slot)
        })
        .unwrap();

    session.remove_aura(slow_slot).unwrap();

    assert_eq!(
        session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::RunBack),
        4.5
    );
    assert_eq!(
        session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::SwimBack),
        2.5
    );
    assert_eq!(
        session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::FlightBack),
        4.5
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&ServerOpcodes::MoveSetRunBackSpeed));
    assert!(opcodes.contains(&ServerOpcodes::MoveSetSwimBackSpeed));
    assert!(opcodes.contains(&ServerOpcodes::MoveSetFlightBackSpeed));
}
#[test]
fn represented_normal_run_speed_uses_cpp_stack_and_not_stack_order_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let main = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        effect_base_points: 40,
        effect_index: 0,
        ..Default::default()
    };
    let stack = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_ALWAYS,
        effect_base_points: 20,
        effect_index: 1,
        ..Default::default()
    };
    let not_stack = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_NOT_STACK,
        effect_base_points: 80,
        effect_index: 2,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_001,
            caster,
            &main,
            RepresentedAuraEffectLikeCpp::Speed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_002,
            caster,
            &stack,
            RepresentedAuraEffectLikeCpp::SpeedAlways,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_003,
            caster,
            &not_stack,
            RepresentedAuraEffectLikeCpp::SpeedNotStack,
            30_000,
        )
        .unwrap();
    session.recompute_represented_run_speed_rate_like_cpp();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 17.64).abs() < 0.0001,
        "C++ Unit::UpdateSpeed uses max(SPEED_ALWAYS, SPEED_NOT_STACK) before AddPct(INCREASE_SPEED)"
    );
    assert_eq!(
        session.forced_speed_changes_like_cpp(UnitMoveTypeLikeCpp::Run),
        1
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::MoveSetRunSpeed),
        "normal run-speed auras are observable through Unit::SetSpeedRate"
    );
}
#[test]
fn represented_dismount_restores_active_normal_run_speed_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let normal = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        effect_base_points: 50,
        effect_index: 0,
        ..Default::default()
    };
    session
        .apply_represented_aura_modifier_like_cpp(
            20_004,
            caster,
            &normal,
            RepresentedAuraEffectLikeCpp::Speed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_run_speed_rate_like_cpp();
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 10.5).abs() < 0.0001
    );

    session.set_mount_capability_store(Arc::new(wow_data::MountCapabilityStore::from_entries([
        wow_data::MountCapabilityEntry {
            id: 77,
            flags: wow_data::MOUNT_CAPABILITY_FLAG_GROUND,
            req_riding_skill: 0,
            req_area_id: 0,
            req_spell_aura_id: 0,
            req_spell_known_id: 0,
            mod_spell_aura_id: 12_346,
            req_map_id: 0,
        },
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        12_346,
        wow_data::SpellInfo {
            spell_id: 12_346,
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    let mounted = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 0,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_like_cpp(100, caster, &mounted)
        .unwrap();
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 14.0).abs() < 0.0001
    );
    let mounted_slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Mounted)).then_some(slot)
        })
        .unwrap();

    session.remove_aura(mounted_slot).unwrap();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 10.5).abs() < 0.0001,
        "C++ dismount recomputes MOVE_RUN with the still-active non-mounted speed auras"
    );
}
#[test]
fn represented_run_speed_applies_cpp_strongest_slow_after_positive_bonus_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let speed = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        effect_base_points: 100,
        effect_index: 0,
        ..Default::default()
    };
    let weak_slow = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED,
        effect_base_points: -20,
        effect_index: 1,
        ..Default::default()
    };
    let strong_slow = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED,
        effect_base_points: -40,
        effect_index: 2,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_005,
            caster,
            &speed,
            RepresentedAuraEffectLikeCpp::Speed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_006,
            caster,
            &weak_slow,
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_007,
            caster,
            &strong_slow,
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_run_speed_rate_like_cpp();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 8.4).abs() < 0.0001,
        "C++ Unit::UpdateSpeed applies strongest SPELL_AURA_MOD_DECREASE_SPEED after positive speed bonuses"
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::MoveSetRunSpeed),
        "slow-driven run-speed changes are observable through Unit::SetSpeedRate"
    );
}
#[test]
fn represented_run_speed_slow_removal_recomputes_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let speed = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        effect_base_points: 100,
        effect_index: 0,
        ..Default::default()
    };
    let slow = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED,
        effect_base_points: -50,
        effect_index: 1,
        ..Default::default()
    };
    session
        .apply_represented_aura_modifier_like_cpp(
            20_008,
            caster,
            &speed,
            RepresentedAuraEffectLikeCpp::Speed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_009,
            caster,
            &slow,
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_run_speed_rate_like_cpp();
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 7.0).abs() < 0.0001
    );
    let slow_slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::DecreaseSpeed))
                .then_some(slot)
        })
        .unwrap();

    session.remove_aura(slow_slot).unwrap();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 14.0).abs() < 0.0001,
        "C++ aura removal recomputes MOVE_RUN and drops the removed slow"
    );
}
#[test]
fn represented_run_speed_use_normal_movement_speed_caps_before_slow_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let speed = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        effect_base_points: 200,
        effect_index: 0,
        ..Default::default()
    };
    let normal_cap = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED,
        effect_base_points: 10,
        effect_index: 1,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_020,
            caster,
            &speed,
            RepresentedAuraEffectLikeCpp::Speed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_021,
            caster,
            &normal_cap,
            RepresentedAuraEffectLikeCpp::UseNormalMovementSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_run_speed_rate_like_cpp();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 10.0).abs() < 0.0001,
        "C++ SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED caps MOVE_RUN before slow/min-speed floors"
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::MoveSetRunSpeed),
        "normal-movement-speed cap changes are observable through Unit::SetSpeedRate"
    );
}
#[test]
fn represented_run_speed_use_normal_movement_speed_removal_recomputes_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let speed = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        effect_base_points: 200,
        effect_index: 0,
        ..Default::default()
    };
    let normal_cap = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED,
        effect_base_points: 10,
        effect_index: 1,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_022,
            caster,
            &speed,
            RepresentedAuraEffectLikeCpp::Speed,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_023,
            caster,
            &normal_cap,
            RepresentedAuraEffectLikeCpp::UseNormalMovementSpeed,
            30_000,
        )
        .unwrap();
    session.recompute_represented_run_speed_rate_like_cpp();
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 10.0).abs() < 0.0001
    );
    let normal_cap_slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::UseNormalMovementSpeed))
                .then_some(slot)
        })
        .unwrap();

    session.remove_aura(normal_cap_slot).unwrap();

    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 21.0).abs() < 0.0001,
        "C++ aura removal recomputes MOVE_RUN and drops the normal-movement-speed cap"
    );
}
