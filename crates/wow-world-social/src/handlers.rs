// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inspect-family packet handlers owned by the Social boundary.
//!
//! C++ source of truth: `WorldSession::HandleInspect`,
//! `WorldSession::HandleRequestHonorStats` and
//! `WorldSession::HandleQueryInspectAchievements`
//! (`src/server/game/Handlers/...`). The handlers read the represented player
//! directory, the caller's own placement and publish; the World session only
//! supplies the borrowed context and implements the host trait (#1263 F5).

use std::sync::Arc;

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_core::{ObjectGuid, Position};
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::inspect::{
    InspectHonorStatsResponse, InspectItem, InspectResult, QueryInspectAchievements,
    RequestHonorStats, RespondInspectAchievements,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::directory::PlayerRegistry;
use wow_world_core::session::{PacketPublicationAccessLikeCpp, SharedCanonicalMapManager};

/// C++ `INSPECT_DISTANCE`: 2D range that gates the achievement query.
const INSPECT_DISTANCE_LIKE_CPP: f32 = 28.0;

/// Borrowed inputs of one inspect-family handler invocation.
///
/// Every input is read once when the context is built, so the three handlers
/// observe one coherent snapshot of the caller and the directory.
pub struct InspectHandlerCxLikeCpp<'a> {
    registry: Option<&'a Arc<PlayerRegistry>>,
    canonical_map_manager: Option<&'a SharedCanonicalMapManager>,
    player_map_id: u16,
    player_position: Option<Position>,
    player_faction_template_id: Option<u32>,
    publication: PacketPublicationAccessLikeCpp<'a>,
}

impl<'a> InspectHandlerCxLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registry: Option<&'a Arc<PlayerRegistry>>,
        canonical_map_manager: Option<&'a SharedCanonicalMapManager>,
        player_map_id: u16,
        player_position: Option<Position>,
        player_faction_template_id: Option<u32>,
        publication: PacketPublicationAccessLikeCpp<'a>,
    ) -> Self {
        Self {
            registry,
            canonical_map_manager,
            player_map_id,
            player_position,
            player_faction_template_id,
            publication,
        }
    }

    /// CMSG_INSPECT (0x3529)
    ///
    /// Parse: packed_guid
    pub fn handle_inspect(&self, mut pkt: WorldPacket) {
        let target_guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(e) => {
                warn!("Inspect: failed to read target_guid: {}", e);
                return;
            }
        };

        let Some(registry) = self.registry else {
            return;
        };

        let entry = match registry.inspect_snapshot(target_guid) {
            Some(entry) => entry,
            None => {
                warn!("Inspect: target {:?} not found in registry", target_guid);
                return;
            }
        };

        // Build item list from visible_items: [(item_id, enchant_display, subclass); 19]
        let mut items: Vec<InspectItem> = Vec::new();
        for (slot, (item_id, _, _)) in entry.visible_items.iter().enumerate() {
            if *item_id != 0 {
                items.push(InspectItem {
                    slot: slot as u8,
                    item_id: *item_id,
                });
            }
        }

        let result = InspectResult {
            target_guid,
            target_name: entry.player_name.clone(),
            race: entry.race,
            class_id: entry.class,
            gender: entry.sex,
            level: entry.level as u32,
            items,
        };

        self.publication.send_packet(&result);
    }

    /// CMSG_REQUEST_HONOR_STATS (0x317e)
    ///
    /// C++ `WorldSession::HandleRequestHonorStats` finds the connected target
    /// player and returns `SMSG_INSPECT_HONOR_STATS`; missing targets produce no
    /// response.
    pub fn handle_request_honor_stats(&self, mut pkt: WorldPacket) {
        let request = match RequestHonorStats::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                warn!("RequestHonorStats: failed to read target: {}", e);
                return;
            }
        };

        let Some(registry) = self.registry else {
            return;
        };

        // C++ `HandleInspectHonorStats` returns without answering when it cannot
        // resolve the target. `None` here means the same thing — including the far
        // teleport window, where the canonical `Player` has left the old map and
        // not yet reached the destination — so we must not answer with zeros and
        // report a real honor level as lost (#252).
        let Some((
            lifetime_hk,
            this_week_contribution,
            yesterday_contribution,
            today_hk,
            yesterday_hk,
            lifetime_max_rank,
            honor_level,
        )) = registry.inspect_honor_stats(request.target, self.canonical_map_manager)
        else {
            return;
        };

        let response = InspectHonorStatsResponse {
            target: request.target,
            lifetime_hk,
            today_contribution: this_week_contribution,
            yesterday_contribution,
            today_hk,
            yesterday_hk,
            lifetime_max_rank,
            honor_level,
        };

        self.publication.send_packet(&response);
    }

    /// CMSG_QUERY_INSPECT_ACHIEVEMENTS (0x3500)
    ///
    /// C++ `HandleQueryInspectAchievements` returns silently if the target is
    /// missing, out of inspect range (`INSPECT_DISTANCE`, 2D), or a valid
    /// attack target. Rust currently has no `PlayerAchievementMgr`, so the
    /// success branch sends a structurally correct empty
    /// `SMSG_RESPOND_INSPECT_ACHIEVEMENTS`.
    pub fn handle_query_inspect_achievements(&self, mut pkt: WorldPacket) {
        let request = match QueryInspectAchievements::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                warn!("QueryInspectAchievements: failed to read target: {}", e);
                return;
            }
        };

        let Some(registry) = self.registry else {
            return;
        };

        let target = match registry.inspect_snapshot(request.guid) {
            Some(target) => target,
            None => return,
        };

        let Some(self_position) = self.player_position else {
            return;
        };

        if target.map_id != self.player_map_id
            || !target
                .position
                .is_within_dist_2d(&self_position, INSPECT_DISTANCE_LIKE_CPP)
        {
            return;
        }

        // Conservative represented `IsValidAttackTarget` guard: without the
        // full faction/PvP combat targetability graph, reject clearly different
        // non-zero faction-template pairs rather than leaking inspect data.
        let self_faction = self.player_faction_template_id.unwrap_or(0);
        if self_faction != 0
            && target.faction_template_id != 0
            && self_faction != target.faction_template_id
        {
            return;
        }

        self.publication.send_packet(&RespondInspectAchievements {
            player: request.guid,
        });
    }
}

/// Builds the inspect handler context from a host's session state.
pub trait SocialInspectHandlerHostLikeCpp<C> {
    fn inspect_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> InspectHandlerCxLikeCpp<'a>;
}

fn handle_inspect_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialInspectHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .inspect_handler_cx_like_cpp(catalogs)
            .handle_inspect(pkt);
    })
}

fn handle_request_honor_stats_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialInspectHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .inspect_handler_cx_like_cpp(catalogs)
            .handle_request_honor_stats(pkt);
    })
}

fn handle_query_inspect_achievements_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialInspectHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .inspect_handler_cx_like_cpp(catalogs)
            .handle_query_inspect_achievements(pkt);
    })
}

/// Register the inspect-family packet entries through their Social owner.
pub fn register_social_inspect_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: SocialInspectHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::Inspect,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_inspect",
        handler: handle_inspect_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestHonorStats,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_honor_stats",
        handler: handle_request_honor_stats_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryInspectAchievements,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_inspect_achievements",
        handler: handle_query_inspect_achievements_thunk::<S, C>,
    })?;
    Ok(())
}
