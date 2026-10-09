// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battleground queue and PvP-flag handlers.
//!
//! C++ source of truth: `src/server/game/Handlers/BattleGroundHandler.cpp` and
//! `MiscHandler.cpp` for the PvP flag opcodes. The represented queue state, the
//! PvP flag transitions and packet publication live in the Core hub, so the
//! World session only builds the borrowed hub context and runs the deferred
//! registry-state publication through a bounded seam (#1263 F5). The area
//! spirit-healer query/queue handlers (`BattleGroundHandler.cpp`) also live
//! here and borrow the world-entity and instance state, as do the battlefield
//! status, battlemaster hello, battlefield list and battlemaster join handlers,
//! which record their represented intents through the hub. The rated arena join
//! and its single-user arena chain now live here too. `#1263 F5 remaining
//! families` moved the skirmish join and the wargame accept registrations here
//! as well (`Opcodes.cpp:220` `CMSG_BATTLEMASTER_JOIN_SKIRMISH` and
//! `Opcodes.cpp:144` `CMSG_ACCEPT_WARGAME_INVITE`, both
//! `STATUS_UNHANDLED`/`Handle_NULL` in C++ and represented in the shell); their
//! bodies stay in the World shell while they need its group/registry
//! orchestration, and the host lends them. Hearth-and-resurrect also stays in
//! the World shell.

use tracing::debug;
use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::gossip::Hello;
use wow_packet::packets::misc::{
    AreaSpiritHealerQuery, AreaSpiritHealerQueue, AreaSpiritHealerTime, BattlefieldLeave,
    BattlefieldListRequest, BattlefieldPort, BattlemasterJoin, BattlemasterJoinArena, RatedPvpInfo,
    RequestBattlefieldStatus, SetPvp, TogglePvp,
};
use wow_packet::{ClientPacket, WorldPacket};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::RepresentedBattlemasterJoinArenaLikeCpp;
use wow_world_core::session::{
    HubMut, PLAYER_FLAGS_IN_PVP_LIKE_CPP, PacketPublicationAccessLikeCpp,
    RepresentedBattlegroundQueueTypeIdLikeCpp,
};
use wow_world_entities::WorldEntitiesState;
use wow_world_instances::InstanceState;
use wow_world_social::SessionSocialLimits;

/// C++ `PLAYER_FLAGS_PVP_TIMER` (private in `wow-entities`).
const PLAYER_FLAGS_PVP_TIMER_LIKE_CPP: u32 = 0x0004_0000;

/// Borrowed inputs of one battleground/PvP handler invocation.
pub struct BattlegroundHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    world_entities: &'a WorldEntitiesState,
    instances: &'a mut InstanceState,
    social: &'a SessionSocialLimits,
    /// The host's World-test flag (World passes `cfg!(test)`); the represented
    /// arena-join records are World-test evidence only.
    world_test_consumer: bool,
}

/// C++ arena-team slot → team size used by the rated arena join
/// (`ArenaTeamMgr` resolves the slot through the all-arenas template).
fn arena_team_type_by_slot_like_cpp(slot: u8) -> Option<u8> {
    match slot {
        0 => Some(2),
        1 => Some(3),
        2 => Some(5),
        _ => None,
    }
}

impl<'a> BattlegroundHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        world_entities: &'a WorldEntitiesState,
        instances: &'a mut InstanceState,
        social: &'a SessionSocialLimits,
        world_test_consumer: bool,
    ) -> Self {
        Self {
            hub,
            world_entities,
            instances,
            social,
            world_test_consumer,
        }
    }

    /// CMSG_REQUEST_BATTLEFIELD_STATUS — client asks for its queue-slot statuses.
    pub async fn handle_request_battlefield_status(&mut self, mut pkt: WorldPacket) {
        if let Err(error) = RequestBattlefieldStatus::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "RequestBattlefieldStatus parse failed: {error}"
            );
            return;
        }

        // C++ iterates PLAYER_MAX_BATTLEGROUND_QUEUES and sends active,
        // confirmation, or queued status only for non-empty queue slots.
        // Rust has no represented battleground queue state in this handler yet,
        // so the no-queue branch is silent.
    }
    /// CMSG_BATTLEMASTER_HELLO — player asks a battlemaster NPC for its queue list.
    /// C++ ref: `WorldSession::HandleBattlemasterHelloOpcode`.
    pub async fn handle_battlemaster_hello(&mut self, mut pkt: WorldPacket) {
        let hello = match Hello::read(&mut pkt) {
            Ok(hello) => hello,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "BattlemasterHello parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when the target cannot be interacted with as a
        // battlemaster. The accepted branch records the list intent until
        // BattlegroundMgr::SendBattlegroundList is live in Rust.
        let _accepted = self.hub.battlemaster_hello_like_cpp(hello.unit);
    }
    /// CMSG_BATTLEFIELD_LIST — player asks for the queue list of a battleground type.
    /// C++ ref: `WorldSession::HandleBattlefieldListOpcode`.
    pub async fn handle_battlefield_list(
        &mut self,
        battlemaster_lists: &wow_data::BattlemasterListStore,
        mut pkt: WorldPacket,
    ) {
        let request = match BattlefieldListRequest::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "BattlefieldList parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sBattlemasterListStore has no ListID row.
        // The accepted branch records the SendBattlegroundList intent until
        // BattlegroundMgr owns live queue/list packets in Rust.
        let _accepted = self
            .hub
            .battlefield_list_like_cpp(battlemaster_lists, request.list_id);
    }
    /// CMSG_BATTLEMASTER_JOIN — player asks to join a battleground queue.
    /// C++ ref: `WorldSession::HandleBattlemasterJoinOpcode`.
    pub async fn handle_battlemaster_join(
        &mut self,
        battlemaster_lists: &wow_data::BattlemasterListStore,
        mut pkt: WorldPacket,
    ) {
        let join = match BattlemasterJoin::read(&mut pkt) {
            Ok(join) => join,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "BattlemasterJoin parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently for missing/invalid queues and early queue gates.
        // The accepted branch records the queue intent until BattlegroundQueue
        // and BattlegroundMgr queue-status packets are live in Rust.
        let _accepted = self.hub.battlemaster_join_like_cpp(
            battlemaster_lists,
            &join.queue_ids,
            join.roles,
            join.blacklist_map,
        );
    }

    /// CMSG_BATTLEMASTER_JOIN_ARENA — player asks to join a rated arena queue.
    /// C++ ref: `WorldSession::HandleBattlemasterJoinArena`.

    pub async fn handle_battlemaster_join_arena(
        &mut self,
        battlemaster_lists: &wow_data::BattlemasterListStore,
        mut pkt: WorldPacket,
    ) {
        let join = match BattlemasterJoinArena::read(&mut pkt) {
            Ok(join) => join,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "BattlemasterJoinArena parse failed: {error}"
                );
                return;
            }
        };

        // C++ gates on already-in-BG, the all-arenas template, disabled arena,
        // group and leader before entering ArenaTeamMgr/queue code. Rust records
        // the bounded queue intent after those representable gates until the
        // live rated-arena manager is ported.
        let _accepted = self.battlemaster_join_arena_like_cpp(
            battlemaster_lists,
            join.team_size_index,
            join.roles,
        );
    }

    /// CMSG_AREA_SPIRIT_HEALER_QUERY — ask an area spirit healer for resurrection timer.
    /// C++ ref: `WorldSession::HandleAreaSpiritHealerQueryOpcode`.
    pub async fn handle_area_spirit_healer_query(&mut self, mut pkt: WorldPacket) {
        let query = match AreaSpiritHealerQuery::read(&mut pkt) {
            Ok(query) => query,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "AreaSpiritHealerQuery parse failed: {error}"
                );
                return;
            }
        };

        let Some(access) = self
            .world_entities
            .represented_area_spirit_healer_access_like_cpp(self.hub.shared(), query.healer_guid)
        else {
            debug!(
                account = self.hub.shared().core.account_id,
                healer = ?query.healer_guid,
                "AreaSpiritHealerQuery ignored without represented area spirit healer"
            );
            return;
        };

        // C++ sends the current shared channel timer or the individual aura
        // duration after casting SPELL_SPIRIT_HEAL_PLAYER_AURA. Spell/aura/channel
        // runtime is still outside this represented handler, so the packet shape
        // and validation are ported and the timer remains zero for now.
        if (access.npc_flags2
            & wow_constants::unit::NPCFlags2::AREA_SPIRIT_HEALER_INDIVIDUAL.bits())
            != 0
        {
            debug!(
                account = self.hub.shared().core.account_id,
                healer = ?query.healer_guid,
                "AreaSpiritHealerQuery individual aura/channel timer is not represented yet"
            );
        }

        self.publication_like_cpp()
            .send_packet(&AreaSpiritHealerTime {
                healer_guid: query.healer_guid,
                time_left_ms: 0,
            });
    }

    /// CMSG_AREA_SPIRIT_HEALER_QUEUE — select an area spirit healer for resurrection.
    /// C++ ref: `WorldSession::HandleAreaSpiritHealerQueueOpcode`.
    pub async fn handle_area_spirit_healer_queue(&mut self, mut pkt: WorldPacket) {
        let queue = match AreaSpiritHealerQueue::read(&mut pkt) {
            Ok(queue) => queue,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "AreaSpiritHealerQueue parse failed: {error}"
                );
                return;
            }
        };

        if self
            .world_entities
            .represented_area_spirit_healer_access_like_cpp(self.hub.shared(), queue.healer_guid)
            .is_none()
        {
            debug!(
                account = self.hub.shared().core.account_id,
                healer = ?queue.healer_guid,
                "AreaSpiritHealerQueue ignored without represented area spirit healer"
            );
            return;
        }

        // C++ also casts SPELL_WAITING_FOR_RESURRECT; deferred until the
        // player spell/aura runtime owns battleground spirit resurrection.
        self.instances
            .set_area_spirit_healer_guid_like_cpp(&mut self.hub, queue.healer_guid);
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    /// CMSG_BATTLEFIELD_PORT — player accepts an invite or leaves a queue slot.
    pub async fn handle_battlefield_port(&mut self, mut pkt: WorldPacket) {
        let port = match BattlefieldPort::read(&mut pkt) {
            Ok(port) => port,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "BattlefieldPort parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently for not-in-queue, invalid queue slot, and
        // AcceptedInvite without an invitation. The accepted/leave branch is
        // represented only until live BattlegroundQueue/BattlegroundMgr exists.
        let _accepted = self
            .hub
            .battlefield_port_like_cpp(port.ticket, port.accepted_invite);
    }

    /// CMSG_REQUEST_RATED_PVP_INFO — always answers the empty C++ payload.
    pub async fn handle_request_rated_pvp_info(&mut self, _pkt: WorldPacket) {
        self.publication_like_cpp()
            .send_packet_realm(&RatedPvpInfo::default());
    }

    /// CMSG_REQUEST_PVP_REWARDS — C++ dispatches to `Player::SendPvpRewards()`,
    /// whose response send is commented out in the canonical source, so the
    /// observable behavior is silence.
    pub async fn handle_request_pvp_rewards(&mut self, _pkt: WorldPacket) {}

    /// CMSG_TOGGLE_PVP — flips the player's PvP flag.
    ///
    /// Returns whether the registry state must be re-published by the host.
    pub async fn handle_toggle_pvp(&mut self, mut pkt: WorldPacket) -> bool {
        if let Err(error) = TogglePvp::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "TogglePvP parse failed: {error}"
            );
            return false;
        }

        self.apply_toggle_pvp_like_cpp()
    }

    /// CMSG_SET_PVP — sets the player's PvP flag explicitly.
    ///
    /// Returns whether the registry state must be re-published by the host.
    pub async fn handle_set_pvp(&mut self, mut pkt: WorldPacket) -> bool {
        let packet = match SetPvp::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "SetPvP parse failed: {error}"
                );
                return false;
            }
        };

        self.apply_set_pvp_like_cpp(packet.enable_pvp)
    }

    /// C++ `WorldSession::HandleTogglePvP`: flips the current represented flag.
    pub fn apply_toggle_pvp_like_cpp(&mut self) -> bool {
        let Some(guid) = self.hub.shared().core.player_guid() else {
            return false;
        };
        let Some(in_pvp) = self.hub.shared().player_has_in_pvp_flag_like_cpp(guid) else {
            return false;
        };
        self.apply_set_pvp_like_cpp(!in_pvp)
    }

    /// C++ `Player::SetPvP` with the represented timer/war-mode gates.
    pub fn apply_set_pvp_like_cpp(&mut self, enable_pvp: bool) -> bool {
        let Some(guid) = self.hub.shared().core.player_guid() else {
            return false;
        };

        if enable_pvp {
            #[cfg_attr(not(test), allow(unused_mut))]
            let mut mutated = self.hub.core.with_owned_player_mut_like_cpp(|player| {
                player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                player.remove_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
            });
            #[cfg(any(test, feature = "test-fixtures"))]
            if mutated.is_none() && self.hub.core.player_handle_like_cpp.is_none() {
                mutated = self
                    .hub
                    .core
                    .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                        player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                        player.remove_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
                    });
                self.hub.fixtures.combat.player_in_pvp_flag_like_cpp = true;
            }
            let _ = mutated;

            if self.hub.shared().player_is_pvp_like_cpp(guid) == Some(false)
                || self
                    .hub
                    .shared()
                    .player_world_local_state_like_cpp()
                    .is_some_and(|state| state.pvp_end_timer_like_cpp().is_some())
            {
                self.hub.update_player_pvp_like_cpp(true, true);
            }
        } else if !self.hub.shared().player_war_mode_local_active_like_cpp() {
            #[cfg_attr(not(test), allow(unused_mut))]
            let mut mutated = self.hub.core.with_owned_player_mut_like_cpp(|player| {
                player.remove_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                player.set_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
            });
            #[cfg(any(test, feature = "test-fixtures"))]
            if mutated.is_none() && self.hub.core.player_handle_like_cpp.is_none() {
                mutated = self
                    .hub
                    .core
                    .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                        player.remove_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                        player.set_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
                    });
                self.hub.fixtures.combat.player_in_pvp_flag_like_cpp = false;
            }
            let _ = mutated;

            let Some(state) = self.hub.shared().player_world_local_state_like_cpp() else {
                return true;
            };
            if !state.is_pvp_hostile_like_cpp()
                && self.hub.shared().player_is_pvp_like_cpp(guid) == Some(true)
            {
                let now = wow_entities::game_time_secs_like_cpp();
                let _ = self.hub.set_player_pvp_end_timer_like_cpp(Some(now));
            }
        }

        true
    }

    /// CMSG_BATTLEFIELD_LEAVE — player asks to leave the current battleground.
    pub async fn handle_battlefield_leave(&mut self, mut pkt: WorldPacket) {
        if let Err(error) = BattlefieldLeave::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "BattlefieldLeave parse failed: {error}"
            );
            return;
        }

        if self.hub.shared().resolved_in_combat_like_cpp() != Some(false)
            && self
                .hub
                .shared()
                .player_in_represented_battleground_like_cpp()
            && !self
                .hub
                .shared()
                .represented_battleground_status_is_wait_leave_like_cpp()
        {
            return;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.hub
                .fixtures
                .battleground
                .represented_battleground_leave_requests_like_cpp = self
                .hub
                .fixtures
                .battleground
                .represented_battleground_leave_requests_like_cpp
                .saturating_add(1);
        }
    }

    /// C++ `WorldSession::HandleBattlemasterJoinArena` gates up to the group
    /// leader check; the accepted intent is recorded while the rated-arena
    /// team/queue manager is still represented rather than live.
    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    fn battlemaster_join_arena_like_cpp(
        &mut self,
        battlemaster_lists: &wow_data::BattlemasterListStore,
        team_size_index: u8,
        roles: u8,
    ) -> bool {
        if self
            .hub
            .shared()
            .player_in_represented_battleground_like_cpp()
        {
            return false;
        }

        let Some(arena_type) = arena_team_type_by_slot_like_cpp(team_size_index) else {
            return false;
        };
        let queue_type_id = RepresentedBattlegroundQueueTypeIdLikeCpp {
            battlemaster_list_id: wow_data::BATTLEGROUND_AA_LIKE_CPP as u16,
            queue_type: 1,
            rated: true,
            team_size: arena_type,
        };
        if !self
            .hub
            .shared()
            .is_valid_battleground_queue_type_id_like_cpp(battlemaster_lists, queue_type_id)
        {
            return false;
        }
        if self
            .hub
            .catalogs
            .disable_mgr
            .as_ref()
            .map(|disable_mgr| {
                disable_mgr.is_disabled_for_like_cpp(
                    wow_data::DISABLE_TYPE_BATTLEGROUND,
                    wow_data::BATTLEGROUND_AA_LIKE_CPP,
                    None,
                    0,
                    None,
                )
            })
            .unwrap_or(false)
        {
            return false;
        }

        let (Some(player_guid), Some(group_guid), Some(group_registry)) = (
            self.hub.shared().core.player_guid(),
            crate::resolved_group_guid_like_cpp(
                self.hub.shared(),
                self.social,
                self.world_test_consumer,
            ),
            self.hub.core.directory.group_registry.as_ref(),
        ) else {
            return false;
        };
        let is_group_leader = group_registry
            .get(&group_guid)
            .map(|group| {
                group.members.contains(&player_guid) && group.is_leader_like_cpp(player_guid)
            })
            .unwrap_or(false);
        if !is_group_leader {
            return false;
        }

        // C++ continues with Player::GetArenaTeamId, ArenaTeamMgr::GetArenaTeamById,
        // Group::CanJoinBattlegroundQueue, AddGroup and status packets. Rust
        // does not have the live rated-arena team/queue manager in this seam yet,
        // so the bounded port records the accepted intent after the representable
        // gates above without pretending that the queue was live.
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer {
            self.hub
                .fixtures
                .battleground
                .represented_battlemaster_join_arenas_like_cpp
                .push(RepresentedBattlemasterJoinArenaLikeCpp {
                    team_size_index,
                    roles,
                    arena_type,
                    group_guid,
                    queue_type_id,
                });
        }
        true
    }
}

/// Builds a battleground handler context from a host's hub.
pub trait BattlegroundHandlerHostLikeCpp<C> {
    fn battleground_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> BattlegroundHandlerCxLikeCpp<'a>;

    /// Re-publishes the registry state after a PvP flag change; the World
    /// session still owns the registry-sync providers.
    fn sync_player_registry_state_after_pvp_change_like_cpp(&mut self);

    /// Selects the `BattlemasterList.db2` catalog from the host's request
    /// catalogs (C++ `sBattlemasterListStore`).
    fn battlemaster_lists_like_cpp(catalogs: &C) -> &wow_data::BattlemasterListStore;

    /// C++ `Opcodes.cpp:220`: `CMSG_BATTLEMASTER_JOIN_SKIRMISH` is
    /// `STATUS_UNHANDLED`/`Handle_NULL`; the Rust port represents the
    /// skirmish-join body in the World shell.
    ///
    /// `#1263 F5 remaining families`: the legacy registration closure
    /// destructured the session catalog view for `BattlemasterList.db2`, so the
    /// host receives that view here.
    fn handle_battlemaster_join_skirmish_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `Opcodes.cpp:144`: `CMSG_ACCEPT_WARGAME_INVITE` is
    /// `STATUS_UNHANDLED`/`Handle_NULL`; the Rust port represents the wargame
    /// accept body in the World shell.
    ///
    /// The legacy registration closure did not read the catalog view, so this
    /// entry point does not carry it.
    fn handle_accept_wargame_invite<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()>;
}

fn handle_request_battlefield_status_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_request_battlefield_status(pkt)
            .await;
    })
}

fn handle_battlemaster_hello_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_battlemaster_hello(pkt)
            .await;
    })
}

fn handle_battlefield_list_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_battlefield_list(S::battlemaster_lists_like_cpp(catalogs), pkt)
            .await;
    })
}

fn handle_battlemaster_join_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_battlemaster_join(S::battlemaster_lists_like_cpp(catalogs), pkt)
            .await;
    })
}

fn handle_battlemaster_join_arena_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_battlemaster_join_arena(S::battlemaster_lists_like_cpp(catalogs), pkt)
            .await;
    })
}

fn handle_area_spirit_healer_query_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_area_spirit_healer_query(pkt)
            .await;
    })
}

fn handle_area_spirit_healer_queue_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_area_spirit_healer_queue(pkt)
            .await;
    })
}

fn handle_battlefield_port_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_battlefield_port(pkt)
            .await;
    })
}

fn handle_request_rated_pvp_info_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_request_rated_pvp_info(pkt)
            .await;
    })
}

fn handle_request_pvp_rewards_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_request_pvp_rewards(pkt)
            .await;
    })
}

fn handle_toggle_pvp_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let changed = session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_toggle_pvp(pkt)
            .await;
        if changed {
            session.sync_player_registry_state_after_pvp_change_like_cpp();
        }
    })
}

fn handle_set_pvp_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let changed = session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_set_pvp(pkt)
            .await;
        if changed {
            session.sync_player_registry_state_after_pvp_change_like_cpp();
        }
    })
}

fn handle_battlefield_leave_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battleground_handler_cx_like_cpp(catalogs)
            .handle_battlefield_leave(pkt)
            .await;
    })
}

/// `#1263 F5 remaining families`: the two PvP entries that lived in the World
/// shell's `handlers/battlegrounds/pvp.rs`.
fn handle_battlemaster_join_skirmish_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .handle_battlemaster_join_skirmish_with_catalogs_like_cpp(catalogs, pkt)
            .await
    })
}

fn handle_accept_wargame_invite_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move { session.handle_accept_wargame_invite(pkt).await })
}

/// Registers the battleground and PvP-flag handlers on the packet registry.
pub fn register_battleground_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: BattlegroundHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlefieldPort,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlefield_port",
        handler: handle_battlefield_port_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestRatedPvpInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_rated_pvp_info",
        handler: handle_request_rated_pvp_info_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestPvpRewards,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_pvp_rewards",
        handler: handle_request_pvp_rewards_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::TogglePvp,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_toggle_pvp",
        handler: handle_toggle_pvp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetPvp,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_pvp",
        handler: handle_set_pvp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlefieldLeave,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlefield_leave",
        handler: handle_battlefield_leave_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AreaSpiritHealerQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_area_spirit_healer_query",
        handler: handle_area_spirit_healer_query_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AreaSpiritHealerQueue,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_area_spirit_healer_queue",
        handler: handle_area_spirit_healer_queue_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestBattlefieldStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_battlefield_status",
        handler: handle_request_battlefield_status_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterHello,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_hello",
        handler: handle_battlemaster_hello_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlefieldList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlefield_list",
        handler: handle_battlefield_list_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterJoin,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_join",
        handler: handle_battlemaster_join_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterJoinArena,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_join_arena",
        handler: handle_battlemaster_join_arena_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterJoinSkirmish,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_join_skirmish",
        handler: handle_battlemaster_join_skirmish_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AcceptWargameInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_accept_wargame_invite",
        handler: handle_accept_wargame_invite_thunk::<S, C>,
    })?;
    Ok(())
}
