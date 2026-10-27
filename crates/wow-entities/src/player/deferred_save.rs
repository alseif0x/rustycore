//! C++ Player::SaveToDB schedules DELAYED_SAVE_PLAYER during far transfer
//! (Player.cpp:19327-19333). Rust retains intent until a confirmed save receipt.
//! No transaction, timer, transport or second Player owner lives here.
use super::Player;
use crate::PlayerResurrectionPhase;

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
    /// Some(true): retained; Some(false): canonical admission permits capture.
    /// A resurrection save is admitted only after corpse deletion. SavingPlayer
    /// marks snapshot eligibility; the existing save/money fences serialize
    /// concurrent captures. An unresolved native death operation also retains
    /// periodic-save intent until its owner completes. None means deferred
    /// revision exhaustion; the application must fail closed, never wrap.
    pub fn defer_save_if_transfer_pending_like_cpp(&mut self) -> Option<bool> {
        let native_death_pending = self.has_unresolved_native_death_operation();
        let transfer = self.teleport_state_like_cpp();
        let transfer_pending = transfer.post_add.is_some()
            || (transfer.far_pending
                && transfer.recovery != crate::PlayerTransferRecovery::Terminal);
        let save_blocked = native_death_pending || transfer_pending;
        let resurrection_phase = self
            .resurrection_state_like_cpp()
            .delayed_after_teleport
            .map(|continuation| continuation.phase);

        match resurrection_phase {
            None if !save_blocked => return Some(false),
            Some(PlayerResurrectionPhase::ReadyToSavePlayer)
            | Some(PlayerResurrectionPhase::PlayerSaveDeferred)
                if !save_blocked =>
            {
                if let Some(continuation) = self
                    .resurrection_state_mut_like_cpp()
                    .delayed_after_teleport
                    .as_mut()
                {
                    continuation.phase = PlayerResurrectionPhase::SavingPlayer;
                }
                return Some(false);
            }
            Some(PlayerResurrectionPhase::ReadyToSavePlayer)
            | Some(PlayerResurrectionPhase::PlayerSaveDeferred)
                if save_blocked =>
            {
                if let Some(continuation) = self
                    .resurrection_state_mut_like_cpp()
                    .delayed_after_teleport
                    .as_mut()
                {
                    continuation.phase = PlayerResurrectionPhase::PlayerSaveDeferred;
                }
            }
            Some(PlayerResurrectionPhase::SavingPlayer) if !save_blocked => {
                // SavingPlayer is the canonical capture-eligible phase, not a
                // one-shot lease. Existing money/save fences serialize writers.
                return Some(false);
            }
            Some(PlayerResurrectionPhase::SavingPlayer) => {
                // A transfer or native death arriving during an uncertain save
                // must not turn its retained phase into a replayable save.
            }
            _ => {}
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
    fn native_death_defers_save_and_stale_ack_preserves_newer_intent() {
        for phase in [
            crate::PlayerDeathOperationPhase::Ready,
            crate::PlayerDeathOperationPhase::InFlight,
        ] {
            let mut player = Player::new(Some(1), false);
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            let context =
                crate::PlayerDeathOperationContext::new(wow_core::ObjectGuid::EMPTY, true);
            let operation = player
                .begin_native_death_operation(context)
                .expect("living Player should admit native death operation");
            if phase == crate::PlayerDeathOperationPhase::InFlight {
                assert_eq!(
                    player.claim_native_death_operation(operation),
                    Some(context)
                );
            }

            assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), Some(true));
            assert!(player.has_deferred_player_save_like_cpp());
            let stale_ack = player.capture_save_acknowledgement_like_cpp();

            assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), Some(true));
            player.acknowledge_saved_projection_like_cpp(stale_ack, Default::default());
            assert!(
                player.has_deferred_player_save_like_cpp(),
                "an older acknowledgement must not clear a newer deferred-save revision"
            );

            if phase == crate::PlayerDeathOperationPhase::Ready {
                assert_eq!(
                    player.claim_native_death_operation(operation),
                    Some(context)
                );
            }
            assert!(player.complete_native_death_operation(operation));
            assert_eq!(
                player.defer_save_if_transfer_pending_like_cpp(),
                Some(false)
            );

            let current_ack = player.capture_save_acknowledgement_like_cpp();
            player.acknowledge_saved_projection_like_cpp(current_ack, Default::default());
            assert!(!player.has_deferred_player_save_like_cpp());
        }
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

    #[test]
    fn full_save_admission_waits_for_bones_and_allows_only_the_saving_phase() {
        let mut player = Player::new(Some(1), false);
        let continuation = crate::PlayerResurrectionContinuation {
            operation: crate::PlayerResurrectionOperation::TeleportRevivePercent {
                map_id: 571,
                position: wow_core::Position::default(),
                restore_percent: 0.5,
            },
            phase: PlayerResurrectionPhase::BonesInFlight,
        };
        player
            .resurrection_state_mut_like_cpp()
            .delayed_after_teleport = Some(continuation);

        assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), Some(true));
        assert_eq!(
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .unwrap()
                .phase,
            PlayerResurrectionPhase::BonesInFlight
        );
        assert!(player.has_deferred_player_save_like_cpp());

        player
            .resurrection_state_mut_like_cpp()
            .delayed_after_teleport
            .as_mut()
            .unwrap()
            .phase = PlayerResurrectionPhase::ReadyToSavePlayer;
        assert_eq!(
            player.defer_save_if_transfer_pending_like_cpp(),
            Some(false)
        );
        assert_eq!(
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .unwrap()
                .phase,
            PlayerResurrectionPhase::SavingPlayer
        );

        assert_eq!(
            player.defer_save_if_transfer_pending_like_cpp(),
            Some(false)
        );
        assert_eq!(
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .unwrap()
                .phase,
            PlayerResurrectionPhase::SavingPlayer,
            "SavingPlayer is snapshot-eligible; the existing save fence serializes writers"
        );
    }

    #[test]
    fn resurrection_save_retries_after_transfer_deferral_without_replaying_bones() {
        let mut player = Player::new(Some(1), false);
        player.teleport_state_mut_like_cpp().far_pending = true;
        player
            .resurrection_state_mut_like_cpp()
            .delayed_after_teleport = Some(crate::PlayerResurrectionContinuation {
            operation: crate::PlayerResurrectionOperation::RequestData(
                crate::PlayerResurrectionRequestLikeCpp {
                    resurrecter: wow_core::ObjectGuid::create_player(1, 77),
                    map_id: 571,
                    position: wow_core::Position::default(),
                    health: 100,
                    mana: 0,
                    aura: 0,
                },
            ),
            phase: PlayerResurrectionPhase::ReadyToSavePlayer,
        });

        assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), Some(true));
        assert_eq!(
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .unwrap()
                .phase,
            PlayerResurrectionPhase::PlayerSaveDeferred
        );
        player.teleport_state_mut_like_cpp().far_pending = false;
        assert_eq!(
            player.defer_save_if_transfer_pending_like_cpp(),
            Some(false)
        );
        assert_eq!(
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .unwrap()
                .phase,
            PlayerResurrectionPhase::SavingPlayer
        );
    }

    #[test]
    fn transfer_does_not_downgrade_uncertain_resurrection_save_phase() {
        let mut player = Player::new(Some(1), false);
        player
            .resurrection_state_mut_like_cpp()
            .delayed_after_teleport = Some(crate::PlayerResurrectionContinuation {
            operation: crate::PlayerResurrectionOperation::TeleportRevivePercent {
                map_id: 571,
                position: wow_core::Position::default(),
                restore_percent: 0.5,
            },
            phase: PlayerResurrectionPhase::SavingPlayer,
        });
        player.teleport_state_mut_like_cpp().far_pending = true;

        assert_eq!(player.defer_save_if_transfer_pending_like_cpp(), Some(true));
        assert_eq!(
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .unwrap()
                .phase,
            PlayerResurrectionPhase::SavingPlayer
        );
        assert!(player.has_deferred_player_save_like_cpp());
    }
}
