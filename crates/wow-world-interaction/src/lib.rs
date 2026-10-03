//! Session NPC interaction state and its session-facing operations.

mod session;
mod state;
#[cfg(any(test, feature = "test-fixtures"))]
mod test_support;

pub use state::{InteractionState, VendorItemCount};
#[cfg(any(test, feature = "test-fixtures"))]
pub use state::VendorBuyItemTestOverrideLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use test_support::SupportFeatureTestFixtureLikeCpp;
