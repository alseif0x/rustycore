// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::{Item, ItemReadView, ItemStorageTemplate};
use wow_constants::InventoryResult;

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
    pub item: ItemReadView<'a>,
}

impl<'a> ItemSlotRef<'a> {
    pub const fn new(bag: u8, slot: u8, item: ItemReadView<'a>) -> Self {
        Self { bag, slot, item }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ItemStorageRef<'a> {
    pub bag: u8,
    pub slot: u8,
    pub item: ItemReadView<'a>,
    pub template: Option<&'a ItemStorageTemplate>,
}

impl<'a> ItemStorageRef<'a> {
    pub const fn new(
        bag: u8,
        slot: u8,
        item: ItemReadView<'a>,
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
    pub source_item: Option<ItemReadView<'a>>,
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
    pub source_item: Option<ItemReadView<'a>>,
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

#[derive(Debug, Clone, Copy)]
pub struct FindEquipSlotArgs<'a> {
    pub proto: &'a ItemStorageTemplate,
    pub slot: u8,
    pub swap: bool,
    pub can_dual_wield: bool,
    pub can_titan_grip: bool,
    pub is_two_hand_used: bool,
    pub has_required_profession_skill: bool,
    pub profession_slot: Option<u8>,
    pub equipped_items: &'a [ItemSlotRef<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct CanEquipItemArgs<'a> {
    pub slot: u8,
    pub proto: Option<&'a ItemStorageTemplate>,
    pub source_item: Option<ItemReadView<'a>>,
    pub source_bop_trade_allowed_for_player: bool,
    pub swap: bool,
    pub not_loading: bool,
    pub is_stunned: bool,
    pub is_charmed: bool,
    pub is_in_combat: bool,
    pub is_in_progress_arena: bool,
    pub weapon_change_timer_active: bool,
    pub current_generic_spell_allows_equip: Option<bool>,
    pub current_channeled_spell_allows_equip: Option<bool>,
    pub heirloom_required_level_failed: bool,
    pub can_use_result: InventoryResult,
    pub can_equip_unique_result: InventoryResult,
    pub can_dual_wield: bool,
    pub can_titan_grip: bool,
    pub is_two_hand_used: bool,
    pub proto_always_allow_dual_wield: bool,
    pub has_required_profession_skill: bool,
    pub profession_slot: Option<u8>,
    pub offhand_can_unequip_result: InventoryResult,
    pub offhand_can_store_result: InventoryResult,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub equipped_items: &'a [ItemSlotRef<'a>],
    pub stored_items: &'a [ItemStorageRef<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanEquipItemOutcome {
    pub result: InventoryResult,
    pub dest: u16,
    pub unique_ignore_slot: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipItemObjectOutcome {
    Equipped,
    Merged,
}

#[derive(Debug, Clone, Copy)]
pub struct CanUnequipItemArgs<'a> {
    pub pos: u16,
    pub source_item: Option<ItemReadView<'a>>,
    pub proto: Option<&'a ItemStorageTemplate>,
    pub swap: bool,
    pub source_is_not_empty_bag: bool,
    pub is_charmed: bool,
    pub is_in_combat: bool,
    pub is_in_progress_arena: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CanUseItemTemplateArgs<'a> {
    pub proto: Option<&'a ItemStorageTemplate>,
    pub skip_required_level_check: bool,
    pub player_level: u8,
    pub team: u32,
    pub allowable_class_matches: bool,
    pub allowable_race_matches: bool,
    pub internal_item: bool,
    pub faction_horde: bool,
    pub faction_alliance: bool,
    pub required_skill: u32,
    pub required_skill_rank: u32,
    pub required_skill_value: u32,
    pub required_spell: u32,
    pub has_required_spell: bool,
    pub base_required_level: u8,
    pub holiday_id: u32,
    pub holiday_active: bool,
    pub required_reputation_faction: u32,
    pub required_reputation_rank: u32,
    pub player_reputation_rank: u32,
    pub effect0_spell_id: Option<u32>,
    pub effect1_spell_id: Option<u32>,
    pub has_effect1_spell: bool,
    pub artifact_specialization: Option<u32>,
    pub primary_specialization: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct CanUseItemArgs<'a> {
    pub source_item: Option<ItemReadView<'a>>,
    pub proto: Option<&'a ItemStorageTemplate>,
    pub not_loading: bool,
    pub is_alive: bool,
    pub player_level: u8,
    pub item_required_level: u8,
    pub source_bop_trade_allowed_for_player: bool,
    pub template_args: CanUseItemTemplateArgs<'a>,
    pub item_skill: u32,
    pub item_skill_value: u32,
    pub has_item_skill: bool,
    pub player_class: u8,
    pub proto_is_heirloom: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct EquippedGemRef {
    pub slot: u8,
    pub entry: u32,
    pub limit_category: u32,
}

impl EquippedGemRef {
    pub const fn new(slot: u8, entry: u32, limit_category: u32) -> Self {
        Self {
            slot,
            entry,
            limit_category,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanEquipUniqueItemTemplateArgs<'a> {
    pub proto: Option<&'a ItemStorageTemplate>,
    pub except_slot: u8,
    pub limit_count: u32,
    pub unique_equippable: bool,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub equipped_items: &'a [ItemStorageRef<'a>],
    pub equipped_gems: &'a [EquippedGemRef],
}

#[derive(Debug, Clone, Copy)]
pub struct SocketedGemUniqueRef<'a> {
    pub proto: Option<&'a ItemStorageTemplate>,
    pub unique_equippable: bool,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub source_limit_category_count: u32,
}

impl<'a> SocketedGemUniqueRef<'a> {
    pub const fn new(
        proto: Option<&'a ItemStorageTemplate>,
        unique_equippable: bool,
        limit_category: Option<&'a ItemLimitCategoryTemplate>,
        source_limit_category_count: u32,
    ) -> Self {
        Self {
            proto,
            unique_equippable,
            limit_category,
            source_limit_category_count,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanEquipUniqueItemArgs<'a> {
    pub source_item: Option<ItemReadView<'a>>,
    pub proto: Option<&'a ItemStorageTemplate>,
    pub except_slot: u8,
    pub limit_count: u32,
    pub unique_equippable: bool,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub equipped_items: &'a [ItemStorageRef<'a>],
    pub equipped_gems: &'a [EquippedGemRef],
    pub socketed_gems: &'a [SocketedGemUniqueRef<'a>],
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
    pub source_item: Option<ItemReadView<'a>>,
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
