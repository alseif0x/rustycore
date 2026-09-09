//! Spawn scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn canonical_spawn_capability_keeps_the_complete_staged_order() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let spawn = RecordingCanonicalSpawnCatalogLikeCpp {
        calls: Arc::clone(&calls),
        fail_on: None,
    };
    let character = RecordingGameEventConditionSaveLikeCpp {
        calls: Arc::clone(&calls),
        fail: false,
    };
    let game_event_world = RecordingGameEventWorldCatalogLikeCpp {
        calls: Arc::clone(&calls),
        fail_prefix: false,
        fail_suffix: false,
    };

    run_empty_canonical_spawn_pipeline_like_cpp(&spawn, &character, &game_event_world)
        .await
        .unwrap();

    assert_eq!(
        *calls.lock().unwrap(),
        [
            "creature-spawns",
            "waypoints",
            "formations",
            "gameobject-spawns",
            "area-trigger-spawns",
            "linked-respawns",
            "pool-templates",
            "pool-creatures",
            "pool-gameobjects",
            "pool-pools",
            "pool-autospawn",
            "world-prefix",
            "character-saves",
            "world-suffix",
            "spawn-groups",
        ]
    );
}
#[tokio::test]
async fn canonical_spawn_failure_stops_every_later_stage() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let spawn = RecordingCanonicalSpawnCatalogLikeCpp {
        calls: Arc::clone(&calls),
        fail_on: Some("waypoints"),
    };
    let character = RecordingGameEventConditionSaveLikeCpp {
        calls: Arc::clone(&calls),
        fail: false,
    };
    let game_event_world = RecordingGameEventWorldCatalogLikeCpp {
        calls: Arc::clone(&calls),
        fail_prefix: false,
        fail_suffix: false,
    };

    let error = run_empty_canonical_spawn_pipeline_like_cpp(&spawn, &character, &game_event_world)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("waypoint"));
    assert_eq!(*calls.lock().unwrap(), ["creature-spawns", "waypoints"]);
}
#[test]
fn game_event_update_inactive_not_active_records_negative_spawn_only_after_init_like_cpp() {
    for (is_system_init, expected_negative_spawns) in [(false, vec![-1]), (true, vec![])] {
        let mut metadata =
            CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
                .with_game_events_like_cpp(game_event_store_with_max(
                    1,
                    [event(1, GameEventStateLikeCpp::Normal, 100, 1_000, 10, 2)],
                ));

        let outcome = metadata.update_game_events_like_cpp(650, is_system_init, |_| false);

        assert_eq!(outcome.negative_spawn_event_ids, expected_negative_spawns);
        assert!(outcome.queued_activation_event_ids.is_empty());
        assert!(outcome.queued_deactivation_event_ids.is_empty());
        assert!(
            metadata
                .game_event_active_set_like_cpp()
                .active_event_ids_like_cpp()
                .collect::<Vec<_>>()
                .is_empty()
        );
    }
}
#[test]
fn game_event_spawn_guids_skip_missing_spawn_metadata_like_cpp() {
    let store = game_event_guid_test_store();
    let mut guids = GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventObjectGuidLoadReportLikeCpp::default();

    apply_game_event_object_guid_row_like_cpp(
        GameEventObjectGuidRowLikeCpp {
            guid: 404,
            event_id: 1,
        },
        SpawnObjectType::Creature,
        &store,
        &mut guids,
        &mut report,
    );

    assert_eq!(guids.creature_guids_like_cpp(1), Some([].as_slice()));
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.skipped_missing_spawn_metadata, 1);
}
#[test]
fn game_event_spawn_guids_skip_out_of_range_like_cpp() {
    let store = game_event_guid_test_store();
    let mut guids = GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventObjectGuidLoadReportLikeCpp::default();

    apply_game_event_object_guid_row_like_cpp(
        GameEventObjectGuidRowLikeCpp {
            guid: 100,
            event_id: -5,
        },
        SpawnObjectType::Creature,
        &store,
        &mut guids,
        &mut report,
    );
    apply_game_event_object_guid_row_like_cpp(
        GameEventObjectGuidRowLikeCpp {
            guid: 102,
            event_id: 4,
        },
        SpawnObjectType::Creature,
        &store,
        &mut guids,
        &mut report,
    );

    assert_eq!(guids.creature_guids_like_cpp(-5), None);
    assert_eq!(guids.creature_guids_like_cpp(4), None);
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.skipped_out_of_range, 2);
}
#[test]
fn canonical_metadata_exposes_game_event_spawn_guid_slices_like_cpp() {
    let mut guids = GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(guids.push_guid_like_cpp(SpawnObjectType::Creature, 1, 100));
    assert!(guids.push_guid_like_cpp(SpawnObjectType::GameObject, -1, 200));
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_spawn_guids_like_cpp(guids);

    assert_eq!(
        metadata.game_event_creature_guids_like_cpp(1),
        Some([100].as_slice())
    );
    assert_eq!(
        metadata.game_event_gameobject_guids_like_cpp(-1),
        Some([200].as_slice())
    );
    assert_eq!(
        metadata.game_event_creature_guids_like_cpp(2),
        Some([].as_slice())
    );
    assert_eq!(metadata.game_event_gameobject_guids_like_cpp(4), None);
}
#[test]
fn area_trigger_spawn_skips_missing_create_properties_like_cpp() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let area_trigger_templates =
        area_trigger_template_store_with(area_trigger_create_properties_row(111), [], []);
    let mut report = SpawnKindLoadReport::default();
    let mut runtime_rows = BTreeMap::new();

    let spawn = area_trigger_row_to_spawn_data_like_cpp(
        &area_trigger_row(302, "0"),
        &maps,
        &difficulties,
        &area_trigger_templates,
        &mut |_| true,
        &mut |_| wow_data::ScriptIdLikeCpp(0),
        &mut runtime_rows,
        &mut report,
    );

    assert!(spawn.is_none());
    assert_eq!(
        report.skipped_invalid_create_properties,
        [(302, 789, false)]
    );
    assert!(runtime_rows.is_empty());
}
#[test]
fn area_trigger_spawn_skips_non_static_create_properties_like_cpp() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let load = |spawn_id, store: wow_data::AreaTriggerTemplateStore| {
        let mut report = SpawnKindLoadReport::default();
        let mut runtime_rows = BTreeMap::new();
        let spawn = area_trigger_row_to_spawn_data_like_cpp(
            &area_trigger_row(spawn_id, "0"),
            &maps,
            &difficulties,
            &store,
            &mut |_| true,
            &mut |_| wow_data::ScriptIdLikeCpp(0),
            &mut runtime_rows,
            &mut report,
        );
        (spawn, runtime_rows, report)
    };

    let mut flags_row = area_trigger_create_properties_row(789);
    flags_row.flags =
        wow_data::area_trigger_template::AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ATTACHED_LIKE_CPP;
    let (spawn, runtime_rows, report) =
        load(302, area_trigger_template_store_with(flags_row, [], []));
    assert!(spawn.is_none());
    assert!(runtime_rows.is_empty());
    assert_eq!(
        report.skipped_nonzero_create_properties_flags,
        [(302, 789, false)]
    );

    let mut curve_row = area_trigger_create_properties_row(789);
    curve_row.move_curve_id = 44;
    let (spawn, runtime_rows, report) =
        load(303, area_trigger_template_store_with(curve_row, [], []));
    assert!(spawn.is_none());
    assert!(runtime_rows.is_empty());
    assert_eq!(report.skipped_create_properties_curves, [(303, 789, false)]);

    let mut time_row = area_trigger_create_properties_row(789);
    time_row.time_to_target = 1;
    let (spawn, runtime_rows, report) =
        load(304, area_trigger_template_store_with(time_row, [], []));
    assert!(spawn.is_none());
    assert!(runtime_rows.is_empty());
    assert_eq!(
        report.skipped_create_properties_time_to_target,
        [(304, 789, false)]
    );

    let (spawn, runtime_rows, report) = load(
        305,
        area_trigger_template_store_with(
            area_trigger_create_properties_row(789),
            [],
            [area_trigger_orbit(789)],
        ),
    );
    assert!(spawn.is_none());
    assert!(runtime_rows.is_empty());
    assert_eq!(report.skipped_create_properties_orbit, [(305, 789, false)]);

    let (spawn, runtime_rows, report) = load(
        306,
        area_trigger_template_store_with(
            area_trigger_create_properties_row(789),
            [
                area_trigger_spline_point(789, 1.0),
                area_trigger_spline_point(789, 2.0),
            ],
            [],
        ),
    );
    assert!(spawn.is_none());
    assert!(runtime_rows.is_empty());
    assert_eq!(
        report.skipped_create_properties_splines,
        [(306, 789, false)]
    );
}
