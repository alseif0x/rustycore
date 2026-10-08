//! Represented instance and difficulty responsibility, separated from the
//! Session root under #613. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

mod difficulty;
mod lfg;
// `pub` only so `session/mod.rs` can re-export the #1263 C2 capture record; the
// enclosing `instances` module stays private, so nothing new becomes reachable.
pub mod map_key;
mod map_resolution;
