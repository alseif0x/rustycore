// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot packet handlers — CMSG_LOOT_UNIT, CMSG_LOOT_ITEM, CMSG_LOOT_RELEASE.
//!
//! Reference: C# Game/Handlers/LootHandler.cs

use std::time::{Duration, Instant};

use tracing::{debug, info, warn};

use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::loot::{
    CreatureLoot, LootEntry, LootItemData, LootItemPkt, LootRelease, LootRemoved, LootResponse,
    LootUnit, SLootRelease,
};
use wow_packet::{ClientPacket, ServerPacket};

use crate::session::WorldSession;

// ── Handler registrations ─────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootUnit,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_unit",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootRelease,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_release",
    }
}

// ── Handler implementations ───────────────────────────────────────

impl WorldSession {
    /// CMSG_LOOT_UNIT — player right-clicks a dead creature to loot it.
    pub async fn handle_loot_unit(&mut self, mut pkt: wow_packet::WorldPacket) {
        let req = match LootUnit::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => { warn!("Bad LootUnit: {e}"); return; }
        };

        let player_guid = match self.player_guid {
            Some(g) => g,
            None => return,
        };

        debug!(account = self.account_id, target = ?req.unit, "CMSG_LOOT_UNIT");

        // Check creature exists and is dead.
        let creature = match self.creatures.get(&req.unit) {
            Some(c) => c,
            None => {
                warn!("LootUnit: creature {:?} not found", req.unit);
                return;
            }
        };

        if creature.is_alive {
            // Can't loot a living creature.
            let response = LootResponse {
                owner: player_guid,
                loot_obj: req.unit,
                failure_reason: 2, // LootError::AlreadyPickedUp or similar
                acquire_reason: 0,
                loot_method: 0,
                threshold: 2,
                coins: 0,
                items: vec![],
                acquired: false,
                ae_looting: false,
            };
            self.send_packet(&response);
            return;
        }

        // Generate or retrieve existing loot.
        if !self.loot_table.contains_key(&req.unit) {
            let loot = generate_creature_loot(req.unit, creature.level, creature.entry);
            self.loot_table.insert(req.unit, loot);
        }

        let loot = self.loot_table.get(&req.unit).unwrap();
        let coins = loot.coins;
        let items: Vec<LootItemData> = loot.items.iter()
            .filter(|e| !e.taken)
            .map(|e| LootItemData {
                loot_list_id: e.loot_list_id,
                ui_type: 0,
                quantity: e.quantity,
                item_id: e.item_id as i32,
                item_context: 0,
                bonus_list_ids: vec![],
                can_loot: true,
            })
            .collect();

        let response = LootResponse {
            owner: player_guid,
            loot_obj: req.unit,
            failure_reason: 0, // No error
            acquire_reason: 0,
            loot_method: 0,   // FreeForAll
            threshold: 2,
            coins,
            items,
            acquired: true,
            ae_looting: false,
        };
        self.send_packet(&response);
    }

    /// CMSG_LOOT_ITEM — player clicks to take a specific item from the loot.
    pub async fn handle_loot_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let req = match LootItemPkt::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => { warn!("Bad LootItem: {e}"); return; }
        };

        let player_guid = match self.player_guid { Some(g) => g, None => return };

        // Collect (loot_obj, list_id, item_id) to send outside the borrow.
        let mut taken_items: Vec<(ObjectGuid, u8, u32)> = Vec::new();

        for loot_req in &req.requests {
            if let Some(loot) = self.loot_table.get_mut(&loot_req.object) {
                if let Some(entry) = loot.items.iter_mut()
                    .find(|e| e.loot_list_id == loot_req.loot_list_id && !e.taken)
                {
                    entry.taken = true;
                    taken_items.push((loot_req.object, entry.loot_list_id, entry.item_id));
                }
            }
        }

        for (loot_obj, list_id, item_id) in taken_items {
            let removed = LootRemoved {
                owner: player_guid,
                loot_obj,
                loot_list_id: list_id,
            };
            self.send_packet(&removed);
            debug!(account = self.account_id, item = item_id, "Looted item");
            // TODO: actually add item to player inventory (DB write).
        }
    }

    /// CMSG_LOOT_RELEASE — player closes the loot window.
    ///
    /// C# ref: `LootHandler.DoLootRelease` (creature branch):
    ///   if loot.IsLooted() && creature.IsFullyLooted() → RemoveDynamicFlag(Lootable)
    ///   → creature.AllLootRemovedFromCorpse() → sets `m_corpseRemoveTime = now + decay`
    pub async fn handle_loot_release(&mut self, mut pkt: wow_packet::WorldPacket) {
        let req = match LootRelease::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => { warn!("Bad LootRelease: {e}"); return; }
        };

        debug!(account = self.account_id, unit = ?req.unit, "CMSG_LOOT_RELEASE");

        // Check if loot is fully taken (all items picked up).
        // Coins are auto-consumed when the loot window opens (sent in LootResponse),
        // so we only check items here.
        // C# ref: `loot.IsLooted()` → no more non-taken items.
        let fully_looted = self.loot_table
            .get(&req.unit)
            .map(|loot| loot.items.iter().all(|e| e.taken))
            .unwrap_or(true); // If no entry at all, treat as fully looted.

        // Remove loot entry from memory.
        self.loot_table.remove(&req.unit);

        let player_guid = match self.player_guid { Some(g) => g, None => return };

        // Acknowledge the release to the client.
        let release = SLootRelease {
            unit: req.unit,
            loot_obj: req.unit,
        };
        self.send_packet(&release);

        // Start corpse despawn timer if fully looted.
        // C# uses `RateCorpseDecayLooted` config × `m_corpseDelay` (default 60s).
        // We use a simple 30s fixed decay.
        if fully_looted {
            if let Some(creature) = self.creatures.get_mut(&req.unit) {
                if !creature.is_alive && creature.corpse_despawn_at.is_none() {
                    const CORPSE_DECAY_SECS: u64 = 30;
                    creature.corpse_despawn_at =
                        Some(Instant::now() + Duration::from_secs(CORPSE_DECAY_SECS));
                    info!(
                        "Creature {:?} (entry {}) fully looted — despawning in {}s",
                        req.unit, creature.entry, CORPSE_DECAY_SECS
                    );
                }
            }
        }

        let _ = player_guid;
    }
}

// ── Loot generation ───────────────────────────────────────────────

/// Generate loot for a dead creature.
///
/// For now: random coins based on level. Item drops TODO (needs loot template DB query).
fn generate_creature_loot(creature_guid: ObjectGuid, level: u8, _entry: u32) -> CreatureLoot {
    // Coin drop: roughly (level * 2) to (level * 5) copper, converted to silver/gold.
    // 1 gold = 10000 copper, 1 silver = 100 copper.
    let base = level as u32;
    // Use GUID counter as a cheap seed for randomness.
    let seed = creature_guid.counter() as u32;
    let copper = base * 200 + (seed % (base * 300 + 1));
    let coins = copper; // Sent as raw copper to client.

    // TODO: query creature_loot_template from world DB for item drops.
    let items = vec![];

    CreatureLoot {
        loot_guid: creature_guid,
        coins,
        items,
        looted_by_player: false,
    }
}
