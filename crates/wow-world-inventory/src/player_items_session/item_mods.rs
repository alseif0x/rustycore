// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented item mods, the item-bonus stat publication and the item-set
//! item records (#1363 slice 12b1; moved from the World
//! `session/player_items/{modifiers,items}.rs`): `Player::_ApplyItemMods` and
//! `AddItemsSetItem`/`RemoveItemsSetItem` over the canonical Player.

use wow_core::ObjectGuid;
use wow_world_core::session::HubMut;

use super::world_test_consumer_like_cpp;
use crate::{RepresentedItemSetSpellEventLikeCpp, VendorSessionHostLikeCpp};

pub fn record_represented_item_mods_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    item_guid: ObjectGuid,
    slot: u8,
    apply: bool,
) -> usize {
    let world_test = world_test_consumer_like_cpp(host);
    let (inventory, hub) = host.split_inventory_mut();
    let HubMut {
        core,
        catalogs,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures,
        ..
    } = hub;
    let inventory_access = core.owned_inventory_access_like_cpp();
    let modifier_access = core.owned_item_modifiers_access_like_cpp();
    let catalogs = crate::ItemModsCatalogsViewLikeCpp::new(
        catalogs.items.store.as_ref(),
        catalogs.items.stats_store.as_ref(),
        catalogs.scaling_stat_distribution_store.as_ref(),
        catalogs.scaling_stat_values_store.as_ref(),
        catalogs.shield_block_regular_game_table.as_ref(),
        catalogs.spell_catalogs.spell_shapeshift_form_store(),
    );
    inventory.record_represented_item_mods_with_access_like_cpp(
        &inventory_access,
        &modifier_access,
        catalogs,
        item_guid,
        slot,
        apply,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_level,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.auras.represented_shapeshift_form_like_cpp,
        world_test,
    )
}

pub fn send_represented_item_bonus_player_stat_update_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
) -> bool {
    // Item changes alter derived stats, vital maxima and weapon ranges
    // together. Publish the same complete projection consumed by combat;
    // sending only the raw bonus accumulator would leave the canonical
    // snapshot stale and could make the client and server disagree after
    // repair or an equipment-set swap. A raw packet is retained only as a
    // fixture/early-login fallback when the complete projection cannot yet
    // be formed; no derived combat snapshot exists in that state.
    if host.send_stat_update() {
        return true;
    }
    let Some(update) = ({
        let (s, h) = host.split_inventory_ref();
        s.represented_item_bonus_player_stat_update_object_like_cpp(h)
    }) else {
        return false;
    };
    host.send_packet(&update);
    true
}

pub fn record_represented_items_set_item_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    item_guid: ObjectGuid,
    apply: bool,
) -> bool {
    !record_represented_items_set_item_events_like_cpp(host, item_guid, apply).is_empty()
}

pub fn record_represented_items_set_item_events_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    item_guid: ObjectGuid,
    apply: bool,
) -> Vec<RepresentedItemSetSpellEventLikeCpp> {
    let world_test = world_test_consumer_like_cpp(host);
    let (state, hub) = host.split_inventory_mut();
    let inventory_access = hub.core.owned_inventory_access_like_cpp();
    let item_modifiers = hub.core.owned_item_modifiers_access_like_cpp();
    let shared = hub.shared();
    let item_sets = shared.owned_item_set_access_like_cpp();
    state.record_represented_items_set_item_like_cpp(
        &inventory_access,
        &item_modifiers,
        &item_sets,
        item_guid,
        apply,
        world_test,
    )
}
