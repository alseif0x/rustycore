//! Account-data and client-state handler regression scenarios.
//!
//! Shared entries/packets/session fixtures live in
//! [`crate::handlers::test_support`]; each scenario module below exercises the
//! production module next to it through `use super::*;`.

use super::*;
use crate::handlers::test_support::*;

mod account_data;
mod client_state;
