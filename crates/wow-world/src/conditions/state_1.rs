//! Condition snapshot borrows state definitions, part 1 of 3.
//!
//! Separated from the conditions.rs root under #656. Behaviour is preserved.

use super::*;

pub const QUEST_STATUS_NONE_LIKE_CPP: u8 = 0;

pub const QUEST_STATUS_COMPLETE_LIKE_CPP: u8 = 1;

pub const QUEST_STATUS_INCOMPLETE_LIKE_CPP: u8 = 3;

pub const QUEST_STATUS_FAILED_LIKE_CPP: u8 = 5;

pub const QUEST_STATUS_REWARDED_LIKE_CPP: u8 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellClickRequirementContextLikeCpp {
    pub clicker_is_player: bool,
    pub clicker_is_friendly_to_summoner: bool,
    pub clicker_is_in_raid_with_summoner: bool,
    pub clicker_is_in_party_with_summoner: bool,
}

pub(super) static CONDITION_MGR_STORE_LIKE_CPP: OnceLock<
    RwLock<Option<Arc<ConditionEntriesByTypeStore>>>,
> = OnceLock::new();

pub(super) fn condition_mgr_store_slot_like_cpp()
-> &'static RwLock<Option<Arc<ConditionEntriesByTypeStore>>> {
    CONDITION_MGR_STORE_LIKE_CPP.get_or_init(|| RwLock::new(None))
}

/// Install the process-wide C++ `sConditionMgr` condition store.
///
/// This keeps the access pattern close to C++ while storing the actual data in an `Arc`, so a
/// future reload can atomically replace the active store without changing call sites.
pub fn set_condition_mgr_store_like_cpp(store: Arc<ConditionEntriesByTypeStore>) {
    *condition_mgr_store_slot_like_cpp().write() = Some(store);
}

/// Return the active C++ `sConditionMgr` store, if startup loaded it.
pub fn condition_mgr_store_like_cpp() -> Option<Arc<ConditionEntriesByTypeStore>> {
    condition_mgr_store_slot_like_cpp().read().as_ref().cloned()
}

/// Clear the process-wide condition store. Used by tests and future reload wiring.
pub fn clear_condition_mgr_store_like_cpp() {
    *condition_mgr_store_slot_like_cpp().write() = None;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConditionMapRef {
    pub map_id: u32,
    pub instance_id: u32,
}

impl ConditionMapRef {
    pub const fn new(map_id: u32, instance_id: u32) -> Self {
        Self {
            map_id,
            instance_id,
        }
    }
}

#[derive(Debug)]
pub struct ConditionSourceInfo<'a> {
    pub condition_targets: [Option<&'a WorldObject>; MAX_CONDITION_TARGETS],
    pub unit_targets: [Option<ConditionUnitSnapshot>; MAX_CONDITION_TARGETS],
    pub unit_aura_targets: [Option<&'a [ConditionAuraEffectSnapshot]>; MAX_CONDITION_TARGETS],
    pub unit_relation_targets: [Option<&'a [ConditionUnitRelationSnapshot]>; MAX_CONDITION_TARGETS],
    pub nearby_creature_targets:
        [Option<&'a [ConditionNearbyCreatureSnapshot]>; MAX_CONDITION_TARGETS],
    pub nearby_gameobject_targets:
        [Option<&'a [ConditionNearbyGameObjectSnapshot]>; MAX_CONDITION_TARGETS],
    pub player_targets: [Option<ConditionPlayerSnapshot>; MAX_CONDITION_TARGETS],
    pub player_quest_targets: [Option<ConditionPlayerQuestSnapshot<'a>>; MAX_CONDITION_TARGETS],
    pub player_progression_targets:
        [Option<ConditionPlayerProgressionSnapshot<'a>>; MAX_CONDITION_TARGETS],
    pub player_condition_contexts:
        [Option<PlayerConditionContextLikeCpp<'a>>; MAX_CONDITION_TARGETS],
    pub player_condition_store: Option<&'a PlayerConditionStore>,
    pub spawn_id_targets: [Option<u64>; MAX_CONDITION_TARGETS],
    pub private_object_targets: [bool; MAX_CONDITION_TARGETS],
    pub string_id_targets: [Option<&'a [&'a str]>; MAX_CONDITION_TARGETS],
    pub realm_achievement_ids: &'a [u32],
    pub map_state: Option<ConditionMapStateSnapshot<'a>>,
    pub condition_map: Option<ConditionMapRef>,
    pub last_failed_condition: Option<&'a Condition>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionUnitSnapshot {
    pub level: u32,
    pub health: u64,
    pub max_health: u64,
    pub class_mask: u32,
    pub race: u8,
    pub creature_type: Option<u32>,
    pub is_alive: bool,
    pub is_charmed: bool,
    pub in_water: bool,
    pub unit_state: u32,
    pub stand_state: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionAuraEffectSnapshot {
    pub spell_id: u32,
    pub effect_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionUnitRelationSnapshot {
    pub to_target_index: usize,
    pub in_party: bool,
    pub in_raid_or_party: bool,
    pub owned_by: bool,
    pub passenger_of: bool,
    pub created_by: bool,
    pub reaction: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionNearbyCreatureSnapshot {
    pub entry: u32,
    pub distance: f32,
    pub is_alive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionNearbyGameObjectSnapshot {
    pub entry: u32,
    pub distance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionPlayerSnapshot {
    pub team: u32,
    pub native_gender: u32,
    pub drunken_state: u32,
    pub can_be_game_master: bool,
    pub is_game_master: bool,
    pub pet_type: Option<u32>,
    pub is_in_flight: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionQuestStatusSnapshot {
    pub quest_id: u32,
    pub status: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionQuestObjectiveProgressSnapshot {
    pub quest_id: u32,
    pub objective_id: u32,
    pub counter: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionPlayerQuestSnapshot<'a> {
    pub statuses: &'a [ConditionQuestStatusSnapshot],
    pub objective_progress: &'a [ConditionQuestObjectiveProgressSnapshot],
    pub rewarded_quest_ids: &'a [u32],
    pub daily_quest_ids: &'a [u32],
}

impl ConditionPlayerQuestSnapshot<'_> {
    pub(super) fn quest_status_like_cpp(self, quest_id: u32) -> u8 {
        self.statuses
            .iter()
            .find(|status| status.quest_id == quest_id)
            .map(|status| status.status)
            .or_else(|| {
                self.is_quest_rewarded_like_cpp(quest_id)
                    .then_some(QUEST_STATUS_REWARDED_LIKE_CPP)
            })
            .unwrap_or(QUEST_STATUS_NONE_LIKE_CPP)
    }

    pub(super) fn is_quest_rewarded_like_cpp(self, quest_id: u32) -> bool {
        self.rewarded_quest_ids.contains(&quest_id)
    }

    pub(super) fn is_daily_quest_done_like_cpp(self, quest_id: u32) -> bool {
        self.daily_quest_ids.contains(&quest_id)
    }

    pub(super) fn quest_is_in_log_like_cpp(self, quest_id: u32) -> bool {
        !matches!(
            self.quest_status_like_cpp(quest_id),
            QUEST_STATUS_NONE_LIKE_CPP | QUEST_STATUS_REWARDED_LIKE_CPP
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionItemCountSnapshot {
    pub item_id: u32,
    pub count: u32,
    pub bank_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionSkillSnapshot {
    pub skill_id: u32,
    pub base_value: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionBattlePetCountSnapshot {
    pub species_id: u32,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionReputationSnapshot {
    pub faction_id: u32,
    pub rank: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionPlayerProgressionSnapshot<'a> {
    pub items: &'a [ConditionItemCountSnapshot],
    pub equipped_item_or_gem_ids: &'a [u32],
    pub skills: &'a [ConditionSkillSnapshot],
    pub spell_ids: &'a [u32],
    pub achievement_ids: &'a [u32],
    pub reputations: &'a [ConditionReputationSnapshot],
    pub title_ids: &'a [u32],
    pub battle_pet_counts: &'a [ConditionBattlePetCountSnapshot],
    pub active_scene_ids: &'a [u32],
}

impl ConditionPlayerProgressionSnapshot<'_> {
    pub(super) fn has_item_count_like_cpp(
        self,
        item_id: u32,
        required_count: u32,
        check_bank: bool,
    ) -> bool {
        self.items
            .iter()
            .find(|item| item.item_id == item_id)
            .is_some_and(|item| {
                let count = if check_bank {
                    item.count.saturating_add(item.bank_count)
                } else {
                    item.count
                };
                count >= required_count
            })
    }

    pub(super) fn has_item_or_gem_equipped_like_cpp(self, item_id: u32) -> bool {
        self.equipped_item_or_gem_ids.contains(&item_id)
    }

    pub(super) fn has_skill_base_value_like_cpp(self, skill_id: u32, required_value: u32) -> bool {
        self.skills
            .iter()
            .find(|skill| skill.skill_id == skill_id)
            .is_some_and(|skill| skill.base_value >= required_value)
    }

    pub(super) fn has_spell_like_cpp(self, spell_id: u32) -> bool {
        self.spell_ids.contains(&spell_id)
    }

    pub(super) fn has_achievement_like_cpp(self, achievement_id: u32) -> bool {
        self.achievement_ids.contains(&achievement_id)
    }

    pub(super) fn has_reputation_rank_like_cpp(self, faction_id: u32, rank_mask: u32) -> bool {
        self.reputations
            .iter()
            .find(|reputation| reputation.faction_id == faction_id)
            .is_some_and(|reputation| ((1_u32 << reputation.rank) & rank_mask) != 0)
    }

    pub(super) fn has_title_like_cpp(self, title_id: u32) -> bool {
        self.title_ids.contains(&title_id)
    }

    pub(super) fn battle_pet_count_like_cpp(self, species_id: u32) -> u32 {
        self.battle_pet_counts
            .iter()
            .find(|pet| pet.species_id == species_id)
            .map(|pet| pet.count)
            .unwrap_or(0)
    }

    pub(super) fn has_active_scene_like_cpp(self, scene_id: u32) -> bool {
        self.active_scene_ids.contains(&scene_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionMapDataSnapshot {
    pub id: u32,
    pub value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionWorldStateSnapshot {
    pub id: u32,
    pub value: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionMapStateSnapshot<'a> {
    pub active_event_ids: &'a [u32],
    pub world_states: &'a [ConditionWorldStateSnapshot],
    pub difficulty_id: u32,
    pub instance_data: &'a [ConditionMapDataSnapshot],
    pub instance_data64: &'a [ConditionMapDataSnapshot],
    pub boss_states: &'a [ConditionMapDataSnapshot],
    pub scenario_step_id: Option<u32>,
}

impl ConditionMapStateSnapshot<'_> {
    pub(super) fn world_state_value_like_cpp(self, world_state_id: u32) -> i32 {
        self.world_states
            .iter()
            .find(|world_state| world_state.id == world_state_id)
            .map(|world_state| world_state.value)
            .unwrap_or(0)
    }

    pub(super) fn instance_data_value_like_cpp(
        self,
        instance_info: ConditionInstanceInfo,
        id: u32,
    ) -> Option<u64> {
        let values = match instance_info {
            ConditionInstanceInfo::Data => self.instance_data,
            ConditionInstanceInfo::Data64 => self.instance_data64,
            ConditionInstanceInfo::BossState => self.boss_states,
            ConditionInstanceInfo::GuidData => return None,
        };
        values
            .iter()
            .find(|data| data.id == id)
            .map(|data| data.value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionMeetResult {
    Evaluated(bool),
    Unsupported,
}

impl ConditionMeetResult {
    pub const fn value(self) -> Option<bool> {
        match self {
            Self::Evaluated(value) => Some(value),
            Self::Unsupported => None,
        }
    }
}

impl<'a> ConditionSourceInfo<'a> {
    /// C++ `ConditionSourceInfo(WorldObject const*, WorldObject const*, WorldObject const*)`.
    pub fn from_targets(
        target0: Option<&'a WorldObject>,
        target1: Option<&'a WorldObject>,
        target2: Option<&'a WorldObject>,
    ) -> Self {
        let condition_targets = [target0, target1, target2];
        let condition_map = condition_targets
            .iter()
            .flatten()
            .next()
            .map(|target| ConditionMapRef::new(target.map_id(), target.instance_id()));

        Self {
            condition_targets,
            unit_targets: [None; MAX_CONDITION_TARGETS],
            unit_aura_targets: [None; MAX_CONDITION_TARGETS],
            unit_relation_targets: [None; MAX_CONDITION_TARGETS],
            nearby_creature_targets: [None; MAX_CONDITION_TARGETS],
            nearby_gameobject_targets: [None; MAX_CONDITION_TARGETS],
            player_targets: [None; MAX_CONDITION_TARGETS],
            player_quest_targets: [None; MAX_CONDITION_TARGETS],
            player_progression_targets: [None; MAX_CONDITION_TARGETS],
            player_condition_contexts: [None; MAX_CONDITION_TARGETS],
            player_condition_store: None,
            spawn_id_targets: [None; MAX_CONDITION_TARGETS],
            private_object_targets: [false; MAX_CONDITION_TARGETS],
            string_id_targets: [None; MAX_CONDITION_TARGETS],
            realm_achievement_ids: &[],
            map_state: None,
            condition_map,
            last_failed_condition: None,
        }
    }

    /// C++ `ConditionSourceInfo(Map const*)`.
    pub const fn from_map(condition_map: ConditionMapRef) -> Self {
        Self {
            condition_targets: [None; MAX_CONDITION_TARGETS],
            unit_targets: [None; MAX_CONDITION_TARGETS],
            unit_aura_targets: [None; MAX_CONDITION_TARGETS],
            unit_relation_targets: [None; MAX_CONDITION_TARGETS],
            nearby_creature_targets: [None; MAX_CONDITION_TARGETS],
            nearby_gameobject_targets: [None; MAX_CONDITION_TARGETS],
            player_targets: [None; MAX_CONDITION_TARGETS],
            player_quest_targets: [None; MAX_CONDITION_TARGETS],
            player_progression_targets: [None; MAX_CONDITION_TARGETS],
            player_condition_contexts: [None; MAX_CONDITION_TARGETS],
            player_condition_store: None,
            spawn_id_targets: [None; MAX_CONDITION_TARGETS],
            private_object_targets: [false; MAX_CONDITION_TARGETS],
            string_id_targets: [None; MAX_CONDITION_TARGETS],
            realm_achievement_ids: &[],
            map_state: None,
            condition_map: Some(condition_map),
            last_failed_condition: None,
        }
    }

    pub fn set_unit_target_snapshot(
        &mut self,
        target_index: usize,
        snapshot: ConditionUnitSnapshot,
    ) {
        if target_index < MAX_CONDITION_TARGETS {
            self.unit_targets[target_index] = Some(snapshot);
        }
    }

    pub fn set_unit_aura_target_snapshot(
        &mut self,
        target_index: usize,
        aura_effects: &'a [ConditionAuraEffectSnapshot],
    ) {
        if target_index < MAX_CONDITION_TARGETS {
            self.unit_aura_targets[target_index] = Some(aura_effects);
        }
    }

    pub fn set_unit_relation_target_snapshot(
        &mut self,
        target_index: usize,
        relations: &'a [ConditionUnitRelationSnapshot],
    ) {
        if target_index < MAX_CONDITION_TARGETS {
            self.unit_relation_targets[target_index] = Some(relations);
        }
    }

    pub fn set_nearby_creature_target_snapshot(
        &mut self,
        target_index: usize,
        creatures: &'a [ConditionNearbyCreatureSnapshot],
    ) {
        if target_index < MAX_CONDITION_TARGETS {
            self.nearby_creature_targets[target_index] = Some(creatures);
        }
    }

    pub fn set_nearby_gameobject_target_snapshot(
        &mut self,
        target_index: usize,
        gameobjects: &'a [ConditionNearbyGameObjectSnapshot],
    ) {
        if target_index < MAX_CONDITION_TARGETS {
            self.nearby_gameobject_targets[target_index] = Some(gameobjects);
        }
    }

    pub fn set_player_target_snapshot(
        &mut self,
        target_index: usize,
        snapshot: ConditionPlayerSnapshot,
    ) {
        if target_index < MAX_CONDITION_TARGETS {
            self.player_targets[target_index] = Some(snapshot);
        }
    }

    pub fn set_player_quest_target_snapshot(
        &mut self,
        target_index: usize,
        snapshot: ConditionPlayerQuestSnapshot<'a>,
    ) {
        if target_index < MAX_CONDITION_TARGETS {
            self.player_quest_targets[target_index] = Some(snapshot);
        }
    }

    pub fn set_player_progression_target_snapshot(
        &mut self,
        target_index: usize,
        snapshot: ConditionPlayerProgressionSnapshot<'a>,
    ) {
        if target_index < MAX_CONDITION_TARGETS {
            self.player_progression_targets[target_index] = Some(snapshot);
        }
    }

    pub fn set_player_condition_context(
        &mut self,
        target_index: usize,
        context: PlayerConditionContextLikeCpp<'a>,
    ) {
        if target_index < MAX_CONDITION_TARGETS {
            self.player_condition_contexts[target_index] = Some(context);
        }
    }

    pub fn set_player_condition_store(&mut self, store: &'a PlayerConditionStore) {
        self.player_condition_store = Some(store);
    }

    pub fn set_realm_achievement_ids(&mut self, achievement_ids: &'a [u32]) {
        self.realm_achievement_ids = achievement_ids;
    }

    pub fn set_map_state_snapshot(&mut self, map_state: ConditionMapStateSnapshot<'a>) {
        self.map_state = Some(map_state);
    }

    pub fn set_spawn_id_target_snapshot(&mut self, target_index: usize, spawn_id: u64) {
        if target_index < MAX_CONDITION_TARGETS {
            self.spawn_id_targets[target_index] = Some(spawn_id);
        }
    }

    pub fn set_private_object_target_snapshot(&mut self, target_index: usize, is_private: bool) {
        if target_index < MAX_CONDITION_TARGETS {
            self.private_object_targets[target_index] = is_private;
        }
    }

    pub fn set_string_id_target_snapshot(
        &mut self,
        target_index: usize,
        string_ids: &'a [&'a str],
    ) {
        if target_index < MAX_CONDITION_TARGETS {
            self.string_id_targets[target_index] = Some(string_ids);
        }
    }

    pub fn mark_failed_like_cpp(&mut self, condition: &'a Condition) {
        self.last_failed_condition = Some(condition);
    }
}

pub(super) fn compare_values_u64_like_cpp(comparison_type: u32, left: u64, right: u64) -> bool {
    match ComparisonType::from_u32(comparison_type) {
        Some(ComparisonType::Eq) => left == right,
        Some(ComparisonType::High) => left > right,
        Some(ComparisonType::Low) => left < right,
        Some(ComparisonType::HighEq) => left >= right,
        Some(ComparisonType::LowEq) => left <= right,
        _ => false,
    }
}

pub(super) fn compare_values_f32_like_cpp(comparison_type: u32, left: f32, right: f32) -> bool {
    match ComparisonType::from_u32(comparison_type) {
        Some(ComparisonType::Eq) => left == right,
        Some(ComparisonType::High) => left > right,
        Some(ComparisonType::Low) => left < right,
        Some(ComparisonType::HighEq) => left >= right,
        Some(ComparisonType::LowEq) => left <= right,
        _ => false,
    }
}

pub(super) fn is_player_object_like_cpp(object: &WorldObject) -> bool {
    matches!(
        object.object().type_id(),
        TypeId::Player | TypeId::ActivePlayer
    )
}

pub(super) fn is_unit_object_like_cpp(object: &WorldObject) -> bool {
    matches!(
        object.object().type_id(),
        TypeId::Unit | TypeId::Player | TypeId::ActivePlayer
    ) || object
        .object()
        .type_mask()
        .intersects(TypeMask::UNIT | TypeMask::PLAYER | TypeMask::ACTIVE_PLAYER)
}

pub(super) fn unit_stand_state_is_sit_like_cpp(stand_state: u32) -> bool {
    stand_state == UnitStandStateType::Sit as u32
        || stand_state == UnitStandStateType::SitChair as u32
        || stand_state == UnitStandStateType::SitLowChair as u32
        || stand_state == UnitStandStateType::SitMediumChair as u32
        || stand_state == UnitStandStateType::SitHighChair as u32
}

pub(super) fn unit_stand_state_is_stand_like_cpp(stand_state: u32) -> bool {
    !unit_stand_state_is_sit_like_cpp(stand_state)
        && stand_state != UnitStandStateType::Sleep as u32
        && stand_state != UnitStandStateType::Kneel as u32
}
