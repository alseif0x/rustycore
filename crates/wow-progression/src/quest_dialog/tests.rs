use super::*;
use super::QuestDialogClassification as QuestDialogClassificationLikeCpp;
use quest_giver_status::*;

#[test]
fn dialog_status_metadata_and_flag_precedence_matches_cpp() {
    // Explicit C++ result tables: normal, important, covenant, legendary, daily.
    let complete = [
        [REWARD_COMPLETE_POI, REWARD_COMPLETE_NO_POI],
        [
            IMPORTANT_QUEST_REWARD_COMPLETE_POI,
            IMPORTANT_QUEST_REWARD_COMPLETE_NO_POI,
        ],
        [
            COVENANT_CALLING_REWARD_COMPLETE_POI,
            COVENANT_CALLING_REWARD_COMPLETE_NO_POI,
        ],
        [
            LEGENDARY_REWARD_COMPLETE_POI,
            LEGENDARY_REWARD_COMPLETE_NO_POI,
        ],
    ];
    let reward = [
        REWARD,
        IMPORTANT_REWARD,
        COVENANT_CALLING_REWARD,
        LEGENDARY_REWARD,
    ];
    let available = [
        [QUEST, TRIVIAL],
        [IMPORTANT_QUEST, TRIVIAL_IMPORTANT_QUEST],
        [COVENANT_CALLING_QUEST, COVENANT_CALLING_QUEST],
        [LEGENDARY_QUEST, TRIVIAL_LEGENDARY_QUEST],
        [DAILY_QUEST, TRIVIAL_DAILY_QUEST],
    ];
    let mut checked = 0;
    // None is distinct from a present, unclassified entry.
    for metadata in [
        None,
        Some((0, 0)),
        Some((0x400, 0)),
        Some((0, 15)),
        Some((0x400, 15)),
    ] {
        let info = metadata.map(|(modifiers, quest_type)| QuestInfoEntry {
            id: 1,
            info_name: String::new(),
            quest_type,
            modifiers,
            profession: 0,
        });
        for legendary in [false, true] {
            for daily in [false, true] {
                for hidden in [false, true] {
                    for trivial in [false, true] {
                        let flags = if daily { QUEST_FLAGS_DAILY_LIKE_CPP } else { 0 }
                            | if hidden {
                                QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP
                            } else {
                                0
                            };
                        let flags_ex = if legendary {
                            QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP
                        } else {
                            0
                        };
                        let classification = QuestDialogClassificationLikeCpp::new(
                            flags,
                            flags_ex,
                            info.as_ref(),
                        );
                        let important = matches!(metadata, Some((0x400, _)));
                        let kind = match metadata {
                            Some((0x400, _)) => 1,
                            Some((_, 15)) => 2,
                            _ if legendary => 3,
                            _ => 0,
                        };
                        let available_kind = if kind == 0 && daily { 4 } else { kind };
                        // Future has no covenant branch: legendary must still win here.
                        let future = if important {
                            FUTURE_IMPORTANT_QUEST
                        } else if legendary {
                            FUTURE_LEGENDARY_QUEST
                        } else {
                            FUTURE
                        };
                        assert_eq!(classification.is_important(), important);
                        assert_eq!(
                            classification.reward_complete(),
                            complete[kind][usize::from(hidden)]
                        );
                        assert_eq!(classification.reward(), reward[kind]);
                        assert_eq!(
                            classification.available(trivial),
                            available[available_kind][usize::from(trivial)]
                        );
                        assert_eq!(classification.future(), future);
                        checked += 1;
                    }
                }
            }
        }
    }
    assert_eq!(checked, 80);
}

#[test]
fn dialog_status_ignores_unrelated_metadata_and_flag_bits() {
    let info = QuestInfoEntry {
        id: 9,
        info_name: "presentation only".into(),
        quest_type: 14,
        modifiers: 0x401,
        profession: 123,
    };
    let important =
        QuestDialogClassificationLikeCpp::new(0x8000_0000, 0x8000_0000, Some(&info));
    assert!(important.is_important());
    assert_eq!(important.reward(), IMPORTANT_REWARD);
    let info = QuestInfoEntry {
        modifiers: 0x200,
        ..info
    };
    let ordinary = QuestDialogClassificationLikeCpp::new(0x8000_0000, 0x8000_0000, Some(&info));
    assert!(!ordinary.is_important());
    assert_eq!(ordinary.reward_complete(), REWARD_COMPLETE_POI);
    assert_eq!(ordinary.available(false), QUEST);
    assert_eq!(ordinary.future(), FUTURE);
}
