// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pet reagent return through the production WorldSession host.

use super::*;
use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_packet::WorldPacket;

const PET_REAGENT_ENTRY: u32 = 847_121;

#[tokio::test]
async fn pet_reagent_return_creates_then_merges_canonical_item_and_pushes_each_time() {
    let (mut session, send_rx) = make_session();
    let owner = install_pet_reagent_fixture(&mut session, 0);

    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let first_guid = match wow_world_application::return_pet_reagent_like_cpp(
        &mut session,
        generators.item.as_ref(),
        &creature_spawns,
        PET_REAGENT_ENTRY,
        1,
    )
    .await
    .expect("first reagent return")
    {
        wow_world_application::PetReagentReturnOutcome::Stored(guid) => guid,
        outcome => panic!("expected a new reagent item, got {outcome:?}"),
    };

    assert_item_push_result(
        &send_rx,
        owner,
        i32::from(wow_entities::INVENTORY_SLOT_ITEM_START),
        1,
    );
    assert!(
        quest_status_authority_complete(&session),
        "a no-match quest preflight must preserve authority for a later return"
    );

    let second_guid = match wow_world_application::return_pet_reagent_like_cpp(
        &mut session,
        generators.item.as_ref(),
        &creature_spawns,
        PET_REAGENT_ENTRY,
        1,
    )
    .await
    .expect("second reagent return")
    {
        wow_world_application::PetReagentReturnOutcome::Stored(guid) => guid,
        outcome => panic!("expected the existing reagent stack, got {outcome:?}"),
    };
    assert_eq!(second_guid, first_guid);
    assert_item_push_result(&send_rx, owner, -1, 2);
    assert!(quest_status_authority_complete(&session));

    let slot = wow_entities::INVENTORY_SLOT_ITEM_START;
    let (inventory_row, item, active_inventory_guid, entry_rows, entry_objects) = session
        .core
        .with_owned_player_like_cpp(|player| {
            let runtime = player.inventory_runtime_like_cpp();
            let row = runtime
                .inventory_items()
                .get(&slot)
                .map(|item| (item.guid, item.entry_id));
            let item = runtime.item_objects().get(&first_guid).map(|item| {
                (
                    item.object().guid(),
                    item.object().entry(),
                    item.count(),
                    item.slot(),
                    item.update_state(),
                )
            });
            let entry_rows = runtime
                .inventory_items()
                .values()
                .filter(|item| item.entry_id == PET_REAGENT_ENTRY)
                .count();
            let entry_objects = runtime
                .item_objects()
                .values()
                .filter(|item| item.object().entry() == PET_REAGENT_ENTRY)
                .count();
            (
                row,
                item,
                player.active_data().inv_slots[usize::from(slot)],
                entry_rows,
                entry_objects,
            )
        })
        .expect("canonical Player inventory");

    assert_eq!(inventory_row, Some((first_guid, PET_REAGENT_ENTRY)));
    assert_eq!(active_inventory_guid, first_guid);
    assert_eq!(
        entry_rows, 1,
        "merge must not create a second inventory row"
    );
    assert_eq!(
        entry_objects, 1,
        "merge must not create a second native Item"
    );
    assert_eq!(
        item,
        Some((
            first_guid,
            PET_REAGENT_ENTRY,
            2,
            slot,
            wow_constants::ItemUpdateState::New,
        )),
        "the canonical Player retains one dirty stack in its top-level slot"
    );
}

#[tokio::test]
async fn pet_reagent_inventory_rejection_preserves_stack_and_sends_no_error_or_push() {
    let (mut session, send_rx) = make_session();
    let owner = install_pet_reagent_fixture(&mut session, 1);
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();

    let stored_guid = match wow_world_application::return_pet_reagent_like_cpp(
        &mut session,
        generators.item.as_ref(),
        &creature_spawns,
        PET_REAGENT_ENTRY,
        1,
    )
    .await
    .expect("initial reagent return")
    {
        wow_world_application::PetReagentReturnOutcome::Stored(guid) => guid,
        outcome => panic!("expected stored reagent item, got {outcome:?}"),
    };
    assert_item_push_result(
        &send_rx,
        owner,
        i32::from(wow_entities::INVENTORY_SLOT_ITEM_START),
        1,
    );

    let outcome = wow_world_application::return_pet_reagent_like_cpp(
        &mut session,
        generators.item.as_ref(),
        &creature_spawns,
        PET_REAGENT_ENTRY,
        1,
    )
    .await
    .expect("inventory admission rejection");
    assert_eq!(
        outcome,
        wow_world_application::PetReagentReturnOutcome::InventoryRejected(
            wow_constants::InventoryResult::ItemMaxCount
        )
    );
    assert_no_equip_error_or_item_push_result(&send_rx);

    let (inventory_row, item, active_inventory_guid, entry_rows, entry_objects) = session
        .core
        .with_owned_player_like_cpp(|player| {
            let runtime = player.inventory_runtime_like_cpp();
            let row = runtime
                .inventory_items()
                .get(&wow_entities::INVENTORY_SLOT_ITEM_START)
                .map(|item| (item.guid, item.entry_id));
            let item = runtime.item_objects().get(&stored_guid).map(|item| {
                (
                    item.object().guid(),
                    item.object().entry(),
                    item.count(),
                    item.slot(),
                    item.update_state(),
                )
            });
            let entry_rows = runtime
                .inventory_items()
                .values()
                .filter(|item| item.entry_id == PET_REAGENT_ENTRY)
                .count();
            let entry_objects = runtime
                .item_objects()
                .values()
                .filter(|item| item.object().entry() == PET_REAGENT_ENTRY)
                .count();
            (
                row,
                item,
                player.active_data().inv_slots
                    [usize::from(wow_entities::INVENTORY_SLOT_ITEM_START)],
                entry_rows,
                entry_objects,
            )
        })
        .expect("canonical Player inventory");

    assert_eq!(
        inventory_row,
        Some((stored_guid, PET_REAGENT_ENTRY)),
        "rejected return leaves the existing top-level inventory row intact"
    );
    assert_eq!(active_inventory_guid, stored_guid);
    assert_eq!(entry_rows, 1);
    assert_eq!(entry_objects, 1);
    assert_eq!(
        item,
        Some((
            stored_guid,
            PET_REAGENT_ENTRY,
            1,
            wow_entities::INVENTORY_SLOT_ITEM_START,
            wow_constants::ItemUpdateState::New,
        )),
        "rejected return leaves the canonical dirty Item unchanged"
    );
}

fn install_pet_reagent_fixture(session: &mut WorldSession, max_count: u32) -> ObjectGuid {
    let owner = session.player_guid().expect("test Player identity");
    install_source_item_template(session, PET_REAGENT_ENTRY, 20, max_count);
    let sparse = session
        .catalogs
        .items
        .stats_store
        .as_ref()
        .and_then(|store| store.sparse_template(PET_REAGENT_ENTRY))
        .copied()
        .expect("source-item fixture sparse template");
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_templates([(PET_REAGENT_ENTRY, sparse)])
            .with_duration_in_inventory([(PET_REAGENT_ENTRY, 0)]),
    ));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp(
        std::iter::empty::<QuestTemplate>(),
    )));
    session.set_item_effect_store(Arc::new(wow_data::ItemEffectStore::from_entries([])));
    session.cache_item_template_addon_quest_log_item_id_like_cpp(PET_REAGENT_ENTRY, 0);
    install_native_quest_log_player_like_cpp(session);
    assert!(session.set_player_inventory_slot_count_like_cpp(wow_entities::INVENTORY_DEFAULT_SIZE));
    session.complete_player_equipment_inventory_authority_load_like_cpp();
    session.complete_player_quest_status_authority_load_like_cpp();
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            if !player.unit().world().object().is_in_world() {
                player.unit_mut().world_mut().object_mut().add_to_world();
            }
        })
        .expect("native quest Player owner");

    let inventory_authority_complete = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .inventory_runtime_like_cpp()
                .equipment_inventory_authority_complete_like_cpp()
        })
        .expect("native inventory authority");
    assert!(inventory_authority_complete);
    assert!(quest_status_authority_complete(session));
    owner
}

fn quest_status_authority_complete(session: &WorldSession) -> bool {
    wow_world_quest::QuestSessionHostLikeCpp::player_quest_gameplay_snapshot_like_cpp(session)
        .is_some_and(|state| state.status_authority_complete_like_cpp())
}

fn assert_item_push_result(
    send_rx: &flume::Receiver<Vec<u8>>,
    owner: ObjectGuid,
    slot_in_bag: i32,
    quantity_in_inventory: i32,
) {
    let packets = std::iter::from_fn(|| send_rx.try_recv().ok()).collect::<Vec<_>>();
    let mut pushes = packets
        .into_iter()
        .filter(|bytes| {
            bytes.len() >= 2
                && u16::from_le_bytes([bytes[0], bytes[1]])
                    == wow_constants::ServerOpcodes::ItemPushResult as u16
        })
        .collect::<Vec<_>>();
    assert_eq!(pushes.len(), 1, "one ItemPushResult per reagent return");

    let mut packet = WorldPacket::from_bytes(&pushes.remove(0));
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::ItemPushResult as u16
    );
    assert_eq!(packet.read_packed_guid().unwrap(), owner);
    assert_eq!(
        packet.read_uint8().unwrap(),
        u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
    );
    assert_eq!(packet.read_int32().unwrap(), slot_in_bag);
    assert_eq!(packet.read_int32().unwrap(), 0, "quest log item id");
    assert_eq!(packet.read_int32().unwrap(), 1, "returned quantity");
    assert_eq!(packet.read_int32().unwrap(), quantity_in_inventory);
}

fn assert_no_equip_error_or_item_push_result(send_rx: &flume::Receiver<Vec<u8>>) {
    let unexpected = std::iter::from_fn(|| send_rx.try_recv().ok())
        .filter_map(|bytes| (bytes.len() >= 2).then(|| u16::from_le_bytes([bytes[0], bytes[1]])))
        .filter(|opcode| {
            *opcode == wow_constants::ServerOpcodes::InventoryChangeFailure as u16
                || *opcode == wow_constants::ServerOpcodes::ItemPushResult as u16
        })
        .collect::<Vec<_>>();
    assert!(
        unexpected.is_empty(),
        "silent inventory rejection emitted EquipError/ItemPushResult: {unexpected:?}"
    );
}
