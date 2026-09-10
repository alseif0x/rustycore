use std::collections::HashSet;

use wow_constants::{
    BagFamilyMask, EnchantmentSlot, InventoryResult, InventoryType, ItemBondingType, ItemClass,
    ItemContext, ItemFieldFlags, ItemFieldFlags2, ItemFlags, ItemModifier, ItemSubClassContainer,
    ItemSubClassQuiver, ItemUpdateState, TypeId, TypeMask,
};
use wow_core::ObjectGuid;

use crate::{
    EntityObject, ObjectDataUpdate, UpdateMask,
    update_fields::{ITEM_DATA_BITS, TYPEID_ITEM},
};

mod item;
mod slots;
mod templates;
mod values;

pub use item::*;
pub use slots::*;
pub use templates::*;
pub use values::*;

use slots::spec_modifier;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
