//! Reputation manager regression scenarios, part 1 of 2.
//!
//! Moved out of the mgr.rs root under #662; every test is unchanged.

use super::*;

#[test]
fn faction_state_defaults_match_cpp_initialize_shape() {
    let flags = ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::AT_WAR;
    let state = FactionStateLikeCpp::new_like_cpp(72, 4, flags);

    assert_eq!(state.id, 72);
    assert_eq!(state.reputation_list_id, 4);
    assert_eq!(state.standing, 0);
    assert_eq!(state.visual_standing_increase, 0);
    assert_eq!(state.flags, flags);
    assert!(state.need_send);
    assert!(state.need_save);
}

#[test]
fn reputation_mgr_initial_state_matches_cpp_constructor_shape() {
    let mgr = ReputationMgrLikeCpp::new_like_cpp();

    assert!(mgr.factions().is_empty());
    assert!(mgr.forced_reactions().is_empty());
    assert_eq!(
        mgr.rank_counters(),
        ReputationRankCountersLikeCpp::default()
    );
    assert!(!mgr.send_faction_increased());
}

#[test]
fn initialize_factions_packet_like_cpp_uses_rep_list_indices_and_clears_need_send() {
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    let mut state = FactionStateLikeCpp::new_like_cpp(
        72,
        4,
        ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::AT_WAR,
    );
    state.standing = 1234;
    state.need_send = true;
    mgr.insert_state_for_test_like_cpp(state);

    let packet = mgr.initialize_factions_packet_like_cpp();

    assert_eq!(
        packet.faction_flags[4],
        (ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::AT_WAR).bits()
    );
    assert_eq!(packet.faction_standings[4], 1234);
    assert!(!packet.faction_has_bonus[4]);
    assert!(!mgr.get_state(4).expect("state remains present").need_send);
}

#[test]
fn set_faction_standing_packet_like_cpp_matches_send_state_order_and_clears_flags() {
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    let mut primary = FactionStateLikeCpp::new_like_cpp(72, 4, ReputationFlagsLikeCpp::VISIBLE);
    primary.standing = 100;
    primary.need_send = true;
    mgr.insert_state_for_test_like_cpp(primary);

    let mut secondary = FactionStateLikeCpp::new_like_cpp(930, 7, ReputationFlagsLikeCpp::VISIBLE);
    secondary.standing = 200;
    secondary.visual_standing_increase = 25;
    secondary.need_send = true;
    mgr.insert_state_for_test_like_cpp(secondary);
    mgr.send_faction_increased = true;

    let packet = mgr.set_faction_standing_packet_like_cpp(Some(4));

    assert_eq!(packet.bonus_from_achievement_system, 0.0);
    assert!(packet.show_visual);
    assert_eq!(
        packet.faction,
        vec![
            FactionStandingDataPacketLikeCpp {
                index: 4,
                standing: 100,
            },
            FactionStandingDataPacketLikeCpp {
                index: 7,
                standing: 25,
            },
        ]
    );
    assert!(!mgr.send_faction_increased());
    assert!(!mgr.get_state(4).expect("primary").need_send);
    assert!(!mgr.get_state(7).expect("secondary").need_send);
}

#[test]
fn set_forced_reactions_packet_like_cpp_matches_cpp_map_order() {
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.apply_force_reaction_like_cpp(930, ReputationRankLikeCpp::Hated, true);
    mgr.apply_force_reaction_like_cpp(72, ReputationRankLikeCpp::Exalted, true);

    let packet = mgr.set_forced_reactions_packet_like_cpp();

    assert_eq!(
        packet.reactions,
        vec![
            ForcedReactionPacketLikeCpp {
                faction: 72,
                reaction: 7,
            },
            ForcedReactionPacketLikeCpp {
                faction: 930,
                reaction: 0,
            },
        ]
    );
}

#[test]
fn reputation_mgr_uses_replist_ordered_faction_state_map_like_cpp() {
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.insert_state_for_test_like_cpp(FactionStateLikeCpp::new_like_cpp(
        10,
        3,
        ReputationFlagsLikeCpp::VISIBLE,
    ));
    mgr.insert_state_for_test_like_cpp(FactionStateLikeCpp::new_like_cpp(
        11,
        1,
        ReputationFlagsLikeCpp::HIDDEN,
    ));

    let keys: Vec<_> = mgr.factions().keys().copied().collect();
    assert_eq!(keys, vec![1, 3]);
    assert_eq!(mgr.get_state(1).map(|state| state.id), Some(11));
    assert_eq!(mgr.get_state(3).map(|state| state.id), Some(10));
}

#[test]
fn apply_force_reaction_insert_and_erase_matches_cpp_map_behavior() {
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

    mgr.apply_force_reaction_like_cpp(72, ReputationRankLikeCpp::Hostile, true);
    assert_eq!(
        mgr.forced_reactions().get(&72),
        Some(&ReputationRankLikeCpp::Hostile)
    );

    mgr.apply_force_reaction_like_cpp(72, ReputationRankLikeCpp::Friendly, true);
    assert_eq!(
        mgr.forced_reactions().get(&72),
        Some(&ReputationRankLikeCpp::Friendly)
    );

    mgr.apply_force_reaction_like_cpp(72, ReputationRankLikeCpp::Friendly, false);
    assert!(!mgr.forced_reactions().contains_key(&72));
}

#[test]
fn initialize_like_cpp_creates_state_for_reputation_factions_only() {
    let mut visible = FactionEntry::for_test_like_cpp(72, 3);
    visible.reputation_race_mask[0] = 1;
    visible.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let hidden = FactionEntry::for_test_like_cpp(73, -1);
    let faction_store = FactionStore::from_entries([visible, hidden]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    assert_eq!(mgr.factions().len(), 1);
    assert!(mgr.get_state(3).is_some());
    assert_eq!(mgr.rank_counters().visible, 1);
    assert!(!mgr.send_faction_increased());
}

#[test]
fn initialize_like_cpp_selects_first_matching_race_class_slot() {
    let mut faction = FactionEntry::for_test_like_cpp(76, 5);
    faction.reputation_race_mask[0] = 1 << 1;
    faction.reputation_class_mask[0] = 1;
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::AT_WAR.bits();
    faction.reputation_race_mask[1] = 1;
    faction.reputation_class_mask[1] = 1;
    faction.reputation_flags[1] =
        (ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::PEACEFUL).bits();
    faction.reputation_base[1] = 9_000;
    let faction_store = FactionStore::from_entries([faction]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    let state = mgr.get_state(5).expect("reputation state");
    assert_eq!(
        state.flags,
        ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::PEACEFUL
    );
    assert_eq!(mgr.rank_counters().visible, 1);
    assert_eq!(mgr.rank_counters().honored, 1);
    assert_eq!(mgr.rank_counters().revered, 0);
    assert_eq!(mgr.rank_counters().exalted, 0);
}

#[test]
fn initialize_like_cpp_uses_class_only_slot_when_race_mask_is_empty() {
    let mut faction = FactionEntry::for_test_like_cpp(77, 6);
    faction.reputation_class_mask[0] = 1 << 1;
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::HIDDEN.bits();
    let faction_store = FactionStore::from_entries([faction]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

    mgr.initialize_like_cpp(&faction_store, None, 1, 2);

    assert_eq!(
        mgr.get_state(6).map(|state| state.flags),
        Some(ReputationFlagsLikeCpp::HIDDEN)
    );
}

#[test]
fn initialize_like_cpp_skips_rank_counters_for_friendship_factions() {
    let mut faction = FactionEntry::for_test_like_cpp(78, 7);
    faction.friendship_rep_id = 4;
    faction.reputation_base[0] = 42_000;
    let faction_store = FactionStore::from_entries([faction]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    assert_eq!(mgr.rank_counters().honored, 0);
    assert_eq!(mgr.rank_counters().revered, 0);
    assert_eq!(mgr.rank_counters().exalted, 0);
}

#[test]
fn initialize_like_cpp_adds_show_propagated_for_paragon_factions() {
    let faction = FactionEntry::for_test_like_cpp(79, 8);
    let faction_store = FactionStore::from_entries([faction]);
    let paragon_store = ParagonReputationStore::from_entries([ParagonReputationEntry {
        id: 1,
        faction_id: 79,
        level_threshold: 10_000,
        quest_id: 100,
    }]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

    mgr.initialize_like_cpp(&faction_store, Some(&paragon_store), 1, 1);

    assert_eq!(
        mgr.get_state(8).map(|state| state.flags),
        Some(ReputationFlagsLikeCpp::SHOW_PROPAGATED)
    );
}

#[test]
fn reputation_to_rank_like_cpp_uses_stock_thresholds_without_friendship_reactions() {
    let faction = FactionEntry::for_test_like_cpp(80, 9);

    assert_eq!(
        reputation_to_rank_like_cpp(&faction, 8_999, None),
        ReputationRankLikeCpp::Friendly
    );
    assert_eq!(
        reputation_to_rank_like_cpp(&faction, 9_000, None),
        ReputationRankLikeCpp::Honored
    );
}

#[test]
fn reputation_to_rank_like_cpp_uses_friendship_thresholds_ordered_by_reaction_threshold() {
    let mut faction = FactionEntry::for_test_like_cpp(81, 10);
    faction.friendship_rep_id = 7;
    let store = FriendshipRepReactionStore::from_entries([
        FriendshipRepReactionEntry {
            id: 3,
            reaction: "three".to_string(),
            friendship_rep_id: 7,
            reaction_threshold: 300,
        },
        FriendshipRepReactionEntry {
            id: 1,
            reaction: "one".to_string(),
            friendship_rep_id: 7,
            reaction_threshold: 100,
        },
        FriendshipRepReactionEntry {
            id: 2,
            reaction: "other".to_string(),
            friendship_rep_id: 8,
            reaction_threshold: 0,
        },
        FriendshipRepReactionEntry {
            id: 4,
            reaction: "two".to_string(),
            friendship_rep_id: 7,
            reaction_threshold: 200,
        },
    ]);

    assert_eq!(
        reputation_to_rank_like_cpp(&faction, 199, Some(&store)),
        ReputationRankLikeCpp::Hated
    );
    assert_eq!(
        reputation_to_rank_like_cpp(&faction, 200, Some(&store)),
        ReputationRankLikeCpp::Hostile
    );
    assert_eq!(
        reputation_to_rank_like_cpp(&faction, 300, Some(&store)),
        ReputationRankLikeCpp::Unfriendly
    );
}

#[test]
fn reputation_to_rank_like_cpp_falls_back_when_friendship_store_is_missing() {
    let mut faction = FactionEntry::for_test_like_cpp(82, 11);
    faction.friendship_rep_id = 9;

    assert_eq!(
        reputation_to_rank_like_cpp(&faction, 21_000, None),
        ReputationRankLikeCpp::Revered
    );
}

#[test]
fn load_from_db_like_cpp_merges_standing_flags_and_clears_clean_rows() {
    let mut faction = FactionEntry::for_test_like_cpp(83, 12);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    faction.reputation_base[0] = 3_000;
    let faction_store = FactionStore::from_entries([faction]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

    mgr.load_from_db_like_cpp(
        [CharacterReputationRowLikeCpp {
            faction_id: 83,
            standing: 6_000,
            flags: ReputationFlagsLikeCpp::VISIBLE.bits(),
        }],
        &faction_store,
        None,
        None,
        1,
        1,
    );

    let state = mgr.get_state(12).expect("loaded state");
    assert_eq!(state.standing, 6_000);
    assert_eq!(state.flags, ReputationFlagsLikeCpp::VISIBLE);
    assert!(!state.need_send);
    assert!(!state.need_save);
    assert_eq!(mgr.rank_counters().honored, 1);
    assert_eq!(mgr.rank_counters().revered, 0);
}

#[test]
fn load_from_db_like_cpp_applies_hostile_at_war_even_when_db_flag_is_clear() {
    let mut faction = FactionEntry::for_test_like_cpp(84, 13);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let faction_store = FactionStore::from_entries([faction]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

    mgr.load_from_db_like_cpp(
        [CharacterReputationRowLikeCpp {
            faction_id: 84,
            standing: -6_000,
            flags: ReputationFlagsLikeCpp::VISIBLE.bits(),
        }],
        &faction_store,
        None,
        None,
        1,
        1,
    );

    let state = mgr.get_state(13).expect("loaded state");
    assert!(state.flags.contains(ReputationFlagsLikeCpp::AT_WAR));
    assert!(state.need_send);
    assert!(state.need_save);
}

#[test]
fn pending_save_rows_like_cpp_preserve_dirty_faction_projection() {
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.insert_state_for_test_like_cpp(FactionStateLikeCpp {
        standing: 123,
        flags: ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::AT_WAR,
        ..FactionStateLikeCpp::new_like_cpp(85, 14, ReputationFlagsLikeCpp::VISIBLE)
    });

    assert_eq!(
        mgr.pending_save_rows_like_cpp(),
        vec![(
            85,
            123,
            (ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::AT_WAR).bits(),
        )]
    );
    assert!(mgr.get_state(14).expect("pending state").need_save);

    mgr.mark_pending_save_to_db_committed_like_cpp();

    assert_eq!(
        mgr.pending_save_rows_like_cpp(),
        Vec::<(u16, i32, u16)>::new()
    );
    assert!(!mgr.get_state(14).expect("saved state").need_save);
}

#[test]
fn pending_save_rows_like_cpp_do_not_clear_dirty_until_commit() {
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.insert_state_for_test_like_cpp(FactionStateLikeCpp {
        standing: 456,
        flags: ReputationFlagsLikeCpp::VISIBLE,
        ..FactionStateLikeCpp::new_like_cpp(86, 15, ReputationFlagsLikeCpp::VISIBLE)
    });

    let rows = mgr.pending_save_rows_like_cpp();

    assert_eq!(
        rows,
        vec![(86, 456, ReputationFlagsLikeCpp::VISIBLE.bits())]
    );
    assert!(
        mgr.get_state(15).expect("pending state").need_save,
        "failed Player::SaveToDB transaction must leave reputation dirty for retry"
    );

    mgr.mark_pending_save_to_db_committed_like_cpp();

    assert!(!mgr.get_state(15).expect("committed state").need_save);
}

#[test]
fn set_one_faction_reputation_like_cpp_applies_incremental_rate_and_marks_visible_dirty() {
    let mut faction = FactionEntry::for_test_like_cpp(86, 15);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([faction.clone()]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    let outcome = mgr.set_one_faction_reputation_like_cpp(
        &faction, 4_000, true, 1.5, None, None, true, None, 0, 0, 1, 1,
    );

    assert!(outcome.applied);
    assert_eq!(outcome.reputation_change, 6_000);
    assert_eq!(outcome.old_rank, Some(ReputationRankLikeCpp::Neutral));
    assert_eq!(outcome.new_rank, Some(ReputationRankLikeCpp::Friendly));
    assert!(outcome.became_visible);
    assert!(mgr.send_faction_increased());
    let state = mgr.get_state(15).expect("mutated state");
    assert_eq!(state.standing, 6_000);
    assert!(state.flags.contains(ReputationFlagsLikeCpp::VISIBLE));
    assert!(state.need_send);
    assert!(state.need_save);
}

#[test]
fn set_reputation_like_cpp_applies_reputation_gain_rate_to_primary_incremental_gain() {
    let mut faction = FactionEntry::for_test_like_cpp(860, 150);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([faction.clone()]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);
    let mut options = set_reputation_options_for_test_like_cpp(true);
    options.reputation_gain_rate = 2.0;

    let outcome = mgr.set_reputation_like_cpp(
        &faction,
        500,
        options,
        &faction_store,
        None,
        None,
        None,
        None,
    );

    assert!(outcome.applied);
    assert_eq!(outcome.send_state_rep_list_id, Some(150));
    let (_, mutation) = outcome.primary_mutation.expect("primary mutation");
    assert_eq!(mutation.reputation_change, 1_000);
    assert_eq!(mgr.get_state(150).expect("mutated state").standing, 1_000);
}

#[test]
fn set_one_faction_reputation_like_cpp_clamps_to_slot_max() {
    let mut faction = FactionEntry::for_test_like_cpp(87, 16);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_max[0] = 9_000;
    let faction_store = FactionStore::from_entries([faction.clone()]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    let outcome = mgr.set_one_faction_reputation_like_cpp(
        &faction, 42_000, false, 1.0, None, None, true, None, 0, 0, 1, 1,
    );

    assert!(outcome.applied);
    assert_eq!(outcome.reputation_change, 9_000);
    assert_eq!(mgr.get_state(16).expect("mutated state").standing, 9_000);
}

#[test]
fn set_one_faction_reputation_like_cpp_forces_at_war_for_hostile_rank() {
    let mut faction = FactionEntry::for_test_like_cpp(88, 17);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([faction.clone()]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    let outcome = mgr.set_one_faction_reputation_like_cpp(
        &faction, -6_000, false, 1.0, None, None, true, None, 0, 0, 1, 1,
    );

    assert!(outcome.applied);
    assert_eq!(outcome.new_rank, Some(ReputationRankLikeCpp::Hostile));
    assert!(outcome.set_at_war_for_hostile);
    assert!(
        mgr.get_state(17)
            .expect("mutated state")
            .flags
            .contains(ReputationFlagsLikeCpp::AT_WAR)
    );
}

#[test]
fn set_one_faction_reputation_like_cpp_updates_honored_revered_exalted_counters() {
    let mut faction = FactionEntry::for_test_like_cpp(89, 18);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([faction.clone()]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    mgr.set_one_faction_reputation_like_cpp(
        &faction, 21_000, false, 1.0, None, None, true, None, 0, 0, 1, 1,
    );

    assert_eq!(mgr.rank_counters().honored, 1);
    assert_eq!(mgr.rank_counters().revered, 1);
    assert_eq!(mgr.rank_counters().exalted, 0);
}

#[test]
fn criteria_progress_like_cpp_exposes_reputation_counters_and_positive_reputation() {
    let mut faction = FactionEntry::for_test_like_cpp(91, 21);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_base[0] = 500;
    faction.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([faction.clone()]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    mgr.set_one_faction_reputation_like_cpp(
        &faction, 21_000, false, 1.0, None, None, true, None, 0, 0, 1, 1,
    );

    assert_eq!(
        mgr.criteria_progress_like_cpp(
            ReputationCriteriaProgressKindLikeCpp::ReputationGained { faction_id: 91 },
            Some(&faction_store),
            1,
            1,
        ),
        Some(21_000)
    );
    assert_eq!(
        mgr.criteria_progress_like_cpp(
            ReputationCriteriaProgressKindLikeCpp::TotalFactionsEncountered,
            Some(&faction_store),
            1,
            1,
        ),
        Some(1)
    );
    assert_eq!(
        mgr.criteria_progress_like_cpp(
            ReputationCriteriaProgressKindLikeCpp::TotalHonoredFactions,
            Some(&faction_store),
            1,
            1,
        ),
        Some(1)
    );
    assert_eq!(
        mgr.criteria_progress_like_cpp(
            ReputationCriteriaProgressKindLikeCpp::TotalReveredFactions,
            Some(&faction_store),
            1,
            1,
        ),
        Some(1)
    );
    assert_eq!(
        mgr.criteria_progress_like_cpp(
            ReputationCriteriaProgressKindLikeCpp::TotalExaltedFactions,
            Some(&faction_store),
            1,
            1,
        ),
        Some(0)
    );
}

#[test]
fn criteria_progress_like_cpp_skips_non_positive_reputation_gained_like_cpp() {
    let mut faction = FactionEntry::for_test_like_cpp(92, 22);
    faction.reputation_race_mask[0] = 1;
    let faction_store = FactionStore::from_entries([faction]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    assert_eq!(
        mgr.criteria_progress_like_cpp(
            ReputationCriteriaProgressKindLikeCpp::ReputationGained { faction_id: 92 },
            Some(&faction_store),
            1,
            1,
        ),
        None
    );
}

#[test]
fn set_one_faction_reputation_like_cpp_uses_paragon_cap_with_no_unclaimed_reward() {
    let mut faction = FactionEntry::for_test_like_cpp(90, 19);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([faction.clone()]);
    let paragon_store = ParagonReputationStore::from_entries([ParagonReputationEntry {
        id: 2,
        faction_id: 90,
        level_threshold: 10_000,
        quest_id: 700,
    }]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, Some(&paragon_store), 1, 1);

    let outcome = mgr.set_one_faction_reputation_like_cpp(
        &faction,
        99_999,
        false,
        1.0,
        None,
        Some(&paragon_store),
        true,
        None,
        0,
        0,
        1,
        1,
    );

    assert!(outcome.applied);
    assert_eq!(outcome.reputation_change, 19_999);
    assert_eq!(
        outcome.paragon_reward_quest_id_to_add_if_template_exists_like_cpp,
        Some(700)
    );
    assert_eq!(mgr.get_state(19).expect("mutated state").standing, 19_999);
    assert!(mgr.send_faction_increased());
}

#[test]
fn set_one_faction_reputation_like_cpp_uses_paragon_cap_when_reward_is_unclaimed() {
    let mut faction = FactionEntry::for_test_like_cpp(91, 20);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([faction.clone()]);
    let paragon_store = ParagonReputationStore::from_entries([ParagonReputationEntry {
        id: 3,
        faction_id: 91,
        level_threshold: 10_000,
        quest_id: 701,
    }]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, Some(&paragon_store), 1, 1);

    let outcome = mgr.set_one_faction_reputation_like_cpp(
        &faction,
        99_999,
        false,
        1.0,
        None,
        Some(&paragon_store),
        false,
        None,
        0,
        0,
        1,
        1,
    );

    assert!(outcome.applied);
    assert_eq!(outcome.reputation_change, 9_999);
    assert_eq!(
        outcome.paragon_reward_quest_id_to_add_if_template_exists_like_cpp,
        None
    );
    assert_eq!(mgr.get_state(20).expect("mutated state").standing, 9_999);
    assert!(mgr.send_faction_increased());
}

#[test]
fn set_one_faction_reputation_like_cpp_applies_renown_level_remainder_and_currency_delta() {
    let mut faction = FactionEntry::for_test_like_cpp(92, 21);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_max[0] = 2_500;
    faction.renown_currency_id = 77;
    let faction_store = FactionStore::from_entries([faction.clone()]);
    let currency_store =
        CurrencyTypesStore::from_entries([currency_entry_for_test_like_cpp(77, 5)]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    let outcome = mgr.set_one_faction_reputation_like_cpp(
        &faction,
        3_000,
        false,
        1.0,
        None,
        None,
        true,
        Some(&currency_store),
        1,
        0,
        1,
        1,
    );

    assert!(outcome.applied);
    assert_eq!(outcome.reputation_change, 3_000);
    assert_eq!(outcome.renown_currency_delta_like_cpp, Some((77, 1)));
    let state = mgr.get_state(21).expect("mutated state");
    assert_eq!(state.standing, 500);
    assert_eq!(state.visual_standing_increase, 3_000);
    assert!(mgr.send_faction_increased());
}

#[test]
fn set_one_faction_reputation_like_cpp_caps_renown_at_max_level_and_clears_remainder() {
    let mut faction = FactionEntry::for_test_like_cpp(93, 22);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_max[0] = 2_500;
    faction.renown_currency_id = 78;
    let faction_store = FactionStore::from_entries([faction.clone()]);
    let currency_store =
        CurrencyTypesStore::from_entries([currency_entry_for_test_like_cpp(78, 3)]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);

    let outcome = mgr.set_one_faction_reputation_like_cpp(
        &faction,
        9_000,
        false,
        1.0,
        None,
        None,
        true,
        Some(&currency_store),
        2,
        0,
        1,
        1,
    );

    assert!(outcome.applied);
    assert_eq!(outcome.reputation_change, 2_500);
    assert_eq!(outcome.renown_currency_delta_like_cpp, Some((78, 3)));
    let state = mgr.get_state(22).expect("mutated state");
    assert_eq!(state.standing, 0);
    assert_eq!(state.visual_standing_increase, 2_500);
}

#[test]
fn set_one_faction_reputation_like_cpp_ignores_positive_renown_when_maxed() {
    let mut faction = FactionEntry::for_test_like_cpp(94, 23);
    faction.reputation_race_mask[0] = 1;
    faction.reputation_max[0] = 2_500;
    faction.renown_currency_id = 79;
    let faction_store = FactionStore::from_entries([faction.clone()]);
    let currency_store =
        CurrencyTypesStore::from_entries([currency_entry_for_test_like_cpp(79, 3)]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);
    mgr.get_state_mut(23).expect("state").need_send = true;
    mgr.get_state_mut(23).expect("state").need_save = true;

    let outcome = mgr.set_one_faction_reputation_like_cpp(
        &faction,
        1,
        false,
        1.0,
        None,
        None,
        true,
        Some(&currency_store),
        3,
        0,
        1,
        1,
    );

    assert!(!outcome.applied);
    let state = mgr.get_state(23).expect("state");
    assert!(!state.need_send);
    assert!(!state.need_save);
}

#[test]
fn set_reputation_like_cpp_applies_db_template_spillover_then_primary_and_send_state() {
    let mut source = FactionEntry::for_test_like_cpp(100, 24);
    source.reputation_race_mask[0] = 1;
    source.reputation_max[0] = 42_000;
    let mut spill = FactionEntry::for_test_like_cpp(101, 25);
    spill.reputation_race_mask[0] = 1;
    spill.reputation_max[0] = 42_000;
    let faction_store = FactionStore::from_entries([source.clone(), spill.clone()]);
    let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
    mgr.initialize_like_cpp(&faction_store, None, 1, 1);
    let mut template = RepSpilloverTemplateLikeCpp::empty_like_cpp();
    template.faction[0] = 101;
    template.faction_rate[0] = 0.5;
    template.faction_rank[0] = ReputationRankLikeCpp::Exalted.as_u8();

    let outcome = mgr.set_reputation_like_cpp(
        &source,
        1_000,
        set_reputation_options_for_test_like_cpp(false),
        &faction_store,
        Some(&template),
        None,
        None,
        None,
    );

    assert!(outcome.applied);
    assert_eq!(
        outcome.script_reputation_change_event,
        Some((100, 1_000, false))
    );
    assert_eq!(outcome.spillover_mutations.len(), 1);
    assert_eq!(outcome.spillover_mutations[0].0, 101);
    assert_eq!(outcome.spillover_mutations[0].1.reputation_change, 500);
    assert_eq!(
        outcome.primary_mutation.as_ref().map(|entry| entry.0),
        Some(100)
    );
    assert_eq!(outcome.send_state_rep_list_id, Some(24));
    assert_eq!(mgr.get_state(24).expect("source state").standing, 1_000);
    assert_eq!(mgr.get_state(25).expect("spill state").standing, 500);
}
