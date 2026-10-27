//! Canonical inventory authority fixtures for World recovery tests.

use crate::session::WorldSession;
use wow_entities::{
    INVENTORY_DEFAULT_SIZE, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END,
    INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START,
};

/// Complete a newly installed Player whose persisted carried inventory is known empty.
pub(crate) fn complete_known_empty_player_inventory_like_cpp(session: &mut WorldSession) {
    session.set_player_inventory_slot_count_like_cpp(INVENTORY_DEFAULT_SIZE);
    session.complete_player_equipment_inventory_authority_load_like_cpp();

    session
        .core
        .with_owned_player_like_cpp(|player| {
            assert_eq!(
                player.cast_all_obtain_backpack_end_like_cpp(),
                Some(INVENTORY_SLOT_ITEM_START + INVENTORY_DEFAULT_SIZE),
                "the known empty backpack has its canonical loaded slot count"
            );
            for slot in INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_END {
                assert_eq!(
                    player.cast_all_obtain_item_at_like_cpp(INVENTORY_SLOT_BAG_0, slot, None),
                    Some(None),
                    "each loaded backpack position is known empty"
                );
            }
            for bag_slot in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
                assert_eq!(
                    player.cast_all_obtain_bag_like_cpp(bag_slot),
                    Some(None),
                    "each loaded equipped-bag position is known empty"
                );
            }
        })
        .expect("canonical Player inventory was installed before its empty load completed");
}

/// Complete a newly installed Player whose persisted currency rows are known empty.
pub(crate) fn complete_known_empty_player_currencies_like_cpp(session: &mut WorldSession) {
    session.set_currency_types_store(std::sync::Arc::new(
        wow_data::CurrencyTypesStore::from_entries([]),
    ));
    session
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_currencies_loaded_like_cpp(true))
        .expect("canonical Player currencies were installed before their empty load completed");
}
