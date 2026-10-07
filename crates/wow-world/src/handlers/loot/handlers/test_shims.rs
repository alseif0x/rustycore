// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the loot handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_constants::ClientOpcodes;
use wow_packet::WorldPacket;
use wow_packet::packets::loot::{LootRoll, MasterLootItem, SetLootSpecialization};
use wow_world_application::LootHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    fn loot_test_cx_like_cpp(&mut self) -> LootHandlerCxLikeCpp<'_> {
        let (loot, hub) = crate::session::split_loot_mut(self);
        LootHandlerCxLikeCpp::new(hub, loot)
    }

    pub async fn handle_set_loot_specialization(&mut self, packet: SetLootSpecialization) {
        self.loot_test_cx_like_cpp()
            .handle_set_loot_specialization(packet)
            .await;
    }

    /// Dispatches through the registered production thunk so the release
    /// context built by the host trait is exercised by every caller.
    pub async fn handle_loot_release(&mut self, pkt: WorldPacket) {
        let entry = crate::session::registry::registered_handler_entries_like_cpp()
            .find(|entry| entry.opcode == ClientOpcodes::LootRelease)
            .expect("registered loot release handler");
        let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
        (entry.handler)(self, &catalogs, pkt).await;
    }

    /// Dispatches through the registered production thunk so the loot owner's
    /// host trait is exercised by every caller. The item-valuation catalogs come
    /// from the session's test stores, exactly as the pre-move test entry point
    /// built them.
    pub async fn handle_loot_unit(&mut self, pkt: WorldPacket) {
        let mut catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
        catalogs.item_valuation =
            std::sync::Arc::new(self.item_valuation_catalogs_for_test_like_cpp());
        let entry = crate::session::registry::registered_handler_entries_like_cpp()
            .find(|entry| entry.opcode == ClientOpcodes::LootUnit)
            .expect("registered loot unit handler");
        (entry.handler)(self, &catalogs, pkt).await;
    }
    /// Dispatches through the registered production thunk so the loot owner's
    /// host trait is exercised by every caller. The packet is encoded by the
    /// caller exactly as the client sends it (`LootItemPkt::read` decodes it
    /// again in the moved body) and the item generator comes from the session's
    /// test stores, exactly as the pre-move test entry point built it.
    pub async fn handle_loot_item(&mut self, pkt: WorldPacket) {
        let mut catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
        catalogs.id_generators = std::sync::Arc::new(self.id_generators_for_test_like_cpp());
        let entry = crate::session::registry::registered_handler_entries_like_cpp()
            .find(|entry| entry.opcode == ClientOpcodes::LootItem)
            .expect("registered loot item handler");
        (entry.handler)(self, &catalogs, pkt).await;
    }

    /// Dispatches through the registered production thunk so the loot owner's
    /// host trait is exercised by every caller. The typed request is encoded as
    /// the client sends it (`ClientPacket::read` decodes it again in the moved
    /// body); the generators and item valuation come from the session's test
    /// stores, exactly as the pre-move test entry point built them.
    pub async fn handle_loot_roll(&mut self, roll: LootRoll) {
        let mut catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
        catalogs.id_generators = std::sync::Arc::new(self.id_generators_for_test_like_cpp());
        catalogs.item_valuation =
            std::sync::Arc::new(self.item_valuation_catalogs_for_test_like_cpp());
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&roll.loot_obj);
        pkt.write_uint8(roll.loot_list_id);
        pkt.write_uint8(roll.roll_type);
        pkt.reset_read();
        let entry = crate::session::registry::registered_handler_entries_like_cpp()
            .find(|entry| entry.opcode == ClientOpcodes::LootRoll)
            .expect("registered loot roll handler");
        (entry.handler)(self, &catalogs, pkt).await;
    }

    /// Dispatches through the registered production thunk so the loot owner's
    /// host trait is exercised by every caller; see `handle_loot_roll` for the
    /// typed-request encoding and the session-derived generator bundle.
    pub async fn handle_master_loot_item(&mut self, master_loot_item: MasterLootItem) {
        let mut catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
        catalogs.id_generators = std::sync::Arc::new(self.id_generators_for_test_like_cpp());
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(master_loot_item.loot.len() as u32);
        pkt.write_packed_guid(&master_loot_item.target);
        for req in &master_loot_item.loot {
            pkt.write_packed_guid(&req.object);
            pkt.write_uint8(req.loot_list_id);
        }
        pkt.reset_read();
        let entry = crate::session::registry::registered_handler_entries_like_cpp()
            .find(|entry| entry.opcode == ClientOpcodes::MasterLootItem)
            .expect("registered master loot item handler");
        (entry.handler)(self, &catalogs, pkt).await;
    }
}
