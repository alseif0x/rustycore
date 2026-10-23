// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Both canonical Players of one open trade under one map-manager lock (#38).
//!
//! C++ `TradeData` reads and writes `_trader->m_trade` synchronously
//! (`TradeData::SetItem` resets the trader's acceptance, `HandleAcceptTradeOpcode`
//! reads the trader's state index and money). Both `m_trade` live on canonical
//! Players inside the shared map manager, so the session reaches the partner's
//! the same way: one lock acquisition, own Player through the owned handle,
//! the partner by GUID on any map. No packet is sent while it is held.

use wow_core::ObjectGuid;
use wow_entities::Player;

use crate::session::state::SessionCore;

/// Which `TradeData` of the pair an operation addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeSideLikeCpp {
    Own,
    Partner,
}

impl TradeSideLikeCpp {
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::Own => Self::Partner,
            Self::Partner => Self::Own,
        }
    }
}

/// Borrowed map manager for one locked trade pair.
pub struct TradePairAccessLikeCpp<'a> {
    manager: &'a mut wow_map::MapManager,
    own: wow_map::PlayerHandle,
    partner_guid: ObjectGuid,
    partner_key: Option<(u32, u32)>,
}

impl TradePairAccessLikeCpp<'_> {
    #[must_use]
    pub fn own_guid(&self) -> ObjectGuid {
        self.own.guid()
    }

    #[must_use]
    pub fn partner_guid(&self) -> ObjectGuid {
        self.partner_guid
    }

    /// Whether `ObjectAccessor`-style lookup found the partner in world.
    #[must_use]
    pub fn partner_present(&self) -> bool {
        self.partner_key.is_some()
    }

    pub fn own<R>(&mut self, f: impl FnOnce(&mut Player) -> R) -> Option<R> {
        self.manager.with_player_mut_like_cpp(self.own, f)
    }

    pub fn partner<R>(&mut self, f: impl FnOnce(&mut Player) -> R) -> Option<R> {
        let (map_id, instance_id) = self.partner_key?;
        let player = self
            .manager
            .find_map_mut(map_id, instance_id)?
            .map_mut()
            .get_typed_player_mut(self.partner_guid)?;
        Some(f(player))
    }

    pub fn side<R>(
        &mut self,
        side: TradeSideLikeCpp,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        match side {
            TradeSideLikeCpp::Own => self.own(f),
            TradeSideLikeCpp::Partner => self.partner(f),
        }
    }

    /// Read both Players at once (C++ distance checks need both positions).
    pub fn read_both<R>(&self, f: impl FnOnce(&Player, &Player) -> R) -> Option<R> {
        let (map_id, instance_id) = self.partner_key?;
        let partner = self
            .manager
            .find_map(map_id, instance_id)?
            .map()
            .get_typed_player(self.partner_guid)?;
        self.manager
            .with_player_like_cpp(self.own, |own| f(own, partner))
    }
}

fn lock_trade_pair_like_cpp<R>(
    manager: &crate::session::SharedCanonicalMapManager,
    own: wow_map::PlayerHandle,
    partner_guid: Option<ObjectGuid>,
    f: impl FnOnce(&mut TradePairAccessLikeCpp<'_>) -> R,
) -> Option<R> {
    let mut manager = manager.lock().ok()?;
    let partner_guid = match partner_guid {
        Some(guid) => guid,
        None => {
            manager
                .with_player_like_cpp(own, |player| player.trade_state_snapshot_like_cpp())??
                .partner_guid
        }
    };
    let mut partner_key = None;
    manager.do_for_all_maps(|managed| {
        if partner_key.is_none()
            && managed
                .map()
                .get_typed_player(partner_guid)
                .is_some_and(|player| player.unit().world().object().is_in_world())
        {
            partner_key = Some((managed.map_id(), managed.instance_id()));
        }
    });
    let mut access = TradePairAccessLikeCpp {
        manager: &mut manager,
        own,
        partner_guid,
        partner_key,
    };
    Some(f(&mut access))
}

/// An owned address of this session's canonical Player for work that
/// outlives the session borrow: the trade commit worker changes both
/// Players after its COMMIT even when the accepting handler was dropped.
#[derive(Clone)]
pub struct TradeCommitTargetLikeCpp {
    manager: crate::session::SharedCanonicalMapManager,
    own: wow_map::PlayerHandle,
}

impl TradeCommitTargetLikeCpp {
    /// Both Players under one lock, the partner named explicitly (its
    /// `m_trade` may already be gone).
    pub fn with_pair_like_cpp<R>(
        &self,
        partner_guid: ObjectGuid,
        f: impl FnOnce(&mut TradePairAccessLikeCpp<'_>) -> R,
    ) -> Option<R> {
        lock_trade_pair_like_cpp(&self.manager, self.own, Some(partner_guid), f)
    }
}

impl SessionCore {
    /// Mirror C++ `WorldSession::isLogingOut` onto this session's Player for
    /// trade admission (`HandleInitiateTradeOpcode` reads it through
    /// `pOther->GetSession()`): set when the logout countdown starts and when
    /// `LogoutPlayer` begins, cleared by `HandleLogoutCancelOpcode`.
    pub fn mark_player_logging_out_like_cpp(&self, logging_out: bool) {
        let _ = self.with_owned_player_mut_like_cpp(|player| {
            player.set_trade_session_logging_out_like_cpp(logging_out)
        });
    }

    /// Run `f` with both Players of this session's open trade under one lock.
    /// `None` when the session has no canonical Player or no open trade; the
    /// partner may be absent (`partner_present() == false`).
    pub fn with_trade_pair_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut TradePairAccessLikeCpp<'_>) -> R,
    ) -> Option<R> {
        let own = self.player_handle_like_cpp?;
        lock_trade_pair_like_cpp(self.canonical_map_manager.as_ref()?, own, None, f)
    }

    #[must_use]
    pub fn trade_commit_target_like_cpp(&self) -> Option<TradeCommitTargetLikeCpp> {
        Some(TradeCommitTargetLikeCpp {
            manager: std::sync::Arc::clone(self.canonical_map_manager.as_ref()?),
            own: self.player_handle_like_cpp?,
        })
    }
}
