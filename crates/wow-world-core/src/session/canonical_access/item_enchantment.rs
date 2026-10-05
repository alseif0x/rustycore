// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::RepresentedPlayerSkillLikeCpp;
use crate::session::SessionCore;
use wow_constants::item::EnchantmentSlot;
use wow_entities::{ApplyEnchantmentArgs, ApplyEnchantmentPlan, ApplyEnchantmentTemplateRef, Item};

/// The owner used by the existing enchantment mutation, including its
/// canonical mutation fallback. No Player or manager guard escapes.
pub struct OwnedItemEnchantmentAccessLikeCpp<'a> {
    core: &'a SessionCore,
    #[cfg(any(test, feature = "test-fixtures"))]
    skill_records: &'a std::collections::HashMap<u16, RepresentedPlayerSkillLikeCpp>,
}

impl SessionCore {
    pub fn owned_item_enchantment_access_like_cpp<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))] skill_records: &'a std::collections::HashMap<
            u16,
            RepresentedPlayerSkillLikeCpp,
        >,
    ) -> OwnedItemEnchantmentAccessLikeCpp<'a> {
        OwnedItemEnchantmentAccessLikeCpp {
            core: self,
            #[cfg(any(test, feature = "test-fixtures"))]
            skill_records,
        }
    }
}

impl OwnedItemEnchantmentAccessLikeCpp<'_> {
    pub fn resolved_player_skill_value_like_cpp(&self, skill_id: u16) -> Option<u16> {
        let records = self
            .core
            .resolved_player_skill_records_for_publication_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.skill_records,
            )?;
        Some(
            crate::session::represented_skill_values_from_records_like_cpp(&records)
                .get(&skill_id)
                .copied()
                .unwrap_or(0),
        )
    }

    pub fn apply_enchantment_plan_like_cpp(
        &self,
        item: &mut Item,
        slot: EnchantmentSlot,
        template: Option<ApplyEnchantmentTemplateRef>,
        args: ApplyEnchantmentArgs,
    ) -> Option<ApplyEnchantmentPlan> {
        self.core.mutate_canonical_player_like_cpp(|player| {
            player.apply_enchantment_plan(Some(item), slot, template, args)
        })
    }
}

pub fn apply_enchantment_template_from_store_like_cpp(
    store: Option<&wow_data::SpellItemEnchantmentStore>,
    enchantment_id: i32,
    required_skill_value: u16,
    condition_fits: bool,
) -> Option<ApplyEnchantmentTemplateRef> {
    let id = u32::try_from(enchantment_id).ok()?;
    store.and_then(|store| store.get(id)).map(|entry| {
        let mut template = ApplyEnchantmentTemplateRef::new(enchantment_id);
        template.condition_id = u32::from(entry.condition_id);
        template.condition_fits = condition_fits;
        template.min_level = entry.min_level;
        template.required_skill_id = u32::from(entry.required_skill_id);
        template.required_skill_rank = entry.required_skill_rank;
        template.required_skill_value = required_skill_value;
        template
    })
}
