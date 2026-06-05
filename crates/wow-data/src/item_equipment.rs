//! Item equipment, armor, damage and durability DB2 readers.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmorLocationEntry {
    pub id: u32,
    pub cloth_modifier: f32,
    pub leather_modifier: f32,
    pub chain_modifier: f32,
    pub plate_modifier: f32,
    pub modifier: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurabilityCostsEntry {
    pub id: u32,
    pub weapon_sub_class_cost: [u16; 21],
    pub armor_sub_class_cost: [u16; 8],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DurabilityQualityEntry {
    pub id: u32,
    pub data: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemArmorQualityEntry {
    pub id: u32,
    pub quality_mod: [f32; 7],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemArmorShieldEntry {
    pub id: u32,
    pub quality: [f32; 7],
    pub item_level: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemArmorTotalEntry {
    pub id: u32,
    pub item_level: i16,
    pub cloth: f32,
    pub leather: f32,
    pub mail: f32,
    pub plate: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemBagFamilyEntry {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemChildEquipmentEntry {
    pub id: u32,
    pub child_item_id: i32,
    pub child_item_equip_slot: u8,
    pub parent_item_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemDamageEntry {
    pub id: u32,
    pub item_level: u16,
    pub quality: [f32; 7],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemEffectEntry {
    pub id: u32,
    pub legacy_slot_index: u8,
    pub trigger_type: i8,
    pub charges: i16,
    pub cooldown_msec: i32,
    pub category_cooldown_msec: i32,
    pub spell_category_id: u16,
    pub spell_id: i32,
    pub chr_specialization_id: u16,
    pub parent_item_id: u32,
}

macro_rules! db2_store {
    ($store:ident, $entry:ty) => {
        pub struct $store {
            entries: HashMap<u32, $entry>,
        }

        impl $store {
            pub fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self {
                    entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
                }
            }

            pub fn get(&self, id: u32) -> Option<&$entry> {
                self.entries.get(&id)
            }

            pub fn values(&self) -> impl Iterator<Item = &$entry> {
                self.entries.values()
            }

            pub fn len(&self) -> usize {
                self.entries.len()
            }

            pub fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }
        }
    };
}

db2_store!(ArmorLocationStore, ArmorLocationEntry);
db2_store!(DurabilityCostsStore, DurabilityCostsEntry);
db2_store!(DurabilityQualityStore, DurabilityQualityEntry);
db2_store!(ItemArmorQualityStore, ItemArmorQualityEntry);
db2_store!(ItemArmorShieldStore, ItemArmorShieldEntry);
db2_store!(ItemArmorTotalStore, ItemArmorTotalEntry);
db2_store!(ItemBagFamilyStore, ItemBagFamilyEntry);
db2_store!(ItemChildEquipmentStore, ItemChildEquipmentEntry);
db2_store!(ItemDamageAmmoStore, ItemDamageEntry);
db2_store!(ItemDamageOneHandStore, ItemDamageEntry);
db2_store!(ItemDamageOneHandCasterStore, ItemDamageEntry);
db2_store!(ItemDamageTwoHandStore, ItemDamageEntry);
db2_store!(ItemDamageTwoHandCasterStore, ItemDamageEntry);
db2_store!(ItemEffectStore, ItemEffectEntry);

impl ArmorLocationStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ArmorLocation.db2", |id, idx, r| {
            ArmorLocationEntry {
                id,
                cloth_modifier: f32_field(r, idx, 0),
                leather_modifier: f32_field(r, idx, 1),
                chain_modifier: f32_field(r, idx, 2),
                plate_modifier: f32_field(r, idx, 3),
                modifier: f32_field(r, idx, 4),
            }
        })
    }
}

impl DurabilityCostsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "DurabilityCosts.db2", |id, idx, r| {
            DurabilityCostsEntry {
                id,
                weapon_sub_class_cost: std::array::from_fn(|i| r.get_array_u16(idx, 0, i)),
                armor_sub_class_cost: std::array::from_fn(|i| r.get_array_u16(idx, 1, i)),
            }
        })
    }
}

impl DurabilityQualityStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "DurabilityQuality.db2", |id, idx, r| {
            DurabilityQualityEntry {
                id,
                data: f32_field(r, idx, 0),
            }
        })
    }
}

impl ItemArmorQualityStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemArmorQuality.db2", |id, idx, r| {
            ItemArmorQualityEntry {
                id,
                quality_mod: f32_array::<7>(r, idx, 0),
            }
        })
    }
}

impl ItemArmorShieldStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemArmorShield.db2", |id, idx, r| {
            ItemArmorShieldEntry {
                id,
                quality: f32_array::<7>(r, idx, 0),
                item_level: r.get_field_u16(idx, 1),
            }
        })
    }
}

impl ItemArmorTotalStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemArmorTotal.db2", |id, idx, r| {
            ItemArmorTotalEntry {
                id,
                item_level: r.get_field_i16(idx, 0),
                cloth: f32_field(r, idx, 1),
                leather: f32_field(r, idx, 2),
                mail: f32_field(r, idx, 3),
                plate: f32_field(r, idx, 4),
            }
        })
    }
}

impl ItemBagFamilyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemBagFamily.db2", |id, idx, r| {
            ItemBagFamilyEntry {
                id,
                name: r.get_field_string(idx, 0),
            }
        })
    }
}

impl ItemChildEquipmentStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemChildEquipment.db2", |id, idx, r| {
            ItemChildEquipmentEntry {
                id,
                child_item_id: r.get_field_i32(idx, 0),
                child_item_equip_slot: r.get_field_u8(idx, 1),
                parent_item_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

macro_rules! impl_item_damage_store {
    ($store:ident, $file:literal) => {
        impl $store {
            pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
                load_store(data_dir, locale, $file, |id, idx, r| ItemDamageEntry {
                    id,
                    item_level: r.get_field_u16(idx, 0),
                    quality: f32_array::<7>(r, idx, 1),
                })
            }
        }
    };
}

impl_item_damage_store!(ItemDamageAmmoStore, "ItemDamageAmmo.db2");
impl_item_damage_store!(ItemDamageOneHandStore, "ItemDamageOneHand.db2");
impl_item_damage_store!(ItemDamageOneHandCasterStore, "ItemDamageOneHandCaster.db2");
impl_item_damage_store!(ItemDamageTwoHandStore, "ItemDamageTwoHand.db2");
impl_item_damage_store!(ItemDamageTwoHandCasterStore, "ItemDamageTwoHandCaster.db2");

impl ItemEffectStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemEffect.db2", |id, idx, r| {
            ItemEffectEntry {
                id,
                legacy_slot_index: r.get_field_u8(idx, 0),
                trigger_type: r.get_field_i8(idx, 1),
                charges: r.get_field_i16(idx, 2),
                cooldown_msec: r.get_field_i32(idx, 3),
                category_cooldown_msec: r.get_field_i32(idx, 4),
                spell_category_id: r.get_field_u16(idx, 5),
                spell_id: r.get_field_i32(idx, 6),
                chr_specialization_id: r.get_field_u16(idx, 7),
                parent_item_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }

    /// C++ toy use checks `item->Effects` for the requested spell id.
    pub fn effect_for_item_spell_like_cpp(
        &self,
        item_id: u32,
        spell_id: i32,
    ) -> Option<&ItemEffectEntry> {
        self.values()
            .find(|entry| entry.parent_item_id == item_id && entry.spell_id == spell_id)
    }
}

fn load_store<T, S>(
    data_dir: &str,
    locale: &str,
    file_name: &str,
    mut read: impl FnMut(u32, usize, &Wdc4Reader) -> T,
) -> Result<S>
where
    S: FromEntries<T>,
{
    let path = Path::new(data_dir).join("dbc").join(locale).join(file_name);
    let reader =
        Wdc4Reader::open(&path).with_context(|| format!("failed to open {}", path.display()))?;

    let mut entries = Vec::with_capacity(reader.total_count());
    for (id, idx) in reader.iter_records() {
        entries.push(read(id, idx, &reader));
    }

    let store = S::from_entries(entries);
    info!("Loaded {} rows from {}", store.len(), path.display());
    Ok(store)
}

trait FromEntries<T> {
    fn from_entries(entries: impl IntoIterator<Item = T>) -> Self;
    fn len(&self) -> usize;
}

macro_rules! impl_from_entries {
    ($store:ident, $entry:ty) => {
        impl FromEntries<$entry> for $store {
            fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self::from_entries(entries)
            }

            fn len(&self) -> usize {
                self.len()
            }
        }
    };
}

impl_from_entries!(ArmorLocationStore, ArmorLocationEntry);
impl_from_entries!(DurabilityCostsStore, DurabilityCostsEntry);
impl_from_entries!(DurabilityQualityStore, DurabilityQualityEntry);
impl_from_entries!(ItemArmorQualityStore, ItemArmorQualityEntry);
impl_from_entries!(ItemArmorShieldStore, ItemArmorShieldEntry);
impl_from_entries!(ItemArmorTotalStore, ItemArmorTotalEntry);
impl_from_entries!(ItemBagFamilyStore, ItemBagFamilyEntry);
impl_from_entries!(ItemChildEquipmentStore, ItemChildEquipmentEntry);
impl_from_entries!(ItemDamageAmmoStore, ItemDamageEntry);
impl_from_entries!(ItemDamageOneHandStore, ItemDamageEntry);
impl_from_entries!(ItemDamageOneHandCasterStore, ItemDamageEntry);
impl_from_entries!(ItemDamageTwoHandStore, ItemDamageEntry);
impl_from_entries!(ItemDamageTwoHandCasterStore, ItemDamageEntry);
impl_from_entries!(ItemEffectStore, ItemEffectEntry);

fn f32_field(reader: &Wdc4Reader, idx: usize, field: usize) -> f32 {
    f32::from_bits(reader.get_field_u32(idx, field))
}

fn f32_array<const N: usize>(reader: &Wdc4Reader, idx: usize, field: usize) -> [f32; N] {
    std::array::from_fn(|i| f32::from_bits(reader.get_array_element(idx, field, i, 32)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durability_costs_store_preserves_cpp_subclass_arrays() {
        let store = DurabilityCostsStore::from_entries([DurabilityCostsEntry {
            id: 10,
            weapon_sub_class_cost: [7; 21],
            armor_sub_class_cost: [3; 8],
        }]);

        let entry = store.get(10).unwrap();
        assert_eq!(entry.weapon_sub_class_cost[20], 7);
        assert_eq!(entry.armor_sub_class_cost[7], 3);
    }

    #[test]
    fn item_effect_store_finds_item_spell_like_cpp() {
        let store = ItemEffectStore::from_entries([
            ItemEffectEntry {
                id: 1,
                legacy_slot_index: 0,
                trigger_type: 0,
                charges: 0,
                cooldown_msec: 0,
                category_cooldown_msec: 0,
                spell_category_id: 0,
                spell_id: 12_345,
                chr_specialization_id: 0,
                parent_item_id: 30_000,
            },
            ItemEffectEntry {
                id: 2,
                legacy_slot_index: 0,
                trigger_type: 0,
                charges: 0,
                cooldown_msec: 0,
                category_cooldown_msec: 0,
                spell_category_id: 0,
                spell_id: 99_999,
                chr_specialization_id: 0,
                parent_item_id: 30_001,
            },
        ]);

        assert_eq!(
            store
                .effect_for_item_spell_like_cpp(30_000, 12_345)
                .unwrap()
                .id,
            1
        );
        assert!(
            store
                .effect_for_item_spell_like_cpp(30_000, 99_999)
                .is_none()
        );
    }

    #[test]
    fn load_item_equipment_db2_subbatch_when_fixtures_exist() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
        if !dbc_dir.exists() {
            eprintln!(
                "Skipping test: DB2 fixture directory not found at {}",
                dbc_dir.display()
            );
            return;
        }

        macro_rules! load_if_exists {
            ($file:literal, $store:ty) => {
                if dbc_dir.join($file).exists() {
                    let _store = <$store>::load(data_dir, locale)
                        .unwrap_or_else(|error| panic!("failed to load {}: {error:#}", $file));
                }
            };
        }

        load_if_exists!("ArmorLocation.db2", ArmorLocationStore);
        load_if_exists!("DurabilityCosts.db2", DurabilityCostsStore);
        load_if_exists!("DurabilityQuality.db2", DurabilityQualityStore);
        load_if_exists!("ItemArmorQuality.db2", ItemArmorQualityStore);
        load_if_exists!("ItemArmorShield.db2", ItemArmorShieldStore);
        load_if_exists!("ItemArmorTotal.db2", ItemArmorTotalStore);
        load_if_exists!("ItemBagFamily.db2", ItemBagFamilyStore);
        load_if_exists!("ItemChildEquipment.db2", ItemChildEquipmentStore);
        load_if_exists!("ItemDamageAmmo.db2", ItemDamageAmmoStore);
        load_if_exists!("ItemDamageOneHand.db2", ItemDamageOneHandStore);
        load_if_exists!("ItemDamageOneHandCaster.db2", ItemDamageOneHandCasterStore);
        load_if_exists!("ItemDamageTwoHand.db2", ItemDamageTwoHandStore);
        load_if_exists!("ItemDamageTwoHandCaster.db2", ItemDamageTwoHandCasterStore);
        load_if_exists!("ItemEffect.db2", ItemEffectStore);
    }
}
