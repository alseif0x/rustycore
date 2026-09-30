//! Borrowed fixture observations and forwards for real gameobject loot application operations.
use super::*;
use crate::session::RepresentedGameObjectUseState;

pub struct LootGameObjectState<'a>(&'a RepresentedGameObjectUseState);

impl LootGameObjectState<'_> {
    pub fn go_type(&self) -> Option<u8> { self.0.go_type }
    pub fn loot_state(&self) -> Option<LootState> { self.0.loot_state }
    pub fn loot_state_unit_guid(&self) -> ObjectGuid { self.0.loot_state_unit_guid }
    pub fn go_state(&self) -> Option<GoState> { self.0.go_state }
    pub fn gameobject_flags(&self) -> u32 { self.0.gameobject_flags }
    pub fn dynamic_flags(&self) -> u32 { self.0.dynamic_flags }
    pub fn chest_restock_time_secs(&self) -> Option<u32> { self.0.chest_restock_time_secs }
    pub fn chest_consumable(&self) -> Option<bool> { self.0.chest_consumable }
    pub fn chest_personal_loot_id(&self) -> Option<u32> { self.0.chest_personal_loot_id }
    pub fn linked_trap_entry(&self) -> Option<u32> { self.0.linked_trap_entry }
    pub fn linked_trap_guid(&self) -> Option<ObjectGuid> { self.0.linked_trap_guid }
    pub fn chest_loot_source(&self) -> Option<GameObjectLootSource> { self.0.chest_loot_source }
    pub fn gathering_node_loot_id(&self) -> Option<u32> { self.0.gathering_node_loot_id }
    pub fn personal_loot_uses(&self) -> u32 { self.0.personal_loot_uses }
    pub fn despawn_delay_secs(&self) -> Option<u32> { self.0.despawn_delay_secs }
    pub fn despawn_delay_until(&self) -> Option<&Instant> { self.0.despawn_delay_until.as_ref() }
    pub fn cooldown_until(&self) -> Option<&Instant> { self.0.cooldown_until.as_ref() }
    pub fn has_goober_source(&self) -> bool { self.0.goober_use_source.is_some() }
}

pub fn loot_gameobject_state_for_test(session: &WorldSession, owner: ObjectGuid) -> Option<LootGameObjectState<'_>> {
    session.represented_gameobject_use_states.get(&owner).map(LootGameObjectState)
}

pub async fn process_loot_commands_for_test(session: &mut WorldSession) {
    session.process_represented_session_commands_like_cpp().await;
}

pub fn set_loot_linked_trap_for_test(session: &mut WorldSession, owner: ObjectGuid, entry: u32) {
    session.represented_gameobject_use_states.entry(owner).or_default().linked_trap_entry = Some(entry);
}

pub fn use_loot_goober_for_test(session: &mut WorldSession, owner: ObjectGuid, player: ObjectGuid, quest: u32, source: wow_entities::GooberUseSource) -> bool {
    session.use_represented_gameobject_goober_state_like_cpp(owner, player, quest, source)
}

pub fn record_loot_display_model_for_test(session: &mut WorldSession, owner: ObjectGuid, display: u32, scale: f32, rotation: [f32; 4]) {
    session.record_represented_gameobject_display_model_like_cpp(owner, display, scale, rotation);
}

pub fn record_loot_lock_for_test(session: &mut WorldSession, owner: ObjectGuid, lock: u32) {
    session.record_represented_gameobject_lock_id_like_cpp(owner, lock);
}

pub fn loot_gameobject_can_store_for_test(session: &WorldSession, owner: ObjectGuid, player: ObjectGuid) -> bool {
    session.represented_gameobject_can_autostore_loot_item_like_cpp(owner, player)
}

pub fn chest_reward_looters_for_test(session: &WorldSession, player: ObjectGuid) -> Vec<ObjectGuid> {
    session.represented_group_looters_at_reward_distance_like_cpp(player)
}

pub async fn generate_chest_loot_for_test(session: &mut WorldSession, owner: ObjectGuid, player: ObjectGuid, source: GameObjectLootSource, looters: &[ObjectGuid]) -> Option<CreatureLoot> {
    let money = session.world_query_catalogs_like_cpp()
        .and_then(|catalogs| catalogs.gameobject.get(owner.entry()))
        .map(|row| (row.min_money, row.max_money)).unwrap_or((0, 0));
    session.generate_represented_gameobject_chest_loot_with_template_money_like_cpp(owner, player, source, looters, money).await
}

pub fn set_loot_gameobject_tappers_for_test(session: &mut WorldSession, owner: ObjectGuid, tappers: Vec<ObjectGuid>) {
    session.represented_gameobject_tap_lists.insert(owner, tappers);
}

pub fn has_personal_loot_money_entry_for_test(session: &WorldSession, owner: ObjectGuid, player: ObjectGuid) -> bool {
    session.represented_personal_loot_money.contains_key(&(owner, player))
}

pub fn is_personal_loot_owner_for_test(session: &WorldSession, owner: ObjectGuid) -> bool {
    session.represented_personal_loot_owners.contains(&owner)
}

pub fn apply_cached_gameobject_loot_release_for_test(session: &mut WorldSession, owner: ObjectGuid, player: ObjectGuid, selected_pool_looted: bool, fully_looted: bool) {
    session.apply_represented_gameobject_loot_release_like_cpp(owner, player, selected_pool_looted, fully_looted, None);
}

pub fn set_loot_lock_spells_for_test(session: &mut WorldSession, spells: Vec<i32>) {
    session.set_known_spells_like_cpp(spells);
}
