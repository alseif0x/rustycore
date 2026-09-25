//! Calendar handler regression scenarios.
//!
//! Shared entries/packets/session fixtures live in
//! [`crate::handlers::test_support`]; the scenario module below exercises the
//! production module next to it through `use super::*;`.

use super::*;
use crate::handlers::test_support::*;

mod calendar;
