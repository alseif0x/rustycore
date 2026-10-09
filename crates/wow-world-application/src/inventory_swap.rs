// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

mod effects;
pub use effects::InventorySwapEffectsCxLikeCpp;
mod equip;
mod equip_contracts;
pub use equip::InventoryEquipCxLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use equip::InventoryEquipFixtureRefsLikeCpp;
pub use equip_contracts::{
    bind_inventory_item_for_destination_like_cpp, item_dynamic_flags_changed_like_cpp,
    item_spell_charges_db_string, item_storage_mutable_persistence_like_cpp,
};
mod committed;
pub use committed::InventoryCommittedSwapCxLikeCpp;
mod positions;
pub use positions::InventoryPositionPublicationCxLikeCpp;
mod relocation;
pub use relocation::InventoryCommittedRelocationCxLikeCpp;
mod storage_move;
pub use storage_move::{
    InventoryStorageMoveCxLikeCpp, InventoryStorageMoveHostLikeCpp,
    InventoryStorageMoveStatsCxLikeCpp, InventoryStorageQuestChecksLikeCpp,
    InventoryStorageTargetLikeCpp, autostore_bank_target_like_cpp,
    bank_store_destination_applies_obtain_spells_like_cpp,
    bank_store_item_added_quest_count_like_cpp, execute_inventory_storage_move_like_cpp,
    inventory_storage_move_quest_directions_like_cpp, plan_inventory_storage_move_like_cpp,
};
