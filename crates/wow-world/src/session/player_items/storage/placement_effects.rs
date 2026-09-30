//! placement effects for the existing storage owner.

use super::*;

impl WorldSession {

    pub(crate) fn remove_inventory_item_duration_refs_like_cpp(&mut self, item_guid: ObjectGuid) {
        let Some(mut item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };

        let Some(removed_enchantments) = self.mutate_canonical_player_like_cpp(|player| {
            let removed_enchantments = player.remove_enchantment_durations(&mut item);
            let _ = player.remove_item_durations(&item);
            removed_enchantments
        }) else {
            return;
        };

        if removed_enchantments.is_empty() {
            return;
        }

        let _ = self.restore_inventory_item_enchantment_durations_like_cpp(
            item_guid,
            &removed_enchantments,
        );
    }
    pub(crate) fn remove_inventory_tradeable_item_like_cpp(&mut self, item_guid: ObjectGuid) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };

        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.remove_tradeable_item(&item);
        });
    }
    pub(crate) fn add_inventory_item_duration_refs_like_cpp(&mut self, item_guid: ObjectGuid) {
        let Some(mut item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };
        let Some((owner_guid, item_update, enchantment_updates)) = self
            .mutate_canonical_player_like_cpp(|player| {
                let item_update = player.add_item_durations(&item);
                let enchantment_updates = player.add_enchantment_durations(&mut item);
                (player.guid(), item_update, enchantment_updates)
            })
        else {
            return;
        };

        self.insert_inventory_item_object(item);
        if let Some(update) = item_update {
            self.send_item_time_update_plan(&update);
        }
        self.send_item_enchant_time_update_plans(owner_guid, &enchantment_updates);
    }
    /// C++ `Player::ApplyItemObtainSpells(item, true)` after `_StoreItem`.
    pub(crate) async fn apply_inventory_item_obtain_spells_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        item_entry: u32,
    ) -> Vec<i32> {
        if self
            .item_template_flags(item_entry)
            .is_some_and(|flags| flags.contains(ItemFlags::LEGACY))
        {
            return Vec::new();
        }
        let spell_ids = self
            .items
            .effect_store
            .as_ref()
            .map(|store| {
                store
                    .item_effects_for_item_id_like_cpp(item_entry)
                    .into_iter()
                    .filter(|effect| {
                        effect.trigger_type == ItemSpelltriggerType::OnPickup as i8
                            && effect.spell_id > 0
                    })
                    .map(|effect| effect.spell_id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let Some(player_guid) = self.player_guid() else {
            return Vec::new();
        };
        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return Vec::new();
        };
        let mut applied = Vec::new();
        for spell_id in spell_ids {
            if visible_auras.values().any(|aura| aura.spell_id == spell_id) {
                continue;
            }
            // C++ `Player::ApplyItemObtainSpells` uses
            // `CastSpellExtraArgs().SetCastItem(item)`, whose default trigger
            // flags are TRIGGERED_NONE: not a triggered cast, and the global
            // cooldown applies.
            if self
                .execute_server_triggered_spell_like_cpp(
                    item_guid_generator,
                    creature_spawn_catalogs,
                    spell_id,
                    player_guid,
                    SpellCastMetadata::default(),
                )
                .await
                .is_ok()
            {
                applied.push(spell_id);
            }
        }
        applied
    }
    #[cfg(test)]
    pub(crate) async fn apply_inventory_item_obtain_spells_like_cpp(
        &mut self,
        item_entry: u32,
    ) -> Vec<i32> {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.apply_inventory_item_obtain_spells_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            item_entry,
        )
        .await
    }
    pub(crate) fn apply_inventory_item_remove_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
        cleared_mainhand_enchantments: &[EnchantmentSlot],
    ) -> bool {
        self.remove_inventory_item_duration_refs_like_cpp(item_guid);
        self.remove_inventory_tradeable_item_like_cpp(item_guid);

        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }

        let _ = self.record_direct_inventory_item_set_remove_like_cpp(bag, slot, item_guid);
        let item_mods_changed =
            self.record_destroyed_inventory_item_mod_remove_like_cpp(bag, slot, item_guid);
        let _ = self
            .clear_inventory_item_equipped_state_like_cpp(item_guid, cleared_mainhand_enchantments);

        if slot < PROFESSION_SLOT_END {
            self.record_inventory_item_combat_stat_recalculations_like_cpp(slot);
        }
        item_mods_changed
    }
    /// C++ `StoreItem`/`BankItem`/`EquipItem` post-placement side effects for
    /// a runtime item whose persistence and position have already committed.
    pub(crate) fn apply_inventory_item_store_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        self.add_inventory_item_duration_refs_like_cpp(item_guid);
        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }

        let _ = self.set_inventory_item_equipped_like_cpp(item_guid, true);
        let _ = self.record_represented_items_set_item_like_cpp(item_guid, true);
        let item_mods_changed = if self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some_and(|item| !item.is_broken())
        {
            self.record_represented_item_mods_like_cpp(item_guid, slot, true) != 0
        } else {
            false
        };
        if slot < PROFESSION_SLOT_END {
            self.record_inventory_item_combat_stat_recalculations_like_cpp(slot);
        }
        item_mods_changed
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_inventory_item_combat_stat_recalculations_like_cpp(&mut self, slot: u8) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.inventory_fixture_diagnostics_enabled_for_test() {
            let attack = match slot {
                EQUIPMENT_SLOT_MAINHAND => Some(WeaponAttackType::BaseAttack),
                EQUIPMENT_SLOT_OFFHAND => Some(WeaponAttackType::OffAttack),
                _ => None,
            };
            if let Some(attack) = attack {
                self.player_item_test_fixture_like_cpp
                    .represented_combat_stat_recalculations_like_cpp
                    .push(RepresentedCombatStatRecalculationLikeCpp::Expertise { attack });
                self.player_item_test_fixture_like_cpp
                    .represented_combat_stat_recalculations_like_cpp
                    .push(RepresentedCombatStatRecalculationLikeCpp::Rating {
                        combat_rating: CR_ARMOR_PENETRATION_LIKE_CPP,
                    });
            }
        }
    }
}
