use super::*;
use wow_constants::DeathState;
use wow_entities::ReactState;

#[test]
fn equal_matching_tuple_moves_source_inner_and_complete_point_or_distract_motor() {
    for point in [false, true] {
        let (mut map, mut source, guid, mut rng) = fixtures::pair(801, point);
        let timeline = source_actor(&source, guid)
            .creature
            .unit()
            .health_state_revision_authority_like_cpp();
        let loot = source_actor(&source, guid)
            .creature
            .loot_authority_like_cpp()
            .clone();
        let revision = source_actor(&source, guid)
            .creature
            .unit()
            .health_state_revision_like_cpp();
        let grid = source.grids.get(&LegacyGridCoord::new(0, 0)).unwrap();
        let before = (
            grid.coord,
            grid.loaded,
            grid.last_player_time,
            grid.player_guids.clone(),
        );
        let summary = map
            .transport_legacy_creature_ownership(&mut source)
            .unwrap();
        assert_eq!(
            summary,
            CreatureActorTransportSummary {
                transported: 1,
                source_inner_winners: 1,
                canonical_inner_winners: 0
            }
        );
        assert_eq!(source_count(&source), 0);
        let grid = source.grids.get(&LegacyGridCoord::new(0, 0)).unwrap();
        assert_eq!(
            (
                grid.coord,
                grid.loaded,
                grid.last_player_time,
                grid.player_guids.clone()
            ),
            before
        );
        let actor = map.creature_actor_mut(guid).unwrap();
        actor.assert_actor_storage_runtime(&mut rng, point);
        assert_eq!(
            (actor.create_data.npc_flags, actor.create_data.scale),
            (0x1234, 1.75)
        );
        assert_eq!(
            (
                actor.creature.respawn_time(),
                actor.creature.respawn_delay()
            ),
            (12345, 73)
        );
        assert_eq!(actor.creature.react_state(), ReactState::Passive);
        assert_eq!(
            actor
                .creature
                .unit()
                .subsystems()
                .auras
                .transform_spell_like_cpp(),
            118
        );
        assert_eq!(
            actor
                .creature
                .unit()
                .subsystems()
                .spells
                .remaining_cooldown_ms(133, 0, 500),
            1500
        );
        assert_eq!(
            actor
                .creature
                .unit()
                .current_spell(wow_entities::CurrentSpellSlot::Generic)
                .unwrap()
                .spell_id,
            133
        );
        assert_eq!(
            actor.creature.unit().health_state_revision_like_cpp(),
            revision
        );
        assert!(
            actor
                .creature
                .unit()
                .shares_health_state_revision_authority_like_cpp(&timeline)
        );
        assert!(
            actor
                .creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(&loot)
        );
        assert!(actor.creature.unit().world().object().is_in_world());
        assert_eq!(map.creature_spawn_id_store_guids_like_cpp(8010), vec![guid]);
        assert_eq!(map.map_object_count(), 1);
    }
}

#[test]
fn greater_source_revision_moves_entire_health_death_aura_and_cast_state() {
    let (mut map, mut source, guid, mut rng) = fixtures::pair(802, true);
    let actor = source_actor_mut(&mut source, guid);
    actor.creature.unit_mut().set_max_health(200);
    actor.creature.unit_mut().set_health(0);
    actor
        .creature
        .unit_mut()
        .set_death_state(DeathState::Corpse);
    let revision = actor.creature.unit().health_state_revision_like_cpp();
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source)
            .unwrap()
            .source_inner_winners,
        1
    );
    let actor = map.creature_actor_mut(guid).unwrap();
    assert_eq!(
        (
            actor.creature.current_health(),
            actor.creature.unit().data().max_health,
            actor.creature.unit().death_state()
        ),
        (0, 200, DeathState::Corpse)
    );
    assert_eq!(
        actor.creature.unit().health_state_revision_like_cpp(),
        revision
    );
    assert_eq!(
        actor
            .creature
            .unit()
            .subsystems()
            .auras
            .transform_spell_like_cpp(),
        118
    );
    assert_eq!(
        actor
            .creature
            .unit()
            .subsystems()
            .spells
            .remaining_cooldown_ms(133, 0, 500),
        1500
    );
    assert_eq!(
        actor
            .creature
            .unit()
            .current_spell(wow_entities::CurrentSpellSlot::Generic)
            .unwrap()
            .spell_id,
        133
    );
    actor.assert_actor_storage_runtime(&mut rng, true);
}

#[test]
fn lower_source_revision_keeps_canonical_whole_inside_the_same_outer_motor() {
    let (mut map, mut source, guid, mut rng) = fixtures::pair(803, false);
    let canonical = map.get_typed_creature_mut(guid).unwrap();
    canonical.unit_mut().set_health(60);
    canonical.set_respawn_delay(91);
    canonical
        .unit_mut()
        .subsystems_mut()
        .auras
        .apply_transform_aura_like_cpp(222, false, None);
    canonical
        .unit_mut()
        .subsystems_mut()
        .spells
        .set_cooldown(456, 500, 2000);
    let revision = canonical.unit().health_state_revision_like_cpp();
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source)
            .unwrap()
            .canonical_inner_winners,
        1
    );
    let actor = map.creature_actor_mut(guid).unwrap();
    assert_eq!(actor.creature.current_health(), 60);
    assert_eq!(actor.creature.respawn_delay(), 91);
    assert_eq!(
        actor
            .creature
            .unit()
            .subsystems()
            .auras
            .transform_spell_like_cpp(),
        222
    );
    assert_eq!(
        actor
            .creature
            .unit()
            .subsystems()
            .spells
            .remaining_cooldown_ms(456, 0, 500),
        2000
    );
    assert!(
        actor
            .creature
            .unit()
            .current_spell(wow_entities::CurrentSpellSlot::Generic)
            .is_none()
    );
    assert_eq!(
        actor.creature.unit().health_state_revision_like_cpp(),
        revision
    );
    assert_eq!(
        (actor.create_data.npc_flags, actor.create_data.scale),
        (0x1234, 1.75)
    );
    actor.assert_actor_storage_runtime(&mut rng, false);
}

#[test]
fn lower_source_revision_cannot_win_after_health_aba_restores_the_same_tuple() {
    let (mut map, mut source, guid, mut rng) = fixtures::pair(804, true);
    let canonical = map.get_typed_creature_mut(guid).unwrap();
    canonical.unit_mut().set_health(10);
    canonical.unit_mut().set_health(75);
    canonical.set_respawn_delay(94);
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source)
            .unwrap()
            .canonical_inner_winners,
        1
    );
    let actor = map.creature_actor_mut(guid).unwrap();
    assert_eq!(actor.creature.current_health(), 75);
    assert_eq!(actor.creature.respawn_delay(), 94);
    actor.assert_actor_storage_runtime(&mut rng, true);
}

#[test]
fn equal_revision_disagreeing_health_max_or_death_keeps_canonical_whole() {
    for mismatch in 0..3 {
        let (mut map, mut source, guid, mut rng) = fixtures::pair(805, false);
        let revision = map
            .get_typed_creature(guid)
            .unwrap()
            .unit()
            .health_state_revision_like_cpp();
        let incoming = source_actor_mut(&mut source, guid);
        match mismatch {
            0 => {
                incoming.creature.unit_mut().set_health(60);
            }
            1 => {
                incoming.creature.unit_mut().set_max_health(200);
            }
            _ => {
                incoming
                    .creature
                    .unit_mut()
                    .set_death_state(DeathState::Corpse);
            }
        }
        // Deliberately malformed mirror fixture: simulate a transported equal
        // revision with a conflicting tuple, without changing production gates.
        incoming
            .creature
            .unit_mut()
            .adopt_committed_health_state_revision_for_mirror_like_cpp(revision);
        assert_eq!(
            map.transport_legacy_creature_ownership(&mut source)
                .unwrap()
                .canonical_inner_winners,
            1
        );
        let actor = map.creature_actor_mut(guid).unwrap();
        assert_eq!(
            (
                actor.creature.current_health(),
                actor.creature.unit().data().max_health,
                actor.creature.unit().death_state()
            ),
            (75, 100, DeathState::Alive)
        );
        assert_ne!(actor.creature.respawn_delay(), 73);
        actor.assert_actor_storage_runtime(&mut rng, false);
    }
}

#[test]
fn ownership_promotion_preserves_existing_cells_active_formation_and_move_memberships() {
    let (mut map, mut source, guid, _) = fixtures::pair(806, true);
    map.active_non_players_like_cpp.insert(guid);
    map.creature_group_holder_like_cpp
        .entry(7000)
        .or_default()
        .insert(guid);
    let grid = crate::GridCoord::new(
        crate::Cell::from_world(1.0, 2.0).grid_x(),
        crate::Cell::from_world(1.0, 2.0).grid_y(),
    );
    let cell = crate::Cell::from_world(1.0, 2.0);
    let current_cell = source_actor(&source, guid)
        .creature
        .unit()
        .world()
        .current_cell();
    map.transport_legacy_creature_ownership(&mut source)
        .unwrap();
    assert!(map.active_non_players_like_cpp.contains(&guid));
    assert!(map.creature_group_holder_like_cpp[&7000].contains(&guid));
    assert!(
        map.get_ngrid(grid)
            .unwrap()
            .get_grid_type(cell.cell_x(), cell.cell_y())
            .unwrap()
            .grid_objects
            .creatures
            .contains(&guid)
    );
    assert_eq!(
        map.get_typed_creature(guid)
            .unwrap()
            .unit()
            .world()
            .current_cell(),
        current_cell
    );
    assert!(map.creatures_to_move.is_empty());
}

#[test]
fn shared_active_loot_is_moved_without_retirement_detach_or_generation_change() {
    let (mut map, mut source, guid, _) = fixtures::pair(807, true);
    source_actor_mut(&mut source, guid)
        .creature
        .replace_loot_authority_like_cpp(None, std::collections::HashMap::new());
    let authority = source_actor(&source, guid)
        .creature
        .loot_authority_like_cpp()
        .clone();
    let stamp = authority.stamp_like_cpp();
    assert_eq!(
        stamp.lifecycle,
        wow_entities::OwnedLootAuthorityLifecycle::Active
    );
    assert!(
        authority.shares_storage_like_cpp(
            map.get_typed_creature(guid)
                .unwrap()
                .loot_authority_like_cpp()
        )
    );
    map.transport_legacy_creature_ownership(&mut source)
        .unwrap();
    assert_eq!(authority.stamp_like_cpp(), stamp);
    assert!(
        authority.shares_storage_like_cpp(
            map.get_typed_creature(guid)
                .unwrap()
                .loot_authority_like_cpp()
        )
    );
}

#[test]
fn spawn_zero_moves_the_actor_without_inventing_a_spawn_index_membership() {
    let (mut map, mut source, guid, _) = fixtures::pair(808, false);
    source_actor_mut(&mut source, guid).creature.set_spawn_id(0);
    map.get_typed_creature_mut(guid).unwrap().set_spawn_id(0);
    map.creatures_by_spawn_id.remove(&8080);
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source)
            .unwrap()
            .transported,
        1
    );
    assert_eq!(map.creature_actor(guid).unwrap().creature.spawn_id(), 0);
    assert!(map.creatures_by_spawn_id.is_empty());
}
