//! Canonical loot lifecycle fixtures retain the real authority and observation tokens.
use super::*;

pub fn mutate_loot_creature_for_test<R>(session: &mut WorldSession, owner: ObjectGuid, mutate: impl FnOnce(&mut crate::map_manager::WorldCreature) -> R) -> Option<R> {
    session.mutate_world_creature(owner, mutate)
}

pub async fn ensure_creature_kill_loot_for_test(session: &mut WorldSession, owner: ObjectGuid) {
    session.ensure_represented_creature_kill_loot_like_cpp(owner).await;
}

pub fn creature_loot_revision_for_test(session: &mut WorldSession, owner: ObjectGuid) -> Option<u64> {
    session.represented_creature_loot_state_like_cpp(owner).map(|state| state.loot_lifecycle_revision())
}

pub fn install_creature_kill_loot_for_test(session: &mut WorldSession, owner: ObjectGuid, authority: &OwnedLootAuthority, generation: u64, revision: u64, shared: Option<CreatureLoot>, personal: HashMap<ObjectGuid, CreatureLoot>) -> bool {
    session.install_represented_creature_kill_loot_if_current_like_cpp(owner, authority, generation, revision, shared, personal)
}

pub fn bind_loot_view_for_test(session: &mut WorldSession, owner: ObjectGuid, generation: u64, authority: &OwnedLootAuthority) {
    session.loot_views.bind_opened(owner, generation, authority);
}

pub fn has_cached_loot_generation_for_test(session: &WorldSession, owner: ObjectGuid) -> bool {
    session.represented_loot_cache_generations_like_cpp.contains_key(&owner)
}

pub fn personal_loot_money_entry_for_test(session: &WorldSession, owner: ObjectGuid, player: ObjectGuid) -> Option<&u32> {
    session.represented_personal_loot_money.get(&(owner, player))
}

pub fn update_loot_gameobject_for_test(session: &WorldSession, owner: ObjectGuid, diff: u32, now: i64) {
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    let mut manager = manager.lock().unwrap();
    manager.find_map_mut(u32::from(session.player_map_id_like_cpp()), 0).unwrap().map_mut().update_game_object_like_cpp(owner, diff, now);
}

pub async fn open_loot_item_window_for_test(session: &mut WorldSession, player: ObjectGuid, item: ObjectGuid) {
    session.open_active_item_loot_view_like_cpp(player, item).await;
}

pub struct GameObjectLootWitness(RepresentedGameObjectLootInstallObservationLikeCpp);

pub fn observe_gameobject_loot_for_test(session: &mut WorldSession, owner: ObjectGuid) -> Option<GameObjectLootWitness> {
    session.represented_gameobject_loot_install_observation_like_cpp(owner).map(GameObjectLootWitness)
}

pub fn upsert_gameobject_pool_for_test(session: &mut WorldSession, owner: ObjectGuid, player: ObjectGuid, loot: CreatureLoot, replace: bool) -> Option<()> {
    session.upsert_represented_personal_gameobject_loot_authority_like_cpp(owner, player, loot, replace)
}

pub fn upsert_observed_gameobject_pool_for_test(session: &mut WorldSession, owner: ObjectGuid, player: ObjectGuid, loot: CreatureLoot, replace: bool, discard_empty: bool, witness: &GameObjectLootWitness) -> Option<()> {
    session.upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(owner, player, loot, replace, discard_empty, &witness.0)
}

#[allow(clippy::too_many_arguments)]
pub fn release_gameobject_observation_for_test(session: &mut WorldSession, owner: ObjectGuid, authority: &OwnedLootAuthority, generation: u64, revision: u64, state: LootState, player: Option<ObjectGuid>, restock: u32, changed: bool) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
    session.set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(owner, authority, generation, revision, state, player, restock, changed)
}

pub fn mutate_loot_gameobject_for_test<R>(session: &mut WorldSession, owner: ObjectGuid, mutate: impl FnOnce(&mut wow_entities::GameObject) -> R) -> Option<R> {
    session.mutate_canonical_gameobject_by_guid_like_cpp(owner, mutate)
}

pub fn release_fishing_hole_for_test(session: &mut WorldSession, owner: ObjectGuid, max_opens: Option<u32>) -> Option<(u32, LootState, wow_map::map::GameObjectSetLootStateOutcomeLikeCpp)> {
    session.release_canonical_fishing_hole_like_cpp(owner, max_opens)
}

pub async fn release_loot_owner_for_test(session: &mut WorldSession, owner: ObjectGuid, player: ObjectGuid) -> bool {
    session.do_loot_release_owner_like_cpp(owner, player).await
}

pub fn close_retired_loot_views_for_test(session: &mut WorldSession, player: ObjectGuid) {
    session.close_retired_active_loot_windows_like_cpp(player);
}

pub fn refresh_loot_summary_for_test(session: &mut WorldSession, owner: ObjectGuid, player: ObjectGuid) {
    session.refresh_represented_loot_owner_canonical_summary_like_cpp(owner, player);
}

pub fn loot_cache_mut_for_test(session: &mut WorldSession, owner: ObjectGuid) -> Option<&mut CreatureLoot> {
    session.loot_table.get_mut(&owner)
}

pub fn share_loot_canonical_map_for_test(source: &WorldSession, target: &mut WorldSession) {
    target.set_canonical_map_manager(Arc::clone(source.canonical_map_manager.as_ref().unwrap()));
}

pub fn legacy_loot_map_key_for_test(session: &WorldSession) -> (u16, u32) {
    session.current_legacy_runtime_map_key_like_cpp()
}

pub fn canonical_loot_player_map_key_for_test(session: &WorldSession) -> Option<wow_map::MapKey> {
    session.current_canonical_player_map_key_like_cpp()
}

pub fn legacy_creature_loot_authority_for_test(session: &WorldSession, owner: ObjectGuid, key: wow_map::MapKey) -> Option<OwnedLootAuthority> {
    session.read_legacy_creature_loot_authority_on_map_like_cpp(owner, key)
}

pub fn canonical_creature_loot_authority_for_test(session: &WorldSession, owner: ObjectGuid, key: wow_map::MapKey) -> Option<OwnedLootAuthority> {
    session.read_canonical_creature_loot_authority_on_map_like_cpp(owner, key)
}
