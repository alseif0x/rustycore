//! Target name-validation rules; collision lookup remains a persistence read.
//! 02245dcd ObjectMgr.cpp:157-174,8732-8805; Util.h:121-337 and
//! Util.cpp:370-423. The source's wide strings hold UTF-16 units even on
//! Linux. Its finite casing must not be replaced with expanding Unicode case.
mod catalog;
pub use catalog::{NameRules, Pattern, Patterns, Validation};

#[derive(Clone, Copy)]
pub struct NamePolicy {
    pub minimum_units: u32,
    pub strict_mask: u32,
    /// Effective Cfg_Categories CreateCharsetMask for the realm timezone;
    /// source fallback when its category is absent is English (2).
    pub creation_charset: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameRejection {
    NoName,
    TooShort,
    TooLong,
    MixedLanguages,
    ThreeConsecutive,
    Profane,
    Reserved,
}

impl NameRejection {
    pub const fn wire_result(self) -> u32 {
        match self {
            Self::NoName => 98,
            Self::TooShort => 99,
            Self::TooLong => 100,
            Self::MixedLanguages => 102,
            Self::ThreeConsecutive => 107,
            Self::Profane => 103,
            Self::Reserved => 104,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameRuleError {
    Engine,
    InvalidLocale,
}

/// Transient request value. No Debug: neither spelling nor regex input belongs
/// in diagnostics. SQL collision lookup uses normalized spelling; source regex
/// validation and reserved-name lookup use the lowercased UTF-16 units.
pub struct CheckedName {
    spelling: String,
    lower: Vec<u16>,
}

impl CheckedName {
    pub fn spelling(&self) -> &str {
        &self.spelling
    }
    pub fn lower_units(&self) -> &[u16] {
        &self.lower
    }
}

pub fn normalize_and_check(name: &str, policy: NamePolicy) -> Result<CheckedName, NameRejection> {
    // Packet decoding has already established UTF-8. The source handler maps
    // an empty/failed normalization to NO_NAME before CheckPlayerName.
    let mut normalized: Vec<u16> = name.encode_utf16().map(lower).collect();
    let Some(first) = normalized.first_mut() else {
        return Err(NameRejection::NoName);
    };
    *first = upper(*first);
    if normalized.len() > 12 {
        return Err(NameRejection::TooLong);
    }
    if normalized.len() < policy.minimum_units as usize {
        return Err(NameRejection::TooShort);
    }
    if !valid_charset(&normalized, policy.strict_mask, policy.creation_charset) {
        return Err(NameRejection::MixedLanguages);
    }
    let spelling = String::from_utf16(&normalized).expect("finite BMP casing preserves UTF-16");
    let lower: Vec<u16> = normalized.into_iter().map(lower).collect();
    if lower
        .windows(3)
        .any(|units| units[0] == units[1] && units[1] == units[2])
    {
        return Err(NameRejection::ThreeConsecutive);
    }
    Ok(CheckedName { spelling, lower })
}

pub fn lower_units(text: &str) -> Vec<u16> {
    text.encode_utf16().map(lower).collect()
}

fn lower(unit: u16) -> u16 {
    match unit {
        0x41..=0x5A | 0xC0..=0xD6 | 0xD8..=0xDE | 0x410..=0x42F => unit + 0x20,
        0x100..=0x12E if unit % 2 == 0 => unit + 1,
        0x1E9E => 0xDF,
        0x401 => 0x451,
        0x152 => 0x153,
        0x178 => 0xFF,
        _ => unit,
    }
}

fn upper(unit: u16) -> u16 {
    match unit {
        0x61..=0x7A | 0xE0..=0xF6 | 0xF8..=0xFE | 0x430..=0x44F => unit - 0x20,
        0x101..=0x12F if unit % 2 == 1 => unit - 1,
        0xDF => 0x1E9E,
        0x451 => 0x401,
        0x153 => 0x152,
        0xFF => 0x178,
        _ => unit,
    }
}

fn basic(unit: u16) -> bool {
    matches!(unit, 0x41..=0x5A | 0x61..=0x7A)
}
fn extended(unit: u16) -> bool {
    basic(unit) || matches!(unit, 0xC0..=0xD6 | 0xD8..=0xF6 | 0xF8..=0xFE | 0x100..=0x12F | 0x1E9E)
}
fn cyrillic(unit: u16) -> bool {
    matches!(unit, 0x410..=0x44F | 0x401 | 0x451)
}
fn korean(unit: u16) -> bool {
    matches!(unit, 0x1100..=0x11F9 | 0x3131..=0x318E | 0xAC00..=0xD7A3 | 0xFF01..=0xFFEE)
}
fn chinese(unit: u16) -> bool {
    matches!(unit, 0x3100..=0x312C | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF)
}

fn valid_charset(units: &[u16], strict: u32, realm: u32) -> bool {
    let all = |predicate: fn(u16) -> bool| units.iter().copied().all(predicate);
    if strict == 0 {
        return all(extended) || all(cyrillic) || all(korean) || all(chinese);
    }
    if strict & 2 != 0
        && (realm == 0
            || realm & 1 != 0 && all(extended)
            || realm & 2 != 0 && all(basic)
            || realm & 4 != 0 && all(cyrillic)
            || realm & 8 != 0 && all(korean)
            || realm & 16 != 0 && all(chinese))
    {
        return true;
    }
    strict & 1 != 0 && all(basic)
}

#[cfg(test)]
mod tests;
