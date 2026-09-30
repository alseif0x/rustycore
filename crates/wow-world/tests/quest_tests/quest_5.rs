//! Quest sharing prerequisite, receiver gate and delivery scenarios.
//!
//! Fixtures and registrations remain in the integration target root.

use super::*;

#[path = "quest_5/delivery.rs"]
mod delivery;
#[path = "quest_5/prerequisites.rs"]
mod prerequisites;
#[path = "quest_5/receiver_gates.rs"]
mod receiver_gates;
