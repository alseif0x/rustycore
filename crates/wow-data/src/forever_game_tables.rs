//! Build-70170 GameTables used while constructing a new player.
//!
//! This is intentionally separate from the legacy `game_tables` projection.
//! The target client has fifteen BaseMp columns, and its three files are
//! loaded as one admission unit.  The source reference is
//! `src/server/game/DataStores/GameTables.cpp::LoadGameTable` (line 44) and
//! `GameTables.h::GtBaseMPEntry/GtHpPerStaEntry/GtXpEntry` (lines 43, 112,
//! and 168) at `02245dcd245e7433e524577656177723d3e4992e`.

use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow};
mod spell_values;
pub use spell_values::{ItemLevelMultipliers, SpellScalingEntry, SpellValueGameTables};

const BASE_MP_FILE: &str = "BaseMp.txt";
const HP_PER_STA_FILE: &str = "HpPerSta.txt";
const XP_FILE: &str = "xp.txt";

const BASE_MP_COLUMNS: usize = 15;
const HP_PER_STA_COLUMNS: usize = 1;
const XP_COLUMNS: usize = 5;

/// C++ `GtBaseMPEntry` in target column order.
#[derive(Clone, Copy, PartialEq)]
pub struct BaseMpEntry {
    pub rogue: f32,
    pub druid: f32,
    pub hunter: f32,
    pub mage: f32,
    pub paladin: f32,
    pub priest: f32,
    pub shaman: f32,
    pub warlock: f32,
    pub warrior: f32,
    pub death_knight: f32,
    pub monk: f32,
    pub demon_hunter: f32,
    pub evoker: f32,
    pub adventurer: f32,
    pub traveler: f32,
}

impl BaseMpEntry {
    fn from_columns(columns: [f32; BASE_MP_COLUMNS]) -> Self {
        Self {
            rogue: columns[0],
            druid: columns[1],
            hunter: columns[2],
            mage: columns[3],
            paladin: columns[4],
            priest: columns[5],
            shaman: columns[6],
            warlock: columns[7],
            warrior: columns[8],
            death_knight: columns[9],
            monk: columns[10],
            demon_hunter: columns[11],
            evoker: columns[12],
            adventurer: columns[13],
            traveler: columns[14],
        }
    }

    fn for_class(self, class_id: u32) -> f32 {
        match class_id {
            1 => self.warrior,
            2 => self.paladin,
            3 => self.hunter,
            4 => self.rogue,
            5 => self.priest,
            6 => self.death_knight,
            7 => self.shaman,
            8 => self.mage,
            9 => self.warlock,
            10 => self.monk,
            11 => self.druid,
            12 => self.demon_hunter,
            13 => self.evoker,
            14 => self.adventurer,
            15 => self.traveler,
            _ => 0.0,
        }
    }
}

/// C++ `GtHpPerStaEntry`.
#[derive(Clone, Copy, PartialEq)]
pub struct HpPerStaEntry {
    pub health: f32,
}

/// C++ `GtXpEntry`.
#[derive(Clone, Copy, PartialEq)]
pub struct XpEntry {
    pub total: f32,
    pub per_kill: f32,
    pub junk: f32,
    pub stats: f32,
    pub divisor: f32,
}

/// The three target tables admitted as one immutable snapshot.
pub struct InitialGameTables {
    base_mp: Vec<BaseMpEntry>,
    hp_per_sta: Vec<HpPerStaEntry>,
    xp: Vec<XpEntry>,
}

impl InitialGameTables {
    /// Load `gt/BaseMp.txt`, `gt/HpPerSta.txt`, and `gt/xp.txt`.
    ///
    /// The directory is the extracted data root, not the `gt` directory.  The
    /// snapshot is returned only after all three files have been read and
    /// admitted; this is not an atomic filesystem snapshot.  No path is
    /// included in errors because callers may pass private locations.
    pub fn load(directory: impl AsRef<Path>) -> Result<Self> {
        let root = directory.as_ref().join("gt");
        let base_mp = read_and_parse(&root.join(BASE_MP_FILE), BASE_MP_FILE, parse_base_mp)?;
        let hp_per_sta = read_and_parse(
            &root.join(HP_PER_STA_FILE),
            HP_PER_STA_FILE,
            parse_hp_per_sta,
        )?;
        let xp = read_and_parse(&root.join(XP_FILE), XP_FILE, parse_xp)?;
        Ok(Self {
            base_mp,
            hp_per_sta,
            xp,
        })
    }

    /// Parse all three tables without filesystem access.  This is a narrow
    /// fixture constructor, not a generic GameTable framework.
    pub fn parse_strs(base_mp: &str, hp_per_sta: &str, xp: &str) -> Result<Self> {
        Ok(Self {
            base_mp: parse_base_mp(base_mp, BASE_MP_FILE)?,
            hp_per_sta: parse_hp_per_sta(hp_per_sta, HP_PER_STA_FILE)?,
            xp: parse_xp(xp, XP_FILE)?,
        })
    }

    pub fn base_mp(&self, level: u32) -> Option<&BaseMpEntry> {
        self.base_mp.get(level as usize)
    }

    pub fn hp_per_sta(&self, level: u32) -> Option<&HpPerStaEntry> {
        self.hp_per_sta.get(level as usize)
    }

    pub fn xp(&self, level: u32) -> Option<&XpEntry> {
        self.xp.get(level as usize)
    }

    /// Number of physical rows in each table, including source row zero.
    pub fn counts(&self) -> [usize; 3] {
        [self.base_mp.len(), self.hp_per_sta.len(), self.xp.len()]
    }

    /// Read-only QA metadata: FNV-1a of every f32's canonical little-endian
    /// bits in source column/physical row order, including row zero. This
    /// permits actual-file comparison with the Linux long-double oracle
    /// without emitting numeric cells; it is not a cryptographic proof.
    pub fn numeric_bit_fingerprints(&self) -> [u64; 3] {
        [
            fingerprint(self.base_mp.iter().flat_map(|r| {
                [
                    r.rogue,
                    r.druid,
                    r.hunter,
                    r.mage,
                    r.paladin,
                    r.priest,
                    r.shaman,
                    r.warlock,
                    r.warrior,
                    r.death_knight,
                    r.monk,
                    r.demon_hunter,
                    r.evoker,
                    r.adventurer,
                    r.traveler,
                ]
            })),
            fingerprint(self.hp_per_sta.iter().map(|r| r.health)),
            fingerprint(
                self.xp
                    .iter()
                    .flat_map(|r| [r.total, r.per_kill, r.junk, r.stats, r.divisor]),
            ),
        ]
    }

    /// C++ `GetGameTableColumnForClass` plus row lookup.
    pub fn base_mana(&self, level: u32, class_id: u32) -> Option<f32> {
        self.base_mp(level).map(|row| row.for_class(class_id))
    }
}

fn fingerprint(values: impl Iterator<Item = f32>) -> u64 {
    values
        .flat_map(|value| value.to_bits().to_le_bytes())
        .fold(14_695_981_039_346_656_037u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211)
        })
}

fn parse_base_mp(content: &str, source: &str) -> Result<Vec<BaseMpEntry>> {
    let rows = parse_float_rows(content, source, BASE_MP_COLUMNS)?;
    Ok(rows
        .into_iter()
        .map(|values| {
            let values: [f32; BASE_MP_COLUMNS] =
                values.try_into().expect("validated BaseMp column count");
            BaseMpEntry::from_columns(values)
        })
        .collect())
}

fn parse_hp_per_sta(content: &str, source: &str) -> Result<Vec<HpPerStaEntry>> {
    let rows = parse_float_rows(content, source, HP_PER_STA_COLUMNS)?;
    Ok(rows
        .into_iter()
        .map(|values| HpPerStaEntry { health: values[0] })
        .collect())
}

fn parse_xp(content: &str, source: &str) -> Result<Vec<XpEntry>> {
    let rows = parse_float_rows(content, source, XP_COLUMNS)?;
    Ok(rows
        .into_iter()
        .map(|values| XpEntry {
            total: values[0],
            per_kill: values[1],
            junk: values[2],
            stats: values[3],
            divisor: values[4],
        })
        .collect())
}

fn read_and_parse<T>(
    path: &Path,
    source: &'static str,
    parse: fn(&str, &str) -> Result<Vec<T>>,
) -> Result<Vec<T>> {
    let content = fs::read_to_string(path).map_err(|_| anyhow!("{source}: cannot read file"))?;
    parse(&content, source)
}

fn parse_float_rows(content: &str, source: &str, value_columns: usize) -> Result<Vec<Vec<f32>>> {
    // std::getline retains header CR; RemoveCRLF truncates data rows at the
    // FIRST CR, not just a trailing CR. str::lines would also alter the header.
    let mut lines = content.split_terminator('\n');
    let header = lines
        .next()
        .ok_or_else(|| anyhow!("{source}: empty file"))?;
    let header_columns = header.split('\t').filter(|value| !value.is_empty()).count();
    let expected_columns = value_columns + 1;
    if header_columns != expected_columns {
        return Err(anyhow!(
            "{source}: header has {header_columns} columns, expected {expected_columns}"
        ));
    }

    let mut rows = vec![vec![0.0; value_columns]];
    for raw_line in lines {
        let line = raw_line.split('\r').next().unwrap_or(raw_line);
        let mut values: Vec<&str> = line.split('\t').collect();
        while values.last().is_some_and(|value| value.is_empty()) {
            values.pop();
        }
        if values.is_empty() || values.len() == 1 {
            break;
        }
        if values.len() != expected_columns {
            return Err(anyhow!(
                "{source}: row has {} columns, expected {expected_columns}",
                values.len()
            ));
        }

        let mut parsed = Vec::with_capacity(value_columns);
        for value in values.into_iter().skip(1) {
            parsed.push(
                parse_decimal(value)
                    .ok_or_else(|| anyhow!("{source}: unsupported numeric cell"))?,
            );
        }
        rows.push(parsed);
    }
    Ok(rows)
}

/// Conservative build-70170 admission parser.
///
/// The target's Linux C++ path uses `StringTo<float>`/`std::stold` and then
/// `.value_or(0.0f)`.  This Rust admission boundary deliberately does not
/// claim complete `StringTo` parity: it accepts fully-consumed decimal values
/// with leading ASCII whitespace and an optional sign, but rejects invalid,
/// non-finite, hexadecimal, underflow, and overflow values instead of silently
/// turning them into zero.  This is intentional asset validation, not a
/// gameplay formula or repair policy.
fn parse_decimal(raw: &str) -> Option<f32> {
    if raw.is_empty() || raw.starts_with("0x") || raw.starts_with("0X") {
        return None;
    }
    if raw.trim_end_matches(|ch: char| ch.is_ascii_whitespace()) != raw {
        return None;
    }

    let value = raw.trim_start_matches(|ch: char| ch.is_ascii_whitespace());
    if value.is_empty() {
        return None;
    }
    let value = if let Some(unsigned) = value.strip_prefix('+') {
        if unsigned.starts_with('+') || unsigned.starts_with('-') {
            return None;
        }
        unsigned
    } else {
        value
    };
    if value.is_empty() || value.starts_with("0x") || value.starts_with("0X") {
        return None;
    }
    // Rust's f64 parser is deliberately an explicit approximation boundary:
    // the target Linux implementation parses long double before narrowing to
    // float.  We fail closed around narrowing rather than claiming bit-for-bit
    // parity before an oracle comparison against the actual target data.
    let parsed = value.parse::<f64>().ok()?;
    if !parsed.is_finite() || (parsed == 0.0 && contains_nonzero_mantissa_digit(value)) {
        return None;
    }
    let narrowed = parsed as f32;
    if !narrowed.is_finite() || (narrowed == 0.0 && parsed != 0.0) {
        return None;
    }
    Some(narrowed)
}

fn contains_nonzero_mantissa_digit(value: &str) -> bool {
    value
        .bytes()
        .take_while(|byte| !matches!(byte, b'e' | b'E'))
        .any(|byte| matches!(byte, b'1'..=b'9'))
}

#[cfg(test)]
#[path = "forever_game_tables/tests.rs"]
mod tests;
