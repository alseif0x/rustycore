//! C++ Player::SaveToDB schedules DELAYED_SAVE_PLAYER during far transfer
//! (Player.cpp:19327-19333). Rust retains intent until a confirmed save receipt.
//! No transaction, timer, transport or second Player owner lives here.
use super::Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct DeferredPlayerSave {
    revision: u64,
    pending: bool,
}

impl DeferredPlayerSave {
    pub(super) fn acknowledge(&mut self, captured: Self) {
        if captured.pending && self.revision == captured.revision {
            self.pending = false;
        }
    }
}

impl Player {
    /// Some(true): retained; Some(false): transfer permits preparation.
    /// None: revision exhaustion; the application must fail closed, never wrap.
    pub fn defer_save_if_transfer_pending_like_cpp(&mut self) -> Option<bool> {
        let transfer = self.teleport_state_like_cpp();
        if transfer.post_add.is_none()
            && (!transfer.far_pending
                || transfer.recovery == crate::PlayerTransferRecovery::Terminal)
        {
            return Some(false);
        }
        self.deferred_save.revision = self.deferred_save.revision.checked_add(1)?;
        self.deferred_save.pending = true;
        Some(true)
    }

    pub fn has_deferred_player_save_like_cpp(&self) -> bool {
        self.deferred_save.pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deferred_intent_is_acknowledged_only_at_its_captured_revision() {
        let mut player = Player::new(Some(1), false);
        assert_eq!(
            player.defer_save_if_transfer_pending_like_cpp(),
            Some(false)
        );
        player.teleport_state_mut_like_cpp().far_pending = true;
        assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), Some(true));
        let old = player.capture_save_acknowledgement_like_cpp();
        assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), Some(true));
        player.acknowledge_saved_projection_like_cpp(old, Default::default());
        assert!(player.has_deferred_player_save_like_cpp());
        let current = player.capture_save_acknowledgement_like_cpp();
        player.teleport_state_mut_like_cpp().far_pending = false;
        player.acknowledge_saved_projection_like_cpp(current, Default::default());
        assert!(!player.has_deferred_player_save_like_cpp());
    }

    #[test]
    fn terminal_source_is_admitted_but_post_add_and_revision_exhaustion_are_not() {
        let mut player = Player::new(Some(1), false);
        player.teleport_state_mut_like_cpp().far_pending = true;
        player.teleport_state_mut_like_cpp().recovery = crate::PlayerTransferRecovery::Terminal;
        assert_eq!(
            player.defer_save_if_transfer_pending_like_cpp(),
            Some(false)
        );
        player.teleport_state_mut_like_cpp().post_add =
            Some(crate::PlayerWorldportPostAddLikeCpp {
                map_id: 0,
                position: wow_core::Position::default(),
                phase: crate::PlayerWorldportPostAddPhaseLikeCpp::BeforeZone,
            });
        assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), Some(true));
        player.deferred_save.revision = u64::MAX;
        assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), None);
        assert!(player.has_deferred_player_save_like_cpp());
        assert_eq!(player.deferred_save.revision, u64::MAX);
    }
}
