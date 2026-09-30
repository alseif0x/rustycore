use super::*;

pub fn set_world_creature_corpse_delay_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    delay_secs: u32,
    fully_skinned: bool,
) -> bool {
    session
        .mutate_world_creature(guid, |world_creature| {
            world_creature
                .creature
                .set_corpse_delay(delay_secs, fully_skinned);
        })
        .is_some()
}

pub fn set_world_creature_corpse_deadline_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    deadline: Option<std::time::Instant>,
) -> bool {
    session
        .mutate_world_creature(guid, |world_creature| {
            world_creature.set_corpse_despawn_at(deadline);
        })
        .is_some()
}

pub fn set_world_creature_expired_corpse_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    expired_at: std::time::Instant,
) -> Option<u64> {
    session
        .mutate_world_creature(guid, |world_creature| {
            world_creature.creature.set_corpse_delay(0, false);
            world_creature.creature.mark_ai_dead(0);
            world_creature.creature.set_corpse_delay(120, false);
            world_creature.set_corpse_despawn_at(Some(expired_at));
            world_creature.corpse_despawn_deadline_ms_like_cpp()
        })
        .flatten()
}

pub fn world_creature_corpse_deadline_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
) -> Option<u64> {
    session
        .mutate_world_creature(guid, |world_creature| {
            world_creature.corpse_despawn_deadline_ms_like_cpp()
        })
        .flatten()
}

pub fn world_creature_corpse_delay_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
) -> Option<u32> {
    session
        .mutate_world_creature(guid, |world_creature| {
            world_creature.corpse_delay_secs_like_cpp()
        })
}

pub fn world_creature_corpse_despawn_at_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
) -> Option<std::time::Instant> {
    session
        .mutate_world_creature(guid, |world_creature| world_creature.corpse_despawn_at())
        .flatten()
}

pub fn world_creature_corpse_despawn_due_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
) -> bool {
    session
        .mutate_world_creature(guid, |world_creature| {
            world_creature.corpse_despawn_due_like_cpp()
        })
        .expect("loot fixture creature exists")
}

pub fn apply_world_creature_corpse_loot_flags_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    had_loot: bool,
    fully_skinned: bool,
) -> bool {
    session
        .mutate_world_creature(guid, |world_creature| {
            world_creature
                .apply_corpse_loot_flags_after_death_state_like_cpp(had_loot, fully_skinned);
        })
        .is_some()
}

pub fn world_creature_has_lootable_dynamic_flag_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
) -> bool {
    session
        .mutate_world_creature(guid, |world_creature| {
            world_creature.has_lootable_dynamic_flag_like_cpp()
        })
        .expect("loot fixture creature exists")
}

pub fn record_gameobject_owner_for_loot_test(
    session: &mut WorldSession,
    gameobject_guid: ObjectGuid,
    owner_guid: ObjectGuid,
) {
    session.record_represented_gameobject_owner_guid_like_cpp(gameobject_guid, owner_guid);
}

pub fn record_gameobject_chest_release_metadata_for_loot_test(
    session: &mut WorldSession,
    gameobject_guid: ObjectGuid,
    source: GameObjectLootSource,
) {
    let personal_loot_id = source.personal_loot_id;
    let state = session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default();
    state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8);
    state.chest_restock_time_secs = Some(source.chest_restock_time_secs);
    state.chest_consumable = Some(source.chest_consumable);
    state.despawn_at_action = source.chest_consumable;
    state.chest_loot_source = Some(source);
    state.chest_personal_loot_id = Some(personal_loot_id);
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectLootReleaseSnapshotForTest {
    pub loot_state: Option<LootState>,
    pub loot_state_unit_guid: ObjectGuid,
    pub go_state: Option<GoState>,
    pub personal_loot_uses: u32,
    pub per_player_despawn_secs: Option<u32>,
    pub per_player_despawn_until: Option<std::time::Instant>,
    pub per_player_state_player_guid: Option<ObjectGuid>,
    pub chest_restock_until: Option<std::time::Instant>,
}

pub fn gameobject_loot_release_snapshot_for_test(
    session: &WorldSession,
    gameobject_guid: ObjectGuid,
) -> Option<GameObjectLootReleaseSnapshotForTest> {
    let state = session.represented_gameobject_use_states.get(&gameobject_guid)?;
    Some(GameObjectLootReleaseSnapshotForTest {
        loot_state: state.loot_state,
        loot_state_unit_guid: state.loot_state_unit_guid,
        go_state: state.go_state,
        personal_loot_uses: state.personal_loot_uses,
        per_player_despawn_secs: state.per_player_despawn_secs,
        per_player_despawn_until: state.per_player_despawn_until,
        per_player_state_player_guid: state.per_player_state_player_guid,
        chest_restock_until: state.chest_restock_until,
    })
}

pub fn record_fishing_hole_max_opens_for_loot_test(
    session: &mut WorldSession,
    gameobject_guid: ObjectGuid,
    max_opens: u32,
) {
    session.record_represented_fishing_hole_max_opens_like_cpp(gameobject_guid, max_opens);
}

pub fn mark_chest_restock_expired_for_loot_test(
    session: &mut WorldSession,
    gameobject_guid: ObjectGuid,
    loot_state: LootState,
) {
    let state = session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default();
    state.loot_state = Some(loot_state);
    state.chest_restock_until = Some(Instant::now() - Duration::from_secs(1));
}

pub fn is_per_player_gameobject_despawned_for_loot_test(
    session: &mut WorldSession,
    gameobject_guid: ObjectGuid,
) -> bool {
    session.represented_gameobject_is_per_player_despawned_like_cpp(gameobject_guid)
}

