use super::Player;
use crate::PlayerDamageControlStateLikeCpp;

impl Player {
    pub const fn damage_control_like_cpp(&self) -> PlayerDamageControlStateLikeCpp {
        self.gameplay_state.damage_control
    }

    pub fn set_cheat_god_like_cpp(&mut self, enabled: bool) {
        self.gameplay_state.damage_control.cheat_god = enabled;
    }

    pub fn set_normal_damage_immune_like_cpp(&mut self, immune: bool) {
        self.gameplay_state.damage_control.normal_damage_immune = immune;
    }

    pub fn set_environmental_damage_immune_like_cpp(&mut self, immune: bool) {
        self.gameplay_state
            .damage_control
            .environmental_damage_immune = immune;
    }
}

#[cfg(test)]
mod tests {
    use super::Player;

    #[test]
    fn player_owns_represented_damage_gates_like_cpp() {
        let mut player = Player::new(Some(7), false);
        player.set_cheat_god_like_cpp(true);
        player.set_normal_damage_immune_like_cpp(true);
        player.set_environmental_damage_immune_like_cpp(true);

        let state = player.damage_control_like_cpp();
        assert!(state.cheat_god);
        assert!(state.normal_damage_immune);
        assert!(state.environmental_damage_immune);
    }
}
