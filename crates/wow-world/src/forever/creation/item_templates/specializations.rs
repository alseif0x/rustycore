//! LoadItemTemplates spec branches, 02245dcd ObjectMgr.cpp:3343-3404.
//! CalculateItemSpecBit ItemTemplate.cpp:296-299: five, not four, per class.
use super::{
    ItemRecord, ItemSpecCatalog, ItemTemplateError, SparseItemRecord, spec_stats::SpecStats,
};
use wow_data::forever_initialization::SpecializationRecord;
const ALL: u128 = (1u128 << 80) - 1;

pub(super) fn sets<'a>(
    basic: &ItemRecord,
    sparse: &SparseItemRecord,
    specs: &ItemSpecCatalog,
    specialization: impl Fn(u32) -> Option<&'a SpecializationRecord>,
) -> Result<(u32, [u128; 3]), ItemTemplateError> {
    let mut mask = 0u32;
    let mut sets = [0u128; 3];
    if let Some(overrides) = specs.overrides(sparse.id) {
        for row in overrides {
            let Some(spec) = specialization(u32::from(row.specialization)) else {
                continue;
            };
            let (class, bit) = bits(spec)?;
            mask |= class;
            sets[0] |= bit;
            sets[1] |= sets[0];
            sets[2] |= sets[0];
        }
    } else {
        let stats = SpecStats::fields(
            basic.class,
            basic.subclass,
            sparse.inventory_type,
            sparse.stat_bonus,
            specs.gem(u32::from(sparse.gem_properties)).map(|r| r.kind),
        );
        for row in specs.specs() {
            if row.item_type != stats.item_type
                || !stats.has(row.primary)
                || !stats.has(row.secondary)
            {
                continue;
            }
            let Some(spec) = specialization(u32::from(row.specialization)) else {
                continue;
            };
            let (class, bit) = bits(spec)?;
            if class & sparse.allowable_class as i32 as u32 == 0 {
                continue;
            }
            mask |= class;
            sets[0] |= bit;
            if row.max_level > 40 {
                sets[1] |= bit;
            }
            if row.max_level >= 110 {
                sets[2] |= bit;
            }
            // Source MinLevel is not consulted by this template operation.
        }
    }
    for set in &mut sets {
        if *set == 0 {
            *set = ALL;
        }
    }
    Ok((mask, sets))
}
fn bits(spec: &SpecializationRecord) -> Result<(u32, u128), ItemTemplateError> {
    if !(1..16).contains(&spec.class) || !(0..5).contains(&spec.order_index) {
        return Err(ItemTemplateError::InvalidSpecialization);
    }
    Ok((
        1u32 << (spec.class - 1),
        1u128 << (u32::from(spec.class - 1) * 5 + spec.order_index as u32),
    ))
}
