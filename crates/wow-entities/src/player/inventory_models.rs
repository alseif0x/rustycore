use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPosCount {
    pub pos: u16,
    pub count: u32,
}

impl ItemPosCount {
    pub const fn new(pos: u16, count: u32) -> Self {
        Self { pos, count }
    }

    pub fn is_contained_in(&self, positions: &[ItemPosCount]) -> bool {
        positions.iter().any(|position| position.pos == self.pos)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ItemSlotRef<'a> {
    pub bag: u8,
    pub slot: u8,
    pub item: &'a Item,
}

impl<'a> ItemSlotRef<'a> {
    pub const fn new(bag: u8, slot: u8, item: &'a Item) -> Self {
        Self { bag, slot, item }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ItemStorageRef<'a> {
    pub bag: u8,
    pub slot: u8,
    pub item: &'a Item,
    pub template: Option<&'a ItemStorageTemplate>,
}

impl<'a> ItemStorageRef<'a> {
    pub const fn new(
        bag: u8,
        slot: u8,
        item: &'a Item,
        template: Option<&'a ItemStorageTemplate>,
    ) -> Self {
        Self {
            bag,
            slot,
            item,
            template,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BagTemplateRef<'a> {
    pub bag: u8,
    pub template: &'a ItemStorageTemplate,
}

impl<'a> BagTemplateRef<'a> {
    pub const fn new(bag: u8, template: &'a ItemStorageTemplate) -> Self {
        Self { bag, template }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanStoreItemArgs<'a> {
    pub bag: u8,
    pub slot: u8,
    pub entry: u32,
    pub count: u32,
    pub proto: Option<&'a ItemStorageTemplate>,
    pub source_item: Option<&'a Item>,
    pub source_is_not_empty_bag: bool,
    pub source_bop_trade_allowed_for_player: bool,
    pub swap: bool,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub slot_items: &'a [ItemSlotRef<'a>],
    pub stored_items: &'a [ItemStorageRef<'a>],
    pub bag_templates: &'a [BagTemplateRef<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct CanBankItemArgs<'a> {
    pub bag: u8,
    pub slot: u8,
    pub proto: Option<&'a ItemStorageTemplate>,
    pub source_item: Option<&'a Item>,
    pub source_is_not_empty_bag: bool,
    pub source_is_bag: bool,
    pub source_is_currency_token: bool,
    pub source_bop_trade_allowed_for_player: bool,
    pub swap: bool,
    pub can_use_result: InventoryResult,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub slot_items: &'a [ItemSlotRef<'a>],
    pub stored_items: &'a [ItemStorageRef<'a>],
    pub bag_templates: &'a [BagTemplateRef<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanStoreItemOutcome {
    pub result: InventoryResult,
    pub no_space_count: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemLimitCategoryTemplate {
    pub id: u32,
    pub quantity: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct CanTakeMoreSimilarItemsArgs<'a> {
    pub proto: Option<&'a ItemStorageTemplate>,
    pub count: u32,
    pub source_item: Option<&'a Item>,
    pub current_item_count: u32,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub current_limit_category_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanTakeMoreSimilarItemsOutcome {
    pub result: InventoryResult,
    pub no_space_count: Option<u32>,
    pub offending_item_id: Option<u32>,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ItemSearchLocation: u8 {
        const EQUIPMENT = 0x01;
        const INVENTORY = 0x02;
        const BANK = 0x04;
        const REAGENT_BANK = 0x08;

        const DEFAULT = Self::EQUIPMENT.bits() | Self::INVENTORY.bits();
        const EVERYWHERE = Self::EQUIPMENT.bits() | Self::INVENTORY.bits()
            | Self::BANK.bits() | Self::REAGENT_BANK.bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemSearchCallbackResult {
    Stop,
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStorageError {
    InvalidPlayerSlot(u8),
    InvalidBagSlot(u8),
    InvalidBagItemSlot(u8),
    UnknownBag(u8),
    EmptyPlayerSlot(u8),
    EmptyBagItemSlot {
        bag: u8,
        slot: u8,
    },
    OccupiedPlayerSlot(u8),
    OccupiedBagItemSlot {
        bag: u8,
        slot: u8,
    },
    MismatchedBagGuid {
        bag: u8,
        expected: ObjectGuid,
        actual: ObjectGuid,
    },
    MismatchedItemGuid {
        slot: u8,
        expected: ObjectGuid,
        actual: ObjectGuid,
    },
    MismatchedBagItemGuid {
        bag: u8,
        slot: u8,
        expected: ObjectGuid,
        actual: ObjectGuid,
    },
    SplitItemLootGenerated,
    InvalidSplitCount {
        available: u32,
        requested: u32,
    },
    TooFewItemsToSplit {
        available: u32,
        requested: u32,
    },
    SplitItemInTrade,
    TopLevelBuybackHiddenFromGetItemByPos(u8),
}

