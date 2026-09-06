use super::Player;
use crate::{PlayerResurrectionRequestLikeCpp, PlayerResurrectionStateLikeCpp};
use wow_core::ObjectGuid;

impl Player {
    pub fn resurrection_state_like_cpp(&self) -> &PlayerResurrectionStateLikeCpp {
        &self.gameplay_state.resurrection
    }

    pub fn resurrection_state_mut_like_cpp(&mut self) -> &mut PlayerResurrectionStateLikeCpp {
        &mut self.gameplay_state.resurrection
    }

    pub fn set_resurrection_request_like_cpp(&mut self, request: PlayerResurrectionRequestLikeCpp) {
        self.gameplay_state.resurrection.request = Some(request);
    }

    pub fn clear_resurrection_request_like_cpp(&mut self) {
        self.gameplay_state.resurrection.request = None;
    }

    pub fn take_resurrection_request_if_requested_by_like_cpp(
        &mut self,
        resurrecter: ObjectGuid,
    ) -> Option<PlayerResurrectionRequestLikeCpp> {
        if !self
            .gameplay_state
            .resurrection
            .request
            .is_some_and(|request| {
                !request.resurrecter.is_empty() && request.resurrecter == resurrecter
            })
        {
            return None;
        }
        self.gameplay_state.resurrection.request.take()
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
            .delayed_after_teleport = Some(request);
        player.resurrection_state_mut_like_cpp().death_timer_active = true;
        player
            .resurrection_state_mut_like_cpp()
            .area_spirit_healer_guid = ObjectGuid::create_player(1, 88);

        assert_eq!(
            player.take_resurrection_request_if_requested_by_like_cpp(ObjectGuid::create_player(
                1, 78
            )),
            None
        );
        assert_eq!(
            player.take_resurrection_request_if_requested_by_like_cpp(resurrecter),
            Some(request)
        );
        assert!(player.resurrection_state_like_cpp().request.is_none());
        assert_eq!(
            player.resurrection_state_like_cpp().delayed_after_teleport,
            Some(request)
        );
        assert!(
            player
                .resurrection_state_like_cpp()
                .self_res_spells
                .contains(&21169)
        );
        assert!(player.resurrection_state_like_cpp().death_timer_active);
    }
}
