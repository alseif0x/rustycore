// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! WorldStateExpression.db2 store and evaluator.
//!
//! C++ refs:
//! - `ConditionMgr::IsMeetingWorldStateExpression`
//! - `WorldStateExpressionEntry`
//! - `WorldStateExpressionValueType`, logic, comparison, operator, and WSE functions.

use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

pub const WSE_FUNCTION_MAX: u32 = 39;

const VALUE_CONSTANT: u8 = 1;
const VALUE_WORLD_STATE: u8 = 2;
const VALUE_FUNCTION: u8 = 3;

const LOGIC_NONE: u8 = 0;
const LOGIC_AND: u8 = 1;
const LOGIC_OR: u8 = 2;
const LOGIC_XOR: u8 = 3;

const COMP_NONE: u8 = 0;
const COMP_EQUAL: u8 = 1;
const COMP_NOT_EQUAL: u8 = 2;
const COMP_LESS: u8 = 3;
const COMP_LESS_OR_EQUAL: u8 = 4;
const COMP_GREATER: u8 = 5;
const COMP_GREATER_OR_EQUAL: u8 = 6;

const OP_NONE: u8 = 0;
const OP_SUM: u8 = 1;
const OP_SUBSTRACTION: u8 = 2;
const OP_MULTIPLICATION: u8 = 3;
const OP_DIVISION: u8 = 4;
const OP_REMAINDER: u8 = 5;

const WSE_FUNCTION_NONE: u32 = 0;
const WSE_FUNCTION_RANDOM: u32 = 1;
const WSE_FUNCTION_MONTH: u32 = 2;
const WSE_FUNCTION_DAY: u32 = 3;
const WSE_FUNCTION_TIME_OF_DAY: u32 = 4;
const WSE_FUNCTION_REGION: u32 = 5;
const WSE_FUNCTION_CLOCK_HOUR: u32 = 6;
const WSE_FUNCTION_OLD_DIFFICULTY_ID: u32 = 7;
const WSE_FUNCTION_HOLIDAY_ACTIVE: u32 = 10;
const WSE_FUNCTION_TIMER_CURRENT_TIME: u32 = 11;
const WSE_FUNCTION_WEEK_NUMBER: u32 = 12;
const WSE_FUNCTION_DIFFICULTY_ID: u32 = 15;
const WSE_FUNCTION_WORLD_STATE_EXPRESSION: u32 = 22;
const WSE_FUNCTION_MERSENNE_RANDOM: u32 = 33;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldStateExpressionEntry {
    pub id: u32,
    pub expression: String,
}

#[derive(Debug, Clone, Default)]
pub struct WorldStateExpressionStore {
    entries: HashMap<u32, WorldStateExpressionEntry>,
}

impl WorldStateExpressionStore {
    pub fn from_entries(entries: impl IntoIterator<Item = WorldStateExpressionEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load WorldStateExpression.db2 from `{data_dir}/dbc/{locale}/WorldStateExpression.db2`.
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("WorldStateExpression.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.insert(
                id,
                WorldStateExpressionEntry {
                    id,
                    expression: reader.get_field_string(idx, 0),
                },
            );
        }

        info!(
            "Loaded {} world state expressions from {}",
            entries.len(),
            path.display()
        );
        Ok(Self { entries })
    }

    pub fn get(&self, id: u32) -> Option<&WorldStateExpressionEntry> {
        self.entries.get(&id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldStateExpressionWorldState {
    pub id: u32,
    pub value: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldStateExpressionTimeLikeCpp {
    /// C++ `tm_mon + 1`.
    pub month: i32,
    /// C++ local `tm_mday + 1`.
    pub day: i32,
    pub hour: i32,
    pub minute: i32,
    pub game_time_secs: i64,
}

impl Default for WorldStateExpressionTimeLikeCpp {
    fn default() -> Self {
        Self {
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            game_time_secs: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WorldStateExpressionContextLikeCpp<'a> {
    pub world_states: &'a [WorldStateExpressionWorldState],
    pub expressions: Option<&'a WorldStateExpressionStore>,
    pub difficulty_id: i32,
    pub old_difficulty_id: i32,
    pub region: i32,
    pub raid_origin_secs: i64,
    pub active_holiday_ids: &'a [u32],
    pub time: WorldStateExpressionTimeLikeCpp,
}

impl Default for WorldStateExpressionContextLikeCpp<'_> {
    fn default() -> Self {
        Self {
            world_states: &[],
            expressions: None,
            difficulty_id: 0,
            old_difficulty_id: -1,
            region: 0,
            raid_origin_secs: 1_135_695_600,
            active_holiday_ids: &[],
            time: WorldStateExpressionTimeLikeCpp::default(),
        }
    }
}

impl WorldStateExpressionContextLikeCpp<'_> {
    fn world_state_value_like_cpp(&self, id: u32) -> i32 {
        self.world_states
            .iter()
            .find(|world_state| world_state.id == id)
            .map(|world_state| world_state.value)
            .unwrap_or(0)
    }

    fn is_holiday_active_like_cpp(&self, id: u32) -> i32 {
        i32::from(self.active_holiday_ids.contains(&id))
    }
}

pub fn is_meeting_world_state_expression_like_cpp(
    expression: &WorldStateExpressionEntry,
    context: &WorldStateExpressionContextLikeCpp<'_>,
) -> bool {
    let bytes = match hex_to_bytes_like_cpp(&expression.expression) {
        Some(bytes) if !bytes.is_empty() => bytes,
        _ => return false,
    };
    let mut reader = WseByteReader::new(&bytes);

    if !reader.read_bool().unwrap_or(false) {
        return false;
    }

    let mut final_result = eval_rel_op_like_cpp(&mut reader, context);
    let mut result_logic = reader.read_u8().unwrap_or(LOGIC_NONE);

    while result_logic != LOGIC_NONE {
        let second_result = eval_rel_op_like_cpp(&mut reader, context);
        final_result = match result_logic {
            LOGIC_AND => final_result && second_result,
            LOGIC_OR => final_result || second_result,
            LOGIC_XOR => final_result != second_result,
            _ => final_result,
        };

        // TrinityCore archived has `>=` here; local legacy has `<`, which breaks
        // chained logic and can read past the buffer. This intentionally follows
        // the corrected upstream semantics while preserving all expression opcodes.
        if reader.is_at_end() {
            break;
        }
        result_logic = reader.read_u8().unwrap_or(LOGIC_NONE);
    }

    final_result
}

fn eval_single_value_like_cpp(
    reader: &mut WseByteReader<'_>,
    context: &WorldStateExpressionContextLikeCpp<'_>,
) -> i32 {
    match reader.read_u8().unwrap_or(0) {
        VALUE_CONSTANT => reader.read_i32().unwrap_or(0),
        VALUE_WORLD_STATE => {
            let id = reader.read_u32().unwrap_or(0);
            context.world_state_value_like_cpp(id)
        }
        VALUE_FUNCTION => {
            let function_type = reader.read_u32().unwrap_or(u32::MAX);
            let arg1 = eval_single_value_like_cpp(reader, context);
            let arg2 = eval_single_value_like_cpp(reader, context);
            if function_type >= WSE_FUNCTION_MAX {
                return 0;
            }
            eval_function_like_cpp(function_type, arg1, arg2, context)
        }
        _ => 0,
    }
}

fn eval_value_like_cpp(
    reader: &mut WseByteReader<'_>,
    context: &WorldStateExpressionContextLikeCpp<'_>,
) -> i32 {
    let left = eval_single_value_like_cpp(reader, context);
    match reader.read_u8().unwrap_or(OP_NONE) {
        OP_NONE => left,
        OP_SUM => left.wrapping_add(eval_single_value_like_cpp(reader, context)),
        OP_SUBSTRACTION => left.wrapping_sub(eval_single_value_like_cpp(reader, context)),
        OP_MULTIPLICATION => left.wrapping_mul(eval_single_value_like_cpp(reader, context)),
        OP_DIVISION => {
            let right = eval_single_value_like_cpp(reader, context);
            left.checked_div(right).unwrap_or(0)
        }
        OP_REMAINDER => {
            let right = eval_single_value_like_cpp(reader, context);
            left.checked_rem(right).unwrap_or(0)
        }
        _ => left,
    }
}

fn eval_rel_op_like_cpp(
    reader: &mut WseByteReader<'_>,
    context: &WorldStateExpressionContextLikeCpp<'_>,
) -> bool {
    let left = eval_value_like_cpp(reader, context);
    match reader.read_u8().unwrap_or(COMP_NONE) {
        COMP_NONE => left != 0,
        COMP_EQUAL => left == eval_value_like_cpp(reader, context),
        COMP_NOT_EQUAL => left != eval_value_like_cpp(reader, context),
        COMP_LESS => left < eval_value_like_cpp(reader, context),
        COMP_LESS_OR_EQUAL => left <= eval_value_like_cpp(reader, context),
        COMP_GREATER => left > eval_value_like_cpp(reader, context),
        COMP_GREATER_OR_EQUAL => left >= eval_value_like_cpp(reader, context),
        _ => false,
    }
}

fn eval_function_like_cpp(
    function_type: u32,
    arg1: i32,
    arg2: i32,
    context: &WorldStateExpressionContextLikeCpp<'_>,
) -> i32 {
    match function_type {
        WSE_FUNCTION_NONE => 0,
        WSE_FUNCTION_RANDOM => random_range_like_cpp(arg1, arg2),
        WSE_FUNCTION_MONTH => context.time.month,
        WSE_FUNCTION_DAY => context.time.day,
        WSE_FUNCTION_TIME_OF_DAY => context.time.hour * 60 + context.time.minute,
        WSE_FUNCTION_REGION => context.region,
        WSE_FUNCTION_CLOCK_HOUR => {
            let current_hour = context.time.hour + 1;
            if current_hour <= 12 {
                if current_hour != 0 { current_hour } else { 12 }
            } else {
                current_hour - 12
            }
        }
        WSE_FUNCTION_OLD_DIFFICULTY_ID => context.old_difficulty_id,
        WSE_FUNCTION_HOLIDAY_ACTIVE => u32::try_from(arg1)
            .ok()
            .map(|id| context.is_holiday_active_like_cpp(id))
            .unwrap_or(0),
        WSE_FUNCTION_TIMER_CURRENT_TIME => context.time.game_time_secs as i32,
        WSE_FUNCTION_WEEK_NUMBER => {
            ((context.time.game_time_secs - context.raid_origin_secs) / 604_800) as i32
        }
        WSE_FUNCTION_DIFFICULTY_ID => context.difficulty_id,
        WSE_FUNCTION_WORLD_STATE_EXPRESSION => {
            let Some(store) = context.expressions else {
                return 0;
            };
            let Ok(id) = u32::try_from(arg1) else {
                return 0;
            };
            store
                .get(id)
                .map(|entry| i32::from(is_meeting_world_state_expression_like_cpp(entry, context)))
                .unwrap_or(0)
        }
        WSE_FUNCTION_MERSENNE_RANDOM => mersenne_random_like_cpp(arg1, arg2),
        _ => 0,
    }
}

fn mersenne_random_like_cpp(arg1: i32, arg2: i32) -> i32 {
    let Ok(max) = u32::try_from(arg1) else {
        return 0;
    };
    if max == 0 {
        return 0;
    }
    if max == 1 {
        return 1;
    }
    let seed = u32::try_from(arg2)
        .ok()
        .filter(|seed| *seed != 0)
        .unwrap_or(1);
    (Mt19937::new(seed).next_u32() % max + 1) as i32
}

fn random_range_like_cpp(arg1: i32, arg2: i32) -> i32 {
    let min = arg1.min(arg2);
    let max = arg1.max(arg2);
    let Ok(span) = u32::try_from(i64::from(max) - i64::from(min) + 1) else {
        return min;
    };
    if span <= 1 {
        return min;
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos())
        .unwrap_or(0);
    min.wrapping_add((nanos % span) as i32)
}

fn hex_to_bytes_like_cpp(input: &str) -> Option<Vec<u8>> {
    let hex: String = input.chars().filter(|ch| !ch.is_whitespace()).collect();
    if hex.len() % 2 != 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for chunk in hex.as_bytes().chunks_exact(2) {
        let s = std::str::from_utf8(chunk).ok()?;
        bytes.push(u8::from_str_radix(s, 16).ok()?);
    }
    Some(bytes)
}

struct WseByteReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> WseByteReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.data.len()
    }

    fn read_u8(&mut self) -> Option<u8> {
        let byte = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(byte)
    }

    fn read_bool(&mut self) -> Option<bool> {
        self.read_u8().map(|value| value != 0)
    }

    fn read_u32(&mut self) -> Option<u32> {
        let bytes = self.data.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(u32::from_le_bytes(bytes.try_into().ok()?))
    }

    fn read_i32(&mut self) -> Option<i32> {
        let bytes = self.data.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(i32::from_le_bytes(bytes.try_into().ok()?))
    }
}

struct Mt19937 {
    state: [u32; 624],
    index: usize,
}

impl Mt19937 {
    fn new(seed: u32) -> Self {
        let mut state = [0; 624];
        state[0] = seed;
        for i in 1..624 {
            state[i] = 1_812_433_253u32
                .wrapping_mul(state[i - 1] ^ (state[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        Self { state, index: 624 }
    }

    fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9D2C_5680;
        y ^= (y << 15) & 0xEFC6_0000;
        y ^ (y >> 18)
    }

    fn twist(&mut self) {
        for i in 0..624 {
            let x = (self.state[i] & 0x8000_0000) + (self.state[(i + 1) % 624] & 0x7FFF_FFFF);
            let mut xa = x >> 1;
            if x % 2 != 0 {
                xa ^= 0x9908_B0DF;
            }
            self.state[i] = self.state[(i + 397) % 624] ^ xa;
        }
        self.index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02X}")).collect()
    }

    fn constant(value: i32) -> Vec<u8> {
        let mut bytes = vec![VALUE_CONSTANT];
        bytes.extend_from_slice(&value.to_le_bytes());
        bytes
    }

    fn world_state(id: u32) -> Vec<u8> {
        let mut bytes = vec![VALUE_WORLD_STATE];
        bytes.extend_from_slice(&id.to_le_bytes());
        bytes
    }

    fn function(function_type: u32, arg1: Vec<u8>, arg2: Vec<u8>) -> Vec<u8> {
        let mut bytes = vec![VALUE_FUNCTION];
        bytes.extend_from_slice(&function_type.to_le_bytes());
        bytes.extend(arg1);
        bytes.extend(arg2);
        bytes
    }

    fn expression_from_rel(rel: Vec<u8>) -> WorldStateExpressionEntry {
        let mut bytes = vec![1];
        bytes.extend(rel);
        bytes.push(LOGIC_NONE);
        WorldStateExpressionEntry {
            id: 1,
            expression: hex(&bytes),
        }
    }

    fn rel(left: Vec<u8>, cmp: u8, right: Vec<u8>) -> Vec<u8> {
        let mut bytes = left;
        bytes.push(OP_NONE);
        bytes.push(cmp);
        bytes.extend(right);
        bytes.push(OP_NONE);
        bytes
    }

    fn rel_value(left_value: Vec<u8>, cmp: u8, right_value: Vec<u8>) -> Vec<u8> {
        let mut bytes = left_value;
        bytes.push(cmp);
        bytes.extend(right_value);
        bytes
    }

    #[test]
    fn world_state_expression_evaluates_comparison_like_cpp() {
        let entry = expression_from_rel(rel(world_state(88), COMP_EQUAL, constant(7)));
        let states = [WorldStateExpressionWorldState { id: 88, value: 7 }];
        let context = WorldStateExpressionContextLikeCpp {
            world_states: &states,
            ..Default::default()
        };

        assert!(is_meeting_world_state_expression_like_cpp(&entry, &context));
    }

    #[test]
    fn world_state_expression_evaluates_arithmetic_like_cpp() {
        let mut left = constant(6);
        left.push(OP_MULTIPLICATION);
        left.extend(constant(7));
        let mut right = constant(42);
        right.push(OP_NONE);
        let entry = expression_from_rel(rel_value(left, COMP_EQUAL, right));

        assert!(is_meeting_world_state_expression_like_cpp(
            &entry,
            &Default::default()
        ));
    }

    #[test]
    fn world_state_expression_handles_division_by_zero_like_cpp() {
        let mut left = constant(6);
        left.push(OP_DIVISION);
        left.extend(constant(0));
        let mut right = constant(0);
        right.push(OP_NONE);
        let entry = expression_from_rel(rel_value(left, COMP_EQUAL, right));

        assert!(is_meeting_world_state_expression_like_cpp(
            &entry,
            &Default::default()
        ));
    }

    #[test]
    fn world_state_expression_chains_logic_with_upstream_bugfix() {
        let mut bytes = vec![1];
        bytes.extend(rel(constant(1), COMP_EQUAL, constant(1)));
        bytes.push(LOGIC_AND);
        bytes.extend(rel(constant(2), COMP_EQUAL, constant(2)));
        bytes.push(LOGIC_OR);
        bytes.extend(rel(constant(0), COMP_NONE, Vec::new()));
        bytes.push(LOGIC_NONE);
        let entry = WorldStateExpressionEntry {
            id: 1,
            expression: hex(&bytes),
        };

        assert!(is_meeting_world_state_expression_like_cpp(
            &entry,
            &Default::default()
        ));
    }

    #[test]
    fn world_state_expression_supports_nested_expression_function() {
        let nested = expression_from_rel(rel(constant(1), COMP_NONE, Vec::new()));
        let store = WorldStateExpressionStore::from_entries([nested]);
        let entry = expression_from_rel(rel(
            function(
                WSE_FUNCTION_WORLD_STATE_EXPRESSION,
                constant(1),
                constant(0),
            ),
            COMP_EQUAL,
            constant(1),
        ));
        let context = WorldStateExpressionContextLikeCpp {
            expressions: Some(&store),
            ..Default::default()
        };

        assert!(is_meeting_world_state_expression_like_cpp(&entry, &context));
    }

    #[test]
    fn mersenne_random_matches_cpp_mt19937_first_draw_shape() {
        assert_eq!(mersenne_random_like_cpp(100, 1), 46);
        assert_eq!(mersenne_random_like_cpp(1, 99), 1);
    }
}
