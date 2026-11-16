use super::SpellDefinitionSeeds;
use wow_data::forever_spells::SpellItemEnchantmentRecord;

impl SpellDefinitionSeeds {
    pub(crate) fn spell_item_enchantment_record(
        &self,
        id: u32,
    ) -> Option<&SpellItemEnchantmentRecord> {
        self.catalog.spell_item_enchantment(id)
    }
}
