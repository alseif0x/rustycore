use crate::session::SessionState;
use crate::session::mailbox::{MapPhaseAdmissionLikeCpp, PlayerRecoveryAdmission};
use crate::session::state::SessionCore;
use wow_core::ObjectGuid;
use wow_handler::PacketUpdatePhase;
use wow_map::{MapKey, MapTickCoordinationStateLikeCpp};

impl SessionCore {
    pub fn is_native_player_death_admission_current(
        &self,
        admission: &crate::session::mailbox::NativePlayerDeathAdmission,
    ) -> bool {
        self.is_map_combat_admission_current(&admission.map_phase, admission.map_tick_epoch)
            && self
                .with_owned_player_like_cpp(|player| {
                    player
                        .native_death_operation_snapshot()
                        .is_some_and(|operation| {
                            operation.id() == admission.operation
                                && operation.phase()
                                    == wow_entities::PlayerDeathOperationPhase::Ready
                        })
                })
                .unwrap_or(false)
    }

    pub fn is_map_combat_admission_current(
        &self,
        phase: &MapPhaseAdmissionLikeCpp,
        map_tick_epoch: u64,
    ) -> bool {
        if self.state != SessionState::LoggedIn
            || !self
                .admission
                .is_map_phase_authority_current(phase.coordinator_id, phase.tick_epoch)
            || phase.phase != PacketUpdatePhase::Map
            || self
                .current_player_binding_guid(phase.registration, phase.handle)
                .is_none()
        {
            return false;
        }
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };
        manager.tick_coordination_like_cpp()
            == MapTickCoordinationStateLikeCpp::AwaitingSessions(map_tick_epoch)
            && manager.map_incarnation_like_cpp(phase.map_key) == Some(phase.map_incarnation)
            && manager.player_active_residence_revision_like_cpp(phase.handle)
                == Some((phase.map_key, phase.residence_revision))
            && manager
                .with_player_like_cpp(phase.handle, |player| {
                    let world = player.unit().world();
                    world.object().is_in_world()
                        && world.has_current_map()
                        && MapKey::new(world.map_id(), world.instance_id()) == phase.map_key
                })
                .unwrap_or(false)
    }

    /// Revalidate the frozen facts used by the existing Map phase admission.
    /// Its original phase, registration, handle, map-incarnation, and residence
    /// checks remain unchanged; no Recovery-only status checks are added here.
    pub fn is_map_phase_admission_current(&self, admission: &MapPhaseAdmissionLikeCpp) -> bool {
        if admission.phase != PacketUpdatePhase::Map
            || self
                .current_player_binding_guid(admission.registration, admission.handle)
                .is_none()
        {
            return false;
        }

        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };
        if manager.map_incarnation_like_cpp(admission.map_key) != Some(admission.map_incarnation) {
            return false;
        }
        manager.player_active_residence_revision_like_cpp(admission.handle)
            == Some((admission.map_key, admission.residence_revision))
    }

    /// Validate an admitted Player recovery against the current session,
    /// directory registration, Map tick, and canonical active Player owner.
    pub fn is_player_recovery_admission_current(
        &self,
        admission: &PlayerRecoveryAdmission,
    ) -> bool {
        if self.state != SessionState::LoggedIn {
            return false;
        }
        let Some(guid) = self.current_player_binding_guid(admission.registration, admission.handle)
        else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };

        if manager.tick_coordination_like_cpp()
            != MapTickCoordinationStateLikeCpp::AwaitingSessions(admission.map_tick_epoch)
            || manager.map_incarnation_like_cpp(admission.map_key)
                != Some(admission.map_incarnation)
            || manager.player_active_residence_revision_like_cpp(admission.handle)
                != Some((admission.map_key, admission.residence_revision))
        {
            return false;
        }

        manager
            .with_player_like_cpp(admission.handle, |player| {
                let world = player.unit().world();
                player.guid() == guid
                    && world.object().is_in_world()
                    && world.has_current_map()
                    && MapKey::new(world.map_id(), world.instance_id()) == admission.map_key
            })
            .unwrap_or(false)
    }

    /// Check identity and directory facts before taking the Map manager lock.
    fn current_player_binding_guid(
        &self,
        registration: crate::session::directory::PlayerRegistration,
        handle: wow_map::PlayerHandle,
    ) -> Option<ObjectGuid> {
        let guid = self.player_guid()?;
        if registration.guid() != guid {
            return None;
        }
        self.player_registry()?.lookup_current(registration)?;
        (self.player_handle_like_cpp == Some(handle)).then_some(guid)
    }
}
