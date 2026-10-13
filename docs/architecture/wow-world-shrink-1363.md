# #1363 wow-world shrink — slice 0 inventory (input for slice 1)

Owner: GitHub #1363 ("Shrink wow-world"), slice 0. Measured on branch
`1363-s0-test-support` against base `d7e2fa2d5` (`origin/3.4.3`). This note is the
dead-code and test-only inventory that slice 1 consumes; it is not a parity or
completion claim, and it records no behaviour change.

## What slice 0 changed

- **Visibility.** 499 `pub fn` items of `wow-world` became `pub(crate)`: 479 inherent
  `WorldSession` methods and 20 other inherent methods / free functions. A function
  stays `pub` when a crate that depends on `wow-world` names it (`world-server` lib,
  tests and build script, and `wow-world`'s own integration tests under `tests/`); the
  set was found by narrowing every exact-`pub` fn and re-widening each one rustc then
  rejected (E0624/E0603/E0364) in `cargo check -p world-server --all-targets` and
  `cargo check -p wow-world --all-targets`, so it is compiler-proven, not name-guessed.
  Trait impls and already restricted items are untouched.
- **Kept `pub` to stay line-neutral (22).** These are external-unused too, but
  `pub(crate)` makes rustfmt reflow their signature and the runtime-ownership hotspot
  ratchet forbids line growth. Slice 1 narrows them together with deletions that pay
  for the reflow: `handle_chat_addon_message_targeted`, `handle_chat_channel_player_command`, `handle_chat_message`, `handle_chat_register_addon_prefixes`, `handle_db_query_bulk`, `handle_guild_bank_remaining_withdraw_money_query`, `handle_move_apply_movement_force_ack`, `handle_move_change_vehicle_seats`, `handle_move_remove_movement_force_ack`, `handle_query_corpse_location`, `handle_quest_giver_query_quest`, `handle_request_vehicle_switch_seat`, `has_item_appearance_like_cpp`, `item_modified_appearance_store`, `represented_request_adjacent_vehicle_seat_like_cpp`, `save_disconnect_player_to_db_like_cpp`, `send_buy_error`, `send_nearby_creatures`, `send_update_world_state_like_cpp`, `set_creature_addon_store_like_cpp`, `set_module_registry_like_cpp`, `set_quest_pool_store`.
- **Session ownership baseline.** `tools/architecture/session-ownership-policy.json`
  records each `WorldSession` impl item's visibility; the regenerated baseline differs
  from the old one only in 505 `"pub"` -> `"pub (crate)"` values (479 impl items, 26
  direct-registry rows of the same methods). Checked by comparing both documents with
  the `visibility` key removed: identical.
- **`wow-world-test-support`.** New dev-only crate (category `tooling`, so no
  production crate can depend on it; dev edges are not product edges). It receives the
  `WorldSession`-free fixtures of `session::tests::fixtures::{creature_ai, map_objects,
  packets, spell_catalog}` and of `canonical_creature_fixtures` (1,955 lines out of
  `crates/wow-world/unit_tests`, 1,995 in, the difference being module headers and
  explicit imports). Moved bodies are unchanged except `pub(in crate::session::tests)`
  / `pub(crate)` -> `pub` and `crate::creature_respawn` / `crate::session::SharedCanonicalMapManager`
  -> their real `wow_world_core` paths. The session-bound fixtures stay in
  `wow-world`; the fixture roots re-export the moved names explicitly, so every
  consumer path is unchanged. No test moved; test identities are unchanged.

## Lint policy decision

No validation gate builds with `-D warnings` (validation-v2 runs `cargo check` /
`cargo test` without it and no clippy gate exists), so slice 0 adds **no**
`#[allow(dead_code)]`. The newly visible warnings are the point of the slice; slice 1
removes them. Existing allowances (`profession`, `spell_acquisition` and
`canonical_creature_fixtures` modules in `lib.rs`) are unchanged, and the items they
cover are not in the lists below.

## Totals (rustc `dead_code`, `cargo check -p wow-world --lib` vs `--lib --profile test`)

`wow-world` lib `dead_code` warnings: 116 at base, 159 on the branch (one warning can
name several items). Per item:

| Class | Newly reported by slice 0 | Already reported at base | Total |
|---|---|---|---|
| Dead in production and test builds (delete) | 28 items, 169 lines | 90 items, 1284 lines | 118 items, 1453 lines |
| Used only by `cfg(test)` / `test-fixtures` code (cfg(test) or delete with its tests) | 107 items, 1009 lines | 143 items, 2055 lines | 250 items, 3064 lines |

Lines are physical lines of the item including its doc comments and attributes.
"Already reported" items were warnings at base too (mostly in private modules);
slice 1 owns them as well. In the test build two `session/mod.rs` imports
(`set_active_player_update_bit_like_cpp`, `UnitVisibilityDetectionStateLikeCpp`)
became unused because only moved fixtures used them; they are left for slice 1 so this
slice does not shrink `crates/wow-world/src/`.

## Dead in production and test builds

### Newly reported by slice 0

| File:line | Item | Kind | Lines |
|---|---|---|---:|
| `handlers/account_data.rs:49` | `handle_tutorial` | method | 3 |
| `handlers/character/character_handler_shims.rs:35` | `handle_get_undelete_cooldown_status` | method | 8 |
| `handlers/collections/mod.rs:44` | `handle_transmogrify_items` | method | 20 |
| `handlers/quest/eligibility.rs:214` | `has_quest` | method | 5 |
| `session/catalogs/operations.rs:15` | `trinity_string_store` | method | 3 |
| `session/catalogs/operations.rs:23` | `heirloom_store` | method | 4 |
| `session/catalogs/operations.rs:31` | `toy_store` | method | 4 |
| `session/catalogs/operations.rs:62` | `rand_prop_points_store` | method | 3 |
| `session/catalogs/operations.rs:110` | `chr_specialization_store` | method | 3 |
| `session/connection_identity.rs:144` | `idle_time` | method | 4 |
| `session/creature_runtime/admitted_creature_execution/lease.rs:24` | `label_like_cpp` | method | 7 |
| `session/player_items/appearance.rs:14` | `item_appearance_store` | method | 4 |
| `session/player_items/appearance.rs:29` | `transmog_set_item_store` | method | 4 |
| `session/player_items/catalog.rs:28` | `item_search_name_store` | method | 4 |
| `session/player_items/catalog.rs:40` | `item_spec_override_store` | method | 3 |
| `session/player_items/catalog.rs:49` | `item_effect_store` | method | 3 |
| `session/player_items/durability.rs:289` | `durability_costs_store` | method | 4 |
| `session/player_items/durability.rs:293` | `durability_quality_store` | method | 4 |
| `session/player_items/enchantment.rs:16` | `item_random_enchantment_template_store` | method | 5 |
| `session/player_items/enchantment.rs:45` | `spell_item_enchantment_store` | method | 3 |
| `session/player_registry_binding.rs:25` | `pending_invites` | method | 3 |
| `session/spell_state/acquisition.rs:9` | `set_spell_acquisition_port_like_cpp` | method | 7 |
| `session/spell_state/catalog.rs:134` | `spell_label_store` | method | 3 |
| `session/spell_state/cooldown.rs:35` | `is_spell_on_cooldown` | method | 30 |
| `session_persistence_capabilities.rs:49` | `set_stored_item_persistence_port_like_cpp` | method | 7 |
| `session_persistence_capabilities.rs:65` | `set_loot_template_catalog_persistence_port_like_cpp` | method | 7 |
| `session_persistence_capabilities.rs:73` | `set_vendor_catalog_persistence_port_like_cpp` | method | 7 |
| `session_persistence_capabilities.rs:81` | `set_visibility_spawn_catalog_persistence_port_like_cpp` | method | 7 |

### Already reported at base

| File:line | Item | Kind | Lines |
|---|---|---|---:|
| `handlers/character/stats.rs:28` | `represented_player_gear_stats_like_cpp` | method | 7 |
| `handlers/character/stats.rs:51` | `apply_represented_shapeshift_base_attack_time_like_cpp` | method | 4 |
| `handlers/character/stats_update.rs:37` | `send_total_stat_percentage_update_like_cpp` | method | 5 |
| `handlers/instances/mod.rs:42` | `reset_represented_instances_like_cpp` | method | 13 |
| `handlers/loot/authority.rs:83` | `represented_master_loot_target_exists_like_cpp` | method | 7 |
| `handlers/loot/authority.rs:91` | `represented_master_loot_target_eligible_like_cpp` | method | 16 |
| `handlers/loot/mod.rs:163` | `MAX_NR_LOOT_ITEMS_LIKE_CPP` | constant | 1 |
| `handlers/loot/mod.rs:306` | `position` | field | 1 |
| `handlers/loot/mod.rs:307` | `display_id` | field | 1 |
| `handlers/loot/mod.rs:308` | `scale` | field | 1 |
| `handlers/loot/mod.rs:309` | `rotation` | field | 1 |
| `handlers/loot/mod.rs:310` | `go_type` | field | 1 |
| `handlers/loot/mod.rs:311` | `interact_radius_override` | field | 1 |
| `handlers/loot/mod.rs:312` | `lock_id` | field | 1 |
| `handlers/loot/mod.rs:313` | `owner_guid` | field | 1 |
| `handlers/loot/mod.rs:632` | `queue_stored_item_money_indeterminate_kick_like_cpp` | function | 12 |
| `handlers/quest/eligibility.rs:43` | `satisfy_quest_skill_like_cpp` | method | 11 |
| `handlers/quest/eligibility.rs:55` | `satisfy_quest_reputation_like_cpp` | method | 58 |
| `handlers/quest/eligibility.rs:123` | `satisfy_quest_exclusive_group_like_cpp` | method | 78 |
| `handlers/quest/mod.rs:127` | `QUEST_FLAGS_REMOVE_SURPLUS_ITEMS_LIKE_CPP` | constant | 1 |
| `handlers/quest/mod.rs:128` | `QUEST_FLAGS_EX_NO_ITEM_REMOVAL_LIKE_CPP` | constant | 1 |
| `handlers/quest/mod.rs:176` | `reputation_rank_from_standing_like_cpp` | function | 3 |
| `handlers/quest/mod.rs:180` | `calculate_pct_i32_f32_like_cpp` | function | 3 |
| `handlers/quest/mod.rs:184` | `player_quest_level_like_cpp` | function | 7 |
| `handlers/quest/persistence.rs:120` | `plan_quest_status_save_like_cpp` | method | 20 |
| `handlers/quest/rewards.rs:17` | `remove_quest_required_items_and_currencies_like_cpp` | method | 55 |
| `handlers/quest/rewards.rs:73` | `record_represented_quest_reward_reputation_like_cpp` | method | 47 |
| `handlers/quest/rewards.rs:121` | `apply_quest_reward_lockout_status_like_cpp` | method | 20 |
| `handlers/quest/rewards/currencies.rs:10` | `grant_quest_reward_currencies_like_cpp` | method | 48 |
| `handlers/quest/rewards/items.rs:10` | `QuestRewardItemGrantLikeCpp` | enum | 5 |
| `handlers/quest/rewards/items.rs:17` | `store_fixed_quest_reward_items_like_cpp` | method | 14 |
| `handlers/quest/rewards/items.rs:32` | `store_chosen_quest_reward_item_like_cpp` | method | 15 |
| `handlers/quest/rewards/items.rs:48` | `store_quest_package_reward_items_like_cpp` | method | 15 |
| `handlers/quest/rewards/items.rs:64` | `apply_quest_reward_item_grant_like_cpp` | method | 138 |
| `handlers/quest/state.rs:228` | `remove_represented_timed_quest_like_cpp` | method | 15 |
| `handlers/trainer.rs:73` | `trainer_spell_product_like_cpp` | function | 6 |
| `handlers/trainer.rs:87` | `trainer_spell_condition_proof_like_cpp` | method | 54 |
| `quest/application/reward_commit.rs:9` | `commit_quest_reward_plan_like_cpp` | method | 22 |
| `session/catalogs/operations.rs:50` | `update_speak_time_with_policy_like_cpp` | method | 8 |
| `session/catalogs/operations.rs:122` | `lfg_dungeons_store` | method | 3 |
| `session/collections/operations.rs:42` | `record_represented_auction_place_bid_like_cpp` | method | 9 |
| `session/combat/melee.rs:73` | `take_canonical_player_attack_swings_like_cpp` | method | 32 |
| `session/faction_reactions.rs:15` | `reset_schedule_like_cpp` | method | 3 |
| `session/instances/instance.rs:61` | `current_map_instanceable_like_cpp` | method | 4 |
| `session/loot/operations.rs:18` | `represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp` | method | 16 |
| `session/loot/operations.rs:42` | `read_canonical_gameobject_loot_authority_on_map_like_cpp` | method | 8 |
| `session/loot/operations.rs:50` | `mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp` | method | 21 |
| `session/loot/operations.rs:83` | `enable_ae_loot_like_cpp` | method | 3 |
| `session/loot/operations.rs:135` | `group_loot_money_persistence_port_like_cpp` | method | 7 |
| `session/mod.rs:974` | `MAP_BATTLEGROUND_LIKE_CPP` | constant | 1 |
| `session/mod.rs:975` | `MAP_ARENA_LIKE_CPP` | constant | 1 |
| `session/movement/state.rs:104` | `calendar_remove_event_like_cpp` | method | 6 |
| `session/persistence/commit.rs:9` | `commit_exclusive_trainer_money_only_like_cpp` | method | 16 |
| `session/player_items/bank.rs:116` | `record_represented_bank_item_move_like_cpp` | method | 8 |
| `session/player_items/durability.rs:33` | `stats_limits_like_cpp` | method | 4 |
| `session/player_items/equipment.rs:9` | `represented_equip_spell_fits_shapeshift_like_cpp` | method | 7 |
| `session/player_items/equipment.rs:22` | `refresh_represented_item_effects_at_form_change_like_cpp` | method | 10 |
| `session/player_items/equipment.rs:48` | `inventory_equip_capabilities_like_cpp` | method | 4 |
| `session/player_items/equipment.rs:102` | `represented_can_equip_unique_item_like_cpp` | method | 14 |
| `session/player_items/equipment.rs:134` | `can_equip_inventory_item_like_cpp` | method | 25 |
| `session/player_items/items.rs:137` | `record_represented_auction_remove_item_like_cpp` | method | 8 |
| `session/player_items/items.rs:145` | `record_represented_auction_sell_item_like_cpp` | method | 8 |
| `session/player_items/modifiers.rs:269` | `represented_can_use_attack_type_like_cpp` | method | 8 |
| `session/player_items/persistence.rs:258` | `stored_item_money_persistence_port_like_cpp` | method | 7 |
| `session/player_items/storage.rs:100` | `represented_item_inventory_type_like_cpp` | method | 8 |
| `session/player_items/storage.rs:161` | `direct_inventory_player_snapshot` | method | 6 |
| `session/player_items/storage.rs:175` | `add_inventory_item_duration_refs_like_cpp` | method | 4 |
| `session/player_items/storage_slots.rs:9` | `represented_direct_inventory_slot_by_guid_like_cpp` | method | 7 |
| `session/progression/reputation.rs:12` | `reputation_rates_like_cpp` | method | 3 |
| `session/quest/persistence.rs:15` | `resolved_current_player_xp_persistence_request_like_cpp` | method | 22 |
| `session/quest/state.rs:43` | `remove_represented_quest_status_like_cpp` | method | 7 |
| `session/quest/state.rs:64` | `settle_represented_rewarded_quest_like_cpp` | method | 15 |
| `session/quest/state.rs:548` | `record_represented_auction_replicate_request_like_cpp` | method | 9 |
| `session/quest_interaction.rs:13` | `represented_has_quest_for_gameobject_like_cpp` | method | 15 |
| `session/quest_interaction.rs:27` | `represented_gameobject_is_for_quests_like_cpp` | method | 15 |
| `session/social/group.rs:271` | `clear_represented_group_subgroup_like_cpp` | method | 4 |
| `session/social_requests.rs:53` | `calendar_community_invite_like_cpp` | method | 26 |
| `session/social_requests.rs:80` | `calendar_add_event_like_cpp` | method | 48 |
| `session/spell_state/acquisition.rs:40` | `commit_exclusive_player_money_and_spell_acquisition_like_cpp` | method | 48 |
| `session/spell_state/aura.rs:73` | `represented_spell_has_mod_shapeshift_effect_like_cpp` | method | 7 |
| `session/spell_state/aura/effect_queries.rs:48` | `aura_has_total_stat_percentage_effect_like_cpp` | method | 7 |
| `session/spell_state/aura_publication.rs:15` | `insert_player_visible_aura_with_provenance_like_cpp` | method | 8 |
| `session/spell_state/shapeshift.rs:10` | `sync_represented_display_power_like_cpp` | method | 4 |
| `session/spell_state/shapeshift.rs:15` | `represented_spell_has_power_display_effect_like_cpp` | method | 7 |
| `session/state/hub.rs:64` | `split_social_inventory_mut` | function | 16 |
| `session/state/hub.rs:81` | `split_social_lifecycle_mut` | function | 20 |
| `session/state/hub.rs:515` | `world_entities` | field | 1 |
| `session/state/hub.rs:567` | `loot` | field | 1 |
| `session/state/hub.rs:677` | `inventory` | field | 1 |
| `session/taxi/operations.rs:217` | `represented_set_taxi_benchmark_mode_like_cpp` | method | 25 |

## Used only by tests

### Newly reported by slice 0

| File:line | Item | Kind | Lines |
|---|---|---|---:|
| `handlers/account_data.rs:23` | `account_data_test_cx_like_cpp` | method | 4 |
| `handlers/account_data.rs:28` | `handle_request_account_data` | method | 5 |
| `handlers/account_data.rs:34` | `handle_update_account_data` | method | 5 |
| `handlers/account_data.rs:40` | `handle_addon_list` | method | 3 |
| `handlers/account_data.rs:44` | `handle_save_cuf_profiles` | method | 4 |
| `handlers/character/bank.rs:317` | `handle_change_bank_bag_slot_flag` | method | 8 |
| `handlers/character/character_handler_shims.rs:44` | `handle_alter_appearance` | method | 3 |
| `handlers/character/character_handler_shims.rs:48` | `handle_confirm_barbers_choice` | method | 3 |
| `handlers/character/character_handler_shims.rs:52` | `handle_set_player_declined_names` | method | 3 |
| `handlers/character/character_handler_shims.rs:65` | `handle_opening_cinematic` | method | 3 |
| `handlers/character/character_handler_shims.rs:69` | `handle_enum_characters` | method | 8 |
| `handlers/character/character_handler_shims.rs:78` | `handle_char_customize` | method | 8 |
| `handlers/character/character_handler_shims.rs:91` | `char_customize_wire_like_cpp` | function | 17 |
| `handlers/character/items/equipment_sets.rs:15` | `handle_save_equipment_set_with_generator_like_cpp` | method | 14 |
| `handlers/character/items/equipment_sets.rs:39` | `handle_assign_equipment_set_spec` | method | 10 |
| `handlers/character/items/equipment_sets.rs:49` | `handle_delete_equipment_set` | method | 9 |
| `handlers/character/items/equipment_sets.rs:57` | `handle_use_equipment_set` | method | 7 |
| `handlers/character/items/handlers.rs:486` | `handle_cancel_temp_enchantment` | method | 7 |
| `handlers/character/query.rs:104` | `handle_item_text_query` | method | 4 |
| `handlers/entities/corpse.rs:266` | `try_handle_client_port_graveyard_like_cpp` | method | 29 |
| `handlers/entities/player.rs:24` | `handle_set_difficulty_id` | method | 4 |
| `handlers/entities/player.rs:29` | `handle_toggle_difficulty` | method | 4 |
| `handlers/entities/player.rs:34` | `handle_set_dungeon_difficulty` | method | 4 |
| `handlers/entities/player.rs:39` | `handle_set_raid_difficulty` | method | 4 |
| `handlers/group/ops_2.rs:14` | `handle_clear_raid_marker` | method | 47 |
| `handlers/instances/mod.rs:32` | `handle_request_raid_info` | method | 4 |
| `handlers/instances/mod.rs:37` | `handle_reset_instances` | method | 4 |
| `handlers/instances/mod.rs:56` | `handle_instance_lock_response` | method | 4 |
| `handlers/instances/mod.rs:63` | `handle_set_saved_instance_extend` | method | 6 |
| `handlers/pets/mod.rs:30` | `handle_battle_pet_delete_pet_represented_like_cpp` | method | 8 |
| `handlers/pets/mod.rs:68` | `handle_battle_pet_modify_name_represented_like_cpp` | method | 8 |
| `handlers/pets/mod.rs:108` | `handle_battle_pet_delete_pet_represented_like_cpp` | method | 18 |
| `handlers/pets/mod.rs:135` | `handle_battle_pet_modify_name_represented_like_cpp` | method | 25 |
| `handlers/progression/mod.rs:20` | `reputation_test_cx_like_cpp` | method | 3 |
| `handlers/progression/mod.rs:24` | `handle_request_forced_reactions` | method | 5 |
| `handlers/progression/mod.rs:30` | `handle_set_faction_at_war` | method | 5 |
| `handlers/progression/mod.rs:36` | `handle_set_faction_not_at_war` | method | 5 |
| `handlers/progression/mod.rs:42` | `handle_set_faction_inactive` | method | 5 |
| `handlers/progression/mod.rs:48` | `handle_set_watched_faction` | method | 5 |
| `handlers/spell/ops_2.rs:392` | `try_handle_cancel_mod_speed_no_control_auras_like_cpp` | method | 23 |
| `handlers/talent/state.rs:63` | `handle_learn_talents` | method | 30 |
| `session/catalogs/operations.rs:200` | `set_cinematic_sequences_store` | method | 3 |
| `session/connection_identity.rs:57` | `set_realm_id` | method | 3 |
| `session/connection_identity.rs:124` | `is_legit_character` | method | 3 |
| `session/creature_runtime/admitted_creature_execution/admission.rs:144` | `publication_events_like_cpp` | method | 5 |
| `session/creature_runtime/admitted_creature_execution/combat_phases.rs:108` | `deferred_respawn_db_mutations_like_cpp` | method | 8 |
| `session/creature_runtime/admitted_creature_execution/combat_phases.rs:118` | `deferred_visibility_refreshes_like_cpp` | method | 18 |
| `session/creature_runtime/admitted_creature_execution/combat_phases.rs:136` | `deferred_plan_events_like_cpp` | method | 14 |
| `session/gameobject_interaction.rs:21` | `summon_private_object_owner_like_cpp` | method | 27 |
| `session/item_modifiers.rs:145` | `player_stats` | method | 3 |
| `session/lifecycle_ops/operations.rs:9` | `set_session_account_state_port_like_cpp` | method | 6 |
| `session/loot/operations.rs:71` | `set_group_loot_money_persistence_port_like_cpp` | method | 6 |
| `session/loot/operations.rs:154` | `set_group_loot_money_persistence_port_like_cpp` | method | 7 |
| `session/mod.rs:948` | `SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_LIKE_CPP` | constant | 1 |
| `session/mod.rs:949` | `SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_GROUP_LIKE_CPP` | constant | 1 |
| `session/movement/state.rs:79` | `remove_legit_character` | method | 3 |
| `session/persistence/plans.rs:24` | `set_character_enumeration_persistence_port_like_cpp` | method | 7 |
| `session/persistence/plans.rs:31` | `set_packet_spoof_ban_persistence_port_like_cpp` | method | 7 |
| `session/persistence/plans.rs:38` | `set_void_storage_persistence_port_like_cpp` | method | 7 |
| `session/persistence/plans.rs:45` | `set_social_persistence_port_like_cpp` | method | 6 |
| `session/persistence/plans.rs:51` | `set_map_corpse_persistence_port_like_cpp` | method | 7 |
| `session/persistence/plans.rs:58` | `set_represented_group_persistence_port_like_cpp` | method | 7 |
| `session/persistence/plans.rs:65` | `set_support_bug_report_persistence_port_like_cpp` | method | 7 |
| `session/persistence/plans.rs:72` | `set_gossip_catalog_persistence_port_like_cpp` | method | 7 |
| `session/persistence/plans.rs:79` | `set_player_name_query_persistence_port_like_cpp` | method | 7 |
| `session/persistence/plans.rs:86` | `set_instance_lock_persistence_port_like_cpp` | method | 7 |
| `session/persistence/plans.rs:178` | `set_account_data_persisted_like_cpp` | method | 11 |
| `session/pets/battle_pet.rs:604` | `battle_pet_remove_pet_durable_like_cpp` | method | 22 |
| `session/pets/battle_pet_journal.rs:286` | `battle_pet_modify_name_durable_like_cpp` | method | 34 |
| `session/pets/persistence.rs:9` | `set_battle_pet_purchase_persistence_port_like_cpp` | method | 6 |
| `session/pets/persistence.rs:44` | `set_battle_pet_purchase_persistence_port_like_cpp` | method | 9 |
| `session/player_cast/state.rs:45` | `last_spell_cast_time_like_cpp` | method | 4 |
| `session/player_items/appearance.rs:340` | `replay_rewarded_quest_item_appearances_like_cpp` | method | 47 |
| `session/player_items/appearance.rs:386` | `replay_rewarded_quest_direct_item_appearances_like_cpp` | method | 27 |
| `session/player_items/appearance.rs:423` | `set_appearance_is_favorite_like_cpp` | method | 8 |
| `session/player_items/appearance.rs:454` | `remove_temporary_item_appearance_like_cpp` | method | 12 |
| `session/player_items/appearance.rs:466` | `items_providing_temporary_appearance_like_cpp` | method | 7 |
| `session/player_items/enchantment.rs:48` | `is_arena_allowed_enchantment` | method | 3 |
| `session/player_items/enchantment.rs:51` | `apply_enchantment_template_ref` | method | 12 |
| `session/player_items/items.rs:50` | `item_display_id` | method | 3 |
| `session/player_items/items.rs:85` | `is_item_bound_account_wide` | method | 4 |
| `session/player_items/persistence.rs:81` | `set_stored_item_money_persistence_port_like_cpp` | method | 6 |
| `session/player_items/persistence.rs:87` | `set_item_template_addon_catalog_persistence_port_like_cpp` | method | 7 |
| `session/player_items/persistence.rs:247` | `send_item_time_update_plan` | method | 4 |
| `session/player_items/persistence.rs:295` | `set_stored_item_money_persistence_port_like_cpp` | method | 7 |
| `session/player_items/persistence.rs:303` | `set_item_template_addon_catalog_persistence_port_like_cpp` | method | 7 |
| `session/quest/persistence.rs:9` | `set_quest_poi_persistence_port_like_cpp` | method | 6 |
| `session/quest/persistence.rs:71` | `set_quest_poi_persistence_port_like_cpp` | method | 6 |
| `session/quest/rewards.rs:15` | `notify_game_event_quest_complete_like_cpp` | method | 15 |
| `session/registry.rs:86` | `contains_handler` | function | 6 |
| `session/registry.rs:93` | `get_handler` | function | 6 |
| `session/runtime_policy_access.rs:15` | `set_represented_is_outdoors_like_cpp` | method | 3 |
| `session/spell_effects/execution_overloads.rs:12` | `execute_spell_with_generator_like_cpp` | method | 26 |
| `session/spell_effects/execution_overloads.rs:50` | `execute_spell_with_target_data_and_generator_like_cpp` | method | 24 |
| `session/spell_effects/execution_overloads.rs:96` | `execute_spell_with_visual_and_generator_like_cpp` | method | 29 |
| `session/spell_state/aura_application.rs:75` | `apply_aura` | method | 16 |
| `session/spell_state/aura_application.rs:438` | `remove_represented_mod_speed_no_control_auras_cancelable_like_cpp` | method | 7 |
| `session/spell_state/aura_application.rs:445` | `remove_represented_cancelable_auras_by_effect_like_cpp` | method | 19 |
| `session/state/hub.rs:502` | `InventoryCx` | struct | 5 |
| `session/state/hub.rs:506` | `cx_inventory` | function | 5 |
| `session/state/hub.rs:568` | `lifecycle` | field | 1 |
| `session/state/hub.rs:662` | `lifecycle` | field | 1 |
| `session/world_entities/creature.rs:11` | `set_canonical_creature_private_object_owner_like_cpp` | method | 14 |
| `session_persistence_capabilities.rs:17` | `set_vendor_trade_persistence_port_like_cpp` | method | 7 |
| `session_persistence_capabilities.rs:25` | `set_player_inventory_persistence_port_like_cpp` | method | 7 |
| `session_persistence_capabilities.rs:33` | `set_player_quest_persistence_port_like_cpp` | method | 7 |
| `session_persistence_capabilities.rs:41` | `set_player_quest_reward_persistence_port_like_cpp` | method | 7 |

### Already reported at base

| File:line | Item | Kind | Lines |
|---|---|---|---:|
| `canonical_player_sync.rs:43` | `sync_player_liquid_status_like_cpp` | function | 5 |
| `handlers/character/mod.rs:369` | `TACT_KEY_TABLE_HASH_LIKE_CPP` | constant | 1 |
| `handlers/character/stats_queries.rs:11` | `level_up_stat_deltas_like_cpp` | method | 7 |
| `handlers/character/stats_update.rs:11` | `player_stat_changes_like_cpp` | method | 6 |
| `handlers/character/stats_update.rs:18` | `player_stat_changes_with_represented_item_bonuses_like_cpp` | method | 9 |
| `handlers/character/stats_update.rs:43` | `send_level_up_stat_update_like_cpp` | method | 5 |
| `handlers/loot/authority.rs:24` | `request_represented_remote_master_loot_give_like_cpp` | method | 24 |
| `handlers/loot/fanout.rs:208` | `send_loot_error_like_cpp` | method | 9 |
| `handlers/loot/mod.rs:176` | `LOCK_KEY_SKILL_LIKE_CPP` | constant | 1 |
| `handlers/loot/mod.rs:177` | `LOCK_KEY_SPELL_LIKE_CPP` | constant | 1 |
| `handlers/loot/mod.rs:178` | `SPELL_EFFECT_OPEN_LOCK_LIKE_CPP` | constant | 1 |
| `handlers/loot/mod.rs:645` | `spawn_loot_claim_persistence_worker_like_cpp` | function | 45 |
| `handlers/loot/sources/gameobject_authority.rs:53` | `upsert_represented_personal_gameobject_loot_authority_like_cpp` | method | 17 |
| `handlers/quest/mod.rs:121` | `QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL` | constant | 1 |
| `handlers/quest/mod.rs:129` | `CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP` | constant | 1 |
| `handlers/quest/state.rs:268` | `get_quest_slot_quest_id_like_cpp` | method | 9 |
| `handlers/trainer.rs:49` | `trainer_spell_class_race_fit_like_cpp` | function | 23 |
| `session/action_bar_adapter.rs:15` | `set_tutorial_int_like_cpp` | method | 3 |
| `session/action_bar_adapter.rs:19` | `apply_tutorial_action_like_cpp` | method | 33 |
| `session/canonical_access/operations.rs:265` | `owned_player_cuf_profiles_like_cpp` | method | 25 |
| `session/character_customization.rs:39` | `represented_learn_title_like_cpp` | method | 11 |
| `session/character_customization.rs:51` | `represented_has_title_like_cpp` | method | 12 |
| `session/character_customization.rs:64` | `represented_set_chosen_title_like_cpp` | method | 11 |
| `session/chat/operations.rs:9` | `apply_chat_away_mode_like_cpp` | method | 8 |
| `session/combat/damage.rs:122` | `set_player_health_after_runtime_damage_like_cpp` | method | 12 |
| `session/combat/melee.rs:111` | `represented_melee_damage_taken_like_cpp` | method | 50 |
| `session/combat/melee.rs:165` | `represented_melee_outcome_facts_like_cpp` | method | 213 |
| `session/combat/melee.rs:379` | `represented_melee_armor_mitigation_like_cpp` | method | 76 |
| `session/combat/melee.rs:454` | `represented_melee_damage_bonus_like_cpp` | method | 44 |
| `session/combat/vitals.rs:9` | `canonical_player_health_snapshot_like_cpp` | method | 8 |
| `session/combat/vitals.rs:45` | `represented_player_power_values_like_cpp` | method | 12 |
| `session/combat/vitals.rs:57` | `resolved_player_power_values_like_cpp` | method | 12 |
| `session/combat/vitals.rs:69` | `resolved_player_power_snapshot_like_cpp` | method | 12 |
| `session/combat/vitals.rs:81` | `set_player_health_like_cpp` | method | 5 |
| `session/instances/difficulty.rs:40` | `player_difficulty_preferences_snapshot_like_cpp` | method | 6 |
| `session/instances/difficulty.rs:55` | `resolved_dungeon_difficulty_id_like_cpp` | method | 4 |
| `session/instances/difficulty.rs:59` | `resolved_raid_difficulty_id_like_cpp` | method | 4 |
| `session/instances/difficulty.rs:63` | `resolved_legacy_raid_difficulty_id_like_cpp` | method | 4 |
| `session/instances/map_resolution.rs:332` | `canonical_map_has_seer_like_object_like_cpp` | method | 7 |
| `session/item_modifiers.rs:28` | `RepresentedItemSetEffectLikeCpp` | type alias | 1 |
| `session/item_modifiers.rs:30` | `RepresentedItemBonusStateLikeCpp` | type alias | 1 |
| `session/item_modifiers.rs:32` | `void_withdrawal_post_store_item_values_update_like_cpp` | function | 43 |
| `session/loot/operations.rs:34` | `read_canonical_creature_loot_authority_on_map_like_cpp` | method | 8 |
| `session/loot/operations.rs:100` | `set_pass_on_group_loot_like_cpp` | method | 4 |
| `session/loot/operations.rs:128` | `set_loot_specialization_id_like_cpp` | method | 4 |
| `session/mod.rs:858` | `QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP` | constant | 1 |
| `session/mod.rs:859` | `QUEST_OBJECTIVE_MAX_REPUTATION_LIKE_CPP` | constant | 1 |
| `session/mod.rs:863` | `QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP` | constant | 1 |
| `session/mod.rs:864` | `QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP` | constant | 1 |
| `session/mod.rs:865` | `QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP` | constant | 1 |
| `session/mod.rs:911` | `MAP_INSTANCE_LIKE_CPP` | constant | 1 |
| `session/mod.rs:912` | `MAP_RAID_LIKE_CPP` | constant | 1 |
| `session/mod.rs:915` | `PLAYER_FLAGS_PVP_TIMER_LIKE_CPP` | constant | 1 |
| `session/mod.rs:922` | `TRADE_STATUS_PLAYER_BUSY_LIKE_CPP` | constant | 1 |
| `session/money/operations.rs:65` | `add_currency_vendor` | method | 16 |
| `session/movement/state.rs:50` | `remove_currency` | method | 5 |
| `session/movement/state.rs:88` | `set_represented_mover_fixed_position_vehicle_like_cpp` | method | 14 |
| `session/movement/state.rs:262` | `resolved_movement_force_mod_magnitude_like_cpp` | method | 10 |
| `session/npc_interaction.rs:20` | `set_player_trainer_interaction_like_cpp` | method | 8 |
| `session/npc_interaction.rs:29` | `reset_player_interaction_if_source_like_cpp` | method | 7 |
| `session/npc_interaction.rs:42` | `resolved_player_interaction_trainer_id_like_cpp` | method | 4 |
| `session/npc_interaction.rs:47` | `player_trainer_interaction_matches_like_cpp` | method | 8 |
| `session/persistence/load.rs:387` | `loaded_action_buttons_snapshot_like_cpp` | method | 27 |
| `session/persistence/save.rs:9` | `current_player_save_to_db_snapshot_like_cpp` | method | 9 |
| `session/persistence/save.rs:77` | `persist_standalone_player_currency_save_like_cpp` | method | 36 |
| `session/persistence/save.rs:193` | `has_complete_player_skill_save_authority_like_cpp` | method | 6 |
| `session/persistence/save.rs:199` | `represented_save_cuf_profiles_like_cpp` | method | 7 |
| `session/player_binding.rs:224` | `set_player_liquid_status_like_cpp` | method | 3 |
| `session/player_binding.rs:228` | `set_player_level_like_cpp` | method | 39 |
| `session/player_condition_values.rs:40` | `RepresentedItemLevelCapsLikeCpp` | type alias | 1 |
| `session/player_items/bank.rs:123` | `represented_bank_bag_slot_flag_like_cpp` | method | 4 |
| `session/player_items/bank.rs:127` | `set_represented_bank_bag_slot_flag_like_cpp` | method | 8 |
| `session/player_items/equipment.rs:26` | `apply_initial_equipped_item_equip_auras_like_cpp` | method | 15 |
| `session/player_items/equipment.rs:158` | `represented_avg_equipped_item_level_like_cpp` | method | 17 |
| `session/player_items/equipment_sets.rs:41` | `delete_represented_equipment_set_like_cpp` | method | 4 |
| `session/player_items/modifiers.rs:21` | `item_limit_category_template_like_cpp` | method | 8 |
| `session/player_items/modifiers.rs:38` | `player_item_modifier_runtime_snapshot_like_cpp` | method | 6 |
| `session/player_items/modifiers.rs:45` | `add_player_item_set_item_like_cpp` | method | 8 |
| `session/player_items/modifiers.rs:54` | `add_player_item_set_bonus_like_cpp` | method | 8 |
| `session/player_items/modifiers.rs:63` | `set_player_item_level_caps_like_cpp` | method | 11 |
| `session/player_items/modifiers.rs:105` | `record_represented_update_item_set_auras_like_cpp` | method | 10 |
| `session/player_items/modifiers.rs:115` | `plan_represented_update_item_set_auras_like_cpp` | method | 19 |
| `session/player_items/modifiers.rs:227` | `apply_represented_item_set_aura_refresh_events_like_cpp` | method | 7 |
| `session/player_items/storage.rs:19` | `move_represented_direct_inventory_item_with_item_mods_like_cpp` | method | 51 |
| `session/player_items/storage.rs:70` | `record_destroyed_inventory_item_mod_remove_like_cpp` | method | 18 |
| `session/player_items/valuation.rs:9` | `set_represented_item_level_caps_like_cpp` | method | 6 |
| `session/player_items/valuation.rs:15` | `set_represented_using_pvp_item_levels_like_cpp` | method | 4 |
| `session/player_items/valuation.rs:19` | `resolved_using_pvp_item_levels_like_cpp` | method | 4 |
| `session/player_items/valuation.rs:23` | `update_represented_item_level_area_based_scaling_like_cpp` | method | 4 |
| `session/player_items/valuation.rs:119` | `represented_item_level_like_cpp` | method | 20 |
| `session/player_presentation.rs:28` | `set_represented_shapeshift_form_like_cpp` | method | 11 |
| `session/player_presentation.rs:37` | `represented_primary_specialization_id_like_cpp` | method | 14 |
| `session/player_presentation.rs:52` | `set_represented_primary_specialization_id_like_cpp` | method | 17 |
| `session/player_presentation.rs:209` | `apply_far_sight_like_cpp` | method | 27 |
| `session/progression_adapters.rs:11` | `set_championing_faction_like_cpp` | method | 12 |
| `session/progression_adapters.rs:24` | `killed_player_credit_with_generator_like_cpp` | method | 15 |
| `session/progression_adapters.rs:52` | `kill_credit_criteria_tree_objective_with_generator_like_cpp` | method | 14 |
| `session/quest/objectives.rs:170` | `update_represented_storing_flag_quest_objective_progress_like_cpp` | method | 120 |
| `session/quest/rewards.rs:30` | `record_represented_rewarded_quest_row_like_cpp` | method | 5 |
| `session/quest/rewards.rs:35` | `complete_player_quest_status_authority_load_like_cpp` | method | 5 |
| `session/quest/rewards.rs:160` | `add_represented_quest_reward_talent_points_like_cpp` | method | 29 |
| `session/quest/rewards.rs:395` | `set_represented_daily_quest_completed_like_cpp_for_test` | method | 14 |
| `session/quest/rewards.rs:431` | `add_currency_quest_reward_like_cpp` | method | 23 |
| `session/quest/state.rs:117` | `ensure_represented_seasonal_event_like_cpp` | method | 7 |
| `session/quest/state.rs:237` | `spell_area_for_quest_map_bounds_like_cpp` | method | 11 |
| `session/quest/state.rs:248` | `spell_area_for_quest_end_map_bounds_like_cpp` | method | 11 |
| `session/quest/state.rs:322` | `set_represented_resurrection_request_like_cpp` | method | 18 |
| `session/quest/state.rs:340` | `represented_resurrection_requested_by_like_cpp` | method | 10 |
| `session/quest/state.rs:515` | `set_represented_df_quest_like_cpp_for_test` | method | 14 |
| `session/quest_interaction.rs:43` | `represented_gameobject_activate_to_quest_like_cpp` | method | 43 |
| `session/rest_progression.rs:92` | `add_represented_xp_rest_bonus_like_cpp` | method | 17 |
| `session/rest_progression.rs:194` | `take_represented_xp_rest_bonus_for_gain_like_cpp` | method | 30 |
| `session/social/duel.rs:13` | `resolved_represented_duel_arbiter_guid_like_cpp` | method | 6 |
| `session/social/guild.rs:9` | `player_guild_state_snapshot_like_cpp` | method | 6 |
| `session/social/guild.rs:52` | `set_represented_guild_id_invited_like_cpp` | method | 15 |
| `session/social/trade.rs:31` | `set_represented_active_trade_partner_like_cpp` | method | 7 |
| `session/social_requests.rs:129` | `set_represented_arena_team_id_invited_like_cpp` | method | 8 |
| `session/spell_effects/effects_player.rs:8` | `loaded_player_visible_items_for_create_like_cpp` | method | 17 |
| `session/spell_state/acquisition.rs:174` | `replace_complete_spell_acquisition_runtime_like_cpp` | method | 36 |
| `session/spell_state/aura_application.rs:27` | `remove_player_visible_aura_like_cpp` | method | 7 |
| `session/spell_state/spell.rs:74` | `spell_linked_like_cpp` | method | 12 |
| `session/spell_state/spell.rs:86` | `spell_area_map_bounds_like_cpp` | method | 8 |
| `session/spell_state/spell.rs:94` | `spell_area_for_area_map_bounds_like_cpp` | method | 11 |
| `session/spell_state/spell.rs:105` | `spell_proc_entry_like_cpp` | method | 14 |
| `session/spell_state/spell.rs:119` | `spells_required_for_spell_like_cpp` | method | 8 |
| `session/spell_state/spell.rs:127` | `is_spell_requiring_spell_like_cpp` | method | 8 |
| `session/spell_state/spell.rs:205` | `begin_represented_trait_authority_load_like_cpp` | method | 8 |
| `session/spell_state/spell.rs:232` | `install_represented_trait_authority_rows_like_cpp` | method | 11 |
| `session/spell_state/spell.rs:271` | `mark_represented_spell_rows_saved_like_cpp` | method | 7 |
| `session/spell_state/spell.rs:279` | `rebase_represented_spells_onto_saved_rows_like_cpp` | method | 9 |
| `session/spell_state/spell.rs:424` | `represented_spell_trait_definition_ids_like_cpp` | method | 10 |
| `session/spell_state/spell.rs:630` | `add_represented_self_res_spell_like_cpp` | method | 19 |
| `session/spell_state/spellbook.rs:28` | `is_spell_learn_spell_like_cpp` | method | 7 |
| `session/spell_state/spellbook.rs:35` | `is_spell_learn_to_spell_like_cpp` | method | 7 |
| `session/spell_state/spellbook.rs:592` | `represented_favorite_known_spells_like_cpp` | method | 10 |
| `session/test_support/operations.rs:225` | `loot_money_persistence_test_result_for_worker_like_cpp` | method | 10 |
| `session/world_entities/creature.rs:116` | `talked_to_creature_with_generator_like_cpp` | method | 16 |
| `session/world_entities/creature.rs:226` | `apply_creature_aura_like_cpp` | method | 27 |
| `session/world_state/operations.rs:178` | `represented_explored_zones_db_string_like_cpp` | method | 5 |
| `session/world_state/operations.rs:189` | `update_area_represented_like_cpp` | method | 9 |
| `session/world_state/operations.rs:565` | `set_area_spirit_healer_guid_like_cpp` | method | 4 |
| `session/world_state/operations.rs:569` | `area_spirit_healer_guid_like_cpp` | method | 10 |
| `session/xp_grants.rs:110` | `give_xp_runtime_like_cpp` | method | 17 |

## Reproduce

```bash
export PROTOC=/home/ubuntu/.local/protoc/bin/protoc CARGO_BUILD_JOBS=1
cargo check --locked -p wow-world --lib --message-format json                 > lib.json
cargo check --locked -p wow-world --lib --profile test --message-format json  > test.json
# dead_code items of the lib artifact (profile.test=false) minus those of the
# lib-test artifact (profile.test=true) are the test-only class.
```

## Slice 1 result (dead code and test-only production items)

Branch `1363-s1-dead-code`, base `d1d0ff94d`. Re-derived from rustc on the branch
itself rather than from the lists above (they had drifted by a few items); the same
three builds are compared: `lib` (production), `lib` with `test-fixtures` (what
dependents' tests link) and `lib --profile test`.

| rustc `wow-world` lib warnings | base | slice 1 |
|---|---:|---:|
| `dead_code`, production lib | 159 | 0 |
| `dead_code`, lib + `test-fixtures` | 165 | 0 |
| `dead_code`, lib test | 118 | 0 |
| `unused_imports`, production lib | 240 | 167 |

- **Deleted.** Every item dead in both the production and the test build (the
  cascade produced no second round), the test-gated items no test reached, the
  four files and the `quest` module those deletions emptied, the dead
  `RepresentedGameObjectLootStateLikeCpp` snapshot (its only reader wanted
  existence, now computed directly from the same three reads), the unread hub
  context fields, the cfg(test)-only `SavedPlayerReceipt::acknowledge` path,
  and imports unused in all three builds (including the two `session/mod.rs`
  test imports slice 0 left behind).
- **Moved to the test tree.** Production methods reached only by `cfg(test)`
  code moved verbatim into the module's `unit_tests/**/f3_shims.rs` (when the
  module already had one) or a new `unit_tests/**/test_only.rs` child module;
  visibility is widened to `pub(crate)` only where a sibling test calls the
  method. Two emptied source modules (`handlers/character/stats_queries.rs`,
  `handlers/group/ops_2.rs`) are now mounted directly from their test file.
- **cfg(test) in place.** Test-only consts, type aliases, free functions with
  path-sensitive callers, two context fields and the imports only those items
  use. `test-fixtures`-gated shims no dependent crate reaches are narrowed to
  `cfg(test)`.
- **The 22 `pub` fns** are `pub(crate)` now; the ones that were then dead are
  deleted and the test-only ones moved, like the rest.
- **No test is deleted.** Every test-only entry point either delegates to the
  live production owner (the application/lifecycle/social handler bodies) or is
  an unregistered represented port (the `0xBADD` battle-pet bodies), so its tests
  keep covering real behaviour; the source-scan `include_str!` list drops the
  deleted `stats_queries.rs`.
- **Policy.** `wow-world-test-support` is a restricted package in
  `tools/architecture/dependency-policy.json` (its current nine workspace
  dependencies, `flume` externally), and the fixture
  `forbidden-test-support-to-wow-world.json` proves a `wow-world` edge is
  rejected.
- **Accounting.** R1 shrink of `crates/wow-world/src/` is 5,237 physical lines
  with no charged growth (the 3,176 lines added under `unit_tests/` are
  uncharged test trees). The hotspot ledger's production and total ceilings fall;
  its test ceilings rise by exactly the reclassified bodies (session +2,860,
  character +227, loot +50, quest +26), recorded as reviewed reclassification.

## Slice 2 result (duplicates of an owner body)

Branch `1363-s2-duplicates`, base `43cd093f7` (slice 1 merged). C++ reference
`woltk-trinity-legacy` at `a5f8da2e`. Includes #1359.

### How the inventory was built

Every `fn` body in `crates/*/src` was extracted with brace matching (comments and
string contents blanked) and normalised by removing whitespace. A `wow-world` body was a
candidate when (a) its normalised text hashed equal to a body in another crate, (b) a
body with the same name elsewhere scored ≥ 0.6 on a character diff, or (c) its
6-token shingle set, with `self`/`hub`/`host`/`session` folded together, had a
Jaccard index ≥ 0.3 with an owner body (≥ 15 lines). Each candidate was then diffed by
hand. Most matches with the same name are adapters that only build the owner's
borrowed context, or wrappers whose body is one call. They are glue, not
duplicates, and are not listed. Only real second copies of an owner body are listed.

### Retired onto the owner

| `wow-world` copy (base) | Owner | Bodies | Parity evidence |
|---|---|---|---|
| `session/quest/objectives.rs` `update_represented_storing_value_quest_objective_progress_like_cpp` (161 lines) | `wow-world-application/src/quest/objectives.rs` `QuestObjectiveProgressCx::update_storing_value_quest_objective_progress_like_cpp` (now `pub`) | differ only in access path | The App body is the shell body ported onto the App owner by #1358. It runs the same filter, the same packets in the same order, `represented_can_complete_quest_after_objective_like_cpp`, completion, save and registry sync. Count updates go through `update_objective_count_like_cpp`, the same clamp and early return as the shell closure. Already live for currency/reputation since #1358. C++ `Player::UpdateQuestObjectiveProgress` `Player.cpp:16181`. |
| `unit_tests/session/quest/objectives/f3_shims.rs` `update_represented_storing_flag_quest_objective_progress_like_cpp` (test tree, 113 lines) | `…::update_storing_flag_quest_objective_progress_like_cpp` (now `pub`) | differ only in access path | same argument as above, through `update_storing_flag_like_cpp` |
| `handlers/quest/state.rs` `complete_represented_quest_like_cpp` + body of `complete_represented_quest_after_objective_with_generator_like_cpp`, and `session/quest/state.rs` `complete_represented_quest_status_like_cpp` (~95 lines) | `QuestObjectiveProgressCx::complete_represented_quest_after_objective_like_cpp` + `quest::completion::complete_represented_quest_status_like_cpp` | differ only in access path | Same eligibility check, INCOMPLETE→COMPLETE transition, evidence record with the same flags, `UpdateVisibleGameobjectsOrSpellClicks`, registry sync. The tracking-event reward calls the same `QuestRewardCx::reward_quest_with_generator_like_cpp` with the same arguments (item 0, ITEM, ITEM, CURRENCY), then the evidence mark and the recursive drain. C++ `Player::CompleteQuest` `Player.cpp:14496`. |
| `session/social_requests.rs` `party_member_power_kind_from_u8_like_cpp` (30) | `wow_world_core::session::power_type_from_u8_like_cpp` | identical | byte-identical match arms |
| `handlers/void_storage.rs` `void_withdrawal_initial_item_flags_like_cpp` (15) | `wow_world_application::vendor::vendor_stored_new_item_flags_like_cpp` | identical | byte-identical |
| `session/quest_dialog.rs` and `handlers/quest/mod.rs` `quest_giver_creature_id_from_source_like_cpp` (9 + 3) | `wow_world_application::quest_giver_creature_id_from_source_like_cpp` | identical | byte-identical |
| `handlers/loot/mod.rs` `ItemTemplateAddonLootMetadataLikeCpp` (+ its two flag consts), `player_class_mask_like_cpp`, `roll_chance_with_rate_like_cpp`, `referenced_loot_max_count_like_cpp` (~33) | `wow_world_lifecycle::loot_template_rules` | identical | byte-identical; `handlers/spell` already used these owners |
| `handlers/loot/sources/gameobject_authority.rs` / `sources/creature.rs` / `claims.rs` sync and prepare bodies (`sync_represented_{gameobject,creature}_loot_to_canonical_like_cpp`, `prepare_owned_loot_authority_for_active_request_outcome_like_cpp`, 133 lines) | `wow-world-application/src/loot_release/authority.rs` (`LootReleaseCxLikeCpp`, now `pub`) | differ only in the fixture gate | The shell gate `represented_local_loot_fixture_allowed_like_cpp()` is `cfg!(test)`, and `loot_release_cx_like_cpp` passes `consumer_test = cfg!(test)`. The helpers the bodies call (`represented_loot_authority_pools_like_cpp`, `reconcile_represented_loot_cache_like_cpp`, the authority lookup, the summary refresh) were already one-call forwarders into the same App context. The shell keeps three one-call forwarders so its callers and tests are unchanged. |
| `handlers/loot/sources/creature_conditions.rs` `load_represented_creature_loot_condition_rows_for_id_like_cpp` + `…_reference_rows_for_id_like_cpp` (76) | `SessionLifecycleState::load_loot_template_condition_rows_like_cpp` / `…_reference_rows_for_id_like_cpp` | differ only in log text | Same port call, row mapping, self-reference filter and normalisation. Only the warn message on a failed load changes ("failed to load represented creature loot conditions" → "failed to load loot template condition rows"). |
| `session/inventory_handler_contexts.rs` `bank_interaction_access_like_cpp` (25) | `HubRef::trainer_npc_interaction_access_like_cpp` (wow-world-core) | identical | same selected catalogs and fixture references |
| `session/world_entities/gameobject_state.rs` `represented_gameobject_go_state_for_viewer_like_cpp` (16) | `wow_world_application::represented_gameobject_go_state_for_viewer_like_cpp` (now `pub`) | identical apart from the viewer GUID parameter | the caller passes `self.player_guid()` |

One World adapter remains: `session/quest/objective_drain.rs`
`quest_objective_operation_like_cpp(QuestObjectiveOperationLikeCpp::{Drain, StoringValue,
StoringFlag (cfg(test)), Complete})`. It builds the `QuestRewardCx` and
`QuestObjectiveProgressCx` borrows once. Each operation runs only the App owner.

**Behaviour.** No packet, write or order changes. The only observable difference is
the warn text above, emitted when a loot-condition catalog load fails.

### Duplicates kept (follow-ups)

| `wow-world` body | Owner candidate | Why not retired here |
|---|---|---|
| `handlers/quest/source_items.rs` `store_quest_source_item_with_generator_like_cpp` (387) | App `quest/reward/item_storage.rs` `store_quest_reward_item_like_cpp` | Not the same operation. The source-item store (quest accept) commits `QuestItemGrant` immediately and runs the bound-objective preflight. The reward store defers the rows to the reward plan's closing `SaveToDB` (`Player.cpp:14867`). A merge needs a commit-timing contract per caller. |
| `spell_acquisition/adapter.rs` `spell_acquisition_snapshot_like_cpp` (169) + `spell_state/spell.rs` `complete_represented_{player_spell_rows,spell_trait_definition_ids,override_spells}_like_cpp` + `profession.rs` `plan_primary_profession_capacity_like_cpp` | App `trainer_purchase/projection.rs` / `controller.rs` (`TrainerOfferCxLikeCpp`) | Same snapshot logic over different inputs. The shell takes `lifecycle` / future player-condition resolutions for effect learning. The App hardcodes the trainer case and reads fixtures through its own skill accessors. Unify by giving the App builder those parameters and constructing its context from the World adapter. |
| `handlers/loot/fanout.rs` `represented_notify_loot_list_like_cpp` (51) | App `loot_release/mod.rs:298` | The **owner looks wrong**. C++ `Loot::NotifyLootList` (`Loot.cpp:637`) sends to every allowed looter. The shell does too: itself directly, others through the registry. The App copy sends the direct packet only when `player_guid == owner_guid`, and `send_loot_list_to_other_allowed_looters_like_cpp` skips the loot *owner* instead of the current player. Fix the App copy against C++ first, then retire the shell copy. |
| `handlers/loot/mod.rs` `player_team_for_race_cpp_representable` (race list incl. 26/27/28/31/35/36/70) | `wow_world_lifecycle::loot_template_rules::player_team_for_race_cpp_representable` (2/5/6/8/9/10 only) | The bodies differ, and `handlers/spell` (item loot) already uses the lifecycle copy. One of the two is wrong for allied/neutral races. Decide against C++ `Player::TeamForRace` / `ChrRaces` before merging. |
| `handlers/dungeon_finding/mod.rs` `lfg_is_disabled_map_type_for_player_like_cpp` (38) | `wow-world-instances` `is_disabled_map_type_for_player_like_cpp` | The LFG copy passes the dungeon difficulty (`player_map_difficulty: Some(..)`) and the instances copy passes `None`. C++ `LFGMgr.cpp:1754` → `DisableMgr::IsDisabledFor(DISABLE_TYPE_MAP, …, player)`. Add a difficulty parameter to the owner, then retire. |
| `handlers/loot/requests/context.rs` / `generation.rs` / `handlers/spell/ops_2.rs` current-player quest-item checks (`has_incomplete_quest_objective_for_object_id`, `has_incomplete_quest_item_drop_for_item`, `player_quest_objective_progress`, ~240 lines in 6 copies) | wow-world-core `session/loot/{context,generation}.rs` `remote_*` over `RepresentedLootPlayerContext` | Same logic, but the current player reads `direct_inventory_item_count_like_cpp` (`None` → false). The remote form reads the snapshot count (missing → 0). Parity on unreadable inventory is unclear. C++ `Player::HasQuestForItem` `Player.cpp:16343`. |
| `session/quest/rewards.rs` `quest_xp_reward_like_cpp` (25) | App `quest/reward/xp.rs:206` | The App reads the player level only when the quest level is 0. Likely equivalent, but routing needs a full `QuestRewardCx`. |

### #1359

- Shell storing-value/flag copies and the `handlers/quest/state.rs` completion copy
  are retired onto the App owner (table above). Every caller listed in #1359 now reaches
  the owner: `creature_kill.rs`, `world_entities/gameobject.rs`, and the
  `progression_adapters` / `world_entities/creature` shims that slice 1 moved into the
  test tree.
- Tests:
  - New focal drain `s2_quest_objective_owner_drain.rs`: recorded reputation and
    currency events apply in order, store partial credit and publish the credits in
    that order, with no completion.
  - Tracking-event evidence assertion in
    `reputation_min_tracking_event_objective_auto_rewards_like_cpp`: one completion
    record, marked auto-rewarded by the App owner.
  - Filtered NpcFlags content in the #1358 LootMoney regression: the refresh update
    carries both NpcFlags words zeroed. The spell-click condition fails, so C++
    `ViewerDependentValue<NpcFlagsTag>` (`ViewerDependentValues.h:291`) clears
    `UNIT_NPC_FLAG_SPELLCLICK`.
- **Not done:** narrowing `QuestRewardPlayerAccessLikeCpp::core_ref_like_cpp` (and the
  `loot_release.rs:91` precedent) to a role-specific accessor. It was optional for this
  slice and is left on #1359.

### Accounting

- `crates/wow-world/src`: −649 / +176 lines (net −473); `unit_tests` −156 / +144.
- Destinations: `wow-world-application` +9 / −7 (net +2: three `pub` widenings and one
  re-export). That is far inside the 15 % budget. `net_move.py check`: S = 490, G = 2, PASS.
- Hotspot ledger: production and total ceilings tightened on `session/mod.rs`,
  `handlers/loot/mod.rs` and `handlers/quest/mod.rs`, with test ceilings unchanged.
  The new tests fit in the test lines freed by the retired flag shim. Physical ceilings
  tightened: `session/mod.rs` 1009 → 1007, `unit_tests/handlers/loot_tests.rs` 927 → 925.
  No ceiling is raised.
- `wow-world` `dead_code`: 0 in the production lib, the lib with `test-fixtures` and
  the lib test build. Production `unused_imports` 167 → 166.
- `session-ownership-policy.json` is reprinted (`print-baseline`).
  - Retired: 9 impl items (the copies above) and the shell storing-value body's two
    direct-registry rows.
  - Added: the adapter `quest_objective_operation_like_cpp`.
  - Moved: the three loot forwarders (to `handlers/loot/claims.rs`) and
    `complete_represented_quest_after_objective_with_generator_like_cpp` (to
    `session/quest/objective_drain.rs`).

## Slice 3a result (creature runtime → `wow-world-creature`)

Branch `1363-s3-creature-crate`, base `d725325b7` (slice 2 merged). Move-only (R2):
bodies are unchanged; only paths, imports, visibility and the rustfmt reflow those
path changes cause differ. C++ anchors are unchanged, so no C++ re-audit applies.

### What moved

- `crates/wow-world/src/session/creature_runtime/**` (26 files) →
  `crates/wow-world-creature/src/**`, with the old `mod.rs` as `lib.rs`. The two
  slice-1 `test_only.rs` accessor files moved to `crates/wow-world-creature/unit_tests/`.
  They are gated `cfg(any(test, feature = "test-fixtures"))`, and `wow-world` enables
  the feature only through its dev-dependency.
- `session/player_melee_application.rs`, the player melee phase that only this runtime
  runs, moved with it. Four creature-only melee values went into the same file:
  `RepresentedMeleeSwingLikeCpp`, `RepresentedArmorMitigationLikeCpp`,
  `RepresentedMeleeDamageBonusLikeCpp` and `legacy_attack_power_multiplier_like_cpp`.
  The three melee range constants moved to `lib.rs`. `PlayerAttackStartLikeCppResult`
  is produced only by `WorldSession` attack start, so it stays in `wow-world` (`session/combat/melee.rs`).
- Two shared items moved down to `wow-world-spell` because both `wow-world` and the
  runtime call them. Neither crate can depend on the other, and a host trait over a pure
  function would be new code with no benefit:
  - `write_absorbed_shield_amount_like_cpp` → `aura_effects.rs`.
  - `SPELL_CAST_SOURCE_NORMAL_LIKE_CPP` and `represented_spell_cast_guid_for_map_like_cpp`
    → `spell_cast_adapter.rs`. `wow-world-application::spell_click_values` re-exports
    them, so the runtime does not depend on the application crate.

### Coupling

- The runtime makes no `WorldSession` call, so **no host trait was needed**: 0 host-trait lines.
- `wow-world-creature` depends on `wow-ai`, `wow-constants`, `wow-core`, `wow-data`,
  `wow-entities`, `wow-map`, `wow-movement`, `wow-packet`, `wow-persistence`,
  `wow-world-core`, `wow-world-entities` and `wow-world-spell`. It has no external crates,
  and it has no edge to `wow-world` or `wow-world-application`.
- `wow-world` re-exports the executor and player-melee entries by name from
  `session/mod.rs`, so `world-server` keeps its `wow_world::session::…` paths unchanged.
- Names that only the unit tests still reach are re-exported under `cfg(test)`. These
  are the single-phase entries, the phase store, the swing helpers, the
  `session_rules` names and the three thin `wow-world-entities` re-export modules.

### Policy and accounting

- `dependency-policy.json`:
  - `wow-world-creature` is in the `application` category with the other `wow-world-*`
    crates.
  - It is a restricted package with exactly the 12 dependencies above, and external
    `normal`/`build` are both empty.
  - The new fixture `forbidden-creature-to-wow-world.json` proves that a `wow-world` edge
    is rejected.
- `net-move-policy.json`: `crates/wow-world-creature/src/` is a destination root, so
  the moved code is charged. Result: S = 10161, G = 10291 (G − S = 130, 1.3 %),
  allowance 11630.1, 0 duplicate bodies, PASS.
- Ceilings are tightened and none is raised:
  - Physical: `session/mod.rs` 1007 → 1002.
  - Hotspot `session/mod.rs`: production 55790 → 45584, test 144768 → 144750,
    total 200558 → 190334.
  - Every new-crate file is under the 1,000-line review limit, so no new physical entry
    is needed. The largest is `creature_aggro_tick.rs` at 953 lines.
- `session-ownership-policy.json` is unchanged: `print-baseline` is identical, because
  the runtime has no `WorldSession` impl items or registry/bridge rows.
- Lines:
  - `crates/wow-world/src`: −10264 / +103 (net −10161).
  - `crates/wow-world-creature/src`: +10249. Of these, `lib.rs` is 181, including about
    108 lines of import prelude and named re-exports.
  - `wow-world-spell`: +63 (the moved shared items).
  - `wow-world-application`: −21.
- Warnings:
  - `wow-world` `dead_code` stays 0 in the production lib, the lib with `test-fixtures`
    and the lib test build.
  - `wow-world` production `unused_imports` 166 → 165.
  - `unused_variables` 13 → 3. The 10 that moved are the pre-existing
    `creature_movement_tick.rs` pathfinding locals, and they are the new crate's only
    warnings.
- Tests: no `#[test]`/`#[ignore]` attribute is added or removed, so the test identities
  are unchanged. The runtime regressions all live in `wow-world`'s scenario suite
  (`session/tests/**`) and use its fixtures; they stay there in 3a.

### Left for 3b

- Move the session-free runtime regressions into `wow-world-creature`. About 22 of the
  83 tests in the 16 scenario files that drive the runtime do not name a session, for
  example `scenarios_world_entities_19/f6_8d2_admitted_execution.rs`. The canonical
  fixtures they use (`shared_canonical_map_manager`, `add_canonical_test_creature_on_map`,
  …) move to `wow-world-test-support` first.
- After that move, drop the `cfg(test)` re-exports that become unused.

## Slice 3b result (session-free runtime scenarios → `wow-world-creature`)

Branch `1363-s3b-creature-tests`, base `78dd5696d` (slice 3a merged). Move-only (R2):
test and fixture bodies are unchanged except for the path rewrites listed below; no
assertion, `#[test]` or `#[ignore]` attribute is added, removed or changed.

### Survey

Every test in the `wow-world` session scenario files that drives the runtime was
classified by its body plus the transitive closure of its file-local helpers. A test
qualifies if neither builds or names a `WorldSession` (`make_session`,
`register_test_creature`, a session-building fixture). 238 runtime tests were
surveyed and 35 qualified. 27 moved:

| From `wow-world` `session/tests/` | Moved | Stayed |
|---|---:|---:|
| `scenarios_world_entities_19/f6_8d2_admitted_execution.rs` (whole file) | 3 | 0 |
| `scenarios_world_entities_31.rs` (whole file) | 9 | 0 |
| `scenarios_world_entities_30.rs` | 9 | 2 |
| `scenarios_world_entities_29.rs` | 2 | 11 |
| `scenarios_world_entities_24.rs` | 2 | 9 |
| `scenarios_world_entities_21.rs` | 1 | 12 |
| `scenarios_world_entities_27.rs` | 1 | 8 |

Eight qualifying tests stayed in `wow-world`:

- Three in `scenarios_world_entities_24.rs` (`creature_spell_runtime_*`, `creature_spell_disable_row_*`).
  They call `creature_spell_bound_hook_tick_fixture_like_cpp`, which builds a session.
- Three in `r1b_incarnation_boundaries.rs` and two in `r1b_incarnation_lifecycle.rs`.
  They share file-local helpers (`canonical_creature_like_cpp`, `corpse_loot_like_cpp`,
  `admission_candidate_like_cpp`, `candidate_loot_like_cpp`) with session tests in the
  same file. Moving them would mean copying those helpers or widening them into a
  shared fixture, which is not a move.

The moved tests live in `crates/wow-world-creature/unit_tests/scenarios/`, keep their
original file names, and are mounted `cfg(test)` from `unit_tests/scenarios.rs`.

### Fixtures

- **Single-phase drivers.** These are the session-free part of
  `session/tests/fixtures/canonical_creatures.rs`: the selection, the five creature phase
  drivers, the player melee driver and `step_creature_movement_like_cpp`.
  - They moved to `crates/wow-world-creature/unit_tests/canonical_creatures.rs`, mounted
    as `pub mod canonical_creatures` under `cfg(any(test, feature = "test-fixtures"))`.
  - `wow-world` reaches them through its fixture prelude.
  - They do not go to `wow-world-test-support`. They name runtime types
    (`AdmittedCreatureExecutionMapLikeCpp`, `PlayerMeleePhaseStateLikeCpp`). If
    test-support depended on `wow-world-creature`, the runtime's own tests would link a
    second copy of the crate through the dev-dependency, and those types would not
    unify.
  - `run_session_player_melee_phase_for_test_like_cpp` needs a session, so it stays in
    `wow-world`.
- **Moved to `wow-world-test-support`** (verbatim, `pub(in crate::session::tests)` → `pub`):
  - `creature_ai`: `aggro_hostile_config_like_cpp`,
    `creature_ai_spell_test_config_like_cpp` and `faction_template_entry`.
  - The new `lifecycle` module: `CreatureMeleeLosTestEnvironment` and its environment
    impl, `melee_los_test_world_object`, `lifecycle_test_map_store_like_cpp` and
    `make_test_world_creature`.
  - `map_objects`: `test_creature_create_data`.
  - `wow-world` imports these back by name. The test-support dependency list is
    unchanged, because every new fixture uses crates it already depends on.
- **Path rewrites.** Each one resolves to the same item:
  - In the drivers, `crate::session::<entry>` → `crate::<entry>`, plus the rustfmt reflow
    this causes in one call.
  - In `make_test_world_creature`, `crate::creature_projection` →
    `wow_world_core::creature_projection`.
  - The moved scenario bodies keep `crate::creature_projection` / `creature_respawn` /
    `creature_movement` / `pathfinder` / `terrain` byte-for-byte. `wow-world-creature`'s
    `lib.rs` has gated `use wow_world_core::{…}` aliases for those five modules, the same
    root re-exports that `wow-world` has.
- **`wow-world-creature` dev-dependencies.** These are not policed by the dependency
  policy (`dev` edges are ignored):
  - `wow-world-test-support`;
  - `wow-recastdetour` with `test-fixtures`, for the navmesh obstacle scenarios;
  - `wow-world-core`, `wow-world-entities` and `wow-world-spell` with `test-fixtures`.
    `wow-world-spell` is needed so its test-fixtures signatures unify with
    `wow-world-entities`'.

### Cleanup and follow-ups from the 3a review

- Dropped from `session/mod.rs`, because only the moved drivers and tests used them:
  - the `cfg(test)` imports of the phase entries, the phase store and
    `is_creature_melee_los_clear_like_cpp`. `creature_ai_can_attack_decision_like_cpp`
    and `creature_melee_spell_miss_threshold_3_3_5_like_cpp` are still used and stay;
  - the `cfg(test)` `creature_spell_admission` re-export module (file deleted).
- `wow-world`'s `test-fixtures` feature now also enables `wow-world-creature/test-fixtures`.
- `forbidden-creature-to-wow-world.json` now reads "…may not depend on wow-world; it
  makes no WorldSession call". The old text claimed the crate inverts through host
  traits, which is not true.
- The 10 pre-existing `unused_variables` in `creature_movement_tick.rs` are left as they
  are (separate change).

### Policy and accounting

- Tests:
  - The workspace `#[test]`/`#[tokio::test]` count is 10215 before and after, and the
    sorted set of test function names is identical. `#[ignore]` is 5 before and after.
  - `wow-world` 3785 → 3758, `wow-world-creature` 0 → 27, and the 27 pass.
- Lines (`git diff --no-renames`):
  - `crates/wow-world/src`: +3 / −20.
  - `crates/wow-world/unit_tests`: +14 / −2565.
  - `crates/wow-world-creature/unit_tests`: +2393.
  - `crates/wow-world-creature/src/lib.rs`: +14.
  - `crates/wow-world-test-support/src`: +273 / −2.
  - `net_move.py check`: S = 17, G = 285 (the moved fixtures, now charged under the
    test-support destination root, plus the `lib.rs` mounts), allowance 978.9,
    0 duplicate bodies, PASS.
- Ceilings are tightened and none is raised:
  - Physical: `session/mod.rs` 1002 → 992; `unit_tests/session_tests.rs` 389 → 387.
  - Hotspot `session/mod.rs`: production 45584 → 45578, test 144750 → 142188, total
    190334 → 187766.
  - Every new file is under the 1,000-line review limit. The largest is
    `scenarios/scenarios_world_entities_30.rs` at 621 lines.
- `dependency-policy.json` is unchanged.
- Warnings:
  - `wow-world` `dead_code` stays 0. The changes are `cfg(test)`-only, and the lib
    test build reports no `wow-world` dead code.
  - No new `wow-world` warning in the lib test build. The two new unused-import
    warnings this move would have caused are removed with the imports.
  - `wow-world-creature` keeps exactly its 10 pre-existing warnings in the production,
    `test-fixtures` and test builds.

### Left for later

- Move the eight qualifying tests that stayed: the five `r1b_*` tests once their shared
  helpers have one home, and the three `creature_spell_runtime_*` tests once a
  session-free variant of the bound-hook fixture exists. Either is a small design step,
  not a move.

## Phase 1 summary (slices 0–3b)

All figures use the hotspot classifier (`hotspot_line_counts`) over the tracked
`crates/wow-world/src/**/*.rs`. It treats top-level `cfg(test)` and
`cfg(any(test, feature = "test-fixtures"))` items as test lines and everything else as
production. It is stricter than the issue's "compiled into production" estimate,
because nested test-only code still counts as production.

| Revision | Files | Physical | Production | Test |
|---|---:|---:|---:|---:|
| `d7e2fa2d5` (phase 1 base) | 552 | 107,370 | 104,326 | 3,044 |
| `43cd093f7` (slice 1 merged) | 543 | 102,133 | 98,700 | 3,433 |
| `d725325b7` (slice 2 merged) | 543 | 101,660 | 98,227 | 3,433 |
| `78dd5696d` (slice 3a merged) | 516 | 91,499 | 88,004 | 3,495 |
| slice 3b head | 515 | 91,482 | 87,998 | 3,484 |

- **Out of wow-world in phase 1:** 15,888 physical lines and 16,328 production lines.
  - Slice 0 changed only visibility.
  - Slice 1: dead and test-only code (−5,237 physical).
  - Slice 2: duplicates retired onto owners (−473).
  - Slice 3a: creature runtime (−10,161).
  - Slice 3b: −17 in `src`, plus 27 tests and their fixtures out of the test tree.
  - The issue planned about 14k for slices 0–3; 3a alone came in at 10.2k.
- **Where it went.**
  - `wow-world-creature` holds 10,226 production lines and 27 tests.
  - `wow-world-test-support` holds 2,266 dev-only lines.
  - Destination growth stayed within budget in every slice. In 3a it was 1.3 % above
    the shrink.
- **Current wow-world production:** 87,998 lines by the classifier. That is about 77k by
  the issue's compiled measure: its ~93.5k baseline minus the 16.3k that left.
- **Phase 2 starts with slice 4a/b, the leaf handler bodies** (void storage, spell,
  entities, travel, movement, LFG, social; about 6.5k). These are the lowest-risk
  remaining moves and need host traits only at small call surfaces.
  - The plan is then loot (5a/b) and quest (6a/b).
  - Before 4a, reassess the plan's per-slice estimates against 3a's actual 10.2k. Keep
    the ≤15 % new-code budget per slice, and split `wow-world-application` by domain
    before it absorbs the handler bodies.

## Slice 4 plan (leaf handler bodies, phase 2)

Measured on `fba425ad1` (phase 1 merged) before anything moved, because the
slice-2 estimate was five times too high. The scan is lexical (the
`wow_world_coupling` blanking and the hotspot classifier), run over every
`crates/wow-world/src/handlers/<family>` directory except the character, loot
and quest families (slices 5, 6 and 8).

- **Classifier production**: what the hotspot classifier counts. It includes
  nested `cfg(test)` shims, so it overstates what can move.
- **Body**: lines of the family's inherent `WorldSession` methods that are not
  `cfg(test)`, `test_shims.rs` or `test_only.rs`.
- **Calls**: distinct `WorldSession` methods those bodies call outside the
  family. This is the host surface if only the family moves.
- **With helpers**: the family plus every `WorldSession` method whose only
  production callers are inside that set (taken transitively), and the host
  surface of that larger set. A body can only move together with such
  single-caller helpers or call them through the host.

| Family | Classifier production | Body | Calls | With helpers: lines / surface |
|---|---:|---:|---:|---:|
| void_storage | 1,641 | 1,377 | 47 | 1,429 / 45 |
| spell | 1,393 | 1,069 | 29 | 1,409 / 35 |
| entities (gameobject use, corpse) | 1,030 | 758 | 39 | 2,508 / 36 |
| movement (+ `movement_host.rs`) | 1,017 | 629 | 16 | 1,299 / 22 |
| travel | 777 | 667 | 31 | 771 / 31 |
| chat | 511 | 136 | 2 | 135 / 2 |
| group | 463 | 118 | 14 | 121 / 13 |
| dungeon_finding | 412 | 330 | 9 | 380 / 7 |
| collections | 373 | 266 | 15 | 276 / 15 |
| battlegrounds | 314 | 43 | 2 | 183 / 2 |
| trainer | 280 | 37 | 1 | 399 / 4 |
| combat | 266 | 210 | 6 | 505 / 9 |
| talent | 231 | 98 | 7 | 368 / 21 |
| social, vehicle, pets, economy, account_data, guild, calendar, support, battlenet, progression, instances | 803 | 353 | ≤ 4 each | ≤ 261 each |

Findings:

- **About 6.5k lines can move, as the issue estimated.** The bodies are 6.1k
  lines, and the single-caller helpers add about 3k, mostly the gameobject-use
  operations in `session/world_entities/gameobject_use*.rs`. The classifier
  figure is higher because about 1.4k of it is nested test shims, which stay.
- **Many small families are already shells.** F5 moved their bodies to
  `wow-world-social` and `wow-world-application` behind handler-level host
  traits (`*HandlerHostLikeCpp`). What is left is the registrar adapter plus
  `cfg(test)` entry points, for example guild, calendar, support, battlenet and
  economy.
- **Cost of a host method.** One trait line block (about 4 lines after rustfmt)
  plus one adapter entry. A forwarding macro brings the adapter entry down to 1–3
  lines (see 4a). At about 6 lines per method, a family stays under the 15 %
  budget only if it moves at least 40 lines per host method.
- **Two things block a move.**
  - A body that uses a `wow-world`-only or `wow-world-application`-only type
    (`QuestObjectiveOperationLikeCpp`, `RepresentedPreparedQuestMenuItemLikeCpp`,
    `RepresentedPlayerConditionContextLikeCpp`, `AppTrainerListCx`) cannot go
    to a domain crate. It stays behind the host instead.
  - A `#[cfg(test)]` block inside a body changes meaning when the body moves,
    because `cfg(test)` is then the domain crate's own test build, not
    `wow-world`'s. Such a body either stays with the host, or its test-only step
    becomes a host hook whose adapter keeps the `cfg(test)` block.

**4a** takes the lowest surface per line. It covers:
- gameobject use with its single-caller helpers;
- combat: the player attack start and the melee delivery;
- the text emote;
- the LFG lock info.

That is about 3.5k lines with 38 domain host methods (see the 4a result). The
corpse handlers were planned for 4a and taken out again. They needed 9 host
methods (teleport, vitals, the resurrection request and a test hook) for about
200 lines, which would have put 4a over the 15 % budget.

**4b** takes the rest, in this order:
1. Movement (1.3k / 22), and the corpse handlers if movement's teleport and
   vitals surface also covers what corpse needs.
2. Vehicle, trainer and talent. Trainer goes to `wow-world-application`
   because its contexts (`AppTrainerListCx`) live there.
3. Battlegrounds. Its two `cfg(test)` fixture pushes become host hooks.
4. Collections and group.
5. Travel, spell and void storage. These have the largest surface (31–45).
   They mostly call inventory, loot and lifecycle operations that slices 5 and
   8 will move, so if the 15 % budget does not hold they wait for those slices.

## Slice 4a result (gameobject use, player attack, text emote, LFG lock info)

Branch `1363-s4a-leaf-handlers`, base `fba425ad1` (phase 1 merged). The move is
move-only (R2): bodies are unchanged apart from the rewrites the inversion
needs, so no C++ re-audit applies. Those rewrites are:
- `self.` becomes `host.`;
- `self.<field>` becomes `host.<field>()` or `host.<field>_mut()`;
- `crate::session::hub_*(self)` and `split_*(self)` become host calls;
- a call to another moved method becomes a free call;
- crate-local paths become the destination's paths.

Packets, their order and persistence are unchanged.

### What moved

- **To `wow-world-entities` (3.0k body lines).**
  - `CMSG_GAME_OBJ_USE` and the gossip interaction check (`gameobject_use_dispatch.rs`).
  - The use preamble and the post-use spell (`gameobject_use_preamble.rs`).
  - The PvP, scripted and per-type gameobject uses (`gameobject_use_pvp.rs`,
    `gameobject_use_scripted.rs`, `gameobject_use_kinds.rs`).
  - Their helpers (`gameobject_use_support.rs`; most had only these callers,
    and a few are still also called from `wow-world` as free functions): the battleground
    object gates, the item forge, the fishing skill base, stealth removal, the
    new-flag and capture-point records, the owner record, the friendliness check
    and the delete/despawn publication.
  - The player attack start, the melee-result delivery and the target-visibility
    context (`player_attack.rs`).
  - The constants and the result enum only these bodies used:
    `SKILL_FISHING_LIKE_CPP`, `PLAYER_FLAGS_UBER_LIKE_CPP` and
    `PlayerAttackStartLikeCppResult`. The battleground ids now come from
    `wow_data::battlemaster_list`, where the same values were already defined.
- **To `wow-world-social` (0.5k body lines).**
  - `CMSG_SEND_TEXT_EMOTE` (`text_emote.rs`).
  - `CMSG_DF_GET_SYSTEM_INFO` with the LFG lock and reward info
    (`lfg_lock_info.rs`).
- **`wow-world-application` is unchanged:** 50,861 lines in `src`, +0 / −0. No
  body needed an application-only type.
- **Registrations do not move.** The F5 registrars and their handler-level
  host traits stay. Their `wow-world` adapters now call the moved body,
  `wow_world_entities::…(self, …)` or `wow_world_social::…(self, …)`, where they
  used to call the inherent method.

### Host surface

- **`wow_world_core::session::SessionHubHostLikeCpp`** (new, 36 lines). This is
  the shared base for every slice-4 host. It has two required methods, `hub_ref`
  and `hub_mut`, and provides `core`, `core_mut`, `player_guid`, `send_packet`,
  `send_raw_packet` and `state`. Each provided method forwards exactly as the
  session's inherent method of the same name does.
- **`WorldEntitiesHostLikeCpp`** (30 methods):
  - the world-entity state accessors and splits;
  - two loot-queue hooks and the quest-source record;
  - loot opening, quest menus, kill credit, auras and mounted-aura removal;
  - the group, duel and phase reads, the map selection and the unit VALUES
    update;
  - one cfg(test) fallback hook.
- **`SocialSessionHostLikeCpp`** (8 methods): the aura interrupt, the instance
  lock, the achievement and item reads, the quest snapshot, and the quest money
  and XP rewards.
- **Adapters are written out in full.** A forwarding macro would have saved
  about 150 lines, but the session-ownership grammar rejects macros inside
  `impl … for WorldSession`. The adapters are `handlers/entities/host.rs` (277
  lines), the `SocialSessionHostLikeCpp` impl in
  `session/chat_handler_contexts.rs`, and the base impl in `session/state/hub.rs`.
- **`cfg(test)` code stays in `wow-world`.** Moving a body would make `cfg(test)`
  mean the destination crate's own test build. So:
  - the attacker-pvp-flags fallback became a host hook whose adapter keeps the
    `cfg(test)` block;
  - the resurrection-request helpers, the mounted-aura removal and the
    quest-objective, quest-menu and graveyard-condition helpers stay in
    `wow-world` behind the host, because they hold `cfg(test)` fixture code or
    `wow-world`-only types.
- **One Arc clone.** The meeting-stone body read the disjoint `catalogs` field
  while mutating world-entity state. It now clones the content-tuning store Arc
  first. Values and order are unchanged.

### Accounting

- **`crates/wow-world/src`.** `git diff --numstat`: −3,646 / +469 (net −3,177).
  The classifier goes from 91,482 physical, 87,998 production and 3,484 test
  lines in 515 files to 88,305, 84,815 and 3,490 in 513 files. Production is
  −3,183.
- **Destinations.**
  - `wow-world-entities/src` +3,387: about 3,020 moved body lines, 178 lines of file
    headers, 58 lines of `lib.rs` mounts and exports and the 136-line host trait.
  - `wow-world-social/src` +590: about 510 moved, 40 header, 15 `lib.rs` and the
    23-line host trait.
  - `wow-world-core/src` +38: the base trait.
- **Budget.** New host code (the three traits plus the adapters) is about 514
  lines against 3,536 moved body lines, 14.5 %.
  - `net_move.py check`: S = 3,177, G = 4,015, PASS, 0 duplicate bodies.
  - G − S also counts the destination headers and export lists, and the rustfmt
    reflow of `host.x_mut()` chains and generic signatures.
- **Policy.**
  - `dependency-policy.json`: `wow-world-social` may depend on `wow-data` (the
    emote and LFG stores), and `wow-world-entities` may use the external `rand`
    crate (the ritual target pick). `wow-world`, `-core` and `-application`
    already had it.
  - `session-ownership-policy.json` is reprinted with `print-baseline`.
- **Ceilings.** All are tightened and none is raised:
  - physical `handlers/chat.rs` 77 → 63 (the module is now mounted only for
    the test build);
  - physical `session/mod.rs` 992 → 989;
  - hotspot `session/mod.rs`: production 45,578 → 43,785, test 142,188 →
    142,050, total 187,766 → 185,835.
  - The character and loot hotspot owners are unchanged. Their call sites keep
    method syntax through three cfg(test) shims in `handlers/entities/gameobject.rs`.
- **Warnings.**
  - `wow-world` `dead_code` stays 0 in the production lib, the lib with
    `test-fixtures` and the lib test build.
  - `unused_imports` falls: 165 → 154, 174 → 163 and 335 → 324.
  - `wow-world-entities`, `-social` and `-core` keep exactly their pre-existing
    warnings.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed; the workspace
  test-name multiset is identical before and after. The scenario tests stay in
  `wow-world`, and their call sites become `wow_world_entities::f(&mut session, …)`.
  One source-scan regression,
  `uncage_item_state_seam_no_longer_names_statement_or_driver_errors`, used the
  moved item-forge method as its end delimiter. It now ends at the impl block.
  (4b replaces that with the helper's own impl-level closing brace.)

## Slice 4b result (movement, vehicle, corpse, arena skirmish)

Branch `1363-s4b-leaf-handlers`, base `4fe68ef71` (4a merged). Move-only (R2),
with the same mechanical rewrites as 4a. Packets, their order, persistence,
registrations and processing flags are unchanged. One new rewrite: a former
`#[cfg(test)]` fixture step inside a moved body becomes

```rust
#[cfg(any(test, feature = "test-fixtures"))]
if let Some(fixtures) = host.test_fixtures_like_cpp() { /* same step */ }
```

`SessionHubHostLikeCpp::test_fixtures_like_cpp` is a provided method that returns
`None`; only the World session overrides it, and only under its own `cfg(test)`,
to return its fixtures. The step therefore runs exactly when it ran before, in
the World test build, and in no other build. That one shared accessor replaces
the per-step hooks 4a needed, so a body with fixture steps no longer costs a host
method per step.

### What moved

- **To `wow-world-entities` (about 1.6k body lines).**
  - `movement_opcodes.rs`: `CMSG_MOVE_*` (`HandleMovementOpcode`), the
    movement side effects, `CMSG_MOVE_SPLINE_DONE` and the plain movement ACK.
    The three aura-interrupt flags they use moved with them; `wow-world`
    re-exports the one it still needs.
  - `movement_acks.rs`: the validated, force, knockback, speed and
    force-magnitude ACKs, `Player::ValidateMovementInfo` sanitisation,
    `Player::HandleFall`, the under-map step, the jump-proc request and the
    spline/taxi completion with its event record.
  - `corpse_handlers.rs`: `CMSG_REQUEST_CEMETERY_LIST`, `CMSG_RESURRECT_RESPONSE`,
    `CMSG_REPOP_REQUEST` and `CMSG_RECLAIM_CORPSE`. The graveyard condition check
    stays in `wow-world` behind the host because it needs the World
    player-condition context.
- **To `wow-world-social` (about 0.2k).** `battlemaster_skirmish.rs`:
  `CMSG_BATTLEMASTER_JOIN_SKIRMISH` and `CMSG_ACCEPT_WARGAME_INVITE`, beside the
  arena-team family F5 already moved there.
- **To `wow-world-application` (about 0.3k), justified.** `vehicle_session.rs`:
  the four seat, dismiss and ride-interact bodies. Their branch decisions
  (`VehicleHandlerAction` and the `*_action_like_cpp` functions) and the
  spell-click plan (`RepresentedSpellClickPlanLikeCpp`) are application types,
  and the vehicle registrar and its context already live there. Slice A can move
  the whole vehicle module to a domain crate in one step.
- **To `wow-world-core` (24 lines).** `PlayerGridLoadOutcomeLikeCpp` and
  `PlayerGridLoadResolverLikeCpp`, which the movement body reads; `wow-world`
  re-exports them at the old path.

### Host surface

- **`PlayerMovementHostLikeCpp`** (entities, 11 methods plus an associated
  type for the World area-trigger catalogs, whose script dispatcher takes the
  session): aura queries and interrupts, zone, exploration, visibility and
  area-trigger updates, durability loss, alive state, registry sync and the
  taxi node map ids.
- **`CorpseHostLikeCpp: PlayerMovementHostLikeCpp`** (entities, 7): the
  graveyard check, the three resurrection-request operations (their
  `cfg!(test)` return logic stays in `wow-world`), teleport, pending teleport
  and the ghost flag.
- **`VehicleSessionHostLikeCpp: PlayerMovementHostLikeCpp`** (application, 2):
  the spell-click plan and the same-raid check.
- **`SocialSessionHostLikeCpp`** gains `resolved_group_guid_like_cpp`.
- **`SessionHubHostLikeCpp`** gains `test_fixtures_like_cpp`.
- **Recursion check.** Several moved names equal registrar trait methods
  (`handle_accept_wargame_invite`, the vehicle `handle_*` entries, the movement
  ACK operations). Every such adapter now calls the destination path
  (`wow_world_social::…`, `wow_world_application::…`, `wow_world_entities::…`),
  never `WorldSession::name`, so no forward can resolve to its own trait method.
  Every host forward still has an inherent method of the same name.
- **Tests.** The movement, corpse and battleground scenarios outside the session
  hotspot call the moved functions by name (`use wow_world_entities::{…}`). The
  session-hotspot scenarios keep method syntax through `cfg(test)` shims in
  `handlers/vehicle/test_shims.rs` and `handlers/movement/test_shims.rs`, so the
  session hotspot's test lines do not grow.

### Accounting

- **`crates/wow-world/src`.** `git diff --numstat`: −2,243 / +362 (net −1,881).
  The classifier goes from 88,305 physical, 84,815 production and 3,490 test lines
  in 513 files to 86,424, 82,922 and 3,502 in 514 files. Production is −1,893.
- **Destinations** (moved body lines / host lines).
  - `wow-world-entities/src` +1,859 −3: about 1,610 moved, 78 trait lines.
  - `wow-world-social/src` +244 −4: about 200 moved, 1 trait line.
  - `wow-world-application/src` +364: about 280 moved, 12 trait lines.
  - `wow-world-core/src` +32: 24 moved type lines, 8 accessor lines.
  - The rest is file headers, `lib.rs` mounts and exports, and rustfmt reflow.
- **`wow-world-application`: 50,861 → 51,225 lines (+364).** Only the vehicle
  bodies, for the reason above.
- **Budget.** About 2,120 moved body lines against about 270 new host lines
  (99 in traits and the accessor, 173 in the `wow-world` adapters
  `session/movement/host.rs`, `handlers/entities/corpse/host.rs`,
  `session/vehicle_handler_contexts.rs`, `session/chat_handler_contexts.rs` and
  `session/state/hub.rs`): 12.8 %. `net_move.py check`: growth 2,492 within
  the 2,936 allowance, 0 duplicate bodies.
- **Policy.**
  - `dependency-policy.json`: `wow-world-entities` may depend on `wow-anticheat`
    (`ValidateMovementInfo`; a two-dependency domain crate).
  - `wow-world` dev-dependencies enable `wow-world-social/test-fixtures`, so the
    moved fixture steps compile in the World test build.
  - `session-ownership-policy.json` is reprinted with `print-baseline`; the delta
    is exactly the moved items and the new adapters and shims.
- **Ceilings.** None raised; tightened:
  - physical `session/mod.rs` 989 → 977, `handlers/movement.rs` 340 → 44;
  - hotspot `session/mod.rs`: production 43,785 → 42,633, total 185,835 →
    184,683, test unchanged at 142,050.
- **Warnings.** `wow-world` `dead_code` stays 0 in the production lib, the lib
  with `test-fixtures` and the lib test build. `unused_imports` falls 154 → 152,
  163 → 161 and 324 → 321. The destination crates keep exactly their
  pre-existing warnings.
- **Tests.** No `#[test]` or `#[ignore]` added or removed; every changed
  assertion is the same check through the free function.
- **4a review follow-ups.** `EMOTE_ONESHOT_NONE_LIKE_CPP` is defined once
  (`chat_handlers.rs`); the stray blank line in
  `session/world_entities/gameobject.rs` is gone; the uncage source scan ends at
  the helper's own closing brace; the post-use spell `_internal` helper is
  crate-private and no longer re-exported.

### Deferred to 4c

Each of these would push the slice over the 15 % budget on its own, or has no
movable body yet:

- **Talent** (~210 movable lines, 12+ host methods). The reset commit and
  publication carry `cfg(test)` persistence-bypass logic on lifecycle state, and
  the learn path calls six spell-book operations. It is cheaper after slice 8
  (character/lifecycle).
- **Trainer** (nothing to move). The handlers are three-line forwards; the
  remaining code is the `AppTrainerListCx` field-borrow builder, which needs
  the session's disjoint fields. It belongs to slice A's context work.
- **Collections** (~300 lines, 11 inventory and spell-cast host methods) and
  **group** (~130 lines, 9 host methods including the social split and the
  application difficulty context).
- **Travel, spell, void storage** (surfaces 23–38). Their bodies call the quest,
  inventory and loot operations that slices 5, 6 and 8 move; after those, most
  of that surface becomes free calls.

### Sizes for slice A

- `wow-world` production (classifier): **82,922** lines (86,424 physical, 514
  files).
- `wow-world-application`: **51,225** physical lines (49,716 production, 170
  files). The vehicle module (`vehicle_handlers.rs` 529 + `vehicle_session.rs`
  357) is one obvious unit for the split.

## Slice A plan (split `wow-world-application` by domain)

Measured on `465f3791a` (slice 4b merged) before anything moved:
`wow-world-application` is 51,225 physical lines in 170 files (49,716
production). The scan is lexical: for every top-level module of the crate it
resolves `crate::…` paths (including names re-exported from `lib.rs`) to the
owning module, and lists the `wow_*` crates the module names.

### Internal graph

- **One strongly connected cluster, held together by inherent impls.**
  `PlayerConditionProjectionCxLikeCpp` is defined in `player_conditions` and
  extended by `impl` blocks in `quest/visibility.rs` and
  `inventory_valuation{,/direct_storage}.rs`. Rust only allows those impls in
  the defining crate, so `quest` (10,778), `player_conditions` (690) and
  `inventory_valuation` (1,280) can only move together. `player_conditions`
  also names `TrainerAdmissionProofLikeCpp` and
  `trainer_condition_admission_proof_like_cpp` (`trainer_purchase/offer.rs`);
  that one enum and function are the only edge back from the cluster to the
  trainer family, and they move down with the cluster.
- **Shared support leaves** used by the cluster and by its consumers: `stats`
  (677, 8 dependents), `registry_sync` (211, 7 dependents), `inventory_scaling`
  (220). They have no in-crate dependencies except each other.
- **Quest consumers.** Modules that name the cluster:
  - `loot_release` (2,470, used by `loot_handlers` 1,957);
  - `aura_removal` (1,586; quest state and condition inputs);
  - `trainer_purchase` (3,699; aura removal, conditions, quest events,
    spell acquisition, profession);
  - `spell_handlers` (896) and `player_handlers` (644), which only borrow
    `SessionQuestState`;
  - `quest_handlers` (373) and `quest_query_handlers` (937);
  - `inventory_move_planning` (323) and `inventory_swap` (2,566), through the
    condition context and the valuation functions;
  - `spell_click_values` (133), through one five-line enum
    (`RepresentedCanSeeSpellClickOutcomeLikeCpp`, defined in
    `quest/visibility.rs`).
- **Leaves.** Every other module has no dependency on another application
  module, or only on `stats` / a sibling of its own family: all handler
  registrars outside the families above, `instances`, `movement_handlers`,
  the vehicle pair, `client_state`, the character family, `player_save`,
  `profession` and `spell_acquisition` (which only uses `profession`).

### Destinations

A domain crate may not depend on `wow-world-application` or `wow-world`; every
new edge below was checked against the current manifests for a cycle (none of
the targets depends on the source). Lines are physical.

| Module(s) | Lines | Destination | New edges |
|---|---:|---|---|
| `vehicle_handlers`, `vehicle_session`, `spell_click_values` (+ the spell-click outcome enum from `quest/visibility.rs`) | 1,019 | `wow-world-entities` (spell click, movement host and vehicle state already live there) | entities → `wow-handler` |
| `movement_handlers` (+ `tail_registrations`) | 1,176 | `wow-world-entities` (the movement bodies moved there in 4b) | (same) |
| `instances` | 1,245 | `wow-world-instances` (as `crate::handlers`) | instances → `wow-handler`, `wow-persistence`, `wow-world-lifecycle`, `wow-world-social` |
| `trade_handlers`, `guild_bank_handlers`, `reputation` | 2,529 | `wow-world-social` | social → `wow-world-inventory`, `wow-world-spell`, `wow-world-entities` (re-derived after A1; see *Plan fixes*) |
| `group_handlers`, `battleground_handlers` | 2,813 | `wow-world-instances` (not social: A1 added instances → social, so social → instances would be a cycle) | instances → `wow-world-loot`, `wow-world-entities`, external `rand` |
| `travel_handlers`, `combat_handlers`, `corpse_handlers`, `gameobject_handlers` | 969 | `wow-world-entities` | entities → `wow-world-lifecycle`, `wow-world-interaction` |
| `dungeon_finding_handlers` | 222 | `wow-world-instances` | — |
| `client_state`, `data_service_handlers`, `player_save` | 2,101 | `wow-world-lifecycle` | — |
| `npc_handlers` | 207 | `wow-world-interaction` | interaction → `wow-handler` |
| `stats`, `registry_sync`, `inventory_scaling`, `equipment_set_use`, `bank`, `vendor`, `collections_handlers`, `battle_pet_handlers` | 3,587 | `wow-world-inventory` | inventory → `wow-world-loot`, `wow-world-spell`, `wow-world-lifecycle`, `wow-world-interaction` |
| `profession`, `spell_acquisition` | 2,728 | `wow-world-spell` | spell → `wow-world-lifecycle`, `wow-persistence`, `wow-config`, external `rand` |
| `character_handlers`, `character_handler_families`, `character_creation`, `character_enumeration`, `character_login_support`, `character_query_handlers` | 3,054 | **new `wow-world-character`** (slice 8 adds ~12.5k; > 3k and one C++ family, `CharacterHandler.cpp` plus login). Needs A4 first (`character_creation` and `character_handler_families` use `stats`). | — |
| `quest`, `quest_handlers`, `quest_query_handlers`, `player_conditions`, `inventory_valuation` (+ the trainer admission proof) | 14,058 | **new `wow-world-quest`** (no existing crate fits; one cluster) | — |
| `loot_handlers`, `loot_release` | 4,427 | **new `wow-world-loot-ops`**: loot *operations* above the quest crate. `wow-world-loot` stays the loot *state* owner; it cannot take them, because `loot_release` calls the quest cluster and the quest cluster depends on `wow-world-loot`. | — |
| `aura_removal`, `trainer_purchase`, `trainer_handlers`, `talent_handlers`, `spell_handlers`, `player_handlers`, `inventory_swap`, `inventory_move_planning`, `character_account_handlers`, `lib.rs` | ~11.1k | **stays** in `wow-world-application` | — |

What stays is cross-domain orchestration: the aura-removal pipeline (item
effects, shapeshift, threat, mount, quest state), trainer purchase (money,
quest events, aura, spell acquisition, conditions), the inventory swap and
move planner (equip rules, quest item counts, conditions, stats), the
spell/player handlers that lend quest state, and the account-handler registrar
that composes bank, vendor, inventory and trainer. A later `wow-world-trainer`
(trainer + talent, ~3.9k) becomes possible once `aura_removal` no longer needs
the quest state; that is re-measured after A6, not planned now.

### Plan fixes (A1 review, re-derived on `1c5c277cc`)

The A1 review found that the destination table's cycle check had run against
the pre-A1 manifests. Re-derived against the post-A1 graph (`cargo metadata`
normal dependencies; `wow-world-*` edges only):

```
entities   → core                     lifecycle → core, visibility
instances  → core, lifecycle, social  social, inventory, spell, loot,
creature   → core, entities, spell      interaction, visibility → core
```

- **The A2 cycle.** A1 made `wow-world-instances` depend on
  `wow-world-social` (`SessionSocialLimits` in `handlers/context.rs`). The old
  A2 row sent `group_handlers` and `battleground_handlers` (both name
  `wow_world_instances::InstanceState`) to social, which would add
  social → instances: a cycle. `SessionSocialLimits` is **not** plain data:
  its fields are `pub(crate)` and typed by social contracts and fixtures, and
  social extends it with inherent `impl` blocks in ten modules (`state`,
  `trade`, `duel`, `duel_publication`, `group`, `group_owner`, `chat`,
  `guild`, `catalogs`, `requests`), which Rust only allows in the defining
  crate. Moving
  it to `wow-world-core` would therefore not be move-only. Instead **group and
  battleground go to `wow-world-instances`** (they move together:
  `battleground_handlers` calls `group_handlers::resolved_group_guid_like_cpp`).
  Their crate uses are `wow-world-{core,instances,lifecycle,loot,social,
  entities}`, `wow-social`, `wow-data`, `wow-entities` and `rand`; the new
  instances edges are `wow-world-loot`, `wow-world-entities` and external
  `rand`. Trade, guild bank and reputation still go to social; their new
  social edges are `wow-world-inventory`, `wow-world-spell` and
  `wow-world-entities` (reputation needs none). No target reaches instances or
  social, so neither edge set closes a cycle.
- **A3–A7 re-checked** against the post-A2 graph (instances → loot, entities;
  social → inventory, spell, entities), applying the rows in order:
  - A3: entities → lifecycle, interaction. Neither reaches entities, social or
    instances. `dungeon_finding_handlers` (instances), `client_state`,
    `data_service_handlers`, `player_save` (lifecycle) and `npc_handlers`
    (interaction, + `wow-handler`) add no `wow-world-*` edge.
  - A4: inventory → loot, spell, lifecycle, interaction. Social now depends
    on inventory, so inventory must never reach social or instances; none of
    the four does (spell gains only lifecycle in A5).
  - A5: spell → lifecycle (+ `wow-persistence`, `wow-config`, `rand`).
    Lifecycle reaches only core and visibility; social and creature depend on
    spell, and spell reaches neither. The character family names `stats`, so
    A5's `wow-world-character` half needs A4 first. Nothing depends on the new
    crate.
  - A6: `wow-world-quest` → core, entities, instances, inventory, lifecycle,
    loot, social, spell. Nothing below depends on it.
  - A7: `wow-world-loot-ops` → quest and the same set. Nothing below depends
    on it.
- **Naming.** The planned `wow-world-looting` becomes **`wow-world-loot-ops`**:
  `loot` vs `looting` is a naming hazard; the suffix names the layer (loot
  operations above quest; `wow-world-loot` stays the loot state owner).
- **`wow-world-character` sits high.** Nothing below it (the domain crates,
  `wow-world-core`) may depend on it; only `wow-world-application` and
  `wow-world` may. Slice 8 moves bodies into visibility, interaction,
  entities and inventory as well as into character, so it needs its own
  cycle pre-check against the graph of that day before any move.
- **Before A6**, check that the quest cluster (`quest`, `quest_handlers`,
  `quest_query_handlers`, `player_conditions`, `inventory_valuation`) needs
  nothing that still stays in application: in particular `aura_removal`,
  `trainer_purchase` (beyond the admission proof that moves down with the
  cluster) and `inventory_swap`. On `1c5c277cc` the lexical scan finds
  `quest` → `player_conditions`, `registry_sync`, `stats`;
  `player_conditions` → `inventory_valuation`, `quest`, `trainer_purchase`
  (the proof only); `inventory_valuation` → `player_conditions`. Re-run it on
  the A6 base; any other edge must be cut first or the cluster boundary
  re-planned.

The verification command for every slice: transitive closures over the
`wow-world*` packages from the head `Cargo.lock` / `cargo metadata`; no
domain crate may reach `wow-world-application` or `wow-world`, and no package
may reach itself.

### PR sequence

Each PR is move-only (R2), passes `net_move.py check`, keeps the test-name
multiset, and updates the handler-contract checker's registrar package/paths
and `handler-module-policy.json` for every moved registrar.

| PR | Content | Lines | Risk |
|---|---|---:|---|
| **A1** | vehicle + spell-click values + movement registrars → entities; instances → instances | ~3.4k | low: leaves, no new crate |
| A2 | trade, guild bank, reputation → social; group, battleground → instances | ~5.3k | low–medium: three new social edges, two new instances edges (+ `rand`) |
| A3 | travel/combat/corpse/gameobject → entities, dungeon finding → instances, client state/data service/player save → lifecycle, npc → interaction | ~3.5k | low |
| A4 | support layer → inventory (stats, registry sync, scaling, equipment-set use, bank, vendor, collections, battle pets) | ~3.6k | medium: shared by the quest cluster and every consumer |
| A5 | spell acquisition + profession → spell; character family → new `wow-world-character` | ~5.8k | medium |
| A6 | new `wow-world-quest` | ~14.1k | medium: one large mechanical move, like 3a |
| A7 | new `wow-world-loot-ops` | ~4.4k | medium |

After A7 the application is ~11k lines of orchestration.

### Where the remaining wow-world slices land

- **Slice 5 (loot)**: loot handler bodies and release/roll operations →
  `wow-world-loot-ops`; pure loot state → `wow-world-loot`. Needs A6 and A7.
- **Slice 6 (quest)**: → `wow-world-quest`. Needs A6.
- **Slice 8 (character)**: login load and character handlers →
  `wow-world-character`; inventory moves and vendor → `wow-world-inventory`;
  visibility, gossip and transports → `wow-world-visibility`,
  `wow-world-interaction` and `wow-world-entities`. Needs A4 and A5.
  Slice 8 needs its own cycle pre-check (see *Plan fixes*): it moves bodies
  into crates that sit below `wow-world-character`, and none of them may gain
  an edge back to it.

## A1 result (vehicle, spell-click values, movement registrars, instances)

Branch `1363-sA1-application-split`, base `465f3791a` (4b merged). Move-only
(R2): every moved body is byte-identical; only `use` paths, `crate::` prefixes
and crate-root re-exports change. Opcode registration and dispatch order are
unchanged (`handler_composition.rs` calls the same registrars in the same
order, through their new crate paths).

### What moved

- **To `wow-world-entities`** (2,197 file lines):
  `movement_handlers.rs` (940) with `movement_handlers/tail_registrations.rs`
  (236), `vehicle_handlers.rs` (529), `vehicle_session.rs` (357) and
  `spell_click_values.rs` (135). The five-line
  `RepresentedCanSeeSpellClickOutcomeLikeCpp` enum moves from
  `quest/visibility.rs` into `spell_click_values.rs`, as the plan says; the
  quest cluster now imports it from entities. The two
  `wow_world_spell::spell_cast_adapter` re-exports that
  `spell_click_values` carried are dropped there (entities does not depend on
  `wow-world-spell`); their two `wow-world` users import them from
  `wow_world_spell` directly.
- **To `wow-world-instances`** (1,245 file lines): `instances/**` becomes
  `crate::handlers` (`context`, `difficulty`, `lock_operations`, `raid_info`,
  `registration`, and the registration test, which needs no `WorldSession`).
- **Callers.** `wow-world` (`handler_composition.rs`, the movement and vehicle
  host adapters, `session/instance_handler_contexts.rs`, `session/mod.rs`,
  `session/spell_click_values.rs` and three unit-test shims) name the new crate
  paths. The handler-contract checker's registrar package/module constants and
  source mounts, and `handler-module-policy.json`, follow the four moved
  registrars (`ApplicationInstances`, `ApplicationVehicle`,
  `ApplicationMovement`, `ApplicationMovementTail`).

### Accounting

- **`wow-world-application/src`: 51,225 → 47,743 lines (−3,482), 170 → 158
  files.** 3,440 lines leave as files; `lib.rs` loses 35 net export lines and
  `quest/` 7 (the enum and its re-exports).
- **Destinations.** `wow-world-entities/src` 10,588 → 12,808 (+2,220: 2,197
  moved, 23 `lib.rs` mounts/exports). `wow-world-instances/src` 1,268 → 2,524
  (+1,256: 1,245 moved, 11 `lib.rs` lines).
- **`crates/wow-world/src`**: +35 −39 (rustfmt folds two vehicle match arms
  onto one line each). `net_move.py check`: growth −6 within the 965.2
  allowance, 0 duplicate bodies; destination roots unchanged (13).
- **Policy (`dependency-policy.json`).** `wow-world-entities` += `wow-handler`
  (the registrars build `PacketHandlerEntry`s). `wow-world-instances` +=
  `wow-handler`, `wow-persistence`, `wow-world-lifecycle`,
  `wow-world-social` and external `tracing`: exactly the crates the moved
  instance bodies already named; none of them depends on instances, so no
  cycle. `wow-world-application` −= `wow-instances` (only `instances/` used
  it). Neither domain crate depends on `wow-world-application` or `wow-world`.
  `session-ownership-policy.json` is reprinted; the delta is only the
  `vehicle_session` package/source path and the `VehicleSessionHostLikeCpp`
  trait path (now `wow_world_entities`), with the resulting sort order.
- **Ceilings.** None raised. Hotspot `session/mod.rs` tightened to the live
  values: production 42,633 → 42,629, total 184,683 → 184,679 (test unchanged).
  Physical `session/mod.rs` stays at 977.
- **Warnings.** `wow-world` `dead_code` is 0 in the production lib, the lib
  with `test-fixtures` and the lib test build; `unused_imports` stays 152 and
  161 in the first two, and no changed line in any build warns. The two
  warnings inside the moved files (`tail_registrations.rs` `ClientPacket`,
  `spell_click_values.rs` `ObjectGuid`/`Position`) are the same lines that
  warned in `wow-world-application`.
- **Tests.** The test-name multiset is unchanged: the one moved test
  (`registrar_adds_entries_without_constructing_host_and_rejects_duplicate`)
  runs in `wow-world-instances` (application 59 → 58, instances 0 → 1). No
  `#[test]` or `#[ignore]` is added or removed elsewhere.

### Next

A2 (social registrars) per the PR sequence above. Re-measure application
before it; the plan's line figures are from `465f3791a`.

## A2 result (trade, guild bank, reputation → social; group, battleground → instances)

Branch `1363-sA2-social-split`, base `1c5c277cc` (A1 merged). Move-only (R2):
every moved body is byte-identical; only `use` lines, self-crate prefixes
(`wow_world_social::` / `wow_world_instances::` → `crate::`) and crate-root
re-exports change, and rustfmt re-sorts the two moved `use crate::InstanceState`
lines. Opcode registration and dispatch order are unchanged
(`handler_composition.rs` calls the same registrars in the same order, through
their new crate paths). The plan was fixed first (*Plan fixes* above): group and
battleground go to instances, not social.

### What moved

- **To `wow-world-social`** (2,529 file lines): `trade_handlers.rs` (1,363),
  `guild_bank_handlers.rs` (859) and `reputation.rs` (307), with their root
  re-exports (the trade/duel helpers `player_trade_state_snapshot_like_cpp`,
  `set_represented_duel_state_like_cpp`,
  `set_represented_duel_arbiter_guid_like_cpp` and
  `resolved_represented_duel_arbiter_guid_like_cpp` included).
- **To `wow-world-instances`** (2,813 file lines): `group_handlers.rs` (1,834)
  and `battleground_handlers.rs` (979), with `GroupPublicationTailLikeCpp` and
  `resolved_group_guid_like_cpp` (battleground calls it as
  `crate::resolved_group_guid_like_cpp`, which now resolves inside instances).
- **Callers.** `wow-world` (`handler_composition.rs`, the battleground, guild,
  trade, reputation and group host adapters, `session/social/{trade,duel,group}.rs`,
  four test shims, `unit_tests/handler_contract_tests.rs` and the duel
  `f3_shims.rs`) name the new crate paths. The handler-contract checker's five
  registrar packages and source mounts, and `handler-module-policy.json`,
  follow the moved registrars (`ApplicationTrade`, `ApplicationGuildBank`,
  `ApplicationReputation`, `ApplicationGroup`, `ApplicationBattleground`; the
  owner labels are unchanged).
- **A1 follow-ups.** The stale "moved to `wow-world-application`" module
  comments in `handlers/vehicle/test_shims.rs` and
  `session/spell_click_values.rs` now name `wow-world-entities`; the A2 host
  adapters' comments name their new owner crate.

### Accounting

- **`wow-world-application/src`: 47,743 → 42,374 lines (−5,369), 158 → 153
  files.** 5,342 lines leave as files; `lib.rs` loses 27 (mounts and exports).
- **Destinations.** `wow-world-social/src` 8,764 → 11,309 (+2,545: 2,529 moved,
  16 `lib.rs`). `wow-world-instances/src` 2,524 → 5,348 (+2,824: 2,813 moved,
  11 `lib.rs`).
- **`crates/wow-world`**: +35 −35 (paths and comments only; no line-count
  change). `net_move.py check`: shrink 0, growth charged 0 (not a move for the
  wow-world root; the moved files are accounted as moves between internal
  crates), 0 duplicate bodies.
- **Policy (`dependency-policy.json`).** `wow-world-social` +=
  `wow-world-entities`, `wow-world-inventory`, `wow-world-spell`.
  `wow-world-instances` += `wow-world-entities`, `wow-world-loot` and external
  `rand`. `wow-world-application` −= `wow-social` (only `group_handlers` used
  it). Social's `test-fixtures` feature (and its dev-dependencies) also
  enables the inventory and spell fixtures, which the moved guild-bank and
  trade `cfg(any(test, feature = "test-fixtures"))` blocks name; in
  application the same blocks were enabled through application's own feature
  list. `session-ownership-policy.json` is reprinted; the delta is only the
  `package`/`source` of 101 entries (96 `group_handlers`, 5
  `battleground_handlers`), application → instances; with those two keys
  masked the old and new policies are equal.
- **Cycle check.** Transitive closures over the head `Cargo.lock`
  (self dev-dependencies excluded): no `wow-world-*` package reaches itself,
  and no domain crate reaches `wow-world-application` or `wow-world`.
  `wow-world-social` → core, entities, inventory, spell;
  `wow-world-instances` → core, entities, inventory, lifecycle, loot, social,
  spell, visibility. Neither reaches the other's dependents.
- **Ceilings.** None raised and none to tighten: the Session logical owner
  (`session/mod.rs` 42,629 / 142,050 / 184,679) and every physical row are
  unchanged, because the wow-world edits are same-length path and comment
  changes.
- **Warnings.** `wow-world` `dead_code` is 0 in the production lib, the lib
  with `test-fixtures` and the lib test build; `unused_imports` stays 152 and
  161 in the first two, and no changed wow-world line warns. The warnings
  inside the moved files (`trade_handlers.rs` `spell_state` never read outside
  fixtures; `group_handlers.rs` two unused `vra` and one unused
  `send_result` macro) are on the same unchanged lines as in application; the
  modules were leaves there (no other application module named them), so no
  lint depends on the crate they live in.
- **Tests.** The test-name multiset is unchanged: none of the five modules
  contains a test, so application stays at 58 library tests and no `#[test]`
  or `#[ignore]` is added or removed. The checker suite passes (453).

### Validation

`VALIDATION_V2_CARGO_JOBS=1 ./tools/validation-v2 final --base origin/3.4.3
--architecture --timings` on the clean head `99610896d` (manifest
`target/validation-v2/manifests/20261012T205553.455633Z-67897-final.json`,
`dirty: false`): **passed, 1,015.9 s**. That is **over the 600 s budget**: the
workspace `cargo check --all-targets` took 495.0 s and the four-crate library
tests 237.4 s with one job, because the new social and instances edges
invalidate every dependent of those two crates (application, wow-world,
creature, world-server and their test targets). Tests: wow-world 3,686 passed
+ 1 ignored, application 58, instances 1, social 0, checker 453. Only this
section was added after the run (documentation-only delta).

### Next

A3 per the PR sequence, re-measured on the A2 merge.

## A3 result (travel, combat, corpse, gameobject → entities; dungeon finding → instances; client state, data service, player save → lifecycle; npc → interaction)

Branch `1363-sA3-application-split`, base `b02845b3f` (A2 merged). Move-only
(R2): every moved body is byte-identical; only `use` lines, self-crate
prefixes (`wow_world_entities::` / `wow_world_lifecycle::` → `crate::`, with
rustfmt grouping the `crate::` imports) and crate-root re-exports change.
Opcode registration and dispatch order and every processing flag are
unchanged (`handler_composition.rs` calls the same registrars in the same
order, through their new crate paths).

### Re-measure and pre-check

- **Base.** `wow-world-application/src` is 42,374 lines in 153 files on
  `b02845b3f`, as A2 reported. The nine A3 modules total 3,499 file lines
  (travel 391, combat 161, corpse 156, gameobject 261, dungeon finding 222,
  client state 1,375, data service 245, player save 481, npc 207), so A3 is
  one PR (below the ~6k split threshold).
- **Leaves.** None of the nine names another application module
  (`crate::` / `super::` scan finds nothing; nothing else in application
  names them; `trainer_purchase/buy_spell.rs` only declares a same-named host
  method). Their `wow_world_*` uses are: travel → core, lifecycle; combat →
  core; corpse → none; gameobject → core, entities, interaction; dungeon
  finding → core; client state → core; data service → core; player save →
  core, lifecycle; npc → core. So dungeon finding, client state, data service,
  player save and npc add **no** `wow-world-*` edge (confirming *Plan fixes*);
  only entities gains lifecycle (travel) and interaction (gameobject).
- **Closure check before moving.** A transitive-closure script over the
  `wow-world*` packages of `Cargo.lock` (and, separately, `cargo metadata`
  normal+build dependencies), run with the two planned edges simulated
  (`--add wow-world-entities->wow-world-lifecycle`,
  `--add wow-world-entities->wow-world-interaction`), found no package that
  reaches itself, no domain crate that reaches `wow-world-application` or
  `wow-world`, and no path from inventory, spell, entities or loot to social or
  instances. The same check on the head `Cargo.lock` and head metadata gives
  the same result. Head closures: entities → core, interaction, lifecycle,
  visibility; interaction → core; lifecycle → core, visibility; instances →
  core, entities, interaction, inventory, lifecycle, loot, social, spell,
  visibility; social → core, entities, interaction, inventory, lifecycle,
  spell, visibility. (The script is kept outside the repository, in the
  operator's work directory; see *Next*.)

### What moved

- **To `wow-world-entities`** (969 file lines): `travel_handlers.rs` (391),
  `combat_handlers.rs` (161), `gameobject_handlers.rs` (261) and the corpse
  registrar as **`corpse_registrations.rs`** (156). Entities already owns the
  four corpse bodies as `corpse_handlers` (slice 4b), so the registrar module
  takes a different name; the exact `ApplicationCorpse` root facade becomes
  `pub use corpse_registrations::{CorpseHandlerHostLikeCpp,
  register_corpse_handlers_like_cpp};`.
- **To `wow-world-instances`** (222): `dungeon_finding_handlers.rs`.
- **To `wow-world-lifecycle`** (2,101 + 1): `client_state.rs` (1,375),
  `data_service_handlers.rs` (245) and `player_save.rs` (481; its three
  `wow_world_lifecycle` imports become one `crate::` import in its own group,
  one blank line more).
- **To `wow-world-interaction`** (207): `npc_handlers.rs`.
- **Callers.** `wow-world` (`handler_composition.rs`, the travel, combat,
  gameobject, dungeon-finding, client-state and data-service context
  adapters, `npc_host.rs`, the corpse host, the save adapters in
  `session/lifecycle/{persistence,finalization}.rs`,
  `session/lifecycle/persistence/deferred.rs`, `session/persistence/save.rs`,
  `handlers/trainer/host.rs`, the character account/query data-service shims,
  five test shims, `unit_tests/handler_contract{,_deferred}_tests.rs` and
  `save_interleaving.rs`) name the new crate paths, and their module comments
  name the new owner crate. The entities `corpse_handlers.rs` and
  `player_attack.rs` comments name the registrar module instead of
  "application".
- **Checker.** The handler-contract checker's registrar packages and source
  mounts, and `handler-module-policy.json`, follow the nine moved registrars
  (`ApplicationTravel`, `ApplicationCombat`, `ApplicationGameObject`,
  `ApplicationCorpse` → entities, the corpse module/facade child renamed to
  `corpse_registrations`; `ApplicationDungeonFinding` → instances;
  `ApplicationClientState`, `ApplicationDataService` → lifecycle;
  `ApplicationNpc` → interaction). The composer test mounts the interaction
  crate root (for the NPC facade) and the corpse facade mutation case now
  targets the entities root. Owner labels are unchanged (rename is the A2
  follow-up).

### Accounting

- **`wow-world-application/src`: 42,374 → 38,833 lines (−3,541), 153 → 144
  files.** 3,499 lines leave as files; `lib.rs` loses 42 net (−44 +2: the
  corpse-facade comment now names only the talent facade).
- **Destinations.** `wow-world-entities/src` 12,808 → 13,793 (+985: 969 moved,
  16 `lib.rs`). `wow-world-instances/src` 5,348 → 5,575 (+227: 222, 5).
  `wow-world-lifecycle/src` 9,286 → 11,407 (+2,121: 2,101 moved, +1 import
  grouping, 19 `lib.rs`). `wow-world-interaction/src` 571 → 782 (+211: 207,
  4).
- **`crates/wow-world`**: +73 −74 (paths and comments; rustfmt folds one
  `let … else` head in `save_interleaving.rs`). `net_move.py check`: shrink 0,
  growth charged 3 (moved files are accounted as moves between internal
  crates), 0 duplicate bodies; exit 0.
- **Policy (`dependency-policy.json`).** `wow-world-entities` +=
  `wow-world-interaction`, `wow-world-lifecycle`. `wow-world-interaction` +=
  `wow-constants`, `wow-handler` and external `tracing` (exactly what the
  moved NPC body names). Instances and lifecycle need nothing new.
  `wow-world-application` keeps every dependency (each is still named by a
  remaining module). No destination's `test-fixtures` feature changes: the
  only moved `cfg(any(test, feature = "test-fixtures"))` blocks
  (`gameobject_handlers`, `player_save`) name items of their own new crate or
  of `wow-persistence`, and `cargo tree -p world-server -e normal,build`
  enables no `test-fixtures` feature.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`; the
  delta is the `PlayerSaveHostLikeCpp` trait path and the player-save types in
  its method signatures (14 occurrences: `wow_world_application` →
  `wow_world_lifecycle`) and the resulting sort order. With those paths
  masked the old and new policies are equal as multisets (4,525 entries).
- **Ceilings.** None raised. The Session logical owner is tightened to the
  live values: production 42,629 unchanged, test 142,050 → 142,049, total
  184,679 → 184,678. Every physical row is unchanged.
- **Warnings.** `wow-world` `dead_code` is 0 in the production lib, the lib
  with `test-fixtures` and the lib test build; `unused_imports` stays 152 and
  161 in the first two (255 in the test build), and no changed wow-world line
  warns. The only warning in a moved file is the unused `ServerPacket` import
  of `dungeon_finding_handlers.rs`, the same unchanged line that warned in
  application (an unused-import lint does not depend on the crate).
- **Tests.** The test-name multiset is unchanged: none of the nine modules
  contains a test, no `#[test]` or `#[ignore]` line is added or removed, and
  application stays at 58 library tests.

### Validation

`final --architecture --timings` runs on the committed head that includes
this section; its manifest path, dirty flag and duration are in the PR body
(this section is not edited after that run, so the PR head is the validated
SHA).

### Next

- A4 (support layer → inventory) per the PR sequence; re-measure application
  on the A3 merge (38,833 expected).
- The closure script used here (Cargo.lock and `cargo metadata` modes, planned
  edges simulated with `--add`, the three rules above) should become a
  tracked `tools/architecture/` check with a self-test, so each later slice
  runs the same command.

## A4 result (support layer → inventory: stats, bank, vendor, collections, battle pets)

Branch `1363-sA4-support-inventory`, base `453affb6a` (A3 merged). Move-only
(R2): every moved body is byte-identical; only `use` lines, self-crate
imports (`wow_world_inventory::InventoryState` → `crate::InventoryState`),
caller crate prefixes and crate-root re-exports change. Opcode registration
and dispatch order and every processing flag are unchanged
(`handler_composition.rs` changes only the three registrar paths).

### Re-measure and pre-check

- **Base.** `wow-world-application/src` is 38,833 lines in 144 files on
  `453affb6a`, as A3 reported. The eight planned units total 3,587 file
  lines (stats 677, registry sync 211, inventory scaling 220, equipment-set
  use 503 with its registration test, bank 173, vendor 541, collections 394,
  battle pets 868), matching the plan.
- **Three units stay in application.** `registry_sync` names
  `super::SessionQuestState` (the quest cluster, still in application) in
  its fixture-gated `PlayerRegistryHydrationContext` and in the
  `cfg(any(test, feature = "test-fixtures"))` argument of
  `sync_player_registry_state_like_cpp`. Moving it would need inventory →
  application (forbidden, and a cycle even as an optional dependency), or a
  generic hydration context (not move-only). `inventory_scaling` and
  `equipment_set_use` both construct `PlayerRegistrySyncContext` and take a
  `PlayerRegistryHydrationContext`, so they stay with it. All three are
  used by the quest cluster, loot release, trainer runtime install, aura
  removal and the inventory swap; the natural home of `registry_sync` is
  the A6 `wow-world-quest` crate (beside `SessionQuestState`). The other
  two stay until that boundary is re-planned (see *Next*). 934 lines.
- **The five that move** name no other application module (`crate::` /
  `super::` scan finds nothing). Their `wow_world_*` uses: stats → core,
  inventory; bank → core, interaction, inventory; vendor → core;
  collections → core, inventory; battle pets → core, lifecycle. So the new
  inventory edges are only `wow-world-interaction` (bank) and
  `wow-world-lifecycle` (battle pets); the planned loot and spell edges came
  from the three units that stay.
- **Closure check before moving**
  (`/home/ubuntu/rustycore-wip/wow_world_closure.py`, Cargo.lock and
  `cargo metadata` modes) with the whole planned set simulated
  (`--add wow-world-inventory->wow-world-{interaction,lifecycle,loot,spell}`):
  no package reaches itself, no domain crate reaches `wow-world-application`
  or `wow-world`, and inventory, spell, entities and loot never reach social
  or instances. The same tracked check (below) on the head lock and head
  metadata passes with the two real edges: inventory → core, interaction,
  lifecycle (closure + visibility); social → core, entities, interaction,
  inventory, lifecycle, spell, visibility; instances → core, entities,
  interaction, inventory, lifecycle, loot, social, spell, visibility.

### What moved

- **To `wow-world-inventory`** (2,653 `src` lines + 651 test lines):
  `stats.rs` (677) as **`stats_application.rs`** and `bank.rs` (173) as
  **`bank_handlers.rs`** (inventory already owns `stats` and `bank`
  modules); `vendor.rs` (541, still `pub mod vendor`) with its
  `unit_tests/vendor_rules.rs` (651, 23 tests; the `#[path]` mount is
  unchanged); `collections_handlers.rs` (394); `battle_pet_handlers.rs`
  (868). `vendor.rs`, `vendor_rules.rs` and `battle_pet_handlers.rs` are
  100% renames; the other three change one import line each.
- **Root exports.** Inventory re-exports exactly the names application
  exported (the bank names as one exact facade item; the battle-pet and
  collections facades keep their shape).
- **Callers.** Application's remaining users of the stat application
  (`aura_removal`, `quest/reward`, `loot_release`, `inventory_swap`,
  `inventory_scaling`, `equipment_set_use`, `character_creation`, the
  character creation family) name `wow_world_inventory::…`; rustfmt
  rewraps two longer call heads in `quest/reward/{removals,xp_grants}.rs`.
  `wow-world` (`handler_composition.rs`, the bank, collections and
  battle-pet host adapters, `session/pets/battle_pet{,_journal}.rs`,
  `session/player_items/bank.rs`, `handlers/character/{mod,stats}.rs`, the
  vendor rules re-export, `void_storage/transfer.rs`, four test shims,
  `unit_tests/handler_contract_tests.rs`, `character_tests/item_4.rs` and
  the source scan in `character_tests/loot.rs`) names the new paths, and
  the adapter comments name the new owner crate. The application
  account-handler module comment names `wow_world_inventory` for vendor
  and bank.
- **Checker.** The handler-contract checker's `BANK_REGISTRAR`,
  `COLLECTIONS_REGISTRAR` and `BATTLE_PET_REGISTRAR` packages and source
  mounts, and `handler-module-policy.json`, follow the three registrars;
  the bank module/facade child is `bank_handlers`, and the composer test's
  bank facade case targets the inventory crate root (index 2). Owner labels
  are unchanged (rename is the A2 follow-up).

### Tracked closure check

The A3 operator script is now `tools/architecture/wow_world_closure.py`
with the self-test `tools/architecture/test_wow_world_closure.py` (seven
fixture-graph cases: lock parsing and self-dependency drop, metadata
dev-edge exclusion, a simulated cycle, a domain crate reaching the top, a
transitive low → instances path, a malformed `--add`, and the CLI exit
status). Run it before each slice with the planned edges:

```
python3 tools/architecture/wow_world_closure.py check \
  --add 'wow-world-inventory->wow-world-lifecycle'   # repeatable
python3 tools/architecture/wow_world_closure.py check --source metadata
python3 -m unittest discover -s tools/architecture -p test_wow_world_closure.py
```

`validation-v2 final` plans the lock-only check (no Cargo call) for a
`Cargo.lock` or `crates/wow-world*/Cargo.toml` change and the self-test when
the check changes; `tools/test_validation_v2.py` pins both, and
`docs/operations/validation-v2.md` documents it.

### Accounting

- **`wow-world-application/src`: 38,833 → 36,164 lines (−2,669), 144 → 139
  files.** 2,653 lines leave as files; `lib.rs` loses 24; the rewraps and
  the account-handler comment add 8. `unit_tests/`: 1,359 → 708.
- **Destination.** `wow-world-inventory/src` 10,990 → 13,668 (+2,678: 2,653
  moved, 25 `lib.rs`); `unit_tests/` 0 → 651.
- **`crates/wow-world`**: +34 −34 (paths and comments). `net_move.py check`:
  shrink 0, growth charged 9 (the `lib.rs` export delta and the two
  rewraps; the moved files are accounted as moves between internal crates),
  0 duplicate bodies; exit 0.
- **Policy (`dependency-policy.json`).** `wow-world-inventory` +=
  `wow-world-interaction`, `wow-world-lifecycle`. No edge removed:
  application still names every dependency it declares. Inventory's
  `test-fixtures` feature is unchanged: the moved fixture blocks
  (`stats_application`, `battle_pet_handlers`) name only `wow-world-core`
  fixtures, which it already forwards; `cargo check -p wow-world-inventory
  --all-targets` and its tests build alone. `cargo tree -p world-server -e
  normal,build -e features` shows no `test-fixtures` and no
  `wow-world-test-support`.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`;
  the delta is the `BankHandlerHostLikeCpp` trait path, the bank and stat
  application types in three method signatures and the collections direct
  registry rows (`wow_world_application` → `wow_world_inventory`), with the
  resulting sort order. With those two crate names masked the old and new
  policies are equal (10,581 entries).
- **Ceilings.** None raised, none to tighten: every hotspot owner (Session
  42,629 / 142,049 / 184,678) and physical row is at its live value, because
  the wow-world edits are same-length path and comment changes.
- **Warnings.** `wow-world` `dead_code` is 0 in the production lib, the lib
  with `test-fixtures` and the lib test build; `unused_imports` is 152 and
  161 in the first two (as in A3), and no changed wow-world line warns. No
  warning falls in a moved file in its new crate.
- **Tests.** The Rust test-name multiset is unchanged (10,214 `#[test]`
  functions, 5 ignored, same names on base and head). The 23 vendor tests
  run in inventory (application 58 → 35, inventory 0 → 23). The new
  closure self-test adds 7 Python cases.

### Validation

`final --architecture --timings` runs on the committed head that includes
this section; its manifest path, dirty flag and duration are in the PR body
(this section is not edited after that run, so the PR head is the validated
SHA).

### Next

- A5 (profession + spell acquisition → spell; character family → new
  `wow-world-character`): the character family's `stats` dependency is now
  in inventory, so A5 is unblocked. Run the tracked closure check with the
  planned edges first.
- A6 must take `registry_sync` with the quest cluster (its fixture
  hydration names `SessionQuestState`). `inventory_scaling` and
  `equipment_set_use` then depend on a crate above inventory; re-plan their
  destination at A6 (they can follow `registry_sync`, or stay in
  application), or make the fixture hydration quest-agnostic in a separate
  behaviour-neutral change before moving them to inventory.

## A5 result (profession, spell acquisition → spell; character family → new `wow-world-character`)

Branch `1363-sA5-spell-character`, base `989b99b6c` (A4 merged). Move-only
(R2): all 29 moved files (12 spell, 17 character) are 100% renames; only
`use` lines, caller crate prefixes, crate-root re-exports and adapter
comments change. Opcode registration and dispatch order and every
processing flag are unchanged (`handler_composition.rs` changes only the
two character registrar paths).

### Re-measure and pre-check

- **Base.** `wow-world-application/src` is 36,164 lines in 139 files on
  `989b99b6c`, as A4 reported. The two units total 5,782 file lines
  (profession 340, spell acquisition 2,388; character family 3,054:
  handlers 700, handler families 1,127, query handlers 754, login support
  286, enumeration 105, creation 82), below the ~6k split threshold, so A5
  is one PR.
- **Nothing stays.** No unit names `registry_sync`, `inventory_scaling`,
  `equipment_set_use` or `SessionQuestState` (the A4 holdbacks). The only
  in-crate edges are inside the units (`spell_acquisition` →
  `profession`; the families → `character_handlers`,
  `character_creation`, `character_enumeration`); the character family's
  stat dependency is `wow_world_inventory` since A4. Their `wow_world_*`
  uses: profession/spell acquisition → core, lifecycle; character →
  core, entities, inventory, lifecycle.
- **Who calls them.** In application only `trainer_purchase` (spell
  acquisition and profession); nothing in application names the
  character family, so **application does not depend on the new crate**
  (the planned application → character edge is not needed). In
  `wow-world`: `handler_composition.rs`, the character host/query
  adapters and shims, `publication/operations.rs`, `profession.rs` and
  `spell_acquisition/{application,effect_learning}.rs`.
- **Closure check before moving**, with every planned edge simulated
  (`--add` spell → lifecycle; character → core, entities, inventory,
  lifecycle; application → character; wow-world → character), lock and
  metadata modes: PASS. The check now also enforces the plan's
  *`wow-world-character` sits high* rule: only `wow-world-application`
  and `wow-world` may reach it (new self-test case: a direct and a
  transitive violation). Head closures: spell → core, lifecycle,
  visibility; character → core, entities, interaction, inventory,
  lifecycle, visibility; nothing but `wow-world` reaches character.

### What moved

- **To `wow-world-spell`** (2,728 `src` lines + 469 test lines):
  `profession.rs` (340) and `spell_acquisition.rs` + `spell_acquisition/`
  (2,388) as **`spell_acquisition_application`** (spell already owns a
  `spell_acquisition` module, the `SessionSpellState` adapter); the
  `#[path]` test mounts keep their relative paths, so
  `unit_tests/profession.rs` (11 tests) and
  `unit_tests/spell_acquisition/persistence.rs` (2 tests) move beside
  them unchanged. Spell re-exports exactly the names application
  exported.
- **To the new `wow-world-character`** (3,054): `character_handlers.rs`,
  `character_handler_families/` (12 files), `character_creation.rs`,
  `character_enumeration.rs`, `character_login_support.rs`,
  `character_query_handlers.rs`, module names unchanged. Its `lib.rs`
  holds the same module declarations and the same re-export items
  (including the exact `ApplicationCharacter` facade item and its
  comment) that application had. The family has no tests.
- **Callers.** Application's trainer purchase names `wow_world_spell::…`
  (the five `crate::…` profession calls in `trainer_purchase/projection.rs`
  become `wow_world_spell::…` and rustfmt rewraps them, +3 lines).
  `wow-world` names `wow_world_spell` / `wow_world_character`; the
  character adapter and shim comments name the new owner crate.
- **Checker.** `CHARACTER_REGISTRAR` and `CHARACTER_QUERY_REGISTRAR` move
  to package `wow-world-character`; the composer test mounts both files
  at their new paths plus the character crate root; the handler
  contract's reviewed production-package set adds `wow-world-character`;
  `handler-module-policy.json` follows both registrars. The
  session-ownership scan gains a `WorldCharacter` package role (scanned
  like the other extracted packages, never a Session contract owner, and
  included in the three role-list tests and the real-root load test), so
  the two `character_query_handlers` registry rows stay inventoried.
  Owner labels are unchanged (rename is the A2 follow-up).

### Accounting

- **`wow-world-application/src`: 36,164 → 30,330 lines (−5,834), 139 →
  112 files.** 5,782 lines leave as files; `lib.rs` loses 55 (234 → 179);
  the projection rewrap adds 3. `unit_tests/`: 708 → 241 (the two moved
  test files; the trainer-purchase test import is rewrapped, +2).
- **Destinations.** `wow-world-spell/src` 6,204 → 8,964 (+2,760: 2,728
  moved, 32 `lib.rs`); `unit_tests/` 218 → 687 (+469).
  `wow-world-character/src` 0 → 3,088 (3,054 moved, 34 `lib.rs`).
- **`crates/wow-world`**: +38 −40 (paths and comments; rustfmt folds one
  shorter `wow_world_spell` import). `net_move.py check`: S 2, G 14, 0
  duplicate bodies, PASS.
- **Edges.** Added: spell → lifecycle (+ `wow-persistence`, `wow-config`,
  external `rand`, dev `tokio`); character → core, entities, inventory,
  lifecycle (+ `wow-constants`, `wow-core`, `wow-data`, `wow-entities`,
  `wow-handler`, `wow-packet`, `wow-persistence`, external `tracing`);
  `wow-world` → character. Removed: application → `wow-config` and
  external `rand` (no remaining user).
- **Policy (`dependency-policy.json`).** `wow-world-character` joins the
  `application` category with the other `wow-world-*` crates, restricted
  to its 11 actual dependencies, external `tracing` only. Spell +=
  `wow-config`, `wow-persistence`, `wow-world-lifecycle`, external
  `rand`; application −= `wow-config`, external `rand`. New fixtures
  `forbidden-character-to-wow-world.json` and
  `forbidden-character-to-world-application.json` (self-test: 24
  fixtures). `net-move-policy.json` adds the character `src/` destination
  root (14; `test_net_move.py` pins it). No `test-fixtures` feature is
  added: the character family has no fixture-gated code, and spell's
  existing feature already covers the moved `commit.rs` fixture argument
  (application's feature forwards it). `cargo tree -p world-server -e
  normal,build -e features` shows no `test-fixtures` and no
  `wow-world-test-support`.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`;
  the delta is the two `character_query_handlers` registry rows
  (package and source path) and the resulting sort order. With
  `wow-world-application` / `wow-world-character` masked the old and new
  policies are equal (10,581 entries).
- **Ceilings.** None raised, none to tighten: every hotspot owner
  (Session 42,629 / 142,049 / 184,678; character 15,088 / 15,708 /
  30,796) is at its live value, and every physical row is unchanged. No
  moved file is above 1,000 lines (largest `character_query_handlers.rs`,
  754).
- **Warnings.** `wow-world` `dead_code` is 0 in the production lib, the
  lib with `test-fixtures` and the lib test build; `unused_imports` is
  152 and 161 in the first two (as in A4), and no changed wow-world line
  warns. No warning falls in a moved file in its new crate
  (`wow-world-character` has none; spell's five are pre-existing, in
  unmoved files).
- **Tests.** The Rust test-name multiset is unchanged (10,214 `#[test]`
  functions, 5 ignored, same names on base and head). The 13 profession
  and persistence tests run in spell (application 35 → 22, spell 12 →
  25). The closure self-test gains one Python case (8).

### Validation

`final --architecture --timings` runs on the committed head that includes
this section; its manifest path, dirty flag and duration are in the PR body
(this section is not edited after that run, so the PR head is the validated
SHA).

Recorded after the merge (#1377 review follow-up): the green `final` on the
merged head `81a8fa27a` took **639.8 s** (warm) with
`VALIDATION_V2_TIMEOUT_SECONDS=1800`. The first, cold run hit the default
900 s per-step timeout on `cargo check --workspace --all-targets --jobs 1`
(no compile error; an edge change forces the workspace re-plan). The 1800 s
override is the documented per-command setting (CI uses 3600) and skips no
check; slices that change crate edges use it for their local `final`.

### Next

- A6 (`wow-world-quest`): re-run the quest-cluster scan on the A5 merge
  (expected application ~30.3k). `registry_sync` goes with the cluster;
  re-plan `inventory_scaling` and `equipment_set_use` there (see A4
  *Next*). Trainer purchase now reaches spell acquisition and profession
  in `wow-world-spell`; its remaining application imports are
  `SessionQuestState`, `PlayerConditionProjectionCxLikeCpp` (quest
  cluster) and `PlayerAuraApplicationCxLikeCpp` (`aura_removal`), which
  is what a later trainer split has to re-measure after A6.
- Slice 8 (character login load and handlers → `wow-world-character`)
  needs its own closure check; the tracked check now rejects any edge
  from a domain crate or `wow-world-core` back to character.

## A6 result (quest cluster → new `wow-world-quest`)

Branch `1363-sA6-quest-crate`, base `b21a2db1a` (A5 merged).

### Re-measure and pre-check

- **Base.** `wow-world-application/src` is 30,330 lines in 112 files on
  `b21a2db1a`, as A5 reported. The cluster (`quest/` 43 files,
  `quest_handlers`, `quest_query_handlers{,/complete_quest}`,
  `player_conditions{,/inputs}`, `inventory_valuation{,/direct_storage}`)
  plus `registry_sync` is 14,269 file lines in 51 files; the trainer
  admission proof (`TrainerAdmissionProofLikeCpp` and
  `trainer_condition_admission_proof_like_cpp`, `trainer_purchase/offer.rs`)
  is 22 more.
- **Scan** (lexical, as in the plan: `crate::…` paths, brace groups, names
  re-exported from `lib.rs`, and `super::…` in direct children of the crate
  root). Out of the cluster + `registry_sync`, the only edge is
  `player_conditions` → `trainer_purchase` for the admission proof (the enum
  and the function, nothing else). The cluster names nothing in
  `aura_removal`, the rest of `trainer_purchase`, `inventory_swap`,
  `inventory_move_planning`, the talent/spell/player handlers,
  `character_account_handlers`, `loot_*`, `inventory_scaling` or
  `equipment_set_use`. Inside: `quest` → `player_conditions`,
  `registry_sync`; `player_conditions` → `inventory_valuation`, `quest`;
  `inventory_valuation` → `player_conditions`; `registry_sync` → `quest`
  (`SessionQuestState`); `quest_handlers` → `quest`,
  `quest_query_handlers`; `quest_query_handlers` → `quest`. Every other
  `super::super::…` path stays inside `quest/`.
- **The admission proof moves down** with the cluster (it has no
  dependency: a `Copy` enum and a three-branch function), so the cluster has
  no edge back into application.
- **Who names the cluster in application** (all application → quest, which
  is allowed): `aura_removal` (conditions, quest state, hydration),
  `trainer_purchase` (conditions, quest state, objective event, sync,
  admission proof), `loot_release` (conditions, quest eligibility, sync,
  gameobject flags), `inventory_swap` and `inventory_move_planning`
  (conditions, valuation, sync), `spell_handlers` and `player_handlers`
  (quest state), `inventory_scaling` and `equipment_set_use` (sync,
  hydration).
- **`inventory_scaling` and `equipment_set_use` stay in application.**
  `registry_sync` goes with the cluster (its fixture hydration names
  `SessionQuestState`). The two consumers are inventory operations (item
  scaling, equipment-set use), not quest logic, so following `registry_sync`
  into the quest crate would put them in the wrong domain, and
  `equipment_set_use` is a handler registrar (more checker churn for no
  ownership gain). They name `wow_world_quest::…` for the sync context; the
  path to `wow-world-inventory` stays the one in A4 *Next*: make the fixture
  hydration quest-agnostic (behaviour-neutral, separate change), then move
  `registry_sync` and both consumers to inventory.
- **Closure check before moving**, every planned edge simulated (`--add`
  quest → core, entities, instances, inventory, lifecycle, loot, social,
  spell; application → quest; `wow-world` → quest), lock and metadata
  modes: PASS. The check now also enforces the plan's *nothing below
  depends on the quest crate* rule: only `wow-world-application`,
  `wow-world` and the planned `wow-world-loot-ops` may reach
  `wow-world-quest` (new self-test case: a direct and a transitive
  violation; 9 cases). A negative simulation (`--add
  wow-world-spell->wow-world-quest`) reports the cycle and the spell →
  social/instances violation.

### What moved

One PR (no A6a/A6b split): 48 of the 51 moved files are 100% renames, so the
~14.3k-line move reviews as a rename set like 3a.

- **To the new `wow-world-quest`** (14,269 file lines, 51 files, module
  names and layout unchanged; the cluster has no `#[path]` mounts): `quest/` (43 files),
  `quest_handlers.rs`, `quest_query_handlers.rs` +
  `quest_query_handlers/complete_quest.rs`, `player_conditions.rs` +
  `player_conditions/inputs.rs`, `inventory_valuation.rs` +
  `inventory_valuation/direct_storage.rs`, `registry_sync.rs`. Its
  `lib.rs` holds the same module declarations and the same re-export items
  application had for them (including the exact `ApplicationQuest` facade
  item and both comments), plus the admission proof.
- **The trainer admission proof** (`TrainerAdmissionProofLikeCpp`,
  `trainer_condition_admission_proof_like_cpp`, 22 lines verbatim) leaves
  `trainer_purchase/offer.rs` for `wow-world-quest/src/trainer_admission.rs`
  and is re-exported from the quest root, so `player_conditions` still names
  it as `crate::…`. Trainer purchase imports it from `wow_world_quest` (in
  `offer.rs` and `controller.rs`); application no longer re-exports it.
- **Visibility.** Three moved files differ only in visibility (R098/R099):
  the `PlayerConditionProjectionCxLikeCpp` fields `inventory`,
  `valuation_catalogs`, `in_combat`, `consumer_test` and its methods
  `player_access_like_cpp`, `complete_player_skill_records_like_cpp`,
  `trainer_spell_condition_proof_like_cpp`;
  `PlayerConditionProjectionInputsLikeCpp::reborrow_like_cpp`; and
  `PlayerRegistrySyncContext::sync_with_fixture_hydration_like_cpp` go from
  `pub(crate)` to `pub`. Each is named by code that stays in application
  (trainer purchase, inventory move planning, aura removal, inventory swap);
  nothing else is widened.
- **Inherent impls** on `PlayerConditionProjectionCxLikeCpp`
  (`quest/visibility.rs`, `inventory_valuation{,/direct_storage}.rs`) stay
  in its defining crate.
- **Callers.** Application's remaining modules (aura removal, trainer
  purchase, loot release, inventory swap and move planning, inventory
  scaling, equipment-set use, spell and player handlers) name
  `wow_world_quest::…` (`crate::…`, `crate::registry_sync::…` and
  `crate::quest::…` paths). `wow-world` (47 files: `handler_composition.rs`
  for the two quest registrars, the quest/visibility/condition/registry
  adapters, test shims, `handler_contract_tests.rs`, the
  `QUEST_HANDLER_REGISTRATIONS` source scan) names the new paths; adapter
  comments name the new owner crate. `use` groups that mixed quest and
  application names are split; rustfmt rewraps where a path length
  changes (three folds in `wow-world`, a few rewraps in application).
- **Registration and dispatch** order and every processing flag are
  unchanged: `handler_composition.rs` changes only the two quest registrar
  paths.
- **Checker.** `QUEST_REGISTRAR` and `QUEST_QUERY_REGISTRAR` move to package
  `wow-world-quest`; the composer test mounts both files at their new paths
  plus the quest crate root, the guard's registrar list names
  `wow_world_quest::register_quest_handlers_like_cpp`, and the inexact quest
  facade case targets the quest crate root by lookup (it used the
  application root index); the reviewed production-package set adds
  `wow-world-quest`; `handler-module-policy.json` follows both registrars.
  The session-ownership scan gains a `WorldQuest` package role mirroring
  `WorldCharacter` at every site (never a Session contract owner; in the
  three role-list tests and the real-root load test), so the four
  `registry_sync` direct-registry rows stay inventoried. Owner labels are
  unchanged (rename is the A2 follow-up).

### What stays in application, and why

- `inventory_scaling` and `equipment_set_use` (inventory operations; see
  the pre-check), now importing the sync context from the quest crate.
- `aura_removal`, `trainer_purchase` (minus the proof), `trainer_handlers`,
  `talent_handlers`, `spell_handlers`, `player_handlers`, `inventory_swap`,
  `inventory_move_planning`, `character_account_handlers`: cross-domain
  orchestration, as planned.
- `loot_handlers`, `loot_release`: A7 (`wow-world-loot-ops`, above quest).

### Accounting

- **`wow-world-application/src`: 30,330 → 15,961 lines (−14,369), 112 → 61
  files.** 14,269 lines leave as files; `lib.rs` 179 → 98 (−81: the moved
  module and export items, the two proof names, rustfmt reflow); `offer.rs`
  −22 (the proof) and the caller path edits +3. `unit_tests/` unchanged
  (241).
- **Destination.** `wow-world-quest/src` 0 → 14,395 (14,269 moved, 96
  `lib.rs`, 30 `trainer_admission.rs`), 53 files.
- **`crates/wow-world`**: +118 −123 in 47 files (paths, comments, the two
  `include_str!` paths, three rustfmt folds, the manifest).
  `net_move.py check`: S 8, G 26, 0 duplicate bodies, PASS.
- **Edges.** Added: quest → core, entities, instances, inventory,
  lifecycle, loot, social, spell (+ `wow-conditions`, `wow-constants`,
  `wow-core`, `wow-data`, `wow-entities`, `wow-handler`, `wow-loot`,
  `wow-packet`, `wow-persistence`, `wow-progression`, `wow-script`,
  external `tracing`); application → quest; `wow-world` → quest. Removed:
  application → `wow-conditions`, `wow-progression`, `wow-script` (no
  remaining user). Head closures: quest → core, entities, instances,
  interaction, inventory, lifecycle, loot, social, spell, visibility;
  application adds quest; only application and `wow-world` reach quest
  (lock and metadata modes PASS).
- **Policy (`dependency-policy.json`).** `wow-world-quest` joins the
  `application` category, restricted to its 19 actual dependencies,
  external `tracing` only; application −= `wow-conditions`,
  `wow-progression`, `wow-script`, += `wow-world-quest`. New fixtures
  `forbidden-quest-to-wow-world.json` and
  `forbidden-quest-to-world-application.json` (self-test: 26 fixtures).
  `net-move-policy.json` adds the quest `src/` destination root (15;
  `test_net_move.py` pins it). The quest crate gets a `test-fixtures`
  feature forwarding to its eight `wow-world-*` dependencies and a
  self dev-dependency (as application has), because the moved code has
  fixture-gated fields and arguments; application's and `wow-world`'s
  `test-fixtures` forward to it. `cargo tree -p world-server -e
  normal,build -e features` shows no `test-fixtures` and no
  `wow-world-test-support`.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`;
  the delta is four WorldSession impl signatures and the four
  `registry_sync` direct-registry rows (`wow_world_application` /
  `wow-world-application` → `wow_world_quest` / `wow-world-quest`) and the
  resulting sort order. With those crate names masked the old and new
  policies hold the same 4,525 list entries.
- **Ceilings.** None raised. Two hotspot owners tightened to their live
  values in `runtime-ownership-ledger.json`: Session test 142,049 →
  142,047 and total 184,678 → 184,676 (the `cfg(test)` refresh import
  folds onto one line); quest handler production 5,506 → 5,501 and total
  16,759 → 16,754 (`find_quest_slot_like_cpp` folds onto one line). Every
  physical row is unchanged; the largest moved file is still
  `quest/objectives.rs` (1,060, as in application).
- **Warnings.** `wow-world` `dead_code` is 0 in the production lib, the
  lib with `test-fixtures` and the lib test build; `unused_imports` is 152
  and 161 in the first two (as in A5). Two of the rewritten `wow-world`
  import lines carry their existing `unused_imports` warning
  (`handlers/quest/mod.rs` `quest_completion_npc_response`,
  `session/mod.rs` dynamic-flags update): same import, crate name only, the
  totals do not change. The warning set of application + quest at head
  equals application's at base in all three builds (lib, `--features
  test-fixtures`, `--profile test`; compared by file and message), so the
  moved files keep their pre-existing warnings and nothing new appears.
- **Tests.** The Rust test-name multiset is unchanged (10,214 `#[test]`
  functions, 5 ignored, same names on base and head). The two
  `quest/dialog_status.rs` tests run in quest (application 19 → 17 test
  functions in `src`, quest 0 → 2). The closure self-test gains one Python
  case (9).

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section; its manifest path, dirty flag and duration are in the
PR body (this section is not edited after that run, so the PR head is the
validated SHA).

### Next

- A7 (`wow-world-loot-ops`): `loot_handlers` and `loot_release` →
  new crate above quest (loot release names the quest cluster, conditions,
  sync and gameobject flags, all now in `wow-world-quest`). The closure
  check already admits `wow-world-loot-ops` → quest.
- `registry_sync`, `inventory_scaling` and `equipment_set_use` → inventory
  after a behaviour-neutral change makes the fixture hydration
  quest-agnostic (A4 *Next*).
- Re-measure a `wow-world-trainer` split: trainer purchase still names
  `SessionQuestState`, `PlayerConditionProjectionCxLikeCpp` (now quest
  crate, an allowed edge) and `PlayerAuraApplicationCxLikeCpp`
  (`aura_removal`, application).

## A7 result (loot operations → new `wow-world-loot-ops`)

Branch `1363-sA7-loot-ops`, base `fff4249ee` (A6 merged). Move-only (R2):
all 17 moved files are 100% renames; only the new crate root, caller crate
prefixes, adapter comments and manifests change. Opcode registration and
dispatch order and every processing flag are unchanged
(`handler_composition.rs` changes only the loot registrar path).

### Re-measure and pre-check

- **Base.** `wow-world-application/src` is 15,961 lines in 61 files on
  `fff4249ee`, as A6 reported. The two units are 4,427 file lines in 17
  files: `loot_handlers.rs` 671 + `loot_handlers/` 6 files (1,286);
  `loot_release/` 10 files (2,470, including its 105-line `tests.rs`).
- **Scan** (lexical, as in A6: `crate::…`, `super::…`, names re-exported from
  `lib.rs`). The only application edge out of the units is
  `loot_handlers` → `crate::LootReleaseCxLikeCpp` (`loot_release`, which moves
  with it). Every other `super::…` stays inside its own module. The units name
  nothing that stays in application. No item had to move down and the
  boundary did not change.
- **Who names the units.** Nothing in application except `lib.rs` (the
  module declarations and the eight re-exported names). So application
  does **not** depend on the new crate, and the planned application →
  loot-ops edge is not needed. In `wow-world`: `handler_composition.rs`
  (registrar), `session_commands.rs`, the loot adapters
  (`handlers/loot/{loot_host,persistence,claims/release,handlers/money,
  handlers/test_shims,sources/gameobject,sources/gameobject_authority}.rs`)
  and five test files.
- **Closure check before moving.** Every planned edge is simulated
  (`--add` loot-ops → core, entities, instances, inventory, lifecycle, loot,
  quest, social, spell; `wow-world` → loot-ops). Lock and metadata modes:
  PASS. The check now also enforces a new rule: only `wow-world-application`
  and `wow-world` may reach `wow-world-loot-ops`. There is a new self-test
  case with a direct violation (quest → loot-ops, which also reports the
  cycle) and a transitive one (loot → loot-ops reached through instances).
  That makes 10 cases, and the case fails when the rule is disabled.

### What moved

- **To the new `wow-world-loot-ops`** (4,427 file lines, 17 files; module
  names and layout unchanged): `loot_handlers.rs` + `loot_handlers/`
  (`item`, `master_loot`, `money`, `roll`, `specialization`, `unit`) and
  `loot_release/` (`mod`, `authority`, `creature`, `detached`, `fanout`,
  `gameobject`, `item`, `publication`, `registry`, `tests`).
  - Its `lib.rs` (22 lines) holds the same two module declarations and the
    same two re-export items application had (`LootReleaseCxLikeCpp` and
    the four loot-release helpers; `LootHandlerCxLikeCpp`,
    `LootHandlerHostLikeCpp`, `register_loot_handlers_like_cpp`).
  - `loot_handlers` keeps naming `crate::LootReleaseCxLikeCpp`.
  - No visibility changes. The units' 19 `pub(crate)`/`pub(super)` items
    have no caller outside the units.
- **Callers.** `wow-world` (16 files) names `wow_world_loot_ops::…`. Four
  adapter comments name the new owner crate. rustfmt folds one call in
  `loot_tests/item_1.rs`.
- **Checker.**
  - `LOOT_REGISTRAR` moves to package `wow-world-loot-ops`.
  - The composer test mounts `loot_handlers.rs` at its new path plus the
    loot-ops crate root. The `crate` facade contract follows the registrar.
  - The reviewed production-package set adds `wow-world-loot-ops`.
  - `handler-module-policy.json` follows the registrar.
  - The session-ownership scan gains a `WorldLootOps` package role,
    mirroring `WorldQuest` at every site: the root constants, the enum, the
    package name, the two non-owner matches in `state_2.rs`, the scan match
    and the repository units in `state_3.rs`. The role never owns a Session
    contract. It is included in the three role-list tests and the real-root
    load test, which asserts that `crate::loot_handlers::master_loot` loads.
    This keeps the 19 `loot_handlers::{master_loot,money}` registry rows
    inventoried.
  - **#1378 follow-up 1:** the duplicate `| PackageRole::WorldQuest` in
    `state_2.rs` is gone; the new role is listed once there. The checker
    builds with its four pre-existing warnings (unused imports and
    `analyze_owner_source`) and no `unreachable_patterns`.

### Accounting

- **`wow-world-application/src`: 15,961 → 11,523 lines (−4,438), 61 → 44
  files.** 4,427 lines leave as files, and `lib.rs` goes from 98 to 87 lines
  (−11: two module lines and the two re-export items). `unit_tests/` is
  unchanged (241).
- **Destination.** `wow-world-loot-ops/src` 0 → 4,449 lines (4,427 moved,
  22 `lib.rs`), 18 files.
- **`crates/wow-world`**: +25 −24 in 17 files (paths, four comments, the
  manifest, one rustfmt fold). `net_move.py check`: S 0, G 11, 0 duplicate
  bodies, PASS. It reports NOT-APPLICABLE as a move check because
  `wow-world/src` does not shrink: the move is application → loot-ops.
- **Edges.**
  - Added: loot-ops → core, entities, instances, inventory, lifecycle,
    loot, quest, social, spell. It also gains `wow-constants`, `wow-core`,
    `wow-data`, `wow-entities`, `wow-handler`, `wow-loot`, `wow-map`,
    `wow-packet`, `wow-persistence` and external `tracing`.
  - Added: `wow-world` → loot-ops.
  - Removed: application → `wow-loot`, `wow-map`, `wow-world-social` (no
    remaining user).
  - Head closures: loot-ops → core, entities, instances, interaction,
    inventory, lifecycle, loot, quest, social, spell, visibility.
    Application's closure is unchanged: it still reaches social through
    quest.
  - Only `wow-world` reaches loot-ops. Only application, `wow-world` and
    loot-ops reach quest. Lock and metadata modes: PASS.
- **Policy (`dependency-policy.json`).**
  - `wow-world-loot-ops` joins the `application` category, restricted to
    its 18 actual dependencies, with external `tracing` only.
  - Application −= `wow-loot`, `wow-map`, `wow-world-social`.
  - New fixtures `forbidden-loot-ops-to-wow-world.json` and
    `forbidden-loot-ops-to-world-application.json` (self-test: 28
    fixtures).
  - `net-move-policy.json` adds the loot-ops `src/` destination root (16;
    `test_net_move.py` pins it).
  - The loot-ops crate gets a `test-fixtures` feature that forwards to its
    nine `wow-world-*` dependencies, plus a self dev-dependency, as quest
    has. The moved code has fixture-gated fields and arguments.
    `wow-world`'s `test-fixtures` and its dev-dependency forward to it.
    Application no longer forwards to social.
  - `cargo tree -p world-server -e normal,build -e features` shows no
    `test-fixtures` and no `wow-world-test-support`.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`.
  The delta is the 19 `loot_handlers` registry rows (package and source
  path) and the resulting sort order. With `wow-world-loot-ops` masked as
  `wow-world-application`, the old and new policies are equal.
- **Ceilings.** None raised.
  - One hotspot owner is tightened to its live values in
    `runtime-ownership-ledger.json`: loot handler test 18,985 → 18,983 and
    total 28,449 → 28,447. This is the `item_1.rs` fold. Production 9,464
    is unchanged.
  - Every physical row is unchanged. No moved file is above 1,000 lines
    (largest `loot_release/gameobject.rs`, 730).
- **Warnings.**
  - `wow-world` `dead_code` is 0 in the production lib, the lib with
    `test-fixtures` and the lib test build. `unused_imports` is 152 and 161
    in the first two, as in A6.
  - No changed `wow-world` line warns.
  - The warning set of application + loot-ops at head equals application's
    at base in all three builds (compared by file and message).
    loot-ops has one warning: the pre-existing production-only
    `unused_mut` in `loot_handlers/money.rs`.
- **Tests.** The Rust test-name multiset is unchanged: 10,215 test
  attributes, 5 ignored, the same names on base and head. The seven
  `loot_release/tests.rs` tests run in loot-ops (application 17 → 10 test
  functions in `src`, loot-ops 0 → 7). The closure self-test gains one
  Python case (10).

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run, so the PR head is
the validated SHA.

## Slice A summary (A1–A7)

Slice A split `wow-world-application` by domain without changing behaviour.
Every PR was move-only, kept the test-name multiset and registration order,
and passed the closure check in lock and metadata modes.

**`wow-world-application/src` (physical lines):**

| After | Lines | Files | Moved out |
|---|---:|---:|---|
| base `465f3791a` | 51,225 | 170 | — |
| A1 | 47,743 | 158 | vehicle, spell-click values, movement registrars → entities; instances → instances |
| A2 | 42,374 | 153 | trade, guild bank, reputation → social; group, battleground → instances |
| A3 | 38,833 | 144 | travel, combat, corpse, gameobject → entities; dungeon finding → instances; client state, data service, player save → lifecycle; npc → interaction |
| A4 | 36,164 | 139 | stats, bank, vendor, collections, battle pets → inventory |
| A5 | 30,330 | 112 | profession, spell acquisition → spell; character family → `wow-world-character` |
| A6 | 15,961 | 61 | quest cluster (+ conditions, valuation, registry sync, trainer admission proof) → `wow-world-quest` |
| A7 | 11,523 | 44 | loot handlers, loot release → `wow-world-loot-ops` |

That is −39,702 lines (−77.5 %). The plan expected about 11k of
orchestration after A7, and 11.5k remain.

**Crates created:**

- `wow-world-character` (A5, 3,088 lines);
- `wow-world-quest` (A6, 14,395);
- `wow-world-loot-ops` (A7, 4,449).

Each is in the `application` dependency category, restricted to its
actual dependencies. Each has forbidden-edge fixtures to `wow-world` and
`wow-world-application`, a net-move destination root, a session-ownership
package role and a closure rule placing it high:

- only application and `wow-world` reach character and loot-ops;
- only application, `wow-world` and loot-ops reach quest.

Existing crates grew from the slice A base:

- entities 10,588 → 13,793;
- inventory 10,990 → 13,668;
- lifecycle 9,286 → 11,407;
- social 8,764 → 11,309;
- spell 6,204 → 8,964;
- instances 1,268 → 5,575;
- interaction 571 → 782.

**What remains in application** (11,523 lines, cross-domain orchestration):

| Module | Lines | Why it stays |
|---|---:|---|
| `trainer_purchase` | 3,680 | money, quest events, aura application, spell acquisition and conditions in one C++ transaction |
| `inventory_swap` | 2,566 | equip rules, quest item counts, conditions, stats and publication |
| `aura_removal` | 1,587 | item effects, shapeshift, threat, mount and quest state |
| `spell_handlers` | 897 | borrows quest state for cast/cancel |
| `character_account_handlers` | 800 | composes bank, vendor, inventory and trainer registrars |
| `player_handlers` | 644 | borrows quest state, instances and visibility |
| `equipment_set_use` | 503 | inventory operation that still needs the quest-crate registry sync |
| `inventory_move_planning` | 324 | conditions and valuation (quest crate) |
| `inventory_scaling` | 220 | inventory operation that still needs the quest-crate registry sync |
| `talent_handlers`, `trainer_handlers` | 215 | registrars of the trainer/talent family |
| `lib.rs` | 87 | module declarations and re-exports |

**Recommendations for what comes next:**

1. **Make quest only quest** (#1378 review follow-up 2). `wow-world-quest`
   also holds four non-quest parts:
   - the player-condition projection (`player_conditions`);
   - `inventory_valuation`;
   - `registry_sync`, which goes to `wow-world-inventory` (point 2 and
     *`registry_sync` destination* below), not to the condition crate;
   - the trainer admission proof.

   Inventory, trainer, aura and equipment code all need these parts.
   Move the condition projection, valuation and the admission proof into a
   lower crate (for example `wow-world-player-conditions`,
   above inventory/spell/loot and below quest) before slice 6 or a trainer
   split. This is not a plain file move.
   `PlayerConditionProjectionCxLikeCpp` is extended by inherent impls in
   `quest/visibility.rs` and in valuation. The quest-side impl blocks must
   first become free functions or an extension trait, in a separate
   behaviour-neutral change. Then the type and valuation can move.
2. **Send `registry_sync`, `inventory_scaling` and `equipment_set_use` to
   `wow-world-inventory`** (A4/A6 *Next*). First make the fixture hydration
   in `registry_sync` quest-agnostic (behaviour-neutral, separate change).
   Then the three move together. This takes the two inventory operations
   (723 lines) out of application.
3. **Re-measure a `wow-world-trainer` split** (trainer purchase + handlers +
   talent, ~3.9k) after point 1. Trainer purchase then reaches the
   condition crate instead of quest. It still needs
   `PlayerAuraApplicationCxLikeCpp` from `aura_removal`: either aura removal
   moves with it, or the trainer crate takes the aura application context
   through a host trait.
4. **wow-world slices.** Lower crates (spell, loot, social, instances,
   character, creature) reach quest logic only through host traits or
   through application/`wow-world` (#1378 review follow-up 4).
   - **Slice 5 (loot, ~8k):** handler bodies and release/roll operations
     go to `wow-world-loot-ops`; pure loot state goes to `wow-world-loot`.
     A7 is its prerequisite and is now done.
   - **Slice 6 (quest, ~7k):** goes to `wow-world-quest`, after point 1 so
     the bodies land in a quest-only crate.
   - **Slice 7 (combat, pets, battle pets, progression, rest, xp, ~5k):**
     goes to entities, spell, inventory and lifecycle by owner. It needs
     its own closure pre-check because entities, spell and inventory may
     not reach social or instances.
   - **Slice 8 (character, ~12.5k):** login load and character handlers go
     to `wow-world-character`. Inventory moves and vendor go to inventory;
     visibility, gossip and transports go to visibility, interaction and
     entities. It needs its own cycle pre-check: nothing below character
     may gain an edge to it.
   - **Slice 9 (spell effects + player cast, ~7k)** follows.
5. **Checker hygiene** (#1378 review follow-up 3). Consider denying
   warnings for `handler-contract-check` in validation-v2. Its four
   remaining warnings (unused imports and `analyze_owner_source`) should be
   cleaned first, in a separate change.

### `registry_sync` destination (#1379 review follow-up 1)

Recommendations 1 and 2 above used to name two homes for `registry_sync`:
a new condition crate below quest, and `wow-world-inventory`. Settled on
`1423cfbfa`: **`registry_sync` goes to `wow-world-inventory`**, with
`inventory_scaling` and `equipment_set_use`, in two changes:

1. Behaviour-neutral: make `PlayerRegistryHydrationContext` quest-agnostic.
   Its only quest dependency is `&SessionQuestState` in fixture-gated code,
   read once for `player_quest_gameplay_fixture_like_cpp()`, whose result
   type is `wow_entities::PlayerQuestGameplayState`. A fixture-gated
   provider (closure or one-method trait) replaces the reference; the read
   still happens at the same step of the sequence.
2. Move the three units. Inventory gains `wow-world-loot` (`LootState`,
   normal) and `wow-world-spell` (`SessionSpellState`, used only by the
   fixture-gated hydration). Neither reaches inventory, social or
   instances on `1423cfbfa` (loot → core; spell → core, lifecycle), so
   there is no cycle.

Why inventory and not the condition crate:

- `registry_sync` has nothing to do with player conditions. It is the
  World-session registry publication: position, fixture hydration, loot
  rolls, party type. Its non-quest inputs are core, loot and spell state.
- Its consumers are inventory operations (`inventory_scaling`,
  `equipment_set_use`, the inventory swap's equip step, item publication)
  and loot release, trainer runtime install, aura removal and quest
  completion. All of these sit at or above inventory and can reach it.
- `equipment_set_use` then needs nothing above inventory. Its
  `wow_world_*` uses on `1423cfbfa` are core, inventory, loot and the two
  `registry_sync` types. `inventory_scaling` is the same plus its own
  private `restored_health_like_cpp`. Neither names a condition,
  valuation or quest type, so neither would need the condition crate.
- In the condition crate, which sits above inventory, `registry_sync` would
  pin every registry-publishing inventory operation above inventory. Those
  operations could then never move to inventory.

Constraint this adds: once inventory → loot exists, `wow-world-loot`
(pure loot state) may never depend on inventory. Slice 5 keeps it that way
(see *Slice 5 plan*).

## Slice 5 plan (loot, phase 2)

Measured on `1423cfbfa` (A7 merged) before anything moved. The phase 1 and
slice A estimates were off, so these figures come from the code, with the
same two lexical tools as slice 4: the hotspot classifier for production
lines, and a scan of the inherent `WorldSession` methods (body lines, the
`self.`/`session.` calls they make to methods defined elsewhere, and the
session fields they touch).

- **`wow-world` production (classifier): 82,910** lines, 86,410 physical,
  514 files.
- **The loot family** is `handlers/loot/**` (9,464 production, 27 files;
  the loot hotspot owner) plus `session/loot/**` (81). It has 128 inherent
  `WorldSession` methods with 7,457 body lines. Together they call 53
  methods defined outside the family and touch 13 session fields. The
  rest is the 554-line `mod.rs` (imports, constants, pure helpers and the
  two persistence workers), the `LootCx`/`LootCxRef` helper impls and
  the F5 adapter `loot_host.rs`.

| Group | Production | Methods / body lines | Outside calls |
|---|---:|---:|---:|
| `sources.rs` + `sources/` (creature, conditions, gameobject, gameobject authority) | 2,858 | 37 / 2,356 | 13 |
| `generation.rs` | 281 | 2 / 212 | 3 |
| `requests/context.rs` | 358 | 11 / 322 | 9 |
| `claims.rs`, `request_cache.rs`, `authority.rs`, `random_properties.rs` | 325 | 18 / 242 | 7 |
| `rolls.rs` | 883 | 11 / 851 | 5 (+7 into the loot family) |
| `requests.rs`, `requests/item_storage{,/disenchant}.rs` | 1,848 | 14 / 1,720 | 25 |
| `persistence.rs`, `money.rs`, `handlers.rs`, `handlers/money.rs` | 1,399 | 13 / 1,175 | 17 |
| `fanout.rs` | 237 | 4 / 222 | 6 |
| `combat_commands.rs`, `visibility_commands.rs` | 293 | 8 / 261 | 8 |
| `loot_host.rs`, `claims/release.rs`, `storage_plans.rs`, `mod.rs` | 982 | 3 / 43 | — |

"Outside calls" counts methods defined outside the loot family. In the
first four groups taken together the count is 29, because their calls
into each other become free calls.

### Destinations

- **`wow-world-loot-ops` (operations above quest).** Every body that
  needs the session goes here, behind a loot host trait in the F5
  inverted pattern. That covers opening and generating loot from sources,
  conditions and quest gates, rolls, item storage, persistence, money and
  the loot command handlers. The trait's supertrait is
  `WorldEntitiesHostLikeCpp`. It reuses the hub, the world-entity
  accessors and splits, and `resolved_group_guid_like_cpp`, and adds only
  what the bodies call.
  - The `LootCx`/`LootCxRef` helper methods in moved files move as free
    functions over the host: `self.<field>` becomes `host.<field>()`, as
    in 4a.
  - The two context structs and their builders stay in `wow-world`. Their
    unit-test shims extend `LootCx` with inherent impls, so the type
    cannot leave the crate.
- **`wow-world-loot` (pure state, below quest).** It takes the session-free
  value helpers that both sides still need: the loot-response item list
  and the client loot type. They use only `wow_loot`/`wow_packet` types,
  which the crate already depends on, so loot gains no edge. It still
  reaches only core.
  - Pure helpers that only moved bodies use stay private in loot-ops.
  - The *registry_sync* constraint above holds: loot never depends on
    inventory.
- **Stays in `wow-world`:**
  - the F5 handler adapters (`loot_host.rs`);
  - the loot-release context builder (`claims/release.rs`, which needs
    every disjoint session field);
  - the `LootCx`/`LootCxRef` builders;
  - the composition setters in `session/loot/operations.rs`;
  - `represented_local_loot_fixture_allowed_like_cpp`. It is
    `cfg!(test)` of the World crate, so moved bodies read it through a
    host method whose adapter keeps it.
  - `combat_commands.rs` and `visibility_commands.rs` are not loot. They
    are creature attack start/stop, PvP combat expiry and visibility
    refresh mailbox handlers, and they go with slices 7 and 11.

### 5a / 5b split

- **5a, lowest surface first.** The sources, generation, request context,
  claims, request cache, authority and random-properties facade groups:
  66 methods, 3,132 body lines, 29 outside calls, about 3.8k production
  lines with their helpers.
  - Their remaining calls are mostly accessors: the group registry, item
    templates and drop rates, loot stores, quest snapshot, known spells,
    the release context, the dungeon checks, the visible-gameobject
    refresh and `give_xp`.
  - The calls back into the rest of the loot family are the
    loot-opened publication, release-all, the dungeon-unlock checks and
    the direct item store.
  - At about 6 lines per host method that is about 2.6 % of the
    moved lines, well inside the 15 % budget.
- **5b, the rest of the operations.** Rolls, requests, item storage and
  disenchant, persistence, both money paths, the loot command handlers
  and fanout: 42 methods, 3,968 body lines, about 4.6k production lines.
  - Their surface is 54 outside methods: inventory insertion and
    publication, quest-objective drain and persistence plans, gold
    mutation, the kick, the creature melee publication and the registry
    sync.
  - That still fits the budget (about 8 %).
  - Slice 6 (quest) turns the quest-objective part into free calls.
    Doing 5b after 6 would make it cheaper, but it does not depend on
    6.
- **Closure.** 5a and 5b need no new `wow-world-*` edge. Loot-ops already
  depends on core, entities, instances, inventory, lifecycle, loot, quest,
  social and spell. The check is still simulated before each move.

## Slice 5a result (loot sources, generation and claim facades → `wow-world-loot-ops`)

Branch `1363-s5a-loot`, base `1423cfbfa` (A7 merged). Move-only (R2), with
the 4a/4b rewrites. Packets, their order, persistence, registrations and
processing flags are unchanged. No registrar moves.

The rewrites are:
- `self.` becomes `host.`;
- a session field becomes a host accessor;
- `hub_ref`/`split_loot_*` become host calls;
- a call to another moved function becomes a free call;
- a `LootCx`/`LootCxRef` helper method becomes a free function over the
  host. `self.<field>` becomes `host.<field>()`, and the one place that
  needed `&mut world_entities` together with the hub uses the
  entities host's `split_world_entities_mut`.

A token comparison of all 92 moved functions against the base finds no
difference beyond these rewrites and rustfmt brace folding. The remaining
differences are the fixture predicate, which is now a host call, and the
three former `#[cfg(test)]` steps.

Those three steps use the 4b mechanism:
`#[cfg(any(test, feature = "test-fixtures"))] if host.test_fixtures_like_cpp().is_some() …`.
They are the pristine-authority fixture bridge in the gameobject-chest and
creature-loot installs, and the fixture loot GUID in
`next_represented_loot_object_guid_like_cpp`.

`represented_local_loot_fixture_allowed_like_cpp` is `cfg!(test)` of the
World crate. It stays in `wow-world`, and the moved bodies read it through
a host method, so production still sees the constant `false`.

### What moved

- **To `wow-world-loot-ops/src/session_loot/`** (module layout as in
  `handlers/loot`). Ninety-two functions moved:
  - `sources.rs` and `sources/{creature, creature_conditions, gameobject,
    gameobject_authority}.rs`: the personal, chest, fishing and gathering
    gameobject loot; creature kill loot and AE targets; condition
    admission; and the gathering, chest and goober state-sync mailbox
    handlers;
  - `generation.rs` (disenchant templates, quest item drops);
  - `request_context.rs` (from `requests/context.rs`: group looters,
    dungeon trash looter, quest gates, loot player context);
  - `claims.rs`, `request_cache.rs`, `authority.rs` and
    `random_properties.rs` (the claim, cache, authority and
    random-properties facades);
  - the helpers only these bodies used, from the World `loot/mod.rs`: the
    master-loot error map, team/faction/quest-mask helpers, the generated
    loot-entry builders, the install observation, the disenchant frame
    and its table, and the loot-mode and condition constants.

  Functions the World still calls are `pub` and re-exported. The helpers
  only the World loot tests name are re-exported under
  `cfg(any(test, feature = "test-fixtures"))`.
- **To `wow-world-loot`** (`response.rs`): the session-free
  `SMSG_LOOT_RESPONSE` values that both sides need,
  `loot_type_for_client_like_cpp` and
  `represented_loot_response_items_like_cpp`. No new dependency is needed.
- **Stays in `wow-world`:**
  - the F5 adapter `loot_host.rs`;
  - the loot-release context builder (`claims/release.rs`);
  - `LootCxRef`;
  - the chest same-map refresh facade. Its name collides with the
    loot-release free function at the crate root, so it is reached through
    a host method.
  - the autostore facade, which `loot_host.rs` calls by UFCS;
  - the fixture predicate;
  - the cfg(test) entry points of `sources.rs` and `sources/gameobject.rs`;
  - everything 5b moves.
- **Retired.** `LootCx` (struct and builder). Its last readers moved; the
  one unit-test shim that used it now sets the group-loot money port on
  the lifecycle state directly. `LootCxRef.loot` is also retired; nothing
  reads it any more.

### Host surface

- **`LootSessionHostLikeCpp: WorldEntitiesHostLikeCpp`** (loot-ops,
  32 methods, 123 lines). It has six field accessors and splits (loot ref
  and mut, the two loot splits, lifecycle, inventory), the fixture
  predicate, and 25 operations: map dungeon checks, release-all, `give_xp`,
  group and player registries, item templates and drop rates, the
  item-addon port, known spells, loot rates and stores, the loot-release
  context, the canonical gameobject mutation, store planning, the quest
  snapshot, the chest refresh, item counts, money roll, loot-opened
  publication, the two dungeon-unlock checks, the two direct item stores
  and the visible-gameobject refresh.
  - The supertrait supplies the hub, the world-entity accessors and
    splits, and `resolved_group_guid_like_cpp`.
- **Adapter** `handlers/loot/session_loot_host.rs` (206 lines). Every
  method is a pure forward to the same-named inherent method or field.
- **Recursion check.** No moved name is forwarded through a trait to
  itself.
  - The entities adapter's four `open_represented_*` forwards and its
    goober refresh name `wow_world_loot_ops::…`.
  - The `loot_host.rs` UFCS calls name `wow_world_loot_ops::…`.
  - Every one of the 32 forwards resolves to an inherent method.
- **Budget.** 329 host lines (trait + adapter) against about 3,880 moved
  lines: 8.5 %.
- **Test shims.** `handlers/loot_test_shims.rs` (354 lines, mounted
  `cfg(test)` from `handlers/mod.rs`, outside the loot hotspot tree) holds
  the 25 method-syntax entry points the loot scenarios call. It also holds
  the test-only re-exports, which `handlers/loot/mod.rs` reaches with one
  `cfg(test)` glob. No test call site changed. Three f3 shim bodies call
  the moved functions; they are one line each.

### Accounting

- **`crates/wow-world/src`.** −4,034 / +751 (net −3,283). Classifier:
  82,910 → **79,633 production (−3,277)**, 86,410 → 83,127 physical, 514
  → 511 files. The +751 is the adapter (206), the test shims (354) and
  the rewritten call sites and shells.
- **Destinations.**
  - `wow-world-loot-ops/src` 4,449 → 8,600 (+4,151): `session_loot/`
    4,096, of which 123 are the trait and about 95 the prelude imports,
    plus 55 lines of `lib.rs` exports.
  - `wow-world-loot/src` +75: 73 moved lines and 2 export lines.
- **`wow-world-application`: 11,523 lines, unchanged.**
- **`net_move.py check`.** S = 3,283, G = 4,226 ≤ 4,408.1, 0 duplicate
  bodies, PASS.
- **Edges.** No `wow-world-*` edge is added or removed. Loot-ops gains
  `wow-social` (`GroupRegistry` in the host signature), `wow-conditions`
  (quest-status constants) and external `rand` (`Rng` for the random
  properties); `dependency-policy.json` adds the three.
  - Head closures are unchanged. Loot reaches only core; only `wow-world`
    reaches loot-ops; quest is reached only by application, `wow-world`
    and loot-ops.
  - The closure check passes in lock and metadata mode, before and after.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`.
  The delta is:
  - the moved impl items;
  - the 24 direct-registry rows that moved to package
    `wow-world-loot-ops` (`crate::session_loot::…`);
  - the adapter's trait impl;
  - the cfg(test) shim impl.

  The syntax-only check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `handlers/loot/mod.rs`: production 9,464 → 5,854, test
    18,983 → 18,969, total 28,447 → 24,823;
  - hotspot `session/mod.rs`: production 42,629 → 42,602, test 142,047 →
    142,042, total 184,676 → 184,644;
  - physical `handlers/loot/sources.rs` 1,003 → 62,
    `handlers/loot/handlers.rs` 996 → 383, `handlers/loot/requests.rs`
    419 → 260.
- **Warnings.**
  - `wow-world` `dead_code` is 0 in the production lib, the lib with
    `test-fixtures` and the lib test build.
  - `unused_imports` 152 → 151, 161 → 160 and 321 → 320 (the stale
    `HighGuid` import is dropped).
  - Every other package's warning set is identical to base in all three
    builds. Loot-ops keeps only its pre-existing `unused_mut`.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed, and the
  test-name multiset is unchanged. The 346 wow-world tests matching `loot`
  pass.

### 5b plan

Rolls, requests, item storage and disenchant, persistence, both money
paths, the loot command handlers and fanout move to `session_loot` behind
the same trait. That is about 4.6k production lines and 54 outside
methods: inventory insertion and publication, quest-objective drain and
persistence plans, gold mutation, the kick, the creature melee
publication and the registry sync.

- The persistence workers and the pure roll helpers (`connected_roll_looters`,
  `start_loot_roll_packet`, `loot_item_context`) go with them.
- `combat_commands.rs` and `visibility_commands.rs` are not loot; they go
  to slices 7 and 11.
- 5b re-measures on its base.

### Follow-ups

- The chest same-map refresh facade could take a distinct name, so the
  host method can go.
- The test shim file is large (354 lines). Converting the loot scenarios
  to free-function calls would retire it, at the cost of loot test-line
  growth, which needs a reviewed test ceiling.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 5b result (loot rolls, requests, item storage, persistence and fanout → `wow-world-loot-ops`)

Branch `1363-s5b-loot`, base `5d0a0b82f` (5a merged). Move-only (R2), with
the 5a rewrites. Packets and their order, money and item mutation order,
item creation and store order, roll RNG order, persistence and save
ordering, kicks, registrations and processing flags are unchanged. No
registrar moves.

### Re-measure and split

On `5d0a0b82f` the planned 5b scope is about 4,520 classifier production
lines, just over the ~4.5k cap:

| Group | Production |
|---|---:|
| `rolls.rs` | 900 |
| `requests.rs`, `requests/item_storage{,/disenchant}.rs` | 1,865 |
| `persistence.rs` | 364 |
| `fanout.rs` | 237 |
| `mod.rs` roll helpers and the two persistence workers | about 190 |
| `money.rs`, `handlers/money.rs`, `handlers.rs` | 963 |

The split follows a call boundary. The rolls, requests, item storage,
persistence and fanout family calls nothing in the money files or the
command-handler files. The money paths and the command handlers call into
that family. So:

- **5b** moves that family, about 3,560 lines.
- **5c** moves the two money paths and the loot command receivers, about
  960 lines.

### What moved

- **To `wow-world-loot-ops/src/session_loot/`** (module layout as in
  `handlers/loot`). Forty-three functions moved:
  - `rolls.rs`: votes, finish, winner storage, the remote winner request,
    the first-open roll start, the tick, and the disenchant skill gate;
  - `requests.rs`: the loot response, loot-opened publication, generation
    match, the dungeon-unlock checks and the stale-view close;
  - `requests/item_storage.rs` and `requests/item_storage/disenchant.rs`:
    direct item storage, disenchant winner storage and the disenchant
    batch. The `LootCxRef` storage-plan helper became a free function over
    the host;
  - `persistence.rs`: the durable fanout preparation, the claim commit, the
    removal publication, the pending-completion replay and the
    active-persistence wait. The claim and item persistence workers and
    `LootClaimPersistenceWorkerError` moved from `mod.rs`. The worker that
    was `cfg(test)` is now gated `cfg(any(test, feature = "test-fixtures"))`;
  - `fanout.rs`: the visibility gate, the loot-list and item-removed
    notifications and the item push result.

  Functions the World still calls are `pub` and re-exported. The ones only
  the World loot tests name are re-exported under
  `cfg(any(test, feature = "test-fixtures"))`.
- **To `wow-world-loot`** (`roll_values.rs`): the session-free roll values
  `connected_roll_looters_like_cpp`,
  `represented_max_enchanting_skill_like_cpp`,
  `start_loot_roll_packet_like_cpp`, `loot_item_context` and
  `LOOT_ROLL_TIMEOUT_MS_LIKE_CPP`. They need only `wow-world-core`,
  `wow-packet`, `wow-constants` and `num-traits`, which the crate already
  has.
- **To `wow-world-quest`** (`session_state/contracts.rs`):
  `QuestSourceItemBoundPreflightLikeCpp` and
  `QuestSourceItemBoundPersistencePlanLikeCpp`. These are two plain records
  the host signatures name. Their fields become `pub`, and the World quest
  module re-exports them.
- **Stays in `wow-world`:**
  - the money paths and the loot command receivers (5c);
  - the F5 adapters;
  - the release context builder;
  - `combat_commands.rs` and `visibility_commands.rs` (slices 7 and 11);
  - the fixture predicate;
  - the unit-test module mounts. `requests.rs`, `persistence.rs` and
    `fanout.rs` are now shells that only mount their `unit_tests` modules.
  - `rolls.rs`, `requests/item_storage{,/disenchant}.rs` and
    `random_properties.rs` are deleted. The last of these had no reader
    left.

### Rewrites and fixture steps

The rewrites are the 5a set:
- `self.` becomes `host.`;
- a field becomes a host accessor;
- `split_*` becomes a host call;
- a call to a moved function becomes a free call.

The World inherent methods that are only `self.core.X(...)` (`kick`,
`send_tx`, `send_equip_error`) are reached as `host.core[_mut]().X(...)`.
The World `cfg(test)` wrappers `tick_represented_loot_rolls_like_cpp`,
`apply_pending_durable_item_loot_completions_like_cpp` and
`wait_for_active_loot_persistence_like_cpp` moved to
`handlers/loot_test_shims.rs` and call the free functions.

A normalised token comparison of all 43 moved functions against the base
finds 38 identical. The other five differ only as follows:
- **Fixture steps.** The two `#[cfg(test)]` store-grant steps in item
  storage and the disenchant batch use the 4b mechanism:
  `#[cfg(any(test, feature = "test-fixtures"))] if host.test_fixtures_like_cpp().is_some() && let Some(grants) = …`.
- **Dungeon-encounter fallback.** It is a `&self` body, so its former
  `#[cfg(test)]`/`#[cfg(not(test))]` pair reads the World `cfg!(test)`
  through `host.represented_local_loot_fixture_allowed_like_cpp()`. The
  production result is still the constant `false`.
- **Loot-opened publication.** It reads the same fixture predicate through
  the host.
- **Storage-plan helper.** It reaches the catalogs through
  `host.hub_ref()`.
- **rustfmt.** It folds two single-expression closures in the store
  bodies.

The 5a sources and claims now call the moved storage, loot-opened and
dungeon-unlock functions as free calls.

### Host surface

- **`LootSessionHostLikeCpp`**: 61 methods, 196 lines (5a: 32 methods,
  123 lines).
  - **Removed (6).** Five 5a methods whose bodies now live in loot-ops: the
    loot-opened publication, the two dungeon-unlock checks and the two
    direct stores. The sixth is the chest same-map refresh (#1380
    follow-up 2). Its World `LootCxRef` facade became the loot-ops free
    function `queue_session_chest_gameobject_state_refresh_for_same_map_like_cpp`,
    so it no longer collides with the loot-release function at the crate
    root.
  - **Added (35).** The quest-state accessors and the inventory split.
    Inventory insertion, lookup and publication. Collection, item-template
    and stats-store reads. The four durable item-loot persistence
    operations of the inventory context. The quest-objective preflight,
    progress, drain, persistence plan and save. Gold read and write.
    Pass-on-loot. The dungeon-lock query. The creature message-to-set
    check. The owner release. `Loot::NotifyMoneyRemoved`. The command
    sender, and the registry sync.
- **Adapter** `handlers/loot/session_loot_host.rs`: 342 lines (5a: 206).
  - Every method is a pure forward to the same-named inherent method or
    field.
  - The four durable item-loot operations and `make_inventory_item_object`
    go through `cx_inventory_ref`, as the moved bodies did.
  - `Loot::NotifyMoneyRemoved` goes through the loot handler owner's
    default method by explicit UFCS.
- **Recursion check.** A scan of the adapter finds that every same-named
  `self.` forward has an inherent `WorldSession` method. No other forward
  names a moved function. rustc reports no `unconditional_recursion`.
- **Budget.** +73 trait lines and +136 adapter lines (209) against about
  3,570 moved lines: 5.9 %. The whole 5a+5b host is 538 lines against
  about 7,450 moved lines: 7.2 %.
- **`LootCxRef`** keeps only `lifecycle`. Its `inventory`, `world_entities`
  and `hub` lost their last readers.
- **Test shims.** `handlers/loot_test_shims.rs` (outside the loot hotspot
  tree) grows by 221 lines to 572. It gains:
  - the three moved `cfg(test)` wrappers;
  - eleven method-syntax entry points for the moved functions the loot
    scenarios call;
  - the test-only names the scenarios read through `super::*`.

  No test call site changed.

### Accounting

- **`crates/wow-world/src`.** −3,767 / +496. Classifier: 79,633 →
  **76,402 production (−3,231)**, 83,127 → 79,856 physical, 511 → 507
  files.
- **Destinations.**
  - `wow-world-loot-ops/src` 8,600 → 12,244 (+3,644). The six moved files
    are 3,462 lines; the rest is the trait, the prelude imports, the
    exports, the chest facade and the 0xBADD note.
  - `wow-world-loot/src` +111 (106 moved, 5 export lines).
  - `wow-world-quest/src` +18.
- **`wow-world-application`: 11,523 lines, unchanged.**
- **`net_move.py check`.** S = 3,271, G = 3,773 ≤ 4,395.6, 0 duplicate
  bodies, PASS.
- **Edges.** No `wow-world-*` edge is added or removed. Loot-ops gains
  external `tokio` and `flume` for the persistence workers;
  `dependency-policy.json` allows both. The closure check passes in lock
  and metadata mode. Loot reaches only core; only `wow-world` reaches
  loot-ops.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`.
  The delta is:
  - the moved impl items;
  - the direct-registry rows that moved to `wow-world-loot-ops`
    (`session_loot::{rolls, persistence, fanout}`) and to `wow-world-loot`
    (`roll_values`);
  - the adapter's trait items;
  - the test-shim impl.

  The syntax-only check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `handlers/loot/mod.rs`: production 5,854 → 2,413, test 18,969
    → 18,929, total 24,823 → 21,342;
  - physical `handlers/loot/requests.rs` 260 → 18, and
    `handlers/loot/handlers.rs` 383 → 379. The calls name
    `wow_world_loot_ops` and pass the session (+3 lines). The stale 0xBADD
    placeholder note moved to the `CMSG_SET_LOOT_SPECIALIZATION`
    registration it describes in loot-ops (−7 lines).
  - The session hotspot is unchanged at its ceiling. Two lifecycle call
    sites import the moved wait so it does not grow.
- **Warnings.**
  - `wow-world` `dead_code` is 0 in the production lib, the lib with
    `test-fixtures` and the lib test build.
  - `unused_imports` 151 → 143, 160 → 152 and 320 → 304. The World loot
    module drops the imports only the moved bodies used, and the
    test-only ones move to the shim file.
  - No new warning in any package. Loot-ops keeps only its pre-existing
    `unused_mut`.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed, and no test
  file changed, so the test-name multiset is unchanged. The 346 wow-world
  tests matching `loot` pass.

### Remaining loot in `wow-world`

- **Production.** `handlers/loot` is 2,413 lines:
  - **5c:** `money.rs` 67, `handlers/money.rs` 515, `handlers.rs` 379;
  - the F5 adapters `loot_host.rs` and `session_loot_host.rs`;
  - the release context builder `claims/release.rs`;
  - `combat_commands.rs` and `visibility_commands.rs` (slices 7 and 11);
  - the `mod.rs` shell;
  - the cfg(test) entry points.

### 5c plan

The two money paths (the durable payout, `CMSG_LOOT_MONEY` and the
apply-loot-money receiver) and the loot command receivers (roll vote,
master-loot give and roll-store-winner) move to `session_loot` behind the
same trait.
- That is about 960 lines.
- The surface is the gold coordinator and persistence, the money recipients
  and the loot-money persistence default methods, the stored-item money
  persistence, and the creature-melee publication.
- The creature melee-damage and loot-release-values receivers in
  `handlers.rs` are not loot. They go with slice 7.

### Follow-ups

- The chest same-map refresh facade has a distinct name, so its host method
  is gone (#1380 follow-up 2).
- The test shim file stays: 572 lines, outside the loot hotspot. Converting
  the loot scenarios to free-function calls would raise the loot test
  ceiling, which this slice does not do (#1380 follow-up 1, deferred).

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 5c result (loot money paths and command receivers → `wow-world-loot-ops`)

Branch `1363-s5c-loot`, base `20a836de5` (5b merged). Move-only (R2), with
the 5a/5b rewrites. Gold mutation order, the money-removed notifications,
the split and share math, the master-loot and roll-vote packet order, kicks,
the persistence and save ordering (#169), registrations and processing
flags are unchanged. No registrar moves.

### Re-measure

On `20a836de5` the 5c scope is 961 lines: `money.rs` 67,
`handlers/money.rs` 516 (including two nested `cfg(test)` entry points the
classifier counts as production) and `handlers.rs` 379, of which 205 are
the three loot receivers and 172 are the two non-loot receivers.

### What moved

- **To `wow-world-loot-ops/src/session_loot/`** (module layout as in
  `handlers/loot`). Five functions moved:
  - `money.rs`: the durable loot-money payout
    (`apply_durable_represented_loot_money_payout_like_cpp`);
  - `handlers/money.rs`: the `CMSG_LOOT_MONEY` body (C++
    `LootHandler.cpp:142` `HandleLootMoneyOpcode`), with its authority,
    stored-item and legacy fixture paths;
  - `handlers.rs`: the roll-vote, master-loot give and roll winner store
    session-command receivers.

  All five are `pub` and re-exported. The `CMSG_LOOT_MONEY` registration
  stays in `loot_handlers`; the World `loot_host.rs` adapter forwards it,
  and the durable payout, to the free functions. `session_commands.rs`
  calls the three receivers as free functions.
- **Stays in `wow-world`:**
  - the creature loot-release VALUES and creature melee-damage receivers in
    `handlers.rs`. They are not loot; they go with slice 7;
  - the F5 adapters, the release context builder, the fixture predicate,
    `combat_commands.rs` and `visibility_commands.rs`.
- **Retired.** `handlers/loot/storage_plans.rs`, a re-export module whose
  last reader moved. The scenarios now take `LootItemClaimCommitContextLikeCpp`
  from the test shims. The `loot_money_durable_outcome_like_cpp`
  re-export in `session/mod.rs` also lost its last production reader, and
  the scenarios now name it through the test shims.

### Rewrites

The 5b set: `self.` becomes `host.`, a field becomes a host accessor, and a
call to a moved function becomes a free call. One `LootCxRef` helper is
inlined over the host: `cx_loot_ref(self).durable_loot_money_persistence_tracker_like_cpp()`
becomes `Arc::clone(host.lifecycle().durable_loot_money_persistence_tracker_like_cpp())`,
which is the helper's body. The World helper stays for its other callers.
`crate::session::` paths to the lifecycle types name `wow_world_lifecycle`.

A normalised token comparison of the moved bodies against the base,
with these rewrites applied, finds no other difference beyond rustfmt
trailing commas. There were no `cfg(test)` steps in the moved bodies. The
two `cfg(test)` entry points (`handle_loot_money`,
`handle_apply_loot_money_like_cpp_command`) dispatch through the
registration and the default method, so they moved unchanged.

### Host surface

- **`LootSessionHostLikeCpp`**: 66 methods, 230 lines (5b: 61 methods,
  196 lines). Five added:
  - four forward by UFCS to the loot handler owner's default methods:
    the apply-loot-money receiver, the money recipient selection, the
    group loot-money persistence worker and the stored-item money
    persistence;
  - one forwards to the World exclusive gold coordinator
    (`mutate_and_persist_player_gold_exclusive_like_cpp`).
- **Adapter** `handlers/loot/session_loot_host.rs`: 389 lines (5b: 342).
- **Recursion check.** The four UFCS forwards name
  `<Self as LootHandlerHostLikeCpp<…>>::…`, whose default bodies call
  `loot_handlers::money` and `wow_world_lifecycle` free functions. The gold
  forward resolves to the inherent method. The moved `CMSG_LOOT_MONEY` body
  awaits the apply receiver, whose default body reaches the moved payout
  through `loot_money_apply_durable_payout_like_cpp`. Neither calls back
  into the `CMSG_LOOT_MONEY` body.
- **Budget.** +34 trait lines and +47 adapter lines (81) against 760 moved
  lines: 10.7 %. The whole 5a–5c host is 619 lines against about 8,210
  moved lines: 7.5 %.

### `loot_test_shims.rs` ratchet (#1381 follow-up 1)

- **Ceiling.** `crates/wow-world/src/handlers/loot_test_shims.rs` now has a
  physical-file ceiling at its live size, 441 lines. It is mounted
  `cfg(test)` outside the loot hotspot tree, so the hotspot cannot guard it.
- **Growth from the move (+39).** The two `cfg(test)` money entry points
  from the moved file, and the re-exports the World loot module no longer
  imports (`loot_is_looted_like_cpp`, `LootItemClaimCommitContextLikeCpp`,
  `loot_money_durable_outcome_like_cpp`).
- **Retirement (−170).** Eleven single-caller method shims are retired. Their
  scenario call sites now call `wow_world_loot_ops::…(&mut session, …)`.
  Each conversion was measured on its own, and only the set that keeps the
  loot test lines at 18,929 was kept. The retired shims are the creature
  loot-release VALUES, the gameobject chest generation, the kill-loot
  install, the durable fanout preparation, the item-removed notification,
  the two personal-gameobject upserts, the master-loot removal, the
  loot-opened publication and the two direct stores.
- **Result.** 572 → 441 lines. The other converted call sites would each add
  one to six test lines, so they stay for a reviewed test-ceiling change.

### Accounting

- **`crates/wow-world/src`.** −1,007 / +108. Classifier: 76,402 →
  **75,503 production (−899)**, 79,856 → 78,957 physical, 507 → 504 files.
- **Destinations.** `wow-world-loot-ops/src` 12,244 → 13,063 (+819). The
  three moved files are 760 lines; the rest is the trait (+34), the
  imports and the exports.
- **`net_move.py check`.** S = 899, G = 819 ≤ 1,905.0, 0 duplicate bodies,
  PASS.
- **Edges.** None added or removed. The closure check passes in lock and
  metadata mode.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`.
  The delta is the moved impl items, the test-shim impl items and one
  `test_only.rs` signature that now names the mailbox path of
  `MasterLootGiveResult`. The syntax-only check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `handlers/loot/mod.rs`: production 2,413 → 1,643, test 18,929
    unchanged, total 21,342 → 20,572;
  - hotspot `session/mod.rs`: production 42,602 → 42,601, total 184,644 →
    184,643;
  - physical `handlers/loot/handlers.rs` 379 → 171;
  - new physical row `handlers/loot_test_shims.rs` at 441.
- **Warnings.** The warning sets of the production lib, the lib with
  `test-fixtures` and the lib plus lib-test build are identical to base:
  171, 243 and 426 lines, `wow-world` `dead_code` 0 in all three.
- **Tests.** No `#[test]`, `#[ignore]` or assertion is added, removed or
  changed, so the test-name multiset is unchanged. The 393 wow-world tests
  matching `loot` or `money` pass.

### C++ comparison

`HandleLootMoneyOpcode` (`LootHandler.cpp:142-214`) was read only to confirm
that the move preserved the existing Rust behaviour. Its known divergences
are already recorded as `#LegacyAudit.LOOT.3`
(`docs/audits/cpp-parity-findings.md`): `MoneyMod` is always 0 because
`SPELL_AURA_MOD_MONEY_GAIN` is not applied, and the
`MoneyLootedFromCreatures` criteria is not updated. No new divergence was
found, and nothing was changed.

### Slice 5 summary (5a–5c)

- **Out of `wow-world`.** 7,407 production lines: 5a −3,277, 5b −3,231 and
  5c −899. Production fell from 79,633 + 3,277 = 82,910 to 75,503.
  - The loot hotspot production fell from 9,464 to 1,643.
  - `wow-world-loot-ops` grew from 4,449 to 13,063 lines.
  - `wow-world-loot` gained the session-free response and roll values.
- **Loot still in `wow-world`, and why.**
  - The F5 adapters `loot_host.rs` (370) and `session_loot_host.rs` (389)
    implement the host traits for `WorldSession`.
  - The loot-release context builder `claims/release.rs` (60) needs every
    disjoint session field.
  - The fixture predicate is the World crate's `cfg!(test)`.
  - The test-shim mounts in the module shells are `cfg(test)`, as are
    `handlers/test_shims.rs` and the `sources` entry points.
  - `loot_test_shims.rs` (441) holds the remaining method shims and
    re-exports.
- **Not loot, still in the tree.** `combat_commands.rs`,
  `visibility_commands.rs` and the two receivers in `handlers.rs` go with
  slices 7 and 11.

### Follow-ups

- Retire the rest of `loot_test_shims.rs` in a slice that reviews the loot
  test ceiling, since each remaining conversion adds test lines.
- Move the two non-loot receivers in `handlers.rs` with slice 7.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 7 plan (combat, pets, battle pets, progression, rest, xp)

Measured on `2d6504c7b` (5c merged) before anything moved, with the slice 5
tools: the hotspot classifier for production lines, and the inherent
`WorldSession` method scan for body lines, outside calls and session fields.

- **`wow-world` production (classifier): 75,503** lines, 78,957 physical,
  504 files.
- The six families hold about 5.9k classifier lines. About 1.3k of that is
  `#[cfg(test)]` methods nested in production impls. The classifier counts
  them as production, but they are fixture paths, not movable bodies.

| Family | Files | Production | Outside calls / fields |
|---|---|---:|---|
| Combat | `session/combat/*`, `handlers/combat*`, `combat_handler_contexts.rs`, `handlers/loot/combat_commands.rs`, the two receivers in `handlers/loot/handlers.rs` | 1,167 | 11 / core, view, catalogs, instances, fixtures, spell state |
| Pets | `session/pets/{pet,summoning,persistence}.rs`, `session/pet_loading.rs`, `handlers/character/world_entry/login/pet_loading.rs`, `handlers/pets/*`, `spell_pet_catalogs.rs`, `pet_dismissal.rs` | 1,293 | 1 / the `PetsCx` split (lifecycle + hub), fixtures |
| Battle pets | `battle_pet_purchase{,/*}.rs` (the #161 saga, 1,263), `session/pets/battle_pet{,_journal,_publication,_slots}.rs`, `session/battle_pet_adapter.rs` | 2,629 | saga: 11 plus 5 `PetsCx` helpers / core; durable ops: 9 / lifecycle, fixtures |
| Progression | `session/progression/*`, `progression_adapters.rs`, `handlers/progression` | 509 | 11 (16 with single-caller helpers) / catalogs, config |
| Rest | `session/rest_progression.rs` | 261 | 5 / lifecycle, fixtures |
| XP | `session/xp_grants.rs` | 120 | 0 / 7 fields (the quest reward context builder) |

Findings:

- **Combat goes to entities.** The player melee result and the attack start
  already live there (4a). The rest of combat needs only six host methods on
  top of `WorldEntitiesHostLikeCpp` and `PlayerMovementHostLikeCpp`.
  - The creature loot-release VALUES receiver is the receiving half of the
    loot-release fanout. Its body calls only loot-ops functions, so it goes to
    loot-ops with one host method (the pending-bind read). In entities it
    would need two loot-ops forwards, and entities may not depend on
    loot-ops.
- **Pets go to lifecycle.** The pet load state and its row types already live
  there. The `PetsCx` methods touch only the lifecycle state and the hub, so
  the host needs only that split and the at-login flags.
- **The battle-pet purchase saga goes to inventory**, beside the battle-pet
  handlers that A4 moved there. It needs 13 host methods. Its offer record
  (`PreparedBattlePetTrainerOfferLikeCpp`) moves down from application, which
  re-exports it at the old path. The durable grant, level and uncage
  operations are `&self` bodies with `cfg(test)`-only return branches, so they
  cost a fixture predicate each.
- **Progression, rest and XP have the highest surface per line.** They are
  mostly composition setters (`set_*_store`), which stay in `wow-world` like
  the loot composition setters, plus small fixture-heavy forwards.
- **Closure.** No new `wow-world-*` edge is needed. Entities, inventory,
  lifecycle and loot-ops already reach core and lifecycle, and loot-ops
  already reaches entities. With no planned edge there is nothing to
  simulate with `--add`; `wow_world_closure.py check` passed on the base
  (lock) and passes on the head (lock and metadata).
  Inventory gains only the external `rand` and `tokio` crates (the saga's
  request key and retry backoff).

**7a, lowest surface first (about 3k lines):**

- combat, with the two receivers and the melee publication path;
- the battle-pet purchase saga;
- pet loading, mode state, the login pet loader and resummon.

**7b (about 2.4k classifier lines, of which about 1k are nested `cfg(test)`
methods):**

- the battle-pet durable grant, level and uncage operations, the journal
  helpers, the adapter and `CMSG_DISMISS_CRITTER`;
- progression (reputation, skills, talents);
- rest and xp;
- the combat and pet leftovers (the resurrection snapshot and the setters).

## Slice 7a result (combat → entities; battle-pet purchase saga → inventory; pet loading → lifecycle)

Branch `1363-s7a-combat`, base `2d6504c7b` (5c merged). Move-only (R2), with
the 4b/5a rewrites:

- `self.` becomes `host.`;
- a field becomes a host accessor;
- `hub_ref`/`hub_mut` and a `split_*` become host calls;
- a call to another moved function becomes a free call;
- a `World` inherent method that is only `self.core.X(...)` (`send_packet_realm`,
  `battlenet_account_id`, `kick`, `send_tx`) is reached as
  `host.core[_mut]().X(...)`.

Damage, threat, melee and regeneration math, the swing presentation order,
the absorb log and shield removal order, the power and food-emote
publication, the PvP and FFA flag transitions, the saga's charge, apply,
publication, completion and compensation order, its retry and backoff, the
pet row filters and resummon, persistence ordering (#169), registrations and
processing flags are unchanged. No registrar moves.

### What moved

- **To `wow-world-entities/src/combat_session/`** (1,023 lines).
  - `regeneration.rs`: `Player::RegenerateAll` with its health input, the
    food/drink selector and visual.
  - `pvp_state.rs`: the PvP flag expiry, `CombatStop`, the contested-PvP reset
    and the realm FFA flag (login and tavern).
  - `damage.rs`: the melee absorb publication and the `SMSG_POWER_UPDATE`
    owner copy.
  - `commands.rs`: the creature attack start and stop, the PvP combat expiry
    and the creature melee-damage receivers.
- **To `wow-world-loot-ops`** (`session_loot/handlers.rs`, +75): the creature
  loot-release VALUES receiver.
- **To `wow-world-inventory/src/battle_pet_purchase/`** (1,264 lines): the
  #161 saga (its state, both ops modules and its doc) and
  `PreparedBattlePetTrainerOfferLikeCpp`.
- **To `wow-world-lifecycle/src/pet_session/`** (1,005 lines):
  - `loading.rs`: the pet row loaders, the declined names, the pet mode state
    and the aura remain-time helper;
  - `login_loading.rs`: the login pet-state loader and the
    `AT_LOGIN_RESET_PET_TALENTS` reset;
  - `summoning.rs`: the temporary-unsummon resummon, with the three pure
    helpers only it used.
- **Each former `PetsCx` method** takes the same lifecycle/hub split from the
  host at its first statement, where the World built the context, and keeps
  the rest of its body.
- **Fixture steps.** The former `#[cfg(test)]` steps use the 4b mechanism:
  `#[cfg(any(test, feature = "test-fixtures"))] if host.test_fixtures_like_cpp()…`.
  They are the PvP-expiry and contested-PvP fallbacks, the pet mode-state
  fixture writes, and the resummon counters. In the resummon the predicate is
  read before the split, because the split holds the host.
- **Stays in `wow-world`:**
  - the composition setters (`set_*_store`, `set_*_game_table(s)`,
    `set_*pvp_realm_like_cpp`, the battle-pet attachment);
  - the resurrection-state snapshot, which is `cfg(test)` fixture code;
  - the `cfg(test)` saga wrappers (`battle_pet_purchase/ops_1.rs`);
  - the f3 shim mounts;
  - everything 7b moves.
- **Retired.** `LootCxRef` (struct, builder and its last helper; #1382 review
  follow-up). The saga was its last production reader.
  `handlers/loot/combat_commands.rs`, `battle_pet_purchase/{ops_2,state}.rs`,
  `handlers/character/world_entry/login/pet_loading.rs` and
  `unit_tests/session/pet_loading/f3_shims.rs` are deleted.

### Host surface

- **`CombatSessionHostLikeCpp: WorldEntitiesHostLikeCpp + PlayerMovementHostLikeCpp`**
  (entities, 6 methods, 13 lines): the visible-set fanout, the melee health
  presentation, the death durability loss, the resting and polymorph reads and
  the FFA realm flag. Adapter `session/combat/host.rs` (33 lines).
- **`LootSessionHostLikeCpp`** gains `has_pending_bind_like_cpp` (+2 trait,
  +3 adapter lines).
- **`BattlePetPurchaseHostLikeCpp: SessionHubHostLikeCpp`** (inventory, 13
  methods, 32 lines):
  - the lifecycle accessor and split;
  - the four journal reads and the lease;
  - the purchase criteria;
  - money read, staging and publication;
  - the quest-objective drain, the dependent spell learn and the exclusive
    money boundary.

  Adapter `battle_pet_purchase/host.rs` (95 lines).
- **`PetSessionHostLikeCpp: SessionHubHostLikeCpp`** (lifecycle, 2 methods):
  the `PetsCx` split and the at-login flags. Adapter `session/pets/host.rs`
  (22 lines).
- **Recursion check.** Every adapter method names an inherent method by
  path (`WorldSession::name(self, …)`), a `PetsCx`/`PetsCxRef` method, a
  session split helper or a field. Each path-named method has an inherent
  `WorldSession` method of that name, so none resolves to its own trait
  method. rustc reports no `unconditional_recursion`.
  No UFCS forward goes to a trait default.
- **Borrows.** The journal-lease forward keeps the `PetsCxRef` borrow across
  its await inside the returned future, as the body did. The drain and money
  boundary forwards return the inherent futures unchanged.
- **Budget.** 52 trait lines and 153 adapter lines (205) against about 3,140
  moved lines: 6.5 %.
- **Test shims.** `handlers/player_test_shims.rs` (150 lines) is
  `#![cfg(test)]`, mounted from `handlers/mod.rs` outside the session and
  character hotspots, with a physical ceiling at its live size. It holds:
  - the regeneration, PvP-expiry, power-update and melee-damage method
    shims;
  - the two pet shims;
  - the six pet row-loader shims that left the session hotspot;
  - the loot-money tracker read that `LootCxRef` served.

  Call sites that only change path are converted:
  - the three `cx_pets(..).begin_…` test calls;
  - the f3 shims of the declined names and the resummon;
  - two source-scan scenarios, which now include the moved login loader file.

### Accounting

- **`crates/wow-world/src`.** Classifier: 75,503 → **72,465 production
  (−3,038)**, 78,957 → 76,108 physical, 504 files.
  - Combat −881.
  - The saga with `LootCxRef` −1,122.
  - Pets and the import tidy −888.
  - The test shim file −147. This is not a reduction of base production.
    The three figures above are net of the new
    `handlers/player_test_shims.rs`, which their commits created and the
    classifier first counted as production. Marking it `#![cfg(test)]` keeps
    those new test-only lines out of the production count. Base production
    code that left `wow-world` is the sum of the three family figures, 2,891.
    (Corrected in 7b, #1383 review follow-up 1.)
- **Destinations.**
  - `wow-world-entities/src` 13,793 → 14,828 (+1,035).
  - `wow-world-inventory/src` 13,668 → 14,942 (+1,274).
  - `wow-world-lifecycle/src` 11,407 → 12,423 (+1,016).
  - `wow-world-loot-ops/src` 13,063 → 13,138 (+75).
- **`wow-world-application`: 11,523 → 11,516 (−7)**, the offer record
  re-export.
- **`net_move.py check`.** PASS, 0 duplicate bodies. The figures are in the
  PR body.
- **Edges.** No `wow-world-*` edge is added or removed. The closure check
  passes in lock and metadata mode. `dependency-policy.json` allows `rand` and
  `tokio` for `wow-world-inventory`.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`. The
  delta is the moved impl items, the three new adapter trait impls, the test
  shim impls and the retired `LootCxRef` items.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `session/mod.rs`: production 42,601 → 41,225, test 142,042 →
    141,986, total 184,643 → 183,211;
  - hotspot `handlers/loot/mod.rs`: production 1,643 → 1,386, total 20,572 →
    20,315;
  - hotspot `handlers/character/mod.rs`: production 15,088 → 14,790, total
    30,796 → 30,498;
  - physical `session/mod.rs` 977 → 976, `handlers/loot/handlers.rs`
    171 → 13, `battle_pet_purchase.rs` 92 → 55;
  - new physical row `handlers/player_test_shims.rs` at 150.
- **Warnings.** The warning counts of the production lib, the lib with
  `test-fixtures` and the lib plus lib-test build equal base: 331, 297 and
  504. `wow-world` `dead_code` is 0 in all three, and every warning sits on an
  import that warned at base.
  - Two messages render differently. The split loot-persistence import now
    prints its path, and the pet-loading import names one item instead of two.
  - The destination crates keep exactly their pre-existing warnings.
  - The imports only the scenarios still name become `cfg(test)`: the pet row
    types, `UnitMoveTypeLikeCpp`, `ActiveState` and `PetType`.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed; the workspace
  test-name multiset is identical (10,214 names, 5 ignores). No assertion
  changes. The focused `wow-world` runs pass: 1,181 tests matching pet, battle
  pet, trainer, money, persistence, loot, regeneration, PvP, melee, combat,
  FFA, login, teleport and transfer.

### 7b plan

7b re-measures on its base. It moves:

- the battle-pet durable grant, level and uncage operations, the journal
  helpers and the battle-pet adapter, to inventory;
- the reputation, skill and talent bodies, to social and spell by owner;
- rest and xp, to lifecycle;
- the combat and pet leftovers.

Most of its lines are `&self` bodies with `cfg(test)` return branches. Each
needs a host fixture predicate, as in 5b, or stays as a test shim.
`CMSG_DISMISS_CRITTER` can go with the battle-pet handlers in inventory.

### Follow-ups

- `loot_test_shims.rs` is still counted as production by the classifier.
  Marking it `#![cfg(test)]`, as `player_test_shims.rs` is, moves 441 lines
  from the production figure to the test figure without changing code.
- The `PetsCx`/`PetsCxRef` battle-pet helpers become free functions over the
  pet host in 7b, which retires `PetsCx`.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 7b result (battle-pet operations → inventory; reputation gain → social; PetsCx retired)

Branch `1363-s7b-progression`, base `ff0c1ea8b` (7a merged). Move-only (R2),
with the 7a rewrites. A call to a retired `PetsCx`/`PetsCxRef` helper becomes
a free call that takes the session as its host.

The following are unchanged:

- the battle-pet add, experience and level arithmetic;
- the owner's lease and capacity rechecks;
- the Login-DB-first, `DestroyItem`-second uncage order and its receipt
  replay;
- the criteria recording;
- the `BattlePetUpdates`, `CastFailed`, battle-pet error and uncage-visual
  packets and their order;
- the reputation gain rates and Recruit-a-Friend bonus;
- #169 ordering, registrations and processing flags.

No registrar moves. `CMSG_DISMISS_CRITTER` keeps its `0x34F9`, `LoggedIn`,
`ThreadUnsafe` row and `handle_dismiss_critter` name.

### Re-measure and split

On `ff0c1ea8b` the 7b families held about 2.7k classifier lines. The battle
pets are about 1.4k, of which about 520 are `#[cfg(test)]` methods nested in
production impls. Progression is about 0.5k, rest 261, XP 120, and the
combat and pet leftovers about 0.2k. Every family together is well under
4.5k. The host budget is not:

- **Battle pets and reputation.** 1,219 moved lines need 173 new host lines
  (14.2 %).
- **Talents.** The talent bodies (about 100 lines) call nine World methods:
  the trait/override reset, the auxiliary-authority invalidation, the two
  talent side-effect methods, the points refresh, the at-login flag read and
  removal, the loaded check and `trinity_string`. That is about 65 host lines
  (65 %).
- **Rest.** The production lines are flag, played-time and rest-flag
  accessors with 2–5 World callers each. Moving them only moves the
  forwarding.
- **XP.** `xp_grants.rs` is the quest-reward context builder over seven
  disjoint fields, like the loot-release builder that 5c kept.

Adding talents, rest or XP would take the slice above 15 %, so 7b splits at
the family boundary. 7b is the battle pets, the reputation gain,
`CMSG_DISMISS_CRITTER` and the `PetsCx` retirement. The talent, skill, rest,
XP and combat/pet leftovers go to 7c.

### What moved

- **To `wow-world-inventory/src/battle_pet_session/`** (1,113 lines):
  - `durable.rs`: the durable `BattlePetMgr::AddPet`,
    `GrantBattlePetExperience` and `GrantBattlePetLevel` operations;
  - `uncage.rs`: the uncage effect (`Spell::EffectCreateBattlePet`), the
    `Player::DestroyItem` phase after the Login DB commit, and the two grant
    effects;
  - `journal.rs`: the helpers that `PetsCx`/`PetsCxRef` owned (owner lease,
    purchase store, `GetPetCount`, `HasMaxPetCount`, journal lease, species
    entry, XP per level, `GetMaxPetLevel`, trainer-purchase criteria), plus
    the stats and species-flag reads and the add-request receipt query;
  - `mod.rs`: the host trait, the outcome and XP-source types, the stats
    application and the two constants. `wow-world` re-exports them at their
    old paths.
- **`CMSG_DISMISS_CRITTER`.** The body joins `BattlePetHandlerCxLikeCpp`
  (+23). Its thunk builds the same context as the eight battle-pet handlers,
  and `BattlePetHandlerHostLikeCpp` loses its dismiss method.
- **To `wow-world-social/src/reputation_gain.rs`** (135 lines): the kill and
  spell reputation gain, the percent before the reward rate, the reward rate
  and the Recruit-a-Friend bonus.
- **The battle-pet purchase saga.** `BattlePetPurchaseHostLikeCpp` now
  extends `BattlePetSessionHostLikeCpp`. The saga calls the journal helpers
  as free functions, so the trait loses seven methods (`lifecycle`,
  `split_lifecycle_mut` and the five former `PetsCx` forwards).
- **`PetsCx`/`PetsCxRef` are retired** (#1383 follow-up), with their builders
  and re-exports.
  - Their three thin wrappers become `WorldSession` methods with the same
    bodies: the journal-lock send, `HasJournalLock` and the represented pet
    read. Login and the spell-effect checks still call them.
  - `PetSessionHostLikeCpp::split_pets_mut` returns `split_lifecycle_mut`,
    which borrows the same fields.
  - The test-only `PetsCx` impls (durable remove and rename, the represented
    delete/rename handlers, the purchase-port setter) become `WorldSession`
    impls in their `unit_tests` files.

### Fixture steps

- **`&mut self` bodies.** The former `#[cfg(test)]` criteria writes use the
  4b mechanism: `#[cfg(any(test, feature = "test-fixtures"))] if let Some(fixtures) = host.test_fixtures_like_cpp()`.
  The trainer-purchase criteria take `split_lifecycle_mut` inside that
  branch, as the `PetsCx` body did.
- **`&self` bodies.** The `#[cfg(test)]` return branches (stats, species flag
  and entry, XP per level, max level, max count) read
  `host.test_fixtures_ref_like_cpp()`. This new provided method on
  `SessionHubHostLikeCpp` (+7 core lines) is the `&self` form of
  `test_fixtures_like_cpp`.
  - It is declared only under `cfg(any(test, feature = "test-fixtures"))`
    and returns `None` by default. Only the World `#[cfg(test)]` override
    returns `Some`.
  - **Proof that the predicate is false in production.** In a production
    build the declaration and every branch that reads it are compiled out.
    In a `test-fixtures` build outside the World tests the default `None`
    applies, which is the former `#[cfg(not(test))]` result.
- **Represented fallbacks.** The no-attachment branches of the three durable
  operations used to call `#[cfg(test)]` represented bodies. They now call
  `battle_pet_{add_pet,grant_experience,grant_level}_fixture_like_cpp`.
  - These host hooks are declared only under the same cfg and return `None`
    by default, which keeps the former `#[cfg(not(test))]` result
    (`MissingAuthority`/`NoJournalLock`).
  - The World `#[cfg(test)]` overrides call the represented bodies.
- **`GetPetCount`** is test-only. It is a
  `cfg(any(test, feature = "test-fixtures"))` free function that takes the
  lifecycle state and the fixtures explicitly.

The represented `#[cfg(test)]` bodies moved to the `#![cfg(test)]`
`handlers/battle_pet_test_shims.rs` (603 lines). These were cage, add,
experience, quality, level, remove, rename, the owner-aura experience entry
and the represented cage handler.
- In their old files they were nested in production impls, so the
  classifier counted them as production. Moving them is a reclassification,
  not a move out of `wow-world`.
- Their bodies are verbatim, except for the free calls to the moved helpers
  and `self.` in place of the `PetsCx` fields.
- The file also holds one-call method entry points for the moved helpers
  that the scenarios call.

### Host surface

- **`BattlePetSessionHostLikeCpp: SessionHubHostLikeCpp`** (inventory,
  52 lines).
  - Production methods: `lifecycle`, `split_lifecycle_mut`,
    `split_inventory_ref`, the uncage item-state load and the
    expected-owner `DestroyItem`.
  - Fixture-only: the three hooks (about 30 lines).
  - Adapter `session/pets/host.rs`, +87 lines, of which 45 are
    `#[cfg(test)]` hook overrides.
- **`SocialSessionHostLikeCpp`** gains the misc-value aura modifier and the
  Recruit-a-Friend check (+7 trait lines). Adapter
  `session/chat_handler_contexts.rs`, +14 lines.
- **Recursion check.** Every adapter method names an inherent method by path
  (`WorldSession::name(self, …)`), a session split or a field. Each
  path-named method has an inherent `WorldSession` method of that name:
  - `uncage_item_state_like_cpp`;
  - `destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp`;
  - the three represented bodies;
  - `resolved_total_represented_aura_modifier_by_misc_value_like_cpp`;
  - `gets_recruit_a_friend_bonus_like_cpp`.

  None resolves to its own trait method, and no UFCS forward goes to a trait
  default. rustc reports no `unconditional_recursion`.
- **Borrows.**
  - In each `let … else` the scrutinee borrows the host only for the
    pattern, so the fixture hook in the `else` branch takes it mutably, as
    the World body took `&mut self`.
  - The journal-lease and receipt-query futures hold `&H` across their
    await, as the `PetsCxRef`/`QuestStateCxRef` bodies held their split.
  - No split is held across a call with different semantics.
- **Budget.** Against 1,219 moved lines:
  - new host lines are 173 (14.2 %);
  - 87 of them are production code (7.1 %);
  - the rest is fixture plumbing (hooks, overrides, `test_fixtures_ref_like_cpp`).

  Retired host lines: the saga trait −12, the saga adapter −35, the battle-pet
  handler trait −7 and its World adapter −7, and `PetsCx`/`PetsCxRef` with
  their builders −46 (−107 in all). The net host is +66 (5.4 %).

### Accounting

- **`crates/wow-world/src`.** Classifier: 72,465 → **70,440 production
  (−2,025)**. Physical: 76,108 → 75,136. Files: 504 → 505. Test: 3,643 →
  4,696.
  - **Battle pets −1,488.**
    - About 520 is the nested `#[cfg(test)]` bodies reclassified into
      `battle_pet_test_shims.rs`.
    - About 970 is production code that left `wow-world`, net of the World
      adapter and the three wrappers.
  - **Reputation −96.**
  - **`loot_test_shims.rs` −441.** This is a reclassification only: the file
    is now `#![cfg(test)]`. See "Follow-ups of #1383" below.
  - Production code that actually left `wow-world` is therefore about 1,065
    lines. The other about 960 lines are test code that the classifier no
    longer counts as production.
- **Destinations.**
  - `wow-world-inventory/src`: 14,942 → 16,091 physical (+1,149); 14,814 →
    15,901 production.
  - `wow-world-social/src`: 11,309 → 11,456 (+147).
  - `wow-world-core/src`: 47,361 → 47,368 (+7, the fixture predicate).
- **`wow-world-application`.** Unchanged at 11,516 (10,715 production).
- **`net_move.py check`.** PASS. S = 972 (physical: the 603-line test shim
  file stays in `wow-world/src`), G = 1,303 ≤ 1,981.6, 0 duplicate bodies.
- **Edges.** No `wow-world-*` edge is added or removed, and no `Cargo.toml`
  changes, so there is nothing to simulate with `--add`.
  `wow_world_closure.py check` passes in lock and metadata mode (17
  packages).
- **`session-ownership-policy.json`** is reprinted with `print-baseline`.
  The delta is:
  - the moved impl items;
  - the two adapter traits' new items and `test_fixtures_ref_like_cpp`;
  - the retired `PetsCx` items and the saga host methods;
  - the battle-pet test shim impls.

  The syntax-only check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `session/mod.rs`: production 41,225 → 39,736, test 141,986 →
    141,901, total 183,211 → 181,637;
  - hotspot `handlers/character/mod.rs`: production 14,790 → 14,788, total
    30,498 → 30,496;
  - physical `session/mod.rs` 976 → 973, `session/state.rs` 175 → 174,
    `handlers/character/world_entry.rs` 354 → 221;
  - new physical row `handlers/battle_pet_test_shims.rs` at 603.
- **Warnings.** The base warning counts of the production lib, the lib with
  `test-fixtures` and the lib plus lib-test build are 331, 297 and 504. The
  head counts are 330, 295 and 502.
  - Every head warning is a base warning. The removed ones are the base
    unused `RepresentedBattlePetQualityOutcomeLikeCpp` re-export and the
    unused `calculate_battle_pet_stats_like_cpp` import.
  - `wow-world` `dead_code` is 0 in all three builds.
  - The inventory and social lib-test builds show no warning in the new or
    changed files.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed. The workspace
  test-name multiset is identical (10,214 names, 5 ignores).
  - No assertion changes.
  - Two `assert_eq!` calls in `scenarios_pets_2.rs` were reflowed onto one
    line by rustfmt after their receiver shortened from
    `crate::session::cx_pets_ref(&session).` to `session.`.
  - The `cx_pets(..)` scenario call sites now call the `WorldSession` method,
    and the dismiss-critter scenarios dispatch through the registered thunk.
  - The focused `wow-world` runs pass: 255 tests matching pet, critter,
    uncage, trainer and journal; 418 matching battle, the spell-state 20–22
    groups, movement 8, world-entities 12, player-items 6 and login; and 156
    matching reputation, progression, kill, spell-state 11, RAF and recruit.

### Follow-ups of #1383

- **The "−147 test shim" wording** in the 7a accounting is corrected above.
  It names new test-only lines kept out of the production count, not a
  reduction.
- **`loot_test_shims.rs` is `#![cfg(test)]`.** It is mounted only under
  `#[cfg(test)]` in `handlers/mod.rs`. Its only reader is the `cfg(test)`
  glob import in `handlers/loot/mod.rs`. So the attribute changes no
  compiled code.
  - The classifier figure moves 441 lines from production to test.
  - Its physical row stays at 441: the header was condensed by two lines to
    make room for the attribute.
  - The loot hotspot is unaffected, because the file is outside its tree.
- **`send_player_power_update_like_cpp` visibility.** It stays
  `pub use`-exported from entities only under
  `cfg(any(test, feature = "test-fixtures"))`. Its only consumer outside
  `combat_session` is still the World `cfg(test)` shim, and the combat
  leftovers moved to 7c. That export is still the narrowest one that serves
  it. 7c revisits it if a leftover gains a production caller.

### 7c plan

- **Talents.** The reset, login reset, learn and notification bodies need
  the nine World methods listed above. Before moving them, 7c checks whether
  the talent side-effect pair and the trait/override reset can move with
  them to spell, which would leave the at-login flags and `trinity_string`
  as the only host methods.
- **Rest.** `current_played_time_values_like_cpp` reads only the lifecycle
  state and can become a `SessionLifecycleState` method. The flag accessors
  need the canonical player and the loaded-flags fixture. The `#[cfg(test)]`
  rest methods (about 170 lines) are test shims.
- **Skills.** `replace_player_skill_records_like_cpp` is a hub forward. The
  tombstone clear has a fixture branch.
- **XP and the combat/pet leftovers.**
  - The XP context builder stays, like the loot-release builder.
  - The resurrection-state snapshot needs the spell-state test accessor.
  - The setters stay as composition.
- **Battle-pet XP-percent aura** (`apply_represented_battle_pet_xp_pct_aura_like_cpp`).
  It is aura application. It stays in `session/battle_pet_adapter.rs` until a
  slice moves the visible-aura publication it calls.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 7c result (XP-percent aura → spell; played time → lifecycle; talents stay as a design step)

Branch `1363-s7c-talents`, base `93ed21e5e` (7b merged). Move-only (R2), with
the 7a rewrites:

- `self.` becomes the destination state's `self.` or `hub.`;
- a World wrapper over a `split_spell_state_{ref,mut}` becomes the same
  `SessionSpellState` call with the hub;
- `self.lifecycle.X()` becomes `self.X()` on `SessionLifecycleState`.

The XP-percent aura's slot, fields, insert-before-publish order and
`SMSG_AURA_UPDATE` are unchanged. So are the played-time arithmetic and
saturation, the talent learn and reset order, the rest and XP math, the
level-up order and resurrection. Packets and their order, #169 ordering,
registrations and processing flags are unchanged too. No registrar moves.

### Re-measure and the talent decision

On `93ed21e5e` the 7c families (talents, skills, reputation, rest, XP, the
battle-pet adapter, progression adapters, the combat and pet leftovers) held
1,340 classifier lines. 326 of them were `#[cfg(test)]` methods nested in
production impls. The production code that could move is small.

- **Talents (171 lines) do not move.** 7b asked whether the talent
  side-effect pair and the trait/override reset could move to spell with the
  talent bodies. They can't without moving more:
  - The pair calls `learn_known_spell_like_cpp`, `remove_known_spell_like_cpp`
    (the recursive `Player::RemoveSpell` closure in
    `session/spell_state/spellbook.rs`; the pair has 12 World call sites) and
    `remove_auras_with_interrupt_flags_like_cpp`. All three are World
    methods.
  - `refresh_represented_talent_points_like_cpp` builds the
    `wow_world_quest::QuestRewardCx`. Quest depends on spell, so spell cannot
    call it.
  - The trait/override reset and the auxiliary-authority invalidation could
    move: they only use `SessionSpellState` and core. But they need a new
    spell host for the split, and it saves nothing while the rest stays.

  A spell host for talents would need nine methods: the spell-state split,
  learn, remove, the aura interrupt removal, the points refresh, the at-login
  read and removal, the loaded check and `trinity_string`. That is about 65
  host lines for about 100 body lines, the surface 7b measured. Per the 7b
  recommendation talents stay. **Design step:** move them after
  `Player::LearnSpell`/`RemoveSpell` (the spellbook learn/remove closure) is
  in `wow-world-spell`, which is slice 9 (spell effects) material. The
  points refresh then stays a single host method, or moves to the
  quest-reward side that owns `QuestRewardCx`. The host is then the at-login
  flags, the loaded check, `trinity_string` and that refresh.
- **Skills.** `replace_player_skill_records_like_cpp` is a hub forward and
  `clear_player_skill_tombstones_like_cpp` is a canonical forward. Both have
  only World callers, so moving them only moves the forwarding. The setters
  are composition.
- **Rest.** The flag accessors are canonical reads with `#[cfg(test)]`
  fallbacks and 2–5 World callers each. They stay for the same reason.
- **XP.** The context builder stays, like the loot-release builder (5c).
- **Resurrection snapshot.** It is deferred. Its three production callers
  are in `wow-world` (spell effects, quest state and teleport). Moving it
  would need a `test_fixtures_ref_like_cpp` branch plus a spell-state test
  accessor on a host, only to forward back. It moves with the first of those
  callers.

### What moved

- **To `wow-world-spell/src/session/spell_state/battle_pet_aura.rs`** (77
  lines):
  - `apply_represented_battle_pet_xp_pct_aura_like_cpp`, now a
    `SessionSpellState` method over `&mut HubMut`. Its slot, insert and
    publication are the `SessionSpellState` methods the World wrappers
    called, and `self.core` becomes `hub.core`.
  - The pure `represented_aura_effect_amounts_like_cpp`.
    `session/aura_effect_values.rs` re-exports it for the mount and
    aura-modifier callers.
  - The spell-effect caller takes `split_spell_state_mut` in place of the
    method (+2 lines).
  - The 7b note said this aura would wait for "the visible-aura publication
    it calls". That publication is already on `SessionSpellState`, so nothing
    more had to move first.
- **To `wow-world-lifecycle/src/state/playtime.rs`** (+13):
  `current_played_time_values_like_cpp` is a `SessionLifecycleState` method
  beside the accessors it reads. The void-storage transfer and the
  persistence fixture call it through `self.lifecycle`.
- **Reclassified, not moved.** These `#[cfg(test)]` methods were nested in
  production impls:
  - `session/rest_progression.rs`: rest state, XP rest bonus, the online
    update and tick, tavern revalidation and the create flags;
  - `session/progression_adapters.rs`: kill credit, XP table and money;
  - `session/progression/{skills,reputation}.rs`;
  - `session/pets/pet.rs`: the petition reads and the tapper-pet kill hook;
  - `session/combat/state.rs`.

  They moved verbatim to the `#![cfg(test)]`
  `handlers/progression_test_shims.rs` (315 lines). That file also holds the
  XP-aura method entry point for the scenario.
  - **Visibility.** The moved methods that were `pub(in crate::session)` are
    now `pub(crate)`. So are the five production helpers they call:
    - `with_owned_player_mut_for_rest_like_cpp`;
    - `represented_recruit_a_friend_xp_rest_state_applies_like_cpp`;
    - the online XP rest-bonus update and tick with policy;
    - the tavern revalidation with catalog.

    Two test-only helpers widen the same way:
    `fixture_set_xp_rest_bonus_like_cpp` and
    `replace_player_skill_runtime_exact_like_cpp`. This is the slice 1
    precedent: a sibling test is a concrete consumer. No item leaves the
    crate.
- **Follow-ups closed:**
  - #1384 follow-up 1: the `split_pets_mut` doc comment names
    `split_lifecycle_mut`.
  - #1383 follow-up 2: the two over-long loot-ops receiver calls in
    `session_commands.rs` wrap their arguments.

### Host surface

- **No new host trait or adapter line.** The aura uses the existing
  `split_spell_state_mut` state/hub pattern of `wow-world-spell`, and the
  played-time values use only lifecycle state. Host share: 0 of 90 moved
  lines (0 %).
- **Recursion and borrows.** No trait method is added, so nothing can
  recurse. The spell-effect caller holds the split only for the synchronous
  call, as each of the three World wrappers did. No `.await` is crossed.
- **Fixture steps.** None. Neither moved body had a `cfg(test)` branch.

### Accounting

- **`crates/wow-world/src`.** Classifier: 70,440 → **70,077 production
  (−363)**. Physical: 75,136 → 75,090 (−46). Files: 505 → 506. Test: 4,696 →
  5,013 (+317).
  - **About 71 is production code that left `wow-world`:**
    - the aura method and its imports (−44);
    - the amounts function (−13);
    - the played-time values (−16);
    - the caller's split (+2).
  - **About 295 is test code reclassified:**
    - rest −144 (with the blank lines between the methods);
    - progression adapters −53;
    - skills −25;
    - reputation −18;
    - pets −48;
    - combat state −7.

    These were already test-only bodies. The classifier now counts them as
    test instead of production.
  - **+3 is other:**
    - the `session_commands.rs` wraps (+4);
    - one rustfmt fold after a widening (−2);
    - the dropped rest import (−1);
    - the rest doc note (+2).
- **Destinations.**
  - `wow-world-spell/src`: 8,964 → 9,043 physical (+79; production 8,700 →
    8,779).
  - `wow-world-lifecycle/src`: 12,423 → 12,436 (+13).
- **`net_move.py check`.** PASS. S = 46, G = 92 ≤ 1,009.3, 0 duplicate
  bodies.
  - S is small because the reclassified bodies stay in `wow-world/src` as
    the test shim file.
- **Edges.** No `Cargo.toml` changes and no new `wow-world-*` edge, so there
  is nothing to simulate with `--add`. `wow_world_closure.py check` passes in
  lock and metadata mode (17 packages).
- **`session-ownership-policy.json`** is reprinted with `print-baseline`. The
  delta:
  - 21 test entry points, now in `crate::handlers::progression_test_shims`:
    the 20 reclassified methods and the XP-aura entry point;
  - `current_played_time_values_like_cpp` and the production XP-aura method,
    which leave the `WorldSession` impls;
  - the seven visibility widenings.

  The syntax-only check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `session/mod.rs`: production 39,736 → 39,369, test 141,901
    unchanged, total 181,637 → 181,270;
  - new physical row `handlers/progression_test_shims.rs` at 315.
- **Warnings.** The `wow-world` warning counts (`--message-format=short`
  lines under `crates/wow-world/`) are lower than base in every build:
  - production lib: 153 → 151;
  - lib with `test-fixtures`: 161 → 159;
  - lib plus lib-test: 297 → 296.

  The removed warnings are base unused imports: the rest
  `Arc`/`AreaTriggerDb2Store`/`PLAYER_FLAGS_RESTING_LIKE_CPP` import, and the
  progression-adapters `catalogs` import, now `cfg(test)`. Every head warning
  is a base warning.
  - `wow-world` `dead_code` is 0 in all three builds.
  - `wow-world-spell` and `wow-world-lifecycle` keep exactly their base
    warnings.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed. The workspace
  test-name multiset is identical (10,214 names, 5 ignores), and no
  assertion changes.
  - One fixture line changes its path to `self.lifecycle`.
  - Focused `wow-world` runs pass:
    - 157 tests matching xp_aura, battle_pet, played_time and void_storage;
    - 473 matching rest, xp, tavern, skill, reputation, petition, tapper,
      combat_stat, kill_credit, gold, progression, talent and login.
- **`send_player_power_update_like_cpp`** (#1383 follow-up 3). No combat
  leftover moved or gained a production caller. Its
  `cfg(any(test, feature = "test-fixtures"))` export from entities stays the
  narrowest one that serves the World shim.

### Deferred, with reasons

- **Talents.** Nine host methods for about 100 body lines. This is the
  design step above.
- **The resurrection snapshot, the skill and rest forwards and the flag
  accessors.** Every caller is in `wow-world`, so moving them only moves the
  forwarding.
- **The XP context builder and the composition setters.** They stay by
  design, like the loot builders and setters.
- **The `#[cfg(test)]` statement branches inside production bodies.** These
  are the death snapshot, the skill tombstones, the flag accessors and the
  at-login flag removal. They need a fixture predicate each when their body
  moves, not before.
- **The battle-pet adapter's `#[cfg(test)]` hook overrides**
  (`session/pets/host.rs`, 45 lines). They belong to the
  `BattlePetSessionHostLikeCpp` impl block and stay with it.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 7 summary (7a–7c)

- **`wow-world` production (classifier).** 75,503 → 70,077 (−5,426).
  - 7a −3,038: 2,891 base production code left `wow-world`, and 147 is its
    test shim kept out of the count.
  - 7b −2,025: about 1,065 moved and about 960 reclassified test code.
  - 7c −363: about 71 moved, about 295 reclassified test code and +3 other.
  - In all, about 4,027 lines of production code left `wow-world`, and about
    1,402 lines of test code no longer count as production.
- **Moved out of `wow-world`:**
  - combat, to entities;
  - the creature loot-release VALUES receiver, to loot-ops;
  - the #161 purchase saga and the durable battle-pet operations, uncage
    effect, journal helpers and `CMSG_DISMISS_CRITTER`, to inventory;
  - pet loading, mode state and resummon, to lifecycle;
  - the reputation gain, to social;
  - the battle-pet XP-percent aura, to spell;
  - the played-time values, to lifecycle.
- **Retired:** `LootCxRef` (7a) and `PetsCx`/`PetsCxRef` (7b).
- **Host.**
  - 7a: 205 new host lines against about 3,140 moved (6.5 %).
  - 7b: 173 against 1,219 (14.2 %), or 5.4 % net of the 107 retired host
    lines.
  - 7c: 0 against 90.
- **Ceilings.** None raised in any part. Four test-shim files are now
  `#![cfg(test)]` and ratcheted:
  - `player_test_shims.rs` (150);
  - `battle_pet_test_shims.rs` (603);
  - `loot_test_shims.rs` (441);
  - `progression_test_shims.rs` (315).
- **Still in `wow-world` from the slice 7 families**, each with its reason
  above:
  - the composition setters and the XP context builder (by design);
  - the talent bodies (a design step after the spellbook learn/remove
    closure moves);
  - the skill, rest-flag and resurrection-snapshot forwards (World callers
    only);
  - the battle-pet adapter's test hook overrides and the represented
    battle-pet fixture bodies (test code).
- **Next.**
  - Slice 6 (quest) needs the `wow-world-quest` split first.
  - Slice 8 (character) is the next large production move with no
    prerequisite.
  - Slice 9 (spell effects) should take the spellbook learn/remove closure
    first, which unblocks the talents.

## Slice 8 plan (character, phase 2)

Measured on `518d8bd66` (7c merged) before anything moved, with the slice 5
and 7 tools: the hotspot classifier for production lines, and a lexical scan
of each family's files. The scan blanks comments, literals and every item or
statement behind `#[cfg(test)]`/`#[cfg(any(test, feature = "test-fixtures"))]`
(nested ones too), then counts the `self.`/`session.` calls to inherent
`WorldSession` methods defined outside the family. That count is the host
surface if only the family moves.

- **`wow-world` production (classifier): 70,077** lines, 75,090 physical,
  506 files.
- **The character family is `handlers/character/**`: 14,788 production
  lines.** The original row estimated about 12.5k.

| Family | Files | Production | Outside calls | Destination |
|---|---|---:|---:|---|
| Visibility (nearby creature/gameobject create, visibility update, spawn-row materialization) | `visibility.rs`, `visibility/{creatures,gameobjects}.rs` + about 90 lines of `mod.rs` helpers | 1,569 | 21 | entities |
| Vendor and bank (list, buy, sell, buyback, repair, extended cost, bank slots, autobank) | `vendor.rs`, `vendor/*`, `vendor_admission.rs`, `bank.rs` | 3,073 | 70 | inventory |
| Item handlers and inventory moves (swap/split/destroy, real swap, item mutations, equipment sets) | `items.rs`, `items/{handlers,destruction,equipment_sets}.rs`, `items/inventory_moves*` | 2,480 | 45 | inventory (the `inventory_swap` users stay behind the host) |
| Gossip (hello, menu build, select, binder) | `gossip.rs` | 747 | 22 (5 into vendor, bank, trainer) | character |
| Character lifecycle (realm list, rename, hearth, entry zone) | `lifecycle.rs`, `entry_zone.rs` | 292 | 7 | character |
| Login load (world entry, initial packets, login loaders, login data, item login load, recovery, transport restore) | `world_entry.rs`, `world_entry/**`, `session_state*`, `items/login_load.rs` | 4,739 | 152 | character |
| Quest-giver status queries | `query.rs` (+ `query/test_shims.rs`, 77, `cfg(test)`-mounted) | 286 | 4 | stays: it names quest types, and character may not reach quest |
| Adapters, shared helpers and composition (`account/**`, `character_handler_host.rs`, `mod.rs`, stats, the support stubs) | | 1,602 | — | stays (F5 adapters and registrar composition) |

Findings:

- **Visibility has the lowest surface per line.** Of its 21 outside calls,
  `send_packet` and `can_see_phase_shift_like_cpp` are already on the hub and
  entities host traits. `realm_id` and `player_registry` are one-line
  `self.core.X()` forwards (the 7a rewrite), and four more (aggro radius,
  creature stats, the two gameobject records) are one-statement
  `split_world_entities_*` forwards that inline as the same state call with
  the hub (the 7c rewrite). That leaves about 13 host operations plus the
  lifecycle and visibility accessors and the quest-state visibility refresh
  flag.
- **Visibility goes to entities, not to `wow-world-visibility`.** The bodies
  read and write `WorldEntitiesState` (the materialized creature
  registration, the gameobject overrides and records), which lives in
  entities, and `wow-world-visibility` sits below lifecycle and entities.
  Entities gains a direct edge to `wow-world-visibility`
  (`VisibilityState`); it already reaches it through lifecycle.
- **Gossip does not go to interaction.** It calls
  `represented_gameobject_gossip_can_interact_with_like_cpp` in entities,
  and entities depends on interaction. It goes to character after vendor and
  bank. Of its calls into vendor, bank and trainer, only the vendor list
  (`handle_list_inventory`, two call sites) now lives in a crate character
  can reach (inventory, 8b). The show-bank publication and the trainer list
  (`handle_trainer_list`, `handle_trainer_list_for_gossip_option_like_cpp`)
  stay in `wow-world`, so those calls stay host methods in 8c. (Corrected in
  8b, #1386 review follow-up 1.)
- **Vendor and bank are the largest host surface** (70): money persistence
  and its exclusive transaction, the quest-objective drain, the item store
  and publication operations, currencies and buyback. 8b re-measures which
  of these are already inventory free functions; if the share stays above
  15 % it splits buy/sell/buyback from repair and bank.
- **Login load is #169-sensitive** (login/logout ordering, the persistence
  fences of the loaders) and has the largest surface (152). It goes last.

**Closure pre-check.** Every edge the plan may need was simulated on the
base with `tools/architecture/wow_world_closure.py check --add …`, in lock
and metadata mode, before anything moved: both PASS (17 packages) with
`wow-world-entities->wow-world-visibility`,
`wow-world-inventory->wow-world-entities` and
`wow-world-character->{interaction,visibility,spell,social,instances,loot}`
added together. Two negative probes fail as they must:
`wow-world-character->wow-world-quest` (quest is reached only by
application, `wow-world` and loot-ops) and
`wow-world-entities->wow-world-character` (a cycle). Each part re-runs the
check for the edges it actually adds.

**Split (each part re-measures on its own base):**

- **8a, visibility → entities (about 1.7k):** the nearby creature and
  gameobject create paths, the visibility update and the spawn-row
  materialization with its pure helpers. The single new edge is
  entities → visibility; entities also needs the external `tokio` (the
  spawn-catalog query timeout).
- **8b, vendor and bank → inventory (about 3.1k, surface 70):** the
  vendor list/buy/sell/buyback/repair paths and the bank slot and autobank
  handlers. Money, item-store and buyback order are #169/persistence
  sensitive and move verbatim.
- **8c, item handlers and inventory moves → inventory; gossip and the
  character lifecycle → character (about 3.5k, surface about 70):** the
  `inventory_swap` users (`wow-world-application`) stay behind host methods.
- **8d, login load → character (about 4.7k, surface 152):** world entry,
  initial packets, the login loaders, login data, the item login load,
  recovery and the transport restore. Its order is the #169 contract; if the
  8d re-measure exceeds about 4.5k, `session_state/login_data.rs` splits off
  into an 8e.
- **Stays:** the quest-giver status queries (quest types), the F5 adapters,
  the registrar composition and the shared `mod.rs` constants.

## Slice 8a result (visibility → entities)

Branch `1363-s8a-character`, base `518d8bd66` (7c merged). Move-only (R2),
with the 4b/5a/7a/7c rewrites:

- `self.` becomes `host.`, and a session field becomes a host accessor
  (`lifecycle`, `visibility`/`visibility_mut`, the entities
  `world_entities`/`world_entities_mut`);
- `self.catalogs` becomes `host.hub_ref().catalogs`, and the one-line
  World forwards `self.realm_id()`, `self.player_registry()` (both
  `self.core.X()`) and `self.player_map_visibility_range_like_cpp()`
  (`self.config.X()`) become `host.core().X()` / `host.hub_ref().config.X()`;
- a World wrapper that is one `split_world_entities_{ref,mut}` plus one
  `WorldEntitiesState` call (creature aggro radius, creature create stats,
  the gameobject DB phase shift and runtime state records) becomes that same
  split and call through the host, as in 7c. Each split is taken for exactly
  that call, as the wrapper took it; no borrow spans another call or an
  `.await`;
- a call between moved functions becomes a free call.

A token comparison of the seven moved functions against the base finds no
other difference (plus rustfmt trailing commas). The create and out-of-range
blocks and their order, the `m_clientGUIDs` publication transitions, the
spawn-row reads and the SQL fallback timeouts, the gameobject record order,
the fishing-hole and equipment RNG draws, the tracing events, #169 ordering,
registrations and processing flags are unchanged. No registrar moves. No body
had a `cfg(test)` branch, so no fixture predicate is needed.

### What moved

- **To `wow-world-entities/src/visibility_session/`:**
  - `nearby.rs`: `send_nearby_creatures_with_catalogs_like_cpp`,
    `send_nearby_gameobjects` and the viewer-dependent creature create block
    (from `handlers/character/visibility/{creatures,gameobjects}.rs`);
  - `update.rs`: `update_visibility_with_catalogs_like_cpp` (from
    `visibility/creatures.rs`);
  - `spawn.rs`: `materialize_creature_spawn_row_with_catalogs_like_cpp` and
    `creature_addon_create_fields_like_cpp` (from `visibility.rs`), the
    single-caller `creature_virtual_items_from_row_with_catalogs_like_cpp`
    (from `items.rs`), and from `mod.rs` the spawn-row helpers (movement
    generator type, wander distance, template speeds, spawn difficulties,
    creature flags, 2D range, gameobject state, movement flags, hover
    offset), the addon and equipment create-field types and the two
    constants;
  - `mod.rs`: the host trait and the exports.
- The nine helpers the creature scenarios call are exported only under
  `cfg(any(test, feature = "test-fixtures"))`; `handlers/character/mod.rs`
  imports them `cfg(test)`, with the five imports only those scenarios still
  read.
- **The World callers** call the free functions: the driver's pending spawn,
  the far-teleport refresh, the forced update and the movement host's
  `update_visibility_with_catalogs_like_cpp` forward. The driver imports the
  entities functions it calls, which shortens the PvP-flag call by the lines
  the import costs (the session hotspot does not grow).
- **Reclassified, not moved.** The four split wrappers above now have only
  scenario callers. They moved verbatim from
  `session/world_entities/{aggro,creature_query,gameobject_overrides,gameobject_state}.rs`
  to the `#![cfg(test)]` `handlers/visibility_test_shims.rs` (122 lines),
  with the three method entry points for the moved bodies
  (`send_nearby_creatures`, `update_visibility`, `send_nearby_gameobjects`).
  The `gameobject_overrides` f3 shim reaches its names through `super::super`,
  because its parent is now only the mount.
- **Retired:** the map visibility range wrapper and the
  `CreatureCreateStatsLikeCpp` re-export (no other reader).

### Host surface

- **`VisibilitySessionHostLikeCpp: WorldEntitiesHostLikeCpp`** (entities, 15
  methods, 62 lines): the lifecycle and visibility accessors (3), the quest
  state's movement visibility refresh request, the viewer-dependent creature
  NPC flags and gameobject dynamic flags, the far-sight sync, the visibility
  source position, the world-creature query and the six canonical
  visible-object queries (gameobjects, dynamic objects, area triggers, the
  corpse/scene/conversation triple, transports, other players).
- **Adapter** `handlers/character/visibility.rs` (144 lines, replacing the
  292-line body file). Every method is a field access or names the inherent
  method by path (`WorldSession::name(self, …)`); each such method exists as
  an inherent `WorldSession` method, so none resolves to its own trait
  method. No forward goes to a trait default.
- The movement host's `update_visibility_with_catalogs_like_cpp` forward now
  names the free function `wow_world_entities::update_visibility_with_catalogs_like_cpp`,
  which is not a trait method, so it cannot recurse.
- **Budget.** 206 host lines (62 trait + 144 adapter) against about 1,775
  moved production lines: 11.6 %. With the whole
  `visibility_session/mod.rs` (106 lines: docs, imports, exports and the
  trait) it is 250 lines, 14.1 %.

### Accounting

- **`crates/wow-world/src`.** Classifier: 70,077 → **68,349 production
  (−1,728)**. Physical: 75,090 → 73,484 (−1,606). Files: 506 → 505. Test:
  5,013 → 5,135 (+122).
  - **About 1,775 is production code that left `wow-world`:**
    `visibility.rs` 292, `visibility/creatures.rs` 1,018,
    `visibility/gameobjects.rs` 259, `items.rs` 61, and 145 of the 162
    `mod.rs` lines.
  - **+144 is the new adapter.**
  - **About 92 is test code reclassified:** the four wrappers (75 lines with
    their impl shells) and 17 `mod.rs` import lines that are now `cfg(test)`.
  - **−5 is retired:** the map range wrapper (3), the re-export (2).
- **Destinations.** `wow-world-entities/src`: 14,828 → 16,841 physical
  (+2,013); production 14,663 → 16,658 (+1,995; the 18 test lines are the
  `cfg`-gated exports).
- **`wow-world-application`.** Unchanged.
- **`net_move.py check`.** PASS. S = 1,606, G = 2,013 ≤ 2,647.3, 0 duplicate
  bodies.
- **Edges.** Entities gains `wow-world-visibility` (`VisibilityState`) and
  the external `tokio` (the spawn-catalog query timeout);
  `dependency-policy.json` allows both. The edge was simulated before the
  move (*Slice 8 plan*) and `wow_world_closure.py check` passes on the head
  in lock and metadata mode (17 packages); character is still reached only
  by application and `wow-world`.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`.
  The delta is the moved impl items, the adapter's 15 trait items, the test
  shim items (three entry points and the four wrappers), the retired
  wrapper, and the `player_registry` direct-registry row, which moved to
  package `wow-world-entities` (`crate::visibility_session::update`). The
  syntax-only check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `handlers/character/mod.rs`: production 14,788 → 13,140, test
    15,708 → 15,706, total 30,496 → 28,846;
  - hotspot `session/mod.rs`: production 39,369 → 39,289, test 141,901
    unchanged, total 181,270 → 181,190;
  - physical `handlers/character/mod.rs` 670 → 522,
    `handlers/character/items.rs` 142 → 81, `session/mod.rs` 973 → 972;
  - new physical row `handlers/visibility_test_shims.rs` at 122.
- **Warnings.** The `wow-world` warning sets (`--message-format=short`
  lines under `crates/wow-world/`, compared without line numbers) equal base
  in all three builds: production lib 151, lib with `test-fixtures` 159, lib
  plus lib-test 296. `wow-world` `dead_code` is 0 in all three.
  `wow-world-entities` keeps exactly its base warnings (9, 5, 5), and the new
  module has none in the entities all-targets build.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed; the workspace
  test-name multiset is identical (10,214 names, 5 ignores), and no assertion
  changes. The only test-file edit is the f3 shim import path. A focused
  `wow-world` run on the first move commit passed 612 tests matching
  visibility, nearby, creature and gameobject.

### 8b plan

8b re-measures on its base. Vendor and bank (`vendor.rs`, `vendor/*`,
`vendor_admission.rs`, `bank.rs`; 3,073 classifier lines, 70 outside calls on
the 8a head) go to `wow-world-inventory`, beside the A4 vendor and bank
handlers.

- Many outside calls are inventory and money operations whose bodies are
  already `InventoryState`/core methods behind one-line World wrappers
  (`split_inventory_*`). 8b inlines those as the same split, which needs
  `split_inventory_{ref,mut}` on the host, and then counts what remains.
- The money path keeps its exclusive persistence boundary
  (`begin_exclusive_player_money_persistence_like_cpp`, the awaited outcome)
  and the quest-objective drain as host methods. Their futures are returned
  unchanged, as in 7a.
- If the host share stays above 15 %, repair and bank split off into 8c.
- The planned inventory → entities edge is simulated again on the 8b base.

### Follow-ups

- `handlers/character/query/test_shims.rs` (77 lines) is mounted only under
  `cfg(test)` but is not `#![cfg(test)]`, so the classifier counts it as
  production. Marking it moves 77 lines to the test figure without code
  change (as `loot_test_shims.rs` in 7b).
- The moved log sites now use the `wow_world_entities::visibility_session`
  tracing target (#1384 follow-up 2 applies here too).
- `visibility_test_shims.rs` retires when the scenarios call the free
  functions and the `WorldEntitiesState` methods directly.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 8b result (vendor and bank → inventory)

Branch `1363-s8b-vendor-bank`, base `c5939436a` (8a merged). Move-only
(R2), with the 4b/5a/7a/7c/8a rewrites:

- `self.` becomes `host.`, and a session field becomes a host accessor
  (`lifecycle`, `interaction`/`interaction_mut`; `self.loot.is_active_loot_guid`
  is the host method `is_active_loot_guid`);
- `self.catalogs`/`self.config` become `host.hub_ref().catalogs`/`.config`,
  `crate::session::hub_ref(self)` becomes `host.hub_ref()`, and the one-line
  World forwards to the catalogs (`condition_store`, `player_condition_store`,
  `currency_types_store`, `item_extended_cost_store`, `item_stats_store`,
  `item_store`, `item_storage_template`, `item_template_max_durability`,
  `item_template_inventory_type`, `repair_cost_rate_like_cpp`) or to the core
  (`kick`, `send_packet_realm`, `send_buy_error`, `send_sell_error`,
  `send_equip_error`) become that same catalog/config/core call;
- a World wrapper that is one `split_inventory_{ref,mut}` plus one
  `InventoryState` call becomes that same split and call through the host
  (`split_inventory_mut` is new on the host; `split_inventory_ref` is the
  battle-pet session host's). Each split is taken for exactly that call, as
  the wrapper took it; no borrow spans another call or an `.await`. The one
  exception is `insert_inventory_item_like_cpp`: the refund passes
  `self.item_template_inventory_type(..)` as an argument, which two-phase
  borrowing allowed only against the method call, so it stays a host method
  at all three call sites;
- a call between moved functions becomes a free call.

A token comparison of the ten moved bodies against the base finds no other
difference (plus rustfmt trailing commas). The four pure helpers are
token-identical. Two statements are not plain renames:

- the bank-slot purchase's `quest_state` enqueue of
  `RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged { old_money,
  new_money }` is the host method
  `enqueue_represented_quest_objective_money_changed_like_cpp(old_money,
  new_money)`, because inventory cannot name the quest event type;
- the two `#[cfg(test)]` steps (the vendor-slot buy override and the
  bank-slot test commit result) use the host fixture accessor:
  `#[cfg(any(test, feature = "test-fixtures"))] if host.test_fixtures_ref_like_cpp().is_some() …`.
  They read the same `InteractionState` and `SessionLifecycleState` fixture
  fields the retired World `cfg(test)` readers read.

The buy/sell/buyback/refund/repair money order, the exclusive money
persistence boundary and its awaited outcome, the item GUID allocation, the
store plans and stack splitting, the buyback slot rotation, the bank slot
price and count publication, the refund currency rollback, the quest
objective drain, the instance/realm socket fences, the packets and their
order, persistence ordering (#169), registrations and processing flags are
unchanged. No registrar moves: the application character/account registrar
keeps every entry, and its World adapter (`account/host.rs`) calls the moved
free functions.

### What moved

- **To `wow-world-inventory/src/vendor_session/`:**
  - `list_inventory.rs`: `handle_list_inventory`;
  - `buy.rs`: `handle_buy_item_with_generator_like_cpp`;
  - `catalog.rs`: the C++-slot catalog resolver
    `resolve_vendor_buy_item_by_cpp_slot` (from `vendor.rs`), the stock update
    `update_vendor_item_current_count` (from `vendor_admission.rs`) and the
    single-caller `item_turnin_persistence_rows_like_cpp` (from `items.rs`);
  - `sell.rs`, `buyback.rs`: the sale and buyback handlers;
  - `refund.rs`, `repair.rs`: the purchase refund and the repair handler
    (from `vendor.rs`);
  - `bank.rs`: `handle_banker_activate` and
    `handle_buy_bank_slot_with_prices_and_generator_like_cpp` (from `bank.rs`);
  - `mod.rs`: the host traits, the exports, and from the character `mod.rs`
    the money gain cap and the currently-looted and non-empty-bag guards, now
    private to the module.
- **Down from `wow-world-application`:** two pure helpers the bodies call,
  `item_purchase_contents_from_extended_cost` (to `wow_world_inventory::vendor`)
  and `item_spell_charges_db_string` (to inventory `persistence.rs`). The
  application keeps both paths as re-exports.
- **Tests that followed their helpers.** The three pure-helper tests
  (`player_money_gain_like_cpp_enforces_max_money_amount`,
  `item_currently_looted_guard_uses_runtime_loot_generated_state`,
  `sell_non_empty_bag_guard_matches_cpp_is_not_empty_bag`) moved verbatim to
  `wow-world-inventory/unit_tests/vendor_session_helpers.rs`.
- **Stays in `wow-world`:** the autobank and autostore moves (they run the
  application storage move, which inventory cannot reach), the show-bank
  publication (gossip calls it) and the trainer gossip menu in `vendor.rs`
  (gossip and the quest queries call it).
- **Reclassified, not moved.** Seven one-statement wrappers whose only
  production callers were the moved bodies moved verbatim to the
  `#![cfg(test)]` `handlers/vendor_test_shims.rs` (137 lines): the two
  inventory count/plan wrappers from `items.rs`, the two buyback wrappers from
  `session/buyback_adapter.rs`, and the bank-slot count, item-object slot and
  sell-error wrappers. The file also holds the method entry points the
  scenarios call (`handle_buy_item`, `handle_repair_item` and
  `handle_buy_bank_slot`, moved verbatim from the `cfg(test)` impls, plus
  `handle_banker_activate` and `update_vendor_item_current_count`).
- **Retired:** five wrappers with no other caller
  (`resolved_current_buyback_slot_like_cpp`, `item_extended_cost_store`,
  `item_store`, `repair_cost_rate_like_cpp`, `remove_buyback_item_like_cpp`),
  which empties and deletes `session/buyback_adapter.rs`; the `cfg(test)`
  readers `vendor_buy_item_test_override_like_cpp` and
  `loot_money_persistence_test_result_for_worker_like_cpp` (its
  `unit_tests/.../operations/test_only.rs` is deleted); the unused
  `player_team_for_race_cpp` re-export in `session/state/hub_support.rs`.
- **#1386 follow-up 4.** `handlers/character/query/test_shims.rs` is mounted
  only under `cfg(test)` (verified: its single mount in `query.rs` is
  `#[cfg(test)] mod test_shims;`). It moved verbatim outside the character
  hotspot tree to `handlers/character_query_test_shims.rs`, mounted from
  `handlers/mod.rs`, and is `#![cfg(test)]`. Marked in place, its 77 lines
  would have left the hotspot's production figure but raised its test
  figure above the ceiling; outside the tree they leave production and the
  file gets its own physical ceiling.

### Host surface

- **`VendorSessionHostLikeCpp: BattlePetPurchaseHostLikeCpp`** (inventory, 27
  methods and one associated type, 105 lines). It reuses the battle-pet
  purchase and session hosts for the lifecycle accessor, `split_inventory_ref`,
  the money read, staging and VALUES publication, the quest-objective drain
  and the exclusive money boundary; their futures are returned unchanged.
  New:
  - the `interaction` accessors, `split_inventory_mut`, the active-loot query
    and the quest-objective money-changed enqueue;
  - the player-condition context (`type PlayerConditionContext`, viewed
    through `PlayerConditionContextSourceLikeCpp`, 6 lines, because the World
    projection type wraps a quest type) and the creature condition object;
  - currencies (`has_currency`, `player_currency_quantity`, the refund and
    vendor currency plans) and the store plans;
  - item creation and publication (`make_inventory_item_object`,
    `insert_inventory_item_like_cpp`, the collection hook, the turn-in
    application, `send_new_item_plan`, `send_stat_update`, the registry sync);
  - feign death, the show-bank publication, the quest-log item lookup, the
    awaited money transaction outcome and the three repair operations.
- **Adapter** `handlers/character/vendor.rs` (207 lines of impls and 13 of
  imports, beside the trainer gossip menu). Every method is a field access or
  names the inherent method by path (`WorldSession::name(self, …)`); each such
  method exists as an inherent `WorldSession` method, so none resolves to its
  own trait method, and rustc reports no `unconditional_recursion`. No
  forward goes to a trait default. The futures (the money outcome, the
  quest-log lookup and the three repairs) are the inherent futures returned
  unchanged.
- **Widening.** `RepresentedPlayerConditionContextLikeCpp` becomes `pub`
  (from `pub(crate)`) because it is the adapter's associated type; it stays in
  the private `session::player_condition_values` module and is re-exported
  `pub(crate)`. The inventory exports are the two traits and the eight
  handlers (consumers: the account adapter, gossip and the test shims), plus
  `update_vendor_item_current_count` under
  `cfg(any(test, feature = "test-fixtures"))`.
- **Budget.** 331 host lines (111 trait + 220 adapter) against 2,905 moved
  production lines: 11.4 %. With the whole `vendor_session/mod.rs` apart
  from the moved helpers and the test mount (175 lines: docs, imports, mounts,
  exports and the traits) it is 395 lines, 13.6 %. Below 15 %, so repair and
  bank stay in 8b; no 8c split of them is needed.

### Accounting

- **`crates/wow-world/src`.** Classifier: 68,349 → **65,470 production
  (−2,879)**. Physical: 73,484 → 70,833 (−2,651). Files: 505 → 500. Test:
  5,135 → 5,363 (+228).
  - **2,905 is production code that left `wow-world`** for inventory: the
    ten bodies and four helpers (2,819 lines of item text) plus the headers,
    imports and impl shells of the deleted files.
  - **+220 is the new adapter.**
  - **146 is reclassified to test:** `query/test_shims.rs` 77, the seven
    wrappers 41 and the three `cfg(test)` method entry points 28.
  - **35 is retired:** the five wrappers and the `buyback_adapter.rs` shell
    (29) and the `cfg(test)` override reader (6).
  - **13 is import and mount edits** (the character `mod.rs` imports that
    only the scenarios read are `cfg(test)`, the moved mounts).
  - The test figure grows by the two `#![cfg(test)]` shim files (137 + 84)
    and the `cfg(test)` imports and mounts (+12), less the retired mounts (−5).
- **Destinations.** `wow-world-inventory/src`: 16,091 → 19,596 physical
  (+3,505); production 15,901 → 19,399 (+3,498; `vendor_session/` is 3,433
  lines, the moved application helpers 61). Its `unit_tests` gains the
  46-line helper test file. The inlined splits make the moved bodies longer
  than the World bodies (each inline is a three- or four-line block).
- **`wow-world-application`.** 11,516 → 11,459 physical (−57): the two
  helpers became re-exports.
- **`buy.rs` is 1,075 lines**, above the 1,000-line cohesion-review mark. It
  holds one function, `Player::BuyItemFromVendorSlot`'s 1,050-line body,
  which a move-only slice does not split; the catalog resolver, stock update
  and turn-in rows are in `catalog.rs`.
- **`net_move.py check`.** PASS. S = 2,651, G = 3,449 ≤ 3,744.6, 0 duplicate
  bodies.
- **Edges.** No `wow-world-*` edge is added. The planned
  `wow-world-inventory->wow-world-entities` edge was simulated on this base
  before anything moved (lock and metadata: PASS, 17 packages), with two
  negative probes failing as they must (`inventory->application`, a cycle;
  `inventory->quest`). The bodies did not need it: they reach entities state
  only through host methods. Inventory gains the external `wow-conditions`
  (the vendor item condition checks); `dependency-policy.json` allows it.
  `wow_world_closure.py check` passes on the head in lock and metadata mode
  (17 packages).
- **`session-ownership-policy.json`** is reprinted with `print-baseline`.
  The delta is the moved and retired impl items, the two adapter impls' items,
  the shim items (the five entry points and the seven wrappers) and the query
  shims' new module path. The syntax-only check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `handlers/character/mod.rs`: production 13,140 → 10,329, test
    15,706 → 15,679, total 28,846 → 26,008;
  - hotspot `session/mod.rs`: production 39,289 → 39,222, test 141,901 →
    141,878, total 181,190 → 181,100;
  - physical `handlers/character/vendor.rs` 788 → 273,
    `handlers/character/items.rs` 81 → 44, `handlers/character/mod.rs`
    522 → 499, `session/mod.rs` 972 → 970;
  - new physical rows `handlers/vendor_test_shims.rs` at 137 and
    `handlers/character_query_test_shims.rs` at 84.
- **Warnings.** The `wow-world` warning sets (`--message-format=short`
  lines under `crates/wow-world/`, compared without line numbers) equal base
  in all three builds: production lib 151, lib with `test-fixtures` 159, lib
  plus lib-test 296. `wow-world` `dead_code` is 0 in all three.
  `wow-world-inventory` (10, 7, 7) and `wow-world-application` (6, 4, 4) keep
  exactly their base warnings, and the new module has none. Three import
  lines changed only to keep the base messages: `SellResult` is a `cfg(test)`
  import of `session/mod.rs`, `BUYBACK_SLOT_COUNT` left its import (the shims
  import it directly), and the vendor-only names in the character `mod.rs`
  imports are dropped or `cfg(test)`.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed; the workspace
  test-name multiset is identical (10,214 names, 5 ignores). Three tests moved
  verbatim to inventory (above). The publication-order scan
  (`committed_money_callers_publish_all_runtime_state_before_reopening_admission`)
  now includes the nine `vendor_session` files, and its required runtime
  publications name the moved forms: `host.stage_player_money_change_like_cpp`,
  `host.apply_item_turnin_changes`, `host.insert_inventory_item_like_cpp`,
  `host.sync_player_registry_state_like_cpp();`,
  `update_vendor_item_current_count(` and the inlined
  `state.<operation>(&mut hub, …)` calls. Each still names the same runtime
  publication, and the no-`.await` and money-guard-drop checks are unchanged,
  so no assertion is weakened. Focused runs on the head:
  `wow-world-inventory` lib 26 passed; `wow-world` lib 283 passed matching
  vendor, bank, buy, sell, refund, repair, committed_money, money, currency
  and query.

### 8c plan

8c re-measures on its base. On the 8b head:

- the item handlers and inventory moves (`items.rs`,
  `items/{handlers,destruction,equipment_sets}.rs`,
  `items/inventory_moves*`) are 2,382 classifier lines with 45 outside calls,
  to inventory. The `inventory_swap` users (`wow-world-application`) stay
  behind host methods. The autobank and autostore moves left in `bank.rs`
  (124 lines) are the same kind of caller and go with them, or stay with the
  application storage move.
- gossip, the character lifecycle and the entry zone (`gossip.rs`,
  `lifecycle.rs`, `entry_zone.rs`) are 1,039 lines with 28 outside calls, to
  character. Gossip's calls to the show-bank publication, the trainer list
  and the trainer gossip menu (still in `vendor.rs`) stay host methods; its
  two vendor-list calls can call `wow_world_inventory::handle_list_inventory`
  only if the character host extends `VendorSessionHostLikeCpp`, otherwise
  they are host methods too.
- 8d (login load, 4,739 lines, 152 outside calls) is unchanged.

### Follow-ups

- `vendor_test_shims.rs` retires when the scenarios call the free functions,
  the splits and the core directly.
- The repair durability bodies (`session/player_items/{durability,bank}.rs`)
  and the quest-log item lookup are still World methods behind the host; they
  are the next inventory candidates.
- `buy.rs` (1,075 lines) is one function; splitting it is a behaviour-preserving
  refactor outside the move-only rule.
- The moved log sites now use the `wow_world_inventory::vendor_session`
  tracing target (#1384 follow-up 2 applies here too).

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 8c result (item handlers and inventory moves → inventory)

Branch `1363-s8c-items-gossip`, base `613948b05` (8b merged). Move-only (R2),
with the 4b/5a/7a/7c/8a/8b rewrites:

- `self.` becomes `host.`, and a session field becomes a host accessor
  (`lifecycle`, `interaction`);
- `self.catalogs` becomes `host.hub_ref().catalogs`,
  `crate::session::hub_ref(self)` becomes `host.hub_ref()`, and the one-line
  forwards `item_storage_template`, `item_template_flags` (catalogs) and
  `send_equip_error`, `send_packet_realm` (core) become that same call;
- a World wrapper that is one `split_inventory_{ref,mut}` plus one
  `InventoryState` call becomes that same split and call through the host
  (`get_inventory_item_by_pos`, `get_inventory_item_by_guid_like_cpp`, the
  resolved item object(s), `direct_item_contains_items`,
  `is_valid_inventory_pos_like_cpp`, `can_unequip_inventory_item_at_like_cpp`,
  the swap preflight, the descendants walk, the non-bank count, the bag-slot
  publication, the enchantment persistence, `inventory_container_db_guid_like_cpp`
  and `apply_inventory_item_object_updates_like_cpp`). Each split is taken for
  exactly that call, as the wrapper took it; no borrow spans another call or an
  `.await`. Where a wrapper call was itself an argument of another wrapper
  call (`direct_item_contains_items` inside `can_unequip…`, three sites), the
  two shared splits nest, both pure reads;
- four wrappers that were one call with session-field arguments become that
  call with the same reads through the host: the item relocation and
  dynamic-flag VALUES publications and the offhand request record (the
  `InventoryState` call through the inventory split, the core accesses through
  its hub), and the bank-use check (`can_use_current_bank_with_access_like_cpp`
  with `host.interaction()`, the hub's NPC access and the core's inventory
  access). The inline bag-object publication block of the real swap reads the
  same fields through the inventory split;
- the application storage move (`execute_inventory_storage_move_like_cpp`) and
  the UFCS plan call (`InventoryStorageMoveHostLikeCpp::storage_move_plan_like_cpp`)
  become host methods;
- a call between moved functions becomes a free call.

A token comparison of the 23 moved bodies against the base finds no other
difference (plus rustfmt trailing commas and two single-expression closures
whose braces rustfmt drops). Swap/split/destroy/equip/store order, the bag
and slot validation, the child redirect overlay and its rollback, the
equip-child plan, the stack merge and real swap persistence requests, the
destroy graph and its quest-status rows, the uncage postcondition, every
packet and publication step and its order, persistence ordering (#169),
registrations and processing flags are unchanged. No registrar moves: the
application character/account registrar keeps every entry, and its World
adapter (`account/host.rs`) calls the moved free functions. No moved body had
a `cfg(test)` branch.

### What moved

- **To `wow-world-inventory/src/item_session/`:**
  - `handlers.rs`: the swap-inv-item, auto-equip, auto-equip-slot, swap-item and
    auto-store-bag handlers (from `items/handlers.rs`);
  - `destruction.rs`: `handle_destroy_item`, the destroy quest plan and the
    full-stack destroy pair (from `items/destruction.rs`);
  - `moves.rs`: the swap queue and step, the redirected-move validation, the
    child redirect plan, the child auto-unequip, the equip-child plan and
    execution and the equip-to-empty move (from `items/inventory_moves.rs`);
  - `item_mutations.rs`, `real_swap.rs`: the offhand auto-unequip, the stack
    merge, the real-swap child plan and the real swap (from
    `items/inventory_moves/{item_mutations,real_swap}.rs`);
  - `bank.rs`: the autobank and autostore-bank moves (from `bank.rs`; they no
    longer name the application, whose storage move is a host method);
  - `mod.rs`: the host trait, the exports, and from the character `mod.rs`
    the destroy-count helper and its enum, the destroy quest-item and
    equip-child plan types, the swap step and the autostore-bank quest-check
    selector.
- **Down from `wow-world-application`** (`inventory_move_contracts.rs`):
  `InventoryStorageTargetLikeCpp`, `InventoryStorageQuestChecksLikeCpp`,
  `autostore_bank_target_like_cpp`, `InventorySwapTargetLikeCpp` and the three
  item helpers of `inventory_swap/equip_contracts.rs`, verbatim. The
  application keeps every path as a re-export.
- **Stays in `wow-world`:** the item turn-in application (`items.rs`), and
  the swap-target validation, the raw equip and the position publication,
  which run application participants; the last three moved verbatim from the
  deleted `inventory_moves{,/item_mutations}.rs` to the adapter file
  `items/host.rs`. The show-bank publication stays in `bank.rs` (gossip).
- **Test that followed its helper.**
  `destroy_item_count_action_matches_cpp_direct_item_branch` moved verbatim
  from `character_tests/item_4.rs` to
  `wow-world-inventory/unit_tests/item_session_helpers.rs`.
- **Reclassified, not moved** (to the `#![cfg(test)]`
  `handlers/item_test_shims.rs`, 249 lines, outside the character and session
  hotspot trees): the seven `cfg(test)` packet entry points of
  `items/handlers.rs` and `bank.rs`, verbatim except that they call the free
  functions; five method entry points for moved functions the scenarios call
  (`plan_destroyed_inventory_quest_persistence_like_cpp`,
  `destroy_inventory_full_stack_by_pos_like_cpp`, the child redirect, equip
  child and real-swap child plans); and six one-statement wrappers whose only
  production callers were the moved bodies (`direct_item_contains_items`,
  `plan_inventory_swap_preflight_like_cpp`,
  `represented_inventory_descendants_postorder_like_cpp`, the relocation and
  dynamic-flag publications, `represented_can_use_current_bank_like_cpp`),
  verbatim from `session/player_items/{items,persistence,storage_slots,publication,bank}.rs`.
  No scenario call site changes.
- **Retired:** `can_unequip_inventory_item_at_like_cpp`,
  `is_valid_inventory_pos_like_cpp` and `inventory_container_db_guid_like_cpp`
  (no other caller), and `record_represented_auto_unequip_offhand_request_like_cpp`,
  whose one remaining caller, a `cfg(test)` statement of
  `player_items/offhand.rs`, now makes the same `self.inventory` call.

### Host surface

- **`ItemSessionHostLikeCpp: VendorSessionHostLikeCpp`** (inventory, 21
  methods, 111 lines). It reuses the vendor, battle-pet purchase and session
  hosts for the inventory splits, the interaction and lifecycle accessors, the
  stat update, the registry sync, the player VALUES publication and the uncage
  state load. New: the storage move and its plan, the swap-target validation,
  the equip/store/bank planners, the raw equip, the committed relocation,
  removal and swap, the remove/store side effects, the quest item removal and
  its persistence plan, the position publication, the item-bonus stat update,
  the bag active-loot query, the loot release and the two `cfg(test)`
  observation records (titan grip, average item level). Their futures (storage
  move, raw equip, loot release) are the inherent futures returned unchanged.
- **Adapter** `handlers/character/items/host.rs` (230 lines of impl and 5 of
  imports, beside the three kept World operations). Every method names the
  inherent method by path (`WorldSession::name(self, …)`), the application
  free function, or the application trait method by UFCS
  (`<WorldSession as InventoryStorageMoveHostLikeCpp>::storage_move_plan_like_cpp`);
  each such inherent method exists, so none resolves to its own trait method.
  No forward goes to a trait default.
- The battle-pet session host's
  `destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp` now
  forwards to the moved free function of the same name, which is not a trait
  method, so it cannot recurse. The toy collection caller calls the moved
  full-stack destroy as a free function.
- **Widening.** The moved functions are `pub(super)` inside the module; the
  exports are the trait and the ten handlers/destroy functions the account
  adapter, the toy collection and the battle-pet adapter call. The two plan
  types (now with `pub` fields), the autostore-bank selector and the four
  plan functions the scenarios read are exported only under
  `cfg(any(test, feature = "test-fixtures"))`.
- **Budget.** 346 host lines (111 trait + 230 adapter + 5 imports) against
  about 2,380 moved production lines: 14.5 %. With the whole
  `item_session/mod.rs` apart from the moved types and helper and the test
  mount (about 185 lines: docs, imports, mounts, exports and the trait) it is
  about 420 lines, 17.6 %.

### Re-measure and the split

On the 8b head the items family was 2,382 classifier lines with 45 outside
calls (as planned), the autobank/autostore pair 124 lines with one outside
call, and gossip, the character lifecycle and the entry zone 1,039 lines with
28 outside calls. 3.5k is below 4.5k, but the host budget decides the split:

- part (a) alone is at 14.5 % only because four field wrappers became the
  same call through the host;
- gossip, lifecycle and the entry zone need about 19 host methods that no
  existing trait lends: the spell execution with its nine arguments and the
  cast GUID, the two trainer lists and the trainer gossip menu, the gossip
  option store (clear, read, replace) and gossip text, the interaction source
  read/write and the close-gossip publication (interaction splits), the
  instanceable-map check, the homebind and teleport, the zone update and
  zone-authority flag (instances), the quest-giver fallback and the corpse
  load. That is about 280 host lines for about 1,000 moved, so (a)+(b) together
  would be near 19 %.

So 8c moves part (a) and part (b) is 8c2 (below).

### Accounting

- **`crates/wow-world/src`.** Classifier: 65,470 → **63,201 production
  (−2,269)**. Physical: 70,833 → 68,829 (−2,004). Files: 500 → 498. Test:
  5,363 → 5,628 (+265).
  - **About 2,380 is production code that left `wow-world`** for inventory:
    the 23 bodies (2,225 lines of item text), the moved types and helper of the
    character `mod.rs` and the headers, imports and impl shells of the five
    deleted files.
  - **+256 is the adapter** (impl and imports of `items/host.rs`; its other 35
    lines are the three World operations relocated verbatim from the deleted
    files).
  - **About 129 is reclassified to test:** the seven `cfg(test)` packet entry
    points (80, counted as production by the classifier because they were
    `cfg(test)` methods inside a production impl) and the six wrappers (49).
  - **41 is retired:** the four wrappers.
  - The rest is import and mount edits (the character `mod.rs` imports that
    only the scenarios read are one `cfg(test)` group).
  - The test figure grows by the shim file (249) and the `cfg(test)` imports
    and mounts, less the moved helper test.
- **Destinations.** `wow-world-inventory/src`: 19,597 → 22,616 physical
  (+3,019); production 19,400 → 22,404 (+3,004; `item_session/` is 2,906
  lines, the moved application contracts 89). Its `unit_tests` gains the
  25-line helper test file. The inlined splits make the moved bodies longer
  than the World bodies.
- **`wow-world-application`.** 11,459 → 11,397 physical (−62): the moved
  vocabulary and helpers became re-exports.
- **`net_move.py check`.** PASS. Growth 2,957 ≤ 3,065.2, 0 duplicate bodies.
- **Edges.** Inventory gains `wow-world-entities` (the creature spawn
  catalogs the item bodies thread to the storage move); `dependency-policy.json`
  allows it. The edge was simulated on this base before anything moved
  (`wow_world_closure.py check --add`, lock and metadata: PASS, 17 packages,
  together with the 8c2 edges `wow-world-character->{interaction,visibility,spell}`),
  with two negative probes failing as they must (`inventory->application`, a
  cycle; `character->quest`). Inventory's `test-fixtures` feature also
  enables entities' fixtures. `wow_world_closure.py check` passes on the head
  in lock and metadata mode (17 packages).
- **`session-ownership-policy.json`** is reprinted with `print-baseline`.
  The delta is the moved, retired and relocated impl items, the adapter's 21
  trait items and the shim items. The syntax-only check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `handlers/character/mod.rs`: production 10,329 → 8,140, test
    15,679 unchanged, total 26,008 → 23,819;
  - hotspot `session/mod.rs`: production 39,222 → 39,136, test 141,878
    unchanged, total 181,100 → 181,014;
  - physical `handlers/character/items.rs` 44 → 40,
    `handlers/character/mod.rs` 499 → 450, `session/mod.rs` 970 → 969;
  - new physical rows `handlers/item_test_shims.rs` at 249 and (#1387 review
    follow-up 1) `wow-world-inventory/src/vendor_session/buy.rs` at 1,075.
- **Warnings.** The `wow-world` warning sets (`--message-format=short`
  lines under `crates/wow-world/`, compared without line numbers) are the base
  sets less four `private_interfaces` warnings that named the moved plan and
  step types, in all three builds: production lib 151 → 147, lib with
  `test-fixtures` 159 → 155, lib plus lib-test 296 → 292. No warning is new.
  `wow-world` `dead_code` is 0 in all three. `wow-world-inventory` (10, 7, 7)
  and `wow-world-application` (6, 4, 4) keep exactly their base warnings, and
  the new modules have none in the inventory and application all-targets
  builds.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed; the workspace
  test-name multiset is identical (10,214 names, 5 ignores). One test moved
  verbatim to inventory (above); no scenario call site changes. The
  publication-order scan
  (`committed_money_callers_publish_all_runtime_state_before_reopening_admission`)
  drops the five deleted World files and includes the seven `item_session`
  files instead. Its vendor-purchase stock marker is tightened from
  `update_vendor_item_current_count(` to the call with its host argument
  (`update_vendor_item_current_count(\n            host,`), as strict as the
  pre-8b `self.update_vendor_item_current_count` (#1387 follow-up 3); no
  other marker changes. Focused runs during the move (before the entry
  points moved to the shim file; the final run covers the head):
  `wow-world-inventory` lib 27
  passed; `wow-world` lib 578 passed matching item, inventory, swap, equip,
  destroy, bank, autostore, autobank, committed_money, child, offhand and
  stack.

### 8c2 plan

Gossip (hello, menu build, select, binder), the character lifecycle (realm
characters, rename enqueue, hearth-and-resurrect, customizations, default
graveyard homebind, map corpse load) and the entry zone (`gossip.rs`,
`lifecycle.rs`, `entry_zone.rs`; 1,039 lines, 28 outside calls on the 8b head)
go to character. Their host (about 19 methods, above) overlaps the 8d
login-load host: teleport, zone update and zone-authority flag, homebind,
interaction source and the corpse load are login operations too, and the
lifecycle functions' only callers are the login loaders. So 8c2 re-measures
on its base and either moves them alone if its host stays within 15 % (the
character host extending `VendorSessionHostLikeCpp` for the feign-death
removal, the condition creature object and context, the show-bank publication
and the free `handle_list_inventory` calls, and the entities host for the
gameobject gossip check), or rides with 8d, whose character host lends those
operations anyway. The planned `wow-world-character->{interaction,visibility,spell}`
edges were simulated with 8c (PASS); `character->quest` stays forbidden, so
the quest-giver fallback and gossip text stay host methods.

### 8d plan

Unchanged: login load (world entry, initial packets, login loaders, login
data, item login load, recovery, transport restore; 4,739 lines, 152 outside
calls on the 8a head) to character, re-measured on its base; if it exceeds
about 4.5k, `session_state/login_data.rs` splits off into an 8e.

### Follow-ups

- `item_test_shims.rs` (249) and `vendor_test_shims.rs` (137) retire when the
  scenarios call the free functions, the splits and the core directly.
- `buy.rs` (1,075, now with its own ratcheted row) is one function; splitting
  it is a behaviour-preserving refactor outside the move-only rule.
- The moved log sites now use the `wow_world_inventory::item_session`
  tracing target (#1384 follow-up 2 applies here too).
- The World `apply_item_turnin_changes`, the swap-target validation, the raw
  equip and the position publication stay World operations behind the vendor
  and item hosts; they move when their application participants can be built
  from the host.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 8c2 result (deferred: gossip, lifecycle and entry zone ride with 8d)

Branch `1363-s8c2-gossip`, base `ca23e294d` (8c merged). Nothing moved. The
re-measure on this base puts the host share above the 15 % budget for every
cut of the family. As the 8c2 plan allows, the family rides with 8d instead
of being forced. No code, ceiling, policy or test file changes; this section
is the only diff.

### Re-measure

The lexical scan is the slice 8 one: comments, literals and `cfg(test)` items
are blanked, and the scan counts `self.`/`session.` calls to `WorldSession`
methods defined outside the family. Production lines come from the hotspot
classifier.

| Family | Production | Outside methods | Call sites |
|---|---:|---:|---:|
| `gossip.rs` + `lifecycle.rs` + `entry_zone.rs` | 1,039 | 28 | 48 |
| login load (8d: `world_entry*`, `session_state*`, `items/login_load.rs`) | 4,739 | 152 | 236 |
| 8d with 8c2 folded in | 5,778 | 171 | 280 |
| `session_state/login_data.rs` alone (the 8e candidate) | 455 | 10 | 11 |

Both families are unchanged since the 8b head (8c touched neither). The
`cfg(test)` binder entry point (11 lines) would go to a shim file. The
trainer gossip helpers and constants of the character `mod.rs` (about 65
lines) would move with gossip, because `vendor.rs`'s trainer gossip menu
also calls them.

### Host surface, drafted

Five of the 28 calls inline as the same call through the hub: `player_guid`,
`send_packet`, `realm_id`, `send_tx`, `mmap_runtime_config_like_cpp`, and the
catalog reads `condition_store` and `player_condition_store`. Six are lent by
existing traits:

- `VendorSessionHostLikeCpp`: feign-death removal, the show-bank
  publication, the condition creature object and the player-condition
  context. Extending it also makes `wow_world_inventory::handle_list_inventory`
  a free call.
- `WorldEntitiesHostLikeCpp`: the gameobject gossip check.
- `CorpseHostLikeCpp`: `teleport_to`.

The 17 operations that remain have no lender:

- the interaction splits (`ref` and `mut`) and `split_lifecycle_ref`;
- the instanceable-map check;
- the cast GUID and the nine-argument spell execution;
- the two trainer lists and the trainer gossip menu;
- the gossip option store (clear, replace and read) and the gossip text;
- the quest-giver fallback;
- the homebind;
- the zone update and the zone-authority flag.

These operations were written out as a trait (`CharacterSessionHostLikeCpp:
VendorSessionHostLikeCpp + WorldEntitiesHostLikeCpp + CorpseHostLikeCpp`) and
a rustfmt-formatted `WorldSession` adapter in the 8b/8c forwarding style.
This was a scratch draft, not committed. It comes to **187 lines (76 trait +
111 adapter)**, which is the minimum before the destination module's
`mod.rs` (docs, mounts, exports) and the adapter imports.

| Cut | Host (minimum) | Moved production | Share |
|---|---:|---:|---:|
| whole family | 187 | about 1,093 (1,028 + about 65 helpers) | 17.1 % (about 19–21 % with `mod.rs` and imports) |
| gossip alone | about 156 (13 methods) | about 801 | 19.5 % |
| lifecycle + entry zone alone | about 48 (4 methods) | 292 | 16.4 % |

Each cut is above 15 % at its minimum, so nothing is forced in.

### Edges (simulated, nothing added)

`wow_world_closure.py check` with
`wow-world-character->{interaction,visibility,spell,instances}` added passes
in lock and metadata mode (17 packages). Three negative probes fail as they
must:

- `character->quest`;
- `entities->character` (a cycle);
- `inventory->character` (a cycle).

On this base only `wow-world` reaches character (application does not), and
character's closure is core, entities, interaction, inventory, lifecycle and
visibility.

### Recommendation for 8d

- **Lifecycle and the entry zone belong to 8d.** The login loaders are their
  only production callers: `session_state.rs` (corpse load, customizations),
  `initial_packets.rs` (entry zone) and `login_recovery.rs` (default graveyard
  homebind). The exceptions are `update_realm_characters` (the character
  handler host) and the rename enqueue (session driver callbacks), and both
  of those callers stay World adapters. If 8d moves without these functions,
  8d needs four extra host methods to call them. Moving them together makes
  those calls free calls. Their own operations are login operations 8d needs
  anyway: homebind, teleport, the zone update and zone authority, and the
  lifecycle split.
- **Gossip shares almost nothing with login load.** Only the five trivial
  forwards above are common to both. Its 13 methods cost about 156 host lines
  wherever it lands. It fits the budget only inside a combined character host
  budget (the 8d trait, which gossip would extend), not as its own slice.
- **Size.** 8d with all of 8c2 is 5,778 lines, above the plan's 4.5k
  threshold. The 8e rule applies:
  - **8d** is login load without `login_data.rs`, plus lifecycle and the
    entry zone, about 4,576 lines;
  - **8e** is `login_data.rs` plus gossip, about 1,200 lines. Gossip's
    methods extend the 8d host trait.

  8d re-measures on its base and decides gossip's place against the 8d/8e
  combined host share. If gossip still does not fit, it waits until its
  trainer and gossip-store callees can move with it.

### Follow-ups

- **#1388 optional follow-up 1 is deferred.** It would add
  `handlers/character/items/host.rs` to the
  `committed_money_callers_publish_all_runtime_state_before_reopening_admission`
  scan. The file has none of the scan's markers, so the assertions would not
  change. But the extra `include_str!` line raises the
  `handlers/character/mod.rs` hotspot's test figure from 15,679 to 15,680,
  above its live ceiling (checked: `check_architecture.py check` fails with
  +1). It goes in with 8d, which shrinks that list.
- **#1388 optional follow-up 2.** `item_session/mod.rs` exports the `plan_*`
  functions under `cfg(any(test, feature = "test-fixtures"))` only. No 8c2
  code depends on them.

### Validation

Documentation only. No Rust, policy or test file changed, so no build or test
evidence is claimed or needed for this delta. The closure runs and the
classifier and architecture checks above are the evidence of the re-measure.
No PR is opened for this section alone. It goes in with the 8d PR, or on its
own if the reviewer prefers.

## Slice 8d result (login load → character)

Branch `1363-s8d-login-load`, base `ca23e294d` (8c merged; the 8c2 section
above rides with this PR). Move-only (R2), with the 4b/5a/7a/7c/8a/8b/8c
rewrites and nothing else:

- `self` becomes `host` (an `&mut self` method becomes
  `fn name<H: CharacterLoginHostLikeCpp>(host: &mut H, …)`, an `&self` one
  takes `host: &H`), a call between moved functions becomes a free call, and a
  session field becomes a host accessor (`lifecycle`/`lifecycle_mut`,
  `visibility_mut`, `loot`/`loot_mut`, `world_entities_mut`), or
  `host.core()`/`host.core_mut()` for `self.core`;
- `crate::session::hub_ref(self)`/`hub_mut(self)` and
  `crate::session::split_*(self)` become the same call through the host, and
  `self.catalogs` becomes `host.hub_ref().catalogs`;
- a one-line World forward becomes that same call: the core forwards (`kick`,
  `send_tx`, `send_packet_realm`, `realm_id`, `player_registry`,
  `battlenet_account_id`, `set_instance_link_rx`), the catalog forwards
  (`currency_types_store`, `item_random_{properties,suffix}_store`,
  `item_stats_store`, `item_storage_template`, `item_template_max_durability`)
  and the config forward (`mmap_runtime_config_like_cpp`);
- a World wrapper that was one `split_{lifecycle,inventory}_{ref,mut}` plus one
  state call becomes that same split and call through the host (19 wrappers:
  the account toy/heirloom rows and sends, the worldport post-add advance, the
  equipment-inventory authority begin/complete, the inventory clear and object
  insert, the buyback insert, the CUF profile load and packet, the
  equipment-set loaded mark and packet, the void-storage loaded mark, the
  resolved inventory item, the favorite appearances, the resting-flag update
  and the loaded customizations). Each split is taken for exactly that call, as
  the wrapper took it; none is held across another call or an `.await` that
  the wrapper did not already hold it across, and none of them is async;
- `crate::session::cx_inventory_ref(self).make_inventory_item_object(` becomes
  `host.make_inventory_item_object(` (the vendor host's method is that exact
  call);
- paths: `crate::session::X`, `crate::FinalizationX` and
  `crate::character_administration::X` become the same item by its defining
  crate (`wow_world_core`, `wow_world_lifecycle`, …), and `crate::terrain`
  becomes `wow_world_core::terrain` (the World module is that re-export).

Two wrappers are deliberately **not** inlined, because the source scans name
their call text: `set_player_currencies_like_cpp` and
`player_currencies_like_cpp` (currency loading), and
`load_character_reputation_rows_like_cpp` (reputation loading). They are host
methods, so the scan markers keep their text except `self.` → `host.`.

A token comparison of all 43 moved bodies against the mechanical rewrite of
the base bodies (the same transform, applied by script) finds no other
difference than: `core()` → `core_mut()` and `lifecycle()` →
`lifecycle_mut()` where the call mutates (7 sites), the three wrappers kept as
host methods (above), the `wow_world_lifecycle::` path of the rename failure
type, rustfmt trailing commas and three single-expression closures whose
braces rustfmt drops.

*Correction (8e, #1389 review follow-up 2).* The paragraph above lists what
the token comparison found *beyond* the rewrite list; it is not the size of
the rewrite. The rewrite inlined **33 one-line World wrappers** at their call
sites, as listed above: the 7 core forwards, the 6 catalog forwards, the
config forward and the 19 `split_*` + one-call wrappers (plus the
`cx_inventory_ref` item-object path). The #1389 review checked each inlined
wrapper against its base body and found them all exact (it counted about
30). The `_mut` sites only make the borrows stricter. No moved body had a `cfg(test)` branch; the three
`#[cfg(test)]` entry points went to the shim file (below).

### What moved

To `wow-world-character/src/login_session/` (21 files, flat):

- `world_entry.rs`: the direct-login fallback, the logout request, the login
  combat-stats snapshot and the `HandlePlayerLogin` packet prelude (from
  `world_entry.rs`), plus `motd_lines_like_cpp` from the character `mod.rs`;
- `initial_packets.rs`: `SendInitialPacketsBeforeAddToMap`,
  `SendInitialPacketsAfterAddToMap` and the initial world-state load;
- `login_sequence.rs`: the persisted transport resolve, the init-transports
  plan and fellow-passenger blocks and the login packet sequence (from
  `session_state.rs`), plus `DEFAULT_MOTD_LIKE_CPP` and
  `primary_power_type_for_class_like_cpp` from the character `mod.rs`;
- `login_recovery.rs`: homebind area, invalid/repaired homebind persistence,
  login zone seeding, homebind retry, partial-login abort and the grid-load
  continuation;
- `inventory_load.rs`: the login inventory, bag, void-storage, equipment-set
  and transmog-outfit load (from `items/login_load.rs`), plus
  `void_storage_login_context_like_cpp`, `slot_to_inventory_type` and the
  bag-slot predicate it alone called;
- `lifecycle.rs`, `entry_zone.rs`: the realm-characters refresh, the rename
  enqueue, hearth-and-resurrect, the customization load, the default graveyard
  homebind and the corpse load; the post-add terrain zone;
- the 14 per-family loaders of `world_entry/login/` (admission, action buttons,
  auras, CUF profiles, currencies, default skills, glyphs, group, reputation,
  skills, spells and favorites, spell-map finalization, talents, transport
  restore). `world_entry/login/mail_loading.rs` (an empty impl) stays.
- `mod.rs`: the host trait, the mounts and the exports.

Down to `wow-world-spell` (`aura_rows.rs`): the two character aura row types
(`CharacterAuraRowLikeCpp`, `CharacterAuraEffectRowLikeCpp`) the aura loader
builds, verbatim with `pub` added (`session/pet_loading.rs`, now empty, is
deleted; `session/mod.rs` re-exports both at their old path).

**Stays in `wow-world`:** the continue-login coordinator
(`world_entry/login.rs`, `handle_continue_player_login_with_module_registry_like_cpp`),
which calls the moved loaders in the same order; `world_entry.rs` is now its
mount and imports the moved functions for it, and `session_state.rs` mounts
`login_data.rs` (8e). Every other caller (account adapter, character handler
host, session driver callbacks, transfer completion, travel) calls the free
function with `self`.

**Reclassified, not moved** (the `#![cfg(test)]`
`handlers/login_test_shims.rs`, 386 lines, outside the character and session
hotspot trees): the three `cfg(test)` entry points (`handle_logout_request`,
`test_load_initial_world_states_for_login_like_cpp`,
`send_initial_packets_after_add_to_map`, verbatim except that they name the
free functions), 16 one-call method forwards for the moved functions the
scenarios call (no scenario call site changes), and nine one-statement session
wrappers whose only production callers were the moved bodies, verbatim from
`session/{lifecycle_ops/operations,persistence/load,player_items/{appearance,equipment_sets,items,persistence_load}}.rs`.

**Retired:** `send_account_toys_like_cpp`, `mmap_runtime_config_like_cpp`,
`item_random_suffix_store`, `item_random_properties_store` (no other caller),
and `handlers/character/item_load_support.rs` (a re-export only the moved load
used).

**Tests that followed their code.** The two inventory-load source scans
(`continue_login_inventory_reads_cross_the_typed_lifecycle_port`,
`continue_login_item_repairs_cross_the_typed_lifecycle_port`) read only the
moved file; they moved verbatim from `character_tests/item_1.rs` to
`wow-world-character/unit_tests/login_session_scans.rs`, only the
`include_str!` path changing.

### Host surface

- **`CharacterLoginHostLikeCpp: VendorSessionHostLikeCpp + CorpseHostLikeCpp +
  WorldEntitiesHostLikeCpp`** (character, 55 methods, 132 lines). The
  supertraits lend the inventory splits, the lifecycle accessor and its mutable
  split, the item object build, the insert, the player money, `teleport_to`,
  the canonical-map ensure, the world-entities accessor and the hub
  (`player_guid`, `send_packet`, `send_raw_packet`, `state`). New: 7 accessors
  (`lifecycle_mut`, `split_lifecycle_ref`, `split_spell_state_ref`,
  `visibility_mut`, `split_visibility_mut`, `loot`, `loot_mut`) and 48
  operations. Five operations return a future
  (`finalize_session_with_generator_like_cpp`,
  `force_update_visibility_with_catalogs_like_cpp`,
  `load_active_player_trait_configs_like_cpp`,
  `send_battle_pet_journal_lock_status_like_cpp`, `send_contact_list_like_cpp`);
  each is the inherent future returned unchanged, awaited at the same point.
- **Adapter** `handlers/character/login_host.rs` (302 lines: 276 of impl, 25
  of header and imports). Every operation names the inherent method by path
  (`WorldSession::name(self, …)`); a script checked that each of the 48 names an
  inherent method of an `impl WorldSession` block, so none resolves to its own
  trait method and no forward reaches a trait default. The accessors are the
  field or the existing `crate::session::split_*` function.
- **Widening.** The moved functions are `pub(super)` inside `login_session`
  unless a World caller needs them; the crate exports the trait, the two loader
  row types, the inventory snapshot and the 30 functions World production
  calls. Ten more (the eight functions only the scenarios call through the
  shim, `motd_lines_like_cpp` and `void_storage_login_context_like_cpp`) are
  exported only under `cfg(any(test, feature = "test-fixtures"))`; the crate
  gains a `test-fixtures` feature that `wow-world`'s enables. The two aura row
  types become `pub` in spell for the loader.
- **Budget.** 434 host lines (132 trait + 302 adapter with its imports)
  against about 3,646 moved production lines (World classifier): 11.9 %. With
  the whole of `login_session/mod.rs` (266: docs, imports, mounts, exports and
  the trait) and the adapter it is 568 lines, 15.6 %.

### Await counts (per function, base vs head)

Counted with a script over the function bodies (comments and literals
blanked), base `origin/3.4.3` vs this head. Every moved function keeps its
count and, by the token comparison above, the same awaited calls in the same
order.

| Function | Base | Head |
|---|---:|---:|
| `send_handle_player_login_packets_like_cpp` | 8 | 8 |
| `load_inventory_for_login_like_cpp` | 7 | 7 |
| `send_login_sequence` | 7 | 7 |
| `send_initial_packets_before_add_to_map` | 5 | 5 |
| `load_login_admission_data_like_cpp` | 3 | 3 |
| `load_character_auras_for_login_like_cpp` | 2 | 2 |
| `send_initial_packets_after_add_to_map_with_catalogs_like_cpp` | 2 | 2 |
| `delete_invalid_character_homebind_like_cpp` | 1 | 1 |
| `handle_hearth_and_resurrect` | 1 | 1 |
| `handle_logout_request_with_generator_like_cpp` | 1 | 1 |
| `load_action_buttons_for_login_like_cpp` | 1 | 1 |
| `load_character_currencies_for_login_like_cpp` | 1 | 1 |
| `load_character_favorite_spells_for_login_like_cpp` | 1 | 1 |
| `load_character_glyphs_for_login_like_cpp` | 1 | 1 |
| `load_character_reputation_for_login_like_cpp` | 1 | 1 |
| `load_character_skill_rows_for_login_like_cpp` | 1 | 1 |
| `load_character_spell_rows_for_login_like_cpp` | 1 | 1 |
| `load_character_talents_for_login_like_cpp` | 1 | 1 |
| `load_cuf_profiles_for_login_like_cpp` | 1 | 1 |
| `load_group_membership_for_login_like_cpp` | 1 | 1 |
| `load_initial_world_states_for_login_like_cpp` | 1 | 1 |
| `load_map_corpse_data_like_cpp` | 1 | 1 |
| `load_player_customizations_like_cpp` | 1 | 1 |
| `persist_repaired_character_homebind_like_cpp` | 1 | 1 |
| `plan_init_transports_like_cpp` | 1 | 1 |
| `repair_character_homebind_like_cpp` | 1 | 1 |
| `resolve_persisted_transport_login_like_cpp` | 1 | 1 |
| `restore_persisted_transport_for_login_like_cpp` | 1 | 1 |
| `update_realm_characters` | 1 | 1 |
| 14 synchronous functions (abort, continue, retry, seed, homebind area, default skills, spell-map finalization, combat stats, fallback login, rename enqueue, post-add zone, default graveyard homebind, fellow passengers, bag-slot predicate) | 0 | 0 |
| **Total moved** | **56** | **56** |
| `cfg(test)` entry points (now in the shim): `handle_logout_request`, `test_load_initial_world_states_for_login_like_cpp`, `send_initial_packets_after_add_to_map` | 1 each | 1 each |
| World coordinator `handle_continue_player_login_with_module_registry_like_cpp` | 32 | 32 |

The World callers keep theirs too (`account/host.rs` 23, `travel.rs` 9,
`character_handler_host.rs` 1, `transfer_completion.rs` 2, unchanged per
file). No future that was awaited inline is spawned, boxed differently or
reordered: the account adapter's hearth future is still
`Box::pin(async move { … .await })` around the same body, and the logout
integration test pins the same future it pinned before.

### Login order

The coordinator's call order and the packet order inside the moved bursts are
the base text (above). The source-order scans still assert the same orders;
their markers change only where the call text changed:

- `character_tests/login.rs`: the coordinator bindings read
  `let talent_rows_complete_like_cpp = load_character_talents_for_login_like_cpp(`,
  `let Some(default_skill_entries) = apply_default_skills_for_login_like_cpp(`,
  `let reputation_rows_complete_like_cpp =\n            load_character_reputation_for_login_like_cpp(self,`
  and `let (mut skill_records, loaded_skill_records_like_cpp) =\n            load_character_skill_rows_for_login_like_cpp(self,`
  instead of `… = self` (the binding and the call it receives, as before);
  the moved-file markers read `host.` for `self.`, `rows_complete\n}` for the
  dedented `rows_complete\n    }`, `resolve_persisted_transport_login_like_cpp(\n            host,`
  and `seed_login_location_zone_area_like_cpp(host, zone, login_homebind)` for
  the method-call forms, and the default-skill forwarder text is the
  dedented `host.hub_mut()` form. Every order assertion is unchanged;
- the scans' `include_str!` lists follow the moved files
  (`login.rs`, `persistence.rs`, `group.rs`, `loot.rs`); `loot.rs` drops the
  emptied World mounts and the deleted `item_load_support.rs` and, as #1388
  follow-up 1, now includes `handlers/character/items/host.rs`;
- `scenarios_login_1.rs` names `wow_world_lifecycle::ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP`
  by path (the session re-export only the moved send read is gone); the
  assertion is the same.

### Re-measure and the split

Re-measured on the base with the slice 8 scan: the planned 8d (login load
without `login_data.rs`, plus lifecycle and the entry zone) is 4,576
production lines with 146 outside methods (236 call sites). That is above the
4.5k threshold, and its host does not fit: the coordinator `login.rs` alone is
922 lines with 86 outside methods (it is the World composition of the login),
and the whole set needs about 94 host methods after inlining, about 800 host
lines, near 18 %. The natural boundary is the coordinator: everything it calls
moves, and it stays as the World caller. Without it the set is 3,654 lines
with 89 outside methods (155 call sites), of which 41 are lent or inline; the
host is the 55 methods above, 11.9 %.

### Edges and closure

Character gains `wow-world-{loot,spell,visibility}` (the loot and visibility
state fields and the spell-state split the moved bodies read), plus
`wow-config`, `wow-network`, `wow-progression`, `flume`, `num-traits` and
`tokio` (MOTD and arena-season config, the instance link, the reputation row,
the rename send future, `ItemContext::from_u8` and the fallback oneshot);
`dependency-policy.json` allows them. The three workspace edges were simulated
on the base before anything moved (`wow_world_closure.py check --add`, lock
and metadata: PASS, 17 packages), with three negative probes failing as they
must (`character->quest`; `spell->character` and `loot->character`, cycles).
On the head `wow_world_closure.py check` passes in both modes: character's
direct set is core, entities, inventory, lifecycle, loot, spell, visibility;
its closure adds only interaction; only `wow-world` reaches character.

### Accounting

- **`crates/wow-world/src`.** Classifier: 63,201 → **59,703 production
  (−3,498)**. Physical: 68,829 → 65,737 (−3,092). Files: 498 → 479. Test:
  5,628 → 6,034 (+406).
  - **About 3,646 is production code that left `wow-world`:** the moved bodies
    of the 22 files (3,616 classifier lines less the 33 of the three entry
    points), the five helpers from the character `mod.rs` (42) and the two aura
    row types (22, to spell).
  - **+301 is the adapter** (`login_host.rs`).
  - **About 84 is reclassified to test:** the three entry points (33) and the
    nine wrappers (51) now in the shim file.
  - **About 32 is retired:** the four wrappers (13), `item_load_support.rs`
    (9) and the rest of `pet_loading.rs` (10).
  - The rest is import and mount edits (the character `mod.rs` imports only
    the scenarios read are `cfg(test)` groups, +18 test lines) and the
    rewritten call sites (`login.rs` −11).
  - The test figure grows by the shim file (386) and those `cfg(test)`
    imports.
- **Destinations.** `wow-world-character/src`: 3,088 → 7,315 physical
  (+4,227; production 3,088 → 7,287); `unit_tests` gains the 51-line scan
  file. `wow-world-spell/src`: 9,043 → 9,076 (+33, the row types). The
  inlined splits and the explicit imports make the moved files longer than the
  World ones.
- **`net_move.py check`.** PASS: growth 4,202 ≤ 4,205.5, 0 duplicate bodies.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`
  (the moved, retired and relocated items, the adapter's 55 trait items and
  the shim items). The syntax-only check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `handlers/character/mod.rs`: production 8,140 → 4,740, test
    15,679 → 15,655, total 23,819 → 20,395;
  - hotspot `session/mod.rs`: production 39,136 → 39,039, test 141,878
    unchanged, total 181,014 → 180,917;
  - physical `handlers/character/world_entry/login.rs` 922 → 897,
    `world_entry.rs` 221 → 29, `session_state.rs` 807 → 15, `items.rs`
    40 → 39, `mod.rs` 450 → 403, `session/mod.rs` 969 → 967;
  - new physical row `handlers/login_test_shims.rs` at 386.
- **Warnings.** The `wow-world` warning sets (`--message-format=short` lines
  under `crates/wow-world/`, compared without line numbers) are the base sets
  less one (the unused `Item` import of the deleted `pet_loading.rs`) in all
  three builds: production lib 147 → 146, lib with `test-fixtures` 155 → 154,
  lib plus lib-test 292 → 291. No warning is new. `wow-world` `dead_code` is
  0 in all three. `wow-world-character` has no warning in any of them, and
  `wow-world-spell` (8), `wow-world-inventory` (10, 7, 7) and
  `wow-world-application` (6, 4, 4) keep exactly their base warnings.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed; the workspace
  test-name multiset is identical (10,214 names, 5 ignores). Two tests moved
  to character (above). No scenario call site changes; the four logout calls
  of the `production_login_player_owner` integration test (built without
  `cfg(test)`, so no shim reaches it) call the production free function with
  `&mut session` instead of the removed method, same arguments and awaits.
  Focused runs during the move (the final run covers the head): `wow-world`
  lib, `wow-world-character` lib and `wow-world-spell` lib, all passing, and
  `production_login_player_owner` 34 passed.

### 8e plan

`session_state/login_data.rs` (455 lines, 10 outside methods: the trait
configs, the spell history and the skill-rewarded spell changes) and gossip
(hello, menu, select, binder; about 800 lines with its trainer-gossip helpers).
`login_data.rs` extends `CharacterLoginHostLikeCpp`; its three functions the
coordinator and the login sequence call become free calls (one host method,
`load_active_player_trait_configs_like_cpp`, retires). Gossip's 13 methods
extend the same trait; re-measure decides whether gossip fits or waits for its
trainer and gossip-store callees. The coordinator `login.rs` (897) stays: it is
the World composition of the login (86 World operations), and moves only when
its callees have owners.

### Follow-ups

- `login_test_shims.rs` (386) retires with the other shims when the scenarios
  call the free functions and splits directly.
- The moved log sites use the `wow_world_character::login_session` tracing
  target (#1384 follow-up 2 applies here too). *Clarified in 8e (#1389
  review follow-up 3):* this includes the `RUST_LOGIN` and `RUST_UPDATEOBJECT`
  login markers of `initial_packets.rs` and `login_recovery.rs`. The message
  text is unchanged, so a grep for the marker still finds them. A
  target-based filter (for example `RUST_LOG=wow_world::handlers::character=debug`)
  no longer selects them; it must name `wow_world_character::login_session`
  (or `wow_world_character`).
- #1388 follow-up 1 is done here (`items/host.rs` is in the publication scan).
- A combined `cargo check -p wow-world-character -p wow-world-spell
  --all-targets` without `wow-world` fails in `wow-world-inventory`: spell's
  dev-dependencies turn on `wow-world-core/test-fixtures` while inventory is
  built without its own. It is the existing feature-unification hazard of any
  inventory dependent built beside spell (*corrected in 8e:* "base has it for
  application" was wrong; `-p wow-world-application -p wow-world-spell
  --all-targets` builds, because application's own dev-dependencies turn on
  inventory's fixtures; fixed for character in 8e);
  `final` always builds them with `wow-world`, whose fixtures unify both.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 8e result (login data and gossip → character)

Branch `1363-s8e-login-data-gossip`, base `39db00fef` (8d merged). Move-only
(R2), with the 8d rewrites and nothing else. Both parts of the 8e plan moved:
the re-measure puts gossip inside the budget (below), so nothing is deferred.

- `self` becomes `host` (`fn name<H: CharacterLoginHostLikeCpp>(host: &mut H, …)`,
  or `host: &H` for an `&self` method); a call between moved functions becomes
  a free call; `self.core.X` becomes `host.core().X`, and `host.core_mut()` for
  the one mutating call (`mutate_world_creature`); `self.lifecycle.X` becomes
  `host.lifecycle().X`;
- `crate::session::hub_ref(self)`/`hub_mut(self)` and
  `crate::session::split_{spell_state_mut,interaction}(self)` become the same
  call through the host, and `self.catalogs` becomes `host.hub_ref().catalogs`;
- **nine one-line World wrappers are inlined** at their call sites, each as the
  exact body of the wrapper:
  - the four catalog forwards `skill_store`, `spell_store` (2 sites),
    `condition_store` and `player_condition_store` become
    `host.hub_ref().catalogs.X()`;
  - the five `split_*` + one-call wrappers
    `record_loaded_character_spell_cooldown_like_cpp`,
    `mark_represented_character_spell_cooldowns_loaded_like_cpp`,
    `record_loaded_character_spell_charge_like_cpp`,
    `mark_represented_character_spell_charges_loaded_like_cpp` (one site each,
    the spell-history load) and `set_player_interaction_source_like_cpp`
    (three sites: the direct interaction, the DB menu build and the trainer
    gossip menu) become the same split and state call in a scoped block. Each
    split is taken for exactly that call, as the wrapper took it, none is held
    across another call or an `.await`, and none of them is async. The
    spell-history blocks have the same form as the two reset blocks already in
    that body;
- paths: `crate::session::GossipOptionInfo` becomes
  `wow_entities::PlayerGossipOptionLikeCpp` (the type the World name
  re-exports), and `super::super::spell_rules::X` becomes
  `wow_world_spell::login_spell_rules::X` (the World module is that glob
  re-export).

`player_interaction_source_guid_like_cpp` is deliberately kept as a host
method, not inlined. It is read inside an `if` condition and a log field, and
inlining it would restructure that expression.

A token comparison of the 17 moved functions against their base bodies (with
`self` read as `host`) finds 4 identical. The other 13 differ only by the
rewrites above, rustfmt trailing commas and one single-expression closure in
`build_gossip_menu` whose braces rustfmt drops. No moved body had a
`cfg(test)` branch. The `cfg(test)` binder entry point went to the shim file
(below).

### What moved

To `wow-world-character`:

- `login_session/login_data.rs`: `skill_rewarded_quest_fallback_allowed_like_cpp`,
  `skill_rewarded_spell_changes_for_login_like_cpp`,
  `load_character_spell_history_packets_like_cpp` and
  `load_active_player_trait_configs_like_cpp` (C++ `Player::_LoadSkills`
  reward changes, `SpellHistory::LoadFromDB`, `Player::_LoadTraits`), from
  `handlers/character/session_state/login_data.rs`;
- `gossip_session.rs`:
  - from `handlers/character/gossip.rs`: the binder activation
    (`HandleBinderActivateOpcode`/`SendBindPoint`), the direct NPC
    interaction, the gossip hello (`HandleGossipHelloOpcode`), the two gossip
    condition checks, the DB gossip menu build (`PrepareGossipMenu`/
    `GetGossipTextId`) and the gossip select
    (`HandleGossipSelectOptionOpcode`);
  - from `handlers/character/vendor.rs`: the prepared trainer gossip menu
    `send_represented_creature_trainer_gossip_menu_like_cpp`;
  - from the character `mod.rs`: the direct-interaction and gossip-option
    constants, `npc_has_direct_interaction_like_cpp` and the trainer gossip
    option helpers.

`login_sequence.rs` now calls the trait-config load as a free call, so the 8d
host method `load_active_player_trait_configs_like_cpp` retires.

**Stays in `wow-world`, calling the free functions with `self`:**

- the continue-login coordinator: the skill-rewarded changes twice and the
  spell history once;
- the character/account handler adapter `account/host.rs`: binder, gossip
  hello and gossip select. Its `Box::pin(async move { … .await })` futures are
  unchanged;
- the quest-giver hello `handlers/quest/handlers/queries.rs`: the DB menu and
  the trainer menu.

`session_state.rs` (the mount) and `gossip.rs` are deleted. `vendor.rs` is now
only the `VendorSessionHostLikeCpp` adapter.

**Reclassified, not moved.** These went to the `#![cfg(test)]`
`handlers/login_data_gossip_test_shims.rs` (141 lines), outside the character
and session hotspot trees:

- the `cfg(test)` binder entry point (`handle_binder_activate`, verbatim
  except that it names the free function);
- seven one-call method forwards for the moved functions the scenarios call.
  No scenario call site changes;
- the five inlined wrappers above, verbatim from
  `session/{spell_state/cooldown,spell_state/spell,npc_interaction}.rs`. The
  scenarios still call them, and they have no production caller left.

**Retired:**

- `handlers/character/session_state.rs`: the mount, 15 lines;
- the empty `handlers/character/world_entry/login/mail_loading.rs`. It was an
  `impl WorldSession {}` that 8d left. Its mount in `login.rs` is gone, and so
  are its two scan includes in `character_tests/{loot,persistence}.rs`. An
  empty impl holds no marker those scans look for, so neither scan loses
  anything it asserts;
- the glob import of the emptied `session/spell_state/cooldown.rs`.

**Tests.**

- The publication scan's `include_str!` of `gossip.rs` follows the file to
  `wow-world-character/src/gossip_session.rs`.
- The scenarios name `wow_world_character::GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP`
  by path (two sites). `GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP` is a `cfg(test)`
  import of the character `mod.rs`. Both constants are exported only under
  `cfg(any(test, feature = "test-fixtures"))`. The assertions are unchanged.

### Host surface

- **`CharacterLoginHostLikeCpp`** grows from 55 to 70 methods and from 132 to
  179 lines:
  - 2 new accessors: `split_spell_state_mut` and `split_interaction`;
  - 14 new operations:
    - the login data: trait-config authority begin and complete, and the
      trait spell-definition ids;
    - gossip: clear, replace and read the gossip options, the interaction
      source GUID, the instanceable-map check, the cast GUID, the
      nine-argument spell execution, the two trainer lists, the creature
      gossip text and the quest-giver fallback;
  - 1 retired: the trait-config load.

  The supertraits lend the rest: the hub (`send_packet`, `player_guid`,
  `core`), the vendor host (feign-death removal, the show-bank publication,
  the condition creature object, the player-condition context and the free
  `handle_list_inventory`) and the entities host (the player-condition id
  check and the gameobject gossip check). Three new operations return a
  future: the spell execution and the two trainer lists. Each is the inherent
  future returned unchanged and awaited at the same point.
- **Adapter** `handlers/character/login_host.rs` grows from 301 to 399 lines.
  A script checked that all 61 `WorldSession::name(self, …)` forwards name a
  method of an inherent `impl WorldSession` block. None resolves to its own
  trait method, so there is no trait recursion.
- **Widening.**
  - The crate exports the five gossip entry points World production calls,
    and two of the login data functions.
  - Exported only under `cfg(any(test, feature = "test-fixtures"))`: the
    other two login data functions (only the scenarios and the login sequence
    call them) and the two gossip-option constants.
  - The direct-interaction constants and the helpers stay private to
    `gossip_session`.
- **Budget.**

  | Cut | Host lines | Moved production | Share |
  |---|---:|---:|---:|
  | 8e (trait +47, adapter +98) | 145 (151 with the trait's 6 import lines) | 1,308 | 11.1 % (11.5 %) |
  | login data alone | 22 | 455 | 4.8 % |
  | gossip alone | 123 (about 129 with imports) | 853 | 14.4 % (about 15.1 %) |
  | character host after 8d + 8e (trait 179 + adapter 399) | 578 | 4,954 | 11.7 % |
  | the same with the whole `login_session/mod.rs` (319) | 718 | 4,954 | 14.5 % |

  By the 8c2/8d rule, gossip is decided against the combined share, which is
  11.5 % for 8e and 11.7 % for the 8d + 8e character host. Even gossip alone is
  at the 15 % line, not above it as in 8c2 (19.5 %). Two things made the
  difference: the trait it extends already lends eight of its calls, and the
  trainer gossip menu (with its helpers) moves with it instead of being a host
  method. So gossip moved.

### Await counts (per function, base vs head)

Counted with the 8d script (comments and literals blanked).

| Function | Base | Head |
|---|---:|---:|
| `build_gossip_menu` | 5 | 5 |
| `handle_gossip_hello` | 2 | 2 |
| `handle_gossip_select_option` | 2 | 2 |
| `handle_npc_direct_interaction` | 2 | 2 |
| `load_active_player_trait_configs_like_cpp` | 2 | 2 |
| `load_character_spell_history_packets_like_cpp` | 2 | 2 |
| `handle_binder_activate_with_generator_like_cpp` | 1 | 1 |
| 10 synchronous functions (the two skill-rewarded functions, the two condition checks, the trainer gossip menu and its four helpers, `npc_has_direct_interaction_like_cpp`) | 0 | 0 |
| **Total moved** | **16** | **16** |
| `cfg(test)` `handle_binder_activate` (now in the shim) | 1 | 1 |
| World coordinator `handle_continue_player_login_with_module_registry_like_cpp` | 32 | 32 |
| `send_login_sequence` (the trait-config load is now a free call) | 7 | 7 |
| `account/host.rs` `binder_activate_like_cpp`, `gossip_hello_like_cpp`, `gossip_select_option_like_cpp` | 1 each | 1 each |
| `queries.rs` `handle_quest_giver_hello` | 1 | 1 |

The awaited calls are the same, in the same order. No future is spawned,
boxed differently or reordered.

### Behaviour

Every moved body is its base text after the rewrites above. In particular,
these are unchanged:

- the gossip menu contents and order: the menu id, the text-id selection
  (last row whose conditions meet), the per-option condition filtering, the
  localized option text, the auto-trainer option and the quest text;
- the NPC interaction checks: the hello flags, the trainer fallback access,
  the quest-giver recheck and the select source revalidation against the
  creature or the gameobject;
- the feign-death removal points;
- the packets and their order: `GossipMessage`, the NPC interaction results,
  `AuctionHelloResponse`, the bank show and the close gossip after the bind
  cast;
- the bind spell cast and its flags;
- the login data load (#169): the spell-history reset, load, record and mark
  order, and the trait-config entries, configs and authority completion.

The opcode registrations, their processing and status flags are untouched
(`wow-world-application` is not in the diff). The adapter only changes the
path of the callee.

### Edges and closure

- Character gains `wow-world-interaction` (the `InteractionState` split, the
  close gossip and the interaction source) and `wow-conditions` (the gossip
  condition evaluation). `dependency-policy.json` allows both.
- The interaction edge was simulated on the base before anything moved
  (`wow_world_closure.py check --add`, lock and metadata: PASS, 17 packages).
  Two negative probes fail as they must: `character->quest`, and
  `interaction->character` (a cycle).
- On the head the check passes in both modes. Character's direct set is core,
  entities, interaction, inventory, lifecycle, loot, spell and visibility,
  which is also its closure. Only `wow-world` reaches character.

### Accounting

- **`crates/wow-world/src`.** Classifier: 59,705 → **58,403 production
  (−1,302)**. Physical: 65,739 → 64,582 (−1,157). Files: 479 → 476. Test:
  6,034 → 6,179 (+145). Re-measured on `39db00fef`; the 8d section's 59,703
  for the identical tree is 2 lines low.
  - **1,308 is production code that left `wow-world`:**
    - `login_data.rs` (455);
    - `gossip.rs` less the binder entry point (736);
    - the trainer gossip menu and its impl shell (39);
    - the constants and helpers of the character `mod.rs` (78).
  - **+98 is the adapter growth.**
  - **58 is reclassified to test:** the binder entry point (11) and the five
    wrappers (47).
  - **30 is retired:**
    - `session_state.rs` (15);
    - `mail_loading.rs` (10) and its mount (2);
    - the glob import and the empty impl of `cooldown.rs` (3).
  - **The rest is import, mount and call-site edits (−4):**
    - the character `mod.rs` imports only the scenarios read are `cfg(test)`;
    - the call sites in `account/host.rs` (+2), `queries.rs` (+1) and
      `login.rs` (+1, the `self,` argument lines);
    - `world_entry.rs` (−2, its module doc condensed).
  - The test figure grows by the shim file (141) and the `cfg(test)` imports.
- **Destination.** `wow-world-character/src`: 7,257 → 8,708 physical
  (+1,451); production 7,229 → 8,671. `gossip_session.rs` is 892 lines,
  `login_data.rs` 488.
- **`net_move.py check`.** PASS: growth 1,451 ≤ 2,175.9, 0 duplicate bodies.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`. The
  delta is exactly:
  - the moved, relocated and retired items;
  - the adapter's 16 new trait items, less the retired one;
  - the 15 shim items.

  The syntax-only check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `handlers/character/mod.rs`: production 4,740 → 3,487, test 15,655
    unchanged, total 20,395 → 19,142. The test figure holds, because the two
    retired scan includes offset the `cfg(test)` imports the scenarios still
    read;
  - hotspot `session/mod.rs`: production 39,039 → 38,989, test 141,878
    unchanged, total 180,917 → 180,867;
  - physical: `handlers/character/mod.rs` 403 → 323, `vendor.rs` 273 → 234,
    `world_entry.rs` 29 → 27, `world_entry/login.rs` 897 → 896;
  - the `session_state.rs` row is retired;
  - new physical row `handlers/login_data_gossip_test_shims.rs` at 141.
- **Warnings.** The `wow-world` warning sets are identical to the base in all
  three builds: production lib 146, lib with `test-fixtures` 154, lib plus
  lib-test 291. They are the `--message-format=short` lines under
  `crates/wow-world/`, compared without line numbers, and both base and head
  were built for this comparison.
  - `wow-world` `dead_code` is 0 in all three builds.
  - `wow-world-character` has no warning in any build.
  - `wow-world-spell`, `wow-world-inventory`, `wow-world-application` and
    `wow-world-interaction` keep exactly their base sets.
- **Tests.**
  - No `#[test]` or `#[ignore]` is added or removed. The workspace test-name
    multiset is identical (10,214 names, 5 ignores).
  - Focused runs during the move (the final run covers the head): the
    `wow-world` lib passed 3,680 with 1 ignored; the `wow-world-character`
    lib passed 2.
- **Tracing.** The moved log sites now use the `wow_world_character::gossip_session`
  and `wow_world_character::login_session::login_data` targets. Their message
  text is unchanged. A target-based filter must name the new targets (see the
  8d follow-up).

### #1389 follow-ups

1. **Done.** Character's `test-fixtures` now forwards to every
   fixture-bearing `wow-world-*` dependency: core, entities, interaction,
   inventory, lifecycle, loot, spell and visibility. Character's own tests
   turn it on through a self dev-dependency, the pattern spell and inventory
   use. The results:
   - `cargo check -p wow-world-character -p wow-world-spell --all-targets`
     builds;
   - with only the inventory part removed, it fails again: 6 errors in
     `wow-world-inventory`, which is the reported failure;
   - `cargo check -p wow-world-character --all-targets` and
     `cargo test -p wow-world-character` build too. The inventory dev-dependency
     alone was not enough: it broke spell in that standalone test build,
     which is why the feature forwards to all eight.
   - Production is unaffected. `cargo tree -p world-server -e normal,features`
     shows no `test-fixtures`.
2. **Done.** The 8d wording is corrected in place, marked as an 8e correction:
   33 one-line wrappers were inlined, all of them verified exact.
3. **Done.** The tracing-target change, including `RUST_LOGIN` and
   `RUST_UPDATEOBJECT`, is noted at the 8d follow-up.
4. **Open.** Retiring `login_test_shims.rs` (386) remains open, and the new
   `login_data_gossip_test_shims.rs` (141) joins it. Both retire when the
   scenarios call the free functions and splits directly.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in the
PR body. This section is not edited after that run.

## Slice 8 summary (8a–8e)

The character family (`handlers/character/**`) was 14,788 production lines on
`518d8bd66`. Slice 8 moved its bodies to their owners:

| Slice | Destination | `wow-world` production | Δ | Moved out | Adapter | Reclassified | Retired |
|---|---|---|---:|---:|---:|---:|---:|
| 8a visibility | entities | 70,077 → 68,349 | −1,728 | about 1,775 | +144 | about 92 | 5 |
| 8b vendor and bank | inventory | 68,349 → 65,470 | −2,879 | 2,905 | +220 | 146 | 35 |
| 8c item handlers and moves | inventory | 65,470 → 63,201 | −2,269 | about 2,380 | +256 | about 129 | 41 |
| 8c2 | (deferred into 8d/8e) | — | 0 | — | — | — | — |
| 8d login load | character (+ two row types to spell) | 63,201 → 59,703 | −3,498 | about 3,646 | +301 | about 84 | about 32 |
| 8e login data and gossip | character | 59,705 → 58,403 | −1,302 | 1,308 | +98 | 58 | 30 |
| **Total** | | **70,077 → 58,403** | **−11,674** | **about 12,014** | **+1,019** | **about 509** | **about 143** |

The rest of the total is import, mount and call-site edits.

*Footnote (9a, #1390 review follow-up 2).* The per-slice Δ column sums to
−11,676, two more than the total row's −11,674. The difference is the 8d
row: 8d recorded 59,703 for the tree that 8e re-measured as 59,705 (see the
8e accounting). The total row is the end-to-end measurement 70,077 → 58,403
and is correct; against the re-measured figure the 8d row reads 63,201 →
59,705, −3,496.

- **Moved vs reclassified.** About 12,014 production lines left `wow-world`
  as moved code. About 509 left the production figure without moving: they
  are `cfg(test)` entry points and wrappers whose only production callers
  moved, now in ratcheted `#![cfg(test)]` shim files. About 143 were retired.
  The World adapters added back 1,019.
- **Character hotspot.** Production went from 14,788 to 3,487 over the slice
  (the hotspot ledger's figure for the `handlers/character/**` tree).
- **What character code remains in `wow-world`, and why:**
  - **the F5 adapters** of the moved bodies, about 1,490 lines:
    `login_host.rs` 399, `account/host.rs` 320, `items/host.rs` 291,
    `vendor.rs` 234, `visibility.rs` 144, `character_handler_host.rs` 106.
    They are the World side of the host traits and go only when `WorldSession`
    itself is split;
  - **the continue-login coordinator** `world_entry/login.rs` (896). It is
    the World composition of the login: 32 awaits over about 86 World
    operations (#169 order). It moves when its callees have owners (8d plan);
  - **the quest-giver status queries** `query.rs` (208). They name quest
    types, and character may not reach quest;
  - **the account collections** `account/collections.rs` (259: account-wide
    collection loading and the login spell projection, still read by the
    coordinator) and `account.rs` (106: enumeration support, account-session
    operations such as `SMSG_CONNECT_TO`);
  - **the shared `mod.rs`** (176 production): constants, re-exports and
    `cfg(test)` scenario imports. Also about 100 lines of small modules:
    `condition_objects.rs`, `stats*.rs`, the re-export files and the
    `cfg(test)` shims.
- **Host shares** (trait plus adapter over moved production; the figure in
  parentheses adds each module's whole `mod.rs`): 8a 11.6 % (14.1 %), 8b
  11.4 % (13.6 %), 8c 14.5 % (17.6 %), 8d 11.9 % (15.6 %), 8e 11.1 %
  (11.5 % with the trait imports). The 8c2 cut was deferred instead of forced
  above 15 %.
- **Open after slice 8.**
  - Retire the shim files (`login_test_shims.rs` 386,
    `login_data_gossip_test_shims.rs` 141, and the item, vendor and query
    shims) by converting the scenario call sites.
  - Move the coordinator once its callees have owners.

## Slice 9 plan (spell effects and player cast, phase 2)

Measured on `b5fdc923b` (8e merged) before anything moved, with the slice 8
tools: the hotspot classifier for production lines, and the lexical scan
that blanks comments, literals and every `cfg(test)` item or statement and
counts the `self.`/`session.` calls to inherent `WorldSession` methods
defined outside the family. Field reads (`self.core`, `self.catalogs`,
`self.spell_state`) are not counted; they become hub or field accessors.
"Outside" includes methods of the other spell families, which become
in-crate calls once those move too, so later parts overstate.

- **`wow-world` production (classifier): 58,403** lines, 64,582 physical,
  476 files.
- **The spell domain is about 14k production lines**, twice the original
  slice 9 row (about 7k): the row did not count the spell-state, aura,
  spell-click and acquisition families.

| Family | Files | Production | Methods | Outside calls |
|---|---|---:|---:|---:|
| Spellbook (known spells, learn/remove, spell rows, traits, overrides, talent-row side effects, acquisition marks) | `session/spell_state/{spellbook,spell,effects,acquisition}.rs` | 1,516 | 63 | 20 |
| Spell effects: execution | `session/spell_effects/{execution*,execution/**,destinations,checks}.rs` | 2,360 | 20 | 77 |
| Spell effects: combat | `session/spell_effects/{effect_combat*,effect_combat/**,combat,threat}.rs` | 2,115 | 34 | 18 |
| Spell effects: the rest | `session/spell_effects/{effects,effects_player,effects_power,effects_progress,effect_apply,effect_summon}.rs` | 2,608 | 58 | 37 |
| Player cast | `session/player_cast/**`, `session/player_cast.rs`, `session/spell_state/cast.rs` | 1,149 | 29 | 20 |
| Spell handlers | `handlers/spell/**` | 1,394 | 32 | 28 |
| Auras left | `session/spell_state/{aura,aura/**,aura_application,aura_publication,mount_aura}.rs` | 1,374 | 46 | 26 |
| Spell click | `session/spell_state/spell_click.rs`, `session/world_entities/spell_click.rs` | 574 | 7 | 11 |
| Talents | `session/progression/talents.rs`, `handlers/talent/**` | 402 | 11 | 12 |
| Acquisition adapters | `spell_acquisition/**`, `session/{effect_learning,trainer_acquisition}.rs` | 1,215 | 9 | 13 |
| Cooldowns and charges | `session/spell_state/cooldown.rs` (the rest left in 8e) | 7 | 0 | 0 |
| Catalog setters | `session/spell_state/catalog.rs` | 133 | 26 | 0 |

Findings:

- **The spellbook has the lowest surface per line** (1,516 lines over 20
  outside calls) and is the prerequisite the 7c talent decision named. Of its
  20 calls, 3 are hub defaults (`player_guid`, `send_packet`) or one-line
  catalog forwards (`spell_store`, `skill_store`); one is a `cfg!(test)`
  term. It goes first.
- **The spell-map dispatch stays on the session for now.**
  `with_player_spell_runtime_like_cpp` and `mutate_player_spell_runtime_like_cpp`
  pass the World `cfg!(test)` flag; as host methods the flag keeps its World
  meaning without a fixture predicate.
- **The login spell application stays with the coordinator.** The
  start-all-spells option, the loaded dependency closure, the login
  proficiencies and combat capabilities, `set_known_spells_like_cpp`, the
  favorites and the login spell reset are called only by the continue-login
  coordinator (`handlers/character/world_entry/login.rs`, plus scenarios).
  Their free-function call syntax wraps longer than the method syntax:
  moving them grew the coordinator from 896 to 913 lines, above its
  physical ceiling, even with an import block. They move with the
  coordinator (8d plan).
- **Talents** need, after the spellbook, the talent reset commit and
  publication (persistence), the points refresh (it builds
  `wow_world_quest::QuestRewardCx`, so it stays a World host method), the
  at-login flags and the loaded check.
- **Spell effects (7.1k) do not fit one part.** Execution has the largest
  surface (77) and goes last.

**Closure pre-check.** Every edge the plan may need was simulated on the base
with `tools/architecture/wow_world_closure.py check --add …`, in lock and
metadata mode, before anything moved: both PASS (17 packages) with
`wow-world-spell->wow-world-{entities,inventory,interaction,visibility,loot}`
added together. Five negative probes fail as they must:
`wow-world-spell->wow-world-quest`, `->wow-world-character` and
`->wow-world-loot-ops` (cycles through their spell dependency), and
`->wow-world-social` and `->wow-world-instances` (cycles through social).
9a needs no edge. Each part re-runs the check for the edges it actually adds.

**Split (each part re-measures on its own base; at most about 4.5k moved
and 15 % host):**

- **9a, the spellbook → spell:** the learn/remove closure, the known-spell
  reads, the spell-row, trait-config and override operations, the
  talent-row spell side effects and the acquisition marks. Result below.
- **9b, talents, auras, spell click and player cast → spell (about 3.5k):**
  talents first (the 7c design step), then the auras left, spell click and
  the player cast with `spell_state/cast.rs`. Adds `spell->entities` (and
  `->inventory` if the off-hand and item checks move instead of staying
  host methods).
- **9c, spell effects without execution → spell (about 4.7k):** the effect
  bodies and the combat effects. If the 9c re-measure exceeds about 4.5k,
  threat (425) waits for 9d.
- **9d, execution and the spell handlers (about 3.8k, surface about 100):**
  the execution entry points, destinations, checks and `handlers/spell`.
  Highest surface last; it may split again.
- **Later:** the acquisition adapters (1.2k) and the coordinator's login
  spell application, with the coordinator.

## Slice 9a result (the spellbook → spell)

Branch `1363-s9a-spellbook`, base `b5fdc923b` (8e merged). Move-only (R2),
with the 8d/8e rewrites and nothing else:

- `self` becomes `host` (`fn name<H: SpellbookHostLikeCpp>(host: &mut H, …)`,
  or `host: &H` for an `&self` method); a call between moved functions
  becomes a free call; `self.core` becomes `host.core()` (the hub default,
  `&self.core`), `self.catalogs` becomes `host.hub_ref().catalogs` and
  `self.spell_state` becomes `host.spell_state()`;
- `crate::session::hub_ref(self)`/`hub_mut(self)` and
  `crate::session::split_spell_state_{ref,mut}(self)` become the same call
  through the host;
- **two one-line World wrappers are inlined**, each as the exact body of the
  wrapper: `self.spell_store()` (in the titan-grip cleanup) becomes
  `host.hub_ref().catalogs.spell_store()`, and `self.skill_store()` (in the
  learn-skill downgrade) becomes `host.hub_ref().catalogs.skill_store()`.
  Both base bodies are `self.catalogs.X()`. Neither is async.

No moved body had a `cfg(test)` branch or a `cfg!(test)` term, so no
fixture predicate was needed. A token comparison of the 38 moved functions
against the same rewrite applied by script to their base bodies finds 36
identical; the other two differ by one rustfmt trailing comma each.

### What moved

To `wow-world-spell/src/spellbook_session/` (C++ `Player::LearnSpell`,
`Player::RemoveSpell`, `Player::GetSpellMap` and the spell-map
bookkeeping around them):

- `spellbook.rs`, from `session/spell_state/spellbook.rs`: the learn-skill
  lookup, the learn-validity and direct-trigger reads, the account-mount
  learn, the learn and dependent learn, the remove closure (remove,
  remove with suppressed messaging and the recursive remove with its seen
  set: next ranks, requiring spells, the learn-skill downgrade, learned
  spells and their overrides, the lower-rank relearn with
  `SMSG_SUPERCEDED_SPELLS`, the trait override, titan grip, dual wield, the
  off-hand check and `SMSG_UNLEARNED_SPELLS`), and the known-spell and
  dependent-spell reads. The previous-skill lookup, the downgrade and the
  recursive remove stay private, as they were;
- `runtime.rs`, from `session/spell_state/spell.rs`: the talent spell and
  override-pair reads, the trait-definition take, the loaded-row install,
  the trait-config begin, complete and clears, the fallback-row clear, the
  row invalidation, the complete and loaded row replacement, the titan-grip
  cleanup (now `pub(super)`), the override add, remove and reads, the
  trait-definition reads and replacement, and the complete-row read;
- `effects.rs`, from `session/spell_state/effects.rs`: the talent-row load
  with its spell side effects and the active-talent spell side effects, with
  `SPELL_AURA_INTERRUPT_FLAG2_CHANGE_TALENT_LIKE_CPP`, which only they read;
- `acquisition.rs`, from `session/spell_state/acquisition.rs`: the
  auxiliary-authority invalidation and the snapshot completeness mark;
- `mod.rs`: the host trait and the mounts.

**Stays in `wow-world`:**

- the login spell application the coordinator calls (plan above):
  `apply_represented_start_all_spells_with_catalogs_like_cpp`,
  `apply_loaded_spell_dependencies_from_roots_like_cpp`,
  `apply_loaded_known_spell_dependencies_like_cpp`,
  `apply_loaded_spell_dependency_skills_like_cpp`,
  `apply_login_known_spell_{proficiencies,combat_capabilities}_like_cpp`,
  `set_known_spells_like_cpp`, `set_represented_favorite_known_spells_like_cpp`
  and `apply_represented_login_spell_reset_if_needed_like_cpp`. They now
  call the moved functions as free calls;
- the spell-map dispatch and its snapshot (the World `cfg!(test)` flag);
- the composition setters (cast unstuck, off-hand check, script and
  acquisition authority), the spell-focus, hit-inert and hit-source reads,
  the reset notification text, the mount promotion and usability, the
  self-res read and the far-teleport interrupt. They belong to later parts
  (cast, auras, acquisition) or are composition.

Every World caller calls the free function with `self` (48 call sites in 22
files). The four host adapters that forwarded a moved method by path now
name the free function: `login_host.rs` (six forwards), `session_loot_host.rs`
(`known_spells_like_cpp`) and `battle_pet_purchase/host.rs` (the dependent
learn). Left as `WorldSession::name(self)`, those forwards would have
resolved to their own trait method once the inherent method was gone; the
compiler's recursion lint caught the loot one in an intermediate build, and a
grep for `WorldSession::`/`Self::` plus method-call syntax of every moved
name finds none left.

**Reclassified, not moved** (the `#![cfg(test)]`
`handlers/spellbook_test_shims.rs`, 433 lines, outside the session hotspot
tree):

- the 11 `#[cfg(test)]` methods nested in the production impls of the four
  files (the fixture known-spell writes, the trait-definition and override
  fixtures, the test dispatch, the loaded-row and acquisition-runtime
  fixtures and the start-all-spells test entry), verbatim except that
  `pub(in crate::session)` reads `pub(crate)`. They were already test-only;
  the classifier counted them as production because they were nested;
- the spellbook's three #1241 F3 test entry points, verbatim from
  `unit_tests/session/spell_state/spellbook/f3_shims.rs` (deleted with its
  mount). This is test code moving, not a production change;
- 23 one-call forwards to the moved functions the scenarios and these
  fixtures call. No scenario call site changes.

**Tests that changed their import path only:** the talent scenarios import
the interrupt-flag constant from `wow_world_spell`; `skill_owner.rs` names
`wow_world_spell::canonical_player_spell_record_like_cpp` by path and
`spellbook_owner.rs` imports it (the session root no longer imports it for
production). No assertion changes.

### Host surface

- **`SpellbookHostLikeCpp: SessionHubHostLikeCpp`** (spell, 8 methods, 24
  lines with docs). The supertrait lends the hub (`core`, `player_guid`,
  `send_packet`, `hub_ref`, `hub_mut`). The methods are 3 accessors (the
  `spell_state` field and the two existing splits) and 5 operations: the
  spell-map read and dispatch, the off-hand auto-unequip, the aura removal
  for a spell and the interrupt-flag aura removal. None is async.
- **Adapter** `session/spell_state/spellbook_host.rs` (41 lines). Each
  operation names the inherent method by path; a script checked that all
  five name a method of an inherent `impl WorldSession` block, so none
  resolves to its own trait method.
- **Widening.**
  - In spell: the crate exports the trait and the 30 functions World
    production calls. Five more (the learn-skill lookup, the suppressed
    remove, the resolved known-spell read, the loaded-row replacement and the
    interrupt-flag constant) are exported only under
    `cfg(any(test, feature = "test-fixtures"))` for the shim and the
    scenarios.
  - In World: `with_player_spell_runtime_like_cpp`,
    `mutate_player_spell_runtime_like_cpp` and
    `player_spell_runtime_snapshot_like_cpp` widen to `pub(crate)` because the
    reclassified fixtures in the shim call them (the 7c precedent). The #754
    rule still holds in production: the dispatch's production callers are
    the remaining `session/spell_state` functions and the host adapter,
    through which only the moved spellbook functions reach it.
- **Budget.** 65 host lines (trait 24 + adapter 41) against 791 moved
  production lines: 8.2 %. With the whole `spellbook_session/mod.rs` (60) it
  is 101 lines, 12.8 %.

### Await counts (per function, base vs head)

Counted with the 8d script (comments and literals blanked) over every
changed World file and the destination files.

| Function | Base | Head |
|---|---:|---:|
| 38 moved functions (none is async) | 0 | 0 |
| 17 async World functions in the changed files (the spell-click caster 2; first-login casts, self-res, spell click, the five account collection loads, achievements, delayed teleport, `teleport_to{,_with_options}`, the stored-item loot save and wait: 1 each) | 18 | 18 |

The rewritten call sites are synchronous calls; none sits in an expression
with an `.await`, and no future is spawned, boxed or reordered.

### Behaviour

Every moved body is its base text after the rewrites above. In particular,
these are unchanged: the learn order (the hit-aura authority invalidation
before the row invalidation and the runtime learn; the dependent mark after
the learn), the remove order (next ranks unless a talent, requiring spells,
the forget with complete-row preservation, the learn-skill downgrade,
learned spells and their overrides, the lower-rank relearn as dependent or
not, `SMSG_SUPERCEDED_SPELLS`, the override and trait-definition cleanup,
titan grip, dual wield, the off-hand check, then `SMSG_UNLEARNED_SPELLS`
with its suppress flag), the account-mount learn, the talent-row and
active-talent side effects (the interrupt-flag aura removal last) and the
acquisition marks. No persistence code moved; the talent commit and the
login load order (#169) are untouched. The opcode registrations, their
processing and status flags are untouched (`wow-world-application` is not in
the diff).

### Edges and closure

No `Cargo.toml` change: spell already depends on core, lifecycle, data,
entities and packet. `wow_world_closure.py check` passes in lock and
metadata mode (17 packages); spell's direct set is core and lifecycle.

### Accounting

- **`crates/wow-world/src`.** Classifier: 58,403 → **57,424 production
  (−979)**. Physical: 64,582 → 64,035 (−547). Files: 476 → 478. Test: 6,179 →
  6,611 (+432).
  - **791 is production code that left `wow-world`:** the 38 moved functions
    (783 lines) and 8 lines of separating blank lines, net of the free-call
    rewrites in the functions that stay in those four files.
  - **233 is reclassified to test:** the 11 nested `#[cfg(test)]` methods.
    They were test-only code at base; this is not a production reduction.
  - **+41 is the adapter**, +1 its mount.
  - **The rest (+3) is call-site and import edits:** the free-call
    rewrites in 15 files (movement state +8, load authority +4, talents +3,
    appearance +2, effect learning +1, others −1 to −3 each) and the
    session-root imports (−5).
  - The test figure grows by the shim file (433), less the F3 shim mount.
- **Destination.** `wow-world-spell/src`: 9,076 → 10,013 physical (+937;
  production 8,812 → 9,742, +930; test +7, the fixture-gated export).
  `spellbook_session/` is 908 lines: `spellbook.rs` 440, `runtime.rs` 257,
  `effects.rs` 121, `mod.rs` 60, `acquisition.rs` 30.
- **`net_move.py check`.** PASS: growth 937 ≤ 1,535.3, 0 duplicate bodies.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`.
  The delta is exactly: 55 items leave `crate::session` (38 moved, 11
  reclassified, 3 relocated F3 entry points and the 3 old visibilities of
  the widened dispatch, read and snapshot), and 48 arrive (37 shim items, the
  adapter's 8 trait items and the 3 widened methods). The syntax-only check
  passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `session/mod.rs`: production 38,989 → 38,012, test 141,878 →
    141,842, total 180,867 → 179,854. The test figure falls because the
    spellbook F3 shim left the session tree; the two test import edits add 1;
  - hotspot `handlers/character/mod.rs`: production 3,487 → 3,486 (the
    collections call site), test 15,655 (live, unchanged), total 19,142 →
    19,141;
  - physical `session/mod.rs` 967 → 962;
  - physical `handlers/character/mod.rs` 323 → 321 (#1390 follow-up 1; the
    file did not change in 9a);
  - new physical row `handlers/spellbook_test_shims.rs` at 433.
- **Warnings.** The `wow-world` warning sets (`--message-format=short` lines
  under `crates/wow-world/`, compared without line numbers against the 8e
  head, whose tree is this base) are identical in all three builds:
  production lib 146, lib with `test-fixtures` 154, lib plus lib-test 291.
  One base warning (the unused `known_spells` binding of the verbatim
  `fixture_set_known_spells_like_cpp`) moved with its method to the shim
  file. `wow-world` `dead_code` is 0 in all three. `wow-world-spell` and
  every other crate keep exactly their base sets.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed; the workspace
  test-name multiset is identical (10,214 names, 5 ignores). Focused runs
  during the move (the final run covers the head): the `wow-world` lib
  passed 3,680 with 1 ignored, and the `wow-world-spell` tests passed 25.

### #1390 follow-ups

1. **Done.** The `handlers/character/mod.rs` physical ceiling is 321, the
   live value. The hotspot test figure (15,655) is the live value too; it
   was not stale.
2. **Done.** The slice 8 summary has a footnote for the 59,703/59,705
   correction.
3. **Checked.** The binder entry point was a `#[cfg(test)]` method nested in
   a production impl. The classifier counts nested `cfg(test)` items as
   production, so moving it to the `#![cfg(test)]` shim lowered the
   classifier figure without removing production code. 8e reported it as
   reclassified, not moved, which is the right label; 9a labels its 233
   lines the same way.
4. **Open.** Retiring the shim files, now including
   `spellbook_test_shims.rs` (433), remains open.

### 9b plan

Talents first: `session/progression/talents.rs` and `handlers/talent/**`
(402) can now call the learn/remove closure in spell. The host adds the
talent reset commit and publication, the points refresh (World:
`QuestRewardCx`), the at-login flag read and removal, the talents-loaded
check and `trinity_string`. Then the auras left (1,374), spell click (574)
and the player cast with `spell_state/cast.rs` (1,149), re-measured for the
15 % budget; spell gains the `entities` edge if the aura or click bodies
need it (simulated PASS above).

### Follow-ups

- The coordinator's login spell application (about 220 lines in
  `session/spell_state/{spellbook,effects}.rs`) moves with the coordinator.
- The moved log sites (none in 9a; the spellbook bodies do not log) need no
  tracing note.
- The spell-map dispatch moves to spell when its last World caller does;
  its `cfg!(test)` flag then becomes the host's fixture predicate.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 9b result (talents, auras, aura sources and the spell-click caster → spell)

Branch `1363-s9b-talents`, base `0f421ba4a` (9a merged). Move-only (R2),
with the 9a rewrites and nothing else:

- `self` becomes `host` (`fn name<H: XHostLikeCpp>(host: &mut H, …)`, or
  `host: &H`); a call between moved functions, or to a function already in
  spell, becomes a free call (`wow_world_spell::f(self, …)` reads `f(host, …)`
  or `crate::f(host, …)`); `self.core` becomes `host.core()`, or
  `host.core_mut()` where the body mutates through the field (the mount
  display RNG and the clickee creature damage); `self.catalogs` and
  `self.config` become `host.hub_ref().catalogs` / `.config`;
  `crate::session::{hub_ref,hub_mut,split_*}(self)` become the same call
  through the host;
- **one one-line World wrapper is inlined**, as its exact body:
  `self.spell_store()` (base body `self.catalogs.spell_store()`) becomes
  `host.hub_ref().catalogs.spell_store()` at its 4 sites (the aura load, the
  capability speed auras, the clickee caster and the cast-spell-info read).
  It is not async;
- `crate::session::game_time_ms_like_cpp()` (a World re-export) names its
  source, `wow_world_core::session::game_time_ms_like_cpp()`;
- **fixture steps.** The four `#[cfg(test)]` blocks of the mounted-aura apply
  and removal use the 4b accessor,
  `#[cfg(any(test, feature = "test-fixtures"))] if let Some(fixtures) = host.test_fixtures_like_cpp()`.
  The character aura load's `#[cfg(test)]`/`#[cfg(not(test))]` pair keeps
  its production arm (`_canonical.then_some(())`) and gates the handle-less
  fixture install on `host.test_fixtures_ref_like_cpp().is_some()`; inside
  it the two-line test-only `mutate_player_aura_subsystem_like_cpp` wrapper
  is written as its body (the spell-state split and the
  `SessionSpellState` call). In a production build the predicate and its
  branch are compiled out.

A token comparison of the 34 moved functions against the same rewrite
applied by script to their base bodies finds only these classes of
difference: visibility, rustfmt trailing commas, the in-crate call paths,
the fixture steps, `core_mut`, the game-time path and one multi-line
`hub_mut(self)` the script did not fold.

### Re-measure and the cut

On `0f421ba4a` (classifier, `wow-world` production 57,424) the 9b families
held: talents 405 lines, auras left 1,371, spell click 574, player cast
1,149 (3,499). Moved as planned, the talent host alone is about 30 % of its
body (7c measured the same), so the slice is cut by host cost, not size:

- **Talents.** The learn, the active and login resets, the reset text, the
  active-talent spell removal and `CMSG_LEARN_TALENT` move. **The respec wipe
  (`CMSG_CONFIRM_RESPEC_WIPE`) stays** in `handlers/talent/state.rs`: it was
  moved first and reverted in this branch, because its host surface (the
  feign-death removal, the talent-reset commit and publication as async
  host methods and an associated committed type, which also needed
  `CommittedRepresentedTalentResetLikeCpp` widened) cost about 36 host lines
  for 93 body lines.
- **Auras.** Everything with a body moves (below). The split forwards over
  `SessionSpellState`, the application-context wrappers (`remove_aura`,
  `apply_aura_with_effect_mask*`, the feign-death removal), the 23-caller
  `apply_represented_aura_modifier_like_cpp` (its call sites would push
  `spell_effects/execution.rs` past 1,000 lines), the coordinator's login
  passive auras and the spell-hit authority check stay.
- **Spell click and cast.** The clickee caster with its creature school
  damage, the spell-area autocast check and the cast-spell-info read move.
  The click plan needs the player-condition context through an associated
  type (the inventory pattern), the plan execution and the player cast need
  the spell execution as async host methods, and `player_cast.rs` is the
  `Runtime` adapter itself; they go with execution (9d).

### What moved

To `wow-world-spell/src/` (C++ `Player::LearnTalent`, the runtime half of
`Player::ResetTalents`, `Unit::_AddAura`/`RemoveAura` bookkeeping,
`Player::_LoadAuras`, `AuraEffect::HandleMount`, `SpellArea::IsFitToRequirements`,
`Player::GetCastSpellInfo` and `Creature::HandleSpellClick` with the
clickee as caster):

- `talent_session/talents.rs`, from `session/progression/talents.rs` and
  `session/movement/state.rs`: `reset_represented_talents_like_cpp`, the
  active and login talent resets, the talent learn, the reset notification
  text with `LANG_RESET_TALENTS{,_TEXT}_LIKE_CPP` (only it reads them) and
  `remove_represented_active_talent_side_effects_like_cpp`;
- `talent_session/handlers.rs`, from `handlers/talent/state.rs`:
  `handle_learn_talent`;
- `aura_session/aura_state.rs`, from `session/spell_state/aura.rs`: the
  caster-less aura visual, the player and unit aura-state masks, the mounted
  amount, the character aura load and the mount display candidates and
  selection;
- `aura_session/mount.rs`, the whole of `session/spell_state/mount_aura.rs`
  (deleted): the mounted aura, its capability speed auras and the removal by
  type;
- `aura_session/application.rs`, from `aura_application.rs`: the
  indoor/outdoor, attribute, spell and interrupt-flag removals, the
  far-teleport moving/turning removal with
  `SPELL_AURA_INTERRUPT_FLAG_MOVING{,_OR_TURNING}_LIKE_CPP` (only it reads
  them), the spell-focus aura and the expiry tick;
- `aura_session/publication.rs`, from `aura_publication.rs`: the visible-aura
  insert, the next free slot and `send_aura_update_applied`;
- `aura_session/sources.rs`, from `spell_state/cast.rs` and
  `aura/spell_hit_authority.rs`: the spell-area autocast check and source
  and the trait-config aura source;
- `spellbook_session/runtime.rs` gains `represented_cast_spell_info_like_cpp`
  (the override read) from `spell_state/cast.rs`;
- `spell_click_session/mod.rs`, from `spell_state/cast.rs` and
  `world_entities/spell_click.rs`: the clickee caster and the creature
  school damage to the clicker and the clickee, with the host trait.

Every World caller calls the free function with `self` (27 sites in 20
files, adapters included). **Recursion check.** Six trait impls forwarded a moved name by path
(`login_host.rs`: the talent reset and the aura load; `entities/host.rs`:
the mounted-aura removal; `spellbook_host.rs`: the spell and interrupt-flag
aura removals; `movement/host.rs` and `chat_handler_contexts.rs`: the
interrupt-flag removal). Each now names `wow_world_spell::…`. A grep of
every moved name with method or `WorldSession::`/`Self::` path syntax over
the production files (the `#![cfg(test)]` shims excluded) finds only those
trait definitions; every host adapter forward was checked by script to name
a method of an inherent `impl WorldSession` block; and the production lib
build reports no `unconditional_recursion`.

**Reclassified, not moved:**

- the `#[cfg(test)]` mounted-aura test entry nested in the production impl
  of `mount_aura.rs` (9 lines), verbatim, to the `#![cfg(test)]`
  `handlers/aura_test_shims.rs`;
- the talents' #1241 F3 test entry points, verbatim (`pub(in crate::session)`
  reads `pub(crate)`) from `unit_tests/session/progression/talents/f3_shims.rs`
  (deleted with its mount) to the `#![cfg(test)]`
  `handlers/talent_test_shims.rs`. This is test code moving;
- 11 one-call forwards to the moved functions the scenarios and fixtures
  call (4 talent, 7 aura). No scenario call site changes.

**Tests that changed their import only:** `persistence/load/f3_shims.rs`
imports `TalentTabStore` (the session root no longer does) and
`scenarios_world_entities_14.rs` imports `SliceRandom` (likewise). No
assertion changes.

### Host surface

| Trait (spell) | Methods | Trait lines | Adapter | Adapter lines |
|---|---:|---:|---|---:|
| `TalentHostLikeCpp: SpellbookHostLikeCpp` | 5: the points refresh (it builds `QuestRewardCx`), the at-login flag read and removal, the talents-loaded check, `trinity_string` | 10 | `session/progression/talent_host.rs` | 23 |
| `AuraHostLikeCpp: SpellbookHostLikeCpp` | 11: the entities split, `remove_aura`, the aura modifier, the mount capability selection and display condition, the collision height, the cast GUID, the threat snapshot, the creature aura tick, the spell-area quest status, the spell-map snapshot | 36 | `session/spell_state/aura_host.rs` | 85 |
| `SpellClickHostLikeCpp: AuraHostLikeCpp` | 2: the registry sync, the unit VALUES update object | 10 | `session/spell_state/spell_click_host.rs` | 24 |

- None is async. Each adapter operation names the inherent method by path
  (script-checked). `SpellbookHostLikeCpp` is unchanged: its two aura
  removals now forward to the moved free functions.
- **Budget.** 188 host lines (traits 56 + adapters 132) against 1,265 moved
  function lines: **14.9 %**.
- **Widening.** In spell the crate exports the three traits and the 21
  functions World production calls; three more (the moving interrupt flag,
  the mount display candidates and the talent learn) only under
  `cfg(any(test, feature = "test-fixtures"))`. No World item is widened.
- **#1391 follow-up (a).** The #754 note on
  `mutate_player_spell_runtime_like_cpp` now names its production writers
  (`session/spell_state` and its `SpellbookHostLikeCpp` adapter, through
  which only the spellbook bodies reach it) and why it is `pub(crate)` (the
  cfg(test) spellbook shim). A `cfg_attr` cannot change visibility, and a
  second gated definition would duplicate the body.

### Await counts (per function, base vs head)

Counted with the 8d script (comments and literals blanked) over every
changed World file and the destination files.

| Function | Base | Head |
|---|---:|---:|
| `execute_represented_spell_click_clickee_caster_like_cpp` (moved) | 2 | 2 |
| `handle_learn_talent`, the two creature-damage bodies (moved, async) | 0 | 0 |
| 30 other moved functions | sync | sync |
| 41 async World functions in the changed files | 88 | 88 |
| `handle_learn_talent` forward in `talent_test_shims.rs` (test only) | – | 1 |

The talent adapter's two thunks keep one `.await` each, and the click-plan
execution keeps its two (the clickee call is now a free call). No future is
spawned, boxed or reordered, and no borrow is split differently across an
await.

### Behaviour

Every moved body is its base text after the rewrites above. In particular:
the talent learn order (loaded check, rank, validation, snapshot, row load,
spell side effects, points refresh), the active reset (runtime take, spell
and override removal per talent, refresh) and the login reset (flag test,
script hook, flag removal with persist, pet removal, reset, talent data
packet, `LANG_RESET_TALENTS` notification); the mounted-aura order
(amount, display, template fallback, slot, presentation, aura insert,
vehicle kit, vehicle rec ID, pet controls, collision height, capability
speed auras, `SMSG_AURA_UPDATE`, mount unit update); the aura load's row
filters, charge rule, slot and cast-GUID allocation and install; the removal
and expiry orders; the autocast and trait-config source rules; the clickee
`SMSG_SPELL_GO` and the damage branches. The respec wipe, its reset cost,
the talent-reset transaction and its publication are untouched (#169). The
opcode registrations, processing and status flags are untouched
(`wow-world-application` is not in the diff). The moved log sites (the
`LearnTalent` read warning, the aura expiry and the two spell-click damage
debug lines) now use the `wow_world_spell::{talent_session,aura_session,spell_click_session}`
tracing targets.

### Edges and closure

`wow-world-spell -> wow-world-entities` is added (the entities split and its
state type, the spell-click plan values, the turning interrupt flag);
spell's `test-fixtures` enables `wow-world-entities/test-fixtures`, and the
dependency policy allows the edge. It was simulated on `b5fdc923b` (9a) and
re-simulated here with `--add`, then checked as added: `wow_world_closure.py
check` passes in lock and metadata mode (17 packages); spell's direct
`wow-world` set is core, entities and lifecycle.

### Accounting

- **`crates/wow-world/src`.** Classifier: 57,424 → **56,283 production
  (−1,141)**. Physical: 64,035 → 63,094 (−941). Files: 478 → 482. Test:
  6,611 → 6,811 (+200).
  - **About 1,290 is production code that left `wow-world`:** the 34
    moved functions (1,265 lines at base), the five constant lines that
    moved with them and about 20 lines of separators and the emptied
    `mount_aura.rs` scaffolding.
  - **12 is reclassified to test:** the mounted-aura `#[cfg(test)]` method
    (9) and three session-root re-exports only the scenarios read, now
    `cfg(test)`. The session root's −10 production is those 3, the five
    moved constant lines and two imports only the moved bodies read.
  - **+132 is the three adapters**, +2 their mounts.
  - **+29 is call-site and comment edits** (execution +6, the #754 note +5,
    talent host thunks +5, movement +5, others).
  - The test figure grows by the two shim files (189) and their mounts
    (+4) and the gated session-root lines with their attributes (+10), less
    the deleted talents F3 shim mount (−3).
- **Destination.** `wow-world-spell/src`: 10,013 → 11,600 physical
  (+1,587; production 9,742 → 11,323, +1,581; test +6, the gated exports).
  New: `aura_session/` 1,091 lines (`aura_state.rs` 365, `mount.rs` 245,
  `application.rs` 211, `sources.rs` 165, `mod.rs` 68, `publication.rs` 37),
  `talent_session/` 244 (`talents.rs` 164, `handlers.rs` 48, `mod.rs` 32),
  `spell_click_session/mod.rs` 203; `spellbook_session/runtime.rs` +18,
  `lib.rs` +35.
- **`net_move.py check`.** PASS: growth 1,587 ≤ 1,949.1, 0 duplicate bodies.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`.
  The delta is exactly: 45 items leave (32 moved session methods,
  `handle_learn_talent` from `handlers::talent::state`, 12 test-fixture
  items: the 11 talents F3 entry points and the mounted-aura test entry),
  and 41 arrive (18 adapter trait items, 23 shim items). The syntax-only
  check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `session/mod.rs`: production 38,012 → 36,898, test 141,842 →
    141,777, total 179,854 → 178,675. The test figure falls because the
    talents F3 shim left the session tree;
  - new physical rows `handlers/talent_test_shims.rs` at 114 and
    `handlers/aura_test_shims.rs` at 75;
  - physical `session/mod.rs` stays at its ceiling (962): the cfg(test)
    re-exports are paid for by the moved constants and dropped imports.
- **Warnings.** The `wow-world` warning sets (`--message-format=short` lines,
  compared without line numbers against the base) are identical in all
  three builds: production lib 146, lib with `test-fixtures` 154, all
  targets 291, and every other crate in those builds keeps its base set.
  `wow-world-spell --all-targets` keeps its base set too. `wow-world`
  `dead_code` is 0 in all three.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed; the workspace
  test-name multiset is identical (10,214 names, 5 ignores). Focused runs
  during the move (the final run covers the head): the `wow-world` lib
  passed 3,680 with 1 ignored, and the `wow-world-spell` tests passed 25.

### 9c plan

Re-measured on this head: spell effects without execution as planned
(about 4.7k; threat waits for 9d if the re-measure exceeds 4.5k). The
remaining 9b families join the execution part (9d), because each needs the
spell execution as an async host method: the click plan and its execution
(500 lines, plus the player-condition context through an associated type),
the player cast (948, with the `Runtime` adapter), the respec wipe (with the
talent-reset commit and publication) and `apply_represented_aura_modifier_like_cpp`
(with the effect bodies that call it). The spell-hit aura authority check
and the coordinator's login passive auras go with the coordinator.

### Follow-ups

- Retire the shim files, now including `talent_test_shims.rs` (114) and
  `aura_test_shims.rs` (75), when the scenarios call the free functions.
- The spell-map dispatch moves when its last World caller does (9a note).

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 9c result (spell effects without execution → spell)

Branch `1363-s9c-spell-effects`, base `923fd0942` (9b merged). Move-only
(R2), with the 9a/9b rewrites and nothing else:

- `self` becomes `host` (`fn name<H: SpellEffectHostLikeCpp>(host: &mut H, …)`,
  or `host: &H`); a call between moved functions, or to a function already
  in spell, becomes a free call; `self.core` becomes `host.core()`, or
  `host.core_mut()` where the body mutates through the field
  (`mutate_world_creature` in the healing, PvE-stop, distract, sanctuary,
  instakill and power-drain bodies); `self.catalogs` and `self.config`
  become `host.hub_ref().catalogs` / `.config`; `self.spell_state` becomes
  `host.spell_state()` for the read and `host.split_spell_state_mut().0` for
  the two execute-log writes; `crate::session::{hub_ref,hub_mut,split_*}(self)`
  become the same call through the host; World re-export paths name their
  source (`crate::session_rules::*` → `crate::{aura_effects,melee_rules}::*`,
  `crate::creature_movement` → `wow_world_core::creature_movement`,
  `crate::session::game_time_ms_like_cpp` → `wow_world_core::session::…`);
- **three one-line World wrappers are inlined**, each as its exact body, none
  async: `self.spell_store()` (body `self.catalogs.spell_store()`) at 6 sites
  (the attribute read, the damage mechanic, the target mechanic mask, the
  heal absorb, the disallowed mount form and the transform-spell check),
  `self.send_packet_realm(…)` (body `self.core.send_packet_realm(…)`) in the
  homebind write and `self.send_tx()` (body `self.core.send_tx()`) in the
  distract effect;
- **fixture steps** use the 4b accessor: the homebind read's
  `#[cfg(test)]` handle-less branch reads
  `host.core().player_handle_like_cpp.is_none() && let Some(fixtures) = host.test_fixtures_ref_like_cpp()`,
  and the dismiss-pet `#[cfg(test)]` block writes through
  `if let Some(fixtures) = host.test_fixtures_like_cpp()`, both under
  `cfg(any(test, feature = "test-fixtures"))`. In a production build they
  are compiled out; in the `test-fixtures` build the accessor is `None`.

A token comparison of the 66 moved functions against the same rewrite
applied by script to their base bodies finds 53 identical; the other 13
differ only by `core_mut` (8), the two fixture steps and the two
execute-log split accessors. The two private bit helpers, the four
constants and the wild summon contracts are byte-identical apart from
`pub(crate)` → `pub` on the contracts.

### Re-measure and the cut

On `923fd0942` (classifier, `wow-world` production 56,283) the planned
families measured: combat 2,113 (`effect_combat.rs` 659, its damage child
524, its healing child 403, `combat.rs` 102, `threat.rs` 425) and the rest
2,608, **4,721 in all, above 4.5k, so threat (425) waits for 9d** as the
plan says. The bodies are then cut by host cost and by the ratchets their
call sites touch, not by family size:

- **Stay for 9d (execution or World-only surface).** The creature spell
  damage with its kill cascade (`apply_damage_from_caster_like_cpp`, 293
  lines: loot, XP, reputation, quest credit and the kill hooks, about ten
  host methods; it becomes one async host method instead), the server
  triggered cast (an execution entry), the implicit-target nearby check
  (the World player-condition context), the viewpoint packet and the
  farsight seer consumer (the World visibility state), force deselect
  (a `#[cfg(test)]` write to the social state), the reputation effect
  (`wow_world_social`, which spell must not reach), the proficiency effect
  (a `#[cfg(test)]` World capability fallback), the quest-complete effect
  (six quest host methods), the stuck effect (six host methods and a
  fixture), the self-resurrect effect (a `cfg(test)`/`cfg(not(test))`
  outcome pair over the combat fixtures) and the homebind persistence (the
  lifecycle port).
- **Stay because of a ratchet.** The taunt, kill-credit, spell-charge,
  raid-marker and transmog effects: their seven call-site lines in
  `execution.rs` (the free call adds a `self,` line to each vertical call)
  would push it from 1,000 to 1,007 lines (the 9b precedent). The summon
  slot effect and its two contracts: moving it would turn
  `MAX_GAMEOBJECT_SLOT_LIKE_CPP` into a test-only session-root re-export and
  grow the session hotspot's test figure, which may not rise. The homebind
  write (`set_represented_homebind_like_cpp`): its login-coordinator call
  site would grow `world_entry/login.rs` from 896 to 899, over its
  ceiling; it is a host method instead, with its fixture branch on the
  session.

### What moved

To `wow-world-spell/src/effect_session/` (C++ `Spell::Effect*` in
`SpellEffects.cpp` and the `Unit` helpers they call: `SpellDamageBonusDone`,
`SpellHealingBonusDone`/`Taken`, `HealBySpell`, `CalcHealAbsorb`,
`EnergizeBySpell`, `Player::InterruptPowerRegen`, `Spell::SendSpellExecuteLog`):

- `combat.rs`, the whole of `effect_combat.rs` except its child mount: the
  damage and healing bonus projections, the attribute, mechanic and school
  reads, the target aura-state, mechanic, health and creature-type reads and
  the two bit helpers; children `combat/damage.rs` (environmental damage and
  the spell-damage entry, with `DAMAGE_FIRE_LIKE_CPP`, only it reads) and
  `combat/healing.rs` (the whole of `healing_application.rs`: heal, heal from
  caster, the heal log, the heal absorb, max-health, percent and leech);
- `pve_combat.rs`, the whole of `combat.rs` (the PvE combat stop);
- `effects.rs`, from `effects.rs`: movie, heirloom, the disallowed mount
  form and transform check (with the three flag constants only they read),
  the object target position, the implicit target conditions, extra
  attacks, inebriate, dismiss pet, dual wield, titan grip, parry, block,
  give honor, modify cooldown, the dungeon-state reads, distract, sanctuary
  and instakill;
- `homebind.rs`: the homebind write wrapper (`set_homebind_like_cpp`), the
  homebind read and the bind effect (`effect_apply.rs`);
- `player.rs`, from `effects_player.rs`: the nearby visibility refresh
  request, the instanceable-map read and the honor XP;
- `power.rs`, from `effects_power.rs`: energize, the regen interrupt, the
  caster-scaled amount, the execute-log entry, its three records and send,
  the power-drain pre-scale and power drain/burn;
- `apply.rs`, from `effect_apply.rs`: teleport units and add farsight;
- `summon.rs` and `summon_contracts.rs`: the wild summon-object effect and
  its contracts;
- `mod.rs`: the host trait and the mounts.

World callers call the free function with `self` (54 sites in 14 files).
Inside `session/spell_effects` the 33 names are imported once in
`spell_effects/mod.rs`, so `execution.rs` reads `apply_x(self, …)`; it ends
at exactly 1,000 lines. **Recursion check.** Five trait impls forwarded a
moved name (`login_host.rs`: the visibility refresh request, the
instanceable read and the homebind read; `session_loot_host.rs`: the two
dungeon-state reads); each now names `wow_world_spell::…`. A grep of every
moved name with method or `WorldSession::`/`Self::` path syntax over the
production files (the `#![cfg(test)]` shims excluded) finds only those trait
definitions; a script checked that all ten `SpellEffectHostLikeCpp` adapter
forwards (and the earlier spell adapters') name a method of an inherent
`impl WorldSession` block; the production, `test-fixtures` and all-targets
builds report no `unconditional_recursion`.

**Reclassified, not moved:** the `#[cfg(test)]` `apply_damage` entry nested
in the production impl of `damage_and_combat_application.rs` (16 lines),
verbatim except that its call to the moved damage entry is a free call, to
the `#![cfg(test)]` `handlers/spell_effect_test_shims.rs`, with 10 one-call
forwards to the moved functions the scenarios and the melee test helper
call. No scenario call site changes.

**Tests that changed a path only:** `scenarios_misc_1.rs`,
`scenarios_spell_state_5.rs` and `scenarios_spell_state_6.rs` name
`wow_world_spell::ApplyEffectSummonObjectWildSessionStatusLikeCpp` (3 sites)
and `wow_world_spell::DAMAGE_FIRE_LIKE_CPP` (2 sites) instead of a new
session-root re-export; no line count changes and no assertion changes.

### Host surface

| Trait (spell) | Methods | Trait lines | Adapter | Adapter lines |
|---|---:|---:|---|---:|
| `SpellEffectHostLikeCpp: SpellClickHostLikeCpp` | 10: the heal and assisting threat forwards, the threatened-by-me owners and the owner threat scale (threat stays for 9d), the creature spell damage (async), `teleport_to` (async), the heirloom upgrade, the homebind write and persistence, the visibility combat reach | 51 | `session/spell_effects/effect_host.rs` | 98 |

- The supertrait lends the registry sync and the unit VALUES update object
  (9b), the aura host's `remove_aura` and the spellbook host's spell-state
  accessors; nothing is added to them. The two async methods return
  `impl Future + Send`; each adapter forward names the inherent method by
  path.
- **Budget.** 149 host lines (trait 51 + adapter 98) against 2,617 moved
  function lines: **5.7 %** (7.4 % with the whole `effect_session/mod.rs`).
- **Widening.** In spell the crate exports the trait and the 39 functions World production calls; 9 more names (the wild summon contracts, the
  fire-damage constant, the spell-damage entry, the wild summon wrapper,
  `apply_heal` and the three target-mask reads) only under
  `cfg(any(test, feature = "test-fixtures"))`. The moved functions only
  spell calls stay unexported; the private helpers stay private. No World
  item is widened.

### Await counts (per function, base vs head)

Counted with the 8d script (comments and literals blanked) over every
changed World file and the destination files.

| Function | Base | Head |
|---|---:|---:|
| `apply_health_leech_like_cpp` (moved) | 2 | 2 |
| `apply_damage_with_generator_like_cpp`, `apply_effect_teleport_units_like_cpp`, `apply_heal`, `apply_heal_max_health_like_cpp`, `apply_heal_pct_like_cpp`, `apply_instakill_like_cpp`, `apply_power_drain_effect_like_cpp` (moved) | 1 each | 1 each |
| `apply_heal_from_caster_like_cpp`, `apply_effect_bind_like_cpp` (moved, async) | 0 | 0 |
| 55 other moved functions | sync | sync |
| 33 async World functions in the changed files (`apply_damage_from_caster_like_cpp` 3, the execution entry and its children, the kill-credit and stuck effects, …) | 68 | 68 |
| `apply_damage` (test only; base nested, head in the shim) | 1 | 1 |
| the two shim forwards (`apply_heal`, `apply_heal_max_health_like_cpp`) and the two async adapter thunks | – | 1, 1, 0, 0 |

No future is spawned, boxed or reordered; every rewritten call site keeps
its `.await` in the same position, and no borrow is split differently
across an await (the moved bodies take `&mut H` exactly where they took
`&mut self`).

### Behaviour

Every moved body is its base text after the rewrites above. Unchanged in
particular: the damage and healing bonus math (attribute early-outs, school
mask, flat benefit, AP coefficient, the versus, aura-state, mechanic and
health-scaling multipliers, Ice Lance and Drain Soul), the heal path order
(taken bonus, heal absorb with one `SMSG_SPELL_HEAL_ABSORB_LOG` per shield
and the aura removal, `DealHeal`, `SMSG_SPELL_HEAL_LOG`, registry sync,
health update, heal threat), the environmental damage log, instakill's
`SMSG_SPELL_INSTAKILL_LOG` before the damage, energize (regen interrupt,
power, assisting threat, `SMSG_SPELL_ENERGIZE_LOG`), power drain/burn and
the execute-log accumulate/take/send order, the RNG draw order (no moved
body draws), the homebind write (canonical write, persistence, then
`SMSG_BIND_POINT_UPDATE` and the realm `SMSG_PLAYER_BOUND`) and the summon
and farsight map calls. No persistence code moved (#169): the homebind
persistence stays on the session as a host method. The opcode
registrations, processing and status flags are untouched
(`wow-world-application` is not in the diff). The moved log sites (the heal
skips and heal info lines, the teleport-units no-op and the visibility refresh info line) now use the
`wow_world_spell::effect_session` tracing targets.

### Edges and closure

No `Cargo.toml` change: every moved body reaches only core, entities,
lifecycle, data, map, packet and the crate itself. `wow_world_closure.py
check` passes in lock and metadata mode (17 packages); spell's direct
`wow-world` set stays core, entities and lifecycle. The five negative
probes (`wow-world-spell->wow-world-{quest,character,loot-ops,social,instances}`)
still fail as they must.

### Accounting

- **`crates/wow-world/src`.** Classifier: 56,283 → **53,691 production
  (−2,592)**. Physical: 63,094 → 60,630 (−2,464). Files: 482 → 482. Test:
  6,811 → 6,939 (+128).
  - **About 2,700 is production code that left `wow-world`:** the 66 moved
    functions (2,617 lines at base with their docs and attributes), the two
    bit helpers (14), the four constants, the wild contracts (26) and about
    40 separator and emptied-file lines.
  - **16 is reclassified to test:** the nested `#[cfg(test)]` `apply_damage`.
  - **+98 is the adapter**, +1 its mount.
  - **The rest (about +31 net) is call-site, import and comment edits:** the
    free calls in `execution.rs` (+17), `durability.rs` (+7) and
    `post_spell_go.rs` (+4), the `spell_effects/mod.rs` import block (+17),
    the session-root imports and constants (−7), `session_rules` (−5) and
    two one-line import drops.
  - The test figure grows by the shim file (125), its mount (+2) and the
    `session_rules` test-only re-export (+1).
- **Destination.** `wow-world-spell/src`: 11,600 → 14,672 physical
  (+3,072; production 11,323 → 14,385, +3,062; test +10, the gated exports).
  New `effect_session/` 3,042 lines: `combat.rs` 694, `effects.rs` 605,
  `power.rs` 492, `combat/healing.rs` 410, `summon.rs` 203, `apply.rs` 126,
  `pve_combat.rs` 104, `homebind.rs` 100, `player.rs` 100, `mod.rs` 96,
  `combat/damage.rs` 79, `summon_contracts.rs` 33; `lib.rs` +30.
- **`net_move.py check`.** PASS: growth 3,072 ≤ 3,549.2, 0 duplicate bodies.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`
  (the persistence snapshot is unchanged). The delta is exactly: 67 items
  leave `crate::session` (66 moved methods and `apply_damage`) with the two
  persistence-access rows of the visibility refresh request, and 23 arrive
  (11 shim items, 10 adapter trait items, the two impls) with those two rows
  under the free function. The syntax-only check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `session/mod.rs`: production 36,898 → 34,310, test 141,777
    (unchanged: the scenarios name the moved test names by path), total
    178,675 → 176,087;
  - physical `session/mod.rs` 962 → 955;
  - new physical row `handlers/spell_effect_test_shims.rs` at 125.
- **Warnings.** The warning sets (`--message-format=short` lines, compared
  without line numbers against the 9b head, whose tree is this base) are
  identical in all three builds for `wow-world` and every other crate in
  them: production lib 146, lib with `test-fixtures` 154, all targets 291.
  `wow-world-spell --all-targets` keeps its base set. `wow-world`
  `dead_code` is 0 in all three; `unconditional_recursion` is 0 in all three.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed; the workspace
  test-name multiset is identical (10,214 names, 5 ignores). Focused run
  during the move (the final run covers the head): the `wow-world` lib passed
  3,680 with 1 ignored.

### 9d plan

Execution, threat and the spell handlers, re-measured on the 9c head: the
execution entry points (`execution.rs` at 1,000 lines, its five children,
`execution_overloads.rs`, `destinations.rs`, `checks.rs`), threat (425),
`handlers/spell/**`, and the effect bodies 9c left: the creature spell
damage with its kill cascade, taunt, kill credit, spell charges, raid
marker, transmog, summon slot (with its contracts and
`MAX_GAMEOBJECT_SLOT_LIKE_CPP`), the server-triggered cast, the
implicit-target nearby check, the viewpoint and farsight seer consumer,
force deselect, reputation, proficiency, quest complete, stuck, self
resurrect, the homebind write and persistence, plus the 9b leftovers (the
click plan, the player cast, the respec wipe and
`apply_represented_aura_modifier_like_cpp`). With execution moving, the
call-site ratchet that held taunt and its neighbours back disappears; the
World-only surfaces (quest, social, visibility, persistence) stay host
methods. Highest surface; it splits again by host cost (about 15 %).

### Follow-ups

- Retire the shim files, now including `spell_effect_test_shims.rs` (125).
- The four `wow_world_spell::` test paths go back to bare names when the
  scenarios move with their bodies.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.
