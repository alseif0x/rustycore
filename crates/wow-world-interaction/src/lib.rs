//! Session NPC interaction state and its session-facing operations.

mod npc_handlers;
mod session;
mod state;
#[cfg(any(test, feature = "test-fixtures"))]
mod test_support;

pub use npc_handlers::{
    NpcHandlerCxLikeCpp, NpcHandlerHostLikeCpp, register_npc_handlers_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use state::VendorBuyItemTestOverrideLikeCpp;
pub use state::{InteractionState, VendorItemCount};
#[cfg(any(test, feature = "test-fixtures"))]
pub use test_support::SupportFeatureTestFixtureLikeCpp;
