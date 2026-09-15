//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn equipment_stats_use_one_canonical_contribution_path_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_001);
    let item_guid = ObjectGuid::create_item(1, 61_001);
    let item_id = 61_001;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 1_000,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 5;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            item_id,
            ItemStatEntry {
                stats: std::array::from_fn(|index| {
                    if index == 0 {
                        (wow_constants::ItemModType::Strength as i8, 10)
                    } else {
                        (wow_constants::ItemModType::None as i8, 0)
                    }
                }),
                resistances: [0; 7],
                armor: 0,
            },
        )],
        [],
    )));
    let item = session.make_inventory_item_object(
        item_guid,
        item_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::EQUIPMENT_SLOT_CHEST,
    );
    session.insert_inventory_item_object(item);
    session.insert_inventory_item_like_cpp(
        wow_entities::EQUIPMENT_SLOT_CHEST,
        InventoryItem {
            guid: item_guid,
            entry_id: item_id,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );
    // A post-login equip applies `_ApplyItemBonuses` to the Player owner once.
    assert!(
        session
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some()
    );
    let changed = session.apply_inventory_item_store_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_CHEST,
        item_guid,
    );
    assert!(changed);
    assert_eq!(
        (
            session.player_race_like_cpp(),
            session.player_class_like_cpp(),
            session.player_level_like_cpp()
        ),
        (1, 5, 80)
    );
    assert!(session.player_stats().is_some());
    let _ = session.send_stat_update();
    let equipped = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("equipped stat projection");
    assert_eq!(
        equipped.stats[wow_constants::Stats::Strength as usize],
        20,
        "C++ Player::_ApplyItemBonuses contributes the item exactly once"
    );

    // C++ `Player::UpdateExpertise` derives the active-player expertise
    // fields from combat-rating expertise. The value must be published on the
    // same Player snapshot as the rest of the equipment projection.
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::RatingModifier {
            rating: wow_entities::ApplyEnchantmentCombatRating::Expertise,
            amount: 46,
            apply: true,
        }
    ));
    let _ = session.send_stat_update();
    let equipped = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("equipped expertise projection");
    assert_eq!(equipped.mainhand_expertise, 46.0);
    assert_eq!(equipped.offhand_expertise, 46.0);
    assert_eq!(equipped.combat_rating_expertise, 46.0);

    session.apply_inventory_item_remove_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_CHEST,
        item_guid,
        &[],
    );
    let _ = session.send_stat_update();
    let unequipped = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("unequipped stat projection");
    assert_eq!(
        unequipped.stats[wow_constants::Stats::Strength as usize],
        10
    );
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::RatingModifier {
            rating: wow_entities::ApplyEnchantmentCombatRating::Expertise,
            amount: 46,
            apply: false,
        }
    ));

    // `_ApplyAllItemMods` must reject a broken item before it reaches the
    // canonical accumulator, exactly as the C++ `Item::IsBroken` gate does.
    assert!(
        session.update_inventory_item_object_like_cpp(item_guid, |item| {
            item.set_max_durability(10);
            item.set_durability(0);
        })
    );
    let _ = session.apply_initial_loaded_item_mods_like_cpp(&[item_guid]);
    let _ = session.send_stat_update();
    let broken = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("broken-item stat projection");
    assert_eq!(
        broken.stats[wow_constants::Stats::Strength as usize],
        10,
        "a broken item contributes no static stats during login"
    );

    // A repaired item then follows the same login path and produces the same
    // projection as equipping it after login.
    assert!(
        session.update_inventory_item_object_like_cpp(item_guid, |item| {
            item.set_durability(10);
        })
    );
    let _ = session.apply_initial_loaded_item_mods_like_cpp(&[item_guid]);
    let _ = session.send_stat_update();
    let loaded = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("loaded stat projection");
    assert_eq!(
        loaded.stats[wow_constants::Stats::Strength as usize],
        equipped.stats[wow_constants::Stats::Strength as usize]
    );

    // A broken equipped item removes its contribution; the durability repair
    // path reapplies it and publishes the same complete projection.
    session.record_represented_item_mods_like_cpp(
        item_guid,
        wow_entities::EQUIPMENT_SLOT_CHEST,
        false,
    );
    assert!(
        session.update_inventory_item_object_like_cpp(item_guid, |item| {
            item.set_durability(0);
        })
    );
    let _ = session.send_stat_update();
    assert_eq!(
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("broken stat projection")
            .stats[wow_constants::Stats::Strength as usize],
        10
    );
    assert!(
        session
            .repair_inventory_item_durability_like_cpp(item_guid, false, 1.0, 1.0)
            .await
    );
    assert_eq!(
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("repaired stat projection")
            .stats[wow_constants::Stats::Strength as usize],
        20
    );
}

#[test]
fn send_new_item_plan_direct_routes_item_push_result_to_realm_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(1);
    session.install_realm_send_channel_for_test(realm_tx);
    let plan = send_new_item_plan(SendNewItemDelivery::Direct);
    let expected = crate::session_rules::item_push_result_from_send_new_item_plan(&plan).to_bytes();

    session.send_new_item_plan(&plan);

    assert_eq!(realm_rx.try_recv().unwrap(), expected);
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn send_item_time_update_plan_sends_cpp_packet() {
    let (session, _, send_rx) = make_session();
    let update = PlayerItemTimeUpdate {
        item_guid: ObjectGuid::new(0, 0x0102),
        expiration: 300,
    };
    let expected = ItemTimeUpdate {
        item_guid: update.item_guid,
        duration_left: update.expiration,
    }
    .to_bytes();

    session.send_item_time_update_plan(&update);

    assert_eq!(send_rx.try_recv().unwrap(), expected);
}
#[test]
fn loaded_inventory_registers_item_and_non_equipped_enchantment_durations_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 122_126);
    let equipped_guid = ObjectGuid::create_item(1, 122_127);
    let backpack_guid = ObjectGuid::create_item(1, 122_128);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );

    let mut equipped = session.make_inventory_item_object(
        equipped_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    equipped.set_expiration(300);
    equipped.set_enchantment(EnchantmentSlot::EnhancementTemporary, 900, 12_000, 0);
    session.insert_inventory_item_object(equipped);

    let mut backpack = session.make_inventory_item_object(
        backpack_guid,
        701,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    backpack.set_expiration(600);
    backpack.set_enchantment(EnchantmentSlot::EnhancementTemporary, 901, 9_000, 0);
    session.insert_inventory_item_object(backpack);

    let (item_updates, enchantment_updates) = session
        .register_loaded_inventory_item_duration_refs_like_cpp(
            &[equipped_guid, backpack_guid],
            &[equipped_guid],
        );

    assert_eq!(
        item_updates,
        vec![
            PlayerItemTimeUpdate {
                item_guid: equipped_guid,
                expiration: 300,
            },
            PlayerItemTimeUpdate {
                item_guid: backpack_guid,
                expiration: 600,
            },
        ]
    );
    assert_eq!(
        enchantment_updates,
        vec![PlayerEnchantTimeUpdate {
            item_guid: backpack_guid,
            slot: EnchantmentSlot::EnhancementTemporary,
            duration_secs: 9,
        }]
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player.item_durations().to_vec()),
        Some(vec![equipped_guid, backpack_guid])
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player.enchant_durations().to_vec()),
        Some(vec![PlayerEnchantDuration {
            item_guid: backpack_guid,
            slot: EnchantmentSlot::EnhancementTemporary,
            left_duration_ms: 9_000,
        }])
    );
    assert!(
        send_rx.try_recv().is_err(),
        "login duration packets are delayed until after the CREATE_OBJECT sequence"
    );
}
#[test]
fn send_item_enchant_time_update_plan_sends_cpp_packet() {
    let (session, _, send_rx) = make_session();
    let owner_guid = ObjectGuid::new(0, 0x0102);
    let update = PlayerEnchantTimeUpdate {
        item_guid: ObjectGuid::new(0, 0x0506),
        slot: EnchantmentSlot::EnhancementSocket,
        duration_secs: 45,
    };
    let expected = ItemEnchantTimeUpdate {
        owner_guid,
        item_guid: update.item_guid,
        duration_left: update.duration_secs,
        slot: update.slot as u32,
    }
    .to_bytes();

    session.send_item_enchant_time_update_plan(owner_guid, &update);

    assert_eq!(send_rx.try_recv().unwrap(), expected);
}
