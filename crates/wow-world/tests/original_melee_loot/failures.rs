use super::support::*;
use super::io::{CatalogPort, Mutation};
use std::sync::Arc;
use wow_map::manager::{MeleeLootError, MeleeKillPhaseError, ActorTickAccessError};
use wow_world::test_fixtures::loot::*;

#[tokio::test]
async fn condition_store_io_generates_real_items_before_original_guid_allocation() {
    let mut f = fixture(4300, 710, true);
    store(&mut f.session, 710);
    let port = Arc::new(CatalogPort::new(f.manager.clone(), f.victim, Mutation::None));
    f.session.set_loot_template_catalog_persistence_port_like_cpp(port.clone());
    let before = next_map_loot_guid_for_test(&f.session, 1, 0).unwrap();
    let pending = pending(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    assert!(port.calls() > 0);
    assert_eq!(next_map_loot_guid_for_test(&f.session, 1, 0).unwrap(), before + 1);
    assert_eq!(f.authority.snapshot_for_player_like_cpp(f.player).unwrap().loot.items.len(), 1);
    retained(&pending, f.root);
}

#[tokio::test]
async fn health_change_during_real_catalog_io_rejects_before_the_guid_counter() {
    let mut f = fixture(4320, 711, true);
    store(&mut f.session, 711);
    let port = Arc::new(CatalogPort::new(f.manager.clone(), f.victim, Mutation::Health));
    f.session.set_loot_template_catalog_persistence_port_like_cpp(port.clone());
    let before = next_map_loot_guid_for_test(&f.session, 1, 0).unwrap();
    let (error, pending) = rejected(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    assert!(matches!(error, MeleeLootError::Phase(MeleeKillPhaseError::HealthRevisionConflict { guid, .. })
        if guid == f.victim));
    assert!(port.calls() > 0);
    assert_eq!(next_map_loot_guid_for_test(&f.session, 1, 0).unwrap(), before);
    assert!(f.authority.snapshot_for_player_like_cpp(f.player).is_none());
    retained(&pending, f.root);
}

#[tokio::test]
async fn changed_loot_generation_during_io_returns_a_typed_error_and_original_pending() {
    let mut f = fixture(4330, 715, true);
    store(&mut f.session, 715);
    let port = Arc::new(CatalogPort::new(f.manager.clone(), f.victim, Mutation::Generation));
    f.session.set_loot_template_catalog_persistence_port_like_cpp(port);
    let before = next_map_loot_guid_for_test(&f.session, 1, 0).unwrap();
    let (error, pending) = rejected(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    assert!(matches!(error, MeleeLootError::Phase(MeleeKillPhaseError::GenerationConflict { guid, .. })
        if guid == f.victim));
    assert_eq!(next_map_loot_guid_for_test(&f.session, 1, 0).unwrap(), before);
    assert!(f.authority.snapshot_for_player_like_cpp(f.player).is_none());
    retained(&pending, f.root);
}

#[tokio::test]
async fn target_readmission_during_io_never_installs_on_the_same_guid_replacement() {
    let mut f = fixture(4340, 712, true);
    store(&mut f.session, 712);
    let port = Arc::new(CatalogPort::new(f.manager.clone(), f.victim, Mutation::TargetAba));
    f.session.set_loot_template_catalog_persistence_port_like_cpp(port);
    let (error, pending) = rejected(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    assert!(matches!(error, MeleeLootError::Phase(MeleeKillPhaseError::TargetActorChanged { guid })
        if guid == f.victim));
    let manager = f.manager.lock().unwrap();
    assert_eq!(manager.find_map(1, 0).unwrap().map().get_typed_creature(f.victim).unwrap()
        .unit().data().health, 75);
    assert!(f.authority.snapshot_for_player_like_cpp(f.player).is_none());
    retained(&pending, f.root);
}

#[tokio::test]
async fn root_readmission_during_io_fails_the_original_slot_before_target_facts() {
    let mut f = fixture(4360, 713, true);
    store(&mut f.session, 713);
    let port = Arc::new(CatalogPort::new(f.manager.clone(), f.victim, Mutation::RootAba(f.root)));
    f.session.set_loot_template_catalog_persistence_port_like_cpp(port);
    let (error, pending) = rejected(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    assert!(matches!(error, MeleeLootError::Phase(MeleeKillPhaseError::RootStale(
        ActorTickAccessError::WitnessMismatch { guid })) if guid == f.root));
    assert!(f.authority.snapshot_for_player_like_cpp(f.player).is_none());
    retained(&pending, f.root);
}

#[tokio::test]
async fn map_replacement_during_io_retains_original_token_and_full_partial_damage() {
    let mut f = fixture(4380, 714, true);
    store(&mut f.session, 714);
    let incarnation = f.prepared.pending().token().incarnation();
    let port = Arc::new(CatalogPort::new(f.manager.clone(), f.victim, Mutation::Map));
    f.session.set_loot_template_catalog_persistence_port_like_cpp(port);
    let (error, pending) = rejected(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    assert!(matches!(error, MeleeLootError::Phase(MeleeKillPhaseError::RootStale(
        ActorTickAccessError::StaleParticipant { admitted_incarnation, .. }))
        if admitted_incarnation == incarnation));
    assert_eq!(pending.token().incarnation(), incarnation);
    assert!(f.authority.snapshot_for_player_like_cpp(f.player).is_none());
    retained(&pending, f.root);
}

#[tokio::test]
async fn different_manager_returns_original_owner_without_test_guid_fallback() {
    let mut f = fixture(4400, 0, true);
    let incarnation = f.prepared.pending().token().incarnation();
    f.session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(wow_map::MapManager::default())));
    let (error, pending) = rejected(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    assert!(matches!(error, MeleeLootError::Phase(MeleeKillPhaseError::RootStale(_))));
    assert_eq!(pending.token().incarnation(), incarnation);
    assert!(f.authority.snapshot_for_player_like_cpp(f.player).is_none());
    retained(&pending, f.root);
}
