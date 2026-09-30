use super::*;
use super::fixtures_state::*;

#[test]
fn gossip_quest_text_filters_race_class_and_level_like_cpp() {
    let (mut session, _send_rx) = make_quest_status_session();
    set_loaded_player_identity_like_cpp(&mut session, 571, 10, 3, 3, 0);
    let entry = 15_278;

    let blood_elf_mask = 1u64 << (10 - 1);
    let hunter_mask = 1u32 << (3 - 1);
    let mage_mask = 1u32 << (8 - 1);
    let human_mask = 1u64 << (1 - 1);

    let mut generic = quest_template(8_325);
    generic.allowable_races = blood_elf_mask;
    generic.min_level = 1;
    generic.log_title = "Reclaiming Sunstrider Isle".into();

    let mut hunter = quest_template(9_393);
    hunter.allowable_races = blood_elf_mask;
    hunter.allowable_classes = hunter_mask;
    hunter.min_level = 1;
    hunter.log_title = "Hunter Training".into();

    let mut mage = quest_template(8_328);
    mage.allowable_races = blood_elf_mask;
    mage.allowable_classes = mage_mask;
    mage.min_level = 1;
    mage.log_title = "Mage Training".into();

    let mut too_high = quest_template(99_001);
    too_high.allowable_races = blood_elf_mask;
    too_high.min_level = 4;

    let mut wrong_race = quest_template(99_002);
    wrong_race.allowable_races = human_mask;
    wrong_race.min_level = 1;

    let mut store = QuestStore::from_quests_like_cpp([generic, hunter, mage, too_high, wrong_race]);
    store
        .starter_quests
        .entry(entry)
        .or_default()
        .extend([8_325, 9_393, 8_328, 99_001, 99_002]);
    session.set_quest_store(Arc::new(store));

    let quest_text = gossip_quest_text_for_test(&session, entry);

    assert_eq!(
        quest_text
            .iter()
            .map(|text| text.quest_id)
            .collect::<Vec<_>>(),
        vec![8_325, 9_393]
    );
    assert!(quest_text.iter().all(|text| text.quest_type == 2));
}

#[test]
fn gossip_quest_text_offers_sallina_followup_after_hunter_training_rewarded_like_cpp() {
    let (mut session, _send_rx) = make_quest_status_session();
    set_loaded_player_identity_like_cpp(&mut session, 530, 10, 3, 3, 0);
    let sallina_entry = 15_513;
    let blood_elf_mask = 1u64 << (10 - 1);
    let hunter_mask = 1u32 << (3 - 1);

    let mut hunter_training = quest_template(9_393);
    hunter_training.allowable_races = blood_elf_mask;
    hunter_training.allowable_classes = hunter_mask;
    hunter_training.min_level = 1;
    hunter_training.log_title = "Hunter Training".into();

    let mut followup = quest_template(10_070);
    followup.allowable_races = blood_elf_mask;
    followup.allowable_classes = hunter_mask;
    followup.min_level = 2;
    followup.prev_quest_id = 9_393;
    followup.exclusive_group = 10_068;
    followup.log_title = "Well Watcher Solanian".into();

    let mut store = QuestStore::from_quests_like_cpp([hunter_training, followup]);
    store
        .ender_quests
        .entry(sallina_entry)
        .or_default()
        .push(9_393);
    store
        .starter_quests
        .entry(sallina_entry)
        .or_default()
        .push(10_070);
    session.set_quest_store(Arc::new(store));
    mark_quest_rewarded_fixture_like_cpp(&mut session, 9_393);

    let quest_text = gossip_quest_text_for_test(&session, sallina_entry);

    assert_eq!(
        quest_text
            .iter()
            .map(|text| (text.quest_id, text.quest_title.as_str(), text.quest_type))
            .collect::<Vec<_>>(),
        vec![(10_070, "Well Watcher Solanian", 2)]
    );
}
