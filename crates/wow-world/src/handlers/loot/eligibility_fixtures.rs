//! Opaque remote-player facts for the actual loot condition adapter.
use super::*;

pub struct LootConditionPlayer(RepresentedLootPlayerContext);

impl LootConditionPlayer {
    pub fn with_known_spells(mut self, spells: Vec<i32>) -> Self { self.0.known_spells = spells; self }
    pub fn with_rewarded_quests(mut self, quests: HashSet<u32>) -> Self { self.0.rewarded_quests = quests; self }
    pub fn set_item_count(&mut self, item: u32, count: u32) { self.0.inventory_item_counts.insert(item, count); }
    pub fn set_objective_counts(&mut self, quest: u32, counts: Vec<i32>) { self.0.active_quest_objective_counts.insert(quest, counts); }
    pub fn quest_status(&self, quest: u32) -> u8 { self.0.quest_status(quest) }
    pub(super) fn quest_item_allowed(&self, session: &WorldSession, item: u32, needs_quest: bool, flags_cu: u32, quest_log_item_id: i32) -> bool {
        session.item_loot_quest_status_allows_for_player_like_cpp(item, needs_quest, ItemTemplateAddonLootMetadataLikeCpp { flags_cu, quest_log_item_id }, &self.0)
    }
    pub fn remote(race: u8, class: u8, gender: u8, level: u8) -> Self {
        Self(RepresentedLootPlayerContext {
            race, class, gender, level, known_spells: Vec::new(),
            active_quest_statuses: HashMap::new(),
            active_quest_objective_counts: HashMap::new(),
            rewarded_quests: HashSet::new(), inventory_item_counts: HashMap::new(),
            is_current: false,
        })
    }

    pub fn with_inventory_and_progress(mut self, statuses: HashMap<u32, u8>, objectives: HashMap<u32, Vec<i32>>, items: HashMap<u32, u32>) -> Self {
        self.0.active_quest_statuses = statuses;
        self.0.active_quest_objective_counts = objectives;
        self.0.inventory_item_counts = items;
        self
    }
}

pub fn evaluate_remote_loot_condition_for_test(session: &WorldSession, condition: &LootConditionRowLikeCpp, player: &LootConditionPlayer) -> Option<bool> {
    session.evaluate_creature_loot_condition_for_player_like_cpp_representable(condition, &player.0)
}
