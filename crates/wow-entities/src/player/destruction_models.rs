use super::*;

#[derive(Debug, Clone, Copy)]
pub struct DestroyItemCountItemRef<'a> {
    pub bag: u8,
    pub slot: u8,
    pub item: &'a Item,
    pub can_unequip_result: InventoryResult,
}

impl<'a> DestroyItemCountItemRef<'a> {
    pub const fn new(bag: u8, slot: u8, item: &'a Item) -> Self {
        Self {
            bag,
            slot,
            item,
            can_unequip_result: InventoryResult::Ok,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyItemCountAction {
    pub bag: u8,
    pub slot: u8,
    pub removed_count: u32,
    pub remaining_count: u32,
    pub destroy_stack: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestroyItemCountPlan {
    pub removed_count: u32,
    pub actions: Vec<DestroyItemCountAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyFilteredItemRef {
    pub bag: u8,
    pub slot: u8,
    pub should_destroy: bool,
}

impl DestroyFilteredItemRef {
    pub const fn new(bag: u8, slot: u8, should_destroy: bool) -> Self {
        Self {
            bag,
            slot,
            should_destroy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyFilteredItemAction {
    pub bag: u8,
    pub slot: u8,
}

