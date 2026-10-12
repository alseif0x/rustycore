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
