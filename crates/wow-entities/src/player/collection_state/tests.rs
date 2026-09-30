use super::*;
use crate::Player;
use std::sync::Arc;
use wow_constants::{
    Gender, InventoryType, ItemBondingType, ItemClass, ItemFlags2, ItemFlags3, ItemQuality,
    ItemSubClassArmor, ItemSubClassWeapon,
};
use wow_core::{ObjectGuid, Position};
use wow_data::item::ItemRecord;
use wow_data::item::stats::{ItemRandomPropertyTemplateEntry, ItemSparseTemplateEntry};
use wow_data::{
    FactionEntry, FactionStore, FactionTemplateStore, ItemEffectEntry, ItemEffectStore,
    ItemModifiedAppearanceEntry, ItemModifiedAppearanceStore, ItemSearchNameEntry,
    ItemSearchNameStore, ItemStatsStore, ItemStore, TransmogSetItemEntry, TransmogSetItemStore,
};
type FavoriteAppearanceStateLikeCpp = PlayerFavoriteAppearanceStateLikeCpp;

mod fixtures;
use fixtures::*;
mod admission_learning;
mod admission_proficiency;
mod admission_reputation;
mod admission_sources;
mod admission_template;
mod boundaries;
mod load_save;
mod sets;
