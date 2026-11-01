// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `CMSG_INITIATE_TRADE` (#38): C++ `WorldSession::HandleInitiateTradeOpcode`
//! (`TradeHandler.cpp:582-701`) and the `PartnerIsSameBnetAccount` half of
//! `WorldSession::SendTradeStatus` (`TradeHandler.cpp:36-42`).
//!
//! Both players' `m_trade` live on their canonical Players inside the shared
//! map manager. C++ checks the target and creates both `TradeData` objects in
//! one synchronous step; Rust does the checks and both installs under one
//! acquisition of the map-manager lock, so two initiators can never open two
//! trades on the same target. The lock is released before any packet is sent.

use wow_constants::unit::{Team, UnitState};
use wow_core::ObjectGuid;
use wow_entities::{Player, WorldObject};
use wow_packet::ServerPacket;
use wow_packet::packets::misc::{
    TRADE_DISTANCE_LIKE_CPP, TRADE_STATUS_DEAD_LIKE_CPP, TRADE_STATUS_LOGGING_OUT_LIKE_CPP,
    TRADE_STATUS_NO_TARGET_LIKE_CPP, TRADE_STATUS_PLAYER_BUSY_LIKE_CPP,
    TRADE_STATUS_STUNNED_LIKE_CPP, TRADE_STATUS_TARGET_DEAD_LIKE_CPP,
    TRADE_STATUS_TARGET_STUNNED_LIKE_CPP, TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP,
    TRADE_STATUS_WRONG_FACTION_LIKE_CPP, TradeStatus,
};
use wow_world_core::session::mailbox::{SendRepresentedTradeStatusLikeCppCommand, SessionCommand};
use wow_world_core::session::state::hub_support::player_team_for_race_cpp;
use wow_world_core::session::{HubRef, SessionCore};

/// C++ `LANG_TRADE_REQ` (`Language.h:1163`).
pub const LANG_TRADE_REQ_LIKE_CPP: u32 = 6609;
/// C++ `LANG_TRADE_OTHER_REQ` (`Language.h:1164`).
pub const LANG_TRADE_OTHER_REQ_LIKE_CPP: u32 = 6610;
/// C++ `PLAYER_FLAGS_EX_MERCENARY_MODE` (`Player.h:463`).
pub const PLAYER_FLAGS_EX_MERCENARY_MODE_LIKE_CPP: u32 = 0x0002;

/// Process-owned C++ `World` trade policy: `CONFIG_TRADE_LEVEL_REQ`
/// (`LevelReq.Trade`, default 1) and `CONFIG_ALLOW_TWO_SIDE_TRADE`
/// (`AllowTwoSide.Trade`, default false), `World.cpp:791,916`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradePolicyLikeCpp {
    pub trade_level_req: u32,
    pub allow_two_side_trade: bool,
}

impl Default for TradePolicyLikeCpp {
    fn default() -> Self {
        Self {
            trade_level_req: 1,
            allow_two_side_trade: false,
        }
    }
}

/// Result of the locked C++ check sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitiateTradeOutcomeLikeCpp {
    /// `if (GetPlayer()->m_trade) return;`, or the own Player is unavailable.
    Silent,
    /// One `SendTradeStatus(info)` with this status and no trade opened.
    Refused(u8),
    /// The level gate: `SendNotification(GetTrinityString(lang), level)` then
    /// `TRADE_STATUS_FAILED`.
    LevelRequirement { lang: u32 },
    /// Both `m_trade` were created; the target receives `PROPOSED`.
    Opened,
}

struct TradeInitiateSideLikeCpp {
    has_trade: bool,
    alive: bool,
    stunned: bool,
    in_flight: bool,
    level: u32,
    team: Team,
    mercenary: bool,
    logging_out: bool,
    world: WorldObject,
}

impl TradeInitiateSideLikeCpp {
    fn of(player: &Player) -> Self {
        let unit = player.unit();
        Self {
            has_trade: player.trade_state_snapshot_like_cpp().is_some(),
            alive: unit.is_alive(),
            stunned: unit.has_unit_state(UnitState::STUNNED.bits()),
            // C++ `Unit::IsInFlight` is `UNIT_STATE_IN_FLIGHT`; the represented
            // taxi flight carries the same fact while the unit state is unset.
            in_flight: unit.has_unit_state(UnitState::IN_FLIGHT.bits())
                || player.taxi_state_like_cpp().is_in_flight_like_cpp(),
            level: u32::from(player.level_like_cpp()),
            team: player_team_for_race_cpp(player.race_like_cpp()),
            mercenary: player.has_player_flag_ex(PLAYER_FLAGS_EX_MERCENARY_MODE_LIKE_CPP),
            logging_out: player.trade_session_logging_out_like_cpp(),
            world: unit.world().clone(),
        }
    }
}

/// Inputs of `HandleInitiateTradeOpcode` that are not canonical Player state.
#[derive(Debug, Clone, Copy)]
pub struct InitiateTradeContextLikeCpp {
    pub policy: TradePolicyLikeCpp,
    /// C++ `WorldSession::isLogingOut()` of the initiator.
    pub logging_out: bool,
    /// C++ `HasPermission(rbac::RBAC_PERM_ALLOW_TWO_SIDE_TRADE)`. RustyCore has
    /// no RBAC; the party-invite gate uses the GM state as its proxy, and so
    /// does this one.
    pub two_side_permission: bool,
}

/// Run the C++ check sequence and, when it passes, install both `m_trade`
/// under one map-manager lock. C++ `pOther->GetSession()->isLogingOut()` is
/// the target Player's logout mark, read under the same lock; the leaving
/// session sets it before its own `TradeCancel`, so a trade either opens
/// before the mark (and that cancel closes it) or is refused.
pub fn initiate_trade_pair_like_cpp(
    core: &SessionCore,
    target_guid: ObjectGuid,
    context: InitiateTradeContextLikeCpp,
) -> InitiateTradeOutcomeLikeCpp {
    use InitiateTradeOutcomeLikeCpp::*;
    let (Some(manager), Some(handle)) = (
        core.canonical_map_manager.as_ref(),
        core.player_handle_like_cpp,
    ) else {
        return Silent;
    };
    let Ok(mut manager) = manager.lock() else {
        return Silent;
    };
    let Some(me) = manager.with_player_like_cpp(handle, TradeInitiateSideLikeCpp::of) else {
        return Silent;
    };
    if me.has_trade {
        return Silent;
    }
    if !me.alive {
        return Refused(TRADE_STATUS_DEAD_LIKE_CPP);
    }
    if me.stunned {
        return Refused(TRADE_STATUS_STUNNED_LIKE_CPP);
    }
    if context.logging_out {
        return Refused(TRADE_STATUS_LOGGING_OUT_LIKE_CPP);
    }
    if me.in_flight {
        return Refused(TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP);
    }
    if me.level < context.policy.trade_level_req {
        return LevelRequirement {
            lang: LANG_TRADE_REQ_LIKE_CPP,
        };
    }

    // C++ `ObjectAccessor::FindPlayer`: an in-world Player on any map.
    let mut target_key = None;
    manager.do_for_all_maps(|managed| {
        if target_key.is_none()
            && let Some(player) = managed.map().get_typed_player(target_guid)
            && player.unit().world().object().is_in_world()
        {
            target_key = Some((managed.map_id(), managed.instance_id()));
        }
    });
    let Some((target_map, target_instance)) = target_key else {
        return Refused(TRADE_STATUS_NO_TARGET_LIKE_CPP);
    };
    let Some(other) = manager
        .find_map(target_map, target_instance)
        .and_then(|managed| managed.map().get_typed_player(target_guid))
        .map(TradeInitiateSideLikeCpp::of)
    else {
        return Refused(TRADE_STATUS_NO_TARGET_LIKE_CPP);
    };

    if target_guid == handle.guid() || other.has_trade {
        return Refused(TRADE_STATUS_PLAYER_BUSY_LIKE_CPP);
    }
    if !other.alive {
        return Refused(TRADE_STATUS_TARGET_DEAD_LIKE_CPP);
    }
    if other.in_flight {
        return Refused(TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP);
    }
    if other.stunned {
        return Refused(TRADE_STATUS_TARGET_STUNNED_LIKE_CPP);
    }
    if other.logging_out {
        return Refused(wow_packet::packets::misc::TRADE_STATUS_TARGET_LOGGING_OUT_LIKE_CPP);
    }
    if (other.team != me.team || other.mercenary || me.mercenary)
        && !context.policy.allow_two_side_trade
        && !context.two_side_permission
    {
        return Refused(TRADE_STATUS_WRONG_FACTION_LIKE_CPP);
    }
    if !other
        .world
        .is_within_dist_in_map(&me.world, TRADE_DISTANCE_LIKE_CPP, false)
    {
        return Refused(TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP);
    }
    if other.level < context.policy.trade_level_req {
        return LevelRequirement {
            lang: LANG_TRADE_OTHER_REQ_LIKE_CPP,
        };
    }

    // C++ `_player->m_trade = new TradeData(_player, pOther);
    //      pOther->m_trade = new TradeData(pOther, _player);`
    let own = manager.with_player_mut_like_cpp(handle, |player| {
        player.open_trade_like_cpp(target_guid);
    });
    if own.is_none() {
        return Silent;
    }
    let installed = manager
        .find_map_mut(target_map, target_instance)
        .and_then(|managed| managed.map_mut().get_typed_player_mut(target_guid))
        .map(|player| player.open_trade_like_cpp(handle.guid()))
        .is_some();
    if !installed {
        let _ = manager.with_player_mut_like_cpp(handle, Player::clear_trade_like_cpp);
        return Refused(TRADE_STATUS_NO_TARGET_LIKE_CPP);
    }
    Opened
}

/// C++ `SendTradeStatus`: `PartnerIsSameBnetAccount = trader &&
/// trader->GetSession()->GetBattlenetAccountId() == GetBattlenetAccountId()`.
/// The relation is symmetric, so the bytes a session builds for its partner
/// carry the same value the partner's own `SendTradeStatus` would compute.
pub fn partner_is_same_bnet_account_like_cpp(hub: HubRef<'_>, partner: Option<ObjectGuid>) -> bool {
    let Some(partner) = partner else {
        return false;
    };
    hub.core
        .player_registry()
        .and_then(|registry| registry.player_name_query_snapshot_like_cpp(partner))
        .is_some_and(|snapshot| snapshot.battlenet_account_id == hub.core.battlenet_account_id())
}

/// Deliver an already-serialized trade status to the partner's own session.
pub fn send_trade_status_to_partner_like_cpp(
    hub: HubRef<'_>,
    partner: ObjectGuid,
    status: TradeStatus,
) {
    hub.core.try_send_connected_player_command_like_cpp(
        partner,
        SessionCommand::SendRepresentedTradeStatusLikeCpp(
            SendRepresentedTradeStatusLikeCppCommand {
                packet_bytes: status.to_bytes(),
            },
        ),
    );
}

/// C++ `SendNotification(char const* format, ...)` for the `%u` level gate.
pub fn trade_level_notification_like_cpp(format: &str, level: u32) -> String {
    let mut output = String::with_capacity(format.len() + 4);
    let mut chars = format.chars();
    let mut consumed = false;
    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }
        match chars.next() {
            Some('%') => output.push('%'),
            Some('u' | 'd' | 'i') if !consumed => {
                output.push_str(&level.to_string());
                consumed = true;
            }
            Some(other) => {
                output.push('%');
                output.push(other);
            }
            None => output.push('%'),
        }
    }
    output
}
