// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure quest dialog presentation, independent of Session and catalog ownership.
//! C++ Player.cpp:15706-15784 (GetQuestDialogStatus), QuestDef.cpp:438-445
//! (IsImportant). Eligibility, level checks and packet publication stay with callers.

use wow_data::progression_rewards::QuestInfoEntry;
use wow_data::quest::{
    QUEST_FLAGS_DAILY_LIKE_CPP, QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP,
    QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP,
};
use wow_packet::packets::quest::quest_giver_status;

#[cfg(test)]
mod tests {
    use super::*;
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
}

pub(super) struct QuestDialogClassificationLikeCpp {
    important: bool,
    covenant_calling: bool,
    legendary: bool,
    daily: bool,
    hide_reward_poi: bool,
}

impl QuestDialogClassificationLikeCpp {
    pub(super) fn new(flags: u32, flags_ex: u32, info: Option<&QuestInfoEntry>) -> Self {
        Self {
            important: info.is_some_and(|info| info.modifiers & 0x400 != 0),
            covenant_calling: info.is_some_and(|info| info.quest_type == 15),
            legendary: flags_ex & QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP != 0,
            daily: flags & QUEST_FLAGS_DAILY_LIKE_CPP != 0,
            hide_reward_poi: flags & QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP != 0,
        }
    }

    pub(super) fn is_important(&self) -> bool {
        self.important
    }

    pub(super) fn reward_complete(&self) -> u64 {
        if self.important {
            if self.hide_reward_poi {
                quest_giver_status::IMPORTANT_QUEST_REWARD_COMPLETE_NO_POI
            } else {
                quest_giver_status::IMPORTANT_QUEST_REWARD_COMPLETE_POI
            }
        } else if self.covenant_calling {
            if self.hide_reward_poi {
                quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_NO_POI
            } else {
                quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_POI
            }
        } else if self.legendary {
            if self.hide_reward_poi {
                quest_giver_status::LEGENDARY_REWARD_COMPLETE_NO_POI
            } else {
                quest_giver_status::LEGENDARY_REWARD_COMPLETE_POI
            }
        } else if self.hide_reward_poi {
            quest_giver_status::REWARD_COMPLETE_NO_POI
        } else {
            quest_giver_status::REWARD_COMPLETE_POI
        }
    }

    pub(super) fn reward(&self) -> u64 {
        if self.important {
            quest_giver_status::IMPORTANT_REWARD
        } else if self.covenant_calling {
            quest_giver_status::COVENANT_CALLING_REWARD
        } else if self.legendary {
            quest_giver_status::LEGENDARY_REWARD
        } else {
            quest_giver_status::REWARD
        }
    }

    pub(super) fn available(&self, trivial: bool) -> u64 {
        if self.important {
            if trivial {
                quest_giver_status::TRIVIAL_IMPORTANT_QUEST
            } else {
                quest_giver_status::IMPORTANT_QUEST
            }
        } else if self.covenant_calling {
            quest_giver_status::COVENANT_CALLING_QUEST
        } else if self.legendary {
            if trivial {
                quest_giver_status::TRIVIAL_LEGENDARY_QUEST
            } else {
                quest_giver_status::LEGENDARY_QUEST
            }
        } else if self.daily {
            if trivial {
                quest_giver_status::TRIVIAL_DAILY_QUEST
            } else {
                quest_giver_status::DAILY_QUEST
            }
        } else if trivial {
            quest_giver_status::TRIVIAL
        } else {
            quest_giver_status::QUEST
        }
    }

    pub(super) fn future(&self) -> u64 {
        if self.important {
            quest_giver_status::FUTURE_IMPORTANT_QUEST
        } else if self.legendary {
            quest_giver_status::FUTURE_LEGENDARY_QUEST
        } else {
            quest_giver_status::FUTURE
        }
    }
}
