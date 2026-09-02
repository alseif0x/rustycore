// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Gossip menus and NPC interaction text.

use wow_persistence::{
    GossipBroadcastTextLocaleRequestLikeCpp, GossipCatalogReadOutcomeLikeCpp,
    GossipCreatureMenuRequestLikeCpp, GossipMenuCatalogRequestLikeCpp,
    GossipNpcTextCatalogRequestLikeCpp,
};

use super::*;

impl WorldSession {
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
        self.gossip_options.clear();

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

    fn gossip_conditions_meet_like_cpp(
        &mut self,
        condition_store: &ConditionEntriesByTypeStore,
        source_type: ConditionSourceType,
        source_group: u32,
        source_entry: i32,
        npc_guid: ObjectGuid,
    ) -> bool {
        let Some(conditions) = condition_store
            .conditions_for_like_cpp(source_type, ConditionId::new(source_group, source_entry, 0))
        else {
            return true;
        };

        let Some(player_object) = self.build_condition_player_object_like_cpp() else {
            warn!(
                "Gossip condition check failed closed: missing player object for {:?}",
                source_type
            );
            return false;
        };
        let Some((source_object, source_unit_snapshot)) =
            self.build_condition_creature_object_like_cpp(npc_guid)
        else {
            warn!(
                "Gossip condition check failed closed: missing source object for {:?}",
                source_type
            );
            return false;
        };

        let Some(player_unit_snapshot) = self.condition_player_unit_snapshot_like_cpp() else {
            return false;
        };
        let player_snapshot = self.condition_player_snapshot_like_cpp();
        let player_condition_store = self.player_condition_store().cloned();
        let Some(player_condition_context) = self.represented_player_condition_context_like_cpp()
        else {
            return false;
        };

        let mut source_info = crate::conditions::ConditionSourceInfo::from_targets(
            Some(&player_object),
            Some(&source_object),
            None,
        );
        source_info.set_unit_target_snapshot(0, player_unit_snapshot);
        source_info.set_player_target_snapshot(0, player_snapshot);
        source_info.set_unit_target_snapshot(1, source_unit_snapshot);
        if let Some(store) = player_condition_store.as_ref() {
            source_info.set_player_condition_store(store.as_ref());
            if let Some(context) = player_condition_context.as_context(self) {
                source_info.set_player_condition_context(0, context);
            }
        }

        crate::conditions::is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            |condition, source_info| match crate::conditions::condition_meets_basic_like_cpp(
                condition,
                source_info,
                |current_area, required_area| current_area == required_area,
            ) {
                crate::conditions::ConditionMeetResult::Evaluated(value) => value,
                crate::conditions::ConditionMeetResult::Unsupported => {
                    warn!(
                        "Gossip condition check failed closed: unsupported {:?} for {:?} {}:{}",
                        condition.condition_type, source_type, source_group, source_entry
                    );
                    false
                }
            },
        )
    }

    fn gossip_menu_text_conditions_meet_like_cpp(
        &mut self,
        condition_store: &ConditionEntriesByTypeStore,
        menu_id: u32,
        text_id: u32,
        npc_guid: ObjectGuid,
    ) -> bool {
        if condition_store
            .conditions_for_like_cpp(
                ConditionSourceType::GossipMenu,
                ConditionId::new(menu_id, text_id as i32, 0),
            )
            .is_some()
        {
            return self.gossip_conditions_meet_like_cpp(
                condition_store,
                ConditionSourceType::GossipMenu,
                menu_id,
                text_id as i32,
                npc_guid,
            );
        }

        self.gossip_conditions_meet_like_cpp(
            condition_store,
            ConditionSourceType::GossipMenu,
            menu_id,
            0,
            npc_guid,
        )
    }

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
        self.gossip_options = stored_options;
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

    pub(crate) fn send_close_gossip_like_cpp(&mut self) {
        self.reset_player_interaction_data_like_cpp();
        self.send_packet_realm(&GossipComplete {
            suppress_sound: false,
        });
    }

    /// Handle CMSG_GOSSIP_SELECT_OPTION — player selects a gossip menu option.
    ///
    /// Routes to the appropriate handler based on the option's OptionNpc value:
    /// 1=Vendor, 3=Trainer, 5=Binder, etc.
    pub async fn handle_gossip_select_option(
        &mut self,
        select: wow_packet::packets::gossip::GossipSelectOption,
    ) {
        use wow_packet::packets::misc::NpcInteractionOpenResult;

        info!(
            "GossipSelectOption: gossip_id={}, option_id={} from account {}",
            select.gossip_id, select.gossip_option_id, self.account_id
        );

        // Find the selected option in our stored gossip data.
        let opt = self
            .gossip_options
            .iter()
            .find(|o| o.gossip_option_id == select.gossip_option_id)
            .cloned();
        let opt = match opt {
            Some(o) => o,
            None => {
                warn!(
                    "GossipSelectOption: unknown gossip_option_id={} — ignoring like C++.",
                    select.gossip_option_id
                );
                return;
            }
        };
        let (option_npc, _action_menu_id) = (opt.option_npc, opt.action_menu_id);

        if self.player_interaction_source_guid_like_cpp() != Some(select.gossip_unit) {
            warn!(
                account = self.account_id,
                requested_source = ?select.gossip_unit,
                active_source = ?self.player_interaction_source_guid_like_cpp(),
                "GossipSelectOption rejected: interaction source mismatch"
            );
            return;
        }
        let npc_guid = select.gossip_unit;
        let source_is_interactable = if npc_guid.is_any_type_creature() {
            let required_flags = if option_npc == GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP {
                NPCFlags1::GOSSIP.bits() | TRAINER_NPC_FLAGS_MASK_LIKE_CPP
            } else {
                NPCFlags1::GOSSIP.bits()
            };
            self.represented_npc_can_interact_with_like_cpp(npc_guid, required_flags, 0)
                .is_some()
        } else if npc_guid.is_game_object() {
            self.represented_gameobject_gossip_can_interact_with_like_cpp(npc_guid)
                .is_some()
        } else {
            false
        };
        if !source_is_interactable {
            warn!(
                account = self.account_id,
                source = ?npc_guid,
                option_npc = option_npc,
                "GossipSelectOption rejected: source no longer interactable"
            );
            return;
        }
        // C++ removes fake death after revalidating the interaction source and
        // before `Player::OnGossipSelect` validates the menu ID.
        self.remove_represented_feign_death_if_needed_like_cpp();
        // The C++ base gossip path rejects a packet menu that is not the
        // currently published menu before executing its built-in action.
        if opt.menu_id != select.gossip_id as u32 {
            warn!(
                account = self.account_id,
                requested_menu_id = select.gossip_id,
                active_menu_id = opt.menu_id,
                "GossipSelectOption rejected: active menu mismatch"
            );
            return;
        }
        if npc_guid.is_game_object() && option_npc != 0 {
            warn!(
                account = self.account_id,
                source = ?npc_guid,
                option_npc,
                "GossipSelectOption rejected: GameObject option is not C++ OptionNpc::None"
            );
            return;
        }
        info!(
            "GossipSelectOption: OptionNpc={} for {:?}",
            option_npc, npc_guid
        );

        let hello = Hello { unit: npc_guid };
        match option_npc {
            1 => {
                // Vendor
                self.handle_list_inventory(hello).await;
            }
            2 => {
                // Taxinode / Flight Master
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 6));
            }
            3 => {
                // Trainer
                self.handle_trainer_list_for_gossip_option_like_cpp(
                    hello,
                    opt.menu_id,
                    opt.order_index,
                )
                .await;
            }
            5 => {
                // Binder (Innkeeper)
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 20));
            }
            6 => {
                // Banker
                self.send_show_bank_like_cpp(npc_guid);
            }
            8 => {
                // Guild Tabard Vendor
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 14));
            }
            9 => {
                // Battlemaster
                info!("Battlemaster interaction (stub)");
            }
            10 => {
                // Auctioneer
                use wow_packet::packets::misc::AuctionHelloResponse;
                self.send_packet(&AuctionHelloResponse::open(npc_guid));
            }
            12 => {
                // Stable Master
                self.send_packet(&NpcInteractionOpenResult::new(npc_guid, 22));
            }
            _ => {
                info!(
                    "GossipSelectOption: unhandled OptionNpc={} — ignored",
                    option_npc
                );
            }
        }
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
