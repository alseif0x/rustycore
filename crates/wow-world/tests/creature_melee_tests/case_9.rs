use super::*;

#[test]
fn legacy_creature_victim_sync_cas_rejects_stale_aba_health_state_like_cpp() {
    let guid = test_creature_guid(91_046);
    let mut victim = wow_world::map_manager::WorldCreature::new(
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
    let sync = creature_melee_sync_state_for_test_like_cpp(&victim, 10);
    victim.creature.unit_mut().set_health(90);
    victim.creature.unit_mut().set_health(100);
    let before = (
        victim.creature.unit().data().health,
        victim.creature.unit().death_state(),
        victim.creature.unit().health_state_revision_like_cpp(),
        victim.creature.loot_lifecycle_revision_like_cpp(),
    );

    assert!(!apply_creature_melee_victim_sync_to_legacy_like_cpp(
        &mut victim,
        &sync,
        wow_entities::game_time_secs_like_cpp(),
    ));

    assert_eq!(
        (
            victim.creature.unit().data().health,
            victim.creature.unit().death_state(),
            victim.creature.unit().health_state_revision_like_cpp(),
            victim.creature.loot_lifecycle_revision_like_cpp(),
        ),
        before
    );
    assert!(
        victim
            .creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&sync.identity.authority)
    );
}
