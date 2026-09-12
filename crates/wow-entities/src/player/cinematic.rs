// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player cinematic and movie state.
//!
//! C++ splits this between the Player and the `CinematicMgr` it owns:
//! `Player::SendCinematicStart` (`Entities/Player/Player.cpp`) sends the packet
//! and calls `CinematicMgr::BeginCinematic` (`CinematicMgr.h:39`), which stores
//! the sequence and its cameras; `CinematicMgr::NextCinematicCamera`
//! (`CinematicMgr.cpp:46`) advances through them; `CinematicMgr::EndCinematic`
//! (`:83`) drops the active sequence; and `Player::SendMovieStart`
//! (`Player.cpp`) sets the movie before sending its packet.
//!
//! Separated from `player_gameplay_state.rs` under #777, which also closed the
//! members: the packets stay with the session that owns the connection, and
//! each operation answers what the caller must send.
//!
//! One departure is preserved, not introduced here: C++ reads the camera at the
//! index it is about to leave and pre-increments, which runs off the end of the
//! array for a malformed sequence; RustyCore advances first and refuses the
//! out-of-bounds edge instead of reproducing undefined behaviour.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCinematicStateLikeCpp {
    cinematic_id: Option<u32>,
    camera_ids: Option<[u16; 8]>,
    camera_index: i32,
    movie_id: Option<u32>,
}

impl Default for PlayerCinematicStateLikeCpp {
    fn default() -> Self {
        Self {
            cinematic_id: None,
            camera_ids: None,
            camera_index: -1,
            movie_id: None,
        }
    }
}

impl PlayerCinematicStateLikeCpp {
    /// The active cinematic sequence, if one is playing.
    #[must_use]
    pub fn cinematic_id_like_cpp(&self) -> Option<u32> {
        self.cinematic_id
    }

    /// The camera list of the active sequence.
    #[must_use]
    pub fn camera_ids_like_cpp(&self) -> Option<[u16; 8]> {
        self.camera_ids
    }

    /// The camera index the sequence has reached; `-1` before the first camera,
    /// as C++ leaves `m_activeCinematicCameraIndex` when a sequence begins.
    #[must_use]
    pub fn camera_index_like_cpp(&self) -> i32 {
        self.camera_index
    }

    /// The movie the client was told to play, if any.
    #[must_use]
    pub fn movie_id_like_cpp(&self) -> Option<u32> {
        self.movie_id
    }

    /// C++ `CinematicMgr::BeginCinematic` (`CinematicMgr.h:39`): the sequence
    /// and its cameras are stored together and the index returns to its
    /// pre-first-camera value, so a new sequence can never continue the
    /// previous one's camera walk.
    pub fn begin_cinematic_like_cpp(&mut self, cinematic_id: u32, camera_ids: [u16; 8]) {
        self.cinematic_id = Some(cinematic_id);
        self.camera_ids = Some(camera_ids);
        self.camera_index = -1;
    }

    /// C++ `CinematicMgr::EndCinematic` (`CinematicMgr.cpp:83`): the active
    /// sequence is dropped with its cameras. Answers the sequence that ended,
    /// which the caller reports, or `None` when none was playing.
    pub fn end_cinematic_like_cpp(&mut self) -> Option<u32> {
        let cinematic_id = self.cinematic_id.take();
        self.camera_ids = None;
        self.camera_index = -1;
        cinematic_id
    }

    /// C++ `CinematicMgr::NextCinematicCamera` (`CinematicMgr.cpp:46`)
    /// advancing to the next camera of the active sequence.
    ///
    /// Answers the camera to move to, or `None` when no sequence is playing,
    /// the sequence carries no cameras, or the walk has reached the end of the
    /// list — the out-of-bounds edge C++ does not guard.
    pub fn next_cinematic_camera_like_cpp(&mut self) -> Option<u16> {
        self.cinematic_id?;
        let camera_ids = self.camera_ids?;
        if self.camera_index >= camera_ids.len() as i32 {
            return None;
        }
        self.camera_index += 1;
        camera_ids.get(self.camera_index as usize).copied()
    }

    /// C++ `Player::SetMovie`, which `Player::SendMovieStart` calls before
    /// sending the packet.
    pub fn set_movie_like_cpp(&mut self, movie_id: Option<u32>) {
        self.movie_id = movie_id;
    }

    /// Take the movie the client finished, so the same completion is not
    /// reported twice.
    pub fn take_movie_like_cpp(&mut self) -> Option<u32> {
        self.movie_id.take()
    }
}
