//! Async MySQL database layer for RustyCore.
//!
//! Provides type-safe database access with prepared statements, matching the
//! C# `MySqlBase<T>` / `PreparedStatement` / `SQLResult` pattern from
//! TrinityCore/RustyCore.
//!
//! # Type Safety
//!
//! Each database connection is parameterized by a statement enum type. This
//! makes it a **compile-time error** to use the wrong statement type on the
//! wrong database:
//!
//! ```ignore
//! use wow_database::*;
//!
//! let login_db: Database<LoginStatements> = Database::open("mysql://...").await?;
//! let world_db: Database<WorldStatements> = Database::open("mysql://...").await?;
//!
//! // This compiles:
//! let mut stmt = login_db.prepare(LoginStatements::SEL_REALMLIST);
//! let result = login_db.query(&stmt).await?;
//!
//! // This would NOT compile:
//! // let stmt = login_db.prepare(WorldStatements::SEL_COMMANDS); // ERROR!
//! ```
//!
//! # Architecture
//!
//! - [`Database<S>`]: Connection pool wrapper, parameterized by statement type
//! - [`PreparedStatement`]: SQL + dynamic parameters (set via `set_u32`, `set_string`, etc.)
//! - [`SqlResult`]: Query result with cursor-style row iteration
//! - [`SqlFields`]: Borrowed view of a single row
//! - [`SqlTransaction`]: Batch of statements executed atomically
//! - Statement enums: [`LoginStatements`], [`WorldStatements`], [`CharStatements`], [`HotfixStatements`]

pub mod database;
pub mod error;
pub mod params;
pub mod result;
pub mod statements;
pub mod transaction;
pub mod updater;

// Re-export primary types at crate root for convenience.
pub use database::{Database, build_connection_string};
pub use error::DatabaseError;
pub use params::{PreparedStatement, SqlParam};
pub use result::{SqlFields, SqlResult};
pub use statements::{
    CharStatements, HotfixStatements, LoginStatements, StatementDef, WorldStatements,
};
pub use transaction::SqlTransaction;

/// Type aliases for each database connection.
pub type LoginDatabase = Database<LoginStatements>;
pub type WorldDatabase = Database<WorldStatements>;
pub type CharacterDatabase = Database<CharStatements>;
pub type HotfixDatabase = Database<HotfixStatements>;
