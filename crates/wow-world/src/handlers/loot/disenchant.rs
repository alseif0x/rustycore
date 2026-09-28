//! Disenchant loot templates and their roll eligibility.
//!
//! Split out of `loot/mod.rs` under #584 (B5); items are unchanged.

use super::*;

pub(in crate::handlers::loot) fn represented_disenchant_loot_plain_row_can_roll_like_cpp(
    row: &LootStoreItem,
    item_exists: bool,
) -> bool {
    row.can_roll_as_plain_entry_like_cpp(item_exists, LOOT_MODE_DEFAULT_LIKE_CPP)
}

pub(in crate::handlers::loot) fn represented_disenchant_loot_reference_row_can_roll_like_cpp(row: &LootStoreItem) -> bool {
    row.can_roll_as_reference_entry_like_cpp(LOOT_MODE_DEFAULT_LIKE_CPP)
}

#[derive(Debug, Clone)]
pub(in crate::handlers::loot) struct DisenchantLootTemplateFrame {
    pub(in crate::handlers::loot) template: LootTemplate,
    pub(in crate::handlers::loot) entry_index: usize,
    pub(in crate::handlers::loot) group_index: usize,
    pub(in crate::handlers::loot) requested_group_id: u8,
}

pub(in crate::handlers::loot) fn disenchant_loot_template_frame_like_cpp(
    rows: Vec<LootStoreItem>,
    requested_group_id: u8,
) -> DisenchantLootTemplateFrame {
    let mut template = LootTemplate::default();
    for row in rows {
        template.add_entry_like_cpp(row);
    }

    DisenchantLootTemplateFrame {
        template,
        entry_index: 0,
        group_index: 0,
        requested_group_id,
    }
}

#[derive(Debug, Clone, Copy)]
pub(in crate::handlers::loot) enum DisenchantLootTemplateTable {
    Disenchant,
    Reference,
}

impl DisenchantLootTemplateTable {
    pub(in crate::handlers::loot) fn name(self) -> &'static str {
        match self {
            Self::Disenchant => "disenchant_loot_template",
            Self::Reference => "reference_loot_template",
        }
    }
}

pub(in crate::handlers::loot) fn loot_item_context(context: u8) -> ItemContext {
    <ItemContext as num_traits::FromPrimitive>::from_u8(context).unwrap_or(ItemContext::None)
}
