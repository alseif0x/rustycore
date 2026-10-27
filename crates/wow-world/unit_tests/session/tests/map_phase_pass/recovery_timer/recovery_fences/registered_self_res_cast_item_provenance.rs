use super::*;

const INEBRIATE_TEST_SPELL: i32 = 61_052;
const CAST_ITEM: u32 = 846_728;
const CAST_ITEM_SLOT: u8 = 24;

fn install_cast_item(session: &mut WorldSession, guid_counter: i64, count: u32) -> ObjectGuid {
    crate::session::tests::install_stackable_test_item_template(session, CAST_ITEM, 20);
    let guid = ObjectGuid::create_item(1, guid_counter);
    let owner = session.player_guid().expect("recovery Player GUID");
    session.insert_inventory_item_like_cpp(
        CAST_ITEM_SLOT,
        crate::session::InventoryItem {
            guid,
            entry_id: CAST_ITEM,
            db_guid: guid.counter() as u64,
            inventory_type: None,
        },
    );
    let mut item = session.make_inventory_item_object(
        guid,
        CAST_ITEM,
        owner,
        count,
        0,
        ItemContext::None,
        CAST_ITEM_SLOT,
    );
    item.set_spell_charges(0, -1);
    session.insert_inventory_item_object(item);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .store_top_level_item(CAST_ITEM_SLOT, guid)
                .expect("cast item backpack slot");
        })
        .expect("canonical Player owns the cast item");
    guid
}

fn install_consumable_effect(session: &mut WorldSession, charges: i16) {
    session.set_item_effect_store(Arc::new(wow_data::ItemEffectStore::from_entries([
        wow_data::ItemEffectEntry {
            id: 846_729,
            legacy_slot_index: 0,
            trigger_type: wow_constants::ItemSpelltriggerType::OnUse as i8,
            charges,
            cooldown_msec: 5_000,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: INEBRIATE_TEST_SPELL,
            chr_specialization_id: 0,
            parent_item_id: CAST_ITEM,
        },
    ])));
}

fn restore_reagent_and_cast_item_catalogs(session: &mut WorldSession) {
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
    let sparse_template = |stackable| wow_data::ItemSparseTemplateEntry {
        flags: [0; 4],
        bag_family: 0,
        start_quest_id: 0,
        stackable,
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
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records([
        item_record(REAGENT_ITEM),
        item_record(CAST_ITEM),
    ])));
    session.set_item_stats_store(Arc::new(
        wow_data::ItemStatsStore::from_sparse_templates([
            (REAGENT_ITEM, sparse_template(20)),
            (CAST_ITEM, sparse_template(20)),
        ]),
    ));
}

fn setup_inebriate_cast(
    has_hit_delay: bool,
    guid_counter: i64,
) -> (
    WorldSession,
    flume::Sender<WorldPacket>,
    ObjectGuid,
    ObjectGuid,
    ObjectGuid,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, _admission, packet_tx, send_rx) = logged_in_recovery_session();
    install_recovery_save_snapshot_inputs(&mut session);
    move_recovery_player_to_difficulty_three(&mut session);
    install_exact_test_spell(
        &mut session,
        INEBRIATE_TEST_SPELL,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE,
        35,
        has_hit_delay,
    );
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_spell_group_store(Arc::new(wow_data::SpellGroupStoreLikeCpp::default()));
    session.set_spell_group_stack_rule_store(Arc::new(
        wow_data::SpellGroupStackRuleStoreLikeCpp::default(),
    ));
    crate::player_inventory_test_fixture::complete_known_empty_player_inventory_like_cpp(
        &mut session,
    );
    crate::player_inventory_test_fixture::complete_known_empty_player_currencies_like_cpp(
        &mut session,
    );
    assert!(wow_world_spell::set_player_aura_authority_complete_like_cpp(
        &mut session,
        true,
    ));
    let player_guid = session.player_guid().expect("recovery Player GUID");
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_death_state(wow_constants::DeathState::Alive);
            player.unit_mut().set_health(100);
            player.set_inebriation_like_cpp(15);
        })
        .expect("canonical Player remains installed");

    let reagent_guid = install_reagent_item(&mut session, guid_counter);
    let cast_item_guid = install_cast_item(&mut session, guid_counter + 1, 2);
    restore_reagent_and_cast_item_catalogs(&mut session);
    install_consumable_effect(&mut session, -1);
    let (realm_tx, realm_rx) = flume::unbounded();
    session.core.install_realm_send_channel_for_test(realm_tx);
    (
        session,
        packet_tx,
        player_guid,
        cast_item_guid,
        reagent_guid,
        send_rx,
        realm_rx,
    )
}

fn exact_test_spell(session: &WorldSession) -> wow_data::SpellInfo {
    let hub = crate::session::hub_ref(session);
    hub.catalogs
        .spell_store()
        .expect("exact test SpellStore")
        .spell_info_for_exact_key_like_cpp(
            INEBRIATE_TEST_SPELL,
            DIFFICULTY,
            hub.catalogs.difficulty_store().map(AsRef::as_ref),
        )
        .expect("exact difficulty-three SpellInfo resolves")
        .expect("exact difficulty-three SpellInfo exists")
}

fn cast_item_metadata(cast_item_guid: ObjectGuid) -> SpellCastMetadata {
    SpellCastMetadata {
        actual_spell_difficulty_id_like_cpp: Some(DIFFICULTY),
        cast_item_entry: Some(CAST_ITEM),
        cast_item_guid: Some(cast_item_guid),
        cast_item_level: Some(71),
        server_normal_cast_like_cpp: true,
        ..Default::default()
    }
}

fn assert_hit_delay_authority(session: &WorldSession, expected: bool) {
    let hub = crate::session::hub_ref(session);
    let store = hub.catalogs.spell_store().expect("exact test SpellStore");
    let hit_delay = store
        .has_hit_delay_at_difficulty_like_cpp(
            INEBRIATE_TEST_SPELL as u32,
            DIFFICULTY,
            hub.catalogs.difficulty_store().map(AsRef::as_ref),
        )
        .expect("loaded selected SpellMisc row determines hit delay");
    assert_eq!(hit_delay, expected);
    let attributes = store
        .misc_attribute_words_at_difficulty_like_cpp(
            INEBRIATE_TEST_SPELL,
            DIFFICULTY,
            hub.catalogs.difficulty_store().map(AsRef::as_ref),
        )
        .expect("loaded selected SpellMisc row determines channel flags");
    let channelled = wow_data::spell::attributes::SPELL_ATTR1_IS_CHANNELLED
        | wow_data::spell::attributes::SPELL_ATTR1_IS_SELF_CHANNELLED;
    assert_eq!(attributes[1] as u32 & channelled, 0);
    assert_eq!(
        wow_world_spell::server_normal_take_cast_item_before_effects_like_cpp(
            hit_delay,
            attributes[1] as u32,
        ),
        expected,
    );
}

async fn execute_inebriate_cast(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    cast_item_guid: ObjectGuid,
) -> Result<(), &'static str> {
    let (cast_id, residence_revision) =
        wow_world_spell::allocate_player_cast_identity_like_cpp(session, INEBRIATE_TEST_SPELL)
            .expect("canonical Map allocates the normal cast identity and residence");
    let mut metadata = cast_item_metadata(cast_item_guid);
    metadata.prepared_residence_revision = Some(
        residence_revision.expect("canonical Player cast identity retains its residence stamp"),
    );
    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            INEBRIATE_TEST_SPELL,
            player_guid,
            cast_id,
            wow_packet::packets::spell::SpellCastVisual::default(),
            wow_packet::packets::spell::SpellTargetData {
                flags: 0x2,
                unit: player_guid,
                ..Default::default()
            },
            metadata,
        )
        .await
}

fn cast_item_count(session: &WorldSession, guid: ObjectGuid) -> u32 {
    session
        .with_owned_player_like_cpp(|player| {
            player
                .inventory_runtime_like_cpp()
                .item_objects()
                .get(&guid)
                .map(wow_entities::Item::count)
        })
        .expect("canonical Player remains installed")
        .expect("cast item remains in its stack")
}

fn cast_item_charge(session: &WorldSession, guid: ObjectGuid) -> i32 {
    session
        .with_owned_player_like_cpp(|player| {
            player
                .inventory_runtime_like_cpp()
                .item_objects()
                .get(&guid)
                .map(|item| item.data().spell_charges[0])
        })
        .expect("canonical Player remains installed")
        .expect("cast item remains in its stack")
}

fn reagent_state(
    session: &WorldSession,
    guid: ObjectGuid,
) -> Option<wow_constants::ItemUpdateState> {
    session
        .with_owned_player_like_cpp(|player| {
            player
                .inventory_runtime_like_cpp()
                .item_objects()
                .get(&guid)
                .map(wow_entities::Item::update_state)
        })
        .expect("canonical Player remains installed")
}

fn set_cast_item_effect_charges(session: &mut WorldSession, charges: i16) {
    install_consumable_effect(session, charges);
}

#[tokio::test]
async fn normal_cast_execution_takes_delayed_cast_item_before_inebriate_effect() {
    let (mut session, _packet_tx, player_guid, cast_item_guid, reagent_guid, send_rx, realm_rx) =
        setup_inebriate_cast(true, 61_050);
    assert_hit_delay_authority(&session, true);
    assert_eq!(cast_item_count(&session, cast_item_guid), 2);
    assert_eq!(cast_item_charge(&session, cast_item_guid), -1);

    execute_inebriate_cast(&mut session, player_guid, cast_item_guid)
        .await
        .expect("normal cast completes after its delayed cast-item debit");

    assert_eq!(cast_item_count(&session, cast_item_guid), 1);
    assert_eq!(
        reagent_state(&session, reagent_guid),
        Some(wow_constants::ItemUpdateState::Removed),
        "the distinct physical reagent is consumed during pre-GO costs"
    );
    assert_eq!(
        realm_rx.try_recv().expect("threshold crossing packet"),
        wow_packet::packets::misc::CrossedInebriationThreshold {
            guid: player_guid,
            threshold: 2,
            item_id: 0,
        }
        .to_bytes(),
        "Source clears cast-item provenance before delayed spell effects"
    );
    assert!(outgoing_opcodes(&send_rx).contains(&ServerOpcodes::SpellGo));
}

#[tokio::test]
async fn normal_cast_execution_keeps_immediate_effect_cast_item_provenance() {
    let (mut session, _packet_tx, player_guid, cast_item_guid, reagent_guid, send_rx, realm_rx) =
        setup_inebriate_cast(false, 61_060);
    assert_hit_delay_authority(&session, false);
    assert_eq!(cast_item_count(&session, cast_item_guid), 2);
    assert_eq!(cast_item_charge(&session, cast_item_guid), -1);

    execute_inebriate_cast(&mut session, player_guid, cast_item_guid)
        .await
        .expect("immediate normal cast completes before its cast-item debit");

    assert_eq!(cast_item_count(&session, cast_item_guid), 1);
    assert_eq!(
        reagent_state(&session, reagent_guid),
        Some(wow_constants::ItemUpdateState::Removed),
        "the distinct physical reagent is consumed during pre-GO costs"
    );
    assert_eq!(
        realm_rx.try_recv().expect("threshold crossing packet"),
        wow_packet::packets::misc::CrossedInebriationThreshold {
            guid: player_guid,
            threshold: 2,
            item_id: CAST_ITEM as i32,
        }
        .to_bytes(),
        "the immediate effect reads Source cast-item metadata before TakeCastItem"
    );
    assert!(outgoing_opcodes(&send_rx).contains(&ServerOpcodes::SpellGo));
}

#[test]
fn application_take_cast_item_preserves_only_live_provenance_and_reports_unavailable_items() {
    let (mut session, _packet_tx, _player_guid, cast_item_guid, _reagent_guid, _send_rx, _realm_rx) =
        setup_inebriate_cast(false, 61_070);
    let spell_info = exact_test_spell(&session);
    assert_eq!(cast_item_count(&session, cast_item_guid), 2);
    assert_eq!(cast_item_charge(&session, cast_item_guid), -1);
    let mut expected = cast_item_metadata(cast_item_guid);
    expected.misc = [13, 29];
    expected.cast_flags = 0x24;
    let taken = wow_world_application::take_server_normal_cast_item_like_cpp(
        &mut session,
        &spell_info,
        expected,
    )
    .expect("consuming one item from a surviving stack succeeds");
    let mut expected_after_take = expected;
    expected_after_take.cast_item_guid = None;
    expected_after_take.cast_item_entry = None;
    assert_eq!(taken, expected_after_take);
    assert_eq!(taken.cast_item_level, Some(71));
    assert_eq!(cast_item_count(&session, cast_item_guid), 1);

    let no_item = SpellCastMetadata {
        cast_item_entry: Some(CAST_ITEM),
        cast_item_level: Some(72),
        ..Default::default()
    };
    assert_eq!(
        wow_world_application::take_server_normal_cast_item_like_cpp(
            &mut session,
            &spell_info,
            no_item,
        )
        .expect("known no-item path is a successful no-op"),
        no_item,
    );

    let ignored = SpellCastMetadata {
        trigger_cast_flags_like_cpp: wow_entities::TRIGGERED_IGNORE_CAST_ITEM_LIKE_CPP,
        ..cast_item_metadata(cast_item_guid)
    };
    assert_eq!(
        wow_world_application::take_server_normal_cast_item_like_cpp(
            &mut session,
            &spell_info,
            ignored,
        )
        .expect("IgnoreCastItem is a successful no-op"),
        ignored,
    );
    assert_eq!(cast_item_count(&session, cast_item_guid), 1);

    set_cast_item_effect_charges(&mut session, 1);
    session.update_inventory_item_object_like_cpp(cast_item_guid, |item| {
        item.set_spell_charges(0, 2);
    });
    let non_expendable = cast_item_metadata(cast_item_guid);
    assert_eq!(
        wow_world_application::take_server_normal_cast_item_like_cpp(
            &mut session,
            &spell_info,
            non_expendable,
        )
        .expect("non-expendable charge use succeeds without consuming the item"),
        non_expendable,
    );
    assert_eq!(cast_item_count(&session, cast_item_guid), 1);

    let missing_physical_guid = ObjectGuid::create_item(1, 61_072);
    assert!(
        session
            .with_owned_player_like_cpp(|player| {
                !player
                    .inventory_runtime_like_cpp()
                    .item_objects()
                    .contains_key(&missing_physical_guid)
            })
            .expect("canonical Player remains installed")
    );
    let missing_physical = SpellCastMetadata {
        cast_item_guid: Some(missing_physical_guid),
        ..cast_item_metadata(cast_item_guid)
    };
    assert!(wow_world_application::take_server_normal_cast_item_like_cpp(
        &mut session,
        &spell_info,
        missing_physical,
    )
    .is_err());
    assert_eq!(cast_item_count(&session, cast_item_guid), 1);
    let missing_entry = SpellCastMetadata {
        cast_item_entry: None,
        ..cast_item_metadata(cast_item_guid)
    };
    let count_before_missing_entry = cast_item_count(&session, cast_item_guid);
    assert!(wow_world_application::take_server_normal_cast_item_like_cpp(
        &mut session,
        &spell_info,
        missing_entry,
    )
    .is_err());
    assert_eq!(
        cast_item_count(&session, cast_item_guid),
        count_before_missing_entry
    );
}
