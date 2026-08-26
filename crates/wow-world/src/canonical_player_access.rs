//! Read one canonical `Player` by GUID and placement, without going through the
//! owning [`WorldSession`](crate::session::WorldSession).
//!
//! Issue #252 retires the temporary `PlayerBroadcastInfo` gameplay mirror. Each
//! mirrored field existed because a remote session appeared unable to reach
//! another player's canonical `Player`. The data was never actually unreachable:
//! `WorldSession::canonical_player_snapshot_like_cpp` already resolves it out of
//! the shared canonical `MapManager`. It was only *unaddressable*, because that
//! accessor is bound to `self` and therefore answers for one GUID — the
//! session's own. The functions here take the placement explicitly, so any
//! caller holding a GUID plus the map key the session directory already stores
//! reads the canonical owner directly, and the mirrored copy can retire.
//!
//! Lock discipline: every function here acquires the canonical `MapManager`
//! mutex and releases it before returning. A caller reading the session
//! directory must copy the placement it needs out of the registry entry and drop
//! that `DashMap` guard *before* calling in. No path then holds a registry shard
//! guard and the canonical mutex at the same time, so this introduces no lock
//! nesting and no new ordering obligation.

use wow_constants::{PowerType, UnitPvpFlags};
use wow_core::ObjectGuid;
use wow_entities::Player;

use crate::session::SharedCanonicalMapManager;

/// Borrow the canonical `Player` for `guid` on one exact map instance.
///
/// Returns `None` when the mutex is poisoned, the map instance is not resident,
/// or the GUID names no player on it — the same three misses the session-bound
/// accessor already treats as "no canonical value".
pub(crate) fn with_canonical_player_at_like_cpp<R>(
    manager: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
    read: impl FnOnce(&Player) -> R,
) -> Option<R> {
    let manager = manager.lock().ok()?;
    let map = manager.find_map(map_id, instance_id)?;
    let player = map.map().get_typed_player(guid)?;
    Some(read(player))
}

/// The `Player`-owned vitals copied into party/full-state and CREATE payloads.
///
/// C++ reads these values directly from the target `Player` in
/// `PartyMemberFullState::Initialize`; keeping the result narrow prevents the
/// session directory from replacing `PlayerBroadcastInfo` with another bag of
/// gameplay state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CanonicalPlayerVitalsLikeCpp {
    pub is_alive: bool,
    pub current_health: u32,
    pub max_health: u32,
    pub power_type: u8,
    pub current_power: u16,
    pub max_power: u16,
    pub base_mana: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CanonicalPlayerPartyStateLikeCpp {
    pub vitals: CanonicalPlayerVitalsLikeCpp,
    pub is_pvp: bool,
    pub is_ffa_pvp: bool,
    pub is_ghost: bool,
    pub is_afk: bool,
    pub is_dnd: bool,
}

fn power_kind_from_u8_like_cpp(power: u8) -> PowerType {
    match power {
        1 => PowerType::Rage,
        2 => PowerType::Focus,
        3 => PowerType::Energy,
        4 => PowerType::Happiness,
        5 => PowerType::Runes,
        6 => PowerType::RunicPower,
        7 => PowerType::SoulShards,
        8 => PowerType::LunarPower,
        9 => PowerType::HolyPower,
        10 => PowerType::AlternatePower,
        11 => PowerType::Maelstrom,
        12 => PowerType::Chi,
        13 => PowerType::Insanity,
        14 => PowerType::ComboPoints,
        15 => PowerType::DemonicFury,
        16 => PowerType::ArcaneCharges,
        17 => PowerType::Fury,
        18 => PowerType::Pain,
        19 => PowerType::Essence,
        20 => PowerType::RuneBlood,
        21 => PowerType::RuneFrost,
        22 => PowerType::RuneUnholy,
        23 => PowerType::AlternateQuest,
        24 => PowerType::AlternateEncounter,
        25 => PowerType::AlternateMount,
        _ => PowerType::Mana,
    }
}

fn power_to_u16_like_cpp(value: i32) -> u16 {
    u16::try_from(value.max(0)).unwrap_or(u16::MAX)
}

/// Read the live C++ vitals tuple from one canonical `Player`.
#[must_use]
pub(crate) fn canonical_player_vitals_like_cpp(player: &Player) -> CanonicalPlayerVitalsLikeCpp {
    let power_type = player.unit().data().display_power;
    let power = power_kind_from_u8_like_cpp(power_type);
    CanonicalPlayerVitalsLikeCpp {
        is_alive: player.unit().is_alive(),
        current_health: player.unit().data().health.min(u64::from(u32::MAX)) as u32,
        max_health: player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
        power_type,
        current_power: power_to_u16_like_cpp(player.get_power(power)),
        max_power: power_to_u16_like_cpp(player.get_max_power(power)),
        base_mana: player.unit().get_create_mana_like_cpp(),
    }
}

pub(crate) fn canonical_player_party_state_like_cpp(
    player: &Player,
) -> CanonicalPlayerPartyStateLikeCpp {
    let pvp = player.unit().pvp_flags_like_cpp();
    CanonicalPlayerPartyStateLikeCpp {
        vitals: canonical_player_vitals_like_cpp(player),
        is_pvp: pvp.contains(UnitPvpFlags::PVP),
        is_ffa_pvp: pvp.contains(UnitPvpFlags::FFA_PVP),
        is_ghost: player.has_player_flag(crate::session::PLAYER_FLAGS_GHOST_LIKE_CPP),
        is_afk: player.has_player_flag(crate::session::PLAYER_FLAGS_AFK_LIKE_CPP),
        is_dnd: player.has_player_flag(crate::session::PLAYER_FLAGS_DND_LIKE_CPP),
    }
}

#[cfg(test)]
pub(crate) fn configure_canonical_player_vitals_for_test(
    manager: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    vitals: (u32, u32, PowerType, i32, i32, i32),
) -> bool {
    let (health, max_health, power, current_power, max_power, base_mana) = vitals;
    let Ok(mut manager) = manager.lock() else {
        return false;
    };
    let Some(player) = manager
        .find_map_mut(571, 0)
        .and_then(|map| map.map_mut().get_typed_player_mut(guid))
    else {
        return false;
    };
    player.unit_mut().set_max_health(u64::from(max_health));
    player.unit_mut().set_health(u64::from(health));
    player.unit_mut().set_create_mana_like_cpp(base_mana);
    player.unit_mut().set_display_power(power);
    player.set_power_index(power, Some(0));
    player.unit_mut().set_max_power(power, max_power);
    player.unit_mut().set_power(power, current_power);
    true
}

#[cfg(test)]
pub(crate) fn configure_canonical_player_party_flags_for_test(
    manager: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> bool {
    let Ok(mut manager) = manager.lock() else {
        return false;
    };
    let Some(player) = manager
        .find_map_mut(571, 0)
        .and_then(|map| map.map_mut().get_typed_player_mut(guid))
    else {
        return false;
    };
    player
        .unit_mut()
        .replace_all_pvp_flags_like_cpp(UnitPvpFlags::PVP | UnitPvpFlags::FFA_PVP);
    player.set_player_flag(crate::session::PLAYER_FLAGS_GHOST_LIKE_CPP);
    true
}

/// The seven honor counters an inspect response carries, in C++ field order.
pub type HonorStatsLikeCpp = (u32, u32, u32, u16, u16, u32, u32);

/// Read the honor block off a canonical `Player`.
///
/// This is the only reader of the honor counters. Before #252 the same values
/// were copied into the broadcast mirror at registration and refreshed again on
/// every registry sync; both copies are gone, so an inspect response can no
/// longer show a stale honor level.
pub(crate) fn canonical_player_honor_stats_like_cpp(player: &Player) -> HonorStatsLikeCpp {
    (
        // C++ reads these six values from `m_activePlayerData`, but Rust's
        // canonical `ActivePlayerDataValues` does not model the historic honor
        // counters yet. Keep the packet field order represented and leave the
        // counters zero until that L4/PvP state is ported.
        0,
        0,
        0,
        0,
        0,
        0,
        player.data().honor_level.max(0) as u32,
    )
}

/// C++ `Unit::m_unitData->Flags2` for one canonical player.
#[must_use]
pub fn canonical_player_unit_flags2_like_cpp(player: &Player) -> u32 {
    player.unit().unit_flags2_like_cpp().bits()
}

/// C++ `PLAYER_FLAGS_CONTESTED_PVP`.
///
/// Re-exported so a caller outside this crate can set up the flag it reads back
/// through [`canonical_player_is_contested_pvp_like_cpp`] without a second
/// literal drifting from the definition.
pub const PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP: u32 =
    crate::session::PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP;

/// C++ `PLAYER_FLAGS_CONTESTED_PVP` on one canonical player.
#[must_use]
pub fn canonical_player_is_contested_pvp_like_cpp(player: &Player) -> bool {
    player.has_player_flag(crate::session::PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP)
}

/// C++ `ReputationMgr` per-faction state flags for one canonical player.
#[must_use]
pub fn canonical_player_reputation_state_flags_like_cpp(player: &Player) -> Vec<(u32, u32)> {
    player
        .gameplay_state()
        .reputations
        .iter()
        .map(|record| (record.faction_id, record.flags))
        .collect()
}

/// C++ `ReputationMgr::_forcedReactions` faction keys for one canonical player.
///
/// Sorted, because the aggro scan compares this against an ordered expectation.
#[must_use]
pub fn canonical_player_forced_reputation_faction_ids_like_cpp(player: &Player) -> Vec<u32> {
    let mut faction_ids: Vec<u32> = player
        .forced_reputation_faction_ids_like_cpp()
        .iter()
        .copied()
        .collect();
    faction_ids.sort_unstable();
    faction_ids
}

/// C++ `ReputationMgr` per-faction standings for one canonical player.
///
/// Missing factions are absent from the list; C++ treats an absent faction as
/// standing 0 at the point of use, so the shape matches the no-state path.
#[must_use]
pub fn canonical_player_reputation_standings_like_cpp(player: &Player) -> Vec<(u32, i32)> {
    player
        .gameplay_state()
        .reputations
        .iter()
        .map(|record| (record.faction_id, record.standing))
        .collect()
}
