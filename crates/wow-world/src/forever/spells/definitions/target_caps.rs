//! Complete 02245dcd SpellMgr.cpp:5422-5576 target-cap phase.
//! SpellInfo.cpp:3655-3703: assign caps, then diagnostic CalcBaseValue only.
#[cfg(test)]
mod tests;
use super::{SpellDefinitionError, SpellDefinitionSeeds, SpellValueError};
use wow_data::forever_birth::item_records::ItemCatalog;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SqrtTargetLimit {
    pub max_targets: i32,
    pub non_diminished_targets: i32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TargetCapCounts {
    pub patch_groups: usize,
    pub requested_spells: usize,
    pub missing_spells: usize,
    pub applications: usize,
    pub missing_value_holders: usize,
    pub missing_holder_effects: usize,
    pub mismatched_values: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellTargetCapError {
    RequiresImmunities,
    AlreadyApplied,
    DefinitionLookup(SpellDefinitionError),
    Value(SpellValueError),
}

struct Rule {
    ids: &'static [u32],
    maximum: i32,
    // None ignores diagnostics; (None, slot) uses current SpellInfo;
    // (Some(id), slot) looks up its difficulty with source fallback.
    holder: Option<(Option<u32>, usize)>,
}
const RULES: [Rule; 25] = [
    Rule {
        ids: &[198030],
        maximum: 5,
        holder: Some((Some(198013), 4)),
    },
    Rule {
        ids: &[453035],
        maximum: 8,
        holder: Some((Some(453034), 1)),
    },
    Rule {
        ids: &[258860],
        maximum: 8,
        holder: Some((None, 1)),
    },
    Rule {
        ids: &[258926],
        maximum: 5,
        holder: Some((None, 1)),
    },
    Rule {
        ids: &[390137],
        maximum: 5,
        holder: Some((Some(389693), 1)),
    },
    Rule {
        ids: &[53385],
        maximum: 5,
        holder: Some((None, 1)),
    },
    Rule {
        ids: &[404358],
        maximum: 5,
        holder: None,
    },
    Rule {
        ids: &[157997],
        maximum: 8,
        holder: Some((None, 2)),
    },
    Rule {
        ids: &[400254],
        maximum: 5,
        holder: Some((None, 2)),
    },
    Rule {
        ids: &[212680],
        maximum: 5,
        holder: Some((Some(212431), 1)),
    },
    Rule {
        ids: &[115310],
        maximum: 5,
        holder: Some((None, 4)),
    },
    Rule {
        ids: &[388615],
        maximum: 5,
        holder: Some((None, 4)),
    },
    Rule {
        ids: &[121253],
        maximum: 5,
        holder: Some((None, 6)),
    },
    Rule {
        ids: &[385060, 385061, 385062],
        maximum: 8,
        holder: Some((Some(385059), 5)),
    },
    Rule {
        ids: &[205472],
        maximum: 8,
        holder: Some((None, 1)),
    },
    Rule {
        ids: &[2120, 1254851],
        maximum: 8,
        holder: Some((None, 1)),
    },
    Rule {
        ids: &[351140],
        maximum: 8,
        holder: None,
    },
    Rule {
        ids: &[199667, 44949, 199852, 199851],
        maximum: 5,
        holder: Some((Some(190411), 2)),
    },
    Rule {
        ids: &[307046],
        maximum: 5,
        holder: Some((Some(306830), 0)),
    },
    Rule {
        ids: &[389860],
        maximum: 5,
        holder: Some((Some(390163), 0)),
    },
    Rule {
        ids: &[400370],
        maximum: 5,
        holder: Some((None, 1)),
    },
    Rule {
        ids: &[435222],
        maximum: 5,
        holder: Some((None, 4)),
    },
    Rule {
        ids: &[1265579, 1265580, 1265581, 1265582],
        maximum: 5,
        holder: Some((Some(1265357), 0)),
    },
    Rule {
        ids: &[1225827, 1279200],
        maximum: 5,
        holder: Some((Some(1226033), 0)),
    },
    Rule {
        ids: &[1261215],
        maximum: 8,
        holder: Some((Some(1261193), 1)),
    },
];

impl SpellDefinitionSeeds {
    pub fn with_target_caps(mut self, items: &ItemCatalog) -> Result<Self, SpellTargetCapError> {
        if self.target_caps.is_some() {
            return Err(SpellTargetCapError::AlreadyApplied);
        }
        if self.immunities.is_none() {
            return Err(SpellTargetCapError::RequiresImmunities);
        }
        let mut counts = TargetCapCounts::default();
        for rule in RULES {
            counts.patch_groups += 1;
            for &id in rule.ids {
                counts.requested_spells += 1;
                let length = self
                    .source_order
                    .as_ref()
                    .expect("immunities admitted source order")
                    .by_spell
                    .get(&id)
                    .map_or(0, Vec::len);
                if length == 0 {
                    counts.missing_spells += 1;
                }
                for position in 0..length {
                    let key =
                        self.source_order.as_ref().expect("admitted order").by_spell[&id][position];
                    // Source assigns caps even for an absent/mismatched diagnostic
                    // holder. Never replace its hardcoded cap with the holder value.
                    self.definitions
                        .get_mut(&key)
                        .expect("admitted definition")
                        .target_limit = SqrtTargetLimit {
                        max_targets: rule.maximum,
                        non_diminished_targets: 0,
                    };
                    counts.applications += 1;
                    let Some((holder_id, slot)) = rule.holder else {
                        continue;
                    };
                    let holder_id = holder_id.unwrap_or(id);
                    let Some(holder) = self
                        .get(holder_id, key.1)
                        .map_err(SpellTargetCapError::DefinitionLookup)?
                    else {
                        counts.missing_value_holders += 1;
                        continue;
                    };
                    if holder.effect(slot).is_none() {
                        counts.missing_holder_effects += 1;
                        continue;
                    }
                    // No variance/CalcValue/RNG: source casts CalcBaseValue directly.
                    let value = self
                        .calculate_startup_base_value(holder_id, key.1, slot, items)
                        .map_err(SpellTargetCapError::Value)?
                        .expect("checked holder and physical slot");
                    let value = super::StartupSpellValue {
                        value,
                        variance: None,
                    }
                    .as_int()
                    .map_err(SpellTargetCapError::Value)?;
                    counts.mismatched_values += usize::from(value != rule.maximum);
                }
            }
        }
        self.target_caps = Some(counts);
        Ok(self)
    }
    pub fn target_cap_counts(&self) -> Option<TargetCapCounts> {
        self.target_caps
    }
}
