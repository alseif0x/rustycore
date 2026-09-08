use super::*;

fn skill_expectation() -> Expectation {
    Expectation::Skill {
        id: 118,
        value: 1,
        max: 1,
    }
}

fn skill_state() -> State {
    State {
        money: 1_000_000,
        saved_spell: false,
        skill_root: Some(SkillRoot {
            id: 118,
            value: 1,
            max: 1,
        }),
    }
}

#[test]
fn default_plan_keeps_direct_spell_persistence() {
    let plan: Plan = serde_json::from_str(r#"{"expected_spell":6197,"action":"verify"}"#).unwrap();
    assert_eq!(plan.persistence, Expectation::DirectSpell {});
    let mut observed = skill_state();
    assert!(!plan.persistence.matches(&observed));
    observed.saved_spell = true;
    assert!(plan.persistence.verify_login(&observed, true).is_ok());
    assert!(plan.persistence.verify_login(&observed, false).is_err());
}

#[test]
fn skill_root_and_fresh_login_knowledge_are_both_required() {
    let expected = skill_expectation();
    let mut observed = skill_state();
    assert!(expected.matches(&observed));
    assert!(expected.verify_login(&observed, true).is_ok());
    assert!(expected.verify_login(&observed, false).is_err());
    observed.skill_root = None;
    assert!(expected.verify_login(&observed, true).is_err());
    for wrong in [
        SkillRoot {
            id: 119,
            value: 1,
            max: 1,
        },
        SkillRoot {
            id: 118,
            value: 0,
            max: 1,
        },
        SkillRoot {
            id: 118,
            value: 1,
            max: 2,
        },
    ] {
        observed.skill_root = Some(wrong);
        assert!(expected.verify_login(&observed, true).is_err());
    }
}

#[test]
fn direct_target_row_cannot_mask_skill_reconstruction() {
    let mut observed = skill_state();
    observed.saved_spell = true;
    assert!(skill_expectation().verify_login(&observed, true).is_err());
}

#[test]
fn typed_skill_contract_round_trips_and_rejects_invalid_fields() {
    let expected = skill_expectation();
    let encoded = serde_json::to_value(expected).unwrap();
    assert_eq!(
        encoded,
        serde_json::json!({"kind":"skill","id":118,"value":1,"max":1})
    );
    let plan: Plan = serde_json::from_value(serde_json::json!({
        "expected_spell":674, "action":"verify", "persistence":encoded,
    }))
    .unwrap();
    assert_eq!(plan.persistence, expected);
    for invalid in [
        r#"{"kind":"skill","id":118,"value":1}"#,
        r#"{"kind":"skill","id":118,"value":1,"max":1,"fallback":true}"#,
        r#"{"kind":"direct_spell","id":118}"#,
        r#"{"kind":"unknown"}"#,
        "null",
    ] {
        assert!(
            serde_json::from_str::<Expectation>(invalid).is_err(),
            "accepted {invalid}"
        );
    }
    for invalid in [
        Expectation::Skill {
            id: 0,
            value: 1,
            max: 1,
        },
        Expectation::Skill {
            id: 118,
            value: 0,
            max: 1,
        },
        Expectation::Skill {
            id: 118,
            value: 2,
            max: 1,
        },
    ] {
        assert!(invalid.validate().is_err());
    }
}
