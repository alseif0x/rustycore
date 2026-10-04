use crate::session::item_modifiers::player_class_mask_for_transmog_like_cpp;
use crate::session::state::SessionCore;
use std::sync::Arc;
use wow_constants::{ItemClass, ItemSubClassArmor};
use wow_core::{ObjectGuid, ObjectGuidGenerator, guid::HighGuid};
use wow_data::{ShieldBlockRegularGameTableLikeCpp, SpellEquippedItemsEntry};
use wow_entities::MAX_ITEM_SPELLS;

impl SessionCore {
    /// Allocate item database/object GUIDs from the process-wide generator.
    ///
    /// C++ initializes this generator once from `MAX(item_instance.guid) + 1`
    /// in `ObjectMgr::SetHighestGuids`, and every `Item::CreateItem` consumes
    /// the next value.  Rust sessions execute concurrently, so the shared
    /// `ObjectGuidGenerator` uses an atomic fetch-add.  Allocations are never
    /// returned after a later persistence failure, matching C++ Item creation.
    pub fn allocate_item_instance_guids_with_generator_like_cpp(
        &self,
        generator: &ObjectGuidGenerator,
        count: usize,
    ) -> Option<Vec<(u64, ObjectGuid)>> {
        if count == 0 {
            return Some(Vec::new());
        }
        if generator.high_guid() != HighGuid::Item {
            return None;
        }

        let realm_id = self.realm_id();
        (0..count)
            .map(|_| {
                let counter = generator.generate();
                let db_guid = u64::try_from(counter).ok()?;
                Some((db_guid, ObjectGuid::create_item(realm_id, counter)))
            })
            .collect()
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn item_spec_class_mask_from_overrides_like_cpp(&self, item_id: u32) -> Option<u32> {
        let overrides = self
            .items
            .spec_override_store
            .as_ref()?
            .overrides_for_item_like_cpp(item_id)?;
        let chr_specializations = self.chr.specialization_store.as_ref()?;

        let mut mask = 0_u32;
        for item_spec_override in overrides {
            if let Some(specialization) =
                chr_specializations.get(u32::from(item_spec_override.spec_id))
            {
                mask |= player_class_mask_for_transmog_like_cpp(specialization.class_id);
            }
        }

        Some(mask)
    }

    /// C++ `DB2Manager::GetItemDisplayId`.
    pub fn item_display_id(&self, item_id: u32, appearance_mod_id: u32) -> Option<u32> {
        let modified = self
            .items
            .modified_appearance_store
            .as_ref()
            .and_then(|store| store.get_for_item(item_id, appearance_mod_id))?;
        let appearance_id = u32::try_from(modified.item_appearance_id).ok()?;
        self.items
            .appearance_store
            .as_ref()
            .and_then(|store| store.item_display_info_id(appearance_id))
    }

    pub fn item_effect_count_like_cpp(&self, item_entry: u32) -> usize {
        self.items
            .effect_store
            .as_ref()
            .map(|store| {
                store
                    .item_effects_for_item_id_like_cpp(item_entry)
                    .len()
                    .min(MAX_ITEM_SPELLS)
            })
            .unwrap_or(0)
    }

    pub fn item_shield_block_value_like_cpp(&self, item_id: u32) -> Option<i16> {
        crate::session::OwnedItemModifiersAccessLikeCpp::item_shield_block_value_from_selected_inputs_like_cpp(
            self.items.store.as_ref(),
            self.items.stats_store.as_ref(),
            self.shield_block_regular_game_table.as_ref(),
            item_id,
        )
    }
}

impl crate::session::OwnedItemModifiersAccessLikeCpp<'_> {
    pub fn item_shield_block_value_from_selected_inputs_like_cpp(
        item_store: Option<&Arc<wow_data::ItemStore>>,
        stats_store: Option<&Arc<wow_data::ItemStatsStore>>,
        shield_block_regular_game_table: Option<&Arc<ShieldBlockRegularGameTableLikeCpp>>,
        item_id: u32,
    ) -> Option<i16> {
        let basic = item_store?.get(item_id)?;
        if basic.class_id != ItemClass::Armor as u8
            || basic.subclass_id != ItemSubClassArmor::Shield as u8
        {
            return None;
        }

        let template = stats_store?.random_property_template(item_id)?;
        let item_level = u32::from(template.item_level);
        let quality = u32::try_from(template.quality).ok()?;
        shield_block_regular_game_table?
            .shield_block_for_quality_like_cpp(item_level, quality)
            .filter(|value| *value != 0)
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn represented_item_fits_spell_requirements_like_cpp(
        &self,
        item_id: u32,
        equipped: &SpellEquippedItemsEntry,
    ) -> bool {
        let Some(item) = self
            .items
            .store
            .as_ref()
            .and_then(|store| store.get(item_id))
        else {
            return false;
        };

        if equipped.equipped_item_class >= 0 {
            if equipped.equipped_item_class != item.class_id as i8 {
                return false;
            }

            if equipped.equipped_item_subclass != 0 {
                let subclass = u32::from(item.subclass_id);
                if subclass >= i32::BITS
                    || (equipped.equipped_item_subclass & (1_i32 << subclass)) == 0
                {
                    return false;
                }
            }
        }

        true
    }
}
