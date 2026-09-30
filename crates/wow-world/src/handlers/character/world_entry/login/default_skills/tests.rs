//! Default-skill admission keeps its independent owner/failure boundary.

use super::*;

fn session() -> WorldSession {
    let (_, incoming) = flume::bounded(10);
    let (outgoing, _) = flume::unbounded();
    WorldSession::new(1, "DefaultSkills".into(), 0, 2, 9, 54261,
        vec![0; 40], "enUS".into(), incoming, outgoing)
}

#[test]
fn absent_default_skill_stores_skip_owner_write() {
    let mut session = session();
    assert_eq!(session.player_guid(), None);
    let mut records = HashMap::new();
    let mut information = BTreeMap::new();
    assert_eq!(session.apply_default_skills_for_login_like_cpp(1, 1, 12, &mut records, &mut information), Some(Vec::new()));
    assert!(!session.player_skill_records_loaded_like_cpp(), "missing stores skip even the empty installation");
}

#[test]
fn empty_default_skill_pass_still_fails_for_stale_owner() {
    let mut session = session();
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(wow_map::MapManager::default())));
    session.set_skill_store(Arc::new(wow_data::SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
        std::iter::empty::<wow_data::SkillLineAbilityRecord>(),
        std::iter::empty::<wow_data::SkillRaceClassInfoRecord>(),
    )));
    session.set_skill_line_store(Arc::new(wow_data::SkillLineStore::from_entries(std::iter::empty::<wow_data::SkillLineEntry>())));
    session.set_skill_tiers_store(Arc::new(wow_data::SkillTiersStoreLikeCpp::default()));
    let mut records = HashMap::new();
    let mut information = BTreeMap::new();
    assert_eq!(session.apply_default_skills_for_login_like_cpp(1, 1, 12, &mut records, &mut information), None);
    assert!(records.is_empty());
    assert!(information.is_empty());
}

