//! Semantic capture diff state definitions, part 1 of 5.
//!
//! Separated from the semantic.rs root under #658. Behaviour is preserved.

use super::*;

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

pub(super) const OBJECT_GUID_COUNTER_MASK: u64 = 0x0000_00FF_FFFF_FFFF;

pub(super) const HIGH_GUID_CREATURE: u8 = 8;

pub(super) const HIGH_GUID_CAST: u8 = 47;

pub(super) const HIGH_GUID_ITEM: u8 = 3;

pub(super) const HIGH_GUID_LOOT_OBJECT: u8 = 15;

pub(super) const HIGH_GUID_PLAYER: u8 = 2;

pub(super) const SPELL_CAST_SOURCE_NORMAL: u8 = 3;

pub(super) const SPELL_MISS_REFLECT: u8 = 11;

pub(super) const GLOBAL_GUID_RESERVED_HIGH_BITS_MASK: u64 = (1_u64 << 42) - 1;

pub(super) const XP_GAIN_REASON_KILL: u8 = 0;

pub(super) const VALUES_TYPE_UNIT: u32 = 1 << 5;

pub(super) const VALUES_TYPE_ACTIVE_PLAYER: u32 = 1 << 7;

pub(super) const UNIT_POWER_PARENT_BLOCKS_MASK: u8 = 1 << 3;

pub(super) const UNIT_POWER_PARENT_BLOCK_3: u32 = 1 << 20;

pub(super) const ISSUE_106_CAPTURE_PLAYER_LOW: u64 = 15;

pub(super) const ISSUE_106_CAPTURE_PLAYER_HIGH: u64 = 0x0800_0400_0000_0000;

pub(super) const ISSUE_106_CAPTURE_ITEM_HIGH: u64 = 0x0C00_0400_0000_0000;

pub(super) const ISSUE_106_CREATURE_IDENTITY: StableObjectGuid = StableObjectGuid {
    high_type: HIGH_GUID_CREATURE,
    realm_id: 1,
    map_id: 530,
    entry: 21_779,
    subtype: 0,
    server_id: 0,
};

pub(super) const ISSUE_106_ITEM_ENTRY: i32 = 30_712;

// Doctor Maleficus' key is stored in the first keyring slot in the restored
// issue-#106 fixture. Pinning the observed C++ destination keeps a bank,
// equipment, or generic inventory update from satisfying this capture.
pub(super) const ISSUE_106_ITEM_SLOT: i32 = 106;

pub(super) const ISSUE_106_ITEM_DYNAMIC_FLAGS: u32 = 0x0020_0001;

pub(super) const ISSUE_106_ITEM_CREATE_ZERO_TAIL_LEN: usize = 220;

pub(super) const ISSUE_106_PING_BODY: [u8; 8] = [b'T', b'O', b'O', b'L', 0, 0, 0, 0];

pub(super) const ISSUE_108_VENDOR_IDENTITY: StableObjectGuid = StableObjectGuid {
    high_type: HIGH_GUID_CREATURE,
    realm_id: 1,
    map_id: 530,
    entry: 18_525,
    subtype: 0,
    server_id: 0,
};

pub(super) const ISSUE_108_VENDOR_MUID: u32 = 59;

pub(super) const ISSUE_108_VENDOR_NEW_QUANTITY: i32 = -1;

pub(super) const ISSUE_108_VENDOR_QUANTITY_BOUGHT: u32 = 1;

pub(super) const ISSUE_24_CREATURE_IDENTITY: StableObjectGuid = StableObjectGuid {
    high_type: HIGH_GUID_CREATURE,
    realm_id: 1,
    map_id: 1,
    entry: 15_271,
    subtype: 0,
    server_id: 0,
};

pub(super) const ISSUE_26_CREATURE_ENTRY: u32 = 22_378;

pub(super) const ISSUE_26_REALM_ID: u16 = 1;

pub(super) const ISSUE_26_MAP_ID: u16 = 530;

pub(super) const ISSUE_26_SPELL_ID: i32 = 15_691;

pub(super) const ISSUE_26_SPELL_X_SPELL_VISUAL_ID: i32 = 244_493;

pub(super) const ISSUE_26_PLAYER_COUNTER: u64 = 15;

pub(super) const ISSUE_26_START_CAST_FLAGS: u32 = 0x0000_0002;

pub(super) const ISSUE_26_GO_CAST_FLAGS: u32 = 0x0000_0100;

pub(super) const ISSUE_26_UNIT_TARGET_FLAGS: u32 = 0x0000_0002;

pub const ISSUE_24_PING_FENCE_WIRE: [u8; 4] = *b"DTOR";

pub const ISSUE_24_PING_FENCE_SERIAL: u32 = u32::from_le_bytes(ISSUE_24_PING_FENCE_WIRE);

pub(super) const ISSUE_24_PING_BODY: [u8; 8] = [
    ISSUE_24_PING_FENCE_WIRE[0],
    ISSUE_24_PING_FENCE_WIRE[1],
    ISSUE_24_PING_FENCE_WIRE[2],
    ISSUE_24_PING_FENCE_WIRE[3],
    0,
    0,
    0,
    0,
];

pub(super) const ISSUE_24_CAPTURE_PLAYER_LOW: u64 = 15;

pub(super) const ISSUE_24_CAPTURE_PLAYER_HIGH: u64 = 0x0800_0400_0000_0000;

pub(super) const ISSUE_24_CREATURE_START: WirePosition = WirePosition::new(
    (-10_118.333_f32).to_bits(),
    2_671.667_f32.to_bits(),
    218.490_f32.to_bits(),
);

pub(super) const ISSUE_24_PLAYER_DESTINATION: WirePosition = WirePosition::new(
    (-10_118.333_f32).to_bits(),
    2_691.667_f32.to_bits(),
    218.490_f32.to_bits(),
);

pub(super) const ISSUE_24_PLAYER_DESTINATION_ORIENTATION_BITS: u32 =
    (-std::f32::consts::FRAC_PI_2).to_bits();

pub(super) const ISSUE_24_OBSTACLE_MIN_X: f32 = -10_123.333;

pub(super) const ISSUE_24_OBSTACLE_MAX_X: f32 = -10_113.333;

pub(super) const ISSUE_24_OBSTACLE_MIN_Y: f32 = 2_676.667;

pub(super) const ISSUE_24_OBSTACLE_MAX_Y: f32 = 2_686.667;

pub(super) const ISSUE_24_POSITION_EPSILON: f32 = 0.05;

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
    pub(super) const fn new(x_bits: u32, y_bits: u32, z_bits: u32) -> Self {
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
    pub(super) fn from_decoded_log_xp_gain(
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

    pub(super) fn from_decoded_loot_removed(
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

    pub(super) fn from_decoded_buy_succeeded(
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

    pub(super) fn from_decoded_send_known_spells(
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

    pub(super) fn from_decoded_spell_go(
        decoded: Result<DecodedSpellGoBody, String>,
        raw_body: &[u8],
    ) -> Self {
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

    pub(super) fn from_decoded_spell_start(
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

    pub(super) fn from_update_object_inv_slots_decode(
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

    pub(super) fn from_decoded_monster_move(
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

pub(super) fn raw_body_sha256(body: &[u8]) -> String {
    format!("{:x}", Sha256::digest(body))
}

/// Detailed semantic comparison attached to a regular body diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticBodyDiff {
    pub comparator: String,
    pub cpp: SemanticBodySide,
    pub rust: SemanticBodySide,
}
