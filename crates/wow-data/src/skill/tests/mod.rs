//! Skill tier and pet spell stores regression scenarios.
//!
//! Separated from the skill.rs root under #638.

use super::*;

const DATA_DIR: &str = "/home/server/woltk-server-core/Data";
const LOCALE: &str = "esES";

fn load_store() -> Option<SkillStore> {
    let path = Path::new(DATA_DIR)
        .join("dbc")
        .join(LOCALE)
        .join("SkillLineAbility.db2");
    if !path.exists() {
        eprintln!("Skipping test: SkillLineAbility.db2 not found");
        return None;
    }
    Some(SkillStore::load(DATA_DIR, LOCALE).expect("failed to load SkillStore"))
}

fn ability(id: u32, skill_line: u16, spell: i32) -> SkillLineAbilityRecord {
    SkillLineAbilityRecord {
        id,
        race_mask: 0,
        skill_line,
        spell,
        min_skill_line_rank: 0,
        class_mask: 0,
        supercedes_spell: 0,
        acquire_method: 0,
        trivial_rank_high: 0,
        trivial_rank_low: 0,
        flags: 0,
        num_skill_ups: 0,
        skillup_skill_line_id: 0,
    }
}

fn race_class_info(
    id: u32,
    skill_id: u16,
    flags: u16,
    availability: i8,
    min_level: i8,
    skill_tier_id: i16,
) -> SkillRaceClassInfoRecord {
    SkillRaceClassInfoRecord {
        id,
        race_mask: 1,
        skill_id,
        class_mask: 1,
        flags,
        availability,
        min_level,
        skill_tier_id,
    }
}

fn skill_line(id: u32, category_id: i8) -> crate::SkillLineEntry {
    crate::SkillLineEntry {
        id,
        display_name: String::new(),
        alternate_verb: String::new(),
        description: String::new(),
        horde_display_name: String::new(),
        override_source_info_display_name: String::new(),
        category_id,
        spell_icon_file_id: 0,
        can_link: 0,
        parent_skill_line_id: 0,
        parent_tier_index: 0,
        flags: 0,
        spell_book_spell_id: 0,
    }
}

fn ability_source(
    record: SkillLineAbilityRecord,
    source: SkillStoreLoadSourceLikeCpp,
) -> SkillLineAbilitySourceRecordLikeCpp {
    SkillLineAbilitySourceRecordLikeCpp {
        source,
        id: record.id,
        race_mask: i128::from(record.race_mask),
        skill_line: i128::from(record.skill_line),
        spell: i128::from(record.spell),
        min_skill_line_rank: i128::from(record.min_skill_line_rank),
        class_mask: i128::from(record.class_mask),
        supercedes_spell: i128::from(record.supercedes_spell),
        acquire_method: i128::from(record.acquire_method),
        trivial_rank_high: i128::from(record.trivial_rank_high),
        trivial_rank_low: i128::from(record.trivial_rank_low),
        flags: i128::from(record.flags),
        num_skill_ups: i128::from(record.num_skill_ups),
        skillup_skill_line_id: i128::from(record.skillup_skill_line_id),
    }
}

fn race_class_source(
    record: SkillRaceClassInfoRecord,
    source: SkillStoreLoadSourceLikeCpp,
) -> SkillRaceClassInfoSourceRecordLikeCpp {
    SkillRaceClassInfoSourceRecordLikeCpp {
        source,
        id: record.id,
        race_mask: i128::from(record.race_mask),
        skill_id: i128::from(record.skill_id),
        class_mask: i128::from(record.class_mask),
        flags: i128::from(record.flags),
        availability: i128::from(record.availability),
        min_level: i128::from(record.min_level),
        skill_tier_id: i128::from(record.skill_tier_id),
    }
}

fn pet_ability(id: u32, skill_line: u16, spell: i32, acquire_method: i8) -> SkillLineAbilityRecord {
    SkillLineAbilityRecord {
        acquire_method,
        ..ability(id, skill_line, spell)
    }
}

fn creature_family(id: u32, skill_line: [i16; 2]) -> CreatureFamilyEntry {
    CreatureFamilyEntry {
        id,
        name: String::new(),
        min_scale: 0.0,
        min_scale_level: 0,
        max_scale: 0.0,
        max_scale_level: 0,
        pet_food_mask: 0,
        pet_talent_type: 0,
        category_enum_id: 0,
        icon_file_id: 0,
        skill_line,
    }
}

fn skill_tier_row(id: u32, value: [u32; MAX_SKILL_STEP_LIKE_CPP]) -> SkillTiersRowLikeCpp {
    SkillTiersRowLikeCpp { id, value }
}

fn pet_default_template(
    entry: u32,
    family: u32,
    spells: [u32; MAX_CREATURE_SPELL_DATA_SLOT_LIKE_CPP],
) -> PetDefaultSpellCreatureTemplateLikeCpp {
    PetDefaultSpellCreatureTemplateLikeCpp {
        entry,
        family,
        spells,
    }
}

fn summon_spell(difficulty_none: bool, effect: u32, misc_value: i32) -> PetDefaultSpellInfoLikeCpp {
    PetDefaultSpellInfoLikeCpp {
        difficulty_none,
        effects: vec![PetDefaultSpellEffectLikeCpp { effect, misc_value }],
    }
}

mod scenarios_1;
mod scenarios_2;
