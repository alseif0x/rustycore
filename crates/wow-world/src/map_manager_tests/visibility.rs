//! Visibility scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn terrain_map_id_without_visible_maps_returns_source_map_like_cpp() {
    let phase_shift = PhaseShift::default();
    let mut called = false;

    let map_id = terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0, |_, _, _| {
        called = true;
        true
    });

    assert_eq!(map_id, 571);
    assert!(!called);
}
#[test]
fn terrain_map_id_single_visible_map_returns_it_like_cpp() {
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(609, 1);
    let mut called = false;

    let map_id = terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0, |_, _, _| {
        called = true;
        false
    });

    assert_eq!(map_id, 609);
    assert!(!called);
}
#[test]
fn terrain_map_id_multiple_visible_maps_uses_child_grid_lookup_like_cpp() {
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(700, 1);
    phase_shift.add_visible_map_id_like_cpp(609, 1);
    let mut checked = Vec::new();

    let map_id = terrain_map_id_for_phase_shift_like_cpp(
        &phase_shift,
        571,
        0.0,
        0.0,
        |visible_map_id, gx, gy| {
            checked.push((visible_map_id, gx, gy));
            visible_map_id == 609
        },
    );

    assert_eq!(map_id, 609);
    assert_eq!(checked, vec![(609, 31, 31)]);
}
#[test]
fn terrain_map_id_multiple_visible_maps_falls_back_to_source_map_like_cpp() {
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(609, 1);
    phase_shift.add_visible_map_id_like_cpp(700, 1);

    let map_id =
        terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0, |_, _, _| false);

    assert_eq!(map_id, 571);
}
#[test]
fn terrain_grid_files_resolve_phase_shift_visible_map_like_cpp() {
    let data_dir = unique_temp_data_dir("terrain-grid-resolver");
    let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
    fs::write(
        data_dir.join("maps").join("0571.tilelist"),
        tilelist_like_cpp([]),
    )
    .expect("write parent tilelist");
    fs::write(
        data_dir.join("maps").join("0609.tilelist"),
        tilelist_like_cpp([grid_idx]),
    )
    .expect("write child tilelist");
    let parent_child_map_data = HashMap::from([(571, vec![609]), (609, Vec::new())]);
    let terrain =
        TerrainGridFilesLikeCpp::load_root_like_cpp(&data_dir, 571, &parent_child_map_data)
            .expect("load terrain grid files");
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(700, 1);
    phase_shift.add_visible_map_id_like_cpp(609, 1);

    assert_eq!(
        terrain.terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0),
        609
    );
    fs::remove_dir_all(data_dir).expect("remove test dir");
}
#[test]
fn terrain_grid_file_index_resolves_root_and_visible_child_map_like_cpp() {
    let data_dir = unique_temp_data_dir("terrain-grid-index");
    let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
    fs::write(
        data_dir.join("maps").join("0571.tilelist"),
        tilelist_like_cpp([]),
    )
    .expect("write parent tilelist");
    fs::write(
        data_dir.join("maps").join("0609.tilelist"),
        tilelist_like_cpp([grid_idx]),
    )
    .expect("write child tilelist");
    let mut index =
        TerrainGridFileIndexLikeCpp::new(&data_dir, [(571, vec![609]), (609, Vec::new())]);
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(609, 1);

    assert_eq!(index.root_map_id_like_cpp(609), 571);
    assert_eq!(
        index.terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0),
        609
    );
    fs::remove_dir_all(data_dir).expect("remove test dir");
}
#[test]
#[test]
#[test]
/// Smoke: `RecipientRule::NearbyVisible` stores all its fields correctly.
#[test]
fn recipient_rule_nearby_visible_stores_fields() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 7);
    let pos = Position::new(1.0, 2.0, 3.0, 0.5);

    let rule = RecipientRule::NearbyVisible {
        source_guid: guid,
        map_id: 571,
        instance_id: 0,
        source_position: pos,
        range: 100.0,
        required_3d: true,
    };

    if let RecipientRule::NearbyVisible {
        source_guid,
        map_id,
        instance_id,
        source_position,
        range,
        required_3d,
    } = rule
    {
        assert_eq!(source_guid, guid);
        assert_eq!(map_id, 571);
        assert_eq!(instance_id, 0);
        assert_eq!(source_position.x, 1.0);
        assert_eq!(source_position.y, 2.0);
        assert_eq!(source_position.z, 3.0);
        assert!((range - 100.0).abs() < f32::EPSILON);
        assert!(required_3d);
    } else {
        panic!("expected NearbyVisible");
    }
}
