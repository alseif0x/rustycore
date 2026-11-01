// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

// Moved down to `wow_world_inventory` with the item handlers that call them
// (#1363 slice 8c); these re-exports keep their paths.
pub use wow_world_inventory::{
    bind_inventory_item_for_destination_like_cpp, item_dynamic_flags_changed_like_cpp,
    item_spell_charges_db_string, item_storage_mutable_persistence_like_cpp,
};
