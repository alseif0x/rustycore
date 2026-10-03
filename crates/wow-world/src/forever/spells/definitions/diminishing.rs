//! 02245dcd SpellInfo.cpp:2991-3457, SpellMgr.cpp:5354-5362.
//! Source rules retain negative bits and repeated visual queries/draws.
mod groups;
#[cfg(test)]
mod tests;
mod visual;
use super::{Definition, Key, SpellDefinitionSeeds};

#[repr(u16)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiminishingGroup {
    #[default]
    None = 0,
    Root = 1,
    Stun = 2,
    Incapacitate = 3,
    Disorient = 4,
    Silence = 5,
    AoeKnockback = 6,
    Taunt = 7,
    LimitOnly = 8,
}
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiminishingType {
    #[default]
    None = 0,
    Player = 1,
    All = 2,
}
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiminishingLevel {
    Second = 1,
    #[default]
    Immune = 3,
    TauntImmune = 4,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiminishingInfo {
    pub group: DiminishingGroup,
    pub return_type: DiminishingType,
    pub maximum_level: DiminishingLevel,
    pub duration_limit_ms: i32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiminishingCounts {
    pub definitions: usize,
    pub visual_queries: usize,
    pub selections: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellDiminishingError {
    RequiresCustomAttributes,
    AlreadyApplied,
    MissingVisualRecord,
    IncompleteDependencies,
    SelectionUnavailable,
    InvalidSelection,
}

impl SpellDefinitionSeeds {
    /// Null-caster/null-viewer source startup uses the final canonical
    /// UnitCondition store, including ID zero. The borrowed selector uses source
    /// SFMT/libstdc++ weighted-or-uniform selection; no callback is retained.
    pub fn with_diminishing_info(
        mut self,
        select: &mut impl FnMut(&[f64]) -> Result<usize, SpellDiminishingError>,
    ) -> Result<Self, SpellDiminishingError> {
        if self.diminishing.is_some() {
            return Err(SpellDiminishingError::AlreadyApplied);
        }
        if self.custom_attributes.is_none() || self.source_order.is_none() {
            return Err(SpellDiminishingError::RequiresCustomAttributes);
        }
        if self.catalog.counts()[48].2 != 0 {
            return Err(SpellDiminishingError::IncompleteDependencies);
        }
        let mut counts = DiminishingCounts::default();
        let length = self
            .source_order
            .as_ref()
            .expect("admitted order")
            .primary
            .len();
        for position in 0..length {
            let key = self.source_order.as_ref().expect("admitted order").primary[position];
            let group = groups::compute(&self.definitions[&key], key.0, &mut || {
                counts.visual_queries += 1;
                visual::null_caster(
                    &self.catalog,
                    &self.definitions[&key],
                    select,
                    &mut counts.selections,
                )
            })?;
            let info = DiminishingInfo {
                group,
                return_type: match group {
                    DiminishingGroup::Taunt | DiminishingGroup::Stun => DiminishingType::All,
                    DiminishingGroup::None | DiminishingGroup::LimitOnly => DiminishingType::None,
                    _ => DiminishingType::Player,
                },
                maximum_level: match group {
                    DiminishingGroup::Taunt => DiminishingLevel::TauntImmune,
                    DiminishingGroup::AoeKnockback => DiminishingLevel::Second,
                    _ => DiminishingLevel::Immune,
                },
                // Source duration calculation is independent of group,
                // positivity/taunt/ID early returns and resulting return type.
                duration_limit_ms: duration(&self.definitions[&key], key),
            };
            self.definitions
                .get_mut(&key)
                .expect("admitted key")
                .diminishing = info;
            counts.definitions += 1;
        }
        self.diminishing = Some(counts);
        Ok(self)
    }
    pub fn diminishing_counts(&self) -> Option<DiminishingCounts> {
        self.diminishing
    }
}
fn duration(definition: &Definition, key: Key) -> i32 {
    let flags = definition.fields.spell_family_flags;
    match definition.fields.spell_family_name {
        3 if flags[0] & 0x800000 != 0 => 3000,
        5 if key.0 == 170995 => 4000,
        9 if key.0 == 117526 => 3000,
        9 if flags[1] & 0x1000 != 0 => 6000,
        53 if flags[2] & 0x800000 != 0 => 4000,
        107 if matches!(key.0, 217832 | 221527) => 4000,
        _ => 8000,
    }
}
