//! Persistence scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn continue_login_has_no_concrete_persistence_after_remaining_writes_move() {
    let source = include_str!("../character/world_entry.rs");
    let (_, tail) = source
        .split_once("pub async fn handle_continue_player_login")
        .expect("continue-login handler starts");
    let (handler, _) = tail
        .split_once("pub(super) fn player_login_combat_stats_like_cpp")
        .expect("continue-login handler ends before packet helper");

    assert!(handler.contains("reset_login_pet_talents_like_cpp"));
    assert!(handler.contains("mark_player_online_like_cpp"));
    for concrete in [
        "char_db()",
        "CharStatements::",
        "SqlTransaction::",
        ".prepare(",
        ".execute(",
        ".commit_transaction(",
    ] {
        assert!(
            !handler.contains(concrete),
            "continue-login still contains concrete persistence: {concrete}"
        );
    }
}
#[tokio::test]
async fn account_collection_loads_cross_the_typed_port_in_login_order_like_cpp() {
    let port = CollectionLoadPortLikeCpp::new([
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Toys(vec![
            AccountToyLoadRowLikeCpp {
                item_id: -1,
                is_favorite: true,
                has_fanfare: true,
            },
            AccountToyLoadRowLikeCpp {
                item_id: 42,
                is_favorite: true,
                has_fanfare: false,
            },
        ])),
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Heirlooms(
            vec![
                AccountHeirloomLoadRowLikeCpp {
                    item_id: -1,
                    flags: 1,
                },
                AccountHeirloomLoadRowLikeCpp {
                    item_id: 43,
                    flags: 2,
                },
            ],
        )),
        AccountCollectionLoadOutcomeLikeCpp::Loaded(
            AccountCollectionLoadedLikeCpp::ItemAppearances {
                appearance_blocks: AccountCollectionRowsLikeCpp::Loaded(vec![
                    AccountMaskBlockLikeCpp {
                        block_index: 1,
                        mask: 2,
                    },
                ]),
                favorite_appearance_ids: AccountCollectionRowsLikeCpp::Loaded(vec![9]),
            },
        ),
        AccountCollectionLoadOutcomeLikeCpp::Loaded(
            AccountCollectionLoadedLikeCpp::TransmogIllusions {
                illusion_blocks: vec![AccountMaskBlockLikeCpp {
                    block_index: 2,
                    mask: 4,
                }],
            },
        ),
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Mounts(vec![
            AccountMountLoadRowLikeCpp {
                mount_spell_id: -1,
                flags: 1,
            },
            AccountMountLoadRowLikeCpp {
                mount_spell_id: 123,
                flags: 2,
            },
        ])),
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_battlenet_account_id(77);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session.load_account_toys_like_cpp().await;
    session.load_account_heirlooms_like_cpp().await;
    session.load_account_item_appearances_like_cpp().await;
    session.load_account_transmog_illusions_like_cpp().await;
    assert!(session.load_account_mounts_like_cpp().await);

    assert_eq!(session.account_toy_rows_like_cpp(), vec![(42, true, false)]);
    assert_eq!(session.account_heirloom_rows_like_cpp(), vec![(43, 2)]);
    assert_eq!(
        session.account_transmog_active_player_rows_like_cpp(),
        vec![0, 2]
    );
    assert!(!session.set_appearance_is_favorite_like_cpp(9, true));
    assert!(session.has_transmog_illusion_like_cpp(66));
    assert_eq!(
        session.account_mount_rows_like_cpp(),
        vec![AccountMount {
            spell_id: 123,
            flags: 2,
        }]
    );
    assert_eq!(
        port.requests(),
        vec![
            AccountCollectionLoadRequestLikeCpp::Toys {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::Heirlooms {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::ItemAppearances {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::TransmogIllusions {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::Mounts {
                bnet_account_id: 77
            },
        ]
    );
}
#[test]
fn persisted_transport_login_resolves_valid_offset_to_current_world_position_like_cpp() {
    let guid = ObjectGuid::create_transport(HighGuid::Transport, 7_002);
    let offset = Position::new(10.0, 20.0, 3.0, 0.25);
    let transport_create = MapTransportCreateLikeCpp {
        guid_low: 7_002,
        entry: 192_241,
        display_id: 3_012,
        scale: 1.0,
        taxi_path_id: 784,
        move_speed: 30,
        accel_rate: 10,
        allow_stopping: false,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        gameobject_flags: 0,
        faction_template: 0,
    };
    let transport_position = TransportCreatePositionLikeCpp {
        map_id: 571,
        position: Position::new(100.0, 200.0, 10.0, PI / 2.0),
        timer_ms: 1,
        total_time_ms: 2,
    };
    let resolved = validate_persisted_transport_login_like_cpp(
        guid,
        offset,
        transport_position,
        transport_create,
    )
    .expect("valid passenger attachment");

    assert_eq!(resolved.guid, guid);
    assert_eq!(resolved.map_id, 571);
    assert_eq!(resolved.offset, offset);
    assert_eq!(
        resolved.transport_create, transport_create,
        "SendInitSelf must not need a second DB query for the own transport CREATE"
    );
    assert_eq!(
        resolved.transport_position, transport_position,
        "SendInitSelf must reuse the validated path-time snapshot"
    );
    assert!((resolved.world_position.x - 80.0).abs() < 0.001);
    assert!((resolved.world_position.y - 210.0).abs() < 0.001);
    assert!((resolved.world_position.z - 13.0).abs() < 0.001);
    assert!((resolved.world_position.orientation - (PI / 2.0 + 0.25)).abs() < 0.001);
}
#[test]
fn persisted_transport_login_rejects_corrupt_offsets_and_world_coordinates_like_cpp() {
    let guid = ObjectGuid::create_transport(HighGuid::Transport, 7_003);
    let transport_create = MapTransportCreateLikeCpp {
        guid_low: 7_003,
        entry: 192_241,
        display_id: 3_012,
        scale: 1.0,
        taxi_path_id: 784,
        move_speed: 30,
        accel_rate: 10,
        allow_stopping: false,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        gameobject_flags: 0,
        faction_template: 0,
    };
    let valid_transport = TransportCreatePositionLikeCpp {
        map_id: 571,
        position: Position::ZERO,
        timer_ms: 1,
        total_time_ms: 2,
    };

    for invalid_offset in [
        Position::new(250.01, 0.0, 0.0, 0.0),
        Position::new(0.0, -250.01, 0.0, 0.0),
        Position::new(0.0, 0.0, f32::INFINITY, 0.0),
        Position::new(0.0, 0.0, 0.0, f32::NAN),
    ] {
        assert!(
            validate_persisted_transport_login_like_cpp(
                guid,
                invalid_offset,
                valid_transport,
                transport_create,
            )
            .is_none()
        );
    }

    assert!(
        validate_persisted_transport_login_like_cpp(
            guid,
            Position::new(1.0, 0.0, 0.0, 0.0),
            TransportCreatePositionLikeCpp {
                position: Position::new(Position::MAP_HALFSIZE_LIKE_CPP, 0.0, 0.0, 0.0,),
                ..valid_transport
            },
            transport_create,
        )
        .is_none(),
        "C++ rejects an attachment whose calculated world coordinate is outside the map"
    );
}
#[test]
fn persisted_transport_login_requires_saved_map_in_transport_route_like_cpp() {
    assert!(transport_route_contains_saved_map_like_cpp(
        [0, 1, 571],
        571
    ));
    assert!(
        !transport_route_contains_saved_map_like_cpp([0, 1, 571], 530),
        "C++ GetTransport(savedMap) rejects a same-GUID transport absent from that map"
    );
}
#[tokio::test]
async fn typed_map_corpse_empty_load_marks_the_map_once_like_cpp() {
    let (session, manager, port) =
        map_corpse_session_with_port_like_cpp(PersistedMapCorpseLoadOutcomeLikeCpp::Loaded {
            corpses: Vec::new(),
            phases: MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
            customizations: MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
        });

    let outcome = session.load_map_corpse_data_like_cpp(571, 9).await;

    assert_eq!(outcome, MapCorpseLoadOutcomeLikeCpp::default());
    assert_eq!(
        port.requests(),
        vec![MapCorpseLoadRequestLikeCpp {
            map_id: 571,
            instance_id: 9,
        }]
    );
    assert!(
        manager
            .lock()
            .unwrap()
            .find_map(571, 9)
            .unwrap()
            .map()
            .corpse_data_loaded_like_cpp()
    );
}
#[test]
fn loaded_positive_random_property_ignores_stale_seed_like_cpp() {
    let mut item = wow_entities::Item::default();
    let properties =
        wow_data::ItemRandomPropertiesStore::from_entries([wow_data::ItemRandomPropertiesEntry {
            id: 77,
            enchantments: [1001, 1002, 1003, 0, 0],
        }]);
    let effective_enchantments =
        [wow_packet::packets::update::ItemEnchantmentValuesUpdate::default();
            wow_entities::MAX_ENCHANTMENT_SLOT];
    let random_properties =
        loaded_item_random_properties_like_cpp(77, 456, Some(&properties), None);

    apply_loaded_item_instance_fields_like_cpp(
        &mut item,
        &effective_enchantments,
        random_properties,
    );

    assert_eq!(item.data().random_properties_id, 77);
    assert_eq!(item.data().property_seed, 0);
}
#[test]
fn loaded_missing_random_property_records_are_rejected_like_cpp() {
    let properties = wow_data::ItemRandomPropertiesStore::from_entries([]);
    let suffixes = wow_data::ItemRandomSuffixStore::from_entries([]);

    assert_eq!(
        loaded_item_random_properties_like_cpp(77, 456, Some(&properties), Some(&suffixes)),
        None
    );
    assert_eq!(
        loaded_item_random_properties_like_cpp(-77, 456, Some(&properties), Some(&suffixes)),
        None
    );
}
#[tokio::test]
async fn initial_world_state_port_applies_saved_overlay_after_templates_like_cpp() {
    let port = CollectionLoadPortLikeCpp::for_initial_world_states([
        PlayerInitialWorldStatesLoadOutcomeLikeCpp {
            templates: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![
                PlayerInitialWorldStateTemplateRowLikeCpp {
                    id: 10,
                    default_value: 1,
                    map_ids_csv: String::new(),
                    area_ids_csv: String::new(),
                },
            ]),
            saved_values: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![
                PlayerInitialWorldStateValueRowLikeCpp { id: 10, value: 22 },
            ]),
        },
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_player_lifecycle_port_like_cpp(port);

    let states = session
        .test_load_initial_world_states_for_login_like_cpp(571, 0)
        .await;

    assert!(states.contains(&(10, 22)));
    assert!(!states.contains(&(10, 1)));
}
#[test]
fn loaded_fist_weapons_mirrors_unarmed_after_all_skill_rows_like_cpp() {
    fn skill_info(skill_id: u16, rank: u16, max_rank: u16) -> wow_data::SkillInfoEntry {
        wow_data::SkillInfoEntry {
            skill_id,
            step: 0,
            rank,
            starting_rank: 1,
            max_rank,
            temp_bonus: 0,
            perm_bonus: 0,
        }
    }

    let mut records = HashMap::from([
        (
            SKILL_UNARMED_LIKE_CPP,
            crate::session::RepresentedPlayerSkillLikeCpp {
                skill_id: SKILL_UNARMED_LIKE_CPP,
                step: 0,
                value: 37,
                max: 400,
                profession_slot: -1,
                state: crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
            },
        ),
        (
            SKILL_FIST_WEAPONS_LIKE_CPP,
            crate::session::RepresentedPlayerSkillLikeCpp {
                skill_id: SKILL_FIST_WEAPONS_LIKE_CPP,
                step: 0,
                value: 12,
                max: 400,
                profession_slot: -1,
                state: crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
            },
        ),
    ]);
    let mut skill_info_by_id = BTreeMap::from([
        (
            SKILL_UNARMED_LIKE_CPP,
            skill_info(SKILL_UNARMED_LIKE_CPP, 37, 400),
        ),
        (
            SKILL_FIST_WEAPONS_LIKE_CPP,
            skill_info(SKILL_FIST_WEAPONS_LIKE_CPP, 12, 400),
        ),
    ]);

    sync_loaded_fist_weapons_with_unarmed_like_cpp(&mut records, &mut skill_info_by_id, 80);

    assert_eq!(
        skill_info_by_id
            .get(&SKILL_FIST_WEAPONS_LIKE_CPP)
            .expect("loaded Fist Weapons slot")
            .rank,
        37
    );
    assert_eq!(
        records
            .get(&SKILL_FIST_WEAPONS_LIKE_CPP)
            .expect("active persisted Fist Weapons")
            .value,
        37
    );
}
#[test]
fn loaded_fist_weapons_without_unarmed_is_cleared_like_cpp_set_skill_zero() {
    let mut records = HashMap::from([(
        SKILL_FIST_WEAPONS_LIKE_CPP,
        crate::session::RepresentedPlayerSkillLikeCpp {
            skill_id: SKILL_FIST_WEAPONS_LIKE_CPP,
            step: 0,
            value: 12,
            max: 400,
            profession_slot: -1,
            state: crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]);
    let mut skill_info_by_id = BTreeMap::from([(
        SKILL_FIST_WEAPONS_LIKE_CPP,
        wow_data::SkillInfoEntry {
            skill_id: SKILL_FIST_WEAPONS_LIKE_CPP,
            step: 0,
            rank: 12,
            starting_rank: 1,
            max_rank: 400,
            temp_bonus: 0,
            perm_bonus: 0,
        },
    )]);

    sync_loaded_fist_weapons_with_unarmed_like_cpp(&mut records, &mut skill_info_by_id, 80);

    assert!(
        !records.contains_key(&SKILL_FIST_WEAPONS_LIKE_CPP),
        "C++ marks the persisted skill deleted"
    );
    let cleared = skill_info_by_id
        .get(&SKILL_FIST_WEAPONS_LIKE_CPP)
        .expect("C++ retains the cleared initial update-field slot");
    assert_eq!(cleared.rank, 0);
    assert_eq!(cleared.max_rank, 0);
}
#[test]
fn homebind_retry_refreshes_zone_when_saved_coordinates_already_match_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 46);
        let homebind_position = Position::new(10.0, 20.0, 30.0, 1.0);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 1,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "MatchingHomebind".to_string(),
            homebind_position,
            1,
            1,
            1,
            10,
            0,
        ));

        let mut map_id = 1;
        let mut zone_id = 999;
        let mut position = homebind_position;
        assert!(session.retry_login_at_homebind_like_cpp(
            &mut map_id,
            &mut zone_id,
            &mut position,
            CharacterLoginLocationLikeCpp {
                map_id: 1,
                bind_area_id: Some(12),
                position: homebind_position,
            },
        ));

        assert_eq!(map_id, 1);
        assert_eq!(zone_id, 12);
        assert_eq!(position, homebind_position);
        assert_eq!(session.player_zone_area_like_cpp(), Some((12, 12)));
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(1, 0))
        );
    });
}
#[test]
fn restored_saved_health_preserves_dead_zero_like_cpp() {
    assert_eq!(restored_saved_health_like_cpp(Some(0), 110), 0);
}
#[test]
fn restored_saved_health_clamps_to_recomputed_max_like_cpp() {
    assert_eq!(restored_saved_health_like_cpp(Some(500), 110), 110);
    assert_eq!(restored_saved_health_like_cpp(Some(77), 110), 77);
}
#[test]
fn login_stat_update_derives_and_syncs_loaded_enchantment_bonuses_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 81);
    let item_guid = ObjectGuid::create_item(1, 82);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 77, 110);
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    session.set_spell_item_enchantment_store(Arc::new(
        wow_data::SpellItemEnchantmentStore::from_entries([
            wow_data::SpellItemEnchantmentEntry {
                id: 920,
                effect_arg: [
                    wow_constants::ItemModType::Stamina as u32,
                    wow_constants::ItemModType::Mana as u32,
                    wow_constants::ItemModType::Strength as u32,
                ],
                effect_points_min: [3, 4, 2],
                item_visual: 0,
                flags: wow_constants::SpellItemEnchantmentFlags::empty(),
                required_skill_id: 0,
                required_skill_rank: 0,
                item_level: 1,
                charges: 0,
                effect: [wow_constants::ItemEnchantmentType::Stat as u8; 3],
                condition_id: 0,
                min_level: 1,
                max_level: 0,
            },
            wow_data::SpellItemEnchantmentEntry {
                id: 921,
                effect_arg: [
                    wow_constants::ItemModType::ManaRegeneration as u32,
                    wow_constants::spell::SpellSchools::Fire as u32,
                    0,
                ],
                effect_points_min: [25, 9, 0],
                item_visual: 0,
                flags: wow_constants::SpellItemEnchantmentFlags::empty(),
                required_skill_id: 0,
                required_skill_rank: 0,
                item_level: 1,
                charges: 0,
                effect: [
                    wow_constants::ItemEnchantmentType::Stat as u8,
                    wow_constants::ItemEnchantmentType::Resistance as u8,
                    wow_constants::ItemEnchantmentType::None as u8,
                ],
                condition_id: 0,
                min_level: 1,
                max_level: 0,
            },
        ]),
    ));
    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::EQUIPMENT_SLOT_CHEST,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 920, 0, 0);
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 921, 0, 0);
    session.insert_inventory_item_object(item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);
    assert!(outcome.send_stat_update);
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base,
        [2, 0, 3, 0, 0]
    );
    assert_eq!(session.represented_item_bonus_state_like_cpp().mana_base, 4);
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .mana_regen_bonus,
        25
    );
    let (_, changes) = session
        .player_stat_changes_with_represented_item_bonuses_like_cpp(true)
        .expect("login stat changes with loaded enchantments");

    assert_eq!(changes.health, 13, "current health clamps to the new max");
    assert_eq!(
        changes.max_health, 13,
        "CreateHealth is zero and stamina supplies max HP"
    );
    assert_eq!(changes.power0, 777, "current mana remains authoritative");
    assert_eq!(
        changes.max_power0, 1324,
        "flat mana is derived before max power"
    );
    assert_eq!(changes.stats, [12, 10, 13, 40, 30]);
    assert_eq!(
        changes.attack_power, -20,
        "ChrClasses priest AP coefficients"
    );
    assert_eq!(changes.mana_regen_combat, 5.0);
    assert_eq!(changes.mana_regen_mp5, 0.0);
    let expected_spirit_regen = 40.0_f32.sqrt() * 30.0 * 0.003345;
    assert!((changes.mana_regen - (5.0 + expected_spirit_regen)).abs() < 0.0001);
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((13, 13))
    );
    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((777, 1324))
    );
    let effective = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("effective combat stats are owned by the canonical Player");
    assert_eq!(effective.stats, [12, 10, 13, 40, 30]);
    assert_eq!(effective.max_mana, 1324);
    assert_eq!(effective.combat_ratings, [0; 32]);
    assert_eq!(effective.resistances, [20, 0, 9, 0, 0, 0, 0]);

    let spell_id = 90_084;
    session.set_spell_store(Arc::new(total_stat_percentage_spell_store_like_cpp(
        spell_id, false,
    )));
    session.set_state(crate::session::SessionState::LoggedIn);
    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply total-stat aura over loaded enchantments");
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((13, 80)),
        "absolute aura recalc keeps the loaded +3 stamina enchant before doubling stamina"
    );

    let slot = session
        .visible_aura_slot_for_spell_like_cpp(spell_id)
        .expect("total-stat aura slot");
    session.remove_aura(slot).expect("remove total-stat aura");
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((13, 13)),
        "absolute aura removal keeps the loaded enchantment bonus"
    );
}

#[test]
fn mana_regen_applies_canonical_aura_producers_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 86);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 77, 110);

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            90_087,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_PCT,
            50,
            PowerType::Mana as i32,
        ),
        (
            90_088,
            wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_REGEN,
            10,
            PowerType::Mana as i32,
        ),
        (
            90_089,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_FROM_STAT,
            25,
            3,
        ),
        (
            90_090,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_INTERRUPT,
            20,
            0,
        ),
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_base_points: amount,
                    effect_misc_value_1: misc_value,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);
    for spell_id in [90_087, 90_088, 90_089, 90_090] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply mana regeneration aura");
    }

    let (_, changes) = session
        .player_stat_changes_with_represented_item_bonuses_like_cpp(true)
        .expect("stat changes with mana regeneration aura");
    let spirit_regen = 40.0_f32.sqrt() * 30.0 * 0.003345;
    let expected_mp5 = 10.0 / 5.0 + 40.0 * 25.0 / 500.0;
    assert!((changes.mana_regen - (spirit_regen * 1.5 + expected_mp5)).abs() < 0.0001);
    assert!((changes.mana_regen_combat - (expected_mp5 + spirit_regen * 1.5 * 0.2)).abs() < 0.0001);
    assert_eq!(changes.mana_regen_mp5, 0.0);
}

#[test]
fn committed_swap_updates_top_level_and_nested_container_positions_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 80);
    let nested_guid = ObjectGuid::create_item(1, 81);
    let backpack_guid = ObjectGuid::create_item(1, 82);
    session.set_player_guid(Some(player_guid));
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_BAG_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: 600,
            db_guid: 80,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: backpack_guid,
            entry_id: 701,
            db_guid: 82,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );
    let bag = session.make_inventory_item_object(
        bag_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    session.insert_inventory_item_object(bag);
    let mut nested = session.make_inventory_item_object(
        nested_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    nested.set_container_guid_and_slot(bag_guid, INVENTORY_SLOT_BAG_START);
    session.insert_inventory_item_object(nested);
    let backpack = session.make_inventory_item_object(
        backpack_guid,
        701,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(backpack);

    assert!(session.apply_committed_inventory_item_swap_like_cpp(
        INVENTORY_SLOT_BAG_START,
        0,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
    ));
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(nested_guid)
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_START, 0)
            .map(|item| item.guid),
        Some(backpack_guid)
    );
}
#[test]
fn loaded_refund_metadata_matches_cpp_load_cleanup() {
    let refundable_flags = (ItemFieldFlags::SOULBOUND | ItemFieldFlags::REFUNDABLE).bits();
    assert_eq!(
        loaded_item_refund_decision(refundable_flags, 7_200, Some(123), Some(45)),
        LoadedItemRefundDecision::Valid {
            paid_money: 123,
            paid_extended_cost: 45,
        }
    );
    assert_eq!(
        loaded_item_refund_decision(refundable_flags, 7_201, Some(123), Some(45)),
        LoadedItemRefundDecision::Clear {
            new_flags: ItemFieldFlags::SOULBOUND.bits(),
        }
    );
    assert_eq!(
        loaded_item_refund_decision(refundable_flags, 10, None, Some(45)),
        LoadedItemRefundDecision::Clear {
            new_flags: ItemFieldFlags::SOULBOUND.bits(),
        }
    );
    assert_eq!(
        loaded_item_refund_decision(ItemFieldFlags::SOULBOUND.bits(), 10, Some(123), Some(45)),
        LoadedItemRefundDecision::None
    );
}

fn mana_power_type_store_like_cpp(
    regen_peace: f32,
    regen_combat: f32,
) -> wow_data::character_progression::PowerTypeStore {
    wow_data::character_progression::PowerTypeStore::from_entries([
        wow_data::character_progression::PowerTypeEntry {
            id: 0,
            name_global_string_tag: String::new(),
            cost_global_string_tag: String::new(),
            power_type_enum: PowerType::Mana as i8,
            min_power: 0,
            max_base_power: 0,
            center_power: 0,
            default_power: 0,
            display_modifier: 1,
            regen_interrupt_time_ms: 0,
            regen_peace,
            regen_combat,
            flags: 0,
        },
    ])
}

fn publish_mana_regen_snapshot_like_cpp(
    session: &mut WorldSession,
    mana_regen: f32,
    mana_regen_combat: f32,
) {
    let mut stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .unwrap_or_default();
    stats.mana_regen = mana_regen;
    stats.mana_regen_combat = mana_regen_combat;
    assert!(
        session
            .mutate_canonical_player_like_cpp(
                |player| player.replace_effective_combat_stats_like_cpp(stats)
            )
            .is_some()
    );
}

#[test]
fn mana_regeneration_tick_suppresses_then_publishes_power_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 87);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 100, 1_000, 100, 100);
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
                // Keep the five-second rule inactive regardless of process uptime.
                player
                    .unit_mut()
                    .set_mp5_regeneration_interrupt_start_like_cpp(
                        crate::session_rules::game_time_ms_like_cpp().wrapping_sub(10_000),
                    );
            })
            .is_some()
    );
    // The canonical snapshot is the sole source for the flat regen fields.
    publish_mana_regen_snapshot_like_cpp(&mut session, 10.0, 3.0);
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    // First second: the two-second publication boundary has not been reached,
    // so the value changes without an SMSG_POWER_UPDATE.
    session.tick_player_regeneration_like_cpp(1_000, &power_types, None);
    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((110, 1_000))
    );
    assert!(
        !drain_server_opcodes(&send_rx).contains(&wow_constants::ServerOpcodes::PowerUpdate),
        "throttled regeneration must not publish before 2000ms"
    );

    // Second second: the boundary is crossed and the packet is sent.
    session.tick_player_regeneration_like_cpp(1_000, &power_types, None);
    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((120, 1_000))
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&wow_constants::ServerOpcodes::PowerUpdate),
        "crossing the 2000ms boundary publishes SMSG_POWER_UPDATE"
    );
}

#[test]
fn mana_regeneration_tick_uses_interrupted_rate_under_the_mp5_rule_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 88);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 100, 1_000, 100, 100);
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
                // A cast paid mana one second ago, so the five-second rule is
                // active and the interrupted flat rate applies.
                player
                    .unit_mut()
                    .set_mp5_regeneration_interrupt_start_like_cpp(
                        crate::session_rules::game_time_ms_like_cpp().wrapping_sub(1_000),
                    );
            })
            .is_some()
    );
    publish_mana_regen_snapshot_like_cpp(&mut session, 10.0, 4.0);
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    session.tick_player_regeneration_like_cpp(1_000, &power_types, None);

    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((104, 1_000)),
        "the interrupted flat modifier is consumed while the MP5 rule holds"
    );
}

#[test]
fn paying_a_mana_cost_arms_the_five_second_mp5_rule_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 89);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 500, 1_000, 100, 100);
    // Keep the five-second rule inactive until the cast exercises the producer.
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_mp5_regeneration_interrupt_start_like_cpp(
                        crate::session_rules::game_time_ms_like_cpp().wrapping_sub(10_000),
                    );
            })
            .is_some()
    );
    assert!(!session.represented_player_mp5_regen_interrupted_like_cpp());

    let spell = wow_data::SpellInfo {
        spell_id: 90_091,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: vec![wow_data::SpellPowerCostInfoLikeCpp {
            order_index: 0,
            power_type: PowerType::Mana as i8,
            mana_cost: 50,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            power_cost_pct: 0.0,
            power_cost_max_pct: 0.0,
            power_pct_per_second: 0.0,
            required_aura_spell_id: 0,
            optional_cost: 0,
        }],
        effects: Vec::new(),
    };
    let visual = wow_packet::packets::spell::SpellCastVisual {
        spell_visual_id: 0,
        script_visual_id: 0,
    };

    assert!(session.take_spell_power_like_cpp(&spell, ObjectGuid::EMPTY, spell.spell_id, &visual));
    assert_eq!(
        session
            .canonical_player_power_snapshot_like_cpp(PowerType::Mana)
            .map(|(current, _)| current),
        Some(450)
    );
    assert!(
        session.represented_player_mp5_regen_interrupted_like_cpp(),
        "Spell::TakePower arms the five-second MP5 rule after a mana cost"
    );
}

fn publish_health_regen_snapshot_like_cpp(
    session: &mut WorldSession,
    spirit: i32,
    health_regen: i32,
) {
    let mut stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .unwrap_or_default();
    stats.stats[4] = spirit;
    stats.health_regen = health_regen;
    assert!(
        session
            .mutate_canonical_player_like_cpp(
                |player| player.replace_effective_combat_stats_like_cpp(stats)
            )
            .is_some()
    );
}

/// Build `sOCTRegenHPGameTable` / `sRegenHPPerSptGameTable` level-80 Priest
/// rows and an empty `sRegenMPPerSptGameTable`.
fn health_regen_game_tables_like_cpp(
    base_ratio: f32,
    more_ratio: f32,
) -> wow_data::RegenGameTablesLikeCpp {
    let mut base_columns = [0.0; wow_data::OctRegenHpGameTableLikeCpp::VALUE_COLUMN_COUNT];
    base_columns[4] = base_ratio; // Priest column
    let mut more_columns = [0.0; wow_data::RegenHpPerSptGameTableLikeCpp::VALUE_COLUMN_COUNT];
    more_columns[4] = more_ratio;
    let mut base_rows = vec![wow_data::OctRegenHpEntryLikeCpp::default(); 79];
    base_rows.push(wow_data::OctRegenHpEntryLikeCpp::from_columns(base_columns));
    let mut more_rows = vec![wow_data::RegenHpPerSptEntryLikeCpp::default(); 79];
    more_rows.push(wow_data::RegenHpPerSptEntryLikeCpp::from_columns(
        more_columns,
    ));
    wow_data::RegenGameTablesLikeCpp::from_tables(
        wow_data::RegenMpPerSptGameTableLikeCpp::from_rows([]),
        wow_data::RegenHpPerSptGameTableLikeCpp::from_rows(more_rows),
        wow_data::OctRegenHpGameTableLikeCpp::from_rows(base_rows),
    )
}

#[test]
fn health_regeneration_tick_heals_the_represented_player_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 90);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
            })
            .is_some()
    );
    // `OCTRegenHPPerSpirit` = Spirit(20) * 0.1 + 0 * 0.2 = 2.0.
    publish_health_regen_snapshot_like_cpp(&mut session, 20, 0);
    let tables = health_regen_game_tables_like_cpp(0.1, 0.2);
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    // C++ `RegenerateAll` only runs `RegenerateHealth` once the two-second
    // window is pending.
    session.tick_player_regeneration_like_cpp(1_000, &power_types, Some(&tables));
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((100, 1_000)),
        "the health branch waits for the 2000ms window"
    );

    session.tick_player_regeneration_like_cpp(1_000, &power_types, Some(&tables));
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((102, 1_000))
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| player
                .unit()
                .unit_data_changes_mask()
                .is_set(wow_entities::UNIT_DATA_HEALTH_BIT))
            .unwrap_or(false),
        "the health write marks the UnitData field for the next VALUES update"
    );
}

#[test]
fn health_regeneration_tick_suppresses_in_combat_without_modifiers_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 91);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
                let mut flags = player.unit().unit_flags_like_cpp();
                flags.insert(wow_constants::unit::UnitFlags::IN_COMBAT);
                player.unit_mut().set_unit_flags_like_cpp(flags);
            })
            .is_some()
    );
    publish_health_regen_snapshot_like_cpp(&mut session, 20, 0);
    let tables = health_regen_game_tables_like_cpp(0.1, 0.2);
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    session.tick_player_regeneration_like_cpp(2_000, &power_types, Some(&tables));

    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((100, 1_000)),
        "in combat without a during-combat aura there is no health regeneration"
    );
}

fn power_type_store_like_cpp(
    power: PowerType,
    regen_peace: f32,
    regen_combat: f32,
) -> wow_data::character_progression::PowerTypeStore {
    wow_data::character_progression::PowerTypeStore::from_entries([
        wow_data::character_progression::PowerTypeEntry {
            id: 0,
            name_global_string_tag: String::new(),
            cost_global_string_tag: String::new(),
            power_type_enum: power as i8,
            min_power: 0,
            max_base_power: 0,
            center_power: 0,
            default_power: 0,
            display_modifier: 1,
            regen_interrupt_time_ms: 0,
            regen_peace,
            regen_combat,
            flags: 0,
        },
    ])
}

fn set_represented_primary_power_like_cpp(
    session: &mut WorldSession,
    power: PowerType,
    current: i32,
    max: i32,
) {
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                for raw in 0..wow_entities::MAX_POWERS as i8 {
                    if let Some(candidate) = <PowerType as num_traits::FromPrimitive>::from_i8(raw)
                    {
                        player.unit_mut().set_power_index(candidate, None);
                    }
                }
                player.unit_mut().set_power_index(power, Some(0));
                player.unit_mut().set_max_power(power, max);
                player.unit_mut().set_power(power, current);
            })
            .is_some()
    );
}

#[test]
fn non_mana_power_regeneration_tick_decays_rage_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 92);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
            })
            .is_some()
    );
    set_represented_primary_power_like_cpp(&mut session, PowerType::Rage, 50, 100);
    publish_health_regen_snapshot_like_cpp(&mut session, 0, 0);
    // C++ `PowerTypeEntry.RegenPeace` for rage is a per-second decay.
    let power_types = power_type_store_like_cpp(PowerType::Rage, -1.0, 0.0);

    session.tick_player_regeneration_like_cpp(2_000, &power_types, None);

    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Rage),
        Some((48, 100)),
        "the non-mana power loop decays the represented rage power"
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&wow_constants::ServerOpcodes::PowerUpdate),
        "crossing the two-second boundary publishes SMSG_POWER_UPDATE for the decayed power"
    );
}

#[test]
fn non_mana_power_regeneration_tick_throttles_energy_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 93);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 4, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
            })
            .is_some()
    );
    set_represented_primary_power_like_cpp(&mut session, PowerType::Energy, 50, 100);
    publish_health_regen_snapshot_like_cpp(&mut session, 0, 0);
    let power_types = power_type_store_like_cpp(PowerType::Energy, 10.0, 0.0);

    session.tick_player_regeneration_like_cpp(1_000, &power_types, None);

    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Energy),
        Some((60, 100))
    );
    assert!(
        !drain_server_opcodes(&send_rx).contains(&wow_constants::ServerOpcodes::PowerUpdate),
        "energy regeneration is throttled before the 2000ms boundary"
    );
}
