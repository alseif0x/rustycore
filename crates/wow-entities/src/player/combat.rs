use super::Player;

impl Player {
    /// C++ `Player::SetAttackSwingError` (`Player.cpp:20625-20631`).
    ///
    /// The packet belongs to the Session connection, so the operation returns
    /// whether the caller must publish the supplied error. The previous error
    /// remains canonical Player state and is updated even when no packet is
    /// needed, preserving C++'s duplicate suppression and clear behavior.
    pub fn set_attack_swing_error_like_cpp(&mut self, error: Option<u8>) -> bool {
        let publish = error.is_some_and(|reason| {
            self.swing_error_msg_like_cpp
                .is_none_or(|previous| previous != reason)
        });
        self.swing_error_msg_like_cpp = error;
        publish
    }

    pub const fn attack_swing_error_like_cpp(&self) -> Option<u8> {
        self.swing_error_msg_like_cpp
    }
}

#[cfg(test)]
mod tests {
    use super::Player;

    #[test]
    fn attack_swing_error_matches_cpp_duplicate_and_clear_rules() {
        let mut player = Player::new(Some(7), false);

        assert!(player.set_attack_swing_error_like_cpp(Some(0)));
        assert_eq!(player.attack_swing_error_like_cpp(), Some(0));
        assert!(!player.set_attack_swing_error_like_cpp(Some(0)));
        assert!(player.set_attack_swing_error_like_cpp(Some(1)));
        assert!(!player.set_attack_swing_error_like_cpp(None));
        assert_eq!(player.attack_swing_error_like_cpp(), None);
    }
}
