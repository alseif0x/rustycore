use crate::{EQUIPMENT_SLOT_END, PlayerInventoryRuntime, PlayerInventoryStorage};
use wow_core::ObjectGuid;

/// The condition columns consumed by native item-gem evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnchantmentConditionFacts {
    id: u32,
    left_operand_types: [u8; 5],
    operators: [u8; 5],
    right_operand_types: [u8; 5],
    right_operands: [u8; 5],
}

impl EnchantmentConditionFacts {
    pub const fn new(
        id: u32,
        left_operand_types: [u8; 5],
        operators: [u8; 5],
        right_operand_types: [u8; 5],
        right_operands: [u8; 5],
    ) -> Self {
        Self {
            id,
            left_operand_types,
            operators,
            right_operand_types,
            right_operands,
        }
    }

    pub const fn id(&self) -> u32 {
        self.id
    }

    pub const fn left_operand_types(&self) -> &[u8; 5] {
        &self.left_operand_types
    }

    pub const fn operators(&self) -> &[u8; 5] {
        &self.operators
    }

    pub const fn right_operand_types(&self) -> &[u8; 5] {
        &self.right_operand_types
    }

    pub const fn right_operands(&self) -> &[u8; 5] {
        &self.right_operands
    }
}

/// Read-only admitted metadata. `Ok(None)` denotes a missing loaded row;
/// `Err` denotes unavailable source authority.
pub trait EnchantmentConditionSource {
    type Error;

    fn condition(&self, id: u32) -> Result<Option<EnchantmentConditionFacts>, Self::Error>;
    fn gem_color_mask(&self, item_entry: u32) -> Result<Option<u32>, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GemConditionOperandSide {
    Left,
    Right,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GemConditionEvaluationError<E> {
    Source(E),
    MissingLiveItem {
        slot: u8,
        guid: ObjectGuid,
    },
    InvalidOperandType {
        row: usize,
        side: GemConditionOperandSide,
        operand_type: u8,
    },
}

/// Evaluate one native gem condition against the current canonical inventory.
///
/// The slot map supplies top-level equipment topology; the runtime object map
/// supplies the actual Item and its complete socket vector.
pub fn enchantment_condition_fits_native_like_cpp<S>(
    inventory: &PlayerInventoryStorage,
    runtime: &PlayerInventoryRuntime,
    condition_id: u32,
    except_slot: i8,
    sources: &S,
) -> Result<bool, GemConditionEvaluationError<S::Error>>
where
    S: EnchantmentConditionSource,
{
    if condition_id == 0 {
        return Ok(true);
    }

    let Some(condition) = sources
        .condition(condition_id)
        .map_err(GemConditionEvaluationError::Source)?
    else {
        return Ok(true);
    };

    let mut gem_counts = [0_u8; 4];
    for (slot, maybe_guid) in inventory
        .items
        .iter()
        .enumerate()
        .take(EQUIPMENT_SLOT_END as usize)
    {
        if slot as i8 == except_slot {
            continue;
        }
        let Some(guid) = maybe_guid else {
            continue;
        };
        let slot = slot as u8;
        let item = runtime
            .item_objects()
            .get(guid)
            .ok_or(GemConditionEvaluationError::MissingLiveItem { slot, guid: *guid })?;
        if item.is_broken() {
            continue;
        }

        for gem in &item.data().gems {
            let Some(color_mask) = sources
                .gem_color_mask(gem.item_id as u32)
                .map_err(GemConditionEvaluationError::Source)?
            else {
                continue;
            };
            for (index, bit) in [1_u32, 2, 4, 8].into_iter().enumerate() {
                if color_mask & bit != 0 {
                    gem_counts[index] = gem_counts[index].wrapping_add(1);
                }
            }
        }
    }

    let mut activate = true;
    for row in 0..5 {
        let left_type = condition.left_operand_types[row];
        if left_type == 0 {
            continue;
        }
        let left = counter_operand(left_type, row, GemConditionOperandSide::Left, &gem_counts)?;
        let right_type = condition.right_operand_types[row];
        let right = if right_type == 0 {
            condition.right_operands[row]
        } else {
            counter_operand(right_type, row, GemConditionOperandSide::Right, &gem_counts)?
        };

        let comparison = match condition.operators[row] {
            2 => Some(left < right),
            3 => Some(left > right),
            5 => Some(left >= right),
            _ => None,
        };
        if let Some(comparison) = comparison {
            activate &= comparison;
        }
    }
    Ok(activate)
}

fn counter_operand<E>(
    operand_type: u8,
    row: usize,
    side: GemConditionOperandSide,
    gem_counts: &[u8; 4],
) -> Result<u8, GemConditionEvaluationError<E>> {
    let Some(index) = operand_type.checked_sub(1).filter(|index| *index < 4) else {
        return Err(GemConditionEvaluationError::InvalidOperandType {
            row,
            side,
            operand_type,
        });
    };
    Ok(gem_counts[usize::from(index)])
}

#[cfg(test)]
#[path = "../../../unit_tests/player/enchantments/condition.rs"]
mod tests;
