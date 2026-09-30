use super::*;
use wow_world::test_fixtures::{
    quest_giver_creature_entry_for_test as quest_giver_creature_id_from_source_like_cpp,
    quest_completion_npc_response_for_test as represented_quest_completion_npc_response_like_cpp,
};

#[test]
fn quest_giver_creature_id_is_zero_for_gameobject_sources_like_cpp() {
    assert_eq!(
        quest_giver_creature_id_from_source_like_cpp(creature_guid(15513, 27)),
        15513
    );
    assert_eq!(
        quest_giver_creature_id_from_source_like_cpp(gameobject_guid(180516, 301)),
        0
    );
}

#[test]
fn query_quest_completion_builds_creature_then_masked_go_entries_like_cpp() {
    let mut store = store_with_quests(&[77]);
    store.ender_quests.entry(1234).or_default().push(77);
    store.ender_quests.entry(12).or_default().push(77);
    store
        .gameobject_ender_quests
        .entry(0x5678)
        .or_default()
        .push(77);

    let response = represented_quest_completion_npc_response_like_cpp(&store, &[77]);

    assert_eq!(response.len(), 1);
    assert_eq!(response[0].quest_id, 77);
    assert_eq!(response[0].npcs, vec![12, 1234, 0x8000_5678u32 as i32]);
}

#[test]
fn query_quest_completion_skips_negative_missing_and_oversized_creature_entries_like_cpp() {
    let mut store = store_with_quests(&[5]);
    store
        .ender_quests
        .entry(i32::MAX as u32 + 1)
        .or_default()
        .push(5);
    store
        .gameobject_ender_quests
        .entry(u32::MAX)
        .or_default()
        .push(5);

    let response = represented_quest_completion_npc_response_like_cpp(&store, &[-1, 999, 5]);

    assert_eq!(response.len(), 1);
    assert_eq!(response[0].quest_id, 5);
    assert_eq!(response[0].npcs, vec![-1]);
}

fn creature_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        1,
        571,
        0,
        entry,
        counter,
    )
}

fn gameobject_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        entry,
        counter,
    )
}

fn store_with_quests(ids: &[u32]) -> wow_data::quest::QuestStore {
    wow_data::quest::QuestStore::from_quests_like_cpp(ids.iter().copied().map(quest_template))
}

fn quest_template(id: u32) -> wow_data::quest::QuestTemplate {
    wow_world::test_fixtures::quest_template_row_for_test(id, 2, format!("Quest {id}"))
}
