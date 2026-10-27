//! C++ Player::_SaveSpells consumes only rows visited while preparing the save
//! (Player.cpp:20399-20451). Rust's confirmed-COMMIT gate must not consume a later row.
use super::*;

fn new_spell(id: i32) -> wow_entities::PlayerKnownSpellRecord {
    wow_entities::PlayerKnownSpellRecord {
        spell_id: id,
        state: wow_entities::PlayerSpellLoadState::New,
        active: true,
        disabled: false,
        favorite: false,
        dependent: false,
    }
}

fn canonical_session(
    outcome: PersistenceOutcomeLikeCpp,
) -> (WorldSession, Arc<RecordingPortLikeCpp>) {
    let (mut session, port) = session_with_port(outcome);
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session
        .with_owned_player_mut_like_cpp(|player| {
            let spells = &mut player.gameplay_state_mut().spells;
            spells.set_row_authority_like_cpp(true, true);
            spells.insert_row_like_cpp(10, new_spell(10));
        })
        .unwrap();
    (session, port)
}

fn install_dirty_failed_quest(session: &mut WorldSession) {
    let mut quest = test_quest_template(81001);
    quest.objectives.push(wow_entities::QuestObjective {
        id: 810010,
        quest_id: 81001,
        obj_type: 0,
        order: 0,
        storage_index: 0,
        object_id: 1,
        amount: 10,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .gameplay_state_mut()
                .quests
                .add_active_status_like_cpp(
                    81001,
                    wow_entities::PlayerQuestStatusRecord {
                        quest_id: 81001,
                        status: wow_conditions::QUEST_STATUS_FAILED_LIKE_CPP,
                        explored: false,
                        accept_time_secs: 12,
                        end_time_secs: 999,
                        objective_counts: vec![99],
                        slot: 0,
                    },
                );
            player.set_quest_slot_like_cpp(0, 81001);
            player.set_quest_slot_end_time_like_cpp(0, 34);
            player.set_quest_slot_counter_like_cpp(0, 0, 7);
        })
        .unwrap();
}

#[tokio::test]
async fn full_save_quest_ack_preserves_native_progress_changed_during_commit() {
    let (mut session, port) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    install_dirty_failed_quest(&mut session);
    let handle = session.core.player_handle_like_cpp.unwrap();
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    *port.during_save.lock().unwrap() = Some(Box::new(move || {
        manager
            .try_lock()
            .unwrap()
            .with_player_mut_like_cpp(handle, |player| {
                player.set_quest_slot_counter_like_cpp(0, 0, 8);
            })
            .unwrap();
    }));
    session.save_current_player_to_db_like_cpp().await;
    let requests = port.character_saves();
    let wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp::Save { status, .. } =
        &requests[0].quest_statuses.as_ref().unwrap()[0]
    else {
        panic!("expected active quest save")
    };
    assert_eq!(status.status, wow_conditions::QUEST_STATUS_FAILED_LIKE_CPP);
    assert_eq!(status.end_time_secs, 34);
    assert_eq!(
        status.objectives[0].count, 7,
        "save reads native fields, not the status compatibility projection"
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|p| p
            .gameplay_state()
            .quests
            .status_save_type_like_cpp(81001)),
        Some(Some(
            wow_entities::PlayerQuestStatusSaveTypeLikeCpp::Default
        ))
    );
    session.save_current_player_to_db_like_cpp().await;
    assert_eq!(
        session.with_owned_player_like_cpp(|p| p
            .gameplay_state()
            .quests
            .status_save_type_like_cpp(81001)),
        Some(None)
    );
    let requests = port.character_saves();
    let wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp::Save { status, .. } =
        &requests[1].quest_statuses.as_ref().unwrap()[0]
    else {
        panic!("expected retry save")
    };
    assert_eq!(status.objectives[0].count, 8);
}

#[tokio::test]
async fn full_save_failed_or_unknown_commit_keeps_quest_dirty() {
    for outcome in [
        PersistenceOutcomeLikeCpp::Failed {
            reason: "rollback".into(),
        },
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "lost commit reply".into(),
        },
    ] {
        let (mut session, port) = canonical_session(outcome);
        install_dirty_failed_quest(&mut session);
        session.save_current_player_to_db_like_cpp().await;
        assert_eq!(port.character_saves().len(), 1);
        assert_eq!(
            session.with_owned_player_like_cpp(|p| p
                .gameplay_state()
                .quests
                .status_save_type_like_cpp(81001)),
            Some(Some(
                wow_entities::PlayerQuestStatusSaveTypeLikeCpp::Default
            ))
        );
    }
}

fn add_dirty_durability_item(session: &mut WorldSession, item_id: i64, durability: u32) {
    session.set_item_effect_store(Arc::new(wow_data::ItemEffectStore::from_entries([])));
    let player_guid = session.player_guid().expect("selected player guid");
    session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .inventory_runtime_mut_like_cpp()
                .set_equipment_inventory_authority_complete_like_cpp(true);
            let mut item = wow_entities::Item::new(0);
            item.object_mut()
                .create(wow_core::ObjectGuid::create_item(1, item_id));
            item.set_owner_guid(player_guid);
            item.set_max_durability(100);
            item.set_durability(durability);
            item.force_state(wow_constants::ItemUpdateState::Changed);
            player
                .inventory_runtime_mut_like_cpp()
                .store_item_object_like_cpp(item);
        })
        .unwrap();
}

#[tokio::test]
async fn full_save_reads_native_map_and_level_despite_stale_session_staging() {
    // Player.cpp:19480-19495 saves GetLevel()/GetMapId(), not Session mirrors.
    for detached in [false, true] {
        let (mut session, port) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
        install_canonical_player_owner_for_test(&mut session, 571, 43);
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        session
            .with_owned_player_mut_like_cpp(|player| player.unit_mut().set_level(73))
            .unwrap();
        session.core.current_map_id = 1;
        session.fixtures.identity.player_level = 11;
        let prepared = session.prepare_player_save_like_cpp(123).unwrap();
        assert_eq!((prepared.header.map_id, prepared.header.level), (571, 73));
        assert_eq!(prepared.request.character.position.map_id, 571);
        assert_eq!(prepared.request.character.position.instance_id, 43);
        assert_eq!(prepared.request.character.level, 73);
        session.save_current_player_to_db_like_cpp().await;
        let requests = port.character_saves();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].character.position.map_id, 571);
        assert_eq!(requests[0].character.position.instance_id, 43);
        assert_eq!(requests[0].character.level, 73);
    }
}

#[tokio::test]
async fn full_save_ack_cleans_the_reputation_row_the_player_owns_per_list_id() {
    let (mut session, port) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session
        .with_owned_player_mut_like_cpp(|p| {
            // #735: the Player owns C++ `FactionStateList` keyed by
            // `ReputationListID` (`ReputationMgr.h:63`), so a second row for the
            // same key replaces the first instead of shadowing it in a vector
            // the save projection would silently drop.
            let reputation = p.reputation_mut_like_cpp();
            reputation.insert_faction_like_cpp(wow_entities::PlayerFactionStateLikeCpp {
                faction_id: 72,
                reputation_list_id: 1,
                standing: 10,
                need_save: true,
                ..Default::default()
            });
            reputation.insert_faction_like_cpp(wow_entities::PlayerFactionStateLikeCpp {
                faction_id: 76,
                reputation_list_id: 1,
                standing: 20,
                need_save: true,
                ..Default::default()
            });
        })
        .unwrap();
    session.save_current_player_to_db_like_cpp().await;
    let requests = port.character_saves();
    assert_eq!(requests[0].reputations.len(), 1);
    assert_eq!(requests[0].reputations[0].faction_id, 76);
    assert_eq!(
        session.with_owned_player_like_cpp(|p| {
            p.reputation_like_cpp()
                .factions_like_cpp()
                .map(|row| row.need_save)
                .collect::<Vec<_>>()
        }),
        Some(vec![false])
    );
}

#[tokio::test]
async fn full_save_ack_rebases_changed_new_spell_for_the_next_transaction() {
    let (mut session, port) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    add_dirty_durability_item(&mut session, 20, 8);
    let item_guid = wow_core::ObjectGuid::create_item(1, 20);
    let handle = session.core.player_handle_like_cpp.unwrap();
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    *port.during_save.lock().unwrap() = Some(Box::new(move || {
        manager
            .try_lock()
            .unwrap()
            .with_player_mut_like_cpp(handle, |player| {
                player
                    .gameplay_state_mut()
                    .spells
                    .row_mut_like_cpp(10)
                    .unwrap()
                    .favorite = true;
                player
                    .inventory_runtime_mut_like_cpp()
                    .item_objects_mut()
                    .get_mut(&item_guid)
                    .unwrap()
                    .set_durability(4);
            })
            .unwrap();
    }));
    session.save_current_player_to_db_like_cpp().await;
    assert_eq!(
        session
            .with_owned_player_like_cpp(|p| p.gameplay_state().spells.rows_like_cpp()[&10].state),
        Some(wow_entities::PlayerSpellLoadState::Changed)
    );
    assert_eq!(
        port.character_saves()[0]
            .inventory_items
            .as_ref()
            .map(|items| items
                .iter()
                .map(|item| (item.item_guid, item.durability))
                .collect::<Vec<_>>()),
        Some(vec![(20, 8)]),
        "the transaction captures only the pre-await canonical durability"
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|player| {
            let item = &player.inventory_runtime_like_cpp().item_objects()[&item_guid];
            (item.data().durability, item.update_state())
        }),
        Some((4, wow_constants::ItemUpdateState::Changed)),
        "full-save acknowledgement does not clear or overwrite item state changed during I/O"
    );
    session.save_current_player_to_db_like_cpp().await;
    let requests = port.character_saves();
    let Some(wow_persistence::PlayerSpellSaveGroupLikeCpp::Complete { rows, .. }) =
        &requests[1].spells
    else {
        panic!("complete spell authority")
    };
    assert_eq!(
        rows[0].state,
        wow_persistence::PlayerSpellStateLikeCpp::Changed
    );
    assert!(rows[0].favorite);
    assert_eq!(
        session
            .with_owned_player_like_cpp(|p| p.gameplay_state().spells.rows_like_cpp()[&10].state),
        Some(wow_entities::PlayerSpellLoadState::Unchanged)
    );
}

#[tokio::test]
async fn full_save_rollback_and_unknown_leave_native_dirty_state_and_distinct_fences() {
    for unknown in [false, true] {
        let outcome = if unknown {
            PersistenceOutcomeLikeCpp::Unknown {
                reason: "lost reply".into(),
            }
        } else {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "definite rollback".into(),
            }
        };
        let (mut session, port) = canonical_session(outcome);
        add_dirty_durability_item(&mut session, 30, 6);
        session.save_current_player_to_db_like_cpp().await;
        assert_eq!(port.character_saves().len(), 1);
        assert_eq!(
            port.character_saves()[0]
                .inventory_items
                .as_ref()
                .map(|items| items
                    .iter()
                    .map(|item| (item.item_guid, item.durability))
                    .collect::<Vec<_>>()),
            Some(vec![(30, 6)])
        );
        assert_eq!(
            session.with_owned_player_like_cpp(|player| {
                let item = &player.inventory_runtime_like_cpp().item_objects()
                    [&wow_core::ObjectGuid::create_item(1, 30)];
                (item.data().durability, item.update_state())
            }),
            Some((6, wow_constants::ItemUpdateState::Changed)),
            "failed or indeterminate commits retain canonical dirty item durability"
        );
        assert_eq!(
            session.with_owned_player_like_cpp(
                |p| p.gameplay_state().spells.rows_like_cpp()[&10].state
            ),
            Some(wow_entities::PlayerSpellLoadState::New)
        );
        assert_eq!(
            session
                .durable_loot_money_persistence_tracker_like_cpp()
                .is_indeterminate_like_cpp(),
            unknown
        );
    }
}

#[tokio::test]
async fn full_save_cancellation_keeps_receipt_unapplied_and_quarantines_unknown_commit() {
    let (mut session, port) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    port.save_pending
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    let mut future = Box::pin(session.save_current_player_to_db_like_cpp());
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(future.as_mut().poll(cx).is_pending()))
            .await
    );
    assert_eq!(port.character_saves().len(), 1);
    manager.try_lock().unwrap().update(10);
    drop(future);
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
    assert_eq!(
        session
            .with_owned_player_like_cpp(|p| p.gameplay_state().spells.rows_like_cpp()[&10].state),
        Some(wow_entities::PlayerSpellLoadState::New)
    );
}

/// The completion-before-capture fence: a durable Item-loot completion that
/// outlived its packet waiter is published before the save captures its
/// snapshot. A capture first would persist the pre-completion balance.
#[tokio::test]
async fn full_save_publishes_pending_durable_item_loot_completion_before_capture() {
    let (mut session, port) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    let player_guid = session.player_guid().expect("canonical Player GUID");
    session
        .with_owned_player_mut_like_cpp(|player| player.set_money(100))
        .unwrap();
    let balance_applied = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut guard = session.begin_durable_item_loot_persistence_like_cpp();
    guard.mark_committed_like_cpp(wow_world_lifecycle::DurableItemLootCompletionLikeCpp {
        owner_guid: ObjectGuid::create_item(1, 61_848),
        loot_list_id: 0,
        player_guid,
        item_owner_auto_release: false,
        durable_item_money_applied_amount: Some(7),
        durable_item_money_notified_amount: Some(7),
        durable_item_money_balance_applied: Some(Arc::clone(&balance_applied)),
        item_fanout: None,
        runtime_inventory_applied: Arc::new(std::sync::atomic::AtomicBool::new(true)),
    });
    drop(guard);

    session.save_current_player_to_db_like_cpp().await;

    let requests = port.character_saves();
    assert_eq!(requests.len(), 1);
    assert!(
        balance_applied.load(std::sync::atomic::Ordering::Acquire),
        "the save must publish the committed Item-loot money before it captures"
    );
    assert_eq!(
        requests[0].character.money, 107,
        "capture must observe the completion the save published, not the pre-completion balance"
    );
}

#[test]
fn full_save_preparation_is_owned_and_matches_previous_projection_for_loaded_groups() {
    let (mut session, _) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.mark_represented_glyphs_loaded_like_cpp();
    session.mark_represented_character_spell_cooldowns_loaded_like_cpp();
    session.record_loaded_character_spell_cooldown_like_cpp(635, 6948, 9_000, 12, 8_000);
    session.mark_represented_character_spell_charges_loaded_like_cpp();
    session.record_loaded_character_spell_charge_like_cpp(42, 7_000, 8_000);
    session.lifecycle.set_tutorials_changed_like_cpp(true);
    session
        .lifecycle
        .set_tutorials_loaded_coherently_for_test_like_cpp(true);
    session
        .with_owned_player_mut_like_cpp(|p| {
            let game = p.gameplay_state_mut();
            game.equipment_sets.mark_loaded_like_cpp();
            let mut stored = wow_entities::PlayerEquipmentSetLikeCpp::equipment(
                1,
                0,
                wow_entities::PlayerEquipmentSetUpdateStateLikeCpp::New,
            );
            stored.guid = 1;
            game.equipment_sets.install_loaded_set_like_cpp(stored);
            game.cuf_profiles_loaded = true;
            game.action_buttons_loaded = true;
            game.talents.mark_talents_loaded_like_cpp();
        })
        .unwrap();
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let header = session
            .current_player_save_to_db_snapshot_like_cpp()
            .unwrap();
        let old = session
            .current_player_character_save_request_like_cpp(&header, 123)
            .unwrap();
        let prepared = session.prepare_player_save_like_cpp(123).unwrap();
        assert_eq!(prepared.request, old);
        session
            .with_owned_player_mut_like_cpp(|p| {
                p.set_money(p.money() + 1);
            })
            .unwrap();
        assert_eq!(prepared.request.character.money, old.character.money);
        assert!(
            session
                .core
                .canonical_map_manager
                .as_ref()
                .unwrap()
                .try_lock()
                .is_ok()
        );
    }
}

#[tokio::test]
async fn full_save_ack_does_not_clean_a_spell_added_after_capture() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let handle = session.core.player_handle_like_cpp.unwrap();
    session
        .with_owned_player_mut_like_cpp(|player| {
            let spells = &mut player.gameplay_state_mut().spells;
            spells.set_row_authority_like_cpp(true, true);
            spells.insert_row_like_cpp(10, new_spell(10));
        })
        .unwrap();
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    *port.during_save.lock().unwrap() = Some(Box::new(move || {
        // Real shared owner remains usable while persistence is pending.
        manager
            .try_lock()
            .expect("save must release its owner guard before I/O")
            .with_player_mut_like_cpp(handle, |player| {
                player
                    .gameplay_state_mut()
                    .spells
                    .insert_row_like_cpp(20, new_spell(20));
            })
            .unwrap();
    }));
    session.save_current_player_to_db_like_cpp().await;
    assert_eq!(port.character_saves().len(), 1);
    session
        .with_owned_player_like_cpp(|player| {
            assert_eq!(
                player.gameplay_state().spells.rows_like_cpp()[&10].state,
                wow_entities::PlayerSpellLoadState::Unchanged
            );
            assert_eq!(
                player.gameplay_state().spells.rows_like_cpp()[&20].state,
                wow_entities::PlayerSpellLoadState::New,
                "the confirmed transaction never contained the later spell"
            );
        })
        .unwrap();
}

#[tokio::test]
async fn full_save_stale_core_receipt_does_not_acknowledge_retired_handle() {
    let (mut session, port) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    let handle = session.core.player_handle_like_cpp.unwrap();
    let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    let retired = Arc::new(std::sync::Mutex::new(None::<Box<wow_entities::Player>>));
    let retired_during_save = Arc::clone(&retired);
    let replacement_handle = Arc::new(std::sync::Mutex::new(None));
    let replacement_handle_during_save = Arc::clone(&replacement_handle);
    *port.during_save.lock().unwrap() = Some(Box::new(move || {
        let mut manager = manager
            .try_lock()
            .expect("the canonical owner guard must be released before persistence");
        let player = manager
            .retire_player_like_cpp(handle)
            .expect("the captured owner is retired while the save is pending");
        let mut replacement = Box::new(wow_entities::Player::new(Some(1), false));
        replacement
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(handle.guid());
        let spells = &mut replacement.gameplay_state_mut().spells;
        spells.set_row_authority_like_cpp(true, true);
        spells.insert_row_like_cpp(10, new_spell(10));
        spells.insert_row_like_cpp(20, new_spell(20));
        let replacement_handle = manager
            .install_detached_player_like_cpp(replacement)
            .expect("the replacement has the same Player GUID and a fresh generation");
        drop(manager);
        *retired_during_save.lock().unwrap() = Some(player);
        *replacement_handle_during_save.lock().unwrap() = Some(replacement_handle);
    }));

    session.save_current_player_to_db_like_cpp().await;

    let requests = port.character_saves();
    assert_eq!(requests.len(), 1);
    assert!(matches!(
        &requests[0].spells,
        Some(wow_persistence::PlayerSpellSaveGroupLikeCpp::Complete { rows, .. })
            if rows.iter().any(|row| row.spell_id == 10)
    ));
    let replacement_handle = replacement_handle
        .lock()
        .unwrap()
        .take()
        .expect("same-GUID replacement was installed while persistence was pending");
    assert_eq!(replacement_handle.guid(), handle.guid());
    assert_ne!(replacement_handle.generation(), handle.generation());
    let retired = retired
        .lock()
        .unwrap()
        .take()
        .expect("the previous canonical incarnation was retired");
    assert_eq!(
        retired.gameplay_state().spells.rows_like_cpp()[&10].state,
        wow_entities::PlayerSpellLoadState::New,
        "the receipt must not acknowledge rows on its retired owner"
    );
    {
        let manager = session
            .core
            .canonical_map_manager
            .as_ref()
            .expect("the map manager remains installed")
            .lock()
            .expect("the map manager lock is available after save completion");
        manager
            .with_player_like_cpp(replacement_handle, |player| {
                assert_eq!(
                    player.gameplay_state().spells.rows_like_cpp()[&10].state,
                    wow_entities::PlayerSpellLoadState::New,
                    "the old receipt must not clean the same spell row on a new incarnation"
                );
                assert_eq!(
                    player.gameplay_state().spells.rows_like_cpp()[&20].state,
                    wow_entities::PlayerSpellLoadState::New,
                    "replacement-only dirty rows must remain untouched"
                );
            })
            .expect("the replacement incarnation remains canonical");
    }
}

#[test]
fn full_save_receipt_intersects_committed_groups_before_acknowledging_rows_and_tutorials() {
    let (mut session, _) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session
        .lifecycle
        .set_tutorials_loaded_coherently_for_test_like_cpp(true);
    session
        .lifecycle
        .set_tutorials_loaded_from_db_like_cpp(true);
    session
        .lifecycle
        .load_tutorials_data_values_like_cpp(Some([7; 8]));
    session.lifecycle.set_tutorials_changed_like_cpp(true);

    let talent_store = session.catalogs.talent_store().map(AsRef::as_ref);
    let spell_store = session
        .catalogs
        .spell_catalogs
        .spell_store()
        .map(AsRef::as_ref);
    let mut owner = session.core.player_save_operation_access_like_cpp(
        talent_store,
        spell_store,
        None,
        None,
        None,
        None,
        None,
        session.catalogs.quests.store.as_deref(),
        session.config.quest_status_keep_abandoned_like_cpp,
    );
    let inputs = session.lifecycle.player_save_session_inputs_like_cpp(1_000);
    let captured = owner
        .capture_like_cpp(inputs)
        .expect("the real canonical owner produces a save receipt");
    let (request, _header, receipt) = captured.into_parts_like_cpp();
    let expected = request.committed_groups_like_cpp();
    assert!(expected.player_spells);
    assert!(expected.tutorials_changed);
    assert!(!expected.tutorials_insert);

    let committed = wow_persistence::PlayerCharacterCommittedGroupsLikeCpp {
        player_spells: false,
        tutorials_changed: false,
        tutorials_insert: true,
        ..Default::default()
    };
    assert!(!committed.player_spells);
    assert!(committed.tutorials_insert);
    let acknowledged = owner
        .acknowledge_like_cpp(receipt, &committed)
        .expect("the receipt still names the current canonical incarnation");
    assert!(!acknowledged.groups.player_spells);
    assert!(!acknowledged.groups.tutorials_changed);
    assert!(
        !acknowledged.groups.tutorials_insert,
        "a committed bit cannot acknowledge a group absent from the captured request"
    );
    drop(owner);

    assert!(
        !wow_world_lifecycle::apply_player_save_acknowledgement_like_cpp(
            &mut session.lifecycle,
            Some(&acknowledged),
        )
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|player| player.gameplay_state().spells.rows_like_cpp()
            [&10]
            .state),
        Some(wow_entities::PlayerSpellLoadState::New),
        "an uncommitted expected spell group remains dirty"
    );
    assert_eq!(session.lifecycle.tutorial_values_like_cpp(), &[7; 8]);
    assert!(session.lifecycle.tutorials_changed_like_cpp());
}

#[tokio::test]
async fn full_save_receipt_preserves_tutorials_changed_after_capture() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session
        .with_owned_player_mut_like_cpp(|player| {
            let spells = &mut player.gameplay_state_mut().spells;
            spells.set_row_authority_like_cpp(true, true);
            spells.insert_row_like_cpp(10, new_spell(10));
        })
        .unwrap();
    session
        .lifecycle
        .set_tutorials_loaded_coherently_for_test_like_cpp(true);
    session
        .lifecycle
        .load_tutorials_data_values_like_cpp(Some([7; 8]));
    session.lifecycle.set_tutorials_changed_like_cpp(true);

    let talent_store = session.catalogs.talent_store().map(AsRef::as_ref);
    let spell_store = session
        .catalogs
        .spell_catalogs
        .spell_store()
        .map(AsRef::as_ref);
    let mut owner = session.core.player_save_operation_access_like_cpp(
        talent_store,
        spell_store,
        None,
        None,
        None,
        None,
        None,
        session.catalogs.quests.store.as_deref(),
        session.config.quest_status_keep_abandoned_like_cpp,
    );
    let inputs = session.lifecycle.player_save_session_inputs_like_cpp(1_000);
    let captured = owner
        .capture_like_cpp(inputs)
        .expect("canonical Player and loaded save authorities are available");
    let (request, header, receipt) = captured.into_parts_like_cpp();
    assert!(request.committed_groups_like_cpp().tutorials_changed);

    let result = wow_world_lifecycle::persist_player_save_request_like_cpp(
        &session.lifecycle,
        &mut owner,
        header.guid,
        request,
    )
    .await;
    let wow_world_lifecycle::PlayerSavePersistenceResultLikeCpp::Applied { committed, .. } = result
    else {
        panic!("the recording lifecycle port applies the captured request");
    };

    session.lifecycle.set_tutorial_int_like_cpp(0, 99);
    session.lifecycle.set_tutorials_changed_like_cpp(true);
    let acknowledged = owner.acknowledge_like_cpp(receipt, &committed);
    drop(owner);
    assert!(
        wow_world_lifecycle::apply_player_save_acknowledgement_like_cpp(
            &mut session.lifecycle,
            acknowledged.as_ref(),
        )
    );
    assert_eq!(session.lifecycle.tutorial_values_like_cpp()[0], 99);
    assert!(session.lifecycle.tutorials_changed_like_cpp());
    assert_eq!(port.character_saves().len(), 1);
}
