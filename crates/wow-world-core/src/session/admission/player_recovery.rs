use crate::session::SessionState;
use crate::session::mailbox::{MapPhaseAdmissionLikeCpp, PlayerRecoveryAdmission};
use crate::session::state::SessionCore;
use wow_core::ObjectGuid;
use wow_handler::PacketUpdatePhase;
use wow_map::{MapKey, MapTickCoordinationStateLikeCpp};

impl SessionCore {
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
