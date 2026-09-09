//! Group scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn canonical_spawn_metadata_spawn_group_helper_filters_by_map_and_template_like_cpp() {
    let (template_store, _) = wow_data::SpawnGroupTemplateStore::from_rows_like_cpp([
        wow_data::SpawnGroupTemplateRow {
            group_id: 20,
            name: "map-one-a".to_string(),
            flags: 0,
        },
        wow_data::SpawnGroupTemplateRow {
            group_id: 21,
            name: "map-one-b".to_string(),
            flags: 0,
        },
        wow_data::SpawnGroupTemplateRow {
            group_id: 22,
            name: "map-two".to_string(),
            flags: 0,
        },
    ]);
    let mut templates = spawn_group_templates_for_spawn_store(&template_store);
    let maps = map_store(&[1, 2]);
    let difficulties = map_difficulty_store(&[(1, 0), (2, 0)]);
    let mut report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();

    let map_one_a = creature_row_to_spawn_data_like_cpp(
        &creature_row(400, 0, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .unwrap();
    let map_one_b = gameobject_row_to_spawn_data_like_cpp(
        &gameobject_row(401, 0, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .unwrap();
    let mut map_two_row = creature_row(402, 0, "0");
    map_two_row.map_id = 2;
    let map_two =
        creature_row_to_spawn_data_like_cpp(&map_two_row, &maps, &difficulties, &mut report)
            .unwrap();

    store.add_object_spawn(&map_one_a, is_personal_phase_like_cpp_represented);
    store.add_object_spawn(&map_one_b, is_personal_phase_like_cpp_represented);
    store.add_object_spawn(&map_two, is_personal_phase_like_cpp_represented);
    let apply = store.apply_spawn_groups_like_cpp(
        &mut templates,
        [
            SpawnGroupMemberRow {
                group_id: 21,
                spawn_type: SpawnObjectType::GameObject as u8,
                spawn_id: 401,
            },
            SpawnGroupMemberRow {
                group_id: 20,
                spawn_type: SpawnObjectType::Creature as u8,
                spawn_id: 400,
            },
            SpawnGroupMemberRow {
                group_id: 22,
                spawn_type: SpawnObjectType::Creature as u8,
                spawn_id: 402,
            },
        ],
    );
    assert_eq!(apply.assigned, 3);

    // Simulate a future C++-shaped filter miss without panicking: the group id is indexed
    // for the map, but `GetSpawnGroupData`/map filtering no longer returns a matching template.
    templates.get_mut(&21).unwrap().map_id = 2;
    let metadata = CanonicalSpawnMetadataLikeCpp::new(store, templates);

    let map_one_groups = metadata.spawn_group_templates_for_map_like_cpp(1);
    assert_eq!(
        map_one_groups
            .iter()
            .map(|(group_id, template)| (*group_id, template.name.as_str()))
            .collect::<Vec<_>>(),
        vec![(20, "map-one-a")]
    );
    let map_two_groups = metadata.spawn_group_templates_for_map_like_cpp(2);
    assert_eq!(
        map_two_groups
            .iter()
            .map(|(group_id, template)| (*group_id, template.name.as_str()))
            .collect::<Vec<_>>(),
        vec![(22, "map-two")]
    );
    assert!(
        metadata
            .spawn_group_templates_for_map_like_cpp(999)
            .is_empty()
    );
}
