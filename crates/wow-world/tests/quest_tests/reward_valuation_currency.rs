//! Currency quest reward owner regression migrated from the external suite.

use super::*;
use std::sync::Arc;
use wow_world::handlers::quest::PlayerQuestStatus;
use wow_world::test_fixtures::{
    CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP,
    QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP, QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP,
};
use wow_world::session::WorldSession;
use wow_world::test_fixtures::*;
use wow_entities::{PlayerCurrency, PlayerCurrencyState};
use wow_data_model::currency::CurrencyGainDelta as PlayerCurrencyDelta;
use wow_constants::quest::QUEST_OBJECTIVE_CURRENCY_LIKE_CPP as QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL;
use wow_constants::currency::CurrencyGainSourceLikeCpp;
use wow_constants::quest::QUEST_STATUS_COMPLETE_LIKE_CPP;
use wow_core::{ObjectGuid, ObjectGuidGenerator, Position, guid::HighGuid};
use wow_data::quest::{
    QuestObjective, QuestStore, QuestTemplate,
    QUEST_ITEM_DROP_COUNT, QUEST_REWARD_CHOICES_COUNT, QUEST_REWARD_CURRENCY_COUNT,
    QUEST_REWARD_DISPLAY_SPELL_COUNT, QUEST_REWARD_ITEM_COUNT,
    QUEST_REWARD_REPUTATIONS_COUNT,
};
use wow_data::{CurrencyTypesEntry, CurrencyTypesStore};
use wow_packet::{ServerPacket, WorldPacket};

#[test]
fn quest_xp_valuation_wiring_preserves_df_gate_and_missing_store_fallback() {
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7020);
    quest.reward_xp_difficulty = 1;
    assert_eq!(quest_xp_reward_for_test(&session, &quest), 50);

    set_player_quest_gameplay_rewarded_for_test(&mut session, quest.id);
    assert_eq!(quest_xp_reward_for_test(&session, &quest), 0);
    quest.special_flags = wow_data::quest::QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
    assert_eq!(quest_xp_reward_for_test(&session, &quest), 50);

    quest.reward_xp_difficulty = 10;
    assert_eq!(quest_xp_reward_for_test(&session, &quest), 4000);
    session.set_quest_xp_store(Arc::new(wow_data::quest_xp::QuestXpStore::default()));
    assert_eq!(quest_xp_reward_for_test(&session, &quest), 0);
    quest.reward_xp_difficulty = 1;
    assert_eq!(quest_xp_reward_for_test(&session, &quest), 0);
}

#[test]
fn quest_money_valuation_wiring_preserves_level_row_and_rounding() {
    use wow_data::progression_rewards::{QuestMoneyRewardEntry, QuestMoneyRewardStore};

    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7021);
    quest.reward_money_difficulty = 1;
    quest.reward_money_multiplier = 1.5;
    assert_eq!(quest_money_reward_for_test(&session, &quest), 0);

    session.set_quest_money_reward_store(Arc::new(QuestMoneyRewardStore::from_entries([
        QuestMoneyRewardEntry { id: 1, difficulty: [0, 3, 0, 0, 0, 0, 0, 0, 0, 0] },
        QuestMoneyRewardEntry { id: 70, difficulty: [0, 7, 0, 0, 0, 0, 0, 0, 0, 0] },
    ])));
    assert_eq!(quest_money_reward_for_test(&session, &quest), 5);
    quest.quest_level = 0;
    quest.quest_max_scaling_level = 70;
    assert_eq!(quest_money_reward_for_test(&session, &quest), 11);
    quest.reward_money_difficulty = 10;
    assert_eq!(quest_money_reward_for_test(&session, &quest), 0);
    quest.reward_money_difficulty = 1;
    quest.quest_max_scaling_level = 0;
    assert_eq!(quest_money_reward_for_test(&session, &quest), 0);
}

#[test]
fn quest_currency_zero_gain_does_not_require_catalog_or_owner() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(None);
    assert!(session.currency_types_store().is_none());
    assert_eq!(quest_currency_gain_for_test(&mut session, 
        394, 0, CurrencyGainSourceLikeCpp::QuestReward,
    ), Ok(None));
    assert!(currency_compatibility_fixture_for_test(&session).is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn quest_currency_catalog_team_and_award_gates_preserve_order() {
    let (mut session, send_rx) = make_session();
    assert_eq!(quest_currency_gain_for_test(&mut session, 
        394, 1, CurrencyGainSourceLikeCpp::QuestReward,
    ), Err(()));

    let mut entry = CurrencyTypesEntry {
        flags: wow_constants::CurrencyTypesFlags::IS_HORDE_ONLY,
        award_condition_id: 1,
        faction_id: 1,
        ..currency_entry_like_cpp(394)
    };
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([entry])));
    assert_eq!(quest_currency_gain_for_test(&mut session, 
        394, 1, CurrencyGainSourceLikeCpp::QuestReward,
    ), Ok(None), "team filtering precedes award conditions");

    entry.flags = wow_constants::CurrencyTypesFlags::empty();
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([entry])));
    assert_eq!(quest_currency_gain_for_test(&mut session, 
        394, 1, CurrencyGainSourceLikeCpp::QuestReward,
    ), Err(()), "award conditions precede faction conversion filtering");

    entry.award_condition_id = 0;
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([entry])));
    assert_eq!(quest_currency_gain_for_test(&mut session, 
        394, 1, CurrencyGainSourceLikeCpp::QuestReward,
    ), Ok(None));

    let azerite_id = wow_constants::CurrencyTypes::Azerite as u32;
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        currency_entry_like_cpp(azerite_id),
    ])));
    assert_eq!(quest_currency_gain_for_test(&mut session, 
        azerite_id, 1, CurrencyGainSourceLikeCpp::QuestReward,
    ), Ok(None));
    assert!(currency_compatibility_fixture_for_test(&session).is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn quest_currency_gain_installs_single_delta_and_retains_new_state() {
    let (mut session, send_rx) = make_session();
    let entry = CurrencyTypesEntry {
        max_qty: 10,
        max_earnable_per_week: 7,
        flags: wow_constants::CurrencyTypesFlags::TRACK_QUANTITY
            | wow_constants::CurrencyTypesFlags::SUPPRESS_CHAT_MESSAGES,
        flags_b: wow_constants::CurrencyTypesFlagsB::USE_TOTAL_EARNED_FOR_EARNED,
        ..currency_entry_like_cpp(394)
    };
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([entry])));
    let first = quest_currency_gain_for_test(&mut session, 
        394, 5, CurrencyGainSourceLikeCpp::QuestReward,
    ).unwrap().unwrap();
    assert_eq!(first.amount, 5);
    assert_eq!(currency_compatibility_fixture_for_test(&session).get(&394).unwrap().state, PlayerCurrencyState::New);
    let second = quest_currency_gain_for_test(&mut session, 
        394, 10, CurrencyGainSourceLikeCpp::QuestReward,
    ).unwrap().unwrap();
    assert_eq!(second, PlayerCurrencyDelta {
        currency_id: 394,
        quantity: 7,
        amount: 2,
        weekly_quantity: Some(7),
        max_quantity: Some(10),
        total_earned: Some(7),
        suppress_chat_log: true,
    });
    assert_eq!(currency_compatibility_fixture_for_test(&session).get(&394), Some(&PlayerCurrency {
        state: PlayerCurrencyState::New,
        quantity: 7,
        weekly_quantity: 7,
        tracked_quantity: 7,
        increased_cap_quantity: 0,
        earned_quantity: 7,
        flags: 0,
    }));
    assert!(send_rx.try_recv().is_err(), "publication belongs to the reward handler");
}

#[test]
fn quest_currency_canonical_noop_and_stale_owner_do_not_install_or_publish() {
    let (mut session, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let player_guid = ObjectGuid::create_player(1, 7022);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    attach_player_controller_for_test(&mut session, 
        player_guid,
        "QuestCurrencyOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571, 1, 1, 20, 0,
    );
    ensure_world_map_for_current_player_for_test(&mut session).expect("world map");
    let old_handle = quest_player_handle_for_test(&session).expect("canonical owner");
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        CurrencyTypesEntry { max_qty: 13, ..currency_entry_like_cpp(394) },
    ])));
    assert_eq!(quest_currency_gain_for_test(&mut session, 
        394, 10, CurrencyGainSourceLikeCpp::QuestReward,
    ).unwrap().unwrap().quantity, 10);
    assert!(remove_current_quest_player_from_map_for_test(&mut session));
    assert_eq!(canonical.lock().unwrap().player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached));
    assert_eq!(quest_currency_gain_for_test(&mut session, 
        394, 3, CurrencyGainSourceLikeCpp::QuestReward,
    ).unwrap().unwrap().quantity, 13);

    let before = player_currencies_snapshot_for_test(&session).unwrap();
    mutate_currency_compatibility_fixture_for_test(&mut session, |currencies| {
        currencies.get_mut(&394).unwrap().quantity = 999;
    });
    let fixture_before = currency_compatibility_fixture_for_test(&session);
    assert_eq!(quest_currency_gain_for_test(&mut session, 
        394, 1, CurrencyGainSourceLikeCpp::QuestReward,
    ), Ok(None));
    assert_eq!(player_currencies_snapshot_for_test(&session), Some(before.clone()));
    assert_eq!(currency_compatibility_fixture_for_test(&session), fixture_before,
        "a capped no-op must not call the writer, which would refresh this fixture mirror");

    let mut replacement = Box::new(wow_entities::Player::new(Some(2), false));
    replacement.unit_mut().world_mut().object_mut().create(player_guid);
    replacement.install_currencies_like_cpp(before.clone());
    let replacement_handle = canonical.lock().unwrap()
        .install_detached_player_like_cpp(replacement).expect("replacement owner");
    assert_ne!(replacement_handle, old_handle);
    assert_eq!(player_currencies_snapshot_for_test(&session), None);
    assert_eq!(quest_currency_gain_for_test(&mut session, 
        394, 0, CurrencyGainSourceLikeCpp::QuestReward,
    ), Ok(None), "zero is gated before ownership");
    assert_eq!(quest_currency_gain_for_test(&mut session, 
        394, 1, CurrencyGainSourceLikeCpp::QuestRewardIgnoreCaps,
    ), Err(()), "stale handles never gain through the fixture fallback");
    assert_eq!(currency_compatibility_fixture_for_test(&session), fixture_before);
    assert_eq!(canonical.lock().unwrap().with_player_like_cpp(replacement_handle,
        |player| player.gameplay_state().currencies.clone()), Some(before));
    assert!(send_rx.try_recv().is_err());
}

fn currency_entry_like_cpp(id: u32) -> CurrencyTypesEntry {
    CurrencyTypesEntry {
        id,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: wow_constants::CurrencyTypesFlags::empty(),
        flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
    }
}

#[tokio::test]
async fn quest_giver_choose_reward_removes_currency_objective_before_rewards_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7016;
    let currency_id = 394;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.objectives.push(QuestObjective {
        id: 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: currency_id as i32,
        amount: 4,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    set_player_gold_for_test(&mut session, 5);
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        currency_entry_like_cpp(currency_id),
    ])));
    assert!(
        quest_currency_gain_for_test(&mut session, 
            currency_id,
            10,
            CurrencyGainSourceLikeCpp::QuestReward,
        )
        .unwrap()
        .is_some()
    );
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_status_for_test(&mut session,
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(
        !contains_player_quest_status_for_test(&session, quest_id)
    );
    assert!(
        contains_rewarded_quest_for_test(&session, quest_id)
    );
    assert_eq!(player_gold_for_test(&session), 42);
    assert_eq!(player_currency_quantity_for_test(&session, currency_id), Some(6));
    assert_eq!(
        send_rx.try_recv().unwrap(),
        wow_packet::packets::misc::SetCurrency {
            type_id: currency_id as i32,
            quantity: 6,
            flags: 0,
            weekly_quantity: None,
            tracked_quantity: None,
            max_quantity: None,
            total_earned: None,
            suppress_chat_log: false,
            quantity_change: Some(-4),
            quantity_gain_source: None,
            quantity_lost_source: Some(CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP),
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
        .to_bytes()
    );
}
