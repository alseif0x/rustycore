// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical cinematic fixture adapters shared with World.

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::PlayerPresentationState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_cinematic_next_camera_events_like_cpp(&self) -> &[u16] {
        &self.represented_cinematic_next_camera_events_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_cinematic_end_events_like_cpp(&self) -> &[u32] {
        &self.represented_cinematic_end_events_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_movie_complete_events_like_cpp(&self) -> &[u32] {
        &self.represented_movie_complete_events_like_cpp
    }
}

impl crate::session::HubMut<'_> {
    pub fn complete_represented_cinematic_like_cpp(&mut self) {
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

    pub fn next_represented_cinematic_camera_like_cpp(&mut self) {
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

    pub fn complete_represented_movie_like_cpp(&mut self) {
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
    pub fn player_cinematic_state_snapshot_like_cpp(
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
    pub fn represented_cinematic_like_cpp(&self) -> Option<u32> {
        self.player_cinematic_state_snapshot_like_cpp()
            .and_then(|state| state.cinematic_id_like_cpp())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_cinematic_camera_index_like_cpp(&self) -> i32 {
        self.player_cinematic_state_snapshot_like_cpp()
            .map_or(-1, |state| state.camera_index_like_cpp())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_movie_like_cpp(&self) -> Option<u32> {
        self.player_cinematic_state_snapshot_like_cpp()
            .and_then(|state| state.movie_id_like_cpp())
    }
}
