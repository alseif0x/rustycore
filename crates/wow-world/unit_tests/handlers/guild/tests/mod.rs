//! Guild handler regression scenarios.
//!
//! Shared entries/packets/session fixtures live in
//! [`crate::handlers::test_support`]. Every guild opcode is now registered by
//! its owner crate, so the scenarios drive the registered handlers through the
//! cfg(test) entry points of `super::test_shims`.

use crate::handlers::test_support::*;

mod guild;
