//! bonus planning for the existing modifiers owner.

use super::*;

impl WorldSession {
    pub(in crate::session) fn record_represented_all_item_mods_like_cpp(
        &mut self,
        targets: &[(u8, ObjectGuid)],
        apply: bool,
    ) {
        for (slot, item_guid) in targets {
            self.record_represented_item_mods_like_cpp(*item_guid, *slot, apply);
        }
    }
    pub(in crate::session) fn record_represented_item_mods_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: u8,
        apply: bool,
    ) -> usize {
        #[cfg(test)]
        {
            self.player_item_test_fixture_like_cpp
                .represented_item_mod_reapply_events_like_cpp
                .push(RepresentedItemModsReapplyEventLikeCpp {
                    item_guid,
                    slot,
                    apply,
                });
        }

        let Some(item_entry) = self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .map(|item| item.object().entry())
        else {
            return 0;
        };
        let Some(item_stats_store) = self.items.stats_store.as_ref().cloned() else {
            return 0;
        };
        let mut planned_actions = Vec::new();

        let scaling_context = self.represented_scaling_stat_context_like_cpp(item_entry);
        if let Some(context) = scaling_context {
            planned_actions.extend(
                item_scaling_stat_bonus_actions_like_cpp(
                    &context.stat_id,
                    &context.bonus,
                    context.ssd_multiplier,
                    apply,
                )
                .into_iter()
                .map(|action| RepresentedItemBonusActionLikeCpp {
                    item_guid,
                    slot,
                    action,
                }),
            );
            if context.spell_bonus > 0 {
                planned_actions.push(RepresentedItemBonusActionLikeCpp {
                    item_guid,
                    slot,
                    action: ApplyEnchantmentEffectAction::SpellPowerBonus {
                        amount: context.spell_bonus as u32,
                        apply,
                    },
                });
            } else if context.spell_bonus < 0 {
                planned_actions.push(RepresentedItemBonusActionLikeCpp {
                    item_guid,
                    slot,
                    action: ApplyEnchantmentEffectAction::UnhandledStatModifier {
                        item_mod: wow_constants::ItemModType::SpellPower,
                        amount: context.spell_bonus.unsigned_abs(),
                        apply,
                    },
                });
            }
        } else if let Some(stat_entry) = item_stats_store.get(item_entry) {
            planned_actions.extend(
                item_stat_bonus_actions_like_cpp(&stat_entry.stats, apply)
                    .into_iter()
                    .map(|action| RepresentedItemBonusActionLikeCpp {
                        item_guid,
                        slot,
                        action,
                    }),
            );
        }

        if let Some(stat_entry) = item_stats_store.get(item_entry) {
            let resistances = self.represented_resistances_with_scaling_armor_like_cpp(
                &stat_entry.resistances,
                scaling_context,
            );
            planned_actions.extend(
                item_resistance_bonus_actions_like_cpp(&resistances, apply)
                    .into_iter()
                    .map(|action| RepresentedItemBonusActionLikeCpp {
                        item_guid,
                        slot,
                        action,
                    }),
            );
        }

        if let Some(action) =
            self.item_shield_block_value_like_cpp(item_entry)
                .and_then(|shield_block_value| {
                    item_shield_block_bonus_action_like_cpp(shield_block_value, true, apply)
                })
        {
            planned_actions.push(RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot,
                action,
            });
        }

        if let (Some(weapon), Some(inventory_type)) = (
            item_stats_store.weapon_template(item_entry),
            self.represented_item_inventory_type_like_cpp(item_entry, item_guid),
        ) {
            let (min_damage, max_damage) =
                self.represented_weapon_damage_bounds_like_cpp(item_entry, weapon);
            // C++ `Player::_ApplyWeaponDamage` (`Player.cpp:7979-8020`) skips the
            // disarm gate in feral form and keeps the existing attack time while
            // the active form carries a `CombatRoundTime`.
            let is_in_feral_form = self
                .canonical_player_snapshot_like_cpp(|player| player.is_in_feral_form_like_cpp())
                .unwrap_or(false);
            // C++ reaches `_ApplyWeaponDamage` for any unit that is not
            // disarmed; an unavailable canonical owner is treated as unflagged.
            let can_use_attack_type = self
                .represented_can_use_attack_type_like_cpp(slot, Some(inventory_type))
                != Some(false);
            let has_shapeshift_combat_round_time = self
                .represented_shapeshift_combat_round_time_like_cpp()
                .is_some();
            planned_actions.extend(
                item_weapon_damage_actions_like_cpp(
                    slot,
                    inventory_type,
                    min_damage,
                    max_damage,
                    weapon.item_delay,
                    apply,
                    is_in_feral_form,
                    can_use_attack_type,
                    has_shapeshift_combat_round_time,
                    true,
                )
                .into_iter()
                .map(|action| RepresentedItemBonusActionLikeCpp {
                    item_guid,
                    slot,
                    action,
                }),
            );
        }

        #[cfg(test)]
        self.player_item_test_fixture_like_cpp
            .represented_item_bonus_actions_like_cpp
            .extend(planned_actions.iter().cloned());
        let action_count = planned_actions.len();
        for planned in planned_actions {
            self.apply_represented_item_bonus_action_state_like_cpp(planned.action);
        }
        action_count
    }
}
