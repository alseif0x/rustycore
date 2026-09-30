use super::*;

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
    pub source_item: Option<&'a Item>,
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
    pub source_item: Option<&'a Item>,
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
    pub source_item: Option<&'a Item>,
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
    pub source_item: Option<&'a Item>,
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
pub enum TitanGripPenaltyAction {
    None,
    Cast(u32),
    Remove(u32),
}
