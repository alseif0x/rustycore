use std::time::{SystemTime, UNIX_EPOCH};

use wow_constants::{TypeId, TypeMask};
use wow_core::ObjectGuid;

use crate::{
    CreateObjectFlags, ObjectDataUpdate, UpdateMask, WorldObject,
    update_fields::{CORPSE_DATA_BITS, TYPEID_CORPSE},
};

pub const CORPSE_ITEMS: usize = 19;
pub const CORPSE_BONES_EXPIRE_SECS: i64 = 60 * 60;
pub const CORPSE_RESURRECTABLE_EXPIRE_SECS: i64 = 3 * 24 * 60 * 60;

pub const CORPSE_DATA_PARENT_BIT: usize = 0;
pub const CORPSE_DATA_CUSTOMIZATIONS_BIT: usize = 1;
pub const CORPSE_DATA_DYNAMIC_FLAGS_BIT: usize = 2;
pub const CORPSE_DATA_OWNER_BIT: usize = 3;
pub const CORPSE_DATA_PARTY_GUID_BIT: usize = 4;
pub const CORPSE_DATA_GUILD_GUID_BIT: usize = 5;
pub const CORPSE_DATA_DISPLAY_ID_BIT: usize = 6;
pub const CORPSE_DATA_RACE_ID_BIT: usize = 7;
pub const CORPSE_DATA_SEX_BIT: usize = 8;
pub const CORPSE_DATA_CLASS_BIT: usize = 9;
pub const CORPSE_DATA_FLAGS_BIT: usize = 10;
pub const CORPSE_DATA_FACTION_TEMPLATE_BIT: usize = 11;
pub const CORPSE_DATA_ITEMS_PARENT_BIT: usize = 12;
pub const CORPSE_DATA_ITEMS_FIRST_BIT: usize = 13;

pub const CORPSE_DYNFLAG_LOOTABLE: u32 = 0x0001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CorpseType {
    Bones = 0,
    ResurrectablePve = 1,
    ResurrectablePvp = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpseCustomizationChoice {
    pub option_id: u32,
    pub choice_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CorpseDataValues {
    pub customizations: Vec<CorpseCustomizationChoice>,
    pub dynamic_flags: u32,
    pub owner: ObjectGuid,
    pub party_guid: ObjectGuid,
    pub guild_guid: ObjectGuid,
    pub display_id: u32,
    pub race_id: u8,
    pub sex: u8,
    pub class: u8,
    pub flags: u32,
    pub faction_template: i32,
    pub items: [u32; CORPSE_ITEMS],
}

impl Default for CorpseDataValues {
    fn default() -> Self {
        Self {
            customizations: Vec::new(),
            dynamic_flags: 0,
            owner: ObjectGuid::EMPTY,
            party_guid: ObjectGuid::EMPTY,
            guild_guid: ObjectGuid::EMPTY,
            display_id: 0,
            race_id: 0,
            sex: 0,
            class: 0,
            flags: 0,
            faction_template: 0,
            items: [0; CORPSE_ITEMS],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CorpseDataUpdate {
    pub mask: UpdateMask,
    pub values: CorpseDataValues,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CorpseValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataUpdate>,
    pub corpse_data: Option<CorpseDataUpdate>,
}

impl CorpseValuesUpdate {
    pub const fn has_data(&self) -> bool {
        self.changed_object_type_mask != 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Corpse {
    world: WorldObject,
    data: CorpseDataValues,
    corpse_data_changes: UpdateMask,
    corpse_type: CorpseType,
    ghost_time: i64,
    cell_coord: Option<(u32, u32)>,
    grid_unload_cleanup_before_delete_count: u32,
    grid_unload_delete_requested: bool,
}

impl Corpse {
    pub fn new(corpse_type: CorpseType) -> Self {
        Self::new_at(corpse_type, unix_now_secs())
    }

    pub fn new_at(corpse_type: CorpseType, ghost_time: i64) -> Self {
        let mut world = WorldObject::new(
            corpse_type != CorpseType::Bones,
            TypeId::Corpse,
            TypeMask::OBJECT | TypeMask::CORPSE,
        );
        world
            .object_mut()
            .create_flags_mut()
            .insert(CreateObjectFlags::STATIONARY);

        Self {
            world,
            data: CorpseDataValues::default(),
            corpse_data_changes: UpdateMask::new(CORPSE_DATA_BITS),
            corpse_type,
            ghost_time,
            cell_coord: None,
            grid_unload_cleanup_before_delete_count: 0,
            grid_unload_delete_requested: false,
        }
    }

    pub const fn world(&self) -> &WorldObject {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut WorldObject {
        &mut self.world
    }

    pub const fn data(&self) -> &CorpseDataValues {
        &self.data
    }

    pub fn corpse_data_changes_mask(&self) -> &UpdateMask {
        &self.corpse_data_changes
    }

    pub fn clear_corpse_data_changes(&mut self) {
        self.corpse_data_changes.reset_all();
    }

    pub const fn corpse_type(&self) -> CorpseType {
        self.corpse_type
    }

    pub const fn ghost_time(&self) -> i64 {
        self.ghost_time
    }

    pub fn reset_ghost_time(&mut self, ghost_time: i64) {
        self.ghost_time = ghost_time;
    }

    pub const fn cell_coord(&self) -> Option<(u32, u32)> {
        self.cell_coord
    }

    pub fn set_cell_coord(&mut self, x: u32, y: u32) {
        self.cell_coord = Some((x, y));
    }

    pub const fn cleanup_before_delete_count(&self) -> u32 {
        self.grid_unload_cleanup_before_delete_count
    }

    pub const fn grid_unload_delete_requested(&self) -> bool {
        self.grid_unload_delete_requested
    }

    pub fn set_destroyed_object(&mut self, destroyed: bool) {
        self.world.object_mut().set_destroyed_object(destroyed);
    }

    pub fn cleanup_before_delete(&mut self) {
        self.grid_unload_cleanup_before_delete_count = self
            .grid_unload_cleanup_before_delete_count
            .saturating_add(1);
    }

    pub fn request_delete_from_grid_unload(&mut self) {
        self.grid_unload_delete_requested = true;
        self.cell_coord = None;
        self.world.clear_current_cell();
    }

    pub fn is_expired(&self, now: i64) -> bool {
        if self.corpse_type == CorpseType::Bones {
            self.ghost_time < now - CORPSE_BONES_EXPIRE_SECS
        } else {
            self.ghost_time < now - CORPSE_RESURRECTABLE_EXPIRE_SECS
        }
    }

    pub fn set_corpse_dynamic_flag(&mut self, flag: u32) {
        self.replace_all_corpse_dynamic_flags(self.data.dynamic_flags | flag);
    }

    /// Replace the C++ `CorpseData::Customizations` dynamic field.
    ///
    /// `Map::LoadCorpseData` uses this after loading all
    /// `corpse_customizations` rows for one corpse.
    pub fn set_customizations(&mut self, customizations: Vec<CorpseCustomizationChoice>) {
        if self.data.customizations != customizations {
            self.data.customizations = customizations;
            self.mark_corpse_data(CORPSE_DATA_CUSTOMIZATIONS_BIT);
        }
    }

    pub fn remove_corpse_dynamic_flag(&mut self, flag: u32) {
        self.replace_all_corpse_dynamic_flags(self.data.dynamic_flags & !flag);
    }

    pub fn replace_all_corpse_dynamic_flags(&mut self, flags: u32) {
        self.set_u32_field(CORPSE_DATA_DYNAMIC_FLAGS_BIT, flags, |data| {
            &mut data.dynamic_flags
        });
    }

    pub fn set_owner_guid(&mut self, owner: ObjectGuid) {
        self.set_guid_field(CORPSE_DATA_OWNER_BIT, owner, |data| &mut data.owner);
    }

    pub fn set_party_guid(&mut self, party_guid: ObjectGuid) {
        self.set_guid_field(CORPSE_DATA_PARTY_GUID_BIT, party_guid, |data| {
            &mut data.party_guid
        });
    }

    pub fn set_guild_guid(&mut self, guild_guid: ObjectGuid) {
        self.set_guid_field(CORPSE_DATA_GUILD_GUID_BIT, guild_guid, |data| {
            &mut data.guild_guid
        });
    }

    pub fn set_display_id(&mut self, display_id: u32) {
        self.set_u32_field(CORPSE_DATA_DISPLAY_ID_BIT, display_id, |data| {
            &mut data.display_id
        });
    }

    pub fn set_race(&mut self, race: u8) {
        self.set_u8_field(CORPSE_DATA_RACE_ID_BIT, race, |data| &mut data.race_id);
    }

    pub fn set_class(&mut self, class: u8) {
        self.set_u8_field(CORPSE_DATA_CLASS_BIT, class, |data| &mut data.class);
    }

    pub fn set_sex(&mut self, sex: u8) {
        self.set_u8_field(CORPSE_DATA_SEX_BIT, sex, |data| &mut data.sex);
    }

    pub fn replace_all_flags(&mut self, flags: u32) {
        self.set_u32_field(CORPSE_DATA_FLAGS_BIT, flags, |data| &mut data.flags);
    }

    pub fn set_faction_template(&mut self, faction_template: i32) {
        self.set_i32_field(CORPSE_DATA_FACTION_TEMPLATE_BIT, faction_template, |data| {
            &mut data.faction_template
        });
    }

    pub fn set_faction(&mut self, faction: u32) {
        self.set_faction_template(faction as i32);
    }

    pub fn set_item(&mut self, slot: usize, item: u32) {
        if slot >= CORPSE_ITEMS || self.data.items[slot] == item {
            return;
        }

        self.data.items[slot] = item;
        self.mark_corpse_data_array(
            CORPSE_DATA_ITEMS_PARENT_BIT,
            CORPSE_DATA_ITEMS_FIRST_BIT,
            slot,
        );
    }

    pub fn changed_object_type_mask(&self) -> u32 {
        self.world.object().changed_object_type_mask()
            | if self.corpse_data_changes.is_any_set() {
                1 << TYPEID_CORPSE
            } else {
                0
            }
    }

    pub fn values_update(&self) -> CorpseValuesUpdate {
        let object_update = self.world.object().values_update();
        CorpseValuesUpdate {
            changed_object_type_mask: self.changed_object_type_mask(),
            object_data: object_update.object_data,
            corpse_data: self
                .corpse_data_changes
                .is_any_set()
                .then(|| CorpseDataUpdate {
                    mask: self.corpse_data_changes.clone(),
                    values: self.data.clone(),
                }),
        }
    }

    fn set_u32_field(
        &mut self,
        bit: usize,
        value: u32,
        field: impl FnOnce(&mut CorpseDataValues) -> &mut u32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_corpse_data(bit);
        }
    }

    fn set_i32_field(
        &mut self,
        bit: usize,
        value: i32,
        field: impl FnOnce(&mut CorpseDataValues) -> &mut i32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_corpse_data(bit);
        }
    }

    fn set_u8_field(
        &mut self,
        bit: usize,
        value: u8,
        field: impl FnOnce(&mut CorpseDataValues) -> &mut u8,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_corpse_data(bit);
        }
    }

    fn set_guid_field(
        &mut self,
        bit: usize,
        value: ObjectGuid,
        field: impl FnOnce(&mut CorpseDataValues) -> &mut ObjectGuid,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_corpse_data(bit);
        }
    }

    fn mark_corpse_data(&mut self, bit: usize) {
        self.corpse_data_changes.set(CORPSE_DATA_PARENT_BIT);
        self.corpse_data_changes.set(bit);
    }

    fn mark_corpse_data_array(
        &mut self,
        parent_bit: usize,
        first_element_bit: usize,
        index: usize,
    ) {
        self.corpse_data_changes.set(parent_bit);
        self.corpse_data_changes.set(first_element_bit + index);
    }
}

fn unix_now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpse_constructor_matches_cpp_base_state() {
        let bones = Corpse::new_at(CorpseType::Bones, 1_000);
        assert_eq!(bones.world().object().type_id(), TypeId::Corpse);
        assert_eq!(
            bones.world().object().type_mask(),
            TypeMask::OBJECT | TypeMask::CORPSE
        );
        assert!(!bones.world().is_world_object());
        assert!(
            bones
                .world()
                .object()
                .create_flags()
                .contains(CreateObjectFlags::STATIONARY)
        );
        assert_eq!(bones.corpse_type(), CorpseType::Bones);
        assert_eq!(bones.ghost_time(), 1_000);
        assert_eq!(bones.cell_coord(), None);
        assert_eq!(bones.cleanup_before_delete_count(), 0);
        assert!(!bones.grid_unload_delete_requested());
        assert!(!bones.corpse_data_changes_mask().is_any_set());

        let resurrectable = Corpse::new_at(CorpseType::ResurrectablePve, 1_000);
        assert!(resurrectable.world().is_world_object());
    }

    #[test]
    fn corpse_data_setters_mark_cpp_bits() {
        let mut corpse = Corpse::new_at(CorpseType::ResurrectablePvp, 10);
        let owner = ObjectGuid::new(1, 2);
        let party = ObjectGuid::new(3, 4);
        let guild = ObjectGuid::new(5, 6);

        corpse.set_owner_guid(owner);
        corpse.set_customizations(vec![CorpseCustomizationChoice {
            option_id: 12,
            choice_id: 34,
        }]);
        corpse.set_party_guid(party);
        corpse.set_guild_guid(guild);
        corpse.set_display_id(1234);
        corpse.set_race(1);
        corpse.set_class(2);
        corpse.set_sex(1);
        corpse.replace_all_flags(0x40);
        corpse.set_faction(35);
        corpse.set_corpse_dynamic_flag(CORPSE_DYNFLAG_LOOTABLE);
        corpse.set_item(3, 777);

        assert_eq!(corpse.data().owner, owner);
        assert_eq!(
            corpse.data().customizations,
            vec![CorpseCustomizationChoice {
                option_id: 12,
                choice_id: 34,
            }]
        );
        assert_eq!(corpse.data().party_guid, party);
        assert_eq!(corpse.data().guild_guid, guild);
        assert_eq!(corpse.data().display_id, 1234);
        assert_eq!(corpse.data().race_id, 1);
        assert_eq!(corpse.data().class, 2);
        assert_eq!(corpse.data().sex, 1);
        assert_eq!(corpse.data().flags, 0x40);
        assert_eq!(corpse.data().faction_template, 35);
        assert_eq!(corpse.data().dynamic_flags, CORPSE_DYNFLAG_LOOTABLE);
        assert_eq!(corpse.data().items[3], 777);
        assert!(
            corpse
                .corpse_data_changes_mask()
                .is_set(CORPSE_DATA_PARENT_BIT)
        );
        assert!(
            corpse
                .corpse_data_changes_mask()
                .is_set(CORPSE_DATA_OWNER_BIT)
        );
        assert!(
            corpse
                .corpse_data_changes_mask()
                .is_set(CORPSE_DATA_CUSTOMIZATIONS_BIT)
        );
        assert!(
            corpse
                .corpse_data_changes_mask()
                .is_set(CORPSE_DATA_ITEMS_PARENT_BIT)
        );
        assert!(
            corpse
                .corpse_data_changes_mask()
                .is_set(CORPSE_DATA_ITEMS_FIRST_BIT + 3)
        );

        corpse.remove_corpse_dynamic_flag(CORPSE_DYNFLAG_LOOTABLE);
        assert_eq!(corpse.data().dynamic_flags, 0);
    }

    #[test]
    fn corpse_expiration_matches_cpp_thresholds() {
        let bones = Corpse::new_at(CorpseType::Bones, 1_000);
        assert!(!bones.is_expired(1_000 + CORPSE_BONES_EXPIRE_SECS));
        assert!(bones.is_expired(1_000 + CORPSE_BONES_EXPIRE_SECS + 1));

        let resurrectable = Corpse::new_at(CorpseType::ResurrectablePve, 1_000);
        assert!(!resurrectable.is_expired(1_000 + CORPSE_RESURRECTABLE_EXPIRE_SECS));
        assert!(resurrectable.is_expired(1_000 + CORPSE_RESURRECTABLE_EXPIRE_SECS + 1));
    }

    #[test]
    fn corpse_values_update_sets_type_bit() {
        let mut corpse = Corpse::new_at(CorpseType::Bones, 10);

        corpse.set_display_id(1234);
        let update = corpse.values_update();

        assert!(update.has_data());
        assert_eq!(update.changed_object_type_mask, 1 << TYPEID_CORPSE);
        let corpse_data = update.corpse_data.unwrap();
        assert_eq!(corpse_data.values.display_id, 1234);
        assert!(corpse_data.mask.is_set(CORPSE_DATA_DISPLAY_ID_BIT));
    }

    #[test]
    fn corpse_grid_unload_helpers_apply_represented_state() {
        let mut corpse = Corpse::new_at(CorpseType::Bones, 10);
        corpse.set_cell_coord(1, 2);
        corpse.world_mut().set_current_cell(1, 2);

        corpse.set_destroyed_object(true);
        corpse.cleanup_before_delete();
        corpse.request_delete_from_grid_unload();

        assert!(corpse.world().object().is_destroyed_object());
        assert_eq!(corpse.cleanup_before_delete_count(), 1);
        assert!(corpse.grid_unload_delete_requested());
        assert_eq!(corpse.cell_coord(), None);
        assert_eq!(corpse.world().current_cell(), None);
        assert!(!corpse.world().object().is_in_grid());
    }
}
