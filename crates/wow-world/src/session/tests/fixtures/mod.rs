//! Responsibility-scoped builders shared by the Session scenario modules.
//!
//! The parent `session::tests` module keeps the scenario registration root and
//! re-exports this narrow fixture surface so existing `use super::*` consumers
//! retain their private access without a second test-support aggregate.

use super::*;

#[path = "creature_ai.rs"]
mod creature_ai_fixtures;
#[path = "creatures.rs"]
mod creature_fixtures;
#[path = "instances.rs"]
mod instance_fixtures;
#[path = "interaction.rs"]
mod interaction_fixtures;
#[path = "item_collections.rs"]
mod item_collection_fixtures;
#[path = "lifecycle.rs"]
mod lifecycle_fixtures;
#[path = "login.rs"]
mod login_fixtures;
#[path = "map_objects.rs"]
mod map_object_fixtures;
#[path = "packets.rs"]
mod packet_fixtures;
#[path = "player_items.rs"]
mod player_item_fixtures;
#[path = "quests.rs"]
mod quest_fixtures;
#[path = "session.rs"]
mod session_fixtures;
#[path = "spell_catalog.rs"]
mod spell_catalog_fixtures;
#[path = "summons.rs"]
mod summon_fixtures;
#[path = "visibility.rs"]
mod visibility_fixtures;

pub(super) use creature_ai_fixtures::*;
pub(super) use creature_fixtures::*;
pub(super) use instance_fixtures::*;
pub(super) use interaction_fixtures::*;
pub(super) use item_collection_fixtures::*;
pub(super) use lifecycle_fixtures::*;
pub(super) use login_fixtures::*;
pub(super) use map_object_fixtures::*;
pub(super) use packet_fixtures::*;
pub(super) use player_item_fixtures::*;
pub(super) use quest_fixtures::*;
pub(super) use session_fixtures::*;
pub(super) use spell_catalog_fixtures::*;
pub(super) use summon_fixtures::*;
pub(super) use visibility_fixtures::*;
