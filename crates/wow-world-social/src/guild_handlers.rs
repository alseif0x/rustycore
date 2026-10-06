// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Guild invitation handler packet bodies.
//!
//! C++ source of truth: `WorldSession::HandleGuildDeclineInvitation`,
//! `HandleAcceptGuildInvite` and `HandleGuildSetAchievementTracking`
//! (`GuildHandler.cpp`). The family owns the packet bodies and the canonical
//! guild-state transitions; the World session only builds the borrowed social
//! state plus hub context (#1263 F5). The guild-bank handlers stay in the
//! World shell, and the auto-decline handler stays until its registry sync is
//! available to this context.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::misc::{AcceptGuildInvite, GuildSetAchievementTracking};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::HubMut;

use crate::SessionSocialLimits;

/// Borrowed inputs of one guild invitation handler invocation.
pub struct GuildHandlerCxLikeCpp<'a> {
    social: &'a mut SessionSocialLimits,
    hub: HubMut<'a>,
}

impl<'a> GuildHandlerCxLikeCpp<'a> {
    pub fn new(social: &'a mut SessionSocialLimits, hub: HubMut<'a>) -> Self {
        Self { social, hub }
    }

    fn player_guild_state_snapshot_like_cpp(&self) -> Option<wow_entities::PlayerGuildState> {
        crate::player_guild_state_snapshot_like_cpp(&self.hub.shared(), self.social)
    }

    fn accept_guild_invitation_like_cpp(&mut self) -> bool {
        let Some(state) = self.player_guild_state_snapshot_like_cpp() else {
            return false;
        };
        if !state.authority_complete || state.guild_id.is_some() {
            return false;
        }

        let Some(guild_id) = state.invited_guild_id else {
            return false;
        };
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = guild_id;

        #[cfg(any(test, feature = "test-fixtures"))]
        self.social
            .record_represented_guild_accept_invite_for_test_like_cpp(guild_id);
        true
    }

    fn decline_guild_invitation_like_cpp(&mut self) -> bool {
        let Some(state) = self.player_guild_state_snapshot_like_cpp() else {
            return false;
        };
        if !state.authority_complete || state.guild_id.is_some() {
            return false;
        }

        let canonical = self
            .hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.clear_guild_invitation_like_cpp())
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.hub.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_guild_state_like_cpp(|state| state.invited_guild_id = None)
                .is_some();
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    fn mutate_player_guild_state_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerGuildState) -> R,
    ) -> Option<R> {
        crate::mutate_player_guild_state_for_test_like_cpp(&self.hub.shared(), self.social, f)
    }

    /// C++ only delegates when `GetPlayer()->GetGuild()` resolves a live guild.
    /// Rust has no represented guild-achievement manager here yet, so the
    /// no-guild branch remains silent.
    pub async fn handle_guild_set_achievement_tracking(&mut self, mut pkt: WorldPacket) {
        if let Err(error) = GuildSetAchievementTracking::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "GuildSetAchievementTracking parse failed: {error}"
            );
        }
    }

    /// CMSG_GUILD_BANK_REMAINING_WITHDRAW_MONEY_QUERY.
    pub async fn handle_guild_bank_remaining_withdraw_money_query(&mut self, _pkt: WorldPacket) {
        // C++ only sends GuildBankRemainingWithdrawMoney when GetPlayer()->GetGuild()
        // resolves a live guild. Rust has no represented guild-bank manager here
        // yet, so the no-guild branch is correctly silent.
    }

    pub async fn handle_guild_decline_invitation(&mut self, _pkt: WorldPacket) {
        self.decline_guild_invitation_like_cpp();
    }

    pub async fn handle_accept_guild_invite(&mut self, mut pkt: WorldPacket) {
        if let Err(error) = AcceptGuildInvite::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "AcceptGuildInvite parse failed: {error}"
            );
            return;
        }

        self.accept_guild_invitation_like_cpp();
    }
}

/// Builds a guild invitation handler context from a host's social state and hub.
pub trait GuildHandlerHostLikeCpp<C> {
    fn guild_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> GuildHandlerCxLikeCpp<'a>;
}

fn handle_guild_set_achievement_tracking_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_handler_cx_like_cpp(catalogs)
            .handle_guild_set_achievement_tracking(pkt)
            .await;
    })
}

fn handle_guild_bank_remaining_withdraw_money_query_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_handler_cx_like_cpp(catalogs)
            .handle_guild_bank_remaining_withdraw_money_query(pkt)
            .await;
    })
}

fn handle_guild_decline_invitation_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_handler_cx_like_cpp(catalogs)
            .handle_guild_decline_invitation(pkt)
            .await;
    })
}

fn handle_accept_guild_invite_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_handler_cx_like_cpp(catalogs)
            .handle_accept_guild_invite(pkt)
            .await;
    })
}

/// Registers the guild invitation handlers on the packet registry.
pub fn register_guild_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: GuildHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GuildSetAchievementTracking,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_set_achievement_tracking",
        handler: handle_guild_set_achievement_tracking_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GuildDeclineInvitation,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_decline_invitation",
        handler: handle_guild_decline_invitation_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AcceptGuildInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_accept_guild_invite",
        handler: handle_accept_guild_invite_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankRemainingWithdrawMoneyQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_remaining_withdraw_money_query",
        handler: handle_guild_bank_remaining_withdraw_money_query_thunk::<S, C>,
    })?;
    Ok(())
}
