//! Coherent item-row capture for the character full-save transaction.

use std::collections::HashSet;

use wow_entities::{Item, Player};
use wow_persistence::{
    PlayerItemDurabilitySave, PlayerItemGemSaveLikeCpp, PlayerItemSaveLikeCpp,
    PlayerItemSaveStateLikeCpp,
};

pub(super) fn capture_durability(player: &Player) -> Option<Vec<PlayerItemDurabilitySave>> {
    let runtime = player.inventory_runtime_like_cpp();
    runtime
        .equipment_inventory_authority_complete_like_cpp()
        .then(|| {
            let buyback_guids: HashSet<_> = runtime
                .buyback_items()
                .values()
                .map(|item| item.guid)
                .collect();
            let mut rows = runtime
                .item_objects()
                .iter()
                .filter(|(guid, item)| {
                    !buyback_guids.contains(guid)
                        && item.owner_guid() == player.guid()
                        && item.data().max_durability > 0
                        && matches!(
                            item.update_state(),
                            wow_constants::ItemUpdateState::New
                                | wow_constants::ItemUpdateState::Changed
                        )
                })
                .map(|(_, item)| PlayerItemDurabilitySave {
                    item_guid: item.object().guid().counter() as u64,
                    durability: item.data().durability,
                })
                .collect::<Vec<_>>();
            rows.sort_by_key(|row| row.item_guid);
            rows
        })
}

pub(super) fn capture_item_saves(
    player: &Player,
    item_effect_store: Option<&wow_data::ItemEffectStore>,
    enchantment_store: Option<&wow_data::SpellItemEnchantmentStore>,
) -> Option<Vec<PlayerItemSaveLikeCpp>> {
    let runtime = player.inventory_runtime_like_cpp();
    let buyback_guids = runtime
        .buyback_items()
        .values()
        .map(|item| item.guid)
        .collect::<HashSet<_>>();
    let mut rows = Vec::new();
    for (&guid, item) in runtime.item_objects() {
        if buyback_guids.contains(&guid) || item.owner_guid() != player.guid() || item.is_in_trade()
        {
            continue;
        }
        let state = match item.update_state() {
            wow_constants::ItemUpdateState::New => PlayerItemSaveStateLikeCpp::New,
            wow_constants::ItemUpdateState::Changed => PlayerItemSaveStateLikeCpp::Changed,
            wow_constants::ItemUpdateState::Removed => PlayerItemSaveStateLikeCpp::Removed,
            wow_constants::ItemUpdateState::Unchanged => continue,
        };
        if state != PlayerItemSaveStateLikeCpp::Removed {
            let effect_store = item_effect_store?;
            if item
                .data()
                .enchantments
                .iter()
                .any(|enchantment| enchantment.id != 0)
                && enchantment_store.is_none()
            {
                return None;
            }
            rows.push(player_item_save_row_like_cpp(
                item,
                state,
                Some(effect_store),
                enchantment_store,
            ));
        } else {
            rows.push(player_item_save_row_like_cpp(
                item,
                state,
                None,
                enchantment_store,
            ));
        }
    }
    rows.sort_by_key(|row| row.item_guid);
    Some(rows)
}

fn player_item_save_row_like_cpp(
    item: &Item,
    state: PlayerItemSaveStateLikeCpp,
    item_effect_store: Option<&wow_data::ItemEffectStore>,
    enchantment_store: Option<&wow_data::SpellItemEnchantmentStore>,
) -> PlayerItemSaveLikeCpp {
    let data = item.data();
    let effect_count = item_effect_store
        .map(|store| {
            store
                .item_effects_for_item_id_like_cpp(item.object().entry())
                .len()
        })
        .unwrap_or(0)
        .min(data.spell_charges.len());
    let charges = data.spell_charges[..effect_count]
        .iter()
        .map(|charge| format!("{charge} "))
        .collect::<String>();

    let mut enchantments = String::new();
    for enchantment in data.enchantments {
        let should_save = enchantment_store
            .and_then(|store| store.get(enchantment.id as u32))
            .is_some_and(|entry| {
                !entry
                    .flags
                    .contains(wow_constants::SpellItemEnchantmentFlags::DO_NOT_SAVE_TO_DB)
            });
        if should_save {
            enchantments.push_str(&format!(
                "{} {} {} ",
                enchantment.id, enchantment.duration, enchantment.charges
            ));
        } else {
            enchantments.push_str("0 0 0 ");
        }
    }

    let gems = (!data.gems.is_empty()).then(|| {
        std::array::from_fn(|index| {
            data.gems
                .get(index)
                .map(|gem| PlayerItemGemSaveLikeCpp {
                    item_id: gem.item_id as u32,
                    bonus_list_ids: gem
                        .bonus_list_ids
                        .iter()
                        .filter(|bonus| **bonus != 0)
                        .map(|bonus| format!("{bonus} "))
                        .collect(),
                    context: gem.context,
                })
                .unwrap_or_else(|| PlayerItemGemSaveLikeCpp {
                    item_id: 0,
                    bonus_list_ids: String::new(),
                    context: 0,
                })
        })
    });
    let transmog_values = [
        wow_constants::ItemModifier::TransmogAppearanceAllSpecs,
        wow_constants::ItemModifier::TransmogAppearanceSpec1,
        wow_constants::ItemModifier::TransmogAppearanceSpec2,
        wow_constants::ItemModifier::TransmogAppearanceSpec3,
        wow_constants::ItemModifier::TransmogAppearanceSpec4,
        wow_constants::ItemModifier::TransmogAppearanceSpec5,
        wow_constants::ItemModifier::EnchantIllusionAllSpecs,
        wow_constants::ItemModifier::EnchantIllusionSpec1,
        wow_constants::ItemModifier::EnchantIllusionSpec2,
        wow_constants::ItemModifier::EnchantIllusionSpec3,
        wow_constants::ItemModifier::EnchantIllusionSpec4,
        wow_constants::ItemModifier::EnchantIllusionSpec5,
        wow_constants::ItemModifier::TransmogSecondaryAppearanceAllSpecs,
        wow_constants::ItemModifier::TransmogSecondaryAppearanceSpec1,
        wow_constants::ItemModifier::TransmogSecondaryAppearanceSpec2,
        wow_constants::ItemModifier::TransmogSecondaryAppearanceSpec3,
        wow_constants::ItemModifier::TransmogSecondaryAppearanceSpec4,
        wow_constants::ItemModifier::TransmogSecondaryAppearanceSpec5,
    ];
    let transmog = std::array::from_fn(|index| item.get_modifier(transmog_values[index]));
    let transmog = transmog.iter().any(|value| *value != 0).then_some(transmog);

    PlayerItemSaveLikeCpp {
        state,
        item_guid: item.object().guid().counter() as u64,
        item_entry: item.object().entry(),
        owner_guid: data.owner.counter() as u64,
        creator_guid: data.creator.counter() as u64,
        gift_creator_guid: data.gift_creator.counter() as u64,
        count: item.count(),
        expiration: data.expiration,
        charges,
        dynamic_flags: data.dynamic_flags,
        enchantments,
        durability: data.durability,
        played_time: data.create_played_time,
        text: item.text().to_owned(),
        battle_pet_species_id: item.get_modifier(wow_constants::ItemModifier::BattlePetSpeciesId),
        battle_pet_breed_data: item.get_modifier(wow_constants::ItemModifier::BattlePetBreedData),
        battle_pet_level: item.get_modifier(wow_constants::ItemModifier::BattlePetLevel),
        battle_pet_display_id: item.get_modifier(wow_constants::ItemModifier::BattlePetDisplayId),
        random_properties_id: data.random_properties_id,
        property_seed: data.property_seed,
        context: data.context as u8,
        bag_guid: item.container_guid().counter() as u64,
        slot: item.slot(),
        gems,
        transmog,
        wrapped: item.is_wrapped(),
    }
}
