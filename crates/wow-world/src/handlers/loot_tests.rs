//! Behaviour tests for [`super`].
//!
//! Extracted from `loot.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.

#![cfg(test)]

use super::{
    CreatureLootReleaseCommandQueueOutcomeLikeCpp, GAMEOBJECT_TYPE_AREADAMAGE,
    GAMEOBJECT_TYPE_BINDER, GAMEOBJECT_TYPE_CHAIR, GAMEOBJECT_TYPE_DOOR,
    GAMEOBJECT_TYPE_GUILD_BANK, GAMEOBJECT_TYPE_QUESTGIVER, INVENTORY_SLOT_BAG_0,
    INVENTORY_SLOT_ITEM_START, ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP,
    ItemTemplateAddonLootMetadataLikeCpp, LOCK_KEY_SKILL_LIKE_CPP, LOCK_KEY_SPELL_LIKE_CPP,
    LOOT_METHOD_GROUP_LIKE_CPP, LOOT_METHOD_MASTER_LIKE_CPP, LOOT_METHOD_ROUND_ROBIN_LIKE_CPP,
    LOOT_MODE_DEFAULT_LIKE_CPP, LOOT_MODE_JUNK_FISH_LIKE_CPP, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP,
    LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP, LootItemClaimCommitContextLikeCpp,
    LootStoreRandomProperties, ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP, ROLL_FLAG_TYPE_NEED_LIKE_CPP,
    ROLL_VOTE_GREED_LIKE_CPP, ROLL_VOTE_NEED_LIKE_CPP, ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP,
    ROLL_VOTE_NOT_VALID_LIKE_CPP, ROLL_VOTE_PASS_LIKE_CPP, RepresentedLootPlayerContext,
    SPELL_EFFECT_OPEN_LOCK_LIKE_CPP, StoredItemMoneyCommitReconciliationLikeCpp,
    StoredItemMoneyDbOutcomeLikeCpp, SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand,
    assign_represented_personal_loot_items_like_cpp,
    classify_stored_item_money_commit_reconciliation_like_cpp,
    creature_loot_is_allowed_to_player_like_cpp, direct_item_count_after_loot_release_like_cpp,
    generated_creature_loot_item_to_entry_like_cpp,
    generated_shared_gameobject_loot_item_to_entry_like_cpp, loot_is_looted_like_cpp,
    loot_item_context, loot_store_data_can_stack_with_item, loot_type_for_client_like_cpp,
    looted_corpse_decay_secs_like_cpp, mark_loot_allowed_for_player_like_cpp,
    mark_loot_item_looted_for_player_like_cpp, prepare_represented_shared_loot_generation_like_cpp,
    queue_creature_loot_release_command_reliably_like_cpp,
    represented_gameobject_display_box_contains_like_cpp,
    represented_gameobject_interaction_distance_like_cpp, represented_loot_object_guid_like_cpp,
    represented_loot_response_items_like_cpp, select_weighted_random_enchantment_like_cpp,
    start_loot_roll_packet_like_cpp, stored_item_money_zero_without_source_outcome_like_cpp,
};
use crate::conditions::QUEST_STATUS_REWARDED_LIKE_CPP;
use crate::session::directory::{
    PlayerBroadcastInfo, PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp,
    PlayerRegistry, PlayerSessionRegistrationLikeCpp,
};
use crate::session::mailbox::{
    ApplyLootMoneyLikeCppCommand, KickLikeCppCommand, LootRollCommandIdentityLikeCpp,
    LootRollVoteCommand, MasterLootGiveResult, SendCreatureSpellCastIfVisibleLikeCppCommand,
    SessionCommand,
};
use crate::session::{
    DurableItemLootCompletionLikeCpp, LootMoneyDeliveryAddressLikeCpp,
    LootMoneyViewerFanoutLikeCpp, RepresentedGameObjectSpellCaster, RepresentedGameObjectUseEffect,
    RepresentedLootRollCriteriaEvent, SessionState, loot_money_durable_outcome_like_cpp,
};
use crate::session_policy::LootDropRatesLikeCpp;
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::time::{Duration, Instant};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Barrier, Mutex, RwLock,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};
use wow_ai::CreatureAI;
use wow_constants::{
    InventoryResult, InventoryType, ItemBondingType, ItemClass, ItemContext, ItemFieldFlags,
    ItemFlags2, ItemQuality, ServerOpcodes, TypeId, TypeMask, UnitDynFlags,
};
use wow_core::{ObjectGuid, ObjectGuidGenerator, Position, guid::HighGuid};
use wow_data::quest::{QUEST_REWARD_REPUTATIONS_COUNT, QuestObjective, QuestStore, QuestTemplate};
use wow_data::{
    AreaTableEntry, AreaTableStore, ChrSpecializationEntry, ChrSpecializationStore,
    ItemDisenchantLootEntry, ItemDisenchantLootStore, ItemRandomEnchantmentTemplateEntry,
    ItemRandomEnchantmentTemplateStore, ItemRandomPropertiesEntry, ItemRandomPropertiesStore,
    ItemRandomPropertyTemplateEntry, ItemRandomSuffixEntry, ItemRandomSuffixStore, ItemRecord,
    ItemSparseTemplateEntry, ItemStatsStore, ItemStore, RandPropPointsEntry, RandPropPointsStore,
    SpellEffectInfo, SpellInfo, SpellMiscEntry, SpellMiscStore, SpellRangeEntry, SpellRangeStore,
    SpellStore,
};
use wow_database::{CharStatements, DatabaseError, SqlTransactionCommitError, StatementDef};
use wow_entities::{
    AccessorObjectKind, CORPSE_DYNFLAG_LOOTABLE, Corpse, CorpseType, Creature, CreatureOwnedLoot,
    GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_FISHING_HOLE, GAMEOBJECT_TYPE_FISHING_NODE,
    GAMEOBJECT_TYPE_GATHERING_NODE, GAMEOBJECT_TYPE_GOOBER, GO_DYNFLAG_LO_NO_INTERACT, GameObject,
    GameObjectLootSource, GameObjectOwnedLoot, GatheringNodeUseSource, GoState, Item,
    ItemCreateInfo, LootState, MAX_ITEM_SPELLS, MAX_MONEY_AMOUNT, ObjectChangedFields, Player,
    WorldObject,
};
use wow_loot::{
    GeneratedLootItem, LOOT_SLOT_TYPE_OWNER_LIKE_CPP, LootClaimPayload, LootConditionRowLikeCpp,
    LootStore, LootStoreItem, LootStoreItemContext, LootStoreKind, LootStores, LootTemplateRow,
    OwnedLootAuthority, OwnedLootAuthorityLifecycle,
};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_ERROR_MASTER_OTHER_LIKE_CPP, LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP,
    LOOT_ERROR_NO_LOOT_LIKE_CPP, LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP, LOOT_ERROR_TOO_FAR_LIKE_CPP,
    LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP, LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
    LOOT_TYPE_CHEST_LIKE_CPP, LOOT_TYPE_CORPSE_LIKE_CPP, LOOT_TYPE_CORPSE_PERSONAL_LIKE_CPP,
    LOOT_TYPE_DISENCHANTING_LIKE_CPP, LOOT_TYPE_FISHING_JUNK_LIKE_CPP, LOOT_TYPE_FISHING_LIKE_CPP,
    LOOT_TYPE_FISHINGHOLE_LIKE_CPP, LOOT_TYPE_GATHERING_NODE_LIKE_CPP, LOOT_TYPE_INSIGNIA_LIKE_CPP,
    LOOT_TYPE_ITEM_LIKE_CPP, LOOT_TYPE_MILLING_LIKE_CPP, LOOT_TYPE_NONE_LIKE_CPP,
    LOOT_TYPE_PICKPOCKETING_LIKE_CPP, LOOT_TYPE_PROSPECTING_LIKE_CPP, LOOT_TYPE_SKINNING_LIKE_CPP,
    LootEntry, LootEntryFlags, LootResponse, LootRoll, MasterLootItem, SetLootSpecialization,
};
use wow_packet::packets::update::{
    CreatureCreateData, ObjectDataValuesUpdate, UnitDataValuesDeltaUpdate,
};
use wow_packet::{ServerPacket, WorldPacket};
use wow_social::group::{GroupInfo, GroupRegistry, PendingInvites};

use crate::session::{
    AuraApplication, InventoryItem, SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP, SpellCastState,
    WorldSession,
};

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_loot_money_persistence_test_result_like_cpp(true);
    (session, send_rx)
}

fn make_session_with_send() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    make_session_with_send_capacity(1)
}

fn make_session() -> WorldSession {
    make_session_with_send().0
}

fn make_visible_creature_spell_session_like_cpp()
-> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let source_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 91_500);
    let manager = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            source_guid,
            777,
            Position::ZERO,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );
    session.set_state(SessionState::LoggedIn);
    session.set_map_manager(manager);
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(source_guid);
    (session, send_rx, source_guid)
}

/// One committed cast. `go_marker` distinguishes the basic frame (`0xBB`)
/// the producer commits for an ordinary receiver from the full combat-log
/// frame (`0xCC`) it commits for an advanced-logging receiver.
fn creature_spell_cast_command_like_cpp(
    source_guid: ObjectGuid,
    committed_visibility_like_cpp: crate::session::mailbox::SharedClientVisibleGuidsLikeCpp,
    go_marker: u8,
) -> SendCreatureSpellCastIfVisibleLikeCppCommand {
    let mut start_packet_bytes = (ServerOpcodes::SpellStart as u16).to_le_bytes().to_vec();
    start_packet_bytes.push(0xAA);
    let mut go_packet_bytes = (ServerOpcodes::SpellGo as u16).to_le_bytes().to_vec();
    go_packet_bytes.push(go_marker);
    SendCreatureSpellCastIfVisibleLikeCppCommand {
        queued_at: Instant::now(),
        source_guid,
        map_id: 571,
        instance_id: 0,
        start_packet_bytes,
        go_packet_bytes,
        committed_visibility_like_cpp,
    }
}

#[tokio::test]
async fn creature_spell_cast_command_sends_start_then_basic_go_after_one_gate_like_cpp() {
    let (mut session, send_rx, source_guid) = make_visible_creature_spell_session_like_cpp();
    let command = creature_spell_cast_command_like_cpp(
        source_guid,
        session.client_visible_guids_like_cpp.clone(),
        0xBB,
    );
    let expected_start = command.start_packet_bytes.clone();
    let expected_go = command.go_packet_bytes.clone();
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(send_rx.try_recv().expect("START frame"), expected_start);
    assert_eq!(send_rx.try_recv().expect("GO frame"), expected_go);
    assert!(send_rx.try_recv().is_err(), "no partial or extra frame");
}

#[tokio::test]
async fn advanced_combat_logging_receives_the_committed_full_creature_spell_go_like_cpp() {
    // The producer committed the full combat-log frame for this receiver.
    let (mut session, send_rx, source_guid) = make_visible_creature_spell_session_like_cpp();
    session.represented_set_advanced_combat_logging_like_cpp(true);
    let command = creature_spell_cast_command_like_cpp(
        source_guid,
        session.client_visible_guids_like_cpp.clone(),
        0xCC,
    );
    let expected_start = command.start_packet_bytes.clone();
    let expected_go = command.go_packet_bytes.clone();
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(send_rx.try_recv().expect("START frame"), expected_start);
    assert_eq!(send_rx.try_recv().expect("full GO frame"), expected_go);
    assert!(send_rx.try_recv().is_err(), "no partial or extra frame");
}

#[tokio::test]
async fn creature_spell_go_keeps_the_committed_frame_after_a_preference_toggle_like_cpp() {
    // C++ chooses the combat-log representation while distributing the cast,
    // so toggling advanced logging before this drain must not retroactively
    // change the frame an earlier cast committed.
    let (mut session, send_rx, source_guid) = make_visible_creature_spell_session_like_cpp();
    let command = creature_spell_cast_command_like_cpp(
        source_guid,
        session.client_visible_guids_like_cpp.clone(),
        0xBB,
    );
    let expected_go = command.go_packet_bytes.clone();
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");
    session.represented_set_advanced_combat_logging_like_cpp(true);

    session
        .process_represented_session_commands_like_cpp()
        .await;

    let _start = send_rx.try_recv().expect("START frame");
    assert_eq!(
        send_rx.try_recv().expect("GO frame"),
        expected_go,
        "the committed basic frame survives a later advanced-logging toggle"
    );
    assert!(send_rx.try_recv().is_err(), "no partial or extra frame");
}

#[tokio::test]
async fn creature_spell_cast_honors_commit_time_visibility_after_exit_like_cpp() {
    // C++ picks recipients synchronously inside `SendSpellGo`, so a
    // visibility exit between that commit and this drain cannot retract a
    // pair the viewer was already selected for.
    let (mut session, send_rx, source_guid) = make_visible_creature_spell_session_like_cpp();
    let command = creature_spell_cast_command_like_cpp(
        source_guid,
        session.client_visible_guids_like_cpp.clone(),
        0xBB,
    );
    let expected_start = command.start_packet_bytes.clone();
    let expected_go = command.go_packet_bytes.clone();
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");
    assert!(
        session.client_visible_guids_like_cpp.remove(&source_guid),
        "the caster leaves the client's visible set before the drain"
    );

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(send_rx.try_recv().expect("START frame"), expected_start);
    assert_eq!(send_rx.try_recv().expect("GO frame"), expected_go);
    assert!(send_rx.try_recv().is_err(), "no partial or extra frame");
}

#[tokio::test]
async fn creature_spell_cast_rejects_command_committed_for_another_session_like_cpp() {
    // A replaced session owns a fresh `HaveAtClient` allocation, so a pair
    // committed against the previous incarnation must not be delivered even
    // when the caster is visible again.
    let (mut session, send_rx, source_guid) = make_visible_creature_spell_session_like_cpp();
    let previous_incarnation = crate::session::mailbox::SharedClientVisibleGuidsLikeCpp::default();
    previous_incarnation.insert(source_guid);
    assert!(
        !session
            .client_visible_guids_like_cpp
            .shares_storage_like_cpp(&previous_incarnation),
        "the fixture models two distinct session incarnations"
    );
    let command = creature_spell_cast_command_like_cpp(source_guid, previous_incarnation, 0xBB);
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "a command committed for another incarnation delivers nothing"
    );
}

#[test]
fn stored_item_money_commit_unknown_requires_joint_balance_and_source_evidence_like_cpp() {
    let outcome = StoredItemMoneyDbOutcomeLikeCpp {
        before: 100,
        after: 107,
        applied_delta: 7,
        notified_amount: 7,
    };
    assert_eq!(
        classify_stored_item_money_commit_reconciliation_like_cpp(outcome, 100, Some(7)),
        StoredItemMoneyCommitReconciliationLikeCpp::RolledBack
    );
    assert_eq!(
        classify_stored_item_money_commit_reconciliation_like_cpp(outcome, 107, None),
        StoredItemMoneyCommitReconciliationLikeCpp::Committed
    );
    assert_eq!(
        classify_stored_item_money_commit_reconciliation_like_cpp(outcome, 100, None),
        StoredItemMoneyCommitReconciliationLikeCpp::Indeterminate,
        "a missing source alone cannot attribute a later consumer's commit to this attempt"
    );
    assert_eq!(
        classify_stored_item_money_commit_reconciliation_like_cpp(outcome, 107, Some(7)),
        StoredItemMoneyCommitReconciliationLikeCpp::Indeterminate
    );
}

#[test]
fn stored_item_money_cap_noop_still_reconciles_source_consumption_like_cpp() {
    let outcome = StoredItemMoneyDbOutcomeLikeCpp {
        before: MAX_MONEY_AMOUNT - 1,
        after: MAX_MONEY_AMOUNT - 1,
        applied_delta: 0,
        notified_amount: 2,
    };
    assert_eq!(
        classify_stored_item_money_commit_reconciliation_like_cpp(
            outcome,
            MAX_MONEY_AMOUNT - 1,
            None,
        ),
        StoredItemMoneyCommitReconciliationLikeCpp::Committed
    );
    assert_eq!(
        classify_stored_item_money_commit_reconciliation_like_cpp(
            outcome,
            MAX_MONEY_AMOUNT - 1,
            Some(2),
        ),
        StoredItemMoneyCommitReconciliationLikeCpp::RolledBack
    );
}

#[test]
fn stored_item_money_zero_without_db_source_is_success_but_positive_is_consumed() {
    let zero = stored_item_money_zero_without_source_outcome_like_cpp(41, 0).unwrap();
    assert_eq!(zero.before, 41);
    assert_eq!(zero.after, 41);
    assert_eq!(zero.applied_delta, 0);
    assert_eq!(zero.notified_amount, 0);
    assert!(stored_item_money_zero_without_source_outcome_like_cpp(41, 1).is_none());
}

#[test]
fn stored_item_money_completion_applies_db_delta_to_divergent_runtime_base_like_cpp() {
    let (db_after, durable_delta) = loot_money_durable_outcome_like_cpp(100, 7);
    let runtime_before = 500;
    let runtime_after = runtime_before + durable_delta;

    assert_eq!(db_after, 107);
    assert_eq!(runtime_after, 507);
    assert_ne!(runtime_after, db_after);
}

#[test]
fn item_instance_guid_allocator_is_shared_across_concurrent_loot_sessions_like_cpp() {
    const WORKERS: usize = 8;
    const GUIDS_PER_WORKER: usize = 128;
    const FIRST_GUID: i64 = 40_000;

    let generator = Arc::new(ObjectGuidGenerator::new(HighGuid::Item, FIRST_GUID));
    let start = Arc::new(Barrier::new(WORKERS));
    let handles = (0..WORKERS)
        .map(|_| {
            let generator = Arc::clone(&generator);
            let start = Arc::clone(&start);
            std::thread::spawn(move || {
                let mut session = make_session();
                session.set_realm_id(7);
                session.set_item_guid_generator_like_cpp(generator);
                start.wait();
                session
                    .allocate_item_instance_guids_like_cpp(GUIDS_PER_WORKER)
                    .expect("shared item allocator must be installed")
            })
        })
        .collect::<Vec<_>>();

    let mut allocated = handles
        .into_iter()
        .flat_map(|handle| handle.join().expect("allocation worker must finish"))
        .collect::<Vec<_>>();
    allocated.sort_unstable_by_key(|(db_guid, _)| *db_guid);

    assert_eq!(allocated.len(), WORKERS * GUIDS_PER_WORKER);
    for (offset, (db_guid, object_guid)) in allocated.iter().enumerate() {
        let expected = FIRST_GUID as u64 + offset as u64;
        assert_eq!(*db_guid, expected);
        assert!(object_guid.is_item());
        assert_eq!(object_guid.counter() as u64, expected);
    }
    assert_eq!(
        generator.next_after_max_used(),
        FIRST_GUID + (WORKERS * GUIDS_PER_WORKER) as i64
    );
}

#[test]
fn item_instance_guid_allocator_fails_closed_and_never_reuses_failed_grant_like_cpp() {
    let mut session = make_session();
    assert_eq!(session.allocate_item_instance_guids_like_cpp(1), None);

    let generator = Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 91_000));
    session.set_item_guid_generator_like_cpp(Arc::clone(&generator));

    // C++ consumes a GUID when Item::CreateItem runs.  A later storage or
    // transaction failure may leave a gap, but must never make that GUID
    // available to a competing durable grant.
    let abandoned_after_persistence_failure = session
        .allocate_item_instance_guids_like_cpp(1)
        .expect("allocator must be installed");
    assert_eq!(abandoned_after_persistence_failure[0].0, 91_000);
    drop(abandoned_after_persistence_failure);

    let next_grant = session
        .allocate_item_instance_guids_like_cpp(1)
        .expect("allocator must remain installed");
    assert_eq!(next_grant[0].0, 91_001);
    assert_eq!(generator.next_after_max_used(), 91_002);
}

fn canonical_world_object(guid: ObjectGuid, map_id: u32, position: Position) -> WorldObject {
    let (type_id, type_mask) = if guid.is_game_object() {
        (TypeId::GameObject, TypeMask::GAME_OBJECT)
    } else {
        (TypeId::Unit, TypeMask::UNIT)
    };
    let mut object = WorldObject::new(false, type_id, type_mask);
    object.object_mut().create(guid);
    object.set_map(map_id, 0).unwrap();
    object.relocate(position);
    object.object_mut().add_to_world();
    object
}

fn attach_canonical_map_object(
    session: &mut WorldSession,
    kind: AccessorObjectKind,
    object: WorldObject,
) {
    let map_id = object.map_id();
    let instance_id = object.instance_id();
    let manager = Arc::new(Mutex::new(wow_map::MapManager::default()));
    {
        let mut manager = manager.lock().unwrap();
        manager
            .create_world_map(map_id, instance_id)
            .map_mut()
            .add_to_map_like_cpp(kind, object)
            .unwrap();
    }
    session.set_canonical_map_manager(manager);
}

fn attach_loot_guid_allocator_for_owner(session: &mut WorldSession, owner_guid: ObjectGuid) {
    let kind = if owner_guid.is_game_object() {
        AccessorObjectKind::GameObject
    } else {
        AccessorObjectKind::Creature
    };
    attach_canonical_map_object(
        session,
        kind,
        canonical_world_object(owner_guid, u32::from(owner_guid.map_id()), Position::ZERO),
    );
}

#[test]
fn map_owned_loot_guid_sequence_is_shared_across_owner_kinds_like_cpp() {
    let mut session = make_session();
    let map_id = 571;
    let realm_id = 7;
    session.set_realm_id(realm_id);
    let shared_owner_counter = 19_700;
    let creature_owner = ObjectGuid::create_world_object(
        HighGuid::Creature,
        0,
        11,
        map_id,
        17,
        101,
        shared_owner_counter,
    );
    let gameobject_owner = ObjectGuid::create_world_object(
        HighGuid::GameObject,
        0,
        12,
        map_id,
        18,
        202,
        shared_owner_counter,
    );
    attach_loot_guid_allocator_for_owner(&mut session, creature_owner);

    let creature_loot = session
        .next_canonical_loot_object_guid_like_cpp(creature_owner)
        .expect("the creature owner map should allocate a LootObject");
    let gameobject_loot = session
        .next_canonical_loot_object_guid_like_cpp(gameobject_owner)
        .expect("the gameobject owner map should share the LootObject sequence");

    assert_ne!(creature_loot, gameobject_loot);
    assert_eq!(creature_loot.counter(), 1);
    assert_eq!(gameobject_loot.counter(), 2);
    for loot_guid in [creature_loot, gameobject_loot] {
        assert_eq!(loot_guid.high_type(), HighGuid::LootObject);
        assert_eq!(loot_guid.sub_type(), 0);
        assert_eq!(loot_guid.realm_id(), realm_id);
        assert_eq!(loot_guid.map_id(), map_id);
        assert_eq!(loot_guid.server_id(), 0);
        assert_eq!(loot_guid.entry(), 0);
    }

    let manager = session.canonical_map_manager.as_ref().unwrap();
    let mut manager = manager.lock().unwrap();
    assert_eq!(
        manager
            .find_map_mut(u32::from(map_id), 0)
            .unwrap()
            .map_mut()
            .get_max_low_guid_like_cpp(HighGuid::LootObject)
            .unwrap(),
        3
    );
}

#[test]
fn loot_guid_allocator_without_canonical_map_fails_without_advancing_like_cpp() {
    let mut session = make_session();
    let owner_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 9, 571, 7, 303, 19_701);

    assert!(
        session
            .next_canonical_loot_object_guid_like_cpp(owner_guid)
            .is_none()
    );

    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);
    let first_allocated = session
        .next_canonical_loot_object_guid_like_cpp(owner_guid)
        .expect("the first allocation after attaching the map should succeed");
    assert_eq!(first_allocated.counter(), 1);
}

#[test]
fn loot_guid_allocator_refuses_different_owner_map_without_advancing_like_cpp() {
    let mut session = make_session();
    let canonical_owner =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 9, 571, 7, 404, 19_702);
    let other_map_owner =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 9, 0, 7, 505, 19_703);
    attach_loot_guid_allocator_for_owner(&mut session, canonical_owner);

    assert!(
        session
            .next_canonical_loot_object_guid_like_cpp(other_map_owner)
            .is_none()
    );

    let first_allocated = session
        .next_canonical_loot_object_guid_like_cpp(canonical_owner)
        .expect("the rejected owner must not consume the canonical map sequence");
    assert_eq!(first_allocated.counter(), 1);
}

#[test]
fn personal_loot_pools_receive_distinct_map_owned_guids_like_cpp() {
    let mut session = make_session();
    session.set_realm_id(7);
    let owner_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 9, 571, 7, 606, 19_704);
    let first_player = ObjectGuid::create_player(1, 42);
    let second_player = ObjectGuid::create_player(1, 77);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);

    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_guid = session
        .next_canonical_loot_object_guid_like_cpp(owner_guid)
        .expect("the base personal pool should receive a map-owned LootObject");
    loot.allowed_looters = vec![second_player, first_player];

    let (shared, personal) = session
        .represented_loot_authority_pools_like_cpp(owner_guid, first_player, loot, true)
        .expect("both personal pools should materialize");

    assert!(shared.is_none());
    assert_eq!(personal.len(), 2);
    let first_pool = personal.get(&first_player).unwrap();
    let second_pool = personal.get(&second_player).unwrap();
    assert_ne!(first_pool.loot_guid, second_pool.loot_guid);
    assert_eq!(first_pool.loot_guid.counter(), 1);
    assert_eq!(second_pool.loot_guid.counter(), 2);
    for (player_guid, pool) in [(first_player, first_pool), (second_player, second_pool)] {
        assert_eq!(pool.loot_guid.high_type(), HighGuid::LootObject);
        assert_eq!(pool.loot_guid.realm_id(), 7);
        assert_eq!(pool.loot_guid.map_id(), 571);
        assert_eq!(pool.loot_guid.server_id(), 0);
        assert_eq!(pool.loot_guid.entry(), 0);
        assert_eq!(pool.allowed_looters, vec![player_guid]);
        assert_eq!(pool.items[0].allowed_looters, vec![player_guid]);
    }
}

fn attach_canonical_gameobject(session: &mut WorldSession, game_object: GameObject) {
    let map_id = game_object.world().map_id();
    let instance_id = game_object.world().instance_id();
    let manager = Arc::new(Mutex::new(wow_map::MapManager::default()));
    {
        let mut manager = manager.lock().unwrap();
        manager
            .create_world_map(map_id, instance_id)
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_game_object(game_object).unwrap(),
            )
            .unwrap();
    }
    session.set_canonical_map_manager(manager);
}

fn attach_canonical_creature(session: &mut WorldSession, creature: Creature) {
    let map_id = creature.unit().world().map_id();
    let instance_id = creature.unit().world().instance_id();
    let manager = Arc::new(Mutex::new(wow_map::MapManager::default()));
    {
        let mut manager = manager.lock().unwrap();
        manager
            .create_world_map(map_id, instance_id)
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_creature(creature).unwrap(),
            )
            .unwrap();
    }
    session.set_canonical_map_manager(manager);
}

fn attach_canonical_corpse(session: &mut WorldSession, corpse: Corpse) {
    let map_id = corpse.world().map_id();
    let instance_id = corpse.world().instance_id();
    let manager = Arc::new(Mutex::new(wow_map::MapManager::default()));
    {
        let mut manager = manager.lock().unwrap();
        manager
            .create_world_map(map_id, instance_id)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_corpse(corpse).unwrap())
            .unwrap();
    }
    session.set_canonical_map_manager(manager);
}

fn make_canonical_corpse_for_session(session: &WorldSession, guid: ObjectGuid) -> Corpse {
    let mut corpse = Corpse::new_at(CorpseType::ResurrectablePvp, 1_000);
    corpse.world_mut().object_mut().create(guid);
    corpse
        .world_mut()
        .set_map(u32::from(session.player_map_id_like_cpp()), 0)
        .unwrap();
    corpse.world_mut().relocate(Position::ZERO);
    corpse.world_mut().object_mut().add_to_world();
    corpse.set_corpse_dynamic_flag(CORPSE_DYNFLAG_LOOTABLE);
    corpse.clear_corpse_data_changes();
    corpse
}

fn canonical_corpse_snapshot(session: &WorldSession, guid: ObjectGuid) -> Option<Corpse> {
    let manager = session.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(u32::from(session.player_map_id_like_cpp()), 0)?;
    map.map().get_typed_corpse(guid).cloned()
}

fn make_canonical_creature_for_session(session: &WorldSession, guid: ObjectGuid) -> Creature {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .set_map(u32::from(session.player_map_id_like_cpp()), 0)
        .unwrap();
    creature.unit_mut().world_mut().relocate(Position::ZERO);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature
}

fn canonical_creature_snapshot(session: &WorldSession, guid: ObjectGuid) -> Option<Creature> {
    let manager = session.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(u32::from(session.player_map_id_like_cpp()), 0)?;
    map.map().get_typed_creature(guid).cloned()
}

fn make_canonical_gameobject_for_session(
    session: &WorldSession,
    guid: ObjectGuid,
    go_type: u8,
) -> GameObject {
    let mut game_object = GameObject::new();
    game_object.world_mut().object_mut().create(guid);
    game_object
        .world_mut()
        .set_map(u32::from(session.player_map_id_like_cpp()), 0)
        .unwrap();
    game_object.world_mut().relocate(Position::ZERO);
    game_object.world_mut().object_mut().add_to_world();
    game_object.set_go_type(go_type);
    game_object
}

fn canonical_gameobject_snapshot(session: &WorldSession, guid: ObjectGuid) -> Option<GameObject> {
    let manager = session.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(u32::from(session.player_map_id_like_cpp()), 0)?;
    map.map().get_typed_game_object(guid).cloned()
}

fn loot_item_packet(object: ObjectGuid, loot_list_id: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(1);
    pkt.write_packed_guid(&object);
    pkt.write_uint8(loot_list_id);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn loot_unit_packet(object: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&object);
    pkt.reset_read();
    pkt
}

fn represented_loot_entry(loot_list_id: u8, item_id: u32, player_guid: ObjectGuid) -> LootEntry {
    LootEntry {
        loot_list_id,
        item_id,
        quantity: 1,
        random_properties_id: 0,
        random_properties_seed: 0,
        item_context: 0,
        flags: LootEntryFlags {
            follow_loot_rules: true,
            freeforall: false,
            blocked: false,
            counted: false,
            under_threshold: false,
            needs_quest: false,
        },
        allowed_looters: vec![player_guid],
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    }
}

fn install_active_item_loot_completion_fixture_like_cpp(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    owner_guid: ObjectGuid,
    coins: u32,
) {
    assert!(owner_guid.is_item());
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(owner_guid);
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: owner_guid,
            coins,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_ITEM_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: vec![represented_loot_entry(0, 25, player_guid)],
            looted_by_player: false,
        },
    );
}

#[test]
fn represented_loot_type_for_client_matches_cpp_aliases() {
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_NONE_LIKE_CPP),
        LOOT_TYPE_NONE_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_CORPSE_LIKE_CPP),
        LOOT_TYPE_CORPSE_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_ITEM_LIKE_CPP),
        LOOT_TYPE_ITEM_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_GATHERING_NODE_LIKE_CPP),
        LOOT_TYPE_GATHERING_NODE_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_CHEST_LIKE_CPP),
        LOOT_TYPE_CHEST_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_CORPSE_PERSONAL_LIKE_CPP),
        LOOT_TYPE_CORPSE_PERSONAL_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_PROSPECTING_LIKE_CPP),
        LOOT_TYPE_DISENCHANTING_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_MILLING_LIKE_CPP),
        LOOT_TYPE_DISENCHANTING_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_INSIGNIA_LIKE_CPP),
        LOOT_TYPE_SKINNING_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_FISHINGHOLE_LIKE_CPP),
        LOOT_TYPE_FISHING_LIKE_CPP
    );
    assert_eq!(
        loot_type_for_client_like_cpp(LOOT_TYPE_FISHING_JUNK_LIKE_CPP),
        LOOT_TYPE_FISHING_LIKE_CPP
    );
}

#[tokio::test]
async fn represented_loot_response_acquire_reason_uses_cpp_loot_type_mapping() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let owner_guid = test_creature_guid(19_096);
    let loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    let entry = represented_loot_entry(0, 25, player_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_PROSPECTING_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![entry],
            looted_by_player: false,
        },
    );
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);

    let response = session
        .represented_loot_response_for_owner_like_cpp(owner_guid, player_guid, false)
        .await
        .unwrap();

    assert_eq!(response.acquire_reason, LOOT_TYPE_DISENCHANTING_LIKE_CPP);
}

#[test]
fn represented_start_loot_roll_carries_cpp_dungeon_encounter_id() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_obj = ObjectGuid::create_world_object(HighGuid::LootObject, 0, 1, 0, 0, 1, 900);
    let entry = represented_loot_entry(0, 25, player_guid);

    let packet = start_loot_roll_packet_like_cpp(
        loot_obj,
        571,
        LOOT_METHOD_GROUP_LIKE_CPP,
        &entry,
        ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP,
        615,
    );

    assert_eq!(packet.dungeon_encounter_id, 615);
}

#[test]
fn represented_loot_item_push_result_uses_realm_route_and_cpp_encounter_fields() {
    let (mut session, instance_rx) = make_session_with_send();
    let (realm_tx, realm_rx) = flume::bounded(1);
    session.install_realm_send_channel_for_test(realm_tx);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 700);
    let entry = represented_loot_entry(0, 25, player_guid);

    session.send_loot_item_push_result(player_guid, item_guid, &entry, 0, 0, 0, 1, 1, false, 615);

    assert!(instance_rx.try_recv().is_err());
    let sent = realm_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::ItemPushResult as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert_eq!(sent.read_uint8().unwrap(), u8::from(INVENTORY_SLOT_BAG_0));
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_int32().unwrap(), 1);
    assert_eq!(sent.read_int32().unwrap(), 1);
    assert_eq!(sent.read_int32().unwrap(), 615);
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_uint32().unwrap(), 0);
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_packed_guid().unwrap(), item_guid);
    assert!(!sent.read_bit().unwrap());
    assert!(!sent.read_bit().unwrap());
    assert_eq!(sent.read_bits(3).unwrap(), 2);
    assert!(!sent.read_bit().unwrap());
    assert!(sent.read_bit().unwrap());
    assert_eq!(sent.read_int32().unwrap(), 25);
}

#[test]
fn creature_generated_loot_entry_uses_item_template_addon_follow_loot_rules_like_cpp() {
    let generated = GeneratedLootItem {
        item_id: 25,
        count: 1,
        loot_list_id: 7,
        random_properties_id: -77,
        random_properties_seed: 456,
        context: ItemContext::DungeonNormal as u8,
        store_item_context: LootStoreItemContext {
            store_kind: LootStoreKind::Creature,
            entry: 100,
            item: LootStoreItem {
                item_id: 25,
                reference: 0,
                chance: 100.0,
                needs_quest: true,
                loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                group_id: 0,
                min_count: 1,
                max_count: 1,
            },
        },
        free_for_all: false,
        follow_loot_rules: false,
        needs_quest: true,
        is_looted: false,
        is_blocked: false,
        is_under_threshold: false,
        is_counted: false,
    };

    let default_entry = generated_creature_loot_item_to_entry_like_cpp(
        generated,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
    );
    assert!(!default_entry.flags.follow_loot_rules);
    assert_eq!(default_entry.loot_list_id, 7);
    assert_eq!(default_entry.random_properties_id, -77);
    assert_eq!(default_entry.random_properties_seed, 456);
    assert_eq!(default_entry.item_context, ItemContext::DungeonNormal as u8);

    let follow_entry = generated_creature_loot_item_to_entry_like_cpp(
        generated,
        ItemTemplateAddonLootMetadataLikeCpp {
            flags_cu: ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP,
            quest_log_item_id: 0,
        },
    );
    assert!(follow_entry.flags.follow_loot_rules);
    assert!(follow_entry.flags.needs_quest);
}

#[test]
fn represented_loot_response_items_use_cpp_ui_type_decision_tree() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 100);

    let mut rolling_entry = represented_loot_entry(0, 25, player_guid);
    rolling_entry.flags.blocked = true;

    let mut won_entry = represented_loot_entry(1, 26, player_guid);
    won_entry.roll_winner = player_guid;

    let mut hidden_entry = represented_loot_entry(2, 27, player_guid);
    hidden_entry.roll_winner = other_guid;

    let mut allowed_entry = represented_loot_entry(3, 28, player_guid);
    allowed_entry.flags.under_threshold = true;

    let loot = CreatureLoot {
        loot_guid,
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player_guid],
        items: vec![rolling_entry, won_entry, hidden_entry, allowed_entry],
        looted_by_player: false,
    };

    let items = represented_loot_response_items_like_cpp(&loot, player_guid);

    assert_eq!(items.len(), 3);
    assert_eq!(items[0].loot_list_id, 0);
    assert_eq!(items[0].ui_type, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP);
    assert_eq!(items[1].loot_list_id, 1);
    assert_eq!(items[1].ui_type, LOOT_SLOT_TYPE_OWNER_LIKE_CPP);
    assert_eq!(items[2].loot_list_id, 3);
    assert_eq!(items[2].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);
}

#[test]
fn represented_ffa_loot_uses_player_ffa_items_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 101);

    let mut ffa_entry = represented_loot_entry(0, 25, player_guid);
    ffa_entry.flags.freeforall = true;
    ffa_entry.allowed_looters.clear();

    let mut loot = CreatureLoot {
        loot_guid,
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: vec![ffa_entry],
        looted_by_player: false,
    };

    mark_loot_allowed_for_player_like_cpp(&mut loot, player_guid);
    mark_loot_allowed_for_player_like_cpp(&mut loot, other_guid);
    assert_eq!(loot.unlooted_count, 2);

    let player_items = represented_loot_response_items_like_cpp(&loot, player_guid);
    let other_items = represented_loot_response_items_like_cpp(&loot, other_guid);
    assert_eq!(player_items.len(), 1);
    assert_eq!(other_items.len(), 1);
    assert_eq!(player_items[0].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);
    assert_eq!(other_items[0].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);

    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, player_guid);
    assert_eq!(loot.unlooted_count, 1);

    let player_ffa = loot
        .player_ffa_items
        .iter()
        .find(|(player, _)| *player == player_guid)
        .and_then(|(_, items)| items.iter().find(|item| item.loot_list_id == 0))
        .unwrap();
    let other_ffa = loot
        .player_ffa_items
        .iter()
        .find(|(player, _)| *player == other_guid)
        .and_then(|(_, items)| items.iter().find(|item| item.loot_list_id == 0))
        .unwrap();

    assert!(player_ffa.is_looted);
    assert!(!other_ffa.is_looted);
    assert!(represented_loot_response_items_like_cpp(&loot, player_guid).is_empty());
    assert_eq!(
        represented_loot_response_items_like_cpp(&loot, other_guid).len(),
        1
    );
}

#[test]
fn prospecting_and_milling_release_consume_at_most_five_source_items_like_cpp() {
    assert_eq!(
        direct_item_count_after_loot_release_like_cpp(20, Some(5)),
        15
    );
    assert_eq!(direct_item_count_after_loot_release_like_cpp(5, Some(5)), 0);
    assert_eq!(direct_item_count_after_loot_release_like_cpp(3, Some(5)), 0);
    assert_eq!(direct_item_count_after_loot_release_like_cpp(20, None), 0);
}

#[test]
fn represented_unlooted_count_counts_shared_items_once_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 102);

    let mut entry = represented_loot_entry(0, 25, player_guid);
    entry.allowed_looters.clear();

    let mut loot = CreatureLoot {
        loot_guid,
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: vec![entry],
        looted_by_player: false,
    };

    mark_loot_allowed_for_player_like_cpp(&mut loot, player_guid);
    assert_eq!(loot.unlooted_count, 1);
    assert!(loot.items[0].flags.counted);

    mark_loot_allowed_for_player_like_cpp(&mut loot, other_guid);
    assert_eq!(loot.unlooted_count, 1);

    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, player_guid);
    assert_eq!(loot.unlooted_count, 0);
    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, player_guid);
    assert_eq!(loot.unlooted_count, 0);
}

#[test]
fn shared_gameobject_normal_item_with_two_looters_counts_once_like_cpp() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let mut entry = represented_loot_entry(0, 25, first);
    entry.allowed_looters = vec![first, second];
    let mut loot = CreatureLoot {
        loot_guid: represented_loot_object_guid_like_cpp(test_gameobject_guid(91_101)),
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: vec![entry],
        looted_by_player: false,
    };

    prepare_represented_shared_loot_generation_like_cpp(&mut loot, &[first, second]);

    assert_eq!(loot.allowed_looters, vec![first, second]);
    assert_eq!(loot.items[0].allowed_looters, vec![first, second]);
    assert_eq!(loot.unlooted_count, 1);
    assert!(loot.items[0].flags.counted);
}

#[test]
fn shared_gameobject_item_evaluates_each_looter_after_roll_like_cpp() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let store_item_context = LootStoreItemContext {
        store_kind: LootStoreKind::Reference,
        entry: 900,
        item: LootStoreItem {
            item_id: 25,
            reference: 0,
            chance: 100.0,
            needs_quest: false,
            loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
            group_id: 0,
            min_count: 1,
            max_count: 1,
        },
    };
    let generated = GeneratedLootItem {
        item_id: 25,
        count: 1,
        loot_list_id: 0,
        random_properties_id: 0,
        random_properties_seed: 0,
        context: ItemContext::None as u8,
        store_item_context,
        free_for_all: false,
        follow_loot_rules: true,
        needs_quest: false,
        is_looted: false,
        is_blocked: false,
        is_under_threshold: false,
        is_counted: false,
    };
    let mut evaluated = Vec::new();

    let entry = generated_shared_gameobject_loot_item_to_entry_like_cpp(
        generated,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
        &[first, second],
        |context, looter| {
            evaluated.push((context, looter));
            looter == second
        },
    );

    assert_eq!(
        evaluated,
        vec![(store_item_context, first), (store_item_context, second)]
    );
    assert_eq!(entry.item_id, 25, "the rolled candidate remains present");
    assert_eq!(entry.allowed_looters, vec![second]);
}

#[test]
fn represented_loot_removed_uses_players_looting_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let open_guid = ObjectGuid::create_player(1, 77);
    let closed_guid = ObjectGuid::create_player(1, 88);
    let stale_guid = ObjectGuid::create_player(1, 99);
    let owner_guid = test_creature_guid(19_095);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (open_tx, open_rx) = flume::bounded::<Vec<u8>>(1);
    let (closed_tx, closed_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        open_guid,
        broadcast_info(open_guid, open_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        closed_guid,
        broadcast_info(closed_guid, closed_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid, open_guid, stale_guid],
            allowed_looters: vec![player_guid, open_guid, closed_guid, stale_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid, open_guid, closed_guid, stale_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session.represented_notify_loot_item_removed_like_cpp(owner_guid, 0);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
    assert_eq!(sent.read_uint8().unwrap(), 0);

    let sent = open_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRemoved as u16
    );
    assert!(closed_rx.try_recv().is_err());
    assert_eq!(
        session.loot_table.get(&owner_guid).unwrap().players_looting,
        vec![player_guid, open_guid]
    );
}

#[test]
fn durable_item_fanout_uses_precommit_union_exact_commit_cut_like_cpp() {
    let before = ObjectGuid::create_player(1, 41);
    let during = ObjectGuid::create_player(1, 42);
    let after = ObjectGuid::create_player(1, 43);

    let viewers = super::durable_loot_item_fanout_viewers_like_cpp(&[before], &[before, during]);

    assert_eq!(viewers, HashSet::from([before, during]));
    assert!(
        !viewers.contains(&after),
        "a later authority sample must never expand the exact commit fanout"
    );
}

#[test]
fn represented_money_removed_erases_missing_players_looting_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let open_guid = ObjectGuid::create_player(1, 77);
    let stale_guid = ObjectGuid::create_player(1, 99);
    let owner_guid = test_creature_guid(19_096);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (open_tx, open_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        open_guid,
        broadcast_info(open_guid, open_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 7,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid, open_guid, stale_guid],
            allowed_looters: vec![player_guid, open_guid, stale_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session.represented_notify_money_removed_like_cpp(owner_guid);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);

    let sent = open_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(
        session.loot_table.get(&owner_guid).unwrap().players_looting,
        vec![player_guid, open_guid]
    );
}

#[test]
fn loot_directory_delivery_rejects_replaced_session_generation_like_cpp() {
    let guid = ObjectGuid::create_player(1, 91);
    let registry = PlayerRegistry::new();
    let (first_tx, first_rx) = flume::bounded(2);
    registry.register_or_replace(guid, broadcast_info(guid, first_tx), Default::default());
    let stale = registry
        .loot_presence(guid)
        .expect("first connected loot recipient");

    let (replacement_tx, replacement_rx) = flume::bounded(2);
    registry.register_or_replace(
        guid,
        broadcast_info(guid, replacement_tx),
        Default::default(),
    );

    assert_eq!(
        registry.send_current_packet(stale.registration, vec![0xAA]),
        Err(crate::session::directory::PlayerDirectorySendError::StaleRegistration)
    );
    assert!(first_rx.try_recv().is_err());
    assert!(replacement_rx.try_recv().is_err());

    let current = registry
        .loot_presence(guid)
        .expect("replacement loot recipient");
    registry
        .send_current_packet(current.registration, vec![0xBB])
        .expect("current generation receives its packet");
    assert_eq!(replacement_rx.try_recv().unwrap(), vec![0xBB]);
}

fn loot_release_packet(object: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&object);
    pkt.reset_read();
    pkt
}

fn loot_money_packet() -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn recv_packet_with_opcode(
    rx: &flume::Receiver<Vec<u8>>,
    opcode: wow_constants::ServerOpcodes,
) -> WorldPacket {
    for _ in 0..8 {
        let sent = rx.try_recv().unwrap();
        let mut packet = WorldPacket::from_bytes(&sent);
        if packet.read_uint16().unwrap() == opcode as u16 {
            return packet;
        }
    }
    panic!("expected packet opcode {:?}", opcode);
}

fn drain_server_opcodes_like_cpp(rx: &flume::Receiver<Vec<u8>>) -> Vec<u16> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        opcodes.push(packet.read_uint16().unwrap());
    }
    opcodes
}

fn test_creature(guid: ObjectGuid, is_alive: bool) -> CreatureAI {
    let mut creature = CreatureAI::new(
        guid,
        1,
        Position::ZERO,
        100,
        1,
        1,
        2,
        0.0,
        1,
        35,
        0,
        0,
        0,
        0,
        0,
        None,
        0,
    );
    creature.is_alive = is_alive;
    creature
}

fn register_test_creature_like_cpp(session: &mut WorldSession, creature: CreatureAI) {
    if session.map_manager.is_none() {
        session.set_map_manager(Arc::new(RwLock::new(crate::map_manager::MapManager::new())));
    }

    let create_data = CreatureCreateData {
        guid: creature.guid,
        entry: creature.entry,
        display_id: creature.display_id,
        native_display_id: creature.display_id,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: i64::from(creature.hp.max(1)),
        max_health: i64::from(creature.max_hp.max(1)),
        level: creature.level,
        faction_template: creature.faction as i32,
        npc_flags: u64::from(creature.npc_flags),
        unit_flags: creature.unit_flags,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: crate::map_manager::WorldCreature::health_aura_state_like_cpp(
            u64::from(creature.hp.max(1)),
            u64::from(creature.max_hp.max(1)),
            true,
        ),
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 1,
        power: [0; 10],
        max_power: [0; 10],
        base_mana: 0,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: 0,
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.0,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 0,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };
    let guid = creature.guid;
    let is_alive = creature.is_alive;
    session.register_world_creature(
        session.player_map_id_like_cpp(),
        creature.current_pos,
        create_data,
        creature.min_dmg,
        creature.max_dmg,
        creature.aggro_radius,
        creature.loot_id,
        0,
        creature.gold_min,
        creature.gold_max,
        creature.boss_id,
        creature.dungeon_encounter_id,
        0,
        0,
        0,
        -1,
    );
    if !is_alive {
        let _ = session.mutate_world_creature(guid, |world_creature| {
            world_creature.creature.mark_ai_dead(0);
        });
    }
}

fn insert_allowed_coin_loot_like_cpp(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
    coins: u32,
) {
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(owner_guid),
            coins,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    if session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .is_some()
    {
        install_cached_test_creature_loot_authority_like_cpp(session, owner_guid, player_guid);
    }
}

/// Legacy packet tests construct the result of `Unit::Kill` directly.
/// Install that fixture into the creature before exercising CMSG_LOOT_UNIT
/// so the request remains a pure read/reconciliation path, like C++.
fn install_cached_test_creature_loot_authority_like_cpp(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    scope_player: ObjectGuid,
) {
    if let Some(loot) = session.loot_table.get_mut(&owner_guid) {
        // The fixtures describe the already-filtered post-FillLoot item
        // set. Rebuild only its derived counters; adding looters here
        // would erase negative eligibility cases the fixture represents.
        super::rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(loot);
    }
    session
        .sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, scope_player)
        .expect("the kill-time loot fixture must install into its creature authority");
}

fn two_sessions_with_authoritative_creature_loot_like_cpp(
    mut loot: CreatureLoot,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    WorldSession,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
    ObjectGuid,
    ObjectGuid,
) {
    let (mut first, first_rx) = make_session_with_send_capacity(32);
    let (mut second, second_rx) = make_session_with_send_capacity(32);
    let first_guid = ObjectGuid::create_player(1, 42);
    let second_guid = ObjectGuid::create_player(1, 43);
    let owner_guid = test_creature_guid(19_500);
    let shared_map = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));

    first.set_player_guid(Some(first_guid));
    second.set_player_guid(Some(second_guid));
    install_limited_test_item_template(&mut first, 25, 0);
    install_limited_test_item_template(&mut second, 25, 0);
    first.set_player_position_like_cpp(Position::ZERO);
    second.set_player_position_like_cpp(Position::ZERO);
    first.set_map_manager(Arc::clone(&shared_map));
    second.set_map_manager(shared_map);
    register_test_creature_like_cpp(&mut first, test_creature(owner_guid, false));

    loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    loot.allowed_looters = vec![first_guid, second_guid];
    for entry in &mut loot.items {
        entry.allowed_looters = vec![first_guid, second_guid];
    }
    first.loot_table.insert(owner_guid, loot);
    first
        .sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, first_guid)
        .unwrap();

    first.set_active_loot_guid(owner_guid);
    let first_response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &first.loot_table[&owner_guid],
        first_guid,
    );
    first.represented_on_loot_opened_like_cpp(owner_guid, first_guid, first_response);
    assert!(second.reconcile_represented_loot_cache_like_cpp(owner_guid, second_guid));
    second.set_active_loot_guid(owner_guid);
    let second_response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &second.loot_table[&owner_guid],
        second_guid,
    );
    second.represented_on_loot_opened_like_cpp(owner_guid, second_guid, second_response);

    (
        first,
        first_rx,
        second,
        second_rx,
        owner_guid,
        first_guid,
        second_guid,
    )
}

fn authoritative_test_loot_like_cpp(coins: u32, with_item: bool) -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins,
        unlooted_count: u8::from(with_item),
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: with_item
            .then(|| LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: Vec::new(),
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            })
            .into_iter()
            .collect(),
        looted_by_player: false,
    }
}

fn authoritative_test_loot_response_like_cpp(
    owner_guid: ObjectGuid,
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
) -> LootResponse {
    LootResponse {
        owner: owner_guid,
        loot_obj: loot.loot_guid,
        failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
        acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
        loot_method: loot.loot_method,
        threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
        coins: loot.coins,
        items: represented_loot_response_items_like_cpp(loot, player_guid),
        currencies: Vec::new(),
        acquired: true,
        ae_looting: false,
    }
}

#[tokio::test]
async fn full_loot_response_queue_rolls_back_open_without_blocking_authority_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 61_900);
    let owner_guid = test_creature_guid(61_901);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));

    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    loot.allowed_looters = vec![player_guid];
    loot.items[0].allowed_looters = vec![player_guid];
    session.loot_table.insert(owner_guid, loot);
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .unwrap();
    session.set_active_loot_guid(owner_guid);
    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &session.loot_table[&owner_guid],
        player_guid,
    );

    let sentinel = vec![0xAA, 0x55];
    session.send_tx().send(sentinel.clone()).unwrap();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let open_thread = std::thread::spawn(move || {
        session.represented_on_loot_opened_like_cpp(owner_guid, player_guid, response);
        done_tx
            .send((
                session.loot_table.contains_key(&owner_guid),
                session.active_loot_view_owners.contains(&owner_guid),
            ))
            .unwrap();
    });

    let (cached, active) = done_rx
        .recv_timeout(Duration::from_millis(500))
        .expect("a full socket queue must not block while loot authority is locked");
    open_thread.join().unwrap();
    assert!(!cached);
    assert!(!active);
    assert_eq!(send_rx.try_recv().unwrap(), sentinel);
    assert!(send_rx.try_recv().is_err(), "no LootResponse was enqueued");

    let rejected = authority.snapshot_for_player_like_cpp(player_guid).unwrap();
    assert!(rejected.loot.players_looting.is_empty());
    assert!(!rejected.loot.looted_by_player);

    let claim = tokio::time::timeout(
        Duration::from_millis(500),
        authority.reserve_item_like_cpp(player_guid, 0),
    )
    .await
    .expect("a failed response enqueue must release the authority mutex")
    .unwrap();
    assert!(claim.rollback_like_cpp());
    authority.retire_like_cpp();
    assert!(authority.is_retired_like_cpp());
}

#[tokio::test]
async fn successful_loot_open_queues_response_before_claim_removal_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 61_910);
    let owner_guid = test_creature_guid(61_911);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));

    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    loot.allowed_looters = vec![player_guid];
    loot.items[0].allowed_looters = vec![player_guid];
    session.loot_table.insert(owner_guid, loot);
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .unwrap();
    session.set_active_loot_guid(owner_guid);
    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &session.loot_table[&owner_guid],
        player_guid,
    );

    session.represented_on_loot_opened_like_cpp(owner_guid, player_guid, response);
    let claim = authority
        .reserve_item_like_cpp(player_guid, 0)
        .await
        .unwrap();
    assert_eq!(claim.commit_like_cpp(), Ok(true));
    let committed = authority.snapshot_for_player_like_cpp(player_guid).unwrap();
    session.represented_notify_loot_item_removed_from_snapshot_like_cpp(
        owner_guid,
        Some(&authority),
        &committed,
        0,
    );

    let opcodes = drain_server_opcodes_like_cpp(&send_rx);
    let response_index = opcodes
        .iter()
        .position(|opcode| *opcode == wow_constants::ServerOpcodes::LootResponse as u16)
        .expect("the accepted opening response was queued");
    let removal_index = opcodes
        .iter()
        .position(|opcode| *opcode == wow_constants::ServerOpcodes::LootRemoved as u16)
        .expect("the committed claim removal was queued");
    assert!(
        response_index < removal_index,
        "the authority lock must order LootResponse before LootRemoved: {opcodes:?}"
    );
}

#[test]
fn stale_player_map_key_does_not_rebind_creature_loot_authorities_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_709);
    let owner_guid = test_creature_guid(61_710);
    session.set_player_guid(Some(player_guid));
    session.set_state(SessionState::LoggedIn);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    let canonical_creature = make_canonical_creature_for_session(&session, owner_guid);
    attach_canonical_creature(&mut session, canonical_creature);

    let (map_id, instance_id) = session.current_legacy_runtime_map_key_like_cpp();
    let map_key = wow_map::MapKey::new(u32::from(map_id), instance_id);
    let legacy_before = session
        .read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
        .expect("the legacy creature owns its pristine authority");
    let canonical_before = session
        .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
        .expect("the canonical creature owns a separate pristine authority");
    assert!(!legacy_before.shares_storage_like_cpp(&canonical_before));
    assert_eq!(session.current_canonical_player_map_key_like_cpp(), None);

    assert!(
        session
            .represented_owned_loot_authority_like_cpp(owner_guid)
            .is_none(),
        "a logged-in player between maps must fail closed"
    );

    let legacy_after = session
        .read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
        .unwrap();
    let canonical_after = session
        .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
        .unwrap();
    assert!(legacy_after.shares_storage_like_cpp(&legacy_before));
    assert!(canonical_after.shares_storage_like_cpp(&canonical_before));
    assert!(
        !legacy_after.shares_storage_like_cpp(&canonical_after),
        "reconciliation must not mutate either stale-map mirror"
    );
}

fn represented_disenchant_test_outputs_like_cpp(
    winner_guid: ObjectGuid,
    item_id: u32,
) -> Vec<LootEntry> {
    (0..2)
        .map(|loot_list_id| LootEntry {
            loot_list_id,
            item_id,
            quantity: 1,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags: LootEntryFlags {
                follow_loot_rules: true,
                ..Default::default()
            },
            allowed_looters: vec![winner_guid],
            roll_winner: winner_guid,
            ffa_looted_by: Vec::new(),
            taken: false,
        })
        .collect()
}

#[tokio::test]
async fn two_sessions_claim_one_authoritative_item_exactly_once_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let loot_obj = represented_loot_object_guid_like_cpp(owner);

    let first_barrier = Arc::clone(&barrier);
    let first_task = tokio::spawn(async move {
        first_barrier.wait().await;
        first.handle_loot_item(loot_item_packet(loot_obj, 0)).await;
        first
    });
    let second_barrier = Arc::clone(&barrier);
    let second_task = tokio::spawn(async move {
        second_barrier.wait().await;
        second.handle_loot_item(loot_item_packet(loot_obj, 0)).await;
        second
    });
    barrier.wait().await;

    let mut first = first_task.await.unwrap();
    let _second = second_task.await.unwrap();
    assert_eq!(grants.load(Ordering::SeqCst), 1);
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 0);
}

#[tokio::test]
async fn durable_direct_item_claim_notifies_removed_before_item_push_like_cpp() {
    let (mut first, first_rx, _second, _second_rx, owner, _first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    // Use one observer channel for both physical routes so this test can
    // assert their relative C++ wire order. Route separation is covered
    // independently by the ItemPushResult test above.
    let shared_send = first.send_tx().clone();
    first.install_realm_send_channel_for_test(shared_send);
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);

    first
        .handle_loot_item(loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;

    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert_eq!(
        drain_server_opcodes_like_cpp(&first_rx),
        vec![
            wow_constants::ServerOpcodes::LootRemoved as u16,
            wow_constants::ServerOpcodes::ItemPushResult as u16,
        ],
        "C++ Player::StoreLootItem notifies removal before SendNewItem"
    );
}

fn install_quest_bound_loot_objective_like_cpp(
    session: &mut WorldSession,
    quest_id: u32,
    item_id: u32,
    current_count: i32,
    required_count: i32,
) {
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: required_count,
        flags: 0,
        flags2: 1,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![current_count],
            slot: 0,
        },
    );
}

#[tokio::test]
async fn quest_bound_loot_credits_objective_without_physical_item_like_cpp() {
    let (mut first, first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let quest_id = 8_336;
    let item_id = 25;
    install_quest_bound_loot_objective_like_cpp(&mut first, quest_id, item_id, 5, 6);
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);

    first
        .handle_loot_item(loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;

    assert_eq!(
        grants.load(Ordering::SeqCst),
        0,
        "C++ StoreNewItem returns nullptr for quest-bound objective credit"
    );
    let status = first.player_quests.get(&quest_id).expect("active quest");
    assert_eq!(status.objective_counts, vec![6]);
    assert_eq!(
        status.status,
        crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert!(!first.item_loot_quest_status_allows_like_cpp(
        item_id,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
    ));

    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 0);

    let opcodes = drain_server_opcodes_like_cpp(&first_rx);
    let bound_credit = wow_constants::ServerOpcodes::ItemPushResult as u16;
    let loot_removed = wow_constants::ServerOpcodes::LootRemoved as u16;
    assert!(opcodes.contains(&bound_credit), "{opcodes:?}");
    assert!(opcodes.contains(&loot_removed), "{opcodes:?}");
    assert!(
        opcodes.iter().position(|opcode| *opcode == bound_credit)
            < opcodes.iter().position(|opcode| *opcode == loot_removed),
        "C++ bound objective notification precedes the committed loot removal: {opcodes:?}"
    );
}

#[tokio::test]
async fn failed_quest_bound_loot_persistence_rolls_back_credit_and_claim_like_cpp() {
    let (mut first, first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let quest_id = 8_336;
    install_quest_bound_loot_objective_like_cpp(&mut first, quest_id, 25, 5, 6);
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), false);

    first
        .handle_loot_item(loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;

    assert_eq!(grants.load(Ordering::SeqCst), 0);
    let status = first.player_quests.get(&quest_id).expect("active quest");
    assert_eq!(status.objective_counts, vec![5]);
    assert_eq!(
        status.status,
        crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(!snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 1);
}

#[tokio::test]
async fn quest_bound_loot_still_requires_can_store_new_item_like_cpp() {
    let (mut first, first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let quest_id = 8_336;
    let item_id = 25;
    install_quest_bound_loot_objective_like_cpp(&mut first, quest_id, item_id, 5, 6);
    install_limited_test_item_template(&mut first, item_id, 1);
    let existing_guid = ObjectGuid::create_item(1, 83_360);
    first.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: existing_guid,
            entry_id: item_id,
            db_guid: existing_guid.counter() as u64,
            inventory_type: None,
        },
    );
    let existing_item = first.make_inventory_item_object(
        existing_guid,
        item_id,
        first_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    first.insert_inventory_item_object(existing_item);
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);

    first
        .handle_loot_item(loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;

    assert_eq!(grants.load(Ordering::SeqCst), 0);
    let status = first.player_quests.get(&quest_id).expect("active quest");
    assert_eq!(status.objective_counts, vec![5]);
    assert_eq!(
        status.status,
        crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(!snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 1);
}

#[tokio::test]
async fn item_grant_commit_unknown_quarantines_claim_and_kicks_even_when_queue_full() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(first_guid, 0)
        .await
        .unwrap();
    let (command_tx, command_rx) = flume::bounded(1);
    command_tx
        .send(SessionCommand::KickLikeCpp(KickLikeCppCommand {
            reason: "preexisting".to_string(),
        }))
        .unwrap();

    let worker = super::spawn_sql_loot_claim_persistence_worker_like_cpp(
        async {
            Err(SqlTransactionCommitError::CommitOutcomeUnknown(
                DatabaseError::Transaction("lost COMMIT reply".to_string()),
            ))
        },
        Some(claim),
        None,
        command_tx,
    )
    .unwrap();
    let result = worker.await.unwrap();

    assert!(matches!(
        result,
        Err(super::LootClaimPersistenceWorkerError::Persistence(
            SqlTransactionCommitError::CommitOutcomeUnknown(_)
        ))
    ));
    assert_eq!(
        authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Quarantined
    );
    assert!(
        authority
            .reserve_item_for_award_like_cpp(first_guid, 0)
            .await
            .is_err(),
        "unknown COMMIT must never reopen the object-owned item claim"
    );

    let _preexisting = command_rx.recv().unwrap();
    let queued = tokio::time::timeout(Duration::from_secs(1), command_rx.recv_async())
        .await
        .expect("full command queue fallback must eventually enqueue the kick")
        .unwrap();
    assert!(matches!(queued, SessionCommand::KickLikeCpp(_)));
}

#[tokio::test]
async fn cancelled_after_runtime_apply_retains_multiviewer_fanout_and_corpse_lifecycle_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&registry));
    second.set_player_registry(registry);

    first.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    let corpse_before = first
        .mutate_world_creature(owner, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.set_corpse_despawn_at(Some(Instant::now() + Duration::from_secs(120)));
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
            creature.corpse_despawn_at()
        })
        .unwrap();
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(first_guid, 0)
        .await
        .unwrap();
    let context = LootItemClaimCommitContextLikeCpp {
        owner_guid: owner,
        loot_obj: represented_loot_object_guid_like_cpp(owner),
        loot_list_id: 0,
        player_guid: first_guid,
        free_for_all: false,
    };
    let fanout = first
        .prepare_durable_loot_item_fanout_like_cpp(&claim, context)
        .expect("pre-COMMIT fanout route");
    assert_eq!(fanout.precommit_snapshot.loot.players_looting.len(), 2);

    let runtime_inventory_applied = Arc::new(AtomicBool::new(true));
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
    let guard = first.begin_durable_item_loot_persistence_like_cpp();
    let worker = super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = commit_rx.await;
            Ok::<(), ()>(())
        },
        Some(claim),
        Some((
            guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid: owner,
                loot_list_id: 0,
                player_guid: first_guid,
                item_owner_auto_release: false,
                durable_item_money_applied_amount: None,
                durable_item_money_notified_amount: None,
                durable_item_money_balance_applied: None,
                item_fanout: Some(fanout),
                runtime_inventory_applied: Arc::clone(&runtime_inventory_applied),
            },
        )),
    )
    .unwrap();
    let waiter = tokio::spawn(async move { worker.await });
    started_rx.await.unwrap();
    waiter.abort();
    let _ = waiter.await;

    first.handle_loot_release(loot_release_packet(owner)).await;
    second.handle_loot_release(loot_release_packet(owner)).await;
    commit_tx.send(()).unwrap();
    first.wait_for_active_loot_persistence_like_cpp().await;

    assert!(!first.is_disconnecting());
    assert!(runtime_inventory_applied.load(Ordering::Acquire));
    assert!(
        drain_server_opcodes_like_cpp(&first_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRemoved as u16))
    );
    assert!(
        drain_server_opcodes_like_cpp(&second_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRemoved as u16))
    );
    let corpse_after = first
        .mutate_world_creature(owner, |creature| {
            (
                creature.corpse_despawn_at(),
                creature.has_lootable_dynamic_flag_like_cpp(),
            )
        })
        .unwrap();
    assert!(!corpse_after.1);
    assert!(corpse_after.0 <= corpse_before);
}

#[test]
fn retired_object_authority_releases_every_session_window_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    authority.retire_like_cpp();
    first.close_retired_active_loot_windows_like_cpp(first_guid);
    second.close_retired_active_loot_windows_like_cpp(second_guid);

    for (session, owner_guid) in [(&first, owner), (&second, owner)] {
        assert!(!session.active_loot_view_owners.contains(&owner_guid));
        assert!(!session.loot_table.contains_key(&owner_guid));
    }
    for rx in [&first_rx, &second_rx] {
        assert_eq!(
            drain_server_opcodes_like_cpp(rx),
            vec![wow_constants::ServerOpcodes::LootRelease as u16]
        );
    }
}

#[tokio::test]
async fn failed_authoritative_item_store_rolls_back_for_retry_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), false);
    let loot_obj = represented_loot_object_guid_like_cpp(owner);

    first.handle_loot_item(loot_item_packet(loot_obj, 0)).await;
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    assert!(
        !authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );

    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    first.handle_loot_item(loot_item_packet(loot_obj, 0)).await;
    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
}

#[tokio::test]
async fn two_sessions_claim_one_authoritative_money_pool_exactly_once_like_cpp() {
    let (first, _first_rx, second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let first_barrier = Arc::clone(&barrier);
    let first_task = tokio::spawn(async move {
        let mut first = first;
        first_barrier.wait().await;
        first.handle_loot_money(loot_money_packet()).await;
        first
    });
    let second_barrier = Arc::clone(&barrier);
    let second_task = tokio::spawn(async move {
        let mut second = second;
        second_barrier.wait().await;
        second.handle_loot_money(loot_money_packet()).await;
        second
    });
    barrier.wait().await;

    let mut first = first_task.await.unwrap();
    let second = second_task.await.unwrap();
    assert_eq!(
        first.player_gold_like_cpp() + second.player_gold_like_cpp(),
        9
    );
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
}

#[tokio::test]
async fn failed_authoritative_money_persistence_rolls_back_for_retry_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, false,
        ));
    first.set_loot_money_persistence_test_result_like_cpp(false);

    first.handle_loot_money(loot_money_packet()).await;
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    assert_eq!(first.player_gold_like_cpp(), 0);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        7
    );

    first.set_loot_money_persistence_test_result_like_cpp(true);
    first.handle_loot_money(loot_money_packet()).await;
    assert_eq!(first.player_gold_like_cpp(), 7);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
}

#[tokio::test]
async fn stale_active_money_view_cannot_claim_replacement_generation_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, false,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let opened_generation = first.active_loot_view_generations_like_cpp[&owner];
    let mut replacement = authoritative_test_loot_like_cpp(11, false);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![first_guid, second_guid];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    assert_ne!(opened_generation, replacement_generation);

    first.handle_loot_money(loot_money_packet()).await;

    assert_eq!(first.player_gold_like_cpp(), 0);
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.generation, replacement_generation);
    assert_eq!(snapshot.loot.coins, 11);
}

#[tokio::test]
async fn stale_active_item_view_cannot_claim_replacement_generation_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let mut replacement = authoritative_test_loot_like_cpp(0, true);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![first_guid, second_guid];
    replacement.items[0].allowed_looters = vec![first_guid, second_guid];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);

    first
        .handle_loot_item(loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;

    assert_eq!(grants.load(Ordering::SeqCst), 0);
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.generation, replacement_generation);
    assert!(!snapshot.loot.items[0].taken);
}

#[tokio::test]
async fn item_waiter_waking_on_replacement_rolls_back_new_generation_claim_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let blocker = authority
        .reserve_item_like_cpp(first_guid, 0)
        .await
        .unwrap();
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let loot_obj = represented_loot_object_guid_like_cpp(owner);
    let waiter = tokio::spawn(async move {
        first.handle_loot_item(loot_item_packet(loot_obj, 0)).await;
        first
    });
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }

    let mut replacement = authoritative_test_loot_like_cpp(0, true);
    replacement.loot_guid = loot_obj;
    replacement.allowed_looters = vec![first_guid, second_guid];
    replacement.items[0].allowed_looters = vec![first_guid, second_guid];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    drop(blocker);
    let _first = waiter.await.unwrap();

    assert_eq!(grants.load(Ordering::SeqCst), 0);
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.generation, replacement_generation);
    assert!(!snapshot.loot.items[0].taken);
}

#[tokio::test]
async fn stale_release_keeps_replacement_viewer_and_pool_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let mut replacement = authoritative_test_loot_like_cpp(13, true);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![first_guid, second_guid];
    replacement.items[0].allowed_looters = vec![first_guid, second_guid];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    authority.add_viewer_like_cpp(first_guid).unwrap();

    assert!(
        first
            .do_loot_release_owner_like_cpp(owner, first_guid)
            .await
    );

    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.generation, replacement_generation);
    assert_eq!(snapshot.loot.coins, 13);
    assert!(!snapshot.loot.items[0].taken);
    assert!(snapshot.loot.players_looting.contains(&first_guid));
    assert!(!first.active_loot_view_owners.contains(&owner));
}

#[tokio::test]
async fn durable_money_delta_is_order_independent_near_gold_cap_like_cpp() {
    let start = MAX_MONEY_AMOUNT - 7;
    let (after_first, first_delta) = loot_money_durable_outcome_like_cpp(start, 5);
    let (after_second, second_delta) = loot_money_durable_outcome_like_cpp(after_first, 5);
    assert_eq!((after_second, first_delta, second_delta), (start + 5, 5, 0));

    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(start);
    let committed = Arc::new(AtomicBool::new(false));
    let command_authority = OwnedLootAuthority::new();
    let command = |durable_delta| ApplyLootMoneyLikeCppCommand {
        recipient: player_guid,
        loot_owner: test_creature_guid(19_510),
        loot_obj: represented_loot_object_guid_like_cpp(test_creature_guid(19_510)),
        amount: 5,
        durable_applied_amount: Arc::new(AtomicU64::new(durable_delta)),
        durable_persistence_tracker: Default::default(),
        sole_looter: false,
        authority: command_authority.clone(),
        authority_generation: 1,
        authority_committed: Arc::clone(&committed),
        send_coin_removed: Arc::new(AtomicBool::new(false)),
        applied: Arc::new(AtomicBool::new(false)),
        published: Arc::new(AtomicBool::new(false)),
    };

    // Runtime delivery is deliberately opposite to the locked DB order.
    session
        .handle_apply_loot_money_like_cpp_command(command(second_delta))
        .await;
    session
        .handle_apply_loot_money_like_cpp_command(command(first_delta))
        .await;

    assert_eq!(session.player_gold_like_cpp(), start + 5);
    let opcodes = drain_server_opcodes_like_cpp(&send_rx);
    assert_eq!(
        opcodes,
        vec![
            wow_constants::ServerOpcodes::LootMoneyNotify as u16,
            wow_constants::ServerOpcodes::LootMoneyNotify as u16,
        ]
    );
}

#[tokio::test]
async fn durable_old_generation_payout_does_not_touch_replacement_loot_like_cpp() {
    let (mut first, first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, false,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let old_generation = first.active_loot_view_generations_like_cpp[&owner];
    let mut replacement = authoritative_test_loot_like_cpp(17, false);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![first_guid, second_guid];
    authority.replace_like_cpp(Some(replacement), HashMap::new());
    authority.add_viewer_like_cpp(first_guid).unwrap();

    first
        .handle_apply_loot_money_like_cpp_command(ApplyLootMoneyLikeCppCommand {
            recipient: first_guid,
            loot_owner: owner,
            loot_obj: represented_loot_object_guid_like_cpp(owner),
            amount: 3,
            durable_applied_amount: Arc::new(AtomicU64::new(3)),
            durable_persistence_tracker: Default::default(),
            sole_looter: true,
            authority: authority.clone(),
            authority_generation: old_generation,
            authority_committed: Arc::new(AtomicBool::new(true)),
            send_coin_removed: Arc::new(AtomicBool::new(true)),
            applied: Arc::new(AtomicBool::new(false)),
            published: Arc::new(AtomicBool::new(false)),
        })
        .await;

    assert_eq!(first.player_gold_like_cpp(), 3);
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.loot.coins, 17);
    assert!(snapshot.loot.players_looting.contains(&first_guid));
    let opcodes = drain_server_opcodes_like_cpp(&first_rx);
    assert!(!opcodes.contains(&(wow_constants::ServerOpcodes::CoinRemoved as u16)));
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| { **opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16 })
            .count(),
        1
    );
}

#[tokio::test]
async fn remote_group_money_is_one_atomic_durable_fanout_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    install_group_loot_group(&mut first, first_guid, second_guid);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    first.handle_loot_money(loot_money_packet()).await;
    second.process_represented_session_commands_like_cpp().await;

    // C++ divides integral copper and discards the remainder.
    assert_eq!(first.player_gold_like_cpp(), 4);
    assert_eq!(second.player_gold_like_cpp(), 4);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
    for opcodes in [
        drain_server_opcodes_like_cpp(&first_rx),
        drain_server_opcodes_like_cpp(&second_rx),
    ] {
        let coin = opcodes
            .iter()
            .position(|opcode| *opcode == wow_constants::ServerOpcodes::CoinRemoved as u16)
            .expect("active viewer must receive CoinRemoved");
        let money = opcodes
            .iter()
            .position(|opcode| *opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16)
            .expect("payout recipient must receive LootMoneyNotify");
        assert!(coin < money, "C++ removes coins before notifying payout");
    }
}

#[test]
fn pickpocket_money_is_not_shared_with_the_group_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let member_guid = ObjectGuid::create_player(1, 43);
    let owner = test_creature_guid(19_501);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    install_group_loot_group(&mut session, player_guid, member_guid);
    let registry = Arc::new(PlayerRegistry::default());
    let (member_tx, _member_rx) = flume::bounded(1);
    registry.register_or_replace(
        member_guid,
        broadcast_info(member_guid, member_tx),
        Default::default(),
    );
    session.set_player_registry(registry);
    let mut loot = authoritative_test_loot_like_cpp(8, false);
    loot.loot_type = LOOT_TYPE_PICKPOCKETING_LIKE_CPP;
    loot.allowed_looters = vec![player_guid, member_guid];
    session.loot_table.insert(owner, loot);

    assert_eq!(
        session.represented_loot_money_recipients_like_cpp(owner),
        vec![player_guid]
    );
}

#[test]
fn vehicle_corpse_money_shares_and_pool_allowed_looters_control_membership_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let member_guid = ObjectGuid::create_player(1, 43);
    let owner = ObjectGuid::create_vehicle_like_cpp(1, 0, 1, 19_502);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    install_group_loot_group(&mut session, player_guid, member_guid);
    let registry = Arc::new(PlayerRegistry::default());
    let (member_tx, _member_rx) = flume::bounded(1);
    registry.register_or_replace(
        member_guid,
        broadcast_info(member_guid, member_tx),
        Default::default(),
    );
    session.set_player_registry(registry);

    let mut loot = authoritative_test_loot_like_cpp(8, false);
    loot.loot_type = LOOT_TYPE_CORPSE_LIKE_CPP;
    loot.allowed_looters = vec![player_guid];
    session.loot_table.insert(owner, loot);
    assert_eq!(
        session.represented_loot_money_recipients_like_cpp(owner),
        vec![player_guid]
    );

    session
        .loot_table
        .get_mut(&owner)
        .unwrap()
        .allowed_looters
        .push(member_guid);
    assert_eq!(
        session.represented_loot_money_recipients_like_cpp(owner),
        vec![player_guid, member_guid]
    );
}

#[test]
fn corpse_money_reward_distance_ignores_range_only_in_same_dungeon_instance_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let member_guid = ObjectGuid::create_player(1, 43);
    let owner = test_creature_guid(19_503);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    install_group_loot_group(&mut session, player_guid, member_guid);
    let registry = Arc::new(PlayerRegistry::default());
    let (member_tx, _member_rx) = flume::bounded(1);
    let mut member = broadcast_info(member_guid, member_tx.clone());
    member.info.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    registry.register_or_replace(member_guid, member, Default::default());
    session.set_player_registry(Arc::clone(&registry));
    let mut loot = authoritative_test_loot_like_cpp(8, false);
    loot.allowed_looters = vec![player_guid, member_guid];
    session.loot_table.insert(owner, loot);

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
    assert_eq!(
        session.represented_loot_money_recipients_like_cpp(owner),
        vec![player_guid]
    );

    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    assert_eq!(
        session.represented_loot_money_recipients_like_cpp(owner),
        vec![player_guid, member_guid]
    );

    let mut wrong_instance = broadcast_info(member_guid, member_tx);
    wrong_instance.info.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    wrong_instance.placement.instance_id = 1;
    registry.register_or_replace(member_guid, wrong_instance, Default::default());
    assert_eq!(
        session.represented_loot_money_recipients_like_cpp(owner),
        vec![player_guid]
    );
}

#[test]
fn chest_allowed_looters_ignore_range_only_in_same_dungeon_instance_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let member_guid = ObjectGuid::create_player(1, 43);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    install_group_loot_group(&mut session, player_guid, member_guid);
    let registry = Arc::new(PlayerRegistry::default());
    let (member_tx, _member_rx) = flume::bounded(1);
    let mut member = broadcast_info(member_guid, member_tx.clone());
    member.info.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    registry.register_or_replace(member_guid, member, Default::default());
    session.set_player_registry(Arc::clone(&registry));

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
    assert_eq!(
        session.represented_group_looters_at_reward_distance_like_cpp(player_guid),
        vec![player_guid]
    );

    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    assert_eq!(
        session.represented_group_looters_at_reward_distance_like_cpp(player_guid),
        vec![player_guid, member_guid]
    );

    let mut wrong_instance = broadcast_info(member_guid, member_tx);
    wrong_instance.info.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    wrong_instance.placement.instance_id = 1;
    registry.register_or_replace(member_guid, wrong_instance, Default::default());
    assert_eq!(
        session.represented_group_looters_at_reward_distance_like_cpp(player_guid),
        vec![player_guid]
    );
}

#[tokio::test]
async fn failed_remote_group_money_transaction_credits_nobody_and_retries_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    install_group_loot_group(&mut first, first_guid, second_guid);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);
    first.set_loot_money_persistence_test_result_like_cpp(false);
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();

    first.handle_loot_money(loot_money_packet()).await;
    second.process_represented_session_commands_like_cpp().await;
    assert_eq!(first.player_gold_like_cpp(), 0);
    assert_eq!(second.player_gold_like_cpp(), 0);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        9
    );

    first.set_loot_money_persistence_test_result_like_cpp(true);
    first.handle_loot_money(loot_money_packet()).await;
    second.process_represented_session_commands_like_cpp().await;
    assert_eq!(first.player_gold_like_cpp(), 4);
    assert_eq!(second.player_gold_like_cpp(), 4);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
}

#[tokio::test]
async fn cancelled_money_waiter_cannot_reopen_a_durable_claim_like_cpp() {
    let (mut first, _first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    first.set_loot_money_persistence_test_result_like_cpp(true);
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let claim = authority.reserve_money_like_cpp(first_guid).await.unwrap();
    let authority_generation = claim.generation_like_cpp();
    let authority_committed = Arc::new(AtomicBool::new(false));
    let application = ApplyLootMoneyLikeCppCommand {
        recipient: second_guid,
        loot_owner: owner,
        loot_obj: represented_loot_object_guid_like_cpp(owner),
        amount: 9,
        durable_applied_amount: Arc::new(AtomicU64::new(0)),
        durable_persistence_tracker: second.durable_loot_money_persistence_tracker_like_cpp(),
        sole_looter: true,
        authority: authority.clone(),
        authority_generation,
        authority_committed: Arc::clone(&authority_committed),
        send_coin_removed: Arc::new(AtomicBool::new(true)),
        applied: Arc::new(AtomicBool::new(false)),
        published: Arc::new(AtomicBool::new(false)),
    };
    let delivery = (
        LootMoneyDeliveryAddressLikeCpp::Source(second.session_command_tx()),
        SessionCommand::ApplyLootMoneyLikeCpp(application),
    );
    let viewer_fanout = LootMoneyViewerFanoutLikeCpp {
        scope_player: first_guid,
        source_player: first_guid,
        source_command_tx: first.session_command_tx(),
        player_registry: first.player_registry().cloned(),
        map_id: first.player_map_id_like_cpp(),
        instance_id: first
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0),
        loot_owner: owner,
        loot_obj: represented_loot_object_guid_like_cpp(owner),
        authority: authority.clone(),
        authority_generation,
        payout_recipients: [second_guid].into_iter().collect(),
    };
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    let persistence = first
        .spawn_group_loot_money_persistence_like_cpp(
            vec![(second_guid, 9)],
            claim,
            vec![delivery],
            Arc::clone(&authority_committed),
            viewer_fanout,
        )
        .unwrap();

    // The outer packet task owns only the JoinHandle. Aborting it must not
    // cancel the detached SQL+authority worker that owns the lease.
    let waiter = tokio::spawn(async move { persistence.await });
    waiter.abort();
    let _ = waiter.await;
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    second.process_represented_session_commands_like_cpp().await;
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
    let zero = authority
        .reserve_money_like_cpp(first_guid)
        .await
        .expect("C++ keeps the view and serializes a later zero-money observation");
    assert_eq!(zero.payload_like_cpp(), &LootClaimPayload::Money(0));
    assert!(zero.commit_like_cpp().unwrap());
    assert!(authority_committed.load(Ordering::Acquire));
    assert_eq!(second.player_gold_like_cpp(), 9);
    let opcodes = drain_server_opcodes_like_cpp(&second_rx);
    let coin = opcodes
        .iter()
        .position(|opcode| *opcode == wow_constants::ServerOpcodes::CoinRemoved as u16)
        .unwrap();
    let money = opcodes
        .iter()
        .position(|opcode| *opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16)
        .unwrap();
    assert!(coin < money);
}

#[tokio::test]
async fn money_viewer_opened_during_persistence_receives_coin_removed_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    first.set_loot_money_persistence_test_result_like_cpp(true);
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    assert!(authority.remove_viewer_like_cpp(second_guid));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&registry));

    let claim = authority.reserve_money_like_cpp(first_guid).await.unwrap();
    let authority_generation = claim.generation_like_cpp();
    let authority_committed = Arc::new(AtomicBool::new(false));
    let application = ApplyLootMoneyLikeCppCommand {
        recipient: first_guid,
        loot_owner: owner,
        loot_obj: represented_loot_object_guid_like_cpp(owner),
        amount: 9,
        durable_applied_amount: Arc::new(AtomicU64::new(0)),
        durable_persistence_tracker: first.durable_loot_money_persistence_tracker_like_cpp(),
        sole_looter: true,
        authority: authority.clone(),
        authority_generation,
        authority_committed: Arc::clone(&authority_committed),
        send_coin_removed: Arc::new(AtomicBool::new(false)),
        applied: Arc::new(AtomicBool::new(false)),
        published: Arc::new(AtomicBool::new(false)),
    };
    let viewer_fanout = LootMoneyViewerFanoutLikeCpp {
        scope_player: first_guid,
        source_player: first_guid,
        source_command_tx: first.session_command_tx(),
        player_registry: Some(registry),
        map_id: first.player_map_id_like_cpp(),
        instance_id: first
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0),
        loot_owner: owner,
        loot_obj: represented_loot_object_guid_like_cpp(owner),
        authority: authority.clone(),
        authority_generation,
        payout_recipients: [first_guid].into_iter().collect(),
    };
    let persistence = first
        .spawn_group_loot_money_persistence_like_cpp(
            vec![(first_guid, 9)],
            claim,
            vec![(
                LootMoneyDeliveryAddressLikeCpp::Source(first.session_command_tx()),
                SessionCommand::ApplyLootMoneyLikeCpp(application),
            )],
            Arc::clone(&authority_committed),
            viewer_fanout,
        )
        .unwrap();

    authority
        .open_view_with_snapshot_like_cpp(second_guid, |_, _| ())
        .expect("late viewer opens before the detached worker commits");
    persistence.await.unwrap().unwrap();
    second.process_represented_session_commands_like_cpp().await;

    assert!(authority_committed.load(Ordering::Acquire));
    assert!(
        drain_server_opcodes_like_cpp(&second_rx)
            .contains(&(wow_constants::ServerOpcodes::CoinRemoved as u16)),
        "a viewer that saw non-zero money during SQL must receive C++ NotifyMoneyRemoved"
    );
}

#[tokio::test]
async fn cancelled_item_waiter_cannot_reopen_a_durable_claim_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, _second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(first_guid, 0)
        .await
        .unwrap();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let worker = super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = release_rx.await;
            Ok::<(), ()>(())
        },
        Some(claim),
        None,
    )
    .unwrap();
    let waiter = tokio::spawn(async move { worker.await });

    started_rx.await.unwrap();
    waiter.abort();
    let _ = waiter.await;
    release_tx.send(()).unwrap();
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }

    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 0);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(first_guid, 0)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn durable_item_completion_auto_releases_only_after_items_and_coins_are_empty_like_cpp() {
    for (coins, should_release) in [(0, true), (7, false)] {
        let (mut session, send_rx) = make_session_with_send_capacity(16);
        let player_guid = ObjectGuid::create_player(1, 61_801 + i64::from(coins));
        let owner_guid = ObjectGuid::create_item(1, 61_811 + i64::from(coins));
        install_active_item_loot_completion_fixture_like_cpp(
            &mut session,
            player_guid,
            owner_guid,
            coins,
        );
        let runtime_inventory_applied = Arc::new(AtomicBool::new(true));
        let guard = session.begin_durable_item_loot_persistence_like_cpp();
        super::spawn_loot_claim_persistence_worker_like_cpp(
            async { Ok::<(), ()>(()) },
            None,
            Some((
                guard,
                DurableItemLootCompletionLikeCpp {
                    owner_guid,
                    loot_list_id: 0,
                    player_guid,
                    item_owner_auto_release: true,
                    durable_item_money_applied_amount: None,
                    durable_item_money_notified_amount: None,
                    durable_item_money_balance_applied: None,
                    item_fanout: None,
                    runtime_inventory_applied,
                },
            )),
        )
        .unwrap()
        .await
        .unwrap()
        .unwrap();

        session.wait_for_active_loot_persistence_like_cpp().await;

        assert!(!session.is_disconnecting());
        assert_eq!(
            session.loot_table.contains_key(&owner_guid),
            !should_release
        );
        assert_eq!(session.is_active_loot_guid(owner_guid), !should_release);
        if !should_release {
            let loot = session.loot_table.get(&owner_guid).unwrap();
            assert!(loot.items[0].taken);
            assert_eq!(loot.coins, coins);
        }
        assert_eq!(
            drain_server_opcodes_like_cpp(&send_rx)
                .into_iter()
                .filter(|opcode| { *opcode == wow_constants::ServerOpcodes::LootRelease as u16 })
                .count(),
            usize::from(should_release),
            "C++ StoreLootItem checks Loot::isLooted(), including money"
        );
    }
}

#[tokio::test]
async fn durable_item_completion_never_auto_releases_creature_or_gameobject_owner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_820);
    session.set_player_guid(Some(player_guid));
    for owner_guid in [
        test_creature_guid(61_821),
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 1, 61_822),
    ] {
        session.set_active_loot_guid(owner_guid);
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: owner_guid,
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_ITEM_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid],
                allowed_looters: vec![player_guid],
                items: vec![represented_loot_entry(0, 25, player_guid)],
                looted_by_player: false,
            },
        );
        let guard = session.begin_durable_item_loot_persistence_like_cpp();
        super::spawn_loot_claim_persistence_worker_like_cpp(
            async { Ok::<(), ()>(()) },
            None,
            Some((
                guard,
                DurableItemLootCompletionLikeCpp {
                    owner_guid,
                    loot_list_id: 0,
                    player_guid,
                    item_owner_auto_release: false,
                    durable_item_money_applied_amount: None,
                    durable_item_money_notified_amount: None,
                    durable_item_money_balance_applied: None,
                    item_fanout: None,
                    runtime_inventory_applied: Arc::new(AtomicBool::new(true)),
                },
            )),
        )
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    }

    session.wait_for_active_loot_persistence_like_cpp().await;

    assert!(!session.is_disconnecting());
    assert!(session.loot_table.values().all(|loot| !loot.items[0].taken));
    assert!(
        !drain_server_opcodes_like_cpp(&send_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRelease as u16))
    );
}

#[tokio::test]
async fn cancelled_item_handler_after_commit_releases_and_forces_inventory_reload_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_830);
    let owner_guid = ObjectGuid::create_item(1, 61_831);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 0);
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
    let guard = session.begin_durable_item_loot_persistence_like_cpp();
    let worker = super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = commit_rx.await;
            Ok::<(), ()>(())
        },
        None,
        Some((
            guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: true,
                durable_item_money_applied_amount: None,
                durable_item_money_notified_amount: None,
                durable_item_money_balance_applied: None,
                item_fanout: None,
                runtime_inventory_applied: Arc::new(AtomicBool::new(false)),
            },
        )),
    )
    .unwrap();
    let waiter = tokio::spawn(async move { worker.await });
    started_rx.await.unwrap();
    waiter.abort();
    let _ = waiter.await;
    commit_tx.send(()).unwrap();

    session.wait_for_active_loot_persistence_like_cpp().await;

    assert!(session.is_disconnecting());
    assert!(!session.loot_table.contains_key(&owner_guid));
    assert!(!session.is_active_loot_guid(owner_guid));
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        1
    );
}

#[tokio::test]
async fn cancelled_world_owner_claim_after_commit_reconciles_cache_and_forces_reload_like_cpp() {
    let (mut session, _send_rx, _second, _second_rx, owner_guid, player_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(player_guid, 0)
        .await
        .unwrap();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
    let guard = session.begin_durable_item_loot_persistence_like_cpp();
    let worker = super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = commit_rx.await;
            Ok::<(), ()>(())
        },
        Some(claim),
        Some((
            guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: false,
                durable_item_money_applied_amount: None,
                durable_item_money_notified_amount: None,
                durable_item_money_balance_applied: None,
                item_fanout: None,
                runtime_inventory_applied: Arc::new(AtomicBool::new(false)),
            },
        )),
    )
    .unwrap();
    let waiter = tokio::spawn(async move { worker.await });
    started_rx.await.unwrap();
    waiter.abort();
    let _ = waiter.await;
    commit_tx.send(()).unwrap();

    session.wait_for_active_loot_persistence_like_cpp().await;

    assert!(session.is_disconnecting());
    assert!(
        authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
    assert!(
        session.loot_table.get(&owner_guid).unwrap().items[0].taken,
        "master/roll/direct world-owner grants share this claimed-store recovery path"
    );
}

#[tokio::test]
async fn failed_item_persistence_publishes_no_removal_or_release_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_840);
    let owner_guid = ObjectGuid::create_item(1, 61_841);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 0);
    let guard = session.begin_durable_item_loot_persistence_like_cpp();
    let result = super::spawn_loot_claim_persistence_worker_like_cpp(
        async { Err::<(), ()>(()) },
        None,
        Some((
            guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: true,
                durable_item_money_applied_amount: None,
                durable_item_money_notified_amount: None,
                durable_item_money_balance_applied: None,
                item_fanout: None,
                runtime_inventory_applied: Arc::new(AtomicBool::new(false)),
            },
        )),
    )
    .unwrap()
    .await
    .unwrap();
    assert!(result.is_err());

    session.wait_for_active_loot_persistence_like_cpp().await;

    assert!(!session.is_disconnecting());
    assert!(session.is_active_loot_guid(owner_guid));
    assert!(!session.loot_table.get(&owner_guid).unwrap().items[0].taken);
    assert!(
        !drain_server_opcodes_like_cpp(&send_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRelease as u16))
    );
}

#[tokio::test]
async fn cancelled_stored_item_money_before_commit_retries_without_local_consumption_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_845);
    let owner_guid = ObjectGuid::create_item(1, 61_846);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 7);
    session.set_player_gold_like_cpp(100);

    let durable_source_row = Arc::new(AtomicBool::new(true));
    let first_source = Arc::clone(&durable_source_row);
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (_commit_tx, commit_rx) = tokio::sync::oneshot::channel::<()>();
    let first_balance_applied = Arc::new(AtomicBool::new(false));
    let first_runtime_applied = Arc::new(AtomicBool::new(false));
    let first_guard = session.begin_durable_item_loot_persistence_like_cpp();
    let first_worker = super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = commit_rx.await;
            first_source
                .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
                .map(|_| ())
                .map_err(|_| ())
        },
        None,
        Some((
            first_guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: false,
                durable_item_money_applied_amount: Some(7),
                durable_item_money_notified_amount: Some(7),
                durable_item_money_balance_applied: Some(Arc::clone(&first_balance_applied)),
                item_fanout: None,
                runtime_inventory_applied: Arc::clone(&first_runtime_applied),
            },
        )),
    )
    .unwrap();
    started_rx.await.unwrap();
    first_worker.abort();
    assert!(first_worker.await.unwrap_err().is_cancelled());

    session.wait_for_active_loot_persistence_like_cpp().await;
    assert!(durable_source_row.load(Ordering::Acquire));
    assert_eq!(session.player_gold_like_cpp(), 100);
    assert_eq!(session.loot_table.get(&owner_guid).unwrap().coins, 7);
    assert!(!first_runtime_applied.load(Ordering::Acquire));

    let retry_source = Arc::clone(&durable_source_row);
    let retry_balance_applied = Arc::new(AtomicBool::new(false));
    let retry_runtime_applied = Arc::new(AtomicBool::new(false));
    let retry_guard = session.begin_durable_item_loot_persistence_like_cpp();
    super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            retry_source
                .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
                .map(|_| ())
                .map_err(|_| ())
        },
        None,
        Some((
            retry_guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: false,
                durable_item_money_applied_amount: Some(7),
                durable_item_money_notified_amount: Some(7),
                durable_item_money_balance_applied: Some(Arc::clone(&retry_balance_applied)),
                item_fanout: None,
                runtime_inventory_applied: Arc::clone(&retry_runtime_applied),
            },
        )),
    )
    .unwrap()
    .await
    .unwrap()
    .unwrap();

    session.wait_for_active_loot_persistence_like_cpp().await;
    assert!(!durable_source_row.load(Ordering::Acquire));
    assert_eq!(session.player_gold_like_cpp(), 107);
    assert_eq!(session.loot_table.get(&owner_guid).unwrap().coins, 0);
    assert!(retry_runtime_applied.load(Ordering::Acquire));
    assert!(session.is_active_loot_guid(owner_guid));
}

#[tokio::test]
async fn cancelled_stored_item_money_after_commit_is_replayed_before_disconnect_save_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_847);
    let owner_guid = ObjectGuid::create_item(1, 61_848);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 7);
    session.set_player_gold_like_cpp(100);

    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
    let balance_applied = Arc::new(AtomicBool::new(false));
    let runtime_money_applied = Arc::new(AtomicBool::new(false));
    let guard = session.begin_durable_item_loot_persistence_like_cpp();
    let worker = super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = commit_rx.await;
            Ok::<(), ()>(())
        },
        None,
        Some((
            guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: false,
                durable_item_money_applied_amount: Some(7),
                durable_item_money_notified_amount: Some(7),
                durable_item_money_balance_applied: Some(Arc::clone(&balance_applied)),
                item_fanout: None,
                runtime_inventory_applied: Arc::clone(&runtime_money_applied),
            },
        )),
    )
    .unwrap();
    let waiter = tokio::spawn(async move { worker.await });
    started_rx.await.unwrap();
    waiter.abort();
    let _ = waiter.await;

    let mut save = Box::pin(session.save_disconnect_player_to_db_like_cpp());
    assert!(
        tokio::time::timeout(Duration::from_millis(5), &mut save)
            .await
            .is_err(),
        "disconnect save must remain pending until durable loot publication is idle"
    );
    commit_tx.send(()).unwrap();
    save.await;

    assert_eq!(session.player_gold_like_cpp(), 107);
    assert!(runtime_money_applied.load(Ordering::Acquire));
}

#[tokio::test]
async fn stored_item_money_save_reconciled_balance_still_publishes_source_once_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_848);
    let owner_guid = ObjectGuid::create_item(1, 61_849);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 7);
    session.set_player_gold_like_cpp(107);
    let balance_applied = Arc::new(AtomicBool::new(true));
    let publication_applied = Arc::new(AtomicBool::new(false));
    let mut guard = session.begin_durable_item_loot_persistence_like_cpp();
    guard.mark_committed_like_cpp(DurableItemLootCompletionLikeCpp {
        owner_guid,
        loot_list_id: 0,
        player_guid,
        item_owner_auto_release: false,
        durable_item_money_applied_amount: Some(7),
        durable_item_money_notified_amount: Some(7),
        durable_item_money_balance_applied: Some(Arc::clone(&balance_applied)),
        item_fanout: None,
        runtime_inventory_applied: Arc::clone(&publication_applied),
    });
    drop(guard);

    session
        .apply_pending_durable_item_loot_completions_like_cpp()
        .await;

    assert_eq!(session.player_gold_like_cpp(), 107);
    assert_eq!(session.loot_table.get(&owner_guid).unwrap().coins, 0);
    assert!(balance_applied.load(Ordering::Acquire));
    assert!(publication_applied.load(Ordering::Acquire));
    assert!(
        drain_server_opcodes_like_cpp(&send_rx)
            .contains(&(wow_constants::ServerOpcodes::LootMoneyNotify as u16))
    );
}

#[tokio::test]
async fn stored_item_money_delete_cas_allows_exactly_one_durable_grant_like_cpp() {
    assert_eq!(
        super::STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP,
        1,
        "the source-row delete must fail the whole transaction after another winner"
    );
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_849);
    let owner_guid = ObjectGuid::create_item(1, 61_850);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 7);
    session.set_player_gold_like_cpp(100);

    let source_row = Arc::new(AtomicBool::new(true));
    let durable_grants = Arc::new(AtomicUsize::new(0));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let source_row = Arc::clone(&source_row);
        let durable_grants = Arc::clone(&durable_grants);
        let guard = session.begin_durable_item_loot_persistence_like_cpp();
        workers.push(
            super::spawn_loot_claim_persistence_worker_like_cpp(
                async move {
                    tokio::task::yield_now().await;
                    source_row
                        .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
                        .map_err(|_| ())?;
                    durable_grants.fetch_add(1, Ordering::SeqCst);
                    Ok::<(), ()>(())
                },
                None,
                Some((
                    guard,
                    DurableItemLootCompletionLikeCpp {
                        owner_guid,
                        loot_list_id: 0,
                        player_guid,
                        item_owner_auto_release: false,
                        durable_item_money_applied_amount: Some(7),
                        durable_item_money_notified_amount: Some(7),
                        durable_item_money_balance_applied: Some(Arc::new(AtomicBool::new(false))),
                        item_fanout: None,
                        runtime_inventory_applied: Arc::new(AtomicBool::new(false)),
                    },
                )),
            )
            .unwrap(),
        );
    }

    let mut successes = 0;
    for worker in workers {
        if matches!(worker.await, Ok(Ok(()))) {
            successes += 1;
        }
    }
    session.wait_for_active_loot_persistence_like_cpp().await;

    assert_eq!(successes, 1);
    assert_eq!(durable_grants.load(Ordering::SeqCst), 1);
    assert_eq!(session.player_gold_like_cpp(), 107);
    assert_eq!(session.loot_table.get(&owner_guid).unwrap().coins, 0);
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| { *opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16 })
            .count(),
        1
    );
}

#[tokio::test]
async fn cancelled_zero_stored_item_money_notifies_once_and_normal_completion_does_not_replay_like_cpp()
 {
    for cancelled_waiter in [true, false] {
        let (mut session, send_rx) = make_session_with_send_capacity(16);
        let player_guid =
            ObjectGuid::create_player(1, if cancelled_waiter { 61_851 } else { 61_852 });
        let owner_guid = ObjectGuid::create_item(1, if cancelled_waiter { 61_853 } else { 61_854 });
        install_active_item_loot_completion_fixture_like_cpp(
            &mut session,
            player_guid,
            owner_guid,
            0,
        );

        if cancelled_waiter {
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
            let guard = session.begin_durable_item_loot_persistence_like_cpp();
            let worker = super::spawn_loot_claim_persistence_worker_like_cpp(
                async move {
                    let _ = started_tx.send(());
                    let _ = commit_rx.await;
                    Ok::<(), ()>(())
                },
                None,
                Some((
                    guard,
                    DurableItemLootCompletionLikeCpp {
                        owner_guid,
                        loot_list_id: 0,
                        player_guid,
                        item_owner_auto_release: false,
                        durable_item_money_applied_amount: Some(0),
                        durable_item_money_notified_amount: Some(0),
                        durable_item_money_balance_applied: Some(Arc::new(AtomicBool::new(false))),
                        item_fanout: None,
                        runtime_inventory_applied: Arc::new(AtomicBool::new(false)),
                    },
                )),
            )
            .unwrap();
            let waiter = tokio::spawn(async move { worker.await });
            started_rx.await.unwrap();
            waiter.abort();
            let _ = waiter.await;
            commit_tx.send(()).unwrap();
            session.wait_for_active_loot_persistence_like_cpp().await;
        } else {
            session.set_loot_money_persistence_test_result_like_cpp(true);
            session.handle_loot_money(loot_money_packet()).await;
            session.wait_for_active_loot_persistence_like_cpp().await;
        }

        let opcodes = drain_server_opcodes_like_cpp(&send_rx);
        assert_eq!(
            opcodes
                .iter()
                .filter(|opcode| {
                    **opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16
                })
                .count(),
            1,
            "zero money still notifies, while a normally published completion is not replayed"
        );
        session.wait_for_active_loot_persistence_like_cpp().await;
        assert!(drain_server_opcodes_like_cpp(&send_rx).is_empty());
    }
}

#[tokio::test]
async fn disconnect_waits_for_item_publication_and_releases_once_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_850);
    let owner_guid = ObjectGuid::create_item(1, 61_851);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 0);
    let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
    let guard = session.begin_durable_item_loot_persistence_like_cpp();
    let worker = super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = commit_rx.await;
            Ok::<(), ()>(())
        },
        None,
        Some((
            guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: true,
                durable_item_money_applied_amount: None,
                durable_item_money_notified_amount: None,
                durable_item_money_balance_applied: None,
                item_fanout: None,
                runtime_inventory_applied: Arc::new(AtomicBool::new(true)),
            },
        )),
    )
    .unwrap();
    let release_commit = tokio::spawn(async move {
        tokio::task::yield_now().await;
        commit_tx.send(()).unwrap();
        worker.await.unwrap().unwrap();
    });

    session
        .cleanup_shared_runtime_state_on_disconnect_like_cpp()
        .await;
    release_commit.await.unwrap();

    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        1
    );
}

#[tokio::test]
async fn disconnect_runs_full_creature_release_lifecycle_after_persistence_like_cpp() {
    let (mut session, _send_rx, _second, _second_rx, owner_guid, _player_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, false,
        ));
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    let before = session
        .mutate_world_creature(owner_guid, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.set_corpse_despawn_at(Some(Instant::now() + Duration::from_secs(120)));
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
            (
                creature.corpse_despawn_at(),
                creature.has_lootable_dynamic_flag_like_cpp(),
            )
        })
        .unwrap();
    assert!(before.1);

    session
        .cleanup_shared_runtime_state_on_disconnect_like_cpp()
        .await;

    let after = session
        .mutate_world_creature(owner_guid, |creature| {
            (
                creature.corpse_despawn_at(),
                creature.has_lootable_dynamic_flag_like_cpp(),
            )
        })
        .unwrap();
    assert!(!after.1);
    assert!(after.0 <= before.0);
}

#[tokio::test]
async fn remote_master_loot_command_transports_and_commits_claim_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };
    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);

    let request = first.request_represented_remote_master_loot_give_like_cpp(
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        entry,
        Some(claim),
    );
    let target = async {
        tokio::task::yield_now().await;
        second.process_represented_session_commands_like_cpp().await;
    };
    let (result, ()) = tokio::join!(request, target);

    assert_eq!(result, MasterLootGiveResult::Stored);
    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
}

#[tokio::test]
async fn remote_master_timeout_then_release_still_fans_out_and_finalizes_corpse_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    first.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    first
        .mutate_world_creature(owner, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
        })
        .unwrap();

    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };

    // The target already closed its own view. The source will close while
    // the detached target worker owns the claim as `Persisting`.
    second.handle_loot_release(loot_release_packet(owner)).await;
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&registry));
    second.set_player_registry(registry);

    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let commit_gate = Arc::new(tokio::sync::Notify::new());
    second.set_loot_item_store_test_commit_gate_like_cpp(Arc::clone(&commit_gate));

    let mut request = Box::pin(first.request_represented_remote_master_loot_give_like_cpp(
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        entry,
        Some(claim),
    ));
    let mut target = Box::pin(async {
        tokio::task::yield_now().await;
        second.process_represented_session_commands_like_cpp().await;
    });
    let result = tokio::select! {
        result = &mut request => result,
        _ = &mut target => panic!("target must remain behind the COMMIT gate"),
    };
    drop(request);
    assert_eq!(result, MasterLootGiveResult::TargetMismatch);

    first.handle_loot_release(loot_release_packet(owner)).await;
    commit_gate.notify_one();
    target.await;

    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
    assert!(
        drain_server_opcodes_like_cpp(&first_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRemoved as u16)),
        "the pre-COMMIT route survives request timeout and CMSG_LOOT_RELEASE"
    );
    assert!(
        !first
            .mutate_world_creature(owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "completion must run AllLootRemovedFromCorpse without an active view"
    );
}

#[tokio::test]
async fn remote_roll_winner_command_transports_and_commits_claim_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(second_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(second_guid, generation, 0, false, Some(second_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };
    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);

    let request = first.request_represented_remote_loot_roll_winner_store_like_cpp(
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        vec![entry],
        false,
        Some(claim),
    );
    let target = async {
        tokio::task::yield_now().await;
        second.process_represented_session_commands_like_cpp().await;
    };
    let (result, ()) = tokio::join!(request, target);

    assert_eq!(result, MasterLootGiveResult::Stored);
    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
}

#[tokio::test]
async fn detached_remote_claim_waits_for_every_authority_viewer_before_corpse_lifecycle_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    first
        .mutate_world_creature(owner, |creature| {
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
        })
        .unwrap();

    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(second_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(second_guid, generation, 0, false, Some(second_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };

    // The remote winner closes, while the original looter deliberately
    // keeps the same authoritative Loot window open.
    second.handle_loot_release(loot_release_packet(owner)).await;
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let player_registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    player_registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&player_registry));
    second.set_player_registry(player_registry);
    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);

    let request = first.request_represented_remote_loot_roll_winner_store_like_cpp(
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        vec![entry],
        false,
        Some(claim),
    );
    let target = async {
        tokio::task::yield_now().await;
        second.process_represented_session_commands_like_cpp().await;
    };
    let (result, ()) = tokio::join!(request, target);

    assert_eq!(result, MasterLootGiveResult::Stored);
    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert!(
        first
            .mutate_world_creature(owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "the detached winner cannot finish lifecycle while the original viewer remains open"
    );
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .players_looting,
        vec![first_guid]
    );

    first.handle_loot_release(loot_release_packet(owner)).await;
    assert!(
        !first
            .mutate_world_creature(owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "the final real viewer release performs the ordinary C++ corpse transition"
    );
}

#[tokio::test]
async fn remote_roll_timeout_then_release_fans_out_once_and_finalizes_corpse_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    first.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    first
        .mutate_world_creature(owner, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
        })
        .unwrap();

    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(second_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(second_guid, generation, 0, false, Some(second_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };

    // The target no longer has a live loot window. The source closes its
    // window only after the remote request times out with the target's
    // detached persistence worker still owning the claim.
    second.handle_loot_release(loot_release_packet(owner)).await;
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&registry));
    second.set_player_registry(registry);

    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let commit_gate = Arc::new(tokio::sync::Notify::new());
    second.set_loot_item_store_test_commit_gate_like_cpp(Arc::clone(&commit_gate));

    let mut request = Box::pin(
        first.request_represented_remote_loot_roll_winner_store_like_cpp(
            second_guid,
            owner,
            represented_loot_object_guid_like_cpp(owner),
            0,
            0,
            vec![entry],
            false,
            Some(claim),
        ),
    );
    let mut target = Box::pin(async {
        tokio::task::yield_now().await;
        second.process_represented_session_commands_like_cpp().await;
    });
    let result = tokio::select! {
        result = &mut request => result,
        _ = &mut target => panic!("target must remain behind the COMMIT gate"),
    };
    drop(request);
    assert_eq!(result, MasterLootGiveResult::TargetMismatch);

    first.handle_loot_release(loot_release_packet(owner)).await;
    commit_gate.notify_one();
    target.await;

    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken,
        "the roll claim is terminal after the durable commit"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&first_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRemoved as u16)
            .count(),
        1,
        "the pre-COMMIT route publishes the roll removal exactly once"
    );
    assert!(
        !first
            .mutate_world_creature(owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "post-COMMIT completion must finish the corpse lifecycle without an active view"
    );
}

#[tokio::test]
async fn local_disenchant_batch_commits_all_materials_and_original_claim_like_cpp() {
    let (mut session, rx, _second, _second_rx, owner, player_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&rx);
    let shared_send = session.send_tx().clone();
    session.install_realm_send_channel_for_test(shared_send);
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(player_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(player_guid, generation, 0, false, Some(player_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(player_guid, 0)
        .await
        .unwrap();
    let materials = represented_disenchant_test_outputs_like_cpp(player_guid, 700);
    let grants = Arc::new(AtomicUsize::new(0));
    session.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);

    assert!(
        session
            .store_direct_disenchant_batch_like_cpp(
                &materials,
                0,
                Some(&claim),
                Some(LootItemClaimCommitContextLikeCpp {
                    owner_guid: owner,
                    loot_obj: represented_loot_object_guid_like_cpp(owner),
                    loot_list_id: 0,
                    player_guid,
                    free_for_all: false,
                }),
            )
            .await
    );
    assert_eq!(grants.load(Ordering::SeqCst), 2);
    assert_eq!(
        drain_server_opcodes_like_cpp(&rx),
        vec![
            wow_constants::ServerOpcodes::ItemPushResult as u16,
            wow_constants::ServerOpcodes::ItemPushResult as u16,
            wow_constants::ServerOpcodes::LootRemoved as u16,
        ],
        "C++ Loot::AutoStore sends every material before the original roll slot is removed"
    );
    assert!(claim.is_committed_like_cpp());
    assert!(
        authority
            .reserve_item_for_award_like_cpp(player_guid, 0)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn failed_disenchant_batch_grants_zero_and_original_slot_retries_like_cpp() {
    let (mut session, _rx, _second, _second_rx, owner, player_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(player_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(player_guid, generation, 0, false, Some(player_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(player_guid, 0)
        .await
        .unwrap();
    let materials = represented_disenchant_test_outputs_like_cpp(player_guid, 700);
    let grants = Arc::new(AtomicUsize::new(0));
    // Both material grants are already planned when this seam rejects the
    // one transaction, modelling a failure while persisting its second
    // output. Runtime observes neither output.
    session.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), false);

    assert!(
        !session
            .store_direct_disenchant_batch_like_cpp(
                &materials,
                0,
                Some(&claim),
                Some(LootItemClaimCommitContextLikeCpp {
                    owner_guid: owner,
                    loot_obj: represented_loot_object_guid_like_cpp(owner),
                    loot_list_id: 0,
                    player_guid,
                    free_for_all: false,
                }),
            )
            .await
    );
    assert_eq!(grants.load(Ordering::SeqCst), 0);
    assert!(!claim.is_committed_like_cpp());
    drop(claim);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(player_guid, 0)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn remote_disenchant_batch_uses_one_command_and_commits_all_materials_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, _first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(second_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(second_guid, generation, 0, false, Some(second_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let materials = represented_disenchant_test_outputs_like_cpp(second_guid, 700);
    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, 700, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);

    let request = first.request_represented_remote_loot_roll_winner_store_like_cpp(
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        materials,
        true,
        Some(claim),
    );
    let target = async {
        tokio::task::yield_now().await;
        // One drain handles the complete two-material result. A former
        // implementation required one command/ack round-trip per item.
        second.process_represented_session_commands_like_cpp().await;
    };
    let (result, ()) = tokio::join!(request, target);

    assert_eq!(result, MasterLootGiveResult::Stored);
    assert_eq!(grants.load(Ordering::SeqCst), 2);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(second_guid, 0)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn remote_disenchant_timeout_then_release_fans_out_once_and_finalizes_corpse_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    first.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    first
        .mutate_world_creature(owner, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
        })
        .unwrap();

    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(second_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(second_guid, generation, 0, false, Some(second_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let materials = represented_disenchant_test_outputs_like_cpp(second_guid, 700);

    second.handle_loot_release(loot_release_packet(owner)).await;
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&registry));
    second.set_player_registry(registry);

    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, 700, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let commit_gate = Arc::new(tokio::sync::Notify::new());
    second.set_loot_item_store_test_commit_gate_like_cpp(Arc::clone(&commit_gate));

    let mut request = Box::pin(
        first.request_represented_remote_loot_roll_winner_store_like_cpp(
            second_guid,
            owner,
            represented_loot_object_guid_like_cpp(owner),
            0,
            0,
            materials,
            true,
            Some(claim),
        ),
    );
    let mut target = Box::pin(async {
        tokio::task::yield_now().await;
        second.process_represented_session_commands_like_cpp().await;
    });
    let result = tokio::select! {
        result = &mut request => result,
        _ = &mut target => panic!("target must remain behind the COMMIT gate"),
    };
    drop(request);
    assert_eq!(result, MasterLootGiveResult::TargetMismatch);

    first.handle_loot_release(loot_release_packet(owner)).await;
    commit_gate.notify_one();
    target.await;

    assert_eq!(grants.load(Ordering::SeqCst), 2);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(second_guid, 0)
            .await
            .is_err(),
        "the original roll claim remains terminal after every material commits"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&first_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRemoved as u16)
            .count(),
        1,
        "the material batch publishes the original roll removal exactly once"
    );
    assert!(
        !first
            .mutate_world_creature(owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "post-COMMIT completion must finish the corpse lifecycle without an active view"
    );
}

#[tokio::test]
async fn cancelled_disenchant_waiter_cannot_reopen_durable_batch_like_cpp() {
    let (mut session, _rx, _second, _second_rx, owner, player_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(player_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(player_guid, generation, 0, false, Some(player_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(player_guid, 0)
        .await
        .unwrap();
    let durable_materials = Arc::new(AtomicUsize::new(0));
    let durable_materials_worker = Arc::clone(&durable_materials);
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let worker = super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = release_rx.await;
            durable_materials_worker.fetch_add(2, Ordering::SeqCst);
            Ok::<(), ()>(())
        },
        Some(claim),
        None,
    )
    .unwrap();
    let waiter = tokio::spawn(async move { worker.await });

    started_rx.await.unwrap();
    waiter.abort();
    let _ = waiter.await;
    release_tx.send(()).unwrap();
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }

    assert_eq!(durable_materials.load(Ordering::SeqCst), 2);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(player_guid, 0)
            .await
            .is_err()
    );
}

fn tap_test_creature_like_cpp(
    session: &mut WorldSession,
    creature_guid: ObjectGuid,
    player_guid: ObjectGuid,
) {
    let _ = session.mutate_world_creature(creature_guid, |world_creature| {
        world_creature
            .creature
            .set_tapped_by_player(player_guid, &[]);
    });
}

fn loot_response_failure_reason(sent: &[u8]) -> u8 {
    loot_response_failure_reason_and_threshold(sent).0
}

fn loot_response_threshold(sent: &[u8]) -> u8 {
    loot_response_failure_reason_and_threshold(sent).1
}

fn loot_response_failure_reason_and_threshold(sent: &[u8]) -> (u8, u8) {
    let mut pkt = WorldPacket::from_bytes(&sent[2..]);
    let _owner = pkt.read_packed_guid().unwrap();
    let _loot_obj = pkt.read_packed_guid().unwrap();
    let failure_reason = pkt.read_uint8().unwrap();
    let _acquire_reason = pkt.read_uint8().unwrap();
    let _loot_method = pkt.read_uint8().unwrap();
    let threshold = pkt.read_uint8().unwrap();
    (failure_reason, threshold)
}

fn test_creature_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, counter)
}

fn test_corpse_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 0, 0, 1, counter)
}

fn broadcast_info(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
) -> PlayerSessionRegistrationLikeCpp {
    let (command_tx, _command_rx) = flume::bounded(1);
    PlayerSessionRegistrationLikeCpp {
        identity: PlayerDirectoryIdentityLikeCpp {
            player_name: format!("Player{}", guid.counter()),
            account_id: guid.counter() as u32,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
        },
        placement: PlayerDirectoryPlacementLikeCpp {
            map_id: 0,
            instance_id: 0,
        },
        info: PlayerBroadcastInfo {
            position: Position::ZERO,
            combat_reach: 0.0,
            liquid_status: 0,
            is_in_world: true,
            active_loot_rolls: Vec::new(),
            in_combat: false,
            pass_on_group_loot: false,
            enchanting_skill: 0,
            is_alive: true,
            transport: None,
            is_afk: false,
            is_dnd: false,
            in_vehicle: false,
            has_vehicle_kit_like_cpp: false,
            party_member_vehicle_seat: 0,
            zone_id: 0,
            spec_id: 0,
            unit_flags: 0,
            unit_state: 0,
            is_game_master: false,
            dungeon_difficulty_id: 1,
            active_expansion: 2,
            pending_quest_sharing: None,
            known_spells: Vec::new(),
            active_quest_statuses: Default::default(),
            active_quest_objective_counts: Default::default(),
            rewarded_quests: Default::default(),
            completed_achievements: Default::default(),
            daily_quests_completed: Default::default(),
            df_quests: Default::default(),
            faction_template_id: 0,
            forced_reputation_ranks: Vec::new(),
            inventory_item_counts: Default::default(),
            party_member_phase_states: Default::default(),
            party_member_auras: Vec::new(),
            party_member_pet_stats: None,
            level: 1,
            gray_level: 0,
            display_id: 49,
            visible_items: std::sync::Arc::new([(0, 0, 0); 19]),
            customizations: std::sync::Arc::default(),
        },
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

fn loot_condition(
    condition_type_or_reference: i32,
    value1: u32,
    value2: u32,
    value3: u32,
) -> LootConditionRowLikeCpp {
    LootConditionRowLikeCpp {
        else_group: 0,
        condition_type_or_reference,
        condition_target: 0,
        value1,
        value2,
        value3,
        string_value1: String::new(),
        negative: false,
        script_name: String::new(),
    }
}

fn test_quest_template(id: u32) -> QuestTemplate {
    QuestTemplate {
        id,
        quest_type: 0,
        quest_level: 1,
        quest_max_scaling_level: 0,
        quest_package_id: 0,
        min_level: 1,
        quest_sort_id: 0,
        quest_info_id: 0,
        suggested_group_num: 0,
        reward_next_quest: 0,
        reward_xp_difficulty: 0,
        reward_xp_multiplier: 1.0,
        reward_money_difficulty: 0,
        reward_money_multiplier: 1.0,
        reward_bonus_money: 0,
        reward_display_spell: [0; 3],
        reward_spell: 0,
        reward_honor: 0,
        reward_title_id: 0,
        reward_skill_line_id: 0,
        reward_skill_points: 0,
        reward_mail_template_id: 0,
        reward_mail_delay_secs: 0,
        reward_mail_sender_entry: 0,
        reward_faction_ids: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_values: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_overrides: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_cap_in: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_flags: 0,
        source_item_id: 0,
        source_item_count: 0,
        source_spell_id: 0,
        limit_time_secs: 0,
        expansion: 0,
        flags: 0,
        flags_ex: 0,
        flags_ex2: 0,
        special_flags: 0,
        event_id_for_quest: 0,
        reward_items: [0; 4],
        reward_amounts: [0; 4],
        reward_currencies: [0; 4],
        reward_currency_amounts: [0; 4],
        item_drop: [0; 4],
        item_drop_quantity: [0; 4],
        log_title: String::new(),
        log_description: String::new(),
        quest_description: String::new(),
        area_description: String::new(),
        quest_completion_log: String::new(),
        objectives: Vec::new(),
        allowable_races: 0,
        allowable_classes: 0,
        max_level: 0,
        prev_quest_id: 0,
        next_quest_id: 0,
        exclusive_group: 0,
        breadcrumb_for_quest_id: 0,
        dependent_previous_quests: Vec::new(),
        dependent_breadcrumb_quests: Vec::new(),
        required_min_rep_faction: 0,
        required_min_rep_value: 0,
        required_max_rep_faction: 0,
        required_max_rep_value: 0,
        required_skill_id: 0,
        required_skill_points: 0,
        reward_choice_items: [(0, 0); 6],
        reward_choice_item_types: [0; 6],
    }
}

#[test]
fn represented_personal_loot_remote_context_uses_registry_fields_like_cpp() {
    let (session, _) = make_session_with_send_capacity(1);
    let remote_context = RepresentedLootPlayerContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 80,
        known_spells: Vec::new(),
        active_quest_statuses: HashMap::new(),
        active_quest_objective_counts: HashMap::new(),
        rewarded_quests: HashSet::new(),
        inventory_item_counts: HashMap::new(),
        is_current: false,
    };

    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(6, 469, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(15, 1, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(16, 1, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(20, 0, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(27, 70, 3, 0),
            &remote_context,
        ),
        Some(true)
    );
}

#[test]
fn represented_personal_loot_remote_quest_and_spell_conditions_use_registry_like_cpp() {
    let (session, _) = make_session_with_send_capacity(1);
    let mut active_quest_statuses = HashMap::new();
    active_quest_statuses.insert(100, crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    active_quest_statuses.insert(200, crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP);
    let mut rewarded_quests = HashSet::new();
    rewarded_quests.insert(300);
    let remote_context = RepresentedLootPlayerContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 80,
        known_spells: vec![12_345],
        active_quest_statuses,
        active_quest_objective_counts: HashMap::new(),
        rewarded_quests,
        inventory_item_counts: HashMap::new(),
        is_current: false,
    };

    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(9, 100, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(28, 200, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(8, 300, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(14, 400, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        remote_context.quest_status(300),
        QUEST_STATUS_REWARDED_LIKE_CPP
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(14, 300, 0, 0),
            &remote_context,
        ),
        Some(false),
        "C++ Player::GetQuestStatus returns REWARDED before QUEST_STATUS_NONE"
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(25, 12_345, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(47, 100, 0x08, 0),
            &remote_context,
        ),
        Some(true)
    );
}

#[test]
fn represented_personal_loot_remote_inventory_and_objective_conditions_use_registry_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    let mut quest_store = QuestStore::new();
    let mut quest = test_quest_template(100);
    quest.objectives.push(QuestObjective {
        id: 11,
        quest_id: 100,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: 7001,
        amount: 7,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest_store.quests.insert(100, quest);
    session.set_quest_store(Arc::new(quest_store));
    let mut active_quest_statuses = HashMap::new();
    active_quest_statuses.insert(100, crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    let mut active_quest_objective_counts = HashMap::new();
    active_quest_objective_counts.insert(100, vec![5]);
    let mut inventory_item_counts = HashMap::new();
    inventory_item_counts.insert(9001, 2);
    let remote_context = RepresentedLootPlayerContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 80,
        known_spells: Vec::new(),
        active_quest_statuses,
        active_quest_objective_counts,
        rewarded_quests: HashSet::new(),
        inventory_item_counts,
        is_current: false,
    };

    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(2, 9001, 2, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(2, 9001, 3, 0),
            &remote_context,
        ),
        Some(false)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(2, 9001, 2, 1),
            &remote_context,
        ),
        None
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(48, 11, 0, 5),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(48, 11, 0, 4),
            &remote_context,
        ),
        Some(false)
    );
}

#[test]
fn represented_personal_loot_remote_has_quest_for_item_objective_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    install_limited_test_item_template(&mut session, 7001, 0);
    let mut quest_store = QuestStore::new();
    let mut quest = test_quest_template(100);
    quest.objectives.push(QuestObjective {
        id: 1,
        quest_id: 100,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: 7001,
        amount: 3,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest_store.quests.insert(100, quest);
    session.set_quest_store(Arc::new(quest_store));

    let mut active_quest_statuses = HashMap::new();
    active_quest_statuses.insert(100, crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    let mut active_quest_objective_counts = HashMap::new();
    active_quest_objective_counts.insert(100, vec![2]);
    let mut remote_context = RepresentedLootPlayerContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 80,
        known_spells: Vec::new(),
        active_quest_statuses,
        active_quest_objective_counts,
        rewarded_quests: HashSet::new(),
        inventory_item_counts: HashMap::new(),
        is_current: false,
    };

    assert!(session.item_loot_quest_status_allows_for_player_like_cpp(
        7001,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
        &remote_context,
    ));

    remote_context
        .active_quest_objective_counts
        .insert(100, vec![3]);
    assert!(!session.item_loot_quest_status_allows_for_player_like_cpp(
        7001,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
        &remote_context,
    ));
}

#[test]
fn represented_personal_loot_remote_has_quest_for_item_drop_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    install_limited_test_item_template(&mut session, 7002, 0);
    let mut quest_store = QuestStore::new();
    let mut quest = test_quest_template(200);
    quest.item_drop[0] = 7002;
    quest.item_drop_quantity[0] = 4;
    quest_store.quests.insert(200, quest);
    session.set_quest_store(Arc::new(quest_store));

    let mut active_quest_statuses = HashMap::new();
    active_quest_statuses.insert(200, crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    let mut inventory_item_counts = HashMap::new();
    inventory_item_counts.insert(7002, 3);
    let mut remote_context = RepresentedLootPlayerContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 80,
        known_spells: Vec::new(),
        active_quest_statuses,
        active_quest_objective_counts: HashMap::new(),
        rewarded_quests: HashSet::new(),
        inventory_item_counts,
        is_current: false,
    };

    assert!(session.item_loot_quest_status_allows_for_player_like_cpp(
        7002,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
        &remote_context,
    ));

    remote_context.inventory_item_counts.insert(7002, 4);
    assert!(!session.item_loot_quest_status_allows_for_player_like_cpp(
        7002,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
        &remote_context,
    ));
}

#[tokio::test]
async fn loot_item_added_progresses_incomplete_quest_item_objective_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let quest_id = 8_336;
    let item_id = 20_482;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 6,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    assert!(session.item_loot_quest_status_allows_like_cpp(
        item_id,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
    ));

    let changed_quest_ids = session
        .apply_quest_source_item_added_non_bound_objective_progress_like_cpp(item_id, 0, 3)
        .await;

    assert_eq!(changed_quest_ids, vec![quest_id]);
    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .expect("quest progress should remain active")
            .objective_counts,
        vec![3]
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ UpdateQuestObjectiveProgress suppresses generic credit packets for ITEM objectives"
    );
}

#[test]
fn banked_quest_item_recomputes_objective_and_reopens_quest_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 1, 0);
    let quest_id = 8_338;
    let item_id = 20_484;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 3,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![3],
            slot: 0,
        },
    );

    let planned = session.plan_bank_item_quest_persistence_like_cpp(item_id, 0, true, 0, 0);
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].objective_counts, vec![0]);
    assert_eq!(
        planned[0].status,
        crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    assert_eq!(
        session.apply_quest_item_removed_like_cpp(item_id),
        vec![quest_id]
    );
    let status = session.player_quests.get(&quest_id).expect("active quest");
    assert_eq!(
        status.status,
        crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    assert_eq!(status.objective_counts, vec![0]);
    let update = send_rx
        .try_recv()
        .expect("banking a quest item should update the quest-log slot");
    assert_eq!(
        WorldPacket::from_bytes(&update).server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateObject)
    );
}

#[tokio::test]
async fn withdrawn_banked_item_restores_bound_objective_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let quest_id = 8_339;
    let item_id = 20_485;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 1,
        flags: 0,
        flags2: 1,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    let planned = session.plan_bank_item_quest_persistence_like_cpp(item_id, 0, false, 0, 1);
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].objective_counts, vec![1]);
    assert_eq!(
        planned[0].status,
        crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    let changed_quest_ids = session
        .apply_quest_item_added_objective_progress_like_cpp(item_id, 0, 1)
        .await;

    assert_eq!(changed_quest_ids, vec![quest_id]);
    let status = session.player_quests.get(&quest_id).expect("active quest");
    assert_eq!(status.objective_counts, vec![1]);
    assert_eq!(
        status.status,
        crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
    );
}

#[tokio::test]
async fn loot_item_eligibility_does_not_treat_complete_quest_as_incomplete_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    let quest_id = 8_337;
    let item_id = 20_483;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    assert!(!session.has_incomplete_quest_objective_for_item_like_cpp(item_id));
    assert!(!session.item_loot_quest_status_allows_like_cpp(
        item_id,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
    ));

    let changed_quest_ids = session
        .apply_quest_source_item_added_non_bound_objective_progress_like_cpp(item_id, 0, 1)
        .await;

    assert!(changed_quest_ids.is_empty());
    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .expect("complete quest should not progress as incomplete")
            .objective_counts,
        vec![0]
    );
}

#[tokio::test]
async fn quest_required_creature_loot_is_not_generated_after_completion_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 8_336;
    let item_id = 20_482;
    let loot_id = 15_274;
    session.set_player_guid(Some(player_guid));
    install_limited_test_item_template(&mut session, item_id, 0);
    install_quest_bound_loot_objective_like_cpp(&mut session, quest_id, item_id, 6, 6);
    session.player_quests.get_mut(&quest_id).unwrap().status =
        crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP;

    let mut creature_store = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    creature_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: loot_id,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: true,
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Creature, creature_store);
    session.set_loot_stores(Arc::new(stores));

    let complete_loot = session
        .generate_represented_creature_loot_items_for_player_like_cpp(loot_id, player_guid)
        .await
        .unwrap();
    assert!(
        complete_loot.is_empty(),
        "C++ LootItem::AllowedForPlayer rejects QuestRequired items after HasQuestForItem becomes false"
    );

    let status = session.player_quests.get_mut(&quest_id).unwrap();
    status.status = crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
    status.objective_counts[0] = 5;
    let incomplete_loot = session
        .generate_represented_creature_loot_items_for_player_like_cpp(loot_id, player_guid)
        .await
        .unwrap();
    assert_eq!(incomplete_loot.len(), 1);
    assert!(incomplete_loot[0].flags.needs_quest);
}

fn install_master_loot_group(
    session: &mut WorldSession,
    master_guid: ObjectGuid,
    candidate_guid: ObjectGuid,
) {
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.add_member(candidate_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
}

fn install_group_loot_group(
    session: &mut WorldSession,
    leader_guid: ObjectGuid,
    candidate_guid: ObjectGuid,
) {
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader_guid);
    group.add_member(candidate_guid);
    group.loot_method = LOOT_METHOD_GROUP_LIKE_CPP;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
}

fn generation_guarded_group_loot_like_cpp(
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
    candidate_guid: ObjectGuid,
) -> CreatureLoot {
    CreatureLoot {
        loot_guid: represented_loot_object_guid_like_cpp(owner_guid),
        coins: 0,
        unlooted_count: 1,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player_guid, candidate_guid],
        items: vec![LootEntry {
            loot_list_id: 0,
            item_id: 25,
            quantity: 1,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags: LootEntryFlags {
                follow_loot_rules: true,
                blocked: true,
                ..Default::default()
            },
            allowed_looters: vec![player_guid, candidate_guid],
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        }],
        looted_by_player: false,
    }
}

async fn open_generation_guarded_group_roll_like_cpp(
    spawn_id: i64,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
    ObjectGuid,
    ObjectGuid,
) {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(spawn_id);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(16);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        generation_guarded_group_loot_like_cpp(owner_guid, player_guid, candidate_guid),
    );
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);

    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    while send_rx.try_recv().is_ok() {}
    while candidate_rx.try_recv().is_ok() {}

    let state = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .expect("first loot generation should start the group roll");
    assert_eq!(state.owner_guid, owner_guid);
    assert_eq!(
        state.authority_generation,
        session
            .represented_loot_cache_generations_like_cpp
            .get(&owner_guid)
            .copied()
            .expect("opened loot cache should be generation-tagged")
    );

    (
        session,
        send_rx,
        candidate_rx,
        player_guid,
        candidate_guid,
        owner_guid,
    )
}

fn replace_generation_guarded_group_loot_like_cpp(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
    candidate_guid: ObjectGuid,
) -> u64 {
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .expect("test creature should expose its object-owned loot authority");
    let previous_generation = authority.generation_like_cpp();
    let retired_generation = authority.retire_like_cpp();
    let replacement =
        generation_guarded_group_loot_like_cpp(owner_guid, player_guid, candidate_guid);
    let replacement_generation = authority
        .replace_retired_generation_like_cpp(retired_generation, Some(replacement), HashMap::new())
        .expect("explicit test generation replaces the observed retired lifetime");
    assert!(replacement_generation > previous_generation);
    replacement_generation
}

fn install_limited_test_item_template(session: &mut WorldSession, entry: u32, max_count: i32) {
    install_limited_test_item_template_with_flags2(session, entry, max_count, 0);
}

fn install_limited_test_item_template_with_flags2(
    session: &mut WorldSession,
    entry: u32,
    max_count: i32,
    flags2: u32,
) {
    install_limited_test_item_template_with_flags2_and_bonding(
        session,
        entry,
        max_count,
        flags2,
        ItemBondingType::None,
    );
}

fn install_limited_test_item_template_with_flags2_and_bonding(
    session: &mut WorldSession,
    entry: u32,
    max_count: i32,
    flags2: u32,
    bonding: ItemBondingType,
) {
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: entry,
        class_id: ItemClass::Consumable as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        entry,
        ItemSparseTemplateEntry {
            flags: [0, flags2, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: 20,
            max_count,
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
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: bonding as u8,
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
        },
    )])));
}

fn install_disenchantable_test_item_template(session: &mut WorldSession, entry: u32) {
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: entry,
        class_id: ItemClass::Armor as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::Chest as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                entry,
                ItemSparseTemplateEntry {
                    flags: [0, 0, 0, 0],
                    bag_family: 0,
                    start_quest_id: 0,
                    stackable: 1,
                    max_count: 0,
                    lock_id: 0,
                    required_reputation_rank: 0,
                    sell_price: 1,
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
                    zone_bound: [0, 0],
                    required_reputation_faction: 0,
                    allowable_class: -1,
                    required_expansion: 0,
                    bonding: ItemBondingType::None as u8,
                    container_slots: 0,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
            [(
                entry,
                ItemRandomPropertyTemplateEntry {
                    item_level: 10,
                    quality: ItemQuality::Rare as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_item_disenchant_loot_store(Arc::new(ItemDisenchantLootStore::from_entries([
        ItemDisenchantLootEntry {
            id: 901,
            subclass: 0,
            quality: ItemQuality::Rare as u8,
            min_level: 1,
            max_level: 20,
            skill_required: 175,
            expansion_id: -2,
            class_id: ItemClass::Armor as u32,
        },
    ])));
}

fn install_active_spell_cast(session: &mut WorldSession, player_guid: ObjectGuid) {
    session.active_spell_cast = Some(SpellCastState {
        spell_id: 133,
        target_guid: player_guid,
        target_data: wow_packet::packets::spell::SpellTargetData {
            flags: 0x2,
            unit: player_guid,
            ..Default::default()
        },
        cast_id: ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 7),
        cast_start_time: std::time::Instant::now(),
        cast_time_ms: 30_000,
        spell_visual: wow_packet::packets::spell::SpellCastVisual {
            spell_visual_id: 1,
            script_visual_id: 0,
        },
        metadata: crate::session::SpellCastMetadata::default(),
    });
}

fn install_visible_aura_with_interrupt_flags(
    session: &mut WorldSession,
    slot: u8,
    spell_id: i32,
    caster_guid: ObjectGuid,
    aura_interrupt_flags: u32,
) {
    session.visible_auras.insert(
        slot,
        AuraApplication {
            spell_id,
            difficulty_id: 0,
            caster_guid,
            slot,
            duration_total: 30_000,
            duration_remaining: 30_000,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 0x0000_0001,
            aura_interrupt_flags,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        },
    );
}

#[tokio::test]
async fn represented_creature_money_uses_cpp_money_drop_rate() {
    let mut session = make_session();
    let owner_guid = test_creature_guid(1);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        money: 2.5,
        ..LootDropRatesLikeCpp::default()
    });

    let loot = session
        .generate_represented_creature_loot_like_cpp(
            owner_guid,
            ObjectGuid::create_player(1, 42),
            10,
            25,
            0,
            100,
            100,
            0,
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.coins, 250);
    assert!(loot.items.is_empty());
}

#[tokio::test]
async fn represented_creature_money_zero_gold_max_stays_zero_like_cpp() {
    let mut session = make_session();
    let owner_guid = test_creature_guid(1);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);

    let loot = session
        .generate_represented_creature_loot_like_cpp(
            owner_guid,
            ObjectGuid::create_player(1, 42),
            10,
            25,
            0,
            0,
            0,
            0,
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.coins, 0);
    assert!(loot.items.is_empty());
}

#[tokio::test]
async fn loot_response_success_keeps_cpp_failure_and_threshold_defaults() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(19_118);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(creature_guid, false));

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.loot_method = LOOT_METHOD_GROUP_LIKE_CPP;
    group.loot_threshold = 4;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session.loot_table.insert(
        creature_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(creature_guid),
            coins: 1,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: player_guid,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    install_cached_test_creature_loot_authority_like_cpp(&mut session, creature_guid, player_guid);

    let response = session
        .represented_loot_response_for_owner_like_cpp(creature_guid, player_guid, false)
        .await
        .unwrap();

    assert_eq!(response.loot_method, LOOT_METHOD_GROUP_LIKE_CPP);
    assert_eq!(
        response.failure_reason,
        LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP
    );
    assert_eq!(response.threshold, LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP);
}

#[tokio::test]
async fn loot_error_response_keeps_cpp_threshold_default_like_cpp() {
    let (session, send_rx) = make_session_with_send();
    let owner = test_creature_guid(19_119);
    let loot_obj = represented_loot_object_guid_like_cpp(owner);

    session.send_loot_error_like_cpp(loot_obj, owner, LOOT_ERROR_TOO_FAR_LIKE_CPP);

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        loot_response_failure_reason(&sent),
        LOOT_ERROR_TOO_FAR_LIKE_CPP
    );
    assert_eq!(
        loot_response_threshold(&sent),
        LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP
    );
}

#[tokio::test]
async fn represented_creature_loot_generation_carries_cpp_dungeon_encounter_id() {
    let mut session = make_session();
    let owner_guid = test_creature_guid(19_097);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);

    let loot = session
        .generate_represented_creature_loot_like_cpp(
            owner_guid,
            ObjectGuid::create_player(1, 42),
            10,
            25,
            0,
            0,
            0,
            615,
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.dungeon_encounter_id, 615);
}

struct OverworldPersonalLootTestFixtureLikeCpp {
    session: WorldSession,
    owner_guid: ObjectGuid,
    first_tapper: ObjectGuid,
    second_tapper: ObjectGuid,
    disconnected_tapper: ObjectGuid,
    normal_item_id: u32,
    alliance_item_id: u32,
}

fn overworld_personal_loot_test_fixture_like_cpp() -> OverworldPersonalLootTestFixtureLikeCpp {
    let mut session = make_session();
    let first_tapper = ObjectGuid::create_player(1, 42);
    let second_tapper = ObjectGuid::create_player(1, 43);
    let disconnected_tapper = ObjectGuid::create_player(1, 44);
    let owner_guid = test_creature_guid(19_098);
    let loot_id = 90_001;
    let normal_item_id = 80_101;
    let alliance_item_id = 80_102;

    session.set_player_guid(Some(first_tapper));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 10, 0);
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

    let registry = Arc::new(PlayerRegistry::default());
    let (second_tx, _second_rx) = flume::bounded(1);
    let mut second = broadcast_info(second_tapper, second_tx);
    second.identity.race = 2;
    registry.register_or_replace(second_tapper, second, Default::default());
    let (disconnected_tx, _disconnected_rx) = flume::bounded(1);
    let mut disconnected = broadcast_info(disconnected_tapper, disconnected_tx);
    disconnected.info.is_in_world = false;
    registry.register_or_replace(disconnected_tapper, disconnected, Default::default());
    session.set_player_registry(registry);

    let item_record = |id| ItemRecord {
        id,
        class_id: ItemClass::Consumable as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    };
    let sparse_template = |flags2| ItemSparseTemplateEntry {
        flags: [0, flags2, 0, 0],
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
        zone_bound: [0, 0],
        required_reputation_faction: 0,
        allowable_class: -1,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: 0,
        inventory_type: InventoryType::NonEquip as i8,
    };
    session.set_item_store(Arc::new(ItemStore::from_records([
        item_record(normal_item_id),
        item_record(alliance_item_id),
    ])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (normal_item_id, sparse_template(0)),
        (
            alliance_item_id,
            sparse_template(ItemFlags2::FactionAlliance as u32),
        ),
    ])));

    let mut creature_store = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    creature_store
        .load_rows_like_cpp(
            [normal_item_id, alliance_item_id].map(|item_id| LootTemplateRow {
                entry: loot_id,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }),
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Creature, creature_store);
    session.set_loot_stores(Arc::new(stores));

    let mut creature = test_creature(owner_guid, false);
    creature.entry = 9_001;
    creature.level = 10;
    creature.loot_id = loot_id;
    creature.gold_min = 7;
    creature.gold_max = 7;
    register_test_creature_like_cpp(&mut session, creature);
    session.mutate_world_creature(owner_guid, |world_creature| {
        world_creature
            .creature
            .set_tapped_by_player(disconnected_tapper, &[first_tapper, second_tapper]);
    });
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);

    OverworldPersonalLootTestFixtureLikeCpp {
        session,
        owner_guid,
        first_tapper,
        second_tapper,
        disconnected_tapper,
        normal_item_id,
        alliance_item_id,
    }
}

fn assert_overworld_personal_loot_generation_like_cpp(
    authority: &OwnedLootAuthority,
    fixture: &OverworldPersonalLootTestFixtureLikeCpp,
) -> (u8, u8) {
    assert!(authority.shared_snapshot_like_cpp().is_none());
    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 2);
    assert!(!personal.contains_key(&fixture.disconnected_tapper));

    let first = &personal[&fixture.first_tapper].loot;
    let second = &personal[&fixture.second_tapper].loot;
    assert_ne!(first.loot_guid, second.loot_guid);
    for (tapper, loot) in [
        (fixture.first_tapper, first),
        (fixture.second_tapper, second),
    ] {
        assert_eq!(loot.loot_guid.high_type(), HighGuid::LootObject);
        assert_eq!(loot.coins, 7);
        assert_eq!(loot.loot_method, 0);
        assert_eq!(loot.allowed_looters, vec![tapper]);
        assert!(
            loot.items
                .iter()
                .all(|item| item.allowed_looters == vec![tapper])
        );
    }
    assert!(
        first
            .items
            .iter()
            .any(|item| item.item_id == fixture.normal_item_id)
    );
    assert!(
        first
            .items
            .iter()
            .any(|item| item.item_id == fixture.alliance_item_id)
    );
    assert_eq!(
        second
            .items
            .iter()
            .map(|item| item.item_id)
            .collect::<Vec<_>>(),
        vec![fixture.normal_item_id],
        "FillLoot eligibility must run for the Horde tapper instead of cloning the first pool"
    );

    let first_normal_slot = first
        .items
        .iter()
        .find(|item| item.item_id == fixture.normal_item_id)
        .unwrap()
        .loot_list_id;
    (first_normal_slot, second.items[0].loot_list_id)
}

async fn assert_overworld_personal_loot_claims_are_independent_like_cpp(
    authority: &OwnedLootAuthority,
    first_tapper: ObjectGuid,
    first_normal_slot: u8,
    second_tapper: ObjectGuid,
    second_normal_slot: u8,
) {
    assert!(
        authority
            .reserve_item_like_cpp(first_tapper, first_normal_slot)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap()
    );
    assert!(
        !authority
            .personal_snapshot_like_cpp(second_tapper)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
    assert!(
        authority
            .reserve_money_like_cpp(first_tapper)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap()
    );
    assert_eq!(
        authority
            .personal_snapshot_like_cpp(second_tapper)
            .unwrap()
            .loot
            .coins,
        7
    );
    assert!(
        authority
            .reserve_item_like_cpp(second_tapper, second_normal_slot)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap()
    );
    assert!(
        authority
            .reserve_money_like_cpp(second_tapper)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap()
    );
}

#[tokio::test]
async fn overworld_creature_builds_independent_personal_loot_per_connected_tapper_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();

    fixture
        .session
        .ensure_represented_creature_kill_loot_like_cpp(fixture.owner_guid)
        .await;

    let authority = fixture
        .session
        .represented_owned_loot_authority_like_cpp(fixture.owner_guid)
        .expect("the dead creature keeps its object-owned loot authority");
    let (first_normal_slot, second_normal_slot) =
        assert_overworld_personal_loot_generation_like_cpp(&authority, &fixture);
    assert_overworld_personal_loot_claims_are_independent_like_cpp(
        &authority,
        fixture.first_tapper,
        first_normal_slot,
        fixture.second_tapper,
        second_normal_slot,
    )
    .await;
}

#[tokio::test]
async fn dungeon_encounter_builds_independent_unlocked_personal_pools_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    fixture
        .session
        .set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 0,
                instance_type: wow_data::map::MAP_INSTANCE,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
    let encounter_id = 733;
    fixture
        .session
        .mutate_world_creature(fixture.owner_guid, |creature| {
            creature.creature.ai_ownership_mut().dungeon_encounter_id = encounter_id;
        });
    fixture
        .session
        .represented_locked_dungeon_encounters
        .insert((fixture.second_tapper, encounter_id));

    fixture
        .session
        .ensure_represented_creature_kill_loot_like_cpp(fixture.owner_guid)
        .await;

    let authority = fixture
        .session
        .represented_owned_loot_authority_like_cpp(fixture.owner_guid)
        .unwrap();
    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 1);
    assert!(personal.contains_key(&fixture.first_tapper));
    assert!(!personal.contains_key(&fixture.second_tapper));
    assert!(!personal.contains_key(&fixture.disconnected_tapper));
    let first = &personal[&fixture.first_tapper].loot;
    assert_eq!(first.dungeon_encounter_id, encounter_id);
    assert_eq!(first.allowed_looters, vec![fixture.first_tapper]);
}

#[tokio::test]
async fn dungeon_trash_builds_one_personal_pool_for_selected_group_looter_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    fixture
        .session
        .set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 0,
                instance_type: wow_data::map::MAP_INSTANCE,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
    let groups = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(fixture.first_tapper);
    group.add_member(fixture.second_tapper);
    group.looter_guid = fixture.second_tapper;
    let group_guid = group.group_guid;
    groups.register_group_like_cpp(group_guid, group);
    fixture.session.group_guid = Some(group_guid);
    fixture
        .session
        .set_group_registry(Arc::clone(&groups), Arc::new(PendingInvites::default()));

    fixture
        .session
        .ensure_represented_creature_kill_loot_like_cpp(fixture.owner_guid)
        .await;

    let authority = fixture
        .session
        .represented_owned_loot_authority_like_cpp(fixture.owner_guid)
        .unwrap();
    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 1);
    assert!(!personal.contains_key(&fixture.first_tapper));
    let selected = &personal[&fixture.second_tapper].loot;
    assert_eq!(selected.dungeon_encounter_id, 0);
    assert_eq!(selected.allowed_looters, vec![fixture.second_tapper]);
    assert!(
        selected
            .items
            .iter()
            .all(|entry| { entry.allowed_looters == vec![fixture.second_tapper] })
    );
    assert_eq!(
        groups.get(&group_guid).unwrap().looter_guid_like_cpp(),
        fixture.first_tapper,
        "non-empty dungeon trash advances the round-robin group looter"
    );
}

#[tokio::test]
async fn cmsg_loot_unit_never_regenerates_after_creature_clear_loot_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    fixture
        .session
        .ensure_represented_creature_kill_loot_like_cpp(fixture.owner_guid)
        .await;
    let authority = fixture
        .session
        .represented_owned_loot_authority_like_cpp(fixture.owner_guid)
        .unwrap();
    fixture
        .session
        .mutate_world_creature(fixture.owner_guid, |creature| {
            creature.creature.clear_loot_like_cpp();
        });
    let retired_generation = authority.generation_like_cpp();

    assert!(
        fixture
            .session
            .represented_loot_response_for_owner_like_cpp(
                fixture.owner_guid,
                fixture.first_tapper,
                false,
            )
            .await
            .is_none()
    );
    assert!(authority.is_retired_like_cpp());
    assert_eq!(authority.generation_like_cpp(), retired_generation);
    assert!(
        authority
            .snapshot_for_player_like_cpp(fixture.first_tapper)
            .is_none()
    );
}

#[test]
fn stale_kill_generator_cannot_install_after_creature_lifecycle_aba_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    let authority = fixture
        .session
        .represented_owned_loot_authority_like_cpp(fixture.owner_guid)
        .unwrap();
    let expected_generation = authority.generation_like_cpp();
    let expected_revision = fixture
        .session
        .represented_creature_loot_state_like_cpp(fixture.owner_guid)
        .unwrap()
        .loot_lifecycle_revision;
    let mut stale_pool = authoritative_test_loot_like_cpp(0, true);
    stale_pool.loot_guid = represented_loot_object_guid_like_cpp(fixture.owner_guid);
    stale_pool.allowed_looters = vec![fixture.first_tapper];
    stale_pool.items[0].allowed_looters = vec![fixture.first_tapper];

    fixture
        .session
        .mutate_world_creature(fixture.owner_guid, |creature| {
            creature.creature.clear_loot_like_cpp();
            creature
                .creature
                .set_death_state_runtime(wow_constants::DeathState::JustRespawned, 0);
            creature
                .creature
                .set_death_state_runtime(wow_constants::DeathState::JustDied, 0);
        });

    assert!(
        !fixture
            .session
            .install_represented_creature_kill_loot_if_current_like_cpp(
                fixture.owner_guid,
                &authority,
                expected_generation,
                expected_revision,
                None,
                HashMap::from([(fixture.first_tapper, stale_pool)]),
            )
    );
    assert!(authority.is_retired_like_cpp());
    assert!(
        authority
            .snapshot_for_player_like_cpp(fixture.first_tapper)
            .is_none()
    );
}

#[tokio::test]
async fn represented_gameobject_chest_loot_carries_cpp_source_metadata() {
    let mut session = make_session();
    let gameobject_guid = test_gameobject_guid(91_001);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    let loot = session
        .generate_represented_gameobject_chest_loot_like_cpp(
            gameobject_guid,
            ObjectGuid::create_player(1, 42),
            source,
            &[],
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.loot_type, LOOT_TYPE_CHEST_LIKE_CPP);
    assert_eq!(loot.dungeon_encounter_id, 733);
    assert_eq!(loot.loot_method, 0);
}

#[tokio::test]
async fn represented_non_encounter_personal_chest_keeps_two_session_pools_independent_like_cpp() {
    let (mut first, _first_rx) = make_session_with_send_capacity(8);
    let (mut second, _second_rx) = make_session_with_send_capacity(8);
    let first_player = ObjectGuid::create_player(1, 42);
    let second_player = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_015);
    let personal_loot_id = 10_015;
    let item_id = 80_015;

    first.set_player_guid(Some(first_player));
    second.set_player_guid(Some(second_player));
    first.set_player_position_like_cpp(Position::ZERO);
    second.set_player_position_like_cpp(Position::ZERO);

    let gameobject =
        make_canonical_gameobject_for_session(&first, gameobject_guid, GAMEOBJECT_TYPE_CHEST as u8);
    attach_canonical_gameobject(&mut first, gameobject);
    second.set_canonical_map_manager(Arc::clone(
        first
            .canonical_map_manager
            .as_ref()
            .expect("both sessions share the canonical map owner"),
    ));

    install_limited_test_item_template(&mut first, item_id, 0);
    install_limited_test_item_template(&mut second, item_id, 0);
    let mut gameobject_store = LootStore::for_kind_like_cpp(LootStoreKind::Gameobject);
    gameobject_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: personal_loot_id,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Gameobject, gameobject_store);
    let stores = Arc::new(stores);
    first.set_loot_stores(Arc::clone(&stores));
    second.set_loot_stores(stores);

    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: true,
        dungeon_encounter_id: 0,
        personal_loot_id,
        ..Default::default()
    };

    first
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    second
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    let authority = canonical_gameobject_snapshot(&first, gameobject_guid)
        .expect("canonical chest remains map-owned")
        .loot_authority_like_cpp()
        .clone();
    assert!(authority.shared_snapshot_like_cpp().is_none());
    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 2);

    let first_pool = personal.get(&first_player).unwrap();
    let second_pool = personal.get(&second_player).unwrap();
    assert_ne!(first_pool.loot.loot_guid, second_pool.loot.loot_guid);
    assert_eq!(first_pool.loot.loot_method, 0);
    assert_eq!(second_pool.loot.loot_method, 0);
    for (player, pool) in [(first_player, first_pool), (second_player, second_pool)] {
        assert_eq!(pool.loot.allowed_looters, vec![player]);
        assert_eq!(pool.loot.items.len(), 1);
        assert_eq!(pool.loot.items[0].item_id, item_id);
        assert_eq!(pool.loot.items[0].allowed_looters, vec![player]);
    }

    let first_slot = first_pool.loot.items[0].loot_list_id;
    authority
        .reserve_item_like_cpp(first_player, first_slot)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_player)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
    assert!(
        !authority
            .snapshot_for_player_like_cpp(second_player)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
}

#[tokio::test]
async fn represented_empty_non_encounter_personal_chest_still_opens_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 141);
    let gameobject_guid = test_gameobject_guid(91_021);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let gameobject = make_canonical_gameobject_for_session(
        &session,
        gameobject_guid,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    attach_canonical_gameobject(&mut session, gameobject);

    session
        .open_represented_gameobject_chest_like_cpp(
            gameobject_guid,
            GameObjectLootSource {
                loot_id: 0,
                dungeon_encounter_id: 0,
                personal_loot_id: 10_021,
                ..Default::default()
            },
        )
        .await;

    let authority = canonical_gameobject_snapshot(&session, gameobject_guid)
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    let pool = authority
        .snapshot_for_player_like_cpp(player_guid)
        .expect("C++ retains the empty non-encounter personal pool");
    assert!(loot_is_looted_like_cpp(&pool.loot));
    assert_eq!(pool.loot.allowed_looters, vec![player_guid]);
    assert!(session.is_active_loot_guid(gameobject_guid));
    let _response = recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootResponse);
}

#[tokio::test]
async fn represented_personal_encounter_late_session_without_canonical_tap_list_fails_closed() {
    let (mut first, _first_rx) = make_session_with_send_capacity(8);
    let (mut second, second_rx) = make_session_with_send_capacity(8);
    let first_player = ObjectGuid::create_player(1, 142);
    let second_player = ObjectGuid::create_player(1, 177);
    let gameobject_guid = test_gameobject_guid(91_018);
    let personal_loot_id = 10_018;
    let item_id = 80_018;

    first.set_player_guid(Some(first_player));
    second.set_player_guid(Some(second_player));
    first.set_player_position_like_cpp(Position::ZERO);
    second.set_player_position_like_cpp(Position::ZERO);
    let gameobject =
        make_canonical_gameobject_for_session(&first, gameobject_guid, GAMEOBJECT_TYPE_CHEST as u8);
    attach_canonical_gameobject(&mut first, gameobject);
    second.set_canonical_map_manager(Arc::clone(
        first
            .canonical_map_manager
            .as_ref()
            .expect("both sessions share the canonical map owner"),
    ));

    install_limited_test_item_template(&mut first, item_id, 0);
    install_limited_test_item_template(&mut second, item_id, 0);
    let mut gameobject_store = LootStore::for_kind_like_cpp(LootStoreKind::Gameobject);
    gameobject_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: personal_loot_id,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Gameobject, gameobject_store);
    let stores = Arc::new(stores);
    first.set_loot_stores(Arc::clone(&stores));
    second.set_loot_stores(stores);

    let source = GameObjectLootSource {
        loot_id: 0,
        dungeon_encounter_id: 733,
        personal_loot_id,
        ..Default::default()
    };
    first
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    let authority = canonical_gameobject_snapshot(&first, gameobject_guid)
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    let first_before = authority
        .snapshot_for_player_like_cpp(first_player)
        .expect("the first opener owns the initial encounter pool");

    second
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 1);
    let first_after = personal.get(&first_player).unwrap();
    assert_eq!(first_after, &first_before);
    assert!(
        authority
            .snapshot_for_player_like_cpp(second_player)
            .is_none(),
        "without canonical GameObject::GetTapList state Rust must not fabricate outsider loot"
    );
    assert!(!second.is_active_loot_guid(gameobject_guid));
    assert!(second_rx.try_recv().is_err());
}

#[tokio::test]
async fn represented_personal_encounter_locked_or_empty_late_player_does_not_install_like_cpp() {
    let (mut first, _first_rx) = make_session_with_send_capacity(8);
    let (mut locked, locked_rx) = make_session_with_send_capacity(8);
    let (mut empty, empty_rx) = make_session_with_send_capacity(8);
    let first_player = ObjectGuid::create_player(1, 242);
    let locked_player = ObjectGuid::create_player(1, 277);
    let empty_player = ObjectGuid::create_player(1, 288);
    let gameobject_guid = test_gameobject_guid(91_019);
    let personal_loot_id = 10_019;
    let item_id = 80_019;
    let encounter_id = 734;

    for (session, player) in [
        (&mut first, first_player),
        (&mut locked, locked_player),
        (&mut empty, empty_player),
    ] {
        session.set_player_guid(Some(player));
        session.set_player_position_like_cpp(Position::ZERO);
    }
    let gameobject =
        make_canonical_gameobject_for_session(&first, gameobject_guid, GAMEOBJECT_TYPE_CHEST as u8);
    attach_canonical_gameobject(&mut first, gameobject);
    let manager = Arc::clone(first.canonical_map_manager.as_ref().unwrap());
    locked.set_canonical_map_manager(Arc::clone(&manager));
    empty.set_canonical_map_manager(manager);

    install_limited_test_item_template(&mut first, item_id, 0);
    install_limited_test_item_template(&mut locked, item_id, 0);
    let mut gameobject_store = LootStore::for_kind_like_cpp(LootStoreKind::Gameobject);
    gameobject_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: personal_loot_id,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Gameobject, gameobject_store);
    let stores = Arc::new(stores);
    first.set_loot_stores(Arc::clone(&stores));
    locked.set_loot_stores(stores);
    locked
        .represented_locked_dungeon_encounters
        .insert((locked_player, encounter_id));

    let source = GameObjectLootSource {
        loot_id: 0,
        dungeon_encounter_id: encounter_id,
        personal_loot_id,
        ..Default::default()
    };
    first
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    let authority = canonical_gameobject_snapshot(&first, gameobject_guid)
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    let first_before = authority
        .snapshot_for_player_like_cpp(first_player)
        .unwrap();

    locked
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    empty
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    assert_eq!(authority.personal_snapshots_like_cpp().len(), 1);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_player)
            .unwrap(),
        first_before
    );
    assert!(
        authority
            .snapshot_for_player_like_cpp(locked_player)
            .is_none()
    );
    assert!(
        authority
            .snapshot_for_player_like_cpp(empty_player)
            .is_none()
    );
    assert!(!locked.is_active_loot_guid(gameobject_guid));
    assert!(!empty.is_active_loot_guid(gameobject_guid));
    assert!(locked_rx.try_recv().is_err());
    assert!(empty_rx.try_recv().is_err());
}

#[test]
fn personal_encounter_late_upsert_cannot_cross_clear_loot_like_cpp() {
    let mut session = make_session();
    let first_player = ObjectGuid::create_player(1, 342);
    let late_player = ObjectGuid::create_player(1, 377);
    let gameobject_guid = test_gameobject_guid(91_020);
    let mut gameobject = make_canonical_gameobject_for_session(
        &session,
        gameobject_guid,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    let mut first_pool = authoritative_test_loot_like_cpp(0, true);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(gameobject_guid);
    first_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    first_pool.allowed_looters = vec![first_player];
    first_pool.items[0].allowed_looters = vec![first_player];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(None, HashMap::from([(first_player, first_pool)]),)
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    attach_canonical_gameobject(&mut session, gameobject);
    let observation = session
        .represented_gameobject_loot_install_observation_like_cpp(gameobject_guid)
        .expect("late generation observes the active chest lifetime");

    let mut stale_late_pool = authoritative_test_loot_like_cpp(0, true);
    stale_late_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        gameobject_guid.realm_id(),
        gameobject_guid.map_id(),
        0,
        0,
        gameobject_guid.counter() + 1,
    );
    stale_late_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    stale_late_pool.allowed_looters = vec![late_player];
    stale_late_pool.items[0].allowed_looters = vec![late_player];
    session
        .represented_personal_loot_owners
        .insert(gameobject_guid);
    session
        .represented_personal_loot_money
        .insert((gameobject_guid, late_player), 0);

    session.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
        gameobject.clear_loot_like_cpp();
    });

    assert!(
        session
            .upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(
                gameobject_guid,
                late_player,
                stale_late_pool,
                false,
                true,
                &observation,
            )
            .is_none()
    );
    assert!(authority.is_retired_like_cpp());
    assert!(authority.personal_snapshots_like_cpp().is_empty());
    assert!(
        !session
            .represented_personal_loot_money
            .contains_key(&(gameobject_guid, late_player))
    );
    assert!(!session.loot_table.contains_key(&gameobject_guid));
}

#[tokio::test]
async fn represented_empty_personal_encounter_chest_does_not_install_or_open_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_016);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let gameobject = make_canonical_gameobject_for_session(
        &session,
        gameobject_guid,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    attach_canonical_gameobject(&mut session, gameobject);

    session
        .open_represented_gameobject_chest_like_cpp(
            gameobject_guid,
            GameObjectLootSource {
                loot_id: 0,
                dungeon_encounter_id: 733,
                personal_loot_id: 10_016,
                ..Default::default()
            },
        )
        .await;

    let gameobject = canonical_gameobject_snapshot(&session, gameobject_guid).unwrap();
    assert!(gameobject.loot_authority_like_cpp().is_pristine_like_cpp());
    assert!(
        gameobject
            .loot_authority_like_cpp()
            .personal_snapshots_like_cpp()
            .is_empty()
    );
    assert!(!session.loot_table.contains_key(&gameobject_guid));
    assert!(
        !session
            .represented_personal_loot_owners
            .contains(&gameobject_guid)
    );
    assert!(
        !session
            .represented_personal_loot_money
            .contains_key(&(gameobject_guid, player_guid))
    );
    assert!(!session.is_active_loot_guid(gameobject_guid));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn represented_nonempty_personal_encounter_chest_keeps_live_canonical_pool_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_017);
    let personal_loot_id = 10_017;
    let item_id = 80_017;
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let gameobject = make_canonical_gameobject_for_session(
        &session,
        gameobject_guid,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    attach_canonical_gameobject(&mut session, gameobject);

    install_limited_test_item_template(&mut session, item_id, 0);
    let mut gameobject_store = LootStore::for_kind_like_cpp(LootStoreKind::Gameobject);
    gameobject_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: personal_loot_id,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Gameobject, gameobject_store);
    session.set_loot_stores(Arc::new(stores));

    session
        .open_represented_gameobject_chest_like_cpp(
            gameobject_guid,
            GameObjectLootSource {
                loot_id: 0,
                dungeon_encounter_id: 733,
                personal_loot_id,
                ..Default::default()
            },
        )
        .await;

    let gameobject = canonical_gameobject_snapshot(&session, gameobject_guid).unwrap();
    let pool = gameobject
        .loot_authority_like_cpp()
        .snapshot_for_player_like_cpp(player_guid)
        .expect("nonempty encounter pool remains map-owned");
    assert_eq!(pool.loot.allowed_looters, vec![player_guid]);
    assert_eq!(pool.loot.items.len(), 1);
    assert_eq!(pool.loot.items[0].item_id, item_id);
    assert!(!loot_is_looted_like_cpp(&pool.loot));
    assert!(session.is_active_loot_guid(gameobject_guid));
}

#[tokio::test]
async fn represented_gameobject_personal_encounter_loot_uses_current_player_when_no_tap_list_like_cpp()
 {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_008);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    let loot = session
        .generate_represented_gameobject_chest_loot_like_cpp(
            gameobject_guid,
            player_guid,
            source,
            &[],
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.allowed_looters, vec![player_guid]);
    assert!(
        loot.items
            .iter()
            .all(|entry| entry.allowed_looters == vec![player_guid])
    );
    assert_eq!(loot.coins, 0);
    assert!(
        session
            .represented_personal_loot_owners
            .contains(&gameobject_guid)
    );
    assert!(
        session
            .represented_personal_loot_money
            .contains_key(&(gameobject_guid, player_guid))
    );
}

#[tokio::test]
async fn represented_gameobject_personal_encounter_loot_uses_tap_list_like_cpp() {
    let mut session = make_session();
    let first_tapper = ObjectGuid::create_player(1, 42);
    let second_tapper = ObjectGuid::create_player(1, 77);
    let non_player_tapper = ObjectGuid::create_item(1, 900);
    let gameobject_guid = test_gameobject_guid(91_009);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);
    session.represented_gameobject_tap_lists.insert(
        gameobject_guid,
        vec![
            second_tapper,
            non_player_tapper,
            first_tapper,
            second_tapper,
        ],
    );
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    let loot = session
        .generate_represented_gameobject_chest_loot_like_cpp(
            gameobject_guid,
            first_tapper,
            source,
            &[],
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.allowed_looters, vec![first_tapper, second_tapper]);
    assert!(
        loot.items
            .iter()
            .all(|entry| entry.allowed_looters == vec![first_tapper, second_tapper])
    );
    assert_eq!(loot.coins, 0);
    assert!(
        session
            .represented_personal_loot_owners
            .contains(&gameobject_guid)
    );
    assert!(
        session
            .represented_personal_loot_money
            .contains_key(&(gameobject_guid, first_tapper))
    );
    assert!(
        session
            .represented_personal_loot_money
            .contains_key(&(gameobject_guid, second_tapper))
    );
}

#[tokio::test]
async fn represented_gameobject_personal_encounter_loot_skips_locked_tappers_like_cpp() {
    let mut session = make_session();
    let locked_tapper = ObjectGuid::create_player(1, 42);
    let open_tapper = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_010);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);
    session
        .represented_gameobject_tap_lists
        .insert(gameobject_guid, vec![locked_tapper, open_tapper]);
    session
        .represented_locked_dungeon_encounters
        .insert((locked_tapper, 733));
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    let loot = session
        .generate_represented_gameobject_chest_loot_like_cpp(
            gameobject_guid,
            locked_tapper,
            source,
            &[],
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.allowed_looters, vec![open_tapper]);
    assert!(
        loot.items
            .iter()
            .all(|entry| entry.allowed_looters == vec![open_tapper])
    );
}

#[tokio::test]
async fn represented_gameobject_personal_encounter_open_does_not_auto_allow_non_tapper_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_tapper = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_011);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session
        .represented_gameobject_tap_lists
        .insert(gameobject_guid, vec![other_tapper]);
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(gameobject_guid));
    assert_eq!(
        session
            .loot_table
            .get(&gameobject_guid)
            .unwrap()
            .allowed_looters,
        vec![other_tapper]
    );
}

#[tokio::test]
async fn represented_gameobject_personal_encounter_open_reads_player_money_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_012);
    let loot_object = represented_loot_object_guid_like_cpp(gameobject_guid);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.loot_table.insert(
        gameobject_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 999,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 733,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    session
        .represented_personal_loot_owners
        .insert(gameobject_guid);
    session
        .represented_personal_loot_money
        .insert((gameobject_guid, player_guid), 123);
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    let mut response =
        recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootResponse);
    assert_eq!(response.read_packed_guid().unwrap(), gameobject_guid);
    assert_eq!(response.read_packed_guid().unwrap(), loot_object);
    // failure_reason: C++ LootResponse::FailureReason defaults to 17 (LOOT_ERROR_NO_LOOT,
    // LootPackets.h:72 "Most common value") and is left unset on a successful loot — the
    // client ignores it once the window opens. (Previously, wrongly asserted as 0.)
    assert_eq!(response.read_uint8().unwrap(), 17);
    assert_eq!(response.read_uint8().unwrap(), LOOT_TYPE_CHEST_LIKE_CPP);
    assert_eq!(response.read_uint8().unwrap(), 0);
    assert_eq!(response.read_uint8().unwrap(), 2);
    assert_eq!(response.read_uint32().unwrap(), 123);
}

#[tokio::test]
async fn represented_gameobject_personal_encounter_money_pickup_consumes_only_player_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_tapper = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_013);
    let loot_object = represented_loot_object_guid_like_cpp(gameobject_guid);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(gameobject_guid);
    session.loot_table.insert(
        gameobject_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 999,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 733,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, other_tapper],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    session
        .represented_personal_loot_owners
        .insert(gameobject_guid);
    session
        .represented_personal_loot_money
        .insert((gameobject_guid, player_guid), 123);
    session
        .represented_personal_loot_money
        .insert((gameobject_guid, other_tapper), 456);

    session.handle_loot_money(loot_money_packet()).await;

    let mut notify =
        recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootMoneyNotify);
    assert_eq!(notify.read_uint64().unwrap(), 123);
    assert_eq!(
        session
            .represented_personal_loot_money
            .get(&(gameobject_guid, player_guid)),
        Some(&0)
    );
    assert_eq!(
        session
            .represented_personal_loot_money
            .get(&(gameobject_guid, other_tapper)),
        Some(&456)
    );
    assert_eq!(session.loot_table.get(&gameobject_guid).unwrap().coins, 999);
}

#[test]
fn represented_gameobject_personal_encounter_items_are_single_tapper_like_cpp() {
    let first_tapper = ObjectGuid::create_player(1, 42);
    let second_tapper = ObjectGuid::create_player(1, 77);
    let mut loot = CreatureLoot {
        loot_guid: represented_loot_object_guid_like_cpp(test_gameobject_guid(91_014)),
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
        dungeon_encounter_id: 733,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![first_tapper, second_tapper],
        items: vec![
            LootEntry {
                loot_list_id: 0,
                item_id: 1_001,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![first_tapper, second_tapper],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            },
            LootEntry {
                loot_list_id: 1,
                item_id: 1_002,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    freeforall: true,
                    ..LootEntryFlags::default()
                },
                allowed_looters: vec![first_tapper, second_tapper],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: vec![first_tapper],
                taken: false,
            },
        ],
        looted_by_player: false,
    };
    let mut rng = StdRng::seed_from_u64(7);

    assign_represented_personal_loot_items_like_cpp(
        &mut loot,
        &[first_tapper, second_tapper],
        &mut rng,
    );

    assert_eq!(loot.unlooted_count, 2);
    assert_eq!(loot.items[0].allowed_looters.len(), 1);
    assert_eq!(loot.items[1].allowed_looters.len(), 1);
    assert!([first_tapper, second_tapper].contains(&loot.items[0].allowed_looters[0]));
    assert!([first_tapper, second_tapper].contains(&loot.items[1].allowed_looters[0]));
    assert!(loot.items[0].flags.counted);
    assert!(!loot.items[1].flags.counted);
    assert_eq!(loot.items[1].ffa_looted_by, Vec::<ObjectGuid>::new());
    assert_eq!(loot.player_ffa_items.len(), 1);
    assert_eq!(loot.player_ffa_items[0].1[0].loot_list_id, 1);
}

#[tokio::test]
async fn represented_gameobject_chest_first_generation_records_use_effects_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_002);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    let source = GameObjectLootSource {
        loot_id: 55,
        use_group_loot_rules: false,
        dungeon_encounter_id: 0,
        personal_loot_id: 0,
        push_loot_id: 0,
        triggered_event_id: 777,
        linked_trap_entry: 888,
        ..Default::default()
    };

    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 777,
            },
            RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                gameobject_guid,
                player_guid,
                trap_entry: 888,
            },
        ]
    );
}

#[tokio::test]
async fn represented_gameobject_chest_push_unique_use_records_effects_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_004);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 0,
        personal_loot_id: 0,
        push_loot_id: 99,
        triggered_event_id: 321,
        linked_trap_entry: 654,
        ..Default::default()
    };

    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 321,
            },
            RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                gameobject_guid,
                player_guid,
                trap_entry: 654,
            },
        ]
    );
}

#[tokio::test]
async fn represented_gameobject_chest_no_loot_unique_use_records_effects_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_005);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 0,
        personal_loot_id: 0,
        push_loot_id: 0,
        triggered_event_id: 901,
        linked_trap_entry: 902,
        ..Default::default()
    };

    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 901,
            },
            RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                gameobject_guid,
                player_guid,
                trap_entry: 902,
            },
        ]
    );
}

#[tokio::test]
async fn represented_gameobject_chest_use_sets_activated_loot_state_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_006);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session
        .open_represented_gameobject_chest_like_cpp(
            gameobject_guid,
            GameObjectLootSource::default(),
        )
        .await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .expect("represented chest use records GO loot state");
    assert_eq!(state.loot_state, Some(LootState::Activated));
    assert_eq!(state.loot_state_unit_guid, player_guid);
}

#[tokio::test]
async fn represented_chest_use_syncs_state_to_same_map_viewers_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let other_map_guid = ObjectGuid::create_player(1, 88);
    let gameobject_guid = test_gameobject_guid(91_010);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (other_command_tx, other_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let (other_send_tx, _other_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());
    let mut other_info = broadcast_info(other_map_guid, other_send_tx);
    other_info.placement.map_id = 1;
    other_info.command_tx = other_command_tx;
    player_registry.register_or_replace(other_map_guid, other_info, Default::default());
    let source = GameObjectLootSource {
        loot_id: 190_010,
        personal_loot_id: 190_011,
        push_loot_id: 190_012,
        chest_restock_time_secs: 30,
        chest_consumable: false,
        chest_quest_id: 777,
        linked_trap_entry: 190_013,
        ..Default::default()
    };

    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(player_registry);
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command)) => command,
        other => panic!("expected chest sync command, got {other:?}"),
    };
    assert_eq!(command.gameobject_guid, gameobject_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.go_type, wow_entities::GAMEOBJECT_TYPE_CHEST as u8);
    assert_eq!(
        command.loot_state,
        Some(wow_entities::LootState::Activated as u8)
    );
    assert_eq!(command.chest_loot_id, 190_010);
    assert_eq!(command.chest_personal_loot_id, 190_011);
    assert_eq!(command.chest_push_loot_id, 190_012);
    assert_eq!(command.chest_quest_id, 777);
    assert_eq!(command.chest_restock_time_secs, 30);
    assert!(!command.chest_consumable);
    assert_eq!(command.linked_trap_entry, Some(190_013));
    assert_eq!(command.linked_trap_guid, None);
    assert!(other_command_rx.try_recv().is_err());
}

#[tokio::test]
async fn chest_state_sync_command_updates_receiver_before_refresh_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_011);
    session.set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(
            SyncChestGameobjectStateAndRefreshLikeCppCommand {
                gameobject_guid,
                map_id: 571,
                instance_id: 0,
                go_type: wow_entities::GAMEOBJECT_TYPE_CHEST as u8,
                loot_state: Some(wow_entities::LootState::Activated as u8),
                loot_state_unit_guid: ObjectGuid::create_player(1, 42),
                chest_loot_id: 190_011,
                chest_personal_loot_id: 190_012,
                chest_push_loot_id: 190_013,
                chest_quest_id: 778,
                chest_restock_time_secs: 45,
                chest_consumable: false,
                linked_trap_entry: Some(190_014),
                linked_trap_guid: Some(test_gameobject_guid(91_014)),
            },
        ))
        .expect("command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .expect("synced chest state");
    assert_eq!(
        state.go_type,
        Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8)
    );
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
    assert_eq!(state.chest_restock_time_secs, Some(45));
    assert_eq!(state.chest_consumable, Some(false));
    assert_eq!(state.chest_personal_loot_id, Some(190_012));
    assert_eq!(state.linked_trap_entry, Some(190_014));
    assert_eq!(state.linked_trap_guid, Some(test_gameobject_guid(91_014)));
    let source = state.chest_loot_source.expect("synced chest source");
    assert_eq!(source.loot_id, 190_011);
    assert_eq!(source.personal_loot_id, 190_012);
    assert_eq!(source.push_loot_id, 190_013);
    assert_eq!(source.chest_quest_id, 778);
}

#[test]
fn represented_goober_use_syncs_shared_state_to_same_map_viewers_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let other_map_guid = ObjectGuid::create_player(1, 88);
    let gameobject_guid = test_gameobject_guid(91_012);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (other_command_tx, other_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let (other_send_tx, _other_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());
    let mut other_info = broadcast_info(other_map_guid, other_send_tx);
    other_info.placement.map_id = 1;
    other_info.command_tx = other_command_tx;
    player_registry.register_or_replace(other_map_guid, other_info, Default::default());

    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(player_registry);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .linked_trap_entry = Some(190_015);

    assert!(session.use_represented_gameobject_goober_state_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::GooberUseSource {
            auto_close_ms: 3_000,
            linked_trap_entry: 190_015,
            ..Default::default()
        },
    ));

    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(command)) => command,
        other => panic!("expected goober sync command, got {other:?}"),
    };
    assert_eq!(command.gameobject_guid, gameobject_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.go_type, GAMEOBJECT_TYPE_GOOBER as u8);
    assert_eq!(command.gameobject_flags & wow_entities::GO_FLAG_IN_USE, 1);
    assert_eq!(
        command.loot_state,
        Some(wow_entities::LootState::Activated as u8)
    );
    assert_eq!(command.loot_state_unit_guid, player_guid);
    assert_eq!(command.go_state, Some(wow_entities::GoState::Active as i8));
    assert_eq!(command.linked_trap_entry, Some(190_015));
    assert!(other_command_rx.try_recv().is_err());
}

#[tokio::test]
async fn goober_state_sync_command_updates_receiver_before_refresh_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_013);
    session.set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(
            SyncGooberGameobjectStateAndRefreshLikeCppCommand {
                gameobject_guid,
                map_id: 571,
                instance_id: 0,
                go_type: GAMEOBJECT_TYPE_GOOBER as u8,
                gameobject_flags: wow_entities::GO_FLAG_IN_USE,
                loot_state: Some(wow_entities::LootState::Activated as u8),
                loot_state_unit_guid: owner_guid,
                go_state: Some(wow_entities::GoState::Active as i8),
                dynamic_flags: wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
                linked_trap_entry: Some(190_016),
                linked_trap_guid: Some(test_gameobject_guid(91_016)),
            },
        ))
        .expect("command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .expect("synced goober state");
    assert_eq!(state.go_type, Some(GAMEOBJECT_TYPE_GOOBER as u8));
    assert_eq!(state.gameobject_flags & wow_entities::GO_FLAG_IN_USE, 1);
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
    assert_eq!(state.loot_state_unit_guid, owner_guid);
    assert_eq!(state.go_state, Some(wow_entities::GoState::Active));
    assert_eq!(
        state.dynamic_flags & wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
        wow_entities::GO_DYNFLAG_LO_NO_INTERACT
    );
    assert_eq!(state.linked_trap_entry, Some(190_016));
    assert_eq!(state.linked_trap_guid, Some(test_gameobject_guid(91_016)));
    assert!(state.cooldown_until.is_none());
    assert!(state.goober_use_source.is_none());
}

#[tokio::test]
async fn represented_gathering_node_first_use_records_effects_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_003);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    let source = GatheringNodeUseSource {
        loot_id: 0,
        despawn_delay_secs: 0,
        triggered_event_id: 123,
        xp_difficulty: 0,
        spell_id: 0,
        max_loots: 10,
        linked_trap_entry: 456,
    };

    session
        .open_represented_gathering_node_like_cpp(gameobject_guid, 190_003, source)
        .await;
    session
        .open_represented_gathering_node_like_cpp(gameobject_guid, 190_003, source)
        .await;

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 123,
            },
            RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                gameobject_guid,
                player_guid,
                trap_entry: 456,
            },
        ]
    );
}

#[tokio::test]
async fn represented_fishing_hole_updates_catch_criteria_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_005);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session
        .open_represented_fishing_hole_like_cpp(gameobject_guid, 190_000, 123)
        .await;

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::FishingHoleCatchCriteriaUpdated {
                gameobject_guid,
                player_guid,
                gameobject_entry: 190_000,
            }
        ]
    );
}

#[tokio::test]
async fn represented_fishing_node_loot_walks_parent_area_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_051);
    let item_id = 80_001;
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
        AreaTableEntry {
            id: 77,
            continent_id: 0,
            parent_area_id: 10,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        AreaTableEntry {
            id: 10,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    install_limited_test_item_template(&mut session, item_id, 0);
    let mut fishing_store = LootStore::for_kind_like_cpp(LootStoreKind::Fishing);
    fishing_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 10,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Fishing, fishing_store);
    session.set_loot_stores(Arc::new(stores));

    session
        .open_represented_fishing_node_loot_like_cpp(gameobject_guid, 77, false)
        .await;

    let loot = session.loot_table.get(&gameobject_guid).unwrap();
    assert_eq!(loot.loot_type, LOOT_TYPE_FISHING_LIKE_CPP);
    assert_eq!(loot.items.len(), 1);
    assert_eq!(loot.items[0].item_id, item_id);
    assert!(session.is_active_loot_guid(gameobject_guid));
}

#[tokio::test]
async fn represented_fishing_node_junk_loot_uses_default_zone_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_052);
    let item_id = 80_002;
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    install_limited_test_item_template(&mut session, item_id, 0);
    let mut fishing_store = LootStore::for_kind_like_cpp(LootStoreKind::Fishing);
    fishing_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 1,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_JUNK_FISH_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Fishing, fishing_store);
    session.set_loot_stores(Arc::new(stores));

    session
        .open_represented_fishing_node_loot_like_cpp(gameobject_guid, 77, true)
        .await;

    let loot = session.loot_table.get(&gameobject_guid).unwrap();
    assert_eq!(loot.loot_type, LOOT_TYPE_FISHING_JUNK_LIKE_CPP);
    assert_eq!(loot.items.len(), 1);
    assert_eq!(loot.items[0].item_id, item_id);
    assert!(session.is_active_loot_guid(gameobject_guid));
}

#[tokio::test]
async fn represented_gathering_node_runtime_state_matches_cpp_side_effects() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_007);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    let source = GatheringNodeUseSource {
        loot_id: 0,
        despawn_delay_secs: 15,
        triggered_event_id: 0,
        xp_difficulty: 0,
        spell_id: 777,
        max_loots: 1,
        linked_trap_entry: 0,
    };

    session
        .open_represented_gathering_node_like_cpp(gameobject_guid, 190_007, source)
        .await;
    session
        .open_represented_gathering_node_like_cpp(gameobject_guid, 190_007, source)
        .await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .expect("represented gathering use records GO state");
    assert_eq!(state.personal_loot_uses, 1);
    assert_eq!(state.go_state, Some(GoState::Active));
    assert_eq!(
        state.dynamic_flags & GO_DYNFLAG_LO_NO_INTERACT,
        GO_DYNFLAG_LO_NO_INTERACT
    );
    assert_eq!(state.loot_state, Some(LootState::Activated));
    assert_eq!(state.loot_state_unit_guid, player_guid);
    assert_eq!(state.despawn_delay_secs, Some(15));
    assert!(state.despawn_delay_until.is_some());
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                gameobject_guid,
                player_guid,
                gameobject_entry: 190_007,
                spell_id: 777,
                go_type: GAMEOBJECT_TYPE_GATHERING_NODE,
                spell_lookup_difficulty_id: 0,
                spell_info_missing: false,
            },
            RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                gameobject_guid,
                target_guid: player_guid,
                caster_guid: player_guid,
                spell_id: 777,
                triggered: false,
                caster: RepresentedGameObjectSpellCaster::User,
                spell_lookup_difficulty_id: 0,
            },
        ]
    );
}

#[tokio::test]
async fn represented_gathering_node_use_refreshes_same_map_gameobject_viewers_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let other_map_guid = ObjectGuid::create_player(1, 88);
    let gameobject_guid = test_gameobject_guid(91_008);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (other_command_tx, other_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let (other_send_tx, _other_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());
    let mut other_info = broadcast_info(other_map_guid, other_send_tx);
    other_info.placement.map_id = 1;
    other_info.command_tx = other_command_tx;
    player_registry.register_or_replace(other_map_guid, other_info, Default::default());

    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(player_registry);
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    let source = GatheringNodeUseSource {
        loot_id: 0,
        despawn_delay_secs: 0,
        triggered_event_id: 0,
        xp_difficulty: 0,
        spell_id: 0,
        max_loots: 1,
        linked_trap_entry: 0,
    };

    session
        .open_represented_gathering_node_like_cpp(gameobject_guid, 190_008, source)
        .await;

    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(command)) => command,
        other => panic!("expected gathering-node sync command, got {other:?}"),
    };
    assert_eq!(command.gameobject_guid, gameobject_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(
        command.go_type,
        wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8
    );
    assert_eq!(
        command.loot_state,
        Some(wow_entities::LootState::Activated as u8)
    );
    assert_eq!(command.go_state, Some(wow_entities::GoState::Active as i8));
    assert_eq!(command.gathering_node_loot_id, Some(0));
    assert_eq!(
        command.dynamic_flags & wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
        wow_entities::GO_DYNFLAG_LO_NO_INTERACT
    );
    assert!(other_command_rx.try_recv().is_err());
}

#[tokio::test]
async fn gathering_node_state_sync_command_updates_receiver_before_refresh_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_009);
    session.set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(
            SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(
                SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand {
                    gameobject_guid,
                    map_id: 571,
                    instance_id: 0,
                    go_type: wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8,
                    loot_state: Some(wow_entities::LootState::Activated as u8),
                    loot_state_unit_guid: ObjectGuid::create_player(1, 42),
                    go_state: Some(wow_entities::GoState::Active as i8),
                    dynamic_flags: wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
                    gathering_node_loot_id: Some(190_009),
                    personal_loot_uses: 1,
                    linked_trap_entry: Some(191_009),
                    linked_trap_guid: Some(test_gameobject_guid(91_010)),
                },
            ),
        )
        .expect("command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .expect("synced gathering node state");
    assert_eq!(
        state.go_type,
        Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8)
    );
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
    assert_eq!(state.go_state, Some(wow_entities::GoState::Active));
    assert_eq!(
        state.dynamic_flags & wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
        wow_entities::GO_DYNFLAG_LO_NO_INTERACT
    );
    assert_eq!(state.gathering_node_loot_id, Some(190_009));
    assert_eq!(state.personal_loot_uses, 1);
    assert_eq!(state.linked_trap_entry, Some(191_009));
    assert_eq!(state.linked_trap_guid, Some(test_gameobject_guid(91_010)));
}

#[tokio::test]
async fn represented_creature_loot_captures_group_method_master_and_round_robin_like_cpp() {
    let mut session = make_session();
    let master_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_049);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);
    install_master_loot_group(&mut session, master_guid, candidate_guid);

    let loot = session
        .generate_represented_creature_loot_like_cpp(owner_guid, master_guid, 10, 25, 0, 0, 0, 0)
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.loot_method, LOOT_METHOD_MASTER_LIKE_CPP);
    assert_eq!(loot.loot_master, master_guid);
    assert_eq!(loot.round_robin_player, master_guid);
}

#[test]
fn represented_gameobject_group_loot_keeps_round_robin_empty_like_cpp() {
    let mut session = make_session();
    let opener = ObjectGuid::create_player(1, 42);
    let candidate = ObjectGuid::create_player(1, 77);
    install_master_loot_group(&mut session, opener, candidate);

    let (loot_method, loot_master, round_robin_player) =
        session.represented_gameobject_chest_group_state_like_cpp(true, opener);

    assert_eq!(loot_method, LOOT_METHOD_MASTER_LIKE_CPP);
    assert_eq!(loot_master, opener);
    assert_eq!(round_robin_player, ObjectGuid::EMPTY);
}

fn test_gameobject_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 1, counter)
}

fn test_item_record(item_id: u32, random_select: u16, random_suffix_group_id: u16) -> ItemRecord {
    ItemRecord {
        id: item_id,
        class_id: 2,
        subclass_id: 7,
        material: 0,
        inventory_type: InventoryType::Chest as i8,
        sheathe_type: 0,
        random_select,
        random_suffix_group_id,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }
}

#[test]
fn loot_item_random_context_runtime_and_persistence_fields_match_entry() {
    let sql = CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT.sql();
    assert!(sql.contains("randomPropertiesId"));
    assert!(sql.contains("randomPropertiesSeed"));
    assert!(sql.contains("context"));
    assert!(
        sql.contains("'', '', ?, ?, ?, ?"),
        "stored-new-item flags must be a bound parameter, not the old hard-coded zero"
    );

    let item_guid = ObjectGuid::create_item(1, 902);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let mut item = Item::new(0);
    item.initialize_created_state(ItemCreateInfo {
        guid: item_guid,
        item_id: 25,
        context: loot_item_context(2),
        owner: Some(owner_guid),
        max_durability: 0,
        expiration: 0,
        spell_charges: [0; MAX_ITEM_SPELLS],
    });
    item.set_random_properties_id(-77);
    item.set_property_seed(456);

    let data = item.data();
    assert_eq!(data.random_properties_id, -77);
    assert_eq!(data.property_seed, 456);
    assert_eq!(u8::try_from(data.context).unwrap_or(0), 2);
}

#[test]
fn stored_new_item_flags_follow_cpp_new_and_binding_rules() {
    let mut session = make_session();
    let item_id = 25;

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnAcquire,
    );
    assert_eq!(
        session.stored_new_item_dynamic_flags_like_cpp(item_id, INVENTORY_SLOT_ITEM_START),
        (ItemFieldFlags::NEW_ITEM | ItemFieldFlags::SOULBOUND).bits()
    );

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::None,
    );
    assert_eq!(
        session.stored_new_item_dynamic_flags_like_cpp(item_id, INVENTORY_SLOT_ITEM_START),
        ItemFieldFlags::NEW_ITEM.bits(),
        "an unbound backpack item must not acquire SOULBOUND"
    );

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnEquip,
    );
    assert_eq!(
        session.stored_new_item_dynamic_flags_like_cpp(item_id, INVENTORY_SLOT_ITEM_START),
        ItemFieldFlags::NEW_ITEM.bits(),
        "C++ bind-if-stored does not bind OnEquip items in backpack slots"
    );
}

#[test]
fn historical_stack_binding_adds_only_soulbound_like_cpp() {
    let mut session = make_session();
    let item_id = 25;
    let historical = Item::new(0);

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnAcquire,
    );
    let flags = session.stored_existing_item_dynamic_flags_like_cpp(
        item_id,
        INVENTORY_SLOT_ITEM_START,
        &historical,
    );
    assert_eq!(flags, ItemFieldFlags::SOULBOUND.bits());
    assert_eq!(flags & ItemFieldFlags::NEW_ITEM.bits(), 0);

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnEquip,
    );
    assert_eq!(
        session.stored_existing_item_dynamic_flags_like_cpp(
            item_id,
            wow_entities::INVENTORY_SLOT_BAG_START,
            &historical,
        ),
        ItemFieldFlags::SOULBOUND.bits(),
        "C++ binds an OnEquip item when that item is stored in a bag-equipment position"
    );
    assert_eq!(
        session.stored_existing_item_dynamic_flags_like_cpp(
            item_id,
            INVENTORY_SLOT_ITEM_START,
            &historical,
        ),
        0,
        "C++ does not bind an OnEquip stack in an ordinary backpack slot"
    );
}

#[tokio::test]
async fn existing_stack_count_and_binding_share_one_sql_transaction() {
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(1)
        .connect_lazy("mysql://rustycore:rustycore@127.0.0.1:1/characters")
        .expect("syntactically valid lazy MySQL pool");
    let char_db = wow_database::CharacterDatabase::from_pool(pool);

    let mut bound_update = wow_database::SqlTransaction::new();
    WorldSession::append_existing_loot_stack_persistence_like_cpp(
        &char_db,
        &mut bound_update,
        77,
        19,
        Some(ItemFieldFlags::SOULBOUND.bits()),
    );
    assert_eq!(
        bound_update.len(),
        2,
        "count and DynamicFlags must be committed or rolled back together"
    );

    let mut count_only_update = wow_database::SqlTransaction::new();
    WorldSession::append_existing_loot_stack_persistence_like_cpp(
        &char_db,
        &mut count_only_update,
        77,
        19,
        None,
    );
    assert_eq!(count_only_update.len(), 1);
}

#[tokio::test]
async fn failed_existing_stack_store_publishes_neither_count_nor_binding() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 77);
    let item_id = 25;
    session.set_player_guid(Some(player_guid));
    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnAcquire,
    );
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: item_id,
            db_guid: 77,
            inventory_type: None,
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        item_id,
        player_guid,
        4,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(item);

    let failing_pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(100))
        .connect_lazy("mysql://rustycore:rustycore@127.0.0.1:1/characters")
        .expect("syntactically valid lazy MySQL pool");
    session.set_char_db(Arc::new(wow_database::CharacterDatabase::from_pool(
        failing_pool,
    )));

    let stored = session
        .store_direct_loot_item_like_cpp(
            &LootEntry {
                loot_list_id: 0,
                item_id,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            },
            0,
        )
        .await;

    assert!(!stored);
    let historical = session
        .inventory_item_objects_like_cpp()
        .get(&item_guid)
        .expect("failed transaction keeps the historical stack");
    assert_eq!(historical.count(), 4);
    assert_eq!(historical.item_flags_bits(), 0);
}

#[test]
fn loot_item_random_context_stack_compatibility_uses_cpp_store_metadata() {
    let item_guid = ObjectGuid::create_item(1, 901);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let mut item = Item::new(0);
    item.initialize_created_state(ItemCreateInfo {
        guid: item_guid,
        item_id: 25,
        context: ItemContext::DungeonHeroic,
        owner: Some(owner_guid),
        max_durability: 0,
        expiration: 0,
        spell_charges: [0; MAX_ITEM_SPELLS],
    });
    item.set_random_properties_id(-77);
    item.set_property_seed(456);

    let matching = LootEntry {
        loot_list_id: 0,
        item_id: 25,
        quantity: 1,
        random_properties_id: -77,
        random_properties_seed: 456,
        item_context: 2,
        flags: LootEntryFlags::default(),
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    };
    assert!(loot_store_data_can_stack_with_item(
        &matching,
        LootStoreRandomProperties { id: -77, seed: 456 },
        &item
    ));

    let different_random = LootEntry {
        random_properties_id: -78,
        ..matching.clone()
    };
    assert!(loot_store_data_can_stack_with_item(
        &different_random,
        LootStoreRandomProperties { id: -77, seed: 456 },
        &item
    ));
    assert!(!loot_store_data_can_stack_with_item(
        &matching,
        LootStoreRandomProperties { id: 0, seed: 0 },
        &item
    ));
}

#[test]
fn loot_item_store_random_properties_are_generated_from_cpp_random_select() {
    let entry = LootEntry {
        loot_list_id: 0,
        item_id: 25,
        quantity: 1,
        random_properties_id: -77,
        random_properties_seed: 456,
        item_context: 2,
        flags: LootEntryFlags::default(),
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    };
    let mut session = make_session();
    session.set_item_store(Arc::new(ItemStore::from_records([test_item_record(
        entry.item_id,
        77,
        0,
    )])));
    session.set_item_random_enchantment_template_store(Arc::new(
        ItemRandomEnchantmentTemplateStore::from_entries([ItemRandomEnchantmentTemplateEntry {
            group_id: 77,
            enchantment_id: 9001,
            chance: 100.0,
        }]),
    ));
    session.set_item_random_properties_store(Arc::new(ItemRandomPropertiesStore::from_entries([
        ItemRandomPropertiesEntry {
            id: 9001,
            enchantments: [1, 2, 3, 0, 0],
        },
    ])));

    let generated = session.generate_loot_store_random_properties_with_rng_like_cpp(
        entry.item_id,
        &mut StdRng::seed_from_u64(1),
    );
    assert_eq!(generated, LootStoreRandomProperties { id: 9001, seed: 0 });
    assert_ne!(generated.id, entry.random_properties_id);
    assert_ne!(generated.seed, entry.random_properties_seed);
}

#[test]
fn loot_item_store_random_suffix_uses_cpp_property_points_seed() {
    let mut session = make_session();
    session.set_item_store(Arc::new(ItemStore::from_records([test_item_record(
        25, 0, 88,
    )])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_random_property_templates([
        (
            25,
            ItemRandomPropertyTemplateEntry {
                item_level: 11,
                quality: ItemQuality::Uncommon as i8,
                inventory_type: InventoryType::Chest as i8,
            },
        ),
    ])));
    session.set_item_random_enchantment_template_store(Arc::new(
        ItemRandomEnchantmentTemplateStore::from_entries([ItemRandomEnchantmentTemplateEntry {
            group_id: 88,
            enchantment_id: 7001,
            chance: 100.0,
        }]),
    ));
    session.set_item_random_suffix_store(Arc::new(ItemRandomSuffixStore::from_entries([
        ItemRandomSuffixEntry {
            id: 7001,
            enchantments: [10, 0, 0, 0, 0],
            allocation_pct: [10000, 0, 0, 0, 0],
        },
    ])));
    session.set_rand_prop_points_store(Arc::new(RandPropPointsStore::from_entries([
        RandPropPointsEntry {
            id: 11,
            damage_replace_stat: 0,
            epic: [900, 0, 0, 0, 0],
            superior: [500, 0, 0, 0, 0],
            good: [123, 0, 0, 0, 0],
        },
    ])));

    let generated = session
        .generate_loot_store_random_properties_with_rng_like_cpp(25, &mut StdRng::seed_from_u64(1));
    assert_eq!(
        generated,
        LootStoreRandomProperties {
            id: -7001,
            seed: 123
        }
    );
}

#[test]
fn random_enchantment_selection_uses_cpp_weighted_chances() {
    let group = [
        ItemRandomEnchantmentTemplateEntry {
            group_id: 1,
            enchantment_id: 10,
            chance: 0.0,
        },
        ItemRandomEnchantmentTemplateEntry {
            group_id: 1,
            enchantment_id: 11,
            chance: 100.0,
        },
    ];
    assert_eq!(
        select_weighted_random_enchantment_like_cpp(&group, &mut StdRng::seed_from_u64(5)),
        Some(11)
    );
}

#[test]
fn gameobject_interaction_distance_uses_cpp_type_branches() {
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_CHEST as u8),
            Some(725)
        ),
        7.25
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_AREADAMAGE as u8),
            None
        ),
        0.0
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_QUESTGIVER as u8),
            None
        ),
        5.5555553
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_BINDER as u8),
            None
        ),
        10.0
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_CHAIR as u8),
            None
        ),
        3.0
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_FISHING_NODE as u8),
            None
        ),
        100.0
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_FISHING_HOLE as u8),
            None
        ),
        20.0 + wow_movement::CONTACT_DISTANCE_LIKE_CPP
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_DOOR as u8),
            None
        ),
        5.0
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(
            Some(GAMEOBJECT_TYPE_GUILD_BANK as u8),
            None
        ),
        10.0
    );
    assert_eq!(
        represented_gameobject_interaction_distance_like_cpp(None, None),
        5.0
    );
}

#[test]
fn gameobject_display_box_interaction_matches_cpp_contains_branch() {
    let display_info = wow_data::GameObjectDisplayInfoEntry {
        id: 77,
        model_name: "test".to_string(),
        geo_box_min: wow_data::Db2Pos3 {
            x: -2.0,
            y: -1.0,
            z: -0.5,
        },
        geo_box_max: wow_data::Db2Pos3 {
            x: 2.0,
            y: 1.0,
            z: 0.5,
        },
        file_data_id: 0,
        object_effect_package_id: 0,
        override_loot_effect_scale: 0.0,
        override_name_scale: 0.0,
    };
    let go_position = Position::ZERO;

    assert!(represented_gameobject_display_box_contains_like_cpp(
        go_position,
        Position::xyz(6.9, 0.0, 0.0),
        &display_info,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
        5.0,
    ));
    assert!(!represented_gameobject_display_box_contains_like_cpp(
        go_position,
        Position::xyz(7.1, 0.0, 0.0),
        &display_info,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
        5.0,
    ));
}

#[test]
fn gameobject_loot_distance_uses_display_box_when_db2_exists_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(19_041);
    session.set_player_guid(Some(player_guid));
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        gameobject_guid,
        gameobject_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_display_model_like_cpp(
        gameobject_guid,
        77,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
    );
    session.set_gameobject_display_info_store(Arc::new(
        wow_data::GameObjectDisplayInfoStore::from_entries([
            wow_data::GameObjectDisplayInfoEntry {
                id: 77,
                model_name: "test".to_string(),
                geo_box_min: wow_data::Db2Pos3 {
                    x: -2.0,
                    y: -1.0,
                    z: -0.5,
                },
                geo_box_max: wow_data::Db2Pos3 {
                    x: 2.0,
                    y: 1.0,
                    z: 0.5,
                },
                file_data_id: 0,
                object_effect_package_id: 0,
                override_loot_effect_scale: 0.0,
                override_name_scale: 0.0,
            },
        ]),
    ));

    session.set_player_position_like_cpp(Position::xyz(6.9, 0.0, 0.0));
    assert!(
        session
            .represented_gameobject_can_autostore_loot_item_like_cpp(gameobject_guid, player_guid)
    );

    session.set_player_position_like_cpp(Position::xyz(7.1, 0.0, 0.0));
    assert!(
        !session
            .represented_gameobject_can_autostore_loot_item_like_cpp(gameobject_guid, player_guid)
    );
}

#[test]
fn gameobject_loot_distance_uses_spell_lock_range_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(19_042);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::xyz(11.0, 0.0, 0.0));
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        gameobject_guid,
        gameobject_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_lock_id_like_cpp(gameobject_guid, 501);
    session.set_lock_store(Arc::new(wow_data::LockStore::from_entries([
        wow_data::LockEntry {
            id: 501,
            index: [7001, 0, 0, 0, 0, 0, 0, 0],
            skill: [0; wow_data::lock::MAX_LOCK_CASE],
            lock_type: [LOCK_KEY_SPELL_LIKE_CPP, 0, 0, 0, 0, 0, 0, 0],
            action: [0; wow_data::lock::MAX_LOCK_CASE],
        },
    ])));
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        7001,
        SpellInfo {
            spell_id: 7001,
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
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([SpellMiscEntry {
        id: 7001,
        attributes: [0; 15],
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index: 0,
        range_index: 77,
        school_mask: 0,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id: 7001,
    }])));
    session.set_spell_range_store(Arc::new(SpellRangeStore::from_entries([SpellRangeEntry {
        id: 77,
        display_name: "lock".to_string(),
        display_name_short: "lock".to_string(),
        flags: 0,
        range_min: [0.0, 0.0],
        range_max: [12.0, 12.0],
    }])));

    assert!(
        session
            .represented_gameobject_can_autostore_loot_item_like_cpp(gameobject_guid, player_guid)
    );

    session.set_player_position_like_cpp(Position::xyz(12.1, 0.0, 0.0));
    assert!(
        !session
            .represented_gameobject_can_autostore_loot_item_like_cpp(gameobject_guid, player_guid)
    );
}

#[test]
fn gameobject_loot_distance_uses_known_open_lock_skill_spell_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(19_043);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::xyz(8.0, 0.0, 0.0));
    session.set_known_spells_like_cpp(vec![8001]);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        gameobject_guid,
        gameobject_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_lock_id_like_cpp(gameobject_guid, 502);
    session.set_lock_store(Arc::new(wow_data::LockStore::from_entries([
        wow_data::LockEntry {
            id: 502,
            index: [333, 0, 0, 0, 0, 0, 0, 0],
            skill: [50, 0, 0, 0, 0, 0, 0, 0],
            lock_type: [LOCK_KEY_SKILL_LIKE_CPP, 0, 0, 0, 0, 0, 0, 0],
            action: [0; wow_data::lock::MAX_LOCK_CASE],
        },
    ])));
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        8001,
        SpellInfo {
            spell_id: 8001,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: SPELL_EFFECT_OPEN_LOCK_LIKE_CPP,
            effect_base_points: 75,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![SpellEffectInfo {
                effect_index: 0,
                effect: SPELL_EFFECT_OPEN_LOCK_LIKE_CPP,
                effect_aura: 0,
                effect_base_points: 75,
                effect_misc_value_1: 333,
                effect_misc_value_2: 0,
                effect_radius_index_1: 0,
                chain_targets: 0,
                implicit_target_1: 0,
                implicit_target_2: 0,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([SpellMiscEntry {
        id: 8001,
        attributes: [0; 15],
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index: 0,
        range_index: 88,
        school_mask: 0,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id: 8001,
    }])));
    session.set_spell_range_store(Arc::new(SpellRangeStore::from_entries([SpellRangeEntry {
        id: 88,
        display_name: "skill".to_string(),
        display_name_short: "skill".to_string(),
        flags: 0,
        range_min: [0.0, 0.0],
        range_max: [9.0, 9.0],
    }])));

    assert!(
        session
            .represented_gameobject_can_autostore_loot_item_like_cpp(gameobject_guid, player_guid)
    );
}

#[test]
fn loot_is_looted_requires_no_money_and_no_unlooted_items_like_cpp() {
    let mut loot = CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins: 1,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: vec![],
        looted_by_player: false,
    };
    assert!(!loot_is_looted_like_cpp(&loot));

    loot.coins = 0;
    loot.items.push(LootEntry {
        loot_list_id: 0,
        item_id: 25,
        quantity: 1,
        random_properties_id: 0,
        random_properties_seed: 0,
        item_context: 0,
        flags: LootEntryFlags::default(),
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    });
    loot.unlooted_count = 1;
    assert!(!loot_is_looted_like_cpp(&loot));

    loot.items[0].taken = true;
    assert!(!loot_is_looted_like_cpp(&loot));

    loot.unlooted_count = 0;
    assert!(loot_is_looted_like_cpp(&loot));
}

#[tokio::test]
async fn loot_unit_live_creature_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_006);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, true));

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_unit_dead_player_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_033);
    session.set_player_guid(Some(player_guid));
    session.set_player_alive_like_cpp(false);
    install_active_spell_cast(&mut session, player_guid);
    install_visible_aura_with_interrupt_flags(
        &mut session,
        3,
        777,
        player_guid,
        SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP,
    );
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.contains_key(&loot_guid));
    assert!(session.active_spell_cast.is_some());
    assert!(session.visible_auras.contains_key(&3));
}

#[tokio::test]
async fn loot_unit_valid_target_interrupts_active_cast_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_034);
    session.set_player_guid(Some(player_guid));
    install_active_spell_cast(&mut session, player_guid);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(session.active_spell_cast.is_none());
}

#[tokio::test]
async fn loot_unit_valid_target_removes_looting_interrupt_auras_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_035);
    session.set_player_guid(Some(player_guid));
    install_visible_aura_with_interrupt_flags(
        &mut session,
        3,
        777,
        player_guid,
        SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP,
    );
    install_visible_aura_with_interrupt_flags(&mut session, 4, 778, player_guid, 0);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(!session.visible_auras.contains_key(&3));
    assert!(session.visible_auras.contains_key(&4));
}

#[tokio::test]
async fn loot_unit_master_looter_first_open_sends_candidate_list_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(3);
    let master_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_046);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    session.set_player_guid(Some(master_guid));
    install_master_loot_group(&mut session, master_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 1,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![master_guid, candidate_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, master_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

    let response = send_rx.try_recv().unwrap();
    let mut response = WorldPacket::from_bytes(&response);
    assert_eq!(
        response.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );

    let loot_list = send_rx.try_recv().unwrap();
    let mut loot_list = WorldPacket::from_bytes(&loot_list);
    assert_eq!(
        loot_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootList as u16
    );
    assert_eq!(loot_list.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(loot_list.read_packed_guid().unwrap(), loot_object);
    assert!(!loot_list.read_bit().unwrap());
    assert!(!loot_list.read_bit().unwrap());

    let candidate_list = send_rx.try_recv().unwrap();
    let mut candidate_list = WorldPacket::from_bytes(&candidate_list);
    assert_eq!(
        candidate_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::MasterLootCandidateList as u16
    );
    assert_eq!(candidate_list.read_packed_guid().unwrap(), loot_object);
    assert_eq!(candidate_list.read_uint32().unwrap(), 2);
    assert_eq!(candidate_list.read_packed_guid().unwrap(), master_guid);
    assert_eq!(candidate_list.read_packed_guid().unwrap(), candidate_guid);
    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .loot_table
            .get(&owner_guid)
            .is_some_and(|loot| loot.looted_by_player)
    );
}

#[tokio::test]
async fn loot_unit_master_looter_candidate_list_is_first_open_only_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let master_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_047);
    session.set_player_guid(Some(master_guid));
    install_master_loot_group(&mut session, master_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(owner_guid),
            coins: 1,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![master_guid, candidate_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, master_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

    let mut master_candidate_lists = 0;
    while let Ok(sent) = send_rx.try_recv() {
        let mut sent = WorldPacket::from_bytes(&sent);
        if sent.read_uint16().unwrap()
            == wow_constants::ServerOpcodes::MasterLootCandidateList as u16
        {
            master_candidate_lists += 1;
        }
    }

    assert_eq!(master_candidate_lists, 1);
}

#[tokio::test]
async fn loot_unit_master_loot_notify_list_fans_out_to_allowed_looters_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let master_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_048);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(2);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(master_guid));
    install_master_loot_group(&mut session, master_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![master_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    ..Default::default()
                },
                allowed_looters: vec![master_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, master_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

    let _response = send_rx.try_recv().unwrap();
    let local_loot_list = send_rx.try_recv().unwrap();
    let mut local_loot_list = WorldPacket::from_bytes(&local_loot_list);
    assert_eq!(
        local_loot_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootList as u16
    );
    assert_eq!(local_loot_list.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(local_loot_list.read_packed_guid().unwrap(), loot_object);
    assert!(local_loot_list.read_bit().unwrap());
    assert!(!local_loot_list.read_bit().unwrap());
    local_loot_list.reset_bits();
    assert_eq!(local_loot_list.read_packed_guid().unwrap(), master_guid);

    let remote_loot_list = candidate_rx.try_recv().unwrap();
    let mut remote_loot_list = WorldPacket::from_bytes(&remote_loot_list);
    assert_eq!(
        remote_loot_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootList as u16
    );
    assert_eq!(remote_loot_list.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(remote_loot_list.read_packed_guid().unwrap(), loot_object);
    assert!(remote_loot_list.read_bit().unwrap());
    assert!(!remote_loot_list.read_bit().unwrap());
    remote_loot_list.reset_bits();
    assert_eq!(remote_loot_list.read_packed_guid().unwrap(), master_guid);
}

#[tokio::test]
async fn loot_unit_group_loot_first_open_starts_roll_for_blocked_item_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let disconnected_guid = ObjectGuid::create_player(1, 88);
    let owner_guid = test_creature_guid(19_049);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(4);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid, disconnected_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

    let response = send_rx.try_recv().unwrap();
    let mut response = WorldPacket::from_bytes(&response);
    assert_eq!(
        response.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    let loot_list = send_rx.try_recv().unwrap();
    let mut loot_list = WorldPacket::from_bytes(&loot_list);
    assert_eq!(
        loot_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootList as u16
    );

    let start_roll = send_rx.try_recv().unwrap();
    let mut start_roll = WorldPacket::from_bytes(&start_roll);
    assert_eq!(
        start_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::StartLootRoll as u16
    );
    assert_eq!(start_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(start_roll.read_int32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 60_000);
    assert_eq!(start_roll.read_uint8().unwrap(), 0x07);
    assert_eq!(start_roll.read_uint32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 0);
    assert_eq!(start_roll.read_uint8().unwrap(), LOOT_METHOD_GROUP_LIKE_CPP);
    assert_eq!(start_roll.read_int32().unwrap(), 0);
    assert_eq!(start_roll.read_bits(2).unwrap(), 0);
    assert_eq!(start_roll.read_bits(3).unwrap(), 1);
    assert!(send_rx.try_recv().is_err());

    let remote_loot_list = candidate_rx.try_recv().unwrap();
    let mut remote_loot_list = WorldPacket::from_bytes(&remote_loot_list);
    assert_eq!(
        remote_loot_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootList as u16
    );
    let remote_start_roll = candidate_rx.try_recv().unwrap();
    let mut remote_start_roll = WorldPacket::from_bytes(&remote_start_roll);
    assert_eq!(
        remote_start_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::StartLootRoll as u16
    );
    assert_eq!(remote_start_roll.read_packed_guid().unwrap(), loot_object);

    let state = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .unwrap();
    assert_eq!(
        state.voters.get(&player_guid).unwrap().vote,
        ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
    );
    assert_eq!(
        state.voters.get(&candidate_guid).unwrap().vote,
        ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
    );
    assert_eq!(
        state.voters.get(&disconnected_guid).unwrap().vote,
        ROLL_VOTE_NOT_VALID_LIKE_CPP
    );

    let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
    assert!(entry.flags.blocked);
    assert!(!entry.flags.under_threshold);
    assert!(
        session
            .loot_table
            .get(&owner_guid)
            .unwrap()
            .looted_by_player
    );
}

#[tokio::test]
async fn loot_unit_group_loot_can_only_roll_greed_removes_need_from_start_mask_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_058);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, _candidate_rx) = flume::bounded::<Vec<u8>>(4);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    install_limited_test_item_template_with_flags2(
        &mut session,
        25,
        0,
        ItemFlags2::CanOnlyRollGreed as u32,
    );
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let start_roll = send_rx.try_recv().unwrap();
    let mut start_roll = WorldPacket::from_bytes(&start_roll);
    assert_eq!(
        start_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::StartLootRoll as u16
    );
    assert_eq!(start_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(start_roll.read_int32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 60_000);
    assert_eq!(
        start_roll.read_uint8().unwrap(),
        ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP & !ROLL_FLAG_TYPE_NEED_LIKE_CPP
    );
}

#[tokio::test]
async fn loot_unit_group_loot_disenchant_mask_uses_cpp_skill_required_gate() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_059);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, _candidate_rx) = flume::bounded::<Vec<u8>>(4);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut candidate_info = broadcast_info(candidate_guid, candidate_tx);
    candidate_info.info.enchanting_skill = 175;
    player_registry.register_or_replace(candidate_guid, candidate_info, Default::default());
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    install_disenchantable_test_item_template(&mut session, 25);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let start_roll = send_rx.try_recv().unwrap();
    let mut start_roll = WorldPacket::from_bytes(&start_roll);
    assert_eq!(
        start_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::StartLootRoll as u16
    );
    assert_eq!(start_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(start_roll.read_int32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 60_000);
    assert_eq!(start_roll.read_uint8().unwrap(), 0x0F);
}

#[test]
fn represented_disenchant_loot_template_row_guards_match_cpp_shape() {
    let valid = LootStoreItem {
        item_id: 10940,
        reference: 0,
        chance: 100.0,
        needs_quest: false,
        loot_mode: super::LOOT_MODE_DEFAULT_LIKE_CPP,
        group_id: 0,
        min_count: 1,
        max_count: 2,
    };
    assert!(super::represented_disenchant_loot_plain_row_can_roll_like_cpp(&valid, true));

    let mut missing_item = valid;
    missing_item.item_id = 0;
    assert!(!super::represented_disenchant_loot_plain_row_can_roll_like_cpp(&missing_item, true));

    let mut bad_count = valid;
    bad_count.max_count = 0;
    assert!(!super::represented_disenchant_loot_plain_row_can_roll_like_cpp(&bad_count, true));

    let reference = LootStoreItem {
        item_id: 0,
        reference: 700,
        chance: 100.0,
        needs_quest: false,
        loot_mode: super::LOOT_MODE_DEFAULT_LIKE_CPP,
        group_id: 0,
        min_count: 1,
        max_count: 1,
    };
    assert!(super::represented_disenchant_loot_reference_row_can_roll_like_cpp(&reference));
}

#[test]
fn represented_disenchant_loot_template_frame_splits_group_rows_like_cpp() {
    let rows = vec![
        LootStoreItem {
            item_id: 10940,
            reference: 0,
            chance: 100.0,
            needs_quest: false,
            loot_mode: super::LOOT_MODE_DEFAULT_LIKE_CPP,
            group_id: 0,
            min_count: 1,
            max_count: 1,
        },
        LootStoreItem {
            item_id: 10978,
            reference: 0,
            chance: 0.0,
            needs_quest: false,
            loot_mode: super::LOOT_MODE_DEFAULT_LIKE_CPP,
            group_id: 2,
            min_count: 1,
            max_count: 1,
        },
        LootStoreItem {
            item_id: 0,
            reference: 700,
            chance: 100.0,
            needs_quest: false,
            loot_mode: super::LOOT_MODE_DEFAULT_LIKE_CPP,
            group_id: 2,
            min_count: 1,
            max_count: 1,
        },
    ];

    let frame = super::disenchant_loot_template_frame_like_cpp(rows, 0);

    assert_eq!(frame.template.entries().len(), 2);
    assert_eq!(frame.template.groups().len(), 2);
    assert_eq!(frame.template.groups()[1].equal_chanced().len(), 1);
    assert_eq!(frame.template.entries()[1].reference, 700);
    assert_eq!(frame.template.entries()[1].group_id, 2);
}

#[test]
fn represented_disenchant_group_roll_uses_caller_rng_like_cpp_count() {
    let rows = vec![LootStoreItem {
        item_id: 10940,
        reference: 0,
        chance: 100.0,
        needs_quest: false,
        loot_mode: super::LOOT_MODE_DEFAULT_LIKE_CPP,
        group_id: 1,
        min_count: 2,
        max_count: 7,
    }];
    let frame = super::disenchant_loot_template_frame_like_cpp(rows, 1);
    let group = &frame.template.groups()[0];

    let mut expected_rng = StdRng::seed_from_u64(0xD15E);
    let _expected_group =
        group.roll_like_cpp(super::LOOT_MODE_DEFAULT_LIKE_CPP, &mut expected_rng, |_| {
            true
        });
    let expected_count = expected_rng.gen_range(2..=7);

    let mut rng = StdRng::seed_from_u64(0xD15E);
    let row = group
        .roll_like_cpp(super::LOOT_MODE_DEFAULT_LIKE_CPP, &mut rng, |_| true)
        .expect("group should roll the guaranteed disenchant row");
    let count = rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));

    assert_eq!(row.item_id, 10940);
    assert_eq!(count, expected_count);
}

#[tokio::test]
async fn loot_unit_group_loot_single_candidate_unblocks_under_threshold_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_050);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    assert!(send_rx.try_recv().is_err());

    let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
    assert!(!entry.flags.blocked);
    assert!(entry.flags.under_threshold);
}

#[tokio::test]
async fn loot_unit_group_loot_pass_on_loot_suppresses_current_prompt_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_051);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(4);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    session.pass_on_group_loot = true;
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let local_auto_pass = send_rx.try_recv().unwrap();
    let mut local_auto_pass = WorldPacket::from_bytes(&local_auto_pass);
    assert_eq!(
        local_auto_pass.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(local_auto_pass.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_auto_pass.read_packed_guid().unwrap(), player_guid);
    assert_eq!(local_auto_pass.read_int32().unwrap(), -1);
    assert_eq!(
        local_auto_pass.read_uint8().unwrap(),
        ROLL_VOTE_PASS_LIKE_CPP
    );
    assert!(send_rx.try_recv().is_err());

    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let remote_start_roll = candidate_rx.try_recv().unwrap();
    let mut remote_start_roll = WorldPacket::from_bytes(&remote_start_roll);
    assert_eq!(
        remote_start_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::StartLootRoll as u16
    );
    let remote_auto_pass = candidate_rx.try_recv().unwrap();
    let mut remote_auto_pass = WorldPacket::from_bytes(&remote_auto_pass);
    assert_eq!(
        remote_auto_pass.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(remote_auto_pass.read_packed_guid().unwrap(), loot_object);
    assert_eq!(remote_auto_pass.read_packed_guid().unwrap(), player_guid);
    assert_eq!(remote_auto_pass.read_int32().unwrap(), -1);
    assert_eq!(
        remote_auto_pass.read_uint8().unwrap(),
        ROLL_VOTE_PASS_LIKE_CPP
    );

    let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
    assert!(entry.flags.blocked);
    assert!(!entry.flags.under_threshold);
}

#[tokio::test]
async fn loot_roll_need_vote_broadcasts_immediate_roll_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(5);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_052);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(5);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let _start_roll = send_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();
    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_NEED_LIKE_CPP,
        })
        .await;

    let local_roll = send_rx.try_recv().unwrap();
    let mut local_roll = WorldPacket::from_bytes(&local_roll);
    assert_eq!(
        local_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(local_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_roll.read_packed_guid().unwrap(), player_guid);
    assert_eq!(local_roll.read_int32().unwrap(), 0);
    assert_eq!(local_roll.read_uint8().unwrap(), ROLL_VOTE_NEED_LIKE_CPP);
    assert_eq!(local_roll.read_int32().unwrap(), 0);
    assert_eq!(local_roll.read_bits(2).unwrap(), 0);
    assert_eq!(local_roll.read_bits(3).unwrap(), 1);

    let remote_roll = candidate_rx.try_recv().unwrap();
    let mut remote_roll = WorldPacket::from_bytes(&remote_roll);
    assert_eq!(
        remote_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(remote_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(remote_roll.read_packed_guid().unwrap(), player_guid);
}

#[tokio::test]
async fn loot_roll_all_voted_finishes_need_winner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_053);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (player_tx, player_rx) = flume::bounded::<Vec<u8>>(8);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        player_guid,
        broadcast_info(player_guid, player_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let _start_roll = send_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();

    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_NEED_LIKE_CPP,
        })
        .await;
    let _local_need_roll = send_rx.try_recv().unwrap();
    let _remote_need_roll = candidate_rx.try_recv().unwrap();

    session.set_player_guid(Some(candidate_guid));
    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        })
        .await;

    let local_greed_roll = send_rx.try_recv().unwrap();
    let mut local_greed_roll = WorldPacket::from_bytes(&local_greed_roll);
    assert_eq!(
        local_greed_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(local_greed_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_greed_roll.read_packed_guid().unwrap(), candidate_guid);

    let mut local_won_locked =
        recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootRollWon);
    assert_eq!(local_won_locked.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_won_locked.read_packed_guid().unwrap(), player_guid);
    let winner_roll = local_won_locked.read_int32().unwrap();
    assert!((1..=100).contains(&winner_roll));
    assert_eq!(
        local_won_locked.read_uint8().unwrap(),
        ROLL_VOTE_NEED_LIKE_CPP
    );
    assert_eq!(local_won_locked.read_int32().unwrap(), 0);
    assert_eq!(local_won_locked.read_bits(2).unwrap(), 0);
    assert_eq!(local_won_locked.read_bits(3).unwrap(), 2);

    let mut original_greed_roll =
        recv_packet_with_opcode(&player_rx, wow_constants::ServerOpcodes::LootRoll);
    assert_eq!(original_greed_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(
        original_greed_roll.read_packed_guid().unwrap(),
        candidate_guid
    );

    let final_replay_to_winner =
        recv_packet_with_opcode(&player_rx, wow_constants::ServerOpcodes::LootRoll);
    let mut final_replay_to_winner = final_replay_to_winner;
    assert!(matches!(
        final_replay_to_winner.read_packed_guid().unwrap(),
        guid if guid == loot_object
    ));
    let _replay_player = final_replay_to_winner.read_packed_guid().unwrap();
    let replay_roll = final_replay_to_winner.read_int32().unwrap();
    assert!((0..=100).contains(&replay_roll));

    let mut original_won_allow =
        recv_packet_with_opcode(&player_rx, wow_constants::ServerOpcodes::LootRollWon);
    assert_eq!(original_won_allow.read_packed_guid().unwrap(), loot_object);
    assert_eq!(original_won_allow.read_packed_guid().unwrap(), player_guid);
    let _roll = original_won_allow.read_int32().unwrap();
    assert_eq!(
        original_won_allow.read_uint8().unwrap(),
        ROLL_VOTE_NEED_LIKE_CPP
    );
    assert_eq!(original_won_allow.read_int32().unwrap(), 0);
    assert_eq!(original_won_allow.read_bits(2).unwrap(), 0);
    assert_eq!(original_won_allow.read_bits(3).unwrap(), 0);

    let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
    assert!(!entry.flags.blocked);
    assert_eq!(entry.roll_winner, player_guid);
    assert!(
        !session
            .represented_loot_rolls
            .contains_key(&(loot_object, 0))
    );
    assert_eq!(
        session.represented_loot_roll_criteria_events[0],
        RepresentedLootRollCriteriaEvent::RollAnyNeed {
            player_guid,
            quantity: 1
        }
    );
    assert_eq!(
        session.represented_loot_roll_criteria_events[1],
        RepresentedLootRollCriteriaEvent::RollAnyGreed {
            player_guid: candidate_guid,
            quantity: 1
        }
    );
    match session.represented_loot_roll_criteria_events[2] {
        RepresentedLootRollCriteriaEvent::RollNeed {
            player_guid: criteria_player,
            item_id,
            roll_number,
        } => {
            assert_eq!(criteria_player, player_guid);
            assert_eq!(item_id, 25);
            assert!((1..=100).contains(&roll_number));
        }
        other => panic!("unexpected criteria event: {other:?}"),
    }
}

#[tokio::test]
async fn loot_roll_timer_expiry_finishes_current_winner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_057);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let _start_roll = send_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();

    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        })
        .await;
    let _local_greed_roll = send_rx.try_recv().unwrap();
    let _remote_greed_roll = candidate_rx.try_recv().unwrap();

    session
        .represented_loot_rolls
        .get_mut(&(loot_object, 0))
        .unwrap()
        .end_time = Instant::now() - Duration::from_millis(1);
    session.tick_represented_loot_rolls_like_cpp().await;

    let mut local_final_replay =
        recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootRoll);
    assert_eq!(local_final_replay.read_packed_guid().unwrap(), loot_object);
    let _replay_player = local_final_replay.read_packed_guid().unwrap();

    let mut local_won_allow =
        recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootRollWon);
    assert_eq!(local_won_allow.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_won_allow.read_packed_guid().unwrap(), player_guid);
    assert!((1..=100).contains(&local_won_allow.read_int32().unwrap()));
    assert_eq!(
        local_won_allow.read_uint8().unwrap(),
        ROLL_VOTE_GREED_LIKE_CPP
    );

    let mut remote_final_replay =
        recv_packet_with_opcode(&candidate_rx, wow_constants::ServerOpcodes::LootRoll);
    assert_eq!(remote_final_replay.read_packed_guid().unwrap(), loot_object);
    let _remote_replay_player = remote_final_replay.read_packed_guid().unwrap();

    let mut remote_won_locked =
        recv_packet_with_opcode(&candidate_rx, wow_constants::ServerOpcodes::LootRollWon);
    assert_eq!(remote_won_locked.read_packed_guid().unwrap(), loot_object);
    assert_eq!(remote_won_locked.read_packed_guid().unwrap(), player_guid);
    assert!((1..=100).contains(&remote_won_locked.read_int32().unwrap()));
    assert_eq!(
        remote_won_locked.read_uint8().unwrap(),
        ROLL_VOTE_GREED_LIKE_CPP
    );

    let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
    assert!(!entry.flags.blocked);
    assert_eq!(entry.roll_winner, player_guid);
    assert!(
        !session
            .represented_loot_rolls
            .contains_key(&(loot_object, 0))
    );
    assert_eq!(
        session.represented_loot_roll_criteria_events[0],
        RepresentedLootRollCriteriaEvent::RollAnyGreed {
            player_guid,
            quantity: 1
        }
    );
    match session.represented_loot_roll_criteria_events[1] {
        RepresentedLootRollCriteriaEvent::RollGreed {
            player_guid: criteria_player,
            item_id,
            roll_number,
        } => {
            assert_eq!(criteria_player, player_guid);
            assert_eq!(item_id, 25);
            assert!((1..=100).contains(&roll_number));
        }
        other => panic!("unexpected criteria event: {other:?}"),
    }
}

#[tokio::test]
async fn stale_loot_roll_vote_does_not_mutate_replacement_generation_like_cpp() {
    let (mut session, send_rx, candidate_rx, player_guid, candidate_guid, owner_guid) =
        open_generation_guarded_group_roll_like_cpp(19_060).await;
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let old_generation = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .unwrap()
        .authority_generation;
    let replacement_generation = replace_generation_guarded_group_loot_like_cpp(
        &mut session,
        owner_guid,
        player_guid,
        candidate_guid,
    );
    assert_ne!(old_generation, replacement_generation);

    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_NEED_LIKE_CPP,
        })
        .await;

    assert!(
        !session
            .represented_loot_rolls
            .contains_key(&(loot_object, 0)),
        "the stale roll must be cancelled instead of routed or voted"
    );
    assert!(send_rx.try_recv().is_err());
    assert!(candidate_rx.try_recv().is_err());
    assert!(session.represented_loot_roll_criteria_events.is_empty());

    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .unwrap();
    let replacement = authority.shared_snapshot_like_cpp().unwrap();
    assert_eq!(replacement.generation, replacement_generation);
    let entry = &replacement.loot.items[0];
    assert!(entry.flags.blocked);
    assert!(entry.roll_winner.is_empty());
    assert!(!entry.taken);
    assert_eq!(replacement.loot.unlooted_count, 1);
}

#[tokio::test]
async fn stale_loot_roll_expiry_does_not_mutate_replacement_generation_like_cpp() {
    let (mut session, send_rx, candidate_rx, player_guid, candidate_guid, owner_guid) =
        open_generation_guarded_group_roll_like_cpp(19_061).await;
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let old_generation = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .unwrap()
        .authority_generation;
    let replacement_generation = replace_generation_guarded_group_loot_like_cpp(
        &mut session,
        owner_guid,
        player_guid,
        candidate_guid,
    );
    assert_ne!(old_generation, replacement_generation);
    session
        .represented_loot_rolls
        .get_mut(&(loot_object, 0))
        .unwrap()
        .end_time = Instant::now() - Duration::from_millis(1);

    session.tick_represented_loot_rolls_like_cpp().await;

    assert!(
        !session
            .represented_loot_rolls
            .contains_key(&(loot_object, 0)),
        "the stale timer must be cancelled without finishing against replacement loot"
    );
    assert!(send_rx.try_recv().is_err());
    assert!(candidate_rx.try_recv().is_err());
    assert!(session.represented_loot_roll_criteria_events.is_empty());

    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .unwrap();
    let replacement = authority.shared_snapshot_like_cpp().unwrap();
    assert_eq!(replacement.generation, replacement_generation);
    let entry = &replacement.loot.items[0];
    assert!(entry.flags.blocked);
    assert!(entry.roll_winner.is_empty());
    assert!(!entry.taken);
    assert_eq!(replacement.loot.unlooted_count, 1);
}

#[tokio::test]
async fn loot_roll_all_passed_unblocks_without_all_passed_to_valid_voters_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_054);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (player_tx, player_rx) = flume::bounded::<Vec<u8>>(8);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        player_guid,
        broadcast_info(player_guid, player_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let _start_roll = send_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();

    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_PASS_LIKE_CPP,
        })
        .await;
    let _local_pass_roll = send_rx.try_recv().unwrap();
    let _remote_pass_roll = candidate_rx.try_recv().unwrap();
    let pass_state = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .expect("roll state should stay open until every voter passes");
    assert_eq!(
        pass_state.voters.get(&player_guid).unwrap().roll_number,
        0,
        "C++ LootRoll::PlayerVote does not call urand for Pass"
    );

    session.set_player_guid(Some(candidate_guid));
    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_PASS_LIKE_CPP,
        })
        .await;

    let candidate_pass_roll = send_rx.try_recv().unwrap();
    let mut candidate_pass_roll = WorldPacket::from_bytes(&candidate_pass_roll);
    assert_eq!(
        candidate_pass_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(candidate_pass_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(
        candidate_pass_roll.read_packed_guid().unwrap(),
        candidate_guid
    );
    assert_eq!(candidate_pass_roll.read_int32().unwrap(), -1);
    assert_eq!(
        candidate_pass_roll.read_uint8().unwrap(),
        ROLL_VOTE_PASS_LIKE_CPP
    );

    let original_pass_roll = player_rx.try_recv().unwrap();
    let mut original_pass_roll = WorldPacket::from_bytes(&original_pass_roll);
    assert_eq!(
        original_pass_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert!(send_rx.try_recv().is_err());
    assert!(player_rx.try_recv().is_err());
    assert!(candidate_rx.try_recv().is_err());

    let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
    assert!(!entry.flags.blocked);
    assert!(entry.roll_winner.is_empty());
    assert!(
        !session
            .represented_loot_rolls
            .contains_key(&(loot_object, 0))
    );
}

#[tokio::test]
async fn loot_roll_vote_command_updates_owner_session_roll_state_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_055);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let _start_roll = send_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();
    let roll_identity = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .unwrap()
        .command_identity
        .clone();

    session
        .session_command_tx()
        .send(SessionCommand::LootRollVote(LootRollVoteCommand {
            voter_guid: candidate_guid,
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_GREED_LIKE_CPP,
            pass_on_group_loot: false,
            roll_identity,
        }))
        .unwrap();
    session
        .process_represented_session_commands_like_cpp()
        .await;

    let local_roll = send_rx.try_recv().unwrap();
    let mut local_roll = WorldPacket::from_bytes(&local_roll);
    assert_eq!(
        local_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(local_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_roll.read_packed_guid().unwrap(), candidate_guid);
    assert_eq!(local_roll.read_int32().unwrap(), -1);
    assert_eq!(local_roll.read_uint8().unwrap(), ROLL_VOTE_GREED_LIKE_CPP);

    let remote_roll = candidate_rx.try_recv().unwrap();
    let mut remote_roll = WorldPacket::from_bytes(&remote_roll);
    assert_eq!(
        remote_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(remote_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(remote_roll.read_packed_guid().unwrap(), candidate_guid);

    let state = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .unwrap();
    assert_eq!(
        state.voters.get(&candidate_guid).unwrap().vote,
        ROLL_VOTE_GREED_LIKE_CPP
    );
}

#[test]
fn loot_roll_vote_command_accepts_exact_enqueued_roll_identity_like_cpp() {
    let loot_object = represented_loot_object_guid_like_cpp(test_creature_guid(19_062));
    let authority = OwnedLootAuthority::new();
    let roll_identity = LootRollCommandIdentityLikeCpp::new_like_cpp(loot_object, 0, authority, 7);
    let command = LootRollVoteCommand {
        voter_guid: ObjectGuid::create_player(1, 77),
        loot_obj: loot_object,
        loot_list_id: 0,
        roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        pass_on_group_loot: false,
        roll_identity: roll_identity.clone(),
    };

    assert!(
        WorldSession::represented_loot_roll_vote_command_targets_identity_like_cpp(
            &command,
            &roll_identity,
        )
    );
}

#[test]
fn queued_loot_roll_vote_rejects_replacement_with_same_key_and_generation_like_cpp() {
    let loot_object = represented_loot_object_guid_like_cpp(test_creature_guid(19_063));
    let authority = OwnedLootAuthority::new();
    let stale_identity =
        LootRollCommandIdentityLikeCpp::new_like_cpp(loot_object, 0, authority.clone(), 7);
    let replacement_identity =
        LootRollCommandIdentityLikeCpp::new_like_cpp(loot_object, 0, authority, 7);
    let stale_command = LootRollVoteCommand {
        voter_guid: ObjectGuid::create_player(1, 77),
        loot_obj: loot_object,
        loot_list_id: 0,
        roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        pass_on_group_loot: false,
        roll_identity: stale_identity,
    };

    assert!(
        !WorldSession::represented_loot_roll_vote_command_targets_identity_like_cpp(
            &stale_command,
            &replacement_identity,
        ),
        "a command queued for the destroyed C++ LootRoll* must not vote on its replacement"
    );
}

#[tokio::test]
async fn loot_roll_remote_session_routes_vote_to_owner_session_like_cpp() {
    let (mut owner_session, owner_rx) = make_session_with_send_capacity(8);
    let (mut remote_session, _remote_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_056);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
    let (owner_registry_tx, _owner_registry_rx) = flume::bounded::<Vec<u8>>(8);
    let player_registry = Arc::new(PlayerRegistry::default());

    let mut owner_info = broadcast_info(player_guid, owner_registry_tx);
    owner_info.command_tx = owner_session.session_command_tx();
    player_registry.register_or_replace(player_guid, owner_info, Default::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );

    owner_session.set_player_registry(Arc::clone(&player_registry));
    owner_session.set_player_guid(Some(player_guid));
    remote_session.set_player_registry(Arc::clone(&player_registry));
    remote_session.set_player_guid(Some(candidate_guid));
    install_group_loot_group(&mut owner_session, player_guid, candidate_guid);

    let mut canonical_player = Player::new(Some(1), false);
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    canonical_player
        .unit_mut()
        .world_mut()
        .set_map(u32::from(owner_guid.map_id()), 0)
        .unwrap();
    canonical_player
        .unit_mut()
        .world_mut()
        .relocate(Position::ZERO);
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    let canonical_creature = make_canonical_creature_for_session(&owner_session, owner_guid);
    let canonical_manager = Arc::new(Mutex::new(wow_map::MapManager::default()));
    {
        let mut manager = canonical_manager.lock().unwrap();
        let map = manager.create_world_map(u32::from(owner_guid.map_id()), 0);
        map.map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_player(canonical_player).unwrap(),
            )
            .unwrap();
        map.map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_creature(canonical_creature).unwrap(),
            )
            .unwrap();
    }
    owner_session.set_canonical_map_manager(canonical_manager);
    let loot_object = owner_session
        .next_represented_loot_object_guid_like_cpp(owner_guid)
        .expect("the canonical owner map must allocate the C++ LootObject identity");

    register_test_creature_like_cpp(&mut owner_session, test_creature(owner_guid, false));
    let mut loot = generation_guarded_group_loot_like_cpp(owner_guid, player_guid, candidate_guid);
    loot.loot_guid = loot_object;
    owner_session.loot_table.insert(owner_guid, loot);
    owner_session
        .sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, player_guid)
        .expect("the fixture loot must be installed into the object-owned authority");
    let installed = owner_session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .and_then(|authority| authority.shared_snapshot_like_cpp())
        .expect("the canonical creature must expose the installed shared loot");
    assert_eq!(installed.loot.loot_guid, loot_object);

    owner_session
        .handle_loot_unit(loot_unit_packet(owner_guid))
        .await;
    let _response = owner_rx.try_recv().unwrap();
    let _loot_list = owner_rx.try_recv().unwrap();
    let _start_roll = owner_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();

    assert!(
        player_registry
            .fixture_snapshot(player_guid)
            .unwrap()
            .active_loot_rolls
            .iter()
            .any(|identity| identity.matches_key_like_cpp(loot_object, 0))
    );

    remote_session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        })
        .await;
    owner_session
        .process_represented_session_commands_like_cpp()
        .await;

    let local_roll = owner_rx.try_recv().unwrap();
    let mut local_roll = WorldPacket::from_bytes(&local_roll);
    assert_eq!(
        local_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(local_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_roll.read_packed_guid().unwrap(), candidate_guid);
    assert_eq!(local_roll.read_int32().unwrap(), -1);
    assert_eq!(local_roll.read_uint8().unwrap(), ROLL_VOTE_GREED_LIKE_CPP);

    let remote_roll = candidate_rx.try_recv().unwrap();
    let mut remote_roll = WorldPacket::from_bytes(&remote_roll);
    assert_eq!(
        remote_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(remote_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(remote_roll.read_packed_guid().unwrap(), candidate_guid);
}

#[tokio::test]
async fn loot_unit_new_main_target_releases_existing_view_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let old_guid = test_creature_guid(19_036);
    let new_guid = test_creature_guid(19_037);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(old_guid);
    insert_allowed_coin_loot_like_cpp(&mut session, old_guid, player_guid, 7);
    register_test_creature_like_cpp(&mut session, test_creature(new_guid, false));
    insert_allowed_coin_loot_like_cpp(&mut session, new_guid, player_guid, 7);

    session.handle_loot_unit(loot_unit_packet(new_guid)).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), old_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert!(!session.is_active_loot_guid(old_guid));
    assert!(session.is_active_loot_guid(new_guid));
    assert!(session.loot_table.contains_key(&old_guid));
}

#[tokio::test]
async fn loot_unit_non_creature_guid_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_019);
    session.set_player_guid(Some(player_guid));

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.contains_key(&loot_guid));
}

#[tokio::test]
async fn loot_unit_creature_too_far_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_016);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let mut creature = test_creature(loot_guid, false);
    creature.current_pos = Position::new(31.0, 0.0, 0.0, 0.0);
    register_test_creature_like_cpp(&mut session, creature);

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.contains_key(&loot_guid));
}

#[tokio::test]
async fn loot_unit_response_uses_loot_owner_not_player_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let owner_guid = test_creature_guid(19_022);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    insert_allowed_coin_loot_like_cpp(&mut session, owner_guid, player_guid, 7);

    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    let response_owner = sent.read_packed_guid().unwrap();
    let response_loot_obj = sent.read_packed_guid().unwrap();
    assert_eq!(response_owner, owner_guid);
    assert_eq!(response_loot_obj.high_type(), HighGuid::LootObject);
    assert_ne!(response_loot_obj, owner_guid);
    assert_ne!(owner_guid, player_guid);
    assert_eq!(
        session.loot_table.get(&owner_guid).unwrap().loot_guid,
        response_loot_obj
    );
    assert!(session.is_active_loot_guid(owner_guid));
}

#[tokio::test]
async fn loot_unit_ae_loot_sends_targets_and_secondary_ack_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(5);
    let player_guid = ObjectGuid::create_player(1, 42);
    let main_guid = test_creature_guid(19_031);
    let secondary_guid = test_creature_guid(19_032);
    session.set_player_guid(Some(player_guid));
    session.set_enable_ae_loot_like_cpp(true);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(main_guid, false));
    register_test_creature_like_cpp(&mut session, test_creature(secondary_guid, false));
    insert_allowed_coin_loot_like_cpp(&mut session, main_guid, player_guid, 7);
    insert_allowed_coin_loot_like_cpp(&mut session, secondary_guid, player_guid, 7);

    session.handle_loot_unit(loot_unit_packet(main_guid)).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::AeLootTargets as u16
    );
    assert_eq!(sent.read_uint32().unwrap(), 2);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), main_guid);
    let main_loot_object = sent.read_packed_guid().unwrap();
    assert_eq!(main_loot_object.high_type(), HighGuid::LootObject);
    sent.read_uint8().unwrap();
    sent.read_uint8().unwrap();
    sent.read_uint8().unwrap();
    sent.read_uint8().unwrap();
    sent.read_uint32().unwrap();
    sent.read_int32().unwrap();
    sent.read_int32().unwrap();
    assert!(sent.read_bit().unwrap());
    assert!(!sent.read_bit().unwrap());

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::AeLootTargetAck as u16
    );
    assert!(sent.read_uint8().is_err());

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), secondary_guid);
    let secondary_loot_object = sent.read_packed_guid().unwrap();
    assert_eq!(secondary_loot_object.high_type(), HighGuid::LootObject);
    sent.read_uint8().unwrap();
    sent.read_uint8().unwrap();
    sent.read_uint8().unwrap();
    sent.read_uint8().unwrap();
    sent.read_uint32().unwrap();
    sent.read_int32().unwrap();
    sent.read_int32().unwrap();
    assert!(sent.read_bit().unwrap());
    assert!(sent.read_bit().unwrap());

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::AeLootTargetAck as u16
    );
    assert!(session.is_active_loot_guid(main_guid));
    assert!(session.active_loot_view_owners.contains(&main_guid));
    assert!(session.active_loot_view_owners.contains(&secondary_guid));
}

async fn open_test_ae_pair_like_cpp(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    primary_guid: ObjectGuid,
    secondary_guid: ObjectGuid,
) -> OwnedLootAuthority {
    session.set_player_guid(Some(player_guid));
    session.set_enable_ae_loot_like_cpp(true);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(session, test_creature(primary_guid, false));
    register_test_creature_like_cpp(session, test_creature(secondary_guid, false));
    insert_allowed_coin_loot_like_cpp(session, primary_guid, player_guid, 7);
    insert_allowed_coin_loot_like_cpp(session, secondary_guid, player_guid, 7);

    session
        .handle_loot_unit(loot_unit_packet(primary_guid))
        .await;

    assert!(session.is_active_loot_guid(primary_guid));
    assert!(session.active_loot_view_owners.contains(&secondary_guid));
    let authority = session
        .represented_owned_loot_authority_like_cpp(secondary_guid)
        .expect("the secondary AE owner must expose its object-owned authority");
    assert!(
        authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .players_looting
            .contains(&player_guid)
    );
    authority
}

#[tokio::test]
async fn disconnect_after_primary_ae_release_closes_secondary_view_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let player_guid = ObjectGuid::create_player(1, 61_711);
    let primary_guid = test_creature_guid(61_712);
    let secondary_guid = test_creature_guid(61_713);
    let secondary_authority =
        open_test_ae_pair_like_cpp(&mut session, player_guid, primary_guid, secondary_guid).await;

    session
        .handle_loot_release(loot_release_packet(primary_guid))
        .await;
    assert!(session.active_loot_guid.is_empty());
    assert!(session.active_loot_view_owners.contains(&secondary_guid));

    session
        .cleanup_shared_runtime_state_on_disconnect_like_cpp()
        .await;

    assert!(session.active_loot_view_owners.is_empty());
    assert!(
        !secondary_authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .players_looting
            .contains(&player_guid),
        "logout must remove the secondary AE viewer even after the primary was released"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        2
    );
}

#[tokio::test]
async fn new_loot_after_primary_ae_release_closes_secondary_before_replacing_tracking_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let player_guid = ObjectGuid::create_player(1, 61_714);
    let primary_guid = test_creature_guid(61_715);
    let secondary_guid = test_creature_guid(61_716);
    let new_guid = test_creature_guid(61_717);
    let secondary_authority =
        open_test_ae_pair_like_cpp(&mut session, player_guid, primary_guid, secondary_guid).await;

    session
        .handle_loot_release(loot_release_packet(primary_guid))
        .await;
    assert!(session.active_loot_guid.is_empty());
    assert!(session.active_loot_view_owners.contains(&secondary_guid));

    session.set_enable_ae_loot_like_cpp(false);
    register_test_creature_like_cpp(&mut session, test_creature(new_guid, false));
    insert_allowed_coin_loot_like_cpp(&mut session, new_guid, player_guid, 7);
    session.handle_loot_unit(loot_unit_packet(new_guid)).await;

    assert!(session.is_active_loot_guid(new_guid));
    assert_eq!(session.active_loot_view_owners.len(), 1);
    assert!(!session.active_loot_view_owners.contains(&secondary_guid));
    assert!(
        !secondary_authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .players_looting
            .contains(&player_guid),
        "the secondary AE viewer must be released before set_active_loot_guid clears tracking"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        2
    );
}

#[tokio::test]
async fn item_loot_releases_ae_view_and_tracks_multiple_items_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let player_guid = ObjectGuid::create_player(1, 61_718);
    let primary_guid = test_creature_guid(61_719);
    let secondary_guid = test_creature_guid(61_720);
    let first_item = ObjectGuid::create_item(1, 61_721);
    let second_item = ObjectGuid::create_item(1, 61_722);
    let secondary_authority =
        open_test_ae_pair_like_cpp(&mut session, player_guid, primary_guid, secondary_guid).await;

    session
        .handle_loot_release(loot_release_packet(primary_guid))
        .await;
    assert!(session.active_loot_guid.is_empty());
    assert!(session.active_loot_view_owners.contains(&secondary_guid));

    session
        .open_active_item_loot_view_like_cpp(player_guid, first_item)
        .await;
    session
        .open_active_item_loot_view_like_cpp(player_guid, second_item)
        .await;

    assert!(session.is_active_loot_guid(first_item));
    assert_eq!(session.active_loot_view_owners.len(), 2);
    assert!(session.active_loot_view_owners.contains(&first_item));
    assert!(session.active_loot_view_owners.contains(&second_item));
    assert!(!session.active_loot_view_owners.contains(&secondary_guid));
    assert!(
        !secondary_authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .players_looting
            .contains(&player_guid),
        "item loot must release the surviving secondary AE viewer first"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        2
    );
}

#[tokio::test]
async fn loot_unit_empty_visible_loot_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_007);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, loot_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_unit_fully_looted_existing_loot_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_017);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: true,
            }],
            looted_by_player: false,
        },
    );

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_unit_without_allowed_loot_for_player_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_018);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![other_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, loot_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.loot_table.get(&loot_guid).unwrap().items[0].allowed_looters,
        vec![other_guid]
    );
    assert!(!session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_unit_non_tapper_existing_tap_list_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_098);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    tap_test_creature_like_cpp(&mut session, loot_guid, other_guid);

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.contains_key(&loot_guid));
}

#[tokio::test]
async fn loot_unit_existing_coin_loot_without_allowed_looter_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_099);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    insert_allowed_coin_loot_like_cpp(&mut session, loot_guid, other_guid, 7);

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert_eq!(session.loot_table.get(&loot_guid).unwrap().coins, 7);
    assert_eq!(
        session.loot_table.get(&loot_guid).unwrap().allowed_looters,
        vec![other_guid]
    );
}

#[tokio::test]
async fn loot_money_non_allowed_active_creature_does_not_take_coins_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_100);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    insert_allowed_coin_loot_like_cpp(&mut session, loot_guid, other_guid, 7);

    session.handle_loot_money(loot_money_packet()).await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(session.player_gold_like_cpp(), 0);
    assert_eq!(session.loot_table.get(&loot_guid).unwrap().coins, 7);
}

#[tokio::test]
async fn loot_item_uses_active_loot_view_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let active_guid = test_creature_guid(19_001);
    let inactive_guid = test_creature_guid(19_002);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(active_guid);
    session.loot_table.insert(
        inactive_guid,
        CreatureLoot {
            loot_guid: inactive_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: Vec::new(),
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(inactive_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!session.loot_table.get(&inactive_guid).unwrap().items[0].taken);
}

#[tokio::test]
async fn loot_money_stale_active_without_loot_view_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_020);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);

    session.handle_loot_money(loot_money_packet()).await;

    assert!(session.is_active_loot_guid(loot_guid));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn loot_money_zero_money_still_notifies_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_021);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session.handle_loot_money(loot_money_packet()).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootMoneyNotify as u16
    );
    assert_eq!(sent.read_uint64().unwrap(), 0);
    assert_eq!(sent.read_uint64().unwrap(), 0);
    assert!(sent.read_bit().unwrap());
    assert_eq!(session.player_gold_like_cpp(), 0);
    assert!(session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_money_coin_removed_uses_loot_object_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let owner_guid = test_creature_guid(19_024);
    let loot_object_guid = represented_loot_object_guid_like_cpp(owner_guid);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(owner_guid);
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object_guid,
            coins: 3,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![],
            looted_by_player: false,
        },
    );

    session.handle_loot_money(loot_money_packet()).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object_guid);
    assert!(session.is_active_loot_guid(owner_guid));
}

#[tokio::test]
async fn loot_money_consumes_all_active_loot_views_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_loot_money_persistence_test_result_like_cpp(true);
    let player_guid = ObjectGuid::create_player(1, 42);
    let owner_one = test_creature_guid(19_025);
    let owner_two = test_creature_guid(19_026);
    let loot_object_one = represented_loot_object_guid_like_cpp(owner_one);
    let loot_object_two = represented_loot_object_guid_like_cpp(owner_two);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(owner_one);
    session.add_active_loot_view_owner_like_cpp(owner_two);
    session.loot_table.insert(
        owner_one,
        CreatureLoot {
            loot_guid: loot_object_one,
            coins: 3,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![],
            looted_by_player: false,
        },
    );
    session.loot_table.insert(
        owner_two,
        CreatureLoot {
            loot_guid: loot_object_two,
            coins: 7,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![],
            looted_by_player: false,
        },
    );

    session.handle_loot_money(loot_money_packet()).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object_one);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootMoneyNotify as u16
    );
    assert_eq!(sent.read_uint64().unwrap(), 3);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object_two);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootMoneyNotify as u16
    );
    assert_eq!(sent.read_uint64().unwrap(), 7);
    assert_eq!(session.player_gold_like_cpp(), 10);
    assert_eq!(session.loot_table.get(&owner_one).unwrap().coins, 0);
    assert_eq!(session.loot_table.get(&owner_two).unwrap().coins, 0);
    assert!(session.active_loot_view_owners.contains(&owner_one));
    assert!(session.active_loot_view_owners.contains(&owner_two));
}

#[tokio::test]
async fn loot_money_gain_completes_money_tracking_event_objective_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(5);
    session.set_loot_money_persistence_test_result_like_cpp(true);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_029);
    let quest_id = 12_530;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 8, // C++ QUEST_OBJECTIVE_MONEY.
        order: 0,
        storage_index: -1,
        object_id: 0,
        amount: 7,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );
    insert_allowed_coin_loot_like_cpp(&mut session, loot_guid, player_guid, 7);

    session.handle_loot_money(loot_money_packet()).await;

    assert_eq!(session.player_gold_like_cpp(), 7);
    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootMoneyNotify as u16
    );
    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::UpdateObject as u16
    );
    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16
    );
    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::QuestUpdateComplete as u16
    );
}

#[tokio::test]
async fn loot_money_splits_corpse_gold_to_near_group_members_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_loot_money_persistence_test_result_like_cpp(true);
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_027);
    let (other_tx, other_rx) = flume::bounded::<Vec<u8>>(2);
    let player_registry = Arc::new(PlayerRegistry::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.add_member(other_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.register_or_replace(
        other_guid,
        broadcast_info(other_guid, other_tx),
        Default::default(),
    );

    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_active_loot_guid(loot_guid);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 9,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, other_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid, other_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session.handle_loot_money(loot_money_packet()).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootMoneyNotify as u16
    );
    assert_eq!(sent.read_uint64().unwrap(), 4);
    assert_eq!(sent.read_uint64().unwrap(), 0);
    assert!(!sent.read_bit().unwrap());

    let sent = other_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootMoneyNotify as u16
    );
    assert_eq!(sent.read_uint64().unwrap(), 4);
    assert_eq!(sent.read_uint64().unwrap(), 0);
    assert!(!sent.read_bit().unwrap());
    assert_eq!(session.player_gold_like_cpp(), 4);
    assert_eq!(session.loot_table.get(&loot_guid).unwrap().coins, 0);
}

#[tokio::test]
async fn loot_item_releases_blocked_item_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_003);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
}

#[tokio::test]
async fn loot_item_releases_when_player_is_not_allowed_looter_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_004);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![other_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
}

#[tokio::test]
async fn loot_item_releases_when_roll_winner_is_different_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let winner_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_005);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: winner_guid,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
}

#[tokio::test]
async fn loot_roll_without_canonical_roll_state_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_loot_roll(LootRoll {
            loot_obj: test_creature_guid(19_006),
            loot_list_id: 0,
            roll_type: 1,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_loot_specialization_matches_cpp_class_validation() {
    let (mut session, send_rx) = make_session_with_send();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_player_class_like_cpp(2);
    session.set_chr_specialization_store(Arc::new(ChrSpecializationStore::from_entries([
        ChrSpecializationEntry {
            id: 65,
            class_id: 2,
            order_index: 0,
            role: 0,
        },
        ChrSpecializationEntry {
            id: 71,
            class_id: 1,
            order_index: 0,
            role: 0,
        },
    ])));

    session
        .handle_set_loot_specialization(SetLootSpecialization { spec_id: 65 })
        .await;
    assert_eq!(session.loot_specialization_id_like_cpp(), 65);

    session
        .handle_set_loot_specialization(SetLootSpecialization { spec_id: 71 })
        .await;
    assert_eq!(session.loot_specialization_id_like_cpp(), 65);

    session
        .handle_set_loot_specialization(SetLootSpecialization { spec_id: 999 })
        .await;
    assert_eq!(session.loot_specialization_id_like_cpp(), 65);

    session
        .handle_set_loot_specialization(SetLootSpecialization { spec_id: 0 })
        .await;
    assert_eq!(session.loot_specialization_id_like_cpp(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_loot_specialization_without_loaded_player_is_ignored_like_cpp_status_guard() {
    let (mut session, _send_rx) = make_session_with_send();
    session.set_player_class_like_cpp(2);
    session.set_chr_specialization_store(Arc::new(ChrSpecializationStore::from_entries([
        ChrSpecializationEntry {
            id: 65,
            class_id: 2,
            order_index: 0,
            role: 0,
        },
    ])));

    session
        .handle_set_loot_specialization(SetLootSpecialization { spec_id: 65 })
        .await;

    assert_eq!(session.loot_specialization_id_like_cpp(), 0);
}

#[tokio::test]
async fn master_loot_item_without_group_sends_didnt_kill_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_master_loot_item(MasterLootItem {
            target: ObjectGuid::create_player(1, 77),
            loot: Vec::new(),
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(
        sent.read_uint8().unwrap(),
        wow_packet::packets::loot::LOOT_ERROR_DIDNT_KILL_LIKE_CPP
    );
}

#[tokio::test]
async fn master_loot_item_uses_group_master_looter_guid_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader_guid = ObjectGuid::create_player(1, 42);
    let master_guid = ObjectGuid::create_player(1, 43);
    let (leader_tx, _leader_rx) = flume::bounded::<Vec<u8>>(2);
    let player_registry = Arc::new(PlayerRegistry::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader_guid);
    group.add_member(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.register_or_replace(
        leader_guid,
        broadcast_info(leader_guid, leader_tx),
        Default::default(),
    );
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(leader_guid));

    session
        .handle_master_loot_item(MasterLootItem {
            target: master_guid,
            loot: Vec::new(),
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(
        sent.read_uint8().unwrap(),
        wow_packet::packets::loot::LOOT_ERROR_DIDNT_KILL_LIKE_CPP
    );

    session.set_player_guid(Some(master_guid));
    session
        .handle_master_loot_item(MasterLootItem {
            target: leader_guid,
            loot: Vec::new(),
        })
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn master_loot_item_missing_target_sends_player_not_found_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let missing_target = ObjectGuid::create_player(1, 77);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));

    session
        .handle_master_loot_item(MasterLootItem {
            target: missing_target,
            loot: Vec::new(),
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(
        sent.read_uint8().unwrap(),
        LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP
    );
}

#[tokio::test]
async fn master_loot_item_non_master_loot_view_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let loot_owner = test_creature_guid(19_082);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));
    session.set_active_loot_guid(loot_owner);
    session.loot_table.insert(
        loot_owner,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![master_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_master_loot_item(MasterLootItem {
            target: master_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        })
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn master_loot_item_ineligible_target_sends_master_other_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let target_guid = ObjectGuid::create_player(1, 77);
    let loot_owner = test_creature_guid(19_080);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
    let (target_tx, _target_rx) = flume::bounded::<Vec<u8>>(2);
    let player_registry = Arc::new(PlayerRegistry::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.register_or_replace(
        target_guid,
        broadcast_info(target_guid, target_tx),
        Default::default(),
    );
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));
    session.set_active_loot_guid(loot_owner);
    session.loot_table.insert(
        loot_owner,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![master_guid, target_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_master_loot_item(MasterLootItem {
            target: target_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
    assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_MASTER_OTHER_LIKE_CPP);
}

#[tokio::test]
async fn master_loot_item_target_not_allowed_for_loot_sends_master_other_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let loot_owner = test_creature_guid(19_083);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));
    session.set_active_loot_guid(loot_owner);
    session.loot_table.insert(
        loot_owner,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![master_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_master_loot_item(MasterLootItem {
            target: master_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
    assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_MASTER_OTHER_LIKE_CPP);
}

#[test]
fn master_loot_inventory_result_mapping_matches_cpp_errors() {
    assert_eq!(
        super::master_loot_error_for_inventory_result_like_cpp(InventoryResult::Ok),
        None
    );
    assert_eq!(
        super::master_loot_error_for_inventory_result_like_cpp(InventoryResult::ItemMaxCount),
        Some(LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP)
    );
    assert_eq!(
        super::master_loot_error_for_inventory_result_like_cpp(InventoryResult::InvFull),
        Some(wow_packet::packets::loot::LOOT_ERROR_MASTER_INV_FULL_LIKE_CPP)
    );
    assert_eq!(
        super::master_loot_error_for_inventory_result_like_cpp(InventoryResult::CantEquipEver),
        Some(LOOT_ERROR_MASTER_OTHER_LIKE_CPP)
    );
}

#[tokio::test]
async fn master_loot_item_self_target_can_store_maps_unique_error_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 700);
    let loot_owner = test_creature_guid(19_081);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));
    session.set_active_loot_guid(loot_owner);
    install_limited_test_item_template(&mut session, 700, 1);
    session.insert_inventory_item_like_cpp(
        35,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: 700,
            inventory_type: None,
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        700,
        master_guid,
        1,
        0,
        ItemContext::None,
        35,
    );
    session.insert_inventory_item_object(item);
    session.loot_table.insert(
        loot_owner,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![master_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 700,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![master_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_master_loot_item(MasterLootItem {
            target: master_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
    assert_eq!(
        sent.read_uint8().unwrap(),
        LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP
    );
}

#[tokio::test]
async fn master_loot_item_self_target_success_marks_removed_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let loot_owner = test_creature_guid(19_082);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);

    session.loot_table.insert(
        loot_owner,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![master_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 701,
                quantity: 3,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![master_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session.mark_represented_master_loot_item_removed_like_cpp(
        loot_owner,
        loot_object,
        0,
        master_guid,
    );

    let loot = session.loot_table.get(&loot_owner).unwrap();
    assert_eq!(loot.items[0].quantity, 0);
    assert!(loot.items[0].is_looted_for_player_like_cpp(master_guid));
    assert_eq!(loot.unlooted_count, 0);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
    assert_eq!(sent.read_uint8().unwrap(), 0);
}

#[tokio::test]
async fn master_loot_item_remote_target_can_store_error_is_reported_by_target_session_like_cpp() {
    let (mut master_session, master_rx) = make_session_with_send();
    let (mut target_session, _target_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let target_guid = ObjectGuid::create_player(1, 77);
    let existing_item_guid = ObjectGuid::create_item(1, 701);
    let loot_owner = test_creature_guid(19_083);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    group.members.push(target_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_send_tx, _target_send_rx) = flume::bounded::<Vec<u8>>(2);
    let mut target_info = broadcast_info(target_guid, target_send_tx);
    target_info.command_tx = target_session.session_command_tx();
    player_registry.register_or_replace(target_guid, target_info, Default::default());

    master_session.group_guid = Some(group_guid);
    master_session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    master_session.set_player_registry(Arc::clone(&player_registry));
    master_session.set_player_guid(Some(master_guid));
    master_session.set_active_loot_guid(loot_owner);
    master_session.loot_table.insert(
        loot_owner,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![target_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 701,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![target_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    target_session.set_player_guid(Some(target_guid));
    install_limited_test_item_template(&mut target_session, 701, 1);
    target_session.insert_inventory_item_like_cpp(
        35,
        InventoryItem {
            guid: existing_item_guid,
            entry_id: 701,
            db_guid: 701,
            inventory_type: None,
        },
    );
    let item = target_session.make_inventory_item_object(
        existing_item_guid,
        701,
        target_guid,
        1,
        0,
        ItemContext::None,
        35,
    );
    target_session.insert_inventory_item_object(item);

    let master_future = master_session.handle_master_loot_item(MasterLootItem {
        target: target_guid,
        loot: vec![wow_packet::packets::loot::LootItemRequest {
            object: loot_object,
            loot_list_id: 0,
        }],
    });
    let target_future = async {
        for _ in 0..8 {
            target_session.process_pending().await;
            tokio::task::yield_now().await;
        }
    };
    tokio::join!(master_future, target_future);

    let sent = master_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
    assert_eq!(
        sent.read_uint8().unwrap(),
        LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP
    );
    assert!(!master_session.loot_table.get(&loot_owner).unwrap().items[0].taken);
}

#[tokio::test]
async fn master_loot_item_remote_target_unavailable_command_reports_player_not_found_like_cpp() {
    let (mut master_session, master_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let target_guid = ObjectGuid::create_player(1, 77);
    let loot_owner = test_creature_guid(19_084);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    group.members.push(target_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_send_tx, _target_send_rx) = flume::bounded::<Vec<u8>>(2);
    let (command_tx, _command_rx) = flume::bounded(0);
    let mut target_info = broadcast_info(target_guid, target_send_tx);
    target_info.command_tx = command_tx;
    player_registry.register_or_replace(target_guid, target_info, Default::default());

    master_session.group_guid = Some(group_guid);
    master_session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    master_session.set_player_registry(player_registry);
    master_session.set_player_guid(Some(master_guid));
    master_session.set_active_loot_guid(loot_owner);
    master_session.loot_table.insert(
        loot_owner,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![target_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 702,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![target_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    master_session
        .handle_master_loot_item(MasterLootItem {
            target: target_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        })
        .await;

    let sent = master_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(
        sent.read_uint8().unwrap(),
        LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP
    );
}

#[tokio::test]
async fn loot_item_creature_too_far_uses_cpp_error() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_008);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);

    let mut creature = test_creature(loot_guid, false);
    creature.current_pos = Position::new(31.0, 0.0, 0.0, 0.0);
    register_test_creature_like_cpp(&mut session, creature);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        loot_response_failure_reason(&sent),
        LOOT_ERROR_TOO_FAR_LIKE_CPP
    );
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_item_creature_distance_can_use_canonical_map_object_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_018);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    attach_canonical_map_object(
        &mut session,
        AccessorObjectKind::Creature,
        canonical_world_object(loot_guid, 0, Position::new(31.0, 0.0, 0.0, 0.0)),
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        loot_response_failure_reason(&sent),
        LOOT_ERROR_TOO_FAR_LIKE_CPP
    );
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_item_creature_pickup_refreshes_canonical_owned_loot_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_115);
    let mut creature = make_canonical_creature_for_session(&session, loot_guid);
    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::new(0, 1));
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: vec![represented_loot_entry(0, 25, player_guid)],
            looted_by_player: false,
        },
    );

    mark_loot_item_looted_for_player_like_cpp(
        session.loot_table.get_mut(&loot_guid).unwrap(),
        0,
        player_guid,
    );
    session.refresh_represented_loot_owner_canonical_summary_like_cpp(loot_guid, player_guid);

    let loot = session.loot_table.get(&loot_guid).unwrap();
    assert!(loot.items[0].is_looted_for_player_like_cpp(player_guid));
    assert_eq!(loot.unlooted_count, 0);
    let canonical = canonical_creature_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::default())
    );
    assert!(canonical.is_fully_looted_like_cpp());
}

#[tokio::test]
async fn loot_item_missing_creature_uses_cpp_no_loot_error() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_009);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        loot_response_failure_reason(&sent),
        LOOT_ERROR_NO_LOOT_LIKE_CPP
    );
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_item_request_uses_loot_object_to_find_active_owner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let owner_guid = test_creature_guid(19_023);
    let loot_object_guid = represented_loot_object_guid_like_cpp(owner_guid);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(owner_guid);
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_object_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object_guid);
    assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_NO_LOOT_LIKE_CPP);
    assert!(!session.loot_table.get(&owner_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(owner_guid));
}

#[tokio::test]
async fn loot_item_request_can_use_secondary_active_loot_object_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let primary_owner = test_creature_guid(19_027);
    let secondary_owner = test_creature_guid(19_028);
    let secondary_loot_object = represented_loot_object_guid_like_cpp(secondary_owner);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(primary_owner);
    session.add_active_loot_view_owner_like_cpp(secondary_owner);
    session.loot_table.insert(
        secondary_owner,
        CreatureLoot {
            loot_guid: secondary_loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(secondary_loot_object, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), secondary_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), secondary_loot_object);
    assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_NO_LOOT_LIKE_CPP);
    assert!(!session.loot_table.get(&secondary_owner).unwrap().items[0].taken);
    assert!(session.active_loot_view_owners.contains(&primary_owner));
    assert!(session.active_loot_view_owners.contains(&secondary_owner));
}

#[tokio::test]
async fn loot_item_missing_gameobject_uses_cpp_release() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_010);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_item_gameobject_too_far_uses_cpp_release() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_029);
    let go_position = Position::new(6.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    attach_canonical_map_object(
        &mut session,
        AccessorObjectKind::GameObject,
        canonical_world_object(loot_guid, 0, go_position),
    );
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        go_position,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_item_gameobject_pickup_refreshes_canonical_owned_loot_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_139);
    let mut game_object =
        make_canonical_gameobject_for_session(&session, loot_guid, GAMEOBJECT_TYPE_CHEST as u8);
    game_object.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(0, 1));
    game_object.set_personal_loot_like_cpp(player_guid, GameObjectOwnedLoot::new(0, 1));
    attach_canonical_gameobject(&mut session, game_object);
    session.set_player_guid(Some(player_guid));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: vec![represented_loot_entry(0, 25, player_guid)],
            looted_by_player: false,
        },
    );

    mark_loot_item_looted_for_player_like_cpp(
        session.loot_table.get_mut(&loot_guid).unwrap(),
        0,
        player_guid,
    );
    session.refresh_represented_loot_owner_canonical_summary_like_cpp(loot_guid, player_guid);

    let loot = session.loot_table.get(&loot_guid).unwrap();
    assert!(loot.items[0].is_looted_for_player_like_cpp(player_guid));
    assert_eq!(loot.unlooted_count, 0);
    let canonical = canonical_gameobject_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&GameObjectOwnedLoot::default())
    );
    assert_eq!(canonical.personal_loot_count_like_cpp(), 0);
    assert_eq!(
        canonical.loot_for_player_like_cpp(player_guid),
        Some(&GameObjectOwnedLoot::default())
    );
    assert!(canonical.is_fully_looted_like_cpp());
}

#[tokio::test]
async fn loot_item_fishing_hole_skips_gameobject_distance_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_030);
    let go_position = Position::new(100.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    attach_canonical_map_object(
        &mut session,
        AccessorObjectKind::GameObject,
        canonical_world_object(loot_guid, 0, go_position),
    );
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        go_position,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_item_owned_gameobject_skips_distance_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_035);
    let go_position = Position::new(100.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        go_position,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_owner_guid_like_cpp(loot_guid, player_guid);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_item_owned_gameobject_skips_distance_from_canonical_created_by_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_036);
    let mut game_object = GameObject::new();
    game_object.world_mut().object_mut().create(loot_guid);
    game_object
        .world_mut()
        .set_map(u32::from(session.player_map_id_like_cpp()), 0)
        .unwrap();
    game_object
        .world_mut()
        .relocate(Position::new(100.0, 0.0, 0.0, 0.0));
    game_object.world_mut().object_mut().add_to_world();
    game_object.set_created_by(player_guid);

    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    attach_canonical_gameobject(&mut session, game_object);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}

#[tokio::test]
async fn loot_release_ignores_guid_outside_active_view_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let active_guid = test_creature_guid(19_011);
    let spoofed_guid = test_creature_guid(19_012);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(active_guid);
    session.loot_table.insert(
        spoofed_guid,
        CreatureLoot {
            loot_guid: spoofed_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(spoofed_guid))
        .await;

    assert!(session.is_active_loot_guid(active_guid));
    assert!(session.loot_table.contains_key(&spoofed_guid));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn loot_release_ignores_active_guid_without_represented_loot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let active_guid = test_creature_guid(19_015);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(active_guid);

    session
        .handle_loot_release(loot_release_packet(active_guid))
        .await;

    assert!(session.is_active_loot_guid(active_guid));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn loot_release_accepts_secondary_active_owner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let primary_guid = test_creature_guid(19_029);
    let secondary_guid = test_creature_guid(19_030);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(primary_guid);
    session.add_active_loot_view_owner_like_cpp(secondary_guid);
    session.loot_table.insert(
        secondary_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(secondary_guid),
            coins: 5,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(secondary_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), secondary_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(session.is_active_loot_guid(primary_guid));
    assert!(session.active_loot_view_owners.contains(&primary_guid));
    assert!(!session.active_loot_view_owners.contains(&secondary_guid));
    assert!(session.loot_table.contains_key(&secondary_guid));
}

#[tokio::test]
async fn loot_release_keeps_unlooted_creature_loot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = test_creature_guid(19_013);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    let _ = session.mutate_world_creature(loot_guid, |world_creature| {
        world_creature
            .creature
            .set_personal_loot_like_cpp(player_guid, CreatureOwnedLoot::new(0, 1));
    });
    let corpse_despawn_before = session
        .mutate_world_creature(loot_guid, |creature| creature.corpse_despawn_at())
        .unwrap()
        .expect("C++ arms corpse removal when the creature reaches JUST_DIED");
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 7,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid, other_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(
        !session.loot_table.contains_key(&loot_guid),
        "the closed session view is a discardable cache; the creature authority keeps loot"
    );
    assert!(session.reconcile_represented_loot_cache_like_cpp(loot_guid, player_guid));
    assert_eq!(session.loot_table[&loot_guid].coins, 7);
    assert!(session.loot_table.contains_key(&loot_guid));
    assert_eq!(
        session.loot_table.get(&loot_guid).unwrap().players_looting,
        vec![other_guid]
    );
    assert_eq!(
        session
            .mutate_world_creature(loot_guid, |creature| creature.corpse_despawn_at())
            .unwrap(),
        Some(corpse_despawn_before),
        "releasing partial loot must not change the existing corpse timer"
    );
}

#[tokio::test]
async fn creature_owned_loot_release_partial_uses_canonical_is_fully_looted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = test_creature_guid(19_113);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    let corpse_despawn_before = session
        .mutate_world_creature(loot_guid, |creature| creature.corpse_despawn_at())
        .unwrap()
        .expect("C++ arms corpse removal when the creature reaches JUST_DIED");
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 7,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid, other_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.contains_key(&loot_guid));
    let canonical = canonical_creature_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::new(7, 0))
    );
    assert_eq!(canonical.personal_loot_count_like_cpp(), 0);
    assert_eq!(
        canonical.loot_for_player_like_cpp(other_guid),
        Some(&CreatureOwnedLoot::new(7, 0))
    );
    assert!(!canonical.is_fully_looted_like_cpp());
    assert!(session.reconcile_represented_loot_cache_like_cpp(loot_guid, player_guid));
    assert_eq!(session.loot_table[&loot_guid].coins, 7);
    assert!(session.loot_table.contains_key(&loot_guid));
    assert_eq!(
        session
            .mutate_world_creature(loot_guid, |creature| creature.corpse_despawn_at())
            .unwrap(),
        Some(corpse_despawn_before),
        "releasing partial canonical loot must not change the existing corpse timer"
    );
}

#[tokio::test]
async fn authoritative_partial_release_clears_round_robin_for_all_sessions_and_forces_dynflags_like_cpp()
 {
    let first_guid = ObjectGuid::create_player(1, 61_890);
    let second_guid = ObjectGuid::create_player(1, 61_891);
    let mut loot = authoritative_test_loot_like_cpp(7, false);
    loot.round_robin_player = first_guid;
    loot.allowed_looters = vec![first_guid, second_guid];
    let (mut first, _first_rx, mut second, _second_rx, owner_guid, _, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(loot);
    // The helper uses its own deterministic player ids; install the exact
    // current round-robin holder from the opened first session.
    let opened_first = first.player_guid().unwrap();
    let opened_second = second.player_guid().unwrap();
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(opened_first)
        .unwrap()
        .generation;
    let mut replacement = authority
        .snapshot_for_player_like_cpp(opened_first)
        .unwrap()
        .loot;
    replacement.round_robin_player = opened_first;
    authority.replace_like_cpp(Some(replacement), HashMap::new());
    let replacement_generation = authority
        .snapshot_for_player_like_cpp(opened_first)
        .unwrap()
        .generation;
    assert_ne!(generation, replacement_generation);
    authority.add_viewer_like_cpp(opened_first).unwrap();
    first
        .active_loot_view_generations_like_cpp
        .insert(owner_guid, replacement_generation);
    first
        .active_loot_view_authorities_like_cpp
        .insert(owner_guid, authority.clone());
    assert!(first.reconcile_represented_loot_cache_like_cpp(owner_guid, opened_first));
    let _ = first.mutate_world_creature(owner_guid, |creature| {
        creature
            .creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .clear_update_mask(false);
    });

    assert!(
        first
            .do_loot_release_owner_like_cpp(owner_guid, opened_first)
            .await
    );

    assert!(
        authority
            .snapshot_for_player_like_cpp(opened_second)
            .unwrap()
            .loot
            .round_robin_player
            .is_empty()
    );
    assert!(second.reconcile_represented_loot_cache_like_cpp(owner_guid, opened_second));
    assert!(
        second
            .loot_table
            .get(&owner_guid)
            .unwrap()
            .round_robin_player
            .is_empty()
    );
    assert!(
        first
            .mutate_world_creature(owner_guid, |creature| {
                creature
                    .creature
                    .unit()
                    .world()
                    .object()
                    .changed_fields()
                    .contains(ObjectChangedFields::DYNAMIC_FLAGS)
            })
            .unwrap()
    );
}

#[tokio::test]
async fn creature_owned_loot_release_fully_consumed_uses_canonical_is_fully_looted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_114);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    let _ = session.mutate_world_creature(loot_guid, |creature| {
        creature.creature.set_corpse_delay(120, false);
    });
    assert_eq!(
        session
            .mutate_world_creature(loot_guid, |creature| creature.corpse_delay_secs_like_cpp())
            .unwrap(),
        120
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.contains_key(&loot_guid));
    let canonical = canonical_creature_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::default())
    );
    assert!(canonical.is_fully_looted_like_cpp());
    let corpse_despawn_at = session
        .mutate_world_creature(loot_guid, |creature| creature.corpse_despawn_at())
        .unwrap()
        .expect("fully looted corpse should start decay timer");
    let remaining = corpse_despawn_at.saturating_duration_since(Instant::now());
    assert!(
        (55..=60).contains(&remaining.as_secs()),
        "C++ uses corpse_delay * Rate.Corpse.Decay.Looted; got {remaining:?}"
    );
}

#[tokio::test]
async fn personal_creature_release_starts_decay_only_after_every_pool_is_looted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let first_player = ObjectGuid::create_player(1, 53);
    let second_player = ObjectGuid::create_player(1, 54);
    let owner_guid = test_creature_guid(19_119);
    let mut creature = make_canonical_creature_for_session(&session, owner_guid);

    let mut first_pool = authoritative_test_loot_like_cpp(0, false);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    first_pool.allowed_looters = vec![first_player];
    let mut second_pool = authoritative_test_loot_like_cpp(0, true);
    second_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner_guid.realm_id(),
        owner_guid.map_id(),
        0,
        0,
        owner_guid.counter() + 1,
    );
    second_pool.allowed_looters = vec![second_player];
    second_pool.items[0].allowed_looters = vec![second_player];
    assert!(
        creature
            .initialize_loot_authority_like_cpp(
                None,
                HashMap::from([(first_player, first_pool), (second_player, second_pool),]),
            )
            .installed()
    );
    let authority = creature.loot_authority_like_cpp().clone();
    attach_canonical_creature(&mut session, creature);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    let corpse_deadline_before = session
        .mutate_world_creature(owner_guid, |creature| {
            creature.creature.set_corpse_delay(120, false);
            let deadline = Instant::now() + Duration::from_secs(120);
            creature.set_corpse_despawn_at(Some(deadline));
            creature.corpse_despawn_at()
        })
        .flatten()
        .expect("dead creature should already own its normal corpse deadline");

    session.set_player_guid(Some(first_player));
    assert!(session.reconcile_represented_loot_cache_like_cpp(owner_guid, first_player));
    session.set_active_loot_guid(owner_guid);
    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &session.loot_table[&owner_guid],
        first_player,
    );
    session.represented_on_loot_opened_like_cpp(owner_guid, first_player, response);
    let _ = drain_server_opcodes_like_cpp(&send_rx);

    assert!(
        session
            .do_loot_release_owner_like_cpp(owner_guid, first_player)
            .await
    );
    assert_eq!(
        session
            .mutate_world_creature(owner_guid, |creature| creature.corpse_despawn_at())
            .flatten(),
        Some(corpse_deadline_before),
        "one empty personal pool must not start global corpse decay while a peer has loot"
    );
    assert!(!authority.is_fully_looted_like_cpp());

    session.set_player_guid(Some(second_player));
    assert!(session.reconcile_represented_loot_cache_like_cpp(owner_guid, second_player));
    session.set_active_loot_guid(owner_guid);
    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &session.loot_table[&owner_guid],
        second_player,
    );
    session.represented_on_loot_opened_like_cpp(owner_guid, second_player, response);
    let claim = authority
        .reserve_item_like_cpp(second_player, 0)
        .await
        .unwrap();
    assert_eq!(claim.commit_like_cpp(), Ok(true));
    let _ = drain_server_opcodes_like_cpp(&send_rx);

    assert!(
        session
            .do_loot_release_owner_like_cpp(owner_guid, second_player)
            .await
    );
    assert!(authority.is_fully_looted_like_cpp());
    let corpse_deadline_after = session
        .mutate_world_creature(owner_guid, |creature| creature.corpse_despawn_at())
        .flatten()
        .expect("last personal pool should start looted-corpse decay");
    assert!(corpse_deadline_after < corpse_deadline_before);
    let remaining = corpse_deadline_after.saturating_duration_since(Instant::now());
    assert!((55..=60).contains(&remaining.as_secs()));
}

#[test]
fn creature_loot_release_dynamic_flags_are_viewer_dependent_like_cpp() {
    let mut session = make_session();
    let first_player = ObjectGuid::create_player(1, 61);
    let second_player = ObjectGuid::create_player(1, 62);
    let unrelated_player = ObjectGuid::create_player(1, 63);
    let owner_guid = test_creature_guid(19_120);
    let mut creature = make_canonical_creature_for_session(&session, owner_guid);

    let mut consumed_pool = authoritative_test_loot_like_cpp(0, false);
    consumed_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    consumed_pool.allowed_looters = vec![first_player];
    let mut live_pool = authoritative_test_loot_like_cpp(0, true);
    live_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner_guid.realm_id(),
        owner_guid.map_id(),
        0,
        0,
        owner_guid.counter() + 1,
    );
    live_pool.allowed_looters = vec![second_player];
    live_pool.items[0].allowed_looters = vec![second_player];
    assert!(
        creature
            .initialize_loot_authority_like_cpp(
                None,
                HashMap::from([(first_player, consumed_pool), (second_player, live_pool),]),
            )
            .installed()
    );
    let authority = creature.loot_authority_like_cpp().clone();
    attach_canonical_creature(&mut session, creature);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.set_player_guid(Some(first_player));

    let update = UnitDataValuesDeltaUpdate {
        object_data: Some(ObjectDataValuesUpdate {
            changed_object_type_mask: 1,
            object_data_mask: 1 << 2,
            entry_id: 0,
            dynamic_flags: UnitDynFlags::Lootable as u32,
            scale: 1.0,
        }),
        ..UnitDataValuesDeltaUpdate::default()
    };
    let dynamic_flags_for = |viewer_guid| {
        session
            .creature_loot_release_values_for_viewer_like_cpp(
                owner_guid,
                viewer_guid,
                false,
                Some(&authority),
                update.clone(),
            )
            .object_data
            .unwrap()
            .dynamic_flags
    };

    assert_eq!(dynamic_flags_for(first_player), 0);
    assert_eq!(
        dynamic_flags_for(second_player),
        UnitDynFlags::Lootable as u32,
        "one exhausted personal pool must not hide another player's live loot"
    );
    assert_eq!(dynamic_flags_for(unrelated_player), 0);
}

#[test]
fn creature_loot_visibility_applies_full_cpp_allowed_to_loot_gate() {
    let round_robin_owner = ObjectGuid::create_player(1, 64);
    let other_player = ObjectGuid::create_player(1, 65);
    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_method = LOOT_METHOD_ROUND_ROBIN_LIKE_CPP;
    loot.round_robin_player = round_robin_owner;
    loot.allowed_looters = vec![round_robin_owner, other_player];
    loot.items[0].allowed_looters = vec![round_robin_owner, other_player];
    loot.items[0].flags.follow_loot_rules = true;

    assert!(creature_loot_is_allowed_to_player_like_cpp(
        true,
        false,
        &loot,
        round_robin_owner,
    ));
    assert!(
        !creature_loot_is_allowed_to_player_like_cpp(true, false, &loot, other_player),
        "ordinary shared round-robin loot belongs only to the selected player"
    );
    assert!(
        !creature_loot_is_allowed_to_player_like_cpp(false, false, &loot, round_robin_owner,),
        "C++ rejects loot visibility for a living creature"
    );
    assert!(
        !creature_loot_is_allowed_to_player_like_cpp(true, true, &loot, round_robin_owner,),
        "C++ HasPendingBind suppresses loot visibility"
    );

    loot.items[0].flags.follow_loot_rules = false;
    assert!(
        creature_loot_is_allowed_to_player_like_cpp(true, false, &loot, other_player),
        "quest/conditional/free-for-player loot remains visible outside round robin"
    );
}

#[tokio::test]
async fn creature_loot_release_command_retries_without_blocking_source_like_cpp() {
    let (command_tx, command_rx) = flume::bounded(1);
    command_tx
        .send(SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp)
        .unwrap();
    let creature_guid = test_creature_guid(19_121);
    assert_eq!(
        queue_creature_loot_release_command_reliably_like_cpp(
            &command_tx,
            SessionCommand::SendCreatureLootReleaseValuesUpdateLikeCpp(
                crate::session::mailbox::SendCreatureLootReleaseValuesUpdateLikeCppCommand {
                    creature_guid,
                    map_id: 0,
                    instance_id: 0,
                    unit_values_update: UnitDataValuesDeltaUpdate::default(),
                    authority: None,
                },
            ),
        ),
        CreatureLootReleaseCommandQueueOutcomeLikeCpp::Retrying,
        "a full peer queue must schedule retry without blocking this session"
    );
    assert!(matches!(
        command_rx.recv().unwrap(),
        SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp
    ));
    let queued = tokio::time::timeout(Duration::from_secs(1), command_rx.recv_async())
        .await
        .expect("detached retry should enqueue after capacity opens")
        .unwrap();
    assert!(matches!(
        queued,
        SessionCommand::SendCreatureLootReleaseValuesUpdateLikeCpp(command)
            if command.creature_guid == creature_guid
    ));
}

#[tokio::test]
async fn creature_owned_loot_release_does_not_extend_expired_corpse_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_118);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, true));
    let expired = Instant::now() - Duration::from_secs(1);
    let deadline_before = session
        .mutate_world_creature(loot_guid, |creature| {
            creature.creature.set_corpse_delay(0, false);
            creature.creature.mark_ai_dead(0);
            creature.creature.set_corpse_delay(120, false);
            creature.set_corpse_despawn_at(Some(expired));
            creature.corpse_despawn_at().unwrap()
        })
        .unwrap();
    assert!(deadline_before <= Instant::now());
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(!session.loot_table.contains_key(&loot_guid));
    let deadline_after = session
        .mutate_world_creature(loot_guid, |creature| creature.corpse_despawn_at())
        .unwrap()
        .expect("expired corpse must retain its existing lifecycle deadline");
    assert_eq!(deadline_after, deadline_before);
    assert!(
        deadline_after <= Instant::now(),
        "C++ AllLootRemovedFromCorpse is a no-op after corpse removal expires"
    );
}

#[tokio::test]
async fn creature_owned_loot_release_fully_consumed_removes_lootable_dynflag_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_116);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.client_visible_guids_like_cpp.insert(loot_guid);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    let _ = session.mutate_world_creature(loot_guid, |creature| {
        creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
    });
    assert!(
        session
            .mutate_world_creature(loot_guid, |creature| creature
                .has_lootable_dynamic_flag_like_cpp())
            .unwrap()
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx),
        vec![
            wow_constants::ServerOpcodes::LootRelease as u16,
            wow_constants::ServerOpcodes::UpdateObject as u16,
        ],
        "C++ ForceUpdateFieldChange must become a visible VALUES update so the client removes the loot cursor"
    );
    assert!(
        !session
            .mutate_world_creature(loot_guid, |creature| creature
                .has_lootable_dynamic_flag_like_cpp())
            .unwrap(),
        "C++ LootHandler::DoLootRelease removes UNIT_DYNFLAG_LOOTABLE when the creature is fully looted"
    );
}

#[test]
fn looted_corpse_decay_uses_cpp_rate_and_ignore_flag() {
    assert_eq!(
        looted_corpse_decay_secs_like_cpp(false, 120, false, 0.5),
        60
    );
    assert_eq!(
        looted_corpse_decay_secs_like_cpp(false, 120, true, 0.5),
        120
    );
    assert_eq!(
        looted_corpse_decay_secs_like_cpp(false, 120, false, -1.0),
        0
    );
    assert_eq!(looted_corpse_decay_secs_like_cpp(true, 120, false, 0.5), 0);
}

#[tokio::test]
async fn creature_skinning_loot_release_despawns_corpse_immediately_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_115);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    let _ = session.mutate_world_creature(loot_guid, |creature| {
        creature.creature.set_corpse_delay(120, false);
    });
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_SKINNING_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let corpse_despawn_at = session
        .mutate_world_creature(loot_guid, |creature| creature.corpse_despawn_at())
        .unwrap()
        .expect("skinned corpse should start decay timer");
    let remaining = corpse_despawn_at.saturating_duration_since(Instant::now());
    assert_eq!(
        remaining.as_secs(),
        0,
        "C++ sets m_corpseRemoveTime = now for fully looted LOOT_SKINNING; got {remaining:?}"
    );
}

#[tokio::test]
async fn player_corpse_loot_release_removes_corpse_lootable_dynflag_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let corpse_guid = test_corpse_guid(19_117);
    let corpse = make_canonical_corpse_for_session(&session, corpse_guid);
    attach_canonical_corpse(&mut session, corpse);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(corpse_guid);
    session.loot_table.insert(
        corpse_guid,
        CreatureLoot {
            loot_guid: corpse_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_INSIGNIA_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    assert_eq!(
        canonical_corpse_snapshot(&session, corpse_guid)
            .unwrap()
            .data()
            .dynamic_flags
            & CORPSE_DYNFLAG_LOOTABLE,
        CORPSE_DYNFLAG_LOOTABLE
    );

    session
        .handle_loot_release(loot_release_packet(corpse_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let corpse = canonical_corpse_snapshot(&session, corpse_guid).unwrap();
    assert_eq!(
        corpse.data().dynamic_flags & CORPSE_DYNFLAG_LOOTABLE,
        0,
        "C++ DoLootRelease removes CORPSE_DYNFLAG_LOOTABLE from fully looted player corpses"
    );
    assert!(
        corpse
            .corpse_data_changes_mask()
            .is_set(wow_entities::CORPSE_DATA_DYNAMIC_FLAGS_BIT)
    );
}

#[tokio::test]
async fn loot_release_keeps_unlooted_gameobject_loot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_014);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.client_visible_guids_like_cpp.insert(loot_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    let state = session
        .represented_gameobject_use_states
        .get(&loot_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(LootState::Activated));
    assert_eq!(state.loot_state_unit_guid, player_guid);
}

#[tokio::test]
async fn loot_release_gameobject_too_far_keeps_state_and_loot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_031);
    let go_position = Position::new(6.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        go_position,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(session.loot_table.contains_key(&loot_guid));
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&loot_guid)
            .unwrap()
            .loot_state,
        None
    );
}

#[tokio::test]
async fn loot_release_owned_gameobject_skips_distance_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_036);
    let go_position = Position::new(100.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        go_position,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_owner_guid_like_cpp(loot_guid, player_guid);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert!(!session.loot_table.contains_key(&loot_guid));
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&loot_guid)
            .unwrap()
            .loot_state,
        Some(LootState::JustDeactivated)
    );
}

#[tokio::test]
async fn loot_release_fully_looted_gameobject_just_deactivates_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_032);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert!(!session.loot_table.contains_key(&loot_guid));
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&loot_guid)
            .unwrap()
            .loot_state,
        Some(LootState::JustDeactivated)
    );
}

#[tokio::test]
async fn gameobject_owned_loot_release_partial_chest_uses_canonical_is_fully_looted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_132);
    let mut game_object =
        make_canonical_gameobject_for_session(&session, loot_guid, GAMEOBJECT_TYPE_CHEST as u8);
    game_object.set_personal_loot_like_cpp(player_guid, GameObjectOwnedLoot::new(0, 1));
    attach_canonical_gameobject(&mut session, game_object);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_chest_release_metadata_like_cpp(
        loot_guid,
        GameObjectLootSource {
            chest_consumable: false,
            chest_restock_time_secs: 7,
            ..Default::default()
        },
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: vec![represented_loot_entry(0, 25, player_guid)],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let canonical = canonical_gameobject_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&GameObjectOwnedLoot::new(0, 1))
    );
    assert_eq!(canonical.personal_loot_count_like_cpp(), 0);
    assert_eq!(
        canonical.loot_for_player_like_cpp(player_guid),
        Some(&GameObjectOwnedLoot::new(0, 1))
    );
    assert!(!canonical.is_fully_looted_like_cpp());
    assert_eq!(canonical.loot_state(), LootState::Activated);
    assert_eq!(canonical.loot_state_unit_guid(), player_guid);
    assert!(canonical.restock_time() > 0);
    assert!(!session.loot_table.contains_key(&loot_guid));
    assert!(session.reconcile_represented_loot_cache_like_cpp(loot_guid, player_guid));
    assert!(session.loot_table.contains_key(&loot_guid));
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&loot_guid)
            .unwrap()
            .loot_state,
        Some(LootState::Activated)
    );
}

#[tokio::test]
async fn loot_release_partial_chest_syncs_state_to_same_map_viewers_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = test_gameobject_guid(19_138);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());

    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_chest_release_metadata_like_cpp(
        loot_guid,
        GameObjectLootSource {
            loot_id: 7_001,
            chest_restock_time_secs: 45,
            chest_consumable: false,
            ..Default::default()
        },
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: Vec::new(),
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let release_bytes = send_rx.try_recv().unwrap();
    let mut release = WorldPacket::from_bytes(&release_bytes);
    assert_eq!(
        release.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command)) => command,
        other => panic!("expected chest release sync command, got {other:?}"),
    };
    assert_eq!(command.gameobject_guid, loot_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(
        command.loot_state,
        Some(wow_entities::LootState::Activated as u8)
    );
    assert_eq!(command.loot_state_unit_guid, player_guid);
    assert_eq!(command.chest_loot_id, 7_001);
    assert_eq!(command.chest_restock_time_secs, 45);
}

#[tokio::test]
async fn gameobject_owned_loot_release_fully_consumed_chest_uses_canonical_is_fully_looted_like_cpp()
 {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_133);
    let mut game_object =
        make_canonical_gameobject_for_session(&session, loot_guid, GAMEOBJECT_TYPE_CHEST as u8);
    game_object.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(0, 1));
    attach_canonical_gameobject(&mut session, game_object);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_chest_release_metadata_like_cpp(
        loot_guid,
        GameObjectLootSource {
            chest_consumable: false,
            chest_restock_time_secs: 7,
            ..Default::default()
        },
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let canonical = canonical_gameobject_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&GameObjectOwnedLoot::default())
    );
    assert!(canonical.is_fully_looted_like_cpp());
    assert_eq!(canonical.loot_state(), LootState::JustDeactivated);
    assert_eq!(canonical.loot_state_unit_guid(), ObjectGuid::EMPTY);
    assert_eq!(canonical.restock_time(), 0);
    assert!(!session.loot_table.contains_key(&loot_guid));
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&loot_guid)
            .unwrap()
            .loot_state,
        Some(LootState::JustDeactivated)
    );
}

#[test]
fn gameobject_loot_release_without_canonical_manager_keeps_represented_restock_fallback_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_135);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_chest_release_metadata_like_cpp(
        loot_guid,
        GameObjectLootSource {
            chest_consumable: false,
            chest_restock_time_secs: 7,
            ..Default::default()
        },
    );

    session.apply_represented_gameobject_loot_release_like_cpp(
        loot_guid,
        player_guid,
        true,
        true,
        None,
    );

    let state = session
        .represented_gameobject_use_states
        .get(&loot_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(LootState::NotReady));
    assert_eq!(state.loot_state_unit_guid, ObjectGuid::EMPTY);
    assert!(state.chest_restock_until.is_some());
}

#[tokio::test]
async fn gameobject_owned_loot_release_personal_chest_syncs_current_player_and_despawns_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_134);
    let game_object =
        make_canonical_gameobject_for_session(&session, loot_guid, GAMEOBJECT_TYPE_CHEST as u8);
    attach_canonical_gameobject(&mut session, game_object);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        loot_guid,
        loot_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_chest_release_metadata_like_cpp(
        loot_guid,
        GameObjectLootSource {
            personal_loot_id: 55,
            chest_consumable: false,
            chest_restock_time_secs: 7,
            ..Default::default()
        },
    );
    session.represented_personal_loot_owners.insert(loot_guid);
    session
        .represented_personal_loot_money
        .insert((loot_guid, player_guid), 0);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 1,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                taken: true,
                ..represented_loot_entry(0, 25, player_guid)
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let canonical = canonical_gameobject_snapshot(&session, loot_guid).unwrap();
    assert_eq!(canonical.shared_loot_like_cpp(), None);
    assert_eq!(
        canonical.personal_loot_like_cpp(player_guid),
        Some(&GameObjectOwnedLoot::default())
    );
    let state = session
        .represented_gameobject_use_states
        .get(&loot_guid)
        .unwrap();
    assert_eq!(state.per_player_state_player_guid, Some(player_guid));
    assert_eq!(state.per_player_despawn_secs, Some(7));
    assert!(state.per_player_despawn_until.is_some());
}

#[tokio::test]
async fn authoritative_partial_gameobject_release_drops_cache_and_reopen_rehydrates_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 451);
    let owner_guid = test_gameobject_guid(19_451);
    let mut gameobject =
        make_canonical_gameobject_for_session(&session, owner_guid, GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::Activated, Some(player_guid));
    let mut pool = authoritative_test_loot_like_cpp(11, true);
    pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    pool.allowed_looters = vec![player_guid];
    pool.items[0].allowed_looters = vec![player_guid];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(None, HashMap::from([(player_guid, pool)]),)
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    attach_canonical_gameobject(&mut session, gameobject);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        owner_guid,
        owner_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    let source = GameObjectLootSource {
        personal_loot_id: 55,
        chest_consumable: false,
        chest_restock_time_secs: 7,
        ..Default::default()
    };
    session.record_represented_gameobject_chest_release_metadata_like_cpp(owner_guid, source);
    assert!(session.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid));
    session.set_active_loot_guid(owner_guid);
    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &session.loot_table[&owner_guid],
        player_guid,
    );
    session.represented_on_loot_opened_like_cpp(owner_guid, player_guid, response);
    let _ = drain_server_opcodes_like_cpp(&send_rx);

    assert!(
        session
            .do_loot_release_owner_like_cpp(owner_guid, player_guid)
            .await
    );
    assert!(!session.loot_table.contains_key(&owner_guid));
    assert!(
        !session
            .represented_loot_cache_generations_like_cpp
            .contains_key(&owner_guid)
    );
    assert!(
        !session
            .represented_personal_loot_money
            .contains_key(&(owner_guid, player_guid))
    );
    let before_reopen = authority
        .snapshot_for_player_like_cpp(player_guid)
        .expect("release preserves the canonical personal pool");
    assert_eq!(before_reopen.loot.coins, 11);
    assert!(!before_reopen.loot.items[0].taken);

    session
        .open_represented_gameobject_chest_like_cpp(owner_guid, source)
        .await;
    assert!(session.loot_table.contains_key(&owner_guid));
    assert!(
        session
            .represented_personal_loot_owners
            .contains(&owner_guid)
    );
    assert_eq!(
        session
            .represented_personal_loot_money
            .get(&(owner_guid, player_guid)),
        Some(&11)
    );
    assert!(session.is_active_loot_guid(owner_guid));

    let slot = before_reopen.loot.items[0].loot_list_id;
    authority
        .reserve_item_like_cpp(player_guid, slot)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();
    assert!(
        authority
            .reserve_item_like_cpp(player_guid, slot)
            .await
            .is_err(),
        "rehydration must not manufacture a second claim"
    );
}

#[tokio::test]
async fn authoritative_partial_personal_creature_release_drops_cache_and_reopen_rehydrates_like_cpp()
 {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    fixture
        .session
        .ensure_represented_creature_kill_loot_like_cpp(fixture.owner_guid)
        .await;
    let authority = fixture
        .session
        .represented_owned_loot_authority_like_cpp(fixture.owner_guid)
        .unwrap();
    let before_release = authority
        .snapshot_for_player_like_cpp(fixture.first_tapper)
        .unwrap();
    let slot = before_release
        .loot
        .items
        .iter()
        .find(|item| item.item_id == fixture.normal_item_id)
        .unwrap()
        .loot_list_id;
    assert!(
        fixture
            .session
            .reconcile_represented_loot_cache_like_cpp(fixture.owner_guid, fixture.first_tapper,)
    );
    let opened = authority
        .add_viewer_like_cpp(fixture.first_tapper)
        .expect("the authoritative personal pool opens");
    fixture.session.set_active_loot_guid(fixture.owner_guid);
    fixture
        .session
        .active_loot_view_generations_like_cpp
        .insert(fixture.owner_guid, opened.generation);
    fixture
        .session
        .active_loot_view_authorities_like_cpp
        .insert(fixture.owner_guid, authority.clone());

    assert!(
        fixture
            .session
            .do_loot_release_owner_like_cpp(fixture.owner_guid, fixture.first_tapper)
            .await
    );
    assert!(!fixture.session.loot_table.contains_key(&fixture.owner_guid));
    assert!(
        !fixture
            .session
            .represented_loot_cache_generations_like_cpp
            .contains_key(&fixture.owner_guid)
    );
    assert!(
        !fixture
            .session
            .represented_personal_loot_money
            .contains_key(&(fixture.owner_guid, fixture.first_tapper))
    );
    let after_release = authority
        .snapshot_for_player_like_cpp(fixture.first_tapper)
        .unwrap();
    assert_eq!(after_release.generation, before_release.generation);
    assert_eq!(after_release.scope, before_release.scope);
    assert_eq!(after_release.loot.coins, before_release.loot.coins);
    assert_eq!(after_release.loot.items, before_release.loot.items);
    assert!(after_release.loot.players_looting.is_empty());
    assert!(
        after_release.loot.looted_by_player,
        "C++ keeps the per-Loot was-opened state after closing the viewer"
    );

    let response = fixture
        .session
        .represented_loot_response_for_owner_like_cpp(
            fixture.owner_guid,
            fixture.first_tapper,
            false,
        )
        .await
        .expect("the creature authority rehydrates a personal view");
    assert_eq!(response.coins, 7);
    assert!(
        fixture
            .session
            .represented_personal_loot_owners
            .contains(&fixture.owner_guid)
    );
    assert_eq!(
        fixture
            .session
            .represented_personal_loot_money
            .get(&(fixture.owner_guid, fixture.first_tapper)),
        Some(&7)
    );
    authority
        .reserve_item_like_cpp(fixture.first_tapper, slot)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();
    assert!(
        authority
            .reserve_item_like_cpp(fixture.first_tapper, slot)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn personal_gameobject_release_deactivates_only_after_every_pool_is_looted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let first_player = ObjectGuid::create_player(1, 51);
    let second_player = ObjectGuid::create_player(1, 52);
    let owner_guid = test_gameobject_guid(19_138);
    let mut gameobject =
        make_canonical_gameobject_for_session(&session, owner_guid, GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::Activated, Some(first_player));

    let mut first_pool = authoritative_test_loot_like_cpp(0, false);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    first_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    first_pool.allowed_looters = vec![first_player];
    let mut second_pool = authoritative_test_loot_like_cpp(0, true);
    second_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner_guid.realm_id(),
        owner_guid.map_id(),
        0,
        0,
        owner_guid.counter() + 1,
    );
    second_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    second_pool.allowed_looters = vec![second_player];
    second_pool.items[0].allowed_looters = vec![second_player];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(
                None,
                HashMap::from([(first_player, first_pool), (second_player, second_pool),]),
            )
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    attach_canonical_gameobject(&mut session, gameobject);
    session.set_player_position_like_cpp(Position::ZERO);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        owner_guid,
        owner_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_chest_release_metadata_like_cpp(
        owner_guid,
        GameObjectLootSource {
            personal_loot_id: 55,
            chest_consumable: false,
            chest_restock_time_secs: 7,
            ..Default::default()
        },
    );

    session.set_player_guid(Some(first_player));
    assert!(session.reconcile_represented_loot_cache_like_cpp(owner_guid, first_player));
    session.set_active_loot_guid(owner_guid);
    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &session.loot_table[&owner_guid],
        first_player,
    );
    session.represented_on_loot_opened_like_cpp(owner_guid, first_player, response);
    let _ = drain_server_opcodes_like_cpp(&send_rx);

    assert!(
        session
            .do_loot_release_owner_like_cpp(owner_guid, first_player)
            .await
    );
    assert_eq!(
        canonical_gameobject_snapshot(&session, owner_guid)
            .unwrap()
            .loot_state(),
        LootState::Activated,
        "one empty personal pool must not globally deactivate a chest while a peer has loot"
    );
    assert!(!authority.is_retired_like_cpp());
    assert!(!authority.is_fully_looted_like_cpp());
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&owner_guid)
            .unwrap()
            .per_player_state_player_guid,
        Some(first_player),
        "C++ still runs OnLootRelease for the selected empty personal pool"
    );

    session.set_player_guid(Some(second_player));
    assert!(session.reconcile_represented_loot_cache_like_cpp(owner_guid, second_player));
    session.set_active_loot_guid(owner_guid);
    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &session.loot_table[&owner_guid],
        second_player,
    );
    session.represented_on_loot_opened_like_cpp(owner_guid, second_player, response);
    let claim = authority
        .reserve_item_like_cpp(second_player, 0)
        .await
        .unwrap();
    assert_eq!(claim.commit_like_cpp(), Ok(true));
    let _ = drain_server_opcodes_like_cpp(&send_rx);

    assert!(
        session
            .do_loot_release_owner_like_cpp(owner_guid, second_player)
            .await
    );
    assert_eq!(
        canonical_gameobject_snapshot(&session, owner_guid)
            .unwrap()
            .loot_state(),
        LootState::JustDeactivated,
        "the last empty personal pool must globally deactivate the chest"
    );
    assert!(authority.is_fully_looted_like_cpp());
    assert!(!authority.is_retired_like_cpp());

    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    let mut manager = manager.lock().unwrap();
    manager
        .find_map_mut(u32::from(session.player_map_id_like_cpp()), 0)
        .unwrap()
        .map_mut()
        .update_game_object_like_cpp(owner_guid, 1, 0);
    drop(manager);
    assert!(
        authority.is_retired_like_cpp(),
        "the canonical JustDeactivated update must clear and retire the completed authority"
    );
}

#[test]
fn personal_gameobject_upsert_before_release_invalidates_global_deactivation_like_cpp() {
    let mut session = make_session();
    let first = ObjectGuid::create_player(1, 61_860);
    let late = ObjectGuid::create_player(1, 61_861);
    let owner_guid = test_gameobject_guid(61_862);
    let mut gameobject =
        make_canonical_gameobject_for_session(&session, owner_guid, GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::Activated, Some(first));
    let mut first_pool = authoritative_test_loot_like_cpp(0, false);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    first_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    first_pool.allowed_looters = vec![first];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(None, HashMap::from([(first, first_pool)]),)
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    attach_canonical_gameobject(&mut session, gameobject);
    session.set_player_guid(Some(first));
    authority.add_viewer_like_cpp(first).unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(first)
        .unwrap()
        .generation;
    let close = authority
        .close_viewer_if_generation_like_cpp(generation, first)
        .unwrap();
    assert!(close.whole_object_fully_looted);

    let mut late_pool = authoritative_test_loot_like_cpp(0, true);
    late_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner_guid.realm_id(),
        owner_guid.map_id(),
        0,
        0,
        owner_guid.counter() + 1,
    );
    late_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    late_pool.allowed_looters = vec![late];
    late_pool.items[0].allowed_looters = vec![late];
    assert!(
        session
            .upsert_represented_personal_gameobject_loot_authority_like_cpp(
                owner_guid, late, late_pool, false,
            )
            .is_some()
    );

    assert!(
        session
            .set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
                owner_guid,
                &authority,
                close.object_generation,
                close.lifecycle_revision,
                LootState::JustDeactivated,
                None,
                0,
                false,
            )
            .is_none(),
        "the late pool revision must invalidate the earlier fully-looted observation"
    );
    assert_eq!(
        canonical_gameobject_snapshot(&session, owner_guid)
            .unwrap()
            .loot_state(),
        LootState::Activated
    );
    assert!(authority.snapshot_for_player_like_cpp(late).is_some());
}

#[test]
fn personal_gameobject_release_before_upsert_rejects_resurrection_like_cpp() {
    let mut session = make_session();
    let first = ObjectGuid::create_player(1, 61_870);
    let late = ObjectGuid::create_player(1, 61_871);
    let owner_guid = test_gameobject_guid(61_872);
    let mut gameobject =
        make_canonical_gameobject_for_session(&session, owner_guid, GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::Activated, Some(first));
    let mut first_pool = authoritative_test_loot_like_cpp(0, false);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    first_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    first_pool.allowed_looters = vec![first];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(None, HashMap::from([(first, first_pool)]),)
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    attach_canonical_gameobject(&mut session, gameobject);
    session.set_player_guid(Some(first));
    authority.add_viewer_like_cpp(first).unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(first)
        .unwrap()
        .generation;
    let close = authority
        .close_viewer_if_generation_like_cpp(generation, first)
        .unwrap();
    assert!(
        session
            .set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
                owner_guid,
                &authority,
                close.object_generation,
                close.lifecycle_revision,
                LootState::JustDeactivated,
                None,
                0,
                false,
            )
            .is_some()
    );

    let mut late_pool = authoritative_test_loot_like_cpp(0, true);
    late_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner_guid.realm_id(),
        owner_guid.map_id(),
        0,
        0,
        owner_guid.counter() + 1,
    );
    late_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    late_pool.allowed_looters = vec![late];
    late_pool.items[0].allowed_looters = vec![late];
    assert!(
        session
            .upsert_represented_personal_gameobject_loot_authority_like_cpp(
                owner_guid, late, late_pool, false,
            )
            .is_none(),
        "a generator finishing after JustDeactivated must not resurrect the object"
    );
    assert_eq!(
        canonical_gameobject_snapshot(&session, owner_guid)
            .unwrap()
            .loot_state(),
        LootState::JustDeactivated
    );
    assert!(authority.snapshot_for_player_like_cpp(late).is_none());
}

#[test]
fn personal_fishing_hole_restock_accepts_second_lifecycle_and_rejects_old_observation_like_cpp() {
    let mut session = make_session();
    let first = ObjectGuid::create_player(1, 61_880);
    let second = ObjectGuid::create_player(1, 61_881);
    let owner_guid = test_gameobject_guid(61_882);
    let mut gameobject = make_canonical_gameobject_for_session(
        &session,
        owner_guid,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    gameobject.set_loot_state(LootState::Activated, Some(first));
    let mut first_pool = authoritative_test_loot_like_cpp(0, false);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    first_pool.loot_type = LOOT_TYPE_FISHINGHOLE_LIKE_CPP;
    first_pool.allowed_looters = vec![first];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(None, HashMap::from([(first, first_pool)]),)
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    let first_generation = authority.generation_like_cpp();
    attach_canonical_gameobject(&mut session, gameobject);
    session.set_player_guid(Some(second));

    let stale_observation = session
        .represented_gameobject_loot_install_observation_like_cpp(owner_guid)
        .unwrap();
    session
        .mutate_canonical_gameobject_by_guid_like_cpp(owner_guid, |gameobject| {
            gameobject.clear_loot_like_cpp();
            gameobject.set_loot_state(LootState::Ready, None);
        })
        .unwrap();

    let mut stale_pool = authoritative_test_loot_like_cpp(0, true);
    stale_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    stale_pool.loot_type = LOOT_TYPE_FISHINGHOLE_LIKE_CPP;
    stale_pool.allowed_looters = vec![second];
    stale_pool.items[0].allowed_looters = vec![second];
    assert!(
        session
            .upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
                owner_guid,
                second,
                stale_pool.clone(),
                false,
                &stale_observation,
            )
            .is_none(),
        "an async generator from before ClearLoot must lose the lifecycle CAS"
    );
    assert!(authority.is_retired_like_cpp());

    assert!(
        session
            .upsert_represented_personal_gameobject_loot_authority_like_cpp(
                owner_guid, second, stale_pool, false,
            )
            .is_some(),
        "a generator started after Ready must install the new fishing-hole lifetime"
    );
    assert!(authority.generation_like_cpp() > first_generation);
    assert!(authority.snapshot_for_player_like_cpp(second).is_some());
    assert_eq!(
        canonical_gameobject_snapshot(&session, owner_guid)
            .unwrap()
            .loot_state(),
        LootState::Ready
    );
}

#[tokio::test]
async fn loot_release_fishing_gameobjects_follow_cpp_state_branches() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let fishing_node = test_gameobject_guid(19_033);
    let fishing_hole = test_gameobject_guid(19_034);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(fishing_node);
    session.add_active_loot_view_owner_like_cpp(fishing_hole);
    for (guid, go_type, loot_type) in [
        (
            fishing_node,
            GAMEOBJECT_TYPE_FISHING_NODE as u8,
            LOOT_TYPE_FISHING_LIKE_CPP,
        ),
        (
            fishing_hole,
            GAMEOBJECT_TYPE_FISHING_HOLE as u8,
            LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
        ),
    ] {
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            guid,
            guid.entry(),
            Position::ZERO,
            go_type,
        );
        session.loot_table.insert(
            guid,
            CreatureLoot {
                loot_guid: guid,
                coins: 0,
                unlooted_count: 1,
                loot_type,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );
    }

    session
        .handle_loot_release(loot_release_packet(fishing_node))
        .await;
    session
        .handle_loot_release(loot_release_packet(fishing_hole))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&fishing_node)
            .unwrap()
            .loot_state,
        Some(LootState::JustDeactivated)
    );
    let hole_state = session
        .represented_gameobject_use_states
        .get(&fishing_hole)
        .unwrap();
    assert_eq!(hole_state.loot_state, Some(LootState::Ready));
    assert_eq!(hole_state.personal_loot_uses, 1);
}

#[tokio::test]
async fn loot_release_fishing_hole_just_deactivates_at_max_opens_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let fishing_hole = test_gameobject_guid(19_037);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(fishing_hole);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        fishing_hole,
        fishing_hole.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    session.record_represented_fishing_hole_max_opens_like_cpp(fishing_hole, 1);
    session.loot_table.insert(
        fishing_hole,
        CreatureLoot {
            loot_guid: fishing_hole,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(fishing_hole))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let hole_state = session
        .represented_gameobject_use_states
        .get(&fishing_hole)
        .unwrap();
    assert_eq!(hole_state.personal_loot_uses, 1);
    assert_eq!(hole_state.loot_state, Some(LootState::JustDeactivated));
}

#[test]
fn concurrent_fishing_hole_releases_cannot_finish_ready_after_max_like_cpp() {
    let mut first = make_session();
    let mut second = make_session();
    let fishing_hole = test_gameobject_guid(61_910);
    let gameobject = make_canonical_gameobject_for_session(
        &first,
        fishing_hole,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    attach_canonical_gameobject(&mut first, gameobject);
    second.set_canonical_map_manager(Arc::clone(first.canonical_map_manager.as_ref().unwrap()));
    let start = Arc::new(Barrier::new(2));

    std::thread::scope(|scope| {
        let first_start = Arc::clone(&start);
        let first_session = &mut first;
        let first_handle = scope.spawn(move || {
            first_start.wait();
            first_session
                .release_canonical_fishing_hole_like_cpp(fishing_hole, Some(2))
                .unwrap()
        });
        let second_start = Arc::clone(&start);
        let second_session = &mut second;
        let second_handle = scope.spawn(move || {
            second_start.wait();
            second_session
                .release_canonical_fishing_hole_like_cpp(fishing_hole, Some(2))
                .unwrap()
        });
        let first_outcome = first_handle.join().unwrap();
        let second_outcome = second_handle.join().unwrap();
        assert_eq!(
            [first_outcome.0, second_outcome.0].into_iter().max(),
            Some(2)
        );
        assert!([first_outcome.1, second_outcome.1].contains(&LootState::JustDeactivated));
    });

    let canonical = canonical_gameobject_snapshot(&first, fishing_hole).unwrap();
    assert_eq!(canonical.use_times(), 2);
    assert_eq!(canonical.loot_state(), LootState::JustDeactivated);
}

#[tokio::test]
async fn gameobject_loot_release_fishing_hole_uses_canonical_use_count_when_represented_stale_like_cpp()
 {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let fishing_hole = test_gameobject_guid(19_136);
    let mut game_object = make_canonical_gameobject_for_session(
        &session,
        fishing_hole,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    game_object.add_use_like_cpp();
    attach_canonical_gameobject(&mut session, game_object);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(fishing_hole);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        fishing_hole,
        fishing_hole.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    session.record_represented_fishing_hole_max_opens_like_cpp(fishing_hole, 2);
    session.loot_table.insert(
        fishing_hole,
        CreatureLoot {
            loot_guid: fishing_hole,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(fishing_hole))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let canonical = canonical_gameobject_snapshot(&session, fishing_hole).unwrap();
    assert_eq!(canonical.use_times(), 2);
    assert_eq!(canonical.loot_state(), LootState::JustDeactivated);
    let hole_state = session
        .represented_gameobject_use_states
        .get(&fishing_hole)
        .unwrap();
    assert_eq!(hole_state.personal_loot_uses, 2);
    assert_eq!(hole_state.loot_state, Some(LootState::JustDeactivated));
}

#[tokio::test]
async fn loot_release_gathering_node_sets_local_active_state_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gathering_node = test_gameobject_guid(19_038);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    let game_object = make_canonical_gameobject_for_session(
        &session,
        gathering_node,
        GAMEOBJECT_TYPE_GATHERING_NODE as u8,
    );
    attach_canonical_gameobject(&mut session, game_object);
    session.set_active_loot_guid(gathering_node);
    session.client_visible_guids_like_cpp.insert(gathering_node);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        gathering_node,
        gathering_node.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_GATHERING_NODE as u8,
    );
    session.loot_table.insert(
        gathering_node,
        CreatureLoot {
            loot_guid: gathering_node,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_GATHERING_NODE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(gathering_node))
        .await;

    let release_bytes = send_rx.try_recv().unwrap();
    let mut release = WorldPacket::from_bytes(&release_bytes);
    assert_eq!(
        release.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    let state = session
        .represented_gameobject_use_states
        .get(&gathering_node)
        .unwrap();
    assert_eq!(state.go_state, Some(GoState::Active));
    assert_eq!(state.loot_state, None);
    let expected = wow_packet::packets::update::UpdateObject::game_object_values_update(
        gathering_node,
        571,
        wow_packet::packets::update::GameObjectDataValuesUpdate {
            changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
            object_data: Some(wow_packet::packets::update::ObjectDataValuesUpdate {
                changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
                object_data_mask: 0x05,
                entry_id: 0,
                dynamic_flags: wow_entities::GO_DYNFLAG_LO_DEPLETED,
                scale: 0.0,
            }),
            game_object_data_mask: 0,
            state_world_effect_ids: Vec::new(),
            enable_doodad_sets: Vec::new(),
            enable_doodad_sets_update_mask: None,
            world_effects: Vec::new(),
            world_effects_update_mask: None,
            display_id: 0,
            spell_visual_id: 0,
            state_spell_visual_id: 0,
            spawn_tracking_state_anim_id: 0,
            spawn_tracking_state_anim_kit_id: 0,
            created_by: ObjectGuid::EMPTY,
            guild_guid: ObjectGuid::EMPTY,
            flags: 0,
            parent_rotation: [0.0; 4],
            faction_template: 0,
            level: 0,
            state: 0,
            type_id: 0,
            percent_health: 0,
            art_kit: 0,
            custom_param: 0,
        },
    )
    .to_bytes();
    assert_eq!(send_rx.try_recv().unwrap(), expected);
}

#[test]
fn partial_gathering_node_release_does_not_run_on_loot_release_state_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_900);
    let gathering_node = test_gameobject_guid(61_901);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        gathering_node,
        gathering_node.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_GATHERING_NODE as u8,
    );

    session.apply_represented_gameobject_loot_release_like_cpp(
        gathering_node,
        player_guid,
        false,
        false,
        None,
    );

    let state = session
        .represented_gameobject_use_states
        .get(&gathering_node)
        .unwrap();
    assert_ne!(state.go_state, Some(GoState::Active));
    assert_eq!(state.loot_state, Some(LootState::Activated));
}

#[tokio::test]
async fn loot_release_personal_chest_records_per_player_despawn_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let restocked_chest = test_gameobject_guid(19_039);
    let fallback_chest = test_gameobject_guid(19_040);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(restocked_chest);
    session.add_active_loot_view_owner_like_cpp(fallback_chest);
    session
        .client_visible_guids_like_cpp
        .insert(restocked_chest);
    session.client_visible_guids_like_cpp.insert(fallback_chest);

    for (guid, restock_time) in [(restocked_chest, 45), (fallback_chest, 0)] {
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            guid,
            guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_chest_release_metadata_like_cpp(
            guid,
            GameObjectLootSource {
                personal_loot_id: 7_001,
                chest_restock_time_secs: restock_time,
                chest_consumable: false,
                ..Default::default()
            },
        );
        session.loot_table.insert(
            guid,
            CreatureLoot {
                loot_guid: guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );
    }

    session
        .handle_loot_release(loot_release_packet(restocked_chest))
        .await;
    session
        .handle_loot_release(loot_release_packet(fallback_chest))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&restocked_chest)
    );
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&fallback_chest)
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&restocked_chest)
            .unwrap()
            .per_player_despawn_secs,
        Some(45)
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&fallback_chest)
            .unwrap()
            .per_player_despawn_secs,
        Some(wow_entities::DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS)
    );
    assert!(
        session
            .represented_gameobject_use_states
            .get(&restocked_chest)
            .unwrap()
            .per_player_despawn_until
            .is_some()
    );
    assert!(session.represented_gameobject_is_per_player_despawned_like_cpp(restocked_chest));
}

#[tokio::test]
async fn loot_release_personal_chest_without_have_at_client_sends_no_out_of_range_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = test_gameobject_guid(19_137);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(chest_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        chest_guid,
        chest_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_chest_release_metadata_like_cpp(
        chest_guid,
        GameObjectLootSource {
            personal_loot_id: 7_001,
            chest_restock_time_secs: 45,
            chest_consumable: false,
            ..Default::default()
        },
    );
    session.loot_table.insert(
        chest_guid,
        CreatureLoot {
            loot_guid: chest_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(chest_guid))
        .await;

    let release_bytes = send_rx.try_recv().unwrap();
    let mut release = WorldPacket::from_bytes(&release_bytes);
    assert_eq!(
        release.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert!(send_rx.try_recv().is_err());
    let state = session
        .represented_gameobject_use_states
        .get(&chest_guid)
        .unwrap();
    assert_eq!(state.per_player_despawn_secs, Some(45));
    assert!(state.per_player_despawn_until.is_some());
    assert_eq!(state.per_player_state_player_guid, Some(player_guid));
}

#[tokio::test]
async fn loot_release_shared_chest_restock_starts_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let partial_chest = test_gameobject_guid(19_041);
    let full_chest = test_gameobject_guid(19_042);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(partial_chest);
    session.add_active_loot_view_owner_like_cpp(full_chest);

    for guid in [partial_chest, full_chest] {
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            guid,
            guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_chest_release_metadata_like_cpp(
            guid,
            GameObjectLootSource {
                loot_id: 7_001,
                chest_restock_time_secs: 45,
                chest_consumable: false,
                ..Default::default()
            },
        );
    }
    session.loot_table.insert(
        partial_chest,
        CreatureLoot {
            loot_guid: partial_chest,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: Vec::new(),
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );
    session.loot_table.insert(
        full_chest,
        CreatureLoot {
            loot_guid: full_chest,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(partial_chest))
        .await;
    session
        .handle_loot_release(loot_release_packet(full_chest))
        .await;

    let partial_state = session
        .represented_gameobject_use_states
        .get(&partial_chest)
        .unwrap();
    assert_eq!(partial_state.loot_state, Some(LootState::Activated));
    assert!(partial_state.chest_restock_until.is_some());
    assert!(session.loot_table.contains_key(&partial_chest));

    let full_state = session
        .represented_gameobject_use_states
        .get(&full_chest)
        .unwrap();
    assert_eq!(full_state.loot_state, Some(LootState::NotReady));
    assert!(full_state.chest_restock_until.is_some());
    assert!(!session.loot_table.contains_key(&full_chest));
}

#[tokio::test]
async fn process_pending_shared_chest_restock_clears_loot_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let chest_guid = test_gameobject_guid(19_043);
    session.set_state(SessionState::LoggedIn);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        chest_guid,
        chest_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    {
        let state = session
            .represented_gameobject_use_states
            .get_mut(&chest_guid)
            .unwrap();
        state.loot_state = Some(LootState::Activated);
        state.chest_restock_until = Some(Instant::now() - Duration::from_secs(1));
    }
    session.loot_table.insert(
        chest_guid,
        CreatureLoot {
            loot_guid: chest_guid,
            coins: 7,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&chest_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(LootState::Ready));
    assert!(state.chest_restock_until.is_none());
    assert!(!session.loot_table.contains_key(&chest_guid));
}

#[tokio::test]
async fn process_pending_shared_chest_restock_syncs_state_to_same_map_viewers_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let chest_guid = test_gameobject_guid(19_044);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());

    session.set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(player_registry);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        chest_guid,
        chest_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_chest_release_metadata_like_cpp(
        chest_guid,
        GameObjectLootSource {
            loot_id: 7_002,
            chest_restock_time_secs: 45,
            chest_consumable: false,
            ..Default::default()
        },
    );
    {
        let state = session
            .represented_gameobject_use_states
            .get_mut(&chest_guid)
            .unwrap();
        state.loot_state = Some(LootState::NotReady);
        state.chest_restock_until = Some(Instant::now() - Duration::from_secs(1));
    }
    session.loot_table.insert(
        chest_guid,
        CreatureLoot {
            loot_guid: chest_guid,
            coins: 7,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session.process_pending().await;

    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command)) => command,
        other => panic!("expected restock chest sync command, got {other:?}"),
    };
    assert_eq!(command.gameobject_guid, chest_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.loot_state, Some(LootState::Ready as u8));
    assert_eq!(command.loot_state_unit_guid, ObjectGuid::EMPTY);
    assert_eq!(command.chest_loot_id, 7_002);
    assert_eq!(command.chest_restock_time_secs, 45);
}
