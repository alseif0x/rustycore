// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Applying one committed cross-session command.
//!
//! The mailbox owns the queue — draining, rail ordering, backpressure and the
//! durable-overflow kick. What a command *does* is gameplay, and #368 moved it
//! out of `session/mailbox/` so the kernel names one entry point instead of the
//! 31 `handlers/` methods these arms reach.
//!
//! The match stays exhaustive with no wildcard on purpose. Unlike the opcode
//! table #359 retired, this pairing is checked by the compiler: a new
//! `SessionCommand` variant does not build until it is applied somewhere. That
//! property is the reason the arms moved intact rather than becoming thunks.

use crate::session::mailbox::{SessionCommand, WorldSessionShutdownFlushResultLikeCpp};
use crate::session::{SessionHandlerCatalogsLikeCpp, SessionState, WorldSession};
// The `LootMoney` command receivers are default methods of the application loot
// owner's host trait (#1263 F5); the arms below keep their original call text.
use wow_world_loot_ops::LootHandlerHostLikeCpp;

impl WorldSession {
    /// Apply one committed command, in the order the mailbox presented it.
    pub(crate) async fn apply_session_command_with_catalogs_like_cpp(
        &mut self,
        catalogs: &SessionHandlerCatalogsLikeCpp,
        command: SessionCommand,
    ) {
        match command {
            SessionCommand::KickLikeCpp(command) => {
                self.kick(&command.reason);
            }
            SessionCommand::WorldSessionShutdownFlushLikeCpp(command) => {
                let _ = command
                    .response_tx
                    .try_send(WorldSessionShutdownFlushResultLikeCpp {
                        diff_ms: command.diff_ms,
                        disconnecting: self.is_disconnecting(),
                    });
            }
            SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command) => {
                wow_world_entities::handle_apply_creature_melee_damage_like_cpp_command_like_cpp(
                    self, command,
                );
            }
            SessionCommand::ApplyPlayerMeleeResultLikeCpp(command) => {
                wow_world_entities::handle_apply_player_melee_result_like_cpp_command_like_cpp(
                    self, command,
                );
            }
            SessionCommand::CreatureAttackStartLikeCpp(command) => {
                wow_world_entities::handle_creature_attack_start_like_cpp_command_like_cpp(
                    self, command,
                );
            }
            SessionCommand::CreatureAttackStopLikeCpp(command) => {
                wow_world_entities::handle_creature_attack_stop_like_cpp_command_like_cpp(
                    self, command,
                );
            }
            SessionCommand::ReconcilePvpCombatExpiryLikeCpp(command) => {
                wow_world_entities::handle_reconcile_pvp_combat_expiry_like_cpp(self, command);
            }
            SessionCommand::ApplyLootMoneyLikeCpp(command) => {
                self.handle_apply_loot_money_with_generator_like_cpp_command(
                    catalogs.id_generators.item.as_ref(),
                    command,
                )
                .await;
            }
            SessionCommand::NotifyLootMoneyRemovedLikeCpp(command) => {
                self.handle_notify_loot_money_removed_like_cpp_command(command);
            }
            SessionCommand::MasterLootGive(command) => {
                wow_world_loot_ops::handle_represented_master_loot_give_command_with_generator_like_cpp(
                    self,
                    catalogs.id_generators.item.as_ref(),
                    command,
                )
                .await;
            }
            SessionCommand::LootRollStoreWinner(command) => {
                wow_world_loot_ops::handle_represented_loot_roll_store_winner_command_with_generator_like_cpp(
                    self,
                    catalogs.id_generators.item.as_ref(),
                    command,
                )
                .await;
            }
            SessionCommand::LootRollVote(command) => {
                wow_world_loot_ops::handle_represented_loot_roll_vote_command_with_generator_like_cpp(
                    self,
                    catalogs.id_generators.item.as_ref(),
                    catalogs.item_valuation.as_ref(),
                    command,
                )
                .await;
            }
            SessionCommand::ResetSeasonalQuestStatus(command) => {
                let _ = wow_world_quest::reset_seasonal_quest_status_like_cpp(
                    self,
                    command.event_id,
                    command.event_start_time,
                );
            }
            SessionCommand::SendVisibleObjectValuesUpdate(command) => {
                self.handle_send_visible_object_values_update_command_like_cpp(command);
            }
            SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(command) => {
                self.handle_refresh_visible_world_creatures_with_catalogs_like_cpp_command_like_cpp(
                    catalogs.creature_spawns.as_ref(),
                    command,
                )
                .await;
            }
            SessionCommand::DestroyVisibleObjectLikeCpp(command) => {
                wow_world_entities::handle_destroy_visible_object_like_cpp_command_like_cpp(
                    self, command,
                );
            }
            SessionCommand::RefreshDeferredPlayerVisibilityLikeCpp(intent) => {
                wow_world_entities::apply_deferred_player_visibility_refresh_like_cpp(
                    self,
                    catalogs.creature_spawns.as_ref(),
                    intent,
                )
                .await;
            }
            SessionCommand::SendCreatureLootReleaseValuesUpdateLikeCpp(command) => {
                wow_world_loot_ops::handle_send_creature_loot_release_values_update_command_like_cpp(
                    self, command,
                );
            }
            SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp => {
                let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
            }
            SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(command) => {
                wow_world_loot_ops::handle_sync_gathering_node_gameobject_state_and_refresh_like_cpp(
                    self, command,
                );
            }
            SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command) => {
                wow_world_loot_ops::handle_sync_chest_gameobject_state_and_refresh_like_cpp(
                    self, command,
                );
            }
            SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(command) => {
                wow_world_loot_ops::handle_sync_goober_gameobject_state_and_refresh_like_cpp(
                    self, command,
                );
            }
            SessionCommand::SetQuestSharingInfoAndSendDetails(command) => {
                wow_world_quest::handle_set_quest_sharing_info_and_send_details_command_like_cpp(
                    self, command,
                );
            }
            SessionCommand::SendRepeatableTurnInRequestItemsLikeCpp(command) => {
                wow_world_quest::handle_send_repeatable_turn_in_request_items_command_like_cpp(
                    self, command,
                );
            }
            SessionCommand::SendRealmPacketLikeCpp(command) => {
                if self.state() == SessionState::LoggedIn
                    && self.player_guid() == Some(command.recipient)
                {
                    self.core.send_raw_packet_realm(&command.packet_bytes);
                }
            }
            SessionCommand::SendPartyUpdateLikeCpp(command) => {
                wow_world_instances::handle_send_party_update_command_like_cpp(self, command);
            }
            SessionCommand::ApplyGroupRemovalLikeCpp(command) => {
                wow_world_instances::handle_apply_group_removal_command_like_cpp(self, command);
            }
            SessionCommand::ApplyGroupJoinLikeCpp(command) => {
                wow_world_instances::handle_apply_group_join_command_like_cpp(self, command);
            }
            SessionCommand::ApplyGroupDifficultyLikeCpp(command) => {
                wow_world_instances::handle_apply_group_difficulty_command_like_cpp(self, command);
            }
            SessionCommand::ApplyGroupSubgroupLikeCpp(command) => {
                wow_world_instances::handle_apply_group_subgroup_command_like_cpp(self, command);
            }
            SessionCommand::SendIfVisibleLikeCpp(command) => {
                self.handle_send_if_visible_like_cpp_command_like_cpp(command, false);
            }
            SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(command) => {
                self.handle_send_creature_spell_cast_if_visible_like_cpp_command_like_cpp(command);
            }
            SessionCommand::SendPlayerSpellIfVisibleLikeCpp(command) => {
                wow_world_spell::handle_player_cast_publication_like_cpp(self, command);
            }
            SessionCommand::SendRealmIfVisibleLikeCpp(command) => {
                self.handle_send_if_visible_like_cpp_command_like_cpp(command, true);
            }
            SessionCommand::SendAddonIfRegisteredLikeCpp(command) => {
                self.handle_send_addon_if_registered_like_cpp_command_like_cpp(command);
            }
            SessionCommand::CancelRepresentedTradeLikeCpp(command) => {
                self.handle_cancel_represented_trade_command_like_cpp(command);
            }
            SessionCommand::SendRepresentedTradeStatusLikeCpp(command) => {
                self.handle_send_represented_trade_status_command_like_cpp(command);
            }
            SessionCommand::UnacceptRepresentedTradeLikeCpp(command) => {
                self.handle_unaccept_represented_trade_command_like_cpp(command);
            }
            SessionCommand::SendRepresentedDuelCountdownLikeCpp(command) => {
                self.send_raw_packet(&command.packet_bytes);
            }
            SessionCommand::SendRepresentedDuelRequestedLikeCpp(command) => {
                {
                    let (s, mut h) = crate::session::split_social_mut(self);
                    s.handle_send_represented_duel_requested_command_like_cpp(&mut h, command)
                };
            }
        }
    }
}
