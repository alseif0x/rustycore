// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Cinematic adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

impl WorldSession {
    pub(in crate::session) fn player_cinematic_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerCinematicStateLikeCpp> {
        let canonical = self.with_owned_player_like_cpp(|player| player.gameplay_state().cinematic);
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(self.represented_cinematic_state_like_cpp);
        }
        None
    }

    pub(crate) fn opening_cinematic_like_cpp(&mut self) -> Option<u32> {
        if self.resolved_player_xp_like_cpp()? != 0 {
            return None;
        }

        let class_store = self.chr.classes_store.as_ref()?;
        let class_entry = class_store.get(u32::from(self.player_class_like_cpp()))?;
        let cinematic_id = if class_entry.cinematic_sequence_id != 0 {
            u32::from(class_entry.cinematic_sequence_id)
        } else {
            let race_store = self.chr.races_store.as_ref()?;
            race_store
                .get(u32::from(self.player_race_like_cpp()))
                .map(|race_entry| race_entry.cinematic_sequence_id as u32)?
        };

        self.send_represented_cinematic_start_like_cpp(cinematic_id);
        Some(cinematic_id)
    }

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
            #[cfg(not(test))]
            let _ = cinematic_id;
            #[cfg(test)]
            self.represented_cinematic_end_events_like_cpp
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
        #[cfg(test)]
        self.represented_cinematic_next_camera_events_like_cpp
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
            #[cfg(not(test))]
            let _ = movie_id;
            #[cfg(test)]
            self.represented_movie_complete_events_like_cpp
                .push(movie_id);
        }
    }

    #[cfg(test)]
    pub(crate) fn represented_cinematic_like_cpp(&self) -> Option<u32> {
        self.player_cinematic_state_snapshot_like_cpp()
            .and_then(|state| state.cinematic_id_like_cpp())
    }

    #[cfg(test)]
    pub(crate) fn represented_cinematic_camera_index_like_cpp(&self) -> i32 {
        self.player_cinematic_state_snapshot_like_cpp()
            .map_or(-1, |state| state.camera_index_like_cpp())
    }

    #[cfg(test)]
    pub(crate) fn represented_cinematic_next_camera_events_like_cpp(&self) -> &[u16] {
        &self.represented_cinematic_next_camera_events_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_cinematic_end_events_like_cpp(&self) -> &[u32] {
        &self.represented_cinematic_end_events_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_movie_like_cpp(&self) -> Option<u32> {
        self.player_cinematic_state_snapshot_like_cpp()
            .and_then(|state| state.movie_id_like_cpp())
    }

    #[cfg(test)]
    pub(crate) fn represented_movie_complete_events_like_cpp(&self) -> &[u32] {
        &self.represented_movie_complete_events_like_cpp
    }
}
