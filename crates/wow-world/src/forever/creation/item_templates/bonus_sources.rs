use super::NumericItemTemplates;
use std::convert::Infallible;
use wow_entities::{
    ItemBonusEntry, ItemBonusSource, ItemBonusTemplateFacts, ItemOffsetCurveFacts,
    ItemScalingConfigFacts, ItemSquishEraFacts,
};

// ItemTemplate.h at 02245dcd: Flags[0], Flags[1], and Flags[3] meanings.
const ITEM_FLAG_NO_DISENCHANT: i32 = 0x0000_8000;
const ITEM_FLAG2_NO_TRADE_BIND_ON_ACQUIRE: i32 = 0x0000_0040;
const ITEM_FLAG4_SCRAPABLE: i32 = 0x0000_0020;
const ITEM_FLAG4_NO_SALVAGE: i32 = 0x0080_0000;
const ITEM_FLAG4_RECRAFTABLE: i32 = 0x0100_0000;

/// Entity's immutable BonusData inputs borrowed from the canonical Item templates.
pub struct ItemBonusSources<'a> {
    templates: &'a NumericItemTemplates,
    actual_realm_patch: Option<i32>,
}

impl<'a> ItemBonusSources<'a> {
    /// The realm patch is an already-resolved fact; `None` represents no realm.
    pub fn new(templates: &'a NumericItemTemplates, actual_realm_patch: Option<i32>) -> Self {
        Self {
            templates,
            actual_realm_patch,
        }
    }
}

impl ItemBonusSource for ItemBonusSources<'_> {
    type Error = Infallible;

    fn template_facts(&self, item_id: u32) -> Result<Option<ItemBonusTemplateFacts>, Self::Error> {
        let Some(template) = self.templates.template(item_id) else {
            return Ok(None);
        };
        let extended = template.extended;
        let flags0 = extended.flags[0];
        let flags2 = extended.flags[1];
        let flags4 = extended.flags[3];
        let effect_ids = self
            .templates
            .effects(item_id)
            .map(|effect| effect.id)
            .collect();

        Ok(Some(ItemBonusTemplateFacts {
            item_id,
            quality: i32::from(extended.quality) as u32,
            item_level: u32::from(extended.item_level),
            required_level: i32::from(extended.required_level),
            item_stat_type: extended.stat_bonus,
            stat_percent_editor: extended.stat_percent,
            item_stat_socket_cost_multiplier: extended.socket_percentage,
            socket_color: extended.socket_type.map(u32::from),
            bonding: extended.bonding,
            content_tuning_id: extended.content_tuning as u32,
            player_level_to_item_level_curve_id: extended.player_level_curve as u32,
            item_level_offset_curve_id: extended.item_level_offset_curve as u32,
            item_level_offset_item_level: extended.item_level_offset as u32,
            item_squish_era_id: extended.squish_era as u32,
            limit_category: extended.limit_category as u32,
            effect_ids,
            can_disenchant: flags0 & ITEM_FLAG_NO_DISENCHANT == 0,
            can_scrap: flags4 & ITEM_FLAG4_SCRAPABLE != 0,
            can_salvage: flags4 & ITEM_FLAG4_NO_SALVAGE == 0,
            can_recraft: flags4 & ITEM_FLAG4_RECRAFTABLE != 0,
            cannot_trade_bind_on_pickup: flags2 & ITEM_FLAG2_NO_TRADE_BIND_ON_ACQUIRE != 0,
        }))
    }

    fn azerite_tier_unlock_set_id(&self, item_id: u32) -> Result<Option<u32>, Self::Error> {
        Ok(self
            .templates
            .data
            .azerite_empowered_item_for_item(item_id)
            .map(|row| row.tier_unlock_set))
    }

    fn bonus_list(
        &self,
        list_id: u32,
    ) -> Result<impl Iterator<Item = ItemBonusEntry>, Self::Error> {
        Ok(self
            .templates
            .data
            .item_bonuses_for_list(list_id)
            .map(|row| ItemBonusEntry {
                bonus_type: u32::from(row.bonus_type),
                values: row.values,
            }))
    }

    fn has_item_effect(&self, effect_id: u32) -> Result<bool, Self::Error> {
        Ok(self.templates.data.effect(effect_id).is_some())
    }

    fn scaling_config(
        &self,
        config_id: u32,
    ) -> Result<Option<ItemScalingConfigFacts>, Self::Error> {
        Ok(self
            .templates
            .data
            .item_scaling_config(config_id)
            .map(|row| ItemScalingConfigFacts {
                item_offset_curve_id: row.offset_curve,
                item_level: row.item_level,
                required_level: row.required_level,
                item_squish_era_id: row.squish_era,
                flags: row.flags,
            }))
    }

    fn offset_curve(&self, curve_id: u32) -> Result<Option<ItemOffsetCurveFacts>, Self::Error> {
        Ok(self
            .templates
            .data
            .item_offset_curve(curve_id)
            .map(|row| ItemOffsetCurveFacts {
                curve_id: row.curve,
                offset: row.offset,
            }))
    }

    fn item_squish_era_index_bound(&self) -> Result<u32, Self::Error> {
        Ok(self.templates.data.item_squish_era_storage_last_index() + 1)
    }

    fn item_squish_era(&self, index: u32) -> Result<Option<ItemSquishEraFacts>, Self::Error> {
        Ok(self
            .templates
            .data
            .item_squish_era(index)
            .map(|row| ItemSquishEraFacts {
                patch: row.patch,
                flags: row.flags,
            }))
    }

    fn actual_realm_patch(&self) -> Option<i32> {
        self.actual_realm_patch
    }
}

#[cfg(test)]
#[path = "../../../../unit_tests/forever/creation/item_templates/bonus_sources.rs"]
mod tests;
