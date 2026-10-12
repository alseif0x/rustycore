//! Creature runtime scenarios that need no `WorldSession`.
//!
//! #1363 slice 3b moved these regressions here from the `wow-world` session
//! scenario suite. They build canonical maps and creatures through the
//! `wow-world-test-support` fixtures and drive the runtime's phase bodies
//! directly or through [`crate::canonical_creatures`]. Scenarios that build a
//! session stay in `wow-world`.

use std::sync::Arc;
use std::time::Duration;

use crate::canonical_creatures::*;
use crate::*;
use wow_constants::TypeId;
use wow_world_entities::creature_spell_admission::creature_spell_target_accepts_npc_attack_like_cpp;
use wow_world_test_support::canonical_creatures::*;
use wow_world_test_support::creature_ai::*;
use wow_world_test_support::lifecycle::*;
use wow_world_test_support::map_objects::*;

#[path = "scenarios/f6_8d2_admitted_execution.rs"]
mod f6_8d2_admitted_execution;
#[path = "scenarios/scenarios_world_entities_21.rs"]
mod scenarios_world_entities_21;
#[path = "scenarios/scenarios_world_entities_24.rs"]
mod scenarios_world_entities_24;
#[path = "scenarios/scenarios_world_entities_27.rs"]
mod scenarios_world_entities_27;
#[path = "scenarios/scenarios_world_entities_29.rs"]
mod scenarios_world_entities_29;
#[path = "scenarios/scenarios_world_entities_30.rs"]
mod scenarios_world_entities_30;
#[path = "scenarios/scenarios_world_entities_31.rs"]
mod scenarios_world_entities_31;
