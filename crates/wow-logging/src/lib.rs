//! Structured logging wrapper over the [`tracing`] crate for the WoW server.
//!
//! Provides a [`LogFilter`] enum that mirrors RustyCore's log filter categories,
//! convenience macros that attach the filter as a structured field, and an
//! [`init_logging`] function that wires up `tracing-subscriber` with an
//! env-filter and human-readable formatted output.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use wow_logging::{init_logging, log_server, log_network};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     init_logging("info")?;
//!     log_server!(info, "Server started on port {}", 8085);
//!     log_network!(debug, "Accepted new connection from {}", "127.0.0.1");
//!     Ok(())
//! }
//! ```

// ---------------------------------------------------------------------------
// Re-exports
// ---------------------------------------------------------------------------

/// Re-export the core tracing macros so downstream crates only need to depend
/// on `wow-logging`.
pub use tracing::{debug, error, info, trace, warn};

/// Re-export the tracing `Level` and `Span` types for advanced usage.
pub use tracing::{Level, Span};

/// Re-export `tracing::instrument` for easy span instrumentation on functions.
pub use tracing::instrument;

// ---------------------------------------------------------------------------
// LogFilter
// ---------------------------------------------------------------------------

/// Log filter categories that mirror the RustyCore C# logging subsystem.
///
/// Each variant maps to a logical subsystem of the server. The convenience
/// macros (e.g. [`log_server!`], [`log_database!`]) attach the filter as a
/// structured `log_filter` field so that subscribers can route or suppress
/// messages per category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogFilter {
    /// General server lifecycle events (startup, shutdown, config).
    Server,
    /// Network I/O, socket management, packet framing.
    Network,
    /// Database queries, connections, migrations.
    Database,
    /// Player-specific events (login, logout, actions).
    Player,
    /// Chat messages, channels, whispers.
    Chat,
    /// Spell casting, aura application, spell scripts.
    Spells,
    /// Map loading, instance management, terrain.
    Maps,
    /// Entity creation, updates, destruction (creatures, game objects, items).
    Entities,
    /// Creature AI, SmartAI, waypoint movement.
    AI,
    /// Content scripts (zone scripts, world events, instance scripts).
    Scripts,
    /// GM / console commands.
    Commands,
    /// Arena system.
    Arena,
    /// Battleground system.
    Battleground,
    /// Looking-for-group / dungeon-finder system.
    Lfg,
    /// Miscellaneous events that don't fit another category.
    Misc,
    /// Data loading from database or files at startup.
    Loading,
    /// Guild system.
    Guild,
    /// Achievement system.
    Achievement,
    /// Condition evaluation system.
    Condition,
    /// Vehicle system.
    Vehicle,
    /// Loot generation and distribution.
    Loot,
    /// Movement handling and validation.
    Movement,
}

impl LogFilter {
    /// Returns the lowercase string representation used as the `log_filter`
    /// structured field value in tracing spans/events.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Server => "server",
            Self::Network => "network",
            Self::Database => "database",
            Self::Player => "player",
            Self::Chat => "chat",
            Self::Spells => "spells",
            Self::Maps => "maps",
            Self::Entities => "entities",
            Self::AI => "ai",
            Self::Scripts => "scripts",
            Self::Commands => "commands",
            Self::Arena => "arena",
            Self::Battleground => "battleground",
            Self::Lfg => "lfg",
            Self::Misc => "misc",
            Self::Loading => "loading",
            Self::Guild => "guild",
            Self::Achievement => "achievement",
            Self::Condition => "condition",
            Self::Vehicle => "vehicle",
            Self::Loot => "loot",
            Self::Movement => "movement",
        }
    }
}

impl std::fmt::Display for LogFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Initialize the global tracing subscriber with an env-filter and formatted
/// output.
///
/// The filter is resolved in this order:
/// 1. The `RUST_LOG` environment variable, if set.
/// 2. The provided `log_level` string (e.g. `"info"`, `"debug,wow_network=trace"`).
///
/// The output format includes:
/// - ISO-8601 timestamps
/// - Log level (colored when writing to a terminal)
/// - Target module path
/// - Structured fields (including `log_filter` from the convenience macros)
///
/// # Errors
///
/// Returns an error if the filter directive string is invalid or if a global
/// subscriber has already been set.
///
/// # Examples
///
/// ```rust,no_run
/// wow_logging::init_logging("info").expect("failed to init logging");
/// ```
pub fn init_logging(log_level: &str) -> Result<(), Box<dyn std::error::Error>> {
    use tracing_subscriber::fmt;
    use tracing_subscriber::EnvFilter;

    // Prefer RUST_LOG env var; fall back to the provided level string.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    let subscriber = fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_level(true)
        .with_ansi(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Convenience macros
// ---------------------------------------------------------------------------

/// Internal helper macro used by the public per-filter macros.
///
/// It delegates to the corresponding `tracing` macro while injecting a
/// `log_filter` field.
#[doc(hidden)]
#[macro_export]
macro_rules! __log_with_filter {
    // trace
    (trace, $filter:expr, $($arg:tt)+) => {
        $crate::trace!(log_filter = $filter.as_str(), $($arg)+)
    };
    // debug
    (debug, $filter:expr, $($arg:tt)+) => {
        $crate::debug!(log_filter = $filter.as_str(), $($arg)+)
    };
    // info
    (info, $filter:expr, $($arg:tt)+) => {
        $crate::info!(log_filter = $filter.as_str(), $($arg)+)
    };
    // warn
    (warn, $filter:expr, $($arg:tt)+) => {
        $crate::warn!(log_filter = $filter.as_str(), $($arg)+)
    };
    // error
    (error, $filter:expr, $($arg:tt)+) => {
        $crate::error!(log_filter = $filter.as_str(), $($arg)+)
    };
}

/// Log with the [`LogFilter::Server`] filter.
///
/// ```rust,ignore
/// log_server!(info, "Server started on port {}", port);
/// log_server!(error, "Failed to bind: {}", err);
/// ```
#[macro_export]
macro_rules! log_server {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Server, $($arg)+)
    };
}

/// Log with the [`LogFilter::Network`] filter.
#[macro_export]
macro_rules! log_network {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Network, $($arg)+)
    };
}

/// Log with the [`LogFilter::Database`] filter.
#[macro_export]
macro_rules! log_database {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Database, $($arg)+)
    };
}

/// Log with the [`LogFilter::Player`] filter.
#[macro_export]
macro_rules! log_player {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Player, $($arg)+)
    };
}

/// Log with the [`LogFilter::Chat`] filter.
#[macro_export]
macro_rules! log_chat {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Chat, $($arg)+)
    };
}

/// Log with the [`LogFilter::Spells`] filter.
#[macro_export]
macro_rules! log_spells {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Spells, $($arg)+)
    };
}

/// Log with the [`LogFilter::Maps`] filter.
#[macro_export]
macro_rules! log_maps {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Maps, $($arg)+)
    };
}

/// Log with the [`LogFilter::Entities`] filter.
#[macro_export]
macro_rules! log_entities {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Entities, $($arg)+)
    };
}

/// Log with the [`LogFilter::AI`] filter.
#[macro_export]
macro_rules! log_ai {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::AI, $($arg)+)
    };
}

/// Log with the [`LogFilter::Scripts`] filter.
#[macro_export]
macro_rules! log_scripts {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Scripts, $($arg)+)
    };
}

/// Log with the [`LogFilter::Commands`] filter.
#[macro_export]
macro_rules! log_commands {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Commands, $($arg)+)
    };
}

/// Log with the [`LogFilter::Arena`] filter.
#[macro_export]
macro_rules! log_arena {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Arena, $($arg)+)
    };
}

/// Log with the [`LogFilter::Battleground`] filter.
#[macro_export]
macro_rules! log_battleground {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Battleground, $($arg)+)
    };
}

/// Log with the [`LogFilter::Lfg`] filter.
#[macro_export]
macro_rules! log_lfg {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Lfg, $($arg)+)
    };
}

/// Log with the [`LogFilter::Misc`] filter.
#[macro_export]
macro_rules! log_misc {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Misc, $($arg)+)
    };
}

/// Log with the [`LogFilter::Loading`] filter.
#[macro_export]
macro_rules! log_loading {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Loading, $($arg)+)
    };
}

/// Log with the [`LogFilter::Guild`] filter.
#[macro_export]
macro_rules! log_guild {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Guild, $($arg)+)
    };
}

/// Log with the [`LogFilter::Achievement`] filter.
#[macro_export]
macro_rules! log_achievement {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Achievement, $($arg)+)
    };
}

/// Log with the [`LogFilter::Condition`] filter.
#[macro_export]
macro_rules! log_condition {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Condition, $($arg)+)
    };
}

/// Log with the [`LogFilter::Vehicle`] filter.
#[macro_export]
macro_rules! log_vehicle {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Vehicle, $($arg)+)
    };
}

/// Log with the [`LogFilter::Loot`] filter.
#[macro_export]
macro_rules! log_loot {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Loot, $($arg)+)
    };
}

/// Log with the [`LogFilter::Movement`] filter.
#[macro_export]
macro_rules! log_movement {
    ($level:ident, $($arg:tt)+) => {
        $crate::__log_with_filter!($level, $crate::LogFilter::Movement, $($arg)+)
    };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_filter_as_str_round_trip() {
        assert_eq!(LogFilter::Server.as_str(), "server");
        assert_eq!(LogFilter::Network.as_str(), "network");
        assert_eq!(LogFilter::Database.as_str(), "database");
        assert_eq!(LogFilter::Player.as_str(), "player");
        assert_eq!(LogFilter::Chat.as_str(), "chat");
        assert_eq!(LogFilter::Spells.as_str(), "spells");
        assert_eq!(LogFilter::Maps.as_str(), "maps");
        assert_eq!(LogFilter::Entities.as_str(), "entities");
        assert_eq!(LogFilter::AI.as_str(), "ai");
        assert_eq!(LogFilter::Scripts.as_str(), "scripts");
        assert_eq!(LogFilter::Commands.as_str(), "commands");
        assert_eq!(LogFilter::Arena.as_str(), "arena");
        assert_eq!(LogFilter::Battleground.as_str(), "battleground");
        assert_eq!(LogFilter::Lfg.as_str(), "lfg");
        assert_eq!(LogFilter::Misc.as_str(), "misc");
        assert_eq!(LogFilter::Loading.as_str(), "loading");
        assert_eq!(LogFilter::Guild.as_str(), "guild");
        assert_eq!(LogFilter::Achievement.as_str(), "achievement");
        assert_eq!(LogFilter::Condition.as_str(), "condition");
        assert_eq!(LogFilter::Vehicle.as_str(), "vehicle");
        assert_eq!(LogFilter::Loot.as_str(), "loot");
        assert_eq!(LogFilter::Movement.as_str(), "movement");
    }

    #[test]
    fn log_filter_display() {
        assert_eq!(format!("{}", LogFilter::Battleground), "battleground");
        assert_eq!(format!("{}", LogFilter::AI), "ai");
    }

    #[test]
    fn convenience_macros_compile() {
        // We cannot easily assert on tracing output without a custom
        // subscriber, but we CAN verify the macros expand without errors.
        // These will be no-ops because no subscriber is installed.
        log_server!(info, "startup complete");
        log_network!(debug, "bytes_read = {}", 1024);
        log_database!(warn, "slow query detected");
        log_player!(trace, "player {} moved", "Alice");
        log_chat!(info, "channel message");
        log_spells!(debug, "casting spell {}", 133);
        log_maps!(info, "loaded map {}", 0);
        log_entities!(debug, "spawned creature");
        log_ai!(trace, "evaluating AI state");
        log_scripts!(info, "script loaded");
        log_commands!(warn, "unknown command");
        log_arena!(info, "arena match started");
        log_battleground!(info, "BG queue pop");
        log_lfg!(debug, "LFG proposal");
        log_misc!(info, "misc event");
        log_loading!(info, "loading data");
        log_guild!(info, "guild created");
        log_achievement!(info, "achievement earned");
        log_condition!(debug, "condition check");
        log_vehicle!(debug, "vehicle entered");
        log_loot!(info, "loot rolled");
        log_movement!(trace, "position update");
    }
}
