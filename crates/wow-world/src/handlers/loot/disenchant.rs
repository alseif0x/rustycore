//! Disenchant persistence table selection and item-context conversion.
//!
//! Split out of `loot/mod.rs` under #584 (B5); items are unchanged.

use super::*;

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
