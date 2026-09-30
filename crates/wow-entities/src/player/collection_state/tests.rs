use super::*;
use std::sync::Arc;
use wow_constants::{
    Gender, InventoryType, ItemBondingType, ItemClass, ItemFlags2, ItemFlags3,
    ItemQuality, ItemSubClassArmor, ItemSubClassWeapon,
};
use wow_core::{ObjectGuid, Position};
use wow_data::item::ItemRecord;
use wow_data::item::stats::{ItemSparseTemplateEntry, ItemRandomPropertyTemplateEntry};
use wow_data::{
    ItemStore, ItemStatsStore, ItemSearchNameStore, ItemSearchNameEntry,
    ItemModifiedAppearanceStore, ItemModifiedAppearanceEntry, ItemEffectStore,
    ItemEffectEntry, FactionStore, FactionEntry, FactionTemplateStore,
    TransmogSetItemStore, TransmogSetItemEntry,
};
use crate::Player;
type FavoriteAppearanceStateLikeCpp = PlayerFavoriteAppearanceStateLikeCpp;

mod fixtures;
use fixtures::*;
mod admission_proficiency;
mod admission_template;
mod admission_learning;
mod admission_reputation;
mod admission_sources;
mod load_save;
mod sets;
mod boundaries;
