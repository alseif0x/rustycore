//! Player update-field values and schema bit positions.

use super::{
    BUYBACK_SLOT_COUNT, EQUIPMENT_SLOT_END, Gender, ObjectDataUpdate, ObjectGuid, PLAYER_SLOT_END,
    Player, UnitDataUpdate, UpdateMask,
};

pub const PLAYER_DATA_PARENT_BIT: usize = 0;
pub const PLAYER_DATA_LOOT_TARGET_GUID_BIT: usize = 6;
pub const PLAYER_DATA_FLAGS_BIT: usize = 7;
pub const PLAYER_DATA_FLAGS_EX_BIT: usize = 8;
pub const PLAYER_DATA_PARTY_TYPE_PARENT_BIT: usize = 32;
pub const PLAYER_DATA_PARTY_TYPE_FIRST_BIT: usize = 33;
pub const PLAYER_DATA_NUM_BANK_SLOTS_BIT: usize = 12;
pub const PLAYER_DATA_NATIVE_SEX_BIT: usize = 13;
pub const PLAYER_DATA_INEBRIATION_BIT: usize = 14;
pub const PLAYER_DATA_PLAYER_TITLE_BIT: usize = 21;
pub const PLAYER_DATA_CURRENT_SPEC_ID_BIT: usize = 24;
pub const PLAYER_DATA_CURRENT_BATTLE_PET_BREED_QUALITY_BIT: usize = 26;
pub const PLAYER_DATA_HONOR_LEVEL_BIT: usize = 27;
pub const PLAYER_DATA_VISIBLE_ITEMS_PARENT_BIT: usize = 61;
pub const PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT: usize = 62;

pub const ACTIVE_PLAYER_DATA_PARENT_BIT: usize = 0;
pub const ACTIVE_PLAYER_DATA_FARSIGHT_OBJECT_BIT: usize = 26;
pub const ACTIVE_PLAYER_DATA_SUMMONED_BATTLE_PET_GUID_BIT: usize = 27;
pub const ACTIVE_PLAYER_DATA_COINAGE_BIT: usize = 28;
pub const ACTIVE_PLAYER_DATA_XP_BIT: usize = 29;
pub const ACTIVE_PLAYER_DATA_NEXT_LEVEL_XP_BIT: usize = 30;
pub const ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_PARENT_BIT: usize = 70;
pub const ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_BIT: usize = 94;
pub const ACTIVE_PLAYER_DATA_CHARACTER_POINTS_BIT: usize = 33;
pub const ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT: usize = 7;
pub const ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT: usize = 8;
pub const ACTIVE_PLAYER_DATA_TOYS_BIT: usize = 9;
pub const ACTIVE_PLAYER_DATA_TRANSMOG_BIT: usize = 10;
pub const ACTIVE_PLAYER_DATA_CONDITIONAL_TRANSMOG_BIT: usize = 11;
pub const ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT: usize = 102;
pub const ACTIVE_PLAYER_DATA_HONOR_BIT: usize = 109;
pub const ACTIVE_PLAYER_DATA_HONOR_NEXT_LEVEL_BIT: usize = 110;
pub const ACTIVE_PLAYER_DATA_NUM_BACKPACK_SLOTS_BIT: usize = 104;
pub const ACTIVE_PLAYER_DATA_INV_SLOTS_PARENT_BIT: usize = 124;
pub const ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT: usize = 125;
pub const ACTIVE_PLAYER_DATA_EXPLORED_ZONES_PARENT_BIT: usize = 298;
pub const ACTIVE_PLAYER_DATA_EXPLORED_ZONES_FIRST_BIT: usize = 299;
pub const ACTIVE_PLAYER_DATA_REST_INFO_PARENT_BIT: usize = 539;
pub const ACTIVE_PLAYER_DATA_REST_INFO_FIRST_BIT: usize = 540;
pub const ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT: usize = 549;
pub const ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT: usize = 550;
pub const ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT: usize = 562;
pub const ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_PARENT_BIT: usize = 628;
pub const ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_FIRST_BIT: usize = 629;
pub const ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT: usize = 636;
pub const ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT: usize = 637;
pub const ACTIVE_PLAYER_DATA_WATCHED_FACTION_INDEX_BIT: usize = 92;
pub const QUESTS_COMPLETED_BITS_SIZE: usize = 875;
pub const QUESTS_COMPLETED_BITS_PER_BLOCK: u32 = 64;
pub const PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP: usize = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisibleItemValues {
    pub item_id: i32,
    pub item_appearance_mod_id: u16,
    pub item_visual: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerDataValues {
    pub loot_target_guid: ObjectGuid,
    pub player_flags: u32,
    pub player_flags_ex: u32,
    pub party_type: [u8; 2],
    pub num_bank_slots: u8,
    pub native_sex: u8,
    pub inebriation: u8,
    pub player_title: i32,
    pub current_spec_id: u32,
    pub current_battle_pet_breed_quality: u8,
    pub honor_level: i32,
    pub visible_items: [VisibleItemValues; EQUIPMENT_SLOT_END as usize],
}

impl Default for PlayerDataValues {
    fn default() -> Self {
        Self {
            loot_target_guid: ObjectGuid::EMPTY,
            player_flags: 0,
            player_flags_ex: 0,
            party_type: [0; 2],
            num_bank_slots: 0,
            native_sex: Gender::Male as u8,
            inebriation: 0,
            player_title: 0,
            current_spec_id: 0,
            current_battle_pet_breed_quality: 0,
            honor_level: 0,
            visible_items: [VisibleItemValues::default(); EQUIPMENT_SLOT_END as usize],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivePlayerDataValues {
    pub farsight_object: ObjectGuid,
    pub summoned_battle_pet_guid: ObjectGuid,
    pub coinage: u64,
    pub xp: i32,
    pub next_level_xp: i32,
    pub character_points: i32,
    pub honor: i32,
    pub honor_next_level: i32,
    pub watched_faction_index: i32,
    pub scaling_player_level_delta: i32,
    pub num_backpack_slots: u8,
    pub inv_slots: [ObjectGuid; PLAYER_SLOT_END],
    pub explored_zones: [u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
    pub rest_info: [PlayerRestInfoValueLikeCpp; 2],
    pub buyback_price: [u32; BUYBACK_SLOT_COUNT],
    pub buyback_timestamp: [i64; BUYBACK_SLOT_COUNT],
    pub bank_bag_slot_flags: [u32; 7],
    pub heirlooms: Vec<i32>,
    pub heirlooms_update_mask: Option<Vec<u32>>,
    pub heirloom_flags: Vec<u32>,
    pub heirloom_flags_update_mask: Option<Vec<u32>>,
    pub toys: Vec<i32>,
    pub toys_update_mask: Option<Vec<u32>>,
    pub transmog: Vec<u32>,
    pub transmog_update_mask: Option<Vec<u32>>,
    pub conditional_transmog: Vec<i32>,
    pub conditional_transmog_update_mask: Option<Vec<u32>>,
    pub quest_completed: [u64; QUESTS_COMPLETED_BITS_SIZE],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerRestInfoValueLikeCpp {
    pub threshold: u32,
    pub state_id: u8,
}

impl Default for ActivePlayerDataValues {
    fn default() -> Self {
        Self {
            farsight_object: ObjectGuid::EMPTY,
            summoned_battle_pet_guid: ObjectGuid::EMPTY,
            coinage: 0,
            xp: 0,
            next_level_xp: 0,
            character_points: 0,
            honor: 0,
            honor_next_level: 0,
            watched_faction_index: -1,
            scaling_player_level_delta: 0,
            num_backpack_slots: 0,
            inv_slots: [ObjectGuid::EMPTY; PLAYER_SLOT_END],
            explored_zones: [0; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
            rest_info: [PlayerRestInfoValueLikeCpp::default(); 2],
            buyback_price: [0; BUYBACK_SLOT_COUNT],
            buyback_timestamp: [0; BUYBACK_SLOT_COUNT],
            bank_bag_slot_flags: [0; 7],
            heirlooms: Vec::new(),
            heirlooms_update_mask: None,
            heirloom_flags: Vec::new(),
            heirloom_flags_update_mask: None,
            toys: Vec::new(),
            toys_update_mask: None,
            transmog: Vec::new(),
            transmog_update_mask: None,
            conditional_transmog: Vec::new(),
            conditional_transmog_update_mask: None,
            quest_completed: [0; QUESTS_COMPLETED_BITS_SIZE],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDataUpdate {
    pub mask: UpdateMask,
    pub values: PlayerDataValues,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivePlayerDataUpdate {
    pub mask: UpdateMask,
    pub values: ActivePlayerDataValues,
    pub rest_info_change_masks: [u8; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataUpdate>,
    pub unit_data: Option<UnitDataUpdate>,
    pub player_data: Option<PlayerDataUpdate>,
    pub active_player_data: Option<ActivePlayerDataUpdate>,
}

impl PlayerValuesUpdate {
    pub const fn has_data(&self) -> bool {
        self.changed_object_type_mask != 0
    }
}

impl Player {
    pub fn player_data_changes_mask(&self) -> &UpdateMask {
        &self.player_data_changes
    }

    pub fn active_player_data_changes_mask(&self) -> &UpdateMask {
        &self.active_player_data_changes
    }

    pub fn clear_player_data_changes(&mut self) {
        self.player_data_changes.reset_all();
    }

    pub fn clear_active_player_data_changes(&mut self) {
        self.active_player_data_changes.reset_all();
        self.rest_info_change_masks = [0; 2];
    }

    pub fn clear_data_changes(&mut self) {
        self.clear_player_data_changes();
        self.clear_active_player_data_changes();
        self.unit.clear_unit_data_changes();
        self.unit.world_mut().object_mut().clear_update_mask(false);
    }

    pub const fn inebriation_like_cpp(&self) -> u8 {
        self.data.inebriation
    }

    pub fn set_inebriation_like_cpp(&mut self, value: u8) {
        self.set_player_u8(PLAYER_DATA_INEBRIATION_BIT, value.min(100), |data| {
            &mut data.inebriation
        });
    }

    pub fn set_player_flag(&mut self, flag: u32) {
        self.replace_all_player_flags(self.data.player_flags | flag);
    }

    pub fn remove_player_flag(&mut self, flag: u32) {
        self.replace_all_player_flags(self.data.player_flags & !flag);
    }

    pub fn has_player_flag(&self, flag: u32) -> bool {
        (self.data.player_flags & flag) != 0
    }

    pub fn set_player_flag_ex(&mut self, flag: u32) {
        self.replace_all_player_flags_ex(self.data.player_flags_ex | flag);
    }

    pub fn remove_player_flag_ex(&mut self, flag: u32) {
        self.replace_all_player_flags_ex(self.data.player_flags_ex & !flag);
    }

    pub fn has_player_flag_ex(&self, flag: u32) -> bool {
        (self.data.player_flags_ex & flag) != 0
    }

    pub fn set_primary_specialization(&mut self, spec: u32) {
        self.set_player_u32(PLAYER_DATA_CURRENT_SPEC_ID_BIT, spec, |data| {
            &mut data.current_spec_id
        });
    }

    pub fn set_farsight_object_like_cpp(&mut self, guid: ObjectGuid) {
        self.set_active_guid(ACTIVE_PLAYER_DATA_FARSIGHT_OBJECT_BIT, guid, |data| {
            &mut data.farsight_object
        });
        self.unit_mut()
            .set_seer_can_always_see_target_guid_like_cpp(guid);
    }

    pub fn set_honor_like_cpp(&mut self, honor: i32) {
        self.set_active_i32_in_section(
            ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT,
            ACTIVE_PLAYER_DATA_HONOR_BIT,
            honor,
            |data| &mut data.honor,
        );
    }

    pub fn set_free_primary_professions(&mut self, points: u16) {
        self.set_active_i32(
            ACTIVE_PLAYER_DATA_CHARACTER_POINTS_BIT,
            i32::from(points),
            |data| &mut data.character_points,
        );
    }

    /// Set C++ `UF::ActivePlayerData::CharacterPoints` without narrowing the
    /// signed update-field value used by talent initialization.
    pub fn set_character_points_like_cpp(&mut self, points: i32) {
        self.set_active_i32(ACTIVE_PLAYER_DATA_CHARACTER_POINTS_BIT, points, |data| {
            &mut data.character_points
        });
    }

    pub fn set_buyback_price(&mut self, slot: usize, price: u32) {
        if slot >= BUYBACK_SLOT_COUNT || self.active_data.buyback_price[slot] == price {
            return;
        }

        self.active_data.buyback_price[slot] = price;
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT,
            slot,
        );
    }

    pub fn mark_buyback_price_changed(&mut self, slot: usize) {
        if slot >= BUYBACK_SLOT_COUNT {
            return;
        }

        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT,
            slot,
        );
    }

    pub fn set_buyback_timestamp(&mut self, slot: usize, timestamp: i64) {
        if slot >= BUYBACK_SLOT_COUNT || self.active_data.buyback_timestamp[slot] == timestamp {
            return;
        }

        self.active_data.buyback_timestamp[slot] = timestamp;
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT,
            slot,
        );
    }

    pub fn mark_buyback_timestamp_changed(&mut self, slot: usize) {
        if slot >= BUYBACK_SLOT_COUNT {
            return;
        }

        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT,
            slot,
        );
    }

    pub(super) fn set_player_u32(
        &mut self,
        bit: usize,
        value: u32,
        field: impl FnOnce(&mut PlayerDataValues) -> &mut u32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_player_data(bit);
        }
    }

    pub(super) fn set_player_i32(
        &mut self,
        bit: usize,
        value: i32,
        field: impl FnOnce(&mut PlayerDataValues) -> &mut i32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_player_data(bit);
        }
    }

    pub(super) fn set_player_u8(
        &mut self,
        bit: usize,
        value: u8,
        field: impl FnOnce(&mut PlayerDataValues) -> &mut u8,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_player_data(bit);
        }
    }

    pub(super) fn set_active_u64(
        &mut self,
        bit: usize,
        value: u64,
        field: impl FnOnce(&mut ActivePlayerDataValues) -> &mut u64,
    ) {
        let target = field(&mut self.active_data);
        if *target != value {
            *target = value;
            self.mark_active_player_data(bit);
        }
    }

    pub(super) fn set_active_i32(
        &mut self,
        bit: usize,
        value: i32,
        field: impl FnOnce(&mut ActivePlayerDataValues) -> &mut i32,
    ) {
        let target = field(&mut self.active_data);
        if *target != value {
            *target = value;
            self.mark_active_player_data(bit);
        }
    }

    pub(super) fn set_active_i32_in_section(
        &mut self,
        parent_bit: usize,
        bit: usize,
        value: i32,
        field: impl FnOnce(&mut ActivePlayerDataValues) -> &mut i32,
    ) {
        let target = field(&mut self.active_data);
        if *target != value {
            *target = value;
            self.mark_active_player_data_section(parent_bit, bit);
        }
    }

    pub(super) fn set_active_u8(
        &mut self,
        bit: usize,
        value: u8,
        field: impl FnOnce(&mut ActivePlayerDataValues) -> &mut u8,
    ) {
        let target = field(&mut self.active_data);
        if *target != value {
            *target = value;
            self.mark_active_player_data(bit);
        }
    }

    pub(super) fn mark_player_data(&mut self, bit: usize) {
        self.player_data_changes.set(PLAYER_DATA_PARENT_BIT);
        self.player_data_changes.set(bit);
    }

    pub(super) fn mark_player_data_array(
        &mut self,
        parent_bit: usize,
        first_element_bit: usize,
        index: usize,
    ) {
        self.player_data_changes.set(parent_bit);
        self.player_data_changes.set(first_element_bit + index);
    }

    pub(super) fn mark_active_player_data(&mut self, bit: usize) {
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_PARENT_BIT);
        self.active_player_data_changes.set(bit);
    }

    pub(super) fn mark_active_player_data_section(&mut self, parent_bit: usize, bit: usize) {
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_PARENT_BIT);
        self.active_player_data_changes.set(parent_bit);
        self.active_player_data_changes.set(bit);
    }

    pub(super) fn mark_active_player_data_array(
        &mut self,
        parent_bit: usize,
        first_element_bit: usize,
        index: usize,
    ) {
        self.active_player_data_changes.set(parent_bit);
        self.active_player_data_changes
            .set(first_element_bit + index);
    }
}
