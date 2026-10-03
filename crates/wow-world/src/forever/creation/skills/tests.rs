mod fixtures;
mod lookup;
use super::*;
use fixtures::{birth, line, rc, world};
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{BirthRecords, RACE_CLASS_HASH},
};
use wow_persistence::forever::creation::SkillTierRow;

#[test]
fn shared_live_rule_preserves_source_signed_level_zero_promotion_without_relaxing_startup_admission()
 {
    let mut ranked = rc(1, 10);
    ranked.tier = 1;
    let ordinary = rc(2, 20);
    let sources = world(
        1,
        6,
        vec![SkillTierRow {
            id: 1,
            values: [75; 16],
        }],
    )
    .with_birth_skills(birth(
        vec![line(10, 6), line(20, 6)],
        vec![ranked, ordinary],
    ))
    .unwrap();
    let ranked = sources.default_skill_request(&ranked, 6, 0).unwrap();
    let ordinary = sources.default_skill_request(&ordinary, 6, 0).unwrap();
    // uint8 0 promotes to int: (-1)*5 narrows to uint16 65531,
    // then max(1,...)/min(cap) produces 75 for rank range and 0 for level.
    assert_eq!((ranked.rank(), ranked.maximum()), (75, 75));
    assert_eq!((ordinary.rank(), ordinary.maximum()), (0, 0));
    assert!(matches!(
        sources.initial_skill_fields(1, 6, 0),
        Err(SourceError::InvalidLevel)
    ));
}

fn request_values(
    sources: &WorldSources,
    race: u8,
    class: u8,
    level: u8,
) -> Vec<(u32, u16, u16, u16, u16)> {
    sources
        .initial_skill_fields(race, class, level)
        .unwrap()
        .default_requests()
        .iter()
        .map(|r| {
            (
                r.source_record(),
                r.skill(),
                r.step(),
                r.rank(),
                r.maximum(),
            )
        })
        .collect()
}

#[test]
fn source_preallocation_is_sorted_capped_and_does_not_apply_default_filters_or_learn() {
    let mut nondefault = rc(1, 20);
    nondefault.availability = 0;
    nondefault.min_level = 100;
    let mut future = rc(2, 10);
    future.min_level = 100;
    let catalog = birth(
        vec![line(20, 6), line(10, 6), line(30, 6)],
        vec![nondefault, future],
    );
    let sources = world(1, 1, vec![])
        .with_birth_skills(Arc::clone(&catalog))
        .unwrap();
    let result = sources.initial_skill_fields(1, 1, 1).unwrap();
    assert_eq!(
        result
            .fields()
            .iter()
            .map(|f| (f.skill(), f.slot(), f.line_field(), f.starting_rank()))
            .collect::<Vec<_>>(),
        [(10, 0, 10, 1), (20, 1, 20, 1)]
    );
    assert!(result.default_requests().is_empty()); // no learned skill from StartingRank=1
    let mut state = result.initialize_player_skills();
    assert_eq!(
        state.status(10),
        Some((0, crate::forever::player::SkillUpdateState::Unchanged))
    );
    assert_eq!(state.fields()[0].line(), 10);
    assert_eq!(state.fields()[0].starting_rank(), 1);
    assert!(!state.has_skill(10));
    state.modify_bonus(&catalog, 10, 50, false).unwrap();
    state.modify_bonus(&catalog, 10, 50, true).unwrap();
    assert_eq!(state.fields()[0].temporary_bonus(), 0);
    assert_eq!(state.fields()[0].permanent_bonus(), 0);
    assert!(!state.has_skill(10)); // Bonuses cannot learn a preallocated skill.
    assert_eq!(sources.birth_skill_source_counts(), Some([1, 2, 1]));

    let catalog = birth(
        (1..=301).rev().map(|id| line(id, 6)).collect(),
        (1..=301).rev().map(|id| rc(id, id as u16)).collect(),
    );
    let sources = world(1, 1, vec![]).with_birth_skills(catalog).unwrap();
    let result = sources.initial_skill_fields(1, 1, 1).unwrap();
    assert_eq!(result.fields().len(), 300);
    assert_eq!(result.fields().last().unwrap().skill(), 300);
    // A skill beyond field capacity still has a Source default call; the
    // actor's SetSkill handles admission, never pretend the call succeeded.
    assert_eq!(result.default_requests().len(), 301);
}

#[test]
fn default_requests_keep_rc_storage_order_duplicates_signed_level_and_bit_masks() {
    let mut early = rc(5, 20);
    early.min_level = -128;
    early.class_mask = -1;
    early.race_mask = 1 << 32; // target race 95 is not bit 94
    let mut late = rc(8, 10);
    late.min_level = 2;
    let mut wrong_class = rc(1, 10);
    wrong_class.class_mask = 2;
    let mut wrong_race = rc(2, 10);
    wrong_race.race_mask = 1 << 33;
    let mut unavailable = rc(3, 10);
    unavailable.availability = 2;
    let catalog = birth(
        vec![line(20, 6), line(10, 6)],
        vec![
            late,
            rc(7, 20),
            early,
            wrong_class,
            wrong_race,
            unavailable,
            rc(9, 999),
        ],
    );
    let sources = world(95, 1, vec![]).with_birth_skills(catalog).unwrap();
    assert_eq!(
        request_values(&sources, 95, 1, 1),
        [(5, 20, 0, 1, 5), (7, 20, 0, 1, 5)]
    );
    assert_eq!(
        request_values(&sources, 95, 1, 2),
        [(5, 20, 0, 1, 10), (7, 20, 0, 1, 10), (8, 10, 0, 1, 10)]
    );
    // Missing SkillLine yields RANGE_NONE; cannot manufacture a grant to 999.
    assert_eq!(sources.birth_skill_source_counts(), Some([1, 2, 4]));
}

#[test]
fn tier_precedence_source_uint16_narrowing_language_mono_and_flags_are_exact() {
    let mut ranked_language = rc(1, 10);
    ranked_language.tier = 7;
    let mut flagged = rc(5, 20);
    flagged.flags = 0x10;
    let mut mono_display_only = rc(6, 21);
    mono_display_only.flags = 0x400; // not a server range classifier
    let mut missing_tier = rc(7, 22);
    missing_tier.tier = -1; // promoted to uint32; no alias to tier 65535
    let mut zero_tier = rc(8, 23);
    zero_tier.tier = 8;
    zero_tier.flags = 0x10;
    let mut values = [0; 16];
    values[0] = 70_000;
    let catalog = birth(
        vec![
            line(10, 10),
            line(11, 10),
            line(12, 8),
            line(960, 6),
            line(20, 6),
            line(21, 6),
            line(22, 6),
            line(23, 8),
        ],
        vec![
            ranked_language,
            rc(2, 11),
            rc(3, 12),
            rc(4, 960),
            flagged,
            mono_display_only,
            missing_tier,
            zero_tier,
        ],
    );
    let sources = world(
        1,
        1,
        vec![
            SkillTierRow { id: 7, values },
            SkillTierRow {
                id: 8,
                values: [0; 16],
            },
            SkillTierRow { id: 65535, values },
        ],
    )
    .with_birth_skills(catalog)
    .unwrap();
    assert_eq!(
        request_values(&sources, 1, 1, 3),
        [
            (1, 10, 1, 1, 70_000u32 as u16),
            (2, 11, 0, 300, 300),
            (3, 12, 0, 1, 1),
            (4, 960, 0, 1, 1),
            (5, 20, 0, 15, 15),
            (6, 21, 0, 1, 15),
            (7, 22, 0, 1, 15),
            (8, 23, 1, 0, 0),
        ]
    );
}

#[test]
fn death_knight_values_clamp_to_real_tier_or_level_max_and_always_max_wins() {
    let mut ranked = rc(2, 20);
    ranked.tier = 7;
    let mut maxed = rc(3, 30);
    maxed.tier = 7;
    maxed.flags = 0x10;
    let mut values = [0; 16];
    values[0] = 75;
    let catalog = birth(
        vec![line(10, 6), line(20, 6), line(30, 6)],
        vec![rc(1, 10), ranked, maxed],
    );
    let sources = world(1, 6, vec![SkillTierRow { id: 7, values }])
        .with_birth_skills(catalog)
        .unwrap();
    assert_eq!(
        request_values(&sources, 1, 6, 1),
        [(1, 10, 0, 1, 5), (2, 20, 1, 1, 75), (3, 30, 1, 75, 75)]
    );
    assert_eq!(
        request_values(&sources, 1, 6, 55),
        [(1, 10, 0, 270, 275), (2, 20, 1, 75, 75), (3, 30, 1, 75, 75)]
    );
    assert_eq!(
        request_values(&sources, 1, 6, 255)[0],
        (1, 10, 0, 1270, 1275)
    );
}

#[test]
fn effective_removals_and_overlays_feed_one_shared_owner_not_a_baseline_mirror() {
    let baseline = BirthRecords {
        skill_lines: vec![line(10, 6), line(20, 6)],
        race_class: vec![rc(1, 10), rc(2, 20)],
        ..Default::default()
    };
    let mut maxed = rc(1, 10);
    maxed.flags = 0x10;
    let official = BirthRecords {
        race_class: vec![maxed],
        ..Default::default()
    };
    let catalog = Arc::new(
        baseline
            .finish(
                official,
                Default::default(),
                &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(RACE_CLASS_HASH, 2, 2)]),
            )
            .unwrap(),
    );
    let sources = world(1, 1, vec![])
        .with_birth_skills(catalog.clone())
        .unwrap();
    assert!(Arc::ptr_eq(
        &sources.skill_sources.as_ref().unwrap().birth,
        &catalog
    ));
    assert_eq!(request_values(&sources, 1, 1, 4), [(1, 10, 0, 20, 20)]);
    assert_eq!(
        sources
            .initial_skill_fields(1, 1, 4)
            .unwrap()
            .fields()
            .len(),
        1
    );
}

#[test]
fn missing_composition_and_bad_identity_fail_while_empty_real_skill_sources_are_allowed() {
    let sources = world(1, 1, vec![]);
    assert!(matches!(
        sources.initial_skill_fields(1, 1, 1),
        Err(SourceError::MissingBirthSkillSources)
    ));
    assert!(matches!(
        sources.initial_skill_fields(1, 1, 0),
        Err(SourceError::InvalidLevel)
    ));
    assert!(matches!(
        sources.initial_skill_fields(2, 1, 1),
        Err(SourceError::MissingDefinition)
    ));
    let sources = sources.with_birth_skills(birth(vec![], vec![])).unwrap();
    let result = sources.initial_skill_fields(1, 1, 1).unwrap();
    assert!(result.fields().is_empty() && result.default_requests().is_empty());
    assert!(matches!(
        world(1, 0, vec![]).with_birth_skills(birth(vec![], vec![])),
        Err(SourceError::InvalidClass)
    ));
    assert!(matches!(
        world(1, 16, vec![]).with_birth_skills(birth(vec![], vec![])),
        Err(SourceError::InvalidClass)
    ));
}
