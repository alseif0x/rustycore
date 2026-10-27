use super::*;
use std::sync::Arc;
use wow_constants::ServerOpcodes;
use wow_data::spell_db2::{
    SpellCastTimesEntry, SpellCategoriesEntry, SpellCategoryEntry, SpellCooldownsEntry,
    SpellEffectDb2Entry, SpellMiscEntry, SpellReagentsEntry,
};

const ON_PICKUP_TEST_SPELL: i32 = 61_090;
const ON_PICKUP_ITEM: u32 = 846_721;
const CREATE_ITEM_OUTPUT: u32 = 846_722;
const CREATE_ITEM_EFFECT_TYPE: u32 = 24;

fn spell_effect_row(
    id: u32,
    spell_id: u32,
    effect: u32,
    effect_base_points: i32,
    effect_item_type: i32,
) -> SpellEffectDb2Entry {
    SpellEffectDb2Entry {
        id,
        difficulty_id: i32::from(DIFFICULTY),
        effect_index: 0,
        effect,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: 0,
        effect_aura_period: 0,
        effect_base_points,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_die_sides: 0,
        effect_item_type,
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
        spell_id,
    }
}

fn install_self_res_onpickup_spell_fixture(
    session: &mut WorldSession,
    onpickup_effect: u32,
    onpickup_cast_time_ms: u32,
) {
    let casting_time_index = if onpickup_cast_time_ms == 0 { 0_u16 } else { 1_u16 };
    let mut self_res_attributes = [0; 15];
    self_res_attributes[0] =
        wow_data::spell::attributes::SPELL_ATTR0_ALLOW_CAST_WHILE_DEAD as i32;
    let misc_rows = [
        SpellMiscEntry {
            id: 1,
            attributes: self_res_attributes,
            difficulty_id: DIFFICULTY,
            spell_id: SELF_RES_TEST_SPELL as u32,
            ..Default::default()
        },
        SpellMiscEntry {
            id: 2,
            casting_time_index,
            difficulty_id: DIFFICULTY,
            spell_id: ON_PICKUP_TEST_SPELL as u32,
            ..Default::default()
        },
    ];
    let onpickup_effect_item_type = if onpickup_effect == CREATE_ITEM_EFFECT_TYPE {
        CREATE_ITEM_OUTPUT as i32
    } else {
        0
    };
    let self_res_effect = spell_effect_row(
        1,
        SELF_RES_TEST_SPELL as u32,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT,
        20,
        0,
    );
    let onpickup_effect_row = spell_effect_row(
        2,
        ON_PICKUP_TEST_SPELL as u32,
        onpickup_effect,
        0,
        onpickup_effect_item_type,
    );
    if onpickup_effect == CREATE_ITEM_EFFECT_TYPE {
        assert_eq!(onpickup_effect_item_type, CREATE_ITEM_OUTPUT as i32);
    }
    let categories = [
        SpellCategoriesEntry {
            id: 1,
            difficulty_id: DIFFICULTY,
            category: 3,
            defense_type: 0,
            dispel_type: 0,
            mechanic: 0,
            prevention_type: 0,
            start_recovery_category: 0,
            charge_category: 0,
            spell_id: SELF_RES_TEST_SPELL as u32,
        },
        SpellCategoriesEntry {
            id: 2,
            difficulty_id: DIFFICULTY,
            category: 4,
            defense_type: 0,
            dispel_type: 0,
            mechanic: 0,
            prevention_type: 0,
            start_recovery_category: 0,
            charge_category: 0,
            spell_id: ON_PICKUP_TEST_SPELL as u32,
        },
    ];
    let cooldowns = [SpellCooldownsEntry {
        id: 1,
        difficulty_id: DIFFICULTY,
        category_recovery_time: 10_000,
        recovery_time: 10_000,
        spell_id: SELF_RES_TEST_SPELL as u32,
        ..Default::default()
    }];
    let cast_times = (onpickup_cast_time_ms != 0).then_some(SpellCastTimesEntry {
        id: u32::from(casting_time_index),
        base: i32::try_from(onpickup_cast_time_ms).expect("fixture cast time fits DB2"),
        ..Default::default()
    });
    let effective = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::spell_db2::SpellCategoriesStore::from_entries(categories),
        wow_data::spell_db2::SpellMiscStore::from_entries(misc_rows.clone()),
        wow_data::spell_db2::SpellEffectDb2Store::from_entries([
            self_res_effect,
            onpickup_effect_row,
        ]),
        wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
        wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
        wow_data::spell_db2::SpellCastTimesStore::from_entries(cast_times),
        wow_data::spell_db2::SpellCooldownsStore::from_entries(cooldowns),
        wow_data::spell_db2::SpellCastingRequirementsStore::from_entries([]),
        wow_data::spell_db2::SpellPowerStore::from_entries([]),
        wow_data::spell_db2::SpellPowerDifficultyStore::from_entries([]),
    );
    let mut spells = crate::selected_spell_fixtures::canonical_spell_store([
        (SELF_RES_TEST_SPELL as u32, DIFFICULTY),
        (ON_PICKUP_TEST_SPELL as u32, DIFFICULTY),
    ])
    .hydrate_effective_core_db2_like_cpp(effective);
    spells.install_effective_cast_costs_like_cpp(
        wow_data::SpellCastCostStoreLikeCpp::from_effective_rows_like_cpp(
            [SpellReagentsEntry {
                id: 1,
                spell_id: SELF_RES_TEST_SPELL,
                reagent: [REAGENT_ITEM as i32, 0, 0, 0, 0, 0, 0, 0],
                reagent_count: [1, 0, 0, 0, 0, 0, 0, 0],
            }],
            [],
        ),
    );
    for spell_id in [SELF_RES_TEST_SPELL, ON_PICKUP_TEST_SPELL] {
        assert!(spells.contains_spell_info_exact_like_cpp(spell_id as u32, DIFFICULTY));
        assert!(!spells.contains_spell_info_exact_like_cpp(spell_id as u32, 0));
    }
    session.set_spell_store(Arc::new(spells));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
    session.set_spell_category_store(Arc::new(wow_data::SpellCategoryStore::from_entries([
        SpellCategoryEntry {
            id: 3,
            name: "self-res test".into(),
            flags: 0,
            uses_per_week: 0,
            max_charges: 0,
            charge_recovery_time: 0,
            type_mask: 0,
        },
        SpellCategoryEntry {
            id: 4,
            name: "OnPickup test".into(),
            flags: 0,
            uses_per_week: 0,
            max_charges: 0,
            charge_recovery_time: 0,
            type_mask: 0,
        },
    ])));
    let mut config = session.config.creature_aggro_config_like_cpp.clone();
    let mut visual_rows = config
        .spell_x_spell_visual_store
        .as_deref()
        .map(|store| store.entries_like_cpp().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    visual_rows.retain(|row| {
        !(row.spell_id == ON_PICKUP_TEST_SPELL as u32 && row.difficulty_id == DIFFICULTY)
    });
    visual_rows.push(wow_data::SpellXSpellVisualEntry {
        id: 2,
        difficulty_id: DIFFICULTY,
        spell_visual_id: 69,
        probability: 1.0,
        flags: 0,
        priority: 0,
        spell_icon_file_id: 0,
        active_icon_file_id: 0,
        viewer_unit_condition_id: 0,
        viewer_player_condition_id: 0,
        caster_unit_condition_id: 0,
        caster_player_condition_id: 0,
        spell_id: ON_PICKUP_TEST_SPELL as u32,
    });
    config.spell_x_spell_visual_store = Some(Arc::new(
        wow_data::SpellXSpellVisualStore::from_entries(visual_rows),
    ));
    session.set_creature_aggro_config_like_cpp(config);
}

fn install_onpickup_item_catalogs(session: &mut WorldSession) {
    let item_record = |id| wow_data::ItemRecord {
        id,
        class_id: wow_constants::ItemClass::Consumable as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: wow_constants::InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    };
    let sparse_template = || wow_data::ItemSparseTemplateEntry {
        flags: [0; 4],
        bag_family: 0,
        start_quest_id: 0,
        stackable: 20,
        max_count: 0,
        lock_id: 0,
        required_reputation_rank: 0,
        sell_price: 0,
        buy_price: 0,
        vendor_stack_count: 1,
        price_variance: 1.0,
        price_random_value: 1.0,
        max_durability: 0,
        other_faction_item_id: 0,
        content_tuning_id: 0,
        player_level_to_item_level_curve_id: 0,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0; 2],
        required_reputation_faction: 0,
        allowable_class: -1,
        required_expansion: 0,
        bonding: wow_constants::ItemBondingType::None as u8,
        container_slots: 0,
        inventory_type: wow_constants::InventoryType::NonEquip as i8,
    };
    let item_level = wow_data::ItemRandomPropertyTemplateEntry {
        item_level: 50,
        quality: wow_constants::ItemQuality::Normal as i8,
        inventory_type: wow_constants::InventoryType::NonEquip as i8,
    };
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records([
        item_record(REAGENT_ITEM),
        item_record(ON_PICKUP_ITEM),
        item_record(CREATE_ITEM_OUTPUT),
    ])));
    session.set_item_stats_store(Arc::new(
        wow_data::ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (REAGENT_ITEM, sparse_template()),
                (ON_PICKUP_ITEM, sparse_template()),
                (CREATE_ITEM_OUTPUT, sparse_template()),
            ],
            [
                (REAGENT_ITEM, item_level),
                (ON_PICKUP_ITEM, item_level),
                (CREATE_ITEM_OUTPUT, item_level),
            ],
        ),
    ));
    session.set_item_effect_store(Arc::new(wow_data::ItemEffectStore::from_entries([
        wow_data::ItemEffectEntry {
            id: 1,
            legacy_slot_index: 0,
            trigger_type: wow_constants::ItemSpelltriggerType::OnPickup as i8,
            charges: 0,
            cooldown_msec: 0,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: ON_PICKUP_TEST_SPELL,
            chr_specialization_id: 0,
            parent_item_id: ON_PICKUP_ITEM,
        },
    ])));
}

fn install_onpickup_item(session: &mut WorldSession, guid_counter: i64) -> ObjectGuid {
    let owner = session.player_guid().expect("recovery Player GUID");
    let slot = wow_entities::INVENTORY_SLOT_ITEM_START + 1;
    let guid = ObjectGuid::create_item(1, guid_counter);
    session.insert_inventory_item_like_cpp(
        slot,
        crate::session::InventoryItem {
            guid,
            entry_id: ON_PICKUP_ITEM,
            db_guid: guid.counter() as u64,
            inventory_type: None,
        },
    );
    let item = session.make_inventory_item_object(
        guid,
        ON_PICKUP_ITEM,
        owner,
        1,
        0,
        ItemContext::None,
        slot,
    );
    session.insert_inventory_item_object(item);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .store_top_level_item(slot, guid)
                .expect("OnPickup item backpack slot");
        })
        .expect("canonical Player owns the OnPickup item");
    guid
}

fn spell_go_ids(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<i32> {
    spell_go_ids_from_frames(&outgoing_frames(send_rx))
}

fn spell_go_ids_from_frames(frames: &[Vec<u8>]) -> Vec<i32> {
    spell_cast_ids_from_frames(frames, ServerOpcodes::SpellGo)
}

fn spell_start_ids_from_frames(frames: &[Vec<u8>]) -> Vec<i32> {
    spell_cast_ids_from_frames(frames, ServerOpcodes::SpellStart)
}

fn spell_cast_ids_from_frames(frames: &[Vec<u8>], opcode: ServerOpcodes) -> Vec<i32> {
    frames
        .iter()
        .cloned()
        .filter_map(|bytes| {
            let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
            (packet.read_uint16().ok()? == opcode as u16)
                .then(|| {
                    packet.read_packed_guid().expect("spell cast caster");
                    packet
                        .read_packed_guid()
                        .expect("spell cast caster unit");
                    packet.read_packed_guid().expect("spell cast cast ID");
                    packet.read_packed_guid().expect("spell cast original cast ID");
                    packet.read_int32().expect("spell cast spell ID")
                })
        })
        .collect()
}

#[tokio::test]
async fn registered_self_res_holds_before_bones_for_timed_unimplemented_onpickup_effect() {
    let (mut session, _packet_tx, send_rx, save, corpse_port, _reagent) =
        setup_registered_self_res(true, 61_060);
    install_self_res_onpickup_spell_fixture(&mut session, CREATE_ITEM_EFFECT_TYPE, 1_500);
    install_onpickup_item_catalogs(&mut session);
    let item_guid = install_onpickup_item(&mut session, 61_061);
    let corpse_guid = add_current_map_corpse(&mut session, 61_062);
    let difficulty = session.core.current_map_difficulty_id_like_cpp();
    let spell_store = session.catalogs.spell_store().expect("exact fixture SpellStore");
    assert!(spell_store.contains_spell_info_exact_like_cpp(
        ON_PICKUP_TEST_SPELL as u32,
        DIFFICULTY
    ));
    assert!(!spell_store.contains_spell_info_exact_like_cpp(ON_PICKUP_TEST_SPELL as u32, 0));
    let (onpickup, selected_difficulty) = spell_store
        .spell_info_for_difficulty_like_cpp(
            ON_PICKUP_TEST_SPELL,
            difficulty,
            session.catalogs.difficulty_store().map(AsRef::as_ref),
        )
        .expect("exact OnPickup SpellInfo resolves")
        .expect("exact OnPickup SpellInfo exists");
    assert_eq!(selected_difficulty, DIFFICULTY);
    assert_eq!(onpickup.cast_time_ms, 1_500, "OnPickup is genuinely timed");
    assert_eq!(onpickup.effects().len(), 1);
    assert_eq!(onpickup.effects()[0].effect, CREATE_ITEM_EFFECT_TYPE);
    assert_eq!(onpickup.effects()[0].effect_index, 0);
    assert_eq!(onpickup.effects()[0].effect_base_points, 0);
    assert!(session
        .catalogs
        .item_template_flags(CREATE_ITEM_OUTPUT)
        .is_some());
    assert!(session
        .catalogs
        .items
        .store
        .as_ref()
        .and_then(|store| store.get(CREATE_ITEM_OUTPUT))
        .is_some());
    assert!(session
        .catalogs
        .items
        .stats_store
        .as_ref()
        .and_then(|store| store.random_property_template(CREATE_ITEM_OUTPUT))
        .is_some());
    assert_eq!(
        wow_world_spell::SpellExecutionHostLikeCpp::server_normal_physical_item_facts_like_cpp(
            &session, item_guid
        ),
        Some((ON_PICKUP_ITEM, 50)),
        "the OnPickup cast uses its carried physical item and loaded ItemSparse level"
    );
    assert!(session
        .catalogs
        .items
        .effect_store
        .as_ref()
        .expect("OnPickup effect catalog")
        .item_effects_for_item_id_like_cpp(ON_PICKUP_ITEM)
        .iter()
        .any(|effect| {
            effect.trigger_type == wow_constants::ItemSpelltriggerType::OnPickup as i8
                && effect.spell_id == ON_PICKUP_TEST_SPELL
        }));

    session
        .dispatch_packet(
            &SessionHandlerCatalogsLikeCpp::default(),
            registered_self_res_packet(),
        )
        .await;

    assert_eq!(
        session.with_owned_player_like_cpp(|player| (
            player.unit().death_state(),
            player.unit().is_alive(),
            player.unit().data().health,
        )),
        Some((wow_constants::DeathState::Alive, true, 0)),
        "the canonical Unit retains Source Alive at zero health during obtain"
    );
    assert_eq!(
        session
            .with_owned_player_like_cpp(|player| {
                player
                    .resurrection_state_like_cpp()
                    .delayed_after_teleport
                    .map(|continuation| continuation.phase)
            })
            .flatten(),
        Some(wow_entities::PlayerResurrectionPhase::ObtainSpellsInFlight)
    );
    let frames = outgoing_frames(&send_rx);
    let go_ids = spell_go_ids_from_frames(&frames);
    assert!(go_ids.contains(&SELF_RES_TEST_SPELL));
    assert!(!go_ids.contains(&ON_PICKUP_TEST_SPELL));
    assert!(!spell_start_ids_from_frames(&frames).contains(&ON_PICKUP_TEST_SPELL));
    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
    assert_eq!(save.character_save_count(), 0);
    assert!(corpse_port.deletes.lock().unwrap().is_empty());
    assert!(session
        .core
        .canonical_map_manager
        .as_ref()
        .expect("canonical MapManager")
        .lock()
        .expect("canonical MapManager")
        .find_map(571, 10_000 + u32::from(DIFFICULTY))
        .expect("difficulty-three map")
        .map()
        .get_typed_corpse(corpse_guid)
        .is_some());
}

#[tokio::test]
async fn normal_onpickup_execution_reports_unimplemented_create_item_effect_at_alive_zero() {
    let (mut session, _packet_tx, send_rx, _save, _corpse, _reagent) =
        setup_registered_self_res(true, 61_063);
    install_self_res_onpickup_spell_fixture(&mut session, CREATE_ITEM_EFFECT_TYPE, 1_500);
    install_onpickup_item_catalogs(&mut session);
    let item_guid = install_onpickup_item(&mut session, 61_064);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_death_state(wow_constants::DeathState::Alive);
            player.unit_mut().set_health(0);
        })
        .expect("native resurrection intermediate Player remains canonical");
    assert_eq!(
        session.with_owned_player_like_cpp(|player| (
            player.unit().death_state(),
            player.unit().is_alive(),
            player.unit().data().health,
        )),
        Some((wow_constants::DeathState::Alive, true, 0)),
        "the canonical Unit retains Source Alive at zero health during direct obtain"
    );
    assert_eq!(
        wow_world_spell::SpellExecutionHostLikeCpp::server_normal_physical_item_facts_like_cpp(
            &session, item_guid
        ),
        Some((ON_PICKUP_ITEM, 50))
    );

    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let reason = wow_world_spell::cast_all_obtain_spells_like_cpp(
        &mut session,
        generators.item.as_ref(),
        &creature_spawns,
    )
    .await
    .expect_err("the normal OnPickup pipeline rejects the unimplemented effect");

    assert!(
        reason.contains("unimplemented spell effect"),
        "expected the route admission reason, got: {reason}"
    );
    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
    assert!(spell_go_ids(&send_rx).is_empty());
}

#[tokio::test]
async fn registered_self_res_completes_after_source_inert_onpickup_effect() {
    let (mut session, _packet_tx, send_rx, save, corpse_port, _reagent) =
        setup_registered_self_res(true, 61_065);
    install_self_res_onpickup_spell_fixture(
        &mut session,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_COMBO_POINTS,
        0,
    );
    install_onpickup_item_catalogs(&mut session);
    install_onpickup_item(&mut session, 61_066);
    let player_guid = session.player_guid().expect("canonical Player GUID");
    add_current_map_corpse(&mut session, 61_067);
    let spell_store = session.catalogs.spell_store().expect("exact fixture SpellStore");
    assert!(spell_store.contains_spell_info_exact_like_cpp(
        SELF_RES_TEST_SPELL as u32,
        DIFFICULTY
    ));
    assert!(spell_store.contains_spell_info_exact_like_cpp(
        ON_PICKUP_TEST_SPELL as u32,
        DIFFICULTY
    ));
    assert!(!spell_store.contains_spell_info_exact_like_cpp(SELF_RES_TEST_SPELL as u32, 0));
    assert!(!spell_store.contains_spell_info_exact_like_cpp(ON_PICKUP_TEST_SPELL as u32, 0));

    session
        .dispatch_packet(
            &SessionHandlerCatalogsLikeCpp::default(),
            registered_self_res_packet(),
        )
        .await;

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 20);
    let go_ids = spell_go_ids(&send_rx);
    assert_eq!(go_ids.len(), 2);
    assert!(go_ids.contains(&SELF_RES_TEST_SPELL));
    assert!(go_ids.contains(&ON_PICKUP_TEST_SPELL));
    assert_eq!(save.character_save_count(), 1);
    assert_eq!(
        corpse_port.deletes.lock().unwrap().as_slice(),
        &[player_guid.counter() as u64]
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|player| {
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
        }),
        Some(None)
    );
    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
}

