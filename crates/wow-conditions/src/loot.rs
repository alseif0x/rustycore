//! LootStore condition-link evaluation.

use num_traits::FromPrimitive;
use wow_constants::ConditionSourceType;
use wow_data::{Condition, ConditionEntriesByTypeStore, ConditionId};
use wow_entities::WorldObject;
use wow_loot::{LootStoreItemContext, condition_source_type_for_loot_store_kind_like_cpp};

use crate::context::ConditionSourceInfo;
use crate::evaluate::is_object_meet_to_conditions_like_cpp;

/// C++ `LootTemplate::LinkConditions` + `LootItem::AllowedForPlayer` condition check.
///
/// Rust keeps loot items immutable and passes `LootStoreItemContext` during fill; this resolves the
/// same condition bucket C++ links into `LootStoreItem::conditions` and evaluates it with the looter
/// as target0.
pub fn is_loot_store_item_meeting_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    context: LootStoreItemContext,
    looter: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    let Some(source_type) = ConditionSourceType::from_u32(
        condition_source_type_for_loot_store_kind_like_cpp(context.store_kind) as u32,
    ) else {
        return true;
    };

    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        source_type,
        ConditionId::new(context.entry, context.item.item_id as i32, 0),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(looter, None, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}
