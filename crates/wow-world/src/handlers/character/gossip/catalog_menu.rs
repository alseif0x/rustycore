// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_persistence::{
    GossipBroadcastTextLocaleRequestLikeCpp, GossipCatalogReadOutcomeLikeCpp,
    GossipCreatureMenuRequestLikeCpp, GossipMenuCatalogRequestLikeCpp,
    GossipNpcTextCatalogRequestLikeCpp,
};

use super::*;

impl WorldSession {
    /// Build a GossipMessage from the database for a creature entry.
    /// Returns None if no gossip menu exists.
    pub(crate) async fn build_gossip_menu(
        &mut self,
        entry: u32,
        npc_flags: u32,
        npc_guid: wow_core::ObjectGuid,
    ) -> Option<GossipMessage> {
        use crate::session::GossipOptionInfo;
        use wow_packet::packets::gossip::ClientGossipOption;

        let catalog = self.gossip_catalog_persistence_port_like_cpp()?;

        // 1. Get MenuID from creature_template_gossip
        let menu_id = match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            catalog.load_creature_gossip_menu_id_like_cpp(GossipCreatureMenuRequestLikeCpp {
                creature_entry: entry,
            }),
        )
        .await
        {
            Ok(GossipCatalogReadOutcomeLikeCpp::Found(menu_id)) => menu_id,
            _ => return None,
        };

        let condition_store = self.condition_store().cloned();

        // 2. Get TextID from gossip_menu, then resolve BroadcastTextID from npc_text.
        // C++ Player::GetGossipTextId iterates every gossip_menu row and keeps the last row whose
        // attached GossipMenu conditions meet for (player, source).
        let text_ids = match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            catalog.load_gossip_menu_text_ids_like_cpp(GossipMenuCatalogRequestLikeCpp { menu_id }),
        )
        .await
        {
            Ok(GossipCatalogReadOutcomeLikeCpp::Found(text_ids)) => text_ids,
            Ok(GossipCatalogReadOutcomeLikeCpp::Missing) => Vec::new(),
            _ => return None,
        };
        let npc_text_id: u32 = if text_ids.is_empty() {
            1
        } else {
            let mut selected = 1;
            for text_id in text_ids {
                let meets = condition_store.as_ref().is_none_or(|store| {
                    self.gossip_menu_text_conditions_meet_like_cpp(
                        store.as_ref(),
                        menu_id,
                        text_id,
                        npc_guid,
                    )
                });
                if meets {
                    selected = text_id;
                }
            }
            selected
        };

        // Resolve BroadcastTextID from npc_text; C++ `GossipMessage::Write`
        // carries optional TextID and BroadcastTextID separately
        // (`Server/Packets/NPCPackets.cpp:106-130`).
        let broadcast_text_id = match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            catalog.load_npc_text_broadcast_id_like_cpp(GossipNpcTextCatalogRequestLikeCpp {
                npc_text_id,
            }),
        )
        .await
        {
            Ok(GossipCatalogReadOutcomeLikeCpp::Found(broadcast_text_id)) => {
                Some(broadcast_text_id)
            }
            _ => None,
        };
        info!(
            "Gossip menu_id={} npc_text_id={} broadcast_text_id={:?}",
            menu_id, npc_text_id, broadcast_text_id
        );

        // 3. Get options from gossip_menu_option
        let raw_options = match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            catalog.load_gossip_menu_options_like_cpp(GossipMenuCatalogRequestLikeCpp { menu_id }),
        )
        .await
        {
            Ok(GossipCatalogReadOutcomeLikeCpp::Found(options)) => options,
            Ok(GossipCatalogReadOutcomeLikeCpp::Missing) => Vec::new(),
            _ => return None,
        };

        // Resolve localized text for each option via OptionBroadcastTextID.
        let locale = self.locale.clone();
        info!(
            "Gossip locale='{}' for {} options",
            locale,
            raw_options.len()
        );
        let mut gossip_options = Vec::new();
        let mut stored_options = Vec::new();
        for opt in &raw_options {
            if let Some(store) = condition_store.as_ref()
                && !self.gossip_conditions_meet_like_cpp(
                    store.as_ref(),
                    ConditionSourceType::GossipMenuOption,
                    menu_id,
                    opt.option_id as i32,
                    npc_guid,
                )
            {
                continue;
            }

            let mut text = opt.option_text.clone();

            if opt.option_broadcast_text_id != 0 && locale != "enUS" {
                if let Ok(GossipCatalogReadOutcomeLikeCpp::Found(localized)) = tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    catalog.load_broadcast_text_locale_like_cpp(
                        GossipBroadcastTextLocaleRequestLikeCpp {
                            broadcast_text_id: opt.option_broadcast_text_id,
                            locale: locale.clone(),
                        },
                    ),
                )
                .await
                {
                    if !localized.is_empty() {
                        text = localized;
                    }
                }
            }

            gossip_options.push(ClientGossipOption {
                gossip_option_id: opt.gossip_option_id,
                option_npc: opt.option_npc,
                option_flags: i8::from(opt.box_coded),
                option_cost: opt.box_money as i32,
                option_language: i32::try_from(opt.language).unwrap_or(i32::MAX),
                flags: opt.flags,
                order_index: opt.option_id as i32,
                status: 0,
                text,
                confirm: opt.box_text.clone(),
                spell_id: opt.spell_id,
                override_icon_id: opt.override_icon_id,
            });

            stored_options.push(GossipOptionInfo {
                gossip_option_id: opt.gossip_option_id,
                menu_id: opt.menu_id,
                order_index: opt.option_id,
                option_npc: opt.option_npc,
                action_menu_id: opt.action_menu_id,
            });

            if opt.action_poi_id != 0
                || opt.gossip_npc_option_id.is_some()
                || opt.box_broadcast_text_id != 0
            {
                debug!(
                    account = self.account_id,
                    menu_id = opt.menu_id,
                    option_id = opt.option_id,
                    action_poi_id = opt.action_poi_id,
                    gossip_npc_option_id = ?opt.gossip_npc_option_id,
                    box_broadcast_text_id = opt.box_broadcast_text_id,
                    "Gossip option loaded C++ auxiliary fields for represented runtime"
                );
            }
        }

        // C++ `Player::PrepareGossipMenu` adds a trainer menu option automatically when
        // a creature has trainer flags but no DB gossip option for `GossipOptionNpc::Trainer`.
        // This is required for mixed questgiver+trainer NPCs such as Ranger Sallina.
        add_represented_trainer_gossip_option_if_missing_like_cpp(
            &mut gossip_options,
            &mut stored_options,
            npc_flags,
        );

        let gossip_text = if npc_flags & NPCFlags1::QUEST_GIVER.bits() != 0 {
            self.represented_creature_gossip_text_like_cpp(entry)
        } else {
            Vec::new()
        };

        if gossip_options.is_empty() && gossip_text.is_empty() {
            return None;
        }

        // Store gossip state for when the player selects an option.
        if !self.replace_player_gossip_options_like_cpp(stored_options) {
            return None;
        }
        self.set_player_interaction_source_like_cpp(npc_guid);

        Some(GossipMessage {
            gossip_guid: npc_guid,
            gossip_id: menu_id as i32,
            friendship_faction_id: 0,
            text_id: None,
            broadcast_text_id,
            gossip_options,
            gossip_text,
        })
    }
}
