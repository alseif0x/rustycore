//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_unit_values_update_preserves_unrepresented_spellclick_delta_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(126);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 706,
            spell_id: 906,
            cast_flags: 0,
            user_type: SPELL_CLICK_USER_RAID_LIKE_CPP,
        }],
        |entry| entry == 706,
        |spell| spell == 906,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        706,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );

    let mut mask = UpdateMask::new(UNIT_DATA_BITS);
    mask.set(113);
    mask.set(114);
    let mut values = UnitDataValues::default();
    values.npc_flags = [UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32, 0];
    let update = UnitValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_UNIT,
        object_data: None,
        unit_data: Some(UnitDataUpdate { mask, values }),
    };

    let packet = session
        .represented_unit_values_update_to_update_object_like_cpp(creature_guid, 571, &update)
        .expect("delta packet");
    let wow_packet::packets::update::UpdateBlock::UnitValuesUpdate { data, .. } = &packet.blocks[0]
    else {
        panic!("unit values update block expected");
    };

    assert_eq!(data.npc_flags[0], UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32);
}
#[test]
fn canonical_player_spells_and_metadata_follow_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_563);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "SpellOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    let owned_row = RepresentedPlayerSpellLikeCpp {
        spell_id: 635,
        active: true,
        disabled: false,
        dependent: false,
        favorite: true,
        state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
    };

    session.set_known_spells_like_cpp(vec![635]);
    assert!(session.set_complete_represented_player_spell_rows_like_cpp([owned_row]));
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([(635, 7)]));
    assert!(session.set_complete_represented_override_spells_like_cpp([(600, 635)]));
    assert_eq!(session.resolved_known_spells_like_cpp(), Some(vec![635]));
    assert_eq!(
        session
            .complete_represented_player_spell_rows_like_cpp()
            .and_then(|rows| rows.get(&635).copied()),
        Some(owned_row)
    );
    assert_eq!(
        session.complete_represented_spell_trait_definition_ids_like_cpp(),
        Some(HashMap::from([(635, 7)]))
    );
    assert_eq!(
        session.complete_represented_override_spells_like_cpp(),
        Some(HashMap::from([(600, BTreeSet::from([635]))]))
    );
    session.mark_represented_character_spell_cooldowns_loaded_like_cpp();
    session.record_loaded_character_spell_cooldown_like_cpp(635, 6948, 9_000, 12, 8_000);
    session.mark_represented_character_spell_charges_loaded_like_cpp();
    session.record_loaded_character_spell_charge_like_cpp(42, 7_000, 8_000);
    let owned_history = session
        .player_spell_history_snapshot_like_cpp()
        .expect("active canonical spell history");
    assert!(owned_history.cooldowns_loaded);
    assert!(owned_history.charges_loaded);
    assert_eq!(owned_history.cooldowns[&635].item_id, 6948);
    assert_eq!(owned_history.cooldowns[&635].cooldown_end_ms, 9_000_000);
    assert_eq!(owned_history.charges[&42].len(), 1);
    session.set_loaded_player_customizations_like_cpp(vec![
        wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate {
            option_id: 50,
            choice_id: 60,
        },
    ]);
    assert!(session.replace_completed_achievement_ids_like_cpp([100, 200]));
    assert_eq!(
        session.completed_achievement_ids_snapshot_like_cpp(),
        Some(HashSet::from([100, 200]))
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(session.resolved_known_spells_like_cpp(), Some(vec![635]));
    assert_eq!(
        session.complete_represented_spell_trait_definition_ids_like_cpp(),
        Some(HashMap::from([(635, 7)]))
    );
    assert_eq!(
        session.player_spell_history_snapshot_like_cpp(),
        Some(owned_history)
    );
    assert_eq!(
        session.completed_achievement_ids_snapshot_like_cpp(),
        Some(HashSet::from([100, 200]))
    );

    let replacement_row = wow_entities::PlayerKnownSpellRecord {
        spell_id: 900,
        state: wow_entities::PlayerSpellLoadState::Unchanged,
        active: true,
        disabled: false,
        favorite: false,
        dependent: false,
    };
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.replace_spell_runtime_like_cpp(wow_entities::PlayerSpellRuntimeState {
        known_spells: vec![900],
        rows: BTreeMap::from([(900, replacement_row.clone())]),
        rows_loaded: true,
        rows_complete: true,
        trait_definition_ids: BTreeMap::from([(900, 11)]),
        trait_definition_ids_complete: true,
        override_spells: BTreeMap::from([(800, BTreeSet::from([900]))]),
        override_spells_complete: true,
        ..Default::default()
    });
    let replacement_history = wow_entities::SpellHistory {
        cooldowns: HashMap::from([(
            900,
            wow_entities::SpellCooldown {
                spell_id: 900,
                item_id: 0,
                cooldown_end_ms: 90_000,
                category_id: 90,
                category_end_ms: 80_000,
                on_hold: false,
            },
        )]),
        cooldowns_loaded: true,
        charges_loaded: true,
        ..Default::default()
    };
    replacement.unit_mut().subsystems_mut().spells.history = replacement_history.clone();
    replacement.gameplay_state_mut().customizations =
        vec![wow_entities::PlayerCustomizationChoice {
            option_id: 70,
            choice_id: 80,
        }];
    replacement.gameplay_state_mut().achievements = vec![wow_entities::PlayerAchievementRecord {
        achievement_id: 900,
        completed_at: None,
    }];
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_known_spells_like_cpp(), None);
    assert_eq!(
        session.complete_represented_player_spell_rows_like_cpp(),
        None
    );
    assert_eq!(
        session.complete_represented_spell_trait_definition_ids_like_cpp(),
        None
    );
    assert_eq!(
        session.complete_represented_override_spells_like_cpp(),
        None
    );
    assert_eq!(session.player_spell_history_snapshot_like_cpp(), None);
    assert_eq!(session.completed_achievement_ids_snapshot_like_cpp(), None);
    assert!(!session.set_complete_represented_player_spell_rows_like_cpp([owned_row]));
    assert_eq!(session.mutate_player_spell_history_like_cpp(|_| ()), None);
    assert!(!session.replace_completed_achievement_ids_like_cpp([100]));
    session.set_loaded_player_customizations_like_cpp(vec![
        wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate {
            option_id: 50,
            choice_id: 60,
        },
    ]);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.spell_runtime_like_cpp().clone()
            }),
        Some(wow_entities::PlayerSpellRuntimeState {
            known_spells: vec![900],
            rows: BTreeMap::from([(900, replacement_row)]),
            rows_loaded: true,
            rows_complete: true,
            trait_definition_ids: BTreeMap::from([(900, 11)]),
            trait_definition_ids_complete: true,
            override_spells: BTreeMap::from([(800, BTreeSet::from([900]))]),
            override_spells_complete: true,
            ..Default::default()
        })
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.unit().subsystems().spells.history.clone()
            }),
        Some(replacement_history)
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.gameplay_state().customizations.clone(),
                player.gameplay_state().achievements.clone(),
            )),
        Some((
            vec![wow_entities::PlayerCustomizationChoice {
                option_id: 70,
                choice_id: 80,
            }],
            vec![wow_entities::PlayerAchievementRecord {
                achievement_id: 900,
                completed_at: None,
            }],
        ))
    );
}
#[test]
fn canonical_player_aura_authority_metadata_follows_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_570);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AuraAuthorityOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.set_player_aura_authority_complete_like_cpp(true));
    assert!(
        session.insert_player_visible_aura_like_cpp(AuraApplication {
            spell_id: 43_621,
            difficulty_id: 0,
            caster_guid: player_guid,
            slot: 8,
            duration_total: 30_000,
            duration_remaining: 20_000,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::SafeFall),
            represented_amount: 7,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        })
    );
    assert!(
        session
            .mutate_player_aura_subsystem_like_cpp(|auras| {
                auras.insert_threat_snapshot_like_cpp(
                    4,
                    wow_entities::AuraThreatSnapshotLikeCpp::new([7, 11], vec![(1, 13, 17, 19)]),
                );
            })
            .is_some()
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.resolved_player_aura_authority_complete_like_cpp(),
        Some(true)
    );
    assert_eq!(
        session
            .resolved_player_visible_auras_like_cpp()
            .expect("detached canonical visible-aura owner")[&8]
            .represented_amount,
        7
    );
    assert_eq!(
        session
            .mutate_player_aura_subsystem_like_cpp(|auras| {
                auras.runtime_application_mut_like_cpp(8).map(|aura| {
                    aura.represented_amount = 21;
                    aura.represented_amount
                })
            })
            .flatten(),
        Some(21)
    );
    session.tombstone_player_spell_hit_aura_authority_like_cpp();
    let detached = session
        .player_aura_subsystem_snapshot_like_cpp()
        .expect("detached canonical aura owner");
    assert!(detached.spell_hit_aura_authority_tombstoned_like_cpp());
    assert_eq!(
        detached
            .threat_snapshot_like_cpp(4)
            .expect("canonical threat snapshot")
            .effects(),
        &[(1, 13, 17, 19)]
    );

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert!(session.player_aura_subsystem_snapshot_like_cpp().is_none());
    assert_eq!(
        session.resolved_player_aura_authority_complete_like_cpp(),
        None
    );
    assert!(!session.set_player_aura_authority_complete_like_cpp(true));
    assert!(
        session
            .mutate_player_aura_subsystem_like_cpp(|auras| {
                auras
                    .runtime_application_mut_like_cpp(8)
                    .map(|aura| aura.represented_amount = 99)
            })
            .flatten()
            .is_none()
    );
    assert!(session.remove_player_visible_aura_like_cpp(8).is_none());
    let replacement_auras = canonical
        .lock()
        .unwrap()
        .with_player_like_cpp(replacement_handle, |player| {
            player.unit().subsystems().auras.clone()
        })
        .expect("replacement aura owner");
    assert!(!replacement_auras.persisted_player_aura_authority_complete_like_cpp());
    assert!(!replacement_auras.spell_hit_aura_authority_tombstoned_like_cpp());
    assert!(replacement_auras.threat_snapshot_like_cpp(4).is_none());
    assert!(replacement_auras.runtime_applications_like_cpp().is_empty());
}
#[test]
fn quest_reputation_gain_respects_no_quest_bonus_aura_gate_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.visible_auras.insert(
        1,
        reputation_aura_for_test(1, RepresentedAuraEffectLikeCpp::ModReputationGain, 25, None),
    );

    assert_eq!(
        session.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Quest,
            80,
            100,
            7,
            false,
        ),
        125
    );
    assert_eq!(
        session.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Quest,
            80,
            100,
            7,
            true,
        ),
        100
    );
}
#[test]
fn reputation_gain_applies_recruit_a_friend_bonus_for_non_spell_sources_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 1);
    let recruit_guid = ObjectGuid::create_player(1, 2);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_recruiter_id_like_cpp(2);
    session.set_reputation_rates_like_cpp(ReputationRatesLikeCpp {
        recruit_a_friend_bonus: 0.1,
        recruit_a_friend_distance: 100.0,
        ..ReputationRatesLikeCpp::default()
    });

    let (recruit_tx, _recruit_rx) = flume::bounded(10);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let mut recruit_info = broadcast_info(recruit_guid, recruit_tx);
    recruit_info.placement.map_id = 571;
    recruit_info.placement.position = Position::new(25.0, 0.0, 0.0, 0.0);
    recruit_info.identity.account_id = 2;
    recruit_info.identity.recruiter_id = 0;
    player_registry.register_or_replace(recruit_guid, recruit_info, Default::default());

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.add_member(recruit_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_state(SessionState::LoggedIn);

    assert_eq!(
        session.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Quest,
            80,
            100,
            7,
            false,
        ),
        110
    );
    assert_eq!(
        session.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Spell,
            80,
            100,
            7,
            false,
        ),
        100,
        "C++ skips Recruit-A-Friend for REPUTATION_SOURCE_SPELL"
    );
}
#[test]
fn player_registry_publishes_multieffect_scalable_aura_points_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 45);
    let registry = Arc::new(PlayerRegistry::default());
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    let canonical = bind_canonical_test_player_to_registry_like_cpp(
        &mut session,
        &registry,
        guid,
        position,
        571,
    );
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("MultiEffectAuraTester".to_string());
    session.set_player_registry(Arc::clone(&registry));
    let mut scalable_aura =
        reputation_aura_for_test(1, RepresentedAuraEffectLikeCpp::ModReputationGain, 35, None);
    scalable_aura.aura_flags |= AFLAG_SCALABLE_LIKE_CPP;
    scalable_aura.effect_mask = 0x0000_0005;
    scalable_aura
        .represented_effect_amounts
        .push(RepresentedAuraEffectAmountLikeCpp {
            effect_index: 2,
            amount: 71,
        });
    session.visible_auras.insert(1, scalable_aura);
    assert!(
        with_canonical_player_at_mut_like_cpp(&canonical, guid, 571, 0, |player| {
            let aura = wow_entities::AppliedAuraRef::new(1, guid, 1, 0x0000_0005);
            player.unit_mut().subsystems_mut().auras.add_applied(aura);
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_visible_with_application_like_cpp(
                    1,
                    aura.aura_ref(),
                    wow_entities::VisibleAuraApplicationLikeCpp::new(
                        AFLAG_SCALABLE_LIKE_CPP | 0x0001,
                        vec![
                            wow_entities::VisibleAuraEffectAmountLikeCpp {
                                effect_index: 0,
                                amount: 35,
                            },
                            wow_entities::VisibleAuraEffectAmountLikeCpp {
                                effect_index: 2,
                                amount: 71,
                            },
                        ],
                    ),
                );
        })
        .is_some()
    );

    session.register_in_player_registry();

    let info = registry.party_member(guid).expect("registered player");
    assert_eq!(info.party_member_auras.len(), 1);
    let aura = &info.party_member_auras[0];
    assert_eq!(aura.active_flags, 0x0000_0005);
    assert_eq!(aura.points, vec![35.0, 71.0]);
}
#[test]
fn represented_vehicle_switch_cross_vehicle_records_spellclick_plan_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 61_201);
    let base = test_creature_guid(61_202);
    let other_vehicle = test_creature_guid(61_203);
    let spell_id = 911_i32;

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "VehicleSwitchSpellClickTester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_SWITCH);
    session.set_player_moved_unit_guid_like_cpp(base);
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 807,
            spell_id: u32::try_from(spell_id).unwrap(),
            cast_flags: NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 807,
        |spell| spell == u32::try_from(spell_id).unwrap(),
    )));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_CONTROL_VEHICLE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_CONTROL_VEHICLE,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    add_canonical_test_creature(
        &canonical,
        other_vehicle,
        807,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );

    assert!(session.represented_request_vehicle_switch_seat_like_cpp(other_vehicle, 3));
    assert_eq!(
        session.represented_vehicle_seat_spell_click_requests_like_cpp(),
        &[RepresentedVehicleSeatSpellClickRequestLikeCpp {
            vehicle_guid: other_vehicle,
            seat_id: 3,
            planned_casts: 1,
            exact_context_unrepresented: false,
        }],
        "C++ cross-vehicle switch delegates to vehUnit->HandleSpellClick(player, seat)"
    );
    assert!(
        session
            .represented_vehicle_seat_change_requests_like_cpp()
            .is_empty()
    );
}
#[test]
fn player_registry_publishes_party_member_pet_aura_flags_and_points_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 50);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 42_001, 101);
    let registry = Arc::new(PlayerRegistry::default());
    let canonical = shared_canonical_map_manager();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("PetAuraOwnerTester".to_string());
    session.set_player_registry(Arc::clone(&registry));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_represented_pet_mode_state_like_cpp(Some(pet_guid), 1, 0);
    add_canonical_test_player_on_map(&canonical, guid, position, 571, 0);
    add_canonical_test_pet_with_visible_aura(
        &canonical,
        pet_guid,
        guid,
        42_001,
        position,
        0,
        Some((3, 12_346, guid, 0x04)),
        Some(wow_entities::VisibleAuraApplicationLikeCpp::new(
            AFLAG_SCALABLE_LIKE_CPP | 0x0001,
            vec![
                wow_entities::VisibleAuraEffectAmountLikeCpp {
                    effect_index: 0,
                    amount: 11,
                },
                wow_entities::VisibleAuraEffectAmountLikeCpp {
                    effect_index: 2,
                    amount: 37,
                },
            ],
        )),
    );

    session.register_in_player_registry();

    let info = registry.party_member(guid).expect("registered player");
    let pet_stats = info
        .party_member_pet_stats
        .as_ref()
        .expect("pet stats are published");
    assert_eq!(pet_stats.auras.len(), 1);
    let aura = &pet_stats.auras[0];
    assert_eq!(aura.spell_id, 12_346);
    assert_eq!(aura.active_flags, 0x04);
    assert_eq!(aura.flags, (AFLAG_SCALABLE_LIKE_CPP | 0x0001) as u16);
    assert_eq!(aura.points, vec![37.0]);
}
#[tokio::test]
async fn spell_effect_school_damage_negative_amount_is_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let spell_id = 723_i32;
    let guid = test_creature_guid(18_011);
    let player_guid = ObjectGuid::create_player(1, 54);
    session.player_guid = Some(player_guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            effect_base_points: -10,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, guid)
        .await
        .expect("negative represented damage should execute as C++ no-op effect");

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(world_creature.current_hp(), 40);
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_direct_heal_and_damage_use_spell_effect_rows_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let spell_id = 724_i32;
    let guid = test_creature_guid(18_012);
    let player_guid = ObjectGuid::create_player(1, 55);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
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
            effects: vec![
                wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                    effect_base_points: 7,
                    ..Default::default()
                },
                wow_data::SpellEffectInfo {
                    effect_index: 1,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
                    effect_base_points: 5,
                    ..Default::default()
                },
            ],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, guid)
        .await
        .expect("represented direct effects should execute per SpellEffectInfo row");

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(
        world_creature.current_hp(),
        38,
        "C++ HandleEffects executes damage and heal rows in effect order"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_self_heal_syncs_player_health_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 44);
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
        guid,
        "Healer".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(50, 100);
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    session.apply_heal(None, guid, 20).await.unwrap();

    assert_eq!(session.player_health_like_cpp(), 70);
    let canonical_health = session
        .mutate_canonical_player_like_cpp(|player| player.unit().data().health)
        .unwrap();
    assert_eq!(canonical_health, 70);
    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);
}
#[tokio::test]
async fn spell_self_heal_skips_dead_player_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 46);
    session.set_player_guid(Some(guid));
    session.set_player_health_like_cpp(0, 100);

    session.apply_heal(None, guid, 20).await.unwrap();

    assert_eq!(session.player_health_like_cpp(), 0);
    assert!(!session.player_is_alive_like_cpp());
    assert!(send_rx.try_recv().is_err());
}
