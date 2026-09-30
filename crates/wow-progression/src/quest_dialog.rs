//! Quest dialog classification from Player.cpp:15706-15784 and QuestDef.cpp:438-445.
//! Eligibility, catalog access and publication remain with callers.

use wow_data_model::quest::QuestInfoEntry;
use wow_constants::quest::{
    QUEST_FLAGS_DAILY as QUEST_FLAGS_DAILY_LIKE_CPP,
    QUEST_FLAGS_EX_LEGENDARY as QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP,
    QUEST_FLAGS_HIDE_REWARD_POI as QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP,
    quest_giver_status,
};

#[cfg(test)]
mod tests;

pub struct QuestDialogClassification {
    important: bool,
    covenant_calling: bool,
    legendary: bool,
    daily: bool,
    hide_reward_poi: bool,
}

impl QuestDialogClassification {
    pub fn new(flags: u32, flags_ex: u32, info: Option<&QuestInfoEntry>) -> Self {
        Self {
            important: info.is_some_and(|info| info.modifiers & 0x400 != 0),
            covenant_calling: info.is_some_and(|info| info.quest_type == 15),
            legendary: flags_ex & QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP != 0,
            daily: flags & QUEST_FLAGS_DAILY_LIKE_CPP != 0,
            hide_reward_poi: flags & QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP != 0,
        }
    }

    pub fn is_important(&self) -> bool {
        self.important
    }

    pub fn reward_complete(&self) -> u64 {
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

    pub fn reward(&self) -> u64 {
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

    pub fn available(&self, trivial: bool) -> u64 {
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

    pub fn future(&self) -> u64 {
        if self.important {
            quest_giver_status::FUTURE_IMPORTANT_QUEST
        } else if self.legendary {
            quest_giver_status::FUTURE_LEGENDARY_QUEST
        } else {
            quest_giver_status::FUTURE
        }
    }
}
