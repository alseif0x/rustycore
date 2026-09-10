//! Slots packets.
//!
//! Separated from item.rs under #693.

use super::*;

pub const MAX_SPECIALIZATIONS: usize = 5;

pub const INVENTORY_SLOT_BAG_0: u8 = 255;

pub const EQUIPMENT_SLOT_HEAD: u8 = 0;

pub const EQUIPMENT_SLOT_NECK: u8 = 1;

pub const EQUIPMENT_SLOT_SHOULDERS: u8 = 2;

pub const EQUIPMENT_SLOT_BODY: u8 = 3;

pub const EQUIPMENT_SLOT_CHEST: u8 = 4;

pub const EQUIPMENT_SLOT_WAIST: u8 = 5;

pub const EQUIPMENT_SLOT_LEGS: u8 = 6;

pub const EQUIPMENT_SLOT_FEET: u8 = 7;

pub const EQUIPMENT_SLOT_WRISTS: u8 = 8;

pub const EQUIPMENT_SLOT_HANDS: u8 = 9;

pub const EQUIPMENT_SLOT_FINGER1: u8 = 10;

pub const EQUIPMENT_SLOT_FINGER2: u8 = 11;

pub const EQUIPMENT_SLOT_TRINKET1: u8 = 12;

pub const EQUIPMENT_SLOT_TRINKET2: u8 = 13;

pub const EQUIPMENT_SLOT_BACK: u8 = 14;

pub const EQUIPMENT_SLOT_MAINHAND: u8 = 15;

pub const EQUIPMENT_SLOT_OFFHAND: u8 = 16;

pub const EQUIPMENT_SLOT_RANGED: u8 = 17;

pub const EQUIPMENT_SLOT_TABARD: u8 = 18;

pub const EQUIPMENT_SLOT_END: u8 = 19;

pub const PROFESSION_SLOT_PROFESSION1_TOOL: u8 = 19;

pub const PROFESSION_SLOT_PROFESSION1_GEAR1: u8 = 20;

pub const PROFESSION_SLOT_PROFESSION1_GEAR2: u8 = 21;

pub const PROFESSION_SLOT_PROFESSION2_TOOL: u8 = 22;

pub const PROFESSION_SLOT_PROFESSION2_GEAR1: u8 = 23;

pub const PROFESSION_SLOT_PROFESSION2_GEAR2: u8 = 24;

pub const PROFESSION_SLOT_COOKING_TOOL: u8 = 25;

pub const PROFESSION_SLOT_COOKING_GEAR1: u8 = 26;

pub const PROFESSION_SLOT_FISHING_TOOL: u8 = 27;

pub const PROFESSION_SLOT_FISHING_GEAR1: u8 = 28;

pub const PROFESSION_SLOT_FISHING_GEAR2: u8 = 29;

pub const PROFESSION_SLOT_START: u8 = 19;

pub const PROFESSION_SLOT_END: u8 = 30;

pub const PROFESSION_SLOT_MAX_COUNT: u8 = 3;

pub const APPEARANCE_MODIFIER_SLOT_BY_SPEC: [ItemModifier; MAX_SPECIALIZATIONS] = [
    ItemModifier::TransmogAppearanceSpec1,
    ItemModifier::TransmogAppearanceSpec2,
    ItemModifier::TransmogAppearanceSpec3,
    ItemModifier::TransmogAppearanceSpec4,
    ItemModifier::TransmogAppearanceSpec5,
];

pub const ILLUSION_MODIFIER_SLOT_BY_SPEC: [ItemModifier; MAX_SPECIALIZATIONS] = [
    ItemModifier::EnchantIllusionSpec1,
    ItemModifier::EnchantIllusionSpec2,
    ItemModifier::EnchantIllusionSpec3,
    ItemModifier::EnchantIllusionSpec4,
    ItemModifier::EnchantIllusionSpec5,
];

pub const SECONDARY_APPEARANCE_MODIFIER_SLOT_BY_SPEC: [ItemModifier; MAX_SPECIALIZATIONS] = [
    ItemModifier::TransmogSecondaryAppearanceSpec1,
    ItemModifier::TransmogSecondaryAppearanceSpec2,
    ItemModifier::TransmogSecondaryAppearanceSpec3,
    ItemModifier::TransmogSecondaryAppearanceSpec4,
    ItemModifier::TransmogSecondaryAppearanceSpec5,
];

pub(super) fn spec_modifier(
    active_talent_group: usize,
    slots: &[ItemModifier; MAX_SPECIALIZATIONS],
) -> ItemModifier {
    slots.get(active_talent_group).copied().unwrap_or(slots[0])
}
