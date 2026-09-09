//! Condition entry model and load reporting state definitions, part 2 of 3.
//!
//! Separated from the conditions.rs root under #638. Behaviour is preserved.

use super::*;

pub fn validate_condition_type_static_like_cpp(
    condition: &mut Condition,
) -> Result<ConditionTypeValidationReportLikeCpp, ConditionTypeValidationErrorLikeCpp> {
    use ConditionTypeValidationErrorLikeCpp as Error;

    match condition.condition_type {
        ConditionType::Aura => {
            if condition.condition_value2 >= MAX_SPELL_EFFECTS_LIKE_CPP {
                return Err(Error::InvalidSpellEffectIndex(condition.condition_value2));
            }
        }
        ConditionType::Item => {
            if condition.condition_value2 == 0 {
                return Err(Error::ZeroItemCount);
            }
        }
        ConditionType::Team => {
            if condition.condition_value1 != Team::Alliance as u32
                && condition.condition_value1 != Team::Horde as u32
            {
                return Err(Error::InvalidTeam(condition.condition_value1));
            }
        }
        ConditionType::Skill => {
            if condition.condition_value2 < 1 {
                return Err(Error::InvalidSkillValue(condition.condition_value2));
            }
        }
        ConditionType::QuestState => {
            if condition.condition_value2 >= (1 << MAX_QUEST_STATUS_LIKE_CPP) {
                return Err(Error::InvalidQuestStateMask(condition.condition_value2));
            }
        }
        ConditionType::Class => {
            let invalid_mask = condition.condition_value1 & !CLASSMASK_ALL_PLAYABLE_LIKE_CPP;
            if invalid_mask != 0 {
                return Err(Error::InvalidClassMask(invalid_mask));
            }
        }
        ConditionType::Race => {
            let invalid_mask =
                u64::from(condition.condition_value1) & !RACEMASK_ALL_PLAYABLE_LIKE_CPP;
            if invalid_mask != 0 {
                return Err(Error::InvalidRaceMask(invalid_mask));
            }
        }
        ConditionType::Gender => {
            if condition.condition_value1 > Gender::Female as u32 {
                return Err(Error::InvalidGender(condition.condition_value1));
            }
        }
        ConditionType::Level => {
            if condition.condition_value2 >= ComparisonType::Max as u32 {
                return Err(Error::InvalidComparisonType {
                    field: 2,
                    value: condition.condition_value2,
                });
            }
        }
        ConditionType::DrunkenState => {
            if condition.condition_value1 > DRUNKEN_SMASHED_LIKE_CPP {
                return Err(Error::InvalidDrunkenState(condition.condition_value1));
            }
        }
        ConditionType::ObjectEntryGuidLegacy => {
            condition.condition_type = ConditionType::ObjectEntryGuid;
            condition.condition_value1 =
                convert_legacy_type_id_like_cpp(condition.condition_value1);
            validate_object_entry_guid_type_like_cpp(condition)?;
        }
        ConditionType::ObjectEntryGuid => validate_object_entry_guid_type_like_cpp(condition)?,
        ConditionType::TypeMaskLegacy => {
            condition.condition_type = ConditionType::TypeMask;
            condition.condition_value1 =
                convert_legacy_type_mask_like_cpp(condition.condition_value1);
            validate_type_mask_like_cpp(condition)?;
        }
        ConditionType::TypeMask => validate_type_mask_like_cpp(condition)?,
        ConditionType::RelationTo => {
            validate_target_selector_like_cpp(condition, 1)?;
            if condition.condition_value2 >= RelationType::Max as u32 {
                return Err(Error::InvalidRelationType(condition.condition_value2));
            }
        }
        ConditionType::ReactionTo => {
            validate_target_selector_like_cpp(condition, 1)?;
            if condition.condition_value2 == 0 {
                return Err(Error::InvalidReactionRankMask(condition.condition_value2));
            }
        }
        ConditionType::DistanceTo => {
            validate_target_selector_like_cpp(condition, 1)?;
            if condition.condition_value3 >= ComparisonType::Max as u32 {
                return Err(Error::InvalidComparisonType {
                    field: 3,
                    value: condition.condition_value3,
                });
            }
        }
        ConditionType::HpVal => {
            if condition.condition_value2 >= ComparisonType::Max as u32 {
                return Err(Error::InvalidComparisonType {
                    field: 2,
                    value: condition.condition_value2,
                });
            }
        }
        ConditionType::HpPct => {
            if condition.condition_value1 > 100 {
                return Err(Error::InvalidComparisonType {
                    field: 1,
                    value: condition.condition_value1,
                });
            }
            if condition.condition_value2 >= ComparisonType::Max as u32 {
                return Err(Error::InvalidComparisonType {
                    field: 2,
                    value: condition.condition_value2,
                });
            }
        }
        ConditionType::SpawnMaskDeprecated => return Err(Error::DeprecatedSpawnMask),
        ConditionType::UnitState => {
            if condition.condition_value1 & UNIT_STATE_ALL_STATE_SUPPORTED_LIKE_CPP == 0 {
                return Err(Error::InvalidUnitState(condition.condition_value1));
            }
        }
        ConditionType::CreatureType => {
            if condition.condition_value1 == 0
                || condition.condition_value1 > CREATURE_TYPE_GAS_CLOUD_LIKE_CPP
            {
                return Err(Error::InvalidCreatureType(condition.condition_value1));
            }
        }
        ConditionType::StandState => {
            let valid = match condition.condition_value1 {
                0 => condition.condition_value2 <= UnitStandStateType::Submerged as u32,
                1 => condition.condition_value2 <= 1,
                _ => false,
            };
            if !valid {
                return Err(Error::InvalidStandState {
                    value1: condition.condition_value1,
                    value2: condition.condition_value2,
                });
            }
        }
        ConditionType::PetType => {
            if condition.condition_value1 >= (1 << MAX_PET_TYPE_LIKE_CPP) {
                return Err(Error::InvalidPetTypeMask(condition.condition_value1));
            }
        }
        ConditionType::InstanceInfo => {
            if condition.condition_value3 == ConditionInstanceInfo::GuidData as u32 {
                return Err(Error::UnsupportedInstanceInfoGuidData);
            }
        }
        ConditionType::BattlePetCount => {
            if condition.condition_value2 > DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP {
                return Err(Error::InvalidBattlePetCount(condition.condition_value2));
            }
            if condition.condition_value3 >= ComparisonType::Max as u32 {
                return Err(Error::InvalidComparisonType {
                    field: 3,
                    value: condition.condition_value3,
                });
            }
        }
        ConditionType::AreaId
        | ConditionType::Alive
        | ConditionType::InWater
        | ConditionType::TerrainSwap
        | ConditionType::Charmed
        | ConditionType::Taxi
        | ConditionType::GameMaster
        | ConditionType::PrivateObject
        | ConditionType::None
        | ConditionType::StringId
        | ConditionType::ItemEquipped
        | ConditionType::ZoneId
        | ConditionType::ReputationRank
        | ConditionType::WorldState
        | ConditionType::ActiveEvent
        | ConditionType::QuestRewarded
        | ConditionType::QuestTaken
        | ConditionType::QuestNone
        | ConditionType::Achievement
        | ConditionType::Title
        | ConditionType::MapId
        | ConditionType::Spell
        | ConditionType::PhaseId
        | ConditionType::QuestComplete
        | ConditionType::NearCreature
        | ConditionType::NearGameObject
        | ConditionType::RealmAchievement
        | ConditionType::DailyQuestDone
        | ConditionType::QuestObjectiveProgress
        | ConditionType::DifficultyId
        | ConditionType::ScenarioStep
        | ConditionType::SceneInProgress
        | ConditionType::PlayerCondition
        | ConditionType::Max => {}
    }

    Ok(ConditionTypeValidationReportLikeCpp {
        useless_value_fields: useless_condition_value_fields_like_cpp(condition),
    })
}

pub fn validate_condition_source_static_like_cpp(
    condition: &Condition,
) -> Result<(), ConditionSourceValidationErrorLikeCpp> {
    use ConditionSourceValidationErrorLikeCpp as Error;

    match condition.source_type {
        ConditionSourceType::None | ConditionSourceType::Max => {
            return Err(Error::InvalidSourceType(condition.source_type));
        }
        ConditionSourceType::SpellImplicitTarget => {
            if condition.source_group == 0 {
                return Err(Error::InvalidSpellImplicitTargetEffectMask(
                    condition.source_group,
                ));
            }
        }
        ConditionSourceType::AreaTrigger => {
            if condition.source_entry != 0 && condition.source_entry != 1 {
                return Err(Error::InvalidAreaTriggerSourceEntry(condition.source_entry));
            }
        }
        ConditionSourceType::ObjectIdVisibility => {
            if condition.source_group == 0 || condition.source_group >= TypeId::Max as u32 {
                return Err(Error::InvalidObjectIdVisibilityObjectType(
                    condition.source_group,
                ));
            }

            if condition.source_group != TypeId::Unit as u32
                && condition.source_group != TypeId::GameObject as u32
            {
                return Err(Error::UncheckedObjectIdVisibilityObjectType(
                    condition.source_group,
                ));
            }
        }
        ConditionSourceType::ReferenceCondition => {
            return Err(Error::InvalidSourceType(condition.source_type));
        }
        ConditionSourceType::CreatureLootTemplate
        | ConditionSourceType::DisenchantLootTemplate
        | ConditionSourceType::FishingLootTemplate
        | ConditionSourceType::GameObjectLootTemplate
        | ConditionSourceType::ItemLootTemplate
        | ConditionSourceType::MailLootTemplate
        | ConditionSourceType::MillingLootTemplate
        | ConditionSourceType::PickpocketingLootTemplate
        | ConditionSourceType::ProspectingLootTemplate
        | ConditionSourceType::ReferenceLootTemplate
        | ConditionSourceType::SkinningLootTemplate
        | ConditionSourceType::SpellLootTemplate
        | ConditionSourceType::GossipMenu
        | ConditionSourceType::GossipMenuOption
        | ConditionSourceType::CreatureTemplateVehicle
        | ConditionSourceType::Spell
        | ConditionSourceType::SpellClickEvent
        | ConditionSourceType::QuestAvailable
        | ConditionSourceType::VehicleSpell
        | ConditionSourceType::SmartEvent
        | ConditionSourceType::NpcVendor
        | ConditionSourceType::SpellProc
        | ConditionSourceType::TerrainSwap
        | ConditionSourceType::Phase
        | ConditionSourceType::Graveyard
        | ConditionSourceType::ConversationLine
        | ConditionSourceType::AreaTriggerClientTriggered
        | ConditionSourceType::TrainerSpell
        | ConditionSourceType::SpawnGroup => {}
    }

    Ok(())
}

pub fn validate_condition_type_external_like_cpp(
    condition: &Condition,
    stores: ConditionExternalValidationStoresLikeCpp<'_>,
) -> Result<(), ConditionTypeValidationErrorLikeCpp> {
    use ConditionTypeValidationErrorLikeCpp as Error;

    if condition.reference_id != 0 {
        return Ok(());
    }

    match condition.condition_type {
        ConditionType::Aura | ConditionType::Spell => {
            if let Some(store) = stores.spell_store
                && store.get(condition.condition_value1 as i32).is_none()
            {
                return Err(Error::NonExistingSpell {
                    condition_type: condition.condition_type,
                    spell_id: condition.condition_value1,
                });
            }
        }
        ConditionType::Item | ConditionType::ItemEquipped => {
            if let Some(store) = stores.item_store
                && store.get(condition.condition_value1).is_none()
            {
                return Err(Error::NonExistingItem {
                    condition_type: condition.condition_type,
                    item_id: condition.condition_value1,
                });
            }
        }
        ConditionType::ZoneId => {
            if let Some(store) = stores.area_table_store {
                let Some(area) = store.get(condition.condition_value1) else {
                    return Err(Error::NonExistingArea {
                        condition_type: condition.condition_type,
                        area_id: condition.condition_value1,
                    });
                };

                if area.parent_area_id != 0 && area.is_subzone_like_cpp() {
                    return Err(Error::ZoneIdUsesSubzone(condition.condition_value1));
                }
            }
        }
        ConditionType::Skill => {
            if let Some(store) = stores.skill_line_store
                && !store.contains_effective_record_like_cpp(condition.condition_value1)
            {
                return Err(Error::NonExistingSkill(condition.condition_value1));
            }
            if let Some(max) = stores.max_skill_value
                && condition.condition_value2 > max
            {
                return Err(Error::SkillValueAboveConfigMax {
                    skill_id: condition.condition_value1,
                    value: condition.condition_value2,
                    max,
                });
            }
        }
        ConditionType::MapId => {
            if let Some(store) = stores.map_store
                && store.get(condition.condition_value1).is_none()
            {
                return Err(Error::NonExistingMap {
                    condition_type: condition.condition_type,
                    map_id: condition.condition_value1,
                });
            }
        }
        ConditionType::PhaseId => {
            if let Some(store) = stores.phase_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingPhase(condition.condition_value1));
            }
        }
        ConditionType::QuestState
        | ConditionType::QuestRewarded
        | ConditionType::QuestTaken
        | ConditionType::QuestNone
        | ConditionType::QuestComplete
        | ConditionType::DailyQuestDone => {
            if let Some(store) = stores.quest_store
                && store.get(condition.condition_value1).is_none()
            {
                return Err(Error::NonExistingQuest {
                    condition_type: condition.condition_type,
                    quest_id: condition.condition_value1,
                });
            }
        }
        ConditionType::QuestObjectiveProgress => {
            if let Some(store) = stores.quest_store {
                let Some(objective) = store.objective_like_cpp(condition.condition_value1) else {
                    return Err(Error::NonExistingQuestObjective(condition.condition_value1));
                };
                let limit = objective.condition_progress_limit_like_cpp();
                if i32::try_from(condition.condition_value3).is_ok_and(|count| count > limit) {
                    return Err(Error::QuestObjectiveCountAboveLimit {
                        objective_id: condition.condition_value1,
                        count: condition.condition_value3,
                        limit,
                    });
                }
            }
        }
        ConditionType::DifficultyId => {
            if let Some(store) = stores.difficulty_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingDifficulty(condition.condition_value1));
            }
        }
        ConditionType::ReputationRank => {
            if let Some(store) = stores.faction_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingFaction(condition.condition_value1));
            }
        }
        ConditionType::Achievement | ConditionType::RealmAchievement => {
            if let Some(store) = stores.achievement_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingAchievement {
                    condition_type: condition.condition_type,
                    achievement_id: condition.condition_value1,
                });
            }
        }
        ConditionType::Title => {
            if let Some(store) = stores.char_titles_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingTitle(condition.condition_value1));
            }
        }
        ConditionType::BattlePetCount => {
            if let Some(store) = stores.battle_pet_species_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingBattlePetSpecies(
                    condition.condition_value1,
                ));
            }
        }
        ConditionType::ScenarioStep => {
            if let Some(store) = stores.scenario_step_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingScenarioStep(condition.condition_value1));
            }
        }
        ConditionType::SceneInProgress => {
            if let Some(store) = stores.scene_script_package_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingSceneScriptPackage(
                    condition.condition_value1,
                ));
            }
        }
        ConditionType::PlayerCondition => {
            if let Some(store) = stores.player_condition_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingPlayerCondition(
                    condition.condition_value1,
                ));
            }
        }
        ConditionType::NearCreature => {
            if let Some(store) = stores.creature_template_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingCreatureTemplate {
                    condition_type: condition.condition_type,
                    entry: condition.condition_value1,
                });
            }
        }
        ConditionType::NearGameObject => {
            if let Some(store) = stores.gameobject_template_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingGameObjectTemplate {
                    condition_type: condition.condition_type,
                    entry: condition.condition_value1,
                });
            }
        }
        ConditionType::ObjectEntryGuid => match condition.condition_value1 {
            value if value == TypeId::Unit as u32 => {
                if condition.condition_value2 != 0
                    && let Some(store) = stores.creature_template_store
                    && !store.contains(condition.condition_value2)
                {
                    return Err(Error::NonExistingCreatureTemplate {
                        condition_type: condition.condition_type,
                        entry: condition.condition_value2,
                    });
                }
                if condition.condition_value3 != 0
                    && let Some(store) = stores.creature_spawn_store
                {
                    let Some(actual_entry) = store.entry_for_guid(condition.condition_value3)
                    else {
                        return Err(Error::NonExistingCreatureGuid(condition.condition_value3));
                    };
                    if condition.condition_value2 != 0 && actual_entry != condition.condition_value2
                    {
                        return Err(Error::CreatureGuidEntryMismatch {
                            guid: condition.condition_value3,
                            expected_entry: condition.condition_value2,
                            actual_entry,
                        });
                    }
                }
            }
            value if value == TypeId::GameObject as u32 => {
                if condition.condition_value2 != 0
                    && let Some(store) = stores.gameobject_template_store
                    && !store.contains(condition.condition_value2)
                {
                    return Err(Error::NonExistingGameObjectTemplate {
                        condition_type: condition.condition_type,
                        entry: condition.condition_value2,
                    });
                }
                if condition.condition_value3 != 0
                    && let Some(store) = stores.gameobject_spawn_store
                {
                    let Some(actual_entry) = store.entry_for_guid(condition.condition_value3)
                    else {
                        return Err(Error::NonExistingGameObjectGuid(condition.condition_value3));
                    };
                    if condition.condition_value2 != 0 && actual_entry != condition.condition_value2
                    {
                        return Err(Error::GameObjectGuidEntryMismatch {
                            guid: condition.condition_value3,
                            expected_entry: condition.condition_value2,
                            actual_entry,
                        });
                    }
                }
            }
            _ => {}
        },
        ConditionType::ActiveEvent => {
            if let Some(store) = stores.active_event_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingActiveEvent(condition.condition_value1));
            }
        }
        ConditionType::WorldState => {
            if let Some(store) = stores.world_state_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingWorldState(condition.condition_value1));
            }
        }
        _ => {}
    }

    Ok(())
}

pub fn validate_condition_source_external_like_cpp(
    condition: &Condition,
    stores: ConditionExternalValidationStoresLikeCpp<'_>,
) -> Result<(), ConditionSourceValidationErrorLikeCpp> {
    let mut condition = condition.clone();
    validate_and_normalize_condition_source_external_like_cpp(&mut condition, stores)
}

pub fn validate_and_normalize_condition_source_external_like_cpp(
    condition: &mut Condition,
    stores: ConditionExternalValidationStoresLikeCpp<'_>,
) -> Result<(), ConditionSourceValidationErrorLikeCpp> {
    use ConditionSourceValidationErrorLikeCpp as Error;

    match condition.source_type {
        source_type if condition_source_is_loot_template_like_cpp(source_type) => {
            if let Some(template_exists) = stores.loot_template_exists
                && !template_exists(source_type, condition.source_group)
            {
                return Err(Error::NonExistingLootTemplate {
                    source_type,
                    source_group: condition.source_group,
                });
            }

            if let Some(source_entry_exists) = stores.loot_source_entry_exists
                && !source_entry_exists(source_type, condition.source_group, condition.source_entry)
            {
                return Err(Error::NonExistingLootSourceEntry {
                    source_type,
                    source_group: condition.source_group,
                    source_entry: condition.source_entry,
                });
            }
        }
        ConditionSourceType::TerrainSwap => {
            if let Some(store) = stores.map_store
                && store.get(condition.source_entry as u32).is_none()
            {
                return Err(Error::NonExistingTerrainSwapMap(
                    condition.source_entry as u32,
                ));
            }
        }
        ConditionSourceType::Phase => {
            if condition.source_entry != 0
                && let Some(store) = stores.area_table_store
                && store.get(condition.source_entry as u32).is_none()
            {
                return Err(Error::NonExistingPhaseArea(condition.source_entry as u32));
            }
        }
        ConditionSourceType::QuestAvailable => {
            if let Some(store) = stores.quest_store
                && store.get(condition.source_entry as u32).is_none()
            {
                return Err(Error::NonExistingQuestAvailable(
                    condition.source_entry as u32,
                ));
            }
        }
        ConditionSourceType::NpcVendor => {
            if let Some(store) = stores.creature_template_store
                && !store.contains(condition.source_group)
            {
                return Err(Error::NonExistingSourceCreatureTemplate {
                    source_type: condition.source_type,
                    entry: condition.source_group as i32,
                });
            }
            if let Some(store) = stores.item_store
                && u32::try_from(condition.source_entry)
                    .ok()
                    .and_then(|item_id| store.get(item_id))
                    .is_none()
            {
                return Err(Error::NonExistingNpcVendorItem(condition.source_entry));
            }
        }
        ConditionSourceType::CreatureTemplateVehicle => {
            if let Some(store) = stores.creature_template_store
                && u32::try_from(condition.source_entry)
                    .ok()
                    .is_none_or(|entry| !store.contains(entry))
            {
                return Err(Error::NonExistingSourceCreatureTemplate {
                    source_type: condition.source_type,
                    entry: condition.source_entry,
                });
            }
        }
        ConditionSourceType::VehicleSpell | ConditionSourceType::SpellClickEvent => {
            if let Some(store) = stores.creature_template_store
                && !store.contains(condition.source_group)
            {
                return Err(Error::NonExistingSourceCreatureTemplate {
                    source_type: condition.source_type,
                    entry: condition.source_group as i32,
                });
            }

            if let Some(store) = stores.spell_store
                && store.get(condition.source_entry).is_none()
            {
                return Err(Error::NonExistingSourceSpell {
                    source_type: condition.source_type,
                    spell_id: condition.source_entry,
                });
            }
        }
        ConditionSourceType::SpellImplicitTarget => {
            if let Some(store) = stores.spell_store {
                let Some(spell) = store.get(condition.source_entry) else {
                    return Err(Error::NonExistingSourceSpell {
                        source_type: condition.source_type,
                        spell_id: condition.source_entry,
                    });
                };
                let normalized_mask =
                    spell.normalized_implicit_target_effect_mask_like_cpp(condition.source_group);
                if normalized_mask == 0 {
                    return Err(Error::InvalidSpellImplicitTargetEffectMask(
                        condition.source_group,
                    ));
                }
                condition.source_group = normalized_mask;
            }
        }
        ConditionSourceType::Spell | ConditionSourceType::SpellProc => {
            if let Some(store) = stores.spell_store
                && store.get(condition.source_entry).is_none()
            {
                return Err(Error::NonExistingSourceSpell {
                    source_type: condition.source_type,
                    spell_id: condition.source_entry,
                });
            }
        }
        ConditionSourceType::TrainerSpell => {
            if let Some(store) = stores.trainer_store
                && !store.contains(condition.source_group)
            {
                return Err(Error::NonExistingTrainer(condition.source_group as i32));
            }

            if let Some(store) = stores.spell_store
                && store.get(condition.source_entry).is_none()
            {
                return Err(Error::NonExistingSourceSpell {
                    source_type: condition.source_type,
                    spell_id: condition.source_entry,
                });
            }
        }
        ConditionSourceType::ConversationLine => {
            if let Some(store) = stores.conversation_line_template_store
                && u32::try_from(condition.source_entry)
                    .ok()
                    .is_none_or(|entry| !store.contains(entry))
            {
                return Err(Error::NonExistingConversationLineTemplate(
                    condition.source_entry,
                ));
            }
        }
        ConditionSourceType::AreaTrigger => {
            if let Some(store) = stores.area_trigger_template_store {
                let id = condition.source_group;
                let is_custom = condition.source_entry == 1;
                if !store.contains(id, is_custom) {
                    return Err(Error::NonExistingAreaTriggerTemplate { id, is_custom });
                }
            }
        }
        ConditionSourceType::AreaTriggerClientTriggered => {
            if let Some(store) = stores.area_trigger_db2_store
                && u32::try_from(condition.source_entry)
                    .ok()
                    .and_then(|id| store.get(id))
                    .is_none()
            {
                return Err(Error::NonExistingClientAreaTrigger(condition.source_entry));
            }
        }
        ConditionSourceType::Graveyard => {
            if let Some(store) = stores.graveyard_store {
                let Some(safe_loc_id) = u32::try_from(condition.source_entry).ok() else {
                    return Err(Error::NonExistingGraveyard {
                        safe_loc_id: condition.source_entry,
                        zone_id: condition.source_group,
                    });
                };

                if store
                    .find_graveyard_data_like_cpp(safe_loc_id, condition.source_group)
                    .is_none()
                {
                    return Err(Error::NonExistingGraveyard {
                        safe_loc_id: condition.source_entry,
                        zone_id: condition.source_group,
                    });
                }
            }
        }
        ConditionSourceType::SpawnGroup => {
            if let Some(store) = stores.spawn_group_store {
                let Some(spawn_group_id) = u32::try_from(condition.source_entry).ok() else {
                    return Err(Error::NonExistingSpawnGroup(condition.source_entry));
                };
                let Some(spawn_group) = store.get(spawn_group_id) else {
                    return Err(Error::NonExistingSpawnGroup(condition.source_entry));
                };
                if spawn_group.is_system_like_cpp() {
                    return Err(Error::SystemSpawnGroup(condition.source_entry));
                }
            }
        }
        ConditionSourceType::ObjectIdVisibility => match condition.source_group {
            value if value == TypeId::Unit as u32 => {
                if let Some(store) = stores.creature_template_store
                    && u32::try_from(condition.source_entry)
                        .ok()
                        .is_none_or(|entry| !store.contains(entry))
                {
                    return Err(Error::NonExistingSourceCreatureTemplate {
                        source_type: condition.source_type,
                        entry: condition.source_entry,
                    });
                }
            }
            value if value == TypeId::GameObject as u32 => {
                if let Some(store) = stores.gameobject_template_store
                    && u32::try_from(condition.source_entry)
                        .ok()
                        .is_none_or(|entry| !store.contains(entry))
                {
                    return Err(Error::NonExistingSourceGameObjectTemplate {
                        source_type: condition.source_type,
                        entry: condition.source_entry,
                    });
                }
            }
            _ => {}
        },
        _ => {}
    }

    Ok(())
}

pub(super) const fn condition_source_is_loot_template_like_cpp(
    source_type: ConditionSourceType,
) -> bool {
    matches!(
        source_type,
        ConditionSourceType::CreatureLootTemplate
            | ConditionSourceType::DisenchantLootTemplate
            | ConditionSourceType::FishingLootTemplate
            | ConditionSourceType::GameObjectLootTemplate
            | ConditionSourceType::ItemLootTemplate
            | ConditionSourceType::MailLootTemplate
            | ConditionSourceType::MillingLootTemplate
            | ConditionSourceType::PickpocketingLootTemplate
            | ConditionSourceType::ProspectingLootTemplate
            | ConditionSourceType::ReferenceLootTemplate
            | ConditionSourceType::SkinningLootTemplate
            | ConditionSourceType::SpellLootTemplate
    )
}

pub fn apply_external_condition_validation_like_cpp(
    report: &mut ConditionLoadReport,
    stores: ConditionExternalValidationStoresLikeCpp<'_>,
) -> Vec<ExternallySkippedConditionLikeCpp> {
    let mut kept = Vec::with_capacity(report.conditions.len());
    let mut skipped = Vec::new();

    for mut condition in report.conditions.drain(..) {
        if let Err(reason) = validate_condition_type_external_like_cpp(&condition, stores) {
            skipped.push(ExternallySkippedConditionLikeCpp {
                condition,
                reason: ConditionRowSkipReason::ConditionTypeValidationFailed(reason),
            });
            continue;
        }

        if let Err(reason) =
            validate_and_normalize_condition_source_external_like_cpp(&mut condition, stores)
        {
            skipped.push(ExternallySkippedConditionLikeCpp {
                condition,
                reason: ConditionRowSkipReason::ConditionSourceValidationFailed(reason),
            });
            continue;
        }

        kept.push(condition);
    }

    report.conditions = kept;
    skipped
}

pub(super) const fn convert_legacy_type_id_like_cpp(legacy_type_id: u32) -> u32 {
    match legacy_type_id {
        0 => TypeId::Object as u32,
        1 => TypeId::Item as u32,
        2 => TypeId::Container as u32,
        3 => TypeId::Unit as u32,
        4 => TypeId::Player as u32,
        5 => TypeId::GameObject as u32,
        6 => TypeId::DynamicObject as u32,
        7 => TypeId::Corpse as u32,
        8 => TypeId::AreaTrigger as u32,
        9 => TypeId::SceneObject as u32,
        10 => TypeId::Conversation as u32,
        _ => TypeId::Object as u32,
    }
}

pub(super) fn convert_legacy_type_mask_like_cpp(legacy_type_mask: u32) -> u32 {
    let mut type_mask = 0;
    for legacy_type_id in 0..11 {
        if legacy_type_mask & (1 << legacy_type_id) != 0 {
            type_mask |= 1 << convert_legacy_type_id_like_cpp(legacy_type_id);
        }
    }

    type_mask
}
