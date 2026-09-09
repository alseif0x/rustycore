//! Gameobject scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn game_event_quest_gameobject_preserves_order_like_cpp() {
    let mut quests = GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut report = GameEventQuestRelationFamilyLoadReportLikeCpp::default();

    apply_game_event_gameobject_quest_relation_row_like_cpp(
        game_event_quest_row(1, 200, 8000),
        &mut quests,
        &mut report,
    );
    apply_game_event_gameobject_quest_relation_row_like_cpp(
        game_event_quest_row(1, 201, 8001),
        &mut quests,
        &mut report,
    );

    let records = quests.gameobject_records_like_cpp(1).unwrap();
    assert_eq!(records[0].giver_id, 200);
    assert_eq!(records[0].quest_id, 8000);
    assert_eq!(records[1].giver_id, 201);
    assert_eq!(records[1].quest_id, 8001);
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 2);
}
