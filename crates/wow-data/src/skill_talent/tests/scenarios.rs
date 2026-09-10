//! Skill and talent store regressions.
//!
//! Moved out of skill_talent.rs under #685; every test is unchanged.

use super::*;

#[test]
fn glyph_required_spec_uses_cpp_parent_relationship() {
    let store = GlyphRequiredSpecStore::from_entries([GlyphRequiredSpecEntry {
        id: 1,
        chr_specialization_id: 2,
        glyph_properties_id: 3,
    }]);

    assert_eq!(store.get(1).unwrap().glyph_properties_id, 3);
}

#[test]
fn profession_skill_for_exp_matches_cpp_parent_child_rules() {
    let store = SkillLineStore::from_entries([
        skill_line(356, 9, 0, 0),
        skill_line(900, 9, 356, i32::MIN),
        skill_line(1_000, 9, 356, 4),
        skill_line(1_001, 9, 356, 5),
        skill_line(777, 11, 0, 0),
        skill_line(2_000, 11, 777, 6),
        skill_line(3_000, 7, 0, 0),
    ]);

    assert_eq!(store.profession_skill_for_exp_like_cpp(356, 0), 1_000);
    assert_eq!(store.profession_skill_for_exp_like_cpp(356, 1), 1_001);
    assert_eq!(store.profession_skill_for_exp_like_cpp(777, 2), 2_000);
    assert_eq!(store.profession_skill_for_exp_like_cpp(1_000, 0), 0);
    assert_eq!(store.profession_skill_for_exp_like_cpp(3_000, 0), 0);
    assert_eq!(store.profession_skill_for_exp_like_cpp(999, 0), 0);
}

#[test]
fn profession_skill_for_negative_expansion_uses_current_expansion_like_cpp() {
    let store =
        SkillLineStore::from_entries([skill_line(356, 9, 0, 0), skill_line(1_002, 9, 356, 6)]);

    assert_eq!(store.profession_skill_for_exp_like_cpp(356, -3), 1_002);
}

#[test]
fn primary_profession_requires_root_category_eleven_and_distinguishes_payload_state() {
    let store = SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
        [
            skill_line(100, 11, 0, 0),
            skill_line(101, 11, 100, 4),
            skill_line(200, 9, 0, 0),
            skill_line(400, 11, 0, 0),
        ],
        [101, 200, 300, 400],
    );

    assert_eq!(
        store.is_primary_profession_skill_like_cpp(100),
        Some(false),
        "a hydrated row removed from effective authority matches failed C++ LookupEntry"
    );
    assert_eq!(store.is_primary_profession_skill_like_cpp(101), Some(false));
    assert_eq!(store.is_primary_profession_skill_like_cpp(400), Some(true));
    assert_eq!(store.is_primary_profession_skill_like_cpp(200), Some(false));
    assert_eq!(
        store.is_primary_profession_skill_like_cpp(300),
        None,
        "effective identity without hydrated category/parent must remain distinguishable"
    );
    assert_eq!(
        store.acquisition_payload_like_cpp(300),
        SkillLineAcquisitionPayloadLikeCpp::Incomplete
    );
    assert!(!store.has_complete_acquisition_payload_like_cpp());
    assert_eq!(
        store.is_primary_profession_skill_like_cpp(999),
        Some(false),
        "C++ LookupEntry failure classifies a genuinely absent ID as non-primary"
    );
    assert_eq!(
        store.acquisition_payload_like_cpp(999),
        SkillLineAcquisitionPayloadLikeCpp::Absent
    );
}

#[test]
fn effective_skill_line_ids_include_overlay_only_records_without_payload() {
    let table_hash = 0xB53D_C9D6;
    let removals = Db2HotfixRemovalStoreLikeCpp::default();
    let effective_ids = compose_effective_skill_line_ids_like_cpp(
        [100, 101],
        [101, 200],
        [200, 300],
        table_hash,
        &removals,
    );
    let mut store =
        SkillLineStore::from_entries([skill_line(100, 9, 0, 0), skill_line(101, 9, 0, 0)]);
    store.effective_record_ids_like_cpp = effective_ids;

    assert_eq!(store.effective_record_count_like_cpp(), 4);
    assert!(store.contains_effective_record_like_cpp(100));
    assert!(store.contains_effective_record_like_cpp(200));
    assert!(store.contains_effective_record_like_cpp(300));
    assert!(
        store.get(200).is_none(),
        "SQL-only identity must not manufacture a hydrated payload"
    );
}

#[test]
fn primary_profession_classification_applies_official_then_custom_sql_collisions() {
    let effective_ids = HashSet::from([100, 200, 300]);
    let fields = compose_effective_skill_line_acquisition_fields_like_cpp(
        [
            (
                100,
                SkillLineAcquisitionFieldsLikeCpp {
                    category_id: 9,
                    parent_skill_line_id: 0,
                    parent_tier_index: 0,
                },
            ),
            (
                200,
                SkillLineAcquisitionFieldsLikeCpp {
                    category_id: 11,
                    parent_skill_line_id: 0,
                    parent_tier_index: 0,
                },
            ),
        ],
        [
            (
                100,
                SkillLineAcquisitionFieldsLikeCpp {
                    category_id: 11,
                    parent_skill_line_id: 0,
                    parent_tier_index: 0,
                },
            ),
            (
                300,
                SkillLineAcquisitionFieldsLikeCpp {
                    category_id: 11,
                    parent_skill_line_id: 0,
                    parent_tier_index: 0,
                },
            ),
        ],
        [(
            200,
            SkillLineAcquisitionFieldsLikeCpp {
                category_id: 11,
                parent_skill_line_id: 100,
                parent_tier_index: 4,
            },
        )],
        &effective_ids,
    );
    let mut store =
        SkillLineStore::from_entries([skill_line(100, 9, 0, 0), skill_line(200, 11, 0, 0)]);
    store.effective_record_ids_like_cpp = effective_ids;
    store.acquisition_fields_by_effective_record_like_cpp = fields;

    assert_eq!(
        store.is_primary_profession_skill_like_cpp(100),
        Some(true),
        "official SQL must replace stale WDC4 category/parent fields"
    );
    assert_eq!(
        store.is_primary_profession_skill_like_cpp(200),
        Some(false),
        "custom SQL must replace the official/WDC4 classification"
    );
    assert_eq!(
        store.is_primary_profession_skill_like_cpp(300),
        Some(true),
        "SQL-only rows are decidable when the exact required fields are hydrated"
    );
    assert_eq!(
        store.acquisition_payload_like_cpp(200),
        SkillLineAcquisitionPayloadLikeCpp::Complete(SkillLineAcquisitionFieldsLikeCpp {
            category_id: 11,
            parent_skill_line_id: 100,
            parent_tier_index: 4,
        })
    );
    assert_eq!(
        store
            .acquisition_children_for_parent_like_cpp(100)
            .collect::<Vec<_>>(),
        vec![(
            200,
            SkillLineAcquisitionPayloadLikeCpp::Complete(SkillLineAcquisitionFieldsLikeCpp {
                category_id: 11,
                parent_skill_line_id: 100,
                parent_tier_index: 4,
            },)
        )],
        "SetSkill parent activation must see final SQL parent/tier payload"
    );
    assert_eq!(
        store.profession_skill_for_exp_like_cpp(100, 0),
        200,
        "profession expansion lookup must use the effective parent/tier payload"
    );
    assert!(store.has_complete_acquisition_payload_like_cpp());
    assert!(
        store.get(300).is_none(),
        "classification hydration must not fabricate the remaining SkillLine payload"
    );
}

#[test]
fn parent_child_projection_retains_unhydrated_effective_identities() {
    let store = SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
        [
            skill_line(100, 11, 0, 0),
            skill_line(200, 11, 100, 4),
            skill_line(300, 11, 999, 4),
        ],
        [100, 150, 200, 300],
    );

    assert_eq!(
        store
            .acquisition_children_for_parent_like_cpp(100)
            .collect::<Vec<_>>(),
        vec![
            (150, SkillLineAcquisitionPayloadLikeCpp::Incomplete),
            (
                200,
                SkillLineAcquisitionPayloadLikeCpp::Complete(SkillLineAcquisitionFieldsLikeCpp {
                    category_id: 11,
                    parent_skill_line_id: 100,
                    parent_tier_index: 4,
                },),
            ),
        ],
        "an effective identity with unknown parentage must not disappear"
    );
    assert_eq!(
        store
            .acquisition_children_for_parent_like_cpp(999)
            .collect::<Vec<_>>(),
        vec![
            (150, SkillLineAcquisitionPayloadLikeCpp::Incomplete),
            (
                300,
                SkillLineAcquisitionPayloadLikeCpp::Complete(SkillLineAcquisitionFieldsLikeCpp {
                    category_id: 11,
                    parent_skill_line_id: 999,
                    parent_tier_index: 4,
                },),
            ),
        ],
    );
}

#[test]
fn final_invalid_skill_line_overlay_replaces_stale_payload_and_can_be_repaired_or_removed() {
    let root = SkillLineAcquisitionFieldsLikeCpp {
        category_id: 11,
        parent_skill_line_id: 0,
        parent_tier_index: 0,
    };
    let repaired_child = SkillLineAcquisitionFieldsLikeCpp {
        category_id: 11,
        parent_skill_line_id: 200,
        parent_tier_index: 4,
    };
    let effective_ids = HashSet::from([100, 200, 300]);
    let fields = compose_effective_skill_line_acquisition_payloads_like_cpp(
        [(100, root), (200, root), (300, root), (400, root)],
        [(100, None), (200, None), (400, None)],
        [(200, Some(repaired_child)), (300, None)],
        &effective_ids,
    );
    let mut store = SkillLineStore::from_entries([
        skill_line(100, 11, 0, 0),
        skill_line(200, 11, 0, 0),
        skill_line(300, 11, 0, 0),
        skill_line(400, 11, 0, 0),
    ]);
    store.effective_record_ids_like_cpp = effective_ids;
    store.acquisition_fields_by_effective_record_like_cpp = fields;

    assert_eq!(
        store.acquisition_payload_like_cpp(100),
        SkillLineAcquisitionPayloadLikeCpp::Incomplete,
        "an invalid official overlay must replace the stale WDC4 payload"
    );
    assert_eq!(
        store.acquisition_payload_like_cpp(200),
        SkillLineAcquisitionPayloadLikeCpp::Complete(repaired_child),
        "a valid custom overlay must repair an invalid official payload"
    );
    assert_eq!(
        store.acquisition_payload_like_cpp(300),
        SkillLineAcquisitionPayloadLikeCpp::Incomplete,
        "an invalid final custom overlay must replace the stale WDC4 payload"
    );
    assert_eq!(
        store.acquisition_payload_like_cpp(400),
        SkillLineAcquisitionPayloadLikeCpp::Absent,
        "a final removal must erase even an invalid SQL overlay identity"
    );
    assert!(!store.has_complete_acquisition_payload_like_cpp());
}

#[test]
fn typed_hotfix_application_retains_invalid_identity_and_custom_repairs_payload() {
    const TABLE_HASH: u32 = 0x51A1_0001;
    let mut base = SkillLineStore::from_entries([skill_line(100, 11, 0, 0)]);
    base.table_hash_like_cpp = Some(TABLE_HASH);
    let store = base
        .apply_hotfix_overlays_like_cpp(
            [SkillLineHotfixOverlayLikeCpp {
                id: 200,
                category_id: i128::from(i8::MAX) + 1,
                parent_skill_line_id: 100,
                parent_tier_index: 4,
            }],
            [SkillLineHotfixOverlayLikeCpp {
                id: 200,
                category_id: 11,
                parent_skill_line_id: 100,
                parent_tier_index: 4,
            }],
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap();

    assert!(store.contains_effective_record_like_cpp(200));
    assert_eq!(
        store.acquisition_payload_like_cpp(200),
        SkillLineAcquisitionPayloadLikeCpp::Complete(SkillLineAcquisitionFieldsLikeCpp {
            category_id: 11,
            parent_skill_line_id: 100,
            parent_tier_index: 4,
        })
    );
}

#[test]
fn effective_skill_line_ids_apply_only_final_matching_table_removals() {
    let table_hash = 0xB53D_C9D6;
    let other_table_hash = 0xAABB_CCDD;
    let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (table_hash, 100, 2),
        (table_hash, 100, 1),
        (table_hash, 101, 1),
        (table_hash, 101, 2),
        (other_table_hash, 102, 2),
        (table_hash, 200, 2),
    ]);

    let effective_ids = compose_effective_skill_line_ids_like_cpp(
        [100, 101, 102],
        [200],
        [],
        table_hash,
        &removals,
    );

    assert!(
        effective_ids.contains(&100),
        "a later non-removal status cancels the earlier removal"
    );
    assert!(!effective_ids.contains(&101));
    assert!(
        effective_ids.contains(&102),
        "another DB2 table hash must not erase SkillLine"
    );
    assert!(
        !effective_ids.contains(&200),
        "final removals also erase SQL-only records"
    );
}

#[test]
fn trainer_validation_uses_effective_skill_line_identity_instead_of_payload() {
    let table_hash = 0xB53D_C9D6;
    let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(table_hash, 100, 2)]);
    let effective_ids =
        compose_effective_skill_line_ids_like_cpp([100], [200], [], table_hash, &removals);
    let mut skill_lines = SkillLineStore::from_entries([skill_line(100, 9, 0, 0)]);
    skill_lines.effective_record_ids_like_cpp = effective_ids;

    let trainer = crate::trainer::TrainerStoreLikeCpp::from_rows_like_cpp(
        [crate::trainer::TrainerRowLikeCpp {
            id: 10,
            trainer_type: crate::trainer::TRAINER_TYPE_TRADESKILL_LIKE_CPP,
            greeting: String::new(),
        }],
        [
            crate::trainer::TrainerSpellRowLikeCpp {
                trainer_id: 10,
                spell: crate::trainer::TrainerSpellLikeCpp {
                    spell_id: 1_000,
                    money_cost: 0,
                    req_skill_line: 100,
                    req_skill_rank: 0,
                    req_ability: [0; 3],
                    req_level: 0,
                },
            },
            crate::trainer::TrainerSpellRowLikeCpp {
                trainer_id: 10,
                spell: crate::trainer::TrainerSpellLikeCpp {
                    spell_id: 1_001,
                    money_cost: 0,
                    req_skill_line: 200,
                    req_skill_rank: 0,
                    req_ability: [0; 3],
                    req_level: 0,
                },
            },
        ],
        [],
        [],
        |_| true,
        |skill_line_id| skill_lines.contains_effective_record_like_cpp(skill_line_id),
        |_| true,
        |_, _| true,
    );

    let loaded = trainer.store.get_trainer_like_cpp(10).unwrap();
    assert!(
        loaded.get_spell_like_cpp(1_000).is_none(),
        "a hydrated SkillLine removed by hotfix_data is not a valid trainer requirement"
    );
    assert!(
        loaded.get_spell_like_cpp(1_001).is_some(),
        "an SQL-only effective SkillLine identity is valid without fabricated payload"
    );
    assert_eq!(
        trainer.report.skipped_spells_missing_skill_line,
        vec![(10, 1_000, 100)]
    );
    assert!(skill_lines.get(200).is_none());
}

#[test]
fn load_skill_talent_db2_subbatch_when_fixtures_exist() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "esES";
    let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
    if !dbc_dir.exists() {
        eprintln!(
            "Skipping test: DB2 fixture directory not found at {}",
            dbc_dir.display()
        );
        return;
    }

    macro_rules! load_if_exists {
        ($file:literal, $store:ty) => {
            if dbc_dir.join($file).exists() {
                let _store = <$store>::load(data_dir, locale)
                    .unwrap_or_else(|error| panic!("failed to load {}: {error:#}", $file));
            }
        };
    }

    load_if_exists!("GlyphBindableSpell.db2", GlyphBindableSpellStore);
    load_if_exists!("GlyphProperties.db2", GlyphPropertiesStore);
    load_if_exists!("GlyphRequiredSpec.db2", GlyphRequiredSpecStore);
    load_if_exists!("GlyphSlot.db2", GlyphSlotStore);
    load_if_exists!("JournalEncounter.db2", JournalEncounterStore);
    load_if_exists!("JournalEncounterSection.db2", JournalEncounterSectionStore);
    load_if_exists!("JournalInstance.db2", JournalInstanceStore);
    load_if_exists!("JournalTier.db2", JournalTierStore);
    load_if_exists!("PvpSeason.db2", PvpSeasonStore);
    load_if_exists!("PvpTalent.db2", PvpTalentStore);
    load_if_exists!("PvpTalentCategory.db2", PvpTalentCategoryStore);
    load_if_exists!("PvpTalentSlotUnlock.db2", PvpTalentSlotUnlockStore);
    load_if_exists!("PvpTier.db2", PvpTierStore);
    if dbc_dir.join("SkillLine.db2").exists() {
        let store = SkillLineStore::load(data_dir, locale)
            .unwrap_or_else(|error| panic!("failed to load SkillLine.db2: {error:#}"));
        assert_eq!(
            store.table_hash_like_cpp(),
            Some(0xB53D_C9D6),
            "the 3.4.3 fixture must expose the WDC4 table hash, not layout hash 0x5CB7F941"
        );
    }
    load_if_exists!("SkillLineXTraitTree.db2", SkillLineXTraitTreeStore);
    load_if_exists!("Talent.db2", TalentStore);
    load_if_exists!("TalentTab.db2", TalentTabStore);
}
