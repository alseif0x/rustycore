// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! WorldSession adapter for the pure primary-profession planner.

use crate::session::WorldSession;
pub(crate) use wow_world_application::{
    DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP, MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP,
    NO_PRIMARY_PROFESSION_EQUIPMENT_SLOT_LIKE_CPP, PlannedPrimaryProfessionLikeCpp,
    PlayerSkillProfessionSnapshotLikeCpp, PrimaryProfessionCapacityAnalysisLikeCpp,
    PrimaryProfessionCapacityPlanErrorLikeCpp, PrimaryProfessionCapacityPlanLikeCpp,
    PrimaryProfessionEquipmentSlotLikeCpp, PrimaryProfessionSlotNormalizationLikeCpp,
    PrimaryProfessionSlotNormalizationReasonLikeCpp, analyze_primary_professions_like_cpp,
    plan_primary_professions_like_cpp,
};

impl WorldSession {
    pub(crate) fn plan_primary_profession_capacity_like_cpp(
        &self,
        requested_skill_ids: impl IntoIterator<Item = u32>,
    ) -> Result<PrimaryProfessionCapacityPlanLikeCpp, PrimaryProfessionCapacityPlanErrorLikeCpp>
    {
        let Some(skill_lines) = self.catalogs.skill_line_store() else {
            return Err(PrimaryProfessionCapacityPlanErrorLikeCpp::MissingSkillLineStore);
        };
        let Some(skills_loaded) = ({
            let (s, h) = crate::session::split_lifecycle_ref(self);
            s.resolved_player_skill_records_loaded_like_cpp(h)
        }) else {
            return Err(PrimaryProfessionCapacityPlanErrorLikeCpp::MissingPlayerSkillSnapshot);
        };
        if !skills_loaded {
            return Err(PrimaryProfessionCapacityPlanErrorLikeCpp::MissingPlayerSkillSnapshot);
        }
        let Some(skill_records) =
            crate::session::hub_ref(self).resolved_player_skill_records_like_cpp()
        else {
            return Err(PrimaryProfessionCapacityPlanErrorLikeCpp::MissingPlayerSkillSnapshot);
        };
        let current_skills =
            skill_records
                .values()
                .map(|skill| PlayerSkillProfessionSnapshotLikeCpp {
                    skill_id: u32::from(skill.skill_id),
                    value: skill.value,
                    profession_slot: skill.profession_slot,
                });

        let analysis = analyze_primary_professions_like_cpp(
            self.max_primary_trade_skills_like_cpp(),
            skill_lines,
            current_skills,
        )?;
        plan_primary_professions_like_cpp(&analysis, skill_lines, requested_skill_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, sync::Arc};

    use crate::session::RepresentedPlayerSkillLikeCpp;
    use wow_data::{SkillLineEntry, SkillLineStore};

    fn skill_line(id: u32, category_id: i8, parent_skill_line_id: u32) -> SkillLineEntry {
        SkillLineEntry {
            id,
            display_name: String::new(),
            alternate_verb: String::new(),
            description: String::new(),
            horde_display_name: String::new(),
            override_source_info_display_name: String::new(),
            category_id,
            spell_icon_file_id: 0,
            can_link: 0,
            parent_skill_line_id,
            parent_tier_index: 0,
            flags: 0,
            spell_book_spell_id: 0,
        }
    }

    fn skill_lines() -> SkillLineStore {
        SkillLineStore::from_entries([
            skill_line(100, 11, 0),
            skill_line(200, 11, 0),
            skill_line(300, 11, 0),
            skill_line(400, 11, 0),
            skill_line(101, 11, 100),
            skill_line(500, 9, 0),
        ])
    }

    #[test]
    fn world_session_capacity_is_loaded_fail_closed_and_independent_from_talent_points() {
        let (_packet_tx, packet_rx) = flume::bounded(1);
        let (send_tx, _send_rx) = flume::bounded(1);
        let mut session = WorldSession::new(
            1,
            "profession-test".to_string(),
            0,
            2,
            2,
            54_261,
            vec![0; 40],
            "enUS".to_string(),
            packet_rx,
            send_tx,
            crate::session::registry::build_dispatch_table(),
        );

        assert_eq!(
            session.max_primary_trade_skills_like_cpp(),
            DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP
        );
        for (configured, expected) in [(0, 0), (1, 1), (2, 2), (11, 11), (12, 2)] {
            session.set_max_primary_trade_skills_like_cpp(configured);
            assert_eq!(
                session.max_primary_trade_skills_like_cpp(),
                expected,
                "runtime value {configured}"
            );
        }

        assert_eq!(
            session.plan_primary_profession_capacity_like_cpp([300]),
            Err(PrimaryProfessionCapacityPlanErrorLikeCpp::MissingSkillLineStore),
            "missing immutable metadata must fail closed"
        );

        session.set_skill_line_store(Arc::new(skill_lines()));
        assert_eq!(
            session.plan_primary_profession_capacity_like_cpp([300]),
            Err(PrimaryProfessionCapacityPlanErrorLikeCpp::MissingPlayerSkillSnapshot),
            "an empty pre-login mirror must not be treated as free capacity"
        );

        session.set_player_skill_records_like_cpp(HashMap::from([
            (
                100,
                RepresentedPlayerSkillLikeCpp {
                    skill_id: 100,
                    step: 1,
                    value: 1,
                    max: 75,
                    profession_slot: 0,
                    state: crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
                },
            ),
            (
                200,
                RepresentedPlayerSkillLikeCpp {
                    skill_id: 200,
                    step: 1,
                    value: 1,
                    max: 75,
                    profession_slot: 1,
                    state: crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
                },
            ),
        ]));
        session.set_max_primary_trade_skills_like_cpp(3);
        session.set_player_character_points_like_cpp(99);
        let with_talent_points = session
            .plan_primary_profession_capacity_like_cpp([300])
            .unwrap();
        assert_eq!(session.player_character_points_like_cpp(), 99);

        session.set_player_character_points_like_cpp(0);
        let without_talent_points = session
            .plan_primary_profession_capacity_like_cpp([300])
            .unwrap();
        assert_eq!(with_talent_points, without_talent_points);
        assert_eq!(session.player_character_points_like_cpp(), 0);

        session.set_max_primary_trade_skills_like_cpp(12);
        assert_eq!(session.max_primary_trade_skills_like_cpp(), 2);
    }
}
