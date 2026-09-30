//! GameObject template, loot and runtime state state definitions, part 2 of 2.
//!
//! Separated from the game_object.rs root under #636. Behaviour is preserved.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameObjectDataValues {
    pub display_id: i32,
    pub created_by: ObjectGuid,
    pub flags: u32,
    pub faction_template: i32,
    pub level: i32,
    pub state: i8,
    pub type_id: i8,
    pub percent_health: u8,
    pub art_kit: u32,
    pub custom_param: u32,
}

impl Default for GameObjectDataValues {
    fn default() -> Self {
        Self {
            display_id: 0,
            created_by: ObjectGuid::EMPTY,
            flags: 0,
            faction_template: 0,
            level: 0,
            state: GoState::Active as i8,
            type_id: 0,
            percent_health: 0,
            art_kit: 0,
            custom_param: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectDataUpdate {
    pub mask: UpdateMask,
    pub values: GameObjectDataValues,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataUpdate>,
    pub game_object_data: Option<GameObjectDataUpdate>,
}

impl GameObjectValuesUpdate {
    pub const fn has_data(&self) -> bool {
        self.changed_object_type_mask != 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObject {
    pub(super) world: WorldObject,
    pub(super) data: GameObjectDataValues,
    pub(super) game_object_data_changes: UpdateMask,
    pub(super) spell_id: u32,
    pub(super) respawn_time: i64,
    pub(super) respawn_delay_time: u32,
    pub(super) despawn_delay: u32,
    pub(super) despawn_respawn_time: u32,
    pub(super) restock_time: i64,
    pub(super) loot_state: LootState,
    pub(super) loot_state_unit_guid: ObjectGuid,
    /// Monotonic identity for one C++ `GameObject::loot` lifetime. `ClearLoot`
    /// advances it before a restock can generate another pool, so async work
    /// captured for the previous use cannot install into the next use of the
    /// same spawn GUID.
    pub(super) loot_lifecycle_revision: u64,
    pub(super) loot_authority: OwnedLootAuthority,
    pub(super) shared_loot: Option<GameObjectOwnedLoot>,
    pub(super) personal_loot: HashMap<ObjectGuid, GameObjectOwnedLoot>,
    pub(super) unique_users: HashSet<ObjectGuid>,
    pub(super) spawned_by_default: bool,
    pub(super) use_times: u32,
    pub(super) cooldown_time: i64,
    pub(super) prev_go_state: GoState,
    pub(super) packed_rotation: i64,
    pub(super) local_rotation: [f32; 4],
    pub(super) spawn_id: u64,
    pub(super) loot_mode: u16,
    pub(super) respawn_compatibility_mode: bool,
    pub(super) anim_kit_id: u16,
    pub(super) world_effect_id: u32,
    pub(super) lifecycle_string_id: String,
    pub(super) linked_trap_guid: ObjectGuid,
    pub(super) stationary_position: Position,
    pub(super) go_anim_progress_like_cpp: u8,
    pub(super) represented_baseline_flags_like_cpp: Option<u32>,
    /// Resolved C++ `GetGOInfo()->chest` source carried only when this entity was
    /// constructed or explicitly seeded with a CHEST template source.
    ///
    /// This is represented template evidence for bounded `GameObject::Update`
    /// branches only; it is not a full live `GameObjectTemplate`/ObjectMgr owner.
    pub(super) chest_loot_source_like_cpp: Option<GameObjectLootSource>,
    /// Resolved C++ `GetGOInfo()->goober` source carried only when this entity was
    /// constructed or explicitly seeded with a GOOBER template source.
    ///
    /// This is represented template evidence for bounded `GameObject::Update`
    /// branches only; it is not a full live `GameObjectTemplate`/ObjectMgr owner.
    pub(super) goober_use_source_like_cpp: Option<GooberUseSource>,
    /// Resolved C++ `GetGOInfo()->GetSpellFocusType/Radius` source carried
    /// only when this entity was constructed or explicitly seeded with a
    /// SPELL_FOCUS/UI_LINK template source.
    ///
    /// This is represented template evidence for bounded `Spell::SearchSpellFocus`
    /// only; it is not a full live `GameObjectTemplate`/ObjectMgr owner.
    pub(super) spell_focus_use_source_like_cpp: Option<SpellFocusUseSource>,
    /// Explicit represented evidence for TrinityCore `GameObject::m_model != nullptr`.
    ///
    /// This flag exists only so map-owned AddToWorld/RemoveFromWorld seams can decide whether
    /// to register a represented `GameObjectModel` key in `Map`'s represented DynamicMapTree.
    /// It is not a real `GameObjectModel`, not model geometry, not `GO_FLAG_MAP_OBJECT`, not
    /// `EnableCollision`, and not DB/model-store hydration. Callers/tests must set it explicitly;
    /// Rust must not infer it from display id, template, or gameobject type until real
    /// `GameObjectModel::Create`/DB2 model runtime exists.
    pub(super) represented_gameobject_model_like_cpp: bool,
    /// Explicit represented evidence for TrinityCore `m_model && m_model->isMapObject()`.
    ///
    /// This is set only by a caller/test that represents `GameObject::CreateModel()` output.
    /// It may be true only when `represented_gameobject_model_like_cpp` is true, and it is the
    /// only represented source that toggles `GO_FLAG_MAP_OBJECT`.
    pub(super) represented_gameobject_model_is_map_object_like_cpp: bool,
    /// Last represented `m_model->enableCollision(enable)` value.
    ///
    /// `None` means the C++ call has not been represented or returned early because there was no
    /// represented model evidence. This is not real collision, BIH, LOS, intersection or height
    /// runtime.
    pub(super) represented_gameobject_model_collision_enabled_like_cpp: Option<bool>,
    /// Explicit represented evidence for TrinityCore `GameObject::m_goData != nullptr`.
    ///
    /// This is only the bounded `GameObjectData` presence needed by `SaveRespawnTime()`; it is not
    /// full ObjectMgr/DB metadata and must not be inferred from `spawn_id` alone.
    pub(super) represented_gameobject_data_present_like_cpp: bool,
    pub(super) grid_unload_cleanup_before_delete_count: u32,
    pub(super) grid_unload_delete_requested: bool,
    pub(super) grid_unload_respawn_relocation_requested: bool,
}

pub(super) fn pack_gameobject_local_rotation(rotation: [f32; 4]) -> i64 {
    const PACK_YZ: i64 = 1 << 20;
    const PACK_X: i64 = PACK_YZ << 1;
    const PACK_YZ_MASK: i64 = (PACK_YZ << 1) - 1;
    const PACK_X_MASK: i64 = (PACK_X << 1) - 1;

    let dot = rotation[0] * rotation[0]
        + rotation[1] * rotation[1]
        + rotation[2] * rotation[2]
        + rotation[3] * rotation[3];
    if dot <= f32::EPSILON {
        return 0;
    }

    let inv_len = 1.0 / dot.sqrt();
    let rx = rotation[0] * inv_len;
    let ry = rotation[1] * inv_len;
    let rz = rotation[2] * inv_len;
    let rw = rotation[3] * inv_len;
    let w_sign = if rw >= 0.0 { 1 } else { -1 };

    let x = ((rx * PACK_X as f32) as i32 as i64) * i64::from(w_sign) & PACK_X_MASK;
    let y = ((ry * PACK_YZ as f32) as i32 as i64) * i64::from(w_sign) & PACK_YZ_MASK;
    let z = ((rz * PACK_YZ as f32) as i32 as i64) * i64::from(w_sign) & PACK_YZ_MASK;

    z | (y << 21) | (x << 42)
}

impl Default for GameObject {
    fn default() -> Self {
        Self::new()
    }
}
