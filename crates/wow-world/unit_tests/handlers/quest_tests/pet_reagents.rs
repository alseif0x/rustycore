// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pet reagent return through the production WorldSession host.

use super::*;
use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_packet::WorldPacket;

const PET_REAGENT_ENTRY: u32 = 847_121;

#[tokio::test]
async fn pet_removal_refunds_temporary_pet_once_and_preserves_stale_guid() {
    let (mut session, send_rx) = make_session();
    let owner = install_pet_reagent_fixture(&mut session, 0);
    const SUMMON: u32 = 847_122;
    let mut spells = crate::selected_spell_fixtures::canonical_spell_store([(SUMMON, 0)]);
    spells.install_effective_cast_costs_like_cpp(
        wow_data::SpellCastCostStoreLikeCpp::from_effective_rows_like_cpp(
            [wow_data::SpellReagentsEntry {
                id: 1,
                spell_id: SUMMON as i32,
                reagent: [PET_REAGENT_ENTRY as i32, 0, 0, 0, 0, 0, 0, 0],
                reagent_count: [1, 0, 0, 0, 0, 0, 0, 0],
            }],
            [],
        ),
    );
    session.set_spell_store(Arc::new(spells));
    session.catalogs.spell_catalogs.serverside_spell_store =
        Some(Arc::new(wow_data::ServersideSpellStoreLikeCpp::default()));
    let stale = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 1, 847_123);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_pet_guid_like_cpp(Some(stale));
            // A type alone is not C++ `InBattleground`; the instance ID is.
            player.set_battleground_type_id_like_cpp(5);
            player.set_battleground_instance_id_like_cpp(0);
            let pets = player.pet_lifecycle_state_mut_like_cpp();
            pets.temporary_unsummoned_pet_number = 42;
            pets.old_pet_spell = SUMMON;
            pets.stable.current_pet_index = Some(0);
        })
        .unwrap();
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    let catalogs = wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
        item_guid_generator: generators.item.as_ref(),
        creature_spawns: &creature_spawns,
        graveyards: &graveyards,
    };

    wow_world_application::remove_pet_not_in_slot_like_cpp(&mut session, catalogs)
        .await
        .unwrap();
    assert_item_push_result(
        &send_rx,
        owner,
        i32::from(wow_entities::INVENTORY_SLOT_ITEM_START),
        1,
    );
    let state = session
        .core
        .with_owned_player_like_cpp(|player| {
            let pets = player.pet_lifecycle_state_like_cpp();
            (
                player.gameplay_state().pet_guid,
                pets.temporary_unsummoned_pet_number,
                pets.stable.current_pet_index,
            )
        })
        .unwrap();
    assert_eq!(state, (Some(stale), 0, None));
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| (
            player
                .battleground_state_like_cpp()
                .battleground_type_id_like_cpp(),
            player.battleground_instance_id_like_cpp(),
        )),
        Some((Some(5), None))
    );
    wow_world_application::remove_pet_not_in_slot_like_cpp(&mut session, catalogs)
        .await
        .unwrap();
    assert_no_equip_error_or_item_push_result(&send_rx);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| {
            player
                .inventory_runtime_like_cpp()
                .item_objects()
                .values()
                .filter(|item| item.object().entry() == PET_REAGENT_ENTRY)
                .map(|item| item.count())
                .sum::<u32>()
        }),
        Some(1)
    );
}

#[tokio::test]
async fn pet_removal_keeps_temporary_marker_in_active_battleground_without_reagent_metadata() {
    let (mut session, send_rx) = make_session();
    let _owner = install_pet_reagent_fixture(&mut session, 0);
    const MISSING_SUMMON: u32 = 847_124;
    let stale = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 1, 847_125);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_pet_guid_like_cpp(Some(stale));
            player.set_battleground_instance_id_like_cpp(31);
            let pets = player.pet_lifecycle_state_mut_like_cpp();
            pets.temporary_unsummoned_pet_number = 42;
            pets.old_pet_spell = MISSING_SUMMON;
            pets.stable.current_pet_index = Some(0);
        })
        .unwrap();
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    let catalogs = wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
        item_guid_generator: generators.item.as_ref(),
        creature_spawns: &creature_spawns,
        graveyards: &graveyards,
    };

    wow_world_application::remove_pet_not_in_slot_like_cpp(&mut session, catalogs)
        .await
        .unwrap();

    assert_no_equip_error_or_item_push_result(&send_rx);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| {
            let pets = player.pet_lifecycle_state_like_cpp();
            (
                player.gameplay_state().pet_guid,
                pets.temporary_unsummoned_pet_number,
                pets.old_pet_spell,
                pets.stable.current_pet_index,
                player.battleground_instance_id_like_cpp(),
            )
        }),
        Some((Some(stale), 42, MISSING_SUMMON, None, Some(31)))
    );
    assert!(
        !session
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
}

#[tokio::test]
async fn pet_removal_missing_summon_catalog_preserves_markers_when_no_battleground_instance() {
    let (mut session, send_rx) = make_session();
    let _owner = install_pet_reagent_fixture(&mut session, 0);
    const MISSING_SUMMON: u32 = 847_126;
    let stale = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 1, 847_127);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_pet_guid_like_cpp(Some(stale));
            player.set_battleground_type_id_like_cpp(5);
            player.set_battleground_instance_id_like_cpp(0);
            let pets = player.pet_lifecycle_state_mut_like_cpp();
            pets.temporary_unsummoned_pet_number = 42;
            pets.old_pet_spell = MISSING_SUMMON;
            pets.stable.current_pet_index = Some(0);
        })
        .unwrap();
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    let catalogs = wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
        item_guid_generator: generators.item.as_ref(),
        creature_spawns: &creature_spawns,
        graveyards: &graveyards,
    };

    assert_eq!(
        wow_world_application::remove_pet_not_in_slot_like_cpp(&mut session, catalogs).await,
        Err("pet summon spell catalog is unavailable")
    );

    assert_no_equip_error_or_item_push_result(&send_rx);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| {
            let pets = player.pet_lifecycle_state_like_cpp();
            (
                player.gameplay_state().pet_guid,
                pets.temporary_unsummoned_pet_number,
                pets.old_pet_spell,
                pets.stable.current_pet_index,
                player.battleground_instance_id_like_cpp(),
            )
        }),
        Some((Some(stale), 42, MISSING_SUMMON, Some(0), None))
    );
    assert!(
        !session
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
}

#[tokio::test]
async fn pet_removal_unavailable_item_metadata_preserves_markers_and_inventory() {
    let (mut session, send_rx) = make_session();
    install_pet_reagent_fixture(&mut session, 0);
    const SUMMON: u32 = 847_128;
    let mut spells = crate::selected_spell_fixtures::canonical_spell_store([(SUMMON, 0)]);
    spells.install_effective_cast_costs_like_cpp(
        wow_data::SpellCastCostStoreLikeCpp::from_effective_rows_like_cpp(
            [wow_data::SpellReagentsEntry {
                id: 2,
                spell_id: SUMMON as i32,
                reagent: [PET_REAGENT_ENTRY as i32, 0, 0, 0, 0, 0, 0, 0],
                reagent_count: [1, 0, 0, 0, 0, 0, 0, 0],
            }],
            [],
        ),
    );
    session.set_spell_store(Arc::new(spells));
    session.catalogs.spell_catalogs.serverside_spell_store =
        Some(Arc::new(wow_data::ServersideSpellStoreLikeCpp::default()));
    session.catalogs.items.effect_store = None;

    let stale = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 1, 847_129);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_pet_guid_like_cpp(Some(stale));
            let pets = player.pet_lifecycle_state_mut_like_cpp();
            pets.temporary_unsummoned_pet_number = 42;
            pets.old_pet_spell = SUMMON;
            pets.stable.current_pet_index = Some(0);
        })
        .unwrap();
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    let catalogs = wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
        item_guid_generator: generators.item.as_ref(),
        creature_spawns: &creature_spawns,
        graveyards: &graveyards,
    };

    assert_eq!(
        wow_world_application::remove_pet_not_in_slot_like_cpp(&mut session, catalogs).await,
        Err("pet reagent ItemEffect metadata is unavailable")
    );

    assert_no_equip_error_or_item_push_result(&send_rx);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| {
            let pets = player.pet_lifecycle_state_like_cpp();
            (
                player.gameplay_state().pet_guid,
                pets.temporary_unsummoned_pet_number,
                pets.old_pet_spell,
                pets.stable.current_pet_index,
                player.inventory_runtime_like_cpp().inventory_items().len(),
                player.inventory_runtime_like_cpp().item_objects().len(),
            )
        }),
        Some((Some(stale), 42, SUMMON, Some(0), 0, 0))
    );
    assert!(
        !session
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
}

#[tokio::test]
async fn pet_reagent_return_creates_then_merges_canonical_item_and_pushes_each_time() {
    let (mut session, send_rx) = make_session();
    let owner = install_pet_reagent_fixture(&mut session, 0);

    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let mut fence = disarmed_pet_reagent_fence(&session);
    let first_guid = match wow_world_application::return_pet_reagent_like_cpp(
        &mut session,
        &mut fence,
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
        &mut fence,
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
    fence.disarm_like_cpp();
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
    let mut fence = disarmed_pet_reagent_fence(&session);

    let stored_guid = match wow_world_application::return_pet_reagent_like_cpp(
        &mut session,
        &mut fence,
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
        &mut fence,
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
    fence.disarm_like_cpp();
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

fn disarmed_pet_reagent_fence(
    session: &WorldSession,
) -> crate::session::PlayerMoneyCommitCancellationFenceLikeCpp {
    crate::session::PlayerMoneyCommitCancellationFenceLikeCpp::new_disarmed_like_cpp(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .clone(),
    )
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
