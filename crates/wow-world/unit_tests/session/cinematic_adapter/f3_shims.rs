// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_cinematic_like_cpp(&self) -> Option<u32> {
        crate::session::hub_ref(self).represented_cinematic_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_cinematic_camera_index_like_cpp(&self) -> i32 {
        crate::session::hub_ref(self).represented_cinematic_camera_index_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_cinematic_next_camera_events_like_cpp(&self) -> &[u16] {
        self.fixtures
            .presentation
            .represented_cinematic_next_camera_events_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_cinematic_end_events_like_cpp(&self) -> &[u32] {
        self.fixtures
            .presentation
            .represented_cinematic_end_events_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_movie_like_cpp(&self) -> Option<u32> {
        crate::session::hub_ref(self).represented_movie_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_movie_complete_events_like_cpp(&self) -> &[u32] {
        self.fixtures
            .presentation
            .represented_movie_complete_events_like_cpp()
    }
}
