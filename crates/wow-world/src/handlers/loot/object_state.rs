//! Loot object state for creatures and game objects: the represented runtime state,
//! the release record, generated-item conversion and corpse decay.
//!
//! Split out of `loot/mod.rs` under #584 (B5); items are unchanged.

use super::*;

#[derive(Clone)]
pub(in crate::handlers::loot) struct AuthoritativeLootReleaseLikeCpp {
    pub(in crate::handlers::loot) authority: OwnedLootAuthority,
    pub(in crate::handlers::loot) selected_generation: u64,
    pub(in crate::handlers::loot) loot: CreatureLoot,
    pub(in crate::handlers::loot) whole_object_fully_looted: bool,
    pub(in crate::handlers::loot) whole_object_fully_skinned: bool,
    pub(in crate::handlers::loot) object_generation: u64,
    pub(in crate::handlers::loot) lifecycle_revision: u64,
    pub(in crate::handlers::loot) require_no_viewers: bool,
}

pub(in crate::handlers::loot) fn generated_creature_loot_item_to_entry_like_cpp(
    item: GeneratedLootItem,
    addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
) -> LootEntry {
    wow_loot::loot_entry_from_generated(item, addon_metadata.follows_loot_rules())
}

pub(in crate::handlers::loot) fn generated_shared_gameobject_loot_item_to_entry_like_cpp<FAllowed>(
    item: GeneratedLootItem,
    addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
    allowed_looters: &[ObjectGuid],
    mut item_allowed_for_player: FAllowed,
) -> LootEntry
where
    FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
{
    wow_loot::shared_loot_entry_from_generated(
        item,
        addon_metadata.follows_loot_rules(),
        allowed_looters,
        &mut item_allowed_for_player,
    )
}

pub(in crate::handlers::loot) use wow_map::map_manager::CreatureLootObservation
    as RepresentedCreatureLootStateLikeCpp;

#[derive(Debug, Clone)]
pub(in crate::handlers::loot) struct RepresentedGameObjectLootInstallObservationLikeCpp {
    pub(in crate::handlers::loot) authority: OwnedLootAuthority,
    pub(in crate::handlers::loot) object_generation: u64,
    pub(in crate::handlers::loot) loot_lifecycle_revision: u64,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::handlers::loot) struct RepresentedGameObjectLootStateLikeCpp {
    pub(in crate::handlers::loot) position: Option<wow_core::Position>,
    pub(in crate::handlers::loot) display_id: Option<u32>,
    pub(in crate::handlers::loot) scale: f32,
    pub(in crate::handlers::loot) rotation: [f32; 4],
    pub(in crate::handlers::loot) go_type: Option<u8>,
    pub(in crate::handlers::loot) interact_radius_override: Option<u32>,
    pub(in crate::handlers::loot) lock_id: Option<u32>,
    pub(in crate::handlers::loot) owner_guid: Option<ObjectGuid>,
}

pub(in crate::handlers) fn represented_gameobject_interaction_distance_like_cpp(
    go_type: Option<u8>,
    interact_radius_override: Option<u32>,
) -> f32 {
    wow_entities::gameobject_interaction_distance(go_type, interact_radius_override, wow_movement::CONTACT_DISTANCE_LIKE_CPP)
}

pub(in crate::handlers::loot) fn represented_gameobject_display_box_contains_like_cpp(
    go_position: wow_core::Position,
    player_position: wow_core::Position,
    display_info: &wow_data::GameObjectDisplayInfoEntry,
    scale: f32,
    rotation: [f32; 4],
    radius: f32,
) -> bool {
    wow_entities::gameobject_display_box_contains(go_position, player_position,
        [display_info.geo_box_min.x, display_info.geo_box_min.y, display_info.geo_box_min.z],
        [display_info.geo_box_max.x, display_info.geo_box_max.y, display_info.geo_box_max.z],
        scale, rotation, radius)
}

#[cfg(test)]
pub(in crate::handlers::loot) fn represented_loot_object_guid_like_cpp(
    owner: ObjectGuid,
) -> ObjectGuid {
    if owner.is_empty() {
        return ObjectGuid::EMPTY;
    }

    ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner.realm_id(),
        owner.map_id(),
        0,
        0,
        owner.counter(),
    )
}

pub(in crate::handlers::loot) use wow_entities::looted_corpse_decay_seconds as looted_corpse_decay_secs_like_cpp;
