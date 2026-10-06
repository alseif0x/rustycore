use wow_constants::BagFamilyMask;
use wow_loot::LootConditionRowLikeCpp;

const CONDITION_SOURCE_TYPE_ITEM_LOOT_TEMPLATE_LIKE_CPP: i32 = 5;
const CONDITION_SOURCE_TYPE_REFERENCE_LOOT_TEMPLATE_LIKE_CPP: i32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrappedGiftRow {
    pub entry: u32,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrappedGiftLoad {
    Found(WrappedGiftRow),
    Missing,
    Unavailable,
}

pub(crate) fn stored_loot_item_should_persist_like_cpp(
    template_exists: bool,
    bag_family: BagFamilyMask,
) -> bool {
    if !template_exists {
        return false;
    }
    !bag_family.contains(BagFamilyMask::CURRENCY_TOKENS)
}

pub(crate) fn stored_item_row_can_load_like_cpp_representable(
    item_id: u32,
    count: u32,
    item_index: u32,
    blocked: bool,
    _needs_quest: bool,
    _random_properties_id: i32,
    _random_properties_seed: i32,
    _context: u8,
    item_exists: bool,
) -> bool {
    item_id != 0 && item_exists && count != 0 && item_index <= u32::from(u8::MAX) && !blocked
}

#[derive(Debug, Clone)]
pub struct LootTemplateRow {
    pub item_id: u32,
    pub reference: u32,
    pub chance: f32,
    pub needs_quest: bool,
    pub loot_mode: u16,
    pub group_id: u8,
    pub min_count: u8,
    pub max_count: u8,
    pub conditions: Vec<LootConditionRowLikeCpp>,
}

#[derive(Debug, Clone, Copy)]
pub enum LootTemplateTable {
    Item,
    Reference,
}

impl LootTemplateTable {
    pub fn name(self) -> &'static str {
        match self {
            Self::Item => "item_loot_template",
            Self::Reference => "reference_loot_template",
        }
    }

    pub fn condition_source_type_like_cpp(self) -> i32 {
        match self {
            Self::Item => CONDITION_SOURCE_TYPE_ITEM_LOOT_TEMPLATE_LIKE_CPP,
            Self::Reference => CONDITION_SOURCE_TYPE_REFERENCE_LOOT_TEMPLATE_LIKE_CPP,
        }
    }
}

#[cfg(test)]
#[path = "../../unit_tests/stored_item_loot.rs"]
mod tests;
