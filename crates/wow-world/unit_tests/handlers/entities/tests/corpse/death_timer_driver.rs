// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Death-timer driver fixtures and tests.

use super::*;

fn assert_local_flags_pending_for_map(session: &WorldSession, flags: u32) {
    let update = session
        .core
        .with_owned_player_like_cpp(|player| {
            assert!(player.unit().world().object().is_in_world());
            assert_eq!(player.gameplay_state().active_local_flags, flags);
            player.values_update(true)
        })
        .expect("current native KillPlayer owner");
    let packet = wow_world_core::entity_update_bridge::player_values_update_to_packet(&update)
        .expect("KillPlayer retains VALUES for Map tail");
    let active = packet.active_player_data.expect("LocalFlags publication");
    assert_eq!(active.local_flags, flags);
    assert_ne!(active.active_player_data_mask[1] & (1 << 6), 0, "parent38");
    assert_ne!(
        active.active_player_data_mask[2] & (1 << 5),
        0,
        "LocalFlags69"
    );
}

fn install_prevent_resurrection_aura_for_driver(session: &mut WorldSession) {
    let spell_id = 94_300;
    let aura_type = wow_data::spell::aura_types::SPELL_AURA_PREVENT_RESURRECTION;
    let mut attributes = [0; 15];
    // Keep this synthetic timer-freeze witness through CORPSE's death cleanup.
    attributes[3] = 0x0010_0000;
    let stores = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::SpellCategoriesStore::from_entries([]),
        wow_data::SpellMiscStore::from_entries([wow_data::SpellMiscEntry {
            id: spell_id as u32,
            spell_id: spell_id as u32,
            attributes,
            ..Default::default()
        }]),
        wow_data::SpellEffectDb2Store::from_entries([wow_data::SpellEffectDb2Entry {
            id: spell_id as u32,
            difficulty_id: 0,
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_amplitude: 0.0,
            effect_attributes: 0,
            effect_aura: i16::try_from(aura_type).expect("PreventResurrection aura type fits i16"),
            effect_aura_period: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            effect_chain_amplitude: 0.0,
            effect_chain_targets: 0,
            effect_die_sides: 0,
            effect_item_type: 0,
            effect_mechanic: 0,
            effect_points_per_resource: 0.0,
            effect_pos_facing: 0.0,
            effect_real_points_per_level: 0.0,
            effect_trigger_spell: 0,
            bonus_coefficient_from_ap: 0.0,
            pvp_multiplier: 0.0,
            coefficient: 0.0,
            variance: 0.0,
            resource_coefficient: 0.0,
            group_size_base_points_coefficient: 0.0,
            effect_misc_value: [0; 2],
            effect_radius_index: [0; 2],
            effect_spell_class_mask: [0; 4],
            implicit_target: [0; 2],
            spell_id: spell_id as u32,
        }]),
        wow_data::SpellShapeshiftStore::from_entries([]),
        wow_data::SpellInterruptsStore::from_entries([]),
        wow_data::SpellCastTimesStore::from_entries([]),
        wow_data::SpellCooldownsStore::from_entries([]),
        wow_data::SpellCastingRequirementsStore::from_entries([]),
        wow_data::SpellPowerStore::from_entries([]),
        wow_data::SpellPowerDifficultyStore::from_entries([]),
    );
    let spells = crate::selected_spell_fixtures::canonical_spell_store([(spell_id as u32, 0)])
        .hydrate_effective_core_db2_like_cpp(stores);
    session.set_spell_store(Arc::new(spells));

    let aura = wow_entities::AuraApplicationLikeCpp {
        spell_id,
        difficulty_id: 0,
        caster_guid: session.player_guid().expect("player GUID"),
        slot: 1,
        duration_total: 10_000,
        duration_remaining: 10_000,
        stack_count: 1,
        aura_flags: 0,
        effect_mask: 1,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: std::time::Instant::now(),
    };
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .insert_runtime_application_like_cpp(aura);
        })
        .expect("canonical Player owner");
}

#[tokio::test]
async fn driver_kill_player_runs_once_after_regeneration_and_waits_for_timer() {
    let (mut session, send_rx, realm_rx, _player_guid, _port) =
        dead_player_session_with_graveyard_like_cpp(4_310);
    let _packet_tx = keep_driver_input_open(&mut session);
    session.core.state = crate::session::SessionState::LoggedIn;
    session.core.with_owned_player_mut_like_cpp(|player| {
        player.gameplay_state_mut().active_local_flags = 0x8000;
        player.clear_active_player_data_changes();
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::JustDied);
    });

    session.update(0).await;

    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.unit().death_state()),
        Some(wow_constants::DeathState::Corpse)
    );
    assert_eq!(death_timer_from_driver(&session), 360_000);
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    assert!(
        current_corpse_guid_like_cpp(&session, session.player_guid().expect("player GUID"))
            .is_none()
    );

    assert_ne!(
        crate::session::hub_ref(&session).active_player_local_flags_like_cpp() & 0x8,
        0,
        "KillPlayer sets PLAYER_LOCAL_FLAG_RELEASE_TIMER on a world map"
    );
    assert_local_flags_pending_for_map(&session, 0x8008);

    session.update(25).await;
    assert_eq!(death_timer_from_driver(&session), 359_975);
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());

    session.update(50).await;

    assert_eq!(death_timer_from_driver(&session), 359_925);
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    let opcodes = sent_opcodes(&send_rx);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::CorpseReclaimDelay as u16)
            .count(),
        1,
        "the driver must consume JUST_DIED once and must not restart its timer"
    );
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );
    assert_eq!(
        realm_rx.try_iter().collect::<Vec<_>>(),
        vec![
            wow_packet::packets::misc::StopMirrorTimer { timer: 0 }.to_bytes(),
            wow_packet::packets::misc::StopMirrorTimer { timer: 1 }.to_bytes(),
            wow_packet::packets::misc::StopMirrorTimer { timer: 2 }.to_bytes(),
            wow_packet::packets::combat::CancelCombat.to_bytes(),
        ],
        "KillPlayer stops every mirror timer before CORPSE's CombatStop, once, on realm"
    );
}

#[tokio::test]
async fn driver_death_timer_expires_at_the_exact_six_minute_boundary() {
    let (mut session, send_rx, realm_rx, player_guid, port) =
        dead_player_session_with_graveyard_like_cpp(4_311);
    let _packet_tx = keep_driver_input_open(&mut session);
    session.core.state = crate::session::SessionState::LoggedIn;
    let (graveyards, conditions) =
        graveyard_store_with_links(0, [wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP], []);
    session.set_graveyard_store(graveyards);
    session.set_condition_store(conditions);
    set_death_timer_for_driver(&mut session, 360_000);

    session.update(359_999).await;

    assert_eq!(death_timer_from_driver(&session), 1);
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_none());
    assert!(port.saves.lock().unwrap().is_empty());

    session.update(1).await;

    assert_eq!(death_timer_from_driver(&session), 0);
    assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    assert!(current_corpse_guid_like_cpp(&session, player_guid).is_some());
    assert_eq!(port.saves.lock().unwrap().len(), 1);
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        1
    );
    assert!(crate::session::hub_ref(&session).near_teleport_pending_like_cpp());
    let teleport = crate::session::hub_ref(&session)
        .player_teleport_state_snapshot_like_cpp()
        .expect("canonical teleport state");
    assert!(!teleport.can_delay);
    assert!(!teleport.has_delayed);
    assert_eq!(teleport.delayed, None);
    assert_eq!(
        teleport.near_destination,
        Some((571, Position::new(50.0, 60.0, 70.0, 0.0)))
    );
    let opcodes = sent_opcodes(&send_rx);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::MoveTeleport as u16)
            .count(),
        1,
        "death-timer graveyard teleport is emitted during expiry, with delayed teleport disabled"
    );
    assert_eq!(
        sent_opcodes(&realm_rx),
        vec![
            ServerOpcodes::PreRessurect as u16,
            ServerOpcodes::CancelCombat as u16,
            ServerOpcodes::StopMirrorTimer as u16,
            ServerOpcodes::StopMirrorTimer as u16,
            ServerOpcodes::StopMirrorTimer as u16,
            ServerOpcodes::CancelCombat as u16,
            ServerOpcodes::DeathReleaseLoc as u16,
        ]
    );
}

#[tokio::test]
async fn driver_death_timer_freezes_in_instance_maps_and_with_prevent_resurrection() {
    let instance_maps = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 33,
        instance_type: wow_data::map::MAP_INSTANCE,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));
    let (mut instance_session, ..) = dead_player_session_on_map_like_cpp(4_312, instance_maps);
    let _instance_packet_tx = keep_driver_input_open(&mut instance_session);
    instance_session.core.state = crate::session::SessionState::LoggedIn;
    instance_session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.gameplay_state_mut().active_local_flags = 0x8008;
            player.clear_active_player_data_changes();
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::JustDied);
        });
    instance_session.update(500).await;
    assert_eq!(death_timer_from_driver(&instance_session), 360_000);
    assert_eq!(
        crate::session::hub_ref(&instance_session).active_player_local_flags_like_cpp() & 0x8,
        0,
        "KillPlayer clears PLAYER_LOCAL_FLAG_RELEASE_TIMER in instances"
    );
    assert_local_flags_pending_for_map(&instance_session, 0x8000);

    let (mut aura_session, ..) = dead_player_session_with_graveyard_like_cpp(4_313);
    let _aura_packet_tx = keep_driver_input_open(&mut aura_session);
    aura_session.core.state = crate::session::SessionState::LoggedIn;
    install_prevent_resurrection_aura_for_driver(&mut aura_session);
    aura_session.core.with_owned_player_mut_like_cpp(|player| {
        player.gameplay_state_mut().active_local_flags = 0x8008;
        player.clear_active_player_data_changes();
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::JustDied);
    });
    aura_session.update(500).await;
    assert_eq!(death_timer_from_driver(&aura_session), 360_000);
    assert!(!crate::session::hub_ref(&aura_session).player_has_ghost_flag_like_cpp());
    assert_eq!(
        crate::session::hub_ref(&aura_session).active_player_local_flags_like_cpp() & 0x8,
        0,
        "KillPlayer clears PLAYER_LOCAL_FLAG_RELEASE_TIMER with PreventResurrection"
    );
    assert_local_flags_pending_for_map(&aura_session, 0x8000);
}

#[tokio::test]
async fn driver_does_not_kill_or_advance_death_timer_while_player_is_detached() {
    let (mut session, send_rx, _realm_rx, _player_guid, _port) =
        dead_player_session_with_graveyard_like_cpp(4_314);
    let _packet_tx = keep_driver_input_open(&mut session);
    session.core.state = crate::session::SessionState::LoggedIn;
    set_death_timer_for_driver(&mut session, 12_345);
    session.core.with_owned_player_mut_like_cpp(|player| {
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::JustDied);
    });
    assert!(
        crate::session::hub_mut(&mut session)
            .remove_current_player_from_canonical_current_map_like_cpp()
    );
    assert!(!session.core.player_is_strictly_in_world_like_cpp());

    session.update(500).await;

    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.unit().death_state()),
        Some(wow_constants::DeathState::JustDied)
    );
    assert_eq!(death_timer_from_driver(&session), 12_345);
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    let opcodes = sent_opcodes(&send_rx);
    for forbidden in [
        ServerOpcodes::CorpseReclaimDelay as u16,
        ServerOpcodes::MoveRoot as u16,
        ServerOpcodes::MoveSetWaterWalk as u16,
    ] {
        assert!(!opcodes.contains(&forbidden));
    }
}
