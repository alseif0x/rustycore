use super::support::*;
use super::io::{CatalogPort, Mutation};
use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_map::manager::{MeleeLootError, MeleeKillPhaseError};
use wow_world::test_fixtures::loot::*;
use wow_world::session::directory::PlayerRegistry;

fn companion(f: &mut Fixture) -> ObjectGuid {
    let other = ObjectGuid::create_player(f.player.realm_id(), f.player.counter() + 1);
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (send, _) = flume::bounded(1);
    let mut registration = player_registration_for_loot_test(other, send);
    registration.placement.map_id = 1;
    registry.register_or_replace(other, registration, Default::default());
    f.session.set_player_registry(registry);
    other
}

#[tokio::test]
async fn missing_boss_lockout_authority_keeps_the_existing_fail_closed_pool_filter() {
    let mut f = fixture_with(4600, 720, true, 777, false);
    map_kind(&mut f.session, true);
    let port = Arc::new(CatalogPort::new(f.manager.clone(), f.victim, Mutation::None));
    f.session.set_loot_template_catalog_persistence_port_like_cpp(port.clone());
    let before = next_map_loot_guid_for_test(&f.session, 1, 0).unwrap();
    let pending = pending(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    retained(&pending, f.root);
    assert_eq!(port.calls(), 0);
    assert_eq!(next_map_loot_guid_for_test(&f.session, 1, 0).unwrap(), before);
    assert!(f.authority.snapshot_for_player_like_cpp(f.player).is_none());
}

#[tokio::test]
async fn dungeon_trash_round_robin_changes_only_after_successful_nonempty_install() {
    let mut f = fixture_with(4620, 721, true, 0, true);
    map_kind(&mut f.session, true);
    store(&mut f.session, 721);
    let other = companion(&mut f);
    install_group_loot_group_for_test(&mut f.session, f.player, other);
    let registry = f.session.group_registry().unwrap().clone();
    let group = wow_world::test_fixtures::mutate_canonical_player_for_test(
        &f.session, |player| player.gameplay_state().group.as_ref()
            .map(|group| group.group_guid.counter() as u64)).unwrap().unwrap();
    assert_eq!(registry.get(&group).unwrap().looter_guid_like_cpp(), f.player);
    let pending = pending(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    retained(&pending, f.root);
    assert!(f.authority.snapshot_for_player_like_cpp(f.player).is_some());
    assert!(f.authority.snapshot_for_player_like_cpp(other).is_none());
    assert_eq!(registry.get(&group).unwrap().looter_guid_like_cpp(), other);
}

#[tokio::test]
async fn dungeon_trash_failed_generation_does_not_advance_the_existing_looter() {
    let mut f = fixture_with(4640, 722, true, 0, true);
    map_kind(&mut f.session, true);
    store(&mut f.session, 722);
    let other = companion(&mut f);
    install_group_loot_group_for_test(&mut f.session, f.player, other);
    let registry = f.session.group_registry().unwrap().clone();
    let group = wow_world::test_fixtures::mutate_canonical_player_for_test(
        &f.session, |player| player.gameplay_state().group.as_ref()
            .map(|group| group.group_guid.counter() as u64)).unwrap().unwrap();
    f.session.set_loot_template_catalog_persistence_port_like_cpp(Arc::new(
        CatalogPort::new(f.manager.clone(), f.victim, Mutation::Health)));
    let (error, pending) = rejected(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    assert!(matches!(error, MeleeLootError::Phase(MeleeKillPhaseError::HealthRevisionConflict { .. })));
    assert_eq!(registry.get(&group).unwrap().looter_guid_like_cpp(), f.player);
    assert!(f.authority.snapshot_for_player_like_cpp(f.player).is_none());
    retained(&pending, f.root);
}

#[tokio::test]
async fn second_personal_pool_failure_retains_first_counter_without_installing_partial_pools() {
    let mut f = fixture_with(4660, 723, true, 0, true);
    map_kind(&mut f.session, false);
    store(&mut f.session, 723);
    let other = companion(&mut f);
    let port = Arc::new(CatalogPort::new(f.manager.clone(), f.victim, Mutation::HealthSecond));
    f.session.set_loot_template_catalog_persistence_port_like_cpp(port.clone());
    let before = next_map_loot_guid_for_test(&f.session, 1, 0).unwrap();
    let (error, pending) = rejected(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    assert!(matches!(error, MeleeLootError::Phase(MeleeKillPhaseError::HealthRevisionConflict { .. })));
    assert_eq!(port.calls(), 2);
    assert_eq!(next_map_loot_guid_for_test(&f.session, 1, 0).unwrap(), before + 1);
    assert!(f.authority.snapshot_for_player_like_cpp(f.player).is_none());
    assert!(f.authority.snapshot_for_player_like_cpp(other).is_none());
    retained(&pending, f.root);
}
