//! Represented loot operations owned at the Session boundary.
//!
//! Separated from the Session root under #632.

use super::*;

mod operations;
mod creature_owner;
mod gameobject_owner;
mod authority_reconciliation;
