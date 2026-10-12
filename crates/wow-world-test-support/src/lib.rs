//! Dev-only fixtures for the `wow-world` unit tests that need no `WorldSession`.
//!
//! #1363 slice 0 seeds this crate with the session-free builders of the
//! `session::tests::fixtures` modules so they stop compiling inside the
//! `wow-world` shell. It is a `dev-dependency` only: no production crate may
//! depend on it, and nothing here may name `WorldSession` or a `wow-world`
//! item. Fixtures that need a session stay in `wow-world`.

pub mod canonical_creatures;
pub mod creature_ai;
pub mod lifecycle;
pub mod map_objects;
pub mod packets;
pub mod spell_catalog;
