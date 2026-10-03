use super::*;

#[test]
fn first_matching_candidate_follows_supplied_source_order_not_specificity_or_default_filters() {
    let mut wrong_race = rc(3, 10);
    wrong_race.race_mask = 2;
    let mut wrong_class = rc(4, 10);
    wrong_class.class_mask = 2;
    let mut generic = rc(2, 10);
    generic.availability = 0;
    generic.min_level = 100;
    generic.tier = -1;
    let mut specific = rc(1, 10);
    specific.class_mask = 1;
    specific.race_mask = 1;
    let raw = birth(
        vec![line(10, 6)],
        vec![specific, generic, wrong_race, wrong_class],
    );
    let s = world(1, 1, vec![])
        .with_birth_skills(raw.clone())
        .unwrap()
        .with_birth_skill_lookup(vec![4, 3, 2, 1])
        .unwrap();
    let selected = s.skill_race_class_info(10, 1, 1).unwrap().unwrap();
    assert_eq!(selected.id, 2);
    assert!(std::ptr::eq(selected, raw.race_class_record(2).unwrap()));
    assert_eq!(s.skill_race_class_info(10, 2, 1).unwrap().unwrap().id, 3);
    assert_eq!(s.skill_race_class_info(10, 1, 2).unwrap().unwrap().id, 4);
    assert!(s.skill_race_class_info(999, 1, 1).unwrap().is_none());
    assert_eq!(
        s.birth_skill_lookup_counts(),
        Some(SkillLookupCounts {
            records: 4,
            skills: 1,
            orphan_records: 0
        })
    );
    assert!(Arc::ptr_eq(&raw, &s.skill_sources.as_ref().unwrap().birth));
}

#[test]
fn orphan_rows_are_excluded_but_zero_skill_id_and_empty_lookups_are_valid() {
    let raw = birth(vec![line(0, 6)], vec![rc(0, 0), rc(7, 99)]);
    let s = world(1, 1, vec![])
        .with_birth_skills(raw)
        .unwrap()
        .with_birth_skill_lookup(vec![0])
        .unwrap();
    assert_eq!(s.skill_race_class_info(0, 1, 1).unwrap().unwrap().id, 0);
    assert!(s.skill_race_class_info(99, 1, 1).unwrap().is_none());
    assert_eq!(
        s.birth_skill_lookup_counts(),
        Some(SkillLookupCounts {
            records: 1,
            skills: 1,
            orphan_records: 1
        })
    );
    let s = world(1, 1, vec![])
        .with_birth_skills(birth(vec![], vec![rc(1, 1)]))
        .unwrap()
        .with_birth_skill_lookup(vec![])
        .unwrap();
    assert!(s.skill_race_class_info(1, 1, 1).unwrap().is_none());
    assert_eq!(s.birth_skill_lookup_counts().unwrap().orphan_records, 1);
}

#[test]
fn exact_set_and_contiguous_ascending_skill_group_admission_reject_fabricated_or_partial_orders() {
    for order in [
        vec![],
        vec![1, 2],
        vec![1, 1, 3],
        vec![1, 2, 9],
        vec![1, 2, 4],
        vec![3, 1, 2],
        vec![1, 3, 2],
    ] {
        let raw = birth(
            vec![line(10, 6), line(20, 6)],
            vec![rc(1, 10), rc(2, 10), rc(3, 20), rc(4, 99)],
        );
        let s = world(1, 1, vec![]).with_birth_skills(raw).unwrap();
        assert!(matches!(
            s.with_birth_skill_lookup(order),
            Err(SourceError::InvalidBirthSkillLookup)
        ));
    }
    let raw = birth(
        vec![line(10, 6), line(20, 6)],
        vec![rc(1, 10), rc(2, 10), rc(3, 20)],
    );
    let s = world(1, 1, vec![])
        .with_birth_skills(raw)
        .unwrap()
        .with_birth_skill_lookup(vec![2, 1, 3])
        .unwrap();
    assert_eq!(s.skill_race_class_info(10, 1, 1).unwrap().unwrap().id, 2);
}

#[test]
fn missing_phase_repeat_and_invalid_shift_inputs_have_explicit_admission() {
    let s = world(1, 1, vec![]);
    assert!(matches!(
        s.skill_race_class_info(1, 1, 1),
        Err(SourceError::MissingBirthSkillSources)
    ));
    assert!(matches!(
        s.with_birth_skill_lookup(vec![]),
        Err(SourceError::MissingBirthSkillSources)
    ));
    let s = world(1, 1, vec![])
        .with_birth_skills(birth(vec![], vec![]))
        .unwrap();
    assert!(matches!(
        s.skill_race_class_info(1, 1, 1),
        Err(SourceError::MissingBirthSkillLookup)
    ));
    let s = s.with_birth_skill_lookup(vec![]).unwrap();
    for class in [0, 16, 32, 255] {
        assert!(matches!(
            s.skill_race_class_info(1, 1, class),
            Err(SourceError::InvalidClass)
        ));
    }
    assert!(matches!(
        s.with_birth_skill_lookup(vec![]),
        Err(SourceError::BirthSkillLookupAlreadyLoaded)
    ));
}
