use wow_constants::{TypeId, TypeMask};
use wow_core::ObjectGuid;

use crate::update_fields::{
    OBJECT_DATA_BITS, OBJECT_DATA_DYNAMIC_FLAGS_BIT, OBJECT_DATA_ENTRY_ID_BIT,
    OBJECT_DATA_PARENT_BIT, OBJECT_DATA_SCALE_BIT, ObjectDataUpdate, ObjectDataValues,
    TYPEID_OBJECT, UpdateMask, ValuesUpdate,
};

use super::{
    CreateObjectFlags, EntityObject, EntityObjectState, ObjectChangedFields, ObjectNotifyFlags,
};

/// Immutable raw Object facts captured at one owner read point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectValuesSnapshot {
    guid: ObjectGuid,
    type_id: TypeId,
    type_mask: TypeMask,
    entry: u32,
    scale: f32,
    dynamic_flags: u32,
    create_flags: CreateObjectFlags,
    state: EntityObjectState,
    changed_fields: ObjectChangedFields,
    notify_flags: ObjectNotifyFlags,
    map_id: Option<u32>,
    instance_id: Option<u32>,
}

impl ObjectValuesSnapshot {
    pub fn capture(object: &EntityObject) -> Self {
        Self {
            guid: object.guid,
            type_id: object.type_id,
            type_mask: object.type_mask,
            entry: object.entry,
            scale: object.scale,
            dynamic_flags: object.dynamic_flags,
            create_flags: object.create_flags,
            state: EntityObjectState::from(object),
            changed_fields: object.changed_fields,
            notify_flags: object.notify_flags,
            map_id: object.map_id,
            instance_id: object.instance_id,
        }
    }

    pub const fn guid(&self) -> ObjectGuid {
        self.guid
    }

    pub const fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub const fn type_mask(&self) -> TypeMask {
        self.type_mask
    }

    pub const fn is_type(&self, mask: TypeMask) -> bool {
        self.type_mask.intersects(mask)
    }

    /// Raw unsigned entry identity, before the signed update-field projection.
    pub const fn entry(&self) -> u32 {
        self.entry
    }

    pub const fn scale(&self) -> f32 {
        self.scale
    }

    pub const fn dynamic_flags(&self) -> u32 {
        self.dynamic_flags
    }

    pub const fn create_flags(&self) -> CreateObjectFlags {
        self.create_flags
    }

    pub const fn state(&self) -> EntityObjectState {
        self.state
    }

    pub const fn changed_fields(&self) -> ObjectChangedFields {
        self.changed_fields
    }

    pub const fn notify_flags(&self) -> ObjectNotifyFlags {
        self.notify_flags
    }

    pub const fn map_id(&self) -> Option<u32> {
        self.map_id
    }

    pub const fn instance_id(&self) -> Option<u32> {
        self.instance_id
    }

    pub fn object_data_values(&self) -> ObjectDataValues {
        object_data_values_for(self.entry, self.dynamic_flags, self.scale)
    }

    pub fn object_data_changes_mask(&self) -> UpdateMask {
        object_data_changes_mask_for(self.changed_fields)
    }

    pub fn changed_object_type_mask(&self) -> u32 {
        changed_object_type_mask_for(self.changed_fields)
    }

    pub fn values_update(&self) -> ValuesUpdate {
        values_update_for(
            self.changed_fields,
            self.entry,
            self.dynamic_flags,
            self.scale,
        )
    }
}

pub(super) fn object_data_values_for(
    entry: u32,
    dynamic_flags: u32,
    scale: f32,
) -> ObjectDataValues {
    ObjectDataValues {
        entry_id: entry as i32,
        dynamic_flags,
        scale,
    }
}

pub(super) fn object_data_changes_mask_for(changed_fields: ObjectChangedFields) -> UpdateMask {
    let mut mask = UpdateMask::new(OBJECT_DATA_BITS);
    if !changed_fields.is_empty() {
        mask.set(OBJECT_DATA_PARENT_BIT);
    }
    if changed_fields.contains(ObjectChangedFields::ENTRY_ID) {
        mask.set(OBJECT_DATA_ENTRY_ID_BIT);
    }
    if changed_fields.contains(ObjectChangedFields::DYNAMIC_FLAGS) {
        mask.set(OBJECT_DATA_DYNAMIC_FLAGS_BIT);
    }
    if changed_fields.contains(ObjectChangedFields::SCALE) {
        mask.set(OBJECT_DATA_SCALE_BIT);
    }
    mask
}

pub(super) fn changed_object_type_mask_for(changed_fields: ObjectChangedFields) -> u32 {
    if changed_fields.is_empty() {
        0
    } else {
        1 << TYPEID_OBJECT
    }
}

pub(super) fn values_update_for(
    changed_fields: ObjectChangedFields,
    entry: u32,
    dynamic_flags: u32,
    scale: f32,
) -> ValuesUpdate {
    let changed_object_type_mask = changed_object_type_mask_for(changed_fields);
    if changed_object_type_mask == 0 {
        return ValuesUpdate::empty();
    }

    ValuesUpdate {
        changed_object_type_mask,
        object_data: Some(ObjectDataUpdate {
            mask: object_data_changes_mask_for(changed_fields),
            values: object_data_values_for(entry, dynamic_flags, scale),
        }),
    }
}

#[cfg(test)]
mod tests {
    use wow_constants::{TypeId, TypeMask};
    use wow_core::ObjectGuid;

    use crate::{
        CreateObjectFlags, EntityObject, ObjectChangedFields, ObjectNotifyFlags,
        update_fields::{ObjectDataValues, TYPEID_OBJECT},
    };

    #[test]
    fn snapshot_keeps_raw_values_with_clean_mask_and_float_bits() {
        let mut object = EntityObject::new(TypeId::Item, TypeMask::OBJECT | TypeMask::ITEM);
        object.create(ObjectGuid::create_item(1, 300));
        object.set_entry(u32::MAX);
        object.set_scale(f32::from_bits(0x8000_0000));
        object.replace_all_dynamic_flags(0xA5A5_0001);
        object
            .create_flags_mut()
            .insert(CreateObjectFlags::NO_BIRTH_ANIM);
        object.bind_map(44, 55);
        object.add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
        object.clear_update_mask(false);

        let snapshot = object.values_snapshot();
        assert_eq!(snapshot.entry(), u32::MAX);
        let values = snapshot.object_data_values();
        assert_eq!(values.entry_id, -1);
        assert_eq!(values.dynamic_flags, 0xA5A5_0001);
        assert_eq!(values.scale.to_bits(), 0x8000_0000);
        assert!(snapshot.changed_fields().is_empty());
        assert!(!snapshot.object_data_changes_mask().is_any_set());
        assert_eq!(snapshot.values_update().object_data, None);
        assert_eq!(
            snapshot.values_update().changed_object_type_mask,
            0,
            "a clean mask does not erase the separately captured raw values"
        );
        assert_eq!(snapshot.guid(), ObjectGuid::create_item(1, 300));
        assert_eq!(snapshot.type_id(), TypeId::Item);
        assert!(snapshot.is_type(TypeMask::ITEM));
        assert!(
            snapshot
                .create_flags()
                .contains(CreateObjectFlags::NO_BIRTH_ANIM)
        );
        assert_eq!(snapshot.map_id(), Some(44));
        assert_eq!(snapshot.instance_id(), Some(55));
        assert_eq!(
            snapshot.notify_flags(),
            ObjectNotifyFlags::VISIBILITY_CHANGED
        );
        assert_eq!(
            values,
            ObjectDataValues {
                entry_id: -1,
                dynamic_flags: 0xA5A5_0001,
                scale: f32::from_bits(0x8000_0000),
            }
        );
    }

    #[test]
    fn snapshot_freezes_lifecycle_masks_and_shared_update_projection() {
        let mut object = EntityObject::new(TypeId::Object, TypeMask::OBJECT);
        object.create(ObjectGuid::create_item(1, 301));
        object.add_to_world();
        object.set_grid_presence(true);
        object.set_is_new_object(true);
        object.set_destroyed_object(true);
        object.bind_map(6, 9);
        object.add_to_notify(ObjectNotifyFlags::AI_RELOCATION);
        object.set_entry(72);
        object.set_scale(f32::NAN);
        object.replace_all_dynamic_flags(0x8000_0001);
        object
            .create_flags_mut()
            .insert(CreateObjectFlags::STATIONARY);

        let snapshot = object.values_snapshot();
        let captured_state = snapshot.state();
        let captured_changed = snapshot.changed_fields();
        assert_eq!(snapshot.scale().to_bits(), object.scale().to_bits());
        let snapshot_update = snapshot.values_update();
        let object_update = object.values_update();
        assert_eq!(
            snapshot_update.changed_object_type_mask,
            object_update.changed_object_type_mask
        );
        let snapshot_data = snapshot_update.object_data.unwrap();
        let object_data = object_update.object_data.unwrap();
        assert_eq!(snapshot_data.mask, object_data.mask);
        assert_eq!(snapshot_data.values.entry_id, object_data.values.entry_id);
        assert_eq!(
            snapshot_data.values.dynamic_flags,
            object_data.values.dynamic_flags
        );
        assert_eq!(
            snapshot_data.values.scale.to_bits(),
            object_data.values.scale.to_bits(),
            "the live object and snapshot use one mask/value projection"
        );
        assert_eq!(snapshot.changed_object_type_mask(), 1 << TYPEID_OBJECT);

        object.set_entry(999);
        object.set_scale(1.0);
        object.replace_all_dynamic_flags(3);
        object.reset_all_notifies();
        object.clear_map_binding();
        object.set_grid_presence(false);

        assert_eq!(snapshot.entry(), 72);
        assert!(snapshot.scale().is_nan());
        assert_eq!(snapshot.dynamic_flags(), 0x8000_0001);
        assert_eq!(snapshot.state(), captured_state);
        assert_eq!(snapshot.changed_fields(), captured_changed);
        assert_eq!(snapshot.notify_flags(), ObjectNotifyFlags::AI_RELOCATION);
        assert_eq!(snapshot.map_id(), Some(6));
        assert_eq!(snapshot.instance_id(), Some(9));
        assert!(snapshot.state().in_world);
        assert!(snapshot.state().in_grid);
        assert!(snapshot.state().is_new_object);
        assert!(snapshot.state().is_destroyed_object);
        assert!(snapshot.state().object_updated);
        assert!(captured_changed.contains(ObjectChangedFields::SCALE));
    }
}
