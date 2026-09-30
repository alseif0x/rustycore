//! Existing resolver fixtures exercise both projections of the same creator.
use super::*;

#[test]
fn compatibility_and_move_projection_preserve_created_fields_and_insertion_flags() {
    for requested in [false, true] {
        let entry = 12_345;
        let mut selected_template = template(entry);
        selected_template.sparring_health_pct = Some(35.5);
        let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [selected_template],
            [spawn(55, entry, requested)],
            [selection(entry)],
        );
        let guid = map_creature_guid(entry, 571, 55);
        let legacy = resolver
            .resolve_loaded_grid_creature_like_cpp(55, guid)
            .unwrap();
        let moved = resolver.resolve_creature_record(55, guid).unwrap();
        assert_eq!(legacy.map_insertion_requested, requested);
        assert_eq!(legacy.map_object_record.is_some(), requested);
        assert_eq!(moved.is_some(), requested);
        assert_eq!(legacy.lifecycle_record.create.guid, guid);
        assert_eq!(
            legacy.lifecycle_record.create.selected_level,
            selection(entry).1.selected_level
        );
        assert_eq!(
            legacy.lifecycle_record.create.stats,
            selection(entry).1.stats
        );
        if let Some(moved) = moved {
            let moved = moved.creature().unwrap();
            let original = &legacy.creature;
            assert_eq!(moved.guid(), original.guid());
            assert_eq!(moved.entry(), original.entry());
            assert_eq!(moved.position(), original.position());
            assert_eq!(moved.level(), original.level());
            assert_eq!(
                (moved.current_health(), moved.max_health()),
                (original.current_health(), original.max_health())
            );
            assert_eq!(moved.lifecycle_metadata(), original.lifecycle_metadata());
            assert_eq!(
                moved.formation_info_like_cpp(),
                original.formation_info_like_cpp()
            );
            assert_eq!(
                moved.default_movement_type(),
                original.default_movement_type()
            );
            assert_eq!(
                moved.movement_flags_like_cpp(),
                original.movement_flags_like_cpp()
            );
            assert_eq!(moved.sparring_health_pct(), original.sparring_health_pct());
            assert_eq!(
                moved.ai_ownership().loot_id,
                original.ai_ownership().loot_id
            );
            assert_eq!(
                moved.ai_ownership().skin_loot_id,
                original.ai_ownership().skin_loot_id
            );
            assert_eq!(
                moved.unit().world().object().is_in_world(),
                original.unit().world().object().is_in_world()
            );
            // The legacy DTO still exposes its original paired snapshot.
            let snapshot = legacy
                .map_object_record
                .as_ref()
                .unwrap()
                .creature()
                .unwrap();
            assert_eq!(snapshot.lifecycle_metadata(), original.lifecycle_metadata());
            assert!(
                snapshot
                    .loot_authority_like_cpp()
                    .shares_storage_like_cpp(original.loot_authority_like_cpp())
            );
        }
    }
}

#[test]
fn both_projections_preserve_missing_input_and_invalid_guid_error_order() {
    let entry = 12_345;
    let guid = map_creature_guid(entry, 571, 55);
    for mode in 0..4 {
        let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            if mode == 1 {
                vec![]
            } else {
                vec![template(entry)]
            },
            if mode == 0 {
                vec![]
            } else {
                vec![spawn(55, entry, true)]
            },
            if mode == 2 {
                vec![]
            } else {
                vec![selection(entry)]
            },
        );
        let guid = if mode == 3 {
            map_creature_guid(entry, 572, 55)
        } else {
            guid
        };
        let expected = resolver
            .resolve_loaded_grid_creature_like_cpp(55, guid)
            .unwrap_err();
        assert_eq!(
            resolver.resolve_creature_record(55, guid).unwrap_err(),
            expected
        );
        match mode {
            0 => assert_eq!(
                expected,
                CreatureLoadedGridResolveErrorLikeCpp::MissingSpawnData { spawn_id: 55 }
            ),
            1 => assert_eq!(
                expected,
                CreatureLoadedGridResolveErrorLikeCpp::MissingTemplate { entry }
            ),
            2 => assert_eq!(
                expected,
                CreatureLoadedGridResolveErrorLikeCpp::MissingRuntimeSelection { entry }
            ),
            _ => assert!(matches!(
                expected,
                CreatureLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid { .. }
            )),
        }
    }
}

#[test]
fn move_projection_retains_vehicle_guid_map_and_spawn_metadata() {
    let entry = 12_345;
    let mut selected_template = template(entry);
    selected_template.vehicle_id = Some(123);
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [selected_template],
        [spawn(55, entry, true)],
        [selection(entry)],
    );
    let guid = map_vehicle_guid(entry, 571, 55);
    let original = resolver
        .resolve_loaded_grid_creature_like_cpp(55, guid)
        .unwrap();
    let moved = resolver.resolve_creature_record(55, guid).unwrap().unwrap();
    let moved = moved.creature().unwrap();
    assert_eq!(moved.guid(), original.creature.guid());
    assert_eq!(
        moved.lifecycle_metadata(),
        original.creature.lifecycle_metadata()
    );
    assert_eq!(
        moved.unit().world().instance_id(),
        original.creature.unit().world().instance_id()
    );
}
