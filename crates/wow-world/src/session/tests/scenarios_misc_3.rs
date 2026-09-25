//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_misc_3/dynamic_object_snapshots.rs"]
mod dynamic_object_snapshots;
#[path = "scenarios_misc_3/map_value_publication.rs"]
mod map_value_publication;
#[path = "scenarios_misc_3/rest_and_far_sight.rs"]
mod rest_and_far_sight;
#[path = "scenarios_misc_3/trainer_interaction.rs"]
mod trainer_interaction;
