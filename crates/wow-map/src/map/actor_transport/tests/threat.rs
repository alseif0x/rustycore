use super::*;
use wow_entities::Player;

fn player(map: &mut Map, counter: i64) -> ObjectGuid {
    let guid = ObjectGuid::create_player(1, counter);
    let mut player = Player::new(None, false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_map(571, 7).unwrap();
    player.unit_mut().world_mut().object_mut().add_to_world();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    guid
}

fn threatened_player_pair(
    map: &mut Map,
    source: &mut MapInstance,
    guid: ObjectGuid,
) -> (ObjectGuid, ObjectGuid) {
    let removed = player(map, 901);
    let added = player(map, 902);
    let canonical = map.get_typed_creature_mut(guid).unwrap();
    canonical
        .unit_mut()
        .subsystems_mut()
        .combat
        .initialize_threat_list_capability(true);
    canonical
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_threat(removed, 17.0);
    let old_ref = canonical
        .unit()
        .subsystems()
        .combat
        .threat_ref(removed)
        .copied()
        .unwrap();
    map.get_typed_player_mut(removed)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .put_threatened_by_me_ref(guid, old_ref);
    let actor = source_actor_mut(source, guid);
    actor
        .creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .initialize_threat_list_capability(true);
    actor
        .creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_threat(added, 31.0);
    (removed, added)
}

#[test]
fn source_winner_adds_current_reciprocal_references_then_purges_removed_targets() {
    let (mut map, mut source, guid, mut rng) = fixtures::pair(821, true);
    let (removed, added) = threatened_player_pair(&mut map, &mut source, guid);
    map.transport_legacy_creature_ownership(&mut source)
        .unwrap();
    assert!(
        map.get_typed_player(added)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threatened_by_me_owner_guids()
            .contains(&guid)
    );
    assert!(
        !map.get_typed_player(removed)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threatened_by_me_owner_guids()
            .contains(&guid)
    );
    assert_eq!(
        map.get_typed_creature(guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threat_value(added),
        Some(31.0)
    );
    map.creature_actor_mut(guid)
        .unwrap()
        .assert_actor_storage_runtime(&mut rng, true);
}

#[test]
fn canonical_winner_does_not_apply_incoming_reciprocal_threat_writes() {
    let (mut map, mut source, guid, _) = fixtures::pair(822, false);
    let (removed, added) = threatened_player_pair(&mut map, &mut source, guid);
    map.get_typed_creature_mut(guid)
        .unwrap()
        .unit_mut()
        .set_health(70);
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source)
            .unwrap()
            .canonical_inner_winners,
        1
    );
    assert!(
        !map.get_typed_player(added)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threatened_by_me_owner_guids()
            .contains(&guid)
    );
    assert!(
        map.get_typed_player(removed)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threatened_by_me_owner_guids()
            .contains(&guid)
    );
    assert_eq!(
        map.get_typed_creature(guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threat_value(removed),
        Some(17.0)
    );
}

#[test]
fn complete_map_moves_every_motor_and_keeps_player_links_and_spawn_cardinality() {
    let (mut map, mut source, first, mut first_rng) = fixtures::pair(823, true);
    let (second, mut second_rng) = fixtures::add_pair(&mut map, &mut source, 824, false);
    let player_guid = player(&mut map, 903);
    let links = map.map_reference_order_like_cpp().to_vec();
    assert_eq!(links, vec![player_guid]);
    let summary = map
        .transport_legacy_creature_ownership(&mut source)
        .unwrap();
    assert_eq!(summary.transported, 2);
    assert_eq!(summary.source_inner_winners, 2);
    assert_eq!(source_count(&source), 0);
    assert_eq!(map.map_object_count(), 3);
    assert_eq!(map.map_reference_order_like_cpp(), links);
    assert_eq!(
        map.creature_spawn_id_store_guids_like_cpp(8230),
        vec![first]
    );
    assert_eq!(
        map.creature_spawn_id_store_guids_like_cpp(8240),
        vec![second]
    );
    map.creature_actor_mut(first)
        .unwrap()
        .assert_actor_storage_runtime(&mut first_rng, true);
    map.creature_actor_mut(second)
        .unwrap()
        .assert_actor_storage_runtime(&mut second_rng, false);
}

#[test]
fn reciprocal_creature_targets_keep_add_and_purge_across_their_own_canonical_winning_moves() {
    let (mut map, mut source, guid, _) = fixtures::pair(825, true);
    let (removed, _) = fixtures::add_pair(&mut map, &mut source, 826, false);
    let (added, _) = fixtures::add_pair(&mut map, &mut source, 829, true);
    let canonical = map.get_typed_creature_mut(guid).unwrap();
    canonical
        .unit_mut()
        .subsystems_mut()
        .combat
        .initialize_threat_list_capability(true);
    canonical
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_threat(removed, 17.0);
    let old_ref = canonical
        .unit()
        .subsystems()
        .combat
        .threat_ref(removed)
        .copied()
        .unwrap();
    map.get_typed_creature_mut(removed)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .put_threatened_by_me_ref(guid, old_ref);
    for target in [removed, added] {
        // Later transport cannot overwrite an earlier reciprocal update with
        // a stale source inner: each secondary intentionally keeps canonical.
        map.get_typed_creature_mut(target)
            .unwrap()
            .unit_mut()
            .set_health(60);
    }
    let incoming = source_actor_mut(&mut source, guid);
    incoming
        .creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .initialize_threat_list_capability(true);
    incoming
        .creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_threat(added, 31.0);
    let summary = map
        .transport_legacy_creature_ownership(&mut source)
        .unwrap();
    assert_eq!(
        (
            summary.transported,
            summary.source_inner_winners,
            summary.canonical_inner_winners
        ),
        (3, 1, 2)
    );
    assert!(
        map.get_typed_creature(added)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threatened_by_me_owner_guids()
            .contains(&guid)
    );
    assert!(
        !map.get_typed_creature(removed)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threatened_by_me_owner_guids()
            .contains(&guid)
    );
    assert_eq!(source_count(&source), 0);
}

#[test]
fn multiple_guids_for_one_spawn_keep_the_original_multimap_cardinality() {
    let (mut map, mut source, first, _) = fixtures::pair(832, true);
    let (second, _) = fixtures::add_pair(&mut map, &mut source, 834, false);
    source_actor_mut(&mut source, second)
        .creature
        .set_spawn_id(8320);
    map.get_typed_creature_mut(second)
        .unwrap()
        .set_spawn_id(8320);
    map.creatures_by_spawn_id.remove(&8340);
    map.creatures_by_spawn_id
        .entry(8320)
        .or_default()
        .insert(second);
    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source)
            .unwrap()
            .transported,
        2
    );
    let mut expected = vec![first, second];
    expected.sort();
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(8320), expected);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(8320), 2);
    assert_eq!(source_count(&source), 0);
}

fn add_source_threat(source: &mut MapInstance, owner: ObjectGuid, target: ObjectGuid, amount: f32) {
    let combat = &mut source_actor_mut(source, owner)
        .creature
        .unit_mut()
        .subsystems_mut()
        .combat;
    combat.initialize_threat_list_capability(true);
    combat.add_threat(target, amount);
}

fn seed_old_reciprocal(
    map: &mut Map,
    source: &mut MapInstance,
    owner: ObjectGuid,
    target: ObjectGuid,
) {
    let combat = &mut map
        .get_typed_creature_mut(owner)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat;
    combat.initialize_threat_list_capability(true);
    combat.add_threat(target, 17.0);
    let reference = combat.threat_ref(target).copied().unwrap();
    map.get_typed_creature_mut(target)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .combat
        .put_threatened_by_me_ref(owner, reference);
    source_actor_mut(source, target)
        .creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .put_threatened_by_me_ref(owner, reference);
}

fn has_reciprocal(map: &Map, target: ObjectGuid, owner: ObjectGuid) -> bool {
    map.get_typed_creature(target)
        .unwrap()
        .unit()
        .subsystems()
        .combat
        .threatened_by_me_owner_guids()
        .contains(&owner)
}

#[test]
fn source_targets_promoted_after_the_owner_keep_both_add_and_purge() {
    let (mut map, mut source, _, _) = fixtures::pair(851, true);
    fixtures::add_pair(&mut map, &mut source, 852, false);
    fixtures::add_pair(&mut map, &mut source, 853, true);
    // Choose roles from the actual HashMap traversal, so both targets are
    // later SOURCE winners regardless of its randomized iteration order.
    let order: Vec<_> = source
        .grids
        .values()
        .flat_map(|grid| grid.creatures.keys().copied())
        .collect();
    let (owner, removed, added) = (order[0], order[1], order[2]);
    seed_old_reciprocal(&mut map, &mut source, owner, removed);
    add_source_threat(&mut source, owner, added, 31.0);
    let planned = map.preflight_creature_transport(&source).unwrap();
    assert_eq!(
        planned.iter().map(|entry| entry.guid).collect::<Vec<_>>(),
        order
    );
    assert!(planned.iter().all(|entry| entry.source_wins));
    drop(planned);

    let summary = map
        .transport_legacy_creature_ownership(&mut source)
        .unwrap();
    assert_eq!(
        (
            summary.transported,
            summary.source_inner_winners,
            summary.canonical_inner_winners
        ),
        (3, 3, 0)
    );
    assert!(has_reciprocal(&map, added, owner));
    assert!(!has_reciprocal(&map, removed, owner));
    assert_eq!(
        map.get_typed_creature(owner)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threat_value(added),
        Some(31.0)
    );
    assert_eq!(source_count(&source), 0);
}

#[test]
fn reciprocal_source_peers_survive_either_hashmap_promotion_order() {
    let (mut map, mut source, first, _) = fixtures::pair(854, true);
    let (second, _) = fixtures::add_pair(&mut map, &mut source, 855, false);
    add_source_threat(&mut source, first, second, 31.0);
    add_source_threat(&mut source, second, first, 47.0);

    let summary = map
        .transport_legacy_creature_ownership(&mut source)
        .unwrap();
    assert_eq!(summary.source_inner_winners, 2);
    assert_eq!(summary.canonical_inner_winners, 0);
    assert!(has_reciprocal(&map, first, second));
    assert!(has_reciprocal(&map, second, first));
    assert_eq!(
        map.get_typed_creature(first)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threat_value(second),
        Some(31.0)
    );
    assert_eq!(
        map.get_typed_creature(second)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threat_value(first),
        Some(47.0)
    );
}

#[test]
fn source_self_targets_add_current_reference_and_purge_old_reference() {
    let (mut map, mut source, added, _) = fixtures::pair(856, true);
    let (removed, _) = fixtures::add_pair(&mut map, &mut source, 857, false);
    add_source_threat(&mut source, added, added, 23.0);
    seed_old_reciprocal(&mut map, &mut source, removed, removed);

    assert_eq!(
        map.transport_legacy_creature_ownership(&mut source)
            .unwrap()
            .source_inner_winners,
        2
    );
    assert!(has_reciprocal(&map, added, added));
    assert!(!has_reciprocal(&map, removed, removed));
    assert_eq!(
        map.get_typed_creature(added)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threat_value(added),
        Some(23.0)
    );
    assert_eq!(
        map.get_typed_creature(removed)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threat_value(removed),
        None
    );
}

#[test]
fn mixed_winners_publish_only_source_deltas_to_all_promoted_targets() {
    let (mut map, mut source, owner, _) = fixtures::pair(858, true);
    let (source_target, _) = fixtures::add_pair(&mut map, &mut source, 859, false);
    let (canonical_target, _) = fixtures::add_pair(&mut map, &mut source, 860, true);
    let ignored_target = player(&mut map, 904);
    seed_old_reciprocal(&mut map, &mut source, owner, canonical_target);
    add_source_threat(&mut source, owner, source_target, 31.0);
    add_source_threat(&mut source, canonical_target, ignored_target, 99.0);
    map.get_typed_creature_mut(canonical_target)
        .unwrap()
        .unit_mut()
        .set_health(60);

    let summary = map
        .transport_legacy_creature_ownership(&mut source)
        .unwrap();
    assert_eq!(
        (
            summary.source_inner_winners,
            summary.canonical_inner_winners
        ),
        (2, 1)
    );
    assert!(has_reciprocal(&map, source_target, owner));
    assert!(!has_reciprocal(&map, canonical_target, owner));
    assert!(
        !map.get_typed_player(ignored_target)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threatened_by_me_owner_guids()
            .contains(&canonical_target)
    );
    assert_eq!(
        map.get_typed_creature(canonical_target)
            .unwrap()
            .unit()
            .data()
            .health,
        60
    );
    assert_eq!(
        map.get_typed_creature(canonical_target)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threat_value(ignored_target),
        None
    );
}
