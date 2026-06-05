//! Economy, collection, cosmetic and battle pet DB2 readers.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuctionHouseEntry {
    pub id: u32,
    pub name: String,
    pub faction_id: u16,
    pub deposit_rate: u8,
    pub consignment_rate: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankBagSlotPricesEntry {
    pub id: u32,
    pub cost: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BattlePetAbilityEntry {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub icon_file_data_id: i32,
    pub pet_type_enum: i8,
    pub cooldown: u32,
    pub battle_pet_visual_id: u16,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BattlePetBreedQualityEntry {
    pub id: u32,
    pub state_multiplier: f32,
    pub quality_enum: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetBreedStateEntry {
    pub id: u32,
    pub battle_pet_state_id: u8,
    pub value: u16,
    pub battle_pet_breed_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetSpeciesStateEntry {
    pub id: u32,
    pub battle_pet_state_id: u8,
    pub value: i32,
    pub battle_pet_species_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencyContainerEntry {
    pub id: u32,
    pub container_name: String,
    pub container_description: String,
    pub min_amount: i32,
    pub max_amount: i32,
    pub container_icon_id: i32,
    pub container_quality: i32,
    pub on_loot_spell_visual_kit_id: i32,
    pub currency_types_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeirloomEntry {
    pub id: u32,
    pub source_text: String,
    pub item_id: i32,
    pub legacy_upgraded_item_id: i32,
    pub static_upgraded_item_id: i32,
    pub source_type_enum: i8,
    pub flags: u8,
    pub legacy_item_id: i32,
    pub upgrade_item_id: [i32; 6],
    pub upgrade_item_bonus_list_id: [u16; 6],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToyEntry {
    pub id: u32,
    pub source_text: String,
    pub item_id: i32,
    pub flags: u8,
    pub source_type_enum: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransmogHolidayEntry {
    pub id: u32,
    pub required_transmog_holiday: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransmogSetEntry {
    pub id: u32,
    pub name: String,
    pub class_mask: i32,
    pub tracking_quest_id: u32,
    pub flags: i32,
    pub transmog_set_group_id: u32,
    pub item_name_description_id: i32,
    pub parent_transmog_set_id: u16,
    pub expansion_id: u8,
    pub ui_order: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransmogSetGroupEntry {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransmogSetItemEntry {
    pub id: u32,
    pub transmog_set_id: u32,
    pub item_modified_appearance_id: u32,
    pub flags: i32,
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

            pub fn len(&self) -> usize {
                self.entries.len()
            }

            pub fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }

            pub fn values(&self) -> impl Iterator<Item = &$entry> {
                self.entries.values()
            }
        }
    };
}

db2_store!(AuctionHouseStore, AuctionHouseEntry);
db2_store!(BankBagSlotPricesStore, BankBagSlotPricesEntry);
db2_store!(BattlePetAbilityStore, BattlePetAbilityEntry);
db2_store!(BattlePetBreedQualityStore, BattlePetBreedQualityEntry);
db2_store!(BattlePetBreedStateStore, BattlePetBreedStateEntry);
db2_store!(BattlePetSpeciesStateStore, BattlePetSpeciesStateEntry);
db2_store!(CurrencyContainerStore, CurrencyContainerEntry);
db2_store!(HeirloomStore, HeirloomEntry);
db2_store!(ToyStore, ToyEntry);
db2_store!(TransmogHolidayStore, TransmogHolidayEntry);
db2_store!(TransmogSetStore, TransmogSetEntry);
db2_store!(TransmogSetGroupStore, TransmogSetGroupEntry);

pub struct TransmogSetItemStore {
    entries: HashMap<u32, TransmogSetItemEntry>,
    by_transmog_set: HashMap<u32, Vec<TransmogSetItemEntry>>,
    by_item_modified_appearance: HashMap<u32, Vec<TransmogSetEntry>>,
}

impl TransmogSetItemStore {
    pub fn from_entries(entries: impl IntoIterator<Item = TransmogSetItemEntry>) -> Self {
        Self::from_entries_and_sets(entries, [])
    }

    /// Build C++ `DB2Manager` transmog secondary indexes.
    pub fn from_entries_and_sets(
        entries: impl IntoIterator<Item = TransmogSetItemEntry>,
        sets: impl IntoIterator<Item = TransmogSetEntry>,
    ) -> Self {
        let mut by_id = HashMap::new();
        let mut by_transmog_set = HashMap::<u32, Vec<TransmogSetItemEntry>>::new();
        let sets_by_id = sets
            .into_iter()
            .map(|set| (set.id, set))
            .collect::<HashMap<_, _>>();
        let validate_sets = !sets_by_id.is_empty();
        let mut by_item_modified_appearance = HashMap::<u32, Vec<TransmogSetEntry>>::new();
        for entry in entries {
            if let Some(set) = sets_by_id.get(&entry.transmog_set_id) {
                by_item_modified_appearance
                    .entry(entry.item_modified_appearance_id)
                    .or_default()
                    .push(set.clone());
            } else if validate_sets {
                continue;
            }
            by_transmog_set
                .entry(entry.transmog_set_id)
                .or_default()
                .push(entry.clone());
            by_id.insert(entry.id, entry);
        }

        Self {
            entries: by_id,
            by_transmog_set,
            by_item_modified_appearance,
        }
    }

    pub fn get(&self, id: u32) -> Option<&TransmogSetItemEntry> {
        self.entries.get(&id)
    }

    pub fn get_transmog_set_items_like_cpp(
        &self,
        transmog_set_id: u32,
    ) -> Option<&[TransmogSetItemEntry]> {
        self.by_transmog_set
            .get(&transmog_set_id)
            .map(Vec::as_slice)
    }

    pub fn get_transmog_sets_for_item_modified_appearance_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> Option<&[TransmogSetEntry]> {
        self.by_item_modified_appearance
            .get(&item_modified_appearance_id)
            .map(Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl AuctionHouseStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "AuctionHouse.db2", |id, idx, r| {
            AuctionHouseEntry {
                id,
                name: r.get_field_string(idx, 0),
                faction_id: r.get_field_u16(idx, 1),
                deposit_rate: r.get_field_u8(idx, 2),
                consignment_rate: r.get_field_u8(idx, 3),
            }
        })
    }
}

impl BankBagSlotPricesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "BankBagSlotPrices.db2", |id, idx, r| {
            BankBagSlotPricesEntry {
                id,
                cost: r.get_field_u32(idx, 0),
            }
        })
    }
}

impl BattlePetAbilityStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "BattlePetAbility.db2", |id, idx, r| {
            BattlePetAbilityEntry {
                id,
                name: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                icon_file_data_id: r.get_field_i32(idx, 2),
                pet_type_enum: r.get_field_i8(idx, 3),
                cooldown: r.get_field_u32(idx, 4),
                battle_pet_visual_id: r.get_field_u16(idx, 5),
                flags: r.get_field_u8(idx, 6),
            }
        })
    }
}

impl BattlePetBreedQualityStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "BattlePetBreedQuality.db2",
            |id, idx, r| BattlePetBreedQualityEntry {
                id,
                state_multiplier: f32_field(r, idx, 0),
                quality_enum: r.get_field_u8(idx, 1),
            },
        )
    }
}

impl BattlePetBreedStateStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "BattlePetBreedState.db2", |id, idx, r| {
            BattlePetBreedStateEntry {
                id,
                battle_pet_state_id: r.get_field_u8(idx, 0),
                value: r.get_field_u16(idx, 1),
                battle_pet_breed_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl BattlePetSpeciesStateStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "BattlePetSpeciesState.db2",
            |id, idx, r| BattlePetSpeciesStateEntry {
                id,
                battle_pet_state_id: r.get_field_u8(idx, 0),
                value: r.get_field_i32(idx, 1),
                battle_pet_species_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl CurrencyContainerStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "CurrencyContainer.db2", |id, idx, r| {
            CurrencyContainerEntry {
                id,
                container_name: r.get_field_string(idx, 0),
                container_description: r.get_field_string(idx, 1),
                min_amount: r.get_field_i32(idx, 2),
                max_amount: r.get_field_i32(idx, 3),
                container_icon_id: r.get_field_i32(idx, 4),
                container_quality: r.get_field_i32(idx, 5),
                on_loot_spell_visual_kit_id: r.get_field_i32(idx, 6),
                currency_types_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl HeirloomStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Heirloom.db2", |id, idx, r| {
            HeirloomEntry {
                id,
                source_text: r.get_field_string(idx, 0),
                item_id: r.get_field_i32(idx, 2),
                legacy_upgraded_item_id: r.get_field_i32(idx, 3),
                static_upgraded_item_id: r.get_field_i32(idx, 4),
                source_type_enum: r.get_field_i8(idx, 5),
                flags: r.get_field_u8(idx, 6),
                legacy_item_id: r.get_field_i32(idx, 7),
                upgrade_item_id: std::array::from_fn(|i| r.get_array_element(idx, 8, i, 32) as i32),
                upgrade_item_bonus_list_id: std::array::from_fn(|i| r.get_array_u16(idx, 9, i)),
            }
        })
    }
}

impl ToyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Toy.db2", |id, idx, r| ToyEntry {
            id,
            source_text: r.get_field_string(idx, 0),
            item_id: r.get_field_i32(idx, 2),
            flags: r.get_field_u8(idx, 3),
            source_type_enum: r.get_field_i8(idx, 4),
        })
    }
}

impl TransmogHolidayStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TransmogHoliday.db2", |id, idx, r| {
            TransmogHolidayEntry {
                id,
                required_transmog_holiday: r.get_field_i32(idx, 1),
            }
        })
    }
}

impl TransmogSetStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TransmogSet.db2", |id, idx, r| {
            TransmogSetEntry {
                id,
                name: r.get_field_string(idx, 0),
                class_mask: r.get_field_i32(idx, 2),
                tracking_quest_id: r.get_field_u32(idx, 3),
                flags: r.get_field_i32(idx, 4),
                transmog_set_group_id: r.get_field_u32(idx, 5),
                item_name_description_id: r.get_field_i32(idx, 6),
                parent_transmog_set_id: r.get_relationship_id(idx).unwrap_or(0) as u16,
                expansion_id: r.get_field_u8(idx, 8),
                ui_order: r.get_field_i16(idx, 9),
            }
        })
    }
}

impl TransmogSetGroupStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TransmogSetGroup.db2", |id, idx, r| {
            TransmogSetGroupEntry {
                id,
                name: r.get_field_string(idx, 0),
            }
        })
    }
}

impl TransmogSetItemStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        Ok(Self::from_entries(load_transmog_set_item_entries(
            data_dir, locale,
        )?))
    }

    pub fn load_with_sets(
        data_dir: &str,
        locale: &str,
        transmog_set_store: &TransmogSetStore,
    ) -> Result<Self> {
        Ok(Self::from_entries_and_sets(
            load_transmog_set_item_entries(data_dir, locale)?,
            transmog_set_store.values().cloned(),
        ))
    }
}

fn load_transmog_set_item_entries(
    data_dir: &str,
    locale: &str,
) -> Result<Vec<TransmogSetItemEntry>> {
    let path = Path::new(data_dir)
        .join("dbc")
        .join(locale)
        .join("TransmogSetItem.db2");
    let reader =
        Wdc4Reader::open(&path).with_context(|| format!("failed to open {}", path.display()))?;

    let mut entries = Vec::with_capacity(reader.total_count());
    for (id, idx) in reader.iter_records() {
        entries.push(TransmogSetItemEntry {
            id,
            transmog_set_id: reader.get_relationship_id(idx).unwrap_or(0),
            item_modified_appearance_id: reader.get_field_u32(idx, 2),
            flags: reader.get_field_i32(idx, 3),
        });
    }

    info!("Loaded {} rows from {}", entries.len(), path.display());
    Ok(entries)
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

fn f32_field(reader: &Wdc4Reader, record_idx: usize, field: usize) -> f32 {
    f32::from_bits(reader.get_field_u32(record_idx, field))
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

impl_from_entries!(AuctionHouseStore, AuctionHouseEntry);
impl_from_entries!(BankBagSlotPricesStore, BankBagSlotPricesEntry);
impl_from_entries!(BattlePetAbilityStore, BattlePetAbilityEntry);
impl_from_entries!(BattlePetBreedQualityStore, BattlePetBreedQualityEntry);
impl_from_entries!(BattlePetBreedStateStore, BattlePetBreedStateEntry);
impl_from_entries!(BattlePetSpeciesStateStore, BattlePetSpeciesStateEntry);
impl_from_entries!(CurrencyContainerStore, CurrencyContainerEntry);
impl_from_entries!(HeirloomStore, HeirloomEntry);
impl_from_entries!(ToyStore, ToyEntry);
impl_from_entries!(TransmogHolidayStore, TransmogHolidayEntry);
impl_from_entries!(TransmogSetStore, TransmogSetEntry);
impl_from_entries!(TransmogSetGroupStore, TransmogSetGroupEntry);
impl_from_entries!(TransmogSetItemStore, TransmogSetItemEntry);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heirloom_store_preserves_cpp_upgrade_arrays() {
        let store = HeirloomStore::from_entries([HeirloomEntry {
            id: 42,
            source_text: "vendor".to_string(),
            item_id: 100,
            legacy_upgraded_item_id: 101,
            static_upgraded_item_id: 102,
            source_type_enum: 3,
            flags: 4,
            legacy_item_id: 99,
            upgrade_item_id: [1, 2, 3, 4, 5, 6],
            upgrade_item_bonus_list_id: [11, 12, 13, 14, 15, 16],
        }]);

        assert_eq!(store.get(42).unwrap().upgrade_item_id[5], 6);
        assert_eq!(store.get(42).unwrap().upgrade_item_bonus_list_id[0], 11);
    }

    #[test]
    fn transmog_set_items_keep_cpp_secondary_index_shape() {
        let store = TransmogSetItemStore::from_entries_and_sets(
            [
                TransmogSetItemEntry {
                    id: 10,
                    transmog_set_id: 7,
                    item_modified_appearance_id: 100,
                    flags: 0,
                },
                TransmogSetItemEntry {
                    id: 11,
                    transmog_set_id: 8,
                    item_modified_appearance_id: 200,
                    flags: 1,
                },
                TransmogSetItemEntry {
                    id: 12,
                    transmog_set_id: 7,
                    item_modified_appearance_id: 101,
                    flags: 2,
                },
                TransmogSetItemEntry {
                    id: 13,
                    transmog_set_id: 99,
                    item_modified_appearance_id: 100,
                    flags: 3,
                },
            ],
            [
                TransmogSetEntry {
                    id: 7,
                    name: "set 7".to_string(),
                    class_mask: 0,
                    tracking_quest_id: 0,
                    flags: 0,
                    transmog_set_group_id: 70,
                    item_name_description_id: 0,
                    parent_transmog_set_id: 0,
                    expansion_id: 0,
                    ui_order: 0,
                },
                TransmogSetEntry {
                    id: 8,
                    name: "set 8".to_string(),
                    class_mask: 0,
                    tracking_quest_id: 0,
                    flags: 0,
                    transmog_set_group_id: 80,
                    item_name_description_id: 0,
                    parent_transmog_set_id: 0,
                    expansion_id: 0,
                    ui_order: 0,
                },
            ],
        );

        assert_eq!(store.get(11).unwrap().item_modified_appearance_id, 200);
        assert_eq!(
            store
                .get_transmog_set_items_like_cpp(7)
                .unwrap()
                .iter()
                .map(|entry| entry.item_modified_appearance_id)
                .collect::<Vec<_>>(),
            vec![100, 101]
        );
        assert!(store.get_transmog_set_items_like_cpp(99).is_none());
        assert_eq!(
            store
                .get_transmog_sets_for_item_modified_appearance_like_cpp(100)
                .unwrap()
                .iter()
                .map(|set| (set.id, set.transmog_set_group_id))
                .collect::<Vec<_>>(),
            vec![(7, 70)]
        );
        assert!(
            store
                .get_transmog_sets_for_item_modified_appearance_like_cpp(999)
                .is_none()
        );
    }

    #[test]
    fn load_item_collections_db2_subbatch_when_fixtures_exist() {
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

        load_if_exists!("AuctionHouse.db2", AuctionHouseStore);
        load_if_exists!("BankBagSlotPrices.db2", BankBagSlotPricesStore);
        load_if_exists!("BattlePetAbility.db2", BattlePetAbilityStore);
        load_if_exists!("BattlePetBreedQuality.db2", BattlePetBreedQualityStore);
        load_if_exists!("BattlePetBreedState.db2", BattlePetBreedStateStore);
        load_if_exists!("BattlePetSpeciesState.db2", BattlePetSpeciesStateStore);
        load_if_exists!("CurrencyContainer.db2", CurrencyContainerStore);
        load_if_exists!("Heirloom.db2", HeirloomStore);
        load_if_exists!("Toy.db2", ToyStore);
        load_if_exists!("TransmogHoliday.db2", TransmogHolidayStore);
        load_if_exists!("TransmogSet.db2", TransmogSetStore);
        load_if_exists!("TransmogSetGroup.db2", TransmogSetGroupStore);
        load_if_exists!("TransmogSetItem.db2", TransmogSetItemStore);
    }
}
