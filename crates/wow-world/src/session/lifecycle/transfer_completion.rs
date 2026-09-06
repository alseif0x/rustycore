//! C++ completes worldport native effects before logout/save (WorldSession.cpp:544).
//! Rust can suspend in visibility or world-state reads after clearing the far flag.
//! Retain only remaining native phases on the canonical Player; never replay the ACK.
use crate::session::WorldSession;
use wow_core::Position;
use wow_entities::{PlayerWorldportPostAddLikeCpp, PlayerWorldportPostAddPhaseLikeCpp as Phase};

impl WorldSession {
    pub(crate) fn begin_worldport_post_add_like_cpp(
        &mut self,
        map_id: u32,
        position: Position,
    ) -> bool {
        self.with_owned_player_mut_like_cpp(|player| {
            if player.unit().world().map_id() != map_id
                || player.unit().world().position() != position
            {
                return false;
            }
            let state = player.teleport_state_mut_like_cpp();
            if state.post_add.is_some() {
                return false;
            }
            state.post_add = Some(PlayerWorldportPostAddLikeCpp {
                map_id,
                position,
                phase: Phase::BeforeZone,
            });
            true
        })
        .unwrap_or(false)
    }

    pub(crate) fn advance_worldport_post_add_like_cpp(&mut self, phase: Phase) -> bool {
        self.with_owned_player_mut_like_cpp(|player| {
            if let Some(progress) = player.teleport_state_like_cpp().post_add
                && (player.unit().world().map_id() != progress.map_id
                    || player.unit().world().position() != progress.position)
            {
                return false;
            }
            if let Some(progress) = &mut player.teleport_state_mut_like_cpp().post_add {
                progress.phase = progress.phase.max(phase);
            }
            true
        })
        .unwrap_or(false)
    }

    /// Finish represented native phases, not client initialization or a durable save.
    /// False forbids ordinary disconnect cleanup: the owner/completion is unavailable.
    /// No packet, query or await is needed; terrain reads remain outside map guards.
    pub fn finish_worldport_native_before_disconnect_like_cpp(&mut self) -> bool {
        if self.player_guid().is_none() {
            return true;
        }
        let Some(state) = self.player_teleport_state_snapshot_like_cpp() else {
            return false;
        };
        let Some(progress) = state.post_add else {
            return true;
        };
        if !self
            .with_owned_player_like_cpp(|player| {
                player.unit().world().map_id() == progress.map_id
                    && player.unit().world().position() == progress.position
            })
            .unwrap_or(false)
        {
            return false;
        }
        if progress.phase == Phase::BeforeZone {
            if !self.apply_post_add_zone_from_terrain_like_cpp(
                progress.map_id as i32,
                &progress.position,
            ) || !self.advance_worldport_post_add_like_cpp(Phase::ZoneApplied)
            {
                return false;
            }
        }
        if progress.phase < Phase::ScalingApplied {
            if self
                .update_represented_item_level_area_based_scaling_with_publication_like_cpp(false)
                .is_none()
                || !self.advance_worldport_post_add_like_cpp(Phase::ScalingApplied)
            {
                return false;
            }
        }
        self.resummon_pet_temporary_unsummoned_like_cpp();
        self.process_represented_delayed_resurrection_after_teleport_like_cpp();
        self.with_owned_player_mut_like_cpp(|player| {
            player.teleport_state_mut_like_cpp().post_add = None;
        })
        .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_save_rejects_retained_post_add_until_native_completion() {
        let (_input, packets) = flume::unbounded();
        let (output, _receiver) = flume::unbounded();
        let mut session = WorldSession::new(
            1,
            "transfer-save".into(),
            0,
            2,
            2,
            54261,
            vec![],
            "enUS".into(),
            packets,
            output,
        );
        session.set_player_guid(Some(wow_core::ObjectGuid::create_player(1, 42)));
        crate::canonical_player_access::install_canonical_player_owner_for_test(
            &mut session,
            571,
            0,
        );
        let position = session.player_position_like_cpp().unwrap();
        assert!(session.prepare_player_save_like_cpp(1).is_some());
        assert!(session.begin_worldport_post_add_like_cpp(571, position));
        assert!(session.prepare_player_save_like_cpp(1).is_none());
        assert!(
            session
                .player_teleport_state_snapshot_like_cpp()
                .unwrap()
                .post_add
                .is_some()
        );
        assert!(session.finish_worldport_native_before_disconnect_like_cpp());
        assert!(session.prepare_player_save_like_cpp(1).is_some());
    }
}
