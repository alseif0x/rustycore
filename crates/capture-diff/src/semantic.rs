//! Narrow semantic packet comparators for fields that are intentionally
//! runtime-allocated, or for a specifically proven accumulated update-mask
//! artifact, and therefore cannot be compared byte-for-byte.
//!
//! Keep this module deliberately small. A semantic comparator is allowed to
//! omit only a field whose value cannot be made stable across equivalent C++
//! and Rust runs, or one exact empty mask fragment whose cadence was reproduced
//! independently; every other decoded bit remains part of the comparison.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{Capture, Direction};

/// `CMSG_LOOT_ITEM` in the 3.4.3 opcode table.
pub const CMSG_LOOT_ITEM: u16 = 0x3211;

/// `SMSG_LOG_XP_GAIN` in the 3.4.3 opcode table.
pub const SMSG_LOG_XP_GAIN: u16 = 0x26E5;

/// `SMSG_LOOT_REMOVED` in the 3.4.3 opcode table.
pub const SMSG_LOOT_REMOVED: u16 = 0x2615;

/// `SMSG_ITEM_PUSH_RESULT` in the 3.4.3 opcode table.
pub const SMSG_ITEM_PUSH_RESULT: u16 = 0x2623;

/// `SMSG_BUY_SUCCEEDED` in the 3.4.3 opcode table.
pub const SMSG_BUY_SUCCEEDED: u16 = 0x26C6;

/// `SMSG_UPDATE_OBJECT` in the 3.4.3 opcode table.
pub const SMSG_UPDATE_OBJECT: u16 = 0x27CB;

/// `CMSG_PING` used as the deterministic end fence of the issue-#106 flow.
pub const CMSG_PING: u16 = 0x3768;

const OBJECT_GUID_COUNTER_MASK: u64 = 0x0000_00FF_FFFF_FFFF;
const HIGH_GUID_CREATURE: u8 = 8;
const HIGH_GUID_ITEM: u8 = 3;
const HIGH_GUID_LOOT_OBJECT: u8 = 15;
const HIGH_GUID_PLAYER: u8 = 2;
const GLOBAL_GUID_RESERVED_HIGH_BITS_MASK: u64 = (1_u64 << 42) - 1;
const XP_GAIN_REASON_KILL: u8 = 0;
const VALUES_TYPE_UNIT: u32 = 1 << 5;
const VALUES_TYPE_ACTIVE_PLAYER: u32 = 1 << 7;
const UNIT_POWER_PARENT_BLOCKS_MASK: u8 = 1 << 3;
const UNIT_POWER_PARENT_BLOCK_3: u32 = 1 << 20;
const ISSUE_106_CAPTURE_PLAYER_LOW: u64 = 15;
const ISSUE_106_CAPTURE_PLAYER_HIGH: u64 = 0x0800_0400_0000_0000;
const ISSUE_106_CAPTURE_ITEM_HIGH: u64 = 0x0C00_0400_0000_0000;
const ISSUE_106_CREATURE_IDENTITY: StableObjectGuid = StableObjectGuid {
    high_type: HIGH_GUID_CREATURE,
    realm_id: 1,
    map_id: 530,
    entry: 21_779,
    subtype: 0,
    server_id: 0,
};
const ISSUE_106_ITEM_ENTRY: i32 = 30_712;
// Doctor Maleficus' key is stored in the first keyring slot in the restored
// issue-#106 fixture. Pinning the observed C++ destination keeps a bank,
// equipment, or generic inventory update from satisfying this capture.
const ISSUE_106_ITEM_SLOT: i32 = 106;
const ISSUE_106_ITEM_DYNAMIC_FLAGS: u32 = 0x0020_0001;
const ISSUE_106_ITEM_CREATE_ZERO_TAIL_LEN: usize = 220;
const ISSUE_106_PING_BODY: [u8; 8] = [b'T', b'O', b'O', b'L', 0, 0, 0, 0];
const ISSUE_108_VENDOR_IDENTITY: StableObjectGuid = StableObjectGuid {
    high_type: HIGH_GUID_CREATURE,
    realm_id: 1,
    map_id: 530,
    entry: 18_525,
    subtype: 0,
    server_id: 0,
};
const ISSUE_108_VENDOR_MUID: u32 = 59;
const ISSUE_108_VENDOR_NEW_QUANTITY: i32 = -1;
const ISSUE_108_VENDOR_QUANTITY_BOUGHT: u32 = 1;

/// Stable identity fields of a world-object `ObjectGuid` whose map-runtime
/// counter has one narrowly reviewed normalization.
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

/// Exact identity of a packed 128-bit `ObjectGuid` whose runtime component is
/// part of the reviewed wire contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactObjectGuid {
    pub low: u64,
    pub high: u64,
}

/// Stable semantic representation of a 3.4.3 `SMSG_LOOT_REMOVED` body.
///
/// Only the lower 40-bit map-runtime counter of a Creature `owner` is absent.
/// The complete loot-object GUID (including its own counter) and list id remain
/// exact so this comparator cannot hide allocation or slot divergences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LootRemovedBody {
    pub owner: StableObjectGuid,
    pub loot_obj: ExactObjectGuid,
    pub loot_list_id: u8,
}

/// Stable semantic representation of the issue-#108 vendor success ACK.
///
/// Only the lower 40-bit map-runtime counter of the reviewed G'eras Creature
/// GUID is absent. The response fields remain exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuySucceededBody {
    pub vendor: StableObjectGuid,
    pub muid: u32,
    pub new_quantity: i32,
    pub quantity_bought: u32,
}

/// One exact `ActivePlayerData::InvSlots` value in a normalized player VALUES
/// update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvSlotValue {
    pub slot: u16,
    pub item: ExactObjectGuid,
}

/// Stable representation of the one-player, one-block VALUES update observed
/// after the issue-#106 item claim.
///
/// C++ carried an empty UnitData parent bit 116 while Rust did not. That one
/// cadence-only mask is absent here; map, player and the complete ActivePlayer
/// InvSlots update remain exact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateObjectInvSlotsBody {
    pub map_id: u16,
    pub player: ExactObjectGuid,
    pub inv_slots: Vec<InvSlotValue>,
}

/// One side of a semantic comparison. A malformed body remains an explicit
/// divergence even when both sides happen to contain the same malformed bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticBodySide {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub log_xp_gain: Option<LogXpGainBody>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub loot_removed: Option<LootRemovedBody>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub buy_succeeded: Option<BuySucceededBody>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub update_object_inv_slots: Option<UpdateObjectInvSlotsBody>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub decode_error: Option<String>,
    /// Strict identity for a raw body that is not eligible for runtime-counter
    /// normalization. Only a valid reviewed Creature shape with a nonzero
    /// counter omits this digest; invalid and valid non-eligible sides retain
    /// it.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub raw_body_sha256: Option<String>,
}

impl SemanticBodySide {
    fn from_decoded_log_xp_gain(
        decoded: Result<DecodedLogXpGainBody, String>,
        raw_body: &[u8],
    ) -> Self {
        match decoded {
            Ok(decoded)
                if decoded.body.reason == XP_GAIN_REASON_KILL
                    && decoded.body.victim.high_type == HIGH_GUID_CREATURE
                    && decoded.runtime_counter == 0 =>
            {
                Self {
                    log_xp_gain: Some(decoded.body),
                    loot_removed: None,
                    buy_succeeded: None,
                    update_object_inv_slots: None,
                    decode_error: Some(
                        "kill XP creature victim has a zero runtime GUID counter".to_string(),
                    ),
                    raw_body_sha256: Some(raw_body_sha256(raw_body)),
                }
            }
            Ok(decoded) => Self {
                log_xp_gain: Some(decoded.body),
                loot_removed: None,
                buy_succeeded: None,
                update_object_inv_slots: None,
                decode_error: None,
                raw_body_sha256: (!decoded.is_creature_kill()).then(|| raw_body_sha256(raw_body)),
            },
            Err(error) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                update_object_inv_slots: None,
                decode_error: Some(error),
                raw_body_sha256: Some(raw_body_sha256(raw_body)),
            },
        }
    }

    fn from_decoded_loot_removed(
        decoded: Result<DecodedLootRemovedBody, String>,
        raw_body: &[u8],
    ) -> Self {
        match decoded {
            Ok(decoded) => {
                let shape_error = decoded.issue_106_shape_error();
                let reviewed_shape = decoded.is_issue_106_reviewed_shape();
                Self {
                    log_xp_gain: None,
                    loot_removed: Some(decoded.body),
                    buy_succeeded: None,
                    update_object_inv_slots: None,
                    decode_error: shape_error,
                    // Only the exact issue-#106 Doctor/LootObject/list shape may
                    // omit the Creature runtime counter. A different Creature
                    // remains byte-strict even if its stable high fields happen
                    // to match another packet.
                    raw_body_sha256: (!reviewed_shape).then(|| raw_body_sha256(raw_body)),
                }
            }
            Err(error) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                update_object_inv_slots: None,
                decode_error: Some(error),
                raw_body_sha256: Some(raw_body_sha256(raw_body)),
            },
        }
    }

    fn from_decoded_buy_succeeded(
        decoded: Result<DecodedBuySucceededBody, String>,
        raw_body: &[u8],
    ) -> Self {
        match decoded {
            Ok(decoded) => {
                let shape_error = decoded.issue_108_shape_error();
                let reviewed_shape = decoded.is_issue_108_reviewed_shape();
                Self {
                    log_xp_gain: None,
                    loot_removed: None,
                    buy_succeeded: Some(decoded.body),
                    update_object_inv_slots: None,
                    decode_error: shape_error,
                    raw_body_sha256: (!reviewed_shape).then(|| raw_body_sha256(raw_body)),
                }
            }
            Err(error) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                update_object_inv_slots: None,
                decode_error: Some(error),
                raw_body_sha256: Some(raw_body_sha256(raw_body)),
            },
        }
    }

    fn from_update_object_inv_slots_decode(
        decoded: UpdateObjectInvSlotsDecode,
        raw_body: &[u8],
    ) -> Self {
        match decoded {
            UpdateObjectInvSlotsDecode::Candidate(decoded) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                update_object_inv_slots: Some(decoded.body),
                decode_error: None,
                raw_body_sha256: None,
            },
            UpdateObjectInvSlotsDecode::NotEligible(reason) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                update_object_inv_slots: None,
                decode_error: Some(format!(
                    "not the reviewed single-player InvSlots VALUES shape: {reason}"
                )),
                raw_body_sha256: Some(raw_body_sha256(raw_body)),
            },
            UpdateObjectInvSlotsDecode::Malformed(error) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                update_object_inv_slots: None,
                decode_error: Some(error),
                raw_body_sha256: Some(raw_body_sha256(raw_body)),
            },
        }
    }
}

fn raw_body_sha256(body: &[u8]) -> String {
    format!("{:x}", Sha256::digest(body))
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
            && self.cpp.loot_removed == self.rust.loot_removed
            && self.cpp.buy_succeeded == self.rust.buy_succeeded
            && self.cpp.update_object_inv_slots == self.rust.update_object_inv_slots
            && self.cpp.raw_body_sha256 == self.rust.raw_body_sha256
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

        if let (Some(cpp), Some(rust)) = (self.cpp.log_xp_gain, self.rust.log_xp_gain) {
            return mismatch_log_xp_gain(cpp, rust);
        }

        if let (Some(cpp), Some(rust)) = (self.cpp.loot_removed, self.rust.loot_removed) {
            return mismatch_loot_removed(cpp, rust, &self.cpp, &self.rust);
        }

        if let (Some(cpp), Some(rust)) = (self.cpp.buy_succeeded, self.rust.buy_succeeded) {
            return mismatch_buy_succeeded(cpp, rust, &self.cpp, &self.rust);
        }

        if let (Some(cpp), Some(rust)) = (
            self.cpp.update_object_inv_slots.as_ref(),
            self.rust.update_object_inv_slots.as_ref(),
        ) {
            return mismatch_update_object_inv_slots(cpp, rust);
        }

        "semantic body shape differs or is missing".to_string()
    }
}

fn mismatch_log_xp_gain(cpp: LogXpGainBody, rust: LogXpGainBody) -> String {
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

fn mismatch_loot_removed(
    cpp: LootRemovedBody,
    rust: LootRemovedBody,
    cpp_side: &SemanticBodySide,
    rust_side: &SemanticBodySide,
) -> String {
    let mut fields = Vec::new();
    if cpp.owner.high_type != rust.owner.high_type {
        fields.push("owner.high_type");
    }
    if cpp.owner.realm_id != rust.owner.realm_id {
        fields.push("owner.realm_id");
    }
    if cpp.owner.map_id != rust.owner.map_id {
        fields.push("owner.map_id");
    }
    if cpp.owner.entry != rust.owner.entry {
        fields.push("owner.entry");
    }
    if cpp.owner.subtype != rust.owner.subtype {
        fields.push("owner.subtype");
    }
    if cpp.owner.server_id != rust.owner.server_id {
        fields.push("owner.server_id");
    }
    if cpp.loot_obj.low != rust.loot_obj.low {
        fields.push("loot_obj.low");
    }
    if cpp.loot_obj.high != rust.loot_obj.high {
        fields.push("loot_obj.high");
    }
    if cpp.loot_list_id != rust.loot_list_id {
        fields.push("loot_list_id");
    }

    if fields.is_empty() {
        if cpp_side.raw_body_sha256 == rust_side.raw_body_sha256 {
            "semantic values are equal".to_string()
        } else {
            "raw body identity differs outside the normalized Creature-owner shape".to_string()
        }
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
    }
}

fn mismatch_update_object_inv_slots(
    cpp: &UpdateObjectInvSlotsBody,
    rust: &UpdateObjectInvSlotsBody,
) -> String {
    let mut fields = Vec::new();
    if cpp.map_id != rust.map_id {
        fields.push("map_id");
    }
    if cpp.player.low != rust.player.low || cpp.player.high != rust.player.high {
        fields.push("player");
    }
    if cpp.inv_slots.len() == rust.inv_slots.len() {
        for (cpp_slot, rust_slot) in cpp.inv_slots.iter().zip(&rust.inv_slots) {
            if cpp_slot.slot != rust_slot.slot {
                fields.push("inv_slots.slot");
            }
            if cpp_slot.item != rust_slot.item {
                fields.push("inv_slots.item");
            }
        }
    } else {
        fields.push("inv_slots.length");
    }

    fields.sort_unstable();
    fields.dedup();
    if fields.is_empty() {
        "semantic values are equal".to_string()
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
    }
}

fn mismatch_buy_succeeded(
    cpp: BuySucceededBody,
    rust: BuySucceededBody,
    cpp_side: &SemanticBodySide,
    rust_side: &SemanticBodySide,
) -> String {
    let mut fields = Vec::new();
    if cpp.vendor.high_type != rust.vendor.high_type {
        fields.push("vendor.high_type");
    }
    if cpp.vendor.realm_id != rust.vendor.realm_id {
        fields.push("vendor.realm_id");
    }
    if cpp.vendor.map_id != rust.vendor.map_id {
        fields.push("vendor.map_id");
    }
    if cpp.vendor.entry != rust.vendor.entry {
        fields.push("vendor.entry");
    }
    if cpp.vendor.subtype != rust.vendor.subtype {
        fields.push("vendor.subtype");
    }
    if cpp.vendor.server_id != rust.vendor.server_id {
        fields.push("vendor.server_id");
    }
    if cpp.muid != rust.muid {
        fields.push("muid");
    }
    if cpp.new_quantity != rust.new_quantity {
        fields.push("new_quantity");
    }
    if cpp.quantity_bought != rust.quantity_bought {
        fields.push("quantity_bought");
    }

    if fields.is_empty() {
        if cpp_side.raw_body_sha256 == rust_side.raw_body_sha256 {
            "semantic values are equal".to_string()
        } else {
            "raw body identity differs outside the reviewed vendor-success shape".to_string()
        }
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
    }
}

/// Compare packet bodies semantically when a reviewed narrow comparator exists.
///
/// Routing is not normalized here: [`crate::diff::DiffReport`] still compares
/// `connection_id`, so `SMSG_LOG_XP_GAIN` must use the realm socket and
/// `SMSG_LOOT_REMOVED` / the reviewed `SMSG_UPDATE_OBJECT` must use the
/// instance socket like C++.
#[must_use]
pub fn compare_packet_bodies(
    direction: Direction,
    opcode: u16,
    cpp: &[u8],
    rust: &[u8],
) -> Option<SemanticBodyDiff> {
    if direction != Direction::S2C {
        return None;
    }

    match opcode {
        SMSG_LOG_XP_GAIN => compare_log_xp_gain_bodies(cpp, rust),
        SMSG_LOOT_REMOVED => compare_loot_removed_bodies(cpp, rust),
        SMSG_BUY_SUCCEEDED => compare_buy_succeeded_bodies(cpp, rust),
        SMSG_UPDATE_OBJECT => compare_update_object_inv_slots_bodies(cpp, rust),
        _ => None,
    }
}

fn compare_buy_succeeded_bodies(cpp: &[u8], rust: &[u8]) -> Option<SemanticBodyDiff> {
    let cpp_decoded = decode_buy_succeeded_body_with_counter(cpp);
    let rust_decoded = decode_buy_succeeded_body_with_counter(rust);

    // Normalize only the exact G'eras fixture identity. The bot preflight
    // pins SQL spawn 96654 and rejects an overlapping same-entry spawn before
    // it performs the purchase.
    let cpp_has_reviewed_vendor = cpp_decoded
        .as_ref()
        .is_ok_and(DecodedBuySucceededBody::has_issue_108_vendor_identity);
    let rust_has_reviewed_vendor = rust_decoded
        .as_ref()
        .is_ok_and(DecodedBuySucceededBody::has_issue_108_vendor_identity);
    if cpp_decoded.is_ok()
        && rust_decoded.is_ok()
        && !cpp_has_reviewed_vendor
        && !rust_has_reviewed_vendor
    {
        return None;
    }

    Some(SemanticBodyDiff {
        comparator: "smsg_buy_succeeded_without_vendor_runtime_guid_counter".to_string(),
        cpp: SemanticBodySide::from_decoded_buy_succeeded(cpp_decoded, cpp),
        rust: SemanticBodySide::from_decoded_buy_succeeded(rust_decoded, rust),
    })
}

fn compare_log_xp_gain_bodies(cpp: &[u8], rust: &[u8]) -> Option<SemanticBodyDiff> {
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
        cpp: SemanticBodySide::from_decoded_log_xp_gain(cpp_decoded, cpp),
        rust: SemanticBodySide::from_decoded_log_xp_gain(rust_decoded, rust),
    })
}

fn compare_loot_removed_bodies(cpp: &[u8], rust: &[u8]) -> Option<SemanticBodyDiff> {
    let cpp_decoded = decode_loot_removed_body_with_counter(cpp);
    let rust_decoded = decode_loot_removed_body_with_counter(rust);

    // Normalize only the exact Doctor/LootObject/list shape proven by the
    // paired issue-#106 capture. A generic Creature predicate would conflate
    // two same-entry world objects because the omitted counter is their only
    // per-instance wire identity. The bot's fixture preflight additionally
    // proves that this entry has one SQL spawn on map 530.
    let cpp_has_reviewed_owner = cpp_decoded
        .as_ref()
        .is_ok_and(DecodedLootRemovedBody::has_issue_106_owner_identity);
    let rust_has_reviewed_owner = rust_decoded
        .as_ref()
        .is_ok_and(DecodedLootRemovedBody::has_issue_106_owner_identity);
    if cpp_decoded.is_ok()
        && rust_decoded.is_ok()
        && !cpp_has_reviewed_owner
        && !rust_has_reviewed_owner
    {
        return None;
    }

    Some(SemanticBodyDiff {
        comparator: "smsg_loot_removed_without_creature_owner_runtime_guid_counter".to_string(),
        cpp: SemanticBodySide::from_decoded_loot_removed(cpp_decoded, cpp),
        rust: SemanticBodySide::from_decoded_loot_removed(rust_decoded, rust),
    })
}

fn compare_update_object_inv_slots_bodies(cpp: &[u8], rust: &[u8]) -> Option<SemanticBodyDiff> {
    let cpp_decoded = decode_update_object_inv_slots_candidate(cpp);
    let rust_decoded = decode_update_object_inv_slots_candidate(rust);

    let cpp_candidate = cpp_decoded.candidate();
    let rust_candidate = rust_decoded.candidate();
    match (cpp_candidate, rust_candidate) {
        // This is the only accepted asymmetry: C++ accumulated UnitData parent
        // bit 116 with no child or payload; Rust emitted only the identical
        // ActivePlayer InvSlots delta.
        (Some(cpp), Some(rust))
            if cpp.has_empty_unit_power_parent && !rust.has_empty_unit_power_parent => {}
        // Identical orientation has no normalization to perform. Leave these
        // UpdateObject bodies under ordinary byte comparison.
        (Some(cpp), Some(rust))
            if cpp.has_empty_unit_power_parent == rust.has_empty_unit_power_parent =>
        {
            return None;
        }
        (None, None)
            if matches!(cpp_decoded, UpdateObjectInvSlotsDecode::NotEligible(_))
                && matches!(rust_decoded, UpdateObjectInvSlotsDecode::NotEligible(_)) =>
        {
            return None;
        }
        // A non-candidate packet paired with a malformed candidate-shaped
        // packet is not evidence that this narrowly reviewed asymmetry is in
        // play. Keep the ordinary raw-body diff in either orientation. Two
        // malformed candidate-shaped packets remain fail-closed below.
        (None, None)
            if matches!(cpp_decoded, UpdateObjectInvSlotsDecode::NotEligible(_))
                && matches!(rust_decoded, UpdateObjectInvSlotsDecode::Malformed(_)) =>
        {
            return None;
        }
        (None, None)
            if matches!(cpp_decoded, UpdateObjectInvSlotsDecode::Malformed(_))
                && matches!(rust_decoded, UpdateObjectInvSlotsDecode::NotEligible(_)) =>
        {
            return None;
        }
        _ => {}
    }

    let reverse_orientation = matches!(
        (cpp_candidate, rust_candidate),
        (Some(cpp), Some(rust))
            if !cpp.has_empty_unit_power_parent && rust.has_empty_unit_power_parent
    );
    let mut cpp_side = SemanticBodySide::from_update_object_inv_slots_decode(cpp_decoded, cpp);
    let mut rust_side = SemanticBodySide::from_update_object_inv_slots_decode(rust_decoded, rust);
    if reverse_orientation {
        let error = "empty UnitData Power parent appears only on Rust; normalization is C++-only"
            .to_string();
        cpp_side.decode_error = Some(error.clone());
        cpp_side.raw_body_sha256 = Some(raw_body_sha256(cpp));
        rust_side.decode_error = Some(error);
        rust_side.raw_body_sha256 = Some(raw_body_sha256(rust));
    }

    Some(SemanticBodyDiff {
        comparator: "smsg_update_object_without_cpp_empty_unit_power_parent".to_string(),
        cpp: cpp_side,
        rust: rust_side,
    })
}

#[derive(Debug, Clone, Copy)]
struct DecodedLogXpGainBody {
    body: LogXpGainBody,
    runtime_counter: u64,
}

#[derive(Debug, Clone, Copy)]
struct DecodedLootRemovedBody {
    body: LootRemovedBody,
    owner_runtime_counter: u64,
}

#[derive(Debug, Clone, Copy)]
struct DecodedBuySucceededBody {
    body: BuySucceededBody,
    vendor_runtime_counter: u64,
}

#[derive(Debug, Clone)]
struct DecodedUpdateObjectInvSlotsBody {
    body: UpdateObjectInvSlotsBody,
    has_empty_unit_power_parent: bool,
}

#[derive(Debug)]
enum UpdateObjectInvSlotsDecode {
    Candidate(DecodedUpdateObjectInvSlotsBody),
    NotEligible(&'static str),
    Malformed(String),
}

impl UpdateObjectInvSlotsDecode {
    fn candidate(&self) -> Option<&DecodedUpdateObjectInvSlotsBody> {
        match self {
            Self::Candidate(decoded) => Some(decoded),
            Self::NotEligible(_) | Self::Malformed(_) => None,
        }
    }
}

enum UpdateObjectInvSlotsFailure {
    NotEligible(&'static str),
    Malformed(String),
}

impl DecodedLootRemovedBody {
    fn has_issue_106_owner_identity(&self) -> bool {
        self.body.owner == ISSUE_106_CREATURE_IDENTITY
    }

    fn issue_106_shape_error(&self) -> Option<String> {
        if !self.has_issue_106_owner_identity() {
            return None;
        }
        if self.owner_runtime_counter == 0 {
            return Some(
                "loot-removed issue-#106 Creature owner has a zero runtime GUID counter"
                    .to_string(),
            );
        }

        if let Some(error) = issue_106_loot_object_error(self.body.loot_obj) {
            return Some(error);
        }
        if self.body.loot_list_id != 0 {
            return Some("loot_list_id is not the reviewed deterministic slot 0".to_string());
        }
        None
    }

    fn is_issue_106_reviewed_shape(&self) -> bool {
        self.has_issue_106_owner_identity() && self.issue_106_shape_error().is_none()
    }
}

impl DecodedBuySucceededBody {
    fn has_issue_108_vendor_identity(&self) -> bool {
        self.body.vendor == ISSUE_108_VENDOR_IDENTITY
    }

    fn issue_108_shape_error(&self) -> Option<String> {
        if !self.has_issue_108_vendor_identity() {
            return None;
        }
        if self.vendor_runtime_counter == 0 {
            return Some("issue-#108 vendor has a zero runtime GUID counter".to_string());
        }
        if self.body.muid != ISSUE_108_VENDOR_MUID {
            return Some(format!(
                "issue-#108 vendor MUID is {}, expected {}",
                self.body.muid, ISSUE_108_VENDOR_MUID
            ));
        }
        if self.body.new_quantity != ISSUE_108_VENDOR_NEW_QUANTITY {
            return Some(format!(
                "issue-#108 vendor NewQuantity is {}, expected {}",
                self.body.new_quantity, ISSUE_108_VENDOR_NEW_QUANTITY
            ));
        }
        if self.body.quantity_bought != ISSUE_108_VENDOR_QUANTITY_BOUGHT {
            return Some(format!(
                "issue-#108 vendor QuantityBought is {}, expected {}",
                self.body.quantity_bought, ISSUE_108_VENDOR_QUANTITY_BOUGHT
            ));
        }
        None
    }

    fn is_issue_108_reviewed_shape(&self) -> bool {
        self.has_issue_108_vendor_identity() && self.issue_108_shape_error().is_none()
    }
}

fn issue_106_loot_object_error(loot_obj: ExactObjectGuid) -> Option<String> {
    let stable = stable_object_guid(loot_obj.low, loot_obj.high);
    if stable.high_type != HIGH_GUID_LOOT_OBJECT
        || stable.realm_id != ISSUE_106_CREATURE_IDENTITY.realm_id
        || stable.map_id != ISSUE_106_CREATURE_IDENTITY.map_id
        || stable.entry != 0
        || stable.subtype != 0
        || stable.server_id != 0
    {
        return Some("loot_obj.high is not the reviewed map-530 LootObject identity".to_string());
    }
    if loot_obj.low & OBJECT_GUID_COUNTER_MASK == 0 {
        return Some("loot_obj.low has a zero runtime GUID counter".to_string());
    }
    None
}

impl DecodedLogXpGainBody {
    fn is_creature_kill(&self) -> bool {
        self.body.reason == XP_GAIN_REASON_KILL && self.body.victim.high_type == HIGH_GUID_CREATURE
    }
}

fn decode_update_object_inv_slots_candidate(body: &[u8]) -> UpdateObjectInvSlotsDecode {
    match decode_update_object_inv_slots_candidate_inner(body) {
        Ok(decoded) => UpdateObjectInvSlotsDecode::Candidate(decoded),
        Err(UpdateObjectInvSlotsFailure::NotEligible(reason)) => {
            UpdateObjectInvSlotsDecode::NotEligible(reason)
        }
        Err(UpdateObjectInvSlotsFailure::Malformed(error)) => {
            UpdateObjectInvSlotsDecode::Malformed(error)
        }
    }
}

fn decode_update_object_inv_slots_candidate_inner(
    body: &[u8],
) -> Result<DecodedUpdateObjectInvSlotsBody, UpdateObjectInvSlotsFailure> {
    let mut cursor = 0usize;
    let num_updates = read_u32(body, &mut cursor, "NumObjUpdates").map_err(|_| {
        UpdateObjectInvSlotsFailure::NotEligible("body is too short for NumObjUpdates")
    })?;
    if num_updates != 1 {
        return Err(UpdateObjectInvSlotsFailure::NotEligible(
            "NumObjUpdates is not exactly one",
        ));
    }

    let map_id =
        read_u16(body, &mut cursor, "MapID").map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    let destroy_or_out_of_range = read_u8(body, &mut cursor, "HasDestroyOrOutOfRange byte")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    if destroy_or_out_of_range == 0x80 {
        return Err(UpdateObjectInvSlotsFailure::NotEligible(
            "packet carries destroy or out-of-range GUIDs",
        ));
    }
    if destroy_or_out_of_range != 0 {
        return Err(UpdateObjectInvSlotsFailure::Malformed(format!(
            "HasDestroyOrOutOfRange byte has non-canonical padding bits: 0x{destroy_or_out_of_range:02X}"
        )));
    }

    let declared_blocks_len = read_u32(body, &mut cursor, "update blocks length")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)? as usize;
    let actual_blocks_len = body.len().saturating_sub(cursor);
    if declared_blocks_len != actual_blocks_len {
        return Err(UpdateObjectInvSlotsFailure::Malformed(format!(
            "update blocks length declares {declared_blocks_len} bytes but {actual_blocks_len} remain"
        )));
    }
    let blocks = &body[cursor..];
    let mut block_cursor = 0usize;
    let update_type = read_u8(blocks, &mut block_cursor, "UpdateType")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    if update_type != 0 {
        return Err(UpdateObjectInvSlotsFailure::NotEligible(
            "the sole update block is not UpdateType::Values",
        ));
    }

    let (player_low, player_high) = read_packed_guid(blocks, &mut block_cursor, "Player")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    if ((player_high >> 58) & 0x3F) as u8 != HIGH_GUID_PLAYER {
        return Err(UpdateObjectInvSlotsFailure::NotEligible(
            "the VALUES owner is not a Player GUID",
        ));
    }
    if player_low == 0 {
        return Err(UpdateObjectInvSlotsFailure::Malformed(
            "Player GUID has a zero low identity".to_string(),
        ));
    }
    if player_high & GLOBAL_GUID_RESERVED_HIGH_BITS_MASK != 0 {
        return Err(UpdateObjectInvSlotsFailure::Malformed(
            "Player GUID sets reserved high-word bits 0..41".to_string(),
        ));
    }

    let declared_values_len = read_u32(blocks, &mut block_cursor, "values length")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)? as usize;
    let actual_values_len = blocks.len().saturating_sub(block_cursor);
    if declared_values_len != actual_values_len {
        return Err(UpdateObjectInvSlotsFailure::Malformed(format!(
            "values length declares {declared_values_len} bytes but {actual_values_len} remain"
        )));
    }
    let values = &blocks[block_cursor..];
    let mut values_cursor = 0usize;
    let has_empty_unit_power_parent =
        decode_reviewed_changed_object_type_mask(values, &mut values_cursor)?;

    if has_empty_unit_power_parent {
        decode_empty_unit_power_parent(values, &mut values_cursor)?;
    }

    let inv_slots = decode_active_player_inv_slots(values, &mut values_cursor)
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    if values_cursor != values.len() {
        return Err(UpdateObjectInvSlotsFailure::Malformed(format!(
            "trailing bytes after ActivePlayer InvSlots: decoded {values_cursor} of {} bytes",
            values.len()
        )));
    }

    Ok(DecodedUpdateObjectInvSlotsBody {
        body: UpdateObjectInvSlotsBody {
            map_id,
            player: ExactObjectGuid {
                low: player_low,
                high: player_high,
            },
            inv_slots,
        },
        has_empty_unit_power_parent,
    })
}

fn decode_reviewed_changed_object_type_mask(
    values: &[u8],
    cursor: &mut usize,
) -> Result<bool, UpdateObjectInvSlotsFailure> {
    let changed_object_type_mask = read_u32(values, cursor, "ChangedObjectTypeMask")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    match changed_object_type_mask {
        VALUES_TYPE_ACTIVE_PLAYER => Ok(false),
        mask if mask == VALUES_TYPE_UNIT | VALUES_TYPE_ACTIVE_PLAYER => Ok(true),
        _ => Err(UpdateObjectInvSlotsFailure::NotEligible(
            "ChangedObjectTypeMask is not ActivePlayer-only or Unit+ActivePlayer",
        )),
    }
}

fn decode_empty_unit_power_parent(
    values: &[u8],
    cursor: &mut usize,
) -> Result<(), UpdateObjectInvSlotsFailure> {
    let blocks_mask = read_u8(values, cursor, "UnitData blocks mask")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    if blocks_mask != UNIT_POWER_PARENT_BLOCKS_MASK {
        return Err(UpdateObjectInvSlotsFailure::Malformed(format!(
            "UnitData blocks mask is 0x{blocks_mask:02X}, expected only block 3"
        )));
    }

    let block_3 = read_u32_be(values, cursor, "UnitData block 3")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    if block_3 != UNIT_POWER_PARENT_BLOCK_3 {
        return Err(UpdateObjectInvSlotsFailure::Malformed(format!(
            "UnitData block 3 is 0x{block_3:08X}, expected only parent bit 116"
        )));
    }
    Ok(())
}

fn decode_active_player_inv_slots(
    values: &[u8],
    cursor: &mut usize,
) -> Result<Vec<InvSlotValue>, String> {
    let blocks_group_0 = read_u32(values, cursor, "ActivePlayer blocks group 0")?;
    let blocks_group_1 = read_u16_be(values, cursor, "ActivePlayer blocks group 1")?;
    if blocks_group_1 != 0 {
        return Err(format!(
            "ActivePlayer blocks group 1 is 0x{blocks_group_1:04X}; InvSlots uses only blocks 3-8"
        ));
    }

    let mut blocks = [0u32; 32];
    for (index, block) in blocks.iter_mut().enumerate() {
        if blocks_group_0 & (1 << index) != 0 {
            *block = read_u32_be(values, cursor, "ActivePlayer block")?;
            if *block == 0 {
                return Err(format!("ActivePlayer group mask names empty block {index}"));
            }
        }
    }

    let mut has_inv_slots_parent = false;
    let mut changed_slots = Vec::new();
    for (block_index, block) in blocks.iter().copied().enumerate() {
        for bit in 0..32u32 {
            if block & (1 << bit) == 0 {
                continue;
            }
            let field = block_index as u32 * 32 + bit;
            match field {
                124 => has_inv_slots_parent = true,
                125..=265 => changed_slots.push((field - 125) as u16),
                _ => {
                    return Err(format!(
                        "ActivePlayer mask contains non-InvSlots field {field}"
                    ));
                }
            }
        }
    }
    if !has_inv_slots_parent {
        return Err("ActivePlayer InvSlots parent bit 124 is absent".to_string());
    }
    if changed_slots.len() != 1 {
        return Err(format!(
            "reviewed loot update requires exactly one InvSlots child, found {}",
            changed_slots.len()
        ));
    }

    let mut inv_slots = Vec::with_capacity(1);
    for slot in changed_slots {
        let (item_low, item_high) = read_packed_guid(values, cursor, "InvSlots item")?;
        if ((item_high >> 58) & 0x3F) as u8 != HIGH_GUID_ITEM || item_low == 0 {
            return Err(format!("InvSlots[{slot}] is not a non-empty Item GUID"));
        }
        if item_high & GLOBAL_GUID_RESERVED_HIGH_BITS_MASK != 0 {
            return Err(format!(
                "InvSlots[{slot}] Item GUID sets reserved high-word bits 0..41"
            ));
        }
        inv_slots.push(InvSlotValue {
            slot,
            item: ExactObjectGuid {
                low: item_low,
                high: item_high,
            },
        });
    }
    Ok(inv_slots)
}

#[derive(Debug, Clone, Copy)]
struct DecodedSingleLootItemRequest {
    loot_obj: ExactObjectGuid,
    loot_list_id: u8,
}

#[derive(Debug, Clone, Copy)]
struct DecodedIssue106ItemPushResult {
    player: ExactObjectGuid,
    slot_in_bag: i32,
    quantity: i32,
    item_guid: ExactObjectGuid,
    item_entry: i32,
}

#[derive(Debug, Clone, Copy)]
struct DecodedIssue106ItemCreate {
    map_id: u16,
    item_guid: ExactObjectGuid,
    owner: ExactObjectGuid,
    contained_in: ExactObjectGuid,
    item_entry: i32,
    stack_count: u32,
}

/// Validate the complete, correlated payload contract of the issue-#106
/// single-item capture.
///
/// Opcode equality alone is insufficient evidence: an unrelated item push or
/// two identically malformed packets could otherwise compare clean. This pins
/// the one-request count, deterministic fixture identities, request/removal
/// LootObject and list id, recipient, quantity, awarded Item GUID, inventory
/// item CreateObject, slot update, and fixed ping fence on each side
/// independently.
pub fn validate_loot_single_item_claim_capture(capture: &Capture) -> Result<(), String> {
    validate_issue_106_capture_topology(capture)?;

    let request = decode_single_loot_item_request(&capture.packets[0].body)?;
    let created = decode_issue_106_item_create(&capture.packets[1].body)?;
    let removed = decode_loot_removed_body_with_counter(&capture.packets[2].body)?;
    validate_issue_106_request_removal(request, removed)?;

    let pushed = decode_issue_106_item_push_result(&capture.packets[3].body)?;
    validate_issue_106_created_grant(created, pushed)?;
    validate_issue_106_inventory_update(&capture.packets[4].body, pushed)?;

    if capture.packets[5].body != ISSUE_106_PING_BODY {
        return Err(format!(
            "CMSG_PING fence body is {:02X?}, expected fixed TOOL/zero-latency body {:02X?}",
            capture.packets[5].body, ISSUE_106_PING_BODY
        ));
    }
    Ok(())
}

fn validate_issue_106_capture_topology(capture: &Capture) -> Result<(), String> {
    const EXPECTED: [(Direction, u32, u16); 6] = [
        (Direction::C2S, 1, CMSG_LOOT_ITEM),
        (Direction::S2C, 1, SMSG_UPDATE_OBJECT),
        (Direction::S2C, 1, SMSG_LOOT_REMOVED),
        (Direction::S2C, 0, SMSG_ITEM_PUSH_RESULT),
        (Direction::S2C, 1, SMSG_UPDATE_OBJECT),
        (Direction::C2S, 1, CMSG_PING),
    ];
    if capture.packets.len() != EXPECTED.len() {
        return Err(format!(
            "{} contains {} packet(s); the semantic loot-claim contract requires exactly {}",
            capture.source,
            capture.packets.len(),
            EXPECTED.len()
        ));
    }
    for (index, (packet, (direction, connection_id, opcode))) in
        capture.packets.iter().zip(EXPECTED).enumerate()
    {
        if packet.direction != direction
            || packet.connection_id != connection_id
            || packet.opcode != opcode
        {
            return Err(format!(
                "{} packet {index} is {} conn={} 0x{:04X}; semantic contract requires {} conn={} 0x{opcode:04X}",
                capture.source,
                packet.direction,
                packet.connection_id,
                packet.opcode,
                direction,
                connection_id
            ));
        }
    }
    Ok(())
}

fn validate_issue_106_request_removal(
    request: DecodedSingleLootItemRequest,
    removed: DecodedLootRemovedBody,
) -> Result<(), String> {
    if !removed.is_issue_106_reviewed_shape() {
        return Err(removed.issue_106_shape_error().unwrap_or_else(|| {
            format!(
                "loot-removed owner {:?} is not the reviewed issue-#106 Doctor identity",
                removed.body.owner
            )
        }));
    }
    if request.loot_obj != removed.body.loot_obj {
        return Err(format!(
            "CMSG_LOOT_ITEM LootObj {:?} does not match SMSG_LOOT_REMOVED {:?}",
            request.loot_obj, removed.body.loot_obj
        ));
    }
    if request.loot_list_id != removed.body.loot_list_id {
        return Err(format!(
            "CMSG_LOOT_ITEM LootListID {} does not match SMSG_LOOT_REMOVED {}",
            request.loot_list_id, removed.body.loot_list_id
        ));
    }
    Ok(())
}

fn validate_issue_106_created_grant(
    created: DecodedIssue106ItemCreate,
    pushed: DecodedIssue106ItemPushResult,
) -> Result<(), String> {
    if created.map_id != ISSUE_106_CREATURE_IDENTITY.map_id {
        return Err(format!(
            "item CreateObject map {} does not match loot owner map {}",
            created.map_id, ISSUE_106_CREATURE_IDENTITY.map_id
        ));
    }
    if created.owner != pushed.player || created.contained_in != pushed.player {
        return Err(format!(
            "item CreateObject owner/contained {:?}/{:?} does not match ItemPushResult player {:?}",
            created.owner, created.contained_in, pushed.player
        ));
    }
    if created.item_guid != pushed.item_guid {
        return Err(format!(
            "item CreateObject GUID {:?} does not match ItemPushResult {:?}",
            created.item_guid, pushed.item_guid
        ));
    }
    if created.item_entry != pushed.item_entry {
        return Err(format!(
            "item CreateObject entry {} does not match ItemPushResult {}",
            created.item_entry, pushed.item_entry
        ));
    }
    if created.stack_count != pushed.quantity as u32 {
        return Err(format!(
            "item CreateObject stack {} does not match ItemPushResult quantity {}",
            created.stack_count, pushed.quantity
        ));
    }
    if pushed.quantity != 1 || pushed.item_entry != ISSUE_106_ITEM_ENTRY {
        return Err(format!(
            "ItemPushResult is item {}/quantity {}, expected fixture item {ISSUE_106_ITEM_ENTRY}/quantity 1",
            pushed.item_entry, pushed.quantity
        ));
    }
    Ok(())
}

fn validate_issue_106_inventory_update(
    body: &[u8],
    pushed: DecodedIssue106ItemPushResult,
) -> Result<(), String> {
    let inventory_update = match decode_update_object_inv_slots_candidate(body) {
        UpdateObjectInvSlotsDecode::Candidate(decoded) => decoded,
        UpdateObjectInvSlotsDecode::NotEligible(reason) => {
            return Err(format!(
                "post-claim SMSG_UPDATE_OBJECT is not the reviewed one-slot InvSlots update: {reason}"
            ));
        }
        UpdateObjectInvSlotsDecode::Malformed(error) => {
            return Err(format!(
                "post-claim SMSG_UPDATE_OBJECT is malformed: {error}"
            ));
        }
    };
    if inventory_update.body.map_id != ISSUE_106_CREATURE_IDENTITY.map_id {
        return Err(format!(
            "post-claim map {} does not match loot owner map {}",
            inventory_update.body.map_id, ISSUE_106_CREATURE_IDENTITY.map_id
        ));
    }
    if inventory_update.body.player != pushed.player {
        return Err(format!(
            "ItemPushResult player {:?} does not match InvSlots player {:?}",
            pushed.player, inventory_update.body.player
        ));
    }
    let [slot] = inventory_update.body.inv_slots.as_slice() else {
        return Err(format!(
            "post-claim InvSlots update contains {} child values, expected one",
            inventory_update.body.inv_slots.len()
        ));
    };
    if i32::from(slot.slot) != pushed.slot_in_bag {
        return Err(format!(
            "ItemPushResult SlotInBag {} does not match InvSlots child {}",
            pushed.slot_in_bag, slot.slot
        ));
    }
    if slot.item != pushed.item_guid {
        return Err(format!(
            "ItemPushResult ItemGUID {:?} does not match InvSlots item {:?}",
            pushed.item_guid, slot.item
        ));
    }
    Ok(())
}

/// Decode only the deterministic one-item CreateObject shape captured for
/// issue #106.
///
/// C++ anchors:
///
/// - `UpdateData::BuildPacket`: one update, map, destroy bit, data length;
/// - `Object::BuildCreateUpdateBlockForPlayer`: type, GUID, TypeID, movement,
///   values;
/// - `Item::BuildValuesCreate`: value length and Owner visibility flag;
/// - `ObjectData::WriteCreate` / `ItemData::WriteCreate`: entry, owner,
///   contained-in GUID and StackCount in that order.
fn decode_issue_106_item_create(body: &[u8]) -> Result<DecodedIssue106ItemCreate, String> {
    let mut cursor = 0usize;
    let num_updates = read_u32(body, &mut cursor, "NumObjUpdates")?;
    if num_updates != 1 {
        return Err(format!(
            "item CreateObject contains {num_updates} object updates, expected exactly one"
        ));
    }
    let map_id = read_u16(body, &mut cursor, "MapID")?;
    let destroy_or_out_of_range = read_u8(body, &mut cursor, "HasDestroyOrOutOfRange byte")?;
    if destroy_or_out_of_range != 0 {
        return Err(format!(
            "item CreateObject HasDestroyOrOutOfRange byte is 0x{destroy_or_out_of_range:02X}, expected canonical false"
        ));
    }

    let declared_blocks_len = read_u32(body, &mut cursor, "update blocks length")? as usize;
    let actual_blocks_len = body.len().saturating_sub(cursor);
    if declared_blocks_len != actual_blocks_len {
        return Err(format!(
            "item CreateObject update blocks length declares {declared_blocks_len} bytes but {actual_blocks_len} remain"
        ));
    }

    let update_type = read_u8(body, &mut cursor, "UpdateType")?;
    if update_type != 1 {
        return Err(format!(
            "item CreateObject UpdateType is {update_type}, expected CreateObject (1)"
        ));
    }
    let (item_low, item_high) = read_packed_guid(body, &mut cursor, "created Item")?;
    let item_guid = ExactObjectGuid {
        low: item_low,
        high: item_high,
    };
    if let Some(error) = issue_106_item_guid_error(item_guid) {
        return Err(format!("item CreateObject {error}"));
    }

    let type_id = read_u8(body, &mut cursor, "TypeID")?;
    if type_id != 1 {
        return Err(format!(
            "item CreateObject TypeID is {type_id}, expected Item (1)"
        ));
    }
    let create_bits: [u8; 3] = read_array(body, &mut cursor, "CreateObjectBits")?;
    if create_bits != [0; 3] {
        return Err(format!(
            "item CreateObject flags are {create_bits:02X?}, expected all 18 Item flags clear"
        ));
    }
    let pause_times = read_i32(body, &mut cursor, "PauseTimes count")?;
    if pause_times != 0 {
        return Err(format!(
            "item CreateObject PauseTimes count is {pause_times}, expected zero"
        ));
    }

    let declared_values_len = read_u32(body, &mut cursor, "values length")? as usize;
    let actual_values_len = body.len().saturating_sub(cursor);
    if declared_values_len != actual_values_len {
        return Err(format!(
            "item CreateObject values length declares {declared_values_len} bytes but {actual_values_len} remain"
        ));
    }
    decode_issue_106_item_create_values(&body[cursor..], map_id, item_guid)
}

fn decode_issue_106_item_create_values(
    values: &[u8],
    map_id: u16,
    item_guid: ExactObjectGuid,
) -> Result<DecodedIssue106ItemCreate, String> {
    let mut values_cursor = 0usize;
    let visibility_flags = read_u8(values, &mut values_cursor, "UpdateFieldFlags")?;
    let item_entry = read_i32(values, &mut values_cursor, "ObjectData.EntryID")?;
    let object_dynamic_flags = read_u32(values, &mut values_cursor, "ObjectData.DynamicFlags")?;
    let object_scale_bits = read_u32(values, &mut values_cursor, "ObjectData.Scale")?;
    if visibility_flags != 0x01
        || object_dynamic_flags != 0
        || object_scale_bits != 1.0_f32.to_bits()
    {
        return Err(format!(
            "item CreateObject ObjectData is not the reviewed owner-visible shape: flags=0x{visibility_flags:02X} dynamic=0x{object_dynamic_flags:08X} scale_bits=0x{object_scale_bits:08X}"
        ));
    }
    if item_entry != ISSUE_106_ITEM_ENTRY {
        return Err(format!(
            "item CreateObject entry is {item_entry}, expected fixture item {ISSUE_106_ITEM_ENTRY}"
        ));
    }

    let (owner_low, owner_high) = read_packed_guid(values, &mut values_cursor, "ItemData.Owner")?;
    let owner = ExactObjectGuid {
        low: owner_low,
        high: owner_high,
    };
    let (contained_low, contained_high) =
        read_packed_guid(values, &mut values_cursor, "ItemData.ContainedIn")?;
    let contained_in = ExactObjectGuid {
        low: contained_low,
        high: contained_high,
    };
    let creator = read_packed_guid(values, &mut values_cursor, "ItemData.Creator")?;
    let gift_creator = read_packed_guid(values, &mut values_cursor, "ItemData.GiftCreator")?;
    let expected_player = ExactObjectGuid {
        low: ISSUE_106_CAPTURE_PLAYER_LOW,
        high: ISSUE_106_CAPTURE_PLAYER_HIGH,
    };
    if owner != expected_player || contained_in != expected_player {
        return Err(format!(
            "item CreateObject Owner/ContainedIn is {owner:?}/{contained_in:?}, expected capture player {expected_player:?}"
        ));
    }
    if creator != (0, 0) || gift_creator != (0, 0) {
        return Err(format!(
            "item CreateObject Creator/GiftCreator is {creator:?}/{gift_creator:?}, expected empty GUIDs"
        ));
    }

    let stack_count = read_u32(values, &mut values_cursor, "ItemData.StackCount")?;
    let expiration = read_u32(values, &mut values_cursor, "ItemData.Expiration")?;
    let mut spell_charges = [0_i32; 5];
    for charge in &mut spell_charges {
        *charge = read_i32(values, &mut values_cursor, "ItemData.SpellCharges")?;
    }
    let item_dynamic_flags = read_u32(values, &mut values_cursor, "ItemData.DynamicFlags")?;
    if stack_count != 1
        || expiration != 0
        || spell_charges != [0; 5]
        || item_dynamic_flags != ISSUE_106_ITEM_DYNAMIC_FLAGS
    {
        return Err(format!(
            "item CreateObject ItemData is not the deterministic one-key shape: stack={stack_count} expiration={expiration} charges={spell_charges:?} dynamic=0x{item_dynamic_flags:08X}"
        ));
    }

    let zero_tail = &values[values_cursor..];
    if zero_tail.len() != ISSUE_106_ITEM_CREATE_ZERO_TAIL_LEN
        || zero_tail.iter().any(|byte| *byte != 0)
    {
        return Err(format!(
            "item CreateObject deterministic ItemData tail has length {} and {} nonzero byte(s), expected {} zero bytes",
            zero_tail.len(),
            zero_tail.iter().filter(|byte| **byte != 0).count(),
            ISSUE_106_ITEM_CREATE_ZERO_TAIL_LEN
        ));
    }

    Ok(DecodedIssue106ItemCreate {
        map_id,
        item_guid,
        owner,
        contained_in,
        item_entry,
        stack_count,
    })
}

fn decode_single_loot_item_request(body: &[u8]) -> Result<DecodedSingleLootItemRequest, String> {
    let mut cursor = 0usize;
    let count = read_u32(body, &mut cursor, "Loot request count")?;
    if count != 1 {
        return Err(format!(
            "CMSG_LOOT_ITEM contains {count} request(s), expected exactly one"
        ));
    }
    let (loot_obj_low, loot_obj_high) = read_packed_guid(body, &mut cursor, "Loot Object")?;
    let loot_obj = ExactObjectGuid {
        low: loot_obj_low,
        high: loot_obj_high,
    };
    if let Some(error) = issue_106_loot_object_error(loot_obj) {
        return Err(format!("CMSG_LOOT_ITEM {error}"));
    }
    let loot_list_id = read_u8(body, &mut cursor, "LootListID")?;
    if loot_list_id != 0 {
        return Err(format!(
            "CMSG_LOOT_ITEM LootListID is {loot_list_id}, expected deterministic slot 0"
        ));
    }
    let soft_interact = read_u8(body, &mut cursor, "IsSoftInteract bit byte")?;
    if soft_interact != 0 {
        return Err(format!(
            "CMSG_LOOT_ITEM IsSoftInteract/padding byte is 0x{soft_interact:02X}, expected 0"
        ));
    }
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after CMSG_LOOT_ITEM: decoded {cursor} of {} bytes",
            body.len()
        ));
    }
    Ok(DecodedSingleLootItemRequest {
        loot_obj,
        loot_list_id,
    })
}

fn decode_issue_106_item_push_result(body: &[u8]) -> Result<DecodedIssue106ItemPushResult, String> {
    let mut cursor = 0usize;
    let (player_low, player_high) = read_packed_guid(body, &mut cursor, "PlayerGUID")?;
    let player = ExactObjectGuid {
        low: player_low,
        high: player_high,
    };
    if player.low != ISSUE_106_CAPTURE_PLAYER_LOW || player.high != ISSUE_106_CAPTURE_PLAYER_HIGH {
        return Err(format!(
            "ItemPushResult PlayerGUID is {player:?}, expected capture character low={ISSUE_106_CAPTURE_PLAYER_LOW} high=0x{ISSUE_106_CAPTURE_PLAYER_HIGH:016X}"
        ));
    }

    let slot = read_u8(body, &mut cursor, "Slot")?;
    let slot_in_bag = read_i32(body, &mut cursor, "SlotInBag")?;
    let quest_log_item_id = read_i32(body, &mut cursor, "QuestLogItemID")?;
    let quantity = read_i32(body, &mut cursor, "Quantity")?;
    let quantity_in_inventory = read_i32(body, &mut cursor, "QuantityInInventory")?;
    let dungeon_encounter_id = read_i32(body, &mut cursor, "DungeonEncounterID")?;
    let battle_pet_species_id = read_i32(body, &mut cursor, "BattlePetSpeciesID")?;
    let battle_pet_breed_id = read_i32(body, &mut cursor, "BattlePetBreedID")?;
    let battle_pet_breed_quality = read_u32(body, &mut cursor, "BattlePetBreedQuality")?;
    let battle_pet_level = read_i32(body, &mut cursor, "BattlePetLevel")?;
    let (item_low, item_high) = read_packed_guid(body, &mut cursor, "ItemGUID")?;
    let item_guid = ExactObjectGuid {
        low: item_low,
        high: item_high,
    };
    if let Some(error) = issue_106_item_guid_error(item_guid) {
        return Err(format!("ItemPushResult {error}"));
    }

    let result_flags = read_u8(body, &mut cursor, "ItemPushResult bit flags")?;
    if result_flags != 0x08 {
        return Err(format!(
            "ItemPushResult flags are 0x{result_flags:02X}, expected non-created/non-pushed normal-display loot flags 0x08"
        ));
    }
    let item_entry = read_i32(body, &mut cursor, "ItemInstance.ItemID")?;
    let random_properties_seed = read_i32(body, &mut cursor, "ItemInstance.RandomPropertiesSeed")?;
    let random_properties_id = read_i32(body, &mut cursor, "ItemInstance.RandomPropertiesID")?;
    let item_bonus_bits = read_u8(body, &mut cursor, "ItemInstance ItemBonus bit byte")?;
    let modifications_bits = read_u8(body, &mut cursor, "ItemInstance modifications bit byte")?;
    if slot != u8::MAX
        || slot_in_bag != ISSUE_106_ITEM_SLOT
        || quest_log_item_id != 0
        || quantity != 1
        || quantity_in_inventory != 1
        || dungeon_encounter_id != 0
        || battle_pet_species_id != 0
        || battle_pet_breed_id != 0
        || battle_pet_breed_quality != 0
        || battle_pet_level != 0
        || random_properties_seed != 0
        || random_properties_id != 0
        || item_bonus_bits != 0
        || modifications_bits != 0
    {
        return Err(format!(
            "ItemPushResult is not the deterministic one-item fixture shape: slot={slot} slot_in_bag={slot_in_bag} quest={quest_log_item_id} quantity={quantity}/{quantity_in_inventory} encounter={dungeon_encounter_id} battle_pet={battle_pet_species_id}/{battle_pet_breed_id}/{battle_pet_breed_quality}/{battle_pet_level} random={random_properties_seed}/{random_properties_id} bonus_bits=0x{item_bonus_bits:02X} mod_bits=0x{modifications_bits:02X}"
        ));
    }
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after ItemPushResult ItemInstance: decoded {cursor} of {} bytes",
            body.len()
        ));
    }

    Ok(DecodedIssue106ItemPushResult {
        player,
        slot_in_bag,
        quantity,
        item_guid,
        item_entry,
    })
}

fn issue_106_item_guid_error(item_guid: ExactObjectGuid) -> Option<String> {
    if item_guid.low == 0
        || item_guid.low & !OBJECT_GUID_COUNTER_MASK != 0
        || item_guid.high != ISSUE_106_CAPTURE_ITEM_HIGH
    {
        return Some(format!(
            "ItemGUID {item_guid:?} is not the canonical non-empty realm-1 Item GUID with server id 0"
        ));
    }
    None
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

/// Decode the opcode-less body emitted by C++
/// `WorldPackets::Loot::LootRemoved::Write`.
///
/// Source anchors:
///
/// - `LootPackets.cpp`: packed Owner, packed LootObj, LootListID;
/// - `ObjectGuid.cpp::operator<<`: low mask, high mask, packed low bytes,
///   packed high bytes;
/// - `ObjectGuidFactory::CreateWorldObject`: lower 40 low-word bits are the
///   map-runtime counter.
pub fn decode_loot_removed_body(body: &[u8]) -> Result<LootRemovedBody, String> {
    decode_loot_removed_body_with_counter(body).map(|decoded| decoded.body)
}

/// Decode the opcode-less body emitted by C++
/// `WorldPackets::Item::BuySucceeded::Write`.
pub fn decode_buy_succeeded_body(body: &[u8]) -> Result<BuySucceededBody, String> {
    decode_buy_succeeded_body_with_counter(body).map(|decoded| decoded.body)
}

fn decode_buy_succeeded_body_with_counter(body: &[u8]) -> Result<DecodedBuySucceededBody, String> {
    let mut cursor = 0usize;
    let (vendor_low, vendor_high) = read_packed_guid(body, &mut cursor, "VendorGUID")?;
    let muid = read_u32(body, &mut cursor, "Muid")?;
    let new_quantity = read_i32(body, &mut cursor, "NewQuantity")?;
    let quantity_bought = read_u32(body, &mut cursor, "QuantityBought")?;
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after QuantityBought: decoded {cursor} of {} bytes",
            body.len()
        ));
    }

    Ok(DecodedBuySucceededBody {
        body: BuySucceededBody {
            vendor: stable_object_guid(vendor_low, vendor_high),
            muid,
            new_quantity,
            quantity_bought,
        },
        vendor_runtime_counter: vendor_low & OBJECT_GUID_COUNTER_MASK,
    })
}

fn decode_loot_removed_body_with_counter(body: &[u8]) -> Result<DecodedLootRemovedBody, String> {
    let mut cursor = 0usize;
    let (owner_low, owner_high) = read_packed_guid(body, &mut cursor, "Owner")?;
    let (loot_obj_low, loot_obj_high) = read_packed_guid(body, &mut cursor, "LootObj")?;
    let loot_list_id = read_u8(body, &mut cursor, "LootListID")?;
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after LootListID: decoded {cursor} of {} bytes",
            body.len()
        ));
    }

    Ok(DecodedLootRemovedBody {
        body: LootRemovedBody {
            owner: stable_object_guid(owner_low, owner_high),
            loot_obj: ExactObjectGuid {
                low: loot_obj_low,
                high: loot_obj_high,
            },
            loot_list_id,
        },
        owner_runtime_counter: owner_low & OBJECT_GUID_COUNTER_MASK,
    })
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
    Ok(DecodedLogXpGainBody {
        body: LogXpGainBody {
            victim: stable_object_guid(low, high),
            original,
            reason,
            amount,
            group_bonus_bits,
        },
        runtime_counter: low & OBJECT_GUID_COUNTER_MASK,
    })
}

fn stable_object_guid(low: u64, high: u64) -> StableObjectGuid {
    let stable_low = low & !OBJECT_GUID_COUNTER_MASK;
    StableObjectGuid {
        high_type: ((high >> 58) & 0x3F) as u8,
        // Keeping all 16 bits between map and high type retains the three
        // reserved bits as part of strict identity for non-canonical inputs.
        realm_id: ((high >> 42) & 0xFFFF) as u16,
        map_id: ((high >> 29) & 0x1FFF) as u16,
        entry: ((high >> 6) & 0x7F_FFFF) as u32,
        subtype: (high & 0x3F) as u8,
        server_id: ((stable_low >> 40) & 0xFF_FFFF) as u32,
    }
}

fn read_packed_guid(body: &[u8], cursor: &mut usize, field: &str) -> Result<(u64, u64), String> {
    let low_mask = read_u8(body, cursor, &format!("{field} low mask"))?;
    let high_mask = read_u8(body, cursor, &format!("{field} high mask"))?;
    let low = read_packed_u64(body, cursor, low_mask, &format!("{field} low word"))?;
    let high = read_packed_u64(body, cursor, high_mask, &format!("{field} high word"))?;
    Ok((low, high))
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

fn read_u16(body: &[u8], cursor: &mut usize, field: &str) -> Result<u16, String> {
    Ok(u16::from_le_bytes(read_array(body, cursor, field)?))
}

fn read_u16_be(body: &[u8], cursor: &mut usize, field: &str) -> Result<u16, String> {
    Ok(u16::from_be_bytes(read_array(body, cursor, field)?))
}

fn read_u32(body: &[u8], cursor: &mut usize, field: &str) -> Result<u32, String> {
    Ok(u32::from_le_bytes(read_array(body, cursor, field)?))
}

fn read_u32_be(body: &[u8], cursor: &mut usize, field: &str) -> Result<u32, String> {
    Ok(u32::from_be_bytes(read_array(body, cursor, field)?))
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
