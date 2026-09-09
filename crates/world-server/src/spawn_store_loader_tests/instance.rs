//! Instance scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn game_event_world_state_invalid_map_and_area_lists_skip_rows_like_cpp() {
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [
            world_state_row(103, 1, "bogus,99", ""),
            world_state_row(104, 2, "1", "bogus,999"),
            world_state_row(105, 3, "1,not-int", "10,bad"),
        ],
        [],
        |map_id| map_id == 1,
        |area_id| (area_id == 10).then_some(1),
    );

    assert_eq!(report.template_rows, 3);
    assert_eq!(report.skipped_invalid_map_list, 1);
    assert_eq!(report.skipped_invalid_area_list, 1);
    assert_eq!(report.templates_loaded, 1);
    assert!(mgr.template_like_cpp(103).is_none());
    assert!(mgr.template_like_cpp(104).is_none());
    assert_eq!(mgr.map_value_like_cpp(1, 105), 3);
    assert_eq!(
        mgr.template_like_cpp(105)
            .map(|template| template.area_ids.contains(&10)),
        Some(true)
    );
}
#[test]
fn game_event_world_state_area_continent_must_match_required_maps_like_cpp() {
    let areas = area_store(&[(20, 2), (21, 1)]);
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [world_state_row(106, 4, "1", "20,21")],
        [],
        |map_id| map_id == 1,
        |area_id| areas.get(area_id).map(|area| area.continent_id),
    );

    assert_eq!(report.templates_loaded, 1);
    assert_eq!(
        mgr.template_like_cpp(106)
            .map(|template| template.area_ids.contains(&20)),
        Some(false)
    );
    assert_eq!(
        mgr.template_like_cpp(106)
            .map(|template| template.area_ids.contains(&21)),
        Some(true)
    );
}
#[test]
fn game_event_spawn_guids_signed_internal_mapping_and_empty_valid_slice_like_cpp() {
    let guids = GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));

    assert_eq!(guids.game_event_size_like_cpp(), 4);
    assert_eq!(guids.internal_event_id_like_cpp(1), Some(4));
    assert_eq!(guids.internal_event_id_like_cpp(-1), Some(2));
    assert_eq!(guids.internal_event_id_like_cpp(-5), None);
    assert_eq!(guids.internal_event_id_like_cpp(4), None);
    assert_eq!(guids.creature_guids_like_cpp(2), Some([].as_slice()));
    assert_eq!(guids.gameobject_guids_like_cpp(-2), Some([].as_slice()));
    assert_eq!(guids.creature_guids_like_cpp(4), None);
}
#[test]
fn spawn_difficulty_parser_matches_cpp_token_rules() {
    let difficulties = map_difficulty_store(&[(1, 0), (1, 1)]);
    let parsed = parse_spawn_difficulties_like_cpp("0,1", 1, false, &difficulties);
    assert_eq!(parsed.difficulties, vec![0, 1]);
    assert_eq!(parsed.report.invalid_tokens_as_none, 0);
    assert!(parsed.report.unsupported.is_empty());

    let parsed = parse_spawn_difficulties_like_cpp("bad,1", 1, false, &difficulties);
    assert_eq!(parsed.difficulties, vec![0, 1]);
    assert_eq!(parsed.report.invalid_tokens_as_none, 1);

    let parsed = parse_spawn_difficulties_like_cpp("0,2,1", 1, false, &difficulties);
    assert_eq!(parsed.difficulties, vec![0, 1]);
    assert_eq!(parsed.report.unsupported, vec![2]);

    let parsed = parse_spawn_difficulties_like_cpp("2", 1, true, &difficulties);
    assert_eq!(parsed.difficulties, vec![2]);

    let parsed = parse_spawn_difficulties_like_cpp("", 1, false, &difficulties);
    assert!(parsed.difficulties.is_empty());
}
#[test]
fn row_conversion_skips_missing_map_and_empty_difficulties() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut report = SpawnKindLoadReport::default();

    let mut missing_map = creature_row(200, 0, "0");
    missing_map.map_id = 999;
    assert!(
        creature_row_to_spawn_data_like_cpp(&missing_map, &maps, &difficulties, &mut report)
            .is_none()
    );
    assert_eq!(report.skipped_missing_map, 1);

    assert!(
        creature_row_to_spawn_data_like_cpp(
            &creature_row(201, 0, ""),
            &maps,
            &difficulties,
            &mut report,
        )
        .is_none()
    );
    assert_eq!(report.skipped_empty_difficulties, 1);
}
