use super::*;
use wow_constants::ItemUpdateState;
use wow_core::ObjectGuid;

fn item(id: i64, owner: ObjectGuid, state: ItemUpdateState) -> Item {
    let mut item = Item::default();
    item.object_mut().create(ObjectGuid::create_item(1, id));
    item.set_owner_guid(owner);
    item.set_max_durability(60);
    item.set_durability(34);
    item.set_count(7);
    item.force_state(state);
    item
}

#[test]
fn partial_durability_requires_existing_changed_owned_identity_without_full_authority() {
    let owner = ObjectGuid::create_player(1, 42);
    let mut player = Player::new(Some(42), false);
    player.unit_mut().world_mut().object_mut().create(owner);
    let runtime = player.inventory_runtime_mut_like_cpp();
    for (id, state) in [
        (30, ItemUpdateState::Changed),
        (20, ItemUpdateState::New),
        (40, ItemUpdateState::Removed),
        (50, ItemUpdateState::Unchanged),
    ] {
        let mut item = item(id, owner, state);
        if state == ItemUpdateState::New {
            item.set_state(ItemUpdateState::Changed);
            assert_eq!(item.update_state(), ItemUpdateState::New);
        }
        runtime.store_item_object_like_cpp(item);
    }
    runtime.store_item_object_like_cpp(item(
        60,
        ObjectGuid::create_player(1, 99),
        ItemUpdateState::Changed,
    ));
    let mut traded = item(70, owner, ItemUpdateState::Changed);
    traded.set_in_trade(true);
    runtime.store_item_object_like_cpp(traded);
    let buyback = item(80, owner, ItemUpdateState::Changed);
    let buyback_guid = buyback.object().guid();
    runtime.store_item_object_like_cpp(buyback);
    runtime.store_buyback_item_in_slot_like_cpp(
        94,
        wow_entities::PlayerInventoryItem {
            guid: buyback_guid,
            entry_id: 2121,
            db_guid: 80,
            inventory_type: None,
        },
    );
    runtime.item_objects_mut().insert(
        ObjectGuid::create_item(1, 90),
        item(91, owner, ItemUpdateState::Changed),
    );
    let mut empty = item(92, owner, ItemUpdateState::Changed);
    empty.object_mut().create(ObjectGuid::EMPTY);
    runtime.store_item_object_like_cpp(empty);
    runtime.store_item_object_like_cpp(item(0, owner, ItemUpdateState::Changed));
    let mut wrong_type = item(93, owner, ItemUpdateState::Changed);
    wrong_type
        .object_mut()
        .create(ObjectGuid::create_player(1, 93));
    runtime.store_item_object_like_cpp(wrong_type);
    let mut nondurable = item(94, owner, ItemUpdateState::Changed);
    nondurable.set_max_durability(0);
    runtime.store_item_object_like_cpp(nondurable);
    assert!(!runtime.equipment_inventory_authority_complete_like_cpp());
    assert_eq!(
        capture_durability(&player),
        Some(vec![PlayerItemDurabilitySave {
            item_guid: 30,
            durability: 34,
        }])
    );
    let current =
        &player.inventory_runtime_like_cpp().item_objects()[&ObjectGuid::create_item(1, 30)];
    assert_eq!(current.update_state(), ItemUpdateState::Changed);
    assert_eq!(current.count(), 7);
    player
        .inventory_runtime_mut_like_cpp()
        .remove_item_object_like_cpp(ObjectGuid::create_item(1, 30));
    assert_eq!(capture_durability(&player), None);
    player
        .inventory_runtime_mut_like_cpp()
        .set_equipment_inventory_authority_complete_like_cpp(true);
    assert_eq!(capture_durability(&player), Some(Vec::new()));
}

#[test]
fn resurrection_partial_inventory_receipt_requires_known_no_pickup_contents_and_exact_durability() {
    use wow_entities::{
        PlayerResurrectionContinuation, PlayerResurrectionOperation as Operation,
        PlayerResurrectionPhase as Phase,
    };
    let owner = ObjectGuid::create_player(1, 42);
    let guid = ObjectGuid::create_item(1, 30);
    let mut player = Player::new(Some(42), false);
    player.unit_mut().world_mut().object_mut().create(owner);
    player
        .inventory_runtime_mut_like_cpp()
        .set_carried_inventory_loaded_like_cpp(true);
    let mut changed = item(30, owner, ItemUpdateState::Changed);
    changed.object_mut().set_entry(2121);
    player
        .inventory_runtime_mut_like_cpp()
        .store_item_object_like_cpp(changed);
    player
        .resurrection_state_mut_like_cpp()
        .delayed_after_teleport = Some(PlayerResurrectionContinuation {
        operation: Operation::ReclaimCorpse {
            restore_percent: 0.5,
        },
        phase: Phase::SavingPlayer,
    });
    let effects = wow_data::ItemEffectStore::from_entries([]);
    let rows = capture_durability(&player).unwrap();
    assert!(resurrection_inventory_covered(
        &player,
        None,
        Some(&rows),
        Some(&effects)
    ));
    assert!(
        !player
            .inventory_runtime_like_cpp()
            .equipment_inventory_authority_complete_like_cpp()
    );
    assert!(!resurrection_inventory_covered(
        &player,
        Some(&[]),
        Some(&rows),
        Some(&effects)
    ));
    assert!(!resurrection_inventory_covered(
        &player,
        None,
        Some(&rows),
        None
    ));
    assert!(!resurrection_inventory_covered(
        &player,
        None,
        None,
        Some(&effects)
    ));
    assert!(!resurrection_inventory_covered(
        &player,
        None,
        Some(&[PlayerItemDurabilitySave {
            item_guid: 30,
            durability: 40
        }]),
        Some(&effects)
    ));
    player
        .inventory_runtime_mut_like_cpp()
        .set_carried_inventory_loaded_like_cpp(false);
    assert!(!resurrection_inventory_covered(
        &player,
        None,
        Some(&rows),
        Some(&effects)
    ));
    player
        .inventory_runtime_mut_like_cpp()
        .set_carried_inventory_loaded_like_cpp(true);
    let pickup = wow_data::ItemEffectStore::from_entries([wow_data::ItemEffectEntry {
        id: 1,
        legacy_slot_index: 0,
        trigger_type: wow_constants::ItemSpelltriggerType::OnPickup as i8,
        charges: 0,
        cooldown_msec: 0,
        category_cooldown_msec: 0,
        spell_category_id: 0,
        spell_id: 123,
        chr_specialization_id: 0,
        parent_item_id: 2121,
    }]);
    assert!(!resurrection_inventory_covered(
        &player,
        None,
        Some(&rows),
        Some(&pickup)
    ));
    player
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .get_mut(&guid)
        .unwrap()
        .set_max_durability(0);
    assert!(!resurrection_inventory_covered(
        &player,
        None,
        Some(&rows),
        Some(&effects)
    ));
    player
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .get_mut(&guid)
        .unwrap()
        .set_max_durability(60);
    for state in [ItemUpdateState::New, ItemUpdateState::Removed] {
        player
            .inventory_runtime_mut_like_cpp()
            .item_objects_mut()
            .get_mut(&guid)
            .unwrap()
            .force_state(state);
        assert!(!resurrection_inventory_covered(
            &player,
            None,
            Some(&rows),
            Some(&effects)
        ));
    }
    player
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .get_mut(&guid)
        .unwrap()
        .force_state(ItemUpdateState::Changed);
    player
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .get_mut(&guid)
        .unwrap()
        .set_in_trade(true);
    assert!(!resurrection_inventory_covered(
        &player,
        None,
        Some(&rows),
        Some(&effects)
    ));
    player
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .get_mut(&guid)
        .unwrap()
        .set_in_trade(false);
    player
        .inventory_runtime_mut_like_cpp()
        .store_buyback_item_in_slot_like_cpp(
            94,
            wow_entities::PlayerInventoryItem {
                guid,
                entry_id: 2121,
                db_guid: 30,
                inventory_type: None,
            },
        );
    assert!(!resurrection_inventory_covered(
        &player,
        None,
        Some(&rows),
        Some(&effects)
    ));
    player
        .inventory_runtime_mut_like_cpp()
        .buyback_items_mut()
        .clear();
    player
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .get_mut(&guid)
        .unwrap()
        .object_mut()
        .create(ObjectGuid::create_item(1, 31));
    assert!(!resurrection_inventory_covered(
        &player,
        None,
        Some(&rows),
        Some(&effects)
    ));
    player
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .get_mut(&guid)
        .unwrap()
        .object_mut()
        .create(guid);
    for (operation, phase, expected) in [
        (
            Operation::RequestData(wow_entities::PlayerResurrectionRequestLikeCpp {
                resurrecter: ObjectGuid::create_player(1, 77),
                map_id: 571,
                position: wow_core::Position::ZERO,
                health: 100,
                mana: 0,
                aura: 0,
            }),
            Phase::SavingPlayer,
            true,
        ),
        (
            Operation::RequestData(wow_entities::PlayerResurrectionRequestLikeCpp {
                resurrecter: ObjectGuid::create_player(1, 77),
                map_id: 571,
                position: wow_core::Position::ZERO,
                health: 100,
                mana: 0,
                aura: 1234,
            }),
            Phase::SavingPlayer,
            false,
        ),
        (
            Operation::SpiritHealer {
                corpse_graveyard: None,
            },
            Phase::SavingPlayer,
            true,
        ),
        (
            Operation::SelfRes {
                health: 35,
                mana: 0,
            },
            Phase::SavingPlayer,
            false,
        ),
        (
            Operation::ReclaimCorpse {
                restore_percent: 0.5,
            },
            Phase::ReadyToSavePlayer,
            false,
        ),
    ] {
        player
            .resurrection_state_mut_like_cpp()
            .delayed_after_teleport = Some(PlayerResurrectionContinuation { operation, phase });
        assert_eq!(
            resurrection_inventory_covered(&player, None, Some(&rows), Some(&effects)),
            expected
        );
    }
    let changed = &player.inventory_runtime_like_cpp().item_objects()[&guid];
    assert_eq!(changed.update_state(), ItemUpdateState::Changed);
    assert_eq!(changed.count(), 7);
    player
        .resurrection_state_mut_like_cpp()
        .delayed_after_teleport = Some(PlayerResurrectionContinuation {
        operation: Operation::ReclaimCorpse {
            restore_percent: 0.5,
        },
        phase: Phase::SavingPlayer,
    });
    player.set_inventory_slot_count(1);
    player
        .store_top_level_item(wow_entities::INVENTORY_SLOT_ITEM_START, guid)
        .unwrap();
    assert!(resurrection_inventory_covered(
        &player,
        None,
        Some(&rows),
        Some(&effects)
    ));
    player
        .inventory_runtime_mut_like_cpp()
        .remove_item_object_like_cpp(guid);
    assert!(
        player
            .inventory_runtime_like_cpp()
            .carried_inventory_loaded_like_cpp()
    );
    assert_eq!(
        player.get_item_by_pos(
            wow_entities::INVENTORY_SLOT_BAG_0,
            wow_entities::INVENTORY_SLOT_ITEM_START
        ),
        Some(guid)
    );
    assert!(
        !resurrection_inventory_covered(&player, None, Some(&rows), Some(&effects)),
        "a dangling native position cannot be authorized by an empty object map"
    );
}
