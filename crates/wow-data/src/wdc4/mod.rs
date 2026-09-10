// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Generic WDC4 (DB2) file parser.
//!
//! Supports the six compression types used in WoW 3.4.3 client data files:
//! None, Immediate (Bitpacked), SignedImmediate, Pallet, PalletArray, Common.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use tracing::{debug, trace};

// ── Constants ────────────────────────────────────────────────────────

mod bits;
mod format;
mod reader;

pub use reader::*;

use bits::{
    merge_relationship_data, read_bits, read_inline_record_id, read_u16_le, read_u32_le,
    read_u64_le, sign_extend, split_common_data, split_pallet_data,
};

use format::{
    CompressionType, FIELD_META_SIZE, FIELD_STORAGE_INFO_SIZE, FieldStorageInfo, HEADER_SIZE,
    SECTION_HEADER_SIZE, Wdc4Header, parse_field_storage_info, parse_header, parse_section_header,
};

// ── Tests ────────────────────────────────────────────────────────────
#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
