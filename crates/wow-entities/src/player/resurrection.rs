use super::Player;
use crate::{
    PlayerResurrectionContinuation, PlayerResurrectionOperation, PlayerResurrectionPhase,
    PlayerResurrectionRequestLikeCpp, PlayerResurrectionStateLikeCpp,
};
use wow_core::ObjectGuid;

impl Player {
    pub fn resurrection_state_like_cpp(&self) -> &PlayerResurrectionStateLikeCpp {
        &self.gameplay_state.resurrection
    }

    pub fn resurrection_state_mut_like_cpp(&mut self) -> &mut PlayerResurrectionStateLikeCpp {
        &mut self.gameplay_state.resurrection
    }

    /// C++ `Player::SetPvPDeath` (`Player.h:1147`): `PLAYER_EXTRA_PVP_DEATH`.
    pub fn set_pvp_death_like_cpp(&mut self, on: bool) {
        const PLAYER_EXTRA_PVP_DEATH: u32 = 0x0100;
        if on {
            self.extra_flags |= PLAYER_EXTRA_PVP_DEATH;
        } else {
            self.extra_flags &= !PLAYER_EXTRA_PVP_DEATH;
        }
    }

    pub fn set_resurrection_request_like_cpp(&mut self, request: PlayerResurrectionRequestLikeCpp) {
        self.gameplay_state.resurrection.request = Some(request);
    }

    pub fn clear_resurrection_request_like_cpp(&mut self) {
        self.gameplay_state.resurrection.request = None;
    }

    /// Atomically consume C++ `_resurrectionData` into the Player-owned
    /// delayed operation before its caller reaches the first async boundary.
    pub fn accept_resurrection_request(
        &mut self,
        resurrecter: ObjectGuid,
    ) -> Option<PlayerResurrectionContinuation> {
        let state = &mut self.gameplay_state.resurrection;
        if state.delayed_after_teleport.is_some()
            || !state.request.is_some_and(|request| {
                !request.resurrecter.is_empty() && request.resurrecter == resurrecter
            })
        {
            return None;
        }
        let request = state.request.take()?;
        let continuation = PlayerResurrectionContinuation {
            operation: PlayerResurrectionOperation::RequestData(request),
            phase: PlayerResurrectionPhase::TeleportInFlight,
        };
        state.delayed_after_teleport = Some(continuation);
        Some(continuation)
    }

    pub fn advance_resurrection_continuation(
        &mut self,
        expected: PlayerResurrectionPhase,
        next: PlayerResurrectionPhase,
    ) -> bool {
        let Some(continuation) = self
            .gameplay_state
            .resurrection
            .delayed_after_teleport
            .as_mut()
        else {
            return false;
        };
        if continuation.phase != expected {
            return false;
        }
        continuation.phase = next;
        true
    }

    pub fn clear_completed_resurrection_continuation(&mut self) -> bool {
        let state = &mut self.gameplay_state.resurrection;
        if !state
            .delayed_after_teleport
            .is_some_and(|continuation| {
                continuation.phase == PlayerResurrectionPhase::Complete
            })
        {
            return false;
        }
        state.delayed_after_teleport = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_core::Position;

    #[test]
    fn player_owns_resurrection_lifecycle_like_cpp() {
        let mut player = Player::new(Some(1), false);
        let resurrecter = ObjectGuid::create_player(1, 77);
        let request = PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        };

        player.set_resurrection_request_like_cpp(request);
        player
            .resurrection_state_mut_like_cpp()
            .self_res_spells
            .insert(21169);
        player
            .resurrection_state_mut_like_cpp()
            .death_timer_remaining_ms = 360_000;
        player
            .resurrection_state_mut_like_cpp()
            .area_spirit_healer_guid = ObjectGuid::create_player(1, 88);

        assert_eq!(player.resurrection_state_like_cpp().request, Some(request));
        assert!(
            player
                .resurrection_state_like_cpp()
                .self_res_spells
                .contains(&21169)
        );
        assert_eq!(
            player
                .resurrection_state_like_cpp()
                .death_timer_remaining_ms,
            360_000
        );
    }

    #[test]
    fn accepted_request_moves_atomically_into_teleport_continuation() {
        let mut player = Player::new(Some(1), false);
        let request = PlayerResurrectionRequestLikeCpp {
            resurrecter: ObjectGuid::create_player(1, 77),
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 21169,
        };
        player.set_resurrection_request_like_cpp(request);
        assert_eq!(
            player.accept_resurrection_request(ObjectGuid::create_player(1, 78)),
            None
        );
        let continuation = player
            .accept_resurrection_request(request.resurrecter)
            .expect("request accepted");
        assert_eq!(
            continuation,
            PlayerResurrectionContinuation {
                operation: PlayerResurrectionOperation::RequestData(request),
                phase: PlayerResurrectionPhase::TeleportInFlight,
            }
        );
        let state = player.resurrection_state_like_cpp();
        assert!(state.request.is_none());
        assert_eq!(state.delayed_after_teleport, Some(continuation));
        assert!(player
            .accept_resurrection_request(request.resurrecter)
            .is_none());
    }
}
