//! Application appearance scenarios retain Session, canonical Player, and packet effects.
use super::*;
use std::sync::Mutex;
use wow_constants::{ItemQuality, ItemSubClassArmor};
use wow_data::item::stats::ItemRandomPropertyTemplateEntry;
use wow_data::{
    ChrSpecializationEntry, ChrSpecializationStore, HeirloomEntry, HeirloomStore,
    ItemAppearanceEntry, ItemAppearanceStore, ItemModifiedAppearanceEntry,
    ItemModifiedAppearanceStore, ItemSearchNameEntry, ItemSearchNameStore,
    ItemSpecOverrideEntry, ItemSpecOverrideStore, ItemStore, ItemStatsStore,
    QuestPackageItemEntry, QuestPackageItemStore, TransmogSetEntry,
    TransmogSetItemEntry, TransmogSetItemStore,
};
use wow_entities::{Player, PlayerFavoriteAppearanceStateLikeCpp as FavoriteAppearanceStateLikeCpp,
    EQUIPMENT_SLOT_HEAD, EQUIPMENT_SLOT_CHEST};
use wow_world::session::WorldSession;
use wow_world::test_fixtures::{
    attach_appearance_player_for_test,
    appearance_on_item_added_for_test,
    appearance_item_spec_class_mask_for_test,
    appearance_heirloom_rows_for_test,
    appearance_has_permanent_for_test,
    appearance_seed_favorite_for_test,
    appearance_seed_temporary_provider_for_test,
    enable_appearance_criteria_diagnostics_for_test,
    appearance_criteria_events_for_test,
    clear_appearance_criteria_events_for_test,
    RepresentedTransmogCriteriaEvent,
    mutate_canonical_player_for_test,
    set_loaded_player_identity_like_cpp,
};

mod fixtures;
use fixtures::*;
mod catalogs;
mod acquisition;
mod runtime_items;
mod rewards;
mod temporary;
mod criteria;
mod sets;
mod boundaries;
