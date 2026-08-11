//! Narrow semantic packet comparators for fields that are intentionally
//! runtime-allocated, intrinsically unordered in C++, or for a specifically
//! proven accumulated update-mask artifact, and therefore cannot be compared
//! byte-for-byte.
//!
//! Keep this module deliberately small. A semantic comparator is allowed to
//! omit only a field whose value cannot be made stable across equivalent C++
//! and Rust runs, canonicalize only proven unordered collection order, or omit
//! one exact empty mask fragment whose cadence was reproduced independently;
//! every other decoded bit remains part of the comparison.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{Capture, Direction};

/// `CMSG_LOOT_ITEM` in the 3.4.3 opcode table.
pub const CMSG_LOOT_ITEM: u16 = 0x3211;

/// `SMSG_LOG_XP_GAIN` in the 3.4.3 opcode table.
pub const SMSG_LOG_XP_GAIN: u16 = 0x26E5;

/// `SMSG_LOOT_REMOVED` in the 3.4.3 opcode table.
pub const SMSG_LOOT_REMOVED: u16 = 0x2615;

/// `SMSG_SEND_KNOWN_SPELLS` in the 3.4.3 opcode table.
pub const SMSG_SEND_KNOWN_SPELLS: u16 = 0x2C27;

/// `SMSG_SPELL_GO` in the 3.4.3 opcode table.
pub const SMSG_SPELL_GO: u16 = 0x2C36;

/// `SMSG_SPELL_START` in the 3.4.3 opcode table.
pub const SMSG_SPELL_START: u16 = 0x2C37;

/// `SMSG_ITEM_PUSH_RESULT` in the 3.4.3 opcode table.
pub const SMSG_ITEM_PUSH_RESULT: u16 = 0x2623;

/// `SMSG_BUY_SUCCEEDED` in the 3.4.3 opcode table.
pub const SMSG_BUY_SUCCEEDED: u16 = 0x26C6;

/// `SMSG_UPDATE_OBJECT` in the 3.4.3 opcode table.
pub const SMSG_UPDATE_OBJECT: u16 = 0x27CB;

/// `CMSG_PING` used as the deterministic end fence of the issue-#106 flow.
pub const CMSG_PING: u16 = 0x3768;

/// `CMSG_MOVE_HEARTBEAT`, the action boundary for the issue-#24 live chase.
pub const CMSG_MOVE_HEARTBEAT: u16 = 0x3A10;

/// `SMSG_ON_MONSTER_MOVE`, carrying the creature's Detour-backed chase spline.
pub const SMSG_ON_MONSTER_MOVE: u16 = 0x2DD4;

const OBJECT_GUID_COUNTER_MASK: u64 = 0x0000_00FF_FFFF_FFFF;
const HIGH_GUID_CREATURE: u8 = 8;
const HIGH_GUID_CAST: u8 = 47;
const HIGH_GUID_ITEM: u8 = 3;
const HIGH_GUID_LOOT_OBJECT: u8 = 15;
const HIGH_GUID_PLAYER: u8 = 2;
const SPELL_CAST_SOURCE_NORMAL: u8 = 3;
const SPELL_MISS_REFLECT: u8 = 11;
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
const ISSUE_24_CREATURE_IDENTITY: StableObjectGuid = StableObjectGuid {
    high_type: HIGH_GUID_CREATURE,
    realm_id: 1,
    map_id: 1,
    entry: 15_271,
    subtype: 0,
    server_id: 0,
};
const ISSUE_26_CREATURE_ENTRY: u32 = 22_378;
const ISSUE_26_REALM_ID: u16 = 1;
const ISSUE_26_MAP_ID: u16 = 530;
const ISSUE_26_SPELL_ID: i32 = 15_691;
const ISSUE_26_SPELL_X_SPELL_VISUAL_ID: i32 = 244_493;
const ISSUE_26_PLAYER_COUNTER: u64 = 15;
const ISSUE_26_START_CAST_FLAGS: u32 = 0x0000_0002;
const ISSUE_26_GO_CAST_FLAGS: u32 = 0x0000_0100;
const ISSUE_26_UNIT_TARGET_FLAGS: u32 = 0x0000_0002;
pub const ISSUE_24_PING_FENCE_WIRE: [u8; 4] = *b"DTOR";
pub const ISSUE_24_PING_FENCE_SERIAL: u32 = u32::from_le_bytes(ISSUE_24_PING_FENCE_WIRE);
const ISSUE_24_PING_BODY: [u8; 8] = [
    ISSUE_24_PING_FENCE_WIRE[0],
    ISSUE_24_PING_FENCE_WIRE[1],
    ISSUE_24_PING_FENCE_WIRE[2],
    ISSUE_24_PING_FENCE_WIRE[3],
    0,
    0,
    0,
    0,
];
const ISSUE_24_CAPTURE_PLAYER_LOW: u64 = 15;
const ISSUE_24_CAPTURE_PLAYER_HIGH: u64 = 0x0800_0400_0000_0000;
const ISSUE_24_CREATURE_START: WirePosition = WirePosition::new(
    (-10_118.333_f32).to_bits(),
    2_671.667_f32.to_bits(),
    218.490_f32.to_bits(),
);
const ISSUE_24_PLAYER_DESTINATION: WirePosition = WirePosition::new(
    (-10_118.333_f32).to_bits(),
    2_691.667_f32.to_bits(),
    218.490_f32.to_bits(),
);
const ISSUE_24_PLAYER_DESTINATION_ORIENTATION_BITS: u32 = (-std::f32::consts::FRAC_PI_2).to_bits();
const ISSUE_24_OBSTACLE_MIN_X: f32 = -10_123.333;
const ISSUE_24_OBSTACLE_MAX_X: f32 = -10_113.333;
const ISSUE_24_OBSTACLE_MIN_Y: f32 = 2_676.667;
const ISSUE_24_OBSTACLE_MAX_Y: f32 = 2_686.667;
const ISSUE_24_POSITION_EPSILON: f32 = 0.05;

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

/// Canonical semantic representation of a 3.4.3
/// `SMSG_SEND_KNOWN_SPELLS` body.
///
/// C++ fills both vectors while iterating `PlayerSpellMap`, an
/// `std::unordered_map`. Their wire order is therefore not a protocol
/// contract. The decoder sorts both unique lists while retaining exact
/// membership, cardinality, favorite membership, and every other body bit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendKnownSpellsBody {
    pub initial_login: bool,
    pub known_spells: Vec<u32>,
    pub favorite_spells: Vec<u32>,
}

/// One exact target location embedded in `SpellTargetData` or TargetPoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellTargetLocationBody {
    pub transport: ExactObjectGuid,
    pub position: WirePosition,
}

/// A spell GUID reference that is either exact or explicitly correlated to
/// the packet's Creature caster. Only exact equality with CasterGUID on that
/// same side produces `Caster`; arbitrary Creature targets remain exact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CorrelatedSpellGuidBody {
    Caster,
    Exact { guid: ExactObjectGuid },
}

/// Complete stable wire representation of `SpellTargetData`.
///
/// Floating-point values retain their exact IEEE-754 bits. The target name is
/// retained as bytes so the comparator never performs a lossy text
/// normalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellTargetDataBody {
    pub flags: u32,
    pub unit: CorrelatedSpellGuidBody,
    pub item: ExactObjectGuid,
    pub src_location: Option<SpellTargetLocationBody>,
    pub dst_location: Option<SpellTargetLocationBody>,
    pub orientation_bits: Option<u32>,
    pub map_id: Option<i32>,
    pub name: Vec<u8>,
}

/// One miss result. `reflect_status` is present only for
/// `SPELL_MISS_REFLECT`, exactly as in the C++ serializer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellMissStatusBody {
    pub reason: u8,
    pub reflect_status: Option<u8>,
}

/// One stable RemainingPower entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellPowerDataBody {
    pub cost: i32,
    pub power_type: i8,
}

/// Complete optional rune state carried by `SpellCastData`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellRuneDataBody {
    pub start: u8,
    pub count: u8,
    pub cooldowns: Vec<u8>,
}

/// Stable semantic representation of a complete 3.4.3 `SMSG_SPELL_GO` body.
///
/// C++ and Rust allocate the lower 40-bit counters of the Creature caster and
/// `CastID`, plus the wrapping `CastTime`, independently. The two GUIDs retain
/// every other identity field. `OriginalCastID` remains completely exact and
/// the creature-AI contract requires it to be EMPTY. Every other byte of
/// `SpellCastData`, including visual/flags, target data, hit/miss topology,
/// optional resource state, and the basic combat-log bit, is decoded and
/// compared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellGoBody {
    pub caster_guid: StableObjectGuid,
    pub caster_unit: StableObjectGuid,
    pub cast_id: StableObjectGuid,
    pub original_cast_id: ExactObjectGuid,
    pub spell_id: i32,
    pub spell_visual_id: i32,
    pub cast_flags: u32,
    pub cast_flags_ex: u32,
    pub missile_travel_time: u32,
    pub missile_pitch_bits: u32,
    pub dest_loc_spell_cast_index: u8,
    pub immunities_school: u32,
    pub immunities_value: u32,
    pub prediction_points: u32,
    pub prediction_type: u8,
    pub prediction_beacon: ExactObjectGuid,
    pub target: SpellTargetDataBody,
    pub hit_targets: Vec<CorrelatedSpellGuidBody>,
    pub miss_targets: Vec<CorrelatedSpellGuidBody>,
    pub miss_status: Vec<SpellMissStatusBody>,
    pub remaining_power: Vec<SpellPowerDataBody>,
    pub remaining_runes: Option<SpellRuneDataBody>,
    pub target_points: Vec<SpellTargetLocationBody>,
    pub ammo_display_id: Option<i32>,
    pub ammo_inventory_type: Option<i32>,
}

/// Stable `SMSG_SPELL_START` representation. It shares the complete
/// SpellCastData shape with [`SpellGoBody`], but START's CastTime is the cast
/// duration and therefore remains exact rather than normalized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellStartBody {
    pub cast: SpellGoBody,
    pub cast_time: u32,
}

/// Full `SMSG_SPELL_GO` decode, retaining the three explicitly normalized
/// exact GUIDs and timestamp for diagnostics and independent contract
/// validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedSpellGoBody {
    pub body: SpellGoBody,
    pub exact_caster_guid: ExactObjectGuid,
    pub exact_caster_unit: ExactObjectGuid,
    pub cast_id: ExactObjectGuid,
    pub cast_time: u32,
}

/// Full START decode with exact same-side GUIDs retained for START→GO
/// correlation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedSpellStartBody {
    pub body: SpellStartBody,
    pub exact_caster_guid: ExactObjectGuid,
    pub exact_caster_unit: ExactObjectGuid,
    pub cast_id: ExactObjectGuid,
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

/// Exact IEEE-754 wire bits for one XYZ value.
///
/// The capture comparator intentionally does not apply an epsilon. An epsilon
/// is used only by the independent fixture-containment checks in the required
/// issue-#24 contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WirePosition {
    pub x_bits: u32,
    pub y_bits: u32,
    pub z_bits: u32,
}

impl WirePosition {
    const fn new(x_bits: u32, y_bits: u32, z_bits: u32) -> Self {
        Self {
            x_bits,
            y_bits,
            z_bits,
        }
    }

    #[must_use]
    pub fn xyz(self) -> [f32; 3] {
        [
            f32::from_bits(self.x_bits),
            f32::from_bits(self.y_bits),
            f32::from_bits(self.z_bits),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterSplineFilterKeyBody {
    pub index: i16,
    pub speed: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterSplineFilterBody {
    pub base_speed_bits: u32,
    pub start_offset: i16,
    pub distance_to_previous_key_bits: u32,
    pub added_to_start: i16,
    pub keys: Vec<MonsterSplineFilterKeyBody>,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MonsterMoveFaceBody {
    Normal,
    Spot {
        position: WirePosition,
    },
    Target {
        direction_bits: u32,
        target: ExactObjectGuid,
    },
    Angle {
        direction_bits: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterSplineSpellEffectExtraBody {
    pub target: ExactObjectGuid,
    pub spell_visual_id: u32,
    pub progress_curve_id: u32,
    pub parabolic_curve_id: u32,
    pub jump_gravity_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterSplineJumpExtraBody {
    pub jump_gravity_bits: u32,
    pub start_time: u32,
    pub duration: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterSplineAnimTierTransitionBody {
    pub tier_transition_id: i32,
    pub start_time: u32,
    pub end_time: u32,
    pub animation_tier: u8,
}

/// Stable semantic representation of a complete 3.4.3
/// `SMSG_ON_MONSTER_MOVE` body.
///
/// The process-global spline ID is the only absent field. The fixture validator
/// pins the mover's lower 40-bit counter to its persistent C++ spawn GUID before
/// this representation is eligible for comparison. Every other decoded bit remains exact,
/// including packed-delta integers rather than their lossy reconstructed
/// floating-point values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterMoveBody {
    pub mover: StableObjectGuid,
    pub current_position: WirePosition,
    pub destination: WirePosition,
    pub crz_teleport: bool,
    pub stop_distance_tolerance: u8,
    pub flags: u32,
    pub elapsed: i32,
    pub move_time: u32,
    pub fade_object_time: u32,
    pub mode: u8,
    pub transport: ExactObjectGuid,
    pub vehicle_seat: i8,
    pub face: MonsterMoveFaceBody,
    pub vehicle_exit_voluntary: bool,
    pub interpolate: bool,
    pub points: Vec<WirePosition>,
    pub packed_deltas: Vec<u32>,
    pub spline_filter: Option<MonsterSplineFilterBody>,
    pub spell_effect_extra: Option<MonsterSplineSpellEffectExtraBody>,
    pub jump_extra: Option<MonsterSplineJumpExtraBody>,
    pub anim_tier_transition: Option<MonsterSplineAnimTierTransitionBody>,
}

/// Full decode result retaining the pinned mover counter and the intentionally
/// normalized spline allocation ID for independent bot/report validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedMonsterMoveBody {
    pub body: MonsterMoveBody,
    pub mover_runtime_counter: u64,
    pub spline_id: u32,
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
    pub send_known_spells: Option<SendKnownSpellsBody>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub spell_go: Option<SpellGoBody>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub spell_start: Option<SpellStartBody>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub update_object_inv_slots: Option<UpdateObjectInvSlotsBody>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub monster_move: Option<MonsterMoveBody>,
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
                    send_known_spells: None,
                    spell_go: None,
                    spell_start: None,
                    update_object_inv_slots: None,
                    monster_move: None,
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
                send_known_spells: None,
                spell_go: None,
                spell_start: None,
                update_object_inv_slots: None,
                monster_move: None,
                decode_error: None,
                raw_body_sha256: (!decoded.is_creature_kill()).then(|| raw_body_sha256(raw_body)),
            },
            Err(error) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                send_known_spells: None,
                spell_go: None,
                spell_start: None,
                update_object_inv_slots: None,
                monster_move: None,
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
                    send_known_spells: None,
                    spell_go: None,
                    spell_start: None,
                    update_object_inv_slots: None,
                    monster_move: None,
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
                send_known_spells: None,
                spell_go: None,
                spell_start: None,
                update_object_inv_slots: None,
                monster_move: None,
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
                    send_known_spells: None,
                    spell_go: None,
                    spell_start: None,
                    update_object_inv_slots: None,
                    monster_move: None,
                    decode_error: shape_error,
                    raw_body_sha256: (!reviewed_shape).then(|| raw_body_sha256(raw_body)),
                }
            }
            Err(error) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                send_known_spells: None,
                spell_go: None,
                spell_start: None,
                update_object_inv_slots: None,
                monster_move: None,
                decode_error: Some(error),
                raw_body_sha256: Some(raw_body_sha256(raw_body)),
            },
        }
    }

    fn from_decoded_send_known_spells(
        decoded: Result<SendKnownSpellsBody, String>,
        raw_body: &[u8],
    ) -> Self {
        match decoded {
            Ok(decoded) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                send_known_spells: Some(decoded),
                spell_go: None,
                spell_start: None,
                update_object_inv_slots: None,
                monster_move: None,
                decode_error: None,
                raw_body_sha256: None,
            },
            Err(error) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                send_known_spells: None,
                spell_go: None,
                spell_start: None,
                update_object_inv_slots: None,
                monster_move: None,
                decode_error: Some(error),
                raw_body_sha256: Some(raw_body_sha256(raw_body)),
            },
        }
    }

    fn from_decoded_spell_go(decoded: Result<DecodedSpellGoBody, String>, raw_body: &[u8]) -> Self {
        match decoded {
            Ok(decoded) => {
                // Keep this normalization specific to a unit Creature cast.
                // Player/item casts retain their raw digest and therefore stay
                // byte-exact even though they share SMSG_SPELL_GO.
                let creature_candidate = exact_guid_high_type(decoded.exact_caster_guid)
                    == HIGH_GUID_CREATURE
                    || exact_guid_high_type(decoded.exact_caster_unit) == HIGH_GUID_CREATURE;
                let shape_error = creature_candidate
                    .then(|| validate_decoded_creature_spell_go(&decoded).err())
                    .flatten();
                let creature_cast = creature_candidate && shape_error.is_none();
                Self {
                    log_xp_gain: None,
                    loot_removed: None,
                    buy_succeeded: None,
                    send_known_spells: None,
                    spell_go: Some(decoded.body),
                    spell_start: None,
                    update_object_inv_slots: None,
                    monster_move: None,
                    decode_error: shape_error,
                    raw_body_sha256: (!creature_cast).then(|| raw_body_sha256(raw_body)),
                }
            }
            Err(error) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                send_known_spells: None,
                spell_go: None,
                spell_start: None,
                update_object_inv_slots: None,
                monster_move: None,
                decode_error: Some(error),
                raw_body_sha256: Some(raw_body_sha256(raw_body)),
            },
        }
    }

    fn from_decoded_spell_start(
        decoded: Result<DecodedSpellStartBody, String>,
        raw_body: &[u8],
    ) -> Self {
        match decoded {
            Ok(decoded) => {
                let creature_candidate = exact_guid_high_type(decoded.exact_caster_guid)
                    == HIGH_GUID_CREATURE
                    || exact_guid_high_type(decoded.exact_caster_unit) == HIGH_GUID_CREATURE;
                let shape_error = creature_candidate
                    .then(|| {
                        validate_creature_spell_cast_shape(
                            &decoded.body.cast,
                            decoded.exact_caster_guid,
                            decoded.exact_caster_unit,
                            decoded.cast_id,
                        )
                        .err()
                    })
                    .flatten();
                let creature_cast = creature_candidate && shape_error.is_none();
                Self {
                    log_xp_gain: None,
                    loot_removed: None,
                    buy_succeeded: None,
                    send_known_spells: None,
                    spell_go: None,
                    spell_start: Some(decoded.body),
                    update_object_inv_slots: None,
                    monster_move: None,
                    decode_error: shape_error,
                    raw_body_sha256: (!creature_cast).then(|| raw_body_sha256(raw_body)),
                }
            }
            Err(error) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                send_known_spells: None,
                spell_go: None,
                spell_start: None,
                update_object_inv_slots: None,
                monster_move: None,
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
                send_known_spells: None,
                spell_go: None,
                spell_start: None,
                update_object_inv_slots: Some(decoded.body),
                monster_move: None,
                decode_error: None,
                raw_body_sha256: None,
            },
            UpdateObjectInvSlotsDecode::NotEligible(reason) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                send_known_spells: None,
                spell_go: None,
                spell_start: None,
                update_object_inv_slots: None,
                monster_move: None,
                decode_error: Some(format!(
                    "not the reviewed single-player InvSlots VALUES shape: {reason}"
                )),
                raw_body_sha256: Some(raw_body_sha256(raw_body)),
            },
            UpdateObjectInvSlotsDecode::Malformed(error) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                send_known_spells: None,
                spell_go: None,
                spell_start: None,
                update_object_inv_slots: None,
                monster_move: None,
                decode_error: Some(error),
                raw_body_sha256: Some(raw_body_sha256(raw_body)),
            },
        }
    }

    fn from_decoded_monster_move(
        decoded: Result<DecodedMonsterMoveBody, String>,
        raw_body: &[u8],
        fixture_eligible: bool,
    ) -> Self {
        match decoded {
            Ok(decoded) => {
                let fixture_error = (!fixture_eligible).then(|| {
                    validate_detour_chase_monster_move(&decoded)
                        .expect_err("ineligible decoded movement must fail the fixture contract")
                });
                Self {
                    log_xp_gain: None,
                    loot_removed: None,
                    buy_succeeded: None,
                    send_known_spells: None,
                    spell_go: None,
                    spell_start: None,
                    update_object_inv_slots: None,
                    monster_move: Some(decoded.body),
                    decode_error: fixture_error
                        .map(|error| format!("not the reviewed issue-#24 movement: {error}")),
                    // The whole independently validated fixture contract,
                    // including the exact persistent spawn counter, is required
                    // before omitting the process-local spline allocation ID.
                    raw_body_sha256: (!fixture_eligible).then(|| raw_body_sha256(raw_body)),
                }
            }
            Err(error) => Self {
                log_xp_gain: None,
                loot_removed: None,
                buy_succeeded: None,
                send_known_spells: None,
                spell_go: None,
                spell_start: None,
                update_object_inv_slots: None,
                monster_move: None,
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
            && self.cpp.send_known_spells == self.rust.send_known_spells
            && self.cpp.spell_go == self.rust.spell_go
            && self.cpp.spell_start == self.rust.spell_start
            && self.cpp.update_object_inv_slots == self.rust.update_object_inv_slots
            && self.cpp.monster_move == self.rust.monster_move
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
            self.cpp.send_known_spells.as_ref(),
            self.rust.send_known_spells.as_ref(),
        ) {
            return mismatch_send_known_spells(cpp, rust);
        }

        if let (Some(cpp), Some(rust)) = (self.cpp.spell_go.as_ref(), self.rust.spell_go.as_ref()) {
            return mismatch_spell_go(cpp, rust, &self.cpp, &self.rust);
        }

        if let (Some(cpp), Some(rust)) = (
            self.cpp.spell_start.as_ref(),
            self.rust.spell_start.as_ref(),
        ) {
            if cpp.cast_time != rust.cast_time {
                return "mismatched field(s): cast_time".to_string();
            }
            return mismatch_spell_go(&cpp.cast, &rust.cast, &self.cpp, &self.rust);
        }

        if let (Some(cpp), Some(rust)) = (
            self.cpp.update_object_inv_slots.as_ref(),
            self.rust.update_object_inv_slots.as_ref(),
        ) {
            return mismatch_update_object_inv_slots(cpp, rust);
        }

        if let (Some(cpp), Some(rust)) = (
            self.cpp.monster_move.as_ref(),
            self.rust.monster_move.as_ref(),
        ) {
            return mismatch_monster_move(cpp, rust, &self.cpp, &self.rust);
        }

        "semantic body shape differs or is missing".to_string()
    }
}

fn mismatch_monster_move(
    cpp: &MonsterMoveBody,
    rust: &MonsterMoveBody,
    cpp_side: &SemanticBodySide,
    rust_side: &SemanticBodySide,
) -> String {
    let mut fields = Vec::new();
    if cpp.mover != rust.mover {
        fields.push("mover");
    }
    if cpp.current_position != rust.current_position {
        fields.push("current_position");
    }
    if cpp.destination != rust.destination {
        fields.push("destination");
    }
    if cpp.crz_teleport != rust.crz_teleport {
        fields.push("crz_teleport");
    }
    if cpp.stop_distance_tolerance != rust.stop_distance_tolerance {
        fields.push("stop_distance_tolerance");
    }
    if cpp.flags != rust.flags {
        fields.push("flags");
    }
    if cpp.elapsed != rust.elapsed {
        fields.push("elapsed");
    }
    if cpp.move_time != rust.move_time {
        fields.push("move_time");
    }
    if cpp.fade_object_time != rust.fade_object_time {
        fields.push("fade_object_time");
    }
    if cpp.mode != rust.mode {
        fields.push("mode");
    }
    if cpp.transport != rust.transport {
        fields.push("transport");
    }
    if cpp.vehicle_seat != rust.vehicle_seat {
        fields.push("vehicle_seat");
    }
    if cpp.face != rust.face {
        fields.push("face");
    }
    if cpp.vehicle_exit_voluntary != rust.vehicle_exit_voluntary {
        fields.push("vehicle_exit_voluntary");
    }
    if cpp.interpolate != rust.interpolate {
        fields.push("interpolate");
    }
    if cpp.points != rust.points {
        fields.push("points");
    }
    if cpp.packed_deltas != rust.packed_deltas {
        fields.push("packed_deltas");
    }
    if cpp.spline_filter != rust.spline_filter {
        fields.push("spline_filter");
    }
    if cpp.spell_effect_extra != rust.spell_effect_extra {
        fields.push("spell_effect_extra");
    }
    if cpp.jump_extra != rust.jump_extra {
        fields.push("jump_extra");
    }
    if cpp.anim_tier_transition != rust.anim_tier_transition {
        fields.push("anim_tier_transition");
    }

    if fields.is_empty() {
        if cpp_side.raw_body_sha256 == rust_side.raw_body_sha256 {
            "semantic values are equal".to_string()
        } else {
            "raw body identity differs outside the reviewed issue-#24 fixture shape".to_string()
        }
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
    }
}

fn mismatch_send_known_spells(cpp: &SendKnownSpellsBody, rust: &SendKnownSpellsBody) -> String {
    let mut fields = Vec::new();
    if cpp.initial_login != rust.initial_login {
        fields.push("initial_login");
    }
    if cpp.known_spells != rust.known_spells {
        fields.push("known_spells");
    }
    if cpp.favorite_spells != rust.favorite_spells {
        fields.push("favorite_spells");
    }

    if fields.is_empty() {
        "semantic values are equal".to_string()
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
    }
}

fn mismatch_spell_go(
    cpp: &SpellGoBody,
    rust: &SpellGoBody,
    cpp_side: &SemanticBodySide,
    rust_side: &SemanticBodySide,
) -> String {
    let mut fields = Vec::new();
    if cpp.caster_guid != rust.caster_guid {
        fields.push("caster_guid");
    }
    if cpp.caster_unit != rust.caster_unit {
        fields.push("caster_unit");
    }
    if cpp.cast_id != rust.cast_id {
        fields.push("cast_id.identity");
    }
    if cpp.original_cast_id != rust.original_cast_id {
        fields.push("original_cast_id");
    }
    if cpp.spell_id != rust.spell_id {
        fields.push("spell_id");
    }
    if cpp.spell_visual_id != rust.spell_visual_id {
        fields.push("spell_visual_id");
    }
    if cpp.cast_flags != rust.cast_flags {
        fields.push("cast_flags");
    }
    if cpp.cast_flags_ex != rust.cast_flags_ex {
        fields.push("cast_flags_ex");
    }
    if cpp.missile_travel_time != rust.missile_travel_time
        || cpp.missile_pitch_bits != rust.missile_pitch_bits
    {
        fields.push("missile_trajectory");
    }
    if cpp.dest_loc_spell_cast_index != rust.dest_loc_spell_cast_index {
        fields.push("dest_loc_spell_cast_index");
    }
    if cpp.immunities_school != rust.immunities_school
        || cpp.immunities_value != rust.immunities_value
    {
        fields.push("immunities");
    }
    if cpp.prediction_points != rust.prediction_points
        || cpp.prediction_type != rust.prediction_type
        || cpp.prediction_beacon != rust.prediction_beacon
    {
        fields.push("prediction");
    }
    if cpp.target != rust.target {
        fields.push("target");
    }
    if cpp.hit_targets != rust.hit_targets {
        fields.push("hit_targets");
    }
    if cpp.miss_targets != rust.miss_targets {
        fields.push("miss_targets");
    }
    if cpp.miss_status != rust.miss_status {
        fields.push("miss_status");
    }
    if cpp.remaining_power != rust.remaining_power {
        fields.push("remaining_power");
    }
    if cpp.remaining_runes != rust.remaining_runes {
        fields.push("remaining_runes");
    }
    if cpp.target_points != rust.target_points {
        fields.push("target_points");
    }
    if cpp.ammo_display_id != rust.ammo_display_id {
        fields.push("ammo_display_id");
    }
    if cpp.ammo_inventory_type != rust.ammo_inventory_type {
        fields.push("ammo_inventory_type");
    }
    if fields.is_empty() {
        if cpp_side.raw_body_sha256 == rust_side.raw_body_sha256 {
            "semantic values are equal".to_string()
        } else {
            "raw body identity differs outside the reviewed Creature-cast shape".to_string()
        }
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
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
        SMSG_SEND_KNOWN_SPELLS => Some(compare_send_known_spells_bodies(cpp, rust)),
        SMSG_SPELL_GO => Some(compare_spell_go_bodies(cpp, rust)),
        SMSG_SPELL_START => Some(compare_spell_start_bodies(cpp, rust)),
        SMSG_UPDATE_OBJECT => compare_update_object_inv_slots_bodies(cpp, rust),
        SMSG_ON_MONSTER_MOVE => compare_monster_move_bodies(cpp, rust),
        _ => None,
    }
}

fn compare_spell_go_bodies(cpp: &[u8], rust: &[u8]) -> SemanticBodyDiff {
    SemanticBodyDiff {
        comparator: "smsg_spell_go_creature_runtime_counters_and_cast_time".to_string(),
        cpp: SemanticBodySide::from_decoded_spell_go(decode_spell_go_body(cpp), cpp),
        rust: SemanticBodySide::from_decoded_spell_go(decode_spell_go_body(rust), rust),
    }
}

fn compare_spell_start_bodies(cpp: &[u8], rust: &[u8]) -> SemanticBodyDiff {
    SemanticBodyDiff {
        comparator: "smsg_spell_start_creature_runtime_counters".to_string(),
        cpp: SemanticBodySide::from_decoded_spell_start(decode_spell_start_body(cpp), cpp),
        rust: SemanticBodySide::from_decoded_spell_start(decode_spell_start_body(rust), rust),
    }
}

fn compare_monster_move_bodies(cpp: &[u8], rust: &[u8]) -> Option<SemanticBodyDiff> {
    let mut cpp_decoded = decode_monster_move_body(cpp);
    let mut rust_decoded = decode_monster_move_body(rust);
    let cpp_is_legacy_fixture = cpp_decoded
        .as_ref()
        .is_ok_and(|decoded| validate_legacy_cpp_detour_chase_monster_move(decoded).is_ok());
    let cpp_is_repaired_fixture = cpp_decoded
        .as_ref()
        .is_ok_and(|decoded| validate_detour_chase_monster_move(decoded).is_ok());
    let cpp_is_fixture = cpp_is_legacy_fixture || cpp_is_repaired_fixture;
    let rust_is_repaired_fixture = rust_decoded
        .as_ref()
        .is_ok_and(|decoded| validate_detour_chase_monster_move(decoded).is_ok());
    let rust_is_legacy_fixture = rust_decoded
        .as_ref()
        .is_ok_and(|decoded| validate_legacy_cpp_detour_chase_monster_move(decoded).is_ok());
    let rust_is_fixture = rust_is_repaired_fixture || rust_is_legacy_fixture;

    // This comparator is intentionally not a generic movement normalization.
    // If neither side satisfies the complete issue-#24 identity, position,
    // topology, and facing contract, ordinary byte comparison remains
    // authoritative.
    if !cpp_is_fixture && !rust_is_fixture {
        return None;
    }

    if cpp_is_fixture && rust_is_fixture {
        for decoded in [&mut cpp_decoded, &mut rust_decoded]
            .into_iter()
            .filter_map(|decoded| decoded.as_mut().ok())
        {
            decoded.body.current_position = ISSUE_24_CREATURE_START;
            decoded.body.move_time = 0;
            decoded.body.points.clear();
            decoded.body.packed_deltas.clear();
            if let MonsterMoveFaceBody::Target { direction_bits, .. } = &mut decoded.body.face {
                // The direction is derived from the final segment. The reviewed
                // repair may route around the opposite side, but the exact
                // target GUID remains authoritative.
                *direction_bits = 0;
            }
        }
    }

    Some(SemanticBodyDiff {
        comparator: "smsg_on_monster_move_issue_24_process_local_route_fields".to_string(),
        cpp: SemanticBodySide::from_decoded_monster_move(cpp_decoded, cpp, cpp_is_fixture),
        rust: SemanticBodySide::from_decoded_monster_move(rust_decoded, rust, rust_is_fixture),
    })
}

fn compare_send_known_spells_bodies(cpp: &[u8], rust: &[u8]) -> SemanticBodyDiff {
    SemanticBodyDiff {
        comparator: "smsg_send_known_spells_unordered_spell_sets".to_string(),
        cpp: SemanticBodySide::from_decoded_send_known_spells(
            decode_send_known_spells_body(cpp),
            cpp,
        ),
        rust: SemanticBodySide::from_decoded_send_known_spells(
            decode_send_known_spells_body(rust),
            rust,
        ),
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

/// Reconstruct the compressed linear spline represented by one MonsterMove.
///
/// C++ emits only the final point plus signed quarter-yard deltas from the
/// midpoint of the first/current and final points. The returned vector is
/// `[current, intermediates..., final]`.
pub fn reconstruct_monster_move_path(body: &MonsterMoveBody) -> Result<Vec<[f32; 3]>, String> {
    let [final_point] = body.points.as_slice() else {
        return Err(format!(
            "compressed issue-#24 movement requires exactly one endpoint, found {}",
            body.points.len()
        ));
    };
    let start = body.current_position.xyz();
    let end = final_point.xyz();
    let middle = [
        (start[0] + end[0]) * 0.5,
        (start[1] + end[1]) * 0.5,
        (start[2] + end[2]) * 0.5,
    ];
    let mut path = Vec::with_capacity(body.packed_deltas.len() + 2);
    path.push(start);
    for packed in &body.packed_deltas {
        let delta = unpack_monster_move_delta(*packed);
        path.push([
            middle[0] - delta[0],
            middle[1] - delta[1],
            middle[2] - delta[2],
        ]);
    }
    path.push(end);
    Ok(path)
}

/// Validate one issue-#24 movement body independently of cross-runtime diff.
///
/// This proves that the packet belongs to the reserved map-1 Tender fixture
/// and carries a compressed path around (not through) the declared missing
/// navmesh square.
pub fn validate_detour_chase_monster_move(
    decoded: &DecodedMonsterMoveBody,
) -> Result<Vec<[f32; 3]>, String> {
    validate_detour_chase_monster_move_for_side(decoded, false)
}

pub fn validate_legacy_cpp_detour_chase_monster_move(
    decoded: &DecodedMonsterMoveBody,
) -> Result<Vec<[f32; 3]>, String> {
    validate_detour_chase_monster_move_for_side(decoded, true)
}

fn validate_detour_chase_monster_move_for_side(
    decoded: &DecodedMonsterMoveBody,
    legacy_cpp: bool,
) -> Result<Vec<[f32; 3]>, String> {
    let movement = &decoded.body;
    if movement.mover != ISSUE_24_CREATURE_IDENTITY {
        return Err(format!(
            "MonsterMove mover {:?} is not the issue-#24 fixture {:?}",
            movement.mover, ISSUE_24_CREATURE_IDENTITY
        ));
    }
    if decoded.mover_runtime_counter == 0 {
        return Err("issue-#24 fixture movement has zero runtime GUID counter".to_string());
    }
    if decoded.spline_id == 0 {
        return Err("issue-#24 fixture movement has zero spline ID".to_string());
    }
    require_position_near(
        movement.current_position,
        ISSUE_24_CREATURE_START,
        ISSUE_24_POSITION_EPSILON,
        "MonsterMove current position",
    )?;
    if movement.destination != WirePosition::new(0, 0, 0) {
        return Err(format!(
            "outer MovementMonsterSpline destination is {:?}, expected exact positive-zero C++ default",
            movement.destination
        ));
    }
    if movement.crz_teleport
        || movement.stop_distance_tolerance != 0
        || movement.flags & 0x0040_0000 != 0
        || movement.elapsed != 0
        || movement.move_time == 0
        || movement.fade_object_time != 0
        || movement.mode != 0
        || movement.transport != (ExactObjectGuid { low: 0, high: 0 })
        || movement.vehicle_seat != -1
        || movement.vehicle_exit_voluntary
        || movement.interpolate
        || movement.spline_filter.is_some()
        || movement.spell_effect_extra.is_some()
        || movement.jump_extra.is_some()
        || movement.anim_tier_transition.is_some()
    {
        return Err(
            "MonsterMove is not the plain, compressed, non-transport chase shape".to_string(),
        );
    }
    let MonsterMoveFaceBody::Target {
        direction_bits,
        target,
    } = &movement.face
    else {
        return Err("MonsterMove does not face the chase target like C++".to_string());
    };
    if *target
        != (ExactObjectGuid {
            low: ISSUE_24_CAPTURE_PLAYER_LOW,
            high: ISSUE_24_CAPTURE_PLAYER_HIGH,
        })
    {
        return Err(format!(
            "MonsterMove facing target {target:?} is not disposable character 15 in realm 1"
        ));
    }
    let direction = f32::from_bits(*direction_bits);
    if (direction - std::f32::consts::FRAC_PI_2).abs() > 0.05 {
        return Err(format!(
            "MonsterMove target-facing direction {direction} is not approximately +pi/2"
        ));
    }
    if movement.packed_deltas.is_empty() {
        return Err(
            "MonsterMove has no packed intermediate; a straight path cannot prove the detour"
                .to_string(),
        );
    }

    let path = reconstruct_monster_move_path(movement)?;
    let destination = ISSUE_24_PLAYER_DESTINATION.xyz();
    let endpoint = *path
        .last()
        .expect("reconstruction always includes start and endpoint");
    let endpoint_distance_2d =
        ((endpoint[0] - destination[0]).powi(2) + (endpoint[1] - destination[1]).powi(2)).sqrt();
    let endpoint_distance_3d =
        (endpoint_distance_2d.powi(2) + (endpoint[2] - destination[2]).powi(2)).sqrt();
    let endpoint_invalid = if legacy_cpp {
        endpoint_distance_2d > 6.0
            || destination[2] - endpoint[2] < 20.0
            || (endpoint[2] - 190.721_01).abs() > 1.0
    } else {
        endpoint_distance_3d > 6.0
    };
    if endpoint[1] <= ISSUE_24_OBSTACLE_MAX_Y || endpoint_invalid {
        return Err(format!(
            "{} chase endpoint {endpoint:?} does not satisfy the reviewed destination contract for {destination:?}",
            if legacy_cpp {
                "legacy C++"
            } else {
                "repaired Rust"
            }
        ));
    }
    if !segment_intersects_issue_24_obstacle(path[0], destination) {
        return Err(
            "fixture start-to-player line does not cross the declared obstacle".to_string(),
        );
    }
    if !path[1..path.len() - 1].iter().any(|point| {
        point[0] < ISSUE_24_OBSTACLE_MIN_X - 0.1 || point[0] > ISSUE_24_OBSTACLE_MAX_X + 0.1
    }) {
        return Err(format!(
            "compressed path {path:?} has no lateral bend outside the obstacle"
        ));
    }
    if path
        .windows(2)
        .any(|segment| segment_intersects_issue_24_obstacle(segment[0], segment[1]))
    {
        return Err(format!(
            "compressed path {path:?} intersects the missing navmesh square"
        ));
    }
    Ok(path)
}

/// Validate the complete three-packet issue-#24 capture contract.
pub fn validate_detour_chase_capture(capture: &Capture) -> Result<(), String> {
    validate_detour_chase_capture_for_side(capture, false)
}

pub fn validate_legacy_cpp_detour_chase_capture(capture: &Capture) -> Result<(), String> {
    validate_detour_chase_capture_for_side(capture, true)
}

fn validate_detour_chase_capture_for_side(
    capture: &Capture,
    legacy_cpp: bool,
) -> Result<(), String> {
    const EXPECTED: [(Direction, u32, u16); 3] = [
        (Direction::C2S, 1, CMSG_MOVE_HEARTBEAT),
        (Direction::S2C, 1, SMSG_ON_MONSTER_MOVE),
        (Direction::C2S, 1, CMSG_PING),
    ];
    if capture.packets.len() != EXPECTED.len() {
        return Err(format!(
            "{} contains {} packet(s); detour-chase contract requires exactly {}",
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
                "{} packet {index} is {} conn={} 0x{:04X}; detour contract requires {} conn={} 0x{opcode:04X}",
                capture.source,
                packet.direction,
                packet.connection_id,
                packet.opcode,
                direction,
                connection_id
            ));
        }
    }
    validate_issue_24_heartbeat(&capture.packets[0].body)?;
    let movement = decode_monster_move_body(&capture.packets[1].body)?;
    if legacy_cpp {
        validate_legacy_cpp_detour_chase_monster_move(&movement)?;
    } else {
        validate_detour_chase_monster_move(&movement)?;
    }
    if capture.packets[2].body != ISSUE_24_PING_BODY {
        return Err(format!(
            "CMSG_PING fence body is {:02X?}, expected fixed DTOR/zero-latency body {:02X?}",
            capture.packets[2].body, ISSUE_24_PING_BODY
        ));
    }
    Ok(())
}

fn validate_issue_24_heartbeat(body: &[u8]) -> Result<(), String> {
    let mut cursor = 0usize;
    let (player_low, player_high) = read_packed_guid(body, &mut cursor, "MovementInfo.Guid")?;
    if (player_low, player_high) != (ISSUE_24_CAPTURE_PLAYER_LOW, ISSUE_24_CAPTURE_PLAYER_HIGH) {
        return Err(format!(
            "heartbeat player GUID is ({player_low:#018X}, {player_high:#018X}), expected disposable character 15 in realm 1"
        ));
    }
    for label in ["MovementFlags", "MovementFlags2", "MovementFlags3"] {
        if read_u32(body, &mut cursor, label)? != 0 {
            return Err(format!("{label} is nonzero in deterministic heartbeat"));
        }
    }
    if read_u32(body, &mut cursor, "Time")? != 0 {
        return Err("heartbeat client time is not deterministic zero".to_string());
    }
    let position = read_wire_position(body, &mut cursor, "Position")?;
    if position != ISSUE_24_PLAYER_DESTINATION {
        return Err(format!(
            "heartbeat destination {:?} differs from fixture destination {:?}",
            position.xyz(),
            ISSUE_24_PLAYER_DESTINATION.xyz()
        ));
    }
    let orientation = read_f32_bits(body, &mut cursor, "Orientation")?;
    if orientation != ISSUE_24_PLAYER_DESTINATION_ORIENTATION_BITS {
        return Err(format!(
            "heartbeat orientation is 0x{orientation:08X}, expected fixture orientation 0x{ISSUE_24_PLAYER_DESTINATION_ORIENTATION_BITS:08X}"
        ));
    }
    for label in ["Pitch", "StepUpStartElevation"] {
        if read_f32_bits(body, &mut cursor, label)? != 0 {
            return Err(format!("{label} is nonzero in deterministic heartbeat"));
        }
    }
    if read_u32(body, &mut cursor, "RemoveMovementForcesCount")? != 0
        || read_u32(body, &mut cursor, "MoveIndex")? != 0
    {
        return Err("heartbeat force count or move index is nonzero".to_string());
    }
    let optional_bits = read_u8(body, &mut cursor, "MovementInfo optional bit byte")?;
    if optional_bits != 0 {
        return Err(format!(
            "heartbeat optional movement bit byte is 0x{optional_bits:02X}, expected zero"
        ));
    }
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after deterministic heartbeat: decoded {cursor} of {}",
            body.len()
        ));
    }
    Ok(())
}

/// Validate the semantic payload boundary for the future
/// `creature-spell-casting-v1` required flow.
///
/// Exact outer action boundaries and socket routing remain the responsibility
/// of the requirement manifest. This independent payload contract requires
/// exactly one adjacent server `SpellStart -> SpellGo` pair, correlated
/// Creature CasterGUID/CasterUnit and CastID values, identical spell/visual/
/// target data across that pair, a normal Cast GUID whose stable identity
/// matches realm/map/spell, EMPTY OriginalCastID, and complete fail-closed
/// SpellCastData decoding on both packets.
pub fn validate_creature_spell_casting_capture(capture: &Capture) -> Result<(), String> {
    let spell_starts = capture
        .packets
        .iter()
        .enumerate()
        .filter(|(_, packet)| packet.opcode == SMSG_SPELL_START)
        .collect::<Vec<_>>();
    let spell_goes = capture
        .packets
        .iter()
        .enumerate()
        .filter(|(_, packet)| packet.opcode == SMSG_SPELL_GO)
        .collect::<Vec<_>>();
    let [(start_index, start_packet)] = spell_starts.as_slice() else {
        return Err(format!(
            "{} contains {} SMSG_SPELL_START packet(s); creature-spell-casting-v1 requires exactly one",
            capture.source,
            spell_starts.len()
        ));
    };
    let [(go_index, go_packet)] = spell_goes.as_slice() else {
        return Err(format!(
            "{} contains {} SMSG_SPELL_GO packet(s); creature-spell-casting-v1 requires exactly one",
            capture.source,
            spell_goes.len()
        ));
    };
    if start_packet.direction != Direction::S2C || go_packet.direction != Direction::S2C {
        return Err(format!(
            "{} carries SpellStart/SpellGo on {}/{}, expected s2c/s2c",
            capture.source, start_packet.direction, go_packet.direction
        ));
    }
    if *go_index != *start_index + 1 {
        return Err(format!(
            "{} does not contain adjacent SMSG_SPELL_START -> SMSG_SPELL_GO (indices {start_index} and {go_index})",
            capture.source
        ));
    }

    let start = decode_spell_start_body(&start_packet.body)?;
    let go = decode_spell_go_body(&go_packet.body)?;
    validate_creature_spell_cast_shape(
        &start.body.cast,
        start.exact_caster_guid,
        start.exact_caster_unit,
        start.cast_id,
    )?;
    validate_decoded_creature_spell_go(&go)?;
    validate_issue_26_creature_spell_start(&start)?;
    validate_issue_26_creature_spell_go(&go)?;

    let mut mismatches = Vec::new();
    if start.exact_caster_guid != go.exact_caster_guid
        || start.exact_caster_unit != go.exact_caster_unit
    {
        mismatches.push("caster");
    }
    if start.cast_id != go.cast_id {
        mismatches.push("cast_id");
    }
    if start.body.cast.spell_id != go.body.spell_id {
        mismatches.push("spell_id");
    }
    if start.body.cast.spell_visual_id != go.body.spell_visual_id {
        mismatches.push("spell_visual_id");
    }
    if start.body.cast.target != go.body.target {
        mismatches.push("target");
    }
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "SMSG_SPELL_START and SMSG_SPELL_GO do not correlate field(s): {}",
            mismatches.join(", ")
        ))
    }
}

fn validate_issue_26_creature_spell_start(start: &DecodedSpellStartBody) -> Result<(), String> {
    validate_issue_26_creature_spell_common(&start.body.cast, "SMSG_SPELL_START")?;
    if start.body.cast.cast_flags != ISSUE_26_START_CAST_FLAGS {
        return Err(format!(
            "SMSG_SPELL_START CastFlags is 0x{:08X}, expected issue-#26 fixture value 0x{ISSUE_26_START_CAST_FLAGS:08X}",
            start.body.cast.cast_flags
        ));
    }
    if start.body.cast_time != 0 {
        return Err(format!(
            "SMSG_SPELL_START CastTime is {}, expected instant issue-#26 fixture value 0",
            start.body.cast_time
        ));
    }
    if !start.body.cast.hit_targets.is_empty() {
        return Err(
            "SMSG_SPELL_START contains HitTargets; issue-#26 requires result topology only in SMSG_SPELL_GO"
                .to_string(),
        );
    }
    Ok(())
}

fn validate_issue_26_creature_spell_go(go: &DecodedSpellGoBody) -> Result<(), String> {
    let victim = validate_issue_26_creature_spell_common(&go.body, "SMSG_SPELL_GO")?;
    if go.body.cast_flags != ISSUE_26_GO_CAST_FLAGS {
        return Err(format!(
            "SMSG_SPELL_GO CastFlags is 0x{:08X}, expected issue-#26 fixture value 0x{ISSUE_26_GO_CAST_FLAGS:08X}",
            go.body.cast_flags
        ));
    }
    let expected_hits = [CorrelatedSpellGuidBody::Exact { guid: victim }];
    if go.body.hit_targets.as_slice() != expected_hits {
        return Err(
            "SMSG_SPELL_GO HitTargets is not exactly the explicit player victim once".to_string(),
        );
    }
    Ok(())
}

fn validate_issue_26_creature_spell_common(
    body: &SpellGoBody,
    packet: &str,
) -> Result<ExactObjectGuid, String> {
    let caster = body.caster_guid;
    if caster.high_type != HIGH_GUID_CREATURE
        || caster.realm_id != ISSUE_26_REALM_ID
        || caster.map_id != ISSUE_26_MAP_ID
        || caster.entry != ISSUE_26_CREATURE_ENTRY
        || caster.subtype != 0
        || caster.server_id != 0
    {
        return Err(format!(
            "{packet} caster identity {:?} is not issue-#26 Cabal Interrogator entry {ISSUE_26_CREATURE_ENTRY} on realm {ISSUE_26_REALM_ID}, map {ISSUE_26_MAP_ID}",
            caster
        ));
    }
    if body.spell_id != ISSUE_26_SPELL_ID {
        return Err(format!(
            "{packet} SpellID is {}, expected issue-#26 Eviscerate {ISSUE_26_SPELL_ID}",
            body.spell_id
        ));
    }
    if body.spell_visual_id != ISSUE_26_SPELL_X_SPELL_VISUAL_ID {
        return Err(format!(
            "{packet} SpellXSpellVisualID is {}, expected issue-#26 fixture value {ISSUE_26_SPELL_X_SPELL_VISUAL_ID}",
            body.spell_visual_id
        ));
    }
    if body.cast_flags_ex != 0 {
        return Err(format!(
            "{packet} CastFlagsEx is 0x{:08X}, expected 0",
            body.cast_flags_ex
        ));
    }
    if body.missile_travel_time != 0 || body.missile_pitch_bits != 0.0_f32.to_bits() {
        return Err(format!(
            "{packet} carries a missile trajectory; issue-#26 Eviscerate is zero-speed"
        ));
    }
    if body.dest_loc_spell_cast_index != 0 {
        return Err(format!(
            "{packet} DestLocSpellCastIndex is {}, expected 0",
            body.dest_loc_spell_cast_index
        ));
    }
    if body.immunities_school != 0 || body.immunities_value != 0 {
        return Err(format!(
            "{packet} carries creature immunities; issue-#26 fixture expects none"
        ));
    }
    if body.prediction_points != 0
        || body.prediction_type != 0
        || body.prediction_beacon != (ExactObjectGuid { low: 0, high: 0 })
    {
        return Err(format!(
            "{packet} carries heal prediction; issue-#26 fixture expects none"
        ));
    }

    let CorrelatedSpellGuidBody::Exact { guid: victim } = &body.target.unit else {
        return Err(format!(
            "{packet} explicit unit target is not a distinct player victim"
        ));
    };
    let victim = *victim;
    let stable_victim = stable_object_guid(victim.low, victim.high);
    if exact_guid_high_type(victim) != HIGH_GUID_PLAYER
        || stable_victim.realm_id != ISSUE_26_REALM_ID
        || stable_victim.map_id != 0
        || stable_victim.entry != 0
        || stable_victim.subtype != 0
        || stable_victim.server_id != 0
        || victim.low & OBJECT_GUID_COUNTER_MASK != ISSUE_26_PLAYER_COUNTER
    {
        return Err(format!(
            "{packet} explicit unit target {victim:?} is not canonical issue-#26 realm-{ISSUE_26_REALM_ID} Player {ISSUE_26_PLAYER_COUNTER}"
        ));
    }
    let target = &body.target;
    if target.flags != ISSUE_26_UNIT_TARGET_FLAGS
        || target.item != (ExactObjectGuid { low: 0, high: 0 })
        || target.src_location.is_some()
        || target.dst_location.is_some()
        || target.orientation_bits.is_some()
        || target.map_id.is_some()
        || !target.name.is_empty()
    {
        return Err(format!(
            "{packet} SpellTargetData is not the exact issue-#26 unit-only target topology"
        ));
    }
    if !body.miss_targets.is_empty() || !body.miss_status.is_empty() {
        return Err(format!(
            "{packet} carries miss topology; issue-#26 fixture requires a successful hit"
        ));
    }
    if !body.remaining_power.is_empty()
        || body.remaining_runes.is_some()
        || !body.target_points.is_empty()
        || body.ammo_display_id.is_some()
        || body.ammo_inventory_type.is_some()
    {
        return Err(format!(
            "{packet} carries power/rune/target-point/ammo optionals absent from issue-#26 Eviscerate"
        ));
    }
    Ok(victim)
}

fn require_position_near(
    actual: WirePosition,
    expected: WirePosition,
    epsilon: f32,
    label: &str,
) -> Result<(), String> {
    let actual = actual.xyz();
    let expected = expected.xyz();
    if actual
        .into_iter()
        .zip(expected)
        .any(|(actual, expected)| (actual - expected).abs() > epsilon)
    {
        return Err(format!(
            "{label} {actual:?} differs from expected {expected:?} by more than {epsilon}"
        ));
    }
    Ok(())
}

fn segment_intersects_issue_24_obstacle(start: [f32; 3], end: [f32; 3]) -> bool {
    // Shrink the open rectangle slightly so a Detour segment along its exact
    // boundary is accepted while a segment through its interior fails.
    let min_x = ISSUE_24_OBSTACLE_MIN_X + 0.05;
    let max_x = ISSUE_24_OBSTACLE_MAX_X - 0.05;
    let min_y = ISSUE_24_OBSTACLE_MIN_Y + 0.05;
    let max_y = ISSUE_24_OBSTACLE_MAX_Y - 0.05;
    let mut t_min = 0.0f32;
    let mut t_max = 1.0f32;
    for (origin, delta, min, max) in [
        (start[0], end[0] - start[0], min_x, max_x),
        (start[1], end[1] - start[1], min_y, max_y),
    ] {
        if delta.abs() <= f32::EPSILON {
            if origin <= min || origin >= max {
                return false;
            }
            continue;
        }
        let first = (min - origin) / delta;
        let second = (max - origin) / delta;
        let enter = first.min(second);
        let leave = first.max(second);
        t_min = t_min.max(enter);
        t_max = t_max.min(leave);
        if t_min >= t_max {
            return false;
        }
    }
    t_max > 0.0 && t_min < 1.0
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

/// Decode the complete opcode-less body emitted by C++
/// `WorldPackets::Movement::MonsterMove::Write`.
///
/// This follows `MovementPackets.cpp` field order exactly and rejects
/// non-canonical packed GUIDs, nonzero bit padding, non-finite floats, trailing
/// bytes, and truncated optional sections. The returned allocation fields are
/// retained for the bot/report contract. The fixture mover counter must equal
/// the persistent spawn GUID; only the process-global spline ID is normalized.
pub fn decode_monster_move_body(body: &[u8]) -> Result<DecodedMonsterMoveBody, String> {
    let mut cursor = 0usize;
    let (mover_low, mover_high) = read_packed_guid(body, &mut cursor, "MoverGUID")?;
    if mover_low == 0 && mover_high == 0 {
        return Err("MoverGUID is empty".to_string());
    }
    let current_position = read_wire_position(body, &mut cursor, "Pos")?;
    let spline_id = read_u32(body, &mut cursor, "SplineData.ID")?;
    let destination = read_wire_position(body, &mut cursor, "SplineData.Destination")?;

    let spline_bits = read_u8(body, &mut cursor, "CrzTeleport/tolerance bit byte")?;
    if spline_bits & 0x0F != 0 {
        return Err(format!(
            "CrzTeleport/tolerance byte has non-canonical padding bits: 0x{spline_bits:02X}"
        ));
    }
    let crz_teleport = spline_bits & 0x80 != 0;
    let stop_distance_tolerance = (spline_bits >> 4) & 0x07;

    let flags = read_u32(body, &mut cursor, "Move.Flags")?;
    let elapsed = read_i32(body, &mut cursor, "Move.Elapsed")?;
    let move_time = read_u32(body, &mut cursor, "Move.MoveTime")?;
    let fade_object_time = read_u32(body, &mut cursor, "Move.FadeObjectTime")?;
    let mode = read_u8(body, &mut cursor, "Move.Mode")?;
    let (transport_low, transport_high) =
        read_packed_guid(body, &mut cursor, "Move.TransportGUID")?;
    let vehicle_seat = read_i8(body, &mut cursor, "Move.VehicleSeat")?;

    let header_bytes: [u8; 5] = read_array(body, &mut cursor, "Move bit header")?;
    let header = header_bytes
        .into_iter()
        .fold(0u64, |value, byte| (value << 8) | u64::from(byte));
    let face_kind = ((header >> 38) & 0x03) as u8;
    let point_count = ((header >> 22) & 0xFFFF) as usize;
    let vehicle_exit_voluntary = header & (1 << 21) != 0;
    let interpolate = header & (1 << 20) != 0;
    let packed_delta_count = ((header >> 4) & 0xFFFF) as usize;
    let has_spline_filter = header & (1 << 3) != 0;
    let has_spell_effect_extra = header & (1 << 2) != 0;
    let has_jump_extra = header & (1 << 1) != 0;
    let has_anim_tier_transition = header & 1 != 0;

    let spline_filter = if has_spline_filter {
        let key_count = read_u32(body, &mut cursor, "SplineFilter key count")? as usize;
        let base_speed_bits = read_f32_bits(body, &mut cursor, "SplineFilter BaseSpeed")?;
        let start_offset = read_i16(body, &mut cursor, "SplineFilter StartOffset")?;
        let distance_to_previous_key_bits =
            read_f32_bits(body, &mut cursor, "SplineFilter DistToPrevFilterKey")?;
        let added_to_start = read_i16(body, &mut cursor, "SplineFilter AddedToStart")?;
        let mut keys = Vec::with_capacity(key_count.min(body.len() / 4));
        for index in 0..key_count {
            keys.push(MonsterSplineFilterKeyBody {
                index: read_i16(body, &mut cursor, &format!("SplineFilter key[{index}].Idx"))?,
                speed: read_u16(
                    body,
                    &mut cursor,
                    &format!("SplineFilter key[{index}].Speed"),
                )?,
            });
        }
        let flag_byte = read_u8(body, &mut cursor, "SplineFilter flags bit byte")?;
        if flag_byte & 0x3F != 0 {
            return Err(format!(
                "SplineFilter flags byte has non-canonical padding bits: 0x{flag_byte:02X}"
            ));
        }
        Some(MonsterSplineFilterBody {
            base_speed_bits,
            start_offset,
            distance_to_previous_key_bits,
            added_to_start,
            keys,
            flags: flag_byte >> 6,
        })
    } else {
        None
    };

    let face = match face_kind {
        0 => MonsterMoveFaceBody::Normal,
        1 => MonsterMoveFaceBody::Spot {
            position: read_wire_position(body, &mut cursor, "Move.FaceSpot")?,
        },
        2 => {
            let direction_bits = read_f32_bits(body, &mut cursor, "Move.FaceDirection")?;
            let (low, high) = read_packed_guid(body, &mut cursor, "Move.FaceGUID")?;
            MonsterMoveFaceBody::Target {
                direction_bits,
                target: ExactObjectGuid { low, high },
            }
        }
        3 => MonsterMoveFaceBody::Angle {
            direction_bits: read_f32_bits(body, &mut cursor, "Move.FaceDirection")?,
        },
        _ => unreachable!("two-bit face kind"),
    };

    let mut points = Vec::with_capacity(point_count.min(body.len() / 12));
    for index in 0..point_count {
        points.push(read_wire_position(
            body,
            &mut cursor,
            &format!("Move.Points[{index}]"),
        )?);
    }
    let mut packed_deltas = Vec::with_capacity(packed_delta_count.min(body.len() / 4));
    for index in 0..packed_delta_count {
        packed_deltas.push(read_u32(
            body,
            &mut cursor,
            &format!("Move.PackedDeltas[{index}]"),
        )?);
    }

    let spell_effect_extra = if has_spell_effect_extra {
        let (low, high) = read_packed_guid(body, &mut cursor, "SpellEffectExtra.TargetGUID")?;
        Some(MonsterSplineSpellEffectExtraBody {
            target: ExactObjectGuid { low, high },
            spell_visual_id: read_u32(body, &mut cursor, "SpellEffectExtra.SpellVisualID")?,
            progress_curve_id: read_u32(body, &mut cursor, "SpellEffectExtra.ProgressCurveID")?,
            parabolic_curve_id: read_u32(body, &mut cursor, "SpellEffectExtra.ParabolicCurveID")?,
            jump_gravity_bits: read_f32_bits(body, &mut cursor, "SpellEffectExtra.JumpGravity")?,
        })
    } else {
        None
    };
    let jump_extra = if has_jump_extra {
        Some(MonsterSplineJumpExtraBody {
            jump_gravity_bits: read_f32_bits(body, &mut cursor, "JumpExtra.JumpGravity")?,
            start_time: read_u32(body, &mut cursor, "JumpExtra.StartTime")?,
            duration: read_u32(body, &mut cursor, "JumpExtra.Duration")?,
        })
    } else {
        None
    };
    let anim_tier_transition = if has_anim_tier_transition {
        Some(MonsterSplineAnimTierTransitionBody {
            tier_transition_id: read_i32(body, &mut cursor, "AnimTier.TierTransitionID")?,
            start_time: read_u32(body, &mut cursor, "AnimTier.StartTime")?,
            end_time: read_u32(body, &mut cursor, "AnimTier.EndTime")?,
            animation_tier: read_u8(body, &mut cursor, "AnimTier.AnimTier")?,
        })
    } else {
        None
    };

    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after MovementMonsterSpline: decoded {cursor} of {} bytes",
            body.len()
        ));
    }

    Ok(DecodedMonsterMoveBody {
        body: MonsterMoveBody {
            mover: stable_object_guid(mover_low, mover_high),
            current_position,
            destination,
            crz_teleport,
            stop_distance_tolerance,
            flags,
            elapsed,
            move_time,
            fade_object_time,
            mode,
            transport: ExactObjectGuid {
                low: transport_low,
                high: transport_high,
            },
            vehicle_seat,
            face,
            vehicle_exit_voluntary,
            interpolate,
            points,
            packed_deltas,
            spline_filter,
            spell_effect_extra,
            jump_extra,
            anim_tier_transition,
        },
        mover_runtime_counter: mover_low & OBJECT_GUID_COUNTER_MASK,
        spline_id,
    })
}

/// Decode TrinityCore's signed quarter-yard `PackedXYZ` representation.
#[must_use]
pub fn unpack_monster_move_delta(packed: u32) -> [f32; 3] {
    fn sign_extend(value: u32, bits: u32) -> i32 {
        let shift = 32 - bits;
        ((value << shift) as i32) >> shift
    }

    [
        sign_extend(packed & 0x7FF, 11) as f32 * 0.25,
        sign_extend((packed >> 11) & 0x7FF, 11) as f32 * 0.25,
        sign_extend((packed >> 22) & 0x3FF, 10) as f32 * 0.25,
    ]
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

/// Decode and canonicalize the opcode-less body emitted by C++
/// `WorldPackets::Spells::SendKnownSpells::Write`.
///
/// The first bit and both counts remain exact. Known and favorite spell IDs
/// are sorted only after canonical body decoding so the comparator mirrors
/// C++'s unordered `PlayerSpellMap` iteration without accepting a different
/// set. Duplicate IDs, favorite IDs absent from the known set, nonzero bit
/// padding, count/length mismatches, and spell ID zero are rejected.
pub fn decode_send_known_spells_body(body: &[u8]) -> Result<SendKnownSpellsBody, String> {
    if body.len() < 9 {
        return Err(format!(
            "SendKnownSpells body is {} bytes; need at least 9",
            body.len()
        ));
    }
    let bit_byte = body[0];
    if bit_byte & 0x7F != 0 {
        return Err(format!(
            "SendKnownSpells InitialLogin byte has non-canonical padding bits: 0x{bit_byte:02X}"
        ));
    }
    let initial_login = bit_byte & 0x80 != 0;
    let known_count = u32::from_le_bytes(body[1..5].try_into().expect("four-byte slice")) as usize;
    let favorite_count =
        u32::from_le_bytes(body[5..9].try_into().expect("four-byte slice")) as usize;
    let spell_count = known_count
        .checked_add(favorite_count)
        .ok_or_else(|| "SendKnownSpells spell counts overflow usize".to_string())?;
    let expected_len = spell_count
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(9))
        .ok_or_else(|| "SendKnownSpells body length overflows usize".to_string())?;
    if body.len() != expected_len {
        return Err(format!(
            "SendKnownSpells counts require {expected_len} bytes but body has {}",
            body.len()
        ));
    }

    let mut cursor = 9;
    let mut read_spells = |count: usize, label: &str| -> Result<Vec<u32>, String> {
        let mut spells = Vec::with_capacity(count);
        for index in 0..count {
            let end = cursor + 4;
            let spell = u32::from_le_bytes(
                body[cursor..end]
                    .try_into()
                    .expect("validated exact body length"),
            );
            cursor = end;
            if spell == 0 {
                return Err(format!("{label}[{index}] has invalid spell ID 0"));
            }
            spells.push(spell);
        }
        spells.sort_unstable();
        if let Some(duplicate) = spells.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(format!(
                "{label} contains duplicate spell ID {}",
                duplicate[0]
            ));
        }
        Ok(spells)
    };

    let known_spells = read_spells(known_count, "KnownSpells")?;
    let favorite_spells = read_spells(favorite_count, "FavoriteSpells")?;
    if let Some(spell) = favorite_spells
        .iter()
        .find(|spell| known_spells.binary_search(spell).is_err())
    {
        return Err(format!(
            "FavoriteSpells contains spell ID {spell} absent from KnownSpells"
        ));
    }

    Ok(SendKnownSpellsBody {
        initial_login,
        known_spells,
        favorite_spells,
    })
}

/// Decode the complete opcode-less C++ `WorldPackets::Spells::SpellGo` body.
///
/// This follows `operator<<(ByteBuffer&, SpellCastData const&)` field for
/// field, including both packed-bit headers and every optional vector. The
/// trailing `CombatLogServerPacket` bit must select the basic packet: a full
/// advanced-combat-log payload is deliberately rejected because accepting it
/// without decoding `SpellCastLogData` would ignore stable bytes.
///
/// The returned stable body normalizes only the lower 40-bit counters of the
/// correlated Creature caster and Cast GUID plus the wrapping CastTime. Exact
/// GUIDs and CastTime remain available on [`DecodedSpellGoBody`] for contract
/// validation and diagnostics.
pub fn decode_spell_go_body(body: &[u8]) -> Result<DecodedSpellGoBody, String> {
    decode_spell_cast_data_body(body, true)
}

/// Decode the complete opcode-less C++ `WorldPackets::Spells::SpellStart`
/// body. START has no combat-log suffix and its CastTime is retained as the
/// exact cast duration.
pub fn decode_spell_start_body(body: &[u8]) -> Result<DecodedSpellStartBody, String> {
    let decoded = decode_spell_cast_data_body(body, false)?;
    Ok(DecodedSpellStartBody {
        body: SpellStartBody {
            cast: decoded.body,
            cast_time: decoded.cast_time,
        },
        exact_caster_guid: decoded.exact_caster_guid,
        exact_caster_unit: decoded.exact_caster_unit,
        cast_id: decoded.cast_id,
    })
}

fn decode_spell_cast_data_body(
    body: &[u8],
    has_spell_go_combat_log_suffix: bool,
) -> Result<DecodedSpellGoBody, String> {
    let mut cursor = 0usize;
    let exact_caster_guid = read_exact_guid(body, &mut cursor, "CasterGUID")?;
    let exact_caster_unit = read_exact_guid(body, &mut cursor, "CasterUnit")?;
    let cast_id = read_exact_guid(body, &mut cursor, "CastID")?;
    let original_cast_id = read_exact_guid(body, &mut cursor, "OriginalCastID")?;
    let spell_id = read_i32(body, &mut cursor, "SpellID")?;
    let spell_visual_id = read_i32(body, &mut cursor, "Visual.SpellXSpellVisualID")?;
    let cast_flags = read_u32(body, &mut cursor, "CastFlags")?;
    let cast_flags_ex = read_u32(body, &mut cursor, "CastFlagsEx")?;
    let cast_time = read_u32(body, &mut cursor, "CastTime")?;
    let missile_travel_time = read_u32(body, &mut cursor, "MissileTrajectory.TravelTime")?;
    let missile_pitch_bits = read_f32_bits(body, &mut cursor, "MissileTrajectory.Pitch")?;
    let dest_loc_spell_cast_index = read_u8(body, &mut cursor, "DestLocSpellCastIndex")?;
    let immunities_school = read_u32(body, &mut cursor, "Immunities.School")?;
    let immunities_value = read_u32(body, &mut cursor, "Immunities.Value")?;
    let prediction_points = read_u32(body, &mut cursor, "Predict.Points")?;
    let prediction_type = read_u8(body, &mut cursor, "Predict.Type")?;
    let prediction_beacon = read_exact_guid(body, &mut cursor, "Predict.BeaconGUID")?;

    let mut counts = MsbBitReader::new(body, cursor, "SpellCastData counts");
    let hit_count = counts.read(16, "HitTargets")? as usize;
    let miss_count = counts.read(16, "MissTargets")? as usize;
    let miss_status_count = counts.read(16, "MissStatus")? as usize;
    let remaining_power_count = counts.read(9, "RemainingPower")? as usize;
    let has_remaining_runes = counts.read(1, "RemainingRunes")? != 0;
    let target_point_count = counts.read(16, "TargetPoints")? as usize;
    let has_ammo_display_id = counts.read(1, "AmmoDisplayID")? != 0;
    let has_ammo_inventory_type = counts.read(1, "AmmoInventoryType")? != 0;
    counts.finish(&mut cursor)?;

    if miss_count != miss_status_count {
        return Err(format!(
            "SpellCastData has {miss_count} MissTargets but {miss_status_count} MissStatus entries"
        ));
    }

    let target = read_spell_target_data(body, &mut cursor, exact_caster_guid)?;

    ensure_count_fits_minimum(body, cursor, hit_count, 2, "HitTargets")?;
    let mut hit_targets = Vec::with_capacity(hit_count);
    for index in 0..hit_count {
        let guid = read_exact_guid(body, &mut cursor, &format!("HitTargets[{index}]"))?;
        hit_targets.push(correlate_caster_guid(guid, exact_caster_guid));
    }

    ensure_count_fits_minimum(body, cursor, miss_count, 2, "MissTargets")?;
    let mut miss_targets = Vec::with_capacity(miss_count);
    for index in 0..miss_count {
        let guid = read_exact_guid(body, &mut cursor, &format!("MissTargets[{index}]"))?;
        miss_targets.push(correlate_caster_guid(guid, exact_caster_guid));
    }

    ensure_count_fits_minimum(body, cursor, miss_status_count, 1, "MissStatus")?;
    let mut miss_status = Vec::with_capacity(miss_status_count);
    for index in 0..miss_status_count {
        let reason = read_u8(body, &mut cursor, &format!("MissStatus[{index}].Reason"))?;
        let reflect_status = if reason == SPELL_MISS_REFLECT {
            Some(read_u8(
                body,
                &mut cursor,
                &format!("MissStatus[{index}].ReflectStatus"),
            )?)
        } else {
            None
        };
        miss_status.push(SpellMissStatusBody {
            reason,
            reflect_status,
        });
    }

    ensure_count_fits_minimum(body, cursor, remaining_power_count, 5, "RemainingPower")?;
    let mut remaining_power = Vec::with_capacity(remaining_power_count);
    for index in 0..remaining_power_count {
        remaining_power.push(SpellPowerDataBody {
            cost: read_i32(body, &mut cursor, &format!("RemainingPower[{index}].Cost"))?,
            power_type: read_i8(body, &mut cursor, &format!("RemainingPower[{index}].Type"))?,
        });
    }

    let remaining_runes = if has_remaining_runes {
        let start = read_u8(body, &mut cursor, "RemainingRunes.Start")?;
        let count = read_u8(body, &mut cursor, "RemainingRunes.Count")?;
        let cooldown_count = read_u32(body, &mut cursor, "RemainingRunes.CooldownsCount")? as usize;
        let cooldowns = read_bytes(
            body,
            &mut cursor,
            cooldown_count,
            "RemainingRunes.Cooldowns",
        )?
        .to_vec();
        Some(SpellRuneDataBody {
            start,
            count,
            cooldowns,
        })
    } else {
        None
    };

    ensure_count_fits_minimum(body, cursor, target_point_count, 14, "TargetPoints")?;
    let mut target_points = Vec::with_capacity(target_point_count);
    for index in 0..target_point_count {
        target_points.push(read_spell_target_location(
            body,
            &mut cursor,
            &format!("TargetPoints[{index}]"),
        )?);
    }

    let ammo_display_id = has_ammo_display_id
        .then(|| read_i32(body, &mut cursor, "AmmoDisplayID"))
        .transpose()?;
    let ammo_inventory_type = has_ammo_inventory_type
        .then(|| read_i32(body, &mut cursor, "AmmoInventoryType"))
        .transpose()?;

    if has_spell_go_combat_log_suffix {
        let mut combat_log = MsbBitReader::new(body, cursor, "SpellGo combat-log bit");
        let has_full_combat_log = combat_log.read(1, "HasLogData")? != 0;
        combat_log.finish(&mut cursor)?;
        if has_full_combat_log {
            return Err(
                "SpellGo carries full SpellCastLogData; creature-spell contract permits only the fully decoded basic packet"
                    .to_string(),
            );
        }
    }
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after {}: decoded {cursor} of {} bytes",
            if has_spell_go_combat_log_suffix {
                "SpellGo combat-log bit"
            } else {
                "SpellStart SpellCastData"
            },
            body.len()
        ));
    }

    Ok(DecodedSpellGoBody {
        body: SpellGoBody {
            caster_guid: stable_object_guid(exact_caster_guid.low, exact_caster_guid.high),
            caster_unit: stable_object_guid(exact_caster_unit.low, exact_caster_unit.high),
            cast_id: stable_object_guid(cast_id.low, cast_id.high),
            original_cast_id,
            spell_id,
            spell_visual_id,
            cast_flags,
            cast_flags_ex,
            missile_travel_time,
            missile_pitch_bits,
            dest_loc_spell_cast_index,
            immunities_school,
            immunities_value,
            prediction_points,
            prediction_type,
            prediction_beacon,
            target,
            hit_targets,
            miss_targets,
            miss_status,
            remaining_power,
            remaining_runes,
            target_points,
            ammo_display_id,
            ammo_inventory_type,
        },
        exact_caster_guid,
        exact_caster_unit,
        cast_id,
        cast_time,
    })
}

fn read_spell_target_data(
    body: &[u8],
    cursor: &mut usize,
    caster: ExactObjectGuid,
) -> Result<SpellTargetDataBody, String> {
    let mut bits = MsbBitReader::new(body, *cursor, "SpellTargetData bits");
    let flags = bits.read(28, "Flags")?;
    let has_src_location = bits.read(1, "HasSrcLocation")? != 0;
    let has_dst_location = bits.read(1, "HasDstLocation")? != 0;
    let has_orientation = bits.read(1, "HasOrientation")? != 0;
    let has_map_id = bits.read(1, "HasMapID")? != 0;
    let name_len = bits.read(7, "NameLength")? as usize;
    bits.finish(cursor)?;

    let unit = correlate_caster_guid(read_exact_guid(body, cursor, "Target.Unit")?, caster);
    let item = read_exact_guid(body, cursor, "Target.Item")?;
    let src_location = has_src_location
        .then(|| read_spell_target_location(body, cursor, "Target.SrcLocation"))
        .transpose()?;
    let dst_location = has_dst_location
        .then(|| read_spell_target_location(body, cursor, "Target.DstLocation"))
        .transpose()?;
    let orientation_bits = has_orientation
        .then(|| read_f32_bits(body, cursor, "Target.Orientation"))
        .transpose()?;
    let map_id = has_map_id
        .then(|| read_i32(body, cursor, "Target.MapID"))
        .transpose()?;
    let name = read_bytes(body, cursor, name_len, "Target.Name")?.to_vec();

    Ok(SpellTargetDataBody {
        flags,
        unit,
        item,
        src_location,
        dst_location,
        orientation_bits,
        map_id,
        name,
    })
}

fn read_spell_target_location(
    body: &[u8],
    cursor: &mut usize,
    field: &str,
) -> Result<SpellTargetLocationBody, String> {
    Ok(SpellTargetLocationBody {
        transport: read_exact_guid(body, cursor, &format!("{field}.Transport"))?,
        position: read_wire_position(body, cursor, &format!("{field}.Location"))?,
    })
}

fn correlate_caster_guid(
    guid: ExactObjectGuid,
    caster: ExactObjectGuid,
) -> CorrelatedSpellGuidBody {
    if guid == caster {
        CorrelatedSpellGuidBody::Caster
    } else {
        CorrelatedSpellGuidBody::Exact { guid }
    }
}

fn validate_decoded_creature_spell_go(decoded: &DecodedSpellGoBody) -> Result<(), String> {
    validate_creature_spell_cast_shape(
        &decoded.body,
        decoded.exact_caster_guid,
        decoded.exact_caster_unit,
        decoded.cast_id,
    )
}

fn validate_creature_spell_cast_shape(
    body: &SpellGoBody,
    exact_caster_guid: ExactObjectGuid,
    exact_caster_unit: ExactObjectGuid,
    exact_cast_id: ExactObjectGuid,
) -> Result<(), String> {
    if exact_caster_guid != exact_caster_unit {
        return Err("Creature SpellCast CasterGUID does not equal CasterUnit".to_string());
    }
    if exact_guid_high_type(exact_caster_guid) != HIGH_GUID_CREATURE {
        return Err("Creature SpellCast caster is not a Creature GUID".to_string());
    }
    if exact_caster_guid.low & OBJECT_GUID_COUNTER_MASK == 0 {
        return Err("Creature SpellCast caster has a zero runtime counter".to_string());
    }
    if body.spell_id <= 0 {
        return Err(format!(
            "Creature SpellCast has invalid spell ID {}",
            body.spell_id
        ));
    }
    if body.original_cast_id != (ExactObjectGuid { low: 0, high: 0 }) {
        return Err("Creature AI SpellCast OriginalCastID is not EMPTY".to_string());
    }
    let cast = body.cast_id;
    if cast.high_type != HIGH_GUID_CAST {
        return Err(format!(
            "Creature SpellCast CastID HighGuid is {}, expected Cast ({HIGH_GUID_CAST})",
            cast.high_type
        ));
    }
    if cast.subtype != SPELL_CAST_SOURCE_NORMAL {
        return Err(format!(
            "Creature SpellCast CastID source is {}, expected NORMAL ({SPELL_CAST_SOURCE_NORMAL})",
            cast.subtype
        ));
    }
    if cast.realm_id != body.caster_guid.realm_id
        || cast.map_id != body.caster_guid.map_id
        || cast.entry != body.spell_id as u32
        || cast.server_id != 0
    {
        return Err(format!(
            "Creature SpellCast CastID identity {:?} does not match caster realm/map and spell {} with server 0",
            cast, body.spell_id
        ));
    }
    if exact_cast_id.low & OBJECT_GUID_COUNTER_MASK == 0 {
        return Err("Creature SpellCast CastID has a zero runtime counter".to_string());
    }
    Ok(())
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

fn exact_guid_high_type(guid: ExactObjectGuid) -> u8 {
    ((guid.high >> 58) & 0x3F) as u8
}

fn read_exact_guid(
    body: &[u8],
    cursor: &mut usize,
    field: &str,
) -> Result<ExactObjectGuid, String> {
    let (low, high) = read_packed_guid(body, cursor, field)?;
    Ok(ExactObjectGuid { low, high })
}

fn ensure_count_fits_minimum(
    body: &[u8],
    cursor: usize,
    count: usize,
    minimum_width: usize,
    field: &str,
) -> Result<(), String> {
    let minimum = count
        .checked_mul(minimum_width)
        .ok_or_else(|| format!("{field} minimum byte length overflows usize"))?;
    let remaining = body.len().saturating_sub(cursor);
    if minimum > remaining {
        return Err(format!(
            "{field} count {count} needs at least {minimum} bytes but only {remaining} remain"
        ));
    }
    Ok(())
}

fn read_bytes<'a>(
    body: &'a [u8],
    cursor: &mut usize,
    len: usize,
    field: &str,
) -> Result<&'a [u8], String> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| format!("offset overflow while reading {field}"))?;
    let bytes = body
        .get(*cursor..end)
        .ok_or_else(|| format!("truncated while reading {field} at byte {cursor}"))?;
    *cursor = end;
    Ok(bytes)
}

/// MSB-first bit reader matching TrinityCore `ByteBuffer::ReadBits`.
///
/// Each SpellCastData bit section is explicitly flushed before byte fields.
/// `finish` therefore rejects nonzero low padding bits rather than silently
/// skipping them.
struct MsbBitReader<'a> {
    body: &'a [u8],
    start: usize,
    bit_offset: usize,
    section: &'static str,
}

impl<'a> MsbBitReader<'a> {
    fn new(body: &'a [u8], start: usize, section: &'static str) -> Self {
        Self {
            body,
            start,
            bit_offset: 0,
            section,
        }
    }

    fn read(&mut self, width: usize, field: &str) -> Result<u32, String> {
        if width > 32 {
            return Err(format!(
                "{} field {field} requests unsupported {width}-bit width",
                self.section
            ));
        }
        let end_bit = self
            .bit_offset
            .checked_add(width)
            .ok_or_else(|| format!("{} bit offset overflow", self.section))?;
        let available_bits = self.body.len().saturating_sub(self.start).saturating_mul(8);
        if end_bit > available_bits {
            return Err(format!(
                "truncated while reading {}.{field} at bit {}",
                self.section, self.bit_offset
            ));
        }

        let mut value = 0u32;
        while self.bit_offset < end_bit {
            let byte = self.body[self.start + self.bit_offset / 8];
            let shift = 7 - (self.bit_offset % 8);
            value = (value << 1) | u32::from((byte >> shift) & 1);
            self.bit_offset += 1;
        }
        Ok(value)
    }

    fn finish(self, cursor: &mut usize) -> Result<(), String> {
        let used_bytes = self.bit_offset.div_ceil(8);
        let remainder = self.bit_offset % 8;
        if remainder != 0 {
            let padding_width = 8 - remainder;
            let padding_mask = ((1u16 << padding_width) - 1) as u8;
            let byte = self.body[self.start + used_bytes - 1];
            if byte & padding_mask != 0 {
                return Err(format!(
                    "{} has non-canonical padding bits in byte 0x{byte:02X}",
                    self.section
                ));
            }
        }
        *cursor = self
            .start
            .checked_add(used_bytes)
            .ok_or_else(|| format!("{} byte offset overflow", self.section))?;
        Ok(())
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

fn read_i8(body: &[u8], cursor: &mut usize, field: &str) -> Result<i8, String> {
    Ok(read_u8(body, cursor, field)? as i8)
}

fn read_i16(body: &[u8], cursor: &mut usize, field: &str) -> Result<i16, String> {
    Ok(i16::from_le_bytes(read_array(body, cursor, field)?))
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

fn read_f32_bits(body: &[u8], cursor: &mut usize, field: &str) -> Result<u32, String> {
    let bits = read_u32(body, cursor, field)?;
    if !f32::from_bits(bits).is_finite() {
        return Err(format!("{field} is not finite"));
    }
    Ok(bits)
}

fn read_wire_position(
    body: &[u8],
    cursor: &mut usize,
    field: &str,
) -> Result<WirePosition, String> {
    Ok(WirePosition {
        x_bits: read_f32_bits(body, cursor, &format!("{field}.x"))?,
        y_bits: read_f32_bits(body, cursor, &format!("{field}.y"))?,
        z_bits: read_f32_bits(body, cursor, &format!("{field}.z"))?,
    })
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
