//! Represented item level and price valuation.

use crate::session::ItemValuationCatalogsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::Arc;
use wow_constants::{InventoryType, ItemBonusType, ItemClass, ItemFlags2};
use wow_entities::Item;

fn item_price_base_with_catalogs_like_cpp(
    catalogs: &ItemValuationCatalogsLikeCpp,
    item_level: u32,
) -> Option<(f32, f32)> {
    catalogs
        .price_base
        .get(item_level)
        .map(|entry| (entry.armor, entry.weapon))
}

impl crate::session::state::SessionCatalogs {
    /// C++ `Item::GetBuyPrice(proto, quality, itemLevel, standardPrice)`.
    ///
    /// This preserves the contrasted branch behavior where `standardPrice`
    /// remains false even after the calculated-price path.
    pub fn item_buy_price_with_catalogs_like_cpp(
        &self,
        catalogs: &ItemValuationCatalogsLikeCpp,
        item_id: u32,
        quality: u32,
        item_level: u32,
    ) -> Option<(u32, bool)> {
        let basic = self.items.store.as_ref()?.get(item_id)?;
        let sparse = self.items.stats_store.as_ref()?.sparse_template(item_id)?;
        let flags2 = sparse.flags[1];
        let standard_price = false;

        if (flags2 & ItemFlags2::OverrideGoldCost as u32) != 0 {
            return Some((sparse.buy_price, standard_price));
        }

        let stores = catalogs.import_prices.as_ref();
        let quality_price = match stores.quality.get(quality + 1) {
            Some(entry) => entry.data,
            None => return Some((0, standard_price)),
        };
        let (base_armor, base_weapon) =
            match item_price_base_with_catalogs_like_cpp(catalogs, item_level) {
                Some(base) => base,
                None => return Some((0, standard_price)),
            };

        let mut inventory_type =
            <InventoryType as num_traits::FromPrimitive>::from_i8(sparse.inventory_type)
                .unwrap_or(InventoryType::NonEquip);
        let mut base_factor = if matches!(
            inventory_type,
            InventoryType::Weapon
                | InventoryType::Weapon2Hand
                | InventoryType::WeaponMainhand
                | InventoryType::WeaponOffhand
                | InventoryType::Ranged
                | InventoryType::Thrown
                | InventoryType::RangedRight
        ) {
            base_weapon
        } else {
            base_armor
        };

        if inventory_type == InventoryType::Robe {
            inventory_type = InventoryType::Chest;
        }

        if basic.class_id == ItemClass::Gem as u8 && basic.subclass_id == 11 {
            inventory_type = InventoryType::Weapon;
            base_factor = base_weapon / 3.0;
        }

        let type_factor = match inventory_type {
            InventoryType::Head
            | InventoryType::Neck
            | InventoryType::Shoulders
            | InventoryType::Chest
            | InventoryType::Waist
            | InventoryType::Legs
            | InventoryType::Feet
            | InventoryType::Wrists
            | InventoryType::Hands
            | InventoryType::Finger
            | InventoryType::Trinket
            | InventoryType::Cloak
            | InventoryType::Holdable => {
                let armor_price = match stores.armor.get(inventory_type as u32) {
                    Some(entry) => entry,
                    None => return Some((0, standard_price)),
                };
                match basic.subclass_id {
                    0 | 1 => armor_price.cloth_modifier,
                    2 => armor_price.leather_modifier,
                    3 => armor_price.chain_modifier,
                    4 => armor_price.plate_modifier,
                    _ => 1.0,
                }
            }
            InventoryType::Shield => match stores.shield.get(2) {
                Some(entry) => entry.data,
                None => return Some((0, standard_price)),
            },
            InventoryType::WeaponMainhand => match stores.weapon.get(1) {
                Some(entry) => entry.data,
                None => return Some((0, standard_price)),
            },
            InventoryType::WeaponOffhand => match stores.weapon.get(2) {
                Some(entry) => entry.data,
                None => return Some((0, standard_price)),
            },
            InventoryType::Weapon => match stores.weapon.get(3) {
                Some(entry) => entry.data,
                None => return Some((0, standard_price)),
            },
            InventoryType::Weapon2Hand => match stores.weapon.get(4) {
                Some(entry) => entry.data,
                None => return Some((0, standard_price)),
            },
            InventoryType::Ranged | InventoryType::RangedRight | InventoryType::Relic => {
                match stores.weapon.get(5) {
                    Some(entry) => entry.data,
                    None => return Some((0, standard_price)),
                }
            }
            _ => return Some((sparse.buy_price, standard_price)),
        };

        let cost = sparse.price_variance
            * type_factor
            * base_factor
            * quality_price
            * sparse.price_random_value;
        Some((cost as u32, standard_price))
    }

    /// C++ `Item::GetSellPrice(proto, quality, itemLevel)`.
    pub fn item_sell_price_with_catalogs_like_cpp(
        &self,
        catalogs: &ItemValuationCatalogsLikeCpp,
        item_id: u32,
        quality: u32,
        item_level: u32,
    ) -> Option<u32> {
        let basic = self.items.store.as_ref()?.get(item_id)?;
        let sparse = self.items.stats_store.as_ref()?.sparse_template(item_id)?;

        if (sparse.flags[1] & ItemFlags2::OverrideGoldCost as u32) != 0 {
            return Some(sparse.sell_price);
        }

        let (cost, standard_price) =
            self.item_buy_price_with_catalogs_like_cpp(catalogs, item_id, quality, item_level)?;
        if standard_price {
            let price_modifier = catalogs
                .item_classes
                .get_by_old_enum(u32::from(basic.class_id))?
                .price_modifier;
            let buy_count = sparse.vendor_stack_count.max(1);
            Some((cost as f32 * price_modifier / buy_count as f32) as u32)
        } else {
            Some(sparse.sell_price)
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn item_valuation_catalogs_for_test_like_cpp(&self) -> ItemValuationCatalogsLikeCpp {
        let mut catalogs = ItemValuationCatalogsLikeCpp::default();
        if let Some(store) = &self.import_price_stores {
            catalogs.import_prices = Arc::clone(store);
        }
        if let Some(store) = &self.item_price_base_store {
            catalogs.price_base = Arc::clone(store);
        }
        if let Some(store) = &self.item_class_store {
            catalogs.item_classes = Arc::clone(store);
        }
        if let Some(store) = &self.item_currency_cost_store {
            catalogs.currency_costs = Arc::clone(store);
        }
        if let Some(store) = &self.item_disenchant_loot_store {
            catalogs.disenchant_loot = Arc::clone(store);
        }
        catalogs
    }

    pub fn represented_item_level_bonus_like_cpp(&self, runtime_item: Option<&Item>) -> i64 {
        let Some(item) = runtime_item else {
            return 0;
        };
        let Some(store) = self.items.bonus_db2_store.as_ref() else {
            return 0;
        };

        item.data()
            .item_bonus_key
            .bonus_list_ids
            .iter()
            .filter_map(|bonus_list_id| u16::try_from(*bonus_list_id).ok())
            .flat_map(|bonus_list_id| store.entries_for_bonus_list_like_cpp(bonus_list_id))
            .filter(|bonus| {
                <ItemBonusType as num_traits::FromPrimitive>::from_u8(bonus.bonus_type)
                    == Some(ItemBonusType::ItemLevel)
            })
            .map(|bonus| i64::from(bonus.value[0]))
            .sum()
    }

    pub fn represented_pvp_item_level_bonus_like_cpp(&self, entry_id: u32) -> u8 {
        self.pvp_item_store
            .as_ref()
            .map(|store| store.item_level_bonus_like_cpp(entry_id))
            .unwrap_or(0)
    }
}
