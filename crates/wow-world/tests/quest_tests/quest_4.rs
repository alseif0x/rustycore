//! Quest sharing scenarios; fixtures remain in the integration target root.
//!
//! Cases are grouped by the admission responsibility they exercise.

use super::*;

#[path = "quest_4/pool_admission.rs"]
mod pool_admission;
#[path = "quest_4/receiver_admission.rs"]
mod receiver_admission;
#[path = "quest_4/receiver_requirements.rs"]
mod receiver_requirements;
