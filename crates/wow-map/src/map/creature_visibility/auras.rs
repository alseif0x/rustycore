use super::*;
use wow_entities::{
    AuraCastProvenanceLikeCpp, AuraRef, AuraSubsystem, VisibleAuraApplicationLikeCpp,
};

/// Raw slot inputs. Flags, mask folds, points and packet casts remain in APP.
#[derive(Debug)]
pub struct CreatureAuraSlotFacts {
    slot: u8,
    aura_ref: AuraRef,
    matching_effect_masks: Vec<u32>,
    application: Option<VisibleAuraApplicationLikeCpp>,
    provenance: AuraCastProvenanceLikeCpp,
}

impl CreatureAuraSlotFacts {
    pub fn capture(auras: &AuraSubsystem, slot: u8) -> Option<Self> {
        let aura_ref = auras.visible_auras.get(&slot).copied()?;
        Some(Self {
            slot,
            aura_ref,
            // Deliberately match spell+caster, not AppliedAuraRef::slot.
            // Retain source order and raw masks for the original APP fold.
            matching_effect_masks: auras
                .applied_auras
                .iter()
                .filter(|applied| applied.aura_ref() == aura_ref)
                .map(|applied| applied.effect_mask)
                .collect(),
            application: auras.visible_aura_applications_like_cpp.get(&slot).cloned(),
            provenance: auras.aura_cast_provenance_like_cpp(slot),
        })
    }

    pub fn slot(&self) -> u8 {
        self.slot
    }

    pub fn aura_ref(&self) -> AuraRef {
        self.aura_ref
    }

    pub fn matching_effect_masks(&self) -> &[u32] {
        &self.matching_effect_masks
    }

    pub fn application(&self) -> Option<&VisibleAuraApplicationLikeCpp> {
        self.application.as_ref()
    }

    pub fn provenance(&self) -> AuraCastProvenanceLikeCpp {
        self.provenance
    }
}

/// Visible slots from the same Creature observation as CREATE; no later reread.
#[derive(Debug)]
pub struct CreatureInitialAuraFacts {
    guid: ObjectGuid,
    level: u8,
    slots: Vec<CreatureAuraSlotFacts>,
}

impl CreatureInitialAuraFacts {
    pub(super) fn capture(guid: ObjectGuid, level: u8, auras: &AuraSubsystem) -> Self {
        Self {
            guid,
            level,
            // The APP still sorts by slot at its existing publication point.
            slots: auras
                .visible_auras
                .keys()
                .filter_map(|slot| CreatureAuraSlotFacts::capture(auras, *slot))
                .collect(),
        }
    }

    pub fn guid(&self) -> ObjectGuid {
        self.guid
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn slots(&self) -> &[CreatureAuraSlotFacts] {
        &self.slots
    }
}
