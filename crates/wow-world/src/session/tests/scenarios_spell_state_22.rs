//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn battle_pet_grant_experience_pet_battle_uses_owner_xp_aura_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1A7);
    let player_guid = ObjectGuid::create_player(1, 226);
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);
    session.set_player_guid(Some(player_guid));
    session.set_represented_battle_pet_xp_per_level_like_cpp(23, 100);
    session.set_represented_battle_pet_xp_per_level_like_cpp(24, 100);
    session.set_represented_battle_pet_xp_per_level_like_cpp(25, 100);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            breed: 7,
            level: 23,
            exp: 0,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
            quality: 3,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);
    session
        .apply_represented_battle_pet_xp_pct_aura_like_cpp(
            99_991,
            player_guid,
            &wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_BATTLE_PET_XP_PCT,
                effect_base_points: 50,
                ..Default::default()
            },
        )
        .expect("represented pet-battle XP aura");
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_with_owner_auras_like_cpp(
            pet_guid,
            200,
            RepresentedBattlePetXpSourceLikeCpp::PetBattle,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::Changed
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("experienced pet");
    assert_eq!(pet.level, MAX_BATTLE_PET_LEVEL_LIKE_CPP);
    assert_eq!(pet.exp, 0);
    let expected = [
        RepresentedBattlePetLevelCriteriaLikeCpp {
            species: 11,
            level: 24,
        },
        RepresentedBattlePetLevelCriteriaLikeCpp {
            species: 11,
            level: 25,
        },
    ];
    assert_eq!(
        session.represented_battle_pet_level_criteria_like_cpp(),
        &expected
    );
    assert_eq!(
        session.represented_battle_pet_active_level_criteria_like_cpp(),
        &expected
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::BattlePetUpdates]
    );
}
#[test]
fn toy_item_spell_effect_guard_uses_item_effect_parent_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_item_effect_store(Arc::new(ItemEffectStore::from_entries([ItemEffectEntry {
        id: 1,
        legacy_slot_index: 0,
        trigger_type: 0,
        charges: 0,
        cooldown_msec: 0,
        category_cooldown_msec: 0,
        spell_category_id: 0,
        spell_id: 12_345,
        chr_specialization_id: 0,
        parent_item_id: 30_000,
    }])));

    assert!(session.toy_item_has_spell_effect_like_cpp(30_000, 12_345));
    assert!(!session.toy_item_has_spell_effect_like_cpp(30_000, 54_321));
    assert!(!session.toy_item_has_spell_effect_like_cpp(30_001, 12_345));
}
#[test]
fn toy_item_spell_cooldown_uses_item_effect_override_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 12_345;
    let spell_info = wow_data::SpellInfo {
        recovery_time_ms: 1_500,
        cooldown_ms: 2_000,
        ..instant_toy_spell_info_like_cpp(spell_id)
    };

    session.set_item_effect_store(Arc::new(ItemEffectStore::from_entries([
        ItemEffectEntry {
            id: 1,
            legacy_slot_index: 0,
            trigger_type: 0,
            charges: 0,
            cooldown_msec: 5_000,
            category_cooldown_msec: 0,
            spell_category_id: 0,
            spell_id,
            chr_specialization_id: 0,
            parent_item_id: 30_000,
        },
        ItemEffectEntry {
            id: 2,
            legacy_slot_index: 0,
            trigger_type: 0,
            charges: 0,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id,
            chr_specialization_id: 0,
            parent_item_id: 30_001,
        },
    ])));

    assert_eq!(
        session.toy_item_spell_cooldown_ms_like_cpp(30_000, spell_id, &spell_info),
        5_000
    );
    assert_eq!(
        session.toy_item_spell_cooldown_ms_like_cpp(30_001, spell_id, &spell_info),
        2_000
    );
}
#[test]
fn handle_use_toy_sends_prepare_and_toy_cast_flags_like_cpp() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (mut session, _, send_rx) = make_session();
            let player_guid = ObjectGuid::create_player(1, 31_100);
            let client_cast_id = ObjectGuid::create_player(1, 31_101);
            let item_id = 30_000;
            let spell_id = 12_345;

            session.set_player_guid(Some(player_guid));
            session.set_player_map_position_like_cpp(571, Position::new(10.0, 10.0, 0.0, 0.0));
            install_stackable_test_item_template(&mut session, item_id, 1);
            session.load_represented_account_toys_like_cpp([(item_id, false, false)]);
            session.set_item_effect_store(Arc::new(ItemEffectStore::from_entries([
                ItemEffectEntry {
                    id: 1,
                    legacy_slot_index: 0,
                    trigger_type: 0,
                    charges: 0,
                    cooldown_msec: 0,
                    category_cooldown_msec: 0,
                    spell_category_id: 0,
                    spell_id,
                    chr_specialization_id: 0,
                    parent_item_id: item_id,
                },
            ])));
            let mut spell_store = SpellStore::new();
            spell_store.insert(spell_id, instant_toy_spell_info_like_cpp(spell_id));
            session.set_spell_store(Arc::new(spell_store));

            session
                .handle_use_toy(write_minimal_use_toy_packet_like_cpp(
                    item_id,
                    spell_id,
                    client_cast_id,
                ))
                .await;

            let prepare = send_rx.try_recv().expect("SpellPrepare packet");
            assert_eq!(
                wow_packet::WorldPacket::from_bytes(&prepare).server_opcode(),
                Some(ServerOpcodes::SpellPrepare)
            );
            let mut prepare_body = WorldPacket::from_bytes(&prepare[2..]);
            assert_eq!(prepare_body.read_packed_guid().unwrap(), client_cast_id);
            let server_cast_id = prepare_body.read_packed_guid().unwrap();
            assert_eq!(server_cast_id.high_type(), HighGuid::Cast);

            let go = send_rx.try_recv().expect("SpellGo packet");
            assert_eq!(
                wow_packet::WorldPacket::from_bytes(&go).server_opcode(),
                Some(ServerOpcodes::SpellGo)
            );
            let mut go_body = WorldPacket::from_bytes(&go[2..]);
            assert_eq!(go_body.read_packed_guid().unwrap(), player_guid);
            assert_eq!(go_body.read_packed_guid().unwrap(), player_guid);
            assert_eq!(go_body.read_packed_guid().unwrap(), server_cast_id);
            assert_eq!(go_body.read_packed_guid().unwrap(), client_cast_id);
            assert_eq!(go_body.read_int32().unwrap(), spell_id);
            let _visual = wow_packet::packets::spell::SpellCastVisual::read(&mut go_body)
                .expect("SpellVisual");
            assert_eq!(go_body.read_uint32().unwrap(), 0);
            assert_eq!(
                go_body.read_uint32().unwrap(),
                CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP
            );
        });
}
#[test]
fn handle_use_toy_rejects_second_cast_on_item_effect_cooldown_like_cpp() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (mut session, _, send_rx) = make_session();
            let player_guid = ObjectGuid::create_player(1, 31_200);
            let first_client_cast_id = ObjectGuid::create_player(1, 31_201);
            let second_client_cast_id = ObjectGuid::create_player(1, 31_202);
            let item_id = 30_000;
            let spell_id = 12_345;

            session.set_player_guid(Some(player_guid));
            session.set_player_map_position_like_cpp(571, Position::new(10.0, 10.0, 0.0, 0.0));
            install_stackable_test_item_template(&mut session, item_id, 1);
            session.load_represented_account_toys_like_cpp([(item_id, false, false)]);
            session.set_item_effect_store(Arc::new(ItemEffectStore::from_entries([
                ItemEffectEntry {
                    id: 1,
                    legacy_slot_index: 0,
                    trigger_type: 0,
                    charges: 0,
                    cooldown_msec: 5_000,
                    category_cooldown_msec: 0,
                    spell_category_id: 0,
                    spell_id,
                    chr_specialization_id: 0,
                    parent_item_id: item_id,
                },
            ])));
            let mut spell_store = SpellStore::new();
            spell_store.insert(spell_id, instant_toy_spell_info_like_cpp(spell_id));
            session.set_spell_store(Arc::new(spell_store));

            session
                .handle_use_toy(write_minimal_use_toy_packet_like_cpp(
                    item_id,
                    spell_id,
                    first_client_cast_id,
                ))
                .await;

            assert_eq!(
                drain_server_opcodes(&send_rx),
                vec![
                    ServerOpcodes::SpellPrepare,
                    ServerOpcodes::SpellGo,
                    ServerOpcodes::CooldownEvent
                ]
            );

            session
                .handle_use_toy(write_minimal_use_toy_packet_like_cpp(
                    item_id,
                    spell_id,
                    second_client_cast_id,
                ))
                .await;

            let failed = send_rx.try_recv().expect("CastFailed packet");
            assert_eq!(
                wow_packet::WorldPacket::from_bytes(&failed).server_opcode(),
                Some(ServerOpcodes::CastFailed)
            );
            let mut failed_body = WorldPacket::from_bytes(&failed[2..]);
            assert_eq!(
                failed_body.read_packed_guid().unwrap(),
                second_client_cast_id
            );
            assert_eq!(failed_body.read_int32().unwrap(), spell_id);
            let _ = wow_packet::packets::spell::SpellCastVisual::read(&mut failed_body)
                .expect("visual");
            assert_eq!(
                failed_body.read_int32().unwrap(),
                SpellCastResult::NotReady as i32
            );
            assert_eq!(failed_body.read_int32().unwrap(), 0);
            assert_eq!(failed_body.read_int32().unwrap(), 0);
            assert!(send_rx.try_recv().is_err());
        });
}
#[tokio::test]
async fn spell_learn_transmog_set_adds_valid_appearances_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 73);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let spell_id = 73_100;
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogSpellTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries([
        TransmogSetItemEntry {
            id: 1,
            transmog_set_id: 700,
            item_modified_appearance_id: 65,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 2,
            transmog_set_id: 700,
            item_modified_appearance_id: 999,
            flags: 0,
        },
    ])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([ItemModifiedAppearanceEntry {
            id: 65,
            item_id: 777,
            item_appearance_modifier_id: 0,
            item_appearance_id: 9_000,
            order_index: 0,
            transmog_source_type_enum: 0,
        }]),
    ));
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET,
                effect_misc_value_1: 700,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented learn transmog set should execute");

    assert!(session.represented_has_item_appearance_like_cpp(65));
    assert!(!session.represented_has_item_appearance_like_cpp(999));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_learn_transmog_set_skips_missing_set_or_non_player_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 74);
    let other_player_guid = ObjectGuid::create_player(1, 75);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let spell_id = 73_101;
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogSpellSkipTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries([])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([]),
    ));
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET,
                effect_misc_value_1: 700,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    for target_guid in [player_guid, other_player_guid] {
        session
            .execute_spell(spell_id, target_guid)
            .await
            .expect("represented learn transmog set guard should no-op");
        assert!(!session.represented_has_item_appearance_like_cpp(65));
        assert_eq!(
            drain_server_opcodes(&send_rx),
            vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
        );
    }
}
#[test]
fn represented_item_mods_apply_scaling_stat_loop_spell_bonus_and_armor_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 902);
    session.set_player_guid(Some(player_guid));
    session.set_player_level_like_cpp(80);
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: 102,
        class_id: ItemClass::Armor as u8,
        subclass_id: 1,
        material: 0,
        inventory_type: InventoryType::Chest as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 78,
        scaling_stat_value: 0x0010_8008,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            102,
            ItemStatEntry {
                stats: [
                    (ItemModType::Intellect as i8, 999),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                ],
                resistances: [17, 0, 0, 0, 0, 0, 0],
                armor: 17,
            },
        )],
        [],
    )));
    let mut stat_id = [-1; 10];
    stat_id[0] = ItemModType::Strength as i32;
    let mut bonus = [0; 10];
    bonus[0] = 5_000;
    session.set_scaling_stat_distribution_store(Arc::new(
        ScalingStatDistributionStore::from_entries([ScalingStatDistributionEntry {
            id: 78,
            player_level_to_item_level_curve_id: 0,
            min_level: 10,
            max_level: 20,
            bonus,
            stat_id,
        }]),
    ));
    session.set_scaling_stat_values_store(Arc::new(ScalingStatValuesStore::from_entries([
        ScalingStatValuesEntry {
            id: 20,
            char_level: 20,
            weapon_dps_1h: 0,
            weapon_dps_2h: 0,
            spellcaster_dps_1h: 0,
            spellcaster_dps_2h: 0,
            ranged_dps: 0,
            wand_dps: 0,
            spell_power: 33,
            shoulder_budget: 0,
            trinket_budget: 0,
            weapon_budget_1h: 0,
            primary_budget: 200,
            ranged_budget: 0,
            tertiary_budget: 0,
            cloth_shoulder_armor: 0,
            leather_shoulder_armor: 0,
            mail_shoulder_armor: 0,
            plate_shoulder_armor: 0,
            cloth_cloak_armor: 0,
            cloth_chest_armor: 77,
            leather_chest_armor: 0,
            mail_chest_armor: 0,
            plate_chest_armor: 0,
        },
    ])));
    session.inventory_items.insert(
        EQUIPMENT_SLOT_CHEST,
        InventoryItem {
            guid: item_guid,
            entry_id: 102,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        102,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    session.insert_inventory_item_object(item);

    session.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_CHEST, true);

    assert_eq!(
        session.represented_item_bonus_actions_like_cpp(),
        &[
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_CHEST,
                action: ApplyEnchantmentEffectAction::UnitModifier {
                    unit_mod: wow_entities::ApplyEnchantmentUnitMod::StatStrength,
                    modifier: wow_entities::ApplyEnchantmentUnitModifier::BaseValue,
                    amount: 100,
                    apply: true,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_CHEST,
                action: ApplyEnchantmentEffectAction::UpdateStatBuffMod(
                    wow_constants::Stats::Strength,
                ),
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_CHEST,
                action: ApplyEnchantmentEffectAction::SpellPowerBonus {
                    amount: 33,
                    apply: true,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_CHEST,
                action: ApplyEnchantmentEffectAction::UnitModifier {
                    unit_mod: wow_entities::ApplyEnchantmentUnitMod::Resistance(
                        wow_constants::spell::SpellSchools::Normal as u32,
                    ),
                    modifier: wow_entities::ApplyEnchantmentUnitModifier::BaseValue,
                    amount: 77,
                    apply: true,
                },
            },
        ],
        "C++ uses ScalingStatDistribution stat slots instead of ItemSparse stats, then applies getSpellBonus and getArmorMod"
    );
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base
            [wow_constants::Stats::Strength as usize],
        100
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .spell_power_bonus,
        33
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .resistances_base[wow_constants::spell::SpellSchools::Normal as usize],
        77
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .stat_buff_updates,
        &[wow_constants::Stats::Strength]
    );
    let stat_changes =
        represented_player_stat_changes_like_cpp(&session.represented_item_bonus_state_like_cpp());
    assert_eq!(
        stat_changes.stats[wow_constants::Stats::Strength as usize],
        100
    );
    assert_eq!(
        stat_changes.stat_pos_buff[wow_constants::Stats::Strength as usize],
        100
    );
    assert_eq!(
        stat_changes.spell_power, 33,
        "C++ ApplySpellPowerBonus updates ModHealingDonePos and magic ModDamageDonePos update fields"
    );
    assert_eq!(
        stat_changes.armor, 77,
        "C++ armor/resistance item mods surface as UnitData::Resistances[0]"
    );

    session.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_CHEST, false);

    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base
            [wow_constants::Stats::Strength as usize],
        0,
        "C++ _ApplyItemBonuses(..., false) removes the same represented stat delta"
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .spell_power_bonus,
        0
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .resistances_base[wow_constants::spell::SpellSchools::Normal as usize],
        0
    );
    let removed_changes =
        represented_player_stat_changes_like_cpp(&session.represented_item_bonus_state_like_cpp());
    assert_eq!(
        removed_changes.stats[wow_constants::Stats::Strength as usize],
        0
    );
    assert_eq!(removed_changes.spell_power, 0);
    assert_eq!(removed_changes.armor, 0);
}
#[test]
fn represented_item_set_add_remove_tracks_threshold_spell_events_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = ObjectGuid::create_item(1, 906);
    let hands_guid = ObjectGuid::create_item(1, 907);
    session.set_player_guid(Some(player_guid));
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 700,
        name: "Test Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| match i {
            0 => 100,
            1 => 101,
            _ => 0,
        }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 1,
            chr_spec_id: 0,
            spell_id: 9001,
            threshold: 2,
            item_set_id: 700,
        },
        ItemSetSpellEntry {
            id: 2,
            chr_spec_id: 0,
            spell_id: 9002,
            threshold: 3,
            item_set_id: 700,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
        100,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        101,
        InventoryType::Hands,
    );

    assert!(!session.record_represented_items_set_item_like_cpp(chest_guid, true));
    assert!(session.record_represented_items_set_item_like_cpp(hands_guid, true));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp(),
        &[RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 700,
            spell_entry_id: 1,
            spell_id: 9001,
            threshold: 2,
            apply: true,
        }]
    );
    assert_eq!(
        session
            .represented_item_set_effect_like_cpp(700)
            .expect("set effect")
            .equipped_items
            .len(),
        2
    );

    assert!(session.record_represented_items_set_item_like_cpp(chest_guid, false));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp()[1],
        RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 700,
            spell_entry_id: 1,
            spell_id: 9001,
            threshold: 2,
            apply: false,
        }
    );
}
