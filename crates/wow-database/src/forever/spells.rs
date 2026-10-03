//! World spell startup acquisition, not client hotfix overlays or save.
//! Preserve observed row order; every decode/query succeeds before publication.
use super::field;
use crate::{SqlResult, WorldDatabase};
use std::sync::Arc;
use wow_persistence::forever::{
    LoadError,
    spells::{
        SpellCustomAttributeRow,
        server::{ServerSpellEffectRow, ServerSpellRow, ServerSpellRows},
    },
};
mod custom;
mod decode;
mod immunities;
mod learning;
#[cfg(test)]
mod tests;

const EFFECTS: &str = "SELECT SpellID, EffectIndex, DifficultyID, Effect, EffectAura, EffectAmplitude, EffectAttributes, EffectAuraPeriod, EffectBonusCoefficient, EffectChainAmplitude, EffectChainTargets, EffectItemType, EffectMechanic, EffectPointsPerResource, EffectPosFacing, EffectRealPointsPerLevel, EffectTriggerSpell, BonusCoefficientFromAP, PvpMultiplier, Coefficient, Variance, ResourceCoefficient, GroupSizeBasePointsCoefficient, EffectBasePoints, EffectMiscValue1, EffectMiscValue2, EffectRadiusIndex1, EffectRadiusIndex2, EffectSpellClassMask1, EffectSpellClassMask2, EffectSpellClassMask3, EffectSpellClassMask4, ImplicitTarget1, ImplicitTarget2 FROM serverside_spell_effect";
const SPELLS: &str = "SELECT Id, DifficultyID, CategoryId, Dispel, Mechanic, Attributes, AttributesEx, AttributesEx2, AttributesEx3, AttributesEx4, AttributesEx5, AttributesEx6, AttributesEx7, AttributesEx8, AttributesEx9, AttributesEx10, AttributesEx11, AttributesEx12, AttributesEx13, AttributesEx14, AttributesEx15, AttributesEx16, Stances, StancesNot, Targets, TargetCreatureType, RequiresSpellFocus, FacingCasterFlags, CasterAuraState, TargetAuraState, ExcludeCasterAuraState, ExcludeTargetAuraState, CasterAuraSpell, TargetAuraSpell, ExcludeCasterAuraSpell, ExcludeTargetAuraSpell, CasterAuraType, TargetAuraType, ExcludeCasterAuraType, ExcludeTargetAuraType, CastingTimeIndex, RecoveryTime, CategoryRecoveryTime, StartRecoveryCategory, StartRecoveryTime, InterruptFlags, AuraInterruptFlags1, AuraInterruptFlags2, ChannelInterruptFlags1, ChannelInterruptFlags2, ProcFlags, ProcFlags2, ProcChance, ProcCharges, ProcCooldown, ProcBasePPM, MaxLevel, BaseLevel, SpellLevel, DurationIndex, RangeIndex, Speed, LaunchDelay, StackAmount, EquippedItemClass, EquippedItemSubClassMask, EquippedItemInventoryTypeMask, ContentTuningId, SpellName, ConeAngle, ConeWidth, MaxTargetLevel, MaxAffectedTargets, SpellFamilyName, SpellFamilyFlags1, SpellFamilyFlags2, SpellFamilyFlags3, SpellFamilyFlags4, DmgClass, PreventionType, AreaGroupId, SchoolMask, ChargeCategoryId FROM serverside_spell";

pub struct ForeverSpellWorldRepository(Arc<WorldDatabase>);
impl ForeverSpellWorldRepository {
    pub async fn load_spell_required(
        &self,
    ) -> Result<Vec<wow_persistence::forever::spells::SpellRequiredRow>, LoadError> {
        rows(&self.0, learning::REQUIRED, learning::required).await
    }
    pub async fn load_spell_learn_spells(
        &self,
    ) -> Result<Vec<wow_persistence::forever::spells::SpellLearnRow>, LoadError> {
        rows(&self.0, learning::LEARN, learning::learn).await
    }
    pub fn new(world: Arc<WorldDatabase>) -> Self {
        Self(world)
    }
    pub async fn load_server_spells(&self) -> Result<ServerSpellRows, LoadError> {
        // SpellMgr::LoadSpellInfoServerside reads effects before main rows.
        let effects = rows(&self.0, EFFECTS, decode::effect).await?;
        let spells = rows(&self.0, SPELLS, decode::spell).await?;
        Ok(ServerSpellRows { effects, spells })
    }

    /// Separate first SQL subphase of LoadSpellInfoCustomAttributes. Empty is
    /// a valid batch; query/decode failure is not an empty successful load.
    pub async fn load_spell_custom_attributes(
        &self,
    ) -> Result<Vec<SpellCustomAttributeRow>, LoadError> {
        rows(&self.0, custom::QUERY, custom::decode).await
    }

    pub async fn load_creature_immunities(
        &self,
    ) -> Result<Vec<wow_persistence::forever::spells::CreatureImmunityRow>, LoadError> {
        rows(&self.0, immunities::QUERY, immunities::decode).await
    }
}

async fn rows<T>(
    world: &WorldDatabase,
    sql: &str,
    decode: fn(&SqlResult) -> Result<T, LoadError>,
) -> Result<Vec<T>, LoadError> {
    let mut result = world
        .direct_query(sql)
        .await
        .map_err(|_| LoadError::Database)?;
    let mut rows = Vec::new();
    if !result.is_empty() {
        loop {
            rows.push(decode(&result)?);
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(rows)
}

fn text(row: &SqlResult, index: usize) -> Result<Vec<u8>, LoadError> {
    // Nullable source GetStringView is empty; non-NULL bytes are never made lossy.
    if row.is_null(index) {
        return Ok(Vec::new());
    }
    row.try_read::<Vec<u8>>(index)
        .or_else(|| row.try_read::<String>(index).map(String::into_bytes))
        .ok_or(LoadError::InvalidRow)
}
