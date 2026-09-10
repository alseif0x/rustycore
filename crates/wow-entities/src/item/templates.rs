//! Templates packets.
//!
//! Separated from item.rs under #693.

use super::*;

pub const BOP_TRADEABLE_DURATION_SECS: u32 = 2 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemBonusKey {
    pub item_id: i32,
    pub bonus_list_ids: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactPower {
    pub artifact_power_id: i16,
    pub purchased_rank: u8,
    pub current_rank_with_bonus: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SocketedGem {
    pub item_id: i32,
    pub context: u8,
    pub bonus_list_ids: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemEnchantment {
    pub id: i32,
    pub duration: u32,
    pub charges: i16,
    pub field_a: u8,
    pub field_b: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemCreateInfo {
    pub guid: ObjectGuid,
    pub item_id: u32,
    pub context: ItemContext,
    pub owner: Option<ObjectGuid>,
    pub max_durability: u32,
    pub expiration: u32,
    pub spell_charges: [i32; MAX_ITEM_SPELLS],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemStorageTemplate {
    pub entry: u32,
    pub class_id: ItemClass,
    pub subclass_id: u32,
    pub inventory_type: InventoryType,
    pub bonding: ItemBondingType,
    pub bag_family: BagFamilyMask,
    pub max_stack_size: u32,
    pub max_count: i32,
    pub item_limit_category: u32,
    pub container_slots: u8,
    pub sell_price: u32,
    pub is_crafting_reagent: bool,
    pub flags: ItemFlags,
}

impl ItemStorageTemplate {
    pub const fn regular_item(entry: u32, max_stack_size: u32) -> Self {
        Self {
            entry,
            class_id: ItemClass::Miscellaneous,
            subclass_id: 0,
            inventory_type: InventoryType::NonEquip,
            bonding: ItemBondingType::None,
            bag_family: BagFamilyMask::NONE,
            max_stack_size,
            max_count: 0,
            item_limit_category: 0,
            container_slots: 0,
            sell_price: 0,
            is_crafting_reagent: false,
            flags: ItemFlags::empty(),
        }
    }

    pub fn is_bound_account_wide(&self) -> bool {
        self.flags.contains(ItemFlags::IS_BOUND_TO_ACCOUNT)
    }

    pub const fn can_change_equip_state_in_combat(&self) -> bool {
        matches!(
            self.inventory_type,
            InventoryType::Relic | InventoryType::Shield | InventoryType::Holdable
        ) || matches!(self.class_id, ItemClass::Weapon | ItemClass::Projectile)
    }
}

pub fn item_can_go_into_bag(proto: &ItemStorageTemplate, bag_proto: &ItemStorageTemplate) -> bool {
    match bag_proto.class_id {
        ItemClass::Container => match bag_proto.subclass_id {
            subclass if subclass == ItemSubClassContainer::Container as u32 => true,
            subclass if subclass == ItemSubClassContainer::SoulContainer as u32 => {
                proto.bag_family.contains(BagFamilyMask::SOUL_SHARDS)
            }
            subclass if subclass == ItemSubClassContainer::HerbContainer as u32 => {
                proto.bag_family.contains(BagFamilyMask::HERBS)
            }
            subclass if subclass == ItemSubClassContainer::EnchantingContainer as u32 => {
                proto.bag_family.contains(BagFamilyMask::ENCHANTING_SUPP)
            }
            subclass if subclass == ItemSubClassContainer::MiningContainer as u32 => {
                proto.bag_family.contains(BagFamilyMask::MINING_SUPP)
            }
            subclass if subclass == ItemSubClassContainer::EngineeringContainer as u32 => {
                proto.bag_family.contains(BagFamilyMask::ENGINEERING_SUPP)
            }
            subclass if subclass == ItemSubClassContainer::GemContainer as u32 => {
                proto.bag_family.contains(BagFamilyMask::GEMS)
            }
            subclass if subclass == ItemSubClassContainer::LeatherworkingContainer as u32 => proto
                .bag_family
                .contains(BagFamilyMask::LEATHERWORKING_SUPP),
            subclass if subclass == ItemSubClassContainer::InscriptionContainer as u32 => {
                proto.bag_family.contains(BagFamilyMask::INSCRIPTION_SUPP)
            }
            subclass if subclass == ItemSubClassContainer::TackleContainer as u32 => {
                proto.bag_family.contains(BagFamilyMask::FISHING_SUPP)
            }
            subclass if subclass == ItemSubClassContainer::CookingContainer as u32 => {
                proto.bag_family.contains(BagFamilyMask::COOKING_SUPP)
            }
            subclass if subclass == ItemSubClassContainer::ReagentContainer as u32 => {
                proto.is_crafting_reagent
            }
            _ => false,
        },
        ItemClass::Quiver => match bag_proto.subclass_id {
            subclass if subclass == ItemSubClassQuiver::Quiver as u32 => {
                proto.bag_family.contains(BagFamilyMask::ARROWS)
            }
            subclass if subclass == ItemSubClassQuiver::AmmoPouch as u32 => {
                proto.bag_family.contains(BagFamilyMask::BULLETS)
            }
            _ => false,
        },
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStateTransition {
    Updated,
    PretendNeverExisted,
}
