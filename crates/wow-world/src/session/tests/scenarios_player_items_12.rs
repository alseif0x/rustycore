//! Session scenarios exercising the represented player-items responsibility.
//!
//! Split out of `session_tests.rs` under #626. Focused children preserve the
//! parent fixture boundary while grouping item updates and player-stat scenarios.

use super::*;

#[path = "scenarios_player_items_12/defensive_combat_stats.rs"]
mod defensive_combat_stats;
#[path = "scenarios_player_items_12/item_equipment_and_updates.rs"]
mod item_equipment_and_updates;
#[path = "scenarios_player_items_12/resistance_and_spell_power.rs"]
mod resistance_and_spell_power;
#[path = "scenarios_player_items_12/weapon_offense.rs"]
mod weapon_offense;
