// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Cinematic adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

impl WorldSession {
    pub(crate) fn opening_cinematic_like_cpp(&mut self) -> Option<u32> {
        if crate::session::hub_ref(self).resolved_player_xp_like_cpp()? != 0 {
            return None;
        }

        let class_store = self.catalogs.chr.classes_store.as_ref()?;
        let class_entry = class_store.get(u32::from(
            crate::session::hub_ref(self).player_class_like_cpp(),
        ))?;
        let cinematic_id = if class_entry.cinematic_sequence_id != 0 {
            u32::from(class_entry.cinematic_sequence_id)
        } else {
            let race_store = self.catalogs.chr.races_store.as_ref()?;
            race_store
                .get(u32::from(
                    crate::session::hub_ref(self).player_race_like_cpp(),
                ))
                .map(|race_entry| race_entry.cinematic_sequence_id as u32)?
        };

        self.send_represented_cinematic_start_like_cpp(cinematic_id);
        Some(cinematic_id)
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::PlayerPresentationState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_cinematic_next_camera_events_like_cpp(&self) -> &[u16] {
        &self.represented_cinematic_next_camera_events_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_cinematic_end_events_like_cpp(&self) -> &[u32] {
        &self.represented_cinematic_end_events_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_movie_complete_events_like_cpp(&self) -> &[u32] {
        &self.represented_movie_complete_events_like_cpp
    }
}

impl crate::session::HubMut<'_> {
    pub(crate) fn complete_represented_cinematic_like_cpp(&mut self) {
        let Some(cinematic_id) = self
            .with_player_cinematic_state_like_cpp(
                wow_entities::PlayerCinematicStateLikeCpp::end_cinematic_like_cpp,
            )
            .flatten()
        else {
            return;
        };
        {
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let _ = cinematic_id;
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures
                .presentation
                .represented_cinematic_end_events_like_cpp
                .push(cinematic_id);
        }
    }

    pub(crate) fn next_represented_cinematic_camera_like_cpp(&mut self) {
        // The owner keeps the recorded departure: C++ checks the previous index
        // before pre-incrementing, and RustyCore refuses the out-of-bounds edge
        // instead of reproducing undefined behavior.
        let Some(Some(camera_id)) = self.with_player_cinematic_state_like_cpp(
            wow_entities::PlayerCinematicStateLikeCpp::next_cinematic_camera_like_cpp,
        ) else {
            return;
        };
        if camera_id == 0 {
            return;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .presentation
            .represented_cinematic_next_camera_events_like_cpp
            .push(camera_id);
    }

    pub(crate) fn complete_represented_movie_like_cpp(&mut self) {
        let Some(movie_id) = self
            .with_player_cinematic_state_like_cpp(
                wow_entities::PlayerCinematicStateLikeCpp::take_movie_like_cpp,
            )
            .flatten()
        else {
            return;
        };
        {
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let _ = movie_id;
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures
                .presentation
                .represented_movie_complete_events_like_cpp
                .push(movie_id);
        }
    }
}

impl crate::session::HubRef<'_> {
    pub(in crate::session) fn player_cinematic_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerCinematicStateLikeCpp> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.gameplay_state().cinematic);
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.fixtures
                    .presentation
                    .represented_cinematic_state_like_cpp,
            );
        }
        None
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_cinematic_like_cpp(&self) -> Option<u32> {
        self.player_cinematic_state_snapshot_like_cpp()
            .and_then(|state| state.cinematic_id_like_cpp())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_cinematic_camera_index_like_cpp(&self) -> i32 {
        self.player_cinematic_state_snapshot_like_cpp()
            .map_or(-1, |state| state.camera_index_like_cpp())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_movie_like_cpp(&self) -> Option<u32> {
        self.player_cinematic_state_snapshot_like_cpp()
            .and_then(|state| state.movie_id_like_cpp())
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/cinematic_adapter/f3_shims.rs"]
mod f3_shims;
