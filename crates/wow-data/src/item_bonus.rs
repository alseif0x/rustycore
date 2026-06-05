//! Item bonus, selector, limit, set and spec DB2 readers.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemBonusDb2Entry {
    pub id: u32,
    pub value: [i32; 4],
    pub parent_item_bonus_list_id: u16,
    pub bonus_type: u8,
    pub order_index: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemBonusListLevelDeltaEntry {
    pub id: u32,
    pub item_level_delta: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemBonusTreeNodeEntry {
    pub id: u32,
    pub item_context: u8,
    pub child_item_bonus_tree_id: u16,
    pub child_item_bonus_list_id: u16,
    pub child_item_level_selector_id: u16,
    pub parent_item_bonus_tree_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemContextPickerEntry {
    pub id: u32,
    pub item_creation_context: u8,
    pub order_index: u8,
    pub p_val: i32,
    pub flags: u32,
    pub player_condition_id: u32,
    pub item_context_picker_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLevelSelectorEntry {
    pub id: u32,
    pub min_item_level: u16,
    pub item_level_selector_quality_set_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLevelSelectorQualityEntry {
    pub id: u32,
    pub quality_item_bonus_list_id: i32,
    pub quality: i8,
    pub parent_ils_quality_set_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLevelSelectorQualitySetEntry {
    pub id: u32,
    pub ilvl_rare: i16,
    pub ilvl_epic: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLimitCategoryEntry {
    pub id: u32,
    pub name: String,
    pub quantity: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLimitCategoryConditionEntry {
    pub id: u32,
    pub add_quantity: i8,
    pub player_condition_id: u32,
    pub parent_item_limit_category_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemModifiedAppearanceExtraEntry {
    pub id: u32,
    pub icon_file_data_id: i32,
    pub unequipped_icon_file_data_id: i32,
    pub sheathe_type: u8,
    pub display_weapon_subclass_id: i8,
    pub display_inventory_type: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemNameDescriptionEntry {
    pub id: u32,
    pub description: String,
    pub color: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSearchNameEntry {
    pub id: u32,
    pub allowable_race: i64,
    pub display: String,
    pub overall_quality_id: u8,
    pub expansion_id: i8,
    pub min_faction_id: u16,
    pub min_reputation: i32,
    pub allowable_class: i32,
    pub required_level: i8,
    pub required_skill: u16,
    pub required_skill_rank: u16,
    pub required_ability: u32,
    pub item_level: u16,
    pub flags: [i32; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSetEntry {
    pub id: u32,
    pub name: String,
    pub set_flags: u32,
    pub required_skill: u32,
    pub required_skill_rank: u16,
    pub item_id: [u32; 17],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSetSpellEntry {
    pub id: u32,
    pub chr_spec_id: u16,
    pub spell_id: u32,
    pub threshold: u8,
    pub item_set_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSpecEntry {
    pub id: u32,
    pub min_level: u8,
    pub max_level: u8,
    pub item_type: u8,
    pub primary_stat: u8,
    pub secondary_stat: u8,
    pub specialization_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSpecOverrideEntry {
    pub id: u32,
    pub spec_id: u16,
    pub item_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemXBonusTreeEntry {
    pub id: u32,
    pub item_bonus_tree_id: u16,
    pub item_id: u32,
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
        }
    };
}

db2_store!(ItemBonusDb2Store, ItemBonusDb2Entry);
db2_store!(ItemBonusListLevelDeltaStore, ItemBonusListLevelDeltaEntry);
db2_store!(ItemBonusTreeNodeStore, ItemBonusTreeNodeEntry);
db2_store!(ItemContextPickerStore, ItemContextPickerEntry);
db2_store!(ItemLevelSelectorStore, ItemLevelSelectorEntry);
db2_store!(ItemLevelSelectorQualityStore, ItemLevelSelectorQualityEntry);
db2_store!(
    ItemLevelSelectorQualitySetStore,
    ItemLevelSelectorQualitySetEntry
);
db2_store!(ItemLimitCategoryStore, ItemLimitCategoryEntry);
db2_store!(
    ItemLimitCategoryConditionStore,
    ItemLimitCategoryConditionEntry
);
db2_store!(
    ItemModifiedAppearanceExtraStore,
    ItemModifiedAppearanceExtraEntry
);
db2_store!(ItemNameDescriptionStore, ItemNameDescriptionEntry);
db2_store!(ItemSearchNameStore, ItemSearchNameEntry);
db2_store!(ItemSetStore, ItemSetEntry);
db2_store!(ItemSetSpellStore, ItemSetSpellEntry);
db2_store!(ItemSpecStore, ItemSpecEntry);
db2_store!(ItemXBonusTreeStore, ItemXBonusTreeEntry);

pub struct ItemSpecOverrideStore {
    entries: HashMap<u32, ItemSpecOverrideEntry>,
    by_item_id: HashMap<u32, Vec<ItemSpecOverrideEntry>>,
}

impl ItemSpecOverrideStore {
    pub fn from_entries(entries: impl IntoIterator<Item = ItemSpecOverrideEntry>) -> Self {
        let mut by_id = HashMap::new();
        let mut by_item_id = HashMap::<u32, Vec<ItemSpecOverrideEntry>>::new();
        for entry in entries {
            by_item_id
                .entry(entry.item_id)
                .or_default()
                .push(entry.clone());
            by_id.insert(entry.id, entry);
        }

        Self {
            entries: by_id,
            by_item_id,
        }
    }

    pub fn get(&self, id: u32) -> Option<&ItemSpecOverrideEntry> {
        self.entries.get(&id)
    }

    /// C++ `DB2Manager::GetItemSpecOverrides`.
    pub fn overrides_for_item_like_cpp(&self, item_id: u32) -> Option<&[ItemSpecOverrideEntry]> {
        self.by_item_id.get(&item_id).map(Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl ItemBonusDb2Store {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemBonus.db2", |id, idx, r| {
            ItemBonusDb2Entry {
                id,
                value: std::array::from_fn(|i| r.get_array_element(idx, 0, i, 32) as i32),
                parent_item_bonus_list_id: r.get_field_u16(idx, 1),
                bonus_type: r.get_field_u8(idx, 2),
                order_index: r.get_field_u8(idx, 3),
            }
        })
    }
}

impl ItemBonusListLevelDeltaStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ItemBonusListLevelDelta.db2",
            |id, idx, r| ItemBonusListLevelDeltaEntry {
                id,
                item_level_delta: r.get_field_i16(idx, 0),
            },
        )
    }
}

impl ItemBonusTreeNodeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemBonusTreeNode.db2", |id, idx, r| {
            ItemBonusTreeNodeEntry {
                id,
                item_context: r.get_field_u8(idx, 0),
                child_item_bonus_tree_id: r.get_field_u16(idx, 1),
                child_item_bonus_list_id: r.get_field_u16(idx, 2),
                child_item_level_selector_id: r.get_field_u16(idx, 3),
                parent_item_bonus_tree_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl ItemContextPickerStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ItemContextPickerEntry.db2",
            |id, idx, r| ItemContextPickerEntry {
                id,
                item_creation_context: r.get_field_u8(idx, 0),
                order_index: r.get_field_u8(idx, 1),
                p_val: r.get_field_i32(idx, 2),
                flags: r.get_field_u32(idx, 3),
                player_condition_id: r.get_field_u32(idx, 4),
                item_context_picker_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl ItemLevelSelectorStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemLevelSelector.db2", |id, idx, r| {
            ItemLevelSelectorEntry {
                id,
                min_item_level: r.get_field_u16(idx, 0),
                item_level_selector_quality_set_id: r.get_field_u16(idx, 1),
            }
        })
    }
}

impl ItemLevelSelectorQualityStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ItemLevelSelectorQuality.db2",
            |id, idx, r| ItemLevelSelectorQualityEntry {
                id,
                quality_item_bonus_list_id: r.get_field_i32(idx, 0),
                quality: r.get_field_i8(idx, 1),
                parent_ils_quality_set_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl ItemLevelSelectorQualitySetStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ItemLevelSelectorQualitySet.db2",
            |id, idx, r| ItemLevelSelectorQualitySetEntry {
                id,
                ilvl_rare: r.get_field_i16(idx, 0),
                ilvl_epic: r.get_field_i16(idx, 1),
            },
        )
    }
}

impl ItemLimitCategoryStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemLimitCategory.db2", |id, idx, r| {
            ItemLimitCategoryEntry {
                id,
                name: r.get_field_string(idx, 0),
                quantity: r.get_field_u8(idx, 1),
                flags: r.get_field_u8(idx, 2),
            }
        })
    }
}

impl ItemLimitCategoryConditionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ItemLimitCategoryCondition.db2",
            |id, idx, r| ItemLimitCategoryConditionEntry {
                id,
                add_quantity: r.get_field_i8(idx, 0),
                player_condition_id: r.get_field_u32(idx, 1),
                parent_item_limit_category_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }

    pub fn conditions_for_parent_like_cpp(
        &self,
        parent_item_limit_category_id: u32,
    ) -> impl Iterator<Item = &ItemLimitCategoryConditionEntry> {
        self.entries.values().filter(move |entry| {
            entry.parent_item_limit_category_id == parent_item_limit_category_id
        })
    }
}

impl ItemModifiedAppearanceExtraStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ItemModifiedAppearanceExtra.db2",
            |id, idx, r| ItemModifiedAppearanceExtraEntry {
                id,
                icon_file_data_id: r.get_field_i32(idx, 0),
                unequipped_icon_file_data_id: r.get_field_i32(idx, 1),
                sheathe_type: r.get_field_u8(idx, 2),
                display_weapon_subclass_id: r.get_field_i8(idx, 3),
                display_inventory_type: r.get_field_i8(idx, 4),
            },
        )
    }
}

impl ItemNameDescriptionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemNameDescription.db2", |id, idx, r| {
            ItemNameDescriptionEntry {
                id,
                description: r.get_field_string(idx, 0),
                color: r.get_field_i32(idx, 1),
            }
        })
    }
}

impl ItemSearchNameStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemSearchName.db2", |id, idx, r| {
            ItemSearchNameEntry {
                id,
                allowable_race: r.get_field_i64(idx, 0),
                display: r.get_field_string(idx, 1),
                overall_quality_id: r.get_field_u8(idx, 3),
                expansion_id: r.get_field_i8(idx, 4),
                min_faction_id: r.get_field_u16(idx, 5),
                min_reputation: r.get_field_i32(idx, 6),
                allowable_class: r.get_field_i32(idx, 7),
                required_level: r.get_field_i8(idx, 8),
                required_skill: r.get_field_u16(idx, 9),
                required_skill_rank: r.get_field_u16(idx, 10),
                required_ability: r.get_field_u32(idx, 11),
                item_level: r.get_field_u16(idx, 12),
                flags: std::array::from_fn(|i| r.get_array_element(idx, 13, i, 32) as i32),
            }
        })
    }
}

impl ItemSetStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemSet.db2", |id, idx, r| ItemSetEntry {
            id,
            name: r.get_field_string(idx, 0),
            set_flags: r.get_field_u32(idx, 1),
            required_skill: r.get_field_u32(idx, 2),
            required_skill_rank: r.get_field_u16(idx, 3),
            item_id: std::array::from_fn(|i| r.get_array_element(idx, 4, i, 32)),
        })
    }
}

impl ItemSetSpellStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemSetSpell.db2", |id, idx, r| {
            ItemSetSpellEntry {
                id,
                chr_spec_id: r.get_field_u16(idx, 0),
                spell_id: r.get_field_u32(idx, 1),
                threshold: r.get_field_u8(idx, 2),
                item_set_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl ItemSpecStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemSpec.db2", |id, idx, r| {
            ItemSpecEntry {
                id,
                min_level: r.get_field_u8(idx, 0),
                max_level: r.get_field_u8(idx, 1),
                item_type: r.get_field_u8(idx, 2),
                primary_stat: r.get_field_u8(idx, 3),
                secondary_stat: r.get_field_u8(idx, 4),
                specialization_id: r.get_field_u16(idx, 5),
            }
        })
    }
}

impl ItemSpecOverrideStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemSpecOverride.db2", |id, idx, r| {
            ItemSpecOverrideEntry {
                id,
                spec_id: r.get_field_u16(idx, 0),
                item_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl ItemXBonusTreeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ItemXBonusTree.db2", |id, idx, r| {
            ItemXBonusTreeEntry {
                id,
                item_bonus_tree_id: r.get_field_u16(idx, 0),
                item_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
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

impl_from_entries!(ItemBonusDb2Store, ItemBonusDb2Entry);
impl_from_entries!(ItemBonusListLevelDeltaStore, ItemBonusListLevelDeltaEntry);
impl_from_entries!(ItemBonusTreeNodeStore, ItemBonusTreeNodeEntry);
impl_from_entries!(ItemContextPickerStore, ItemContextPickerEntry);
impl_from_entries!(ItemLevelSelectorStore, ItemLevelSelectorEntry);
impl_from_entries!(ItemLevelSelectorQualityStore, ItemLevelSelectorQualityEntry);
impl_from_entries!(
    ItemLevelSelectorQualitySetStore,
    ItemLevelSelectorQualitySetEntry
);
impl_from_entries!(ItemLimitCategoryStore, ItemLimitCategoryEntry);
impl_from_entries!(
    ItemLimitCategoryConditionStore,
    ItemLimitCategoryConditionEntry
);
impl_from_entries!(
    ItemModifiedAppearanceExtraStore,
    ItemModifiedAppearanceExtraEntry
);
impl_from_entries!(ItemNameDescriptionStore, ItemNameDescriptionEntry);
impl_from_entries!(ItemSearchNameStore, ItemSearchNameEntry);
impl_from_entries!(ItemSetStore, ItemSetEntry);
impl_from_entries!(ItemSetSpellStore, ItemSetSpellEntry);
impl_from_entries!(ItemSpecStore, ItemSpecEntry);
impl_from_entries!(ItemSpecOverrideStore, ItemSpecOverrideEntry);
impl_from_entries!(ItemXBonusTreeStore, ItemXBonusTreeEntry);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_bonus_store_preserves_cpp_value_array() {
        let store = ItemBonusDb2Store::from_entries([ItemBonusDb2Entry {
            id: 1,
            value: [10, 20, 30, 40],
            parent_item_bonus_list_id: 2,
            bonus_type: 3,
            order_index: 4,
        }]);

        assert_eq!(store.get(1).unwrap().value[3], 40);
    }

    #[test]
    fn item_spec_override_store_groups_rows_by_item_like_cpp_db2_manager() {
        let store = ItemSpecOverrideStore::from_entries([
            ItemSpecOverrideEntry {
                id: 1,
                spec_id: 66,
                item_id: 777,
            },
            ItemSpecOverrideEntry {
                id: 2,
                spec_id: 70,
                item_id: 778,
            },
            ItemSpecOverrideEntry {
                id: 3,
                spec_id: 65,
                item_id: 777,
            },
        ]);

        assert_eq!(store.get(2).unwrap().spec_id, 70);
        let overrides = store
            .overrides_for_item_like_cpp(777)
            .expect("C++ DB2Manager returns a vector for items with override rows");
        assert_eq!(
            overrides
                .iter()
                .map(|entry| entry.spec_id)
                .collect::<Vec<_>>(),
            vec![66, 65]
        );
        assert!(store.overrides_for_item_like_cpp(999).is_none());
    }

    #[test]
    fn load_item_bonus_db2_subbatch_when_fixtures_exist() {
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

        load_if_exists!("ItemBonus.db2", ItemBonusDb2Store);
        load_if_exists!("ItemBonusListLevelDelta.db2", ItemBonusListLevelDeltaStore);
        load_if_exists!("ItemBonusTreeNode.db2", ItemBonusTreeNodeStore);
        load_if_exists!("ItemContextPickerEntry.db2", ItemContextPickerStore);
        load_if_exists!("ItemLevelSelector.db2", ItemLevelSelectorStore);
        load_if_exists!(
            "ItemLevelSelectorQuality.db2",
            ItemLevelSelectorQualityStore
        );
        load_if_exists!(
            "ItemLevelSelectorQualitySet.db2",
            ItemLevelSelectorQualitySetStore
        );
        load_if_exists!("ItemLimitCategory.db2", ItemLimitCategoryStore);
        load_if_exists!(
            "ItemLimitCategoryCondition.db2",
            ItemLimitCategoryConditionStore
        );
        load_if_exists!(
            "ItemModifiedAppearanceExtra.db2",
            ItemModifiedAppearanceExtraStore
        );
        load_if_exists!("ItemNameDescription.db2", ItemNameDescriptionStore);
        load_if_exists!("ItemSearchName.db2", ItemSearchNameStore);
        load_if_exists!("ItemSet.db2", ItemSetStore);
        load_if_exists!("ItemSetSpell.db2", ItemSetSpellStore);
        load_if_exists!("ItemSpec.db2", ItemSpecStore);
        load_if_exists!("ItemSpecOverride.db2", ItemSpecOverrideStore);
        load_if_exists!("ItemXBonusTree.db2", ItemXBonusTreeStore);
    }
}
