//! Target ObjectMgr.cpp:4409-4476,7990-7995 at 02245dcd. Immutable XP
//! startup authority; this does not initialize health, spells or a Player.
use super::SourceError;
use std::collections::BTreeSet;
use wow_data::forever_game_tables::InitialGameTables;
use wow_persistence::forever::creation::XpOverride;

pub(super) struct Progression {
    xp: Vec<u32>,
    max_level: u8,
    override_count: usize,
}

impl Progression {
    pub(super) fn load(
        rows: Vec<XpOverride>,
        tables: &InitialGameTables,
        max_level: u8,
    ) -> Result<Self, SourceError> {
        // DBCEnums.h MAX_LEVEL=123, World.cpp Min=1/Max=123. Require
        // coverage instead of following an out-of-bounds C++ vector access.
        if !(1..=123).contains(&max_level) {
            return Err(SourceError::InvalidLevelCap);
        }
        let count = tables.counts()[2];
        if count < usize::from(max_level) {
            return Err(SourceError::InsufficientXpCoverage);
        }
        let mut xp = Vec::with_capacity(count);
        xp.push(0); // unused GT row zero is not copied into XP[0]
        for level in 1..count {
            xp.push(unsigned(
                tables
                    .xp(level as u32)
                    .ok_or(SourceError::InsufficientXpCoverage)?
                    .total,
            )?);
        }
        let override_count = rows.len();
        let mut seen = BTreeSet::new();
        for row in rows {
            if !seen.insert(row.level) {
                return Err(SourceError::DuplicateIdentity);
            }
            // Level zero is a legal SQL override; max-level itself is NOT.
            if row.level < max_level {
                xp[usize::from(row.level)] = row.experience;
            }
        }
        for level in 1..usize::from(max_level) {
            if xp[level] == 0 {
                // Unsigned addition in source is defined modulo 2^32.
                xp[level] = xp[level - 1].wrapping_add(12_000);
            }
        }
        Ok(Self {
            xp,
            max_level,
            override_count,
        })
    }

    pub(super) fn xp(&self, level: u8) -> u32 {
        self.xp.get(usize::from(level)).copied().unwrap_or(0)
    }

    pub(super) fn max_level(&self) -> u8 {
        self.max_level
    }

    pub(super) fn base_mana(
        &self,
        tables: &InitialGameTables,
        class: u8,
        level: u8,
    ) -> Result<u32, SourceError> {
        if level == 0 {
            return Err(SourceError::InvalidLevel);
        }
        if class >= 16 {
            return Err(SourceError::InvalidClass);
        }
        unsigned(
            tables
                .base_mana(u32::from(level.min(self.max_level)), u32::from(class))
                .ok_or(SourceError::MissingManaRow)?,
        )
    }

    pub(super) fn override_count(&self) -> usize {
        self.override_count
    }

    pub(super) fn counts(&self) -> [usize; 2] {
        [self.xp.len(), usize::from(self.max_level)]
    }
}

pub(super) fn unsigned(value: f32) -> Result<u32, SourceError> {
    // C++ float-to-uint32 outside the representable range is undefined. Keep
    // valid truncation, including -0; do not use Rust's saturating cast as a
    // fabricated baseline for corrupt/nonfinite/negative assets.
    if !value.is_finite() || !(0.0..4_294_967_296.0).contains(&value) {
        return Err(SourceError::InvalidGameTableValue);
    }
    Ok(value as u32)
}

/// ObjectMgr.cpp:4494-4562. The source loop starts at cap-1, not cap;
/// retain its boundary and its missing newer-class cases rather than inventing
/// the old 3.4.3/3.3.5 growth formula. Spirit is unchanged in every case.
pub(super) fn extrapolate_stats(
    class: u8,
    cap: u8,
    level: u8,
    mut stats: [i32; 5],
) -> Result<[i32; 5], SourceError> {
    for lvl in (cap - 1)..level {
        let one = |condition| i32::from(condition);
        let two = |high, low| if high { 2 } else { one(low) };
        let odd = lvl % 2 != 0;
        let delta = match class {
            1 => [
                two(lvl > 23, lvl > 1),
                one(lvl > 36 || (lvl > 6 && odd)),
                two(lvl > 23, lvl > 1),
                one(lvl > 9 && !odd),
            ],
            2 => [
                one(lvl > 3),
                one(lvl > 38 || (lvl > 7 && !odd)),
                two(lvl > 33, lvl > 1),
                one(lvl > 6 && odd),
            ],
            3 => [
                one(lvl > 4),
                two(lvl > 33, lvl > 1),
                one(lvl > 4),
                one(lvl > 8 && odd),
            ],
            4 => [
                one(lvl > 5),
                two(lvl > 16, lvl > 1),
                one(lvl > 4),
                one(lvl > 8 && !odd),
            ],
            5 => [
                one(lvl > 9 && !odd),
                one(lvl > 38 || (lvl > 8 && odd)),
                one(lvl > 5),
                two(lvl > 22, lvl > 1),
            ],
            7 => [
                one(lvl > 34 || (lvl > 6 && odd)),
                one(lvl > 7 && !odd),
                one(lvl > 4),
                one(lvl > 5),
            ],
            8 => [
                one(lvl > 9 && !odd),
                one(lvl > 9 && !odd),
                one(lvl > 5),
                two(lvl > 24, lvl > 1),
            ],
            9 => [
                one(lvl > 9 && !odd),
                one(lvl > 9 && !odd),
                two(lvl > 38, lvl > 3),
                two(lvl > 33, lvl > 2),
            ],
            11 => [
                two(lvl > 38, lvl > 6 && odd),
                two(lvl > 38, lvl > 8 && odd),
                two(lvl > 32, lvl > 4),
                if lvl > 38 { 3 } else { one(lvl > 4) },
            ],
            _ => [0; 4],
        };
        for (stat, amount) in stats[..4].iter_mut().zip(delta) {
            *stat = stat.checked_add(amount).ok_or(SourceError::StatsOverflow)?;
        }
    }
    Ok(stats)
}

#[cfg(test)]
#[path = "progression_tests.rs"]
mod tests;
