use super::support::*;
use wow_world::test_fixtures::loot::*;
use std::sync::Arc;

#[tokio::test]
async fn original_shared_generation_installs_on_the_same_target_and_keeps_root_outcome() {
    let mut f = fixture(4100, 700, true);
    store(&mut f.session, 700);
    let identity = (f.prepared.pending().token().key(), f.prepared.pending().token().incarnation(),
        f.prepared.pending().token().effective_diff_ms());
    let result = f.session.consume_melee_loot(&f.tick, f.prepared).await;
    let pending = pending(result);
    retained(&pending, f.root);
    assert_eq!((pending.token().key(), pending.token().incarnation(),
        pending.token().effective_diff_ms()), identity);
    let snapshot = f.authority.snapshot_for_player_like_cpp(f.player).unwrap();
    assert_eq!(snapshot.loot.coins, 7);
    assert_eq!(snapshot.loot.items.len(), 1);
    assert_eq!(snapshot.loot.items[0].item_id, 80101);
    assert_eq!(snapshot.loot.allowed_looters, vec![f.player]);
    assert_ne!(snapshot.loot.loot_guid, represented_loot_object_guid_for_test(f.victim));
    let cached = loot_for_test(&f.session, f.victim).unwrap();
    assert_eq!(cached.loot_guid, snapshot.loot.loot_guid);
    assert_eq!(cached.coins, snapshot.loot.coins);
    assert_eq!(cached.items[0].item_id, snapshot.loot.items[0].item_id);
    let maps = f.manager.lock().unwrap();
    assert_eq!(maps.find_map(1, 0).unwrap().map().get_typed_creature(f.victim).unwrap()
        .unit().data().health, 0);
}

#[tokio::test]
async fn overworld_personal_generation_restores_the_actual_authority_cache_scope() {
    let mut f = fixture(4120, 701, true);
    map_kind(&mut f.session, false);
    store(&mut f.session, 701);
    let pending = pending(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    retained(&pending, f.root);
    let snapshot = f.authority.snapshot_for_player_like_cpp(f.player).unwrap();
    assert_eq!(snapshot.scope, wow_entities::OwnedLootScope::Personal(f.player));
    assert_eq!(snapshot.loot.allowed_looters, vec![f.player]);
    assert_eq!(snapshot.loot.coins, 7);
    assert_eq!(loot_for_test(&f.session, f.victim).unwrap().loot_guid, snapshot.loot.loot_guid);
}

#[tokio::test]
async fn dungeon_trash_generates_one_real_selected_looter_pool() {
    let mut f = fixture(4140, 702, true);
    map_kind(&mut f.session, true);
    store(&mut f.session, 702);
    let pending = pending(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    retained(&pending, f.root);
    let snapshot = f.authority.snapshot_for_player_like_cpp(f.player).unwrap();
    assert_eq!(snapshot.scope, wow_entities::OwnedLootScope::Personal(f.player));
    assert_eq!(snapshot.loot.dungeon_encounter_id, 0);
    assert_eq!(snapshot.loot.items.len(), 1);
}

#[tokio::test]
async fn no_tapper_keeps_original_pending_without_query_or_guid_allocation() {
    let mut f = fixture(4160, 703, false);
    let before = next_map_loot_guid_for_test(&f.session, 1, 0).unwrap();
    let pending = pending(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    retained(&pending, f.root);
    assert_eq!(next_map_loot_guid_for_test(&f.session, 1, 0).unwrap(), before);
    assert!(f.authority.snapshot_for_player_like_cpp(f.player).is_none());
    assert!(!has_loot_for_test(&f.session, f.victim));
}

#[tokio::test]
async fn zero_loot_id_generates_money_without_item_catalog_io() {
    let mut f = fixture(4180, 0, true);
    let port = Arc::new(super::io::CatalogPort::new(f.manager.clone(), f.victim,
        super::io::Mutation::None));
    f.session.set_loot_template_catalog_persistence_port_like_cpp(port.clone());
    let pending = pending(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    retained(&pending, f.root);
    assert_eq!(port.calls(), 0);
    let snapshot = f.authority.snapshot_for_player_like_cpp(f.player).unwrap();
    assert_eq!(snapshot.loot.coins, 7);
    assert!(snapshot.loot.items.is_empty());
}

#[tokio::test]
async fn missing_item_store_keeps_the_existing_empty_items_fallback() {
    let mut f = fixture(4200, 704, true);
    let pending = pending(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    retained(&pending, f.root);
    let snapshot = f.authority.snapshot_for_player_like_cpp(f.player).unwrap();
    assert_eq!(snapshot.loot.coins, 7);
    assert!(snapshot.loot.items.is_empty());
    assert!(has_loot_for_test(&f.session, f.victim));
}

#[tokio::test]
async fn session_lookup_change_does_not_redirect_the_original_map_loot_counter() {
    let mut f = fixture(4220, 0, true);
    f.session.fixture_melee_set_map_position(2, wow_core::Position::ZERO);
    f.manager.lock().unwrap().create_world_map(2, 0);
    let before_original = next_map_loot_guid_for_test(&f.session, 1, 0).unwrap();
    let before_other = next_map_loot_guid_for_test(&f.session, 2, 0).unwrap();
    let pending = pending(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    retained(&pending, f.root);
    let snapshot = f.authority.snapshot_for_player_like_cpp(f.player).unwrap();
    assert_eq!(snapshot.loot.loot_guid.map_id(), 1);
    assert_eq!(next_map_loot_guid_for_test(&f.session, 1, 0).unwrap(), before_original + 1);
    assert_eq!(next_map_loot_guid_for_test(&f.session, 2, 0).unwrap(), before_other);
}

#[tokio::test]
async fn original_route_does_not_install_a_prepopulated_pristine_cache_fixture() {
    let mut f = fixture(4240, 0, true);
    insert_allowed_coin_loot_for_test(&mut f.session, f.victim, f.player, 999);
    let pending = pending(f.session.consume_melee_loot(&f.tick, f.prepared).await);
    retained(&pending, f.root);
    assert_eq!(f.authority.snapshot_for_player_like_cpp(f.player).unwrap().loot.coins, 7);
    assert_eq!(loot_for_test(&f.session, f.victim).unwrap().coins, 7);
}
