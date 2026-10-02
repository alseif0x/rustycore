// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::fixtures` (#1241 F3-0): the cfg(test) groups, moved unchanged.

use super::*;

/// Test-only fixture groups (#1241 F3-0): the 11 cfg(test) domain groups, nested unchanged so an F3
/// context borrows one member instead of eleven.
pub(crate) struct SessionFixtures {
    /// Player identity fixtures: race, class, level, gender, name, create mode, faction template,
    /// scale and zone/area state.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) identity: PlayerIdentityState,
    /// Account collections: mounts, heirlooms, toys, item appearances, transmog illusions and
    /// completed achievements.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) collections: CollectionsState,
    /// Player aura fixtures: visible auras, aura authority, the spell-hit tombstone, threat-aura
    /// snapshots and the shapeshift form.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) auras: AuraState,
    /// XP, talents, glyphs and respec, skills and proficiencies, reputation and rest fixtures.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) progression: ProgressionState,
    /// Combat target and flags, vitals and powers, GM and immunity flags, PvP flags and timers,
    /// death and resurrection.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) combat: CombatState,
    /// Player movement fixtures: position, flags, jump and fall, acks, speeds and force mods, and
    /// vehicle movement sinks.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) movement: MovementState,
    /// Near, far and delayed teleport state and acks, the pending teleport, the homebind and
    /// spline-done taxi events.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) teleport: TeleportState,
    /// Taxi flight and mount/vehicle kit state: destinations, seat state, vehicle requests,
    /// transport attach and mount counters.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) vehicles: TaxiVehicleState,
    /// Represented pet state, pet stable, react and command state, pet speeds, temporary (un)summon
    /// and mount pet-control counters.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) pets: PetState,
    /// Battleground and arena membership and the represented battlemaster, battlefield and wargame
    /// request sinks.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) battleground: BattlegroundState,
    /// Player presentation fixtures: unit flags and scale, stand state and emote, action bars,
    /// cinematics, CUF profiles and barber requests.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) presentation: PlayerPresentationState,
}
