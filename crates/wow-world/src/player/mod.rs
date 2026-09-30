//! Player modules.
//!
//! Grouped from the player_* siblings under #697; each module keeps its
//! items and its re-exported names.

pub mod directory_canonical_queries;
#[cfg(any(test, feature = "test-fixtures"))]
pub mod directory_test_fixtures;
#[cfg(test)]
#[path = "../../unit_tests/player/inventory_persistence_test_fixture.rs"]
pub mod inventory_persistence_test_fixture;
#[cfg(test)]
pub mod lifecycle_contract;
pub mod quest_persistence_projection;
#[cfg(test)]
#[path = "../../unit_tests/player/quest_persistence_test_fixture.rs"]
pub mod quest_persistence_test_fixture;

#[cfg(any(test, feature = "test-fixtures"))]
#[cfg(test)]
pub use inventory_persistence_test_fixture::*;
#[cfg(test)]
pub use lifecycle_contract::*;
#[cfg(test)]
pub use quest_persistence_test_fixture::*;
