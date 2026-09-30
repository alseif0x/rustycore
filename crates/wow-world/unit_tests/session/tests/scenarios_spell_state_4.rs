//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn stand_state_live_bridge_removes_standing_auras_and_fans_out_values_like_cpp() {
    run_canonical_player_owner_test(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let (mut source, _, source_send_rx) = make_session();
                let (mut viewer, _, viewer_send_rx) = make_session();
                let canonical = shared_canonical_map_manager();
                let source_guid = ObjectGuid::create_player(1, 90_210);
                let viewer_guid = ObjectGuid::create_player(1, 90_211);
                let position = Position::ZERO;
                let represented_standing_slot = 11;
                let represented_kept_slot = 12;
                let represented_snapshot_standing_slot = 13;
                let standing_spell_id = 70_001;
                let kept_spell_id = 70_002;
                let snapshot_standing_spell_id = 70_003;
                let canonical_standing = wow_entities::AppliedAuraRef::new(
                    standing_spell_id as u32,
                    source_guid,
                    represented_standing_slot,
                    0x1,
                );
                let canonical_kept = wow_entities::AppliedAuraRef::new(
                    kept_spell_id as u32,
                    source_guid,
                    represented_kept_slot,
                    0x1,
                );
                let canonical_snapshot_standing = wow_entities::AppliedAuraRef::new(
                    snapshot_standing_spell_id as u32,
                    source_guid,
                    represented_snapshot_standing_slot,
                    0x1,
                );
                let canonical_standing_owned = wow_entities::OwnedAuraRef::new(
                    standing_spell_id as u32,
                    source_guid,
                    None,
                );
                let canonical_kept_owned =
                    wow_entities::OwnedAuraRef::new(kept_spell_id as u32, source_guid, None);
                let canonical_snapshot_standing_owned = wow_entities::OwnedAuraRef::new(
                    snapshot_standing_spell_id as u32,
                    source_guid,
                    None,
                );
                let mut spell_store = wow_data::SpellStore::new();
                spell_store.insert_spell_interrupt_flags_like_cpp(
                    standing_spell_id,
                    [0x20, 0],
                    [0, 0],
                );
                spell_store.insert_spell_interrupt_flags_for_difficulty_like_cpp(
                    standing_spell_id,
                    2,
                    [wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP, 0],
                    [0, 0],
                );
                spell_store.insert_spell_interrupt_flags_like_cpp(
                    kept_spell_id,
                    [wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP, 0],
                    [0, 0],
                );
                spell_store.insert_spell_interrupt_flags_like_cpp(
                    snapshot_standing_spell_id,
                    [0x20, 0],
                    [0, 0],
                );
                let difficulty_store =
                    wow_data::DifficultyStore::from_entries([wow_data::DifficultyEntry {
                        id: 2,
                        instance_type: 0,
                        flags: 0,
                        fallback_difficulty_id: 0,
                        toggle_difficulty_id: 0,
                    }]);

                source.set_player_guid(Some(source_guid));
                source.player_name = Some("BridgeSource".into());
                source.set_player_map_position_like_cpp(571, position);
                source.set_canonical_map_manager(Arc::clone(&canonical));
                source.set_spell_store(Arc::new(spell_store));
                source.set_difficulty_store(Arc::new(difficulty_store));
                source.set_player_stand_state_like_cpp(UnitStandStateType::Sit);
                add_canonical_test_player_on_map_with_difficulty(
                    &canonical,
                    source_guid,
                    position,
                    571,
                    0,
                    2,
                );
                assert_eq!(
                    source.current_canonical_player_map_difficulty_id_like_cpp(),
                    Some(2)
                );
                source
                    .mutate_canonical_player_like_cpp(|player| {
                        player
                            .unit_mut()
                            .set_stand_state_like_cpp(UnitStandStateType::Sit);
                        player.clear_data_changes();
                        player
                            .unit_mut()
                            .subsystems_mut()
                            .auras
                            .register_applied_aura(canonical_standing, None, 0, 0);
                        player
                            .unit_mut()
                            .subsystems_mut()
                            .auras
                            .add_owned(canonical_standing_owned);
                        player
                            .unit_mut()
                            .subsystems_mut()
                            .auras
                            .register_applied_aura(canonical_kept, None, 0x20, 0);
                        player
                            .unit_mut()
                            .subsystems_mut()
                            .auras
                            .add_owned(canonical_kept_owned);
                        player
                            .unit_mut()
                            .subsystems_mut()
                            .auras
                            .register_applied_aura(
                                canonical_snapshot_standing,
                                None,
                                // A resolved difficulty/hotfix/server-side snapshot
                                // must not be overridden by the selected DB2 row.
                                wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP,
                                0,
                            );
                        player
                            .unit_mut()
                            .subsystems_mut()
                            .auras
                            .add_owned(canonical_snapshot_standing_owned);
                        player.unit_mut().subsystems_mut().auras.set_visible(
                            represented_standing_slot,
                            canonical_standing.aura_ref(),
                        );
                        player
                            .unit_mut()
                            .subsystems_mut()
                            .auras
                            .set_visible(represented_kept_slot, canonical_kept.aura_ref());
                        player.unit_mut().subsystems_mut().auras.set_visible(
                            represented_snapshot_standing_slot,
                            canonical_snapshot_standing.aura_ref(),
                        );
                    })
                    .unwrap();

                for (slot, spell_id) in [
                    (represented_standing_slot, standing_spell_id),
                    (represented_kept_slot, kept_spell_id),
                    (
                        represented_snapshot_standing_slot,
                        snapshot_standing_spell_id,
                    ),
                ] {
                    source.visible_auras.insert(
                        slot,
                        AuraApplication {
                            spell_id,
                            difficulty_id: 0,
                            caster_guid: source_guid,
                            slot,
                            duration_total: 30_000,
                            duration_remaining: 30_000,
                            stack_count: 1,
                            aura_flags: 0x1,
                            effect_mask: 0x1,
                            // One removable aura proves a missing/zero snapshot is
                            // hydrated from the active difficulty's SpellInfo. Two nonzero
                            // resolved snapshots prove DB2 cannot add or remove Standing.
                            aura_interrupt_flags: if spell_id == standing_spell_id {
                                0
                            } else if spell_id == kept_spell_id {
                                0x20
                            } else {
                                wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP
                            },
                            aura_interrupt_flags2: 0,
                            represented_effect: None,
                            represented_amount: 0,
                            represented_effect_amounts: Vec::new(),
                            represented_misc_value: None,
                            represented_multiplier: 1.0,
                            applied_at: Instant::now(),
                        },
                    );
                }

                let non_standing_outcome = source.apply_represented_live_intent_like_cpp(
                    RepresentedLiveIntentLikeCpp::StandStateChanged(
                        RepresentedStandStateChangedLikeCpp {
                            state: UnitStandStateType::Kneel,
                        },
                    ),
                );
                assert_eq!(
                    non_standing_outcome,
                    RepresentedLiveIntentApplyOutcomeLikeCpp::Applied(
                        RepresentedLiveIntentAppliedLikeCpp::StandStateChanged {
                            canonical_field_changed: true,
                            canonical_auras_removed: 0,
                            represented_auras_removed: 0,
                            channel_cancellation_boundary: None,
                        },
                    )
                );
                assert!(
                    source
                        .visible_auras
                        .contains_key(&represented_standing_slot)
                );
                assert_eq!(
                    drain_server_opcodes(&source_send_rx),
                    vec![ServerOpcodes::StandStateUpdate, ServerOpcodes::UpdateObject,]
                );
                source
                    .mutate_canonical_player_like_cpp(|player| {
                        assert!(
                            player
                                .unit()
                                .unit_data_changes_mask()
                                .is_set(wow_entities::UNIT_DATA_STAND_STATE_BIT),
                            "without a visibility registry, the canonical delta remains pending"
                        );
                        assert!(
                            player.unit().world().object().is_object_updated(),
                            "the in-world Player is queued for canonical object updates"
                        );
                    })
                    .unwrap();
                let canonical_send_summary = canonical
                    .lock()
                    .unwrap()
                    .find_map_mut(571, 0)
                    .expect("canonical test map")
                    .map_mut()
                    .send_object_updates_like_cpp();
                assert_eq!(canonical_send_summary.queued_before, 1);
                assert_eq!(canonical_send_summary.processed, 1);
                source
                    .mutate_canonical_player_like_cpp(|player| {
                        assert!(
                            !player
                                .unit()
                                .unit_data_changes_mask()
                                .is_set(wow_entities::UNIT_DATA_STAND_STATE_BIT),
                            "canonical SendObjectUpdates consumes the queued StandState delta"
                        );
                        assert!(!player.unit().world().object().is_object_updated());
                    })
                    .unwrap();
                source.represented_live_applications_like_cpp.clear();

                viewer.set_player_guid(Some(viewer_guid));
                viewer.set_player_map_position_like_cpp(571, position);
                viewer.state = SessionState::LoggedIn;
                viewer.client_visible_guids_like_cpp.insert(source_guid);
                let registry = Arc::new(PlayerRegistry::default());
                let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
                let mut viewer_info = broadcast_info_with_command(
                    viewer_guid,
                    registry_send_tx,
                    viewer.session_command_tx(),
                );
                viewer_info.placement.map_id = 571;
                registry.register_or_replace(viewer_guid, viewer_info, Default::default());
                source.set_player_registry(registry);

                let outcome = source.apply_represented_live_intent_like_cpp(
                    RepresentedLiveIntentLikeCpp::StandStateChanged(
                        RepresentedStandStateChangedLikeCpp {
                            state: UnitStandStateType::Stand,
                        },
                    ),
                );
                viewer.process_represented_session_commands_like_cpp().await;

                assert_eq!(
                    outcome,
                    RepresentedLiveIntentApplyOutcomeLikeCpp::Applied(
                        RepresentedLiveIntentAppliedLikeCpp::StandStateChanged {
                            canonical_field_changed: true,
                            canonical_auras_removed: 2,
                            represented_auras_removed: 2,
                            channel_cancellation_boundary: None,
                        },
                    )
                );
                assert_eq!(
                    drain_server_opcodes(&source_send_rx),
                    vec![
                        ServerOpcodes::AuraUpdate,
                        ServerOpcodes::AuraUpdate,
                        ServerOpcodes::StandStateUpdate,
                        ServerOpcodes::UpdateObject,
                    ]
                );
                assert_eq!(
                    drain_server_opcodes(&viewer_send_rx),
                    vec![
                        ServerOpcodes::AuraUpdate,
                        ServerOpcodes::AuraUpdate,
                        ServerOpcodes::UpdateObject,
                    ]
                );
                assert!(
                    !source
                        .visible_auras
                        .contains_key(&represented_standing_slot)
                );
                assert!(source.visible_auras.contains_key(&represented_kept_slot));
                assert!(
                    !source
                        .visible_auras
                        .contains_key(&represented_snapshot_standing_slot)
                );
                source
                    .mutate_canonical_player_like_cpp(|player| {
                        assert_eq!(
                            player.unit().stand_state_like_cpp(),
                            UnitStandStateType::Stand
                        );
                        assert!(
                            !player
                                .unit()
                                .subsystems()
                                .auras
                                .has_applied(canonical_standing)
                        );
                        assert!(player.unit().subsystems().auras.has_applied(canonical_kept));
                        assert!(
                            !player
                                .unit()
                                .subsystems()
                                .auras
                                .has_applied(canonical_snapshot_standing)
                        );
                        assert!(
                            !player
                                .unit()
                                .subsystems()
                                .auras
                                .visible_auras
                                .contains_key(&represented_standing_slot)
                        );
                        assert_eq!(
                            player
                                .unit()
                                .subsystems()
                                .auras
                                .visible_auras
                                .get(&represented_kept_slot),
                            Some(&canonical_kept.aura_ref())
                        );
                        assert!(
                            !player
                                .unit()
                                .subsystems()
                                .auras
                                .visible_auras
                                .contains_key(&represented_snapshot_standing_slot)
                        );
                        assert!(
                            !player
                                .unit()
                                .subsystems()
                                .auras
                                .has_owned(canonical_standing_owned)
                        );
                        assert!(
                            player
                                .unit()
                                .subsystems()
                                .auras
                                .has_owned(canonical_kept_owned)
                        );
                        assert!(
                            !player
                                .unit()
                                .subsystems()
                                .auras
                                .has_owned(canonical_snapshot_standing_owned)
                        );
                        assert!(
                            player
                                .unit()
                                .unit_data_changes_mask()
                                .is_set(wow_entities::UNIT_DATA_STAND_STATE_BIT),
                            "the transitional fanout must not consume the canonical StandState delta"
                        );
                    })
                    .unwrap();
                assert_eq!(
                    source.represented_live_applications_like_cpp(),
                    &[RepresentedLiveApplicationLikeCpp {
                        intent: RepresentedLiveIntentLikeCpp::StandStateChanged(
                            RepresentedStandStateChangedLikeCpp {
                                state: UnitStandStateType::Stand,
                            },
                        ),
                        outcome,
                    }]
                );
            });
    });
}
#[test]
fn stand_state_casting_standing_channel_interrupts_non_melee_spells_like_cpp() {
    run_canonical_player_owner_test(|| {
        let (mut session, _, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 90_212);
        let position = Position::ZERO;
        let generic_spell_id = 71_001;
        let autorepeat_spell_id = 71_002;
        let channel_spell_id = 71_003;
        let melee_spell_id = 71_004;
        let standing_aura_spell_id = 71_005;
        let standing_aura_slot = 13;
        let standing_aura = wow_entities::AppliedAuraRef::new(
            standing_aura_spell_id as u32,
            player_guid,
            standing_aura_slot,
            0x1,
        );
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert_spell_interrupt_flags_like_cpp(channel_spell_id, [0, 0], [0x20, 0]);
        spell_store.insert_spell_interrupt_flags_for_difficulty_like_cpp(
            channel_spell_id,
            2,
            [0, 0],
            [wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP, 0],
        );
        spell_store.insert_spell_interrupt_flags_like_cpp(
            standing_aura_spell_id,
            [wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP, 0],
            [0, 0],
        );

        session.set_player_guid(Some(player_guid));
        session.player_name = Some("StandBoundary".into());
        session.set_player_map_position_like_cpp(571, position);
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_spell_store(Arc::new(spell_store));
        session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_entries([
            wow_data::DifficultyEntry {
                id: 2,
                instance_type: 0,
                flags: 0,
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
        ])));
        session.set_player_stand_state_like_cpp(UnitStandStateType::Sit);
        add_canonical_test_player_on_map_with_difficulty(
            &canonical,
            player_guid,
            position,
            571,
            0,
            2,
        );

        let generic =
            wow_entities::CurrentSpellRef::new(generic_spell_id as u32, Some(player_guid), None)
                .with_cast_time_ms(1_000)
                .with_state(wow_constants::SpellState::Preparing);
        let autorepeat =
            wow_entities::CurrentSpellRef::new(autorepeat_spell_id as u32, Some(player_guid), None);
        let channel =
            wow_entities::CurrentSpellRef::new(channel_spell_id as u32, Some(player_guid), None)
                .with_state(wow_constants::SpellState::Casting);
        let melee =
            wow_entities::CurrentSpellRef::new(melee_spell_id as u32, Some(player_guid), None);
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_stand_state_like_cpp(UnitStandStateType::Sit);
                let unit = player.unit_mut();
                let spells = &mut unit.subsystems_mut().spells;
                spells.set_current_spell(wow_entities::CurrentSpellSlot::Melee, melee);
                spells.set_current_spell(wow_entities::CurrentSpellSlot::Generic, generic);
                spells.set_current_spell(wow_entities::CurrentSpellSlot::Autorepeat, autorepeat);
                spells.set_current_spell(wow_entities::CurrentSpellSlot::Channeled, channel);
                unit.subsystems_mut().auras.register_applied_aura(
                    standing_aura,
                    None,
                    wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP,
                    0,
                );
                unit.subsystems_mut()
                    .auras
                    .set_visible(standing_aura_slot, standing_aura.aura_ref());
                player.clear_data_changes();
            })
            .unwrap();
        session.visible_auras.insert(
            standing_aura_slot,
            AuraApplication {
                spell_id: standing_aura_spell_id,
                difficulty_id: 0,
                caster_guid: player_guid,
                slot: standing_aura_slot,
                duration_total: 30_000,
                duration_remaining: 30_000,
                stack_count: 1,
                aura_flags: 0x1,
                effect_mask: 0x1,
                aura_interrupt_flags: wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP,
                aura_interrupt_flags2: 0,
                represented_effect: None,
                represented_amount: 0,
                represented_effect_amounts: Vec::new(),
                represented_misc_value: None,
                represented_multiplier: 1.0,
                applied_at: Instant::now(),
            },
        );
        session.set_active_spell_cast_like_cpp(Some(SpellCastState {
            spell_id: generic_spell_id,
            target_guid: player_guid,
            target_data: wow_entities::SpellCastTargetsLikeCpp::default(),
            cast_id: ObjectGuid::new(6, 71_006),
            cast_start_time: Instant::now(),
            cast_time_ms: 1_000,
            spell_visual: wow_entities::SpellCastVisualLikeCpp {
                spell_visual_id: 0,
                script_visual_id: 0,
            },
            metadata: SpellCastMetadata::default(),
        }));

        let outcome = session.apply_represented_live_intent_like_cpp(
            RepresentedLiveIntentLikeCpp::StandStateChanged(RepresentedStandStateChangedLikeCpp {
                state: UnitStandStateType::Stand,
            }),
        );

        assert_eq!(
            outcome,
            RepresentedLiveIntentApplyOutcomeLikeCpp::Applied(
                RepresentedLiveIntentAppliedLikeCpp::StandStateChanged {
                    canonical_field_changed: true,
                    canonical_auras_removed: 1,
                    represented_auras_removed: 1,
                    channel_cancellation_boundary: Some(
                        RepresentedStandChannelCancellationBoundary::Interrupted {
                            spell_id: channel_spell_id as u32,
                            canonical_spells_interrupted: 3,
                            session_cast_interrupted: true,
                        },
                    ),
                },
            )
        );
        assert_eq!(
            session.player_stand_state_like_cpp(),
            UnitStandStateType::Stand
        );
        assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
        assert!(!session.visible_auras.contains_key(&standing_aura_slot));
        assert_eq!(
            session.represented_live_applications_like_cpp(),
            &[RepresentedLiveApplicationLikeCpp {
                intent: RepresentedLiveIntentLikeCpp::StandStateChanged(
                    RepresentedStandStateChangedLikeCpp {
                        state: UnitStandStateType::Stand,
                    },
                ),
                outcome,
            }]
        );
        assert_eq!(
            drain_server_opcodes(&send_rx),
            vec![
                ServerOpcodes::AuraUpdate,
                ServerOpcodes::StandStateUpdate,
                ServerOpcodes::UpdateObject,
            ]
        );
        session
            .mutate_canonical_player_like_cpp(|player| {
                let unit = player.unit();
                assert_eq!(unit.stand_state_like_cpp(), UnitStandStateType::Stand);
                assert!(!unit.subsystems().auras.has_applied(standing_aura));
                assert!(
                    !unit
                        .subsystems()
                        .auras
                        .visible_auras
                        .contains_key(&standing_aura_slot)
                );
                assert_eq!(
                    unit.current_spell(wow_entities::CurrentSpellSlot::Melee),
                    Some(melee)
                );
                assert_eq!(
                    unit.current_spell(wow_entities::CurrentSpellSlot::Generic),
                    None
                );
                assert_eq!(
                    unit.current_spell(wow_entities::CurrentSpellSlot::Autorepeat),
                    None
                );
                assert_eq!(
                    unit.current_spell(wow_entities::CurrentSpellSlot::Channeled),
                    None
                );
                assert!(!unit.has_unit_state(UnitState::CASTING.bits()));
            })
            .unwrap();
    });
}
#[test]
fn stand_state_channel_boundary_requires_casting_standing_transition() {
    let run_case = |channel_state, channel_flags: [u32; 2], requested_state, guid_counter| {
        let (mut session, _, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, guid_counter);
        let channel_spell_id = 72_001;
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert_spell_interrupt_flags_like_cpp(channel_spell_id, [0, 0], channel_flags);
        session.set_player_guid(Some(player_guid));
        session.set_player_map_position_like_cpp(571, Position::ZERO);
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_spell_store(Arc::new(spell_store));
        add_canonical_test_player_on_map(&canonical, player_guid, Position::ZERO, 571, 0);
        let channel =
            wow_entities::CurrentSpellRef::new(channel_spell_id as u32, Some(player_guid), None)
                .with_state(channel_state);
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .subsystems_mut()
                    .spells
                    .set_current_spell(wow_entities::CurrentSpellSlot::Channeled, channel);
                player.clear_data_changes();
            })
            .unwrap();
        session.set_active_spell_cast_like_cpp(Some(SpellCastState {
            spell_id: 72_002,
            target_guid: player_guid,
            target_data: wow_entities::SpellCastTargetsLikeCpp::default(),
            cast_id: ObjectGuid::new(6, 72_003),
            cast_start_time: Instant::now(),
            cast_time_ms: 1_000,
            spell_visual: wow_entities::SpellCastVisualLikeCpp {
                spell_visual_id: 0,
                script_visual_id: 0,
            },
            metadata: SpellCastMetadata::default(),
        }));

        let outcome = session.apply_represented_live_intent_like_cpp(
            RepresentedLiveIntentLikeCpp::StandStateChanged(RepresentedStandStateChangedLikeCpp {
                state: requested_state,
            }),
        );
        assert!(session.active_spell_cast_snapshot_like_cpp().is_some());
        session
            .mutate_canonical_player_like_cpp(|player| {
                assert_eq!(
                    player
                        .unit()
                        .current_spell(wow_entities::CurrentSpellSlot::Channeled),
                    Some(channel)
                );
            })
            .unwrap();
        let mut expected_opcodes = vec![ServerOpcodes::StandStateUpdate];
        if requested_state != UnitStandStateType::Stand {
            expected_opcodes.push(ServerOpcodes::UpdateObject);
        }
        assert_eq!(drain_server_opcodes(&send_rx), expected_opcodes);
        outcome
    };

    for (state, flags, requested_state, guid_counter, field_changed) in [
        (
            wow_constants::SpellState::Delayed,
            [wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP, 0],
            UnitStandStateType::Stand,
            90_213,
            false,
        ),
        (
            wow_constants::SpellState::Casting,
            [0, 0],
            UnitStandStateType::Stand,
            90_214,
            false,
        ),
        (
            wow_constants::SpellState::Casting,
            [wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP, 0],
            UnitStandStateType::Kneel,
            90_215,
            true,
        ),
    ] {
        assert_eq!(
            run_case(state, flags, requested_state, guid_counter),
            RepresentedLiveIntentApplyOutcomeLikeCpp::Applied(
                RepresentedLiveIntentAppliedLikeCpp::StandStateChanged {
                    canonical_field_changed: field_changed,
                    canonical_auras_removed: 0,
                    represented_auras_removed: 0,
                    channel_cancellation_boundary: None,
                },
            )
        );
    }
}
#[test]
fn stand_state_casting_channel_with_unknown_metadata_records_boundary_and_applies() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_216);
    let channel_spell_id = 73_001;
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_stand_state_like_cpp(UnitStandStateType::Sit);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    add_canonical_test_player_on_map(&canonical, player_guid, Position::ZERO, 571, 0);
    let channel =
        wow_entities::CurrentSpellRef::new(channel_spell_id as u32, Some(player_guid), None)
            .with_state(wow_constants::SpellState::Casting);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .set_stand_state_like_cpp(UnitStandStateType::Sit);
            player
                .unit_mut()
                .subsystems_mut()
                .spells
                .set_current_spell(wow_entities::CurrentSpellSlot::Channeled, channel);
            player.clear_data_changes();
        })
        .unwrap();

    let outcome = session.apply_represented_live_intent_like_cpp(
        RepresentedLiveIntentLikeCpp::StandStateChanged(RepresentedStandStateChangedLikeCpp {
            state: UnitStandStateType::Stand,
        }),
    );

    assert_eq!(
        outcome,
        RepresentedLiveIntentApplyOutcomeLikeCpp::Applied(
            RepresentedLiveIntentAppliedLikeCpp::StandStateChanged {
                canonical_field_changed: true,
                canonical_auras_removed: 0,
                represented_auras_removed: 0,
                channel_cancellation_boundary: Some(
                    RepresentedStandChannelCancellationBoundary::UnknownInterruptMetadata {
                        spell_id: channel_spell_id as u32,
                    },
                ),
            },
        )
    );
    assert_eq!(
        session.player_stand_state_like_cpp(),
        UnitStandStateType::Stand
    );
    assert_eq!(
        session.represented_live_applications_like_cpp(),
        &[RepresentedLiveApplicationLikeCpp {
            intent: RepresentedLiveIntentLikeCpp::StandStateChanged(
                RepresentedStandStateChangedLikeCpp {
                    state: UnitStandStateType::Stand,
                },
            ),
            outcome,
        }]
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::StandStateUpdate, ServerOpcodes::UpdateObject]
    );
    session
        .mutate_canonical_player_like_cpp(|player| {
            assert_eq!(
                player.unit().stand_state_like_cpp(),
                UnitStandStateType::Stand
            );
            assert_eq!(
                player
                    .unit()
                    .current_spell(wow_entities::CurrentSpellSlot::Channeled),
                Some(channel)
            );
        })
        .unwrap();
}
