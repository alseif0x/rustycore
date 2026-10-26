//! Native normal Player cast cost and readiness rules.

use std::collections::HashMap;

use wow_data::{
    DifficultyStore, ItemEffectStore, ServersideSpellStoreLikeCpp, SpellCategoryStore,
    SpellClassOptionsStore, SpellInfo, SpellStore,
};
use wow_entities::{Player, SpellCastMetadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerNormalCostResultLikeCpp {
    Affordable,
    MissingReagent(u32),
    MissingCurrency(u16),
    Unavailable(&'static str),
}

#[derive(Clone)]
pub struct ServerNormalCastCostFactsLikeCpp<'a> {
    pub spell: &'a SpellInfo,
    pub metadata: SpellCastMetadata,
    pub costs: Option<&'a wow_data::SpellCastCostStoreLikeCpp>,
    pub physical_item: Option<PhysicalCastItemLikeCpp>,
    pub physical_item_state_complete: bool,
    pub inventory_authority_complete: bool,
    pub item_effect_store: Option<&'a ItemEffectStore>,
    pub cast_item_no_reagent_cost: Option<bool>,
    pub no_reagent_while_preparing: bool,
    pub no_reagent_aura_authority_complete: bool,
    pub no_reagent_cost_mask: [u32; 4],
    pub spell_store: Option<&'a SpellStore>,
    pub spell_categories: Option<&'a SpellCategoryStore>,
    pub class_options: Option<&'a SpellClassOptionsStore>,
    pub difficulties: Option<&'a DifficultyStore>,
    pub reagent_counts: Option<HashMap<u32, u32>>,
    pub currency_quantities: Option<HashMap<u32, u32>>,
    pub currency_authority_complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalCastItemLikeCpp {
    pub guid: wow_core::ObjectGuid,
    pub entry_id: u32,
    pub spell_charges: [i32; wow_entities::MAX_ITEM_SPELLS],
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerNormalCastCostPlanLikeCpp {
    pub reagent_debits: Vec<(u32, u32)>,
    pub currency_debits: Vec<(u32, u32)>,
    pub clears_cast_item_before_go: bool,
}

#[derive(Clone)]
pub struct ServerNormalCooldownFactsLikeCpp<'a> {
    pub spell: &'a SpellInfo,
    pub metadata: SpellCastMetadata,
    pub spell_store: Option<&'a SpellStore>,
    pub difficulties: Option<&'a DifficultyStore>,
    pub item_effect_store: Option<&'a ItemEffectStore>,
    pub spell_categories: Option<&'a SpellCategoryStore>,
    /// `None` is unavailable item-template classification when a physical item
    /// is present; it is not evidence that the item is not a potion.
    pub cast_item_is_potion: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerNormalCooldownMutationLikeCpp {
    None,
    ConsumeCharge {
        charge_category_id: u32,
        recovery_ms: u32,
        max_charges: i32,
    },
    StartCooldown {
        spell_id: u32,
        item_id: u32,
        cooldown_ms: u32,
        category_id: u32,
        category_cooldown_ms: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerNormalCooldownPlanLikeCpp {
    pub mutation: ServerNormalCooldownMutationLikeCpp,
    pub starts_at_cast_start: bool,
}

pub type ServerNormalCastItemUsePlanLikeCpp = wow_entities::PlayerSpellCastItemUsePlanLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerNormalCastItemUsePlanResultLikeCpp {
    NoItem,
    Planned(ServerNormalCastItemUsePlanLikeCpp),
    Unavailable(&'static str),
}

const SPELL_ATTR0_PASSIVE_LIKE_CPP: u32 = 0x0000_0040;
const SPELL_ATTR0_COOLDOWN_ON_EVENT_LIKE_CPP: u32 = 0x0200_0000;
const SPELL_ATTR12_START_COOLDOWN_ON_CAST_START_LIKE_CPP: u32 = 0x0080_0000;
const SPELL_CATEGORY_FLAG_COOLDOWN_STARTS_ON_EVENT_LIKE_CPP: i32 = 0x04;
const SPELL_ATTR1_IS_CHANNELLED_LIKE_CPP: u32 = 0x0000_0004;
const SPELL_ATTR1_IS_SELF_CHANNELLED_LIKE_CPP: u32 = 0x0000_0040;

pub fn server_normal_cooldown_plan_like_cpp(
    facts: ServerNormalCooldownFactsLikeCpp<'_>,
) -> Result<ServerNormalCooldownPlanLikeCpp, &'static str> {
    if facts.metadata.trigger_cast_flags_like_cpp
        & wow_entities::TRIGGERED_IGNORE_SPELL_AND_CATEGORY_CD_LIKE_CPP
        != 0
    {
        return Ok(ServerNormalCooldownPlanLikeCpp {
            mutation: ServerNormalCooldownMutationLikeCpp::None,
            starts_at_cast_start: false,
        });
    }
    let actual_difficulty_id = facts
        .metadata
        .actual_spell_difficulty_id_like_cpp
        .ok_or("actual SpellInfo key missing for cooldown")?;
    let spell_store = facts
        .spell_store
        .ok_or("SpellStore unavailable for cooldown")?;
    let categories = facts
        .spell_categories
        .ok_or("SpellCategory catalog unavailable for cooldown")?;
    let attributes = spell_store
        .misc_attribute_words_at_difficulty_like_cpp(
            facts.spell.spell_id,
            actual_difficulty_id,
            facts.difficulties,
        )
        .map_err(|_| "selected SpellMisc attributes unavailable for cooldown")?;
    let starts_at_cast_start =
        attributes[12] & SPELL_ATTR12_START_COOLDOWN_ON_CAST_START_LIKE_CPP != 0;
    let spell_id = facts.spell.spell_id as u32;
    let base_category_id = spell_store
        .category_id_at_difficulty(
            facts.spell.spell_id as u32,
            actual_difficulty_id,
            facts.difficulties,
        )
        .map_err(|_| "selected SpellCategories row unavailable for cooldown")?;
    let charge_category_id = spell_store
        .charge_category_id_at_difficulty_like_cpp(
            facts.spell.spell_id as u32,
            actual_difficulty_id,
            facts.difficulties,
        )
        .map_err(|_| "selected SpellCategories charge row unavailable")?;
    let (_, base_cooldown_ms, base_category_cooldown_ms) = spell_store
        .cooldown_components_at_difficulty_like_cpp(
            facts.spell.spell_id,
            actual_difficulty_id,
            facts.difficulties,
        )
        .map_err(|_| "selected SpellCooldowns component unavailable")?;

    let mut category_id = base_category_id;
    let mut cooldown_ms = base_cooldown_ms;
    let mut category_cooldown_ms = base_category_cooldown_ms;
    let item_id = facts.metadata.cast_item_entry.unwrap_or(0);
    if let Some(item_id) = facts.metadata.cast_item_entry {
        if facts.metadata.cast_item_guid.is_none() {
            return Err("physical cast item GUID unavailable for cooldown override");
        }
        let item_effects = facts
            .item_effect_store
            .ok_or("ItemEffect catalog unavailable for cooldown override")?;
        if let Some(effect) = item_effects
            .item_effects_for_item_id_like_cpp(item_id)
            .into_iter()
            .find(|effect| effect.spell_id == facts.spell.spell_id)
        {
            if effect.cooldown_msec >= 0 || effect.category_cooldown_msec >= 0 {
                cooldown_ms = effect.cooldown_msec.max(0) as u32;
                category_cooldown_ms = effect.category_cooldown_msec.max(0) as u32;
                category_id = u32::from(effect.spell_category_id);
            }
        }
    }

    let charge_category = categories.get(charge_category_id);
    let max_charges = charge_category.map_or(0, |category| i32::from(category.max_charges));
    let charge_recovery_ms =
        charge_category.map_or(0, |category| category.charge_recovery_time.max(0) as u32);
    if charge_category_id != 0 && max_charges > 0 && charge_recovery_ms > 0 {
        return Ok(ServerNormalCooldownPlanLikeCpp {
            mutation: ServerNormalCooldownMutationLikeCpp::ConsumeCharge {
                charge_category_id,
                recovery_ms: charge_recovery_ms,
                max_charges,
            },
            starts_at_cast_start,
        });
    }

    let category_flags = categories.get(category_id).map_or(0, |row| row.flags);
    let cooldown_starts_on_event = attributes[0] & SPELL_ATTR0_COOLDOWN_ON_EVENT_LIKE_CPP != 0
        || category_flags & SPELL_CATEGORY_FLAG_COOLDOWN_STARTS_ON_EVENT_LIKE_CPP != 0;
    let passive = attributes[0] & SPELL_ATTR0_PASSIVE_LIKE_CPP != 0;
    let is_potion = if item_id == 0 {
        false
    } else {
        facts
            .cast_item_is_potion
            .ok_or("cast item classification unavailable for cooldown")?
    };
    if cooldown_starts_on_event || passive || is_potion {
        return Ok(ServerNormalCooldownPlanLikeCpp {
            mutation: ServerNormalCooldownMutationLikeCpp::None,
            starts_at_cast_start: false,
        });
    }

    // C++ `Spell::prepare` sends item/normal cooldowns with Attr12. All other
    // normal casts call HandleCooldowns after TakePower/TakeReagents and before
    // GO; the caller uses this marker only to avoid starting it twice.
    if cooldown_ms == 0 && category_cooldown_ms == 0 {
        return Ok(ServerNormalCooldownPlanLikeCpp {
            mutation: ServerNormalCooldownMutationLikeCpp::None,
            starts_at_cast_start: false,
        });
    }
    Ok(ServerNormalCooldownPlanLikeCpp {
        mutation: ServerNormalCooldownMutationLikeCpp::StartCooldown {
            spell_id,
            item_id,
            cooldown_ms,
            category_id,
            category_cooldown_ms,
        },
        starts_at_cast_start,
    })
}

pub fn apply_server_normal_cooldown_plan_like_cpp(
    history: &mut wow_entities::SpellHistory,
    plan: ServerNormalCooldownPlanLikeCpp,
    now_ms: u64,
) -> bool {
    match plan.mutation {
        ServerNormalCooldownMutationLikeCpp::None => false,
        ServerNormalCooldownMutationLikeCpp::ConsumeCharge {
            charge_category_id,
            recovery_ms,
            max_charges,
        } => history.consume_charge(charge_category_id, now_ms, recovery_ms, max_charges),
        ServerNormalCooldownMutationLikeCpp::StartCooldown {
            spell_id,
            item_id,
            cooldown_ms,
            category_id,
            category_cooldown_ms,
        } => history.start_cooldown(
            now_ms,
            spell_id,
            item_id,
            u64::from(cooldown_ms),
            category_id,
            u64::from(category_cooldown_ms),
            false,
        ),
    }
}

pub fn server_normal_global_cooldown_state_like_cpp(
    player: &Player,
    spell_id: i32,
    actual_difficulty_id: u8,
    spells: &SpellStore,
    difficulties: Option<&DifficultyStore>,
    now_ms: u64,
) -> Result<(u32, bool), &'static str> {
    let category_id = spells
        .start_recovery_category_at_difficulty_like_cpp(
            spell_id as u32,
            actual_difficulty_id,
            difficulties,
        )
        .map_err(|_| "selected SpellCategories GCD row unavailable")?;
    let history = &player.unit().subsystems().spells.history;
    Ok((
        category_id,
        category_id != 0 && history.has_global_cooldown(category_id, now_ms),
    ))
}

pub fn add_server_normal_global_cooldown_like_cpp(
    history: &mut wow_entities::SpellHistory,
    category_id: u32,
    duration_ms: u32,
    now_ms: u64,
) -> bool {
    if category_id == 0 || duration_ms == 0 {
        return false;
    }
    history.add_global_cooldown(category_id, now_ms, u64::from(duration_ms));
    true
}

pub fn server_normal_cast_item_use_plan_like_cpp(
    physical_item: Option<PhysicalCastItemLikeCpp>,
    effects: Option<&ItemEffectStore>,
    max_stack_size: Option<u32>,
    trigger_cast_flags_like_cpp: u32,
) -> ServerNormalCastItemUsePlanResultLikeCpp {
    let Some(item) = physical_item else {
        return ServerNormalCastItemUsePlanResultLikeCpp::NoItem;
    };
    if trigger_cast_flags_like_cpp & wow_entities::TRIGGERED_IGNORE_CAST_ITEM_LIKE_CPP != 0 {
        return ServerNormalCastItemUsePlanResultLikeCpp::NoItem;
    }
    let Some(effects) = effects else {
        return ServerNormalCastItemUsePlanResultLikeCpp::Unavailable(
            "ItemEffect catalog unavailable for cast item use",
        );
    };
    let Some(max_stack_size) = max_stack_size else {
        return ServerNormalCastItemUsePlanResultLikeCpp::Unavailable(
            "cast item max stack size unavailable",
        );
    };
    let mut updated_charges = item.spell_charges;
    let mut expendable = false;
    let mut without_charges = false;
    let mut marks_item_changed = false;
    let mut writes_charges = false;
    for effect in effects.item_effects_for_item_id_like_cpp(item.entry_id) {
        let slot = usize::from(effect.legacy_slot_index);
        if slot >= item.spell_charges.len() {
            continue;
        }
        if effect.charges < 0 {
            expendable = true;
        }
        if effect.charges == 0 {
            continue;
        }
        let current = if max_stack_size == 1 {
            updated_charges[slot]
        } else {
            item.spell_charges[slot]
        };
        let remaining = if current > 0 {
            current - 1
        } else if current < 0 {
            current + 1
        } else {
            0
        };
        if current != 0 {
            marks_item_changed = true;
            if max_stack_size == 1 {
                updated_charges[slot] = remaining;
                writes_charges = true;
            }
        }
        // C++ assigns this for every charge-bearing effect; the final one wins.
        without_charges = remaining == 0;
    }
    ServerNormalCastItemUsePlanResultLikeCpp::Planned(ServerNormalCastItemUsePlanLikeCpp {
        item_guid: item.guid,
        entry_id: item.entry_id,
        expected_charges: item.spell_charges,
        updated_charges,
        writes_charges,
        marks_item_changed,
        consumes_one_item: expendable && without_charges,
    })
}

/// Source `Spell::cast`: a physical cast item is taken immediately after GO
/// only for a delayed nonchannel spell; other item use follows effects.
pub fn server_normal_take_cast_item_before_effects_like_cpp(
    has_hit_delay: bool,
    attributes1: u32,
) -> bool {
    has_hit_delay
        && attributes1
            & (SPELL_ATTR1_IS_CHANNELLED_LIKE_CPP | SPELL_ATTR1_IS_SELF_CHANNELLED_LIKE_CPP)
            == 0
}

pub fn server_normal_cost_plan_like_cpp(
    facts: ServerNormalCastCostFactsLikeCpp<'_>,
) -> Result<ServerNormalCastCostPlanLikeCpp, ServerNormalCostResultLikeCpp> {
    let Some(actual_difficulty_id) = facts.metadata.actual_spell_difficulty_id_like_cpp else {
        return Err(ServerNormalCostResultLikeCpp::Unavailable(
            "actual SpellInfo key missing",
        ));
    };
    let physical_item = facts.physical_item;
    if !facts.physical_item_state_complete
        || facts.metadata.cast_item_guid.is_some() != physical_item.is_some()
        || facts.metadata.cast_item_entry.is_some() != physical_item.is_some()
        || physical_item.is_some_and(|item| {
            facts.metadata.cast_item_guid != Some(item.guid)
                || facts.metadata.cast_item_entry != Some(item.entry_id)
        })
    {
        return Err(ServerNormalCostResultLikeCpp::Unavailable(
            "physical cast item identity unavailable",
        ));
    }
    if physical_item.is_some() && !facts.inventory_authority_complete {
        return Err(ServerNormalCostResultLikeCpp::Unavailable(
            "complete Player inventory authority unavailable for cast item",
        ));
    }
    if facts.metadata.trigger_cast_flags_like_cpp
        & wow_entities::TRIGGERED_IGNORE_POWER_AND_REAGENT_COST_LIKE_CPP
        != 0
    {
        return Ok(ServerNormalCastCostPlanLikeCpp::default());
    }
    let Some(costs) = facts.costs else {
        return Err(ServerNormalCostResultLikeCpp::Unavailable(
            "effective spell cost rows unavailable",
        ));
    };

    let reagent_cost = costs.reagent_for_spell_like_cpp(facts.spell.spell_id);
    let currencies = costs.currencies_for_spell_like_cpp(facts.spell.spell_id);
    let has_reagent = reagent_cost.is_some_and(|cost| cost.reagent.iter().any(|entry| *entry > 0));
    let has_currency_cost = currencies.iter().any(|cost| cost.currency_count != 0);
    if !has_reagent && !has_currency_cost {
        return Ok(ServerNormalCastCostPlanLikeCpp::default());
    }
    let item_exempts_costs = if physical_item.is_some() {
        let Some(no_reagent_cost) = facts.cast_item_no_reagent_cost else {
            return Err(ServerNormalCostResultLikeCpp::Unavailable(
                "cast item template flags unavailable",
            ));
        };
        no_reagent_cost
    } else {
        false
    };
    let can_skip_item_and_currency_costs = if facts.no_reagent_while_preparing || item_exempts_costs
    {
        true
    } else {
        if !facts.no_reagent_aura_authority_complete {
            return Err(ServerNormalCostResultLikeCpp::Unavailable(
                "complete untombstoned Player aura authority unavailable",
            ));
        }
        if facts.no_reagent_cost_mask == [0; 4] {
            false
        } else {
            let Some(class_options) = facts.class_options else {
                return Err(ServerNormalCostResultLikeCpp::Unavailable(
                    "SpellClassOptions authority unavailable",
                ));
            };
            let Some(difficulties) = facts.difficulties else {
                return Err(ServerNormalCostResultLikeCpp::Unavailable(
                    "difficulty catalog unavailable for spell family",
                ));
            };
            let family = match wow_data::spell::resolve_spell_family(
                class_options,
                facts.spell.spell_id,
                actual_difficulty_id,
                Some(difficulties),
            ) {
                Ok((_, flags)) => flags,
                Err(_) => {
                    return Err(ServerNormalCostResultLikeCpp::Unavailable(
                        "spell family difficulty metadata unavailable",
                    ));
                }
            };
            family
                .iter()
                .zip(facts.no_reagent_cost_mask)
                .any(|(spell_mask, aura_mask)| *spell_mask & aura_mask != 0)
        }
    };
    if can_skip_item_and_currency_costs {
        return Ok(ServerNormalCastCostPlanLikeCpp::default());
    }
    if !facts.inventory_authority_complete {
        return Err(ServerNormalCostResultLikeCpp::Unavailable(
            "complete Player inventory authority unavailable for full-save cost capture",
        ));
    }

    let mut plan = ServerNormalCastCostPlanLikeCpp::default();
    let mut current_cast_item = physical_item;
    if has_reagent {
        if !facts.inventory_authority_complete {
            return Err(ServerNormalCostResultLikeCpp::Unavailable(
                "complete Player inventory authority unavailable for reagent debit",
            ));
        }
        let Some(item_counts) = facts.reagent_counts else {
            return Err(ServerNormalCostResultLikeCpp::Unavailable(
                "canonical inventory topology unavailable",
            ));
        };
        let Some(reagent_cost) = reagent_cost else {
            return Err(ServerNormalCostResultLikeCpp::Unavailable(
                "spell reagent row unavailable",
            ));
        };
        let item_effects = if physical_item.is_some() {
            match facts.item_effect_store {
                Some(store) => Some(store),
                None => {
                    return Err(ServerNormalCostResultLikeCpp::Unavailable(
                        "cast item effect rows unavailable",
                    ));
                }
            }
        } else {
            None
        };
        let mut remaining_item_counts = item_counts.clone();
        for index in 0..reagent_cost.reagent.len() {
            let entry = reagent_cost.reagent[index];
            if entry <= 0 {
                continue;
            }
            let entry = entry as u32;
            let mut required = reagent_cost.reagent_count[index] as u32;
            if current_cast_item.is_some_and(|item| item.entry_id == entry) {
                let item = current_cast_item.expect("cast item was checked above");
                let consumes_item_as_reagent = item_effects
                    .expect("physical cast item has loaded effect authority")
                    .item_effects_for_item_id_like_cpp(entry)
                    .into_iter()
                    .any(|effect| {
                        usize::from(effect.legacy_slot_index) < item.spell_charges.len()
                            && effect.charges < 0
                            && item.spell_charges[usize::from(effect.legacy_slot_index)]
                                .unsigned_abs()
                                < 2
                    });
                if consumes_item_as_reagent {
                    required = required.saturating_add(1);
                }
                current_cast_item = None;
                plan.clears_cast_item_before_go = true;
            }
            let available = remaining_item_counts.get(&entry).copied().unwrap_or(0);
            if available < required {
                return Err(ServerNormalCostResultLikeCpp::MissingReagent(entry));
            }
            if required != 0 {
                plan.reagent_debits.push((entry, required));
                remaining_item_counts.insert(entry, available - required);
            }
        }
    }

    if has_currency_cost {
        if !facts.currency_authority_complete {
            return Err(ServerNormalCostResultLikeCpp::Unavailable(
                "complete Player currency authority unavailable for currency debit",
            ));
        }
        let Some(currency_quantities) = facts.currency_quantities else {
            return Err(ServerNormalCostResultLikeCpp::Unavailable(
                "canonical Player currency state unavailable",
            ));
        };
        let mut remaining_currency_quantities = currency_quantities.clone();
        for cost in currencies {
            let currency_id = u32::from(cost.currency_types_id);
            let required = u32::from(cost.currency_count);
            let available = remaining_currency_quantities
                .get(&currency_id)
                .copied()
                .unwrap_or(0);
            if available < required {
                return Err(ServerNormalCostResultLikeCpp::MissingCurrency(
                    cost.currency_types_id,
                ));
            }
            if required != 0 {
                plan.currency_debits.push((currency_id, required));
                remaining_currency_quantities.insert(currency_id, available - required);
            }
        }
    }
    Ok(plan)
}

pub fn server_normal_cost_result_like_cpp(
    facts: ServerNormalCastCostFactsLikeCpp<'_>,
) -> ServerNormalCostResultLikeCpp {
    match server_normal_cost_plan_like_cpp(facts) {
        Ok(_) => ServerNormalCostResultLikeCpp::Affordable,
        Err(reason) => reason,
    }
}

pub fn server_normal_cooldown_ready_like_cpp(
    player: &Player,
    spell: &SpellInfo,
    actual_difficulty_id: u8,
    metadata: SpellCastMetadata,
    spells: &SpellStore,
    serverside_spells: &ServersideSpellStoreLikeCpp,
    class_options: Option<&SpellClassOptionsStore>,
    difficulties: Option<&DifficultyStore>,
    item_effects: Option<&ItemEffectStore>,
    spell_categories: Option<&SpellCategoryStore>,
) -> Result<bool, &'static str> {
    let history = &player.unit().subsystems().spells.history;
    if !history.cooldowns_loaded {
        return Err("native SpellHistory cooldown ledger is unavailable");
    }

    let spell_id = spell.spell_id as u32;
    let prevention_type = spells
        .prevention_type_at_difficulty_like_cpp(spell_id, actual_difficulty_id, difficulties)
        .map_err(|_| "spell prevention metadata is unavailable")?;
    if prevention_type & 1 != 0 {
        let hit_metadata = spells
            .hit_metadata_for_difficulty_like_cpp(
                spell.spell_id,
                actual_difficulty_id,
                difficulties,
            )
            .ok_or("spell school metadata is unavailable")?;
        if history.is_school_locked(
            u32::from(hit_metadata.school_mask),
            wow_world_core::session::unix_now_ms_like_cpp(),
        ) {
            return Ok(false);
        }
    }
    let cast_item = match metadata.cast_item_guid {
        Some(guid) => {
            let item = player
                .inventory_runtime_like_cpp()
                .item_objects()
                .get(&guid)
                .filter(|item| {
                    item.owner_guid() == player.guid()
                        && item.update_state() != wow_constants::ItemUpdateState::Removed
                })
                .ok_or("physical cast item identity unavailable")?;
            if metadata.cast_item_entry != Some(item.object().entry()) {
                return Err("physical cast item entry does not match GUID");
            }
            Some(item)
        }
        None if metadata.cast_item_entry.is_some() => {
            return Err("cast item entry is missing its physical GUID");
        }
        None => None,
    };

    let item_effect = if let Some(item) = cast_item {
        let store = item_effects.ok_or("ItemEffect catalog is unavailable")?;
        store
            .item_effects_for_item_id_like_cpp(item.object().entry())
            .into_iter()
            .find(|effect| effect.spell_id == spell.spell_id)
    } else {
        None
    };
    let category_id = match item_effect {
        Some(effect) => u32::from(effect.spell_category_id),
        None => spells
            .category_id_at_difficulty(spell_id, actual_difficulty_id, difficulties)
            .map_err(|_| "spell category metadata is unavailable")?,
    };

    let ignored_by_aura = crate::active_spell_cooldown_aura_affects_like_cpp(
        player,
        spell_id,
        actual_difficulty_id,
        spells,
        serverside_spells,
        class_options,
        difficulties,
    )
    .map_err(|_| "active cooldown aura metadata is unavailable")?;
    // In the pinned target source `SpellInfo::CooldownAuraSpellId` starts at
    // zero and has no writers (`SpellInfo.h:371`, `SpellHistory.cpp:678`), so
    // its HasAura branch is known inert for this build.
    if !ignored_by_aura && history.has_cooldown_membership_like_cpp(spell_id, category_id) {
        return Ok(false);
    }

    let charge_category_id = spells
        .charge_category_id_at_difficulty_like_cpp(spell_id, actual_difficulty_id, difficulties)
        .map_err(|_| "spell charge category metadata is unavailable")?;
    if charge_category_id == 0 {
        return Ok(true);
    }
    if !history.charges_loaded {
        return Err("native SpellHistory charge ledger is unavailable");
    }
    let categories = spell_categories.ok_or("SpellCategory catalog is unavailable")?;
    let max_charges = categories
        .get(charge_category_id)
        .map_or(0, |category| i32::from(category.max_charges));
    Ok(history.has_charge(charge_category_id, max_charges))
}

#[cfg(test)]
#[path = "../../unit_tests/player_cast_normal_costs_tests.rs"]
mod tests;
