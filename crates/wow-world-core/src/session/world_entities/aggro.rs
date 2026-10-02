use crate::session::state::SessionCore;

const DEFAULT_PLAYER_COMBAT_REACH_LIKE_CPP: f32 = 1.5;

impl SessionCore {
    pub fn canonical_player_combat_reach_snapshot_like_cpp(&self) -> f32 {
        self.canonical_player_snapshot_like_cpp(|player| player.unit().data().combat_reach)
            .unwrap_or(0.0)
    }

    pub fn player_interaction_combat_reach_like_cpp(&self) -> f32 {
        let canonical_reach = self.canonical_player_combat_reach_snapshot_like_cpp();
        if canonical_reach > 0.0 {
            canonical_reach
        } else {
            DEFAULT_PLAYER_COMBAT_REACH_LIKE_CPP
        }
    }
}
