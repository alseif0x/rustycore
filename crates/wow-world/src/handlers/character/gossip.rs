// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Gossip menus and NPC interaction text.

use super::*;

mod binder;
mod conditions;
mod catalog_menu;
mod selection;

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) mod fixture_access;

impl WorldSession {
    /// Direct interaction for NPCs without gossip menus (banker, auctioneer, etc.).
    pub(super) async fn handle_npc_direct_interaction(&mut self, hello: Hello, npc_flags: u32) {
        use wow_packet::packets::misc::{AuctionHelloResponse, NpcInteractionOpenResult};

        // This is Rust's shortcut for C++ `PrepareGossipMenu` followed by a
        // built-in gossip option. C++ `SendGossipMenu` first replaces the
        // complete InteractionData with this validated source, even when the
        // service subsequently emits a dedicated packet.
        self.set_player_interaction_source_like_cpp(hello.unit);

        // This shortcut represents selecting one of C++'s built-in gossip
        // options immediately after publishing it. `HandleGossipSelectOptionOpcode`
        // removes fake death after source validation and before dispatching the
        // service. Keep the empty-menu fallback out of that transition.
        if npc_has_direct_interaction_like_cpp(npc_flags) {
            self.remove_represented_feign_death_if_needed_like_cpp();
        }

        if npc_flags & DIRECT_VENDOR_MASK_LIKE_CPP != 0 {
            self.handle_list_inventory(hello).await;
        } else if npc_flags & DIRECT_TRAINER_MASK_LIKE_CPP != 0 {
            self.handle_trainer_list(hello).await;
        } else if npc_flags & DIRECT_AUCTIONEER_LIKE_CPP != 0 {
            self.send_packet(&AuctionHelloResponse::open(hello.unit));
        } else if npc_flags & DIRECT_BANKER_LIKE_CPP != 0 {
            self.send_show_bank_like_cpp(hello.unit);
        } else if npc_flags & DIRECT_FLIGHT_MASTER_LIKE_CPP != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 6));
        } else if npc_flags & DIRECT_TABARD_DESIGNER_LIKE_CPP != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 14));
        } else if npc_flags & DIRECT_STABLE_MASTER_LIKE_CPP != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 22));
        } else if npc_flags & DIRECT_GUILD_BANKER_LIKE_CPP != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 10));
        } else {
            self.send_packet(&GossipMessage::empty(hello.unit, 0, 1));
        }
    }

    pub async fn handle_gossip_hello(&mut self, hello: Hello) {
        info!(
            "GossipHello for {:?} from account {}",
            hello.unit, self.account_id
        );

        const GOSSIP_FLAG: u32 = 0x1;

        // C++ `HandleGossipHelloOpcode` resolves the creature through
        // `GetNPCIfCanInteractWith(..., UNIT_NPC_FLAG_GOSSIP, ...)` before
        // preparing DB-backed gossip, including quest text synthesized from a
        // gossip menu with no options.
        let gossip_access =
            self.represented_npc_can_interact_with_like_cpp(hello.unit, GOSSIP_FLAG, 0);
        let trainer_access = match gossip_access {
            Some(access) if access.npc_flags & TRAINER_NPC_FLAGS_MASK_LIKE_CPP != 0 => Some(access),
            Some(_) => None,
            None => self.represented_npc_can_interact_with_like_cpp(
                hello.unit,
                TRAINER_NPC_FLAGS_MASK_LIKE_CPP,
                0,
            ),
        };
        let Some(validated_access) = gossip_access.as_ref().or(trainer_access.as_ref()) else {
            debug!(
                account = self.account_id,
                source = ?hello.unit,
                "GossipHello rejected before clearing or publishing player-menu state"
            );
            return;
        };
        let (resolved_npc_flags, resolved_entry) =
            (validated_access.npc_flags, validated_access.entry);
        info!(
            "GossipHello npc_flags=0x{:X} entry={} for {:?}",
            resolved_npc_flags, resolved_entry, hello.unit
        );

        // C++ pauses the creature and clears PlayerMenu only after
        // GetNPCIfCanInteractWith has accepted the source.
        self.mutate_world_creature(hello.unit, |creature| {
            creature.pause_interaction_movement_like_cpp();
        });
        self.clear_player_gossip_options_like_cpp();

        if let Some(access) = gossip_access.as_ref() {
            if let Some(msg) = self
                .build_gossip_menu(access.entry, access.npc_flags, hello.unit)
                .await
            {
                info!(
                    "Sending GossipMessage with {} options and {} quests for entry {}",
                    msg.gossip_options.len(),
                    msg.gossip_text.len(),
                    access.entry
                );
                self.send_packet(&msg);
                return;
            }
        }

        if let Some(access) = trainer_access {
            if self.send_represented_creature_trainer_gossip_menu_like_cpp(
                hello.unit,
                access.entry,
                access.npc_flags,
            ) {
                info!(
                    "GossipHello trainer fallback sent prepared gossip menu for entry={} {:?}",
                    access.entry, hello.unit
                );
                return;
            }
        }

        // No DB gossip menu found. C++ `HandleQuestgiverHelloOpcode` uses the
        // same prepared-gossip path as `HandleGossipHelloOpcode`; the represented
        // seam currently models the quest part of that prepared menu.
        if (resolved_npc_flags & NPCFlags1::QUEST_GIVER.bits()) != 0
            && !npc_has_direct_interaction_like_cpp(resolved_npc_flags)
            && resolved_entry != 0
        {
            if self
                .represented_npc_can_interact_with_like_cpp(
                    hello.unit,
                    NPCFlags1::QUEST_GIVER.bits(),
                    0,
                )
                .is_none()
            {
                debug!(
                    "GossipHello questgiver fallback rejected by C++ interaction checks for {:?}",
                    hello.unit
                );
                return;
            }
            if self.use_represented_creature_questgiver_like_cpp(hello.unit, resolved_entry) {
                info!(
                    "GossipHello questgiver fallback consumed entry={} for {:?}",
                    resolved_entry, hello.unit
                );
                return;
            }
            info!(
                "GossipHello questgiver fallback found no quest menu for entry={} {:?}",
                resolved_entry, hello.unit
            );
        }

        // No gossip or quest menu found — fall back to direct interaction based on NPC flags.
        self.handle_npc_direct_interaction(hello, resolved_npc_flags)
            .await;
    }

    pub(crate) fn send_close_gossip_like_cpp(&mut self) {
        self.reset_player_interaction_data_like_cpp();
        self.send_packet_realm(&GossipComplete {
            suppress_sound: false,
        });
    }

    // ── NPC activation handlers ───────────────────────────────────────────────

    /// Handle CMSG_QUERY_NPC_TEXT — client requests NPC text for gossip.
    pub async fn handle_query_npc_text(&mut self, query: QueryNpcText) {
        debug!(
            "QueryNpcText: text_id={} for account {}",
            query.text_id, self.account_id
        );

        // For now, respond with a default "found" response.
        // BroadcastTextID=0 tells the client to use local DB2 data for text.
        self.send_packet(&QueryNpcTextResponse::with_text(query.text_id, 0));
    }
}
