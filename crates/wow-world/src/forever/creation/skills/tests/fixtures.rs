use super::super::*;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{BirthRecords, SkillLineRecord},
    forever_game_tables::InitialGameTables,
};
use wow_persistence::forever::creation::{
    ClassLevelStats, CreationWorldRows, RaceStats, SkillTierRow, StartDefinition,
};

pub(super) fn line(id: u32, category: i8) -> SkillLineRecord {
    SkillLineRecord {
        id,
        category,
        spell_icon_file: 0,
        can_link: 0,
        parent_skill: 0,
        parent_tier_index: 0,
        flags: 0,
        spell_book_spell: 0,
        expansion_name_shared_string: 0,
        horde_expansion_name_shared_string: 0,
    }
}
pub(super) fn rc(id: u32, skill: u16) -> SkillRaceClassRecord {
    SkillRaceClassRecord {
        id,
        skill,
        class_mask: 0,
        flags: 0,
        availability: 1,
        min_level: 1,
        tier: 0,
        race_mask: 0,
    }
}
pub(super) fn birth(
    skill_lines: Vec<SkillLineRecord>,
    race_class: Vec<SkillRaceClassRecord>,
) -> Arc<BirthCatalog> {
    Arc::new(
        BirthRecords {
            skill_lines,
            race_class,
            ..Default::default()
        }
        .finish(
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap(),
    )
}
pub(super) fn world(race: u8, class: u8, skill_tiers: Vec<SkillTierRow>) -> WorldSources {
    let tables = InitialGameTables::parse_strs(
        "L\t1\t2\t3\t4\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\n",
        "L\tHealth\n",
        "L\tTotal\tKill\tJunk\tStats\tDivisor\n1\t400\t0\t0\t0\t0\n2\t500\t0\t0\t0\t0\n",
    )
    .unwrap();
    WorldSources::from_rows(
        CreationWorldRows {
            definitions: vec![StartDefinition {
                race,
                class,
                map: 0,
                position: [0.; 4],
                npe_map: None,
                npe_position: [None; 4],
                npe_transport: None,
                intro_movie: None,
                intro_scene: None,
                npe_intro_scene: None,
            }],
            race_stats: vec![RaceStats {
                race,
                modifiers: [0; 5],
            }],
            class_stats: vec![ClassLevelStats {
                class,
                level: 1,
                stats: [1; 5],
            }],
            skill_tiers,
            ..Default::default()
        },
        |_| true,
        |_| true,
        &tables,
        2,
    )
    .unwrap()
}
