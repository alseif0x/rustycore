//! Loaded item projections remain in Character; fixtures borrow their original inputs.
use super::super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadedItemRandomPropertiesForTest(LoadedItemRandomPropertiesLikeCpp);

pub fn loaded_item_random_properties_for_test(id: i32, seed: i32, properties: Option<&wow_data::ItemRandomPropertiesStore>, suffixes: Option<&wow_data::ItemRandomSuffixStore>) -> Option<LoadedItemRandomPropertiesForTest> {
    loaded_item_random_properties_like_cpp(id, seed, properties, suffixes).map(LoadedItemRandomPropertiesForTest)
}
pub fn apply_loaded_item_instance_fields_for_test(item: &mut wow_entities::Item, enchantments: &[ItemEnchantmentValuesUpdate; wow_entities::MAX_ENCHANTMENT_SLOT], properties: Option<LoadedItemRandomPropertiesForTest>) {
    apply_loaded_item_instance_fields_like_cpp(item, enchantments, properties.map(|properties| properties.0));
}
pub fn apply_loaded_item_storage_mutable_fields_for_test(item: &mut wow_entities::Item, stored_expiration: u32, template_expiration: u32, charges: &str, effect_count: usize) -> bool {
    apply_loaded_item_storage_mutable_fields_like_cpp(item, stored_expiration, template_expiration, charges, effect_count)
}
pub fn loaded_item_slot_applies_equipped_enchantments_for_test(slot: u8) -> bool {
    loaded_item_slot_applies_equipped_enchantments_like_cpp(slot)
}
pub fn loaded_socketed_gems_for_test(fields: [(i32, String, u8); 3]) -> Vec<SocketedGem> {
    loaded_socketed_gems_like_cpp(fields)
}
pub fn loaded_item_effective_enchantments_for_test(loaded: Option<&[ItemEnchantmentValuesUpdate; wow_entities::MAX_ENCHANTMENT_SLOT]>, id: i32, properties: Option<&wow_data::ItemRandomPropertiesStore>, suffixes: Option<&wow_data::ItemRandomSuffixStore>) -> [ItemEnchantmentValuesUpdate; wow_entities::MAX_ENCHANTMENT_SLOT] {
    loaded_item_effective_enchantments_like_cpp(loaded, id, properties, suffixes)
}
pub fn loaded_item_enchantments_for_test(enchantments: &str) -> Option<[ItemEnchantmentValuesUpdate; wow_entities::MAX_ENCHANTMENT_SLOT]> {
    loaded_item_enchantments_like_cpp(enchantments)
}
