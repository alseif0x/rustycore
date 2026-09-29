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
    LootEntry {
        loot_list_id: item.loot_list_id as u8,
        item_id: item.item_id,
        quantity: item.count,
        random_properties_id: item.random_properties_id,
        random_properties_seed: item.random_properties_seed,
        item_context: item.context,
        flags: LootEntryFlags {
            follow_loot_rules: !item.needs_quest || addon_metadata.follows_loot_rules(),
            freeforall: item.free_for_all,
            blocked: item.is_blocked,
            counted: item.is_counted,
            under_threshold: item.is_under_threshold,
            needs_quest: item.needs_quest,
        },
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: item.is_looted,
    }
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
    let store_item_context = item.store_item_context;
    let mut entry = generated_creature_loot_item_to_entry_like_cpp(item, addon_metadata);
    for looter in allowed_looters {
        if item_allowed_for_player(store_item_context, *looter) {
            entry.add_allowed_looter_like_cpp(*looter);
        }
    }
    entry
}

#[derive(Debug, Clone)]
pub(in crate::handlers::loot) struct RepresentedCreatureLootStateLikeCpp {
    pub(in crate::handlers::loot) is_alive: bool,
    pub(in crate::handlers::loot) position: wow_core::Position,
    pub(in crate::handlers::loot) level: u8,
    pub(in crate::handlers::loot) entry: u32,
    pub(in crate::handlers::loot) loot_id: u32,
    pub(in crate::handlers::loot) gold_min: u32,
    pub(in crate::handlers::loot) gold_max: u32,
    pub(in crate::handlers::loot) dungeon_encounter_id: u32,
    pub(in crate::handlers::loot) tappers: Vec<ObjectGuid>,
    pub(in crate::handlers::loot) loot_lifecycle_revision: u64,
}

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
    // C++ ref: GameObject.cpp GetInteractionDistance().
    // Spell-lock range remains with the typed GameObject/SpellInfo port.
    if let Some(override_hundredths) = interact_radius_override.filter(|value| *value != 0) {
        return override_hundredths as f32 / 100.0;
    }

    match go_type.map(u32::from) {
        Some(GAMEOBJECT_TYPE_AREADAMAGE) => 0.0,
        Some(GAMEOBJECT_TYPE_QUESTGIVER)
        | Some(GAMEOBJECT_TYPE_TEXT)
        | Some(GAMEOBJECT_TYPE_FLAGSTAND)
        | Some(GAMEOBJECT_TYPE_FLAGDROP)
        | Some(GAMEOBJECT_TYPE_MINI_GAME) => 5.5555553,
        Some(GAMEOBJECT_TYPE_BINDER) => 10.0,
        Some(GAMEOBJECT_TYPE_CHAIR) | Some(GAMEOBJECT_TYPE_BARBER_CHAIR) => 3.0,
        Some(GAMEOBJECT_TYPE_FISHING_NODE) => 100.0,
        Some(GAMEOBJECT_TYPE_FISHING_HOLE) => 20.0 + wow_movement::CONTACT_DISTANCE_LIKE_CPP,
        Some(GAMEOBJECT_TYPE_CAMERA)
        | Some(GAMEOBJECT_TYPE_MAP_OBJECT)
        | Some(GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY)
        | Some(GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING)
        | Some(GAMEOBJECT_TYPE_DOOR) => 5.0,
        Some(GAMEOBJECT_TYPE_GUILD_BANK) | Some(GAMEOBJECT_TYPE_MAILBOX) => 10.0,
        _ => 5.0,
    }
}

pub(in crate::handlers::loot) fn represented_gameobject_display_box_contains_like_cpp(
    go_position: wow_core::Position,
    player_position: wow_core::Position,
    display_info: &wow_data::GameObjectDisplayInfoEntry,
    scale: f32,
    rotation: [f32; 4],
    radius: f32,
) -> bool {
    let min_x = display_info.geo_box_min.x * scale - radius;
    let min_y = display_info.geo_box_min.y * scale - radius;
    let min_z = display_info.geo_box_min.z * scale - radius;
    let max_x = display_info.geo_box_max.x * scale + radius;
    let max_y = display_info.geo_box_max.y * scale + radius;
    let max_z = display_info.geo_box_max.z * scale + radius;

    let dx = player_position.x - go_position.x;
    let dy = player_position.y - go_position.y;
    let dz = player_position.z - go_position.z;
    let [qx, qy, qz, qw] = rotation;
    let iqx = -qx;
    let iqy = -qy;
    let iqz = -qz;

    let tx = 2.0 * (iqy * dz - iqz * dy);
    let ty = 2.0 * (iqz * dx - iqx * dz);
    let tz = 2.0 * (iqx * dy - iqy * dx);
    let local_x = dx + qw * tx + (iqy * tz - iqz * ty);
    let local_y = dy + qw * ty + (iqz * tx - iqx * tz);
    let local_z = dz + qw * tz + (iqx * ty - iqy * tx);

    local_x >= min_x
        && local_x <= max_x
        && local_y >= min_y
        && local_y <= max_y
        && local_z >= min_z
        && local_z <= max_z
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

pub(in crate::handlers::loot) fn looted_corpse_decay_secs_like_cpp(
    is_fully_skinned: bool,
    corpse_delay_secs: u32,
    ignore_decay_ratio: bool,
    corpse_decay_looted_rate: f32,
) -> u32 {
    if is_fully_skinned {
        return 0;
    }

    let rate = if ignore_decay_ratio {
        1.0
    } else {
        corpse_decay_looted_rate.max(0.0)
    };
    ((corpse_delay_secs as f32) * rate) as u32
}
