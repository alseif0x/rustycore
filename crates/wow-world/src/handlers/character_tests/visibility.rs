//! Visibility scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn map_corpse_loader_applies_persisted_phases_and_customizations_once_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0).map_mut();
    assert!(map.load_grid(10.0, 20.0));
    let row = LoadedMapCorpseRowLikeCpp {
        position: Position::new(10.0, 20.0, 30.0, 1.5),
        map_id: 571,
        display_id: 12_345,
        items: std::array::from_fn(|slot| slot as u32 + 100),
        race: 4,
        class: 1,
        sex: 0,
        flags: 0x20,
        dynamic_flags: 0x01,
        ghost_time: 1_000,
        corpse_type: CorpseType::ResurrectablePve,
        instance_id: 0,
        owner_db_guid: 77,
    };
    let phases = HashMap::from([(77, BTreeSet::from([9, 10]))]);
    let choices = vec![
        CorpseCustomizationChoice {
            option_id: 101,
            choice_id: 201,
        },
        CorpseCustomizationChoice {
            option_id: 102,
            choice_id: 202,
        },
    ];
    let customizations = HashMap::from([(77, choices.clone())]);
    let factions = HashMap::from([(4, 35)]);

    let mut invalid_position_row = row.clone();
    invalid_position_row.position.x = f32::NAN;
    let outcome = materialize_loaded_map_corpses_like_cpp(
        map,
        9,
        vec![invalid_position_row, row.clone()],
        &phases,
        &customizations,
        &factions,
    );

    assert_eq!(outcome.rows_seen, 2);
    assert_eq!(outcome.corpses_added, 1);
    assert_eq!(outcome.invalid_position_rows, 1);
    assert!(map.corpse_data_loaded_like_cpp());
    let corpse_guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 9, 571, 0, 0, 2);
    let corpse = map.get_typed_corpse(corpse_guid).unwrap();
    assert_eq!(corpse.data().owner, ObjectGuid::create_player(9, 77));
    assert_eq!(corpse.data().customizations, choices);
    assert_eq!(corpse.data().items, row.items);
    assert_eq!(corpse.data().faction_template, 35);
    assert!(corpse.world().phase_shift().has_phase_like_cpp(9));
    assert!(corpse.world().phase_shift().has_phase_like_cpp(10));
    assert!(!corpse.corpse_data_changes_mask().is_any_set());
    assert!(corpse.world().object().is_in_world());
    assert!(
        map.nearby_cell_guids_like_cpp(10.0, 20.0, 1.0)
            .world
            .corpses
            .contains(&corpse_guid)
    );

    let duplicate = materialize_loaded_map_corpses_like_cpp(
        map,
        9,
        vec![row],
        &phases,
        &customizations,
        &factions,
    );
    assert!(duplicate.already_loaded);
    assert_eq!(duplicate.corpses_added, 0);
}
