//! Spell handlers state definitions, part 1 of 1.
//!
//! Separated from the spell.rs root under #662. Behaviour is preserved.

use super::*;
use wow_constants::{ItemFieldFlags, ItemUpdateState};

pub(super) const LOOT_MODE_DEFAULT_LIKE_CPP: u16 = 1;

pub(super) const MAX_NR_LOOT_ITEMS_LIKE_CPP: usize = 18;

pub(super) const MAX_LOOT_REFERENCE_FRAMES_LIKE_CPP: u32 = 64;

pub(super) const ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP: u32 = 0x0002;

pub(super) const ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP: u32 = 0x0004;

pub(super) const CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP: i32 = 51;

pub(super) const CONDITION_TYPE_MASK_LIKE_CPP: i32 = 52;

pub(super) const TYPEID_PLAYER_LIKE_CPP: u32 = 6;

pub(super) const PLAYER_TYPE_MASK_LIKE_CPP: u32 = 0x0001 | 0x0020 | 0x0040;

pub(super) const MAP_BATTLEGROUND_LIKE_CPP: i8 = 3;

pub(super) const MAP_ARENA_LIKE_CPP: i8 = 4;

pub(super) fn normalize_item_money_loot_bounds_like_cpp(
    min_money: u32,
    max_money: u32,
) -> (u32, u32) {
    if min_money > max_money {
        (max_money, min_money)
    } else {
        (min_money, max_money)
    }
}

pub(super) use wow_world_lifecycle::{
    LootTemplateRow, LootTemplateTable, WrappedGiftLoad, WrappedGiftRow,
};

#[cfg(test)]
pub(super) fn apply_wrapped_gift_transform_like_cpp(
    item: &mut wow_entities::Item,
    entry: u32,
    flags: u32,
    max_durability: u32,
) -> u32 {
    let durability = item.data().durability;

    item.set_gift_creator(ObjectGuid::EMPTY);
    item.object_mut().set_entry(entry);
    item.replace_all_item_flags(ItemFieldFlags::from_bits_retain(flags));
    item.set_max_durability(max_durability);
    item.set_state(ItemUpdateState::Changed);
    durability
}

pub(super) fn roll_chance_with_rate_like_cpp<R: Rng + ?Sized>(
    chance: f32,
    rate: f32,
    rng: &mut R,
) -> bool {
    if chance >= 100.0 {
        return true;
    }
    rng.gen_range(0.0f32..100.0f32) < chance * rate
}

pub(super) fn referenced_loot_max_count_like_cpp(max_count: u8, rate: f32) -> u32 {
    ((max_count as f32) * rate) as u32
}

pub(super) fn loot_template_plain_row_can_roll_like_cpp(
    item_id: u32,
    chance: f32,
    needs_quest: bool,
    loot_mode: u16,
    min_count: u8,
    max_count: u8,
    item_exists: bool,
    allowed_for_player: bool,
) -> bool {
    if item_id == 0 || !item_exists || min_count == 0 || max_count < min_count {
        return false;
    }

    if needs_quest && !allowed_for_player {
        return false;
    }

    if chance == 0.0 || (chance != 0.0 && chance < 0.000001) {
        return false;
    }

    loot_mode & LOOT_MODE_DEFAULT_LIKE_CPP != 0
}

pub(super) fn loot_template_reference_row_can_roll_like_cpp(
    reference: u32,
    chance: f32,
    loot_mode: u16,
    min_count: u8,
) -> bool {
    reference != 0 && min_count != 0 && chance != 0.0 && loot_mode & LOOT_MODE_DEFAULT_LIKE_CPP != 0
}

pub(super) fn loot_template_group_row_can_roll_like_cpp(
    item_id: u32,
    chance: f32,
    needs_quest: bool,
    loot_mode: u16,
    min_count: u8,
    max_count: u8,
    item_exists: bool,
    allowed_for_player: bool,
) -> bool {
    if item_id == 0
        || !item_exists
        || min_count == 0
        || max_count < min_count
        || (needs_quest && !allowed_for_player)
    {
        return false;
    }

    if chance != 0.0 && chance < 0.000001 {
        return false;
    }

    loot_mode & LOOT_MODE_DEFAULT_LIKE_CPP != 0
}

pub(super) fn roll_group_loot_row_like_cpp<R, F, G, H>(
    rows: &[LootTemplateRow],
    group_id: u8,
    item_exists: F,
    allowed_for_player: G,
    item_drop_rate: H,
    rng: &mut R,
) -> Option<LootTemplateRow>
where
    R: Rng + ?Sized,
    F: Fn(u32) -> bool,
    G: Fn(&LootTemplateRow) -> bool,
    H: Fn(u32) -> f32,
{
    let possible: Vec<&LootTemplateRow> = rows
        .iter()
        .filter(|row| {
            row.group_id == group_id
                && row.reference == 0
                && loot_template_group_row_can_roll_like_cpp(
                    row.item_id,
                    row.chance,
                    row.needs_quest,
                    row.loot_mode,
                    row.min_count,
                    row.max_count,
                    item_exists(row.item_id),
                    allowed_for_player(row),
                )
        })
        .collect();

    let explicitly_chanced: Vec<&LootTemplateRow> = possible
        .iter()
        .copied()
        .filter(|row| row.chance != 0.0)
        .collect();
    if !explicitly_chanced.is_empty() {
        let mut roll = rng.gen_range(0.0f32..100.0f32);
        for row in explicitly_chanced {
            if row.chance >= 100.0 {
                return Some((*row).clone());
            }
            roll -= row.chance * item_drop_rate(row.item_id);
            if roll < 0.0 {
                return Some((*row).clone());
            }
        }
    }

    let equal_chanced: Vec<&LootTemplateRow> = possible
        .iter()
        .copied()
        .filter(|row| row.chance == 0.0)
        .collect();
    if equal_chanced.is_empty() {
        None
    } else {
        Some((*equal_chanced[rng.gen_range(0..equal_chanced.len())]).clone())
    }
}

#[derive(Debug)]
pub(super) struct LootTemplateFrame {
    pub(super) rows: Vec<LootTemplateRow>,
    pub(super) condition_references: HashMap<u32, Vec<LootConditionRowLikeCpp>>,
    pub(super) index: usize,
    pub(super) group_id: u8,
    pub(super) groups_enqueued: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct ItemTemplateAddonLootMetadataLikeCpp {
    pub(super) flags_cu: u32,
    pub(super) quest_log_item_id: i32,
}

impl ItemTemplateAddonLootMetadataLikeCpp {
    pub(super) fn ignores_quest_status(self) -> bool {
        self.flags_cu & ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP != 0
    }

    pub(super) fn follows_loot_rules(self) -> bool {
        self.flags_cu & ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP != 0
    }
}

pub(super) fn item_loot_quest_status_allows_like_cpp(
    ignores_quest_status: bool,
    needs_quest: bool,
    has_non_none_start_quest_status: bool,
    has_quest_for_item: bool,
) -> bool {
    ignores_quest_status
        || ((!needs_quest && !has_non_none_start_quest_status) || has_quest_for_item)
}

pub(super) fn loot_entry_flags_for_row_metadata_like_cpp(
    needs_quest: bool,
    item_flags: ItemFlags,
    addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
) -> LootEntryFlags {
    LootEntryFlags {
        follow_loot_rules: !needs_quest || addon_metadata.follows_loot_rules(),
        freeforall: item_flags.contains(ItemFlags::MULTI_DROP),
        blocked: false,
        counted: false,
        under_threshold: false,
        needs_quest,
    }
}

pub(super) fn player_quest_status_mask_like_cpp(status: Option<u8>, rewarded: bool) -> u32 {
    if rewarded {
        return 1 << 6;
    }

    match status {
        Some(2) => 1 << 1,
        Some(1) => 1 << 3,
        Some(3) => 1 << 5,
        _ => 1 << 0,
    }
}

pub(super) fn player_class_mask_like_cpp(class_id: u8) -> Option<u32> {
    if (1..=13).contains(&class_id) {
        Some(1_u32 << (class_id - 1))
    } else {
        None
    }
}

pub(super) fn player_race_mask_like_cpp(race_id: u8) -> Option<u32> {
    let bit = match race_id {
        1..=11 => race_id - 1,
        22 => 21,
        24..=32 => race_id - 1,
        34 => 11,
        35 => 12,
        36 => 13,
        37 => 14,
        52 => 16,
        70 => 15,
        _ => return None,
    };
    Some(1_u32 << bit)
}

pub(super) fn player_team_for_race_cpp_representable(race: u8) -> u32 {
    match race {
        2 | 5 | 6 | 8 | 9 | 10 => 67,
        _ => 469,
    }
}

pub(super) fn add_loot_item_stacks_like_cpp(
    loot_items: &mut Vec<LootEntry>,
    item_id: u32,
    mut count: u32,
    max_stack_size: u32,
    flags: LootEntryFlags,
) {
    while count > 0 && loot_items.len() < MAX_NR_LOOT_ITEMS_LIKE_CPP {
        let quantity = count.min(max_stack_size);
        loot_items.push(LootEntry {
            loot_list_id: loot_items.len() as u8,
            item_id,
            quantity,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags,
            allowed_looters: Vec::new(),
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        });
        count = count.saturating_sub(max_stack_size);
    }
}

pub(super) fn add_loot_template_row_item_like_cpp<F>(
    loot_items: &mut Vec<LootEntry>,
    row: &LootTemplateRow,
    flags: LootEntryFlags,
    max_stack_size: F,
    rng: &mut impl Rng,
) where
    F: Fn(u32) -> u32,
{
    let rolled_count = rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));
    add_loot_item_stacks_like_cpp(
        loot_items,
        row.item_id,
        rolled_count,
        max_stack_size(row.item_id).max(1),
        flags,
    );
}
