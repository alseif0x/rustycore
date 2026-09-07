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
        let rc_info = self.skill_race_class_info_like_cpp(skill_id, race, class)?;
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
}
