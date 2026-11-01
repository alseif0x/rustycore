use anyhow::{Context, Result};
use wow_persistence::{
    ServersideSpellEffectPersistenceRowLikeCpp, ServersideSpellPersistenceRowLikeCpp,
};

use crate::field_like_cpp::{FieldNumberLikeCpp, FieldRowLikeCpp, NativeFieldLikeCpp};
use crate::{WorldDatabase, WorldStatements};

fn numeric_array<T: FieldNumberLikeCpp, const N: usize>(
    row: &[NativeFieldLikeCpp],
    first_column: usize,
    table: &str,
    field: &str,
) -> Result<[T; N]> {
    let mut values = [T::default(); N];
    for (index, value) in values.iter_mut().enumerate() {
        *value = row.get_like_cpp(
            first_column + index,
            &format!("{table}.{field}{}", index + 1),
        )?;
    }
    Ok(values)
}

fn decode_effect(row: &[NativeFieldLikeCpp]) -> Result<ServersideSpellEffectPersistenceRowLikeCpp> {
    Ok(ServersideSpellEffectPersistenceRowLikeCpp {
        spell_id: row.get_like_cpp(0, "serverside_spell_effect.SpellID")?,
        effect_index: row.get_like_cpp(1, "serverside_spell_effect.EffectIndex")?,
        difficulty_id: row.get_like_cpp(2, "serverside_spell_effect.DifficultyID")?,
        effect: row.get_like_cpp(3, "serverside_spell_effect.Effect")?,
        effect_aura: i32::from(row.get_like_cpp::<i16>(4, "serverside_spell_effect.EffectAura")?),
        effect_amplitude: row.get_like_cpp(5, "serverside_spell_effect.EffectAmplitude")?,
        effect_attributes: row.get_like_cpp(6, "serverside_spell_effect.EffectAttributes")?,
        effect_aura_period: row.get_like_cpp(7, "serverside_spell_effect.EffectAuraPeriod")?,
        effect_bonus_coefficient: row
            .get_like_cpp(8, "serverside_spell_effect.EffectBonusCoefficient")?,
        effect_chain_amplitude: row
            .get_like_cpp(9, "serverside_spell_effect.EffectChainAmplitude")?,
        effect_chain_targets: row.get_like_cpp(10, "serverside_spell_effect.EffectChainTargets")?,
        effect_item_type: row.get_like_cpp(11, "serverside_spell_effect.EffectItemType")?,
        effect_mechanic: row.get_like_cpp(12, "serverside_spell_effect.EffectMechanic")?,
        effect_points_per_resource: row
            .get_like_cpp(13, "serverside_spell_effect.EffectPointsPerResource")?,
        effect_pos_facing: row.get_like_cpp(14, "serverside_spell_effect.EffectPosFacing")?,
        effect_real_points_per_level: row
            .get_like_cpp(15, "serverside_spell_effect.EffectRealPointsPerLevel")?,
        effect_trigger_spell: row.get_like_cpp(16, "serverside_spell_effect.EffectTriggerSpell")?,
        bonus_coefficient_from_ap: row
            .get_like_cpp(17, "serverside_spell_effect.BonusCoefficientFromAP")?,
        pvp_multiplier: row.get_like_cpp(18, "serverside_spell_effect.PvpMultiplier")?,
        coefficient: row.get_like_cpp(19, "serverside_spell_effect.Coefficient")?,
        variance: row.get_like_cpp(20, "serverside_spell_effect.Variance")?,
        resource_coefficient: row
            .get_like_cpp(21, "serverside_spell_effect.ResourceCoefficient")?,
        group_size_base_points_coefficient: row
            .get_like_cpp(22, "serverside_spell_effect.GroupSizeBasePointsCoefficient")?,
        effect_base_points: row.get_like_cpp(23, "serverside_spell_effect.EffectBasePoints")?,
        effect_misc_value: numeric_array(row, 24, "serverside_spell_effect", "EffectMiscValue")?,
        effect_radius_index: numeric_array(
            row,
            26,
            "serverside_spell_effect",
            "EffectRadiusIndex",
        )?,
        effect_spell_class_mask: numeric_array(
            row,
            28,
            "serverside_spell_effect",
            "EffectSpellClassMask",
        )?,
        implicit_target: [
            i32::from(row.get_like_cpp::<i16>(32, "serverside_spell_effect.ImplicitTarget1")?),
            i32::from(row.get_like_cpp::<i16>(33, "serverside_spell_effect.ImplicitTarget2")?),
        ],
    })
}

fn decode_spell(row: &[NativeFieldLikeCpp]) -> Result<ServersideSpellPersistenceRowLikeCpp> {
    Ok(ServersideSpellPersistenceRowLikeCpp {
        spell_id: row.get_like_cpp(0, "serverside_spell.SpellID")?,
        difficulty_id: row.get_like_cpp(1, "serverside_spell.DifficultyID")?,
        category_id: row.get_like_cpp(2, "serverside_spell.CategoryID")?,
        dispel: row.get_like_cpp(3, "serverside_spell.Dispel")?,
        mechanic: row.get_like_cpp(4, "serverside_spell.Mechanic")?,
        attributes: row.get_like_cpp(5, "serverside_spell.Attributes")?,
        attributes_ex: numeric_array(row, 6, "serverside_spell", "AttributesEx")?,
        stances: row.get_like_cpp(20, "serverside_spell.Stances")?,
        stances_not: row.get_like_cpp(21, "serverside_spell.StancesNot")?,
        targets: row.get_like_cpp(22, "serverside_spell.Targets")?,
        target_creature_type: row.get_like_cpp(23, "serverside_spell.TargetCreatureType")?,
        requires_spell_focus: row.get_like_cpp(24, "serverside_spell.RequiresSpellFocus")?,
        facing_caster_flags: row.get_like_cpp(25, "serverside_spell.FacingCasterFlags")?,
        caster_aura_state: row.get_like_cpp(26, "serverside_spell.CasterAuraState")?,
        target_aura_state: row.get_like_cpp(27, "serverside_spell.TargetAuraState")?,
        exclude_caster_aura_state: row
            .get_like_cpp(28, "serverside_spell.ExcludeCasterAuraState")?,
        exclude_target_aura_state: row
            .get_like_cpp(29, "serverside_spell.ExcludeTargetAuraState")?,
        caster_aura_spell: row.get_like_cpp(30, "serverside_spell.CasterAuraSpell")?,
        target_aura_spell: row.get_like_cpp(31, "serverside_spell.TargetAuraSpell")?,
        exclude_caster_aura_spell: row
            .get_like_cpp(32, "serverside_spell.ExcludeCasterAuraSpell")?,
        exclude_target_aura_spell: row
            .get_like_cpp(33, "serverside_spell.ExcludeTargetAuraSpell")?,
        caster_aura_type: row.get_like_cpp(34, "serverside_spell.CasterAuraType")?,
        target_aura_type: row.get_like_cpp(35, "serverside_spell.TargetAuraType")?,
        exclude_caster_aura_type: row.get_like_cpp(36, "serverside_spell.ExcludeCasterAuraType")?,
        exclude_target_aura_type: row.get_like_cpp(37, "serverside_spell.ExcludeTargetAuraType")?,
        casting_time_index: row.get_like_cpp(38, "serverside_spell.CastingTimeIndex")?,
        recovery_time: row.get_like_cpp(39, "serverside_spell.RecoveryTime")?,
        category_recovery_time: row.get_like_cpp(40, "serverside_spell.CategoryRecoveryTime")?,
        start_recovery_category: row.get_like_cpp(41, "serverside_spell.StartRecoveryCategory")?,
        start_recovery_time: row.get_like_cpp(42, "serverside_spell.StartRecoveryTime")?,
        interrupt_flags: row.get_like_cpp(43, "serverside_spell.InterruptFlags")?,
        aura_interrupt_flags: numeric_array(row, 44, "serverside_spell", "AuraInterruptFlags")?,
        channel_interrupt_flags: numeric_array(
            row,
            46,
            "serverside_spell",
            "ChannelInterruptFlags",
        )?,
        proc_flags: numeric_array(row, 48, "serverside_spell", "ProcFlags")?,
        proc_chance: row.get_like_cpp(50, "serverside_spell.ProcChance")?,
        proc_charges: row.get_like_cpp(51, "serverside_spell.ProcCharges")?,
        proc_cooldown: row.get_like_cpp(52, "serverside_spell.ProcCooldown")?,
        proc_base_ppm: row.get_like_cpp(53, "serverside_spell.ProcBasePPM")?,
        max_level: row.get_like_cpp(54, "serverside_spell.MaxLevel")?,
        base_level: row.get_like_cpp(55, "serverside_spell.BaseLevel")?,
        spell_level: row.get_like_cpp(56, "serverside_spell.SpellLevel")?,
        duration_index: row.get_like_cpp(57, "serverside_spell.DurationIndex")?,
        range_index: row.get_like_cpp(58, "serverside_spell.RangeIndex")?,
        speed: row.get_like_cpp(59, "serverside_spell.Speed")?,
        launch_delay: row.get_like_cpp(60, "serverside_spell.LaunchDelay")?,
        stack_amount: row.get_like_cpp(61, "serverside_spell.StackAmount")?,
        equipped_item_class: row.get_like_cpp(62, "serverside_spell.EquippedItemClass")?,
        equipped_item_sub_class_mask: row
            .get_like_cpp(63, "serverside_spell.EquippedItemSubClassMask")?,
        equipped_item_inventory_type_mask: row
            .get_like_cpp(64, "serverside_spell.EquippedItemInventoryTypeMask")?,
        content_tuning_id: row.get_like_cpp(65, "serverside_spell.ContentTuningId")?,
        spell_name: row.get_string_like_cpp(66, "serverside_spell.SpellName")?,
        cone_angle: row.get_like_cpp(67, "serverside_spell.ConeAngle")?,
        cone_width: row.get_like_cpp(68, "serverside_spell.ConeWidth")?,
        max_target_level: row.get_like_cpp(69, "serverside_spell.MaxTargetLevel")?,
        max_affected_targets: row.get_like_cpp(70, "serverside_spell.MaxAffectedTargets")?,
        spell_family_name: row.get_like_cpp(71, "serverside_spell.SpellFamilyName")?,
        spell_family_flags: numeric_array(row, 72, "serverside_spell", "SpellFamilyFlags")?,
        dmg_class: row.get_like_cpp(76, "serverside_spell.DmgClass")?,
        prevention_type: row.get_like_cpp(77, "serverside_spell.PreventionType")?,
        area_group_id: row.get_like_cpp(78, "serverside_spell.AreaGroupId")?,
        school_mask: row.get_like_cpp(79, "serverside_spell.SchoolMask")?,
        charge_category_id: row.get_like_cpp(80, "serverside_spell.ChargeCategoryId")?,
    })
}

async fn query_rows<T>(
    db: &WorldDatabase,
    statement: WorldStatements,
    table: &'static str,
    mut decode: impl FnMut(&[NativeFieldLikeCpp]) -> Result<T>,
) -> Result<Vec<T>> {
    let mut result = db
        .query(&db.prepare(statement))
        .await
        .with_context(|| format!("query {table}"))?;
    let mut rows = Vec::with_capacity(result.count());
    if result.is_empty() {
        return Ok(rows);
    }
    let mut row_index = 0;
    loop {
        let native = result
            .native_row_like_cpp()
            .with_context(|| format!("{table} row {row_index} native fields"))?;
        append_checked(&mut rows, table, row_index, &native, &mut decode)?;
        row_index += 1;
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

fn append_checked<T>(
    rows: &mut Vec<T>,
    table: &str,
    row_index: usize,
    native: &[NativeFieldLikeCpp],
    decode: &mut impl FnMut(&[NativeFieldLikeCpp]) -> Result<T>,
) -> Result<()> {
    rows.push(decode(native).with_context(|| format!("{table} row {row_index}"))?);
    Ok(())
}

pub(super) async fn serverside_effects(
    db: &WorldDatabase,
) -> Result<Vec<ServersideSpellEffectPersistenceRowLikeCpp>> {
    query_rows(
        db,
        WorldStatements::SEL_SERVERSIDE_SPELL_EFFECT,
        "serverside_spell_effect",
        decode_effect,
    )
    .await
}

pub(super) async fn serverside_spells(
    db: &WorldDatabase,
) -> Result<Vec<ServersideSpellPersistenceRowLikeCpp>> {
    query_rows(
        db,
        WorldStatements::SEL_SERVERSIDE_SPELL,
        "serverside_spell",
        decode_spell,
    )
    .await
}

#[cfg(test)]
#[path = "checked_rows_tests.rs"]
mod tests;
