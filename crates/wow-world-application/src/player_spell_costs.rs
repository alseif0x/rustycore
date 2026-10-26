//! Cross-domain Source `TakePower`/`TakeReagents`/cooldown orchestration.
//!
//! Cost and cooldown rules remain in `wow_world_spell`; the canonical Player
//! item, currency and SpellHistory owners remain in Entities. This operation
//! sequences those owners with the existing inventory and quest participants
//! before the execution host publishes SpellGo.

use wow_core::ObjectGuid;
use wow_entities::SpellCastMetadata;
use wow_world_inventory::ItemSessionHostLikeCpp;
use wow_world_quest::{QuestItemStorageHostLikeCpp, RepresentedQuestObjectiveProgressEventLikeCpp};
use wow_world_spell::{
    PhysicalCastItemLikeCpp, ServerNormalCastCostFactsLikeCpp,
    ServerNormalCastItemUsePlanResultLikeCpp, ServerNormalCooldownFactsLikeCpp,
    ServerNormalCooldownMutationLikeCpp, ServerNormalPreGoFailureLikeCpp,
    ServerNormalPreGoPlanLikeCpp, SpellExecutionHostLikeCpp,
};

fn item_obtain_spell_ids_for_removal_like_cpp(
    item_entry: u32,
    item_flags: Option<wow_constants::ItemFlags>,
    item_effects: Option<&wow_data::ItemEffectStore>,
) -> Result<Vec<i32>, &'static str> {
    let item_flags = item_flags.ok_or("item template flags unavailable before item removal")?;
    if item_flags.contains(wow_constants::ItemFlags::LEGACY) {
        return Ok(Vec::new());
    }
    let item_effects = item_effects.ok_or("ItemEffect catalog unavailable before item removal")?;
    Ok(item_effects
        .item_effects_for_item_id_like_cpp(item_entry)
        .into_iter()
        .filter(|effect| {
            effect.trigger_type == wow_constants::ItemSpelltriggerType::OnPickup as i8
                && effect.spell_id > 0
        })
        .map(|effect| effect.spell_id)
        .collect())
}

fn item_looted_spell_id_for_removal_like_cpp(
    item_entry: u32,
    item_flags: Option<wow_constants::ItemFlags>,
    item_effects: Option<&wow_data::ItemEffectStore>,
) -> Result<Option<i32>, &'static str> {
    let item_flags = item_flags.ok_or("item template flags unavailable before item removal")?;
    if item_flags.contains(wow_constants::ItemFlags::LEGACY) {
        return Ok(None);
    }
    let item_effects = item_effects.ok_or("ItemEffect catalog unavailable before item removal")?;
    Ok(item_effects
        .item_effects_for_item_id_like_cpp(item_entry)
        .into_iter()
        .find(|effect| effect.trigger_type == wow_constants::ItemSpelltriggerType::OnLooted as i8)
        .map(|effect| effect.spell_id))
}

fn item_obtain_spell_ids_from_host_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &H,
    item_entry: u32,
) -> Result<Vec<i32>, &'static str> {
    let hub = host.hub_ref();
    let spell_ids = item_obtain_spell_ids_for_removal_like_cpp(
        item_entry,
        hub.catalogs.item_template_flags(item_entry),
        hub.catalogs.items.effect_store.as_deref(),
    )?;
    if !spell_ids.is_empty() {
        if hub.player_aura_authority_ready_like_cpp() != Some(true) {
            return Err(
                "complete untombstoned Player aura authority unavailable before item removal",
            );
        }
        if hub.resolved_player_visible_auras_like_cpp().is_none() {
            return Err("Player aura applications unavailable before item removal");
        }
    }
    Ok(spell_ids)
}

fn remove_item_obtain_auras_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    item_entry: u32,
) -> Result<(), &'static str> {
    let spell_ids = item_obtain_spell_ids_from_host_like_cpp(host, item_entry)?;
    remove_item_obtain_aura_ids_like_cpp(spell_ids, |spell_id| {
        wow_world_spell::remove_represented_auras_due_to_spell_like_cpp(host, spell_id)
    });
    Ok(())
}

fn remove_item_looted_aura_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    item_entry: u32,
    item_guid: ObjectGuid,
) -> Result<(), &'static str> {
    let hub = host.hub_ref();
    let spell_id = item_looted_spell_id_for_removal_like_cpp(
        item_entry,
        hub.catalogs.item_template_flags(item_entry),
        hub.catalogs.items.effect_store.as_deref(),
    )?;
    let Some(spell_id) = spell_id else {
        return Ok(());
    };
    if hub.player_aura_authority_ready_like_cpp() != Some(true)
        || hub.resolved_player_visible_auras_like_cpp().is_none()
    {
        return Err(
            "complete untombstoned Player aura authority unavailable before looted-item removal",
        );
    }
    wow_world_spell::remove_represented_auras_due_to_item_spell_like_cpp(host, spell_id, item_guid)
        .map(|_| ())
}

fn remove_item_obtain_aura_ids_like_cpp(
    spell_ids: impl IntoIterator<Item = i32>,
    mut remove_spell_auras: impl FnMut(i32) -> usize,
) -> usize {
    spell_ids
        .into_iter()
        .map(|spell_id| remove_spell_auras(spell_id))
        .sum()
}

pub fn server_normal_cast_cost_facts_like_cpp<'a, H>(
    host: &'a H,
    spell: &'a wow_data::SpellInfo,
    metadata: SpellCastMetadata,
) -> ServerNormalCastCostFactsLikeCpp<'a>
where
    H: SpellExecutionHostLikeCpp,
{
    let hub = host.hub_ref();
    let spell_store = hub.catalogs.spell_store().map(AsRef::as_ref);
    let costs = spell_store.and_then(|store| store.effective_cast_costs_like_cpp());
    let reagent_cost = costs.and_then(|costs| costs.reagent_for_spell_like_cpp(spell.spell_id));
    let currencies = costs
        .map(|costs| costs.currencies_for_spell_like_cpp(spell.spell_id))
        .unwrap_or_default();
    let has_reagent = reagent_cost.is_some_and(|cost| cost.reagent.iter().any(|entry| *entry > 0));
    let has_currency = currencies.iter().any(|cost| cost.currency_count != 0);
    let difficulty_store = hub.catalogs.difficulty_store().map(AsRef::as_ref);
    let native = hub.core.with_owned_player_like_cpp(|player| {
        let physical_item = metadata.cast_item_guid.and_then(|guid| {
            player
                .inventory_runtime_like_cpp()
                .item_objects()
                .get(&guid)
                .filter(|item| {
                    item.owner_guid() == player.guid()
                        && !item.is_in_trade()
                        && item.update_state() != wow_constants::ItemUpdateState::Removed
                })
                .map(|item| PhysicalCastItemLikeCpp {
                    guid,
                    entry_id: item.object().entry(),
                    spell_charges: item.data().spell_charges,
                })
        });
        let reagent_counts = has_reagent
            .then(|| {
                player.item_counts_for_location_like_cpp(
                    wow_entities::ItemSearchLocation::EQUIPMENT
                        | wow_entities::ItemSearchLocation::INVENTORY
                        | wow_entities::ItemSearchLocation::REAGENT_BANK,
                )
            })
            .flatten();
        let currency_quantities =
            (has_currency && player.gameplay_state().currencies_loaded_like_cpp).then(|| {
                player
                    .gameplay_state()
                    .currencies
                    .iter()
                    .map(|(&id, currency)| (id, currency.quantity))
                    .collect()
            });
        (
            physical_item,
            player
                .inventory_runtime_like_cpp()
                .equipment_inventory_authority_complete_like_cpp(),
            player.no_reagent_cost_mask_like_cpp(),
            player.unit().unit_flags_like_cpp(),
            reagent_counts,
            currency_quantities,
            player.gameplay_state().currencies_loaded_like_cpp,
        )
    });
    let physical_item_state_complete = native.is_some();
    let (
        physical_item,
        inventory_authority_complete,
        no_reagent_cost_mask,
        unit_flags,
        reagent_counts,
        currency_quantities,
        currency_authority_complete,
    ) = native.unwrap_or((
        None,
        false,
        [0; 4],
        wow_constants::UnitFlags::empty(),
        None,
        None,
        false,
    ));
    let cast_item_no_reagent_cost = physical_item.and_then(|item| {
        hub.catalogs
            .item_template_flags(item.entry_id)
            .map(|flags| flags.contains(wow_constants::ItemFlags::NO_REAGENT_COST))
    });
    let no_reagent_while_preparing = metadata
        .actual_spell_difficulty_id_like_cpp
        .zip(spell_store)
        .is_some_and(|(actual_difficulty_id, store)| {
            store.has_attribute_for_difficulty_like_cpp(
                spell.spell_id,
                actual_difficulty_id,
                difficulty_store,
                5,
                wow_data::spell::attributes::SPELL_ATTR5_NO_REAGENT_COST_WITH_AURA,
            ) && unit_flags.contains(wow_constants::UnitFlags::PREPARATION)
        });

    ServerNormalCastCostFactsLikeCpp {
        spell,
        metadata,
        costs,
        physical_item,
        physical_item_state_complete,
        inventory_authority_complete,
        item_effect_store: hub.catalogs.items.effect_store.as_deref(),
        cast_item_no_reagent_cost,
        no_reagent_while_preparing,
        no_reagent_aura_authority_complete: hub.player_aura_authority_ready_like_cpp()
            == Some(true),
        no_reagent_cost_mask,
        spell_store,
        spell_categories: hub.catalogs.spell_catalogs.spell_category_store.as_deref(),
        class_options: hub.catalogs.spell_class_options_store().map(AsRef::as_ref),
        difficulties: difficulty_store,
        reagent_counts,
        currency_quantities,
        currency_authority_complete: currency_authority_complete
            && hub.catalogs.currency_types_store().is_some(),
    }
}

pub fn server_normal_cooldown_facts_like_cpp<'a, H>(
    host: &'a H,
    spell: &'a wow_data::SpellInfo,
    metadata: SpellCastMetadata,
) -> Option<ServerNormalCooldownFactsLikeCpp<'a>>
where
    H: SpellExecutionHostLikeCpp,
{
    let hub = host.hub_ref();
    let spell_store = hub.catalogs.spell_store()?;
    let cast_item_is_potion = match metadata.cast_item_entry {
        Some(item_id) => {
            let item = hub.catalogs.items.store.as_ref()?.get(item_id)?;
            Some(item.class_id == 0 && item.subclass_id == 1)
        }
        None => Some(false),
    };
    Some(ServerNormalCooldownFactsLikeCpp {
        spell,
        metadata,
        spell_store: Some(spell_store),
        difficulties: hub.catalogs.difficulty_store().map(AsRef::as_ref),
        item_effect_store: hub.catalogs.items.effect_store.as_deref(),
        spell_categories: hub.catalogs.spell_catalogs.spell_category_store.as_deref(),
        cast_item_is_potion,
    })
}

pub fn apply_server_normal_cooldown_at_cast_start_like_cpp<H>(
    host: &mut H,
    spell: &wow_data::SpellInfo,
    metadata: SpellCastMetadata,
) -> Option<()>
where
    H: SpellExecutionHostLikeCpp,
{
    let plan = wow_world_spell::server_normal_cooldown_plan_like_cpp(
        server_normal_cooldown_facts_like_cpp(host, spell, metadata)?,
    )
    .ok()?;
    if !plan.starts_at_cast_start {
        return Some(());
    }
    let requires_cooldowns = !matches!(plan.mutation, ServerNormalCooldownMutationLikeCpp::None);
    let requires_charges = matches!(
        plan.mutation,
        ServerNormalCooldownMutationLikeCpp::ConsumeCharge { .. }
    );
    let now_ms = wow_world_core::session::unix_now_ms_like_cpp();
    let applied = host.core().with_owned_player_mut_like_cpp(|player| {
        let history = &mut player.unit_mut().subsystems_mut().spells.history;
        if (requires_cooldowns && !history.cooldowns_loaded)
            || (requires_charges && !history.charges_loaded)
        {
            return false;
        }
        wow_world_spell::apply_server_normal_cooldown_plan_like_cpp(history, plan, now_ms)
            || !requires_cooldowns
    })?;
    applied.then_some(())
}

pub fn server_normal_pre_go_like_cpp<H>(
    host: &mut H,
    spell_info: &wow_data::SpellInfo,
    cast_id: ObjectGuid,
    visual: &wow_packet::packets::spell::SpellCastVisual,
    metadata: SpellCastMetadata,
) -> Result<ServerNormalPreGoPlanLikeCpp, ServerNormalPreGoFailureLikeCpp>
where
    H: SpellExecutionHostLikeCpp + ItemSessionHostLikeCpp + QuestItemStorageHostLikeCpp,
{
    let cost_facts = server_normal_cast_cost_facts_like_cpp(host, spell_info, metadata);
    let cost_plan = wow_world_spell::server_normal_cost_plan_like_cpp(cost_facts.clone()).map_err(
        |failure| match failure {
            wow_world_spell::ServerNormalCostResultLikeCpp::MissingReagent(entry_id) => {
                ServerNormalPreGoFailureLikeCpp::MissingReagent(entry_id)
            }
            wow_world_spell::ServerNormalCostResultLikeCpp::MissingCurrency(currency_id) => {
                ServerNormalPreGoFailureLikeCpp::MissingCurrency(currency_id)
            }
            wow_world_spell::ServerNormalCostResultLikeCpp::Affordable => {
                ServerNormalPreGoFailureLikeCpp::Unavailable(
                    "cost planner returned an invalid affordable error",
                )
            }
            wow_world_spell::ServerNormalCostResultLikeCpp::Unavailable(reason) => {
                ServerNormalPreGoFailureLikeCpp::Unavailable(reason)
            }
        },
    )?;

    // Validate every required owner before TakePower. The immutable result is
    // deliberately discarded; each live reagent step is replanned after its
    // quest callback so callbacks cannot stale later physical-item facts.
    let resource_preflight = host.core().with_owned_player_like_cpp(|player| {
        player.plan_spell_cast_resource_debits_like_cpp(
            &cost_plan.reagent_debits,
            &cost_plan.currency_debits,
        )
    });
    let resource_preflight =
        resource_preflight
            .flatten()
            .ok_or(ServerNormalPreGoFailureLikeCpp::Unavailable(
                "canonical native inventory or currency debit authority unavailable",
            ))?;
    for action in resource_preflight.item_actions() {
        if action.destroys_stack() {
            item_obtain_spell_ids_from_host_like_cpp(host, action.entry_id())
                .map_err(ServerNormalPreGoFailureLikeCpp::Unavailable)?;
        }
    }

    let mut committed_metadata = metadata;
    if cost_plan.clears_cast_item_before_go {
        committed_metadata.cast_item_guid = None;
        committed_metadata.cast_item_entry = None;
    }
    let cooldown_plan = wow_world_spell::server_normal_cooldown_plan_like_cpp(
        server_normal_cooldown_facts_like_cpp(host, spell_info, committed_metadata).ok_or(
            ServerNormalPreGoFailureLikeCpp::Unavailable(
                "normal cooldown catalogs or item classification unavailable",
            ),
        )?,
    )
    .map_err(ServerNormalPreGoFailureLikeCpp::Unavailable)?;

    let cast_item_use = if committed_metadata.cast_item_guid.is_some() {
        let item_guid = committed_metadata
            .cast_item_guid
            .expect("checked physical item GUID");
        let physical_item = host
            .core()
            .with_owned_player_like_cpp(|player| {
                player
                    .inventory_runtime_like_cpp()
                    .item_objects()
                    .get(&item_guid)
                    .filter(|item| {
                        item.owner_guid() == player.guid()
                            && !item.is_in_trade()
                            && item.update_state() != wow_constants::ItemUpdateState::Removed
                            && committed_metadata.cast_item_entry == Some(item.object().entry())
                    })
                    .map(|item| PhysicalCastItemLikeCpp {
                        guid: item_guid,
                        entry_id: item.object().entry(),
                        spell_charges: item.data().spell_charges,
                    })
            })
            .flatten()
            .ok_or(ServerNormalPreGoFailureLikeCpp::Unavailable(
                "physical cast item owner or identity changed before GO",
            ))?;
        let item_catalogs = host.hub_ref().catalogs;
        let max_stack_size = item_catalogs
            .item_storage_template(physical_item.entry_id)
            .map(|template| template.max_stack_size);
        match wow_world_spell::server_normal_cast_item_use_plan_like_cpp(
            Some(physical_item),
            item_catalogs.items.effect_store.as_deref(),
            max_stack_size,
            committed_metadata.trigger_cast_flags_like_cpp,
        ) {
            ServerNormalCastItemUsePlanResultLikeCpp::NoItem => false,
            ServerNormalCastItemUsePlanResultLikeCpp::Planned(plan) => {
                let count = host
                    .core()
                    .with_owned_player_like_cpp(|player| {
                        player
                            .inventory_runtime_like_cpp()
                            .item_objects()
                            .get(&item_guid)
                            .map(|item| item.count())
                    })
                    .flatten();
                if plan.consumes_one_item && count == Some(1) {
                    item_obtain_spell_ids_from_host_like_cpp(host, physical_item.entry_id)
                        .map_err(ServerNormalPreGoFailureLikeCpp::Unavailable)?;
                }
                true
            }
            ServerNormalCastItemUsePlanResultLikeCpp::Unavailable(reason) => {
                return Err(ServerNormalPreGoFailureLikeCpp::Unavailable(reason));
            }
        }
    } else {
        false
    };

    let take_cast_item_before_effects = if cast_item_use {
        let hub = host.hub_ref();
        let store =
            hub.catalogs
                .spell_store()
                .ok_or(ServerNormalPreGoFailureLikeCpp::Unavailable(
                    "SpellStore unavailable for cast item hit-delay phase",
                ))?;
        let actual_difficulty_id = committed_metadata
            .actual_spell_difficulty_id_like_cpp
            .ok_or(ServerNormalPreGoFailureLikeCpp::Unavailable(
                "actual SpellInfo key unavailable for cast item hit-delay phase",
            ))?;
        let difficulties = hub.catalogs.difficulty_store().map(AsRef::as_ref);
        let hit_delay = store
            .has_hit_delay_at_difficulty_like_cpp(
                spell_info.spell_id as u32,
                actual_difficulty_id,
                difficulties,
            )
            .map_err(|_| {
                ServerNormalPreGoFailureLikeCpp::Unavailable(
                    "selected SpellMisc hit-delay authority unavailable",
                )
            })?;
        let attributes = store
            .misc_attribute_words_at_difficulty_like_cpp(
                spell_info.spell_id,
                actual_difficulty_id,
                difficulties,
            )
            .map_err(|_| {
                ServerNormalPreGoFailureLikeCpp::Unavailable(
                    "selected SpellMisc channel authority unavailable",
                )
            })?;
        wow_world_spell::server_normal_take_cast_item_before_effects_like_cpp(
            hit_delay,
            attributes[1],
        )
    } else {
        false
    };

    let requires_cooldown_history = !cooldown_plan.starts_at_cast_start
        && !matches!(
            cooldown_plan.mutation,
            ServerNormalCooldownMutationLikeCpp::None
        );
    let requires_charge_history = matches!(
        cooldown_plan.mutation,
        ServerNormalCooldownMutationLikeCpp::ConsumeCharge { .. }
    );
    let history_ready = host.core().with_owned_player_like_cpp(|player| {
        let history = &player.unit().subsystems().spells.history;
        (!requires_cooldown_history || history.cooldowns_loaded)
            && (!requires_charge_history || history.charges_loaded)
    });
    if history_ready != Some(true) {
        return Err(ServerNormalPreGoFailureLikeCpp::Unavailable(
            "native SpellHistory cooldown or charge ledger unavailable before GO",
        ));
    }

    let skips_power_and_reagents = committed_metadata.trigger_cast_flags_like_cpp
        & wow_entities::TRIGGERED_IGNORE_POWER_AND_REAGENT_COST_LIKE_CPP
        != 0;
    if !skips_power_and_reagents
        && metadata.cast_item_guid.is_none()
        && !wow_world_spell::take_spell_power_like_cpp(
            host,
            spell_info,
            cast_id,
            spell_info.spell_id,
            visual,
        )
    {
        return Err(ServerNormalPreGoFailureLikeCpp::Power);
    }

    for &(entry_id, amount) in &cost_plan.reagent_debits {
        let mut remaining = amount;
        while remaining != 0 {
            let action = host
                .core()
                .with_owned_player_like_cpp(|player| {
                    player.plan_spell_cast_reagent_step_like_cpp(entry_id, remaining)
                })
                .flatten()
                .ok_or(ServerNormalPreGoFailureLikeCpp::Unavailable(
                    "live reagent stack changed after pre-GO validation",
                ))?;
            if action.destroys_stack() {
                remove_item_obtain_auras_like_cpp(host, action.entry_id())
                    .map_err(ServerNormalPreGoFailureLikeCpp::Unavailable)?;
                remove_item_looted_aura_like_cpp(host, action.entry_id(), action.item_guid())
                    .map_err(ServerNormalPreGoFailureLikeCpp::Unavailable)?;
            }
            host.core()
                .with_owned_player_mut_like_cpp(|player| {
                    player.apply_spell_cast_reagent_step_like_cpp(action)
                })
                .flatten()
                .ok_or(ServerNormalPreGoFailureLikeCpp::Unavailable(
                    "canonical reagent stack changed before debit",
                ))?;

            if action.destroys_stack() {
                let _ = host.apply_inventory_item_remove_side_effects_like_cpp(
                    action.bag(),
                    action.slot(),
                    action.item_guid(),
                    &[],
                );
            }
            let _ = host.apply_quest_item_removed_like_cpp(entry_id);
            if action.destroys_stack() {
                wow_world_inventory::VendorSessionHostLikeCpp::sync_player_registry_state_like_cpp(
                    host,
                );
            }
            publish_spell_cost_item_step_like_cpp(host, action);
            remaining -= action.removed_count();
        }
    }

    for &(currency_id, amount) in &cost_plan.currency_debits {
        let action = host
            .core()
            .with_owned_player_like_cpp(|player| {
                player.plan_spell_cast_currency_debit_like_cpp(currency_id, amount)
            })
            .flatten()
            .ok_or(ServerNormalPreGoFailureLikeCpp::Unavailable(
                "live currency changed after pre-GO validation",
            ))?;
        let change = i32::try_from(action.amount()).map_err(|_| {
            ServerNormalPreGoFailureLikeCpp::Unavailable(
                "currency debit exceeds the represented quest event range",
            )
        })?;
        host.core()
            .with_owned_player_mut_like_cpp(|player| {
                player.apply_spell_cast_currency_debit_like_cpp(action)
            })
            .flatten()
            .ok_or(ServerNormalPreGoFailureLikeCpp::Unavailable(
                "canonical currency changed before debit",
            ))?;
        let (quest_state, _) = host.split_quest_state_mut();
        quest_state.enqueue_represented_quest_objective_progress_like_cpp(
            RepresentedQuestObjectiveProgressEventLikeCpp::CurrencyChanged {
                currency_id,
                change: -change,
            },
        );
        host.send_packet(&wow_packet::packets::misc::SetCurrency::spell_loss(
            action.currency_id() as i32,
            action.remaining_quantity() as i32,
            action.amount() as i32,
        ));
    }

    if !cooldown_plan.starts_at_cast_start
        && !matches!(
            cooldown_plan.mutation,
            ServerNormalCooldownMutationLikeCpp::None
        )
    {
        let now_ms = wow_world_core::session::unix_now_ms_like_cpp();
        let applied = host.core().with_owned_player_mut_like_cpp(|player| {
            wow_world_spell::apply_server_normal_cooldown_plan_like_cpp(
                &mut player.unit_mut().subsystems_mut().spells.history,
                cooldown_plan,
                now_ms,
            )
        });
        if applied != Some(true) {
            return Err(ServerNormalPreGoFailureLikeCpp::Unavailable(
                "native SpellHistory cooldown mutation failed before GO",
            ));
        }
    }

    Ok(ServerNormalPreGoPlanLikeCpp {
        metadata: committed_metadata,
        take_cast_item: cast_item_use,
        take_cast_item_before_effects,
    })
}

pub fn take_server_normal_cast_item_like_cpp<H>(
    host: &mut H,
    spell_info: &wow_data::SpellInfo,
    metadata: SpellCastMetadata,
) -> bool
where
    H: SpellExecutionHostLikeCpp + ItemSessionHostLikeCpp + QuestItemStorageHostLikeCpp,
{
    let Some(item_guid) = metadata.cast_item_guid else {
        return true;
    };
    let item = host
        .core()
        .with_owned_player_like_cpp(|player| {
            let item = player
                .inventory_runtime_like_cpp()
                .item_objects()
                .get(&item_guid)?;
            (item.owner_guid() == player.guid()
                && !item.is_in_trade()
                && item.update_state() != wow_constants::ItemUpdateState::Removed
                && metadata.cast_item_entry == Some(item.object().entry()))
            .then_some(PhysicalCastItemLikeCpp {
                guid: item_guid,
                entry_id: item.object().entry(),
                spell_charges: item.data().spell_charges,
            })
        })
        .flatten();
    let Some(item) = item else {
        tracing::warn!(item_guid = ?item_guid, "Normal Player cast item disappeared before TakeCastItem");
        return false;
    };
    let catalogs = host.hub_ref().catalogs;
    let max_stack_size = catalogs
        .item_storage_template(item.entry_id)
        .map(|template| template.max_stack_size);
    let plan = match wow_world_spell::server_normal_cast_item_use_plan_like_cpp(
        Some(item),
        catalogs.items.effect_store.as_deref(),
        max_stack_size,
        metadata.trigger_cast_flags_like_cpp,
    ) {
        ServerNormalCastItemUsePlanResultLikeCpp::NoItem => return true,
        ServerNormalCastItemUsePlanResultLikeCpp::Planned(plan) => plan,
        ServerNormalCastItemUsePlanResultLikeCpp::Unavailable(reason) => {
            tracing::warn!(
                spell_id = spell_info.spell_id,
                reason,
                "Normal cast item metadata unavailable at TakeCastItem"
            );
            return false;
        }
    };

    let removal_position = (plan.consumes_one_item
        && host.core().with_owned_player_like_cpp(|player| {
            player
                .inventory_runtime_like_cpp()
                .item_objects()
                .get(&plan.item_guid)
                .is_some_and(|item| item.count() == 1)
        }) == Some(true))
    .then(|| {
        host.core()
            .with_owned_player_like_cpp(|player| {
                player.spell_cast_item_position_like_cpp(plan.item_guid)
            })
            .flatten()
    })
    .flatten();
    if plan.consumes_one_item && removal_position.is_some() {
        if let Err(reason) = remove_item_obtain_auras_like_cpp(host, plan.entry_id) {
            tracing::warn!(item_guid = ?plan.item_guid, reason, "Normal cast item obtain aura removal authority unavailable");
            return false;
        }
        if let Err(reason) = remove_item_looted_aura_like_cpp(host, plan.entry_id, plan.item_guid) {
            tracing::warn!(item_guid = ?plan.item_guid, reason, "Normal cast item looted aura removal authority unavailable");
            return false;
        }
    }
    let outcome = host
        .core()
        .with_owned_player_mut_like_cpp(|player| player.apply_spell_cast_item_use_like_cpp(plan))
        .flatten();
    let Some(outcome) = outcome else {
        tracing::warn!(item_guid = ?plan.item_guid, "Normal Player cast item changed during TakeCastItem");
        return false;
    };
    if outcome.removed_one {
        if outcome.removed_stack
            && let Some((bag, slot)) = removal_position
        {
            let _ = host.apply_inventory_item_remove_side_effects_like_cpp(
                bag,
                slot,
                outcome.item_guid,
                &[],
            );
        }
        let _ = host.apply_quest_item_removed_like_cpp(outcome.entry_id);
        if outcome.removed_stack {
            wow_world_inventory::VendorSessionHostLikeCpp::sync_player_registry_state_like_cpp(
                host,
            );
            host.send_packet(&wow_packet::packets::update::UpdateObject::destroy_objects(
                vec![outcome.item_guid],
                host.core().player_map_id_like_cpp(),
            ));
        } else {
            host.send_packet(
                &wow_packet::packets::update::UpdateObject::item_stack_count_update(
                    outcome.item_guid,
                    host.core().player_map_id_like_cpp(),
                    outcome.remaining_count,
                ),
            );
        }
    }
    true
}

fn publish_spell_cost_item_step_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &H,
    action: wow_entities::PlayerSpellCastItemDebitActionLikeCpp,
) {
    let map_id = host.core().player_map_id_like_cpp();
    if action.destroys_stack() {
        host.send_packet(&wow_packet::packets::update::UpdateObject::destroy_objects(
            vec![action.item_guid()],
            map_id,
        ));
    } else {
        host.send_packet(
            &wow_packet::packets::update::UpdateObject::item_stack_count_update(
                action.item_guid(),
                map_id,
                action.remaining_count(),
            ),
        );
    }
}

#[cfg(test)]
#[path = "../unit_tests/player_spell_costs_tests.rs"]
mod tests;
