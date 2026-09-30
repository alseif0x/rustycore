use super::*;

#[test]
fn legacy_creature_victim_sync_cas_rejects_same_guid_replacement_like_cpp() {
    let guid = test_creature_guid(91_045);
    let original = wow_world::map_manager::WorldCreature::new(
        guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        100,
        80,
        10,
        10,
        20.0,
        1,
        35,
        0,
        0,
    );
    let sync = creature_melee_sync_state_for_test_like_cpp(&original, 10);
    let mut replacement = wow_world::map_manager::WorldCreature::new(
        guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        100,
        80,
        10,
        10,
        20.0,
        1,
        35,
        0,
        0,
    );
    let before = (
        replacement.creature.unit().data().health,
        replacement.creature.unit().death_state(),
        replacement.creature.unit().health_state_revision_like_cpp(),
        replacement.creature.loot_lifecycle_revision_like_cpp(),
    );

    assert!(!apply_creature_melee_victim_sync_to_legacy_like_cpp(
        &mut replacement,
        &sync,
        wow_entities::game_time_secs_like_cpp(),
    ));

    assert_eq!(
        (
            replacement.creature.unit().data().health,
            replacement.creature.unit().death_state(),
            replacement.creature.unit().health_state_revision_like_cpp(),
            replacement.creature.loot_lifecycle_revision_like_cpp(),
        ),
        before
    );
    assert!(
        !replacement
            .creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&sync.identity.authority)
    );
}
