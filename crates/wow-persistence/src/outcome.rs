//! Shared future, database-affinity and classified persistence-result vocabulary.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use std::future::Future;
use std::pin::Pin;

/// A future returned by a port method.
pub type PersistenceFutureLikeCpp<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The logical databases the lifecycle can address. Deliberately not a
/// connection, pool or URL — only which store a request belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalDatabaseLikeCpp {
    Characters,
    Login,
    World,
}

/// The normalized result of one lifecycle write.
///
/// `Unknown` is not a failure and not a success. The frozen contract requires
/// that an indeterminate outcome fences further mutation instead of being
/// collapsed into either, so it stays a distinct variant here rather than
/// being flattened into `Result`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceOutcomeLikeCpp {
    /// The write is durable. `rows` is what the adapter reported.
    Applied { rows: u64 },
    /// The write definitely did not apply; runtime state is unchanged.
    Failed { reason: String },
    /// The outcome could not be determined. The caller must fence.
    Unknown { reason: String },
}

impl PersistenceOutcomeLikeCpp {
    pub fn is_applied(&self) -> bool {
        matches!(self, Self::Applied { .. })
    }

    /// True when the caller may not assume either outcome and must fence.
    pub fn is_indeterminate(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }
}
