//! Player reads and writes of C++ `HandleLogoutRequestOpcode` and
//! `HandleLogoutCancelOpcode` (`MiscHandler.cpp:238-311`) on the canonical
//! `Player`: the admission inputs, `SetRooted` and `UNIT_FLAG_STUNNED`.

use wow_constants::{MovementFlag, ServerOpcodes, UnitFlags, UnitStandStateType, UnitState};
use wow_packet::ServerPacket;

/// C++ `HandleLogoutRequestOpcode`: "frozen by GM via freeze command".
pub const GM_FREEZE_SPELL_ID_LIKE_CPP: u32 = 9454;

/// The C++ `Player` predicates `HandleLogoutRequestOpcode` reads, taken in
/// one canonical-Player read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogoutAdmissionSnapshotLikeCpp {
    /// `Unit::IsInFlight` (`UNIT_STATE_IN_FLIGHT`, set by a taxi flight).
    pub in_flight: bool,
    /// `HasPlayerFlag(PLAYER_FLAGS_RESTING)`.
    pub resting: bool,
    /// `Unit::IsInCombat`.
    pub in_combat: bool,
    /// `Unit::IsFalling`: `MOVEMENTFLAG_FALLING | MOVEMENTFLAG_FALLING_FAR`.
    pub falling: bool,
    /// `Player::duel` set, or aura 9454 (GM freeze).
    pub dueling_or_frozen: bool,
    /// `Unit::CanFreeMove` (`Unit.cpp:9191`).
    pub can_free_move: bool,
    pub stand_state: UnitStandStateType,
}

/// C++ RBAC composition for permission 1 and its linked role 194: account
/// grants override the default, and every applicable account deny wins last.
pub fn effective_instant_logout_permission_like_cpp(
    security: u8,
    account_permissions: &[(u32, bool)],
) -> bool {
    let mut account_grant = false;
    let mut account_deny = false;
    for &(permission_id, granted) in account_permissions {
        if !matches!(permission_id, 1 | 194) {
            continue;
        }
        if granted {
            account_grant = true;
        } else {
            account_deny = true;
        }
    }
    !account_deny && (account_grant || security > 0)
}

impl crate::session::HubRef<'_> {
    /// `None` when the session has no canonical Player.
    pub fn logout_admission_snapshot_like_cpp(&self) -> Option<LogoutAdmissionSnapshotLikeCpp> {
        self.core.with_owned_player_like_cpp(|player| {
            let unit = player.unit();
            let in_flight = unit.has_unit_state(UnitState::IN_FLIGHT.bits())
                || player.taxi_state_like_cpp().is_in_flight_like_cpp();
            let movement_flags = unit.movement_flags_like_cpp();
            let can_free_move = !in_flight
                && !unit.has_unit_state(
                    (UnitState::CONFUSED
                        | UnitState::FLEEING
                        | UnitState::IN_FLIGHT
                        | UnitState::ROOT
                        | UnitState::STUNNED
                        | UnitState::DISTRACTED)
                        .bits(),
                )
                && unit
                    .subsystems()
                    .control
                    .owner_guid
                    .is_none_or(|owner| owner.is_empty());
            LogoutAdmissionSnapshotLikeCpp {
                in_flight,
                resting: player.has_player_flag(crate::session::PLAYER_FLAGS_RESTING_LIKE_CPP),
                in_combat: unit.subsystems().combat.has_combat(),
                falling: movement_flags
                    .intersects(MovementFlag::FALLING | MovementFlag::FALLING_FAR),
                dueling_or_frozen: player.duel_info_like_cpp().is_some()
                    || unit
                        .subsystems()
                        .auras
                        .has_aura_spell_like_cpp(GM_FREEZE_SPELL_ID_LIKE_CPP),
                can_free_move,
                stand_state: unit.stand_state_like_cpp(),
            }
        })
    }

    /// C++ `RBAC_PERM_INSTANT_LOGOUT` (permission 1), composed during account
    /// lookup. Existing isolated fixtures retain the default-security rule.
    pub fn has_instant_logout_permission_like_cpp(&self) -> bool {
        self.core
            .account_state
            .instant_logout_permission
            .unwrap_or(self.core.security > 0)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/logout_wait.rs"]
mod tests;

impl crate::session::HubMut<'_> {
    /// C++ `Unit::SetRooted(apply)` (`Unit.cpp:10893-10926`) for the Player
    /// moving itself: the movement flags, then `SMSG_MOVE_ROOT`/`UNROOT` to the
    /// player and `MoveUpdate` to the others.
    pub fn set_player_rooted_like_cpp(&mut self, apply: bool) {
        if let Some(mut flags) = self.shared().resolved_player_movement_flags_like_cpp() {
            if apply {
                flags.remove(MovementFlag::MASK_MOVING);
                flags.insert(MovementFlag::ROOT);
            } else {
                flags.remove(MovementFlag::ROOT);
            }
            self.set_player_movement_flags_like_cpp(flags);
        }
        self.send_player_move_set_flag_like_cpp(if apply {
            ServerOpcodes::MoveRoot
        } else {
            ServerOpcodes::MoveUnroot
        });
    }

    /// C++ `SetUnitFlag`/`RemoveUnitFlag(UNIT_FLAG_STUNNED)`: the canonical
    /// `UnitData::Flags` changes and its values update reaches the player and
    /// the players that see it. Returns whether the canonical Player exists.
    pub fn set_player_unit_flag_stunned_like_cpp(&mut self, apply: bool) -> bool {
        let Some(player_guid) = self.core.player_guid() else {
            return false;
        };
        let Some(flags) = self.core.with_owned_player_mut_like_cpp(|player| {
            let mut flags = player.unit().unit_flags_like_cpp();
            flags.set(UnitFlags::STUNNED, apply);
            player.unit_mut().set_unit_flags_like_cpp(flags);
            flags
        }) else {
            return false;
        };
        // `UnitData::Flags` is `UpdateField<uint32, 32, 41>`.
        let mut values = wow_packet::packets::update::UnitDataValuesDeltaUpdate::default();
        values.unit_data_mask[1] = (1 << (32 - 32)) | (1 << (41 - 32));
        values.flags = flags.bits();
        let bytes = wow_packet::packets::update::UpdateObject::unit_values_update(
            player_guid,
            self.core.player_map_id_like_cpp(),
            values,
        )
        .to_bytes();
        self.core.send_raw_packet(&bytes);
        self.shared()
            .broadcast_to_movement_set_like_cpp(bytes, false);
        true
    }
}
