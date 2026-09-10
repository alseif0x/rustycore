//! WDC4 reader regressions.
//!
//! Separated from wdc4.rs under #685.

use super::*;

// Integration test: parse real Item.db2 if available

/// Diagnostic test: probe ItemSparse.db2 field layout to find stat modifier fields.
mod scenarios;
