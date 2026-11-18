// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item enchantment, sockets and random properties.

use super::super::*;
use super::bonus_actions::{
    get_attack_by_slot, item_mod_type_from_u32, primary_stat_actions, rating_actions, unit_modifier,
};

impl Player {
    pub fn enchant_durations(&self) -> &[PlayerEnchantDuration] {
        &self.enchant_durations
    }

    pub fn add_enchantment_durations(&mut self, item: &mut Item) -> Vec<PlayerEnchantTimeUpdate> {
        let mut updates = Vec::new();
        for slot in ENCHANTMENT_DURATION_SLOTS {
            let enchantment = item.data().enchantments[slot as usize];
            if enchantment.id != 0 && enchantment.duration > 0 {
                if let Some(update) =
                    self.add_enchantment_duration(item, slot, enchantment.duration)
                {
                    updates.push(update);
                }
            }
        }
        updates
    }

    pub fn add_enchantment_duration(
        &mut self,
        item: &mut Item,
        slot: EnchantmentSlot,
        duration_ms: u32,
    ) -> Option<PlayerEnchantTimeUpdate> {
        let item_guid = item.object().guid();
        if let Some(index) = self
            .enchant_durations
            .iter()
            .position(|duration| duration.item_guid == item_guid && duration.slot == slot)
        {
            let old_duration = self.enchant_durations.remove(index);
            item.set_enchantment_duration(slot, old_duration.left_duration_ms);
        }

        if duration_ms == 0 {
            return None;
        }

        self.enchant_durations.push(PlayerEnchantDuration {
            item_guid,
            slot,
            left_duration_ms: duration_ms,
        });
        Some(PlayerEnchantTimeUpdate {
            item_guid,
            slot,
            duration_secs: duration_ms / 1000,
        })
    }

    pub fn remove_enchantment_durations(&mut self, item: &mut Item) -> Vec<PlayerEnchantDuration> {
        let item_guid = item.object().guid();
        let mut removed = Vec::new();
        self.enchant_durations.retain(|duration| {
            if duration.item_guid == item_guid {
                item.set_enchantment_duration(duration.slot, duration.left_duration_ms);
                removed.push(*duration);
                false
            } else {
                true
            }
        });
        removed
    }

    pub fn remove_enchantment_duration_references(
        &mut self,
        item: &Item,
    ) -> Vec<PlayerEnchantDuration> {
        let item_guid = item.object().guid();
        let mut removed = Vec::new();
        self.enchant_durations.retain(|duration| {
            if duration.item_guid == item_guid {
                removed.push(*duration);
                false
            } else {
                true
            }
        });
        removed
    }

    pub fn update_enchant_time(
        &mut self,
        items: &[PlayerEnchantDurationItemRef],
        time_ms: u32,
    ) -> Vec<UpdateEnchantTimeAction> {
        let mut actions = Vec::new();
        let mut kept = Vec::with_capacity(self.enchant_durations.len());
        for mut duration in std::mem::take(&mut self.enchant_durations) {
            let item = items
                .iter()
                .find(|item| item.item_guid == duration.item_guid && item.slot == duration.slot);
            if item.is_none_or(|item| item.enchantment_id == 0) {
                actions.push(UpdateEnchantTimeAction::RemoveMissingEnchantment {
                    item_guid: duration.item_guid,
                    slot: duration.slot,
                });
                continue;
            }
            if duration.left_duration_ms <= time_ms {
                actions.push(UpdateEnchantTimeAction::ClearExpired {
                    item_guid: duration.item_guid,
                    slot: duration.slot,
                });
                continue;
            }

            duration.left_duration_ms -= time_ms;
            kept.push(duration);
        }
        self.enchant_durations = kept;
        actions
    }

    pub fn send_enchantment_durations_plan(&self) -> Vec<PlayerEnchantTimeUpdate> {
        self.enchant_durations
            .iter()
            .map(|duration| PlayerEnchantTimeUpdate {
                item_guid: duration.item_guid,
                slot: duration.slot,
                duration_secs: duration.left_duration_ms / 1000,
            })
            .collect()
    }

    pub fn remove_arena_enchantments(
        &mut self,
        enchantment_slot: EnchantmentSlot,
        items: &[ArenaEnchantmentItemRef],
    ) -> Vec<RemoveArenaEnchantmentAction> {
        let mut actions = Vec::new();
        let mut kept_durations = Vec::with_capacity(self.enchant_durations.len());

        for duration in std::mem::take(&mut self.enchant_durations) {
            if duration.slot != enchantment_slot {
                kept_durations.push(duration);
                continue;
            }

            if let Some(item) = arena_enchantment_ref_by_guid(items, duration.item_guid) {
                if item.enchantment_id != 0 && item.arena_allowed {
                    kept_durations.push(duration);
                    continue;
                }
                if item.enchantment_id != 0 {
                    actions.push(RemoveArenaEnchantmentAction::ClearEquippedEnchantment {
                        item_guid: duration.item_guid,
                        enchantment_slot,
                    });
                    continue;
                }
            }

            actions.push(RemoveArenaEnchantmentAction::RemoveDurationReference {
                item_guid: duration.item_guid,
                enchantment_slot,
            });
        }
        self.enchant_durations = kept_durations;

        let inventory_end =
            INVENTORY_SLOT_ITEM_START.saturating_add(self.active_data.num_backpack_slots);
        for slot in INVENTORY_SLOT_ITEM_START..inventory_end {
            if let Some(item_guid) = self.get_item_by_pos(INVENTORY_SLOT_BAG_0, slot) {
                push_arena_inventory_enchantment_action(
                    &mut actions,
                    items,
                    item_guid,
                    INVENTORY_SLOT_BAG_0,
                    slot,
                    enchantment_slot,
                );
            }
        }

        for bag_slot in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
            if let Some(bag) = self
                .inventory
                .bags
                .get(bag_slot as usize)
                .and_then(Option::as_ref)
            {
                for slot in 0..bag.bag_size {
                    if let Some(item_guid) = bag.item_by_pos(slot) {
                        push_arena_inventory_enchantment_action(
                            &mut actions,
                            items,
                            item_guid,
                            bag_slot,
                            slot,
                            enchantment_slot,
                        );
                    }
                }
            }
        }

        actions
    }

    pub fn apply_enchantment_plan(
        &mut self,
        item: Option<&mut Item>,
        slot: EnchantmentSlot,
        enchantment: Option<ApplyEnchantmentTemplateRef>,
        args: ApplyEnchantmentArgs,
    ) -> ApplyEnchantmentPlan {
        let Some(item) = item else {
            return ApplyEnchantmentPlan {
                result: ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::MissingItem),
            };
        };
        if !item.is_equipped() {
            return ApplyEnchantmentPlan {
                result: ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::NotEquipped),
            };
        }

        let enchantment_id = item.data().enchantments[slot as usize].id;
        if enchantment_id == 0 {
            return ApplyEnchantmentPlan {
                result: ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::NoEnchantment),
            };
        }

        let Some(enchantment) =
            enchantment.filter(|enchantment| enchantment.enchantment_id == enchantment_id)
        else {
            return ApplyEnchantmentPlan {
                result: ApplyEnchantmentResult::Skipped(
                    ApplyEnchantmentSkipReason::MissingEnchantmentTemplate,
                ),
            };
        };

        if !args.ignore_condition && enchantment.condition_id != 0 && !enchantment.condition_fits {
            return ApplyEnchantmentPlan {
                result: ApplyEnchantmentResult::Skipped(
                    ApplyEnchantmentSkipReason::ConditionFailed,
                ),
            };
        }
        if i32::from(enchantment.min_level) > self.unit.data().level {
            return ApplyEnchantmentPlan {
                result: ApplyEnchantmentResult::Skipped(
                    ApplyEnchantmentSkipReason::PlayerLevelTooLow,
                ),
            };
        }
        if !enchantment.skill_fits() {
            return ApplyEnchantmentPlan {
                result: ApplyEnchantmentResult::Skipped(
                    ApplyEnchantmentSkipReason::RequiredSkillTooLow,
                ),
            };
        }

        if is_socket_enchantment_slot(slot) {
            if let Some(socket_context) = args.socket_context {
                if socket_context.socket_color == 0 {
                    let Some(prismatic_enchantment) = socket_context.prismatic_enchantment else {
                        return ApplyEnchantmentPlan {
                            result: ApplyEnchantmentResult::Skipped(
                                ApplyEnchantmentSkipReason::MissingPrismaticEnchantment,
                            ),
                        };
                    };
                    if !prismatic_enchantment.skill_fits() {
                        return ApplyEnchantmentPlan {
                            result: ApplyEnchantmentResult::Skipped(
                                ApplyEnchantmentSkipReason::PrismaticRequiredSkillTooLow,
                            ),
                        };
                    }
                }

                if let Some(gem_requirement) = socket_context.gem_requirement {
                    if !gem_requirement.skill_fits() {
                        return ApplyEnchantmentPlan {
                            result: ApplyEnchantmentResult::Skipped(
                                ApplyEnchantmentSkipReason::GemRequiredSkillTooLow,
                            ),
                        };
                    }
                }
            }
        }

        let item_guid = item.object().guid();
        let mut duration_action = None;
        if args.apply_dur {
            if args.apply {
                let duration_ms = item.data().enchantments[slot as usize].duration;
                if duration_ms > 0 {
                    duration_action = self
                        .add_enchantment_duration(item, slot, duration_ms)
                        .map(ApplyEnchantmentDurationAction::Added);
                }
            } else {
                let had_duration = self
                    .enchant_durations
                    .iter()
                    .any(|duration| duration.item_guid == item_guid && duration.slot == slot);
                self.add_enchantment_duration(item, slot, 0);
                if had_duration {
                    duration_action =
                        Some(ApplyEnchantmentDurationAction::Removed { item_guid, slot });
                }
            }
        }

        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Applied {
                item_guid,
                slot,
                enchantment_id,
                apply: args.apply,
                effects_allowed: !item.is_broken(),
                update_permanent_visible_item: slot == EnchantmentSlot::EnhancementPermanent,
                duration_action,
            },
        }
    }

    pub fn apply_enchantment_effect_actions(
        &self,
        item: &Item,
        item_template: Option<&ItemStorageTemplate>,
        enchantment_slot: EnchantmentSlot,
        apply: bool,
        effects: &[ApplyEnchantmentEffectRef],
    ) -> Vec<ApplyEnchantmentEffectAction> {
        self.apply_enchantment_effect_actions_for_enchantment(
            item,
            item_template,
            enchantment_slot,
            0,
            None,
            apply,
            effects,
        )
    }

    pub fn apply_enchantment_effect_actions_for_enchantment(
        &self,
        item: &Item,
        item_template: Option<&ItemStorageTemplate>,
        enchantment_slot: EnchantmentSlot,
        enchantment_id: i32,
        random_suffix: Option<ApplyEnchantmentRandomSuffixRef>,
        apply: bool,
        effects: &[ApplyEnchantmentEffectRef],
    ) -> Vec<ApplyEnchantmentEffectAction> {
        if item.is_broken() {
            return Vec::new();
        }

        effects
            .iter()
            .flat_map(|effect| {
                apply_enchantment_effect_action(
                    item,
                    item_template,
                    enchantment_slot,
                    enchantment_id,
                    random_suffix,
                    apply,
                    *effect,
                )
            })
            .collect()
    }

    pub fn update_skill_enchantments_plan(
        &self,
        skill_id: u16,
        curr_value: u16,
        new_value: u16,
        items: &[SkillEnchantmentItemRef],
        enchantments: &[SkillEnchantmentTemplateRef],
    ) -> Vec<UpdateSkillEnchantmentAction> {
        let mut actions = Vec::new();

        for inventory_slot in 0..INVENTORY_SLOT_BAG_END {
            let Some(item) = items
                .iter()
                .find(|item| item.inventory_slot == inventory_slot)
                .copied()
            else {
                continue;
            };

            for (slot_index, enchantment_slot) in ENCHANTMENT_DURATION_SLOTS.iter().enumerate() {
                let enchantment_id = item.enchantment_ids[slot_index];
                if enchantment_id == 0 {
                    continue;
                }

                let Some(enchantment) = enchantments
                    .iter()
                    .find(|enchantment| enchantment.enchantment_id == enchantment_id)
                else {
                    actions.push(
                        UpdateSkillEnchantmentAction::MissingEnchantmentTemplateAbort {
                            item_guid: item.item_guid,
                            inventory_slot: item.inventory_slot,
                            enchantment_slot: *enchantment_slot,
                            enchantment_id,
                        },
                    );
                    return actions;
                };

                if enchantment.required_skill_id == skill_id {
                    if let Some(apply) = skill_enchantment_transition(
                        curr_value,
                        new_value,
                        enchantment.required_skill_rank,
                    ) {
                        push_update_skill_enchantment_action(
                            &mut actions,
                            item,
                            *enchantment_slot,
                            enchantment_id,
                            UpdateSkillEnchantmentReason::EnchantmentRequiredSkill,
                            apply,
                        );
                    }
                }

                if is_socket_enchantment_slot(*enchantment_slot)
                    && item.socket_colors[slot_index - EnchantmentSlot::EnhancementSocket as usize]
                        == 0
                {
                    let prismatic_enchantment_id =
                        item.enchantment_ids[EnchantmentSlot::EnhancementSocketPrismatic as usize];
                    let Some(prismatic_enchantment) = enchantments
                        .iter()
                        .find(|enchantment| enchantment.enchantment_id == prismatic_enchantment_id)
                    else {
                        continue;
                    };

                    if prismatic_enchantment.required_skill_id == skill_id {
                        if let Some(apply) = skill_enchantment_transition(
                            curr_value,
                            new_value,
                            prismatic_enchantment.required_skill_rank,
                        ) {
                            push_update_skill_enchantment_action(
                                &mut actions,
                                item,
                                *enchantment_slot,
                                enchantment_id,
                                UpdateSkillEnchantmentReason::PrismaticRequiredSkill,
                                apply,
                            );
                        }
                    }
                }
            }
        }

        actions
    }
}

const ENCHANTMENT_DURATION_SLOTS: [EnchantmentSlot; MAX_ENCHANTMENT_SLOT] = [
    EnchantmentSlot::EnhancementPermanent,
    EnchantmentSlot::EnhancementTemporary,
    EnchantmentSlot::EnhancementSocket,
    EnchantmentSlot::EnhancementSocket2,
    EnchantmentSlot::EnhancementSocket3,
    EnchantmentSlot::EnhancementSocketBonus,
    EnchantmentSlot::EnhancementSocketPrismatic,
    EnchantmentSlot::EnhancementUse,
    EnchantmentSlot::Property0,
    EnchantmentSlot::Property1,
    EnchantmentSlot::Property2,
    EnchantmentSlot::Property3,
    EnchantmentSlot::Property4,
];

fn arena_enchantment_ref_by_guid(
    items: &[ArenaEnchantmentItemRef],
    guid: ObjectGuid,
) -> Option<ArenaEnchantmentItemRef> {
    items.iter().find(|item| item.guid == guid).copied()
}

fn push_arena_inventory_enchantment_action(
    actions: &mut Vec<RemoveArenaEnchantmentAction>,
    items: &[ArenaEnchantmentItemRef],
    item_guid: ObjectGuid,
    bag: u8,
    slot: u8,
    enchantment_slot: EnchantmentSlot,
) {
    match arena_enchantment_ref_by_guid(items, item_guid) {
        Some(item) if item.arena_allowed => {}
        Some(_) => actions.push(RemoveArenaEnchantmentAction::ClearInventoryEnchantment {
            item_guid,
            bag,
            slot,
            enchantment_slot,
        }),
        None => actions.push(RemoveArenaEnchantmentAction::MissingInventoryItemRef {
            item_guid,
            bag,
            slot,
            enchantment_slot,
        }),
    }
}

const fn is_socket_enchantment_slot(slot: EnchantmentSlot) -> bool {
    matches!(
        slot,
        EnchantmentSlot::EnhancementSocket
            | EnchantmentSlot::EnhancementSocket2
            | EnchantmentSlot::EnhancementSocket3
    )
}

fn apply_enchantment_effect_action(
    item: &Item,
    item_template: Option<&ItemStorageTemplate>,
    enchantment_slot: EnchantmentSlot,
    enchantment_id: i32,
    random_suffix: Option<ApplyEnchantmentRandomSuffixRef>,
    apply: bool,
    effect: ApplyEnchantmentEffectRef,
) -> Vec<ApplyEnchantmentEffectAction> {
    match effect.effect_kind {
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::None) => {
            vec![ApplyEnchantmentEffectAction::Noop]
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::CombatSpell) => {
            vec![ApplyEnchantmentEffectAction::DeferredCombatSpell]
        }
        ApplyEnchantmentEffectKind::Known(
            kind @ (ItemEnchantmentType::Damage | ItemEnchantmentType::Totem),
        ) => {
            let Some(template) = item_template else {
                return vec![ApplyEnchantmentEffectAction::MissingItemTemplateForAttack {
                    effect_kind: ApplyEnchantmentEffectKind::Known(kind),
                }];
            };
            let attack_type = get_attack_by_slot(item.slot(), template.inventory_type);
            if attack_type == WeaponAttackType::Max {
                vec![ApplyEnchantmentEffectAction::Noop]
            } else {
                vec![ApplyEnchantmentEffectAction::UpdateDamageDoneMods {
                    attack_type,
                    modifier_slot: if apply { -1 } else { enchantment_slot as i16 },
                }]
            }
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::EquipSpell) => {
            if effect.arg == 0 {
                vec![ApplyEnchantmentEffectAction::Noop]
            } else if apply {
                vec![ApplyEnchantmentEffectAction::CastEquipSpell {
                    spell_id: effect.arg,
                    item_guid: item.object().guid(),
                }]
            } else {
                vec![ApplyEnchantmentEffectAction::RemoveEquipSpellAura {
                    spell_id: effect.arg,
                    item_guid: item.object().guid(),
                }]
            }
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::Resistance) => {
            let amount =
                resolve_enchantment_effect_amount(item, enchantment_id, random_suffix, effect);
            vec![ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(effect.arg),
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount,
                apply,
            }]
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::Stat) => {
            let amount =
                resolve_enchantment_effect_amount(item, enchantment_id, random_suffix, effect);
            apply_enchantment_stat_actions(item_mod_type_from_u32(effect.arg), amount, apply)
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::UseSpell) => {
            vec![ApplyEnchantmentEffectAction::DeferredUseSpell]
        }
        ApplyEnchantmentEffectKind::Known(
            ItemEnchantmentType::PrismaticSocket
            | ItemEnchantmentType::ArtifactPowerBonusRankByType
            | ItemEnchantmentType::ArtifactPowerBonusRankByID
            | ItemEnchantmentType::BonusListID
            | ItemEnchantmentType::BonusListCurve
            | ItemEnchantmentType::ArtifactPowerBonusRankPicker,
        ) => vec![ApplyEnchantmentEffectAction::Noop],
        ApplyEnchantmentEffectKind::Unknown(effect_type) => {
            vec![ApplyEnchantmentEffectAction::Unknown { effect_type }]
        }
    }
}

fn resolve_enchantment_effect_amount(
    item: &Item,
    enchantment_id: i32,
    random_suffix: Option<ApplyEnchantmentRandomSuffixRef>,
    effect: ApplyEnchantmentEffectRef,
) -> u32 {
    if effect.amount != 0
        || !matches!(
            effect.effect_kind,
            ApplyEnchantmentEffectKind::Known(
                ItemEnchantmentType::Resistance | ItemEnchantmentType::Stat
            )
        )
    {
        return effect.amount;
    }

    let Some(random_suffix) = random_suffix else {
        return effect.amount;
    };
    if item.data().random_properties_id.unsigned_abs() != random_suffix.id {
        return effect.amount;
    }

    random_suffix
        .amount_for(enchantment_id, item.data().property_seed)
        .unwrap_or(effect.amount)
}

pub(in crate::player) fn apply_enchantment_stat_actions(
    item_mod: ItemModType,
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    match item_mod {
        ItemModType::Mana => vec![unit_modifier(
            ApplyEnchantmentUnitMod::Mana,
            ApplyEnchantmentUnitModifier::BaseValue,
            amount,
            apply,
        )],
        ItemModType::Health => vec![unit_modifier(
            ApplyEnchantmentUnitMod::Health,
            ApplyEnchantmentUnitModifier::BaseValue,
            amount,
            apply,
        )],
        ItemModType::Agility => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatAgility,
            Stats::Agility,
            amount,
            apply,
        ),
        ItemModType::Strength => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatStrength,
            Stats::Strength,
            amount,
            apply,
        ),
        ItemModType::Intellect => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatIntellect,
            Stats::Intellect,
            amount,
            apply,
        ),
        ItemModType::Spirit => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatSpirit,
            Stats::Spirit,
            amount,
            apply,
        ),
        ItemModType::Stamina => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatStamina,
            Stats::Stamina,
            amount,
            apply,
        ),
        ItemModType::DefenseSkillRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::DefenseSkill], amount, apply)
        }
        ItemModType::DodgeRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::Dodge], amount, apply)
        }
        ItemModType::ParryRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::Parry], amount, apply)
        }
        ItemModType::BlockRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::Block], amount, apply)
        }
        ItemModType::HitMeleeRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HitMelee], amount, apply)
        }
        ItemModType::HitRangedRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HitRanged], amount, apply)
        }
        ItemModType::HitSpellRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HitSpell], amount, apply)
        }
        ItemModType::CritMeleeRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::CritMelee], amount, apply)
        }
        ItemModType::CritRangedRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::CritRanged], amount, apply)
        }
        ItemModType::CritSpellRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::CritSpell], amount, apply)
        }
        ItemModType::HasteSpellRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HasteSpell], amount, apply)
        }
        ItemModType::HitRating => rating_actions(
            &[
                ApplyEnchantmentCombatRating::HitMelee,
                ApplyEnchantmentCombatRating::HitRanged,
                ApplyEnchantmentCombatRating::HitSpell,
            ],
            amount,
            apply,
        ),
        ItemModType::CritRating => rating_actions(
            &[
                ApplyEnchantmentCombatRating::CritMelee,
                ApplyEnchantmentCombatRating::CritRanged,
                ApplyEnchantmentCombatRating::CritSpell,
            ],
            amount,
            apply,
        ),
        ItemModType::HasteRating => rating_actions(
            &[
                ApplyEnchantmentCombatRating::HasteMelee,
                ApplyEnchantmentCombatRating::HasteRanged,
                ApplyEnchantmentCombatRating::HasteSpell,
            ],
            amount,
            apply,
        ),
        ItemModType::ExpertiseRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::Expertise], amount, apply)
        }
        ItemModType::AttackPower => vec![
            unit_modifier(
                ApplyEnchantmentUnitMod::AttackPower,
                ApplyEnchantmentUnitModifier::TotalValue,
                amount,
                apply,
            ),
            unit_modifier(
                ApplyEnchantmentUnitMod::AttackPowerRanged,
                ApplyEnchantmentUnitModifier::TotalValue,
                amount,
                apply,
            ),
        ],
        ItemModType::RangedAttackPower => vec![unit_modifier(
            ApplyEnchantmentUnitMod::AttackPowerRanged,
            ApplyEnchantmentUnitModifier::TotalValue,
            amount,
            apply,
        )],
        ItemModType::ManaRegeneration => {
            vec![ApplyEnchantmentEffectAction::ManaRegenBonus { amount, apply }]
        }
        ItemModType::ArmorPenetrationRating => rating_actions(
            &[ApplyEnchantmentCombatRating::ArmorPenetration],
            amount,
            apply,
        ),
        ItemModType::SpellPower => {
            vec![ApplyEnchantmentEffectAction::SpellPowerBonus { amount, apply }]
        }
        ItemModType::HealthRegen => {
            vec![ApplyEnchantmentEffectAction::HealthRegenBonus { amount, apply }]
        }
        ItemModType::SpellPenetration => {
            vec![ApplyEnchantmentEffectAction::SpellPenetrationBonus { amount, apply }]
        }
        ItemModType::BlockValue => vec![ApplyEnchantmentEffectAction::BaseModFlatValue {
            base_mod: ApplyEnchantmentBaseMod::ShieldBlockValue,
            amount,
            apply,
        }],
        _ => vec![ApplyEnchantmentEffectAction::UnhandledStatModifier {
            item_mod,
            amount,
            apply,
        }],
    }
}
