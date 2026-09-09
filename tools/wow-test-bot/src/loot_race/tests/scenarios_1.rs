//! Loot-race scenarios, part 1.
//!
//! Split out of the inline test module under #634; assertions unchanged.

use super::*;

#[test]
fn loot_fixture_health_guard_produces_one_health_like_cpp() {
    assert_eq!(
        generated_fixture_health_like_cpp(115, GUARDED_FIXTURE_HEALTH_MODIFIER),
        1
    );
    assert!(validate_guarded_fixture_health(GUARDED_FIXTURE_HEALTH_MODIFIER, 1).is_ok());
}
#[tokio::test]
async fn cancellation_token_cannot_lose_a_preexisting_cancel() {
    let sync = LootRaceSync::new();
    sync.cancel("deterministic peer failure");
    tokio::time::timeout(Duration::from_millis(50), sync.cancelled())
        .await
        .expect("CancellationToken retains cancellation before waiter registration");
    assert!(sync
        .cancellation_error()
        .expect_err("cancelled sync must fail")
        .to_string()
        .contains("deterministic peer failure"));
}
#[test]
fn respawn_cleanup_scope_is_exact_and_fail_closed() {
    assert_eq!(validate_respawn_cleanup_scope(0, &[], &[]).unwrap(), None);
    assert_eq!(
        validate_respawn_cleanup_scope(0, &[], &[(1_700_000_000, 0, 0)]).unwrap(),
        Some((1_700_000_000, 0, 0))
    );
    assert!(validate_respawn_cleanup_scope(
        0,
        &[],
        &[(1_700_000_000, 0, 0), (1_700_000_001, 0, 1)]
    )
    .is_err());
    assert!(validate_respawn_cleanup_scope(0, &[], &[(1_700_000_000, 1, 0)]).is_err());
    assert!(validate_respawn_cleanup_scope(0, &[], &[(1_700_000_000, 0, 1)]).is_err());
    assert!(validate_respawn_cleanup_scope(0, &[], &[(0, 0, 0)]).is_err());
    assert!(
        validate_respawn_cleanup_scope(0, &[(1_600_000_000, 0, 0)], &[(1_700_000_000, 0, 0)])
            .is_err()
    );
}
#[test]
fn cleanup_marker_publish_is_0600_and_both_present_recovery_is_idempotent() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "wow-test-bot-journal-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let path = directory.join("fixture.journal");
    let journal = FixtureJournal { path: path.clone() };
    let payload = b"durable fixture snapshot\n";
    write_test_journal(&path, payload);

    journal.complete().unwrap();
    let marker = cleanup_marker_path(&path);
    assert!(!path.exists());
    assert_eq!(
        fs::metadata(&marker).unwrap().permissions().mode() & 0o777,
        0o600
    );
    validate_cleanup_marker(&marker, None).unwrap();

    // Model SIGKILL after atomic marker rename but before journal unlink.
    // Recovery re-verifies DB state, then accepts only the same digest.
    write_test_journal(&path, payload);
    journal.complete().unwrap();
    assert!(!path.exists());

    // A different pending snapshot must not be hidden by a stale marker.
    write_test_journal(&path, b"different snapshot\n");
    assert!(journal.complete().is_err());
    assert!(path.exists());

    fs::remove_file(&path).unwrap();
    fs::remove_file(&marker).unwrap();
    fs::remove_dir(&directory).unwrap();
}
#[test]
fn loot_fixture_health_guard_rejects_original_or_stale_runtime_data() {
    let original_health = generated_fixture_health_like_cpp(115, 1.5);
    assert_eq!(original_health, 173);
    assert!(validate_guarded_fixture_health(1.5, original_health).is_err());
    assert!(
        validate_guarded_fixture_health(GUARDED_FIXTURE_HEALTH_MODIFIER, 2).is_err(),
        "a non-default classification multiplier or stale runtime must fail closed"
    );
}
#[test]
fn single_item_fixture_requires_exact_keyring_destination_empty() {
    let required = Some((15, LOOT_ITEM_CAPTURE_KEYRING_SLOT));
    validate_required_empty_top_level_slot(15, &[35, 50], required).unwrap();
    assert!(validate_required_empty_top_level_slot(
        15,
        &[LOOT_ITEM_CAPTURE_KEYRING_SLOT],
        required
    )
    .is_err());
    validate_required_empty_top_level_slot(16, &[LOOT_ITEM_CAPTURE_KEYRING_SLOT], required)
        .unwrap();
}
#[test]
fn sql_spawn_uniqueness_is_required_for_entry_map_runtime_auto_discovery() {
    validate_unique_sql_spawn(&[1_117], 1_117, 21_779, 530).unwrap();

    let missing = validate_unique_sql_spawn(&[], 1_117, 21_779, 530)
        .expect_err("a missing SQL spawn must fail closed");
    assert!(missing.to_string().contains("exactly one"));

    let ambiguous = validate_unique_sql_spawn(&[1_117, 2_268], 1_117, 21_779, 530)
        .expect_err("same-entry map ambiguity must fail closed");
    assert!(ambiguous.to_string().contains("2 SQL spawns"));

    let mismatch = validate_unique_sql_spawn(&[1_117], 2_268, 21_779, 530)
        .expect_err("the unique row must equal the configured spawn");
    assert!(mismatch.to_string().contains("not configured spawn"));
}
#[test]
fn runtime_auto_discovery_preserves_cpp_full_guid_including_realm() {
    const CPP_DOCTOR_COUNTER: u64 = 268;
    const CPP_DOCTOR_HIGH: u64 = 0x2000_0442_4015_44C0;
    let options = runtime_discovery_options(0);
    let payload = update_object_with_guid(CPP_DOCTOR_COUNTER, CPP_DOCTOR_HIGH);

    assert_eq!(
        target_seen_in_update(&options, SMSG_UPDATE_OBJECT, &payload).unwrap(),
        Some(CPP_DOCTOR_COUNTER)
    );
    assert_eq!(
        options.resolved_runtime_guid().unwrap(),
        (CPP_DOCTOR_COUNTER, CPP_DOCTOR_HIGH)
    );
    assert_eq!(
        options.resolved_packed_guid().unwrap(),
        build_packed_guid(CPP_DOCTOR_COUNTER, CPP_DOCTOR_HIGH)
    );
    assert_eq!((CPP_DOCTOR_HIGH >> 42) & GUID_REALM_MASK, 1);

    let reconstructed_without_realm = create_creature_guid_raw(530, 21_779, 268);
    assert_ne!(
        reconstructed_without_realm,
        (CPP_DOCTOR_COUNTER, CPP_DOCTOR_HIGH),
        "the SQL spawn/counter helper must not replace the full discovered C++ GUID"
    );
}
#[test]
fn runtime_override_and_two_bot_convergence_fail_closed() {
    const CPP_DOCTOR_HIGH: u64 = 0x2000_0442_4015_44C0;
    let strict = runtime_discovery_options(1_117);
    let mismatch = target_seen_in_update(
        &strict,
        SMSG_UPDATE_OBJECT,
        &update_object_with_guid(268, CPP_DOCTOR_HIGH),
    )
    .expect_err("a stale SQL-spawn-as-counter override must be rejected");
    assert!(mismatch
        .to_string()
        .contains("did not match discovered counter 268"));

    let first = runtime_discovery_options(0);
    let mut second = first.clone();
    second.participant = 1;
    assert_eq!(
        target_seen_in_update(
            &first,
            SMSG_UPDATE_OBJECT,
            &update_object_with_guid(268, CPP_DOCTOR_HIGH),
        )
        .unwrap(),
        Some(268)
    );
    assert_eq!(
        target_seen_in_update(
            &second,
            SMSG_UPDATE_OBJECT,
            &update_object_with_guid(268, CPP_DOCTOR_HIGH),
        )
        .unwrap(),
        Some(268)
    );

    let different = target_seen_in_update(
        &second,
        SMSG_UPDATE_OBJECT,
        &update_object_with_guid(269, CPP_DOCTOR_HIGH),
    )
    .expect_err("two bots must not bind different runtime counters");
    assert!(different.to_string().contains("different live ObjectGuids"));
}
#[test]
fn gameobject_discovery_keeps_sql_spawn_and_runtime_counter_distinct_like_cpp() {
    const LIVE_COUNTER: u64 = 40;
    let options = gameobject_discovery_options(0);
    let high = gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID, DEFAULT_CREATURE_ENTRY);
    let exact = update_object_with_guid(LIVE_COUNTER, high);
    assert_eq!(
        target_seen_in_update(&options, SMSG_UPDATE_OBJECT, &exact).unwrap(),
        Some(LIVE_COUNTER)
    );
    assert_eq!(
        options.resolved_runtime_guid().unwrap(),
        (LIVE_COUNTER, high)
    );
    assert_ne!(LIVE_COUNTER, DEFAULT_CREATURE_SPAWN_GUID);
}
#[test]
fn gameobject_discovery_requires_exact_high_type_entry_and_map() {
    const LIVE_COUNTER: u64 = 40;
    let high = gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID, DEFAULT_CREATURE_ENTRY);
    let creature_high = (HIGH_GUID_CREATURE << 58)
        | (u64::from(RACE_GAMEOBJECT_MAP_ID) << 29)
        | (u64::from(DEFAULT_CREATURE_ENTRY) << 6);
    let wrong_entry_high =
        gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID, DEFAULT_CREATURE_ENTRY + 1);
    let wrong_map_high =
        gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID + 1, DEFAULT_CREATURE_ENTRY);

    for wrong_high in [creature_high, wrong_entry_high, wrong_map_high] {
        let options = gameobject_discovery_options(0);
        assert_eq!(
            target_seen_in_update(
                &options,
                SMSG_UPDATE_OBJECT,
                &update_object_with_guid(LIVE_COUNTER, wrong_high),
            )
            .unwrap(),
            None
        );
    }

    let wrong_opcode = gameobject_discovery_options(0);
    assert_eq!(
        target_seen_in_update(
            &wrong_opcode,
            SMSG_LOOT_RESPONSE,
            &update_object_with_guid(LIVE_COUNTER, high),
        )
        .unwrap(),
        None
    );
}
#[test]
fn gameobject_runtime_override_checks_the_live_counter_not_the_sql_spawn() {
    const LIVE_COUNTER: u64 = 40;
    let high = gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID, DEFAULT_CREATURE_ENTRY);
    let exact = gameobject_discovery_options(LIVE_COUNTER);
    assert_eq!(
        target_seen_in_update(
            &exact,
            SMSG_UPDATE_OBJECT,
            &update_object_with_guid(LIVE_COUNTER, high),
        )
        .unwrap(),
        Some(LIVE_COUNTER)
    );

    let stale = gameobject_discovery_options(LIVE_COUNTER + 1);
    let error = target_seen_in_update(
        &stale,
        SMSG_UPDATE_OBJECT,
        &update_object_with_guid(LIVE_COUNTER, high),
    )
    .expect_err("a mismatching live GameObject counter override must fail closed");
    assert!(error
        .to_string()
        .contains("did not match discovered counter 40"));
}
#[test]
fn gameobject_discovery_deduplicates_one_guid_and_rejects_packet_ambiguity() {
    const LIVE_COUNTER: u64 = 40;
    let high = gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID, DEFAULT_CREATURE_ENTRY);

    let mut duplicate = update_object_with_guid(LIVE_COUNTER, high);
    duplicate.extend_from_slice(&update_object_with_guid(LIVE_COUNTER, high));
    let options = gameobject_discovery_options(0);
    assert_eq!(
        target_seen_in_update(&options, SMSG_UPDATE_OBJECT, &duplicate).unwrap(),
        Some(LIVE_COUNTER)
    );

    let mut ambiguous = update_object_with_guid(LIVE_COUNTER, high);
    ambiguous.extend_from_slice(&update_object_with_guid(LIVE_COUNTER + 1, high));
    let options = gameobject_discovery_options(0);
    let error = target_seen_in_update(&options, SMSG_UPDATE_OBJECT, &ambiguous)
        .expect_err("two distinct matching GameObjects in one update must fail closed");
    assert!(error.to_string().contains("2 distinct live ObjectGuid"));
}
#[test]
fn gameobject_discovery_requires_two_bots_to_converge_on_one_full_guid() {
    const LIVE_COUNTER: u64 = 40;
    let high = gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID, DEFAULT_CREATURE_ENTRY);
    let first = gameobject_discovery_options(0);
    let mut second = first.clone();
    second.participant = 1;

    assert_eq!(
        target_seen_in_update(
            &first,
            SMSG_UPDATE_OBJECT,
            &update_object_with_guid(LIVE_COUNTER, high),
        )
        .unwrap(),
        Some(LIVE_COUNTER)
    );
    assert_eq!(
        target_seen_in_update(
            &second,
            SMSG_UPDATE_OBJECT,
            &update_object_with_guid(LIVE_COUNTER, high),
        )
        .unwrap(),
        Some(LIVE_COUNTER)
    );

    let error = target_seen_in_update(
        &second,
        SMSG_UPDATE_OBJECT,
        &update_object_with_guid(LIVE_COUNTER + 1, high),
    )
    .expect_err("two bots must not bind different live GameObject GUIDs");
    assert!(error.to_string().contains("different live ObjectGuids"));
}
#[tokio::test]
async fn peer_failure_cancels_a_phase_barrier_without_waiting_for_timeout() {
    let options = gameobject_discovery_options(0);
    let sync = options.sync.clone();
    let waiter = tokio::spawn(async move {
        wait_phase(&options, &options.sync.logged_in, "test peer barrier").await
    });
    tokio::task::yield_now().await;
    sync.cancel("peer failed before reaching the barrier");

    let error = tokio::time::timeout(Duration::from_millis(250), waiter)
        .await
        .expect("cancellation must wake the peer promptly")
        .expect("barrier waiter task must not panic")
        .expect_err("the peer barrier must fail after cancellation");
    assert!(error
        .to_string()
        .contains("peer failed before reaching the barrier"));
}
#[test]
fn logout_complete_requires_cpp_empty_body_on_both_routes() {
    assert_eq!(
        logout_completion_route(SMSG_LOGOUT_COMPLETE, &[], LogoutCompletionRoute::Realm).unwrap(),
        Some(LogoutCompletionRoute::Realm)
    );
    assert_eq!(
        logout_completion_route(SMSG_LOGOUT_COMPLETE, &[], LogoutCompletionRoute::Instance)
            .unwrap(),
        Some(LogoutCompletionRoute::Instance)
    );
    assert_eq!(
        logout_completion_route(SMSG_LOOT_RESPONSE, &[1], LogoutCompletionRoute::Realm).unwrap(),
        None
    );
    assert!(
        logout_completion_route(SMSG_LOGOUT_COMPLETE, &[0], LogoutCompletionRoute::Realm).is_err()
    );
}
#[test]
fn cpp_realm_only_party_opcodes_fail_fast_on_instance() {
    for opcode in [
        SMSG_PARTY_INVITE,
        SMSG_PARTY_UPDATE,
        SMSG_PARTY_COMMAND_RESULT,
        SMSG_PARTY_MEMBER_FULL_STATE,
    ] {
        let error =
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Instance).unwrap_err();
        assert!(error.to_string().contains("CONNECTION_TYPE_REALM"));
    }
}
#[test]
fn party_route_guard_accepts_cpp_realm_and_unrelated_instance_packets() {
    validate_party_packet_route_like_cpp(SMSG_PARTY_UPDATE, PartyPacketRoute::Realm).unwrap();
    validate_party_packet_route_like_cpp(SMSG_TIME_SYNC_REQUEST, PartyPacketRoute::Instance)
        .unwrap();
}
#[test]
fn party_update_proves_exact_normal_home_two_player_roster() {
    let leader = create_player_guid_raw(15, ITEM_TEST_REALM);
    let peer = create_player_guid_raw(16, ITEM_TEST_REALM);
    let payload = party_update_for_test(
        0,
        0,
        1,
        0,
        (77, 88),
        leader,
        [leader, peer],
        PERSONAL_LOOT_METHOD_LIKE_CPP,
    );

    validate_party_update_like_cpp(&payload, leader, peer, leader).unwrap();
}
#[test]
fn group_capacity_winner_requires_exact_five_member_roster() {
    let options = group_capacity_options_for_test(15);
    let roster: Vec<_> = [13, 14, 17, 18, 15]
        .into_iter()
        .map(|guid| create_player_guid_raw(guid, realm_id()))
        .collect();
    let payload =
        group_capacity_party_update_for_test(4, create_player_guid_raw(14, realm_id()), &roster);
    assert_eq!(
        validate_group_capacity_party_update(&payload, &options).unwrap(),
        GroupCapacityPartyUpdateEvidence::CompleteRoster
    );

    let six_member_roster: Vec<_> = [13, 14, 17, 18, 15, 16]
        .into_iter()
        .map(|guid| create_player_guid_raw(guid, realm_id()))
        .collect();
    let six_member_payload = group_capacity_party_update_for_test(
        4,
        create_player_guid_raw(14, realm_id()),
        &six_member_roster,
    );
    assert!(
        validate_group_capacity_party_update(&six_member_payload, &options)
            .unwrap_err()
            .to_string()
            .contains("members=6")
    );
}
#[test]
fn group_capacity_runtime_boundary_accepts_only_exact_connected_pair() {
    let options = group_capacity_options_for_test(15);
    let leader = create_player_guid_raw(14, realm_id());
    let candidate = create_player_guid_raw(15, realm_id());
    let payload = group_capacity_party_update_for_test(4, leader, &[leader, candidate]);
    assert_eq!(
        validate_group_capacity_party_update(&payload, &options).unwrap(),
        GroupCapacityPartyUpdateEvidence::ConnectedOnlyRoster
    );

    let wrong_peer = create_player_guid_raw(16, realm_id());
    let wrong = group_capacity_party_update_for_test(4, leader, &[leader, wrong_peer]);
    assert!(validate_group_capacity_party_update(&wrong, &options).is_err());
}
#[test]
fn group_capacity_fixture_rejects_online_initial_member() {
    validate_group_capacity_initial_member_offline(17, 0).unwrap();
    let error = validate_group_capacity_initial_member_offline(17, 1)
        .expect_err("an online filler would change the connected-only PartyUpdate roster");
    assert!(error.to_string().contains("initial member 17"));
    assert!(error.to_string().contains("offline"));
}
#[test]
fn group_capacity_persistence_identifies_and_matches_wire_winner() {
    let fixture = group_capacity_fixture_for_test();
    let evidence =
        validate_group_capacity_persisted_members(&fixture, &[13, 14, 15, 17, 18]).unwrap();
    assert_eq!(
        evidence,
        GroupCapacityPersistenceEvidence {
            final_member_count: 5,
            winning_candidate_guid: 15,
        }
    );
    validate_group_capacity_winner_consistency(15, evidence.winning_candidate_guid).unwrap();
    let error = validate_group_capacity_winner_consistency(16, evidence.winning_candidate_guid)
        .expect_err("wire and CharacterDB winners must be the same candidate");
    assert!(error.to_string().contains("winner mismatch"));
    assert!(error.to_string().contains("wire added GUID 16"));
    assert!(error.to_string().contains("persisted GUID 15"));
}
#[test]
fn group_capacity_party_update_requires_cpp_optional_tail_exactly() {
    let options = group_capacity_options_for_test(15);
    let leader = create_player_guid_raw(14, realm_id());
    let candidate = create_player_guid_raw(15, realm_id());

    let no_option_bits =
        group_capacity_party_update_with_optional_bits_for_test(4, leader, &[leader, candidate], 0);
    let error = validate_group_capacity_party_update(&no_option_bits, &options)
        .expect_err("normal party update must advertise loot and difficulty settings");
    assert!(error.to_string().contains("loot=false"));
    assert!(error.to_string().contains("difficulty=false"));

    let valid = group_capacity_party_update_for_test(4, leader, &[leader, candidate]);
    let empty_guid_len = build_packed_guid(0, 0).len();
    let optional_tail_len = 1 + empty_guid_len + 1 + 12;
    let tail_start = valid.len() - optional_tail_len;

    let mut wrong_method = valid.clone();
    wrong_method[tail_start] = 3;
    assert!(
        validate_group_capacity_party_update(&wrong_method, &options)
            .unwrap_err()
            .to_string()
            .contains("settings differed")
    );

    let mut wrong_threshold = valid.clone();
    wrong_threshold[tail_start + 1 + empty_guid_len] = 4;
    assert!(
        validate_group_capacity_party_update(&wrong_threshold, &options)
            .unwrap_err()
            .to_string()
            .contains("settings differed")
    );

    let difficulty_start = tail_start + 1 + empty_guid_len + 1;
    let mut swapped_difficulties = valid.clone();
    swapped_difficulties[difficulty_start..difficulty_start + 4]
        .copy_from_slice(&14u32.to_le_bytes());
    swapped_difficulties[difficulty_start + 4..difficulty_start + 8]
        .copy_from_slice(&1u32.to_le_bytes());
    assert!(
        validate_group_capacity_party_update(&swapped_difficulties, &options)
            .unwrap_err()
            .to_string()
            .contains("settings differed")
    );

    let mut missing_tail = valid.clone();
    missing_tail.truncate(tail_start);
    assert!(
        validate_group_capacity_party_update(&missing_tail, &options)
            .unwrap_err()
            .to_string()
            .contains("PartyLootSettings.Method")
    );

    let mut trailing = valid;
    trailing.push(0xFF);
    assert!(validate_group_capacity_party_update(&trailing, &options)
        .unwrap_err()
        .to_string()
        .contains("trailing byte"));
}
#[test]
fn group_capacity_full_result_requires_invite_group_full() {
    let mut payload = pack_msb_fields(&[(0, 9), (0, 4), (4, 6)]);
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&build_packed_guid(0, 0));
    validate_group_full_result(&payload).unwrap();

    let wrong = pack_msb_fields(&[(0, 9), (0, 4), (5, 6)]);
    assert!(validate_group_full_result(&wrong).is_err());
}
#[test]
fn group_capacity_invite_result_requires_ok_for_the_exact_candidate() {
    let mut ok = pack_msb_fields(&[(7, 9), (0, 4), (0, 6)]);
    ok.extend_from_slice(&0u32.to_le_bytes());
    ok.extend_from_slice(&build_packed_guid(0, 0));
    ok.extend_from_slice(b"Lfgheal");
    validate_group_invite_ok_result(&ok, "Lfgheal").unwrap();

    let mut wrong_name = pack_msb_fields(&[(7, 9), (0, 4), (0, 6)]);
    wrong_name.extend_from_slice(&0u32.to_le_bytes());
    wrong_name.extend_from_slice(&build_packed_guid(0, 0));
    wrong_name.extend_from_slice(b"Lfgmage");
    assert!(validate_group_invite_ok_result(&wrong_name, "Lfgheal").is_err());

    let mut wrong_faction = pack_msb_fields(&[(7, 9), (0, 4), (8, 6)]);
    wrong_faction.extend_from_slice(&0u32.to_le_bytes());
    wrong_faction.extend_from_slice(&build_packed_guid(0, 0));
    wrong_faction.extend_from_slice(b"Lfgheal");
    let error = validate_group_invite_ok_result(&wrong_faction, "Lfgheal")
        .expect_err("WRONG_FACTION must fail before the accept barrier");
    assert!(error.to_string().contains("result=8"));
}
#[test]
fn party_update_rejects_non_personal_loot_for_shared_chest_race() {
    let leader = create_player_guid_raw(15, ITEM_TEST_REALM);
    let peer = create_player_guid_raw(16, ITEM_TEST_REALM);
    let payload = party_update_for_test(0, 0, 1, 0, (77, 88), leader, [leader, peer], 0);

    let error = validate_party_update_like_cpp(&payload, leader, peer, leader)
        .expect_err("shared chest race must pin C++ PERSONAL_LOOT");
    assert!(error.to_string().contains("PERSONAL_LOOT"));
}
#[test]
fn party_update_rejects_non_home_empty_or_wrong_roster_states() {
    let leader = create_player_guid_raw(15, ITEM_TEST_REALM);
    let peer = create_player_guid_raw(16, ITEM_TEST_REALM);

    let non_home = party_update_for_test(
        0,
        1,
        1,
        0,
        (77, 88),
        leader,
        [leader, peer],
        PERSONAL_LOOT_METHOD_LIKE_CPP,
    );
    assert!(
        validate_party_update_like_cpp(&non_home, leader, peer, leader)
            .unwrap_err()
            .to_string()
            .contains("normal HOME")
    );

    let empty_group = party_update_for_test(
        0,
        0,
        1,
        0,
        (0, 0),
        leader,
        [leader, peer],
        PERSONAL_LOOT_METHOD_LIKE_CPP,
    );
    assert!(
        validate_party_update_like_cpp(&empty_group, leader, peer, leader)
            .unwrap_err()
            .to_string()
            .contains("empty PartyGUID")
    );

    let duplicate = party_update_for_test(
        0,
        0,
        1,
        0,
        (77, 88),
        leader,
        [leader, leader],
        PERSONAL_LOOT_METHOD_LIKE_CPP,
    );
    assert!(
        validate_party_update_like_cpp(&duplicate, leader, peer, leader)
            .unwrap_err()
            .to_string()
            .contains("did not contain exactly")
    );

    let wrong_receiver_index = party_update_for_test(
        0,
        0,
        1,
        1,
        (77, 88),
        leader,
        [leader, peer],
        PERSONAL_LOOT_METHOD_LIKE_CPP,
    );
    assert!(
        validate_party_update_like_cpp(&wrong_receiver_index, leader, peer, leader)
            .unwrap_err()
            .to_string()
            .contains("expected receiver")
    );
}
#[test]
fn party_update_rejects_truncated_or_trailing_payloads() {
    let leader = create_player_guid_raw(15, ITEM_TEST_REALM);
    let peer = create_player_guid_raw(16, ITEM_TEST_REALM);
    let payload = party_update_for_test(
        0,
        0,
        1,
        0,
        (77, 88),
        leader,
        [leader, peer],
        PERSONAL_LOOT_METHOD_LIKE_CPP,
    );

    let truncated = &payload[..payload.len() - 1];
    assert!(validate_party_update_like_cpp(truncated, leader, peer, leader).is_err());

    let mut trailing = payload;
    trailing.push(0);
    assert!(
        validate_party_update_like_cpp(&trailing, leader, peer, leader)
            .unwrap_err()
            .to_string()
            .contains("trailing byte")
    );
}
#[test]
fn party_invite_matches_cpp_bit_and_field_order() {
    let payload = build_party_invite("Peer", 0x11, 0x22).unwrap();
    assert_eq!(&payload[..4], &[0x00, 0x02, 0x00, 0x00]);
    assert_eq!(&payload[4..8], &[0, 0, 0, 0]);
    assert!(payload.ends_with(b"Peer"));
}
#[test]
fn loot_item_claim_contains_one_exact_request_and_soft_interact_false() {
    let window = LootWindow {
        owner_low: 1,
        owner_high: 2,
        loot_low: 0x11,
        loot_high: 0x22,
        coins: 10,
        item_entry: 46_052,
        quantity: 1,
        loot_list_id: 7,
        loot_method: PERSONAL_LOOT_METHOD_LIKE_CPP,
    };
    let payload = build_loot_item_claim(&window);
    assert_eq!(&payload[..4], &1u32.to_le_bytes());
    assert_eq!(&payload[payload.len() - 2..], &[7, 0]);
}
#[test]
fn creature_guid_and_loot_unit_match_cpp_wire() {
    let (low, high) = create_creature_guid_raw(0, 62, 279_748);
    assert_eq!(low, 279_748);
    assert_eq!(high >> 58, 8);
    assert_eq!((high >> 29) & 0x1FFF, 0);
    assert_eq!((high >> 6) & 0x7F_FFFF, 62);
    assert_eq!(
        build_packed_guid(low, high),
        vec![0x07, 0x83, 0xC4, 0x44, 0x04, 0x80, 0x0F, 0x20]
    );
}
#[test]
fn loot_removed_requires_the_full_discovered_creature_owner_guid() {
    const CREATURE_COUNTER: u64 = 268;
    const CREATURE_HIGH: u64 = 0x2000_0442_4015_44C0;
    let expected_owner = (CREATURE_COUNTER, CREATURE_HIGH);
    let loot_obj = (
        41,
        (HIGH_GUID_LOOT_OBJECT << 58) | (1 << 42) | (u64::from(530u16) << 29),
    );
    let removal_payload = |owner: (u64, u64)| {
        let mut payload = build_packed_guid(owner.0, owner.1);
        payload.extend_from_slice(&build_packed_guid(loot_obj.0, loot_obj.1));
        payload.push(3);
        payload
    };

    let mut evidence = WireEvidence::default();
    record_evidence(
        SMSG_LOOT_REMOVED,
        &removal_payload(expected_owner),
        expected_owner,
        &mut evidence,
    )
    .unwrap();
    assert_eq!(
        evidence.loot_removed,
        vec![LootRemovedEvidence {
            owner_low: expected_owner.0,
            owner_high: expected_owner.1,
            loot_low: loot_obj.0,
            loot_high: loot_obj.1,
            loot_list_id: 3,
        }]
    );

    let wrong_runtime_counter = (CREATURE_COUNTER + 1, CREATURE_HIGH);
    let error = record_evidence(
        SMSG_LOOT_REMOVED,
        &removal_payload(wrong_runtime_counter),
        expected_owner,
        &mut evidence,
    )
    .expect_err("a removal for another runtime creature must fail closed");
    assert!(error.to_string().contains("discovered world-object GUID"));
    assert_eq!(evidence.loot_removed.len(), 1);
}
#[test]
fn item_push_parser_consumes_the_complete_cpp_343_shape_and_rejects_a_tail() {
    let mut payload = build_packed_guid(0x0102, 0);
    payload.push(4);
    for value in [-1, 777, 3, 9, 615, 123, 188, 26, 25] {
        payload.extend_from_slice(&i32::to_le_bytes(value));
    }
    payload.extend_from_slice(&build_packed_guid(0x0506, 0));
    // Pushed=true, Created=false, DisplayText=EncounterLoot,
    // IsBonusRoll=false, IsEncounterLoot=true, then byte-align.
    payload.push(0x92);
    payload.extend_from_slice(&9001i32.to_le_bytes());
    payload.extend_from_slice(&12i32.to_le_bytes());
    payload.extend_from_slice(&(-77i32).to_le_bytes());
    payload.push(0x00); // ItemBonus absent, then byte-align.
    payload.push(0x00); // ItemModList has zero 6-bit entries.

    assert_eq!(
        parse_item_push(&payload).unwrap(),
        ItemPush {
            player_low: 0x0102,
            player_high: 0,
            slot: 4,
            slot_in_bag: -1,
            quest_log_item_id: 777,
            quantity: 3,
            quantity_in_inventory: 9,
            dungeon_encounter_id: 615,
            item_guid_low: 0x0506,
            item_guid_high: 0,
            pushed: true,
            created: false,
            display_text: 2,
            is_bonus_roll: false,
            is_encounter_loot: true,
            item_entry: 9001,
        }
    );

    payload.push(0xAA);
    let error = parse_item_push(&payload)
        .expect_err("SMSG_ITEM_PUSH_RESULT trailing bytes must fail closed");
    assert!(error.to_string().contains("unexpected trailing bytes"));
}
#[test]
fn atomic_item_wire_proves_one_logical_grant_for_direct_or_cpp_group_fanout() {
    for group_broadcast in [false, true] {
        let (evidence, removal) = valid_atomic_item_outcome(group_broadcast);
        let grant = validate_atomic_item_wire_outcome_like_cpp(
            &evidence,
            ITEM_TEST_CHARACTERS,
            ITEM_TEST_ENTRY,
            1,
            removal,
            ITEM_TEST_REALM,
        )
        .unwrap();
        assert_eq!(grant.owner_guid, ITEM_TEST_CHARACTERS[0]);
        assert_eq!(grant.push, valid_atomic_item_push());
    }
}
