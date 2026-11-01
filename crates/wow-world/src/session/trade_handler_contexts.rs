// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the trade handler context (#1263 F5).
//!
//! The social crate owns the bodies and their context; the session only
//! lends its hub, social state and inventory state, so no session reference
//! crosses into the handler.

use wow_world_social::{TradeHandlerCxLikeCpp, TradeHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl TradeHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn trade_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> TradeHandlerCxLikeCpp<'a> {
        let logging_out = self.lifecycle.is_logging_out_like_cpp();
        let active_loot_guid = self.loot.active_loot_guid_like_cpp();
        let (social, inventory, spell_state, hub) = crate::session::split_trade_mut(self);
        TradeHandlerCxLikeCpp::new(hub, social, inventory, spell_state)
            .with_initiate_context_like_cpp(*catalogs.trade_policy, logging_out)
            .with_active_loot_guid_like_cpp(active_loot_guid)
    }

    fn trade_item_guid_generator_like_cpp(
        &self,
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> std::sync::Arc<wow_core::ObjectGuidGenerator> {
        std::sync::Arc::clone(&catalogs.id_generators.item)
    }
}

/// The World half of the trade exchange (#38 slice 3): the session's money
/// fence, its trade persistence capability and its own publication.
impl wow_world_social::TradeCommitHostLikeCpp for WorldSession {
    fn trade_commit_cx_like_cpp(&mut self) -> TradeHandlerCxLikeCpp<'_> {
        let active_loot_guid = self.loot.active_loot_guid_like_cpp();
        let (social, inventory, spell_state, hub) = crate::session::split_trade_mut(self);
        TradeHandlerCxLikeCpp::new(hub, social, inventory, spell_state)
            .with_active_loot_guid_like_cpp(active_loot_guid)
    }

    fn begin_trade_money_fence_like_cpp(
        &mut self,
    ) -> wow_world_social::TradeHostFutureLikeCpp<'_, Option<wow_world_social::TradeFenceLikeCpp>>
    {
        Box::pin(async move {
            self.begin_exclusive_player_money_persistence_like_cpp()
                .await
                .map(|fence| Box::new(fence) as wow_world_social::TradeFenceLikeCpp)
        })
    }

    fn player_trade_persistence_port_like_cpp(
        &self,
    ) -> Option<std::sync::Arc<dyn wow_persistence::PlayerTradePersistencePortLikeCpp>> {
        self.lifecycle.player_trade_persistence_port_like_cpp()
    }

    fn publish_trade_side_like_cpp(
        &mut self,
        side: wow_world_core::session::mailbox::TradeSidePublicationLikeCpp,
        item_guid_generator: std::sync::Arc<wow_core::ObjectGuidGenerator>,
    ) -> wow_world_social::TradeHostFutureLikeCpp<'_, ()> {
        Box::pin(async move {
            publish_trade_side_like_cpp(self, side, &item_guid_generator).await;
        })
    }

    fn trade_money_tracker_like_cpp(
        &self,
    ) -> std::sync::Arc<wow_world_core::loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp>
    {
        std::sync::Arc::clone(
            self.lifecycle
                .durable_loot_money_persistence_tracker_like_cpp(),
        )
    }

    fn quarantine_trade_like_cpp(&mut self, reason: &str) {
        self.lifecycle
            .durable_loot_money_persistence_tracker_like_cpp()
            .mark_indeterminate_like_cpp();
        self.core.kick(reason);
    }
}

/// C++ `MoveItemFromInventory`/`MoveItemToInventory` client effects, the
/// `ModifyMoney` coinage update and `TRADE_STATUS_COMPLETE`, then the money
/// criteria — `MoneyChanged` queued once for the Player's net change and
/// drained, which raises `player.money_changed` after the COMMIT.
async fn publish_trade_side_like_cpp(
    session: &mut WorldSession,
    side: wow_world_core::session::mailbox::TradeSidePublicationLikeCpp,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) {
    use wow_entities::INVENTORY_SLOT_BAG_0;
    use wow_packet::packets::update::{ItemCreateData, ItemEnchantmentValuesUpdate, UpdateObject};

    let map_id = session.core.player_map_id_like_cpp();
    if !side.removed.is_empty() {
        session.send_packet(&UpdateObject::destroy_objects(
            side.removed.iter().map(|(guid, _, _)| *guid).collect(),
            map_id,
        ));
    }
    if !side.stored.is_empty() {
        let creates = side
            .stored
            .iter()
            .map(|item| {
                let data = item.data();
                let mut enchantments = [ItemEnchantmentValuesUpdate::default(); 13];
                for (dst, src) in enchantments.iter_mut().zip(data.enchantments.iter()) {
                    *dst = ItemEnchantmentValuesUpdate {
                        item_enchantment_mask: 0x1F,
                        id: src.id,
                        duration: src.duration,
                        charges: src.charges,
                        field_a: src.field_a,
                        field_b: src.field_b,
                    };
                }
                let container_slots = session
                    .catalogs
                    .item_storage_template(item.object().entry())
                    .map_or(0, |template| u32::from(template.container_slots));
                ItemCreateData {
                    item_guid: item.object().guid(),
                    entry_id: item.object().entry() as i32,
                    owner_guid: data.owner,
                    contained_in: data.contained_in,
                    stack_count: data.stack_count,
                    dynamic_flags: data.dynamic_flags,
                    durability: data.durability,
                    max_durability: data.max_durability,
                    random_properties_seed: data.property_seed,
                    random_properties_id: data.random_properties_id,
                    enchantments,
                    gems: wow_world_inventory::loaded_item_support::loaded_socketed_gem_create_updates_like_cpp(
                        &data.gems,
                    ),
                    context: u8::try_from(data.context).unwrap_or(0),
                    container_slots,
                    container_item_guids: [wow_core::ObjectGuid::EMPTY; 36],
                }
            })
            .collect();
        session.send_packet(&UpdateObject::create_stored_items(creates, map_id));
    }
    for (guid, count) in &side.merged {
        session.send_packet(&UpdateObject::item_stack_count_update(
            *guid, map_id, *count,
        ));
    }

    let mut inv_slot_changes = Vec::new();
    let mut visible_item_changes = Vec::new();
    let mut bag_slot_changes = Vec::new();
    for (_, bag, slot) in &side.removed {
        if *bag == INVENTORY_SLOT_BAG_0 {
            inv_slot_changes.push((*slot, wow_core::ObjectGuid::EMPTY));
            if *slot < 19 {
                visible_item_changes.push((*slot, 0i32, 0u16, 0u16));
            }
        } else {
            bag_slot_changes.push((*bag, *slot));
        }
    }
    for item in &side.stored {
        if item.bag_slot() == INVENTORY_SLOT_BAG_0 {
            inv_slot_changes.push((item.slot(), item.object().guid()));
        } else {
            bag_slot_changes.push((item.bag_slot(), item.slot()));
        }
    }
    session.send_player_values_update_from_entity_bridge(
        &inv_slot_changes,
        &visible_item_changes,
        &[],
        &[],
        Some(side.money_after),
    );
    for (bag, slot) in bag_slot_changes {
        let (state, hub) = crate::session::split_inventory_ref(session);
        state.send_bag_slot_values_update_like_cpp(hub, bag, slot);
    }
    if !visible_item_changes.is_empty() {
        session.send_stat_update();
    }
    session.send_raw_packet(&side.complete_packet);

    if side.money_before != side.money_after {
        session
            .quest_state
            .enqueue_represented_quest_objective_progress_like_cpp(
                wow_world_quest::RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money: side.money_before,
                    new_money: side.money_after,
                },
            );
    }
    session.sync_player_registry_state_like_cpp();
    session
        .drain_represented_quest_objective_progress_with_generator_like_cpp(item_guid_generator)
        .await;
}
