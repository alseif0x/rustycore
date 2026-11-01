//! Retain transfer-blocked save intent on the checked Player, never on Session.
//! The resume moved to `wow_world_application::session_lifecycle` (#1363 slice 13b).
pub use wow_world_lifecycle::PlayerSaveOutcomeLikeCpp;
