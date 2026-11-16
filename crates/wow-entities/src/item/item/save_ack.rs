//! Preserves the represented inventory ACK comparison; it is not a complete
//! native Item persistence projection.

use wow_constants::{ItemModifier, ItemUpdateState};
use wow_core::ObjectGuid;

use crate::item::{ItemEnchantment, MAX_ENCHANTMENT_SLOT, MAX_ITEM_SPELLS, SocketedGem};

use super::Item;

const SAVED_MODIFIERS: [ItemModifier; 22] = [
    ItemModifier::TransmogAppearanceAllSpecs,
    ItemModifier::TransmogAppearanceSpec1,
    ItemModifier::TransmogAppearanceSpec2,
    ItemModifier::TransmogAppearanceSpec3,
    ItemModifier::TransmogAppearanceSpec4,
    ItemModifier::TransmogAppearanceSpec5,
    ItemModifier::EnchantIllusionAllSpecs,
    ItemModifier::EnchantIllusionSpec1,
    ItemModifier::EnchantIllusionSpec2,
    ItemModifier::EnchantIllusionSpec3,
    ItemModifier::EnchantIllusionSpec4,
    ItemModifier::EnchantIllusionSpec5,
    ItemModifier::TransmogSecondaryAppearanceAllSpecs,
    ItemModifier::TransmogSecondaryAppearanceSpec1,
    ItemModifier::TransmogSecondaryAppearanceSpec2,
    ItemModifier::TransmogSecondaryAppearanceSpec3,
    ItemModifier::TransmogSecondaryAppearanceSpec4,
    ItemModifier::TransmogSecondaryAppearanceSpec5,
    ItemModifier::BattlePetSpeciesId,
    ItemModifier::BattlePetBreedData,
    ItemModifier::BattlePetLevel,
    ItemModifier::BattlePetDisplayId,
];

/// Only the persisted fields used by the existing inventory ACK comparison.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ItemPersistentSaveStateLikeCpp {
    guid: ObjectGuid,
    entry: u32,
    owner: ObjectGuid,
    creator: ObjectGuid,
    gift_creator: ObjectGuid,
    stack_count: u32,
    expiration: u32,
    dynamic_flags: u32,
    spell_charges: [i32; MAX_ITEM_SPELLS],
    enchantments: [ItemEnchantment; MAX_ENCHANTMENT_SLOT],
    durability: u32,
    create_played_time: u32,
    gems: Vec<SocketedGem>,
    random_properties_id: i32,
    property_seed: i32,
    context: i32,
    saved_modifiers: [u32; SAVED_MODIFIERS.len()],
    slot: u8,
    container: ObjectGuid,
    container_slot: u8,
    text: String,
    update_state: ItemUpdateState,
}

impl Item {
    pub(crate) fn persistent_save_state_like_cpp(&self) -> ItemPersistentSaveStateLikeCpp {
        ItemPersistentSaveStateLikeCpp {
            guid: self.object.guid(),
            entry: self.object.entry(),
            owner: self.data.owner,
            creator: self.data.creator,
            gift_creator: self.data.gift_creator,
            stack_count: self.data.stack_count,
            expiration: self.data.expiration,
            dynamic_flags: self.data.dynamic_flags,
            spell_charges: self.data.spell_charges,
            enchantments: self.data.enchantments,
            durability: self.data.durability,
            create_played_time: self.data.create_played_time,
            gems: self.data.gems.clone(),
            random_properties_id: self.data.random_properties_id,
            property_seed: self.data.property_seed,
            context: self.data.context,
            saved_modifiers: SAVED_MODIFIERS.map(|modifier| self.get_modifier(modifier)),
            slot: self.slot,
            container: self.container,
            container_slot: self.container_slot,
            text: self.text.clone(),
            update_state: self.update_state,
        }
    }
}

impl ItemPersistentSaveStateLikeCpp {
    pub(crate) const fn update_state(&self) -> ItemUpdateState {
        self.update_state
    }

    pub(crate) fn matches_item(&self, item: &Item) -> bool {
        self.guid == item.object.guid()
            && self.entry == item.object.entry()
            && self.owner == item.data.owner
            && self.creator == item.data.creator
            && self.gift_creator == item.data.gift_creator
            && self.stack_count == item.data.stack_count
            && self.expiration == item.data.expiration
            && self.dynamic_flags == item.data.dynamic_flags
            && self.spell_charges == item.data.spell_charges
            && self.enchantments == item.data.enchantments
            && self.durability == item.data.durability
            && self.create_played_time == item.data.create_played_time
            && self.gems == item.data.gems
            && self.random_properties_id == item.data.random_properties_id
            && self.property_seed == item.data.property_seed
            && self.context == item.data.context
            && self.saved_modifiers == SAVED_MODIFIERS.map(|modifier| item.get_modifier(modifier))
            && self.slot == item.slot
            && self.container == item.container
            && self.container_slot == item.container_slot
            && self.text == item.text
            && self.update_state == item.update_state
    }
}

#[cfg(test)]
mod tests {
    use super::{Item, SAVED_MODIFIERS};
    use wow_constants::ItemModifier;

    #[test]
    fn every_saved_modifier_is_compared_independently() {
        for modifier in SAVED_MODIFIERS {
            let original = Item::new(0);
            let captured = original.persistent_save_state_like_cpp();
            assert!(captured.matches_item(&original));

            let mut changed = Item::new(0);
            changed.set_modifier(modifier, 1);
            assert!(
                !captured.matches_item(&changed),
                "saved modifier {modifier:?} participates in the receipt"
            );
        }
    }

    #[test]
    fn non_saved_modifier_is_excluded_from_receipt_comparison() {
        let original = Item::new(0);
        let captured = original.persistent_save_state_like_cpp();
        let mut changed = Item::new(0);
        changed.set_modifier(ItemModifier::UpgradeId, 1);

        assert!(captured.matches_item(&changed));
    }

    #[test]
    fn queue_only_change_is_excluded_from_receipt_comparison() {
        let original = Item::new(0);
        let captured = original.persistent_save_state_like_cpp();
        let mut changed = Item::new(0);
        changed.set_queue_pos(4);

        assert!(captured.matches_item(&changed));
    }
}
