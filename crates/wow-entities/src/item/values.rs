//! Values packets.
//!
//! Separated from item.rs under #693.

use super::*;

pub const MAX_ITEM_SPELLS: usize = 5;

pub const MAX_ENCHANTMENT_SLOT: usize = 13;

pub const MAX_INSPECTED_ENCHANTMENT_SLOT: usize = 8;

pub const ITEM_MODIFIER_COUNT: usize = 58;

pub const ITEM_DATA_PARENT_BIT: usize = 0;

pub const ITEM_DATA_ARTIFACT_POWERS_BIT: usize = 1;

pub const ITEM_DATA_GEMS_BIT: usize = 2;

pub const ITEM_DATA_OWNER_BIT: usize = 3;

pub const ITEM_DATA_CONTAINED_IN_BIT: usize = 4;

pub const ITEM_DATA_CREATOR_BIT: usize = 5;

pub const ITEM_DATA_GIFT_CREATOR_BIT: usize = 6;

pub const ITEM_DATA_STACK_COUNT_BIT: usize = 7;

pub const ITEM_DATA_EXPIRATION_BIT: usize = 8;

pub const ITEM_DATA_DYNAMIC_FLAGS_BIT: usize = 9;

pub const ITEM_DATA_PROPERTY_SEED_BIT: usize = 10;

pub const ITEM_DATA_RANDOM_PROPERTIES_ID_BIT: usize = 11;

pub const ITEM_DATA_DURABILITY_BIT: usize = 12;

pub const ITEM_DATA_MAX_DURABILITY_BIT: usize = 13;

pub const ITEM_DATA_CREATE_PLAYED_TIME_BIT: usize = 14;

pub const ITEM_DATA_CONTEXT_BIT: usize = 15;

pub const ITEM_DATA_CREATE_TIME_BIT: usize = 16;

pub const ITEM_DATA_ARTIFACT_XP_BIT: usize = 17;

pub const ITEM_DATA_ITEM_APPEARANCE_MOD_ID_BIT: usize = 18;

pub const ITEM_DATA_MODIFIERS_BIT: usize = 19;

pub const ITEM_DATA_DYNAMIC_FLAGS2_BIT: usize = 20;

pub const ITEM_DATA_ITEM_BONUS_KEY_BIT: usize = 21;

pub const ITEM_DATA_DEBUG_ITEM_LEVEL_BIT: usize = 22;

pub const ITEM_DATA_SPELL_CHARGES_PARENT_BIT: usize = 23;

pub const ITEM_DATA_SPELL_CHARGES_FIRST_BIT: usize = 24;

pub const ITEM_DATA_ENCHANTMENT_PARENT_BIT: usize = 29;

pub const ITEM_DATA_ENCHANTMENT_FIRST_BIT: usize = 30;

pub const ITEM_DATA_BASE_ALLOWED_MASK: [u32; 2] = [0xE029_CE7F, 0x0000_07FF];

pub const ITEM_DATA_OWNER_ALLOWED_MASK: [u32; 2] = [0x1FD6_3180, 0x0000_0000];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDataValues {
    pub artifact_powers: Vec<ArtifactPower>,
    pub gems: Vec<SocketedGem>,
    pub owner: ObjectGuid,
    pub contained_in: ObjectGuid,
    pub creator: ObjectGuid,
    pub gift_creator: ObjectGuid,
    pub stack_count: u32,
    pub expiration: u32,
    pub dynamic_flags: u32,
    pub property_seed: i32,
    pub random_properties_id: i32,
    pub durability: u32,
    pub max_durability: u32,
    pub create_played_time: u32,
    pub context: i32,
    pub create_time: i64,
    pub artifact_xp: u64,
    pub item_appearance_mod_id: u8,
    pub dynamic_flags2: u32,
    pub modifiers: [u32; ITEM_MODIFIER_COUNT],
    pub item_bonus_key: ItemBonusKey,
    pub debug_item_level: u16,
    pub spell_charges: [i32; MAX_ITEM_SPELLS],
    pub enchantments: [ItemEnchantment; MAX_ENCHANTMENT_SLOT],
}

impl Default for ItemDataValues {
    fn default() -> Self {
        Self {
            artifact_powers: Vec::new(),
            gems: Vec::new(),
            owner: ObjectGuid::EMPTY,
            contained_in: ObjectGuid::EMPTY,
            creator: ObjectGuid::EMPTY,
            gift_creator: ObjectGuid::EMPTY,
            stack_count: 0,
            expiration: 0,
            dynamic_flags: 0,
            property_seed: 0,
            random_properties_id: 0,
            durability: 0,
            max_durability: 0,
            create_played_time: 0,
            context: ItemContext::None as i32,
            create_time: 0,
            artifact_xp: 0,
            item_appearance_mod_id: 0,
            dynamic_flags2: 0,
            modifiers: [0; ITEM_MODIFIER_COUNT],
            item_bonus_key: ItemBonusKey::default(),
            debug_item_level: 0,
            spell_charges: [0; MAX_ITEM_SPELLS],
            enchantments: [ItemEnchantment::default(); MAX_ENCHANTMENT_SLOT],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemDataUpdate {
    pub mask: UpdateMask,
    pub values: ItemDataValues,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataUpdate>,
    pub item_data: Option<ItemDataUpdate>,
}

impl ItemValuesUpdate {
    pub const fn has_data(&self) -> bool {
        self.changed_object_type_mask != 0
    }
}
