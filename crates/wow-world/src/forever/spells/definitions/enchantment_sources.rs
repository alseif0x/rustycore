use super::SpellDefinitionSeeds;
use wow_data::forever_spells::{SpellItemEnchantmentConditionRecord, SpellItemEnchantmentRecord};

impl SpellDefinitionSeeds {
    pub(crate) fn spell_item_enchantment_record(
        &self,
        id: u32,
    ) -> Option<&SpellItemEnchantmentRecord> {
        self.catalog.spell_item_enchantment(id)
    }

    pub(crate) fn spell_item_enchantment_condition_record(
        &self,
        id: u32,
    ) -> Option<&SpellItemEnchantmentConditionRecord> {
        self.catalog.spell_item_enchantment_condition(id)
    }
}
