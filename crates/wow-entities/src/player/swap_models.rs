use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapItemPreflightItem {
    pub is_bag: bool,
    pub is_empty_bag: bool,
    pub is_child: bool,
    pub parent_pos: Option<u16>,
    pub can_unequip_result: InventoryResult,
}

impl SwapItemPreflightItem {
    pub const fn regular() -> Self {
        Self {
            is_bag: false,
            is_empty_bag: false,
            is_child: false,
            parent_pos: None,
            can_unequip_result: InventoryResult::Ok,
        }
    }

    pub const fn bag(is_empty_bag: bool) -> Self {
        Self {
            is_bag: true,
            is_empty_bag,
            is_child: false,
            parent_pos: None,
            can_unequip_result: InventoryResult::Ok,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemPreflightResult {
    NoSource,
    ChildRedirect {
        first_src: u16,
        first_dst: u16,
        second_src: u16,
        second_dst: u16,
    },
    Error(InventoryResult),
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapItemPreflightPlan {
    pub result: SwapItemPreflightResult,
    pub src_unequip_swap: Option<bool>,
    pub dst_unequip_swap: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemEmptyDestinationResult {
    OccupiedDestination,
    InvalidDestinationNoop,
    Error(InventoryResult),
    MoveToInventory {
        quest_added_from_bank: bool,
    },
    MoveToBank {
        quest_removed: bool,
    },
    Equip {
        dest: u16,
        auto_unequip_offhand: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapItemEmptyDestinationPlan {
    pub result: SwapItemEmptyDestinationResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemMergeFillResult {
    ContinueToRealSwap,
    InvalidDestinationNoop,
    MoveMergedStackToInventory,
    MoveMergedStackToBank,
    EquipMergedStack {
        dest: u16,
        auto_unequip_offhand: bool,
    },
    PartialFill {
        source_remaining_count: u32,
        destination_count: u32,
        send_updates: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapItemMergeFillPlan {
    pub result: SwapItemMergeFillResult,
    pub send_refund_info: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemRealSwapValidationSubject {
    Source,
    Destination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemRealSwapTarget {
    Inventory,
    Bank,
    Equip { dest: u16 },
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemRealSwapValidationResult {
    Error {
        result: InventoryResult,
        subject: SwapItemRealSwapValidationSubject,
    },
    Continue {
        source_target: SwapItemRealSwapTarget,
        destination_target: SwapItemRealSwapTarget,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapItemRealSwapValidationPlan {
    pub result: SwapItemRealSwapValidationResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapBagItemRef {
    pub slot: u8,
    pub can_go_into_empty_bag: bool,
}

impl SwapBagItemRef {
    pub const fn new(slot: u8, can_go_into_empty_bag: bool) -> Self {
        Self {
            slot,
            can_go_into_empty_bag,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapBagRef<'a> {
    pub is_empty: bool,
    pub bag_size: u8,
    pub items: &'a [SwapBagItemRef],
}

impl<'a> SwapBagRef<'a> {
    pub const fn new(is_empty: bool, bag_size: u8, items: &'a [SwapBagItemRef]) -> Self {
        Self {
            is_empty,
            bag_size,
            items,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapBagItemMove {
    pub from_slot: u8,
    pub to_slot: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwapItemBagExchangeResult {
    Continue,
    Error(InventoryResult),
    Exchange {
        empty_bag_is_source: bool,
        moves: Vec<SwapBagItemMove>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwapItemBagExchangePlan {
    pub result: SwapItemBagExchangeResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapItemRealSwapExecutionPlan {
    pub remove_destination_update: bool,
    pub remove_source_update: bool,
    pub source_target: SwapItemRealSwapTarget,
    pub destination_target: SwapItemRealSwapTarget,
    pub apply_item_dependent_auras: bool,
    pub release_loot: bool,
    pub auto_unequip_offhand: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemErrorItemOrder {
    SourceDestination,
    SourceOnly,
    DestinationSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemMissingPhase {
    EmptyDestination,
    MergeFill,
    RealSwapValidation,
    BagExchange,
    RealSwapExecution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwapItemOrchestrationResult {
    NoSource,
    ChildRedirect {
        first_src: u16,
        first_dst: u16,
        second_src: u16,
        second_dst: u16,
    },
    Error {
        result: InventoryResult,
        item_order: SwapItemErrorItemOrder,
    },
    EmptyDestination(SwapItemEmptyDestinationPlan),
    MergeFill(SwapItemMergeFillPlan),
    RealSwap {
        bag_exchange: SwapItemBagExchangePlan,
        execution: SwapItemRealSwapExecutionPlan,
    },
    InconsistentRealSwapTargets {
        validation_source_target: SwapItemRealSwapTarget,
        validation_destination_target: SwapItemRealSwapTarget,
        execution_source_target: SwapItemRealSwapTarget,
        execution_destination_target: SwapItemRealSwapTarget,
    },
    MissingPhase(SwapItemMissingPhase),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwapItemOrchestrationPlan {
    pub result: SwapItemOrchestrationResult,
}

