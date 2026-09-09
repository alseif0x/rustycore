//! Quest scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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
