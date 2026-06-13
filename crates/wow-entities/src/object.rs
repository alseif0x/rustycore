use bitflags::bitflags;
use wow_constants::{TypeId, TypeMask};
use wow_core::ObjectGuid;

use crate::update_fields::{
    OBJECT_DATA_BITS, OBJECT_DATA_DYNAMIC_FLAGS_BIT, OBJECT_DATA_ENTRY_ID_BIT,
    OBJECT_DATA_PARENT_BIT, OBJECT_DATA_SCALE_BIT, ObjectDataUpdate, ObjectDataValues,
    TYPEID_OBJECT, UpdateMask, ValuesUpdate,
};

bitflags! {
    /// Rust representation of TrinityCore `CreateObjectBits`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct CreateObjectFlags: u32 {
        const NO_BIRTH_ANIM = 1 << 0;
        const ENABLE_PORTALS = 1 << 1;
        const PLAY_HOVER_ANIM = 1 << 2;
        const MOVEMENT_UPDATE = 1 << 3;
        const MOVEMENT_TRANSPORT = 1 << 4;
        const STATIONARY = 1 << 5;
        const COMBAT_VICTIM = 1 << 6;
        const SERVER_TIME = 1 << 7;
        const VEHICLE = 1 << 8;
        const ANIM_KIT = 1 << 9;
        const ROTATION = 1 << 10;
        const AREA_TRIGGER = 1 << 11;
        const GAME_OBJECT = 1 << 12;
        const SMOOTH_PHASING = 1 << 13;
        const THIS_IS_YOU = 1 << 14;
        const SCENE_OBJECT = 1 << 15;
        const ACTIVE_PLAYER = 1 << 16;
        const CONVERSATION = 1 << 17;
    }
}

bitflags! {
    /// Changed subset of C++ `UF::ObjectData`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ObjectChangedFields: u8 {
        const ENTRY_ID = 1 << 0;
        const DYNAMIC_FLAGS = 1 << 1;
        const SCALE = 1 << 2;
    }
}

bitflags! {
    /// Rust representation of TrinityCore `NotifyFlags`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ObjectNotifyFlags: u16 {
        const AI_RELOCATION = 0x01;
        const VISIBILITY_CHANGED = 0x02;
        const ALL = 0xFF;
    }
}

/// Minimal canonical `Object` state from TrinityCore `Object`.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityObject {
    guid: ObjectGuid,
    type_id: TypeId,
    type_mask: TypeMask,
    entry: u32,
    scale: f32,
    dynamic_flags: u32,
    create_flags: CreateObjectFlags,
    in_world: bool,
    in_grid: bool,
    is_new_object: bool,
    is_destroyed_object: bool,
    object_updated: bool,
    changed_fields: ObjectChangedFields,
    notify_flags: ObjectNotifyFlags,
    map_id: Option<u32>,
    instance_id: Option<u32>,
}

impl Default for EntityObject {
    fn default() -> Self {
        Self::new(TypeId::Object, TypeMask::OBJECT)
    }
}

impl EntityObject {
    pub fn new(type_id: TypeId, type_mask: TypeMask) -> Self {
        Self {
            guid: ObjectGuid::EMPTY,
            type_id,
            type_mask,
            entry: 0,
            scale: 1.0,
            dynamic_flags: 0,
            create_flags: CreateObjectFlags::empty(),
            in_world: false,
            in_grid: false,
            is_new_object: false,
            is_destroyed_object: false,
            object_updated: false,
            changed_fields: ObjectChangedFields::empty(),
            notify_flags: ObjectNotifyFlags::empty(),
            map_id: None,
            instance_id: None,
        }
    }

    pub fn create(&mut self, guid: ObjectGuid) {
        self.object_updated = false;
        self.guid = guid;
    }

    pub const fn guid(&self) -> ObjectGuid {
        self.guid
    }

    pub const fn entry(&self) -> u32 {
        self.entry
    }

    pub const fn scale(&self) -> f32 {
        self.scale
    }

    pub const fn dynamic_flags(&self) -> u32 {
        self.dynamic_flags
    }

    pub const fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub const fn type_mask(&self) -> TypeMask {
        self.type_mask
    }

    pub fn set_type(&mut self, type_id: TypeId, type_mask: TypeMask) {
        self.type_id = type_id;
        self.type_mask = type_mask;
    }

    pub fn is_type(&self, mask: TypeMask) -> bool {
        self.type_mask.intersects(mask)
    }

    pub const fn create_flags(&self) -> CreateObjectFlags {
        self.create_flags
    }

    pub fn create_flags_mut(&mut self) -> &mut CreateObjectFlags {
        &mut self.create_flags
    }

    pub const fn is_in_world(&self) -> bool {
        self.in_world
    }

    pub const fn is_in_grid(&self) -> bool {
        self.in_grid
    }

    pub const fn is_new_object(&self) -> bool {
        self.is_new_object
    }

    pub const fn is_destroyed_object(&self) -> bool {
        self.is_destroyed_object
    }

    pub const fn is_object_updated(&self) -> bool {
        self.object_updated
    }

    pub const fn changed_fields(&self) -> ObjectChangedFields {
        self.changed_fields
    }

    pub const fn notify_flags(&self) -> ObjectNotifyFlags {
        self.notify_flags
    }

    pub fn add_to_notify(&mut self, flags: ObjectNotifyFlags) {
        self.notify_flags.insert(flags);
    }

    pub fn is_need_notify(&self, flags: ObjectNotifyFlags) -> bool {
        self.notify_flags.intersects(flags)
    }

    pub fn reset_all_notifies(&mut self) {
        self.notify_flags = ObjectNotifyFlags::empty();
    }

    pub fn object_data_values(&self) -> ObjectDataValues {
        ObjectDataValues {
            entry_id: self.entry as i32,
            dynamic_flags: self.dynamic_flags,
            scale: self.scale,
        }
    }

    pub fn object_data_changes_mask(&self) -> UpdateMask {
        let mut mask = UpdateMask::new(OBJECT_DATA_BITS);
        if !self.changed_fields.is_empty() {
            mask.set(OBJECT_DATA_PARENT_BIT);
        }
        if self.changed_fields.contains(ObjectChangedFields::ENTRY_ID) {
            mask.set(OBJECT_DATA_ENTRY_ID_BIT);
        }
        if self
            .changed_fields
            .contains(ObjectChangedFields::DYNAMIC_FLAGS)
        {
            mask.set(OBJECT_DATA_DYNAMIC_FLAGS_BIT);
        }
        if self.changed_fields.contains(ObjectChangedFields::SCALE) {
            mask.set(OBJECT_DATA_SCALE_BIT);
        }
        mask
    }

    pub fn changed_object_type_mask(&self) -> u32 {
        if self.changed_fields.is_empty() {
            0
        } else {
            1 << TYPEID_OBJECT
        }
    }

    pub fn values_update(&self) -> ValuesUpdate {
        let changed_object_type_mask = self.changed_object_type_mask();
        if changed_object_type_mask == 0 {
            return ValuesUpdate::empty();
        }

        ValuesUpdate {
            changed_object_type_mask,
            object_data: Some(ObjectDataUpdate {
                mask: self.object_data_changes_mask(),
                values: self.object_data_values(),
            }),
        }
    }

    pub const fn map_id(&self) -> Option<u32> {
        self.map_id
    }

    pub const fn instance_id(&self) -> Option<u32> {
        self.instance_id
    }

    pub fn add_to_world(&mut self) {
        if self.in_world {
            return;
        }

        self.in_world = true;
        self.clear_update_mask(false);
    }

    pub fn remove_from_world(&mut self) {
        if !self.in_world {
            return;
        }

        self.in_world = false;
        self.clear_update_mask(true);
    }

    pub fn set_entry(&mut self, entry: u32) {
        if self.entry != entry {
            self.entry = entry;
            self.changed_fields.insert(ObjectChangedFields::ENTRY_ID);
            self.add_to_object_update_if_needed();
        }
    }

    pub fn set_scale(&mut self, scale: f32) {
        if self.scale != scale {
            self.scale = scale;
            self.changed_fields.insert(ObjectChangedFields::SCALE);
            self.add_to_object_update_if_needed();
        }
    }

    pub fn set_dynamic_flag(&mut self, flag: u32) {
        self.replace_all_dynamic_flags(self.dynamic_flags | flag);
    }

    pub fn remove_dynamic_flag(&mut self, flag: u32) {
        self.replace_all_dynamic_flags(self.dynamic_flags & !flag);
    }

    pub fn has_dynamic_flag(&self, flag: u32) -> bool {
        (self.dynamic_flags & flag) != 0
    }

    pub fn replace_all_dynamic_flags(&mut self, flags: u32) {
        if self.dynamic_flags != flags {
            self.dynamic_flags = flags;
            self.changed_fields
                .insert(ObjectChangedFields::DYNAMIC_FLAGS);
            self.add_to_object_update_if_needed();
        }
    }

    pub fn force_dynamic_flags_update_like_cpp(&mut self) {
        self.changed_fields
            .insert(ObjectChangedFields::DYNAMIC_FLAGS);
        self.add_to_object_update_if_needed();
    }

    pub fn replace_all_dynamic_flags_suppressed(&mut self, flags: u32) {
        self.dynamic_flags = flags;
    }

    pub fn set_is_new_object(&mut self, enable: bool) {
        self.is_new_object = enable;
    }

    pub fn set_destroyed_object(&mut self, destroyed: bool) {
        self.is_destroyed_object = destroyed;
    }

    pub fn set_grid_presence(&mut self, in_grid: bool) {
        self.in_grid = in_grid;
    }

    pub fn bind_map(&mut self, map_id: u32, instance_id: u32) {
        self.map_id = Some(map_id);
        self.instance_id = Some(instance_id);
    }

    pub fn clear_map_binding(&mut self) {
        self.map_id = None;
        self.instance_id = None;
        self.in_grid = false;
    }

    pub fn clear_update_mask(&mut self, remove: bool) {
        self.changed_fields = ObjectChangedFields::empty();
        if self.object_updated {
            if remove {
                self.remove_from_object_update();
            }
            self.object_updated = false;
        }
    }

    pub(crate) fn add_to_object_update_if_needed(&mut self) {
        if self.in_world && !self.object_updated {
            self.object_updated = self.add_to_object_update();
        }
    }

    fn add_to_object_update(&self) -> bool {
        true
    }

    fn remove_from_object_update(&mut self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityObjectState {
    pub in_world: bool,
    pub in_grid: bool,
    pub is_new_object: bool,
    pub is_destroyed_object: bool,
    pub object_updated: bool,
}

impl From<&EntityObject> for EntityObjectState {
    fn from(object: &EntityObject) -> Self {
        Self {
            in_world: object.in_world,
            in_grid: object.in_grid,
            is_new_object: object.is_new_object,
            is_destroyed_object: object.is_destroyed_object,
            object_updated: object.object_updated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_object_matches_cpp_constructor_state() {
        let object = EntityObject::default();

        assert_eq!(object.guid(), ObjectGuid::EMPTY);
        assert_eq!(object.type_id(), TypeId::Object);
        assert_eq!(object.type_mask(), TypeMask::OBJECT);
        assert_eq!(object.entry(), 0);
        assert_eq!(object.scale(), 1.0);
        assert_eq!(object.dynamic_flags(), 0);
        assert!(!object.is_in_world());
        assert!(!object.is_in_grid());
        assert!(!object.is_new_object());
        assert!(!object.is_destroyed_object());
        assert!(!object.is_object_updated());
        assert!(object.changed_fields().is_empty());
    }

    #[test]
    fn type_ids_and_masks_match_object_guid_h_values() {
        assert_eq!(TypeId::Object as u32, 0);
        assert_eq!(TypeId::Unit as u32, 5);
        assert_eq!(TypeId::Player as u32, 6);
        assert_eq!(TypeId::GameObject as u32, 8);
        assert_eq!(TypeMask::OBJECT.bits(), 0x0001);
        assert_eq!(TypeMask::UNIT.bits(), 0x0020);
        assert_eq!(TypeMask::PLAYER.bits(), 0x0040);
        assert_eq!(TypeMask::GAME_OBJECT.bits(), 0x0100);
        assert!(TypeMask::SEER.contains(TypeMask::PLAYER));
        assert!(TypeMask::SEER.contains(TypeMask::UNIT));
        assert!(TypeMask::SEER.contains(TypeMask::DYNAMIC_OBJECT));
    }

    #[test]
    fn create_sets_guid_and_clears_update_state_like_cpp_create() {
        let mut object = EntityObject::default();
        object.add_to_world();
        object.set_entry(42);
        assert!(object.is_object_updated());

        let guid = ObjectGuid::new(7, 11);
        object.create(guid);

        assert_eq!(object.guid(), guid);
        assert!(!object.is_object_updated());
    }

    #[test]
    fn add_and_remove_world_clear_update_mask_like_cpp() {
        let mut object = EntityObject::default();

        object.set_entry(42);
        assert!(
            object
                .changed_fields()
                .contains(ObjectChangedFields::ENTRY_ID)
        );
        assert!(!object.is_object_updated());

        object.add_to_world();
        assert!(object.is_in_world());
        assert!(object.changed_fields().is_empty());
        assert!(!object.is_object_updated());

        object.set_scale(2.0);
        assert!(object.changed_fields().contains(ObjectChangedFields::SCALE));
        assert!(object.is_object_updated());

        object.remove_from_world();
        assert!(!object.is_in_world());
        assert!(object.changed_fields().is_empty());
        assert!(!object.is_object_updated());
    }

    #[test]
    fn dynamic_flags_match_cpp_flag_helpers() {
        let mut object = EntityObject::default();
        object.add_to_world();

        object.set_dynamic_flag(0x04);
        object.set_dynamic_flag(0x10);
        assert_eq!(object.dynamic_flags(), 0x14);
        assert!(object.has_dynamic_flag(0x04));
        assert!(object.has_dynamic_flag(0x10));
        assert!(object.is_object_updated());

        object.remove_dynamic_flag(0x04);
        assert_eq!(object.dynamic_flags(), 0x10);

        object.replace_all_dynamic_flags(0x80);
        assert_eq!(object.dynamic_flags(), 0x80);
    }

    #[test]
    fn force_dynamic_flags_update_marks_field_without_value_change_like_cpp() {
        let mut object = EntityObject::default();
        object.add_to_world();
        assert!(object.changed_fields().is_empty());
        assert!(!object.is_object_updated());

        object.force_dynamic_flags_update_like_cpp();

        assert_eq!(object.dynamic_flags(), 0);
        assert!(
            object
                .changed_fields()
                .contains(ObjectChangedFields::DYNAMIC_FLAGS)
        );
        assert!(object.is_object_updated());
        assert!(
            object
                .object_data_changes_mask()
                .is_set(OBJECT_DATA_DYNAMIC_FLAGS_BIT)
        );
    }

    #[test]
    fn notify_flags_match_cpp_relocation_visibility_helpers() {
        let mut object = EntityObject::default();

        assert_eq!(object.notify_flags(), ObjectNotifyFlags::empty());
        assert!(!object.is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED));

        object.add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
        assert!(object.is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED));
        assert!(!object.is_need_notify(ObjectNotifyFlags::AI_RELOCATION));
        assert_eq!(object.notify_flags(), ObjectNotifyFlags::VISIBILITY_CHANGED);

        object.add_to_notify(ObjectNotifyFlags::AI_RELOCATION);
        assert!(object.is_need_notify(ObjectNotifyFlags::ALL));
        assert_eq!(
            object.notify_flags(),
            ObjectNotifyFlags::AI_RELOCATION | ObjectNotifyFlags::VISIBILITY_CHANGED
        );

        object.reset_all_notifies();
        assert_eq!(object.notify_flags(), ObjectNotifyFlags::empty());
    }

    #[test]
    fn object_data_update_mask_uses_cpp_object_data_bits() {
        let mut object = EntityObject::default();
        object.add_to_world();

        object.set_entry(42);
        object.set_dynamic_flag(0x04);
        object.set_scale(2.0);

        let mask = object.object_data_changes_mask();
        assert_eq!(mask.get_block(0), 0b1111);
        assert!(mask.is_set(OBJECT_DATA_PARENT_BIT));
        assert!(mask.is_set(OBJECT_DATA_ENTRY_ID_BIT));
        assert!(mask.is_set(OBJECT_DATA_DYNAMIC_FLAGS_BIT));
        assert!(mask.is_set(OBJECT_DATA_SCALE_BIT));
        assert_eq!(object.changed_object_type_mask(), 1 << TYPEID_OBJECT);

        let update = object.values_update();
        assert!(update.has_data());
        assert_eq!(update.changed_object_type_mask, 1);
        let object_data = update.object_data.unwrap();
        assert_eq!(object_data.mask.get_block(0), 0b1111);
        assert_eq!(object_data.values.entry_id, 42);
        assert_eq!(object_data.values.dynamic_flags, 0x04);
        assert_eq!(object_data.values.scale, 2.0);
    }

    #[test]
    fn map_and_grid_state_are_rust_bridge_fields_for_canonical_map_ownership() {
        let mut object = EntityObject::default();

        object.bind_map(530, 7);
        object.set_grid_presence(true);
        assert_eq!(object.map_id(), Some(530));
        assert_eq!(object.instance_id(), Some(7));
        assert!(object.is_in_grid());

        object.clear_map_binding();
        assert_eq!(object.map_id(), None);
        assert_eq!(object.instance_id(), None);
        assert!(!object.is_in_grid());
    }
}
