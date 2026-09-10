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
pub struct BattlePetSpeciesEntry {
    pub id: u32,
    pub description: String,
    pub source_text: String,
    pub creature_id: i32,
    pub summon_spell_id: i32,
    pub icon_file_data_id: i32,
    pub pet_type_enum: u8,
    pub flags: i32,
    pub source_type_enum: i8,
    pub card_ui_model_scene_id: i32,
    pub loadout_ui_model_scene_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetSpeciesStateEntry {
    pub id: u32,
    pub battle_pet_state_id: u8,
    pub value: i32,
    pub battle_pet_species_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetCalculatedStatsLikeCpp {
    pub max_health: u32,
    pub power: u32,
    pub speed: u32,
}

pub const BATTLE_PET_STATE_STAT_POWER_LIKE_CPP: u8 = 18;
pub const BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP: u8 = 19;
pub const BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP: u8 = 20;
pub const BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP: i32 = 0x00002;
pub const BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP: i32 = 0x00004;
pub const BATTLE_PET_SPECIES_FLAG_NOT_TRADABLE_LIKE_CPP: i32 = 0x00010;
pub const BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP: i32 = 0x00040;
pub const BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP: i32 = 0x00080;
pub const BATTLE_PET_SPECIES_FLAG_RANDOM_DISPLAY_LIKE_CPP: i32 = 0x00800;

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
db2_store!(BattlePetSpeciesStore, BattlePetSpeciesEntry);
db2_store!(BattlePetSpeciesStateStore, BattlePetSpeciesStateEntry);
db2_store!(CurrencyContainerStore, CurrencyContainerEntry);
db2_store!(HeirloomStore, HeirloomEntry);
db2_store!(ToyStore, ToyEntry);
db2_store!(TransmogHolidayStore, TransmogHolidayEntry);
db2_store!(TransmogSetStore, TransmogSetEntry);
db2_store!(TransmogSetGroupStore, TransmogSetGroupEntry);

pub fn calculate_battle_pet_stats_like_cpp(
    breed: u16,
    species: u32,
    quality: u8,
    level: u16,
    breed_states: &BattlePetBreedStateStore,
    species_states: &BattlePetSpeciesStateStore,
    breed_qualities: &BattlePetBreedQualityStore,
) -> Option<BattlePetCalculatedStatsLikeCpp> {
    let breed_id = u32::from(breed);
    if !breed_states
        .values()
        .any(|entry| entry.battle_pet_breed_id == breed_id)
    {
        return None;
    }

    let mut health = battle_pet_breed_state_value_like_cpp(
        breed_states,
        breed_id,
        BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
    ) as f32;
    let mut power = battle_pet_breed_state_value_like_cpp(
        breed_states,
        breed_id,
        BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
    ) as f32;
    let mut speed = battle_pet_breed_state_value_like_cpp(
        breed_states,
        breed_id,
        BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
    ) as f32;

    health += battle_pet_species_state_value_like_cpp(
        species_states,
        species,
        BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
    ) as f32;
    power += battle_pet_species_state_value_like_cpp(
        species_states,
        species,
        BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
    ) as f32;
    speed += battle_pet_species_state_value_like_cpp(
        species_states,
        species,
        BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
    ) as f32;

    if let Some(quality_entry) = breed_qualities
        .values()
        .find(|entry| entry.quality_enum == quality)
    {
        health *= quality_entry.state_multiplier;
        power *= quality_entry.state_multiplier;
        speed *= quality_entry.state_multiplier;
    }

    health *= f32::from(level);
    power *= f32::from(level);
    speed *= f32::from(level);

    Some(BattlePetCalculatedStatsLikeCpp {
        max_health: (health / 20.0).round() as u32 + 100,
        power: (power / 100.0).round() as u32,
        speed: (speed / 100.0).round() as u32,
    })
}

fn battle_pet_breed_state_value_like_cpp(
    store: &BattlePetBreedStateStore,
    breed_id: u32,
    state_id: u8,
) -> u16 {
    store
        .values()
        .find(|entry| {
            entry.battle_pet_breed_id == breed_id && entry.battle_pet_state_id == state_id
        })
        .map(|entry| entry.value)
        .unwrap_or(0)
}

fn battle_pet_species_state_value_like_cpp(
    store: &BattlePetSpeciesStateStore,
    species_id: u32,
    state_id: u8,
) -> i32 {
    store
        .values()
        .find(|entry| {
            entry.battle_pet_species_id == species_id && entry.battle_pet_state_id == state_id
        })
        .map(|entry| entry.value)
        .unwrap_or(0)
}

pub struct TransmogSetItemStore {
    entries: HashMap<u32, TransmogSetItemEntry>,
    by_transmog_set: HashMap<u32, Vec<TransmogSetItemEntry>>,
    by_item_modified_appearance: HashMap<u32, Vec<TransmogSetEntry>>,
}

impl TransmogSetItemStore {
    pub fn from_entries(entries: impl IntoIterator<Item = TransmogSetItemEntry>) -> Self {
        let mut by_id = HashMap::new();
        let mut by_transmog_set = HashMap::<u32, Vec<TransmogSetItemEntry>>::new();
        for entry in entries {
            by_transmog_set
                .entry(entry.transmog_set_id)
                .or_default()
                .push(entry.clone());
            by_id.insert(entry.id, entry);
        }

        Self {
            entries: by_id,
            by_transmog_set,
            by_item_modified_appearance: HashMap::new(),
        }
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
        let mut by_item_modified_appearance = HashMap::<u32, Vec<TransmogSetEntry>>::new();
        for entry in entries {
            if let Some(set) = sets_by_id.get(&entry.transmog_set_id) {
                by_item_modified_appearance
                    .entry(entry.item_modified_appearance_id)
                    .or_default()
                    .push(set.clone());
            } else {
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

impl BattlePetSpeciesEntry {
    pub fn has_flag_like_cpp(&self, flag: i32) -> bool {
        self.flags & flag != 0
    }

    pub fn cant_battle_like_cpp(&self) -> bool {
        self.has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP)
    }

    pub fn not_tradable_like_cpp(&self) -> bool {
        self.has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_NOT_TRADABLE_LIKE_CPP)
    }
}

impl BattlePetSpeciesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "BattlePetSpecies.db2", |id, idx, r| {
            BattlePetSpeciesEntry {
                id,
                description: r.get_field_string(idx, 0),
                source_text: r.get_field_string(idx, 1),
                creature_id: r.get_field_i32(idx, 3),
                summon_spell_id: r.get_field_i32(idx, 4),
                icon_file_data_id: r.get_field_i32(idx, 5),
                pet_type_enum: r.get_field_u8(idx, 6),
                flags: r.get_field_i32(idx, 7),
                source_type_enum: r.get_field_i8(idx, 8),
                card_ui_model_scene_id: r.get_field_i32(idx, 9),
                loadout_ui_model_scene_id: r.get_field_i32(idx, 10),
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

    /// C++ `DB2Manager::GetHeirloomByItemId`.
    pub fn get_by_item_id_like_cpp(&self, item_id: u32) -> Option<&HeirloomEntry> {
        self.values()
            .find(|entry| u32::try_from(entry.item_id).ok() == Some(item_id))
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

    /// C++ `DB2Manager::IsToyItem` indexes toys by `ToyEntry::ItemID`.
    pub fn get_by_item_id_like_cpp(&self, item_id: u32) -> Option<&ToyEntry> {
        self.values()
            .find(|entry| u32::try_from(entry.item_id).ok() == Some(item_id))
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
impl_from_entries!(BattlePetSpeciesStore, BattlePetSpeciesEntry);
impl_from_entries!(BattlePetSpeciesStateStore, BattlePetSpeciesStateEntry);
impl_from_entries!(CurrencyContainerStore, CurrencyContainerEntry);
impl_from_entries!(HeirloomStore, HeirloomEntry);
impl_from_entries!(ToyStore, ToyEntry);
impl_from_entries!(TransmogHolidayStore, TransmogHolidayEntry);
impl_from_entries!(TransmogSetStore, TransmogSetEntry);
impl_from_entries!(TransmogSetGroupStore, TransmogSetGroupEntry);
impl_from_entries!(TransmogSetItemStore, TransmogSetItemEntry);

#[cfg(test)]
#[path = "item_collections/tests/mod.rs"]
mod tests;
