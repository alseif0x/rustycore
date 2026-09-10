//! Fail-closed provenance for capture imports.
//!
//! Capture wrappers publish a raw artifact and a side-specific manifest only
//! after the accredited process is gone and the runtime has been restored.
//! Import validates those manifests against the raw bytes, keeps exact copies
//! beside the committed fixture, and publishes the complete derived flow with
//! one atomic directory exchange. [`verify_required_lineage`] then binds the
//! copied raw manifests to every filtered output used by `verify-required`.

use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{Capture, Direction, PacketBoundary};
use crate::semantic::{
    CorrelatedSpellGuidBody, ISSUE_24_PING_FENCE_SERIAL, SMSG_SPELL_GO, SMSG_SPELL_START,
    decode_spell_go_body, decode_spell_start_body,
};

mod state_1;
mod state_2;
mod state_3;
mod state_4;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;
#[allow(unused_imports)]
pub use state_3::*;
#[allow(unused_imports)]
pub use state_4::*;

#[cfg(test)]
#[path = "lineage_tests/mod.rs"]
mod tests;
