//! Exact effect-target metadata, not execution or initialized SpellInfo state.
mod table;
#[cfg(test)]
mod tests;
use super::TargetObject;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EffectTargetType {
    None = 0,
    Explicit = 1,
    Caster = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectTargetInfo {
    effect: u32,
}
impl EffectTargetInfo {
    pub fn from_id(effect: u32) -> Option<Self> {
        table::EFFECTS.get(effect as usize).map(|_| Self { effect })
    }
    pub fn id(self) -> u32 {
        self.effect
    }
    pub fn implicit_type(self) -> EffectTargetType {
        table::EFFECTS[self.effect as usize].0
    }
    pub fn used_object_type(self) -> TargetObject {
        table::EFFECTS[self.effect as usize].1
    }
    /// SpellInfo.cpp:835-856. Each supplied variant covers its complete source
    /// flag group. Caller supplies current src/dst and implicit/provided masks.
    pub fn missing_target_mask(
        self,
        source_set: bool,
        destination_set: bool,
        provided: u32,
    ) -> u32 {
        let mut mask = self.used_object_type().flag_mask();
        if provided & UNIT_MASK != 0 {
            mask &= !UNIT_MASK;
        }
        if provided & CORPSE_MASK != 0 {
            mask &= !(UNIT_MASK | CORPSE_MASK);
        }
        if provided & 0x4000 != 0 {
            mask &= !(0x4000 | 0x800 | 0x10);
        }
        if provided & 0x800 != 0 {
            mask &= !(0x800 | 0x4000);
        }
        if provided & 0x10 != 0 {
            mask &= !(0x10 | 0x4000);
        }
        if destination_set || provided & 0x40 != 0 {
            mask &= !0x40;
        }
        if source_set || provided & 0x20 != 0 {
            mask &= !0x20;
        }
        mask
    }
}
// SpellDefines.h:339-342. No reuse of old cross-version target masks.
pub(super) const UNIT_MASK: u32 = 0x0011_058e;
pub(super) const CORPSE_MASK: u32 = 0x8200;
