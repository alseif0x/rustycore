//! SpellEffectInfo.cpp (in SpellInfo.cpp):403-449, including blank gap defaults.
use wow_data::forever_spells::{SpellCatalog, SpellEffectRecord, SpellRadiusRecord};

#[derive(Default)]
pub struct SpellEffectSeed<'a> {
    pub index: u32,
    pub effect: u32,
    pub aura: u32,
    pub aura_period: u32,
    pub base_points: f32,
    pub real_points_per_level: f32,
    pub points_per_resource: f32,
    pub amplitude: f32,
    pub chain_amplitude: f32,
    pub bonus_coefficient: f32,
    pub misc_values: [i32; 2],
    pub mechanic: u32,
    pub position_facing: f32,
    pub implicit_targets: [u32; 2],
    pub chain_targets: i32,
    pub item_type: u32,
    pub trigger_spell: u32,
    pub class_mask: [u32; 4],
    pub bonus_coefficient_from_ap: f32,
    pub scaling_class: i32,
    pub scaling_coefficient: f32,
    pub scaling_variance: f32,
    pub scaling_resource_coefficient: f32,
    pub attributes: u32,
    pub radii: [Option<&'a SpellRadiusRecord>; 2],
}

pub(super) fn seed<'a>(
    catalog: &'a SpellCatalog,
    index: u32,
    row: Option<&SpellEffectRecord>,
) -> SpellEffectSeed<'a> {
    let Some(row) = row else {
        return SpellEffectSeed {
            index,
            ..Default::default()
        };
    };
    SpellEffectSeed {
        index,
        effect: row.effect,
        aura: row.effect_aura as u32,
        aura_period: row.effect_aura_period as u32,
        base_points: row.effect_base_points,
        real_points_per_level: row.effect_real_points_per_level,
        points_per_resource: row.effect_points_per_resource,
        amplitude: row.effect_amplitude,
        chain_amplitude: row.effect_chain_amplitude,
        bonus_coefficient: row.effect_bonus_coefficient,
        misc_values: row.effect_misc_value,
        mechanic: row.effect_mechanic as u32,
        position_facing: row.effect_pos_facing,
        implicit_targets: row.implicit_target.map(|target| target as u32),
        chain_targets: row.effect_chain_targets,
        item_type: row.effect_item_type as u32,
        trigger_spell: row.effect_trigger_spell as u32,
        class_mask: row.effect_spell_class_mask,
        bonus_coefficient_from_ap: row.bonus_coefficient_from_ap,
        scaling_class: row.scaling_class,
        scaling_coefficient: row.coefficient,
        scaling_variance: row.variance,
        scaling_resource_coefficient: row.resource_coefficient,
        attributes: row.effect_attributes as u32,
        radii: row.effect_radius_index.map(|id| catalog.spell_radius(id)),
    }
}

impl SpellEffectSeed<'_> {
    pub(in crate::forever::spells) fn into_owned_values(
        self,
    ) -> crate::forever::spells::definitions::SpellEffectValues {
        crate::forever::spells::definitions::SpellEffectValues {
            index: self.index,
            effect: self.effect,
            aura: self.aura,
            aura_period: self.aura_period,
            base_points: self.base_points,
            real_points_per_level: self.real_points_per_level,
            points_per_resource: self.points_per_resource,
            amplitude: self.amplitude,
            chain_amplitude: self.chain_amplitude,
            bonus_coefficient: self.bonus_coefficient,
            misc_values: self.misc_values,
            mechanic: self.mechanic,
            position_facing: self.position_facing,
            implicit_targets: self.implicit_targets,
            chain_targets: self.chain_targets,
            item_type: self.item_type,
            trigger_spell: self.trigger_spell,
            class_mask: self.class_mask,
            bonus_coefficient_from_ap: self.bonus_coefficient_from_ap,
            scaling_class: self.scaling_class,
            scaling_coefficient: self.scaling_coefficient,
            scaling_variance: self.scaling_variance,
            scaling_resource_coefficient: self.scaling_resource_coefficient,
            attributes: self.attributes,
            radius_ids: self.radii.map(|row| row.map(|row| row.id)),
        }
    }
}
