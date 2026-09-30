// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

impl WorldSession {
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
        let opt = self.player_gossip_option_like_cpp(select.gossip_option_id);
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
}
