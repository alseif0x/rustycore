//! Persisted skill hydration; distinct from acquiring a new skill.
use super::*;

impl SkillStore {
    /// C++ `Player::_LoadSkills` followed by `Player::UpdateSkillsForLevel`.
    pub fn loaded_skill_info_like_cpp(
        &self,
        skill_id: u16,
        race: u8,
        class: u8,
        level: u8,
        mut rank: u16,
        mut max_rank: u16,
        skill_line_store: &SkillLineStore,
        skill_tiers_store: &SkillTiersStoreLikeCpp,
    ) -> Option<SkillInfoEntry> {
        let rc_info = self.loaded_race_class_info_like_cpp(skill_id, race, class)?;
        match self.skill_range_type_like_cpp(rc_info, skill_line_store, skill_tiers_store) {
            SkillRangeTypeLikeCpp::Language => {
                rank = 300;
                max_rank = 300;
            }
            SkillRangeTypeLikeCpp::Level => {
                max_rank = u16::from(level).saturating_mul(5);
                if rc_info.flags & SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
                    rank = max_rank;
                }
            }
            SkillRangeTypeLikeCpp::Mono => {
                rank = 1;
                max_rank = 1;
            }
            SkillRangeTypeLikeCpp::Rank | SkillRangeTypeLikeCpp::None => {}
        }

        let step = match skill_line_store.acquisition_payload_like_cpp(u32::from(skill_id)) {
            SkillLineAcquisitionPayloadLikeCpp::Complete(skill)
                if matches!(
                    skill.category_id,
                    SKILL_CATEGORY_SECONDARY_LIKE_CPP | SKILL_CATEGORY_PROFESSION_LIKE_CPP
                ) =>
            // Pinned 3.4.3 C++ `Player::_LoadSkills` computes both secondary
            // and profession steps as `max / 75`. It does not reverse-map
            // custom `SkillTiersEntry::Value` rows on this load path.
            {
                max_rank / 75
            }
            _ => 0,
        };

        Some(SkillInfoEntry {
            skill_id,
            step,
            rank,
            starting_rank: 1,
            max_rank,
            temp_bonus: 0,
            perm_bonus: 0,
        })
    }

    fn loaded_race_class_info_like_cpp(
        &self,
        skill_id: u16,
        race: u8,
        class: u8,
    ) -> Option<&SkillRaceClassInfoRecord> {
        // C++ Player::_LoadSkills (25723) does not consult Availability or
        // MinLevel. Those govern acquisition, not an already persisted skill.
        // Do not pick arbitrarily when candidates disagree on consumed fields,
        // or recover from an invalid/missing source by ignoring its diagnostic.
        if self
            .invalid_race_class_by_skill_like_cpp
            .get(&skill_id)
            .is_some_and(|diagnostics| {
                diagnostics.iter().any(|diagnostic| {
                    !matches!(
                        diagnostic,
                        SkillStoreLoadDiagnosticLikeCpp::ConflictingRaceClassInfo { .. }
                    )
                })
            })
        {
            return None;
        }
        let candidates = self.skill_race_class_info_candidates_like_cpp(skill_id, race, class);
        let first = *candidates.first()?;
        candidates
            .iter()
            .all(|candidate| {
                candidate.flags == first.flags && candidate.skill_tier_id == first.skill_tier_id
            })
            .then_some(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bow_rows() -> [SkillRaceClassInfoRecord; 2] {
        let specific = SkillRaceClassInfoRecord {
            id: 126,
            race_mask: 650,
            skill_id: 45,
            class_mask: 4,
            flags: 128,
            availability: 1,
            min_level: 0,
            skill_tier_id: 0,
        };
        let broad = SkillRaceClassInfoRecord {
            id: 127,
            race_mask: 32767,
            class_mask: 13,
            availability: 0,
            ..specific.clone()
        };
        [specific, broad]
    }

    fn store(rows: [SkillRaceClassInfoRecord; 2]) -> SkillStore {
        let mut store = SkillStore::from_skill_line_abilities_and_race_class_like_cpp([], rows);
        // Exercise the production diagnostic route, not only a raw DB2 store.
        store.invalid_race_class_by_skill_like_cpp.insert(
            45,
            vec![SkillStoreLoadDiagnosticLikeCpp::ConflictingRaceClassInfo {
                skill_id: 45,
                first_record_id: 126,
                second_record_id: 127,
            }],
        );
        store
    }

    fn bow_line() -> SkillLineStore {
        SkillLineStore::from_entries([crate::SkillLineEntry {
            id: 45,
            display_name: String::new(),
            alternate_verb: String::new(),
            description: String::new(),
            horde_display_name: String::new(),
            override_source_info_display_name: String::new(),
            category_id: 0,
            spell_icon_file_id: 0,
            can_link: 0,
            parent_skill_line_id: 0,
            parent_tier_index: 0,
            flags: 0,
            spell_book_spell_id: 0,
        }])
    }

    #[test]
    fn persisted_bows_ignore_acquisition_only_overlap_without_allowing_acquisition() {
        let [a, b] = bow_rows();
        for rows in [[a.clone(), b.clone()], [b, a]] {
            let store = store(rows);
            assert!(store.skill_race_class_info_like_cpp(45, 10, 3).is_none());
            let loaded = store
                .loaded_skill_info_like_cpp(
                    45,
                    10,
                    3,
                    3,
                    1,
                    15,
                    &bow_line(),
                    &SkillTiersStoreLikeCpp::default(),
                )
                .expect("existing bow skill has identical load semantics in both rows");
            assert_eq!((loaded.skill_id, loaded.rank, loaded.max_rank), (45, 1, 15));
            assert!(store.loaded_race_class_info_like_cpp(45, 10, 2).is_none());
        }
    }

    #[test]
    fn persisted_load_rejects_conflicting_consumed_fields_and_invalid_sources() {
        let mut rows = bow_rows();
        rows[1].flags ^= SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP;
        assert!(
            store(rows)
                .loaded_race_class_info_like_cpp(45, 10, 3)
                .is_none()
        );
        let mut rows = bow_rows();
        rows[1].skill_tier_id = 1;
        assert!(
            store(rows)
                .loaded_race_class_info_like_cpp(45, 10, 3)
                .is_none()
        );
        let mut store = store(bow_rows());
        store
            .invalid_race_class_by_skill_like_cpp
            .get_mut(&45)
            .unwrap()
            .push(SkillStoreLoadDiagnosticLikeCpp::MissingEffectiveSkillLine {
                record_id: 126,
                skill_id: 45,
            });
        assert!(store.loaded_race_class_info_like_cpp(45, 10, 3).is_none());
        assert!(store.loaded_race_class_info_like_cpp(46, 10, 3).is_none());
    }
}
