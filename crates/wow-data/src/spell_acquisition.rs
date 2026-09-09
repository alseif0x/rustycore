//! Compact effective metadata used to plan player spell acquisition.
//!
//! This is intentionally not another general `SpellInfo` store.  It composes
//! the seven DB2 families needed by acquisition in the same order as
//! `DB2StorageBase::LoadFromDB` and `DB2Manager::LoadHotfixData`:
//!
//! ```text
//! WDC4 -> official SQL -> custom SQL -> final RecordRemoved
//! ```
//!
//! Overlay payload is kept raw until composition is complete.  Consequently
//! an invalid official/custom row still replaces the older row with the same
//! record id and fails closed; it can only be repaired by a later overlay.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use anyhow::{Context, Result};

use crate::{Db2HotfixRemovalStoreLikeCpp, wdc4::Wdc4Reader};

mod state_1;
mod state_3;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
#[allow(unused_imports)]
pub use state_3::*;

mod state_2_ops_1;
mod state_2_ops_2;
#[cfg(test)]
#[path = "spell_acquisition_tests.rs"]
mod tests;
