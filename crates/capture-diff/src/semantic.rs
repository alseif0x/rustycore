//! Narrow semantic packet comparators for fields that are intentionally
//! runtime-allocated and therefore cannot be compared byte-for-byte.
//!
//! Keep this module deliberately small. A semantic comparator is allowed to
//! omit only a field whose value cannot be made stable across equivalent C++
//! and Rust runs; every other decoded bit remains part of the comparison.

use serde::{Deserialize, Serialize};

use crate::model::Direction;

/// `SMSG_LOG_XP_GAIN` in the 3.4.3 opcode table.
pub const SMSG_LOG_XP_GAIN: u16 = 0x26E5;

const OBJECT_GUID_COUNTER_MASK: u64 = 0x0000_00FF_FFFF_FFFF;
const HIGH_GUID_CREATURE: u8 = 8;
const XP_GAIN_REASON_KILL: u8 = 0;

/// Stable identity fields of the victim ObjectGuid in `SMSG_LOG_XP_GAIN`.
///
/// TrinityCore creates world-object GUIDs as:
///
/// - high word: high type, realm, map, entry, subtype;
/// - low word: server id (upper 24 bits), runtime counter (lower 40 bits).
///
/// The runtime counter is deliberately absent. All other 88 GUID bits are
/// decoded and compared, including subtype and server id in addition to the
/// explicitly gameplay-relevant type/realm/map/entry fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StableObjectGuid {
    pub high_type: u8,
    pub realm_id: u16,
    pub map_id: u16,
    pub entry: u32,
    pub subtype: u8,
    pub server_id: u32,
}

/// Stable semantic representation of a 3.4.3 `SMSG_LOG_XP_GAIN` body.
///
/// `group_bonus_bits` intentionally compares the exact IEEE-754 wire bits,
/// rather than applying an epsilon that could hide a protocol divergence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogXpGainBody {
    pub victim: StableObjectGuid,
    pub original: i32,
    pub reason: u8,
    pub amount: i32,
    pub group_bonus_bits: u32,
}

/// One side of a semantic comparison. A malformed body remains an explicit
/// divergence even when both sides happen to contain the same malformed bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticBodySide {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub log_xp_gain: Option<LogXpGainBody>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub decode_error: Option<String>,
}

impl SemanticBodySide {
    fn from_decoded(decoded: Result<DecodedLogXpGainBody, String>) -> Self {
        match decoded {
            Ok(decoded)
                if decoded.body.reason == XP_GAIN_REASON_KILL
                    && decoded.body.victim.high_type == HIGH_GUID_CREATURE
                    && decoded.runtime_counter == 0 =>
            {
                Self {
                    log_xp_gain: None,
                    decode_error: Some(
                        "kill XP creature victim has a zero runtime GUID counter".to_string(),
                    ),
                }
            }
            Ok(decoded) => Self {
                log_xp_gain: Some(decoded.body),
                decode_error: None,
            },
            Err(error) => Self {
                log_xp_gain: None,
                decode_error: Some(error),
            },
        }
    }
}

/// Detailed semantic comparison attached to a regular body diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticBodyDiff {
    pub comparator: String,
    pub cpp: SemanticBodySide,
    pub rust: SemanticBodySide,
}

impl SemanticBodyDiff {
    /// True only when both bodies decoded successfully and all stable fields
    /// are equal. Decode failures are never accepted as a clean comparison.
    #[must_use]
    pub fn is_identical(&self) -> bool {
        self.cpp.decode_error.is_none()
            && self.rust.decode_error.is_none()
            && self.cpp.log_xp_gain == self.rust.log_xp_gain
    }

    /// Concise field-level explanation for terminal reports.
    #[must_use]
    pub fn mismatch_summary(&self) -> String {
        if let Some(error) = &self.cpp.decode_error {
            return format!("C++ decode error: {error}");
        }
        if let Some(error) = &self.rust.decode_error {
            return format!("Rust decode error: {error}");
        }

        let Some(cpp) = self.cpp.log_xp_gain else {
            return "C++ semantic body is missing".to_string();
        };
        let Some(rust) = self.rust.log_xp_gain else {
            return "Rust semantic body is missing".to_string();
        };

        let mut fields = Vec::new();
        if cpp.victim.high_type != rust.victim.high_type {
            fields.push("victim.high_type");
        }
        if cpp.victim.realm_id != rust.victim.realm_id {
            fields.push("victim.realm_id");
        }
        if cpp.victim.map_id != rust.victim.map_id {
            fields.push("victim.map_id");
        }
        if cpp.victim.entry != rust.victim.entry {
            fields.push("victim.entry");
        }
        if cpp.victim.subtype != rust.victim.subtype {
            fields.push("victim.subtype");
        }
        if cpp.victim.server_id != rust.victim.server_id {
            fields.push("victim.server_id");
        }
        if cpp.original != rust.original {
            fields.push("original");
        }
        if cpp.reason != rust.reason {
            fields.push("reason");
        }
        if cpp.amount != rust.amount {
            fields.push("amount");
        }
        if cpp.group_bonus_bits != rust.group_bonus_bits {
            fields.push("group_bonus");
        }

        if fields.is_empty() {
            "semantic values are equal".to_string()
        } else {
            format!("mismatched field(s): {}", fields.join(", "))
        }
    }
}

/// Compare packet bodies semantically when a reviewed narrow comparator exists.
///
/// Routing is not normalized here: [`crate::diff::DiffReport`] still compares
/// `connection_id`, so `SMSG_LOG_XP_GAIN` must use the realm socket like C++.
#[must_use]
pub fn compare_packet_bodies(
    direction: Direction,
    opcode: u16,
    cpp: &[u8],
    rust: &[u8],
) -> Option<SemanticBodyDiff> {
    if direction != Direction::S2C || opcode != SMSG_LOG_XP_GAIN {
        return None;
    }

    let cpp_decoded = decode_log_xp_gain_body_with_counter(cpp);
    let rust_decoded = decode_log_xp_gain_body_with_counter(rust);

    // Normalize runtime counters only for the reviewed rested-XP shape: kill
    // XP whose victim is a real Creature GUID. For every other valid XP shape
    // (for example NO_KILL with ObjectGuid::Empty), retain raw byte comparison.
    let cpp_is_creature_kill = cpp_decoded
        .as_ref()
        .is_ok_and(DecodedLogXpGainBody::is_creature_kill);
    let rust_is_creature_kill = rust_decoded
        .as_ref()
        .is_ok_and(DecodedLogXpGainBody::is_creature_kill);
    if cpp_decoded.is_ok()
        && rust_decoded.is_ok()
        && !cpp_is_creature_kill
        && !rust_is_creature_kill
    {
        return None;
    }

    Some(SemanticBodyDiff {
        comparator: "smsg_log_xp_gain_without_runtime_guid_counter".to_string(),
        cpp: SemanticBodySide::from_decoded(cpp_decoded),
        rust: SemanticBodySide::from_decoded(rust_decoded),
    })
}

#[derive(Debug, Clone, Copy)]
struct DecodedLogXpGainBody {
    body: LogXpGainBody,
    runtime_counter: u64,
}

impl DecodedLogXpGainBody {
    fn is_creature_kill(&self) -> bool {
        self.body.reason == XP_GAIN_REASON_KILL && self.body.victim.high_type == HIGH_GUID_CREATURE
    }
}

/// Decode the opcode-less body emitted by C++
/// `WorldPackets::Character::LogXPGain::Write`.
///
/// Source anchors (identical in both legacy references):
///
/// - `CharacterPackets.cpp`: packed Victim, Original, Reason, Amount,
///   GroupBonus in that order;
/// - `ObjectGuid.cpp::operator<<`: low mask, high mask, packed low bytes,
///   packed high bytes;
/// - `ObjectGuidFactory::CreateWorldObject`: lower 40 low-word bits are the
///   runtime counter.
pub fn decode_log_xp_gain_body(body: &[u8]) -> Result<LogXpGainBody, String> {
    decode_log_xp_gain_body_with_counter(body).map(|decoded| decoded.body)
}

fn decode_log_xp_gain_body_with_counter(body: &[u8]) -> Result<DecodedLogXpGainBody, String> {
    let mut cursor = 0usize;
    let low_mask = read_u8(body, &mut cursor, "Victim low mask")?;
    let high_mask = read_u8(body, &mut cursor, "Victim high mask")?;
    let low = read_packed_u64(body, &mut cursor, low_mask, "Victim low word")?;
    let high = read_packed_u64(body, &mut cursor, high_mask, "Victim high word")?;

    let original = read_i32(body, &mut cursor, "Original")?;
    let reason = read_u8(body, &mut cursor, "Reason")?;
    let amount = read_i32(body, &mut cursor, "Amount")?;
    let group_bonus_bits = read_u32(body, &mut cursor, "GroupBonus")?;
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after GroupBonus: decoded {cursor} of {} bytes",
            body.len()
        ));
    }

    // Counter is the only intentionally normalized value. Retain the upper
    // 24 server-id bits from the low word and every bit of the high word.
    let stable_low = low & !OBJECT_GUID_COUNTER_MASK;
    Ok(DecodedLogXpGainBody {
        body: LogXpGainBody {
            victim: StableObjectGuid {
                high_type: ((high >> 58) & 0x3F) as u8,
                realm_id: ((high >> 42) & 0xFFFF) as u16,
                map_id: ((high >> 29) & 0x1FFF) as u16,
                entry: ((high >> 6) & 0x7F_FFFF) as u32,
                subtype: (high & 0x3F) as u8,
                server_id: ((stable_low >> 40) & 0xFF_FFFF) as u32,
            },
            original,
            reason,
            amount,
            group_bonus_bits,
        },
        runtime_counter: low & OBJECT_GUID_COUNTER_MASK,
    })
}

fn read_packed_u64(body: &[u8], cursor: &mut usize, mask: u8, field: &str) -> Result<u64, String> {
    let mut bytes = [0u8; 8];
    for (index, byte) in bytes.iter_mut().enumerate() {
        if mask & (1 << index) != 0 {
            *byte = read_u8(body, cursor, field)?;
            if *byte == 0 {
                return Err(format!(
                    "{field} uses non-canonical packed encoding at byte {index}"
                ));
            }
        }
    }
    Ok(u64::from_le_bytes(bytes))
}

fn read_u8(body: &[u8], cursor: &mut usize, field: &str) -> Result<u8, String> {
    let Some(value) = body.get(*cursor).copied() else {
        return Err(format!("truncated while reading {field} at byte {cursor}"));
    };
    *cursor += 1;
    Ok(value)
}

fn read_i32(body: &[u8], cursor: &mut usize, field: &str) -> Result<i32, String> {
    Ok(i32::from_le_bytes(read_array(body, cursor, field)?))
}

fn read_u32(body: &[u8], cursor: &mut usize, field: &str) -> Result<u32, String> {
    Ok(u32::from_le_bytes(read_array(body, cursor, field)?))
}

fn read_array<const N: usize>(
    body: &[u8],
    cursor: &mut usize,
    field: &str,
) -> Result<[u8; N], String> {
    let end = cursor
        .checked_add(N)
        .ok_or_else(|| format!("offset overflow while reading {field}"))?;
    let bytes = body
        .get(*cursor..end)
        .ok_or_else(|| format!("truncated while reading {field} at byte {cursor}"))?;
    *cursor = end;
    bytes
        .try_into()
        .map_err(|_| format!("invalid width while reading {field}"))
}
