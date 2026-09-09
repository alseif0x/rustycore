//! Skill tier and pet spell stores state definitions, part 2 of 2.
//!
//! Separated from the skill.rs root under #638. Behaviour is preserved.

use super::*;

pub(super) fn skill_line_ability_rank_row_from_source_like_cpp(
    record: &SkillLineAbilitySourceRecordLikeCpp,
) -> Option<SkillLineAbilityRankRowLikeCpp> {
    skill_line_ability_rank_row_from_raw_like_cpp(record.id, record.spell, record.supercedes_spell)
}

pub(super) fn skill_line_ability_rank_row_from_raw_like_cpp(
    record_id: u32,
    spell_raw: i128,
    supercedes_spell_raw: i128,
) -> Option<SkillLineAbilityRankRowLikeCpp> {
    if i32::try_from(supercedes_spell_raw).ok() == Some(0) {
        return None;
    }

    // Both DB2 members are signed `int32`, but `LoadSpellRanks` passes them
    // to `GetSpellInfo(uint32)` and stores them in `std::map<uint32, uint32>`.
    // Preserve that defined modulo-2^32 conversion for every representable
    // source value; only a value outside C++'s `int32` domain is ambiguous.
    let spell_id = i32::try_from(spell_raw).ok().map(|value| value as u32);
    let supercedes_spell_id = i32::try_from(supercedes_spell_raw)
        .ok()
        .map(|value| value as u32);
    match (spell_id, supercedes_spell_id) {
        (Some(spell_id), Some(supercedes_spell_id)) => Some(SkillLineAbilityRankRowLikeCpp::Edge {
            record_id,
            spell_id,
            supercedes_spell_id,
        }),
        _ => Some(SkillLineAbilityRankRowLikeCpp::Indeterminate {
            record_id,
            spell_raw,
            supercedes_spell_raw,
        }),
    }
}

pub(super) fn skill_line_ability_from_source_like_cpp(
    record: SkillLineAbilitySourceRecordLikeCpp,
    diagnostics: &mut Vec<SkillStoreLoadDiagnosticLikeCpp>,
) -> Option<SkillLineAbilityRecord> {
    // Source records retain SQL values in i128 so out-of-schema overlays can
    // be diagnosed. Enforce C++'s signed-i16 source domain first, then expose
    // only a nonnegative, validated identifier through the public u16 field.
    let skill_line = i16::try_from(record.skill_line)
        .ok()
        .and_then(|value| u16::try_from(value).ok());
    let skillup_skill_line_id = i16::try_from(record.skillup_skill_line_id)
        .ok()
        .filter(|value| *value >= 0);
    let (Some(skill_line), Some(skillup_skill_line_id)) = (skill_line, skillup_skill_line_id)
    else {
        diagnostics.push(
            SkillStoreLoadDiagnosticLikeCpp::InvalidSkillLineAbilityIdentifier {
                source: record.source,
                record_id: record.id,
                spell: record.spell,
                skill_line: record.skill_line,
                skillup_skill_line_id: record.skillup_skill_line_id,
            },
        );
        return None;
    };

    Some(SkillLineAbilityRecord {
        id: record.id,
        race_mask: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "RaceMask",
            record.race_mask,
            diagnostics,
        )?,
        skill_line,
        spell: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "Spell",
            record.spell,
            diagnostics,
        )?,
        min_skill_line_rank: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "MinSkillLineRank",
            record.min_skill_line_rank,
            diagnostics,
        )?,
        class_mask: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "ClassMask",
            record.class_mask,
            diagnostics,
        )?,
        supercedes_spell: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "SupercedesSpell",
            record.supercedes_spell,
            diagnostics,
        )?,
        acquire_method: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "AcquireMethod",
            record.acquire_method,
            diagnostics,
        )?,
        trivial_rank_high: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "TrivialSkillLineRankHigh",
            record.trivial_rank_high,
            diagnostics,
        )?,
        trivial_rank_low: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "TrivialSkillLineRankLow",
            record.trivial_rank_low,
            diagnostics,
        )?,
        flags: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "Flags",
            record.flags,
            diagnostics,
        )?,
        num_skill_ups: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "NumSkillUps",
            record.num_skill_ups,
            diagnostics,
        )?,
        skillup_skill_line_id,
    })
}

pub(super) fn skill_line_ability_skill_key_from_source_like_cpp(
    record: &SkillLineAbilitySourceRecordLikeCpp,
) -> Option<u16> {
    let skillup_skill_line_id = i16::try_from(record.skillup_skill_line_id).ok()?;
    if skillup_skill_line_id != 0 {
        return u16::try_from(skillup_skill_line_id).ok();
    }

    i16::try_from(record.skill_line)
        .ok()
        .and_then(|skill_line| u16::try_from(skill_line).ok())
}

pub(super) fn skill_race_class_info_from_source_like_cpp(
    record: SkillRaceClassInfoSourceRecordLikeCpp,
    diagnostics: &mut Vec<SkillStoreLoadDiagnosticLikeCpp>,
) -> Option<SkillRaceClassInfoRecord> {
    // As above, the public u16 is a post-validation representation; it does
    // not widen C++ `SkillRaceClassInfoEntry::SkillID` beyond signed i16.
    let Some(skill_id) = i16::try_from(record.skill_id)
        .ok()
        .and_then(|value| u16::try_from(value).ok())
    else {
        diagnostics.push(
            SkillStoreLoadDiagnosticLikeCpp::InvalidSkillRaceClassInfoIdentifier {
                source: record.source,
                record_id: record.id,
                race_mask: record.race_mask,
                skill_id: record.skill_id,
                class_mask: record.class_mask,
            },
        );
        return None;
    };

    Some(SkillRaceClassInfoRecord {
        id: record.id,
        race_mask: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillRaceClassInfo,
            record.source,
            record.id,
            "RaceMask",
            record.race_mask,
            diagnostics,
        )?,
        skill_id,
        class_mask: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillRaceClassInfo,
            record.source,
            record.id,
            "ClassMask",
            record.class_mask,
            diagnostics,
        )?,
        flags: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillRaceClassInfo,
            record.source,
            record.id,
            "Flags",
            record.flags,
            diagnostics,
        )?,
        availability: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillRaceClassInfo,
            record.source,
            record.id,
            "Availability",
            record.availability,
            diagnostics,
        )?,
        min_level: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillRaceClassInfo,
            record.source,
            record.id,
            "MinLevel",
            record.min_level,
            diagnostics,
        )?,
        skill_tier_id: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillRaceClassInfo,
            record.source,
            record.id,
            "SkillTierID",
            record.skill_tier_id,
            diagnostics,
        )?,
    })
}

pub(super) fn checked_source_field_like_cpp<T>(
    table: SkillStoreTableLikeCpp,
    source: SkillStoreLoadSourceLikeCpp,
    record_id: u32,
    field: &'static str,
    value: i128,
    diagnostics: &mut Vec<SkillStoreLoadDiagnosticLikeCpp>,
) -> Option<T>
where
    T: TryFrom<i128>,
{
    match T::try_from(value) {
        Ok(value) => Some(value),
        Err(_) => {
            diagnostics.push(SkillStoreLoadDiagnosticLikeCpp::InvalidSourceField {
                table,
                source,
                record_id,
                field,
                value,
            });
            None
        }
    }
}

pub(super) fn append_conflicting_race_class_diagnostics_like_cpp(
    records: &[SkillRaceClassInfoRecord],
    diagnostics: &mut Vec<SkillStoreLoadDiagnosticLikeCpp>,
) {
    for (index, first) in records.iter().enumerate() {
        for second in &records[index + 1..] {
            if first.skill_id != second.skill_id
                || !race_masks_overlap_like_cpp(first.race_mask, second.race_mask)
                || !class_masks_overlap_like_cpp(first.class_mask, second.class_mask)
            {
                continue;
            }

            if same_race_class_payload_like_cpp(first, second) {
                continue;
            }

            diagnostics.push(SkillStoreLoadDiagnosticLikeCpp::ConflictingRaceClassInfo {
                skill_id: first.skill_id,
                first_record_id: first.id,
                second_record_id: second.id,
            });
        }
    }
}

pub(super) fn same_race_class_payload_like_cpp(
    first: &SkillRaceClassInfoRecord,
    second: &SkillRaceClassInfoRecord,
) -> bool {
    first.flags == second.flags
        && first.availability == second.availability
        && first.min_level == second.min_level
        && first.skill_tier_id == second.skill_tier_id
}

pub(super) fn race_masks_overlap_like_cpp(first: i64, second: i64) -> bool {
    first == 0 || second == 0 || first & second != 0
}

pub(super) fn class_masks_overlap_like_cpp(first: i32, second: i32) -> bool {
    matches!(first, -1 | 0) || matches!(second, -1 | 0) || first & second != 0
}

/// Check if a race matches a race mask. Mask of 0 means "all races".
pub(super) fn matches_race(mask: i64, race: u8) -> bool {
    mask == 0 || (mask & race_mask_for_race_like_cpp(race)) != 0
}

/// C++ `Trinity::RaceMask::GetMaskForRace`.
///
/// Returns zero for IDs that are not player races in C++ `Races`.
pub fn race_mask_for_race_like_cpp(race: u8) -> i64 {
    let bit = match race {
        1..=11 | 22 | 24..=32 => Some(race - 1),
        34 => Some(11),
        35 => Some(12),
        36 => Some(13),
        37 => Some(14),
        52 => Some(16),
        70 => Some(15),
        _ => None,
    };
    bit.map(|bit| 1_i64 << bit).unwrap_or(0)
}

/// Check if a class matches a class mask. Mask of 0 means "all classes".
pub(super) fn matches_class(mask: i32, class: u8) -> bool {
    if matches!(mask, -1 | 0) {
        return true;
    }
    if !(CLASS_WARRIOR_LIKE_CPP..MAX_CLASSES_LIKE_CPP).contains(&class) {
        return false;
    }
    mask & (1_i32 << (class - 1)) != 0
}
