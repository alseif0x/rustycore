// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session mailbox pump.
//!
//! One consumer — the owning session task — drains both rails and applies each
//! committed command. Draining order is deliberate: the durable creature rail
//! is presented first up to its first visibility-gated packet so a pending
//! refresh can run before it, then the bounded general rail, then the deferred
//! durable suffix. An overflowed durable backlog disconnects the desynchronized
//! session rather than dropping authoritative transitions.

use super::protocol::*;
use crate::session::{SessionState, WorldSession};

impl WorldSession {
    /// Clone the C++-style cross-session command channel for this active
    /// session.
    ///
    /// Worldserver-level registries use this as the Rust equivalent of holding
    /// a `WorldSession*` in `World::m_sessions` for commands such as
    /// `World::KickAll`; session state is still mutated only by the session
    /// task when it drains the channel.
    pub fn session_command_tx(&self) -> flume::Sender<SessionCommand> {
        self.session_command_tx.clone()
    }

    pub(crate) fn drain_session_commands(&self) -> Vec<SessionCommand> {
        let durable_commands = self
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .map(|mut pending| pending.drain_like_cpp())
            .unwrap_or_default();
        // Drain the bounded general rail before the first durable presentation
        // packet so a pending visibility refresh can run first. The rails do
        // not yet share an enqueue ordinal, so this is not a global cross-rail
        // ordering guarantee. The spell pair itself still occupies one durable
        // command and therefore cannot be split by this merge.
        let first_visible = durable_commands
            .iter()
            .position(|command| {
                matches!(
                    command,
                    SessionCommand::SendIfVisibleLikeCpp(_)
                        | SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(_)
                        | SessionCommand::SendRealmIfVisibleLikeCpp(_)
                        | SessionCommand::SendRealmIfVisibleFromLegacySourceLikeCpp(_)
                )
            })
            .unwrap_or(durable_commands.len());
        let mut commands = durable_commands;
        let deferred_durable_suffix = commands.split_off(first_visible);
        while let Ok(command) = self.session_command_rx.try_recv() {
            commands.push(command);
        }
        commands.extend(deferred_durable_suffix);
        commands
    }

    pub(crate) fn take_durable_creature_runtime_overflow_like_cpp(&self) -> bool {
        self.durable_creature_runtime_commands_like_cpp
            .lock()
            .map(|mut pending| pending.take_overflowed_and_discard_like_cpp())
            .unwrap_or(true)
    }

    pub(crate) async fn process_represented_session_commands_like_cpp(&mut self) {
        self.apply_pending_durable_item_loot_completions_like_cpp()
            .await;
        let creature_runtime_overflowed = self.take_durable_creature_runtime_overflow_like_cpp();
        if creature_runtime_overflowed {
            self.kick(
                "authoritative creature runtime command backlog overflowed; disconnecting desynchronized session",
            );
            return;
        }
        let commands = self.drain_session_commands();
        for command in commands {
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
                    self.handle_apply_creature_melee_damage_like_cpp_command_like_cpp(command);
                }
                SessionCommand::CreatureAttackStartLikeCpp(command) => {
                    self.handle_creature_attack_start_like_cpp_command_like_cpp(command);
                }
                SessionCommand::CreatureAttackStopLikeCpp(command) => {
                    self.handle_creature_attack_stop_like_cpp_command_like_cpp(command);
                }
                SessionCommand::ReconcilePvpCombatExpiryLikeCpp(command) => {
                    self.handle_reconcile_pvp_combat_expiry_like_cpp(command);
                }
                SessionCommand::ApplyLootMoneyLikeCpp(command) => {
                    self.handle_apply_loot_money_like_cpp_command(command).await;
                }
                SessionCommand::NotifyLootMoneyRemovedLikeCpp(command) => {
                    self.handle_notify_loot_money_removed_like_cpp_command(command);
                }
                SessionCommand::MasterLootGive(command) => {
                    self.handle_represented_master_loot_give_command_like_cpp(command)
                        .await;
                }
                SessionCommand::LootRollStoreWinner(command) => {
                    self.handle_represented_loot_roll_store_winner_command_like_cpp(command)
                        .await;
                }
                SessionCommand::LootRollVote(command) => {
                    self.handle_represented_loot_roll_vote_command_like_cpp(command)
                        .await;
                }
                SessionCommand::ResetSeasonalQuestStatus(command) => {
                    let _ = self.reset_seasonal_quest_status_like_cpp(
                        command.event_id,
                        command.event_start_time,
                    );
                }
                SessionCommand::SendVisibleObjectValuesUpdate(command) => {
                    self.handle_send_visible_object_values_update_command_like_cpp(command);
                }
                SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(command) => {
                    self.handle_refresh_visible_world_creatures_like_cpp_command_like_cpp(command)
                        .await;
                }
                SessionCommand::SendCreatureLootReleaseValuesUpdateLikeCpp(command) => {
                    self.handle_send_creature_loot_release_values_update_command_like_cpp(command);
                }
                SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp => {
                    let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
                }
                SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(command) => {
                    self.handle_sync_gathering_node_gameobject_state_and_refresh_like_cpp(command);
                }
                SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command) => {
                    self.handle_sync_chest_gameobject_state_and_refresh_like_cpp(command);
                }
                SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(command) => {
                    self.handle_sync_goober_gameobject_state_and_refresh_like_cpp(command);
                }
                SessionCommand::SetQuestSharingInfoAndSendDetails(command) => {
                    self.handle_set_quest_sharing_info_and_send_details_command_like_cpp(command);
                }
                SessionCommand::SendRepeatableTurnInRequestItemsLikeCpp(command) => {
                    self.handle_send_repeatable_turn_in_request_items_command_like_cpp(command);
                }
                SessionCommand::SendRealmPacketLikeCpp(command) => {
                    if self.state() == SessionState::LoggedIn
                        && self.player_guid() == Some(command.recipient)
                    {
                        self.send_raw_packet_realm(&command.packet_bytes);
                    }
                }
                SessionCommand::SendPartyUpdateLikeCpp(command) => {
                    self.handle_send_party_update_command_like_cpp(command);
                }
                SessionCommand::ApplyGroupRemovalLikeCpp(command) => {
                    self.handle_apply_group_removal_command_like_cpp(command);
                }
                SessionCommand::ApplyGroupJoinLikeCpp(command) => {
                    self.handle_apply_group_join_command_like_cpp(command);
                }
                SessionCommand::ApplyGroupDifficultyLikeCpp(command) => {
                    self.handle_apply_group_difficulty_command_like_cpp(command);
                }
                SessionCommand::ApplyGroupSubgroupLikeCpp(command) => {
                    self.handle_apply_group_subgroup_command_like_cpp(command);
                }
                SessionCommand::SendIfVisibleLikeCpp(command) => {
                    self.handle_send_if_visible_like_cpp_command_like_cpp(command, false, false);
                }
                SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(command) => {
                    self.handle_send_creature_spell_cast_if_visible_like_cpp_command_like_cpp(
                        command,
                    );
                }
                SessionCommand::SendRealmIfVisibleLikeCpp(command) => {
                    self.handle_send_if_visible_like_cpp_command_like_cpp(command, true, false);
                }
                SessionCommand::SendRealmIfVisibleFromLegacySourceLikeCpp(command) => {
                    self.handle_send_if_visible_like_cpp_command_like_cpp(command, true, true);
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
                    self.handle_send_represented_duel_countdown_command_like_cpp(command);
                }
                SessionCommand::SendRepresentedDuelRequestedLikeCpp(command) => {
                    self.handle_send_represented_duel_requested_command_like_cpp(command);
                }
            }
        }
        self.flush_pending_visibility_refresh_like_cpp().await;
    }
}
