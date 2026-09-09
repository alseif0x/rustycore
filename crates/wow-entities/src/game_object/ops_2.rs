//! GameObject template, loot and runtime state operations, part 2 of 2.
//!
//! The inherent `GameObject` impl is divided by responsibility under
//! #636; every method keeps its original body.

use super::*;

impl GameObject {
    pub const fn linked_trap_guid_like_cpp(&self) -> ObjectGuid {
        self.linked_trap_guid
    }
    pub fn set_linked_trap_like_cpp(&mut self, linked_trap_guid: ObjectGuid) {
        self.linked_trap_guid = linked_trap_guid;
    }
    /// Represented `GAMEOBJECT_BYTES_1` animation progress used by
    /// `GameObject::GetGoAnimProgress()` in bounded map-owned update seams.
    ///
    /// C++ anchor: `GameObject.cpp:951-1132` passes `GameObjectData::animprogress`
    /// through `GameObject::Create(..., animProgress, ...)` and calls
    /// `SetGoAnimProgress(...)` for the represented create branches here.
    pub const fn go_anim_progress_like_cpp(&self) -> u8 {
        self.go_anim_progress_like_cpp
    }
    pub fn set_go_anim_progress_like_cpp(&mut self, progress: u8) {
        self.go_anim_progress_like_cpp = progress;
    }
    /// Explicit represented baseline for the flags restored by
    /// `GameObject::Update` after `SendGameObjectDespawn()`.
    ///
    /// This is not a full `GameObjectOverride` runtime. It is populated from the
    /// lifecycle/template flags when available or by tests/callers with explicit
    /// source evidence; absent source means no represented restore should clobber
    /// current runtime flags.
    pub const fn represented_baseline_flags_like_cpp(&self) -> Option<u32> {
        self.represented_baseline_flags_like_cpp
    }
    pub fn set_represented_baseline_flags_like_cpp(&mut self, flags: Option<u32>) {
        self.represented_baseline_flags_like_cpp = flags;
    }
    pub fn restore_represented_baseline_flags_like_cpp(&mut self) -> bool {
        if let Some(flags) = self.represented_baseline_flags_like_cpp {
            self.set_flags(flags);
            true
        } else {
            false
        }
    }
    pub const fn owner_guid(&self) -> ObjectGuid {
        self.data.created_by
    }
    pub fn set_path_progress_for_client(&mut self, progress: f32) {
        let had_dynamic_flags_change = self
            .world
            .object()
            .changed_fields()
            .contains(ObjectChangedFields::DYNAMIC_FLAGS);
        let path_progress = (progress.clamp(0.0, 1.0) * 65_535.0) as u32;
        let dynamic_flags = (self.world.object().dynamic_flags() & 0xFFFF) | (path_progress << 16);

        if had_dynamic_flags_change {
            self.world
                .object_mut()
                .replace_all_dynamic_flags(dynamic_flags);
        } else {
            self.world
                .object_mut()
                .replace_all_dynamic_flags_suppressed(dynamic_flags);
        }
    }
    pub fn changed_object_type_mask(&self) -> u32 {
        self.world.object().changed_object_type_mask()
            | if self.game_object_data_changes.is_any_set() {
                1 << TYPEID_GAME_OBJECT
            } else {
                0
            }
    }
    pub fn values_update(&self) -> GameObjectValuesUpdate {
        let object_update = self.world.object().values_update();
        GameObjectValuesUpdate {
            changed_object_type_mask: self.changed_object_type_mask(),
            object_data: object_update.object_data,
            game_object_data: self.game_object_data_changes.is_any_set().then(|| {
                GameObjectDataUpdate {
                    mask: self.game_object_data_changes.clone(),
                    values: self.data,
                }
            }),
        }
    }
    pub(super) fn set_u32_field(
        &mut self,
        bit: usize,
        value: u32,
        field: impl FnOnce(&mut GameObjectDataValues) -> &mut u32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_game_object_data(bit);
        }
    }
    pub(super) fn set_i32_field(
        &mut self,
        bit: usize,
        value: i32,
        field: impl FnOnce(&mut GameObjectDataValues) -> &mut i32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_game_object_data(bit);
        }
    }
    pub(super) fn set_i8_field(
        &mut self,
        bit: usize,
        value: i8,
        field: impl FnOnce(&mut GameObjectDataValues) -> &mut i8,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_game_object_data(bit);
        }
    }
    pub(super) fn set_u8_field(
        &mut self,
        bit: usize,
        value: u8,
        field: impl FnOnce(&mut GameObjectDataValues) -> &mut u8,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_game_object_data(bit);
        }
    }
    pub(super) fn set_guid_field(
        &mut self,
        bit: usize,
        value: ObjectGuid,
        field: impl FnOnce(&mut GameObjectDataValues) -> &mut ObjectGuid,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_game_object_data(bit);
        }
    }
    pub(super) fn mark_game_object_data(&mut self, bit: usize) {
        self.game_object_data_changes
            .set(GAME_OBJECT_DATA_PARENT_BIT);
        self.game_object_data_changes.set(bit);
    }
}
