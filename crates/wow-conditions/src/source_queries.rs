//! ConditionMgr source-specific queries other than spell click and loot.

use wow_constants::ConditionSourceType;
use wow_data::{
    Condition, ConditionEntriesByTypeStore, ConditionId, PlayerConditionContextLikeCpp,
    PlayerConditionStore,
};
use wow_entities::WorldObject;

use crate::context::{
    ConditionMeetResult, ConditionPlayerSnapshot, ConditionSourceInfo, ConditionUnitSnapshot,
};
use crate::evaluate::is_object_meet_to_conditions_like_cpp;

/// C++ `ConditionMgr::IsObjectMeetingVehicleSpellConditions`.
pub fn is_object_meeting_vehicle_spell_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    creature_id: u32,
    spell_id: u32,
    player: Option<&'a WorldObject>,
    vehicle: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        ConditionSourceType::VehicleSpell,
        ConditionId::new(creature_id, spell_id as i32, 0),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(player, vehicle, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}

/// C++ `ConditionMgr::IsObjectMeetingSmartEventConditions`.
pub fn is_object_meeting_smart_event_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    entry_or_guid: i64,
    event_id: u32,
    source_type: u32,
    unit: Option<&'a WorldObject>,
    base_object: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        ConditionSourceType::SmartEvent,
        ConditionId::new(event_id + 1, entry_or_guid as i32, source_type),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(unit, base_object, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}

/// C++ `ConditionMgr::IsObjectMeetingVendorItemConditions`.
pub fn is_object_meeting_vendor_item_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    creature_id: u32,
    item_id: u32,
    player: Option<&'a WorldObject>,
    vendor: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        ConditionSourceType::NpcVendor,
        ConditionId::new(creature_id, item_id as i32, 0),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(player, vendor, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}

/// Evaluate vendor conditions from already-resolved runtime snapshots.
///
/// The snapshot adapter belongs to the condition boundary; handlers remain
/// responsible for constructing the snapshots and deciding what to do when
/// this read-only query rejects an item.
pub fn is_vendor_item_conditions_with_snapshots_like_cpp(
    condition_store: &ConditionEntriesByTypeStore,
    creature_id: u32,
    item_id: u32,
    player: Option<&WorldObject>,
    vendor: Option<&WorldObject>,
    player_unit_snapshot: ConditionUnitSnapshot,
    player_snapshot: ConditionPlayerSnapshot,
    vendor_unit_snapshot: Option<ConditionUnitSnapshot>,
    player_condition_store: Option<&PlayerConditionStore>,
    player_condition_context: Option<PlayerConditionContextLikeCpp<'_>>,
) -> bool {
    is_object_meeting_vendor_item_conditions_like_cpp(
        condition_store,
        creature_id,
        item_id,
        player,
        vendor,
        |condition, source_info| {
            source_info.set_unit_target_snapshot(0, player_unit_snapshot);
            source_info.set_player_target_snapshot(0, player_snapshot);
            if let Some(vendor_unit_snapshot) = vendor_unit_snapshot {
                source_info.set_unit_target_snapshot(1, vendor_unit_snapshot);
            }
            if let (Some(store), Some(context)) = (player_condition_store, player_condition_context)
            {
                source_info.set_player_condition_store(store);
                source_info.set_player_condition_context(0, context);
            }
            match crate::condition_meets_basic_like_cpp(
                condition,
                source_info,
                |current_area, required_area| current_area == required_area,
            ) {
                ConditionMeetResult::Evaluated(value) => value,
                ConditionMeetResult::Unsupported => false,
            }
        },
    )
}

/// C++ `ConditionMgr::GetConditionsForAreaTrigger`.
pub fn conditions_for_area_trigger_like_cpp(
    condition_store: &ConditionEntriesByTypeStore,
    area_trigger_id: u32,
    is_server_side: bool,
) -> Option<&[Condition]> {
    condition_store
        .conditions_for_like_cpp(
            ConditionSourceType::AreaTrigger,
            ConditionId::new(area_trigger_id, i32::from(is_server_side), 0),
        )
        .map(|conditions| conditions.as_slice())
}

/// C++ `ConditionMgr::IsObjectMeetingTrainerSpellConditions`.
pub fn is_object_meeting_trainer_spell_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    trainer_id: u32,
    spell_id: u32,
    player: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        ConditionSourceType::TrainerSpell,
        ConditionId::new(trainer_id, spell_id as i32, 0),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(player, None, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}

/// C++ `ConditionMgr::IsObjectMeetingVisibilityByObjectIdConditions`.
pub fn is_object_meeting_visibility_by_object_id_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    object_type: u32,
    entry: u32,
    seer: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        ConditionSourceType::ObjectIdVisibility,
        ConditionId::new(object_type, entry as i32, 0),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(seer, None, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}
