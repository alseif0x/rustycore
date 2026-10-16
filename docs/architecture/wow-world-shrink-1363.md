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

A token comparison of the 68 moved bodies (the 66 methods and the two free
bit helpers) against the same rewrite applied by script to their base
bodies finds 55 identical; the other 13 differ only by `core_mut` (9
places), the two fixture steps and the two execute-log split accessors. The
two private bit helpers, the four constants and the wild summon contracts
are byte-identical apart from `pub(crate)` → `pub` on the contracts. *(9d
correction, #1393 review: the counts read "66 moved functions", "53
identical" and `core_mut` (8).)*

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
  - hotspot `session/mod.rs`: production 36,898 → 34,311, test 141,777
    (unchanged: the scenarios name the moved test names by path), total
    178,675 → 176,088;
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

## Slice 9d result (execution, threat and the execution-bound effects → spell)

Branch `1363-s9d-execution`, base `7e0180dae` (9c merged). Move-only (R2),
with the 9a–9c rewrites and nothing else:

- `self` becomes `host` (`fn name<H: SpellExecutionHostLikeCpp>(host: &mut H, …)`
  for the execution, `SpellEffectHostLikeCpp` for the effect bodies,
  `SpellbookHostLikeCpp` for the cast-state forwards, `AuraHostLikeCpp` for
  the flight flags); a call between moved functions, or to a function
  already in spell, becomes a free call; `self.core` becomes `host.core()`,
  or `host.core_mut()` where the body mutates through the field (6 places:
  `mutate_world_creature` in the assisting-threat body (2), the threat and
  threat-percent effects and the taunt, and the runtime RNG draw of the
  randomized destination fallback); `self.catalogs` becomes
  `host.hub_ref().catalogs`; `self.spell_state` becomes
  `host.split_spell_state_mut().0` for the execute-log clear;
  `crate::session::{hub_ref,hub_mut,split_*}(self)` become the same call
  through the host; World re-export paths name their source
  (`player_cast::wire::PlayerCastPublicationPhaseLikeCpp` → `crate::…`,
  `crate::creature_projection` → `wow_world_core::creature_projection`,
  `crate::creature_movement` → `wow_world_core::creature_movement`,
  `crate::session::game_time_ms_like_cpp` → `wow_world_core::session::…`);
  the private execution children keep `pub(super)`;
- **three one-line World wrappers are inlined**, each as its exact body,
  none async: `self.spell_store()` (body `self.catalogs.spell_store()`) at 5
  sites (the execution spell-info read, the completion read, the two
  assisting-threat reads and the taunt mask), `self.group_registry()` (body
  `self.core.group_registry()`) and `self.player_registry()` (body
  `self.core.player_registry()`), both in the raid-marker effect;
- **no fixture step moves**: every moved body is free of `cfg(test)`. The
  bodies that carry one (self resurrect, proficiency, force deselect, the
  farsight seer consumer, stuck) stay on the session as host methods.

A token comparison of the 37 moved bodies against the same rewrite applied
by script to their base bodies finds 23 identical; the other 14 differ only
by rustfmt (match arms that became blocks, wrapped generic parameters and
trailing commas), `pub(super)` on the six execution children, `core_mut`
(the 6 places above), the wire path and the execute-log split accessor.
The constant, the target-resolution struct and the slot contracts are
byte-identical apart from `pub(crate)` → `pub` on the contracts.

### Re-measure and the cut

On `7e0180dae` (classifier, `wow-world` production 53,691) the 9d families
measured 7.1k: execution with destinations and checks 2,387 (41 outside
methods, 79 sites), threat 425, the effect bodies 9c left 1,600, the player
cast with `spell_state/cast.rs` 948, the spell click 389 and
`handlers/spell` 1,336. That is above 4.5k, so the slice is cut by host
cost:

- **9d moves the execution and what it alone reaches**: the entry point and
  its five children, the production overload, the server-triggered cast,
  the implicit destinations, threat, and the effect bodies whose host cost
  is one method or none (taunt, transmog set, kill credit, spell charges,
  raid marker, summon slot). With execution moving, the 9c call-site
  ratchet that held the last five back is gone. The four cast-state split
  forwards (the go-phase publication, the frame and interrupted-frame
  publication, the power take) and the flight-flag update move with their
  caller instead of becoming host methods.
- **Host methods (World-only surfaces)**: the quest-complete effect (quest),
  reputation (`wow_world_social`), force deselect (a `cfg(test)` social
  write), the farsight seer consumer with the viewpoint packet
  (visibility), proficiency, stuck and self resurrect (each with a
  `cfg(test)` fixture or outcome pair), the durability, duel and learn-spell
  effects (items, social, acquisition), the creature aura and the aura with
  provenance (application context), the initial threat, the cast
  preparation checks (player cast, 9e), and the nearby-entry search with its
  implicit-condition check (the World player-condition context; moving it
  would add a `wow-conditions` edge and an associated context type). The
  creature spell damage with its kill cascade was already one.
- **Left for 9e+**: the player cast, `checks.rs`, the click plan,
  `handlers/spell`, the respec wipe and `apply_represented_aura_modifier_like_cpp`
  (plan below).

### What moved

To `wow-world-spell/src/execution_session/` (C++ `Spell::prepare`,
`Spell::_cast`, `Spell::SelectSpellTargets`, `Spell::SendSpellGo`,
`Spell::HandleEffects`, `Spell::finish` for the represented player cast):

- `execution.rs`, the whole of the World `execution.rs`: the entry point
  (978 lines at base, one function) and its creature-aura duration constant;
  it ends at 996 lines because its imports live in `mod.rs`;
- `completion.rs`, `post_spell_go.rs`, `preparation.rs`,
  `primary_effect_fallback.rs`, `target_resolution.rs`, the five World
  `execution/` children, now siblings so the entry point file stays under
  1,000 lines;
- `overloads.rs`: the production overload (from `execution_overloads.rs`)
  and the server-triggered cast (from `effects.rs`);
- `destinations.rs`: nine of the ten destination resolvers (focus, facing
  override, home, effective caster, transport, DB caster, nearby entry,
  the gameobject-summon fallback and the randomized fallback);
- `cast_state.rs`: the four `SessionSpellState` split forwards from
  `player_cast/{wire,publication,power}.rs`;
- `mod.rs`: the host trait and the mounts.

To `wow-world-spell/src/effect_session/`: `threat.rs` (the whole of
`threat.rs`: heal and assisting threat, the canonical sync, the threat and
threat-percent effects, the threatened-by-me owners and the owner scaling),
`taunt.rs`, `progress.rs` (transmog set, kill credit, spell charges with the
charge restore from `spell_state/catalog.rs`, raid marker), `summon_slot.rs`
and the slot contracts appended to `summon_contracts.rs` (the whole of
`session/summon_object_contracts.rs`). To `aura_session/publication.rs`:
the flight-flag update.

World callers call the free function with `self` (13 sites in 10 files).
`session/spell_effects/{execution.rs,execution/,execution_overloads.rs,threat.rs,effect_summon.rs,effects_power.rs}`,
`session/player_cast/power.rs` and `session/summon_object_contracts.rs` are
deleted; `player_cast/wire.rs` keeps only the phase re-export the cast
runtime and a scenario name.

**Recursion check.** One trait impl forwarded a moved name by path
(`login_host.rs`: the character crate's execution entry); it now names
`wow_world_spell::…`. The four threat forwards of the 9c adapter are gone
with their trait methods; the 9c bodies that called them make free calls.
A grep of every moved name with method or `WorldSession::`/`Self::` path
syntax over the production files (the `#![cfg(test)]` shims excluded)
finds only that trait definition line. A script checked that all 18
`SpellExecutionHostLikeCpp` and 11 `SpellEffectHostLikeCpp` forwards name an
ungated method of an inherent `impl WorldSession` block (258 forwards
across all adapters; the only exceptions are three pre-existing
`cfg(test)` battle-pet fixture forwards to shim methods in `pets/host.rs`).
The production, `test-fixtures` and all-targets builds report no
`unconditional_recursion`.

**Reclassified, not moved:** the five `#[cfg(test)]` overloads nested in the
production impl of `execution_overloads.rs` (101 lines; the classifier
counted them as production), verbatim except that their calls to the moved
entry points are free calls, to the `#![cfg(test)]`
`handlers/spell_execution_test_shims.rs`, with 9 one-call forwards to the
moved functions the scenarios and the `execution_overloads` test fixture
call. The shim also mounts that fixture
(`unit_tests/session/spell_effects/execution_overloads/test_only.rs`,
unchanged), whose parent module is gone. No scenario call site changes.

**Tests that changed a path only:** `scenarios_misc_1.rs` and
`scenarios_movement_1.rs` name `wow_world_spell::ApplyEffectSummonObjectSlotSessionStatusLikeCpp`
(2 sites) and `scenarios_misc_8.rs` names
`crate::entity_update_bridge::player_values_update_to_update_object` (3
sites) instead of a session-root import; no line count changes and no
assertion changes.

### Host surface

| Trait (spell) | Methods | Trait lines | Adapter | Adapter lines |
|---|---:|---:|---|---:|
| `SpellExecutionHostLikeCpp: SpellEffectHostLikeCpp + wow_world_inventory::BattlePetSessionHostLikeCpp` | 18: cast-execution mutation, cast preparation checks, forced visibility (async), farsight seer consumer, nearby-entry search, initial threat, creature aura, aura with provenance, the two durability effects, duel, learn spell (async), reputation, proficiency, quest complete (async), stuck (async), self resurrect, force deselect | 95 | `session/spell_effects/execution_host.rs` | 181 |
| `SpellEffectHostLikeCpp` (9c) | −4 threat forwards, +5: threat aura hydration, creature visible-set broadcast, transmog set, resolved group, creature kill (async) | 51 → 44 | `session/spell_effects/effect_host.rs` | 98 → 81 |

- The battle-pet supertrait lends the uncage, level and experience effects,
  which the execution calls in `wow_world_inventory` unchanged.
- **Budget.** 276 new host lines (trait 95 + adapter 181) against 2,895
  moved function lines: **9.5 %**; net of the 9c host that shrank (−7 trait,
  −17 adapter) 252 lines, **8.7 %**.
- **Widening.** In spell the crate exports the new trait and the 7
  functions World production calls (the two execution entries, the
  server-triggered cast, the go-phase publication, the frame publication,
  the threat sync and the threatened-by-me owners); 7 more names (the
  transport destination, the power take, the flight flags, the slot effect,
  the charge effect and the slot status and outcome) only under
  `cfg(any(test, feature = "test-fixtures"))`. The 27 effect exports only
  the World execution used are no longer exported; the three the 9c shims
  call (`apply_effect_add_farsight_like_cpp`, `apply_heal_max_health_like_cpp`,
  `implicit_target_conditions_like_cpp`) are test-fixture gated. No World
  item is widened.

### Await counts (per function, base vs head)

Counted with the 8d script (comments and literals blanked) over every
changed World and spell file.

| Function | Base | Head |
|---|---:|---:|
| `execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp` (moved) | 19 | 19 |
| `apply_post_spell_go_target_effects_like_cpp` (moved) | 5 | 5 |
| the production overload, the server-triggered cast, kill credit (moved) | 1 each | 1 each |
| 32 other moved functions | sync | sync |
| the five test overloads (base nested, head in the shim) | 1 each | 1 each |
| 25 other async functions in the changed files (the creature damage 3, the 9c heal and leech bodies, the cast ticks, the toy and self-res handlers, …) | 28 | 28 |
| the five new async adapter thunks (forced visibility, learn spell, quest complete, stuck, creature kill), the two 9c thunks and the seven `login_host` thunks, the execution forward among them | 0 | 0 |
| the shim forward of the production overload (test only) | – | 1 |

Base 60 awaits in 44 functions, head 61 in 50: the difference is the new
thunks (0 each) and the test-only forward (1). No future is spawned, boxed
or reordered; every rewritten call site keeps its `.await` in the same
position, and no borrow is split differently across an await (the moved
bodies take `&mut H` exactly where they took `&mut self`; the six
`core_mut` sites are synchronous closures).

### Behaviour

Every moved body is its base text after the rewrites above. Unchanged in
particular: the preparation order (cast checks, interrupted-frame
publication on failure, power take, cast-execution mutation); target
selection and the destination order (focus, home, DB, nearby entry,
transport, facing override, the randomized fallback drawing from the
session runtime RNG at the same point); `SMSG_SPELL_GO` (player frame or
direct packet, then the creature visible-set broadcast); the post-spell-go
target effects, the gameobject summon and the forced visibility; the
execute-log clear and the direct-effect dispatch order with every effect
arm, the aura arms with their speed recomputes and flight flags; the
primary fallback; completion (initial threat, cast-execution mutation,
execute log); the threat math (heal and assisting threat, threat
percent, taunt, owner scaling); the raid-marker transition and fanout; the
slot summon resolver. No persistence code moved (#169): the homebind
persistence and every World-only effect stay on the session. The opcode
registrations, processing and status flags are untouched
(`wow-world-application`, `session/registry.rs` and `world-server` are not
in the diff). The moved log sites (the execution info and debug lines, the
fallback, preparation, taunt, transmog and kill-credit debug lines) now use
the `wow_world_spell::{execution_session,effect_session}` tracing targets.

### Edges and closure

`wow-world-spell -> wow-world-inventory` is added for the three battle-pet
effects the execution calls; spell's `test-fixtures` enables
`wow-world-inventory/test-fixtures` and the dependency policy allows the
edge. It was simulated first (it was also in the slice 9 pre-check) and
checked as added: `wow_world_closure.py check` passes in lock and metadata
mode (17 packages); spell's direct `wow-world` set is core, entities,
inventory and lifecycle (closure adds interaction and visibility through
inventory). The five negative probes
(`wow-world-spell->wow-world-{quest,character,loot-ops,social,instances}`)
still fail as they must.

### Accounting

- **`crates/wow-world/src`.** Classifier: 53,691 → **50,696 production
  (−2,995)**. Physical: 60,630 → 57,867. Files: 482 → 472. Test: 6,939 →
  7,171 (+232).
  - **About 3,035 is production code that left `wow-world`:** the 37 moved
    functions (2,895 lines at base with their docs and attributes), the
    constant, the target-resolution struct, the slot contracts module (33)
    and the emptied files' headers, imports and impl scaffolding.
  - **101 is reclassified to test:** the five nested `#[cfg(test)]`
    overloads.
  - **+181 is the new adapter**; the 9c adapter shrinks by 17.
  - **The rest (about −23 net) is call-site, import, mount and doc edits:**
    the free calls in `aggro.rs` (+3), `collections/mod.rs`,
    `player_cast/state.rs`, `spell_click.rs` and the `effect_combat.rs` doc
    (+1 each), the `spell_effects/mod.rs` import block and mounts (−21), the
    session-root imports (−8) and the `spell_state` re-export (−1).
  - The test figure grows by the shim (231) and its mount (+2) and the two
    session-root lines that became `cfg(test)` (+2), less the
    `execution_overloads.rs` fixture mount (−3).
- **Destination.** `wow-world-spell/src`: 14,672 → 18,053 physical
  (+3,381; production 14,385 → 17,753, +3,368; test +13, the gated
  exports). New `execution_session/` 2,291 lines: `execution.rs` 996,
  `destinations.rs` 457, `mod.rs` 164, `post_spell_go.rs` 139,
  `target_resolution.rs` 120, `preparation.rs` 96,
  `primary_effect_fallback.rs` 94, `completion.rs` 88, `overloads.rs` 83,
  `cast_state.rs` 54; `effect_session/` gains `threat.rs` 442,
  `summon_slot.rs` 221, `progress.rs` 211, `taunt.rs` 163 and
  `summon_contracts.rs` +28; `aura_session/publication.rs` +9; `lib.rs` +7.
- **`net_move.py check`.** PASS: growth 3,381 ≤ 3,862.2, 0 duplicate bodies.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`
  (the persistence snapshot is unchanged). The delta is exactly: 49 items
  leave `crate::session` (37 moved methods, the 4 threat trait forwards, the
  5 test overloads and the 3 fixture methods of the remounted
  `execution_overloads` test file), and 40 arrive (14 shim items, 3 fixture
  methods under the shim, 23 adapter trait items) with 3 impls; the 5
  raid-marker registry rows move with their body to `wow-world-spell`. The
  syntax-only check passes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `session/mod.rs`: production 34,311 → 31,315, test 141,777 →
    141,687 (the remounted fixture left the tree), total 176,088 → 173,002;
  - physical `session/mod.rs` 955 → 949;
  - new physical row `handlers/spell_execution_test_shims.rs` at 231.
  The cohesion-review count stays at 71: the moved entry point file is 996
  lines.
- **Warnings.** The warning sets (`--message-format=short` lines, compared
  without line numbers against the 9c head, whose tree is this base) are
  identical in all three builds for `wow-world` and every other crate in
  them: production lib 146, lib with `test-fixtures` 154, all targets 291.
  `wow-world-spell --all-targets` keeps its base set (5). `wow-world`
  `dead_code` is 0 in all three; `unconditional_recursion` is 0 in all
  three.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed; the workspace
  test-name multiset is identical (10,214 names, 5 ignores). Focused run
  during the move (the final run covers the head): the `wow-world` lib
  passed 833 filtered tests (spell, threat, taunt, summon, execute, charge,
  raid, transmog, kill credit, destination, cast).

### #1393 follow-ups

1. **Done.** `effect_session/combat/healing.rs` logs "Skipping self heal…"
   and "Healed self" again.
2. **Done.** The 9c token-comparison note is corrected in place, marked as
   a 9d correction: 68 bodies (66 methods and the two free bit helpers),
   55 identical, `core_mut` at 9 places.

### 9e plan

Re-measured on this head (classifier): the player cast with
`spell_state/cast.rs` and `checks.rs` 1,115 (14 outside methods, 28 sites),
the click plan in `spell_state/spell_click.rs` 390 (6, 7),
`handlers/spell` 1,336 (28, 55), the respec wipe (about 93) and
`apply_represented_aura_modifier_like_cpp` (46), about 3.0k in all.

- **9e, the player cast and the handlers → spell:** the cast runtime
  adapter (`player_cast.rs` with the `Runtime` impl), the cast checks with
  `RepresentedMountSpellCheckOutcomeLikeCpp` (the battle-pet checks reach
  inventory through the new edge), the click plan and its execution (the
  player-condition context through an associated type, the inventory
  pattern), `handlers/spell`, the respec wipe with the talent-reset commit
  and publication as host methods, and the aura modifier (its 22 call sites
  are now in spell). It re-measures first and splits again by host cost
  (about 15 %) if needed.
- **Stay on the session (World-only surfaces):** the creature spell damage
  with its kill cascade, quest complete, reputation, force deselect, the
  farsight seer consumer, the nearby-entry search, the durability, duel and
  learn-spell effects, the homebind write and persistence. Proficiency,
  stuck and self resurrect stay until their `cfg(test)` fixture pairs have
  accessors.
- **Later:** the acquisition adapters and the coordinator's login spell
  application, with the coordinator (unchanged from the slice 9 plan).

### Follow-ups

- Retire the shim files, now including `spell_execution_test_shims.rs`
  (231), when the scenarios call the free functions; the
  `execution_overloads` test fixture then mounts from its owner again.
- Four 9b `aura_session` exports have no World production caller
  (`apply_represented_mounted_aura_like_cpp`,
  `apply_represented_provide_spell_focus_aura_like_cpp`,
  `represented_player_aura_state_mask_like_cpp`,
  `represented_unit_aura_state_mask_like_cpp`); gate or drop them with the
  shim retirement.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 9e result (player cast, cast checks, spell handlers and the spell-click plan → spell)

Branch `1363-s9e-player-cast`, base `c9d3222b3` (9d merged). Move-only (R2),
with the 9a–9d rewrites and nothing else:

- `self` becomes `host` (`fn name<H: Bound>(host: &mut H, …)`, `host: &H`
  for an `&self` method), with the narrowest host the body needs:
  `SpellExecutionHostLikeCpp` for the cast checks and the cast driver,
  `SessionHubHostLikeCpp` for the publication rail and the DisableMgr gate,
  and the new `PlayerCastHostLikeCpp` for the three handlers and the
  spell-click plan and execution; a call between moved functions, or to a
  function already in spell, becomes a free call; `self.core` becomes
  `host.core()`, or `host.core_mut()` where the body mutates through it (the
  two AI spell-click inform writes of the click execution:
  `mutate_world_creature` and `mutate_canonical_creature_by_guid_like_cpp`);
  `self.catalogs` becomes `host.hub_ref().catalogs`;
  `crate::session::{hub_ref,split_spell_state_*}(self)` become the same call
  through the host;
- World re-export paths name their source: `crate::session::SpellCastMetadata`
  → `wow_entities::SpellCastMetadata`, `crate::session::CreatureSpawnCatalogsLikeCpp`
  → `wow_world_entities::…`, `crate::session::ProgressionCatalogsLikeCpp` →
  `wow_world_core::session::…`, `crate::session::PlayerGridLoadResolverLikeCpp`
  → `wow_world_core::session::map_admission::…`; the cast handler's
  `&AreaTriggerCatalogsLikeCpp` becomes `&H::AreaTriggerCatalogsLikeCpp` (the
  movement host's associated type, which the World binds to the same type);
  `RepresentedPendingSpellCastRequestLikeCpp` keeps its name through a local
  `use wow_entities::PendingSpellCastRequestLikeCpp as …` (the World alias);
  the clickee caster is named `super::` from the click plan;
- **five World wrappers are inlined**, each as its exact body, none async:
  `self.spell_store()` (body `self.catalogs.spell_store()`; the cast handler
  and the seat plan, one site each), `self.player_condition_store()` (body
  `self.catalogs.player_condition_store()`; the plan) and `self.disable_mgr()`
  (body `self.catalogs.disable_mgr()`; the DisableMgr gate) become
  `host.hub_ref().catalogs.X()`; the two split wrappers
  `resolved_has_represented_aura_effect_with_misc_value_like_cpp` and
  `search_spell_focus_like_cpp` (one site each, the cast preparation check)
  become the same split and state call inside the `if` block that already
  scoped the call, the 8e form. No split is held across another call or an
  `.await`;
- **one fixture step uses the host accessor:** the ready-cast take's
  `#[cfg(test)]` handle-less branch is
  `#[cfg(any(test, feature = "test-fixtures"))] if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none()`,
  which is `Some` only in the World `cfg(test)` build, so it keeps its World
  meaning;
- **the aura modifier is a provided method** of `AuraHostLikeCpp` (body
  verbatim with `self.core` → `self.core()` and the three spell calls as
  in-crate calls, plus `where Self: Sized`), not a free function: its 23
  spell call sites (22 in `execution_session/execution.rs`, one in the
  mount aura) keep their `host.apply_represented_aura_modifier_like_cpp(…)`
  form. As free calls they would add a line each and push the 996-line
  execution entry point over the 1,000-line limit. `WorldSession` no longer
  overrides it (the adapter forward is gone);
- the battle-pet and mount checks and their outcome enum become private to
  `execution_session/checks.rs` (they were `pub(in crate::session)`; only the
  cast preparation check calls them).

A token comparison of the 13 moved functions against the same rewrite
applied by script (literals and comments masked, so no string literal is
touched) to their base bodies finds 5 identical; the other 8 differ only by
the inlined splits (cast preparation), `pub` → private (battle-pet and mount
checks), the fixture accessor (ready-cast take), the source paths and the
associated catalog type (the three handlers), and `super::`, `core_mut` (2)
and a rustfmt trailing comma (click execution). The aura modifier body is
compared by hand: identical apart from `self.core()` and the in-crate paths.
No string literal, constant, operator or control-flow token changed.

### Re-measure and the cut

On `c9d3222b3` (classifier, `wow-world` production 50,696) the 9e families
measured 3.0k as planned, but 960 of `handlers/spell` is not spell:
`handle_open_item`, the wrapped gifts, the item loot-template generation,
the addon loot loads and the loot-condition/quest-objective readers
(`ops_1.rs`/`ops_2.rs`) are the item loot family that `SpellHandler.cpp`
happens to host. They read the quest objective snapshot and the stored-loot
persistence, so they belong to `wow-world-loot-ops`, which spell must not
reach. The cut, by host cost:

- **9e moves** the cast preparation checks (`player_cast/checks.rs` and
  `spell_effects/checks.rs` with `RepresentedMountSpellCheckOutcomeLikeCpp`),
  the ready-cast take and the active-cast tick, the cast publication rail,
  the DisableMgr spell gate, `CMSG_CAST_SPELL`, `CMSG_SPELL_CLICK`,
  `CMSG_SELF_RES`, the spell-click plan (with the World player-condition
  context through an associated type, the 8b inventory pattern) and its
  execution, and the aura modifier.
- **Stays on the session, with the reason:**
  - the `player_cast::Runtime` impl (`session/player_cast.rs`, 277 lines with
    the interruption): it
    is already the World side of the request lifecycle's own host trait. Its
    `visual`, `install` and `start` carry `#[cfg(test)]` fixture branches and
    read the World config. Moving it needs a blanket
    `impl<H: …> Runtime for H`, a design change rather than a move;
  - the pending-request tick, the request replacement and the pending owner
    transition (`player_cast/state.rs`): the owner transition takes a
    `cfg(any(test, feature = "test-fixtures"))` fallback closure parameter,
    which a host-trait signature cannot carry without a feature-mismatch risk
    across crates. The tick is a host method of the cast handler;
  - the cast identity allocation (`player_cast/identity.rs`, a `cfg(test)`
    static counter) and the interruptions (they call `wow_world_application`,
    which depends on spell);
  - the first-login cast spells: they belong to the login coordinator (slice 9
    plan);
  - **the respec wipe (`CMSG_CONFIRM_RESPEC_WIPE`) is deferred.** It is 90
    lines over four host operations: the World-private
    `CommittedRepresentedTalentResetLikeCpp` (an associated type), the
    async commit and publication, and feign-death removal (application). On
    top of that, its `represented_can_confirm_respec_wipe_like_cpp` wrapper
    shares its name with the free helper it calls, so it would inline into
    the `if` condition as a block, and two of its constants are read by the
    talent scenarios. That is about 60 host lines for 90 moved, and it is
    talent-reset persistence orchestration. It goes with slice 13 (persistence
    and lifecycle orchestration), which owns the commit and publication;
  - the open-item loot family (above): with the loot-ops follow-up.

### What moved

To `wow-world-spell/src/execution_session/checks.rs` (C++ `Spell::CheckCast`
for the spell focus, the battle-pet effects and the mount spells,
`SpellInfo::CheckLocation`): `check_represented_cast_preparation_like_cpp`,
`check_represented_battle_pet_spell_like_cpp`,
`check_represented_mount_spell_like_cpp` and the outcome enum. The execution
preparation now calls the cast check as a free function.

To the new `wow-world-spell/src/player_cast_session/` (C++
`WorldSession::HandleCastSpellOpcode`, `HandleSpellClick`,
`HandleSelfResOpcode`, the Player cast driver and the cast publication):

- `driver.rs`: `take_ready_player_cast_like_cpp`,
  `tick_active_spell_cast_with_generator_like_cpp`;
- `publication.rs`: `handle_player_cast_publication_like_cpp`,
  `is_spell_disabled_for_player_like_cpp`;
- `handlers.rs`: `handle_cast_spell_with_catalogs_like_cpp`,
  `handle_spell_click_with_generator_like_cpp`,
  `handle_self_res_with_generator_like_cpp`;
- `mod.rs`: the host trait.

To `wow-world-spell/src/spell_click_session/plan.rs`:
`represented_handle_spell_click_plan_like_cpp`,
`represented_handle_spell_click_plan_with_seat_like_cpp`,
`execute_represented_spell_click_plan_with_generator_like_cpp`. To
`aura_session/mod.rs`: `apply_represented_aura_modifier_like_cpp` (the
provided method).

World callers call the free function with `self`: the driver tick, the
session command (publication), the pending tick, the `Runtime` impl
(`disabled`, `check_preconditions`), the three spell-handler adapter
forwards (`spell_handler_contexts.rs`) and the vehicle adapter forward
(seat plan). `session/player_cast/checks.rs`,
`session/spell_effects/checks.rs` and `session/spell_state/spell_click.rs`
are deleted.

**Recursion check.** A grep of every moved and inlined name with method or
`WorldSession::`/`Self::` path syntax over the production files (the
`#![cfg(test)]` shims excluded) finds only the four trait-definition lines of
the application and entities adapters, whose bodies now name
`wow_world_spell::…`. All six new forwards (`PlayerCastHostLikeCpp`: the
pending tick, the self-res check, the spell-click creature snapshot, the
player-condition context; `SpellExecutionHostLikeCpp`: the battle-pet journal
lock and the battle pet) name an ungated method of an inherent
`impl WorldSession` block. The aura modifier shim names the trait's provided
method by UFCS, and `WorldSession` does not override it. The production,
`test-fixtures` and all-targets builds report no `unconditional_recursion`.

**Reclassified, not moved:** the four `#[cfg(test)]` overloads nested in the
moved production impls (`tick_active_spell_cast`, `handle_cast_spell`,
`handle_self_res`, `execute_represented_spell_click_plan_like_cpp`, 53 lines
that the classifier counted as production), verbatim except that their calls
are free calls, and the `search_spell_focus_like_cpp` split wrapper (7), whose
only production caller moved and inlined it but which a scenario calls. All
of them move to the new `#![cfg(test)]` `handlers/player_cast_test_shims.rs`,
together with one-call forwards for the moved functions the scenarios call.
The two other inlined split wrappers (`resolved_has_…_with_misc_value`, 9
lines; `disable_mgr`, 4) had no caller left and are **retired**. The names
only the session scenarios read after the move (the spell-click values, the
reaction input, `SpellTargetData`, the battle-pet level cap, the spell-focus
object, the two `SPELL_CLICK_USER_*` constants) are re-exported from the
shim's `session_test_reexports` module, which the session root glob-imports
under `cfg(test)`. The two aura interrupt flags that were already
`cfg(test)` session-root imports moved there too, so the session tree's test
ceiling does not grow. No scenario call site changes.

### Host surface

| Trait (spell) | Methods | Trait lines | Adapter | Adapter lines |
|---|---:|---:|---|---:|
| `PlayerCastHostLikeCpp: SpellExecutionHostLikeCpp + player_cast::Runtime + wow_world_entities::PlayerMovementHostLikeCpp` | 1 associated type (player-condition context) + 4: pending-request tick (async), self-res spell check, spell-click creature snapshot, player-condition context | 23 | `session/player_cast/host.rs` | 35 |
| `SpellExecutionHostLikeCpp` (9d) | −1 (cast preparation checks), +2 (battle-pet journal lock, battle pet: both pass the World `cfg!(test)`) | 95 → 93 | `session/spell_effects/execution_host.rs` | 181 → 175 |
| `AuraHostLikeCpp` (9b) | the modifier becomes a provided method | — | `session/spell_state/aura_host.rs` | 85 → 68 |

- The supertraits lend the rest: the movement host for the embedded
  `MoveUpdate`, `Runtime` for `player_cast::request`, and the execution chain
  for the execution, the clickee caster and the cast identity.
- **Budget.** Gross new host lines: 58 (trait 23 + adapter 35) plus the two
  execution methods (5 trait + 9 adapter lines), 72 in all, against 1,093
  moved function lines: **6.6 %**. Net of what shrank (the cast-check
  declaration −7 and forward −15, the aura forward −17 and declaration −8),
  25 lines, **2.3 %**.
- **Widening.** In spell the crate exports the new trait and the 7 functions
  World production calls (the three handlers, the tick, the publication rail,
  the DisableMgr gate, the seat plan) and `check_represented_cast_preparation_like_cpp`
  (the `Runtime` adapter); 3 more (the ready-cast take, the plan without seat,
  the click execution) only under `cfg(any(test, feature = "test-fixtures"))`.
  `execute_represented_spell_click_clickee_caster_like_cpp` is no longer
  exported (its only World caller moved). **9d follow-up:** the four 9b
  `aura_session` exports without a World production caller
  (`apply_represented_mounted_aura_like_cpp`,
  `apply_represented_provide_spell_focus_aura_like_cpp`,
  `represented_player_aura_state_mask_like_cpp`,
  `represented_unit_aura_state_mask_like_cpp`) are now test-fixture gated; their
  in-crate callers import them from `aura_session` (same lines). No World
  item is widened.

### Await counts (per function, base vs head)

Counted with the 8d script (comments and literals blanked) over every
changed World and spell file.

| Function | Base | Head |
|---|---:|---:|
| `handle_cast_spell_with_catalogs_like_cpp` (moved; the movement update, the pending tick) | 2 | 2 |
| `handle_spell_click_with_generator_like_cpp`, `handle_self_res_with_generator_like_cpp` (moved) | 1 each | 1 each |
| `execute_represented_spell_click_plan_with_generator_like_cpp` (moved) | 2 | 2 |
| `tick_active_spell_cast_with_generator_like_cpp` (moved) | 1 | 1 |
| 8 other moved functions and the aura modifier | sync | sync |
| the four reclassified test overloads | 1 each | 1 each |
| the three spell-handler adapter thunks, the pending tick (stays) | 1 each | 1 each |
| the new pending-tick adapter forward | – | 0 |
| 25 other async functions in the changed files (the open-item family, the session commands, the vehicle handlers, …) | unchanged | unchanged |

Base 85 awaits in 42 functions, head 85 in 43: the difference is the new
forward (0). No future is spawned, boxed or reordered; every rewritten call
site keeps its `.await` in the same position, and no borrow is split
differently across an await (the moved bodies take `&mut H` exactly where
they took `&mut self`; the two `core_mut` sites are synchronous closures; the
two inlined splits are synchronous `&self` reads).

### Behaviour

Every moved body is its base text after the rewrites above. Unchanged in
particular: the `CMSG_CAST_SPELL` order (player, parse, spell-info gate before
the embedded `MoveUpdate`, the request, the immediate pending tick); the
request validation and result codes (`player_cast::request` over the same
`Runtime` impl); the cast-check order (spell focus aura, focus object search,
`SPELL_FAILED_REQUIRES_SPELL_FOCUS` with the focus id in FailedArg1, the
battle-pet checks in effect order with their result codes, the mount and
shapeshift-form capability, the water gate, the disallowed form with
`MOUNT_RESULT_SHAPESHIFTED` and `DONT_REPORT`); the ready-cast residence
fence and the execution with the `CastFailed` fallback; the publication
opcode and visibility gate; the DisableMgr reference; the spell-click user
types, the condition evaluation with its snapshots and player-condition
context, the vehicle control effect and the cast flags, and the execution
order with its skip counters and AI inform; `CMSG_SELF_RES` and the self-res
spell removal on success; the aura modifier slot, row and `SMSG_AURA_UPDATE`.
No persistence code moved (#169). The opcode registrations, processing and
status flags are untouched (`wow-world-application`, `session/registry.rs`
and `world-server` are not in the diff; the application registrar still
calls the same World adapter, whose forwards now name `wow_world_spell`).
The moved log sites keep their messages and fields; their tracing targets
are now `wow_world_spell::{execution_session,player_cast_session,spell_click_session}`.

### Edges and closure

`wow-world-spell -> wow-conditions` is added for the spell-click plan
(`ConditionUnitSnapshot`, `is_object_meeting_spell_click_conditions_like_cpp`,
`condition_meets_basic_like_cpp`). `wow-conditions` is a leaf `application`
crate (constants, data, entities, loot) that core, entities and inventory
already use, and the dependency policy now allows the edge. It was
simulated first with `wow_world_closure.py check --add wow-world-spell->wow-conditions`
in lock and metadata mode (both PASS), and passes as added (17 packages). No
`wow-world` edge is added. The five negative probes
(`wow-world-spell->wow-world-{quest,character,loot-ops,social,instances}`)
still fail as they must.

### Accounting

- **`crates/wow-world/src`.** Classifier: 50,696 → **49,482 production
  (−1,214)**. Physical: 57,867 → 56,805 (−1,062). Files: 472 → 471. Test:
  7,171 → 7,323 (+152).
  - **1,099 is production code that left `wow-world`:** the 13 moved
    functions and the aura modifier (1,093 lines at base with their docs and
    attributes) and the outcome enum (6).
  - **60 is reclassified to test:** the four nested `#[cfg(test)]` overloads
    (53) and the `search_spell_focus_like_cpp` wrapper (7).
  - **13 is retired:** the two inlined wrappers left without a caller.
  - **+12 is the adapter delta:** the new adapter (+35), the execution
    adapter (−6) and the aura adapter (−17).
  - **The rest (about −54) is file scaffolding, import, mount and call-site
    edits:** the three deleted files' headers and impl blocks, the
    session-root imports (−19), `handlers/spell.rs` imports (−3), and the
    free calls in the driver (+1) and the `Runtime` impl.
  - The test figure grows by the shim file (150) and the two
    `cfg(test)` mount and re-export lines (+2 each in `handlers/mod.rs` and
    `player_cast.rs`), less the two session-root lines that moved to the
    shim (−2).
- **Destination.** `wow-world-spell/src`: 18,053 → 19,387 physical
  (+1,334; production 17,753 → 19,077, +1,324; test +10, the gated exports).
  New `player_cast_session/` 496 lines (`handlers.rs` 249, `driver.rs` 103,
  `publication.rs` 85, `mod.rs` 59), `spell_click_session/plan.rs` 401,
  `execution_session/checks.rs` 362; `aura_session/mod.rs` +52, `lib.rs` +16,
  the execution module imports and call sites +0 net.
- **`net_move.py check`.** PASS: growth 1,334 ≤ 2,069.8, 0 duplicate bodies.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`
  (the persistence snapshot is unchanged). The delta is exactly: 23 items
  leave `crate::session` (the 14 moved methods, the 4 reclassified test
  overloads, the 3 inlined wrappers and the 2 trait forwards of the cast check
  and the aura modifier), and 17 arrive (10 shim items, 6 adapter trait
  methods and the associated type) with 2 impls. The
  syntax-only check passes.
- **Policy.** `dependency-policy.json` allows `wow-world-spell ->
  wow-conditions` (the edge above). No other policy file changes.
- **Ceilings.** None raised. Tightened to live values:
  - hotspot `session/mod.rs`: production 31,315 → 30,380, test 141,687 →
    141,685, total 173,002 → 172,065;
  - physical `session/mod.rs` 949 → 928, `handlers/spell.rs` 242 → 62;
  - new physical row `handlers/player_cast_test_shims.rs` at 150.
- **Warnings.** The warning sets (`--message-format=short` lines, compared
  without line numbers against the 9d head, whose tree is this base) are
  identical for `wow-world` in all three builds: production lib 146, lib with
  `test-fixtures` 154, all targets 291. `wow-world-spell` keeps its base set
  (lib 8, all targets 5). The other crates in those builds have no source
  change. `wow-world` `dead_code` is 0 in all three; `unconditional_recursion`
  is 0 in all three.
- **Tests.** No `#[test]` or `#[ignore]` is added or removed; the workspace
  test-name multiset is identical (10,214 names, 5 ignores). Focused run
  during the move (the final run covers the head): the `wow-world` lib passed
  905 filtered tests (spell, cast, click, self_res, aura, mount, battle_pet,
  publication, disable, focus).

### Follow-ups

- **The open-item loot family** (`handlers/spell/ops_{1,2}.rs`, about 960
  production lines) goes to `wow-world-loot-ops`, not spell: `CMSG_OPEN_ITEM`,
  the wrapped gift, the item loot templates and the loot-condition readers.
- **The respec wipe** goes with slice 13 (above).
- **The `Runtime` impl and the pending-request tick** move together once the
  pending owner transition's fixture fallback has an accessor (a blanket
  `Runtime` impl over a host trait is the design step).
- Retire the shim files, now including `player_cast_test_shims.rs` (150),
  when the scenarios call the free functions; `session_test_reexports` then
  goes too.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run.

## Slice 9 summary (9a–9e)

The spell domain was about 14k production lines on `b5fdc923b` (slice 9
plan). Slice 9 moved its bodies to `wow-world-spell`:

| Slice | `wow-world` production | Δ | Moved out | Adapter | Reclassified | Retired |
|---|---|---:|---:|---:|---:|---:|
| 9a spellbook | 58,403 → 57,424 | −979 | 791 | +42 | 233 | 0 |
| 9b talents, auras, aura sources, spell-click caster | 57,424 → 56,283 | −1,141 | about 1,290 | +134 | 12 | 0 |
| 9c effect bodies | 56,283 → 53,691 | −2,592 | about 2,700 | +99 | 16 | 0 |
| 9d execution, threat, execution-bound effects | 53,691 → 50,696 | −2,995 | about 3,035 | +164 | 101 | 0 |
| 9e player cast, checks, handlers, click plan | 50,696 → 49,482 | −1,214 | 1,099 | +12 | 60 | 13 |
| **Total** | **58,403 → 49,482** | **−8,921** | **about 8,915** | **about +451** | **422** | **13** |

The rest of each row is import, mount, scaffolding and call-site edits.
`wow-world-spell/src` grew from 9,076 to 19,387 physical lines (production
8,812 → 19,077) over the slice; every part passed `net_move.py` with no
duplicate body.

- **Moved vs reclassified.** About 8,915 production lines left `wow-world` as
  moved code. 422 left the production figure without moving: `cfg(test)`
  entry points nested in production impls and wrappers whose only production
  callers moved, now in ratcheted `#![cfg(test)]` shim files
  (`spellbook_test_shims.rs`, `talent_test_shims.rs`, `aura_test_shims.rs`,
  `spell_effect_test_shims.rs`, `spell_execution_test_shims.rs`,
  `player_cast_test_shims.rs`). 13 were retired. The World adapters added back
  about 451.
- **Edges.** `wow-world-spell` gained `wow-world-entities` (9b),
  `wow-world-inventory` (9d) and `wow-conditions` (9e); it still reaches none
  of quest, character, loot-ops, social or instances.
- **What spell code remains in `wow-world` (about 6.0k production lines in
  the spell directories, from 7.2k on the 9e base), and why:**
  - **the F5 adapters**, about 470 lines (`execution_host.rs` 175,
    `effect_host.rs` 81, `aura_host.rs` 68, `talent/host.rs` 50,
    `spellbook_host.rs` 41, `player_cast/host.rs` 35, `spell_click_host.rs`
    24): the World side of the host traits, which go only when `WorldSession`
    itself is split;
  - **the World-only effect surfaces** (about 1,150 lines in
    `spell_effects/{effect_combat/damage_and_combat_application,effects,effects_progress,effects_player,effect_apply,destinations}.rs`):
    the creature spell damage with its kill cascade, quest complete,
    reputation, force deselect, the farsight seer consumer, the nearby-entry
    search with the player-condition context, durability, duel and
    learn-spell, the homebind write and proficiency, stuck and self resurrect
    with their `cfg(test)` fixture pairs. Each reaches quest, social,
    visibility, items or acquisition, which spell must not reach, or carries a
    fixture pair without an accessor;
  - **the request-lifecycle `Runtime` impl, the pending tick and the cast
    identity** (about 550 lines in `session/player_cast*` besides its
    adapter): see the 9e follow-ups;
  - **the acquisition adapters** (`trainer_acquisition.rs` 657,
    `spell_acquisition/**` about 450, `effect_learning.rs` 104) and the
    login spell application: with the login coordinator (slice 9 plan);
  - **the aura application context and the spell-state wrappers**
    (`aura_application.rs` 241, `spell_state/{spell,spellbook,effects,catalog,spell_publication}.rs`
    about 680): the `wow_world_application` aura-application context (which
    depends on spell) and catalog setters and split wrappers the World still
    calls;
  - **the open-item loot family** (about 960 in `handlers/spell/ops_{1,2}.rs`):
    loot, for loot-ops;
  - **the respec wipe** (about 130 in `handlers/talent/state.rs`): slice 13.

## Slice 10 plan (world entities and gameobject state, phase 2)

Measured on `b240d01a6` (9e merged) before anything moved: the hotspot
classifier for production lines, and the slice 8/9 lexical scan for the calls
each family makes to inherent `WorldSession` methods (or `crate::session::`
splits and other crates' free functions with the session as argument) defined
outside the family. Field reads (`self.core`, `self.world_entities`,
`self.catalogs`, `self.config`) are not counted; they become hub or field
accessors.

- **`wow-world` production (classifier): 49,482** lines, 56,805 physical,
  471 files.
- **The world-entity domain left in `wow-world` is about 3.4k production
  lines**: `session/world_entities/**` (3,250) and the capture-point removal
  in `session/movement/movement_publication.rs` (37). The original row
  (about 4k) also counted the gameobject-use handlers, which 4a moved.
  `handlers/entities/**` (682) is now the F5 adapters (`host.rs`,
  `corpse/host.rs`) and `cfg(test)` shims; the adapters stay until
  `WorldSession` itself splits.

| Family | Files | Body lines | Outside calls | Host cost if moved to entities |
|---|---|---:|---:|---|
| GameObject update tick and linked-trap despawn (`GameObject::Update` states, traps, chests, capture points, despawn/respawn) | `gameobject.rs` tick, `spawn.rs` | 696 | 8 | +2 (loot-cache removal, chest refresh queue), −1 (the trap despawn host method retires) |
| Last-update gameobject publication (visual despawn, capture-point removal, the shared visible-set filter) | `gameobject_state.rs`, `gameobject_query.rs` summary, `movement_publication.rs` | 357 | 5 | +1 (`current_seer_guid_like_cpp`) |
| Visible-object queries (gameobjects with the canonical create data, creatures, the seer's combat reach) | `gameobject_query.rs`, `gameobject.rs` create data, `creature_query.rs`, `aggro.rs` reach | 323 | 4 | −2 (two visibility host methods retire); the quest crate's pure per-viewer GoState projection (16 lines) moves down with it |
| Creature publication and represented creature auras (message-to-set gates, aura apply/tick/publish, creature broadcast) | `creature.rs`, `creature_publication.rs` | 347 | 7 | 0 |
| Creature kill, reputation and death-state completion | `creature_kill.rs` | 439 | 16 | ~9 (social reputation gain, loot-ops kill loot, quest objective, XP, tapper pet hooks, currency store, social split) |
| Spell threat (initial threat, threat-relevant auras) | `aggro.rs` | 255 | 8 | spell-bound (two `wow_world_spell` calls and the pull publication): belongs to `wow-world-spell`, not entities |
| Spell-click visibility, viewer NPC flags | `spell_click.rs`, `creature_interaction.rs` | 120 | 4 | quest-bound (`wow_world_quest` predicate, condition projection): stays behind the host |
| Committed creature melee health presentation | `creature.rs` | 61 | 1 | 1, plus a fixture accessor for its `cfg(test)` block |
| Dynamic flags, kill credit, gameobject mutate | `gameobject.rs` | 73 | 5 | quest-bound; already host methods; stay |
| Catalog setters (`pub` API for world-server) | `creature_query.rs`, `gameobject_query.rs`, `aggro.rs` | ~60 | 0 | stay (session construction API) |
| `cfg(test)` registry and test entries nested in production impls | `creature_registry.rs`, `creature.rs`, `creature_kill.rs`, `aggro.rs` | ~375 | — | reclassification only (test shims), no host |

**Closure pre-check** (lock and metadata, on the base): the four 10a families
need **no new edge**; `check` passes as is (17 packages). The forbidden
targets were probed and fail as they must:
`wow-world-entities->wow-world-{quest,loot-ops,spell,social}` (cycles or
forbidden reach). `wow-world-entities->wow-world-loot` would pass, but 10a
uses one host method for the loot-cache removal instead of an edge (no
`Cargo.lock` change, so no extra recompilation in `final`).

**Split (each part re-measures on its own base; at most about 4.5k moved and
15 % host), lowest host surface first:**

- **10a → entities (about 1.7k, host net 0):** the four families with net
  host cost ≤ 2: the update tick and trap despawn, the last-update
  publication, the visible-object queries and the creature publication and
  auras. Also #1395 follow-up 1 (the session test re-exports move to a
  session-side `#![cfg(test)]` file).
- **10b (about 1.2k):** creature kill and reputation → entities behind about
  nine host methods (the 15 % budget holds only if the reputation branch and
  the death-state completion move together); the melee health presentation
  with a fixture accessor; spell threat → `wow-world-spell` (spell may reach
  entities; entities may not reach spell); and the `cfg(test)` registry
  reclassification. Spell-click visibility, dynamic flags and kill credit stay
  behind the host (quest-bound).

## Slice 10a result (gameobject update and publication, visible-object queries, creature publication → entities)

Branch `1363-s10a-world-entities`, base `b240d01a6` (9e merged). Move-only
(R2), with the slice 4–9 rewrites and nothing else:

- `self` becomes `host` with the narrowest bound the body needs:
  `WorldEntitiesHostLikeCpp` for the update tick, the linked-trap despawn,
  the creature publication and auras and the creature queries;
  `VisibilitySessionHostLikeCpp` for the last-update publication, the
  gameobject query and the seer's combat reach. `self.core` becomes
  `host.core()` (`core_mut()` where the body mutates: the client-visible GUID
  set and the canonical creature aura writes), `self.world_entities` becomes
  `world_entities()`/`world_entities_mut()`, `self.visibility` (two delivery
  admissions) becomes `visibility_mut()`, `self.config`/`self.catalogs` read
  through `host.hub_ref()`, `crate::session::{hub_ref,hub_mut,split_*}(self)`
  become the same host calls, a call between moved functions becomes a free
  call, `wow_world_entities::` becomes `crate::` and
  `crate::creature_projection` becomes `wow_world_core::creature_projection`
  (the same module the World re-exported);
- **two World wrappers are inlined**, each as its exact body, none async:
  `self.spell_store()` (body `self.catalogs.spell_store()`; the creature
  aura application) becomes `host.hub_ref().catalogs.spell_store()`, and
  `self.canonical_creature_access_like_cpp(guid)` (body: the
  `split_world_entities_ref` split and the state call; the creature
  broadcast) becomes that split and call in a block. The World wrapper stays
  for its other callers. The private `canonical_creature_aura_slot_like_cpp`
  split moves as a private free function;
- **the quest crate's pure per-viewer GoState projection**
  (`represented_gameobject_go_state_for_viewer_like_cpp`, 16 lines, no
  quest type) moves down to `visibility_session/queries.rs` with its only
  World caller; `wow-world-quest` imports it from entities and drops it from
  its three re-export lists.

A token comparison of the 19 moved functions against the same rewrite applied
by script to their base bodies (comments masked) finds 2 identical; the other
17 differ only by accessor parentheses, `_mut` accessors, free-call
argument order, `crate::`/`wow_world_core::` paths, the two inlined
wrappers, the three host calls (loot-cache removal, chest refresh, goober
refresh) and rustfmt trailing commas. No string literal, constant, operator or
control-flow token changed.

### What moved

- **`gameobject_update_session.rs` (692 lines):** the represented
  `GameObject::Update` tick (per-player state expiry, despawn delays,
  respawns, door/button and goober resets, charges, bomb and non-bomb traps,
  fishing bobbers, chest restocks, capture points and the
  `GO_JUST_DEACTIVATED` cleanup) and the linked-trap despawn.
- **`gameobject_publication_session.rs` (396):** the `SMSG_GAMEOBJECT_DESPAWN`
  visual despawn and the capture-point removal from the map's last update
  summary, with their shared visible-set filter. The emptied
  `session/movement/movement_publication.rs` is deleted.
- **`visibility_session/queries.rs` (363):** the nearby gameobject query with
  the canonical create data, the two nearby creature queries, the seer's
  combat reach and the GoState projection.
- **`creature_publication_session.rs` (361):** the two
  `SendMessageToSet` receiver gates, the creature visible-set broadcast and
  the creature aura apply, slot publication and tick.
- **Stays in `wow-world`, with the reason:** the quest-bound dynamic flags,
  kill credit and spell-click visibility; creature kill and reputation,
  spell threat and the melee health presentation (10b); the catalog setters
  (world-server construction API); the `cfg(test)` registry and test entries
  (10b reclassification).

### Host surface

- `WorldEntitiesHostLikeCpp` gains `remove_cached_loot_for_owner_like_cpp`
  (the loot state's method; its result was already discarded) and
  `queue_session_chest_gameobject_state_refresh_for_same_map_like_cpp` (the
  loot-ops facade), and loses `despawn_represented_linked_trap_by_guid_like_cpp`
  (its entities caller now makes the free call). The goober refresh in the
  tick uses the existing host method, whose adapter calls the same loot-ops
  function the body called.
- `VisibilitySessionHostLikeCpp` gains `current_seer_guid_like_cpp` (the World
  method, widened from `pub(in crate::session)` to `pub(crate)` for the
  adapter; it keeps its `cfg(test)` seer fixture) and loses
  `visible_world_creatures_from_map_like_cpp` and
  `visible_gameobjects_from_canonical_map_like_cpp`.
- New host code: 3 trait methods (about 9 lines) and 3 adapter entries
  (about 15 lines), against about 1,720 moved lines: **about 1.4 %**. Net, the
  host shrinks (3 methods retired, about 35 lines).
- No new crate edge: `wow_world_closure.py check` PASS in lock and metadata
  mode (17 packages); `Cargo.lock` is unchanged.

### Await counts and recursion

- None of the 19 moved functions is `async` or contains `.await` (base 0,
  head 0); the driver calls the tick, the capture-point removal and the visual
  despawn at the same points, in the same order.
- Every moved name was grepped across the production (non-shim) `wow-world`
  files. The six remaining references are trait-impl forwards and inherent
  callers, and every one names the free function
  (`wow_world_entities::…(self, …)`): the execution host (creature aura),
  the aura host (creature aura tick), the effect host (creature broadcast,
  combat reach), the loot host (message-to-set gate), and the combat-reach
  callers in `visibility/operations.rs` and `instances/map_resolution.rs` (3).
  The new forwards resolve to the inherent `current_seer_guid_like_cpp`, the
  `LootState` method and the loot-ops free function.
  `unconditional_recursion`: 0 in the production lib, the lib with
  `test-fixtures` and the lib test build.

### #1395 follow-up 1

`session_test_reexports` left `handlers/player_cast_test_shims.rs` for the
`#![cfg(test)]` `session/test_reexports.rs`, glob-imported by the session
root under `cfg(test)`; `session/mod.rs` no longer imports from `handlers`.
It also carries `RepresentedCapturePointStateLikeCpp`, whose last production
reader moved. Its 23 test lines are offset in the session hotspot by moving
the creature-publication F3 shim mount and the scripted-goober and per-type
gameobject-use test entries (whose bodies moved in 4a) to
`handlers/world_entities_test_shims.rs`; the session test ceiling falls.

### Accounting

- **`wow-world` production (classifier): 49,482 → 47,711 (−1,771)**;
  physical 56,805 → 55,155, 471 → 470 files.
  - **Moved out:** about 1,720 production lines (the 19 function bodies with
    their doc comments, and the emptied `movement_publication.rs`).
  - **Reclassified:** 35 (the `cfg(test)`-mounted `gameobject_use_scripted.rs`
    and `gameobject_use_types.rs`, which the classifier counted as production,
    now in the `#![cfg(test)]` shim).
  - The rest is imports, re-exports and the forward rewrites.
- **Destinations.** `wow-world-entities/src` +1,857 / −20 (the four new
  files 1,812, `lib.rs` +18); `wow-world-quest/src` +3 / −22.
- `net_move.py check`: S = 1,650, G = 1,819, allowance 2,693.5, PASS, 0
  duplicate bodies.
- **Ceilings** (none raised):
  - hotspot `session/mod.rs`: production 30,380 → 28,563, test 141,685 →
    141,677, total 172,065 → 170,240;
  - hotspot `handlers/character/mod.rs`: production 3,486 → 3,469, total
    19,141 → 19,124 (the visibility adapter);
  - hotspot `handlers/loot/mod.rs` unchanged (the forward's extra argument
    line is offset by merging the adapter's two `wow_world_loot_ops`
    imports);
  - physical `session/mod.rs` 928 → 927, `handlers/player_cast_test_shims.rs`
    150 → 128; new ratcheted row `handlers/world_entities_test_shims.rs` at
    125.
  - `session-ownership-policy.json` is reprinted with `print-baseline`; the
    delta is the moved items, the new adapter entries and the shims.
- **Warnings.** Per lint and crate, identical to the base in the production
  lib, the lib with `test-fixtures` and the lib test build (`wow-world`
  `dead_code` 0 in all three).
- **Tests.** No test file and no `#[test]`/`#[ignore]` line changed; the
  scenarios keep method syntax through the shim forwards.

### 10b plan

Re-measure on the 10a merge. In order: the `cfg(test)` registry and test
entries reclassification (about 375, no host); the melee health presentation
with a fixture accessor; creature kill, reputation and death-state
completion → entities behind about nine host methods, only if it stays
within 15 %; spell threat → `wow-world-spell`. Spell-click visibility, the
dynamic flags and kill credit stay behind the host (quest-bound).

### Follow-ups

- The visual despawn body repeats the visible-set filter of
  `visible_gameobject_guids_from_last_update_summary_like_cpp` (moved as is);
  a later behaviour-neutral change can make it call the shared filter.
- The world-entities shim retires with the other shims when the scenarios
  call the free functions directly.

## Slice 10b result (registry reclassification, melee health presentation, creature kill → entities, spell threat → spell)

Branch `1363-s10b-world-entities`, base `d1b2bbc77` (10a merged). Move-only
(R2), with the slice 4–10a rewrites and nothing else:

- `self` becomes `host` with the narrowest bound the body needs:
  `CombatSessionHostLikeCpp` for the melee health presentation,
  `WorldEntitiesHostLikeCpp` for the creature kill family,
  `SpellEffectHostLikeCpp` for the spell initial threat, `AuraHostLikeCpp`
  for the threat aura snapshot and `SessionHubHostLikeCpp` for the
  threat-relevant aura sync and hydration. `self.core` becomes `host.core()`
  (`core_mut()` where the body mutates: the five `mutate_world_creature`
  calls), `self.world_entities` becomes `host.world_entities_mut()`,
  `self.catalogs`/`self.config` read through `host.hub_ref()`,
  `crate::session::{hub_ref,hub_mut,split_world_entities_*}(self)` become the
  same host calls, `wow_world_spell::f(self, …)` becomes `crate::f(host, …)`,
  a call between moved functions becomes a free call, and the World alias
  `CanonicalThreatAuraSnapshotLikeCpp` names its source
  (`wow_entities::AuraThreatSnapshotLikeCpp`);
- **two World wrappers are inlined**, each as its exact body, none async:
  `self.currency_types_store()` (body `self.catalogs.currency_types_store()`;
  the kill reputation branch) and `self.spell_store()` (body
  `self.catalogs.spell_store()`; four sites in the initial threat). Both World
  wrappers stay for their other callers;
- **statements that become host calls:** the social split
  `{ let (s, h) = split_social_ref(self); s.represented_player_group_reward_state_like_cpp(h, guid) }`
  (two sites, the adapter repeats it), `self.loot.cached_loot_for_owner_like_cpp`,
  `wow_world_social::calculate_kill_reputation_gain_like_cpp(self, …)`,
  `wow_world_loot_ops::ensure_represented_creature_kill_loot_like_cpp(self, …)`,
  `self.give_xp`, `self.on_creature_killed_with_generator_like_cpp`,
  `self.publish_spell_pull_attack_start_like_cpp` and the
  `self.view.last_presented_creature_melee_health_state_revision_like_cpp`
  field (a `&mut u64` accessor);
- **the `#[cfg(test)]` steps use the host fixture accessors.** A
  `#[cfg(test)]` statement becomes
  `#[cfg(any(test, feature = "test-fixtures"))] if host.test_fixtures_ref_like_cpp().is_some() …`
  (the kill reputation record, the kill-hook call in the reward queue), the
  seven consecutive `#[cfg(test)]` records of the death-state completion
  become one such block in the same order, the melee presentation's
  handle-less fallback returns `None` unless the fixtures are present and its
  fixture write is `if let Some(fixtures) = host.test_fixtures_like_cpp()`,
  `#[cfg(not(test))]` becomes `#[cfg(not(any(test, feature = "test-fixtures")))]`,
  and the threat aura sync's two `cfg!(test)` arguments read one
  `world_test` local set from the same accessor. The accessors are `Some` only
  in the World `cfg(test)` build, so each step keeps its World meaning; in a
  World build with only `test-fixtures` they are `None`, as `cfg(test)` was
  false there. The two test-only kill-hook recorders (the kill hooks and the
  tapper-pet `KilledUnit` hook, which was a World test-shim method) move as
  `cfg(any(test, feature = "test-fixtures"))` functions.

A token comparison of the 12 moved functions against the same rewrite applied
by script to their base bodies (comments compared separately) finds 2
identical (the kill reputation award and the threat aura hydration); the other
10 differ only by the host calls and the fixture gates above, `core_mut`,
`pub` → private or `pub(crate)`, the high-water mark accessor, the type alias
and rustfmt trailing commas. No string literal, comment, constant, operator or
control-flow token changed.

### Re-measure and the cut

On `d1b2bbc77` (classifier, `wow-world` production 47,711) the 10b families
measured as planned: the `cfg(test)` registry and test entries 323
reclassifiable production lines (plan about 375, which also counted the
73-line test-only kill-hook recorder; that one moved with its caller), the melee health presentation 63 function
lines, the creature kill family 486 (plus the 30-line test-only tapper-pet
hook in `handlers/progression_test_shims.rs`) and the spell threat 258.
Everything fits one part: 807 moved function lines and 94 new host lines.

### What moved

- **Reclassified to `handlers/world_entities_registry_test_shims.rs`
  (`#![cfg(test)]`, new ratcheted row):** the five `#[cfg(test)]` creature
  registration and removal entry points (the whole of
  `session/world_entities/creature_registry.rs`), `talked_to_creature_like_cpp`,
  `on_creature_killed` and the two canonical threat readers (widened from
  `pub(in crate::session)` to `pub(crate)` because the file is outside the
  session tree), verbatim. The emptied `creature_registry.rs`, `spawn.rs`
  (#1396 follow-up 2), `gameobject_overrides.rs` and later `creature.rs` are
  deleted; the creature-registry F3 shim is mounted from the new shim file,
  the creature, gameobject-override and spawn F3 shims from
  `session/world_entities/mod.rs`. `mod test_reexports;` gains its outer
  `#[cfg(test)]` (#1396 follow-up 1), and the cfg(test) spell-area import
  joins `session/test_reexports.rs` to keep `session/mod.rs` within its
  ceiling.
- **`wow-world-entities/src/combat_session/commands.rs` (+63):** the committed
  creature melee health presentation, a private free function of its only
  caller, the creature melee-damage receiver.
- **`wow-world-entities/src/creature_kill_session.rs` (560):** the kill
  reputation group rate (C++ `KillRewarder`), the per-faction branch and the
  award (`Player::RewardReputation(Unit*, float)`), the deferred kill reward
  queue (loot, XP, reputation, quest kill credit, death state, VALUES
  publication), the death-state completion (`Unit::Kill`'s
  `setDeathState(JUST_DIED)` with the corpse loot flags) and the two test-only
  hook recorders.
- **`wow-world-spell/src/effect_session/initial_threat.rs` (291):**
  `Spell::HandleThreatSpells`' initial threat (with
  `ThreatManager::ForwardThreatForAssistingMe` on the positive path), the
  canonical threat aura snapshot, and the `SPELL_AURA_MOD_THREAT` aura sync
  (private) and hydration.
- **Stays in `wow-world`, with the reason:** the quest kill credit
  `on_creature_killed_with_generator_like_cpp` (quest-bound; now a host method
  of the kill queue); the spell-click visibility, the viewer NPC flags, the
  dynamic flags and gameobject kill credit (quest-bound); the catalog setters
  (`set_creature_aggro_config_like_cpp`, `set_spell_threat_store` and the
  query setters: world-server construction API); the split wrappers the
  scenarios and adapters still name. `session/world_entities/` is down to 383
  production lines (3,250 on the slice 10 base).

### Host surface

| Part | New trait | New adapter | Retired trait | Retired adapter | Moved function lines |
|---|---:|---:|---:|---:|---:|
| Melee presentation (`CombatSessionHostLikeCpp`: the presentation forward becomes the high-water mark accessor) | 3 | 5 | 4 | 6 | 63 |
| Creature kill (`WorldEntitiesHostLikeCpp` +6: group reward state, kill reputation gain, cached loot, kill loot, `give_xp`, quest kill credit; `LootSessionHostLikeCpp::give_xp` retires onto the supertrait method) | 24 | 46 | 6 | 8 | 486 |
| Spell threat (`SpellEffectHostLikeCpp` +pull publication −hydration; `AuraHostLikeCpp` +entities ref split −threat snapshot; `SpellExecutionHostLikeCpp` −initial threat) | 6 | 10 | 15 | 33 | 258 |
| **10b** | **33** | **61** | **25** | **47** | **807** |

- New host code is 94 lines against 807 moved function lines: **11.6 %**
  (creature kill alone 70 / 486, **14.4 %**; melee 12.7 %; threat 6.2 %). Net
  of the 72 retired lines it is +22, **2.7 %**.
- The kill family's adapters are the World operations the body called:
  the social split, `wow_world_social::calculate_kill_reputation_gain_like_cpp`,
  `LootState::cached_loot_for_owner_like_cpp`,
  `wow_world_loot_ops::ensure_represented_creature_kill_loot_like_cpp`, and
  the inherent `give_xp` and `on_creature_killed_with_generator_like_cpp`.
  `LootSessionHostLikeCpp` already had `give_xp` with the same signature;
  with the supertrait method its own copy became ambiguous, so it retires and
  the loot-ops gathering-node XP call resolves to the supertrait method, whose
  adapter forwards to the same inherent `WorldSession::give_xp`.

### Await counts and recursion

- Per function, base vs head: the reward queue 3 → 3 (kill loot, XP, quest
  kill credit, each through a host method that returns the World future
  unawaited); the other 11 moved functions 0 → 0. Their callers are
  unchanged: the driver's `process_pending_with_catalogs_like_cpp` 11 → 11,
  the cross-crate kill fixture 4 → 4, the World spell damage
  `apply_damage_from_caster_like_cpp` 3 → 3, the loot-ops gathering and
  chest openers 2/3 → 2/3. No borrow is held across an await differently.
- Every moved name was grepped across the production (non-shim) `wow-world`
  files. The remaining references are free calls
  (`wow_world_entities::…(self, …)` from the driver, the cross-crate fixture
  and the World spell damage; `wow_world_spell::hydrate_…(self)` from the
  World spell damage). Every new trait-impl forward resolves to an inherent
  method (`give_xp`, `on_creature_killed_with_generator_like_cpp`,
  `publish_spell_pull_attack_start_like_cpp`), a state method
  (`SessionSocialLimits`, `LootState`), a field, the
  `split_world_entities_ref` free function or another crate's free function.
  The retired forwards (`present_committed_…`, `apply_spell_initial_threat_…`,
  `canonical_threat_aura_snapshot_…`, `hydrate_…`) are deleted with their
  inherent targets, so no `WorldSession::name(self, …)` path can fall through
  to a trait method. `unconditional_recursion`: 0 in the production lib, the
  lib with `test-fixtures` and the lib test build.

### Behaviour

The kill reward queue keeps its order: pending kill loot (sorted, deduped),
then per reward XP, reputation rate, reputation (faction 1 then 2), quest kill
credit, the test hook record, the death-state completion and its VALUES
publication to a client that sees the creature. The death-state completion
keeps the lootable read, the skinning decision, the single `JUST_DIED`
transition and the record order. The threat math, the pull publication, the
canonical threat sync and the aura hydration order are token-identical; the
melee presentation keeps its revision high-water mark, canonical reread,
fixture mirror and registry sync. Packets and their order, persistence (#169),
registrations and processing flags are unchanged.

### Edges and closure

No new crate edge: `wow-world-entities` already depended on `wow-loot` (the
`CreatureLoot` type of the cached-loot method) and `wow-world-spell` on
`wow-world-entities`. `wow_world_closure.py check` PASS in lock and metadata
mode (17 packages); `Cargo.lock` is unchanged. The edges the moved bodies
would otherwise need were probed and fail as they must:
`wow-world-entities->wow-world-{spell,social,loot-ops,quest}` (cycles or
forbidden reach), so the social, loot-ops, XP and quest steps are host
methods.

### Accounting

- **`wow-world` production (classifier): 47,711 → 46,591 (−1,120)**;
  physical 55,155 → 54,369, 470 → 467 files.
  - **Reclassified, not production reduction:** 323 (unit 1: the nested
    `#[cfg(test)]` entry points, the `cfg(test)`-only registry file and the
    emptied files' headers).
  - **Moved:** 797 classifier lines (melee −75, creature kill −436, threat
    −286). Of the 807 moved function lines, 167 are test-only code the
    classifier counted as production (the nested `#[cfg(test)]` steps of the
    melee presentation, the reputation branch, the reward queue and the
    death-state completion, and the test-only kill-hook recorder), so the
    moved production proper is about 630 lines.
  - The rest is the adapters (+61 / −47 in `wow-world`), imports, shim
    forwards and call-site edits.
- **Destinations.** `wow-world-entities/src` +666 / −9 (production 18,496 →
  19,047, test 183 → 289: the two `cfg(any(test, feature = "test-fixtures"))`
  hook recorders); `wow-world-spell/src` +320 / −34 (production 19,077 →
  19,362); `wow-world-loot-ops/src` −6.
- `net_move.py check`: S = 786, G = 937, allowance 1,786.3, PASS, 0 duplicate
  bodies.
- **Ceilings** (none raised):
  - hotspot `session/mod.rs`: production 28,563 → 27,405, test 141,677 →
    141,663, total 170,240 → 169,068;
  - hotspot `handlers/loot/mod.rs`: production 1,386 → 1,378, total 20,315 →
    20,307 (the retired `give_xp` forward);
  - physical `session/mod.rs` 927 → 925, `handlers/progression_test_shims.rs`
    315 → 284; new ratcheted row
    `handlers/world_entities_registry_test_shims.rs` at 361.
  - `session-ownership-policy.json` is reprinted with `print-baseline`; the
    delta is the moved and retired items, the new adapter entries and the
    shims.
- **Warnings.** Per file and lint, the `wow-world` warnings are identical to
  the base in the production lib (146), the lib with `test-fixtures` (154) and
  the lib test build (291), and the `wow-world-entities`/`wow-world-spell`
  all-targets warnings are identical (10); `wow-world` `dead_code` 0 in all
  three. (A fresh compile of the base gives these counts; a replayed cached
  build can show fewer.)
- **Tests.** No test file and no `#[test]`/`#[ignore]` line changed; the
  test-name multiset is identical (10,214 test functions, 5 ignored, lexical).
  The scenarios keep method syntax through the shim forwards.

### Follow-ups

- The test shims (`world_entities_registry_test_shims.rs`,
  `world_entities_test_shims.rs`) retire when the scenarios call the free
  functions directly.
- `WorldEntitiesHostLikeCpp` and `SpellEffectHostLikeCpp` both carry
  `on_creature_killed_with_generator_like_cpp`; they can share one method
  when a common supertrait exists.
- The remaining `session/world_entities/` code is quest-bound (spell-click
  visibility, NPC flags, dynamic flags, gameobject kill credit) or the
  session construction API; it moves with the quest-bound hosts, not alone.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
on the PR head; the manifest, duration and test counts are recorded on the
PR.

## Slice 10 summary (10a–10b)

The world-entity domain was about 3.4k production lines in `wow-world` on
`b240d01a6` (slice 10 plan).

| Slice | `wow-world` production | Δ | Moved out | Reclassified | New host (gross / net) |
|---|---|---:|---:|---:|---|
| 10a gameobject update and publication, visible-object queries, creature publication → entities | 49,482 → 47,711 | −1,771 | about 1,720 | 35 | 24 / −11 |
| 10b registry reclassification, melee presentation, creature kill → entities, spell threat → spell | 47,711 → 46,591 | −1,120 | 797 (807 function lines, 167 of them test-only) | 323 | 94 / +22 |
| **Total** | **49,482 → 46,591** | **−2,891** | **about 2,520** | **358** | **118 / +11** |

- `session/world_entities/` went from 3,250 to 383 production lines;
  `session/movement/movement_publication.rs` is gone. What stays is
  quest-bound or construction API (above).
- `wow-world-entities/src` grew by about 2.5k physical lines (10a 1,857, 10b
  657) and `wow-world-spell/src` by 286; every part passed `net_move.py` with
  no duplicate body and no new crate edge.
- Both parts kept the host within budget: 1.4 % (10a) and 11.6 % (10b, 14.4 %
  for the creature kill alone), with 10a's and 10b's retirements making the
  net host growth about 11 lines for the whole slice.
- #1395 follow-up 1 (10a) and #1396 follow-ups 1 and 2 (10b) are done.

**Next.** Slice 11 (movement, teleport, instances, world state; about 2.9k
production lines in `session/{movement,instances,world_state}` plus the
teleport handlers) is the recommended next slice: its bodies already have the
entities movement host (`PlayerMovementHostLikeCpp`) and the instance and
visibility hosts to build on, and it does not touch the #169 persistence
order. Slice 13 (persistence and lifecycle orchestration, about 2.6k in
`session/{persistence,lifecycle}`) follows, with its own ordering review.
Slice 12 (base-module logic and the thunk sweep) goes last, because the
thunks it sweeps retire as the domains above leave.

## Slice 11 plan (movement, teleport, instances, world state, phase 2)

Measured on `24013c9ec` (10b merged) before anything moved: the hotspot
classifier for production lines and the slice 8–10 lexical scan for the calls
each family makes to `WorldSession` methods defined outside it (field reads of
`self.core`, `self.catalogs`, `self.config` become hub accessors and are not
counted).

- **`wow-world` production (classifier): 46,591** lines, 54,369 physical,
  467 files.
- **The slice 11 domain left in `wow-world` is 2,908 production lines**:
  `session/movement/**` 1,231 (`transfer.rs` 850, `far_transfer.rs` 125,
  `state.rs` and the spline stub 144, the `PlayerMovementHostLikeCpp`
  adapter `host.rs` 85, empty F3 mount stubs 27), `session/instances/**`
  1,113 (`map_key.rs` 514, `map_resolution.rs` 336, `difficulty.rs` 108,
  `binding.rs` 81, `instance.rs` 62, `mod.rs` and the fixture re-export 12)
  and `session/world_state/**` 564. The movement handlers and ACKs already
  left in 4b; the original row (about 3.5k) counted them.
- **Not in slice 11:** the world-port response (`handlers/travel/travel.rs`
  `handle_world_port_response_with_catalogs_like_cpp` and the self-create
  in `worldport_create.rs`, about 380) is the far-transfer *completion*
  (`HandleMoveWorldportAck`: AddPlayerToMap, the post-add publication and the
  #169 save fence) and goes with `session/lifecycle/transfer_*` to slice 13
  (lifecycle orchestration, with its own ordering review).

| Family | Lines | Outside calls | Destination and host cost |
|---|---:|---:|---|
| Canonical-map object queries (`map_resolution.rs`: transports, dynamic objects, area triggers, corpses/scene objects/conversations) and the dynamic-object VALUES publication from the last `Map::SendObjectUpdates` (`map_key.rs`), with the dynamic-object create-data helper (`object_updates.rs`) | about 500 | 3 (phase shift, seer GUID, visibility state: all already on the entities visibility host) | entities (`visibility_session`); **−4** host methods (the four `VisibilitySessionHostLikeCpp` query forwards retire) |
| Teleport and far transfer (`Player::TeleportTo`: the gates, the near teleport, the delayed teleport, the far-transfer detach and writer fence, `HandleMoveTeleportAck`'s near state machine, the delayed resurrection; `transfer.rs`, `far_transfer.rs`) | about 930 (+ about 45 `cfg(test)` reclassified) | 11 | entities (`teleport_session`) behind a new `PlayerTeleportHostLikeCpp: CombatSessionHostLikeCpp`: map disabled, cannot-enter (instances), known spells, the far-teleport spell interrupt and aura removal (spell), battleground leave, visibility detection sync, pending destination get/set, the active area trigger, logout flag, pet resummon; about 8 % |
| World state (area/zone update with the rest flags and hostility, area triggers and taverns, exploration XP) | 557 | 16 | `wow-world-instances` (its state already owns the tavern catalog check, the explored-zone fixtures and the zone-area authority); about 12 host methods (rest flags ×8, give_xp, phase shift ×2, view flags), the aura removal through a direct `instances → spell` edge (simulated, PASS) and `teleport_to` through the entities free function |
| Instance admission and create-map side effects (`player_cannot_enter_target_map`, the create-map player/lock contexts, side effects, owner GUID, the access requirement in `map_admission.rs`) | about 450 | 12 | `wow-world-instances`; group lookups through the social split the crate already reaches, the condition context through the host |
| Player-owner residence (`ensure_canonical_player_owner_for_map`), movement state leftovers (`state.rs`), the split wrappers (`instance.rs`, `binding.rs`, `difficulty.rs`) and the catalog setters | about 450 | — | the split wrappers and setters are session construction API and the F3/adapter surface; they stay until `WorldSession` itself splits, or retire when their last caller moves |

**Closure pre-check** (lock and metadata, on the base, `wow_world_closure.py
check --add …`): 11a needs **no new edge** (`check` passes as is, 17
packages). For 11b `wow-world-instances->wow-world-spell` passes in both
modes (instances already reaches spell through social). The forbidden
targets fail as they must: `wow-world-entities->wow-world-{instances,social,spell}`
and `wow-world-lifecycle->wow-world-instances` (cycles or forbidden reach),
so the instance, battleground and spell steps of the teleport body are host
methods.

**Split (each part re-measures on its own base; at most about 4.5k moved and
15 % host), lowest host surface first:**

- **11a → entities (about 1.4k, host net about +2 %):** the canonical-map
  queries and the dynamic-object publication (host net negative), then the
  teleport and far-transfer family; also the #1397 nits (the
  `gameobject_overrides` F3 shim import, a stale `self` comment in
  `creature_kill_session.rs`).
- **11b → instances (about 1.0k):** world state and the instance admission
  with the create-map side effects, behind one instances host trait; the
  movement state leftovers where their callers have moved.

## Slice 11a result (canonical-map object queries, dynamic-object publication, teleport and far transfer → entities)

Branch `1363-s11a-movement`, base `24013c9ec` (10b merged). Move-only (R2),
with the slice 4–10 rewrites and nothing else:

- `self` becomes `host` with the narrowest bound the body needs:
  `VisibilitySessionHostLikeCpp` for the canonical-map queries and the
  dynamic-object publication, the new `PlayerTeleportHostLikeCpp` for the
  teleport family. `self.core` becomes `host.core()` (`core_mut()` only for the
  `SessionState::Transfer` write and `kick`), `self.catalogs` reads through
  `host.hub_ref()`, `crate::session::{hub_ref,hub_mut}(self)` become the same
  host calls, `self.visibility` becomes `host.visibility_mut()`,
  `crate::entity_update_bridge::f` and
  `crate::session::represented_dynamic_object_values_update_delivery_fingerprint_like_cpp`
  name their sources (`wow_world_core::entity_update_bridge::f`,
  `wow_world_visibility::…`; both were re-exports of the same functions),
  `wow_world_entities::f(self, …)` becomes `crate::f(host, …)`, and a call
  between moved functions becomes a free call;
- **two World wrappers are inlined**, each as its exact body, neither async:
  `self.send_packet_realm(p)` (body `self.core.send_packet_realm(pkt)`; the
  TRANSFER_PENDING send of the far teleport and of the delayed far teleport)
  and `self.kick(r)` (body `self.core.kick(reason)`; the two writer-fence
  failures). Both World wrappers stay for their other callers;
  `self.player_guid()` and `self.send_packet(…)` resolve to the
  `SessionHubHostLikeCpp` provided methods, whose bodies are the inherent
  ones;
- **statements that become host calls:** the instance gates
  (`is_map_disabled_for_player_like_cpp`,
  `player_cannot_enter_target_map_like_cpp`),
  `wow_world_spell::known_spells_like_cpp(self)`,
  `self.interrupt_non_melee_spells_for_far_teleport_like_cpp()`,
  `wow_world_spell::remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp(self)`,
  `self.request_represented_battleground_leave_like_cpp()`,
  `self.sync_current_player_session_visibility_detection_like_cpp()`, the
  pending far destination get/set, `self.view.active_area_trigger` (a
  `&mut Option<u32>` accessor), `self.lifecycle.player_logout_like_cpp()` and
  `wow_world_lifecycle::resummon_pet_temporary_unsummoned_like_cpp(self)`;
  `sync_player_registry_state_like_cpp` and `update_zone_represented_like_cpp`
  were already `PlayerMovementHostLikeCpp` methods;
- **the `#[cfg(test)]` steps use the host fixture accessors.** The delayed
  resurrection record, the delayed-operations counter and the ACK event
  record become
  `#[cfg(any(test, feature = "test-fixtures"))] if let Some(fixtures) = host.test_fixtures_like_cpp() …`,
  the handle-less resurrection fallback takes the request through the same
  accessor, `cfg!(test)` in the schedule result becomes one `world_test` local
  read from `test_fixtures_ref_like_cpp`, and `#[cfg(not(test))]` becomes
  `#[cfg(not(any(test, feature = "test-fixtures")))]` (the ACK record's
  unused-argument tuple also names `host`, which only the fixture branch
  reads). The accessors are `Some` only in the World `cfg(test)` build, so each
  step keeps its World meaning.

A token comparison of the 13 teleport functions against the same rewrite
applied by script to their base bodies (comments compared separately) finds
the differences limited to the multi-line `self.core`/`self.catalogs` chains,
the fixture gates above, `pub(in crate::session)`/`pub(crate)` → `pub` or
`pub(super)`, and rustfmt trailing commas; the five visibility functions differ
only by the rewrites in the first item. No string literal (including the C2
capture root `"WorldSession::teleport_to_with_options::far_transfer_detach"`),
comment, constant, operator or control-flow token changed.

### Re-measure

On `24013c9ec` (classifier, `wow-world` production 46,591) the 11a families
measured as planned: the canonical-map queries 269 function lines, the
dynamic-object publication 189 and its create-data helper 21 (479), the
teleport family 933 (transfer.rs 819 in ten functions, far_transfer.rs 114 in
three) plus 11 lines of nested `#[cfg(test)]` readers. Everything fits one
part: 1,412 moved function lines and 70 new host lines.

### What moved

- **`wow-world-entities/src/visibility_session/canonical_objects.rs` (306):**
  the dynamic-object create data (from `session/object_updates.rs`, re-exported
  by the session root for the spell-effect caller) and the four canonical-map
  queries (`map_resolution.rs`): transports (`Map::SendInitTransports`'s
  whole-map scan), dynamic objects, area triggers and the
  corpse/scene-object/conversation triple. The visibility update calls them
  directly; three stay `pub(crate)`.
- **`visibility_session/dynamic_object_values.rs` (202):** the dynamic-object
  VALUES publication from the last map-owned `Map::SendObjectUpdates`
  snapshot (`map_key.rs`), called by the World driver as a free function.
- **`wow-world-entities/src/teleport_session/` (1,138: `mod.rs` 71 with the
  host trait and the eight teleport-only constants, `teleport.rs` 506,
  `far_transfer.rs` 147, `teleport_ack.rs` 414):** `Player::TeleportTo`
  (`teleport_to_with_options`: the terminal-recovery, catalog/coordinate,
  DisableMgr, battleground, expansion and death-knight gates, the vehicle
  exit, the same-map near teleport, `Map::PlayerCannotEnter`, the seamless
  gate, the delay, the combat/pvp/pet/dynamic-object/area-trigger/spell
  cleanup, TRANSFER_PENDING, the far detach with the C2 capture, the pending
  destination, the far-pending fence, `SessionState::Transfer`, the writer
  fence and SUSPEND_TOKEN), the pending save destination, the delayed teleport
  after the update, the delayed far teleport (`far_transfer.rs`), the near
  `HandleMoveTeleportAck` state machine with its event record, and the
  resurrection delayed across a teleport (schedule and process).
  `session/movement/far_transfer.rs` is deleted; `transfer.rs` keeps the
  public `teleport_to`, which the integration tests and the World callers use.
- **Reclassified to `handlers/teleport_test_shims.rs` (`#![cfg(test)]`, new
  ratcheted row, 67):** the two `#[cfg(test)]` readers nested in
  `transfer.rs` (`represented_delayed_resurrection_after_teleport_like_cpp`,
  `move_teleport_ack_events_like_cpp`) verbatim, with
  `player_resurrection_state_snapshot_like_cpp` widened from
  `pub(in crate::session)` to `pub(crate)` because the file is outside the
  session tree; one-call method forwards to the moved free functions the
  scenarios call; and the far-transfer F3 shim mount.
  `handlers/canonical_objects_test_shims.rs` (`#![cfg(test)]`, new ratcheted
  row, 35) forwards the two moved visibility functions the scenarios call. The
  teleport constants, `MoveTeleportAckActionLikeCpp`,
  `TELE_TO_SEAMLESS_LIKE_CPP` and the dynamic-object VALUES bridge, which only
  the scenarios still name, join `session/test_reexports.rs`.
- **Callers:** the driver (delayed teleport, dynamic-object publication),
  `transfer_completion.rs` (delayed resurrection), the #169 save
  (`save.rs`, both pending-destination reads), the movement handler adapter
  (teleport ACK) and the corpse adapter (resurrection schedule) call the free
  functions.
- **#1397 nits:** `unit_tests/session/world_entities/gameobject_overrides/f3_shims.rs`
  imports `super::*`; the `creature_kill_session.rs` hoist comment names
  `host.hub_ref()`.
- **Stays in `wow-world`, with the reason:** the world-port response
  (far-transfer completion, slice 13), world state and the instance admission
  and create-map side effects (11b), the movement state leftovers, the split
  wrappers and the catalog setters.

### Host surface

| Part | New trait | New adapter | Retired trait | Retired adapter | Moved function lines |
|---|---:|---:|---:|---:|---:|
| Canonical-map queries and dynamic-object publication (`VisibilitySessionHostLikeCpp` −4 query forwards) | 0 | 0 | 26 | 49 | 479 |
| Teleport and far transfer (`PlayerTeleportHostLikeCpp: CombatSessionHostLikeCpp`, 12 methods) | 21 | 49 | 0 | 0 | 933 |
| **11a** | **21** | **49** | **26** | **49** | **1,412** |

- New host code is 70 lines against 1,412 moved function lines: **5.0 %**
  (teleport alone 70 / 933, **7.5 %**). Net of the 75 retired lines it is −5.
- The teleport adapter (`session/movement/host.rs`) forwards to the inherent
  `WorldSession` operations, `wow_world_spell` free functions, the lifecycle
  free function, the view field and the lifecycle state.

### Await counts and recursion

- Per function, base vs head: `teleport_to_with_options` 1 → 1 (the writer
  fence), the delayed far teleport 1 → 1 (the writer fence), the delayed
  teleport after the update 1 → 1 (the delayed far teleport); the other 15
  moved functions 0 → 0. Callers unchanged: `teleport_to` 1 → 1, the driver's
  delayed-teleport step 1 → 1. No borrow is held across an await differently
  (the fence future borrows `host.core()` as it borrowed `self.core`).
- Every moved name was grepped across the production (non-shim) `wow-world`
  files: the remaining references are free calls
  (`wow_world_entities::…(self, …)` from the driver, the save,
  `transfer_completion.rs`, `teleport_to`, the movement and corpse adapters)
  and the session root's create-data re-export. The two adapters that
  forwarded through `WorldSession::handle_move_teleport_ack_like_cpp` and
  `WorldSession::schedule_represented_resurrection_after_teleport_like_cpp`
  now call the free functions, so no `WorldSession::name(self, …)` path can
  fall through to a trait method of the same name. Every new trait-impl
  forward resolves to an inherent method
  (`is_map_disabled_for_player_like_cpp`,
  `player_cannot_enter_target_map_like_cpp`,
  `interrupt_non_melee_spells_for_far_teleport_like_cpp`,
  `request_represented_battleground_leave_like_cpp`,
  `sync_current_player_session_visibility_detection_like_cpp`,
  `pending_teleport_like_cpp`, `set_pending_teleport_like_cpp`), a field, the
  lifecycle state or another crate's free function (`known_spells_like_cpp`
  names `wow_world_spell::` explicitly, because the World test build has a
  test-only inherent of that name). `unconditional_recursion`: 0 in the
  production lib, the lib with `test-fixtures` and the lib test build.

### Behaviour

The teleport keeps its gate order and its packets in order: the abort
packets, TRANSFER_PENDING on the realm channel, the transport server-time
reset, the far detach and C2 capture, the pending destination, the
far-pending fence, `SessionState::Transfer`, the writer fence, SUSPEND_TOKEN
with the movement counter; the near teleport's move-update fan-out before the
MOVE_TELEPORT to the mover; the ACK's state machine, zone update, PvP flag,
pet resummon and delayed resurrection. The pending save destination (#169)
reads the same far and near state at the same two points. The map queries
keep one manager lock each and the same filters; the dynamic-object
publication keeps its seer and shared-vision gates, its fingerprint admission
and its send order. Registrations and processing flags are unchanged.

### Edges and closure

No new crate edge: `Cargo.lock` and the manifests are unchanged.
`wow_world_closure.py check` PASS in lock and metadata mode (17 packages).
`wow-world-entities->wow-world-{instances,social,spell}` were probed in the
plan and fail as they must, so the instance, battleground and spell steps are
host methods.

### Accounting

- **`wow-world` production (classifier): 46,591 → 45,137 (−1,454)**;
  physical 54,369 → 53,029, 467 → 468 files (`far_transfer.rs` deleted, two
  shim files added).
  - **Reclassified, not production reduction:** 11 (the two `#[cfg(test)]`
    teleport readers).
  - **Moved:** 1,412 function lines (visibility 479, teleport 933; 44 of the
    teleport lines are former `#[cfg(test)]` steps the classifier counted as
    production, so the moved production proper is about 1,370) and the eight
    teleport-only constants.
  - The rest (about 31) is the adapters (−49 retired, +49 new), the deleted
    file's header and the imports.
- **Destination.** `wow-world-entities/src` +1,649 physical (production
  19,047 → 20,694, test 289 → 291).
- `net_move.py check`: S = 1,340, G = 1,649, allowance 2,368.0, PASS, 0
  duplicate bodies.
- **Ceilings** (none raised):
  - hotspot `session/mod.rs`: production 27,405 → 26,002, test 141,663 →
    141,658, total 169,068 → 167,660;
  - hotspot `handlers/character/mod.rs`: production 3,469 → 3,416, total
    19,124 → 19,071 (the four retired visibility forwards);
  - physical `session/mod.rs` 925 → 910; new ratcheted rows
    `handlers/teleport_test_shims.rs` at 67 and
    `handlers/canonical_objects_test_shims.rs` at 35.
  - `session-ownership-policy.json` is reprinted with `print-baseline`; the
    delta is the moved and retired items, the new adapter entries and the
    shims.
- **Warnings.** Per file and lint, the `wow-world` warnings are identical to
  the base in the production lib (146 → 145), the lib with `test-fixtures`
  (154 → 153) and the lib test build (291 → 290) except one: the pre-existing
  `unused variable: destination_map_id` of the far detach moved with its body
  to `teleport_session/teleport.rs`, so the `wow-world-entities`,
  `wow-world-spell` and `wow-world-instances` all-targets count goes 15 → 16
  with that same warning. `wow-world` `dead_code` 0 in all three.
- **Tests.** No `#[test]` changed; the test-name multiset is identical
  (10,214 test functions, lexical). The only test-file edit is the #1397 nit
  import. The scenarios keep method syntax through the shim forwards.

### 11b plan

On the 11a base: world state (`session/world_state/operations.rs`, 557) and
the instance admission and create-map side effects (`map_key.rs` 325
remaining, the access requirement in `map_admission.rs`) → `wow-world-instances`
behind one instances host trait, with the `instances → spell` edge for the
indoor/outdoor aura removal (simulated, PASS) and the area-trigger teleport
through `wow_world_entities::teleport_to_with_options`. Re-measure first; if
the rest-flag surface (eight methods) pushes world state over 15 %, the rest
flags move with it or world state waits for slice 13's rest/lifecycle step.
The movement state leftovers (`state.rs`, 131) follow their callers.

### Follow-ups

- The test shims (`teleport_test_shims.rs`, `canonical_objects_test_shims.rs`)
  retire when the scenarios call the free functions directly.
- `CorpseHostLikeCpp` and `PlayerTeleportHostLikeCpp` both carry
  `pending_teleport_like_cpp`; the corpse host can take the teleport host as
  supertrait and drop its copy (and its `schedule_…` forward) in a later
  slice.
- The pre-existing unused `destination_map_id` in the far detach is left for
  a behaviour-neutral cleanup outside a move-only slice.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
on the PR head; the manifest, duration and test counts are recorded on the
PR.

## Slice 11b result (world state and the instance admission → instances)

Branch `1363-s11b-world-state`, base `3e1190e65` (11a merged). Move-only
(R2), with the slice 4–11a rewrites and nothing else:

- `self` becomes `host` bound by the new
  `InstanceSessionHostLikeCpp: PlayerTeleportHostLikeCpp + AuraHostLikeCpp`.
  The supertraits already carry what the bodies call on other hosts: the
  phase shift, `can_see_phase_shift`, `give_xp` and the group GUID
  (`WorldEntitiesHostLikeCpp`), the FFA realm flag
  (`CombatSessionHostLikeCpp`), the active area trigger
  (`PlayerTeleportHostLikeCpp`), and the bounds that
  `wow_world_spell::remove_indoor_outdoor_auras_for_current_position_represented_like_cpp`,
  `wow_world_entities::set_represented_ffa_pvp_flag_like_cpp` and
  `wow_world_entities::teleport_to_with_options` take;
- `self.core` becomes `host.core()`, `self.catalogs` reads through
  `host.hub_ref().catalogs`, `crate::session::{hub_ref,hub_mut,split_instances_ref,split_instances_mut}(self)`
  become the same host calls, `self.instances` becomes
  `host.split_instances_ref().0` (`_mut().0` for the fixture records),
  `self.view.is_pvp_realm_like_cpp` / `is_ffa_pvp_realm_like_cpp` become the
  host methods, `self.view.active_area_trigger` becomes
  `*host.active_area_trigger_mut_like_cpp()`, and a call between moved
  functions becomes a free call;
- **the area-trigger catalogs** are read through
  `AreaTriggerCatalogAccessLikeCpp` (`catalogs.destinations` and
  `catalogs.taverns` become `destinations()` / `taverns()`), because the World
  `AreaTriggerCatalogsLikeCpp` also holds the `WorldSession`-typed script
  dispatcher and cannot leave `wow-world`;
- **wrappers inlined as their exact bodies** (none async except the
  teleport): `self.teleport_to(map, pos)` (body
  `wow_world_entities::teleport_to_with_options(self, map, pos, TELE_TO_NONE_LIKE_CPP).await`;
  the area-trigger teleport, one await before and after), and the eight World
  instance-split wrappers `let (state, hub) = split_instances_ref(self);
  state.f(hub, …)` at their 11 call sites:
  `create_map_db2_entries_like_cpp` (4), `create_map_difficulty_context_like_cpp`,
  `cannot_enter_existing_instance_lock_like_cpp`,
  `check_instance_count_probe_like_cpp`,
  `represented_player_difficulty_id_for_map_entry_like_cpp`,
  `represented_group_difficulty_id_for_map_entry_like_cpp`,
  `resolved_player_recent_instance_id_like_cpp` and (`split_instances_mut`)
  `set_represented_player_recent_instance_like_cpp`. Three of those wrappers
  had no other caller and retire; the two that only scenarios still call move
  verbatim to the shim file; the other three stay for their World callers;
- **the `#[cfg(test)]` steps use the host fixture accessors**: the four
  area/zone criteria records, the world-map overlay record and the
  handle-less explored-zones fallback become
  `#[cfg(any(test, feature = "test-fixtures"))] if host.test_fixtures_ref_like_cpp().is_some() …`,
  so each keeps its World test-build meaning.

A token comparison of the 19 moved functions against their base bodies finds
the differences limited to the rewrites above, the generic signatures and
`pub(crate)`/`pub(in crate::session)` → `pub`/`pub(super)`/private. No string
literal, comment, constant, operator or control-flow token changed.

### Re-measure and the cut

On `3e1190e65` (classifier, `wow-world` production 45,137) world state is 557
lines in `session/world_state/operations.rs`, of which 479 function lines
move (468 + the 22 lines of the two rest-flag helpers in
`session/rest_progression.rs` that only world state called); the instance
admission and create-map side effects are 279 lines of `map_key.rs`, 52 of
`binding.rs`/`instance.rs` and 20 of `map_admission.rs`. Everything fits one
part: 816 moved function lines and 102 new host lines.

The rest flags stay host methods: the six rest-flag operations cost 24 host
lines (2.9 % of the moved lines) and the whole host is 12.5 %, under the 15 %
budget, so neither moving the canonical rest-flag operations nor deferring
world state to slice 13 is needed.

### What moved

- **`wow-world-instances/src/instance_session/` (1,113 physical):**
  - `mod.rs` (102): the host trait, the area-trigger catalog trait and the
    constants that left the session root (`REST_FLAG_IN_{TAVERN,CITY,FACTION_AREA}`,
    `AREA_FLAG_{ENEMIES_PVP_FLAGGED,CONTESTED,COMBAT_ZONE}`, the five
    `TRANSFER_ABORT_*` codes the admission returns);
  - `zone_area.rs` (274): `Player::UpdateZone`/`UpdateArea` with the RestMgr
    area, city and tavern flags, the deferred rest-flag publication and the
    zone hostility update, plus `update_represented_rest_flag_like_cpp` and
    `set_represented_tavern_resting_like_cpp` from `rest_progression.rs`;
  - `area_triggers.rs` (186): `Player::IsInAreaTriggerRadius`, the tavern
    area-trigger transition and the movement-driven area-trigger check with
    its teleport;
  - `exploration.rs` (130): the explored-zones snapshot and
    `Player::CheckAreaExploreAndOutdoor` (the indoor/outdoor aura removal
    through `wow_world_spell`, the explored-zone bit, the values update and
    the exploration XP);
  - `admission.rs` (283): `Map::PlayerCannotEnter` for a dungeon target
    (`player_cannot_enter_target_map_like_cpp`, every gate and result code in
    order), the create-map player context, the create-map decision key, the
    active instance lock context, the lock instance-id side effect and the
    instance owner GUID;
  - `create_map.rs` (137): the create-map side effects (player and group
    recent instance, lock creation and instance-id update, battleground entry
    count) with `CreateMapSideEffectApplySummaryLikeCpp`.
- **`wow-world-core`:** `player_team_id_for_race_cpp` (6 lines) joins
  `player_team_for_race_cpp` in `state/hub_support.rs`, because both the
  moved player context and the World appearance code read it.
- **`wow-world-entities`:** `TELE_TO_NONE_LIKE_CPP` joins the other teleport
  options in `teleport_session/mod.rs`; the World root re-imports it.
- **Reclassified to `handlers/instance_session_test_shims.rs`
  (`#![cfg(test)]`, new ratcheted row, 107):** the two split wrappers only the
  scenarios call, verbatim, and one-call method forwards to the moved free
  functions the scenarios and F3 shims call (explored-zones snapshot, area and
  zone updates, tavern resting, create-map side effects, player context,
  active lock and owner GUID), so no scenario call site changes. The names
  only the scenarios read (the summary, the lock token, two rest flags, two
  area flags) join `session/test_reexports.rs`.
- **Callers:** the movement adapter (zone update, exploration, area triggers,
  `player_cannot_enter_target_map_like_cpp`), the login adapter (zone update
  without the rest packet), `HandleAreaTrigger` (`travel.rs`: radius, tavern),
  the tavern recheck (`catalogs/operations.rs`), the explored-zones load
  (`persistence/load.rs`) and the map entry (`lifecycle/map_entry.rs`: player
  context, active lock ×2, side effects) call the free functions.
- **Stays in `wow-world`, with the reason:** `dispatch_area_trigger_script_like_cpp`
  (the dispatcher is `Fn(&mut WorldSession, …)`);
  `access_requirement_abort_like_cpp` (its condition context borrows the
  `WorldSession`, and the slice 13 map entry calls it too; a host method
  here); the movement state leftovers (`state.rs`: the rest-flag removal is
  the host's forward target and the tavern recheck's, and the feign-death,
  moved-unit, at-login-flag, transport-position and visibility-source
  operations are forwards behind the vendor, talent, login and visibility
  adapters); the instance split wrappers with World callers, the setters and
  `set_player_zone_area_authority_complete_like_cpp` (construction API); the
  map entry and the world-port response (slice 13).

### Host surface

| Part | New trait | New adapter | Retired | Moved function lines |
|---|---:|---:|---:|---:|
| World state and instance admission (`InstanceSessionHostLikeCpp`, 13 methods; `AreaTriggerCatalogAccessLikeCpp`, 2) | 32 | 70 | 0 | 816 |

- New host code is 102 lines against 816 moved function lines: **12.5 %**.
  The six rest-flag methods are 24 of them (2.9 %); without them 9.6 %.
- The adapter (`session/instances/host.rs`) forwards to the inherent
  `WorldSession` operations, the instance split and the view field; the
  existing movement, teleport and login adapters now call the free functions.

### Await counts and recursion

- Per function, base vs head:
  `check_area_explore_and_outdoor_represented_with_catalogs_like_cpp` 1 → 1
  (`give_xp`), `check_area_triggers_with_catalogs_like_cpp` 1 → 1 (the
  teleport: `teleport_to` → its inlined body `teleport_to_with_options`, the
  same future); the other 17 moved functions 0 → 0. Callers unchanged: the
  test wrapper `check_area_explore_and_outdoor_represented_like_cpp` 1 → 1;
  the movement adapters return the free functions' futures as before. No
  borrow is held across an await differently (`give_xp` and the teleport
  borrow `host` exactly as they borrowed `self`).
- Every moved name was grepped across the production (non-shim) `wow-world`
  files: the remaining references are free calls
  (`wow_world_instances::…(self, …)`) from the callers listed above and the
  two `#[cfg(test)]` wrappers left in `world_state/operations.rs`. The four
  adapters that forwarded through `WorldSession::name(self, …)`
  (`update_zone_represented_like_cpp`,
  `check_area_explore_and_outdoor_represented_with_catalogs_like_cpp`,
  `check_area_triggers_with_catalogs_like_cpp`,
  `player_cannot_enter_target_map_like_cpp`, and the login adapter's
  `update_zone_represented_without_rest_update_packet_like_cpp`) now name the
  free function, so no path can fall through to a trait method of the same
  name. Every new trait-impl forward resolves to an unconditional inherent
  method (`set_represented_rest_flag_like_cpp`,
  `remove_represented_rest_flag_like_cpp`,
  `defer/end_player_rest_flag_sync_like_cpp`,
  `clear_player_deferred_rest_flag_update_like_cpp`,
  `send_represented_resting_player_flag_update_like_cpp`,
  `access_requirement_abort_like_cpp`,
  `current_player_is_in_raid_group_like_cpp`,
  `authoritative_group_membership_like_cpp`, `player_loading`), the instance
  split free functions or the view field; no UFCS call reaches a trait
  default. `unconditional_recursion`: 0 in the production lib, the lib with
  `test-fixtures` and the lib test build.

### Behaviour

Zone and area updates keep their order: the spell-hit authority
invalidation, the area/zone id writes, the deferred rest-flag sync, the area
update without its own packet, the hostility update, the city-rest rule, the
end of the deferred sync and the conditional resting packet with the deferred
clear, then the criteria records. The area-trigger check keeps its exit then
entry order, the tavern short-circuit and the teleport; the tavern transition
keeps the rest flag, the resting packet and the FFA toggle. Exploration keeps
the alive/taxi gates, the indoor/outdoor aura removal, the explored bit, the
values update, the XP grant and `SMSG_EXPLORATION_EXPERIENCE`.
`Map::PlayerCannotEnter` keeps every gate and result code in order (map not
allowed, difficulty, GM bypass, access requirement, need group, max players,
zone in combat, the instance lock deny reason, too many instances) and holds
the map manager lock over the same span. The create-map side effects keep
their order and summary counts. Persistence (#169) is untouched; registrations
and processing flags are unchanged.

### Edges and closure

One new crate edge, `wow-world-instances → wow-world-spell` (simulated in
the plan; `wow_world_closure.py check` PASS in lock and metadata mode, 17
packages): instances reaches neither character, quest, loot-ops, application
nor `wow-world`, and inventory, spell, entities and loot still reach neither
social nor instances. `wow-world-instances` also takes `wow-ai` (for
`max_level_for_expansion_like_cpp`, already a dependency of `wow-world-core`);
both are added to its `dependency-policy.json` allowlist.

### Accounting

- **`wow-world` production (classifier): 45,137 → 44,322 (−815)**; physical
  53,029 → 52,321, 468 → 470 files (the adapter and the shim file added).
  - **Moved:** 816 function lines in 19 functions, their 24 doc-comment
    lines, the 11-line summary struct, eleven constants and the 6-line
    team-id helper.
  - **Reclassified, not production reduction:** 12 (the two split wrappers
    in the shim file).
  - **Retired:** 21 (the three split wrappers whose only callers moved).
  - **Added:** the 70-line adapter, 17 lines of call-site rewrapping
    (`map_entry.rs` 14, `travel.rs` 2, `load.rs` 1) and the imports.
- **Destinations.** `wow-world-instances/src` +1,133 physical (production
  5,538 → 6,660, test 37 → 48), `wow-world-core/src` +9, `wow-world-entities/src` +1.
- `net_move.py check`: S = 708, G = 1,143, allowance 1,704.4, PASS, 0
  duplicate bodies.
- **Ceilings** (none raised):
  - hotspot `session/mod.rs`: production 26,002 → 25,185, test 141,658 →
    141,656, total 167,660 → 166,841. The six test-only imports the move
    leaves in the root join `session/test_reexports.rs`, and to keep the test
    ceiling the five battle-pet adapter singletons join the same file as one
    group and the four auction-contract singletons become one `cfg(test)`
    group (imports only);
  - physical `session/mod.rs` 910 → 887; new ratcheted row
    `handlers/instance_session_test_shims.rs` at 107.
  - `session-ownership-policy.json` is reprinted with `print-baseline`; the
    delta is the moved and retired items, the new adapter entries, the shims
    and the direct-registry rows that moved to `wow-world-instances`.
- **Warnings.** Per file, lint and full message, the warnings of the
  production lib, the lib with `test-fixtures`, the lib test build and the
  `wow-world-{core,entities,spell,instances}` all-targets build are identical
  to the base, except the pre-existing `unused variable: destination_map_id`
  of the far detach, which is gone (below). `wow-world` `dead_code` 0 in all
  three.
- **Separately, behaviour-neutral:** the far detach's
  `let destination_map_id = u16::try_from(new_map).unwrap_or(u16::MAX);`
  (unused since before 11a) is renamed `_destination_map_id`. The initializer
  has no side effect and the value is `Copy`, and the binding is kept, so
  evaluation and drop order are unchanged (#1398 follow-up).
- **Tests.** No test file changed; the test-name multiset is identical. The
  scenarios keep method syntax through the shim forwards.

### Follow-ups

- The shim files (`instance_session_test_shims.rs` and the 11a shims) retire
  when the scenarios call the free functions directly.
- The map entry (`session/lifecycle/map_entry.rs`) re-implements most of the
  `Map::PlayerCannotEnter` gates for the login and far-transfer entry; it now
  calls the moved player context, lock context and side effects, and moves
  with slice 13 (lifecycle orchestration) behind `InstanceSessionHostLikeCpp`.
- `access_requirement_abort_like_cpp` can follow once the player-condition
  context no longer borrows the `WorldSession`.
- `wow-world-instances` re-exports its rest-flag, area-flag and
  transfer-abort constants publicly for the World root and the scenarios;
  they can narrow as those readers move (as the nine teleport constants of
  11a, #1398 review nit).

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
on the PR head; the manifest, duration and test counts are recorded on the
PR.

## Slice 11 summary (11a–11b)

The slice 11 domain was 2,908 production lines in
`session/{movement,instances,world_state}` on `24013c9ec` (slice 11 plan).

| Slice | `wow-world` production | Δ | Moved out (function lines) | Reclassified | New host (gross / net) |
|---|---|---:|---:|---:|---|
| 11a canonical-map queries, dynamic-object publication, teleport and far transfer → entities | 46,591 → 45,137 | −1,454 | 1,412 | 11 | 70 / −5 |
| 11b world state and instance admission → instances | 45,137 → 44,322 | −815 | 816 | 12 | 102 / +102 |
| **Total** | **46,591 → 44,322** | **−2,269** | **2,228** | **23** | **172 / +97** |

- `session/world_state/` went from 564 to 88 production lines,
  `session/instances/` from 1,113 to 362 (655 after 11a; 70 of the 362 are
  the new adapter) and `session/movement/` from 1,231 to 323 (the public
  `teleport_to`, the adapters and the `state.rs` forwards). What stays is construction API, the World-typed
  area-trigger script dispatch, the access requirement and the slice 13 map
  entry and world-port response.
- `wow-world-entities/src` grew by about 1.65k physical lines (11a) and
  `wow-world-instances/src` by 1,133 (11b); both parts passed `net_move.py`
  with no duplicate body. 11a added no crate edge; 11b added
  `wow-world-instances → wow-world-spell` (closure PASS) and `→ wow-ai`.
- Host share: 5.0 % (11a, 7.5 % for the teleport alone) and 12.5 % (11b).
- #1398 follow-up (the unused `destination_map_id`) is done; its other
  follow-ups (the shims, the corpse/teleport host overlap, the public teleport
  constants) remain.

**Next.** Slice 13 (persistence and lifecycle orchestration) is the
recommended next slice: `session/{persistence,lifecycle}` (about 2.6k), the
world-port response (`handlers/travel/travel.rs` and `worldport_create.rs`,
about 380) and the map entry, which can take `InstanceSessionHostLikeCpp` for
its admission and side-effect steps now that those bodies live in
`wow-world-instances`. It needs its own ordering review of the #169 save
fence, `AddPlayerToMap` and the post-add publication, and should be measured
and split (13a the map entry and world-port response with the instance host,
13b the persistence orchestration) before anything moves. Slice 12
(base-module logic and the thunk sweep) stays last.

## Slice 13 plan (persistence and lifecycle orchestration, phase 2)

Measured on `d8e8caf5c` (11b merged) before anything moved: the hotspot
classifier for production lines and the slice 8–11 lexical scan for the calls
each family makes to `WorldSession` methods defined outside it (field reads of
`self.core`, `self.catalogs` become hub accessors and are not counted).

- **`wow-world` production (classifier): 44,322** lines, 52,321 physical,
  470 files.
- **The slice 13 domain left in `wow-world` is about 4.6k production lines**
  in six families. The original row (about 2.6k) counted only
  `session/{persistence,lifecycle}`; the world-port response (slice 11 plan),
  the map entry (slice 11b follow-up), the respec wipe (slice 9e) and the
  continue-login coordinator (slice 8d/8e) were added to it since.

| Family | Lines | Outside calls | Destination and host cost |
|---|---:|---:|---|
| Map entry (`session/lifecycle/map_entry.rs` 370: `MapManager::CreateMap` admission for login and the far-transfer entry, the synchronous attach; with `ensure_canonical_player_owner_for_map_like_cpp` from `instances/map_resolution.rs`, 37, and `create_map_decision_difficulty_id_like_cpp` from `map_admission.rs`, 9) | about 410 | 10 | `wow-world-instances` (`instance_session`): three calls are already `InstanceSessionHostLikeCpp` methods (access requirement, raid group, loading GUID), five are instance-split wrappers that inline as their bodies, the owner residence moves with it; **one** new host method (`ensure_canonical_player_owner_exists_like_cpp`, which constructs the initial Player and stays in the World) |
| World-port response and far-transfer completion (`HandleMoveWorldportAck`: `handlers/travel/travel.rs` 222 with the self-CREATE send, `travel/worldport_create.rs` 190 `Map::SendInitSelf`, `session/lifecycle/transfer_completion.rs` 144 and `transfer_recovery.rs` 55: the post-add phases, the native completion before disconnect, the homebind recovery) | about 610 | 15 + 9 + 4 + 3 | `wow-world-character` (`worldport_session`), beside the login's `Map::AddPlayerToMap`, its self CREATE and `SendInitialPacketsAfterAddToMap`, which it calls: the character login host already carries the quest log, the CREATE flags, the money, the lifecycle split, `set_state`, `send_stat_update` and the item-level scaling; the attach is the instances free function through a new **`wow-world-character → wow-world-instances`** edge; **one** new host method (the deferred save resume, 13b's, returned as the World future) |
| Persistence orchestration (`session/persistence/**` 1,144: load, load authority, plans, the save, the money/void-storage/talent-reset commits; `session/lifecycle/persistence{,/deferred,/prepared}.rs` 284: `Player::SaveToDB` participants, the prepared and deferred save) | about 1,430 | 23 + 4 | the #169 save fence and its participants drain quest objectives and loot completions, which only `wow-world`/`wow-world-application` may reach; candidate `wow-world-application` with **`application → loot-ops`** (simulated, PASS) or the lifecycle save host growing; re-measure in 13b |
| Session finalization, cleanup, logout and account lifecycle (`finalization.rs` 218, `cleanup.rs` 74, `account.rs` 227, `logout.rs` 22, `lifecycle_ops/operations.rs` 248) | about 790 | 17 + 4 + 3 + 9 | with 13b: the finalization ledger calls the native completion (13a), the save (13b), loot release and account saves |
| Respec wipe (`CMSG_CONFIRM_RESPEC_WIPE`, `handlers/talent/state.rs` about 95, with the talent-reset commit and publication in `persistence/commit.rs`) | about 95 + the commit | 6 | with 13b: it is the talent-reset persistence orchestration (slice 9e deferral) and moves with its commit |
| Continue-login coordinator (`handlers/character/world_entry/login.rs` 896) | 896 | 60 | last (13c), once 13b gives the save/load steps owners: 60 outside calls is still about 20 % host on its own |

**Closure pre-check** (lock and metadata, on the base,
`wow_world_closure.py check --add …`): `wow-world-character->wow-world-instances`
passes in both modes (character is not a low crate, instances does not reach
character); `wow-world-application->wow-world-loot-ops` and
`wow-world-application->wow-world-character` pass (application is top). The
forbidden targets fail as they must: `wow-world-instances->wow-world-character`
(only the top may reach character) and `wow-world-lifecycle->wow-world-instances`
(cycles through entities, inventory and spell), so the world-port response
cannot live in instances (it calls the character after-add burst) and the
completion cannot live in lifecycle.

**Split (each part re-measures on its own base; at most about 4.5k moved and
15 % host):**

- **13a → instances and character (about 1.0k, host about 5 %):** the map
  entry onto `InstanceSessionHostLikeCpp`, and the world-port response with the
  self CREATE, the far-transfer completion and the homebind recovery to
  `wow-world-character` (#169-sensitive: AddPlayerToMap, the post-add
  publication, the native completion before the deferred save).
- **13b → application (about 2.3k, re-measure):** the persistence
  orchestration (save fence, participants, load, commits), finalization,
  cleanup, logout, account lifecycle and the respec wipe with the talent-reset
  commit. If the host passes 15 %, the account lifecycle and finalization
  split into 13c.
- **13c:** the continue-login coordinator, once its callees have owners.

## Slice 13a result (map entry → instances, far-transfer completion → character)

Branch `1363-s13a-map-entry`, base `d8e8caf5c` (11b merged). Move-only (R2),
with the slice 4–11 rewrites and nothing else:

- `self` becomes `host`. The map entry is bound by `InstanceSessionHostLikeCpp`;
  the far-transfer completion by the new
  `WorldportHostLikeCpp: CharacterLoginHostLikeCpp + InstanceSessionHostLikeCpp + VisibilitySessionHostLikeCpp + SpellEffectHostLikeCpp + PetSessionHostLikeCpp`.
  The supertraits carry what the bodies already called on other hosts.
- Accessors: `self.core` becomes `host.core()`. It becomes `core_mut()` only
  for `send_time_sync` and the inlined `kick`. `self.catalogs` reads through
  `host.hub_ref().catalogs`. `crate::session::{hub_ref,hub_mut,split_lifecycle_ref,split_lifecycle_mut}(self)`
  become the same host calls.
- Call rewrites:
  - A call between moved functions becomes a free call.
  - `wow_world_instances::f(self, …)` becomes `f(host, …)` inside instances.
  - `wow_world_character::f(self, …)` becomes `crate::f(host, …)` inside
    character.
  - `crate::session::{SessionState,PlayerSaveOutcomeLikeCpp}` name their
    sources, `wow_world_core` and `wow_world_lifecycle`. They were re-exports.
  - `crate::handlers::character::default_display_id` becomes
    `crate::character_creation::default_display_id`. That is the same
    re-export of `wow_world_core::session::default_display_id`.
- **One disambiguation.** `pending_teleport_like_cpp` exists on both
  `CorpseHostLikeCpp` and `PlayerTeleportHostLikeCpp`, and both forward to the
  same inherent. The three call sites name the teleport host by UFCS:
  `PlayerTeleportHostLikeCpp::pending_teleport_like_cpp(host)`.
- **Wrappers inlined as their exact bodies.** Only one is async:
  - `self.teleport_to(map, pos)` becomes
    `wow_world_entities::teleport_to_with_options(host, map, pos, TELE_TO_NONE_LIKE_CPP).await`.
    This is the homebind recovery, which keeps one await.
  - `self.kick(r)` becomes `host.core_mut().kick(r)`, at 8 sites.
  - `self.battlenet_account_id()` becomes `host.core().battlenet_account_id()`.
  - `self.skill_store()` becomes `host.hub_ref().catalogs.skill_store()`.
  - Five instance split wrappers at 7 sites:
    `create_map_db2_entries_like_cpp` ×2, `create_map_difficulty_context_like_cpp`
    (inside the manager-lock closure, as before),
    `cannot_enter_existing_instance_lock_like_cpp`,
    `check_instance_count_like_cpp` and `add_instance_enter_time_like_cpp`.
  - `resolved_inventory_items_like_cpp` becomes the inventory split. The
    block is parenthesised because a `let … else` initializer cannot end in
    `}`.
  - `account_{toy,heirloom}_active_player_rows_like_cpp` become the lifecycle
    split, as in the login self CREATE.
- **What happened to the inlined wrappers:**
  - `cannot_enter_existing_instance_lock_like_cpp` and
    `check_instance_count_like_cpp` retire.
  - `add_instance_enter_time_like_cpp`, `create_map_difficulty_context_like_cpp`
    and the two account-row wrappers have only scenario callers left. They
    move verbatim to the shim file.
  - The other World wrappers stay for their callers.
- **Every other call keeps its form.** The hub-host provided
  `player_guid`/`send_packet`/`state` have the inherent bodies. The
  existing character, vendor, battle-pet, teleport and instance adapters
  forward to the inherent of the same name.

A token comparison of the 16 moved functions against their base bodies
(`/home/ubuntu/rustycore-wip/s13a/cmp.py`) finds four identical. The rest
differ only by the rewrites above, the generic signatures and the visibility
changes: `pub(crate)`/`pub(in crate::session)` became `pub` (re-exported),
`pub(super)` or private. No string literal changed: the `kick` reasons, the
`[FAR_TELEPORT]` messages and "worldport self CREATE …" are verbatim. No
comment, constant, operator or control-flow token changed either.

### Re-measure

On `d8e8caf5c` the 13a families measured as planned: 984 moved function
lines. That is the map entry 351, the owner residence 37, the decision
difficulty 9, the world-port response and self-CREATE send 222, the self
CREATE build 183, the post-add phases and native completion 134, and the
homebind recovery 48.

### What moved

- **`wow-world-instances/src/instance_session/map_entry.rs` (435 physical):**
  - `try_attach_worldport_destination_like_cpp` and its `_with_publication`
    form;
  - `ensure_canonical_world_map_for_current_player_like_cpp`;
  - `prepare_canonical_map_entry_like_cpp` and its `_with_publication` form,
    which is `MapManager::CreateMap` with every admission gate in order;
  - `ensure_canonical_player_owner_for_map_like_cpp`;
  - `create_map_decision_difficulty_id_like_cpp`.

  `InstanceSessionHostLikeCpp` gains `ensure_canonical_player_owner_exists_like_cpp`.
  It constructs the initial Player through the World's `initial_player_box_like_cpp`.
- **`wow-world-character/src/worldport_session/` (727 physical):**
  - `mod.rs` (52): the host trait;
  - `response.rs` (245): `HandleMoveWorldportAck` and the self-CREATE send;
  - `self_create.rs` (203): `Map::SendInitSelf`'s player CREATE;
  - `completion.rs` (156): the post-add begin/advance, the native completion
    before disconnect and the pending far entry;
  - `recovery.rs` (71): the ACK readiness, the homebind recovery and the
    terminal recovery.

  `handlers/travel/worldport_create.rs` and `session/lifecycle/{map_entry,transfer_recovery}.rs`
  are deleted. `transfer_completion.rs` keeps only its four save-fence
  regressions. Their `use super::*` became one braced import of the two names
  they read, so the line count is unchanged.
- **New edge `wow-world-character → wow-world-instances`.** It is in the
  manifest, `Cargo.lock`, `test-fixtures` forwarding and the
  `dependency-policy.json` allowlist.
- **What stays in the World as forwards:**
  - `WorldSession::handle_world_port_response_with_catalogs_like_cpp` (pub).
    The `wow-world/tests` integration tests call it as a method, and so does
    the `cfg(test)` `handle_world_port_response`.
  - `WorldSession::ensure_canonical_world_map_for_current_player_like_cpp`.
    The login coordinator calls it twice, and so does the `test-fixtures`
    cross-crate fixture. Rewriting the login call would wrap past the 896-line
    ceiling of `world_entry/login.rs`.

  Both name the free function explicitly.
- **Callers:**
  - The travel registrar context calls
    `wow_world_character::handle_world_port_response_with_catalogs_like_cpp`.
  - Finalization calls `wow_world_character::finish_worldport_native_before_disconnect_like_cpp`.
  - The entities adapter calls `wow_world_instances::ensure_canonical_world_map_for_current_player_like_cpp`.
- **Reclassified to `handlers/worldport_test_shims.rs`** (`#![cfg(test)]`,
  new ratcheted row, 84 lines). It holds one-call method forwards for the
  scenarios: attach, entry preparation, owner residence, recovery, native
  completion, post-add begin, and the self-CREATE send and build. It also
  holds the four wrappers above, verbatim.

  The four transfer-abort codes only the scenarios read
  (`MAX_PLAYERS`, `NEED_GROUP`, `TOO_MANY_INSTANCES`, `ZONE_IN_COMBAT`) join
  `session/test_reexports.rs`. Its header comment is rewrapped from six to
  four lines, so the session test lines do not grow.
- **Stays in `wow-world`, with the reason:**
  - the initial Player construction (host method);
  - the deferred save resume (13b persistence; host method returning the
    World future);
  - `pending_teleport_like_cpp`/`set_pending_teleport_like_cpp`, the teleport
    and corpse adapters' targets;
  - the access requirement (its condition context borrows the session).

### Host surface

| Part | New trait | New adapter | Retired | Moved function lines |
|---|---:|---:|---:|---:|
| Map entry (`InstanceSessionHostLikeCpp` +1) | 1 | 3 | 0 | 397 |
| Far-transfer completion (`WorldportHostLikeCpp`, 1 method) | 12 | 22 | 0 | 587 |
| **13a** | **13** | **25** | **0** | **984** |

- New host code is 38 lines against 984 moved function lines: **3.9 %**.
- With the two retained World forwards (travel 19, map entry 8) it is 65
  lines, **6.6 %**.

### Await table (per function, base vs head)

| Function | Base | Head | Awaited, in order |
|---|---:|---:|---|
| `handle_world_port_response_with_catalogs_like_cpp` | 6 | 6 | recovery (reject branch); self-CREATE send; nearby creatures; nearby gameobjects; `send_initial_packets_after_add_to_map`; deferred save resume (host future, unchanged) |
| `send_player_self_create_for_teleport_like_cpp` | 0 | 0 | — (async fn, awaited once by the caller) |
| `recover_rejected_worldport_like_cpp` | 1 | 1 | the homebind teleport (`teleport_to` → its body, the same `teleport_to_with_options` future) |
| map entry ×7, completion ×4, self-CREATE build, ACK readiness, terminal recovery | 0 | 0 | — (synchronous; no guard crosses an await) |
| travel registrar thunk | 1 | 1 | the response future |
| `cfg(test)` `handle_world_port_response` / integration tests | 1 | 1 (+1 forward layer) | the World forward, which awaits the same future |
| finalization `NativeTransfer` step | 0 | 0 | synchronous call |

- No future that was awaited inline is spawned, reordered or held across a
  different borrow. Each await borrows `host` as it borrowed `self`.
- The deferred-save host method returns `WorldSession`'s own future.

### Behaviour (#169)

The world-port response keeps its exact order. Each item is a packet or a
DB step:

1. the disconnecting/recovery gate, the far-pending semaphore and the pending
   destination;
2. the attach (`MapManager::CreateMap` admission and `AddPlayerToMap`
   residence) or the recovery;
3. the semaphore and destination clear;
4. `SMSG_RESUME_TOKEN` with the pre-reset movement counter;
5. the counter reset, `SMSG_MOVE_SET_ACTIVE_MOVER` and the time sync;
6. the self CREATE;
7. the post-add begin;
8. the nearby creatures and gameobjects (visibility);
9. `SendInitialPacketsAfterAddToMap`;
10. the native completion: post-add zone, item-level scaling, pet resummon,
    delayed resurrection;
11. the deferred save (the #169 save fence) and its outcome gate;
12. the final stat update and `LoggedIn`.

`SMSG_NEW_WORLD` is still sent at the suspend-token response, as before. The
map entry keeps every gate, result code and publish flag in order, and holds
the map manager lock over the same span. The logout/finalization native
completion runs at the same step. Registrations and processing flags are
unchanged.

### Recursion

- Every moved name was grepped across the production (non-shim) `wow-world`
  files. What remains is the two retained forwards, which name the free
  function explicitly, and free calls from the travel context, finalization
  and the entities adapter.
- No adapter forwards through `WorldSession::name(self, …)` to a moved name.
  The new forwards resolve to inherent methods:
  `ensure_canonical_player_owner_exists_like_cpp` and
  `resume_deferred_player_save_with_generator_like_cpp`.
- `unconditional_recursion`: 0 in the production lib, the lib with
  `test-fixtures`, the lib test build and the destination all-targets build.

### Edges and closure

One new edge, `wow-world-character → wow-world-instances` (simulated in the
plan). `wow_world_closure.py check` passes in lock and metadata mode (17
packages). Character's closure gains instances and social. Only
`wow-world`/`wow-world-application` reach character, and instances still
does not reach character.

### Accounting

- **`wow-world` production (classifier): 44,322 → 43,306 (−1,016).**
  Physical lines 52,321 → 51,391; files 470 → 469 (three deleted, the
  adapter and the shim added).
  - **Moved:** 984 function lines.
  - **Reclassified, not production reduction:** 20 (the four wrappers in the
    shim file).
  - **Retired:** 18 (two split wrappers).
  - **Added:** the adapters (25) and the two forwards (27). The rest is
    headers and imports.
- **Destinations:**
  - `wow-world-instances/src` +449 physical (`map_entry.rs` 435, mod/lib 14);
  - `wow-world-character/src` +738 (`worldport_session` 727, lib 11).
- `net_move.py check`: S = 930, G = 1,187, allowance 1,937.5, PASS, 0
  duplicate bodies.
- **Ceilings** (none raised):
  - hotspot `session/mod.rs`: production 25,185 → 24,562, test 141,656 →
    141,656, total 166,841 → 166,218;
  - physical `session/mod.rs` 887 → 882;
  - new ratcheted row `handlers/worldport_test_shims.rs` at 84;
  - `session-ownership-policy.json` reprinted with `print-baseline`. The
    delta is the moved and retired items, the new adapter entries and the
    shims.
- **Warnings.** Per crate, file, lint and message, these builds are
  identical to the base:
  - the production lib;
  - the lib with `test-fixtures`;
  - the lib test build (`--profile test`, base rebuilt from the same tree).

  The `wow-world-{core,entities,instances,character}` all-targets build is
  identical except that the pre-existing instances `set_player_difficulty_like_cpp`
  dead-code warning no longer appears. The cause is feature unification:
  character's `test-fixtures` now forwards `wow-world-instances/test-fixtures`
  in that combined command, which compiles the method's test-fixture caller.
  No code changed there. `wow-world` `dead_code` is 0 in all three builds.
- **Tests.** No test function changed and the test-name multiset is
  identical. The only test-module edit is the import line in
  `transfer_completion.rs`. The scenarios keep method syntax through the
  shim forwards. Focused run before final: 241 passed (map entry, travel,
  transfer, post-add, worldport, teleport, instance filters).

### 13b plan

On the 13a base, re-measure the persistence orchestration
(`session/persistence/**`, `session/lifecycle/persistence{,/deferred,/prepared}.rs`,
about 1.4k) with the deferred-save resume and the respec wipe with its
talent-reset commit. Candidate destination is `wow-world-application`, with
`application → loot-ops` and `application → character` (both simulated,
PASS) for the save participants. Finalization, cleanup, logout and the
account lifecycle (about 0.8k) go with it if the host stays under 15 %, or
else in 13c with the continue-login coordinator.

### Follow-ups

- The shim file retires when the scenarios call the free functions directly.
- The two retained World forwards retire with the login coordinator (13c)
  and when the integration tests name the character function.
- `CorpseHostLikeCpp` and `PlayerTeleportHostLikeCpp` both carry
  `pending_teleport_like_cpp`. This is the #1398 follow-up, now visible as
  the UFCS disambiguation here.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
on the PR head; the manifest, duration and test counts are recorded on the
PR.

## Slice 13b result (lifecycle orchestration and the respec wipe → application)

Branch `1363-s13b-persistence`, base `293055e87` (13a merged). Move-only (R2),
with the slice 4–13a rewrites and nothing else.

### Re-measure and the cut

The 13b families measure 2,318 production lines on the base (classifier). Of
those, only about 0.73k is orchestration that can leave the World:

- **Moved (715 function lines):** the finalization ledger executor
  (`finalize_session` 60, the step executor 119, two private helpers 8), the
  disconnect cleanup 27, the logout buyback clear and the account appearance
  and illusion saves 175, the deferred save resume 32, the periodic save entry
  19, and `CMSG_CONFIRM_RESPEC_WIPE` 61 with its trainer check 27, the
  talent-reset commit 108 and publication 79. The committed-reset record and
  the four respec constants move with them.
- **Stays, and why:**
  - the `PlayerSaveHostLikeCpp` adapter (`lifecycle/persistence.rs`, about
    130). It is the World's impl of a lifecycle trait, so it must live here.
    The save fence itself is `wow_world_lifecycle::player_save` already;
  - the money-fence constructors (`plans.rs`) and other split forwards. They
    borrow disjoint `WorldSession` fields; they are host adapters by nature;
  - `#[cfg(test)]` fixture code (`lifecycle/persistence/prepared.rs`, the
    fixture snapshot of `save.rs`, about 160);
  - the login hydration steps (`persistence/load.rs` 532, `load_authority.rs`
    72, `lifecycle_ops/operations.rs` 233) and the void-storage unlock commit.
    They are `Player::LoadFromDB` and collection steps, not orchestration.
    Their right owners are domain crates (inventory, instances, quest), and
    most are `CharacterLoginHostLikeCpp` methods the login coordinator calls.
    They go with 13c.

Moved lines are far below 4.5k and the host is about 6 %, so no split was
needed and finalization stayed in 13b.

### What moved

`wow-world-application/src/session_lifecycle/` (976 physical):

- `mod.rs` (63): `SessionLifecycleHostLikeCpp: WorldportHostLikeCpp + LootSessionHostLikeCpp + TalentHostLikeCpp + PlayerSaveHostLikeCpp + Send`
  with one method, `session_account_save_cx_like_cpp`. It returns
  `SessionAccountSaveCxLikeCpp`, which is the World `LifecycleCx` without the
  instance state these steps never read;
- `finalization.rs` (236): `finalize_session_with_generator_like_cpp` and
  `execute_finalization_step` with `finalization_result` and
  `finalization_identity_is_current`;
- `cleanup.rs` (63): `cleanup_shared_runtime_state`;
- `account_saves.rs` (191): `clear_buyback_on_logout`,
  `save_account_item_appearances_like_cpp` and
  `save_account_transmog_illusions_like_cpp`, as methods of the cx;
- `player_save.rs` (65): the deferred save resume and the periodic save
  entry;
- `talent_reset.rs` (358): the respec wipe, its trainer check, the
  talent-reset commit (with `_at`) and publication, and the committed record.

Rewrites, beyond `self` → `host`:

- **Field accessors.** `self.lifecycle` becomes `host.lifecycle_mut()`
  (`CharacterLoginHostLikeCpp`) in these `&mut` bodies, and `self.core`
  becomes `host.core()` (`core_mut()` only for the inlined `kick`). The one
  place a field-disjoint borrow was needed (`install_finalization` reads the
  Player handle) uses `host.split_lifecycle_mut()`. That is the same two
  fields, in the same order.
- **Split forwards** become their host forms:
  `crate::session::split_{lifecycle,inventory}_mut(self)` and
  `split_lifecycle_ref(self)` become `host.split_*()`, and
  `crate::session::hub_{ref,mut}(self)` becomes `host.hub_*()`.
  `crate::session::cx_lifecycle(self)` becomes
  `host.session_account_save_cx_like_cpp()`.
- **Paths.** `crate::FinalizationOutcome` and `crate::FinalizationDisposition`
  now name `wow_world_lifecycle`; they were re-exports.
- **Calls between moved functions** become free calls.
- **UFCS where two supertraits carry the same name.** Each impl forwards to
  the same `WorldSession` inherent or field:
  - `LootSessionHostLikeCpp::loot`/`loot_mut` (also on the character host);
  - `PlayerTeleportHostLikeCpp::sync_current_player_session_visibility_detection_like_cpp`
    (also on the character host);
  - `BattlePetPurchaseHostLikeCpp::resolved_player_money_like_cpp` and
    `::drain_represented_quest_objective_progress_with_generator_like_cpp`
    (also on the loot host);
  - `SpellbookHostLikeCpp::split_spell_state_mut` (also on the character
    host).

  The drain and money-fence impls return the inherent future unchanged.
- **`#[cfg(test)]` steps** run through the hub fixture accessors (slice 4b).
  `test_fixtures_ref_like_cpp().is_some()` is the World test build. It gates
  the deferred-save fixture return and the talent-reset COMMIT fixture seams.
  `test_fixtures_like_cpp()` records the respec request; that was
  `record_represented_confirm_respec_wipe_like_cpp`, whose production body was
  empty.

**Wrappers inlined as their exact bodies** (one-line or two-line):

- the six account split forwards (`mark_character_offline`,
  `mark_character_account_offline_like_cpp`,
  `mark_login_account_offline_on_disconnect_like_cpp`,
  `save_account_{mounts,toys,heirlooms}_like_cpp`);
- `unregister_from_player_registry` ×2;
- `set_player_logout_like_cpp` ×2 (the lifecycle flag, then the visibility
  sync);
- `restore_realm_channels` and `clear_inventory_items_and_objects_like_cpp` ×2;
- `represented_talent_reset_persistence_plan_like_cpp`;
- `send_buy_error` (`host.core().send_buy_error`);
- `kick` ×4 (`host.core_mut().kick`);
- `record_represented_confirm_respec_wipe_like_cpp`;
- the `represented_can_confirm_respec_wipe_like_cpp` method wrapper. The free
  function it called moved verbatim.

Of these, `clear_inventory_items_and_objects_like_cpp`,
`record_represented_confirm_respec_wipe_like_cpp` and the can-confirm wrapper
retire. The six account forwards, `unregister_from_player_registry`,
`set_player_logout_like_cpp`, `restore_realm_channels`, `send_buy_error` and
the talent-reset plan have only scenario callers left. They move verbatim to
the shim file.

A token comparison of the 15 moved functions against their base bodies
(`/home/ubuntu/rustycore-wip/s13b/cmp.py`) finds five identical. These are the
two finalization helpers, the two collection saves and the free trainer check.
The rest differ only by the rewrites above. No string literal, comment,
constant, operator or control-flow token changed. The kick reasons, the
warn messages and the `talent reset` operation tag are verbatim.

**World side:**

- the public `finalize_session_with_generator_like_cpp` and
  `cleanup_shared_runtime_state` stay as one-call forwards that name the free
  function (world-server, the integration tests, the character login adapter);
- the `WorldportHostLikeCpp` adapter returns the application resume future;
- the talent registrar adapter and the session driver call the free
  functions;
- the new adapter is `session/lifecycle/lifecycle_host.rs`;
- `handlers/session_lifecycle_test_shims.rs` (`#![cfg(test)]`, new ratcheted
  row, 205) holds the one-call forwards the scenarios use (finalization step,
  periodic save, talent-reset commit, respec wipe, the three `LifecycleCx`
  saves), the verbatim wrappers above with the two `cfg(test)`
  disconnect-cleanup entries, and the remounted `persistence/commit.rs` F3
  shim;
- the test-only respec constants are re-exported from the application in
  `handlers/talent/state.rs`.

### Host surface

| Part | New trait/struct | New adapter | Retained forwards | Moved function lines |
|---|---:|---:|---:|---:|
| 13b | 12 | 15 | 15 | 715 |

- New host code is 27 lines against 715 moved: **3.8 %**.
- With the two retained World forwards it is 42 lines, **5.9 %**.

### Await table (per function, base vs head)

| Function | Base | Head | Awaited, in order |
|---|---:|---:|---|
| `finalize_session_with_generator_like_cpp` | 1 | 1 | the step executor, once per ledger step |
| `execute_finalization_step` | 14 | 14 | loot-persistence wait; loot release-all; buyback clear; character save; mounts; toys; heirlooms; appearances; illusions; character offline; character-account offline; login-account offline; instance-send fence; realm `LogoutComplete` send |
| `clear_buyback_on_logout` / appearances / illusions | 1/1/1 | 1/1/1 | the port call |
| `resume_deferred_player_save_with_generator_like_cpp` | 1 | 1 | the full save |
| `process_pending_periodic_player_save_with_generator_like_cpp` | 1 | 1 | the full save |
| `handle_confirm_respec_wipe_with_generator_like_cpp` | 2 | 2 | commit; publication |
| `commit_represented_talent_reset_like_cpp` | 1 | 1 | `_at` |
| `commit_represented_talent_reset_at_like_cpp` | 2 | 2 | exclusive money fence; `persist_talent_reset` |
| `publish_committed_represented_talent_reset_like_cpp` | 1 | 1 | the objective drain, after `drop(money_persistence)` |
| cleanup, the trainer check, the two helpers | 0 | 0 | — |
| World `finalize_session` forward / integration tests | 1 | 1 (+1 forward layer) | the application future |
| worldport adapter (resume) | — | — | returns the application future unchanged |

- The inlined account wrappers held the lifecycle split across their one
  await; the inlined arm holds the same split across the same await.
- No future that was awaited inline is spawned, reordered or held across a
  different borrow.

### Behaviour (#169)

**Finalization order is unchanged.** These are the ledger steps:

1. `NativeTransfer`: the character native completion, the
   character-selection deferral, then the logout flag, the visibility sync and
   the clear logout time;
2. `LootSettlement`;
3. `Buyback`;
4. `CharacterSave`, the #169 save fence through
   `wow_world_lifecycle::save_current_player_to_db`;
5. the account saves: mounts, toys, heirlooms, appearances, illusions;
6. the offline marks: character, character-account, login-account;
7. `Retirement`: registry, visibility notice, map unregister;
8. `LogoutPublication`: the instance write fence, then the realm route;
9. `Release`.

The previous-ledger handover and the interrupt rules are unchanged. So is the
identity check before each step.

**Talent reset.** The commit order is:

1. the reset cost;
2. the exclusive money fence;
3. the money check;
4. the plan;
5. the fixture seams;
6. the cancellation fence;
7. `persist_talent_reset`;
8. the outcome classification: applied, rolled back, or unknown, which
   quarantines.

The publication still has no await before `drop(money_persistence)`, and
drains only afterwards. Registrations, statuses and processing flags are
unchanged.

The scenario tests asserting this order pass unchanged, with no assertion
touched:

- the finalization ledger tests in `wow-world/tests/production_login_player_owner`;
- the logout-publication fence test in `lifecycle/finalization.rs`;
- the talent-reset rollback, unknown-COMMIT and publication scenarios;
- the buyback durability tests.

### Recursion

- Every moved name was grepped across the production (non-shim) `wow-world`
  files. What remains is two kinds of call:
  - the two pub forwards, which name `wow_world_application::*` explicitly;
  - the worldport adapter, the talent adapter and the driver, which call the
    free functions.
- The character login adapter's `finalize_session` and
  `cleanup_shared_runtime_state` resolve to the World inherents, which
  forward to the free functions. No moved body calls those host methods.
- `unconditional_recursion`: 0 in the production lib, the lib with
  `test-fixtures` and the lib test build.

### Edges and closure

Two new edges, recorded in the manifest, `Cargo.lock`, the `test-fixtures`
forwarding and the `dependency-policy.json` allowlist:

- **`wow-world-application → wow-world-character`**, for the native completion
  the `NativeTransfer` step calls (#1400 follow-up 1);
- **`wow-world-application → wow-world-loot-ops`**, for the loot settlement.

Both were simulated before the move (`wow_world_closure.py check --add …`,
lock and metadata, PASS). After the move the closure passes in both modes (17
packages). Only `wow-world` reaches application. Character and loot-ops do
not reach application.

The 13c login coordinator must therefore land in `wow-world-application` or
stay in the World, never in character: character → application would be a
cycle.

### Accounting

- **`wow-world` production (classifier): 43,306 → 42,437 (−869).** Physical
  lines 51,391 → 50,766; files 469 → 470 (`account.rs` deleted; the adapter and
  the shim added).
  - **Moved:** 715 function lines, plus the committed-reset record and the
    respec constants (about 15).
  - **Reclassified, not production reduction:** about 101. These are the
    verbatim wrappers and the two `cfg(test)` cleanup entries, which the
    classifier counted as production, now in the `#![cfg(test)]` shim.
  - **Retired:** 22 (three wrappers).
  - **Added:** the adapter (25) and the two forwards (15).
- **`wow-world-application/src`: 11,397 → 12,385 physical (+988)**; 10,596 →
  11,571 classifier production; 44 → 50 files.
  - All of the growth is `session_lifecycle/`: 715 moved lines, with the
    expansion of the inlined wrappers and the generic signatures, the trait,
    the cx struct and the headers.
  - This is orchestration: the finalization ledger over lifecycle, loot,
    inventory and the character completion, and the talent-reset money
    transaction with its publication.
- `net_move.py check`: S = 625, G = 988, allowance 1,617.2, PASS, 0 duplicate
  bodies.
- **Ceilings** (none raised):
  - hotspot `session/mod.rs`: production 24,562 → 23,812, test 141,656 →
    141,650, total 166,218 → 165,462;
  - physical `session/mod.rs` stays at 882;
  - new ratcheted row `handlers/session_lifecycle_test_shims.rs` at 205;
  - `session-ownership-policy.json` reprinted with `print-baseline`. The delta
    is the moved and retired items, the adapter method and the shims.
- **Warnings.** The same per crate, file, lint and message as the base in the
  production lib, the lib with `test-fixtures` and the lib test build. The one
  exception is that the pre-existing `cleanup.rs` unused-import warning is
  gone, because the `cfg(test)` entries that used it moved to the shim.
  `wow-world` `dead_code` is 0 in all three builds.
- **Tests.**
  - The test-name multiset is identical (10,214 tests, 5 ignored), and no test
    file changed.
  - The scenarios keep method syntax through the shim forwards.
  - Focused run before final: 440 `wow-world` lib tests (finalization, logout,
    cleanup, buyback, account collections, talent/respec, save, persistence,
    connection), 34 `production_login_player_owner` and 13
    `wow-world-application` tests passed.

### #1400 follow-up 3

The `session/test_reexports.rs` header is four lines of at most 98 characters
and names "follow-up 1" again. The change is comment-only and the test lines
are unchanged.

### 13c plan

On the 13b base:

- the continue-login coordinator (`handlers/character/world_entry/login.rs`)
  goes to `wow-world-application`, or stays in the World, never in character;
- with it, the login hydration steps it calls (`persistence/load.rs`,
  `load_authority.rs`, `lifecycle_ops/operations.rs`, the void-storage unlock
  commit), each to its domain crate where layering allows:
  - void storage and heirlooms → inventory;
  - difficulties and instance time restrictions → instances;
  - titles and rest → quest and lifecycle.

Re-measure first. The coordinator alone was about 20 % host at 60 outside
calls.

### Follow-ups

- The shim file retires when the scenarios call the free functions directly.
- The pub `finalize_session` and `cleanup_shared_runtime_state` forwards retire
  when world-server and the integration tests name the application functions.
- The UFCS sites mark duplicate host names: `loot`/`loot_mut` and
  `resolved_player_money`/`drain` on the character, loot and battle-pet hosts;
  `sync_current_player_session_visibility_detection` and
  `split_spell_state_mut` on the character and teleport/spellbook hosts. They
  can be consolidated once one supertrait owns each.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
passed on the 13b PR head `f4403e32a` in 570 s (manifest
`target/validation-v2/manifests/20261015T050603.972605Z-1326267-final.json`;
`wow-world` lib 3,680 passed and 1 ignored, `wow-world-application` 13
passed). The PR merged as `887726217` (#1401 review follow-up: the evidence
is recorded here, not only on the PR).

## Slice 13c result (continue-login coordinator → application, login hydration → spell, inventory, instances)

Branch `1363-s13c-login-coordinator`, base `887726217` (13b merged).
Move-only (R2), with the slice 4–13b rewrites and nothing else.

### Re-measure and the plan

On the base the coordinator (`handlers/character/world_entry/login.rs`, 896
physical, 880 function lines) called 61 distinct `WorldSession` methods. Seven
were already host-trait methods. Of the other 54:

- **25 were one-line split, hub or core forwards** (`kick`, `realm_id`,
  `session_mgr`, the connect-to setters, the loaded-player flag/power/name
  setters, the xp clamp, the toy, heirloom, achievement and account-data loads,
  the slot counts, the three item-time/enchantment publications, the skill
  replacement, the spell-hit tombstone, the account mount rows, the
  `PLAYER_FLAGS` fixture step). They inline as their bodies.
- **16 hydration bodies had no World-only call** once those forwards inline.
  Each was checked against the host traits its destination crate can name:
  - **spell** (`SpellbookHostLikeCpp`, `SpellEffectHostLikeCpp`,
    `SpellExecutionHostLikeCpp`, `TalentHostLikeCpp`, and the inventory
    `BattlePetSessionHostLikeCpp`): the login spell application, the
    first-login flag and casts, the mount promotion and the account mount
    load with its `set_account_mounts`;
  - **inventory** (`VendorSessionHostLikeCpp`): the account item-appearance
    and transmog-illusion loads;
  - **instances** (`InstanceSessionHostLikeCpp`): the explored-zones and
    difficulty loads;
  - **application**, beside the coordinator: the titles load (its fixture
    step needs the loot-ops `quest_state_mut`), the group-leader flag (no
    application → social edge) and the login known-spell projection (it
    reads the character host's `split_lifecycle_ref`).
- **13 stay World operations** and become the coordinator's host methods
  (list under *Host surface*).

**Closure check.** Every destination already depends on what its moved bodies
name: spell → inventory, entities, lifecycle, core; inventory → lifecycle,
core; instances → core, entities; application → every crate above. No
`wow-world*` edge is added. `wow_world_closure.py check` passed on the base
and passes on the head, in lock and metadata mode (17 packages). The one new
edge is `wow-world-application → wow-module-api` (the coordinator's
`ModuleRegistry` parameter). It is a foundation crate outside the closure
tool, recorded in `Cargo.lock` and the `dependency-policy.json` allowlist.

The coordinator itself fits: about 1.5k moved function lines over 13 host
methods, about 7 % host, so no 13d split was needed.

### What moved

- **`wow-world-application/src/continue_login/`** (1,237 physical):
  - `mod.rs` (115): `ContinueLoginHostLikeCpp: SessionLifecycleHostLikeCpp + SpellExecutionHostLikeCpp`
    with 13 methods, and the coordinator's imports (the World pattern:
    `login.rs` reaches them through `use super::*`, which keeps it under the
    1,000-line review limit);
  - `login.rs` (980): `handle_continue_player_login_with_module_registry_like_cpp`;
  - `hydration.rs` (142): the titles load, the group-leader flag with its
    `PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP`, the login known-spell projection and
    the legacy backpack-size compatibility.
- **`wow-world-spell/src/login_spell_application.rs`** (418):
  `set_known_spells_like_cpp`, `set_represented_favorite_known_spells_like_cpp`,
  `apply_loaded_known_spell_dependencies_like_cpp`,
  `apply_loaded_spell_dependencies_from_roots_like_cpp`,
  `apply_loaded_spell_dependency_skills_like_cpp`,
  `apply_represented_start_all_spells_with_catalogs_like_cpp`,
  `apply_login_known_spell_combat_capabilities_like_cpp`,
  `apply_represented_first_login_flag_if_needed_like_cpp`,
  `apply_represented_first_login_cast_spells_with_catalogs_like_cpp` (with
  its `CAST_FLAG_PENDING_LIKE_CPP`), `promote_loaded_character_mount_spells_like_cpp`,
  `load_account_mounts_like_cpp` and `set_account_mounts_like_cpp`.
- **`wow-world-inventory/src/account_collection_loading.rs`** (146): the
  account item-appearance and transmog-illusion loads. The
  `InitialLoadedItemModsOutcomeLikeCpp` struct moves to `enchantment.rs`
  (the host method returns it).
- **`wow-world-instances/src/instance_session/login_load.rs`** (86): the
  explored-zones load and the difficulty load (a `LifecycleCx` method; it
  now borrows the same two fields, instance state and hub, through
  `split_instances_mut`).
- **`wow-world-core`**: `PlayerRestRatePolicyLikeCpp` (the host method takes
  it), re-exported at its World path.

**Rewrites, beyond `self` → `host`:**

- **Field accessors.** `self.lifecycle` becomes `host.lifecycle_mut()` in the
  coordinator and `host.lifecycle()` (battle-pet host) in the loaders.
  `self.core` becomes `host.core()`, or `core_mut()` for `kick` and the
  connect-to setters. `self.catalogs` becomes `host.hub_ref().catalogs`.
  `self.spell_state.record_login_spell_history_packets_like_cpp` becomes
  `SpellbookHostLikeCpp::split_spell_state_mut(host).0.…`, the 11a precedent.
- **Split forwards** become their host forms; `crate::session::hub_{ref,mut}(self)`
  becomes `host.hub_*()`.
- **`#[cfg(test)]` steps** run only in the World test build, through the
  hub fixture accessors (slice 4b): the loaded-flag canonicalization in the
  coordinator, the titles fixture in `hydration.rs` and the explored-zones
  fixture in `login_load.rs` are gated on
  `test_fixtures_ref_like_cpp().is_some()`.
- **UFCS where two supertraits carry the same name.** Each impl forwards to
  the same `WorldSession` inherent or split:
  - `CharacterLoginHostLikeCpp::sync_current_player_session_visibility_detection_like_cpp`
    (also on the teleport host), in the inlined `set_player_loading`;
  - `SpellbookHostLikeCpp::split_spell_state_mut` (also on the character
    host), twice;
  - `BattlePetSessionHostLikeCpp::split_inventory_ref` (also on the loot
    host), three times.

**Wrappers inlined as their exact bodies** (one-line or two-line):

- `set_player_loading` (the lifecycle flag, then the visibility sync);
- `kick` ×10 (`host.core_mut().kick`), `set_connect_to_{key,serial}`,
  `session_mgr`, `realm_id`, `battlenet_account_id` ×3;
- the lifecycle splits `set_loaded_player_{name,flags,flags_ex,powers}_like_cpp`,
  `clamp_loaded_player_xp_to_next_level_like_cpp`,
  `load_account_{toys,heirlooms}_like_cpp`, `load_completed_achievements_like_cpp`,
  `load_player_account_data_like_cpp`, `account_mount_rows_like_cpp` ×2,
  `add_account_mount_with_faction_counterpart_like_cpp` and
  `expand_account_mount_faction_definitions_like_cpp`;
- the inventory splits `set_player_{inventory_slot,bank_bag_slot}_count_like_cpp`,
  `load_represented_account_{item_appearances,transmog_illusions}_like_cpp` ×2
  each, `send_item_time_update_plans`, `send_item_enchant_time_update_plans` and
  `send_loaded_equipped_item_enchantment_updates_like_cpp`;
- the hub and catalog forwards `skill_store`, `spell_store`,
  `replace_player_skill_records_like_cpp`;
- the spell-state split `tombstone_player_spell_hit_aura_authority_like_cpp`;
- `ensure_canonical_world_map_for_current_player_like_cpp` ×2 (the 13a World
  forward, which names the instances free function);
- `primary_max_power_for_class_like_cpp` (the World private forward to the
  inventory function);
- the `#[cfg(test)]` `apply_loaded_player_flags_to_canonical_like_cpp` F3
  shim (the lifecycle split under the fixture gate).

**What happened to them:**

- **Retired** (no caller left): `send_item_time_update_plans`,
  `send_item_enchant_time_update_plans`,
  `add_account_mount_with_faction_counterpart_like_cpp` and the private
  `primary_max_power_for_class_like_cpp`; the re-export modules whose only
  production reader was the coordinator, `handlers/character/{creation_support,login_support,spell_rules}.rs`
  and `vendor/rules.rs`; the `world_entry.rs` mount and the emptied
  `account/collections.rs`.
- **Moved verbatim to the new `#![cfg(test)]` `handlers/continue_login_test_shims.rs`**
  (only scenarios call them): the toy and heirloom loads, the canonical
  world-map entry forward, the xp clamp, the account-data, achievement and
  appearance loads, the flag and power setters, the slot counts, the loaded
  enchantment publication, the skill replacement, the spell-hit tombstone,
  the `cfg(test)` first-login cast entry of `spell_state/cast.rs` and the
  difficulty forward of the `persistence/load` F3 shim. The file also holds
  one-call method forwards to every moved free function the scenarios call,
  so no call site changes.
- **Stay for other callers:** `set_player_loading` (pub),
  `expand_account_mount_faction_definitions_like_cpp`,
  `load_represented_player_difficulties_like_cpp`'s callers in the scenarios
  (through the shim) and the rest.

A token comparison of the 21 moved functions against their base bodies
(`/home/ubuntu/rustycore-wip/s13c/cmp13c.py`, and a whole-body token diff
for the coordinator) finds two identical (the first-login flag and the
backpack-size helper). The rest differ only by the rewrites above, the
generic signatures and rustfmt commas. No string literal, comment, constant,
operator or control-flow token changed: the `kick` reasons, every
`info!`/`warn!` message and the `RUSTYCORE_REST_TRACE` gate are verbatim.

**World side:**

- `handlers/character/continue_login_host.rs` (118) replaces `world_entry.rs`.
  It holds the `ContinueLoginHostLikeCpp` adapter and the public method form
  of the coordinator, a one-call forward that the `wow-world/tests`
  integration test pins as a future;
- the session driver (`session/connection.rs`) calls the free function;
- the cross-crate fixture calls the instances map entry directly, so the
  World `ensure_canonical_world_map_for_current_player_like_cpp` forward
  leaves production (#1400 follow-up 2, the map-entry half);
- the names only the scenarios still read are `cfg(test)`: the character
  ones join the existing `cfg(test)` import groups of
  `handlers/character/mod.rs`, the session ones join `session/test_reexports.rs`;
- `session/mod.rs` loses `CAST_FLAG_PENDING_LIKE_CPP` and
  `PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP` (moved).

**Deferred, with the reason:**

- **The 13 host methods** below. Each needs a World-only operation, a
  World-typed context or a `cfg(test)` fixture branch with no accessor yet.
- **The void-storage unlock commit** (`persistence/commit.rs`) is the
  void-storage unlock handler's step, not a login step. It moves with that
  handler (4c).
- **The void-storage, group and trait-config load steps** left in
  `persistence/{load,load_authority}.rs` and `lifecycle_ops/operations.rs`.
  They are `CharacterLoginHostLikeCpp` methods that the character loaders call
  (adapters), or have other World callers.
- **The acquisition adapters** (`spell_acquisition/**`,
  `session/{effect_learning,trainer_acquisition}.rs`, about 1.2k). They are
  the World's impls of the application and acquisition runtime traits and
  the borrowed-context builders. They are adapters by nature and leave only
  with the `WorldSession` split (slice 12).
- **The public `finalize_session` and `cleanup_shared_runtime_state`
  forwards.** world-server and 28 integration-test references use method
  syntax, so their callers do not allow it yet.

### Host surface

`ContinueLoginHostLikeCpp` methods, each the same-named `WorldSession`
operation:

1. `apply_initial_loaded_item_mods_like_cpp` (the `_ApplyAllItemMods`
   replay over the World aura application context);
2. `apply_loaded_known_spell_previous_rank_passive_auras_like_cpp` and
3. `apply_login_passive_known_spell_auras_like_cpp` (both build the World
   `player_aura_application_cx_like_cpp`);
4. `apply_login_known_spell_proficiencies_like_cpp` (reads the persistent
   capability snapshot, which has a `cfg(test)` fixture branch);
5. `apply_offline_xp_rest_bonus_with_policy_like_cpp` (rest fixtures);
6. `apply_represented_login_spell_reset_if_needed_like_cpp` (calls the
   quest-reward spell grant, which builds `QuestRewardCx`);
7. `load_instance_time_restrictions_like_cpp` (needs the lifecycle port next
   to the instance state; the instance host has no lifecycle access);
8. `load_player_quests` (375 lines over eight World quest-status
   installers; it goes with slice 6);
9. `load_represented_xp_rest_bonus_like_cpp` (`cfg(test)` rest-flag helper);
10. `set_loaded_player_identity_like_cpp` (`cfg(not(test))` and fixture
    identity branches);
11. `set_represented_at_login_flags_like_cpp`;
12. `set_represented_guild_id_like_cpp` (both have `cfg(test)` fixture
    branches);
13. `sync_player_spell_hit_aura_authority_to_canonical_like_cpp` (the
    World difficulty-dependent empty-source check).

The two future-returning methods return the session's own future unchanged.

| Part | New trait | New adapter | Retained forward | Moved function lines |
|---|---:|---:|---:|---:|
| 13c | 36 | 69 | 27 | 1,540 |

- New host code is 105 lines against 1,540 moved: **6.8 %**.
- With the retained World method forward it is 132 lines, **8.6 %**.

### Await table (per function, base vs head)

| Function | Base | Head | Awaited, in order |
|---|---:|---:|---|
| `handle_continue_player_login_with_module_registry_like_cpp` | 32 | 32 | base row; admission data; homebind delete; homebind repair; mail; pet state; group membership; toys; heirlooms; item appearances; transmog illusions; mounts; inventory; CUF profiles; currencies; spell rows; favourites; skill rows; talents; glyphs; action buttons; transport restore; reputation; quests (host future); achievements; instance time restrictions (host future); account data; auras; spell history; `send_login_sequence`; first-login casts; online mark |
| `apply_represented_first_login_cast_spells_with_catalogs_like_cpp` | 1 | 1 | `execute_server_triggered_spell_like_cpp`, once per create-mode spell |
| `load_account_mounts_like_cpp`, `load_account_item_appearances_like_cpp`, `load_account_transmog_illusions_like_cpp` | 1 each | 1 each | the account-collection port |
| the other 17 moved functions | 0 | 0 | — |
| session driver (`poll_instance_link`) | 1 | 1 | the coordinator future (free function) |
| World method form / integration test | 1 | 1 (+1 forward layer) | the application future |

- The inlined async wrappers (toys, heirlooms, achievements, account data)
  held the lifecycle split across their one await. The inlined block holds
  the same split across the same await.
- No future that was awaited inline is spawned, reordered or held across a
  different borrow.

### Behaviour (#169)

**The login order is unchanged.** The load order is the await order above.
The packet order is:

1. `SMSG_RESUME_COMMS`;
2. `send_login_sequence`;
3. the item-time, enchant-time and loaded-enchantment updates;
4. the login spell reset and the login talent reset;
5. the first-login flag, the casts, the explored zones and the reputation;
6. the spell-hit authority tombstone or sync;
7. the online mark;
8. the module `OnPlayerLogin`.

The coordinator runs no save, so the #169 save fence is not on this path.
Every early return (claim release, `kick`, abort) is at the same point.

**The source-order scans pass unchanged in strictness.** They now read the
application file. Seven markers follow their text:

- the reputation and skill-row bindings (`host,` for `self,` and the dedent);
- `host.load_player_quests().await;`;
- `.load_player_account_data_like_cpp(&mut hub, guid)`;
- `state.clamp_loaded_player_xp_to_next_level_like_cpp(&mut hub)`;
- `promote_loaded_character_mount_spells_like_cpp(host, &known_spells);`,
  which still finds the first call;
- the mail early-return block.

Every order assertion is unchanged. Following the moved files:

- the committed-money scan (`loot.rs`) includes the three moved collection
  files and drops the five retired World files;
- two scenario import lines name `wow_world_spell::login_spell_rules` and
  `wow_world_inventory::vendor`, which the retired re-export modules
  re-exported.

### Recursion

- Every moved name was grepped across the production (non-shim) `wow-world`
  files. What remains:
  - the driver and the method form, which name `wow_world_application::*`;
  - the entities adapter and the cross-crate fixture, which name
    `wow_world_instances::ensure_canonical_world_map_for_current_player_like_cpp`;
  - the `cfg(test)` import of the backpack helper.
- Each of the 13 adapter methods forwards through `WorldSession::name(self, …)`
  to an inherent of that name. Each inherent is still defined (in
  `player_items/modifiers.rs`, `spell_state/{aura_application,spellbook,effects}.rs`,
  `catalogs/operations.rs`, `persistence/load.rs`, `handlers/quest/persistence.rs`,
  `lifecycle_ops/operations.rs`, `social/guild.rs` and
  `spell_state/aura/spell_hit_authority.rs`).
- The UFCS sites resolve to `crate::session::split_{spell_state_mut,inventory_ref}`
  and the visibility-sync inherent, as the base wrappers did.
- `unconditional_recursion`: 0 in the production lib, the lib with
  `test-fixtures` and the lib test build.

### Edges and closure

There is no new `wow-world*` edge. The closure passes in lock and metadata
mode, before and after (17 packages):

- only `wow-world` reaches application;
- character and loot-ops do not reach application;
- instances does not reach character.

The new foundation edge `wow-world-application → wow-module-api` is in the
manifest, `Cargo.lock` and the `dependency-policy.json` allowlist.

### Accounting

- **`wow-world` production (classifier): 42,437 → 40,743 (−1,694).**
  Physical lines 50,766 → 49,349; files 470 → 465 (seven deleted; the
  adapter and the shim added).
  - **Moved:** 1,540 function lines, plus `PlayerRestRatePolicyLikeCpp`,
    `InitialLoadedItemModsOutcomeLikeCpp` and two constants (29).
  - **Reclassified, not production reduction:** 106. These are the inlined
    wrappers the scenarios still call and the `cfg(test)` first-login cast
    entry, which the classifier counted as production, now in the
    `#![cfg(test)]` shim.
  - **Retired:** 86. That is three wrappers (20), the private power forward
    (3), the four re-export modules (36) and the `world_entry.rs` mount (27).
  - **Added:** the adapter file (118).
  - The remaining 51 are the imports and module headers the moved bodies
    used.
- **Destinations (physical):**
  - `wow-world-application/src`: 12,385 → 13,635 (+1,250), classifier
    production 11,571 → 12,821; 50 → 53 files. That is `continue_login/`
    1,237, `lib.rs` +9 and the #1401 nit +4.
  - `wow-world-spell/src` +430: `login_spell_application.rs` 418 and the
    re-exports 12.
  - `wow-world-inventory/src` +160: the loaders 146, the moved struct 10 and
    the re-exports 4.
  - `wow-world-instances/src` +91: `login_load.rs` 86 and the re-exports 5.
  - `wow-world-core/src` +22: the rest-rate policy.
- **Why application grows.** This is orchestration: the C++
  `HandlePlayerLogin` composition over character, spell, inventory,
  instances, lifecycle and the World host, with its title, group-leader and
  known-spell projection helpers. The growth over the moved lines is the
  inlined forwards (about 90 lines of split blocks), the generic signatures
  and the import block.
- `net_move.py check`: S = 1,417, G = 1,953, allowance 2,448.9, PASS, 0
  duplicate bodies.
- **Ceilings** (none raised):
  - physical rows retired: `world_entry/login.rs` (896) and `world_entry.rs` (27);
  - physical rows tightened: `handlers/character/mod.rs` 321 → 297,
    `handlers/character/vendor.rs` 234 → 232, `session/mod.rs` 882 → 874;
  - new ratcheted row `handlers/continue_login_test_shims.rs` at 265;
  - hotspot `session/mod.rs`: production 23,812 → 23,247, test 141,650 →
    141,645, total 165,462 → 164,892;
  - hotspot `handlers/character/mod.rs`: production 3,416 → 2,287, test
    15,655 → 15,655, total 19,071 → 17,942;
  - `session-ownership-policy.json` reprinted with `print-baseline`. The
    delta is the moved and retired items, the adapter methods and the shims.
- **Warnings.** Per crate, file, lint and message, these builds are
  identical to the base:
  - the production lib;
  - the lib with `test-fixtures`;
  - the lib test build.

  The one exception is that the pre-existing `world_entry/login.rs`
  unused-import warning is gone with the file. `wow-world` `dead_code` is 0
  in all three builds.
- **Tests.**
  - The test-name multiset is identical. No `#[test]` function was added,
    removed or renamed, and no assertion changed. The test-file edits are
    `include_str!` paths, the seven markers above and two import lines.
  - Focused run before final: `wow-world` lib 3,680 passed and 1 ignored
    (the full suite, as at 13b), and `production_login_player_owner` 34
    passed.

### #1401 follow-ups

- **2.** `session_lifecycle/finalization.rs` lines 105 and 223 are wrapped
  under 100 columns.
- **1.** The 13b PR body's "+988 is all `session_lifecycle/`" is recorded
  correctly above: 976 there plus 12 in `lib.rs` and `talent_handlers.rs`.
- **4.** The coordinator landed in application, not character.
- **3.** The 13b *Validation* section now records its concrete evidence.

### Follow-ups

- **The shims** (`continue_login_test_shims.rs` 265, with the 13a and 13b
  ones) retire when the scenarios call the free functions directly.
- **The 13 coordinator host methods** retire as their domains move:
  - the quest-status load with slice 6;
  - the aura and item-mod replays with the application aura owner;
  - the fixture-branch setters when their fixtures gain accessors;
  - the time restrictions with an instance-host lifecycle accessor.
- **The public method form of the coordinator** and the `finalize_session`
  and `cleanup_shared_runtime_state` forwards retire when world-server and
  the integration tests name the application functions.
- **The UFCS sites** add `split_inventory_ref` (battle-pet and loot hosts) to
  the 13b list of duplicate host names.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed PR head. Its SHA, manifest, duration and test counts
are recorded on the PR, and in this document's next slice.

## Slice 13 summary (13a–13c)

The slice 13 domain was about 4.6k production lines in six families on
`d8e8caf5c` (slice 13 plan).

| Slice | `wow-world` production | Δ | Moved out (function lines) | Reclassified | New host (lines, share) |
|---|---|---:|---:|---:|---|
| 13a map entry → instances, far-transfer completion → character | 44,322 → 43,306 | −1,016 | 984 | 20 | 38, 3.9 % |
| 13b lifecycle orchestration and respec wipe → application | 43,306 → 42,437 | −869 | 715 (+15) | 101 | 27, 3.8 % |
| 13c continue-login coordinator → application, login hydration → spell, inventory, instances | 42,437 → 40,743 | −1,694 | 1,540 (+29) | 106 | 105, 6.8 % |
| **Total** | **44,322 → 40,743** | **−3,579** | **3,239 (+44)** | **227** | **170, 5.2 %** |

- **Destinations:**
  - `wow-world-application` grew from 11,397 to 13,635 physical lines
    (+988 in 13b, +1,250 in 13c). It now holds the session lifecycle
    orchestration and the continue-login coordinator: the C++
    `WorldSession::LogoutPlayer`/`HandlePlayerLogin` compositions.
  - `wow-world-character` gained the world-port response (+738, 13a).
  - `wow-world-instances` gained the map entry (+449, 13a) and the login
    explored-zones and difficulty loads (+91, 13c).
  - `wow-world-spell` gained the login spell application (+430).
  - `wow-world-inventory` gained the account appearance loads (+160).
- **Edges.** 13a added character → instances; 13b added application →
  character and application → loot-ops; 13c added only the foundation edge
  application → `wow-module-api`. The closure passed in both modes at each
  step.
- **#169.** The save fence, the finalization ledger order, the world-port
  response order and the login load and packet order are unchanged. Every
  await of every moved function is at the same point in the same order.
- **What stays in the World from the slice 13 families, and why:**
  - the `PlayerSaveHostLikeCpp` adapter and the money-fence constructors,
    which are adapters;
  - the login hydration steps that are `CharacterLoginHostLikeCpp` methods
    (void storage, group, trait-config loads);
  - the 13 coordinator host methods (above);
  - the void-storage unlock commit, which goes with its handler (4c);
  - the public `finalize_session`, `cleanup_shared_runtime_state` and
    coordinator method forwards, which have world-server and integration-test
    callers.

**What remains for #1363 overall:**

- **Slice 12:** the base-module logic and the thunk sweep. The World
  adapters, the split forwards and the context builders, the acquisition
  adapters among them, retire as `WorldSession` is split.
- **The quest split, then slice 6:** make `wow-world-quest` quest-only (the
  condition projection, valuation and admission proof to a lower crate,
  `registry_sync` to inventory), then move the quest bodies (about 7k). The
  quest-status login load and the login spell reset's quest-reward grant go
  with it.
- **4c:** the deferred families (collections, group, travel, spell, void
  storage with its unlock commit, the trainer context).
- **Cleanup:** retire the `#![cfg(test)]` shim files by converting the
  scenario call sites, retire the remaining public World forwards with
  their world-server and integration-test callers, and consolidate the
  duplicate host names the UFCS sites mark.

## Slice Q plan (make `wow-world-quest` quest-only)

Measured on `9ddc39f38` (slice 13c merged) before anything moved. This is
the prerequisite for slice 6: the quest bodies must land in a crate that
holds only quest code (Slice A summary, recommendations 1 and 2; #1378
review follow-up 2).

**`wow-world-quest/src`: 14,394 physical lines in 53 files** (classifier
production 13,745). Four parts of it are not quest:

| Part | Lines | Users outside quest | Quest-only blocker |
|---|---:|---|---|
| `player_conditions{,/inputs}` | 690 | application (trainer purchase, trainer views), loot-ops (gameobject release), `wow-world` (condition values, trainer acquisition) | the fixture-gated `quest_state: &SessionQuestState` field and constructor arguments |
| `inventory_valuation{,/direct_storage}` | 1,280 | application (inventory move planning), `wow-world` (item loading) | none (inherent impls on the projection, which moves with it) |
| `trainer_admission` | 30 | application (trainer offer and controller), `wow-world` (trainer handler) | none |
| `registry_sync` | 211 | application (equipment-set use, inventory scaling, inventory swap equip, aura item scaling, trainer runtime install), loot-ops, `wow-world` (5 files), quest completion | the fixture-gated `&SessionQuestState` **and** `&SessionSpellState` in its hydration context (spell → inventory exists since 13c, so inventory can no longer depend on spell) |

The quest side extends `PlayerConditionProjectionCxLikeCpp` with one
inherent method (`represented_quest_available_conditions_meet_like_cpp`,
`quest/visibility.rs`, two quest callers). An inherent impl must live in
the defining crate, so it has to become a free function first.

**Decisions:**

- **New crate `wow-world-player-conditions`** for the projection, the
  valuation and the admission proof (1,999 lines). No existing crate fits:
  the projection reads inventory, social and (fixture-only) instances and
  spell state, so it sits above them, and quest, loot-ops, application and
  `wow-world` read it, so it sits below quest. Its direct dependencies are
  core, inventory, social, instances and spell (plus `wow-conditions`,
  `wow-constants`, `wow-core`, `wow-data`, `wow-entities`; no external
  crate).
- **`registry_sync` → `wow-world-inventory`**, as settled on `1423cfbfa`
  (*`registry_sync` destination*). Inventory gains `wow-world-loot`
  (`LootState`; loot depends on core only). Unlike that note, the fixture
  hydration must also stop naming `SessionSpellState`, because
  spell → inventory now exists.
- **Fixture seams in `wow-world-core`.** Two one-method, fixture-gated
  traits beside `PlayerRegistryHydrationAccessLikeCpp`:
  `PlayerQuestGameplayFixtureSourceLikeCpp` (implemented by
  `SessionQuestState`) and `KnownSpellsFixtureSourceLikeCpp` (implemented by
  `SessionSpellState`), both with `Sync` as supertrait. The fixture fields
  become `&dyn …` references; the read stays lazy, at the same step.
  Production builds have no field, argument or trait.
- **`inventory_scaling` and `equipment_set_use` stay in application for
  now (Q2).** They no longer need quest after `registry_sync` moves (they
  name `wow_world_inventory` for it), so they are not a quest-only blocker.
  Moving them is a separate review unit: `equipment_set_use` is a handler
  registrar (checker contract, composer guard and tests,
  `handler-module-policy.json`, the exact facade), and inventory already
  has a `crate::equipment_set_use` module, so the move needs a module
  name decision that is not move-only.
- **Closure check before moving** (`--add`
  `wow-world-player-conditions` → core, inventory, social, instances,
  spell; quest, loot-ops, application, `wow-world` → player-conditions;
  inventory → loot), lock and metadata modes: PASS, and all existing rules
  hold. A new rule places the crate: only quest, loot-ops, application and
  `wow-world` may reach it (inventory, spell, entities and loot already
  cannot, because it reaches social and instances).
- **Order of changes** (separate commits): prep 1 (free function), prep 2
  (quest fixture seam in the projection), closure rule, move to the new
  crate, an import-line offset (below), prep 3 (registry-sync fixture
  seams), move `registry_sync`.

## Slice Q result (condition projection, valuation, admission proof → new `wow-world-player-conditions`; `registry_sync` → inventory)

Branch `1363-sQ-quest-only`, base `9ddc39f38`. Behaviour-neutral prep plus
move-only (R2) steps. Player-condition evaluation, inventory valuation,
registry sync (publication order: position, fixture hydration, loot rolls,
party type) and trainer admission results are unchanged; no await, packet
or persistence order is touched.

### Prep (behaviour-neutral)

1. **Prep 1** (`c0aab9fb9`): the inherent
   `PlayerConditionProjectionCxLikeCpp::represented_quest_available_conditions_meet_like_cpp`
   in `quest/visibility.rs` becomes the free function
   `quest::visibility::represented_quest_available_conditions_meet_like_cpp(conditions, …)`.
   The body is dedented one level and `self.` becomes `conditions.`. The
   three `self.player` reads go through the existing
   `player_access_like_cpp()` accessor, which returns `&self.player`. The
   two callers (`dialog_status`, `quest_eligibility`) pass
   `self.conditions` first. `git diff -w` is 34 lines: same reads, same
   order, same result.
2. **Prep 2** (`7d94655e4`, fixture-only): the fixture-gated `quest_state`
   field and constructor arguments of `PlayerConditionProjectionCxLikeCpp`,
   `PlayerConditionProjectionInputsLikeCpp` and
   `player_condition_projection_cx_like_cpp` change from
   `&crate::SessionQuestState` to
   `&dyn PlayerQuestGameplayFixtureSourceLikeCpp`. `SessionQuestState`
   implements the trait by calling its existing inherent
   `player_quest_gameplay_fixture_like_cpp`. The projection still reads
   the fixture in `project_values_before_item_level_like_cpp`, at the same
   step, only when `consumer_test` and the Player handle is absent.
   Callers keep passing `&SessionQuestState` (unsized coercion).
3. **Prep 3** (`0502ab280`, fixture-only): the same for
   `registry_sync`. `PlayerRegistryHydrationContext.spells`/`.quests`, its
   constructor and the two fixture arguments of
   `sync_player_registry_state_like_cpp` take
   `&dyn KnownSpellsFixtureSourceLikeCpp` and
   `&dyn PlayerQuestGameplayFixtureSourceLikeCpp`. `SessionSpellState`
   implements the new spell trait with its inherent
   `known_spells_fixture_like_cpp`. Both traits gain `Sync` as supertrait,
   because the borrowed sources are held across existing awaits and the
   concrete references were `Send`. `hydrate()` reads both fixtures at the
   same step under the same conditions.

Nothing outside `cfg(any(test, feature = "test-fixtures"))` changes in
preps 2 and 3, so the production build is byte-for-byte the same code
path. The fixture-gated paths still read the same fixture values at the
same step, and the full test suites (the `final` run below) exercise them.

### What moved

- **To the new `wow-world-player-conditions`** (`89beacb0d`, 5 R100
  renames, 2,001 lines, no visibility change): `player_conditions.rs` +
  `player_conditions/inputs.rs`, `inventory_valuation.rs` +
  `inventory_valuation/direct_storage.rs`, `trainer_admission.rs`. Its
  `lib.rs` (28) holds the same module declarations and re-export items the
  quest root had for them. The valuation's inherent impls on the
  projection move with the type.
- **To `wow-world-inventory`** (`bb62db1b2`, R100, 213 lines):
  `registry_sync.rs`. Inventory re-exports the same three names the quest
  root did.
- **The quest root** re-exports no condition, valuation, admission or
  registry name; no compatibility re-export is kept. Callers name the new
  owners directly:
  - quest: 5 files for the projection, `quest/completion.rs` for the sync
    (rustfmt reflow);
  - application: trainer purchase (`offer`, `controller`,
    `runtime_install`), `aura_removal` (trainer views, item scaling),
    inventory move planning, inventory swap equip, inventory scaling,
    equipment-set use;
  - loot-ops: gameobject release, release registry;
  - `wow-world`: trainer handler, condition values, item loading, trainer
    acquisition (one import line), canonical player sync, registry
    binding and three `player_items` files.
- **Import-line offset** (`5c57eb32b`): the longer crate path no longer
  fits `trainer_acquisition.rs`'s `let` line, so the file imports
  `PlayerConditionProjectionInputsLikeCpp` (+1 line in the Session hotspot,
  whose ceiling is at its live value). Rather than raise the ceiling,
  `session/driver/mod.rs` drops `use wow_packet::WorldPacket;`, which rustc
  reports unused in all three builds. No code change; one pre-existing
  warning fewer.

**What `wow-world-quest` holds now:** `quest/` (43 files),
`quest_handlers`, `quest_query_handlers{,/complete_quest}` and `lib.rs`.
Quest only.

### Edges and closure

- **Added:** player-conditions → core, inventory, social, instances, spell
  (+ the five foundation/domain crates above); quest, loot-ops,
  application, `wow-world` → player-conditions; inventory → loot.
  **Removed:** none (quest still uses all 20 of its dependencies).
- **Head closures:** player-conditions → core, entities, instances,
  interaction, inventory, lifecycle, loot, social, spell, visibility. Only
  quest, loot-ops, application and `wow-world` reach it. Only application,
  loot-ops and `wow-world` reach quest; only application and `wow-world`
  reach character and loot-ops. Inventory, spell, entities and loot reach
  neither social nor instances. `wow_world_closure.py check`: PASS in lock
  and metadata modes (18 packages).
- **New closure rule** (`ce429b832`) with a self-test case (11 cases). The
  case covers the cycle and the quest reach if the crate depended on quest,
  and a direct and a transitive violation from below. It fails when the
  rule is disabled.
- `cargo tree -p world-server -e normal,build -e features` shows no
  `test-fixtures` and no `wow-world-test-support`.

### Accounting

| Crate `src` (physical) | Base | Head | Δ |
|---|---:|---:|---:|
| `wow-world-quest` | 14,394 (53 files) | 12,179 (47) | −2,215 |
| `wow-world-player-conditions` | — | 2,029 (6) | +2,029 |
| `wow-world-inventory` | 22,776 | 22,996 | +220 |
| `wow-world-application` | 13,635 | 13,637 | +2 |
| `wow-world-core` | 47,399 | 47,425 | +26 |
| `wow-world-spell` | 20,103 | 20,112 | +9 |
| `wow-world` | 49,349 | 49,349 | 0 |

- **Quest −2,215:**
  - files out: 2,211 (the condition files 1,970 at base plus the 30-line
    proof, and `registry_sync` 211);
  - `lib.rs` 96 → 74 (−22);
  - +18 from prep 1 (rustfmt of the free function, +9 net over three
    files) and the fixture trait impl (+8), plus 1 reflow in completion.
  - Classifier production 13,745 → 11,597.
- **New crate:** 2,001 moved lines (`player_conditions.rs` is 586, +1
  from prep 2's rustfmt) plus `lib.rs` 28.
- **Inventory +220:** `registry_sync` 213 (211 + 2 from prep 3's rustfmt)
  and the re-exports 7.
- **Core +26:** the two fixture traits and their re-exports. **Spell +9:**
  the trait impl.
- **Application +2:** the rustfmt reflow in `runtime_install.rs`.
  Classifier production 12,821 → 12,823.
- **`wow-world` production (classifier): 40,743 → 40,743.** That is +1
  import and −1 unused import; this slice moves nothing out of
  `wow-world`.
- `net_move.py check --base origin/3.4.3`: NOT-APPLICABLE (exit 0; S = 0,
  G = 71, 0 duplicate bodies). G is the two fixture traits, their impls,
  the new `lib.rs` and the rustfmt deltas.

### Policy and ceilings

- **`dependency-policy.json`:**
  - `wow-world-player-conditions` joins the `application` category,
    restricted to its 10 actual dependencies, with no external
    dependency;
  - quest, loot-ops and application += player-conditions;
  - inventory += `wow-world-loot`.
- **Fixtures.** New `forbidden-player-conditions-to-wow-world.json`,
  `…-to-world-application.json` and `…-to-quest.json`.
- **`net-move-policy.json`** adds the destination root (17;
  `test_net_move.py` pins it).
- **`test-fixtures`.** The new crate has a `test-fixtures` feature that
  forwards to core, instances, inventory, social and spell, plus a self
  dev-dependency, as quest has. Quest, loot-ops, application and
  `wow-world` forward to it, and inventory now also forwards to loot.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`.
  The delta is one WorldSession signature path
  (`player_condition_projection_cx_like_cpp`'s return type) and the four
  `registry_sync` direct-registry rows (package and source
  `wow-world-quest` → `wow-world-inventory`, and sort order). With those
  names masked the rows are identical. The condition files have no rows,
  so the new crate needs no session-ownership package role.
- **Ceilings.** None raised and none changed: the Session hotspot stays at
  23,247 / 164,892 through the offset above. No physical row names a
  moved file.

### Warnings and tests

- **Warnings.** Every `wow-world*` crate is force-recompiled at base and at
  head (each `lib.rs` touched) and compared by file, lint and message,
  with the moved paths mapped. The production lib, the lib with
  `test-fixtures` and the lib test build are identical, except that the
  dropped `driver/mod.rs` import warning is gone (266 → 265, 310 → 309,
  371 → 370). `wow-world` `dead_code` is 0 in all three builds.
  - The `test-fixtures` lib build of `wow-world` alone (without
    `cfg(test)`) already fails at base, with 87 `E0061` errors in
    test-only call shapes. Its error set is unchanged at head; this slice
    does not cause it.
- **Tests.** The Rust test-name multiset is identical: 10,214 `#[test]`
  functions and 5 ignored, with the same names. No moved module had
  tests. The closure self-test gains one Python case (11).

### Q2 plan (follow-up)

- Move `inventory_scaling` (220) and `equipment_set_use` (462 + 41 test
  lines) from application to `wow-world-inventory`. Both now need only
  core, inventory, loot and the inventory-owned registry sync.
- Before the move, choose a module name for the registrar, because
  inventory already has `crate::equipment_set_use` (the slot/plan helpers).
- Then follow `EQUIPMENT_SET_USE_REGISTRAR` with the checker:
  `direct_builder.rs`, the composer guard and tests, and
  `handler-module-policy.json`. Then the application facade. About −723
  application lines.

### Slice 6 plan (quest bodies → `wow-world-quest`)

- Re-measure the `wow-world` quest family on the Q merge: the
  `handlers/quest/**` hotspot (production 5,494) and `session/quest/**`,
  with the quest-status login load and the login spell reset's quest-reward
  grant.
- Move them into the now quest-only crate behind the existing quest
  host traits. Lower crates keep reaching quest logic only through host
  traits or application/`wow-world`.
- Condition and valuation reads go to `wow-world-player-conditions`, which
  quest already depends on.

### Follow-ups

- Q2 (above).
- The trainer split (Slice A summary, recommendation 3) can now reach the
  condition crate instead of quest. Trainer purchase still names
  `SessionQuestState` for quest events, and `PlayerAuraApplicationCxLikeCpp`
  from `aura_removal`.
- The `wow-world` lib with `test-fixtures` but without `cfg(test)` does
  not compile at base (87 errors). Either fix it or stop listing it as one
  of the three warning builds.

### Validation

`final --architecture --timings` (`VALIDATION_V2_CARGO_JOBS=1`,
`VALIDATION_V2_TIMEOUT_SECONDS=1800`) runs on the committed head that
includes this section. Its manifest path, dirty flag and duration are in
the PR body. This section is not edited after that run, so the PR head is
the validated SHA.

## Warning recipe (canonical from slice 6a; #1403 follow-up 1)

Measured on `18cb95eff` (slice Q merged). From slice 6a on, every slice
compares warnings with exactly these three commands, at base and at head,
with `PROTOC` set and the per-worktree `CARGO_TARGET_DIR`:

```sh
cargo check --locked -p wow-world --lib                          --jobs 1 --message-format json > lib.json
cargo check --locked -p wow-world --lib --features test-fixtures --jobs 1 --message-format json > fixtures.json
cargo check --locked -p wow-world --lib --tests                  --jobs 1 --message-format json > tests.json
```

(`--locked` added after the #1404 review; it fails instead of updating
`Cargo.lock`, so every slice measures the committed dependency graph.) Count
and compare with `tools/architecture/warning_units.py` (`count`, `compare`,
`self-test`; slice 6b).

- **What each build is.**
  1. The production lib: `wow-world` and every `wow-world-*` crate without
     `test-fixtures`.
  2. The lib with `test-fixtures` and without `cfg(test)`: what dependents'
     tests link. It **compiles at base** (exit 0, no error).
  3. `--lib --tests`: the `test-fixtures` lib (dev-dependencies unify the
     features), the lib unit-test build (`cfg(test)`) and the two
     integration tests (`production_character_rename`,
     `production_login_player_owner`).
- **Counting.** Cargo emits a unit's messages before its
  `compiler-artifact` line, so each warning is attributed to the unit
  that follows it (with `--jobs 1` the units are sequential; cached units
  replay their messages). A unit is the target name plus
  `profile.test`. Count warnings by unit and lint code; compare head with
  base by file, lint and message, with moved paths mapped.
- **Base figures (`18cb95eff`):**

| Build | `wow-world` lib | `wow-world` lib test | All `wow-world*` lib units | `wow-world` `dead_code` |
|---|---:|---:|---:|---:|
| production lib | 142 (137 `unused_imports`, 3 `unused_mut`, 2 `unused_variables`) | — | 265 | 0 |
| lib + `test-fixtures` | 150 (145, 3, 2) | — | 230 | 0 |
| `--lib --tests` | 150 (same set as above) | 197 (149 `unused_imports`, 24 `unused_mut`, 24 `unused_variables`) | 427 | 0 / 0 |

  The integration-test units report no warning. No build reports
  `unconditional_recursion`.
- **Correction of earlier figures.** Slice Q reported that the
  `test-fixtures` lib "already fails at base, with 87 `E0061` errors". The
  canonical command above compiles at slice Q's merge, and `final`'s
  `cargo check --workspace --all-targets` compiles the same configuration.
  The 87 errors therefore came from a different, unrecorded invocation,
  not from this build, and no feature forwarding needed fixing
  (`wow-progression/test-fixtures` and
  `wow-spell-acquisition/test-fixtures` come only from wow-world's
  dev-dependencies; the `test-fixtures` lib does not need them). rustc runs
  the `dead_code` lint only after type checking succeeds. So in any
  earlier slice whose "lib + `test-fixtures`" build did not type-check,
  the reported "`wow-world` `dead_code` 0" for that build was not
  measured. Slice Q's 310 / 309 figures and the earlier slices' "lib with
  `test-fixtures`" figures may not have covered `wow-world` itself. They
  are not comparable with the table above. The table is the new
  baseline: `dead_code` is 0 in all three builds at `18cb95eff`.
- **Forbidden-edge fixture** (#1403 follow-up 2):
  `fixtures/forbidden-loot-to-inventory.json`. `wow-world-loot` may not
  depend on `wow-world-inventory`. The architecture self-test passes with
  32 fixtures, and fails when the fixture's expectation is inverted.

## Slice 6 plan (quest bodies → `wow-world-quest`, phase 2)

Measured on `18cb95eff` (slice Q merged) before anything moved, with the
hotspot classifier and a lexical index of the inherent `WorldSession`
methods (bodies, callers, field reads and the `WorldSession` methods each
body calls).

- **The quest family in `wow-world`: 7,665 classifier production lines in
  32 files** (7,896 physical):
  - `handlers/quest/**` 5,494 (the hotspot): the handler entry points
    (`handlers/` 2,101 function lines: quest-giver hello, status, query,
    accept, confirm-accept, request-reward, choose-reward, POI, push to
    party, adventure map), objective progress (`objectives.rs` 615),
    source-item storage (398), persistence and the login quest-status load
    (507), the item-mutation persistence plans (405), log slots (173),
    eligibility (144), reward validation and controller (270), sharing
    helpers (47) and the shared helpers of `mod.rs`;
  - `session/quest/**` 1,163: state accessors and installers, giver and
    seasonal status, rewards projections, publication, the objective drain;
  - `session/quest_dialog.rs` 385 and `session/quest_interaction.rs` 61.
- **Its whole surface is 131 methods (6,021 function lines) calling 31
  outside `WorldSession` methods** (inventory and item storage, group and
  player registry, spell, publication). Moving the handler entry points
  first would need about 45 host methods; moving bottom-up needs few.
- **Names already owned by the quest crate stay as host adapters.**
  `player_quest_gameplay_snapshot_like_cpp`,
  `mutate_player_quest_gameplay_like_cpp`,
  `invalidate_player_quest_status_authority_like_cpp`,
  `find_quest_slot_like_cpp`, `quest_log_create_entries_like_cpp`,
  `send_represented_quest_log_slot_update_like_cpp` and the pending
  sharing reads are World forwards over existing quest-crate functions of
  the same or a colliding name; they stay until their callers move.
- **Destination: `wow-world-quest` only.** Its users stay application,
  loot-ops and `wow-world`; no lower crate gains a quest edge.

| Part | Scope | Function lines | Host |
|---|---|---:|---|
| **6a** | quest state installers and log slots, quest-status authority, seasonal load/reset and completed-quest bits, quest status save and the login quest-status load, item-mutation persistence plans, quest-giver menus and the prepared-quest/query/request-items/details/offer-reward publication, XP and money projections, `CanRewardQuest`/`CanCompleteRepeatableQuest`, the login rewarded-spell grant | 2,123 (52 functions) | 14 methods; no new edge |
| **6b** | objective progress (`objectives.rs`), source-item storage, the objective drain and its operation, reward validation and controller, eligibility, the remaining state helpers | about 1,900 | about 18 (item storage, templates, registry sync, visibility refresh, condition projection) |
| **6c** | the handler entry points (`handlers/**`, `handlers.rs`), the sharing helpers and the `mod.rs` helpers; the registrar then calls the moved bodies and `handlers/host.rs` retires | about 2,500 | about 10 (group and player registries, realm send, gossip menu, POI cache) |

- **Quest-bound world-entity leftovers** (kill credit, dynamic flags,
  spell-click visibility) are not in this family: they are host methods of
  the entities and spell hosts that read quest state. They follow 6c,
  when no World quest body is left for them to call.
- **Closure.** 6a adds no crate edge (no manifest or lock change): every
  type the moved bodies name comes from a crate quest already depends on
  (core, entities, lifecycle, spell, `wow-*` foundations). The closure
  check passes on the unchanged graph in lock and metadata modes at base
  and head. 6b and 6c are expected to need none either (inventory, social
  and spell are already quest dependencies); each re-runs the check.

## Slice 6a result (quest state, persistence, login quest-status load, quest-giver dialog and publication → quest)

Branch `1363-s6a-quest`, base `18cb95eff`. Move-only (R2) with the slice
4–13 rewrites, after the warning-recipe commit above.

### What moved

**To `wow-world-quest/src/quest_session/`** (new module, 2,494 physical
lines), behind `QuestSessionHostLikeCpp: SessionHubHostLikeCpp`:

- `persistence.rs` (466): `save_represented_quest_status_like_cpp`,
  `save_changed_represented_quest_statuses_like_cpp`,
  `save_represented_quest_statuses_completed_after_like_cpp`,
  `save_quest_to_db` and `load_player_quests` (the C++
  `Player::_LoadQuestStatus`, `_LoadQuestStatusObjectives`,
  `_LoadQuestStatusRewarded`, `_LoadDailyQuestStatus`, weekly, monthly and
  seasonal loads);
- `item_mutations.rs` (427): the bank, transfer and withdrawal plans
  (`ItemAddedQuestCheck`/`ItemRemovedQuestCheck`), the source-item bound
  objective plan, `quest_bound_item_objective_quest_order_like_cpp` (from
  `objectives.rs`) and `ItemTransferQuestPersistencePlanLikeCpp`;
- `state.rs` (191): the rewarded-duplicate removal, the free-slot search
  and its private helper, the status insert, rewarded flag, explored mark,
  the four loaded-status installers and the quest level;
- `giver.rs` (494): the status-authority load start, the status reads,
  the spell-area quest status, the seasonal load and reset with their
  outcome and row types, the quest-giver query and the request-items and
  details packets;
- `dialog.rs` (463): the creature and gameobject quest-giver use, the
  menus (`PrepareQuestMenu`), the list entry, the prepared-quest
  publication (`SendPreparedQuest`, from the deleted
  `session/quest/publication.rs`), `RepresentedPreparedQuestMenuItemLikeCpp`
  with its log-row helper and the three menu-icon constants;
- `rewards.rs` (342): the rewarded-spell grant
  (`LearnQuestRewardedSpells`, bound by the spell crate's
  `AuraHostLikeCpp`), the completed-quest bits, the XP and money rewards,
  `CanRewardQuest` and `CanCompleteRepeatableQuest`, and the offer-reward
  packet;
- `quest_session.rs` (111): the module root, imports and the host trait.

**Rewrites, beyond `self` → `host`:** `self.catalogs` →
`host.hub_ref().catalogs`; `self.core` → `host.core()`; `self.lifecycle`,
`self.quest_state` → `host.lifecycle()`, `host.quest_state()` (the `&` in
front of them and of `catalogs` is dropped: six arguments in two
functions);
`self.world_entities` → `host.world_entities_mut()`;
`crate::session::{hub_ref,split_quest_state_ref,split_quest_state_mut}(self)`
→ the host forms; `wow_world_quest::` → `crate::`; a call between moved
functions becomes a free call; `cfg!(test)` (two arguments) →
`host.world_test_consumer_like_cpp()`. The World constants
`QUEST_OBJECTIVE_{ITEM,CURRENCY,MONEY}_LIKE_CPP` and
`QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP` (and its `_LOCAL`
aliases) are imported from `wow_constants::quest`, which defines the same
values (1, 4, 8, 0x1); the identifiers in the bodies are unchanged. The two `#[cfg(test)]` fixture
branches of the completed-quest bits become
`#[cfg(any(test, feature = "test-fixtures"))]` and are gated on the same
flag, read once before the split (two lines each). The unused `&self` of
`finish_item_transfer_quest_persistence_like_cpp` becomes `_host`.

**Inlined one-line wrappers (each its exact body; the World wrapper is
deleted only where no caller remains):**

- `quest_rewards_block_like_cpp(quest)` ×2 (details, offer reward) →
  `crate::represented_quest_rewards_block_like_cpp(quest)`; the World
  wrapper retires;
- `self.is_quest_disabled_like_cpp(quest.id)` ×1 →
  `crate::quest_is_disabled_like_cpp(host.hub_ref(), quest.id)`;
- `self.spell_store()` ×1 and `self.skill_store()` ×1 →
  `host.hub_ref().catalogs.spell_store()` / `.skill_store()`.

None is async. (The code commit message says "two"; the four above are
the complete list.)

**Token comparison.** Each of the 52 moved functions was compared with its
base body after the same rewrite applied by script (`/tmp/s6a/tools/cmp.py`,
comments kept): 25 are identical. The other 27 differ only by the dropped
`&`, `pub(crate)`/private visibility, the `_host` name, rustfmt (trailing
commas, generic-bound wrapping, one single-expression closure unwrapped
from `{ … }`), the four inlined wrappers, the log-row helper's path, the
spell bounds and the two fixture gates. No string literal, constant,
operator or control-flow token changed: every `info!`/`warn!`/`debug!`
message is verbatim.

**World side:**

- `session/quest/host.rs` (65): the `QuestSessionHostLikeCpp` adapter;
- every production caller names the free function
  (`wow_world_quest::name(self, …)`), including the trait-impl forwards of
  the continue-login, loot, item, entities, aura, chat, login and quest
  query hosts;
- `handlers/quest_session_test_shims.rs` (219, `#![cfg(test)]`, mounted
  from `handlers/mod.rs` outside both hotspot trees): 19 one-call method
  forwards the scenarios call, and the two `#[cfg(test)]` readers nested
  in the files the bodies left (`seed_seasonal_quest_status_like_cpp`,
  `represented_quest_statuses_for_save_like_cpp`), moved verbatim;
- `session/quest/publication.rs` and `handlers/quest/persistence/item_mutations.rs`
  are deleted; `session/quest_dialog.rs` keeps only its re-exports and the
  item-objective helper.

**Deferred, with the reason:** the quest POI store load
(`quest_poi_store_like_cpp` writes the session catalog cache, which the
hub lends only shared; it goes with its handler in 6c); the async
completion wrappers and the confirm-accept local state in
`handlers/quest/state.rs` (they call the objective drain, 6b); the
eligibility context and `represented_quest_is_important_like_cpp` (6b);
the quest-crate-name adapters above; and the session setters
(`set_quest_*_store`, world-server construction API).

### Host surface

`QuestSessionHostLikeCpp` (14 methods, each the same-named World field,
split or inherent): `quest_state`, `split_quest_state_ref`,
`split_quest_state_mut`, `lifecycle`, `world_entities_mut`,
`world_test_consumer_like_cpp` (`cfg!(test)`),
`player_quest_gameplay_snapshot_like_cpp`,
`mutate_player_quest_gameplay_like_cpp`,
`invalidate_player_quest_status_authority_like_cpp`, `can_take_quest`,
`represented_quest_is_important_like_cpp`,
`represented_inventory_item_counts_like_cpp` and
`represented_gameobject_questgiver_can_interact_with_like_cpp` (the
`QuestStateCxRef` read the body made). `player_guid` and `send_packet`
come from `SessionHubHostLikeCpp`, as in earlier slices.

| Part | New trait | New adapter | Moved function lines | Share |
|---|---:|---:|---:|---:|
| 6a | 39 | 65 | 2,123 | **4.9 %** |

### Await counts (per function, base vs head)

| Function | Base | Head | Awaited, in order |
|---|---:|---:|---|
| `load_player_quests` | 8 | 8 | active statuses; objectives; rewarded; stale-rewarded migration save (per quest); daily; weekly; monthly; seasonal |
| `save_represented_quest_status_like_cpp` | 1 | 1 | `save_quest_to_db` |
| `save_changed_represented_quest_statuses_like_cpp` | 1 | 1 | the quest crate's changed-status save |
| `save_represented_quest_statuses_completed_after_like_cpp` | 1 | 1 | the status save, per completed quest |
| `save_quest_to_db` | 1 | 1 | the quest crate's status save |
| the other 47 moved functions | 0 | 0 | — |

Every World caller awaits at the same point (the source-item storage,
reward flow, acceptance and the confirm-accept local state); the
continue-login and loot host forwards return the moved future unchanged.

### Behaviour (#169)

- **Persistence order is unchanged.** The login load reads the same rows
  in the same order, installs them through the same canonical Player
  mutations, migrates stale rewarded rows with the same save before the
  daily load, and logs the same summary. Quest status saves run at the
  same points of accept, confirm-accept, objective progress, source-item
  storage and reward, in the same order relative to the item and money
  persistence fences.
- **Packets are unchanged:** `SMSG_QUEST_GIVER_QUEST_LIST_MESSAGE`,
  `…_QUEST_DETAILS`, `…_REQUEST_ITEMS` and `…_OFFER_REWARD_MESSAGE` are
  built from the same fields at the same branch of `SendPreparedQuest` and
  the query.
- **Registration and processing flags are untouched** (no registrar or
  opcode file changed).
- **Fixture branches:** the World `#[cfg(test)]` branches and `cfg!(test)`
  arguments now read `world_test_consumer_like_cpp`, which is the World
  crate's `cfg!(test)`. Production builds compile neither branch, as
  before; the World test build runs them, as before; another crate's
  `test-fixtures` build sees `false`, as the World `#[cfg(test)]` did.

### Recursion

- Every moved name was grepped across the production (non-shim)
  `wow-world` files. What remains are the eleven same-named trait-impl
  definitions (loot ×2, continue-login, item, entities ×2, aura, quest
  query, login, chat ×2) and two quest-query forwards under other names
  (`complete_quest_can_reward_…`, `complete_quest_can_complete_repeatable_…`);
  each body names `wow_world_quest::…(self, …)`.
- The adapter's six operation forwards use `WorldSession::name(self, …)`
  and resolve to inherents that are still defined
  (`session/quest/state.rs`, `session/quest/giver.rs`,
  `handlers/quest/eligibility.rs`, `session/player_items/storage.rs`); the
  splits and the interaction read name the World split functions.
- `unconditional_recursion`: 0 in the three canonical builds.

### Edges and closure

No crate edge is added or removed: no `Cargo.toml` and no `Cargo.lock`
change. `wow_world_closure.py check`: PASS in lock and metadata modes (18
packages), before and after. Only application, loot-ops and `wow-world`
reach quest. `cargo tree -p world-server -e normal,build -e features`
shows no `test-fixtures` and no `wow-world-test-support`.

### Accounting

- **`wow-world` production (classifier): 40,743 → 38,509 (−2,234).**
  Physical 49,349 → 47,335 (−2,014); 465 files (two deleted, the adapter
  and the shim added).
  - **Moved:** 2,205 production lines. That is the 52 functions with
    their doc comments (2,123) and the moved items (82): the dialog types,
    seasonal types, log-row helper and menu-icon constants of
    `quest_dialog.rs`/`session/mod.rs` and
    `ItemTransferQuestPersistencePlanLikeCpp`.
  - **Reclassified, not production reduction:** 35. These are the two
    nested `#[cfg(test)]` readers, now in the `#![cfg(test)]` shim.
  - **Retired:** 88. That is the rewards-block wrapper (6), the shells of
    the two deleted files (20), the four World quest-objective constant
    definitions (the bodies import the identical `wow_constants::quest`
    ones), and the imports, re-exports and emptied impl shells the moved
    bodies alone used.
  - **Added:** the adapter (65) and the caller rewrites (net +29 from
    rustfmt: `wow_world_quest::name(self, …)` is longer than
    `self.name(…)`).
- **Destination:** `wow-world-quest/src` 12,179 → 14,705 physical
  (+2,526): `quest_session/` 2,494 and the `lib.rs` re-export (+32);
  classifier production 11,597 → 14,123, 47 → 54 files. Over the 2,205
  moved lines the growth is the module headers and imports, the host
  trait and the generic signatures.
- `net_move.py check --base origin/3.4.3`: S = 2,014, G = 2,526,
  allowance 3,075.7, PASS, 0 duplicate bodies.
- **The remaining quest family in `wow-world`:** 5,429 production lines
  (from 7,665), with the 65-line adapter.

### Policy and ceilings

- **Hotspots** (`runtime-ownership-ledger.json`, tightened to the live
  values, none raised):
  - `session/mod.rs`: production 23,247 → 21,964, test 141,645 →
    141,644, total 164,892 → 163,608;
  - `handlers/quest/mod.rs`: production 5,501 → 4,537 (live base 5,494),
    test 11,253, total 16,754 → 15,790;
  - `handlers/loot/mod.rs`: production 1,378 → 1,377, total 20,307 →
    20,306 (the loot host's two forwards; the extra argument line is
    offset by merging its three `std` imports).
  - Test lines do not grow: the names only the scenarios read join
    `session/test_reexports.rs` (`ResetSeasonalQuestStatusReasonLikeCpp`,
    `SeasonalQuestStatusDbRowLikeCpp` and the quest-objective constants).
    This is offset by replacing the five `#[cfg(test)]` quest-objective
    constants of `session/mod.rs` with the identical `wow_constants::quest`
    re-exports there. Likewise, the two quest statuses only the quest
    scenarios read become one `#[cfg(test)]` import in
    `handlers/quest/mod.rs`, offset by turning its two `#[cfg(test)]`
    objective-type constants into aliases in the existing `#[cfg(test)]`
    `wow_constants::quest` group. The values are the same (1, 4, 6, 7, 8,
    16, 17, 18).
- **Physical:** `session/mod.rs` 874 → 845; new ratcheted row
  `handlers/quest_session_test_shims.rs` at 219.
- **`session-ownership-policy.json`** is reprinted with
  `print-baseline`. The delta is the moved and reclassified items, the 14
  adapter methods and the shim forwards. `session-ownership-check check
  --syntax-only` passes.
- No dependency-policy, net-move-policy or fixture change beyond the
  recipe commit's fixture.

### Warnings and tests

Canonical recipe (above), base `18cb95eff` vs head:

| Build | `wow-world` lib | `wow-world` lib test | `wow-world-quest` | All `wow-world*` lib units |
|---|---:|---:|---:|---:|
| production lib | 142 → 136 | — | 23 → 27 | 265 → 263 |
| lib + `test-fixtures` | 150 → 144 | — | 15 → 17 | 230 → 226 |
| `--lib --tests` | 150 → 144 | 197 → 193 | 15 → 17 | 427 → 419 |

- **No new warning.** Compared by crate, file, lint and message:
  - the two `session/quest_dialog.rs` unused imports (`InventoryResult`,
    `quest`) are gone with the imports;
  - the completed-quest-bit functions' pre-existing `unused_mut` (2) and
    `unused_variables` (`quest_state`, 2) move with them from
    `session/quest/rewards.rs` to `quest_session/rewards.rs`, same
    messages. In the `test-fixtures` builds only the two `unused_mut`
    remain there, because the fixture branch now compiles under
    `test-fixtures`.
- `wow-world` `dead_code`: 0 in all three builds. `unconditional_recursion`:
  0.
- **Tests.** No test file changed. The test-name multiset is identical:
  10,214 `#[test]` functions and 5 ignored. Feedback run before final:
  `cargo test -p wow-world --lib -- quest seasonal void_storage loot
  item_transfer bank`: 848 passed.

### 6b plan

Re-measure on the 6a merge. Move objective progress (`objectives.rs`), the
source-item storage, the objective drain with
`QuestObjectiveOperationLikeCpp`, the reward controller and validation,
the eligibility context and the async completion wrappers to
`quest_session`. The host gains the item-storage and template operations
the source-item storage calls, `sync_player_registry_state_like_cpp`,
`update_visible_gameobjects_or_spell_clicks_like_cpp` and the condition
projection: about 18 methods for about 1,900 lines. `can_take_quest` and
`represented_quest_is_important_like_cpp` leave the host when eligibility
moves.

### Follow-ups

- 6c (plan above); then the quest-bound world-entity leftovers.
- The new shim retires with the others when the scenarios call the free
  functions directly.
- The six quest-crate-name adapters (snapshot, mutate, invalidate, slot
  search, log entries, log-slot update) and the pending-sharing reads
  retire when their last World caller moves.
- #1403 follow-ups 3–5 (the `final` budget, Q2 and the trainer split, the
  two long lines) are unchanged by this slice.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section. Its manifest path,
dirty flag and duration are in the PR body. This section is not edited
after that run, so the PR head is the validated SHA.

## Slice 6b result (item-objective progress, source-item grant, reward-inventory admission and eligibility → quest)

Branch `1363-s6b-quest`, base `ac3d396b7` (6a merged). Move-only (R2) with
the slice 4–13 rewrites, after a tooling commit for the #1404 review nits.

### Re-measure and the cut

The 6b family measured on the base: 54 inherent methods (1,674 function
lines) in `handlers/quest/{objectives,source_items,eligibility,state}.rs`,
`handlers/quest/rewards/**` and `session/quest/**`. Three parts of the 6b
plan do not move, and the cut is 28 functions (**1,297 function lines**) behind
**16 new host methods net** (18 added, 2 removed):

- **The objective drain and the reward commit stay with the host.**
  `quest_objective_operation_like_cpp` and
  `reward_represented_quest_with_generator_like_cpp` build
  `QuestRewardCx`/`QuestObjectiveProgressCx` from ten disjoint `WorldSession`
  fields (inventory, lifecycle, quest state, catalogs, config, social, loot,
  world entities, core and the fixtures split). The body *is* that split, so
  moving it would need a host method returning the split, which is the body
  itself. They are the host's objective-completion operation:
  `complete_represented_quest_after_objective_with_generator_like_cpp`
  becomes a host method, and the drain wrapper and
  `QuestObjectiveOperationLikeCpp` stay with them.
- **`quest_source_item_quest_log_item_id_like_cpp` stays** for the same
  reason as 6a's POI store: it writes the session catalog cache
  (`cache_item_template_addon_quest_log_item_id_like_cpp` takes
  `&mut SessionCatalogs`), which the hub lends only shared. It becomes a
  host method of the item-storage sub-trait.
- **The pending-sharing accessors and the adventure-map record** of
  `session/quest/state.rs` go with their handlers in 6c.

### What moved

**To `wow-world-quest/src/quest_session/`**:

- `objectives.rs` (522): `ItemAddedQuestCheck` with and without the bound
  filter (`apply_quest_item_added_objective_progress_with_generator_like_cpp`,
  `…_filtered_like_cpp`, the non-bound source-item form), the bound and
  non-bound added-item state transitions and their publication,
  `ItemRemovedQuestCheck`, `SendQuestUpdateAddItem`
  (`send_quest_bound_item_update_like_cpp`) and the source-item bound
  objective progress and preflight;
- `source_items.rs` (410): `store_quest_source_item_with_generator_like_cpp`
  (the quest-acceptance `StoreNewItem` path) and
  `QuestSourceItemStoreOutcomeLikeCpp`;
- `reward_validation.rs` (215): the reward-choice and quest-package checks,
  `send_quest_failed_like_cpp`, the reward inventory plan and
  `CanRewardQuest(quest, rewardType, rewardId, true)`
  (`represented_can_reward_quest_inventory_like_cpp`), with
  `QuestChoiceItemLikeCpp`;
- `eligibility.rs` (134): the giver status, `represented_quest_is_important_like_cpp`
  with the dialog classification, `CanTakeQuest`, `CanSeeStartQuest`, the
  eligibility-context builder and `is_quest_disabled_like_cpp`;
- `state.rs` (+88): the 6a deferrals
  `complete_represented_quest_after_add_with_generator_like_cpp` and
  `add_quest_confirm_accept_local_state_like_cpp`, and the login aura-source
  predicate `represented_quest_login_aura_sources_are_hit_inert_like_cpp`
  with its two quest-flag constants (`QUEST_FLAGS_PLAYER_CAST_ACCEPT`,
  `QUEST_FLAGS_EX_RECAST_ACCEPT_SPELL_ON_LOGIN`, same values);
- `quest_session.rs` (+88 net): the module mounts, imports and the host
  traits; `lib.rs` (+22 net): the re-exports.

**Rewrites, beyond the 6a ones** (`self` → `host`, `self.catalogs` →
`host.hub_ref().catalogs`, `self.core`/`lifecycle`/`quest_state` → the host
forms with the argument `&` dropped, `wow_world_quest::` → `crate::`, a call
between moved functions becomes a free call, `cfg!(test)` →
`host.world_test_consumer_like_cpp()`): `self.fixtures` →
`host.hub_ref().fixtures` in the two `#[cfg(any(test, feature =
"test-fixtures"))]` eligibility arguments (the quest crate's
`test-fixtures` forwards the core one the `HubRef` field is gated on);
`crate::session::cx_inventory_ref(self).make_inventory_item_object(` →
`host.make_inventory_item_object(` (the context read becomes the host method
of that name, as 6a did for the gameobject quest-giver read);
`WorldSession::represented_quest_dialog_classification_like_cpp` (an
associated function) → the free function, and its return path
`crate::handlers::quest::dialog_status::QuestDialogClassificationLikeCpp` →
`crate::QuestDialogClassificationLikeCpp` (the World path was a re-export of
it). The rules are committed as data:
`tools/architecture/move_token_rules/1363-s6b.json` with the 28 pairs in
`1363-s6b.pairs.tsv`.

**Inlined one-line wrappers (each its exact body; none async):**

- `self.item_storage_template(id)` ×1 → `host.hub_ref().catalogs.item_storage_template(id)`;
- `self.item_template_flags3(id)` ×1 → `host.hub_ref().catalogs.item_template_flags3(id)`;
  the World wrapper retires (no caller left);
- `self.item_template_max_durability(id)` ×1 → `host.hub_ref().catalogs.item_template_max_durability(id)`;
- `self.item_template_inventory_type(id)` ×1 → `host.hub_ref().item_template_inventory_type(id)`;
- `self.send_equip_error(…)` ×6 → `host.core().send_equip_error(…)`.

**Token comparison** (`tools/architecture/move_token_compare.py --base
ac3d396b7 --pairs … --rules …`; rules apply outside literals and comments,
so a changed string stays a difference): 12 of 28 functions are identical.
The other 16 differ only by: the item-storage bound
(`QuestItemStorageHostLikeCpp` instead of `QuestSessionHostLikeCpp`, 10
functions); `pub(crate)` for the eight functions only the crate calls; the
generic order and `host: &H` of `with_quest_eligibility_cx_like_cpp<R, H>`
(its `<R>` signature is outside the rule); no host parameter on the dialog
classification (it never took `self`); and rustfmt unwrapping two
single-expression closures from `{ … }` (`can_take_quest`, the completion
check in the filtered progress). No string literal, constant, operator or
control-flow token changed.

**World side:**

- `session/quest/host.rs` (65 → 81): the six new `QuestSessionHostLikeCpp`
  forwards, without `can_take_quest`, `represented_quest_is_important_like_cpp`
  and `world_test_consumer_like_cpp`;
- `handlers/quest/item_storage_host.rs` (94, new): the
  `QuestItemStorageHostLikeCpp` adapter, beside the existing quest-query host
  adapter, so the session tree does not grow;
- every production caller names the free function, including the
  trait-impl forwards of the loot (×2), item, storage-move (×2), quest-query
  and chat paths; `handlers/character/query.rs` imports
  `get_represented_quest_giver_status_with_catalog_like_cpp` once;
- `handlers/quest_item_test_shims.rs` (92, `#![cfg(test)]`, mounted from
  `handlers/mod.rs`): the four `#[cfg(test)]` wrappers nested in
  `handlers/quest/objectives.rs` and `handlers/quest/state.rs`, verbatim
  except that their one call names the moved function, and the
  `can_take_quest`/`apply_quest_item_removed_like_cpp` forwards the scenarios
  call;
- `handlers/quest/source_items.rs`, `handlers/quest/rewards/validation.rs` and
  `handlers/quest/dialog_status.rs` (a re-export only the classification
  read) are deleted; `handlers/quest/objectives.rs` keeps only the
  catalog-cache lookup.

**Visibility widened** (each with its concrete consumer):
`QuestChoiceItemLikeCpp`'s three fields become `pub` (the World packet
reader builds it, the reward commit and choice match read it, the scenarios
build it); `apply_quest_source_item_bound_objective_progress_for_object_with_generator_like_cpp`
is `pub` (the scenarios' wrapper in the shim calls it); the two quest-flag
constants are exported for the scenario that reads them through
`session/test_reexports.rs`.

### Host surface

`QuestSessionHostLikeCpp` gains
`complete_represented_quest_after_objective_with_generator_like_cpp`
(async, `impl Future<Output = bool> + Send`),
`sync_player_registry_state_like_cpp`,
`update_visible_gameobjects_or_spell_clicks_like_cpp`,
`send_represented_quest_log_slot_update_like_cpp`,
`player_condition_projection_cx_like_cpp` and
`player_target_spell_is_hit_inert_like_cpp`, and loses `can_take_quest` and
`represented_quest_is_important_like_cpp` (now moved free functions, which
the 6a dialog and reward bodies call as such). The new sub-trait
`QuestItemStorageHostLikeCpp: QuestSessionHostLikeCpp` carries the
inventory operations of the item-objective, source-item and reward-inventory
bodies: `quest_source_item_quest_log_item_id_like_cpp` (async),
`get_inventory_item_by_pos`, `resolved_inventory_item_like_cpp`,
`resolved_inventory_item_object_like_cpp`,
`represented_non_bank_item_count_like_cpp`,
`apply_inventory_item_object_updates_like_cpp`,
`insert_inventory_item_like_cpp`, `insert_inventory_item_object`,
`make_inventory_item_object`, `send_new_item_plan`,
`send_player_values_update_from_entity_bridge` and
`plan_store_new_direct_inventory_item`. Ten moved functions are bound by
the sub-trait, the other eighteen by the main trait.

| Part | New trait | New adapter | Moved function lines | Share |
|---|---:|---:|---:|---:|
| 6a | 39 | 65 | 2,123 | 4.9 % |
| 6b | 63 (main +14 net, sub-trait 49) | 110 (`host.rs` +16, `item_storage_host.rs` 94) | 1,297 | **13.3 %** |

**Counting method (#1405 review nit 3).** "Function lines" here and in 6a are
each moved function's lines from its first doc-comment or attribute line
through its closing brace, which is what `move_token_compare.py` extracts.
Counted from the `fn` line instead, the same 28 functions are 1,254 lines at
the base, and the 6b share is 173 / 1,254 = **13.8 %**. Both are under 15 %.

**#1404 nit 1 (fold `world_test_consumer_like_cpp`).** The value is equal
in all three builds: production — World `cfg!(test)` is false, and
`test_fixtures_ref_like_cpp` does not exist (it is gated on the core
`test-fixtures`); the `test-fixtures` lib — `cfg!(test)` is false and the
World override of `test_fixtures_ref_like_cpp` is `#[cfg(test)]`, so the
trait default returns `None`; the World test build — `cfg!(test)` is true
and the override returns `Some`, and there the quest crate's
`test-fixtures` is on (World dev-dependency). Because the accessor does not
exist in production, the four unconditional call sites cannot name it
directly; the fold is therefore a provided trait method that reads it under
`#[cfg(any(test, feature = "test-fixtures"))]` and is `false` otherwise, and
the World override is deleted. The predicate now has one source.

### Await counts (per function, base vs head)

| Function | Base | Head | Awaited, in order |
|---|---:|---:|---|
| `store_quest_source_item_with_generator_like_cpp` | 8 | 8 | log-item id; bound preflight; status save (per changed quest); completed-after save (no-grant); grant transaction; non-bound progress; status save (per quest); completed-after save |
| `apply_quest_source_item_bound_objective_preflight_with_generator_like_cpp` | 2 | 2 | bound progress for the entry; for the log item |
| `add_quest_confirm_accept_local_state_like_cpp` | 2 | 2 | complete-after-add; status save |
| `apply_quest_item_added_objective_progress_filtered_like_cpp` | 1 | 1 | complete-after-add (per quest) |
| `apply_quest_source_item_bound_objective_progress_for_object_with_generator_like_cpp` | 1 | 1 | complete-after-add (per quest) |
| the two `…_with_generator_like_cpp` added-item wrappers | 1 | 1 | the filtered progress |
| `complete_represented_quest_after_add_with_generator_like_cpp` | 1 | 1 | the host objective-completion operation |
| the other 20 moved functions | 0 | 0 | — |

Every World caller awaits at the same point (acceptance, confirm-accept,
reward flow, spell-effect progress, storage move); the loot host forwards
return the moved future unchanged, and the storage-move forward awaits it
inside the same `async move`.

### Behaviour (#169)

- **Objective progress is unchanged:** the same status walk, bound filter,
  completion loop and `SMSG_QUEST_UPDATE_COMPLETE`, the same quest-bound
  `SMSG_ITEM_PUSH_RESULT` (display type 3, direct or group delivery by
  `DontReportLootLogToParty`), and the registry sync last.
- **The source-item grant is unchanged:** the bound preflight and its saves
  first; GUID allocation, the destination walk and the `QuestItemGrant`
  transaction before any in-memory change; then stack updates, new items,
  `SMSG_UPDATE_OBJECT` creates, stack-count updates and the player values
  update; then the non-bound progress, the status saves and the item push
  result, in the base order. Every failure sends the same equip error.
- **Reward admission and eligibility are unchanged:** the same checks in
  the same order with the same `SMSG_QUEST_GIVER_QUEST_FAILED` and equip
  errors; `CanTakeQuest`/`CanSeeStartQuest` evaluate the same eligibility
  context.
- **Packets, registration and processing flags** are untouched (no
  registrar or opcode file changed).

### Recursion

- Grepping every moved name across the production (non-shim) `wow-world`
  files leaves three same-named trait-impl definitions (the loot host ×2 and
  the item host); each body calls `wow_world_quest::…(self, …)`. Every other
  reference names the free function.
- The 18 new adapter forwards use `WorldSession::name(self, …)` and resolve
  to inherents that are still defined (`session/quest/objective_drain.rs`,
  `session/player_registry_binding.rs`, `session/visibility/refresh.rs`,
  `handlers/quest/state.rs`, `session/player_condition_values.rs`,
  `session/spell_state/spell.rs`, `handlers/quest/objectives.rs`,
  `session/player_items/{storage_slots,storage,bank,persistence}.rs`,
  `session/publication/operations.rs`); `make_inventory_item_object` calls
  the inventory-context method. Inherent methods win over the same-named
  `VendorSessionHostLikeCpp` methods in `WorldSession::name` paths.
- `world_test_consumer_like_cpp` is now a trait default that `WorldSession`
  does not override.
- `unconditional_recursion`: 0 in the three canonical builds.

### Edges and closure

No crate edge is added or removed: no `Cargo.toml` or `Cargo.lock` change
(quest already depends on inventory, player-conditions, entities and core).
`wow_world_closure.py check`: PASS in lock and metadata modes (18
packages).

### Accounting

- **`wow-world` production (classifier): 38,509 → 37,246 (−1,263).**
  Physical 47,335 → 46,169 (−1,166); 465 → 464 files (three deleted, the
  item-storage adapter and the shim added).
  - **Moved:** 1,310 production lines: the 28 functions with their doc
    comments (1,297), the two moved types (11) and the two quest-flag
    constants (2).
  - **Reclassified, not production reduction:** 59, the four nested
    `#[cfg(test)]` wrappers now in the `#![cfg(test)]` shim.
  - **Retired:** 37: the `item_template_flags3` wrapper (3),
    `dialog_status.rs` (6), the headers, module mounts, empty impl shells and
    blank separators of the deleted and emptied files, the imports only the
    moved bodies read (dropped, or made `#[cfg(test)]` where the scenarios
    still read them), and the offsets below.
  - **Added:** 143: the adapters (110) and the caller rewrites (33, rustfmt
    wrapping of `wow_world_quest::name(self, …)`).
- **Destination:** `wow-world-quest/src` physical 14,705 → 16,184 (+1,479),
  classifier production 14,123 → 15,602, 54 → 58 files. Over the 1,310
  moved lines the growth is the module headers and imports, the two host
  traits and the generic signatures.
- `net_move.py check --base origin/3.4.3`: S = 1,166, G = 1,479, allowance
  2,185.3, PASS, 0 duplicate bodies.
- **The remaining quest family in `wow-world`:** 4,149 production lines (from
  5,418 on this base), including the two adapters (175).

### Policy and ceilings

- **Hotspots** (`runtime-ownership-ledger.json`, tightened to the live
  values, none raised): `session/mod.rs` production 21,964 → 21,941, test
  141,644 → 141,643, total 163,608 → 163,584; `handlers/quest/mod.rs`
  production 4,537 → 3,290, test 11,253 → 11,252, total 15,790 → 14,542.
  `handlers/character/mod.rs` and `handlers/loot/mod.rs` stay at their
  ceilings: the caller rewrites there (+3 and +2) are offset by merging the
  character `gossip`/`misc`/`quest` packet imports into one line and
  dropping its `BindPointUpdate` import (−3), which was redundant with the
  module's glob import rather than unused (#1405 review nit 4), and by
  dropping the
  loot host's `std::future::Future` import, which the 2024 prelude provides
  (−2). Test lines do not grow: the item names the quest scenarios still
  read join the existing `#[cfg(test)]` `wow_constants` import,
  `InventoryItem` joins the scenarios' own `crate::session` import, seven
  imports the test build reports unused in `quest_tests.rs` and one in
  `creature_aura_application.rs` are dropped, and the two quest-flag
  constants join the `wow_world_quest` group of `session/test_reexports.rs`.
- **Physical:** `session/mod.rs` 845 → 843, `handlers/quest/rewards.rs`
  11 → 10, `unit_tests/handlers/quest_tests.rs` 982 → 979; new ratcheted row
  `handlers/quest_item_test_shims.rs` at 92.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`; the
  delta is the moved items, the new adapter methods and the shim forwards.
  `session-ownership-check check --syntax-only` passes.
- No dependency-policy, net-move-policy or fixture change.
- **Tools** (#1404 nits 2 and 3): `tools/architecture/warning_units.py`
  (`count`, `compare`, `self-test`) and
  `tools/architecture/move_token_compare.py` (`--base/--pairs/--rules`,
  `self-test`), each self-test passing; the recipe commands gain `--locked`.

### Warnings and tests

Canonical recipe (with `--locked`), base `ac3d396b7` vs head, counted with
`warning_units.py`:

| Build | `wow-world` lib | `wow-world` lib test | `wow-world-quest` | All `wow-world*` lib units |
|---|---:|---:|---:|---:|
| production lib | 136 → 135 | — | 27 → 27 | 263 → 262 |
| lib + `test-fixtures` | 144 → 143 | — | 17 → 17 | 226 → 225 |
| `--lib --tests` | 144 → 143 | 193 → 189 | 17 → 17 | 419 → 414 (with the lib test) |

- **No new warning.** `warning_units.py compare` by unit kind, lint, file and
  message reports nothing only in the head in any build. Only in the base:
  the glob-redundant `BindPointUpdate` import (production and both test builds) and, in the
  lib test, the two `quest_tests.rs` import groups and the
  `creature_aura_application.rs` import dropped above. The moved bodies carry
  no warning.
- `wow-world` `dead_code`: 0 in all three builds. `unconditional_recursion`: 0.
- **Tests.** The test-name multiset is identical: 10,214 `#[test]`/`#[tokio::test]`
  functions, 5 ignored. Only imports changed in the two test files.
  Feedback runs before final: `cargo test -p wow-world --lib -- quest item
  void_storage loot bank vendor spell_hit login chat character query`: 1,523
  passed; `cargo test -p wow-world-quest`: passed.

### 6c plan

Re-measure on the 6b merge. Move the handler entry points
(`handlers/quest/handlers/**`, `handlers/quest/handlers.rs`: hello, status,
query, accept, confirm-accept, request-reward, choose-reward, POI, push to
party, adventure map), the sharing helpers (`handlers/quest/sharing.rs`),
the `mod.rs` helpers (choice reader and match, the push-eligibility helpers,
the POI store builder), the pending-sharing accessors and the adventure-map
record of `session/quest/state.rs`, so the registrar calls the moved bodies
and `handlers/quest/handlers/host.rs` retires. Measured on this head the
handler files are 2,371 production lines and the sharing helpers 46, so
about 2,500 with the helpers; the host
gains the group and player registries, realm send, gossip menu and the
catalog-cache writes (POI store, quest-log item id), which then lets
`quest_source_item_quest_log_item_id_like_cpp` move too. The objective
drain and reward commit stay host operations until the session owners they
lend are reachable from the hub.

### Follow-ups

- 6c (above); then the quest-bound world-entity leftovers.
- The two shims retire when the scenarios call the free functions directly.
- The quest-crate-name adapters of 6a (snapshot, mutate, invalidate, slot
  search, log entries, log-slot update) and the pending-sharing reads retire
  with their last World caller.
- #1403 follow-ups 3–5 are unchanged by this slice.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section. Its manifest path,
dirty flag and duration are in the PR body. This section is not edited
after that run, so the PR head is the validated SHA.

## Slice 6c result (quest packet entry points, sharing, POI store and the quest-log item-id lookup → quest)

Branch `1363-s6c-quest`, base `5c04ace6b` (6b merged). Move-only (R2) with
the slice 4–13 rewrites, after a tooling commit for the #1405 review
follow-ups (`warning_units.py count` exits 1 when it counted a compiler
error; `final` runs the `warning_units` and `move_token_compare` self-tests
when either file changes, pinned in `tools/test_validation_v2.py`). Because
the slice changes `tools/validation-v2`, `final` runs `test_validation_v2.py`.
The self-test inherited the recipe's `VALIDATION_V2_TIMEOUT_SECONDS=1800` and
failed its default-timeout pin. That failure was pre-existing and is unrelated
to the change, so the self-test now drops the variable from its fixture
environment, like `VALIDATION_V2_CARGO_JOBS`.

### Re-measure and the cut

On the base the 6c family was the handler files (`handlers/quest/handlers.rs`
and `handlers/quest/handlers/**`, 2,371 production lines), the sharing
helpers (46), the `mod.rs` helpers, the POI store load, the quest-log item-id
lookup, the quest-giver source checks and the pending-sharing install. The
cut is **31 functions, 2,588 function lines** (counted from the first doc
comment or attribute line, as `move_token_compare.py` extracts; 2,547 from the
`fn` line), plus 29 constant lines and one 3-line orphan doc comment, behind
**9 new host methods** and one swapped method. About 2.6k, so no split.

What does not move, with the reason:

- **The reward commit and the objective drain** stay host operations (as in
  6b): `reward_represented_quest_with_generator_like_cpp` and
  `drain_represented_quest_objective_progress_with_generator_like_cpp` build
  their contexts from ten disjoint `WorldSession` fields. Both become host
  methods of the new trait.
- **`record_represented_adventure_map_start_quest_like_cpp`** is a World
  test recorder on the instances state (`#[cfg(test)]` body, empty in
  production); it becomes a host method.
- **The gossip menu** (`wow_world_character::build_gossip_menu` and the
  trainer gossip fallback) is reached through two host methods, so the quest
  crate gains no character edge.
- **`resolved_group_guid_like_cpp`** reads the World social split; host
  method.

### What moved

**To `wow-world-quest/src/quest_session/`** (lines are physical, after
rustfmt and the generic signatures):

- `giver_handlers.rs` (504): `HandleAdventureMapStartQuest`,
  `HandleQuestgiverStatusQueryOpcode`, `HandleQuestgiverHelloOpcode`,
  `HandleQuestgiverQueryQuestOpcode` and `HandleQuestPOIQuery`; the POI store
  load (`quest_poi_store_like_cpp`, the 6a deferral) and its builder; the
  query-quest reader; the status-query source
  (`ObjectAccessor::GetObjectByTypeMask`, from `handlers/quest/eligibility.rs`)
  and the accept/involved source checks (`hasQuest`/`hasInvolvedQuest` with
  `CanInteractWithQuestGiver`, from `session/quest/giver.rs`, which the
  complete-quest host also calls);
- `accept_handlers.rs` (584): `HandleQuestgiverAcceptQuestOpcode`,
  `HandleQuestConfirmAccept`, the accept-quest reader and the pending-sharing
  install (`set_represented_pending_quest_sharing_like_cpp`, from
  `session/quest/state.rs`);
- `reward_handlers.rs` (375): `HandleQuestgiverRequestRewardOpcode`,
  `HandleQuestgiverChooseRewardOpcode`, the `QuestChoiceItem` reader, the
  loaded reward-choice match and `QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP`;
- `sharing.rs` (1,253): `HandlePushQuestToParty` (990 lines on its own), the
  two receiver-side session commands, the repeatable turn-in prompt
  (`send_repeatable_turn_in_request_items_like_cpp`, from
  `session/player_items/publication.rs`), the share helpers of
  `handlers/quest/sharing.rs`, the push-eligibility helpers of `mod.rs` and
  the 27 push-reason constants with `QUEST_FLAGS_SHARABLE_LIKE_CPP`;
- `source_items.rs` (+45): the quest-log item-id lookup
  (`quest_source_item_quest_log_item_id_like_cpp`, the 6b deferral);
- `quest_session.rs` (+77 net): the module mounts, imports and the new host
  trait; `lib.rs` (+29 net): the re-exports; `quest_handlers.rs` (−30 net):
  the registrar's host trait and thunks (below).

**Rewrites, beyond the 6a/6b ones:** `self.quest_state.record_…` and
`.fixture_set_…` → `host.quest_state_mut()` (and `session.quest_state.record_…`
in the confirm-accept closure); `self.catalogs.quest_poi_store_like_cpp =` and
`self.catalogs.cache_item_template_addon_quest_log_item_id_like_cpp` →
`host.catalogs_mut()`; `self.hub`/`self.world_entities` (the `QuestStateCxRef`
methods) → `host.hub_ref()`/`host.world_entities()`;
`crate::session::split_world_entities_mut(self)` →
`host.split_world_entities_mut()`; `crate::session::cx_quest_state_ref(self).represented_quest_giver_*(` →
the moved free function; `wow_world_character::{build_gossip_menu, send_represented_creature_trainer_gossip_menu_like_cpp}(self, …)` →
the host methods; `crate::session::directory::PlayerQuestSharingSnapshot` and
`crate::session::mailbox::` → their `wow_world_core` paths;
`WorldSession::read_quest_choice_item_like_cpp`/`…_matches_loaded_type_like_cpp`
→ the free functions; `session: &mut WorldSession` → `session: &mut H`; a bare
`self` argument → `host`; `cfg!(test)` → `world_test`, read once at the top of
the two functions that record outcomes
(`let world_test = host.world_test_consumer_like_cpp();`, because
`host.quest_state_mut().record_…(host.…)` would borrow the host twice). The
`#[cfg(test)]` fixture branch of the pending-sharing install becomes
`#[cfg(any(test, feature = "test-fixtures"))]` gated on
`host.world_test_consumer_like_cpp()`, as 6a did. The rules are committed as
data: `tools/architecture/move_token_rules/1363-s6c.json` with the 31 pairs in
`1363-s6c.pairs.tsv`.

**Inlined wrappers (each its exact body; none async):**

- `self.player_registry()` ×2 → `host.core().player_registry()`;
- `self.group_registry()` ×2 → `host.core().group_registry()`;
- `self.send_packet_realm(…)` ×1 → `host.core().send_packet_realm(…)`;
- `self.send_equip_error(…)` ×2 → `host.core().send_equip_error(…)`;
- `self.item_storage_template(id)` ×1 → `host.hub_ref().catalogs.item_storage_template(id)`;
- `self.item_template_start_quest_id(id)` ×1 → `host.hub_ref().catalogs.item_template_start_quest_id(id)`;
- `self.represented_meets_player_condition_id_like_cpp(id)` ×1 →
  `wow_world_player_conditions::meets_player_condition_id_like_cpp(&host.player_condition_projection_cx_like_cpp(), host.hub_ref().catalogs.player_condition_store.as_deref(), id)`;
- `quest_has_represented_item_objective_like_cpp(quest)` ×1 →
  `crate::represented_quest_has_item_objective_like_cpp(quest)`; the World
  wrapper retires;
- `cx_quest_state_ref(self).quest_poi_persistence_port_like_cpp()` ×1 →
  `host.lifecycle().quest_poi_persistence_port_like_cpp().cloned()`; the
  World reader and the `QuestStateCxRef::lifecycle` field it alone read
  retire;
- `cx_inventory_ref(self).item_template_addon_catalog_persistence_port_like_cpp()` ×1 →
  `host.lifecycle().item_template_addon_catalog_persistence_port_like_cpp().cloned()`;
  the World reader stays (loot and spell call it);
- the three quest-crate-name adapters the moved bodies called, each its
  statements in place: `find_quest_slot_like_cpp` ×1 (the owner access and
  `crate::find_quest_slot_like_cpp(&owner, host.quest_state(), id, world_test)`;
  the World adapter retires), `represented_pending_quest_sharing_like_cpp` ×1
  (the quest-state split and the crate function) and
  `clear_represented_pending_quest_sharing_like_cpp` ×1 (the split, the crate
  function and the registry sync). The last two World wrappers have no
  production caller left and move verbatim to the test shim.

**Token comparison** (`move_token_compare.py --base 5c04ace6b --pairs … --rules …`):
**27 of 31 functions are identical** after the rules. The other four differ
only by the items above: `handle_quest_poi_query` (the inlined slot search),
`handle_quest_confirm_accept_with_generator_like_cpp` (the `world_test` read
and the two inlined pending-sharing wrappers),
`handle_push_quest_to_party` (the `world_test` read) and
`set_represented_pending_quest_sharing_like_cpp` (the fixture gate). No
string literal, comment, constant, operator or control-flow token changed.

**The registrar.** `register_quest_handlers_like_cpp` is unchanged: the same
eleven entries in the same order with the same opcode, `SessionStatus`,
`PacketProcessing` and `handler_name`. Its thunks now call the moved bodies
(`crate::handle_…(session, …)`), and `QuestHandlerHostLikeCpp<C>` keeps its
name and the checker's exact `S: QuestHandlerHostLikeCpp<C> + Send, C: Sync`
shape. Its three methods are the catalog fields the legacy closures
destructured (`adventure_map_pois`, `quest_info`, `id_generators.item`) as
associated functions over `C`. Its supertraits are the complete-quest host,
the new session host and `Sync`, because the boxed futures hold a shared
session borrow across an await, as the World bodies did. So
`handlers/quest/handlers/host.rs` (133) retires. The handler-contract
constants, source mounts and `handler-module-policy.json` are unchanged,
because no registrar moved. `handler_composition.rs` is unchanged.

**World side:**

- `handlers/quest/quest_handler_host.rs` (97, new): the
  `QuestHandlerSessionHostLikeCpp` adapter and the three catalog accessors;
  `item_storage_host.rs` swaps its log-item-id forward for `catalogs_mut`;
- every production caller names the free function: the two session
  commands (`session_commands.rs`), the complete-quest host's involved-source
  check, and the log-item-id callers (the vendor host, the storage-move
  host, the void-storage withdrawal);
- `handlers/quest_handler_test_shims.rs` (264, `#![cfg(test)]`, mounted from
  `handlers/mod.rs`): the former `handlers/quest/handlers/test_shims.rs`
  verbatim; the six `#[cfg(test)]` entry wrappers verbatim except that their
  one call names the moved free function; the two pending-sharing readers;
  one-call forwards for the moved functions the scenarios call; and a
  `quest_handler_test_reexports` module with the moved constants, readers
  and POI builder and the names only the quest scenarios still read;
- deleted: `handlers/quest/handlers.rs`, `handlers/quest/handlers/**`,
  `handlers/quest/{sharing,persistence,objectives,eligibility}.rs`.
  `handlers/quest/mod.rs` keeps the shared imports and constants of the
  remaining adapters, and the test-only names move to the shim;
- the session re-exports only the moved bodies read leave `session/mod.rs`,
  `session/quest_dialog.rs` and `session/currency_adapter.rs`. The five that
  session scenarios still name join `session/test_reexports.rs`;
- tests: `quest_tests.rs` drops the `include_str!` of the deleted
  `handlers.rs` from the registration scan; `quest_tests/quest_6.rs` expects
  the moved thunk calls (`crate::handle_push_quest_to_party(session, pkt)`,
  `crate::handle_quest_giver_query_quest(session, pkt)` and
  `::handle_quest_giver_{accept_quest,request_reward,choose_reward}_with_generator_like_cpp(`)
  instead of the retired host-trait calls. Each check is the same `contains`
  on the same registrar source, so none is weaker. The two session giver
  test shims call the moved source checks.

**Visibility widened** (each with its concrete consumer): the handler entry
points, the two session commands, the source checks, the log-item lookup,
the pending-sharing install, the POI store and builder, the two packet
readers, the choice reader and match, and the constants are `pub`. Their
consumers are the World command dispatch, hosts and void-storage withdrawal,
and the World scenarios through the test shim. The share helpers and the
push-eligibility helpers stay private to `sharing.rs`.

### Host surface

`QuestHandlerSessionHostLikeCpp: QuestItemStorageHostLikeCpp` (9 methods):
`quest_state_mut`, `world_entities` (fields), `split_world_entities_mut`
(split), `resolved_group_guid_like_cpp`, `build_gossip_menu` (async),
`send_represented_creature_trainer_gossip_menu_like_cpp`,
`record_represented_adventure_map_start_quest_like_cpp`,
`reward_represented_quest_with_generator_like_cpp` (async) and
`drain_represented_quest_objective_progress_with_generator_like_cpp`
(async). `QuestItemStorageHostLikeCpp` trades
`quest_source_item_quest_log_item_id_like_cpp` for `catalogs_mut`.
`QuestHandlerHostLikeCpp<C>` shrinks from ten session methods to the three
catalog accessors.

| Part | New trait | New adapter | Moved function lines | Share |
|---|---:|---:|---:|---:|
| 6a | 39 | 65 | 2,123 | 4.9 % |
| 6b | 63 | 110 | 1,297 (1,254 from `fn`) | 13.3 % (13.8 %) |
| 6c | 68 (session host 46, `catalogs_mut` 3, registrar host 19) | 97 | 2,588 (2,547 from `fn`) | **6.4 %** (6.5 %) |

These are gross figures. The 6c host retires more than it adds: the old
registrar host trait (66 lines) and its World adapter `handlers/host.rs`
(133), plus the swapped method (4) and its adapter (3). So the net host
change is −41 lines.

### Await counts (per function, base vs head)

| Function | Base | Head | Awaited, in order |
|---|---:|---:|---|
| `handle_quest_confirm_accept_with_generator_like_cpp` | 4 | 4 | confirm-accept local state (no-grant branch); local state (grant branch); source-item store; local state (no source item) |
| `handle_quest_giver_accept_quest_with_generator_like_cpp` | 2 | 2 | complete-after-add; status save |
| `handle_quest_giver_request_reward_with_generator_like_cpp` | 2 | 2 | complete-after-add; completed-after save |
| `handle_quest_giver_choose_reward_with_generator_like_cpp` | 2 | 2 | reward commit; objective drain (boxed, as before) |
| `handle_quest_giver_hello` | 1 | 1 | gossip menu |
| `handle_quest_poi_query` | 1 | 1 | POI store load |
| `quest_poi_store_like_cpp` | 1 | 1 | POI rows load |
| `quest_source_item_quest_log_item_id_like_cpp` | 1 | 1 | item-template-addon row load |
| the other 23 moved functions | 0 | 0 | — |

Every caller awaits at the same point. The registrar thunks await the moved
future inside the same boxed `async move`. The vendor and storage-move hosts
return or await the moved future unchanged. The void-storage withdrawal and
the source-item grant await it where they awaited the World method.

### Behaviour (#169)

- **Packets and order are unchanged:** `SMSG_QUEST_GIVER_STATUS`, the
  gossip/trainer/quest-list fallback order of the hello, the query
  responses, `SMSG_QUEST_POI_QUERY_RESPONSE` on the realm connection,
  `SMSG_QUEST_GIVER_QUEST_COMPLETE` after the accept's save, registry sync
  and slot update, the offer-reward dialog, the push results to the sender
  and to each receiver, and the receiver commands. The equip errors and
  their limit-category argument are unchanged.
- **Persistence is unchanged:** accept and confirm-accept save the status at
  the same points; request-reward saves the completed-after statuses after
  `CompleteQuest`; choose-reward runs the reward commit and then the drain;
  the POI and item-template-addon caches are filled once, as before.
- **Registration and processing flags** are unchanged (above).
- **Fixture branches:** the pending-sharing fixture fallback runs only in
  the World test build, as the World `#[cfg(test)]` branch did.

### Recursion

- Grepping every moved name across the production (non-shim) `wow-world`
  files leaves one same-named trait-impl definition: the vendor host's
  `quest_source_item_quest_log_item_id_like_cpp`, which calls
  `wow_world_quest::…(self, entry_id)`. Every other reference names the free
  function.
- The adapter forwards use `WorldSession::name(self, …)` and resolve to
  inherents that are still defined (`session/social/group.rs`,
  `session/quest/state.rs`, `handlers/quest/rewards/controller.rs`,
  `session/quest/objective_drain.rs`). The split names the World split
  function, and the gossip forwards name the character crate's free
  functions. Inherent methods win over the same-named social, entities and
  spell-effect host methods in `WorldSession::name` paths.
- `unconditional_recursion`: 0 in the three canonical builds.

### Edges and closure

No crate edge is added or removed: no `Cargo.toml` or `Cargo.lock` change
(quest already depends on core, instances, social, lifecycle,
player-conditions and entities). `wow_world_closure.py check`: PASS in lock
and metadata modes (18 packages).

### Accounting

- **`wow-world` production (classifier): 37,246 → 34,219 (−3,027).**
  Physical 46,169 → 43,406 (−2,763); 464 → 455 files (eleven deleted; the
  adapter and the shim added).
  - **Moved:** 2,620 production lines. That is the 31 functions with their
    doc comments (2,588), the 29 constants and the 3-line orphan doc comment
    of the retired close-quest handler (kept with the POI handler, which
    rustc already attached it to).
  - **Reclassified, not production reduction:** 11, the two pending-sharing
    readers now in the `#![cfg(test)]` shim.
  - **Retired:** 498. That is the registrar adapter `handlers/host.rs` (133),
    the slot-search adapter (4), the item-objective wrapper (5), the POI-port
    reader and its impl shell, and the `QuestStateCxRef::lifecycle` field.
    The rest is the headers, module mounts and imports of the eleven deleted
    files, and the `mod.rs` and session imports and re-exports only the
    moved bodies read. That includes the `handlers/quest/mod.rs` imports
    the base already reported unused in production; the names the scenarios
    still read moved to the test shim.
  - **Added:** 102: the adapter (97) and the caller rewrites (+8), less the
    item-storage adapter's swap (−3).
- **Destination:** `wow-world-quest/src` physical 16,184 → 19,021 (+2,837),
  classifier production 15,602 → 18,437, 58 → 62 files. Over the 2,620
  moved lines the growth is the module headers and imports, the host trait
  and the generic signatures (rustfmt wraps the long `<H: …>` headers).
- `net_move.py check --base origin/3.4.3`: S = 2,763, G = 2,837, allowance
  3,862.2, PASS, 0 duplicate bodies.
- **The remaining quest family in `wow-world`:** 1,155 production lines (from
  4,149), of which 374 are the four adapters.

### Policy and ceilings

- **Hotspots** (`runtime-ownership-ledger.json`, tightened to the live
  values, none raised): `session/mod.rs` production 21,941 → 21,741, test
  141,643 → 141,641, total 163,584 → 163,382; `handlers/quest/mod.rs`
  production 3,290 → 457, test 11,252 → 11,252, total 14,542 → 11,709.
  The session test lines do not grow: the five quest/currency names the
  session scenarios still read join `session/test_reexports.rs`, offset by
  merging the six `cfg(test)` `battleground_adapter` re-exports and the four
  `cfg(test)` `duel_contracts` re-exports of `session/mod.rs` into one group
  each.
- **Physical:** `session/mod.rs` 843 → 825,
  `unit_tests/handlers/quest_tests.rs` 979 → 978; the
  `handlers/quest/handlers.rs` row retires with the file; new ratcheted row
  `handlers/quest_handler_test_shims.rs` at 264. `quest_session/sharing.rs`
  (1,253) is above the 1,000-line review budget and below the 2,000 default
  ceiling. Its push handler is one 990-line function, which a move-only slice
  cannot split. The World `handlers/quest/handlers/sharing.rs` it replaces
  was 1,029.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`; the
  delta is the moved items, the adapter methods, the shim forwards and the
  relocated direct-registry rows (the same nine rows, now under
  `wow-world-quest`). `session-ownership-check check --syntax-only` passes.
- No dependency-policy, net-move-policy, handler-module-policy or fixture
  change.

### Warnings and tests

Canonical recipe (with `--locked`), base `5c04ace6b` vs head, counted with
`warning_units.py` (which now fails on a counted error; none here):

| Build | `wow-world` lib | `wow-world` lib test | `wow-world-quest` | All `wow-world*` lib units |
|---|---:|---:|---:|---:|
| production lib | 135 → 125 | — | 27 → 27 | 262 → 252 |
| lib + `test-fixtures` | 143 → 133 | — | 17 → 17 | 225 → 215 |
| `--lib --tests` | 143 → 133 | 189 → 181 | 17 → 17 | 225 → 215 |

- **No new warning.** `warning_units.py compare` (with the moved
  `test_shims.rs` path mapped) reports nothing only in the head in any
  build. Only in the base: the ten `handlers/quest/mod.rs` import warnings
  of the production build and the eight of the lib test. The import
  rewrite drops those names, or moves them to the test shim where only the
  scenarios read them. The moved bodies carry no warning.
- `wow-world` `dead_code`: 0 in all three builds. `unconditional_recursion`:
  0.
- **Tests.** The test-name multiset is identical: 9,492
  `#[test]`/`#[tokio::test]` functions under `crates/` by this slice's
  count (the 6b figure used a wider tree), 5 ignored. Feedback runs before
  final: `cargo test -p wow-world --lib -- quest void_storage vendor loot item
  bank share push poi adventure gossip giver`: 1,208 passed; `cargo test -p
  wow-world-quest`: passed.
- **Pre-existing:** the handler-contract checker's
  `repository_handler_contract_passes` expects an owner list without
  `wow-world-player-conditions` (added by slice Q), so it fails at this base
  as well. This slice adds no package; the list is a follow-up.

### Follow-ups

- The quest-bound world-entity leftovers (kill credit, dynamic flags,
  spell-click visibility), and the reward commit and objective drain once
  the session owners they lend are reachable from the hub.
- The three quest shims retire when the scenarios call the free functions
  directly; the remaining quest-crate-name adapters (gameplay snapshot,
  mutate, invalidate, log entries, log-slot update) retire with their last
  World caller.
- #1405 nit 5: the twelve `QuestItemStorageHostLikeCpp` inventory operations
  move to an inventory-owned host. #1405 nit 6: the `'\u{..}'` char literals
  and multi-line attributes in `move_token_compare.py`. Run the per-slice
  token comparison in `final` (only the self-test is wired).
- `QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP` now exists as a public constant in
  `reward_handlers.rs` and as the private copy in
  `quest_query_handlers/complete_quest.rs`; consolidate them.
- The checker owner-list expectation above.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section. Its manifest path,
dirty flag and duration are in the PR body. This section is not edited
after that run, so the PR head is the validated SHA.

## Slice 6 summary (6a–6c)

The slice 6 quest family was 7,665 classifier production lines in 32
`wow-world` files on `18cb95eff` (slice 6 plan).

| Slice | `wow-world` production | Δ | Moved out (function lines) | Reclassified | New host (lines, share) |
|---|---|---:|---:|---:|---|
| 6a state, persistence, login quest load, giver dialog and publication | 40,743 → 38,509 | −2,234 | 2,123 (+82 items) | 35 | 104, 4.9 % |
| 6b item-objective progress, source-item grant, reward admission, eligibility | 38,509 → 37,246 | −1,263 | 1,297 (+13 items) | 59 | 173, 13.3 % |
| 6c packet entry points, sharing, POI store, log-item lookup | 37,246 → 34,219 | −3,027 | 2,588 (+32 items) | 11 | 165 gross (−41 net), 6.4 % |
| **Total** | **40,743 → 34,219** | **−6,524** | **6,008 (+127)** | **105** | **442 gross, 7.4 %** |

- **Destination:** `wow-world-quest` grew from 12,179 to 19,021 physical
  lines (classifier production 11,597 → 18,437). Its `quest_session` module
  now holds the represented quest state, persistence and login load, the
  dialog and publication, item-objective progress, the source-item grant,
  reward admission, eligibility and every quest packet entry point. Its
  users are still application, loot-ops and `wow-world`; no crate edge was
  added in 6a–6c.
- **Registration:** the eleven quest-giver registrations (and the query
  registrar) stayed in `wow-world-quest` in the same order with the same
  flags; 6c only pointed their thunks at the moved bodies and retired the
  World adapter.
- **#169.** Every await of every moved function is at the same point in the
  same order across the three parts; packet order, save points and the
  item/money fences are unchanged.
- **What quest code remains in `wow-world` (1,155 production lines), and
  why:**
  - the four host adapters (374): the session host, the item-storage host,
    the handler-session host with the registrar's catalog accessors, and the
    query/complete-quest host;
  - the **reward commit** (`handlers/quest/rewards/controller.rs`) and the
    **objective drain** (`session/quest/objective_drain.rs`), which build
    `QuestRewardCx`/`QuestObjectiveProgressCx` from ten disjoint session
    fields and stay host operations until those owners are reachable from
    the hub;
  - the quest-crate-name adapters over the canonical Player quest state
    (gameplay snapshot, mutate, invalidate the status authority, the
    quest-log create entries and slot update), which many World callers
    still use;
  - the gameobject quest-giver interaction context
    (`session/quest_interaction.rs`), the seasonal-change flag, the
    rewarded talent points and the quest store setters (the world-server
    construction API);
  - the quest-bound **world-entity leftovers** (kill credit, gameobject
    dynamic flags, spell-click visibility): host methods of the entities and
    spell hosts that read quest state. They were never part of this family.

**What remains for #1363 overall:**

- **Q2:** `inventory_scaling` and `equipment_set_use` from application to
  `wow-world-inventory` (about −723 application lines; Q2 plan above).
  Done: see the Q2 result below.
- **4c:** the deferred families (collections, group, travel, spell, void
  storage with its unlock commit, the trainer context, talent).
- **Slice 12:** the base-module logic and the thunk sweep. The World adapters
  (the four quest hosts among them), the split forwards and the context
  builders retire as `WorldSession` is split.
- **Cleanup:** retire the `#![cfg(test)]` shim files (three for quest) by
  converting the scenario call sites, retire the remaining public World
  forwards with their world-server and integration-test callers, and
  consolidate the duplicate host names and constants.

## Slice Q2 result (`inventory_scaling` and the equipment-set use registrar → `wow-world-inventory`)

Branch `1363-sQ2-inventory`, base `bc5bc0be7` (slice 6c merged). Move-only
(R2), after a tooling commit for the #1406 review follow-up 1. Inventory
scaling (the PvP item-level switch, the integer health restoration, the
registry sync and the stat publication) and equipment-set use (item swap
order, slot and combat checks, registry sync, stat or raw bonus update,
`SMSG_USE_EQUIPMENT_SET_RESULT`) are unchanged; no await, packet or
persistence order is touched (neither unit awaits).

### Checker owner list and `final` wiring (#1406 follow-up 1)

- `handler-contract-check/src/tests/scenarios_1.rs`: the repository test's
  pinned production-package list gains `wow-world-player-conditions` (slice
  Q added the package; the comment now names it). The checker lib suite
  passes again: 453 passed, 0 failed (`cargo test --locked --jobs 1
  --manifest-path tools/architecture/handler-contract-check/Cargo.toml`).
- `tools/validation-v2`: `final` also plans the checker's format check and
  lib suite when the checked package set can change without a checker file:
  a root `Cargo.toml` or `Cargo.lock` change, or a `crates/*/Cargo.toml` that
  gains or loses a `wow-handler` dependency against the base (a new crate
  that declares it and a deleted one that did count; comments and other
  dependency edits do not). `handler_graph_changed` reads the base manifest
  with `git show`; quick is unchanged.
  `tools/test_validation_v2.py` pins it: root manifest and lock in final and
  not in quick, unrelated paths never, and nine fixture manifests in a
  scratch repository (keeps, gains, loses, comment-only, dotted
  `wow-handler.workspace`, deleted with and without, new with and without).
  The suite fails when the trigger is disabled.
  `docs/operations/validation-v2.md` documents it.

### What moved

- `wow-world-application/src/inventory_scaling.rs` (220, R94) →
  `wow-world-inventory/src/inventory_scaling.rs`. The name is free in
  inventory (its `scaling` module is the item-level math).
- `wow-world-application/src/equipment_set_use.rs` (462, R98) and its
  `equipment_set_use/registration_tests.rs` (41, R100) →
  `wow-world-inventory/src/equipment_set_use_handlers.rs` and
  `equipment_set_use_handlers/registration_tests.rs`. **Module name:**
  inventory already has a private `equipment_set_use` module (the
  `InventoryState` slot, plan and move helpers the context calls), so the
  registrar module is `equipment_set_use_handlers`, following
  `bank_handlers`, `collections_handlers` and `void_storage_handlers`. The
  public item names are unchanged, so no caller sees the module name.
- **The only edits in the moved files:** `wow_world_inventory::` →
  `crate::` (5 import lines and 5 body paths), the test `#[path]` string,
  and rustfmt moving the `use crate::…` lines to the top of the import
  block. `git diff -M bc5bc0be7` shows nothing else. Doc comments that call
  the registrar "Application-owned" are kept verbatim (move-only).
- **Token comparison** (`move_token_compare.py --base bc5bc0be7 --pairs
  tools/architecture/move_token_rules/1363-sQ2.pairs.tsv --rules
  …/1363-sQ2.json`; one rule, `wow_world_inventory::` → `crate::`): **15 of
  15 functions identical.** Without the rule 13 of 15 are identical and
  the two others differ only by the crate path. The three same-named `new`
  constructors of the equipment-set file cannot be told apart by name, so
  the whole-file diff above covers them.
- **Root exports.** Inventory re-exports exactly the names application did:
  `InventoryScalingApplicationCxLikeCpp`; `EquipmentSetUseContextLikeCpp`,
  `EquipmentSetUseHandlerHostLikeCpp`, `EquipmentSetUseItemModsStoresLikeCpp`
  and `register_equipment_set_use_handler_like_cpp` (one exact facade item);
  `EquipmentSetUseFixtureRefsLikeCpp` under the same fixture `cfg`.
  Application keeps no compatibility re-export.
- **Callers** name `wow_world_inventory::…`: application
  `aura_removal/item_scaling.rs`; `wow-world` `handler_composition.rs`,
  `session/player_items/equipment_sets.rs` (the host impl and the context
  builder) and `session/player_items/valuation.rs`. No visibility changed.
- **Not done (#1405 nit 5, optional):** moving the twelve
  `QuestItemStorageHostLikeCpp` inventory operations to an inventory-owned
  host trait. That is a new trait plus a split World impl and a
  session-ownership reprint, not a move, so it stays a follow-up.

### Registration

- `register_equipment_set_use_handler_like_cpp` is token-identical: one
  `PacketHandlerEntry` for `UseEquipmentSet`, `SessionStatus::LoggedIn`,
  `PacketProcessing::Inplace`, `handler_name` `handle_use_equipment_set`,
  the same thunk and the same `S: EquipmentSetUseHandlerHostLikeCpp<C> +
  Send, C: Sync` bounds. `handler_composition.rs` calls it at the same
  position with the same type arguments; only the crate path changed. The
  registration test moved with it and passes in inventory.
- **Checker** (as A2 did for the bank registrar):
  `EQUIPMENT_SET_USE_REGISTRAR` package `wow-world-inventory`, module and
  facade child `equipment_set_use_handlers`; the owner label
  `ApplicationEquipmentSetUse` is unchanged. The composer guard tests mount
  the inventory file and target the inventory root facade and the
  `wow_world_inventory::` call (alias and substitution cases unchanged in
  strength). The direct-builder test reads the inventory file.
  `handler-module-policy.json` moves the capability owner to
  `wow-world-inventory` / `crate::equipment_set_use_handlers`.
- **`session-ownership-policy.json`** is reprinted with `print-baseline`:
  20 lines, the host impl's trait path and two signatures,
  `wow_world_application` → `wow_world_inventory` (and their sort
  position). Nothing else.

### Edges and closure

No `Cargo.toml` or `Cargo.lock` change. Inventory already depended on
core, loot, `wow-handler`, `wow-packet`, `wow-data`, `wow-entities`,
`wow-constants` and `wow-core`; application still uses loot elsewhere.
`wow_world_closure.py check`: PASS in lock and metadata modes (18
packages). Inventory's closure is core, entities, interaction, lifecycle,
loot and visibility: it reaches none of social, instances, quest,
player-conditions, application or `wow-world`. `cargo tree -p world-server
-e normal,build -e features` shows no `test-fixtures` and no
`wow-world-test-support`.

### Accounting

| Crate `src` | Base physical | Head physical | Δ | Base production | Head production |
|---|---:|---:|---:|---:|---:|
| `wow-world-application` | 13,637 (53 files) | 12,905 (50) | −732 | 12,823 | 12,208 |
| `wow-world-inventory` | 22,996 (83) | 23,730 (86) | +734 | 22,721 | 23,338 |
| `wow-world` | 43,406 | 43,406 | 0 | 34,219 | 34,219 |

- Application −732: the three files (723) and `lib.rs` −9. Inventory
  +734: the three files (723) and `lib.rs` +11 (two `mod` lines, the
  re-exports and a two-line facade comment).
- `net_move.py check --base origin/3.4.3`: NOT-APPLICABLE (exit 0; S = 0,
  G = 2, 0 duplicate bodies), because `wow-world` does not shrink.

### Policy and ceilings

`handler-module-policy.json` and `session-ownership-policy.json` as above.
No ceiling raised or changed; no physical or hotspot row names a moved file
(`physical-files` and `hotspot-ratchet` pass). No dependency-policy,
net-move-policy or fixture change.

### Warnings and tests

Canonical recipe (with `--locked`), base `bc5bc0be7` vs head, plus `-p
wow-world-inventory -p wow-world-application --lib --tests` for the moved
test modules; counted with `warning_units.py` (no error) and compared with
the moved paths mapped:

| Build | `wow-world` lib | `wow-world` lib test | inventory lib | application lib | All `wow-world*` lib units |
|---|---:|---:|---:|---:|---:|
| production lib | 125 → 125 | — | 10 → 11 | 6 → 5 | 252 → 252 |
| lib + `test-fixtures` | 133 → 133 | — | 7 → 8 | 4 → 3 | 215 → 215 |
| `--lib --tests` | 133 → 133 | 181 → 181 | 7 → 8 | 4 → 3 | 215 → 215 |

- The one warning that moves is the unused `use wow_core::ObjectGuid;` in
  the equipment-set file (the body names `wow_core::ObjectGuid` in full);
  it is kept, as move-only. In the domain build the inventory lib test goes
  7 → 8 and the application lib test 3 → 2 for the same reason.
  `warning_units.py compare`: nothing only in the head or only in the base,
  in all four builds.
- `wow-world` `dead_code`: 0 in all three builds. `unconditional_recursion`:
  0.
- **Tests.** The test-name multiset is identical: 9,492
  `#[test]`/`#[tokio::test]` functions under `crates/`, 5 ignored. The two
  moved tests (`scaling_restoration_retains_integer_floor_and_zero_maximum_guard`
  and the registrar test) now run in `wow-world-inventory`.

### Follow-ups

- #1405 nit 5 (above).
- The moved doc comments still say "Application" for the equipment-set
  registrar; reword them in a non-move PR. The owner label
  `ApplicationEquipmentSetUse` likewise (the A2 label rename).
- The unused `wow_core::ObjectGuid` import can go in a cleanup PR.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section, together with the
handler-contract repository `check`. The manifest path, dirty flag and
duration are in the PR body. This section is not edited after that run, so
the PR head is the validated SHA.

## Slice 4c plan (remaining leaf families, phase 2)

Measured on `6fe5b5d2f` (slice Q2 merged) before anything moved, with the
hotspot classifier and a lexical index of the inherent `WorldSession` methods
(bodies, the `WorldSession` methods each calls, and the host-trait methods
that already exist in or below each candidate destination). **New host** is
what a move would add after crediting the existing host traits and inlining
the one-line catalog/core wrappers.

| Family | Classifier production | Body (functions) | Outside calls | New host | Destination | Blocker |
|---|---:|---:|---:|---:|---|---|
| open-item loot (`handlers/spell/ops_{1,2}.rs`, the 9e follow-up) | 1,115 | 823 (16) | 20 | 4 | `wow-world-loot-ops` | none |
| void storage (handlers, session adapter, unlock commit, loaded read) | 1,724 + 41 | 1,442 (31) | 31 | 11 | `wow-world-application` | the transfer carries the quest item-transfer plan (a `wow_world_quest` type) and calls seven quest functions, so the bodies sit above quest; `SessionIdGeneratorsLikeCpp` is a World type (moves to core) |
| collections (`CMSG_USE_TOY`, `CMSG_ADD_TOY`) | 495 | 294 (3) | 14 | 7–9 | spell (use) / inventory (add) | spell cast state and execution, the toy account, inventory destruction and the player-condition projection: over 15 % alone |
| group (command receivers, reconciliation, publication tail) | 933 (233 shims) | 457 (27; about 170 are `split_social_*` wrappers with many World callers) | 8 | about 6 | `wow-world-instances` | about 330 movable lines for about 55 host lines: over 15 % alone |
| travel (area trigger, activate taxi) | 561 | 345 (11) | 7 | 2 + a catalog type | instances (area trigger) / entities (taxi) | `AreaTriggerCatalogsLikeCpp` holds the World-typed script dispatcher (`Arc<dyn Fn(&mut WorldSession, …)>`): a design step (dispatcher type parameter or associated type) before the body can move |
| talent | 162 | 44 (3) | 0 | — | — | nothing movable: the catalog setters are the world-server construction API, and `refresh_represented_talent_points_like_cpp` is a `cfg`-split field-borrow builder (slice 12) |
| trainer | 937 | 664 (7): 636 context builders, 28 forwards | 0 | — | — | `AppTrainer*Cx` builders borrow six disjoint session fields with `cfg!(test)` and fixture refs: the session split (slice 12), not a leaf move |

**Order by host share, lowest first:**

- **4c:** the open-item loot family (about 4 % host) and void storage with
  its unlock commit (about 9 %). About 2.4k moved lines.
- **4d:** after two design steps, travel (the area-trigger catalogs get a
  dispatcher type parameter in instances; the area trigger and its client
  conditions go to instances, `CMSG_ACTIVATE_TAXI` and the taxi helpers to
  entities), group (the receivers, reconciliation and publication tail to
  instances behind about 6 host methods; the `split_social_*` wrappers stay
  as adapters) and collections (use toy to spell, add toy to inventory).
  About 900 moved lines for about 110 host lines (about 12 %); none of
  the three fits alone.
- **Slice 12:** talent and trainer, whose remaining World code is field-borrow
  context builders and construction setters, not leaf bodies.

**Edges, simulated before moving** (`wow_world_closure.py check`, lock and
metadata): 4c adds no `wow-world*` edge; application already depends on
inventory, quest, loot, lifecycle and core, and loot-ops on lifecycle,
inventory, loot and quest. The only new dependency is the external
`num-traits` for application (`FromPrimitive` in the item helpers), added
to `dependency-policy.json`. 4d needs instances → `wow-conditions` (the
area-trigger client conditions), to be simulated with `--add`.

## Slice 4c result (open-item loot family → loot-ops; void storage and its unlock commit → application)

Branch `1363-s4c-leaf-handlers`, base `6fe5b5d2f` (slice Q2 merged).
Move-only (R2) after a tooling commit for #1407 follow-ups 1–2. Packets,
their order, the money fence, the void-storage persistence port and
transaction, registrations and processing flags are unchanged; no registrar
moved.

### validation-v2 (#1407 follow-ups 1–2), commit `462426f32`

- **Quick pin (follow-up 1).** `quick` returned before the handler-graph
  trigger, so `profile == "final"` was unreachable for quick and the old pin
  (no `cargo test --lib` in quick) could not fail. The trigger is now
  computed once before the quick branch, behaviour unchanged (the guard keeps
  it false in quick, and quick's checker condition is the old one), and the
  pins assert that quick plans **no command referencing `CHECKER_MANIFEST`**
  for the root manifest, the lock and every fixture manifest. Removing
  `profile == "final" and` now fails the self-test (`AssertionError:
  Cargo.toml`).
- **Trigger (follow-up 2).** `handler_graph_changed` fires in `final` when a
  `crates/*/Cargo.toml` changes its parsed `[dependencies]`,
  `[dev-dependencies]`, `[build-dependencies]`, `[features]` or `[target.*]`
  tables or `package.name` against the base, in any TOML spelling,
  including a new, deleted or unparsable manifest; comment-only and other
  metadata edits (description, `[lints]`, the same dependency in table form)
  do not. Pins: 19 manifests (moves to `[dev-dependencies]`, default-feature
  toggles, `[dependencies.wow-handler]`, quoted keys, renames, target
  tables, multi-manifest diffs). Dropping `"features"`, `"target"` or
  `package.name` from the projection each fails the self-test; the
  `[dev-dependencies]` and `[build-dependencies]` tables were not isolated
  by any of these pins until slice 4d added one fixture for each. The
  docstring and `docs/operations/validation-v2.md` describe exactly this.

### What moved

- **Open-item loot family → `wow-world-loot-ops`**
  (`session_loot/sources/open_item.rs`, `open_item_conditions.rs`), commit
  `38e229ff1`: `CMSG_OPEN_ITEM` (C++ `HandleOpenItemOpcode`), the item loot
  view, the wrapped gift (open and stale destruction), the item
  loot-template generation, the `item_template_addon` money and
  loot-metadata reads (an `InventoryCxRef` impl before), and the item-loot
  condition readers and quest gates: 17 functions, 909 base lines.
  - Host: **`OpenItemHostLikeCpp: LootSessionHostLikeCpp`** (4 methods:
    `get_inventory_item_by_pos`, the wrapped-gift row application,
    `remove_fully_looted_runtime_item`, `send_stat_update`), implemented in
    `handlers/open_item_host.rs` outside the hotspot trees. Adding the four
    to `LootSessionHostLikeCpp` instead would have grown the loot hotspot by
    22 lines.
  - Inlined one-line World wrappers: `item_template_flags`,
    `item_template_lock_id`, `lock_entry_exists_like_cpp`
    (`host.hub_ref().catalogs.…`), `send_equip_error`
    (`host.core().…`), and the loot-condition wrapper whose name equals
    the `wow_loot` free function it calls (inlined so the moved call names
    the single `wow_loot` definition; a self-resolving name there would have
    been a compile error, not runtime recursion). `wow_world_spell::known_spells_like_cpp(self)` becomes
    `host.known_spells_like_cpp()`, whose World impl is that call.
  - The spell registrar's adapter calls `wow_world_loot_ops::handle_open_item`.
- **Void storage → `wow-world-application`** (`void_storage_session/`),
  commit `661a58c59`: the four handlers (unlock, query, transfer, swap),
  the item and publication helpers, the seven represented void-storage reads
  and writes of `session/void_storage_adapter.rs`, the **unlock commit**
  (`session/persistence/commit.rs`, the 13c deferral) and the loaded-state
  read: 31 functions, 1,484 base lines, plus the module's 70 lines of
  constants and plan types.
  - Host: **`VoidStorageSessionHostLikeCpp: ItemSessionHostLikeCpp +
    QuestItemStorageHostLikeCpp`** (11 methods: the loot split, the unlock
    flag and player-flags reads, the values snapshot, four inventory reads
    and plans, the committed new-item apply, the enchantment-duration
    refresh, the bag-slot values send), implemented in
    `handlers/void_storage/session_host.rs`. The seven quest functions take
    the host directly.
  - Names both supertrait chains carry (`lifecycle`,
    `send_player_values_update_from_entity_bridge`,
    `sync_player_registry_state_like_cpp`, `make_inventory_item_object`) use
    the explicit trait path; every World impl of each forwards to the same
    field or inherent method.
  - Inlined: `kick` (`host.core_mut()`), `realm_id`,
    `item_storage_template`, `item_template_max_durability`,
    `item_template_inventory_type`.
  - The unlock commit's former `#[cfg(test)]` step is
    `#[cfg(any(test, feature = "test-fixtures"))] if host.test_fixtures_ref_like_cpp().is_some() && …`,
    the 4b/13c gate, so it still runs exactly in the World test build.
  - `SessionIdGeneratorsLikeCpp` (with its `cfg`-gated `Default`) and
    `PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP` move to `wow-world-core`,
    re-exported at the World paths. `LifecycleCxRef` and `cx_lifecycle_ref`
    retire with their last reader; the session's generator imports are now
    used by `trainer_acquisition.rs` and `inventory_handler_contexts.rs`.
- **Token comparison** (`move_token_compare.py --base 6fe5b5d2f`, rules
  `move_token_rules/1363-s4c{,-void}.{json,pairs.tsv}`): open-item **16 of
  17** identical, void storage **30 of 31**. The two others differ only by a
  rustfmt brace reflow after the dedent (a match-arm block and a closure
  block that now fit on one line).
- **Visibility.** Only what a cross-crate consumer needs is `pub`: the
  handlers and host traits (World adapters), and under
  `cfg(any(test, feature = "test-fixtures"))` the functions the World test
  shims forward to; `VoidWithdrawalItemPublicationLikeCpp` becomes `pub`
  for the same reason. The rest is `pub(crate)` or private.

### Await counts (per function, base vs head)

| Function | Base | Head |
|---|---:|---:|
| `handle_open_item` | 7 | 7 |
| `generate_item_loot_template_entries_like_cpp` | 6 | 6 |
| `open_wrapped_gift_like_cpp` | 3 | 3 |
| `open_active_item_loot_view_like_cpp`, `destroy_stale_wrapped_gift_like_cpp`, the three `load_item_template_addon_*` reads, `save_new_stored_item_loot_like_cpp` | 1 each | 1 each |
| `handle_void_storage_transfer_with_generators_like_cpp` | 4 | 4 |
| `handle_void_storage_unlock_with_generator_like_cpp` | 3 | 3 |
| `handle_void_storage_swap_item` | 2 | 2 |

Token identity puts every await at the same point in the same order.

### Recursion

- Every moved name was grepped across production (non-shim) `wow-world`.
  The only hits are the adapters, which call the explicit free function
  (`wow_world_loot_ops::handle_open_item`, `wow_world_application::handle_void_storage_*`);
  `SpellHandlerHostLikeCpp::handle_open_item` shares the moved name, which
  is why the adapter no longer says `WorldSession::handle_open_item`.
- Every new host forward is `WorldSession::name(self, …)` over an existing
  inherent method of that name, or `crate::session::split_loot_mut`.
- `unconditional_recursion`: 0 in all three canonical builds.

### Registration

No registrar moved. The spell and void-storage registrations, their order,
`SessionStatus`, `PacketProcessing` and thunks are untouched; only the World
adapters' call targets changed. No checker constant, mount or
`handler-module-policy.json` change.

### Edges and closure

No `wow-world*` edge. `Cargo.toml`: application gains `num-traits`
(external, in `dependency-policy.json`); `Cargo.lock` gains that one
dependency line. `wow_world_closure.py check`: PASS in lock and metadata
modes (18 packages). `cargo tree -p world-server -e normal,build -e
features` shows no `test-fixtures` and no `wow-world-test-support`.

### Accounting

| Crate `src` | Base physical | Head physical | Base production | Head production |
|---|---:|---:|---:|---:|
| `wow-world` | 43,406 (455 files) | 41,144 (451) | **34,219** | **31,664** |
| `wow-world-loot-ops` | 13,132 | 14,144 | 13,047 | 14,054 |
| `wow-world-application` | 12,905 | 14,708 | 12,208 | 13,993 |
| `wow-world-core` | 47,425 | 47,455 | 45,591 | 45,608 |

- **`wow-world` production −2,555** (open-item part −949, void part
  −1,606). `git diff --numstat` on `crates/wow-world/src`: −2,756 / +494.
  - **Moved:** 2,393 function lines (909 + 1,484) and 83 item lines (the
    70 void constants and plan types, the 12-line generator type, one
    constant).
  - **Reclassified production → test:** `handlers/spell/state.rs` (11, now
    `#![cfg(test)]`: only the spell scenarios read its re-exports), the
    `item_template_lock_id` wrapper (3, to the open-item shim file) and the
    two `cfg(test)` packet entry points nested in the moved void files
    (about 12, to the void shim file).
  - **Retired:** `lock_entry_exists_like_cpp`, `LifecycleCxRef` and
    `cx_lifecycle_ref`, the loot-condition wrapper, the empty
    `persistence/commit.rs` and the moved files' headers and `impl` lines.
  - **New host in `wow-world`:** 130 adapter lines (`open_item_host.rs`
    41 without the relocated wrapper, `void_storage/session_host.rs` 89),
    plus 5 in `void_storage/host.rs`.
- **Destinations:** loot-ops +1,012 physical (909 body lines, 17-line
  trait, headers and exports); application +1,803 (1,523 body lines after
  rustfmt, 70 item lines, 42-line trait, headers and exports); core +30
  (the generator type with its `Default`, the constant, imports).
- **Budget.** New host lines (traits 17 + 42, adapters 41 + 89 + 5) are
  194 against 2,476 moved lines: **7.8 %** (open-item 58 / 909 = 6.4 %,
  void 136 / 1,567 = 8.7 %).
- `net_move.py check --base origin/3.4.3`: S = 2,262, G = 2,845 within the
  3,336 allowance, 0 duplicate bodies, PASS.

### Policy and ceilings

- **Ceilings tightened, none raised.** Hotspot `session/mod.rs`: production
  21,741 → 21,606, test 141,641 → 141,630 (the generator `Default` left
  with the type), total 163,382 → 163,236. The loot, character and quest
  hotspots are unchanged.
- **New physical rows** at their live size, as for the earlier shim files:
  `handlers/open_item_test_shims.rs` (31) and
  `handlers/void_storage_test_shims.rs` (231), both `#![cfg(test)]`,
  mounted outside the hotspot trees.
- `session-ownership-policy.json` reprinted with `print-baseline`; the delta
  is the moved and retired items, the two new host impls and the shims.
  `dependency-policy.json`: application → `num-traits`.

### Warnings and tests

Canonical recipe (with `--locked`), base `6fe5b5d2f` vs head, counted and
compared with `warning_units.py`:

| Build | `wow-world` lib | `wow-world` lib test | loot-ops lib | application lib | All `wow-world*` lib units |
|---|---:|---:|---:|---:|---:|
| production lib | 125 → 122 | — | 1 → 1 | 5 → 5 | 252 → 249 |
| lib + `test-fixtures` | 133 → 131 | — | 0 → 0 | 3 → 3 | 215 → 213 |
| `--lib --tests` | 133 → 131 | 181 → 179 | 0 → 0 | 3 → 3 | 215 → 213 |

- Nothing only in the head in any build. Gone: the unused `BagFamilyMask`
  import of `handlers/spell.rs`, the unused `WrappedGiftRow` re-export of
  `handlers/spell/state.rs`, and the production-only unused `HighGuid` of
  `session/catalog_capabilities.rs`.
- `wow-world` `dead_code`: 0 in all three builds. `unconditional_recursion`:
  0.
- **Tests.** The test-name multiset is identical: 9,492
  `#[test]`/`#[tokio::test]` functions under `crates/`, 5 ignored. Scenario
  call sites keep method syntax through the two shim files, except the two
  void-storage loaded-state assertions, which now call
  `wow_world_application::represented_void_storage_loaded_like_cpp(&session)`
  (the same check; `LifecycleCxRef` retired). The void-storage source scan
  `void_storage_mutation_paths_have_no_concrete_persistence_after_port_cut`
  now audits the World adapter files and every file of the moved module, and
  checks both directories for unaudited files (stricter, not weaker).
  Focused run during the move: `cargo test -p wow-world --lib -- void
  open_item item_3 player_items_4 equipment_set handler_contract wrapped
  generator spell::tests`: 252 passed, 1 ignored.

### Remaining (4d and beyond)

- **4d** (above): travel after the area-trigger catalog dispatcher step,
  group receivers/reconciliation/publication tail, collections.
- **Slice 12:** talent and trainer context builders.
- **Cleanup:** retire the shim files (now including the two above) by
  converting the scenario call sites.

### Follow-ups

- #1407 follow-up 3 (reword the moved "Application" doc comments and the
  `ApplicationEquipmentSetUse` label, drop the unused
  `wow_core::ObjectGuid` import) and #1405 nit 5 remain open.
- `LootSessionHostLikeCpp`, `QuestItemStorageHostLikeCpp` and the inventory
  host chain now share several same-named forwards; the void-storage bodies
  name the trait explicitly at those sites. Consolidating the duplicate host
  names stays with the cleanup.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section (no registrar moved,
so no separate handler-contract `check` is required; `final` runs the
checker suite because `Cargo.lock` changed). The manifest path, dirty flag
and duration are in the PR body. This section is not edited after that
run, so the PR head is the validated SHA.

## Slice 4d result (travel, group receivers/reconciliation/publication tail, collections → instances, spell, inventory)

Branch `1363-s4d-leaf-handlers`, base `dc17fe39b` (slice 4c merged).
Move-only (R2) after a tooling commit for the #1408 follow-ups. Packets and
their order, the #743 reconciliation, the group publication tail, the toy
account row and item destruction order (#169), registrations and processing
flags are unchanged; no registrar moved.

### #1408 follow-ups, commit `6dec1fbbc`

- **Trigger fixtures (follow-up 1).** `test_validation_v2.py` gains
  `dev-dependencies-only` and `build-dependencies-only`: the base has
  `[dependencies]`, the head adds the same table plus one `wow-handler`
  entry in only that table. Mutation check: removing `"dev-dependencies"`
  from `GRAPH_TABLES` fails the self-test (`AssertionError:
  dev-dependencies-only`), and so does removing `"build-dependencies"`
  (`AssertionError: build-dependencies-only`); with the table restored the
  self-test passes. `docs/operations/validation-v2.md` now says the
  self-test has, for each table and the package name, one fixture where only
  that part differs, which covers those cases and not every manifest edit;
  the package-list pin belongs to the checker's lib test. The 4c section
  says the two tables were not isolated before this slice.
- **Ledger history (follow-up 2).** The `session/mod.rs` hotspot
  `latest_growth_review` keeps its whole history in the field itself, as
  every other entry does. Slice 4c replaced the chain (6c back to 9e), and
  slice 9e had already replaced the one before it (9d back to #1241 F1 and
  #584). Both were restored from `bc5bc0be7` and `b240d01a6~1`; an audit of
  the last 15 ledger commits found no other replaced chain. This slice's
  review is prepended with `Previous: ` in front of the restored chain
  (39,436 characters; the 4c text is contained verbatim).
- **Doc wording (follow-up 3).** The 4c note on the inlined loot-condition
  wrapper now says a self-resolving name would have been a compile error,
  not runtime recursion.

### Re-measure and the design step

Re-measured on `dc17fe39b` with the lexical index of the 4c plan, then
again while moving (the existing host traits now cover more than in 4c):

| Family | Moved function lines | New host methods | Destination |
|---|---:|---:|---|
| travel (area trigger + client conditions, activate taxi + mount-display filter) | 305 | 1 (+2 catalog accessors) | `wow-world-instances` |
| group (5 command receivers, reconciliation, publication tail, 4 wrappers only they called) | 332 (+10 enum) | 5 | `wow-world-instances` |
| collections (use toy, add toy) | 262 (+1 constant) | 1 | `wow-world-spell`, `wow-world-inventory` |

- **Travel design decision.** The area-trigger catalogs hold
  `AreaTriggerScriptDispatcherLikeCpp = Arc<dyn Fn(&mut WorldSession, …)>`.
  A dispatcher generic over a host trait would make the catalog type generic
  and touch the world-server construction and the script seam; that is not
  small. Instead the dispatcher stays a World type and does not cross:
  the moved body takes the catalogs as `&H::AreaTriggerCatalogsLikeCpp`, the
  associated type the movement host already declares (World binds it to
  `AreaTriggerCatalogsLikeCpp`), reads `db2`, `scripts`, `taverns` and
  `destinations` through `AreaTriggerCatalogAccessLikeCpp` (two accessors
  added), and calls one host method, `dispatch_area_trigger_script_like_cpp(catalogs, …)`,
  whose World impl is `WorldSession::dispatch_area_trigger_script_like_cpp(self, catalogs.script_dispatcher.as_ref(), …)`,
  the call the body made. Behaviour-neutral: the same `Arc` clone and
  call on the same session at the same point.
- **Taxi destination.** The 4c plan put `CMSG_ACTIVATE_TAXI` in entities.
  Its usable-mount-display filter reads the aura host's
  `represented_mount_x_display_usable_like_cpp` (spell, above entities), so
  entities would need a host method for the filter. In instances (above
  spell) the body and the filter both move with no new host method. The
  first commit (`4b5ef9813`) used entities; `a0ed45ba3` moves it.
- **Edges simulated before moving.** `wow_world_closure.py check --add
  "wow-world-instances->wow-conditions"`: PASS in lock and metadata modes
  (19 packages with the simulated edge). Group and collections add no edge
  (instances already depends on social, lifecycle and `wow-world-social`;
  spell on inventory; inventory on core).

### What moved

- **Travel → `wow-world-instances`** (`instance_session/area_trigger_opcode.rs`,
  `instance_session/activate_taxi.rs`), commits `4b5ef9813`, `a0ed45ba3`:
  `CMSG_AREA_TRIGGER` (C++ `MiscHandler.cpp:478` `HandleAreaTriggerOpcode`)
  with `area_trigger_client_conditions_meet_like_cpp`, and
  `CMSG_ACTIVATE_TAXI` (`TaxiHandler.cpp:167`) with
  `represented_taxi_usable_mount_displays_like_cpp`
  (`session/taxi/operations.rs`).
  - Host: **`AreaTriggerHostLikeCpp: InstanceSessionHostLikeCpp<AreaTriggerCatalogsLikeCpp: AreaTriggerCatalogAccessLikeCpp>`**,
    one method (above). The taxi bodies take `InstanceSessionHostLikeCpp`.
  - Inlined one-line World wrappers: `teleport_to`
    (`wow_world_entities::teleport_to_with_options(host, …, TELE_TO_NONE_LIKE_CPP)`,
    its whole body) and `condition_store` (`host.hub_ref().catalogs.condition_store()`).
- **Group → `wow-world-instances`** (`group_session.rs`), commit
  `41b6ca19b`: the `ApplyGroupRemoval`/`ApplyGroupJoin`/`SendPartyUpdate`/
  `ApplyGroupDifficulty`/`ApplyGroupSubgroup` receivers
  (`handlers/group/commands.rs`, deleted), `reconcile_group_state_like_cpp`
  with `apply_authoritative_group_state_like_cpp` and
  `GroupReconciliationOutcomeLikeCpp`, `apply_group_publication_tail_like_cpp`
  with `send_group_party_update_after_owner_like_cpp`, and the four wrappers
  only these bodies called (`apply_group_subgroup_like_cpp`,
  `defer_group_state_reconciliation_like_cpp`,
  `send_destroyed_group_party_update_like_cpp`,
  `reconcile_group_difficulty_like_cpp`), moved with them.
  - Host: **`GroupSessionHostLikeCpp: InstanceSessionHostLikeCpp`**, five
    methods: `split_social_ref`/`split_social_mut` (the World splits),
    `lifecycle` (the field), `update_visible_gameobjects_or_spell_clicks_like_cpp`
    and `apply_group_difficulty_like_cpp` (inherent).
  - Inlined one-line World wrappers (each the same `SessionSocialLimits`
    operation over the same split): `set_owned_player_group_like_cpp`,
    `send_player_party_type_update_like_cpp`,
    `next_group_update_sequence_number_like_cpp` (parenthesised, because it
    is a `let … else` scrutinee), `sync_player_registry_party_member_party_type_like_cpp`;
    and `send_packet_realm`, `player_registry` (`host.core().…`).
  - Callers: `session_commands.rs` and the driver call the free functions;
    `GroupHandlerHostLikeCpp::run_group_publication_tail_like_cpp` calls
    `wow_world_instances::apply_group_publication_tail_like_cpp`.
- **Collections**, commit `7a9cdc5f4`: `CMSG_USE_TOY` (`ToyHandler.cpp:54`)
  → `wow-world-spell` (`toy_session.rs`) with
  `CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP`, over the existing
  `SpellExecutionHostLikeCpp` (no new method); `CMSG_ADD_TOY`
  (`ToyHandler.cpp:28`) → `wow-world-inventory` (`toy_session.rs`) behind
  **`AddToyHostLikeCpp: ItemSessionHostLikeCpp`** (one method,
  `can_use_inventory_item_represented_like_cpp`).
  - Inlined one-line World wrappers: `item_storage_template`, `spell_store`
    (catalog reads), `has_account_toy_like_cpp`/`add_account_toy_like_cpp`
    (the lifecycle split), `get_inventory_item_by_guid_like_cpp`/
    `resolved_inventory_item_objects_like_cpp` (the inventory split),
    `send_equip_error`/`send_packet_realm` (core) and
    `set_active_spell_cast_like_cpp`, inlined as
    `{ let cast = …; host.mutate_cast_execution_like_cpp(|state| state.active = cast) }`:
    the argument is bound first as the wrapper bound it, and the discarded
    `.is_some()` is not evaluated (as a block statement it would be an
    `unused_must_use` warning; it is a pure read).
  - The two stale doc paragraphs above the toy body
    (`CMSG_COLLECTION_ITEM_SET_FAVORITE`, `CMSG_TRANSMOGRIFY_ITEMS`, whose
    handlers moved earlier) are dropped; the toy documentation moves to the
    module docs.
- **Adapters.** One World file outside the hotspot trees,
  `handlers/leaf_session_hosts.rs` (commit `ab00265da`), holds the
  area-trigger catalog access (moved from `session/instances/host.rs` with its
  two new accessors) and the three host impls.
- **Retired with their last production reader** (commit `cca8252e4`, found
  by the canonical warning recipe): `has_account_toy_like_cpp`,
  `send_player_party_type_update_like_cpp`, the `retain_targets`/`retain_visual`
  re-exports and `session/taxi_contracts.rs` (its test readers name the
  `wow_world_core` path). Moved verbatim to the `#![cfg(test)]` shim files
  for their scenarios: `send_equip_error`, `add_account_toy_like_cpp`,
  `set_active_spell_cast_like_cpp`, `next_group_update_sequence_number_like_cpp`.
- **Token comparison** (`move_token_compare.py --base dc17fe39b`, rules
  `move_token_rules/1363-s4d-{travel,group,collections}.{json,pairs.tsv}`):
  travel **3 of 4** identical, group **13 of 13**, collections **2 of 2**.
  The other differs only by a rustfmt closure-brace reflow after the dedent
  (`|current_area, required_area| { … }` now fits without the block).
- **Visibility.** `pub` only for what World calls: the handlers,
  reconciliation and tail, the host traits, the toy constant, and under
  `cfg(any(test, feature = "test-fixtures"))` the taxi mount-display filter
  the shims forward to. Everything else is private.

### Host surface

New host lines are 125 against 910 moved lines (899 function, 10 enum,
1 constant): **13.7 %**.

- Traits: `AreaTriggerHostLikeCpp` 13, the two catalog accessors 3,
  `GroupSessionHostLikeCpp` 14, `AddToyHostLikeCpp` 9 (39 lines with docs).
- World adapter `handlers/leaf_session_hosts.rs`: 94 lines, of which 8 are
  the relocated catalog impl (not new): 86.
- **Unmerged measurement (#1409 review follow-up 2).** Before `ab00265da`
  merged the adapters and trimmed four trait doc lines, the same methods
  took 148 lines: traits 43, and three adapter files
  (`travel_host.rs` 54, `group_session_host.rs` 38, `toy_host.rs` 21 = 113,
  less the 8 relocated catalog lines). That is **16.3 %** of 910 (16.5 %
  with the two extra per-file mount lines). The merge removed file headers,
  imports and doc lines, not methods or coupling: the method count is the
  same in both. Later slices report both figures when they merge adapters.

### Await counts (per function, base vs head)

| Function | Base | Head |
|---|---:|---:|
| `handle_area_trigger_with_catalogs_like_cpp` | 1 | 1 |
| `apply_group_publication_tail_like_cpp` | 1 | 1 |
| `handle_use_toy_with_generator_like_cpp` | 1 | 1 |
| `handle_add_toy` | 1 | 1 |

Every other moved function has none, base and head. Token identity puts
each await at the same point.

### Recursion

- Every moved name was grepped across production (non-shim) `wow-world`.
  The hits are the registrar adapters, `session_commands.rs`, the driver and
  the tail adapter, each calling the explicit free function
  (`wow_world_instances::…`, `wow_world_spell::…`, `wow_world_inventory::…`).
  `TravelHandlerHostLikeCpp::handle_activate_taxi`/`handle_area_trigger_with_catalogs_like_cpp`
  and `CollectionsHandlerHostLikeCpp::handle_add_toy` share the moved names;
  their impls name the crate path.
- Every new host forward resolves to an inherent method
  (`WorldSession::dispatch_area_trigger_script_like_cpp`,
  `…::update_visible_gameobjects_or_spell_clicks_like_cpp`,
  `…::apply_group_difficulty_like_cpp`,
  `…::can_use_inventory_item_represented_like_cpp`), an explicit free
  function (`crate::session::split_social_{ref,mut}`) or a field. The
  dispatch forward shares its name with the inherent method and differs in
  its parameter types; `WorldSession::name` resolves to the inherent item
  first, and a trait resolution would not type-check.
- `unconditional_recursion`: 0 in all three canonical builds.

### Registration

No registrar moved. The travel, collections and group registrations, their
order, `SessionStatus`, `PacketProcessing`, handler names and thunks are
untouched; only the World adapters' call targets changed. No checker
constant, mount or `handler-module-policy.json` change, so no
handler-contract `check` is required. The `handler_contract` lib tests ran
in the focused run below.

### Edges and closure

`wow-world-instances` → `wow-conditions` (external to `wow-world*`, the
area-trigger client conditions), in `dependency-policy.json`; `Cargo.lock`
gains that one line. No `wow-world*` edge. `wow_world_closure.py check`:
PASS in lock and metadata modes (18 packages). `cargo tree -p world-server
-e normal,build -e features` shows no `test-fixtures` and no
`wow-world-test-support`.

### Accounting

| Crate `src` | Base physical | Head physical | Base production | Head production |
|---|---:|---:|---:|---:|
| `wow-world` | 41,144 (451 files) | 40,282 (453) | **31,664** | **30,664** |
| `wow-world-instances` | 7,248 | 8,090 | 7,200 | 8,038 |
| `wow-world-spell` | 20,112 | 20,343 | 19,795 | 20,026 |
| `wow-world-inventory` | 23,730 | 23,855 | 23,338 | 23,463 |
| `wow-world-entities` | 20,986 | 20,987 | 20,695 | 20,696 |

- **`wow-world` production −1,000.** By file (classifier): `collections/mod.rs`
  −322, `travel/travel.rs` −286, `session/social/group.rs` −169,
  `group/commands.rs` −150 (deleted), `group_application_handler_contexts.rs`
  −68, `taxi/operations.rs` −37, `instances/host.rs` −13,
  `lifecycle_ops/operations.rs` −13, `player_items/equipment.rs` −11,
  `instances/difficulty.rs` −10, `taxi_contracts.rs` −7 (deleted),
  `player_cast/state.rs` −5, `session/mod.rs` −3, `spell_cast_adapter.rs` −2,
  `group.rs` −1, `leaf_session_hosts.rs` +94, three one-line doc/mount edits +3.
  - **Moved:** 910 lines (899 function lines, the 10-line outcome enum, the
    toy constant).
  - **Reclassified production → test:** 48 lines: the four wrappers moved
    to the shim files (31) and the two `cfg(test)` entry points that were
    nested in the moved impls, which the classifier counts as production
    (`handle_area_trigger` 6, `handle_use_toy` 11).
  - **Retired:** 131 lines: the two dead wrappers (8), the taxi re-export
    module and its mount (9), the collapsed adapter re-export (2), the two
    stale toy doc paragraphs (13) and the moved files' headers, imports,
    `impl` lines and blank lines (99). The 8-line catalog impl is relocated,
    not retired.
  - **New host in `wow-world`:** 86 adapter lines (above), plus 3 lines of
    docs and mounts.
- **Destinations (production):** instances +838 (`group_session.rs` 457,
  `area_trigger_opcode.rs` 252, `activate_taxi.rs` 114, mounts, exports and
  the catalog accessors 15); spell +231 (`toy_session.rs`); inventory +125
  (`toy_session.rs`); entities +1 (a doc line).
- `net_move.py check --base origin/3.4.3`: S = 862, G = 1,199 within the
  1,866.1 allowance, 0 duplicate bodies, PASS.

### Policy and ceilings

- **Ceilings tightened, none raised.** Hotspot `session/mod.rs`: production
  21,606 → 21,272, test 141,630 unchanged, total 163,236 → 162,902.
  Physical `session/mod.rs` 825 → 820, `handlers/group.rs` 75 → 74. The
  other hotspots are unchanged.
- **New physical rows** at their live size, all `#![cfg(test)]` and mounted
  from `handlers/mod.rs` outside the hotspot trees:
  `handlers/travel_test_shims.rs` (31), `handlers/group_session_test_shims.rs`
  (37), `handlers/collections_session_test_shims.rs` (64).
- `session-ownership-policy.json` reprinted with `print-baseline`; the delta
  is the moved and retired items, the new host impls and the shims
  (`check --syntax-only` PASS). `dependency-policy.json`: instances →
  `wow-conditions`.
- The test lines of `session/mod.rs` do not grow: the two scenarios that read
  `CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP` name it as
  `wow_world_spell::CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP` (same check, same
  value), instead of a new `cfg(test)` re-export.

### Warnings and tests

Canonical recipe (with `--locked`), base `dc17fe39b` vs head, counted and
compared with `warning_units.py`:

| Build | `wow-world` lib | `wow-world` lib test | All `wow-world*` lib units |
|---|---:|---:|---:|
| production lib | 122 → 120 | — | 249 → 247 |
| lib + `test-fixtures` | 131 → 129 | — | 213 → 211 |
| `--lib --tests` | 131 → 129 | 179 → 178 | 213 → 211 |

- `compare`: nothing only in the head in any build. Gone: the two unused
  import groups of `handlers/collections/mod.rs` (and the lib-test copy).
  The destination crates keep exactly their pre-existing warnings.
- `wow-world` `dead_code`: 0 in all three builds. `unconditional_recursion`: 0.
- **Tests.** The test-name multiset is identical: 9,492
  `#[test]`/`#[tokio::test]` functions under `crates/`, 5 ignored, base and
  head. Scenario call sites keep method syntax through the shim files; the
  edits under `unit_tests` are two constant paths, one import path
  (`RepresentedActivateTaxiLikeCpp` from `wow_world_core`) and the unused
  `use super::*` of the collections test module. No assertion changed in
  meaning. Focused run during the move: `cargo test -p wow-world --lib --
  area_trigger taxi group toy collections send_equip_error party travel
  handler_contract`: 381 passed, 1 ignored.

### Remaining (after 4d)

The leaf families are done. What is left under `handlers/` is registrar
adapters, `cfg(test)` shims and the field-borrow context builders:

- **talent and trainer** (`trainer*` 761, `talent*` 99 production lines):
  context builders and construction setters, slice 12.
- **Shim files counted as production.** `handlers/group/test_shims.rs` (233),
  `handlers/travel/travel/test_shims.rs` (40) and
  `handlers/collections/test_shims.rs` (47) are mounted with `#[cfg(test)]`
  but lack `#![cfg(test)]`, so the classifier counts them as production.
  Marking them is a reclassification for the cleanup, not a move.

### Follow-ups

- #1407 follow-up 3 and #1405 nit 5 remain open.
- The shim files retire with the cleanup (the scenarios call the free
  functions directly).
- `represented_taxi_usable_mount_displays_like_cpp` keeps its base
  `#[allow(dead_code)]` (verbatim); it is used, so the attribute can go in a
  later behaviour-neutral cleanup.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section and the slice 4
summary (no registrar moved, so no separate handler-contract `check`;
`final` runs the checker suite because `Cargo.lock` and crate manifests
changed). The manifest path, dirty flag and duration are in the PR body.
This section is not edited after that run, so the PR head is the validated
SHA.

## Slice 4 summary (4a–4d)

| Slice | Moved | New host | Host share | `wow-world` production |
|---|---:|---:|---:|---|
| 4a (gameobject use, player attack, text emote, LFG lock info) | 3,536 | ~514 | 14.5 % | 87,998 → 84,815 |
| 4b (movement, vehicle, corpse, arena skirmish) | ~2,120 | ~270 | 12.8 % | 84,815 → 82,922 |
| 4c (open-item loot, void storage and its unlock commit) | 2,476 | 194 | 7.8 % | 34,219 → 31,664 |
| 4d (travel, group receivers/reconciliation/tail, collections) | 910 | 125 | 13.7 % | 31,664 → 30,664 |
| **Slice 4** | **~9,040** | **~1,100** | **~12 %** | −8,631 across the four (other slices ran between 4b and 4c) |

- Every leaf handler family planned for slice 4 has moved; talent and
  trainer were re-planned as slice 12 (their remaining World code is
  field-borrow context builders, not bodies).
- Every slice was move-only: token-identical bodies after documented
  mechanical rules (the only differences rustfmt brace reflows), equal
  per-function await counts, no registrar change except where noted, no
  new `wow-world*` edge outside the layering (4c and 4d each add one
  external dependency), `dead_code` 0 and no new warning in the three
  canonical builds, and the test-name multiset unchanged.
- Host trait pattern (F5 inverted): a destination trait over
  `SessionHubHostLikeCpp`, the World impl in a `handlers/*` adapter file
  outside the hotspot trees, test entry points in ratcheted `#![cfg(test)]`
  shim files.

### Plan for slice 12 and the final cleanup

Lexical estimate on this head (lexical index of inherent `WorldSession`
methods; to be re-measured before each step):

1. **12a: talent and trainer leftovers.** `trainer*` (761) and `talent*`
   (99) are `AppTrainer*Cx` and talent context builders that borrow six
   disjoint session fields with `cfg!(test)` fixture refs. Move the builders
   to `wow-world-application`/`wow-world-spell` as functions over a split
   that returns the borrowed fields (one `split_trainer_mut` host method),
   and give the `cfg!(test)` flag the 4b fixture-hook form. The catalog
   setters stay (world-server construction API).
2. **12b: base-module logic.** Remaining non-forward inherent bodies in
   `session/**` (persistence load helpers, player-items, visibility refresh,
   player-condition projection builders): measure per module, move those
   whose callers are already in domain crates, under the same 15 % rule.
3. **12c: thin `WorldSession` forward sweep.** About 509 of the 862
   production inherent methods are single-statement forwards (about 2.4k
   lines). Where every caller is a host impl or a registrar adapter, point
   the caller at the target (split, free function or core) and delete the
   forward; where scenarios are the only other caller, move it to a shim.
   Verify with the recursion rule (every forward resolves to an inherent
   method or an explicit free function) and the warning recipe.
4. **12d: adapter retirement.** 85 trait impls for `WorldSession` and about
   1.3k lines of `*_handler_contexts.rs`. Merge host traits that now
   duplicate forwards (`LootSessionHostLikeCpp`,
   `QuestItemStorageHostLikeCpp`, the inventory chain), and collapse
   per-family adapter files into one per destination crate.
5. **Final cleanup.** Convert scenario call sites to the free functions and
   retire the `#![cfg(test)]` shim files (and mark or retire the three
   `#[cfg(test)]`-mounted shim files above); drop the `#[allow(dead_code)]`
   left on moved items that are used; tighten every ceiling to its final
   value; re-measure and record the final `wow-world` production in the
   phase 2 summary.

## Slice 12 plan (re-measured on the 12a head)

Re-measured on the 12a head with the hotspot classifier and a lexical index
of the production inherent `WorldSession` methods (comments and literals
blanked; methods of `cfg(test)` impls, nested `cfg(test)` methods, test
paths and shim files excluded; "single-statement" means at most one
statement and no `let`/`if`/`match`/loop/`return`). This index is stricter
than the one behind the slice 4 summary's 862 / 509: on the 4d base it
counts **765 methods (13,478 lines), 371 single-statement (2,046 lines)**,
and on the 12a head **757 (12,777 lines), 367 single-statement (2,018
lines)**. `wow-world` production is **29,898** (base 30,664).

1. **12a (this slice): done.** The context builders moved; what is left of
   `trainer*`/`talent*` is the registrar adapters (`handlers/trainer/host.rs`
   172, `handlers/talent/host.rs` 52, `session/progression/talent_host.rs`
   23), module roots and docs (`handlers/trainer.rs` 39, `handlers/talent.rs`
   22, `talent/state.rs` 11) and the two catalog setters (17), which stay
   (world-server construction API). The adapters are 12d's.
2. **12b: base-module logic.** 390 non-forward inherent methods (about
   10.8k lines). The largest groups: `session/player_items` 85 non-forward
   of 141 (2,646 lines with the forwards), `session/spell_effects` 13
   (1,086), `session/spell_state` 34 (788 with forwards), `session/persistence`
   20 (387), `session/social` 16, `session/quest` 8, `session/visibility` 9,
   `spell_acquisition/adapter.rs` 2 (288), `session/stand_state_adapter.rs`
   2 (302). `session/driver` (707), `session_commands.rs` (192) and
   `session/map_admission.rs` (176) are the runtime shell and stay. Move per
   module where every production caller is already a domain crate or a host
   impl, under the 15 % rule; player-items first (inventory), then spell
   effects and spell state (spell).
3. **12c: thin forward sweep.** 367 single-statement forwards (2,018
   lines). Where every caller is a host impl or a registrar adapter, point it
   at the target (split, free function or core) and delete the forward;
   scenario-only callers move to a shim. Largest groups: `session/player_items`
   56, `session/catalogs` 37 (setters stay), `handlers/account_data` 31,
   `session/spell_state` 31, `session/world_entities` 14.
4. **12d: adapter retirement.** 84 production `impl … for WorldSession`
   blocks (83 at the 4d base; 12a adds `TrainerSessionHostLikeCpp`), 26
   `*_handler_contexts.rs` files (1,320 lines) and 38 `*host*.rs` adapter
   files (4,699 lines; largest `character/login_host.rs` 399, `loot/session_loot_host.rs`
   385, `loot/loot_host.rs` 368, `entities/host.rs` 335). Merge host traits
   that duplicate forwards and regroup the adapters by destination crate
   (the 4d review's follow-up 2), reporting merged and unmerged host shares.
5. **Final cleanup.** 52 shim files (7,634 physical lines); 19 of them lack
   `#![cfg(test)]` and are counted as production (1,655 classifier lines),
   so marking them is a reclassification of about 1.7k lines, not a move.
   Then convert scenario call sites to the free functions and retire the
   shims, drop the used `#[allow(dead_code)]` leftovers, tighten every
   ceiling and record the final production in the phase 2 summary.

## Slice 12a result (trainer contexts and CMSG_TRAINER_LIST → application, talent-point refresh → quest)

Branch `1363-s12a-trainer-talent`, base `8e618d0e2` (slice 4d merged).
Move-only (R2) after a tooling commit for the #1409 follow-ups. Trainer
list/buy/unlearn order, the talent learn/reset path, money and persistence
order (#169), packets, registrations and processing flags are unchanged; no
registrar moved.

### #1409 follow-ups, commit `a49985733`

- **Standalone `--tests` (follow-up 1).** `wow-world-instances` gets a
  `test-fixtures` feature forwarding to core, entities, lifecycle, loot,
  social and spell, plus a self dev-dependency (the spell/social pattern;
  `Cargo.lock` gains the self edge). `cargo check --locked --jobs 1 -p <crate>
  --tests`, one crate at a time, for the 17 `wow-world-*` crates:
  **before**, only `wow-world-instances` failed (11 `E0061`/`E0599` errors in
  `wow-world-inventory` built without its fixtures); **after**, all 17 pass.
  At the 12a head (after `wow-world-application` gained the social edge)
  the same loop passes again for all 17.
- **validation-v2.** `final` now plans `cargo check --locked --tests -p
  <crate>` alone for each changed (not deleted) `crates/wow-world-*/Cargo.toml`,
  because the reverse-closure check unifies features and cannot see a
  missing forward. Pinned in `tools/test_validation_v2.py` (a domain
  manifest triggers it; `wow-world`'s own manifest, another crate's manifest,
  a domain source file and a deleted manifest do not; quick never). Mutation
  check: disabling the planner branch fails the self-test
  (`AssertionError: crates/wow-world-fixture/Cargo.toml`). Documented in
  `docs/operations/validation-v2.md`.
- **Unmerged host share (follow-up 2):** recorded under the 4d host surface
  (148 lines, 16.3 %, against the merged 13.7 %).

### What moved

- **Trainer contexts → `wow-world-application`**
  (`trainer_purchase/session_context.rs`), commit `a8c2f06ff`: the purchase
  (`trainer_buy_context_like_cpp`), purchase-admission
  (`trainer_buy_admission_context_like_cpp`) and list-publication
  (`trainer_list_publication_context_like_cpp`) context builders from the
  World `session/trainer_acquisition.rs` (deleted). They stay `pub(crate)`.
  - Host: **`TrainerSessionHostLikeCpp: SessionHubHostLikeCpp`**
    (`trainer_purchase/session_host.rs`), one required method,
    `split_trainer_mut`, returning **`TrainerSessionFieldsLikeCpp`**: the
    `HubMut` plus `lifecycle`, `inventory`, `spell_state`, `quest_state`,
    `loot`, `interaction`, `social` and `instances`, the same disjoint field
    borrows the World bodies took. The World impl (in
    `handlers/trainer/host.rs`, outside the hotspot trees) builds it from the
    fields.
  - **Fixture hook.** The World `cfg!(test)` argument becomes
    `host.world_test_consumer_like_cpp()`, a trait default (the 6a quest form
    over the 4b `test_fixtures_ref_like_cpp` hook) read once before the
    split: `Some` fixtures only in the World test build, so the value is
    `cfg!(test)` of `wow-world` in all three builds.
  - **Mechanical rules** (`move_token_rules/1363-s12a-trainer.json`): the
    signatures; a prologue that reads the hook and destructures the split;
    `&mut self.<field>` and `&self.<field>` → the split binding (whole-field
    borrows), `self.catalogs` → `session_catalogs` (the bodies shadow
    `catalogs` with their own catalog views), `self.` → nothing for the
    remaining `core`/`config`/`fixtures` paths, `cfg!(test)` → `world_test`,
    `wow_world_application::` → `crate::`. **7 of 7 identical** after the
    rules (`move_token_compare.py --base 8e618d0e2`).
- **`CMSG_TRAINER_LIST` → `wow-world-application`** (`session_host.rs`):
  `handle_trainer_list` and `handle_trainer_list_for_gossip_option_like_cpp`
  (C++ `NPCHandler.cpp:98` `HandleTrainerListOpcode`/`SendTrainerList`;
  async, no await), the private `send_trainer_list_like_cpp` and
  `trainer_offer_decision_like_cpp` from `handlers/trainer.rs`, over the same
  host. `TrainerBuySpellHostLikeCpp` drops its three context/decision methods
  and takes `TrainerSessionHostLikeCpp` as supertrait; the purchase body
  calls the free functions at the same four points (two context builds,
  the admission build, the decision). Callers: the trainer registrar adapter
  and the character login host name `wow_world_application::handle_trainer_list*`.
- **Talent-point refresh → `wow-world-quest`** (`quest_session/rewards.rs`):
  `refresh_represented_talent_points_like_cpp` from
  `session/progression/talents.rs`, over the existing
  `QuestSessionHostLikeCpp::split_quest_state_mut` (no new host method). It
  cannot go to spell: its body is `QuestRewardCx`'s refresh, and spell is
  below quest. Rules (`1363-s12a-talent.json`): the signature and a prologue
  (hook, split, `HubMut` destructure, the production `let _ = quest_state`),
  `&self.catalogs` → `session_catalogs`, `&self.quest_state` → `&*quest_state`,
  `self.` → nothing, `cfg!(test)` → `world_test`, `wow_world_quest::` →
  `crate::`. **1 of 1 identical.** World callers (`persistence/load.rs` ×2,
  `player_binding.rs`, the `set_num_talents_at_level_store` setter and the
  spell `TalentHostLikeCpp` impl) call
  `wow_world_quest::refresh_represented_talent_points_like_cpp(self)`. The
  catalog setters stay.
- **Shims.** The `cfg(test)` `handle_trainer_buy_spell` entry point (a
  nested `#[cfg(test)]` method of `handlers/trainer.rs`) moved verbatim to
  the new `#![cfg(test)]` `handlers/trainer_talent_test_shims.rs`, with
  one-call forwards for the moved names the scenarios call
  (`handle_trainer_list`, `trainer_offer_decision_like_cpp`,
  `refresh_represented_talent_points_like_cpp`). The imports of
  `handlers/trainer.rs` that only the regressions read are `#[cfg(test)]`.
- **Retired with its last production reader** (commit `330756ef8`, found by
  the warning recipe): the session root's `ObjectGuidGenerator` import (the
  deleted builder file read it as `super::ObjectGuidGenerator`); its two
  `cfg(test)` readers import it from `wow_core` at the same line counts.
- **Inlined one-line wrappers:** none. The three removed
  `TrainerBuySpellHostLikeCpp` methods were World forwards to the moved
  builders and the decision; the purchase body now calls those free
  functions directly (`host.f(a)` → `f(host, a)`), so the forwards retire
  instead of being inlined.
- **Visibility.** `pub` for what World calls (the list handlers, the host
  trait and split struct); `trainer_offer_decision_like_cpp` is re-exported
  only under `cfg(any(test, feature = "test-fixtures"))` for the shim; the
  builders and `send_trainer_list_like_cpp` are crate-private.

### Host surface

Moved: **708 function lines** (counted at the base from the first doc or
attribute line through the closing brace): builders 639 (55 + 213 + 371),
list/decision 32, talent refresh 37.

- New host: the split struct 12 and the trait 14 (with the 7-line hook
  default) in `session_host.rs`, and the World impl 22 in
  `handlers/trainer/host.rs`: **48 lines, 6.8 %**. No adapter file was
  merged, so the merged and unmerged figures are the same.
- Retired host: the three trait methods with docs (21) and their World
  forwards (26): 47 lines. Net host +1.
- Not counted above, reported for completeness: the two new files' headers
  and imports (45) and the prologues inside the moved bodies (70 lines after
  rustfmt explodes the destructures; the moved bodies grew only 708 → 735,
  because rustfmt rejoined 43 `self\n.fixtures\n.x` chains). Everything
  counted: 48 + 45 + 27 = 120 lines, 16.9 %; net of the retired host, 73,
  10.3 %.

### Await counts (per function, base vs head)

| Function | Base | Head |
|---|---:|---:|
| `handle_trainer_buy_spell_with_generator_like_cpp` (caller, unmoved) | 5 | 5 |
| `handle_trainer_list`, `handle_trainer_list_for_gossip_option_like_cpp` (async) | 0 | 0 |
| the three builders, `send_trainer_list_like_cpp`, `trainer_offer_decision_like_cpp`, the talent refresh | 0 | 0 |

`TrainerHandlerHostLikeCpp::handle_trainer_list_like_cpp` keeps its one
await (now of the free function); the login host returns the future as
before.

### Recursion

- Every moved name was grepped in production (non-shim) `wow-world`: the
  builders, `send_trainer_list_like_cpp` and the decision have no hit;
  `handle_trainer_list*` appear only in the login host and the trainer
  registrar adapter, each calling `wow_world_application::…`; the talent
  refresh appears only as `wow_world_quest::refresh_represented_talent_points_like_cpp(self)`
  (five call sites, one of them the `TalentHostLikeCpp` method of the same
  name).
- The new forward (`split_trainer_mut`) builds a struct from fields. In
  application, the purchase body calls the imported free functions; the
  trait methods of those names no longer exist, so nothing can resolve to
  them. The shims name the crate paths.
- `unconditional_recursion`: 0 in all three canonical builds.

### Registration

No registrar moved; the trainer and talent registrations, their order,
`SessionStatus`, `PacketProcessing`, handler names and thunks are
unchanged (`handler_name: "handle_trainer_list"` is a string literal, not a
path). No checker constant, mount or `handler-module-policy.json` change, so
no handler-contract `check` is required; `final` runs the checker lib suite
because `Cargo.lock` and crate manifests changed.

### Edges and closure

`wow-world-application` → `wow-world-social` (the split names
`SessionSocialLimits`), simulated first: `wow_world_closure.py check --add
"wow-world-application->wow-world-social"` PASS in lock and metadata modes;
added to `dependency-policy.json` and to the application's `test-fixtures`
forward. `wow-world-instances` → itself (dev). Closure at head: PASS in
lock and metadata modes (18 packages). `cargo tree -p world-server -e
normal,build -e features` shows no `test-fixtures` and no
`wow-world-test-support`.

### Accounting

| Crate `src` | Base physical | Head physical | Base production | Head production |
|---|---:|---:|---:|---:|
| `wow-world` | 40,282 (453 files) | 39,586 (453) | **30,664** | **29,898** |
| `wow-world-application` | 14,708 | 15,462 | 13,993 | 14,745 |
| `wow-world-quest` | 19,021 | 19,071 | 18,437 | 18,487 |

- **`wow-world` production −766.** By file: `trainer_acquisition.rs` −658
  (deleted), `handlers/trainer.rs` −64, `progression/talents.rs` −37,
  `handlers/trainer/host.rs` −4, `session/mod.rs` −3.
  - **Moved:** 708 function lines.
  - **Reclassified production → test:** 27: the nested `cfg(test)`
    `handle_trainer_buy_spell` (19) and the regression-only imports of
    `handlers/trainer.rs` now gated (8).
  - **Retired:** 27: the deleted file's header, imports and `impl` lines
    (19), `handlers/trainer.rs`'s `Arc` import, `impl` lines and section
    header net of the new 3-line note (5), the `trainer_acquisition` mount
    and the `ObjectGuidGenerator` import reflow in `session/mod.rs` (3).
  - **Host adapter, net:** −4 (+22 split impl, −26 retired forwards, doc and
    import lines).
- **Destinations (production):** application +752 (`session_context.rs`
  675, `session_host.rs` 91, mounts and exports 9, `buy_spell.rs` −21 net
  of the removed trait methods and the import changes); quest +50
  (`rewards.rs` 49, the export 1).
- `net_move.py check --base origin/3.4.3`: S = 696, G = 804 within the
  1,691.8 allowance, 0 duplicate bodies, PASS.

### Policy and ceilings

- **Ceilings tightened, none raised.** Hotspot `session/mod.rs`: production
  21,272 → 20,574, test 141,630 unchanged, total 162,902 → 162,204 (the
  review is prepended to the field with `Previous: `). Physical
  `session/mod.rs` 820 → 817, `handlers/trainer.rs` 114 → 60.
- **New physical row** at its live size: `handlers/trainer_talent_test_shims.rs`
  (58), `#![cfg(test)]`, mounted from `handlers/mod.rs` outside the hotspot
  trees.
- `session-ownership-policy.json` reprinted with `print-baseline` (the moved
  methods, the new `TrainerSessionHostLikeCpp` impl and the shim);
  `check --syntax-only` PASS. `dependency-policy.json`: application → social.

### Warnings and tests

Canonical recipe (with `--locked`), base `8e618d0e2` vs head, counted and
compared with `warning_units.py`:

| Build | `wow-world` lib | `wow-world` lib test | All `wow-world*` lib units |
|---|---:|---:|---:|
| production lib | 120 → 118 | — | 247 → 245 |
| lib + `test-fixtures` | 129 → 127 | — | 211 → 209 |
| `--lib --tests` | 129 → 127 | 178 → 178 | 211 → 209 |

- `compare`: nothing only in the head in any build (after `330756ef8`).
  Gone: the two unused import groups of `handlers/trainer.rs`. The
  destination crates keep exactly their pre-existing warnings.
- `wow-world` `dead_code`: 0 in all three builds. `unconditional_recursion`: 0.
- **Tests.** The test-name multiset is identical: 9,492
  `#[test]`/`#[tokio::test]` functions under `crates/`, 5 ignored, base and
  head. No scenario call site changed; the only test-tree edit is the
  player-items f3 shim's import (`use {super::*, wow_core::ObjectGuidGenerator};`).
  No assertion changed. Focused run during the move: `cargo test -p
  wow-world --lib -- trainer talent respec handler_contract battle_pet`:
  282 passed, 1 ignored.

### Follow-ups

- #1407 follow-up 3 and #1405 nit 5 remain open.
- 12b–12d and the cleanup as re-planned above. The trainer purchase host
  (`TrainerBuySpellHostLikeCpp`, 9 methods) and the talent hosts are 12d
  adapter candidates.
- The standalone `--tests` check covers only changed domain manifests; a
  forwarding change that reaches a crate whose manifest is unchanged is
  still caught only by a manual loop.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section and the slice 12
plan. The manifest path, dirty flag and duration are in the PR body. This
section is not edited after that run, so the PR head is the validated SHA.

## Slice 12b plan: sub-slices, floor and done criterion (#1410 follow-ups 1 and 3)

Re-measured on `91d9f12bb` (12a merged) with the slice 12b1 index: production
inherent `WorldSession` methods outside shim files and `#![cfg(test)]` files,
methods of `cfg(test)` impls and `#[cfg(test)]` methods excluded, comments and
literals blanked, line counts from the first doc or attribute line through the
closing brace; "forward" means at most one statement and no
`let`/`if`/`match`/loop/`return`. It finds **377 non-forward methods (10,762
lines)** and 299 forwards (1,769 lines); the 12a index (390 / ~10.8k) counted
a few cfg-gated and shim-file methods more. `wow-world` production is 29,898.

### Sub-slices

Each sub-slice is one PR under the same rules as 12a (F5 inverted host trait,
bodies token-identical after documented mechanical rules, host ≤ 15 % merged
and unmerged, ≤ ~4.5k moved, closure simulated first, no ceiling raised).

| Sub-slice | Scope (World module → destination) | Non-forward methods / lines | Notes |
|---|---|---:|---|
| **12b1** | `session/player_items` → `wow-world-inventory`: durability loss and repair, item mods and the item-bonus stat publication, offhand auto-unequip, item-set item records, appearance/collection gates | 31 / ~1.2k (measured in the 12b1 result) | this PR; only what inventory can own without reaching spell, application, quest or player-conditions |
| 12b1b | `session/player_items` remainder: the application context builders and planning wrappers (equip, position publication, swap effects, committed relocation/swap/removal, storage/bank/equip planning: ~470) → `wow-world-application`; the inventory field-borrow builders (area-based scaling, average item level, equipment-set use context, ~190) → `wow-world-inventory` behind one split host method; loot/battle-pet leftovers (~100) | ~45 / ~1.0k planned; **21 functions / 659 lines moved** (10 non-forward / 483; 9 / 463 of them in `session/**`, the tenth in `handlers/character/items/host.rs`), see the 12b1b result | the aura/enchantment/obtain-spell paths below go with 12b2; the split wrappers go with 12c and the loot/publication leftovers with 12b7 |
| 12b2 | `session/spell_effects` (1,110) plus the player-items spell paths (durability-damage effect 48, loaded enchantments 205, initial item mods and set auras 93, obtain spells 63) → `wow-world-spell` | 13 + ~8 / ~1.5k; **11 functions / 625 lines moved** (to spell, and quest complete to quest), see the 12b2 result | the damage kill cascade, the nearby-entry pair, reputation, force deselect and the item-enchantment / item-mod / set-aura family (917) are 12b2b and 12b5, by layering |
| 12b3 | `session/spell_state` → `wow-world-spell` | 34 / 616 (+31 forwards, 196); re-measured 29 / 556 outside the setters; **16 moved / 198 lines, 2 / 51 reclassified to a shim**, plus the 12b5 effect pair (2 / 124) to social, see the 12b3 result | the spell-map dispatch (#754), the snapshot, the spell-hit authority pair, the aura-application builder, the two login passive-aura replays and the login spell reset stay (fixture seam or above spell) |
| 12b4 | `session/persistence`, `session/player_bootstrap`, `session/player_binding` → `wow-world-character`/`-lifecycle` | 24 / 934 | the login bootstrap (406) is one method; it moves only if its callers are domain crates |
| 12b5 | `session/social`, `session/chat` → `wow-world-social` | 21 / 406; re-measured 22 / 412; **14 moved / 357 to instances, social, character and quest; 4 field-split forwards retired**, see the 12b5–12b6 result | the reputation and force-deselect effects of `session/spell_effects` went with 12b3; the trade context builder and the group-guid seam are staying candidates |
| 12b6 | `session/quest`, `session/world_entities`, `session/visibility`, `session/deferred_visibility` → quest / entities / visibility | 26 / 644; re-measured 27 / 648; **9 moved / 179 (194 with the forward and module function) to entities; 1 field-split forward retired**, see the 12b5–12b6 result | the three context builders and the session-command visibility steps are staying candidates; the resurrection pair and the quest-state split seam wait for a fixture or 12c step |
| 12b7 | leftovers: `session/player_cast` (220), `session/stand_state_adapter` (311), `session/canonical_access` (232), money/xp/presentation/publication (491) | ~41 / ~1.25k | each measured again first; anything whose callers are not yet domain crates is recorded as staying |
| stays | runtime shell: `session/driver`, `session_commands.rs`, `session/map_admission.rs` | 23 / 1,273 | not moved by 12b |
| stays | construction API: `session/catalogs` and the store/config setters across `session/**` | 10 / 253 + 37 forwards | world-server construction API |

### Realistic floor and done criterion for #1363

- **Floor.** About **10–13k** `wow-world` production lines remain when phase 2
  is done, and that is the target, not a failure to shrink further:
  - the runtime shell (driver, `session_commands`, `map_admission`, the
    registry and dispatch, the session root and state struct): ~3k;
  - the World host impls: 84 `impl … for WorldSession` blocks in 38
    `*host*.rs` adapter files (4,699 lines) and 26 `*_handler_contexts.rs`
    files (1,320 lines), ~6k together;
  - the setters and construction API used by `world-server` (~1k);
  - the forwards that host impls and scenarios still call.
- **12c nets only a few hundred to ~1k lines.** A forward can be deleted only
  where every caller is repointed; most remaining forwards are the targets of
  host impls, which the deletion only moves into the impl.
- **12d regroups rather than removes.** It merges host traits that duplicate
  forwards and collapses the ~6k of adapter files into one adapter per
  destination crate, reporting merged and unmerged host shares. Its line delta
  is small.
- **The final cleanup reclassifies ~1.7k shim lines** (the 19 shim files
  without `#![cfg(test)]`) from production to test; that is a classifier
  change, not a move.
- **Done** when: every 12b sub-slice is merged or recorded here as staying
  with its reason; 12c, 12d and the cleanup are merged; no non-forward logic
  body remains in `session/**` outside the runtime shell and the construction
  API; every ceiling is tightened to its final value; and the final production
  figure with its split into the four groups above is recorded in the phase 2
  summary.
- **Mechanical form** (#1411 review follow-up 2; defined in the 12b1b result
  below): `python3 tools/architecture/session_method_index.py check --done`
  passes, i.e. 0 non-forward methods in `session/**` outside the explicit
  allow-list `tools/architecture/session-method-index.json`, whose staying set
  is bounded at 43 methods / 1,282 lines and may only shrink. (Re-baselined to
  44 / 1,190 by the #1413 index fixes, with `session/map_admission.rs` moved
  to the outside ratchet and a whole-crate ceiling added; see the 12b2
  result.)

### Standalone `--tests` over every domain crate (#1410 follow-up 3)

`final` now plans `cargo check --locked --tests -p <crate>` alone for every
existing `crates/wow-world-*` crate (17 today, in name order) whenever any
`crates/wow-world-*/Cargo.toml` changes or is deleted, not only for the changed
manifest: a `test-fixtures` forwarding change in one manifest can break a
dependent whose manifest did not change. A change to `wow-world`'s own
manifest, another crate's manifest or a domain source file does not trigger
it, and `quick` never does. Pinned in `tools/test_validation_v2.py` (two domain
fixture crates; a changed, a second changed and a deleted domain manifest each
plan both standalone checks, the other three changes plan none). Mutation
checks: disabling the branch fails the self-test (`AssertionError:
crates/wow-world-fixture/Cargo.toml`), and so does narrowing it back to the
changed crate. Documented in `docs/operations/validation-v2.md`.

## Slice 12b1 result (player-item durability, repair, item mods, offhand and appearance → inventory)

Branch `1363-s12b1-player-items`, base `91d9f12bb` (slice 12a merged).
Move-only (R2) after the plan/tooling commit above. Durability loss and
repair (including the money/durability persistence order, #169), item mods,
the item-bonus stat publication, the item-set item records, the offhand
auto-unequip (direct move, item-set and item-mod removal, mail fallback),
item storage/move/stack, the appearance gate and collection hook, item object
publication and every packet are unchanged; no registrar moved.

### Re-measure and the cut

`session/player_items` on the base: 86 non-forward methods (2,376 lines) and
57 forwards (394). Only the part inventory can own without reaching spell,
application, quest or player-conditions moves here (12b1a is the whole of
12b1; no second PR is needed for this destination). The rest is recorded in
the 12b plan as 12b1b (application builders, inventory field-borrow builders)
and 12b2 (aura, enchantment and obtain-spell paths, which call spell). After
the move `session/player_items` has 53 non-forward methods (1,187 lines) and
51 forwards (352); the whole index goes from 377 to 343 non-forward methods.

### What moved

Into `wow-world-inventory/src/player_items_session/` (commit `d2012215e`),
as free functions over the existing hosts:

| Head file | Functions (base file) | Base lines |
|---|---|---:|
| `durability.rs` | `apply_represented_durability_loss_all`, `…_points_loss_all`, `…_points_loss_at_slot`, `…_loss_at_slot`, `…_loss_item`, `…_points_loss`, `apply_durability_damage_pct_effect`, `represented_durability_loss_aura_multiplier`, `represented_prevent_durability_loss`, `apply_represented_durability_loss_on_death`, `publish_creature_melee_death_durability_loss`, `represented_player_in_battleground` (`durability.rs`) | 211 |
| `repair.rs` | `repair_inventory_item_durability_with_generator`, `apply_inventory_item_durability_repair_runtime`, `repair_all_…_with_player_money_and_generator` (`durability.rs`), `repair_all_…_with_guild_bank_and_generator` (`bank.rs`) | 359 |
| `item_mods.rs` | `record_represented_item_mods`, `send_represented_item_bonus_player_stat_update` (`modifiers.rs`), `record_represented_items_set_item`, `…_events` (`items.rs`) | 79 |
| `offhand.rs` | `represented_auto_unequip_offhand_if_need` (`offhand.rs`, deleted), `move_represented_direct_inventory_item_to_pos` (`storage_slots.rs`), `move_represented_direct_inventory_item`, `record_direct_inventory_item_set_remove` (`storage.rs`), `record_represented_offhand_item_mod_remove` (`items.rs`), `plan_store_existing_direct_inventory_item`, `plan_store_existing_inventory_item` (`persistence.rs`) | 220 |
| `appearance.rs` | `on_item_added_to_collection` (`items.rs`), `add_item_appearance_for_item`, `add_item_appearance_for_runtime_item`, `can_add_item_appearance_represented` (`appearance.rs`), the free `player_class_by_armor_subclass_like_cpp` (`session/item_modifiers.rs`) | 350 |
| **Total** | 31 methods and 1 free function | **1,219** |

(All names end in `_like_cpp`.) The bodies grew to 1,365 lines after rustfmt,
mostly from the inlined split blocks.

- **Host.** The functions take `H: VendorSessionHostLikeCpp` (the offhand
  group `H: ItemSessionHostLikeCpp`, whose `plan_store_existing_inventory_item_at_like_cpp`,
  titan-grip and average-item-level records it calls).
  - `VendorSessionHostLikeCpp` loses the three repair methods (the repair
    handler calls the free functions) and gains one,
    `item_appearance_known_spells_like_cpp` (the World impl names
    `wow_world_spell::known_spells_like_cpp`; inventory is below spell, and the
    name avoids the same-named methods of the login, loot, teleport and trade
    hosts). It keeps `on_item_added_to_collection_like_cpp`, whose World impl
    now names the free function, so vendor buy and the void-storage transfer
    are untouched (an earlier draft retired it too and grew
    `vendor_session/buy.rs` past its ceiling by 3 lines; commit `4e2245539`
    restores the method instead of raising the ceiling).
  - `ItemSessionHostLikeCpp` loses `send_represented_item_bonus_player_stat_update_like_cpp`;
    `item_mutations.rs` and `real_swap.rs` call the free function
    (`host.f()` → `crate::f(host)`).
  - **Fixture hook.** The former World `cfg!(test)` arguments and the
    `#[cfg(test)]` steps (the guild-bank withdraw record, the offhand request
    record and its `stored_destination`) read the private
    `world_test_consumer_like_cpp(host)` helper (`test_fixtures_ref_like_cpp().is_some()`
    under `test-fixtures`, `false` otherwise), the 6a/12a form: `Some`
    fixtures only in the World test build, so the value is `cfg!(test)` of
    `wow-world` in all three builds. Each step is `#[cfg(any(test, feature =
    "test-fixtures"))]` and runs only when the hook is true.
- **Mechanical rules** (`move_token_rules/1363-s12b1-{host,split}.json`,
  pairs alongside): the signatures (`&self`/`&mut self` → `host: &H`/`&mut H`
  with the bound and the head visibility); the hook prologue where the body
  reads `cfg!(test)` or has a `#[cfg(test)]` step; the field-split prologue
  (`let (inventory, hub) = host.split_inventory_mut(); let HubMut { core, … } = hub;`)
  for the two bodies that borrowed `self.core`/`self.catalogs`/`self.fixtures`
  and `self.inventory` together; the `#[cfg(test)]` step rewrites;
  `crate::session::split_inventory_{ref,mut}(self)`/`hub_{ref,mut}(self)` →
  the host methods; `wow_world_spell::known_spells_like_cpp(self)` → the new
  host method; the inlined wrappers below; `self.<moved>(` → `<moved>(host, `;
  `self.catalogs.`/`self.config.` → `host.hub_ref().…`, `self.core.` →
  `host.core().`, `self.lifecycle.` → `host.lifecycle().`, `self.inventory.` →
  `host.split_inventory_{mut,ref}().0.`; `self.` → `host.`; `cfg!(test)` →
  `world_test`; `wow_world_inventory::` → `crate::`. Rules never touch string
  literals. **32 of 32 identical** after the rules except
  `can_add_item_appearance_represented_like_cpp`, whose only difference is
  one rustfmt brace reflow (the `and_then(|skill| { … })` closure body lost
  its braces once the inlined call fit on one line).
- **Inlined one-line wrappers** (each verified to be one split plus one state
  call, or one catalog read): `resolved_inventory_item_like_cpp`,
  `resolved_inventory_item_object_like_cpp`, `resolved_inventory_item_objects_like_cpp`,
  `resolved_inventory_items_like_cpp`, `get_inventory_item_by_pos`,
  `send_bag_slot_values_update_like_cpp`, `represented_item_reputation_rank_like_cpp`
  (`split_inventory_ref` + call); `apply_inventory_item_object_updates_like_cpp`,
  `remove_inventory_item_like_cpp`, `remove_inventory_item_duration_refs_like_cpp`,
  `remove_inventory_tradeable_item_like_cpp`, `add_item_appearance_like_cpp`,
  `add_temporary_item_appearance_like_cpp` (`split_inventory_mut` + call);
  `add_account_heirloom_like_cpp` (`split_lifecycle_mut` + call);
  `item_modified_appearance_ref`, `item_modified_appearance_for_item`,
  `item_storage_template`, `item_template_quality` (`self.catalogs.<same>`);
  `kick` (`self.core.kick`). The host's `split_inventory_ref`,
  `split_inventory_mut`, `split_lifecycle_mut`, `lifecycle`, `hub_ref` and
  `hub_mut` are the World's same splits. Of these wrappers,
  `item_template_quality`, `represented_item_reputation_rank_like_cpp` and the
  two remove wrappers lost their last caller and are deleted; the appearance
  adds, the two modified-appearance reads and the heirloom add are called only
  by scenarios and moved to the shim.
- **World callers** name the free functions:
  `session/movement/host.rs`, `session/combat/host.rs`,
  `session/spell_effects/execution_host.rs` (the percent effect),
  `session/spell_state/spellbook_host.rs`, `handlers/loot/session_loot_host.rs`,
  `handlers/character/vendor.rs` (the collection hook), `session/persistence/load.rs`,
  `session/player_items/persistence_load.rs` and the bodies that stay:
  `apply_durability_damage_effect_like_cpp` (it logs through
  `wow_world_spell`, so it waits for 12b2), `apply_initial_loaded_item_mods_like_cpp`
  and `apply_initial_item_set_auras_like_cpp`.
- **Shims.** `handlers/player_items_test_shims.rs` (`#![cfg(test)]`,
  mounted from `handlers/mod.rs` outside the hotspot trees): the three nested
  `#[cfg(test)]` repair entry points (verbatim except that they call the free
  functions), one-call forwards for the 11 moved names the scenarios call, the
  five scenario-only wrappers above, and the offhand-request accessor from the
  retired `unit_tests/session/player_items/offhand/f3_shims.rs` (its mount in
  the deleted `offhand.rs` goes too). No scenario call site changes.
- **Retired with their last production reader** (commit `b36c50cd0`, found by
  the warning recipe): the session root's `is_represented_bag_slot`,
  `player_team_id_for_race_cpp`, `item_values_update_to_update_object` and
  `player_class_by_armor_subclass_like_cpp` imports and the `InventoryType`,
  `ItemFlags2` and nine `wow_entities` names; the names only the scenarios read
  (`RepresentedItemSetSpellEventLikeCpp`, `player_class_mask_for_transmog_like_cpp`,
  `ItemFlags3`, `ItemQuality`, `ItemSubClassArmor`, `ItemSubClassWeapon`,
  `EQUIPMENT_SLOT_OFFHAND`, `MAX_MONEY_AMOUNT`) become `cfg(test)` imports.
- **Visibility.** `pub` (re-exported from the crate root) for what World
  calls; `can_add_item_appearance_represented_like_cpp`,
  `record_represented_items_set_item_like_cpp`,
  `record_direct_inventory_item_set_remove_like_cpp`,
  `apply_inventory_item_durability_repair_runtime_like_cpp` and the three
  repair functions are re-exported only under `cfg(any(test, feature =
  "test-fixtures"))` for the shim (the repairs are `pub(crate)` otherwise, for
  the repair handler); the rest are module-private.
- **#1405 nit 5** (move `QuestItemStorageHostLikeCpp`'s 12 inventory ops to an
  inventory host) did not fall out: none of the moved bodies calls them. It
  stays a 12d adapter-merge item.

### Host surface

Moved: **1,219 function lines** (from the first doc or attribute line through
the closing brace, at the base).

- New host: the trait method with its doc (3) and its World impl (3), and the
  fixture-hook helper (11): **17 lines, 1.4 %**. No adapter file was merged,
  so the merged and unmerged figures are the same.
- Retired host: the three `VendorSessionHostLikeCpp` repair methods (20) and
  `ItemSessionHostLikeCpp::send_represented_item_bonus_player_stat_update_like_cpp`
  (1), and their World forwards (43 + 3): **67 lines**. Net host −50.
- Everything counted, reported for completeness: the new files' headers,
  imports, exports and blank lines between functions (210 lines including the
  11-line helper) plus the trait method and impl (6): 216 lines, 17.7 %; net
  of the retired host, 149, 12.2 %. The prologues and the rustfmt growth of
  the inlined blocks are inside the 1,365 head function lines.

### Await counts (per function, base vs head)

| Function | Base | Head |
|---|---:|---:|
| `repair_inventory_item_durability_with_generator_like_cpp` | 4 | 4 |
| `repair_all_inventory_item_durability_with_player_money_and_generator_like_cpp` | 3 | 3 |
| `repair_all_inventory_item_durability_with_guild_bank_and_generator_like_cpp` | 1 | 1 |
| `handle_repair_item_with_generator_like_cpp` (caller, unmoved) | 3 | 3 |
| the three shim repair entry points | 1 each | 1 each |
| every other moved function | 0 | 0 |

### Recursion

- Every moved name (and every retired or shimmed wrapper) was grepped in
  production (non-shim) `wow-world`: each hit is a `wow_world_inventory::<name>(self, …)`
  call (movement, combat, spell execution, spellbook, loot and vendor host
  impls; `persistence/load.rs`, `persistence_load.rs`; the three World bodies
  that stay) or the host-impl method declaration itself; there is no
  `WorldSession::<moved>` or `self.<moved>(` left.
- The trait methods that kept a moved name (`VendorSessionHostLikeCpp::on_item_added_to_collection_like_cpp`,
  the loot-ops host's method of the same name, the spell execution, movement,
  combat and spellbook host methods) have World impls that name the free
  function by path; the free functions call no host method of their own name.
- The only new forward, `item_appearance_known_spells_like_cpp`, calls
  `wow_world_spell::known_spells_like_cpp`, an explicit free function.
- `unconditional_recursion`: 0 in all three canonical builds.

### Registration

No registrar moved; the item, vendor, repair and bank registrations, their
order, `SessionStatus`, `PacketProcessing`, handler names and thunks are
unchanged. No checker constant, mount or `handler-module-policy.json` change.

### Edges and closure

No new crate edge (inventory already depends on core, entities, lifecycle,
loot, persistence, packet and constants); no manifest or `Cargo.lock` change.
Closure at head: PASS in lock and metadata modes (18 packages). Nothing in the
moved code names `wow_world_spell`, `wow_world_application`,
`wow_world_quest`, `wow_world_player_conditions`, `wow_world_social` or
`wow_world_instances`.

### Accounting

| Crate `src` | Base physical | Head physical | Base production | Head production |
|---|---:|---:|---:|---:|
| `wow-world` | 39,586 (453 files) | 38,420 (453) | **29,898** | **28,528** |
| `wow-world-inventory` | 23,855 (87) | 25,421 (93) | 23,463 | 25,007 |

- **`wow-world` production −1,370:**
  - **Moved:** 1,219 function lines.
  - **Reclassified production → test:** 80 function lines (the three nested
    `#[cfg(test)]` repair entry points, 46, and the five scenario-only
    wrappers, 34, now in the `#![cfg(test)]` shim).
  - **Retired:** 29 (the four dead wrappers, 18; the deleted `offhand.rs`
    header and empty impl, 10; its mount line, 1) and 17 import lines in
    `session/mod.rs` and `session/item_modifiers.rs` (of the names they held,
    eight became `cfg(test)` imports, +11 test lines).
  - **Host adapter, net:** −43 (−46 retired forwards, +3 new impl).
  - **Caller rewrites:** +18 (the path calls rustfmt wraps in five World
    files).
  - −1,219 − 80 − 29 − 17 − 43 + 18 = −1,370.
- **Destination (production):** inventory +1,544: `player_items_session/`
  1,553, `lib.rs` export 4, `vendor_session/repair.rs` +5 (the import),
  `vendor_session/mod.rs` −17 (−20 repair methods, +3 new method),
  `item_session/mod.rs` −1.
- `net_move.py check --base origin/3.4.3`: S = 1,166, G = 1,566 within the
  2,185.3 allowance, 0 duplicate bodies, PASS.

### Policy and ceilings

- **Ceilings tightened, none raised.** Hotspot `session/mod.rs`: production
  20,574 → 19,247, test 141,630 → 141,621, total 162,204 → 160,868. Hotspot
  `handlers/character/mod.rs`: production 2,287 → 2,244, test 15,655
  unchanged (the committed-money publication scan follows the repairs to
  `player_items_session/repair.rs` at the same line count), total 17,942 →
  17,899. Physical `session/mod.rs` 817 → 814, `handlers/character/vendor.rs`
  232 → 192, `handlers/trainer_talent_test_shims.rs` 58 → 57 (#1410 nit 4: the
  redundant inner `#[cfg(test)]` is dropped).
- **New physical row** at its live size: `handlers/player_items_test_shims.rs`
  (195), `#![cfg(test)]`.
- `session-ownership-policy.json` reprinted with `print-baseline` (the moved
  and retired methods, the shim surface, the new host method);
  `check --syntax-only` PASS. `dependency-policy.json` unchanged.

### Warnings and tests

Canonical recipe (with `--locked`), base `91d9f12bb` vs head, counted and
compared with `warning_units.py`:

| Build | `wow-world` lib | `wow-world` lib test | All `wow-world*` lib units |
|---|---:|---:|---:|
| production lib | 118 → 118 | — | 245 → 245 |
| lib + `test-fixtures` | 127 → 127 | — | 209 → 209 |
| `--lib --tests` | 127 → 127 | 178 → 178 | 209 → 209 |

- `compare`: nothing only in the base or only in the head in any build
  (after `b36c50cd0`; before it, the eight now-unused import groups above).
  `wow-world-inventory` keeps exactly its pre-existing warnings (11 / 8 / 8).
- `wow-world` `dead_code`: 0 in all three builds. `unconditional_recursion`: 0.
- **Tests.** The test-name multiset is identical: 9,492
  `#[test]`/`#[tokio::test]` functions under `crates/`, 5 ignored, base and
  head. No scenario call site changed. The one test-tree edit is the
  committed-money publication scan (`unit_tests/handlers/character_tests/loot.rs`):
  it drops the deleted `offhand.rs` from its include list, adds
  `player_items_session/repair.rs`, and its two durability needles take the
  bodies' rewrite (`self.stage_player_money_change_like_cpp` →
  `host.stage_player_money_change_like_cpp`,
  `self.apply_inventory_item_durability_repair_runtime_like_cpp(item_guid)` →
  `apply_inventory_item_durability_repair_runtime_like_cpp(host, item_guid)`),
  with the same two assertions (no await, the runtime publication before the
  money-guard drop). No assertion changed. Focused run during the move:
  `cargo test -p wow-world --lib`: 3,680 passed, 1 ignored.

### Follow-ups

- 12b1b and 12b2 as planned above; the stayed
  `apply_durability_damage_effect_like_cpp` moves with 12b2.
- #1405 nit 5 stays a 12d item; #1407 follow-up 3 remains open.
- `VendorSessionHostLikeCpp::on_item_added_to_collection_like_cpp` is now a
  forward to a free function of the same crate; 12d can retire it once
  `vendor_session/buy.rs` has room (or is split).

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section. The manifest path,
dirty flag and duration are in the PR body. This section is not edited after
that run, so the PR head is the validated SHA.

## Slice 12b1b result (player-items application builders, inventory field-borrow builders and leftovers)

Branch `1363-s12b1b-player-items`, base `e5c8195b0` (slice 12b1 merged).

### Mechanical done criterion (#1411 review follow-ups 2 and 3)

- **Index tool.** `tools/architecture/session_method_index.py` is the 12b1
  index, committed with a self-test:
  - `index [--methods] [--json PATH]` prints the totals and one row per module
    (`session/<dir>`, `handlers/<dir>`, else the file) with non-forward and
    forward counts and lines;
  - `check [--done]` applies the policy below;
  - `self-test` pins the parser (forward/non-forward, literal and comment
    masking, `#[cfg(test)]` methods and impls, impls inside a top-level
    `cfg(test)` module, trait impls ignored, `#![cfg(test)]` and shim files
    skipped) and the policy arithmetic.
- **Definition.** It counts the production inherent `WorldSession` methods in
  `crates/wow-world/src/**.rs` with the 12b plan's rules (shim files and
  `#![cfg(test)]` files skipped, `cfg(test)` impls and methods excluded, lines
  from the first doc or attribute line through the closing brace; a forward
  has at most one `;` and no `let`/`if`/`match`/`for`/`while`/`loop`/`return`).
  It also skips impl blocks inside a top-level `cfg(test)` item, and it takes
  line numbers from the masked text (which keeps every newline) instead of
  mixing byte offsets of the masked text with character offsets of the
  source. The 12b1 scratch index did the latter, so after non-ASCII text its
  attribute look-up drifted by a few lines and missed three `#[cfg(test)]`
  methods: on `e5c8195b0` the tool drops `update`, `process_pending` and
  `session_handler_catalogs_for_test_like_cpp` in `session/driver/mod.rs`, so
  the index reads **340 non-forward methods (9,513 lines) and 293 forwards
  (1,729 lines)** instead of 343 / 9,569.
- **Allow-list** (`tools/architecture/session-method-index.json`, paths under
  `crates/wow-world/src`):
  - runtime shell: `session/driver/` (18 methods), `session/map_admission.rs`
    (1), `session/construction.rs::new`,
    `session/connection.rs::poll_instance_link_with_module_registry_like_cpp`
    (called only by the driver and `connection.rs`) and
    `session/connection_identity.rs::set_state`;
  - construction API: the 21 non-forward store/registry/authority setters
    `world-server` calls from `session_resources.rs` / `session_factory.rs`,
    named one by one.
  - Everything else in `session/**` is logic that must leave. In particular
    the rest of `session/catalogs` (the rest-bonus ticks and
    `send_session_init_packets_with_policy_like_cpp`, 6 methods / 233 lines)
    is **not** construction API: the 12b plan's "construction API 10 / 253"
    row overstated it and those six join 12b7. `session_commands.rs` (1 / 193)
    and `handlers/**` are outside `session/**` and outside the done scope.
- **Bounds.** The staying set is **43 methods / 1,282 lines** on the base and
  `staying_bound` pins it (it may only shrink). The non-forward methods in
  `session/**` outside the allow-list are a ratchet, `outside_ceiling`:
  **270 / 7,320** on the base, tightened by each 12b slice; the #1363 done
  criterion is `check --done` (outside = 0).
- **Wiring.** `final` plans `session_method_index.py check` for any
  `crates/wow-world/src/**` change and for the tool, its policy or
  `hotspot_metrics.py`, and the tool's `self-test` when the tool changes;
  `quick` never does (pinned in `tools/test_validation_v2.py`; dropping the
  `crates/wow-world/src/` trigger fails it with `AssertionError:
  crates/wow-world/src/session/mod.rs`). Documented in
  `docs/operations/validation-v2.md`.
- **Follow-up 3.** The standalone `--tests` checks now take `-p` from the
  manifest's `[package] name` (directory name only if the manifest cannot be
  read). The fixture gives `wow-world-other` the package name
  `wow-world-other-package`; reverting to the directory name fails the
  self-test (`AssertionError: crates/wow-world-fixture/Cargo.toml`).

### Re-measure and the cut

Re-measured on `e5c8195b0` with the committed index: `session/player_items`
has 53 non-forward methods (1,187 lines) and 51 forwards (352). This slice
moves what has a clean owner and an existing or single new host:

- to `wow-world-application`: the equip, position-publication and swap-effect
  context builders, the swap side effects, the raw equip and the position
  publication, the committed relocation/swap/removal publications, and the
  storage, bank and equip planning with the swap-target validation;
- to `wow-world-inventory`: the area-based item-level scaling application and
  the equipment-set use context (one new split host method), and the uncage
  item-state load (battle-pet leftover).

**21 functions, 659 base lines** (10 non-forward methods / 483 lines, the
rest forwards; corrected after #1413's review: 9 / 463 of them are in
`session/**`, the tenth, `validate_inventory_swap_target_like_cpp` (20), was in
`handlers/character/items/host.rs`). Far under the 4.5k limit; no split needed.

**Deferred** (recorded, not moved):
- **To 12b2 (spell).** The spell paths: the durability-damage effect (53),
  loaded enchantments (60 + 77 + 68), initial item mods and set auras (53 + 44
  plus two 7-line helpers) and the obtain spells (63).
- **To 12c.** About 28 `split_inventory_{ref,mut}` + one-state-call wrappers
  (4–22 lines each, about 180 lines). The index counts them as non-forward
  because of their `let (state, hub) = …` split. They have many World and
  scenario callers, so they go with the forward sweep, which has to retire
  them to reach `check --done`.
- **To 12b7.** The loot/publication leftovers:
  - `remove_fully_looted_runtime_item` (17: a loot-ops host caller, but four
    World calls inside);
  - `represented_bag_contains_active_item_loot_like_cpp` (21: `&self` loot
    and inventory reads that no existing inventory host lends together);
  - `send_new_item_plan` and `broadcast_item_push_result_to_group` (39: they
    read the group registry through the World `resolved_group_guid_like_cpp`);
  - `record_represented_avg_equipped_item_level_update_like_cpp` (13: a
    `#[cfg(test)]`-only body).
- **To 12c.** `represented_average_item_level_like_cpp` (14, a forward; its
  callers are the social LFG host and the chat context).

After the move `session/player_items` has 44 non-forward methods (724 lines)
and 42 forwards (190). The whole index goes from 340 / 9,513 to **330
non-forward methods / 9,030 lines** (forwards 293 / 1,729 → 282 / 1,553). The
`session/**` outside-allow-list ratchet goes from 270 / 7,320 to **261 /
6,857**; the staying set is unchanged at 43 / 1,282.

### What moved

| Destination file | Functions (base file) | Base lines |
|---|---|---:|
| `wow-world-application/src/player_items_session/builders.rs` | `inventory_equip_cx_like_cpp` (125), `inventory_position_publication_cx_like_cpp` (44) (`publication.rs`); `inventory_swap_effects_cx_like_cpp` (47), `apply_inventory_item_remove_side_effects_like_cpp` (10), `apply_inventory_item_store_side_effects_like_cpp` (11) (`storage.rs`); `execute_inventory_equip_to_empty_raw_like_cpp` (10), `publish_inventory_position_changes_like_cpp` (4) (`handlers/character/items/host.rs`) | 251 |
| `…/player_items_session/committed.rs` | `apply_committed_inventory_item_{relocation,swap,removal}_like_cpp` (28 + 24 + 20, `persistence.rs`) | 72 |
| `…/player_items_session/planning.rs` | `validate_inventory_swap_target_like_cpp` (20, `handlers/character/items/host.rs`); `plan_equip_existing_inventory_item_like_cpp` (17, `equipment.rs`); `plan_bank_existing_inventory_item_at_like_cpp` (18, `bank.rs`); `plan_store_existing_inventory_item_at_like_cpp` (18), `plan_store_new_direct_inventory_item{,_at,_with_overlays_like_cpp}` (7 + 18 + 18), `plan_store_direct_inventory_item_like_cpp` (26) (`persistence.rs`) | 142 |
| `wow-world-inventory/src/player_items_session/scaling.rs` | `update_represented_item_level_area_based_scaling_with_publication_like_cpp` (`valuation.rs`) | 78 |
| `…/player_items_session/equipment_sets.rs` | `build_equipment_set_use_context_like_cpp` (`equipment_sets.rs`) | 96 |
| `wow-world-inventory/src/battle_pet_session/item_state.rs` | `uncage_item_state_like_cpp` (`items.rs`) | 20 |
| **Total** | 21 functions | **659** |

The bodies grew to 742 lines after rustfmt, mostly from the exploded
split destructures.

- **Hosts.**
  - **Application builders, committed publications, side effects, raw equip,
    position publication.** `H: TrainerSessionHostLikeCpp`, through its
    existing `split_trainer_mut`. Slice 12a introduced that split, and it
    holds exactly the disjoint field borrows these bodies took (the hub,
    `lifecycle`, `inventory`, `spell_state`, `quest_state`, `loot`, …). The
    former World `cfg!(test)` argument is its `world_test_consumer_like_cpp`.
    The trait's trainer name is now too narrow. Renaming it (for example to a
    session-fields split) is a 12d adapter-merge item: R2 forbids a rename
    inside a move slice, and a second identical split would only duplicate
    host code.
  - **Planning and the swap-target validation.** `H: QuestSessionHostLikeCpp`,
    whose `player_condition_projection_cx_like_cpp` is the World impl
    forwarding to the same `WorldSession::player_condition_projection_cx_like_cpp`
    the bodies called. `host.core()` replaces `self.core`.
  - **Inventory builders.** The new **`PlayerItemsSessionHostLikeCpp:
    SessionHubHostLikeCpp`** in `player_items_session/mod.rs`, with one
    method, `split_player_items_mut`, returning
    **`PlayerItemsSessionFieldsLikeCpp`**: the `HubMut`, `inventory`, `loot`,
    and, under `cfg(any(test, feature = "test-fixtures"))`, `spell_state` and
    `quest_state` as the `&dyn` fixture sources the registry hydration takes.
    The World impl sits in `handlers/character/items/host.rs`. The 12b1
    `world_test_consumer_like_cpp(host)` helper replaces `cfg!(test)`.
  - **Uncage.** `H: BattlePetSessionHostLikeCpp`, whose `lifecycle()` replaces
    `self.lifecycle`. The trait keeps `uncage_item_state_like_cpp` (two
    inventory callers); its World impl names the free function.
- **Item-level constants.** `Self::{MIN,MAX}_ITEM_LEVEL_LIKE_CPP` (World
  associated consts 1 and 1300) → `wow_world_player_conditions::{MIN,MAX}_ITEM_LEVEL_LIKE_CPP`.
  Those are the same values, already used by the valuation this cx feeds.
  `wow-world-player-conditions` now re-exports them; the line count is
  unchanged.
- **Mechanical rules**, with pairs alongside:
  - `move_token_rules/1363-s12b1b-app-split.json`:
    - the signatures and the split prologue (`let world_test = …;` where the
      body reads `cfg!(test)`, then `let TrainerSessionFieldsLikeCpp { hub:
      HubMut { core, catalogs, … }, inventory, … } = host.split_trainer_mut();`
      with the production-unused bindings under `cfg(any(test, feature =
      "test-fixtures"))`);
    - `self.<builder>()` → `<builder>(host)`;
    - `&mut self.inventory` → `inventory`;
    - `&self.{inventory,lifecycle,spell_state,quest_state}` → `&*…`;
    - `&self.{loot,catalogs,config}` → the binding;
    - `self.` → nothing;
    - the constants;
    - `cfg!(test)` → `world_test`;
    - `wow_world_application::` → `crate::`.
  - `1363-s12b1b-app-plan.json`:
    - the signatures (`&self` → `host: &H`);
    - `self.<moved>(` → `<moved>(host, `;
    - `self.player_condition_projection_cx_like_cpp()` → `host.…()`;
    - `self.core.` → `host.core().`;
    - `wow_world_application::` → `crate::`.
  - `1363-s12b1b-inventory.json`: the same field-split form over
    `split_player_items_mut`, `self.lifecycle.` → `host.lifecycle().`,
    `cfg!(test)` → `world_test` and `wow_world_inventory::` → `crate::`.
  - Rules never touch string literals.
  - **Result: 20 of 21 identical** (`move_token_compare.py --base e5c8195b0`:
    10/10, 8/8, 2/3). The one difference is
    `build_equipment_set_use_context_like_cpp`'s former `#[cfg(test)]` step.
    The base had `#[cfg(test)] let registry_hydration = Some(X);
    #[cfg(all(not(test), feature = "test-fixtures"))] let registry_hydration
    = None;`. The head has `#[cfg(any(test, feature = "test-fixtures"))] let
    registry_hydration = if world_test { Some(X) } else { None };`. That is
    the 12b1 fixture-step form: `Some` exactly in the World test build. A
    rule cannot express it because the base span contains the
    `"test-fixtures"` literal. A scripted check confirms it is the only
    difference: applying that substitution to the rewritten base gives the
    head token stream exactly.
- **Inlined one-line wrappers:** none. The bodies call only the moved
  functions, the split, the projection, `core()` and `lifecycle()`.
- **World callers** name the free functions:
  - the `ItemSessionHostLikeCpp` impl (`handlers/character/items/host.rs`);
  - the vendor adapter (`handlers/character/vendor.rs`, through a
    `wow_world_application as application` alias on an existing
    `handlers/character/mod.rs` import line, so the file keeps its 192-line
    ceiling);
  - the quest item-storage, loot and void-storage hosts;
  - the character login host (the scaling);
  - the `EquipmentSetUseHandlerHostLikeCpp` impl;
  - the battle-pet host (uncage).
- **Shims.** `handlers/player_item_builders_test_shims.rs` (`#![cfg(test)]`,
  mounted from `handlers/mod.rs` outside the hotspot trees) holds one-call
  forwards for the 16 moved names the scenarios call. No scenario call site
  changes. A new file was needed because `player_items_test_shims.rs` is
  ratcheted at 195.
- **Source scan.** `unit_tests/session/tests/lifecycle_persistence/player_persistence.rs`
  (`uncage_item_state_seam_no_longer_names_statement_or_driver_errors`)
  follows the body to `wow-world-inventory/src/battle_pet_session/item_state.rs`:
  - new needle `pub async fn uncage_item_state_like_cpp`;
  - the end marker is the top-level `\n}\n`;
  - the same four assertions;
  - the same line count.
- **Retired with their last production reader:**
  - the session root's `DirectInventoryStorageOverlayLikeCpp` re-export and
    the one in `inventory_request_contracts.rs` (the void-storage scenarios
    now import it from `wow_world_inventory`, inside their existing
    `cfg(test)` block);
  - the root's `NULL_BAG` import;
  - `ItemPosCount` and `NULL_SLOT` join an existing `cfg(test)` import.
- **Visibility.** `pub`, re-exported from each crate root, for what World
  calls. The context builders and `plan_store_direct_inventory_item_like_cpp`
  stay module-private.

### Host surface

Moved: **659 function lines** (base, first doc or attribute line through the
closing brace).

- **New host:** 38 lines, **5.8 %**:
  - `PlayerItemsSessionFieldsLikeCpp` and `PlayerItemsSessionHostLikeCpp`
    with their docs (17);
  - the World impl (21).
- The application part reuses two existing hosts and adds none. No adapter
  file was merged, so the merged and unmerged figures are the same.
- **Retired host:** none. The trait methods the World adapters implement
  stay, now naming the free functions.
- **Everything counted**, reported for completeness: the destinations grew by
  912 production lines.
  - 742 of them are the moved bodies (after rustfmt).
  - The other 170 are the seven new files' headers and imports, the
    application module root (37), the re-exports and the 17-line host
    declaration.
  - With the 21-line World impl, that is 191 lines, **29.0 %**. The new-file
    overhead is high because the 21 bodies landed in six new files.

### Await counts (per function, base vs head)

| Function | Base | Head |
|---|---:|---:|
| `execute_inventory_equip_to_empty_raw_like_cpp` | 1 | 1 |
| `uncage_item_state_like_cpp` | 1 | 1 |
| the two shim entry points of the same names | — | 1 each |
| every other moved function | 0 | 0 |

The World trait impls that return these futures (`ItemSessionHostLikeCpp::execute_inventory_equip_to_empty_raw_like_cpp`
and `BattlePetSessionHostLikeCpp::uncage_item_state_like_cpp`) return them
unawaited at base and at head.

### Recursion

- **Production grep.** Every moved name was grepped in production
  (non-shim) `wow-world`. Each hit is either a path call
  (`wow_world_application::<name>(self, …)`, `application::<name>(self, …)`
  or `wow_world_inventory::<name>(self, …)`) or a trait-impl method
  declaration. There is no `WorldSession::<moved>` or `self.<moved>(` left.
  - In particular `handlers/loot/session_loot_host.rs` called
    `self.plan_store_new_direct_inventory_item(…)` by method syntax. Without
    the inherent method, that call would have resolved to the trait method
    itself. It now names the free function.
- **Free functions.** The moved functions call only `split_trainer_mut`,
  `world_test_consumer_like_cpp`, `player_condition_projection_cx_like_cpp`,
  `core`, `split_player_items_mut` and `lifecycle` on the host. None of these
  shares a name with a moved function.
- `unconditional_recursion`: 0 in all three canonical builds.

### Registration

No registrar moved. The item, bank, equipment-set, void-storage, loot and
quest registrations, their order, `SessionStatus`, `PacketProcessing`,
handler names and thunks are unchanged. No checker constant or
`handler-module-policy.json` change.

### Edges and closure

- No new crate edge: application already depends on inventory, quest and
  player-conditions; inventory already depends on core and loot.
- No manifest or `Cargo.lock` change.
- Closure at head: PASS (lock mode, 18 packages).
- The moved inventory code names no crate above inventory.

### Accounting

| Crate `src` | Base physical | Head physical | Base production | Head production |
|---|---:|---:|---:|---:|
| `wow-world` | 38,420 (453 files) | 38,010 (454) | **28,528** | **27,884** |
| `wow-world-inventory` | 25,421 (93) | 25,724 (96) | 25,007 | 25,306 |
| `wow-world-application` | 15,462 (59) | 16,075 (63) | 14,745 | 15,358 |
| `wow-world-player-conditions` | 2,029 (6) | 2,029 (6) | 2,025 | 2,025 |

- **`wow-world` production −644:**
  - **Moved:** −659 function lines.
  - **Reclassified production → test:** 0. The shim forwards are new test
    lines (+233 in the shim, +2 for its mount).
  - **Retired:** −7: three import/re-export lines; the rest (4) is the
    emptied inherent `impl WorldSession { … }` block in `items/host.rs` and
    separator blank lines left by the deleted methods (the residual of the
    per-file deltas).
  - **Host adapter:** +22 (the World `PlayerItemsSessionHostLikeCpp` impl and
    its separator).
  - **Caller rewrites:** 0 net.
  - −659 − 7 + 22 = −644.
- **Destination production:** inventory +299, application +613,
  player-conditions 0.
- **`net_move.py check --base origin/3.4.3`:** S = 410, G = 916 within the
  1,391.5 allowance, 0 duplicate bodies, PASS.

### Policy and ceilings

- **Ceilings tightened, none raised:**
  - **Hotspot `session/mod.rs`:** production 19,247 → 18,618 and total
    160,868 → 160,239; test unchanged at 141,621. The uncage scan edit keeps
    its line count.
  - **Hotspot `handlers/character/mod.rs`:** production 2,244 → 2,229 and
    total 17,899 → 17,884; test unchanged at 15,655.
  - **Physical `session/mod.rs`:** 814 → 813.
  - **`session-method-index.json` `outside_ceiling`:** 270 / 7,320 → 261 /
    6,857.
- `handlers/character/vendor.rs` stays at its 192 ceiling. The alias keeps
  the rewritten calls on one line each.
- `handlers/player_item_builders_test_shims.rs` (233 lines, `#![cfg(test)]`)
  needs no physical row.
- `session-ownership-policy.json` is reprinted with `print-baseline` (the
  moved methods, the shim surface, the new impl); `check --syntax-only`
  PASS. `dependency-policy.json` is unchanged.

### Warnings and tests

Canonical recipe (with `--locked`), base `e5c8195b0` vs head, counted and
compared with `warning_units.py`:

| Build | `wow-world` lib | `wow-world` lib test | All `wow-world*` lib units |
|---|---:|---:|---:|
| production lib | 118 → 118 | — | 245 → 245 |
| lib + `test-fixtures` | 127 → 127 | — | 209 → 209 |
| `--lib --tests` | 127 → 127 | 178 → 178 | 209 → 209 |

- **`compare`:** nothing appears only in the base or only in the head, in
  any build. Before the import retirement above, the head had the
  `DirectInventoryStorageOverlayLikeCpp` re-exports and `ItemPosCount`,
  `NULL_BAG`, `NULL_SLOT` as new unused imports.
- `wow-world-inventory`, `wow-world-application` and
  `wow-world-player-conditions` keep exactly their pre-existing warnings.
- **`wow-world`:** `dead_code` 0 in all three builds; `unconditional_recursion`
  0.
- **Tests.** The test-name multiset is identical: 9,492
  `#[test]`/`#[tokio::test]` functions under `crates/`, 5 ignored, at base
  and at head. No scenario call site changed. No assertion changed; the only
  test-tree edit is the uncage scan above.
- **Focused run during the move:** `cargo test -p wow-world --lib`: 3,680
  passed, 1 ignored. `cargo check --tests` passes alone for inventory,
  application and player-conditions.

### Follow-ups

- **12b2:** the spell paths above, plus `apply_durability_damage_effect_like_cpp`.
- **12c:** the split + one-call wrappers and `represented_average_item_level_like_cpp`.
- **12b7:** the loot/publication leftovers listed under the cut.
- **12d:**
  - rename `TrainerSessionHostLikeCpp`/`TrainerSessionFieldsLikeCpp` to a
    general session-fields split;
  - consider giving the planning functions a narrower host than
    `QuestSessionHostLikeCpp`;
  - the 12b1 carry-overs (the vendor collection-hook forward, #1405 nit 5).
- **Shims.** The `#![cfg(test)]` shims retire in the final cleanup.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section. The manifest path,
dirty flag and duration are in the PR body. This section is not edited after
that run, so the PR head is the validated SHA.

## Slice 12b2 result (spell-effect and player-item spell bodies → spell, quest complete → quest)

Branch `1363-s12b2-spell-effects`, base `aa034f7e0` (slice 12b1b merged).

### #1413 follow-ups: the index tool, commit `cc1920b8c`

The review of #1413 found five gaps in `tools/architecture/session_method_index.py`.
All are fixed in a separate commit, before anything moved:

- **F1 (qualified impls).** `IMPL_RE` matched only `impl WorldSession {`. It now
  also matches path-qualified impls (`impl super::WorldSession`,
  `impl super::super::WorldSession`, `impl crate::session::WorldSession`) and
  generic ones (`impl<'a> WorldSession`, also across a line break before `{`).
  It still skips trait impls (`impl Trait for WorldSession`,
  `impl crate::Trait for WorldSession`) and `WorldSessionExt`.
  - Newly indexed in production: `session/dispatch.rs` (`dispatch_packet` 92,
    `is_status_allowed` 18), `session/admission.rs`
    (`evaluate_packet_spoof_like_cpp` 48, `flush_packet_spoof_ban_like_cpp` 4,
    and the forward `packet_spoof_now_secs_like_cpp`),
    `handlers/account_data/client_state.rs`, `handlers/entities/corpse.rs`,
    `handlers/travel/travel.rs` and `spell_acquisition/adapter.rs` (5 methods).
  - `session/driver/phases.rs` (`impl super::super::WorldSession`) holds only
    two `#[cfg(test)]` methods, so it adds nothing.
  - **Classified explicitly.** The dispatch and admission methods are C++
    `WorldSession::Update` packet dispatch and `DosProtection`, so they are the
    runtime shell. They are allow-listed method by method; the
    `session/driver/` entry already covers `phases.rs`.
- **F2 (`map_admission.rs`).** `access_requirement_abort_like_cpp` (176) is
  C++ `Map::PlayerCannotEnter` / `AccessRequirement` gameplay logic, not shell.
  It leaves the allow-list and counts in the outside ratchet; its planned home
  is `wow-world-instances`.
- **F3 and F4 (production by cfg, not by file name).**
  - Production is now decided along the module tree. The walk starts at
    `lib.rs` and follows `mod x;`, including `#[path]` and nesting in inline
    `mod m { mod x; }` blocks.
  - Any module, impl, method or nested item is excluded when its `cfg(...)`
    is off in a build without `test` and without the dev-only `test-fixtures`
    feature. An inner `#![cfg(..)]` gates its whole file or block. The cfg
    predicate is evaluated with Kleene logic; an unknown cfg counts as
    possibly on, so it never hides code.
  - The file-name rule for shims is gone. No production shim file exists:
    every `*shim*` file is mounted under a gated `mod`.
  - Excluded now, all correctly:
    - `session/driver/cross_crate_fixtures.rs` (`cfg(any(test, feature =
      "test-fixtures"))` mount; 3 non-forward / 87 lines and 2 forwards that
      the old index counted as runtime shell);
    - `session/lifecycle/persistence/prepared.rs::prepare_player_save_like_cpp`
      (42; a `#[cfg(test)] mod prepared;`);
    - `session/xp_grants.rs::quest_xp_gain_context_like_cpp` (76; a
      `cfg(any(test, feature = "test-fixtures"))` method);
    - `battle_pet_purchase/ops_1.rs` (2 methods; a `#[cfg(test)] mod`).
  - A source the walk cannot reach fails `check`, as does an allow-list
    entry that matches no non-forward method.
- **F4 (whole-crate ceiling).** The new `crate_ceiling` bounds the
  non-forward methods of the whole crate, so logic cannot leave `session/**`
  for `handlers/` unseen. `check --done` is unchanged (outside = 0).
- **F4 (forward heuristic).** A forward now also has no closure, no
  `async`/`unsafe` block and at most one `?`. Closures are found
  lexically: a `|` after `(`, `,`, `=`, `{`, `:` or `move`. A boolean `||`
  stays an operator. This reclassifies 10 former forwards as non-forward:
  - nine in `session/**`, for example `has_currency` (`is_some_and(|q| …)`),
    `next_represented_spell_cast_guid_like_cpp` (a `.map(|(guid, _)| guid)`
    chain) and `trinity_string_like_cpp`;
  - `session/connection_identity.rs::set_realm_names_like_cpp`, which
    `world-server` calls from `session_resources.rs`, so it joins the
    construction API.
- **Self-test.** It pins every case above:
  - qualified, generic and trait impls;
  - `cfg(test)`, `cfg(any(test, feature = "test-fixtures"))`,
    `cfg(feature = "test-fixtures")`, `cfg(not(test))` and nested gated
    modules;
  - closure chains, `async` blocks, `?` chains, and `||` staying a forward;
  - a temporary crate with `#[path]`, inline nesting, gated and ungated shim
    names, an inner `#![cfg(test)]` and an unmounted file;
  - the crate ceiling, an escape into `handlers/`, and a stale entry.
- **Mutation checks**, each run against the self-test:
  - reverting `IMPL_RE` fails it (`AssertionError` listing only the
    unqualified impls);
  - letting a gated `mod` count as production fails it (the gated and
    fixture modules appear);
  - dropping the closure test fails it;
  - dropping the crate ceiling fails it;
  - dropping the stale-entry check fails it;
  - treating `test` as unknown instead of off fails it.
  - Removing the impl-level gate check did not fail it, because the
    method-level check already covers a gated impl's methods. The redundant
    check was removed.
- **Re-baseline** on `aa034f7e0` (same code, new measure):

| Measure | Old tool | Fixed tool |
|---|---:|---:|
| All non-forward (crate) | 330 / 9,030 | **341 / 9,374** |
| Forwards (crate) | 282 / 1,553 | 275 / 1,535 |
| Staying (`staying_bound`) | 43 / 1,282 | **44 / 1,190** |
| Outside (`outside_ceiling`) | 261 / 6,857 | **269 / 6,972** |
| Crate (`crate_ceiling`, new) | — | **341 / 9,374** |

- **The staying set** loses `map_admission.rs` (176) and the fixture file
  (87), and gains dispatch and admission (4 / 162) and `set_realm_names_like_cpp`
  (9).
  - Its method bound goes from 43 to 44. That is the same set under a stricter
    definition of forward, not new staying code. Its line bound falls from
    1,282 to 1,190.
- **The outside ratchet** gains `map_admission` (1 / 176) and nine closure
  bodies (9 / 57), and loses the two cfg-gated methods above (2 / 118).
- These are measurement changes, recorded here. Every later slice tightens
  from these values.
- **Doc nit.** The 12b1b result now says that 9 of its 10 non-forward methods
  (463 of 483 lines) were in `session/**`. The tenth,
  `validate_inventory_swap_target_like_cpp`, was in
  `handlers/character/items/host.rs`.

### Re-measure and the cut

The fixed index on `cc1920b8c` measured the planned scope:

- `session/spell_effects`: 13 non-forward methods, 1,110 lines.
- The player-item spell paths:
  - `apply_durability_damage_effect_like_cpp` (53);
  - the obtain spells (63);
  - the loaded enchantments with their effects and spell actions (60 + 77 +
    68);
  - the initial item mods and set auras (53 + 44);
  - two 7-line helpers.

Spell sits below quest, social, loot-ops, player-conditions and application
(the closure rules). So each body went where its calls let it go, under one of
the hosts that `WorldSession` already implements:

- **To `wow-world-spell`** (spell may reach inventory, entities and lifecycle):
  - the stuck, self-resurrect and proficiency effects;
  - the durability-damage effect and the item obtain spells;
  - the homebind write, its persistence request builder and the ordered
    persistence hand-off;
  - the farsight seer consumer with its viewpoint create.
- **To `wow-world-quest`:** the quest-complete effect. Its body calls four
  quest functions and nothing above quest. `QuestSessionHostLikeCpp` already
  lends everything else it calls, so it needs no new host.

**Moved: 10 inherent methods and 1 free function, 625 base lines** (611 of them
non-forward methods in `session/**`).

**Deferred to 12b2b** (recorded, not moved; 917 lines):

- **`apply_damage_from_caster_like_cpp` (304).** Its kill cascade reaches
  loot-ops (`ensure_represented_creature_kill_loot_like_cpp`), quest (the
  creature-kill credit), XP (`give_xp`), the tap group and the creature
  movement stop. That would take at least six new host methods. Destination:
  `wow-world-loot-ops`, or application.
- **`represented_nearby_entry_destination_like_cpp` and its implicit-condition
  check (132 + 55).** They read through the World-local
  `RepresentedPlayerConditionContextLikeCpp::as_context(&WorldSession)`
  wrapper, so they need `wow-world-player-conditions` and the wrapper
  retired first.
- **`apply_reputation_effect_like_cpp` (86).** It calls
  `wow_world_social::calculate_reputation_gain_like_cpp` and builds
  `wow_progression::mgr::SetReputationOptionsLikeCpp`. `wow-world-social`
  does not depend on `wow-progression`, and adding that edge would make this
  a manifest change. Destination: `wow-world-social` with that edge, in 12b5.
- **`apply_force_deselect_effect_like_cpp` (38).** Its fixture step records
  into the social state (`RepresentedForceDeselectLikeCpp` is a social type).
  Destination: social, in 12b5.
- **The loaded-enchantment, initial item-mod and item-set-aura family (about
  362 lines, 7 methods).** It applies auras through the application
  `PlayerAuraApplicationCxLikeCpp` (`apply_aura_with_effect_mask_like_cpp`,
  `apply_initial_item_equip_auras_like_cpp`), so spell cannot own it.
  Destination: `wow-world-application`, with an aura-application context host
  method and the World item-enchantment context builder.

### What moved

| Destination file | Functions (base file) | Base lines |
|---|---|---:|
| `wow-world-spell/src/execution_session/player_effects.rs` | `apply_stuck_effect_like_cpp` (93, `spell_effects/effect_apply.rs`), `apply_self_resurrect_effect_like_cpp` (84, `effects_player.rs`), `apply_proficiency_effect_like_cpp` (78, `effects_progress.rs`) | 255 |
| `…/execution_session/item_effects.rs` | `apply_durability_damage_effect_like_cpp` (53, `player_items/durability.rs`), `apply_inventory_item_obtain_spells_with_generator_like_cpp` (63, `player_items/storage.rs`) | 116 |
| `…/execution_session/farsight.rs` | `send_set_viewpoint_target_visibility_like_cpp` (63), `consume_add_farsight_set_seer_outcome_like_cpp` (52) (`spell_effects/effects.rs`) | 115 |
| `…/effect_session/homebind.rs` | `set_represented_homebind_like_cpp` (18, `effects.rs`), `persist_player_homebind_like_cpp` (30, `effects_player.rs`), the free `player_homebind_update_request_like_cpp` (14, `persistence/plans.rs`) | 62 |
| `wow-world-quest/src/quest_session/spell_effect.rs` | `apply_quest_complete_effect_like_cpp` (77, `effects_progress.rs`) | 77 |
| **Total** | 11 functions | **625** |

After rustfmt the bodies take 646 lines, mostly from wrapped signatures and the
fixture steps.

- **Hosts.**
  - **Stuck, self-resurrect, proficiency:** `H: SpellExecutionHostLikeCpp`.
    The trait retires these three forwards. The execution entry
    (`execution.rs`) now calls the free functions instead of `host.<name>()`.
    `spell_state/spellbook.rs` in the World calls
    `wow_world_spell::apply_proficiency_effect_like_cpp(self, …)`.
  - **Durability damage:** `H: SpellEffectHostLikeCpp + VendorSessionHostLikeCpp`.
    These are the two hosts its inventory and execute-log calls already take.
  - **Obtain spells:** `H: SpellExecutionHostLikeCpp`.
  - **Homebind write:** `H: SpellEffectHostLikeCpp`.
  - **Homebind persistence:** `H: SpellEffectHostLikeCpp + BattlePetSessionHostLikeCpp`.
    The battle-pet host lends `lifecycle()` and `split_lifecycle_mut()`.
  - **Viewpoint create:** `H: VisibilitySessionHostLikeCpp` (entities).
  - **Farsight consumer:** `H: SpellExecutionHostLikeCpp + VisibilitySessionHostLikeCpp`.
  - **Quest complete:** `H: QuestSessionHostLikeCpp`.
  - **Trait methods kept, now naming the free function.** The World impls of
    `consume_add_farsight_set_seer_outcome_like_cpp`,
    `apply_durability_damage_effect_like_cpp`,
    `apply_quest_complete_effect_like_cpp`, `persist_player_homebind_like_cpp`
    and `set_represented_homebind_like_cpp` stay. Each needs a bound the
    execution or effect host does not have, or has a caller above spell (the
    application login calls the homebind write).
  - **New host methods on `SpellExecutionHostLikeCpp`:**
    - `player_resurrection_state_snapshot_like_cpp`,
      `set_player_ghost_flag_like_cpp` and `set_player_alive_like_cpp`, the
      same-named World operations the stuck effect calls;
    - two `cfg(any(test, feature = "test-fixtures"))` hooks with no-op
      defaults, `mutate_player_persistent_capability_state_like_cpp` (the
      proficiency fixture branch) and `set_represented_seer_guid_fixture_like_cpp`
      (the farsight fixture step). The World implements both only under
      `cfg(test)`.
    - A hook is needed for the seer fixture because the spell crate's own
      build does not enable the visibility crate's fixture setter.
- **Mechanical rules**, with pairs alongside:
  - `move_token_rules/1363-s12b2-spell-exec.json`:
    - the signatures;
    - `self.config.` → `host.hub_ref().config.`;
    - `self.catalogs.` → `host.hub_ref().catalogs.`;
    - `self.core.client_visible_guids_like_cpp.insert(` → `host.core_mut()…`;
    - `self.core.` → `host.core().`;
    - `crate::session::hub_ref(self)` → `host.hub_ref()`;
    - `crate::session::split_spell_state_mut(self)` → `host.split_spell_state_mut()`;
    - `crate::session::split_visibility_ref(self)` → `(host.visibility(), host.hub_ref())`;
    - `self.visibility.observe_farsight_object_like_cpp(` → `host.visibility_mut()…`;
    - `self.lifecycle.player_lifecycle_port_like_cpp()` → `host.lifecycle()…`;
    - `self.lifecycle.queue_player_homebind_persistence_like_cpp(` → `host.split_lifecycle_mut().0…`;
    - the three inlined wrappers below;
    - `crate::session::dynamic_object_create_data_from_canonical_like_cpp` →
      `wow_world_entities::…`, and `crate::session::player_homebind_update_request_like_cpp`
      → the local name (paths of the same functions);
    - `<fn>(self,` → `<fn>(host,`, and `wow_world_spell::` → `crate::`;
    - `self.` → `host.`;
    - `#[cfg(test)]` → `#[cfg(any(test, feature = "test-fixtures"))]` (and
      `not(test)` likewise);
    - `host.fixtures.` → `fixtures.`.
  - `1363-s12b2-quest.json`: the signature, `self.catalogs.`, `self.core.`,
    `wow_world_quest::<fn>(self,` → `crate::<fn>(host,` and `self.` → `host.`.
  - Rules never touch string literals.
  - **Result: 7 of 11 identical** (`move_token_compare.py --base aa034f7e0`:
    6/10 and 1/1). The other four differ only in the form of their former
    World `#[cfg(test)]` fixture step. In each case the step now runs exactly
    when the World test session runs it, because only that session returns
    its fixtures from `test_fixtures_like_cpp` / `test_fixtures_ref_like_cpp`
    (`hub.rs`, `#[cfg(test)]`). In production builds the step is absent, as
    before.
    - `apply_stuck_effect_like_cpp`: the `{ … }` block becomes
      `if let Some(fixtures) = host.test_fixtures_like_cpp() { … }`, the
      established spell form (`aura_session/mount.rs`).
    - `set_represented_homebind_like_cpp`: `if handle.is_none() { … }` becomes
      `if handle.is_none() && let Some(fixtures) = host.test_fixtures_like_cpp() { … }`.
      This mirrors the sibling `represented_homebind_like_cpp` read moved in 9c.
    - `apply_self_resurrect_effect_like_cpp`:
      `if handle.is_some() || fixtures.alive { return None; }` becomes
      `if handle.is_some() { return None; } let fixtures =
      host.test_fixtures_like_cpp()?; if fixtures.alive { return None; }`.
      The two forms return `None` for the same inputs. Outside the World
      test session the closure returns `None`, which is the base's
      `cfg(not(test))` `outcome = canonical`.
    - `apply_proficiency_effect_like_cpp`: a
      `let world_test = host.test_fixtures_ref_like_cpp().is_some();` under
      the fixture cfg, and `handle.is_none().then(…)` becomes
      `(world_test && handle.is_none()).then(…)` in both branches. The guard
      keeps the `.expect("handle-less fixture capability owner")` out of every
      non-World-test build, where the base had no branch at all.
- **Inlined one-line wrappers** (each verified identical to the World body it
  replaces):
  - `WorldSession::spell_last_cast_time_like_cpp` (`player_cast/state.rs`:
    `let (state, hub) = split_spell_state_ref(self); state.spell_last_cast_time_like_cpp(hub, id)`)
    → `({ let (state, hub) = host.split_spell_state_ref(); state.spell_last_cast_time_like_cpp(hub, HEARTHSTONE_SPELL_ID_LIKE_CPP) })`.
    The World wrapper stays for its other callers.
  - `WorldSession::resolved_inventory_item_like_cpp` (`player_items/storage.rs`,
    the same split over `split_inventory_ref`) → the same block over
    `host.split_inventory_ref()`. The World wrapper stays.
  - `WorldSession::item_template_flags` (`player_items/catalog.rs`:
    `self.catalogs.item_template_flags(id)`) → `host.hub_ref().catalogs.item_template_flags(id)`.
    Its last production caller moved, so the wrapper moved verbatim into the
    test shim; only a scenario calls it.
- **World callers** name the free functions:
  - `session/spell_effects/execution_host.rs`: the farsight consumer, the
    durability effect and quest complete;
  - `session/spell_effects/effect_host.rs`: the homebind write and
    persistence;
  - `session/player_items/storage_move_host.rs`: the obtain spells;
  - `session/spell_state/spellbook.rs`: proficiency, for the login known-spell
    proficiencies.
- **Shims.** `handlers/spell_effect_session_test_shims.rs` (`#![cfg(test)]`,
  mounted from `handlers/mod.rs` outside the hotspot trees, 42 lines) holds
  one-call forwards for the three moved names the scenarios call
  (`set_represented_homebind_like_cpp`,
  `send_set_viewpoint_target_visibility_like_cpp`,
  `consume_add_farsight_set_seer_outcome_like_cpp`) and the moved
  `item_template_flags` read. No scenario call site changes. The existing
  spell shim files are ratcheted, so a new file was needed.
- **Scenario path edits** (same line counts, no assertion touched). Two
  scenarios named a retired session-root re-export through `crate::session::`.
  They now name the function's crate directly, the 12b1b precedent:
  - `scenarios_quest_1.rs` (two calls) → `wow_world_spell::player_homebind_update_request_like_cpp`;
  - `scenarios_visibility_1.rs` (one call) → `wow_world_entities::dynamic_object_create_data_from_canonical_like_cpp`.
- **Files retired:**
  - `session/spell_effects/effect_apply.rs`;
  - `session/spell_effects/effects_player.rs`. Its `#[cfg(test)]` mount of
    `unit_tests/session/spell_effects/effects_player/test_only.rs` (no tests)
    moves to `spell_effects/mod.rs` with the same three lines.
- **Visibility.**
  - `pub` and re-exported from the crate root where the World calls them: the
    farsight consumer, durability, obtain spells, proficiency, the homebind
    write and persistence, and quest complete.
  - The viewpoint create and the request builder are re-exported only under
    `cfg(any(test, feature = "test-fixtures"))`, for the shim and the scenario.
  - Stuck and self-resurrect stay crate-internal.

### Host surface

Moved: **625 function lines** (base).

- **New host: 46 lines, 7.4 %.**
  - The five `SpellExecutionHostLikeCpp` declarations with docs and the two
    defaults (23).
  - The World impls (23; 12 of them `cfg(test)`).
- **Retired host: 12 lines** (the three trait declarations and their World
  impls). Net: 34 lines, 5.4 %.
- No adapter file was merged, so the merged and unmerged figures are the same.
- **Everything counted**, reported for completeness: the destinations grew by
  760 production lines (spell +666, quest +94).
  - 646 are the moved bodies (after rustfmt).
  - The other 114 are:
    - the five new files' headers and imports;
    - the module and re-export lines;
    - the 23-line trait growth;
    - the execution call-site rewrap (+1).

### Await counts (per function, base vs head)

| Function | Base | Head |
|---|---:|---:|
| `apply_stuck_effect_like_cpp` | 1 | 1 |
| `apply_inventory_item_obtain_spells_with_generator_like_cpp` | 1 | 1 |
| `apply_quest_complete_effect_like_cpp` | 1 | 1 |
| every other moved function | 0 | 0 |

- The execution entry awaits `apply_stuck_effect_like_cpp(host)` once, where it
  awaited `host.apply_stuck_effect_like_cpp()` once before.
- The World impls of `apply_quest_complete_effect_like_cpp` and
  `storage_move_apply_obtain_spells_like_cpp` return or await the same single
  future as before.

### Recursion

- **Production grep.** Every moved name was grepped in production (non-shim)
  `wow-world`. Each hit is a path call (`wow_world_spell::<name>(self, …)` or
  `wow_world_quest::<name>(self, …)`) or a trait-impl declaration. There is no
  `WorldSession::<moved>` or `self.<moved>(` left.
  - The trap was the five kept trait impls. Their bodies said
    `WorldSession::<name>(self, …)`, which without the inherent method would
    resolve to the trait method itself. All five now name the free function.
- **New host methods.** The World impls of
  `player_resurrection_state_snapshot_like_cpp`,
  `set_player_ghost_flag_like_cpp`, `set_player_alive_like_cpp` and
  `mutate_player_persistent_capability_state_like_cpp` name existing inherent
  methods (`combat/death.rs`, `player_vitals_adapter.rs`,
  `persistence/plans.rs`), which UFCS resolves before any trait method. The
  seer hook writes the field directly.
- **Free functions.** No moved body calls a host method of its own name. The
  consumer calls the viewpoint create as a free function. The homebind,
  durability and quest-complete bodies call only other names.
- `unconditional_recursion`: 0 in all three canonical builds.

### Registration

No registrar moved. Spell execution, effect dispatch, opcode registrations,
their order, `SessionStatus`, `PacketProcessing`, handler names and thunks are
unchanged. No checker constant or `handler-module-policy.json` change.

### Edges and closure

- No new crate edge:
  - spell already depends on inventory and entities;
  - quest already depends on spell, conditions and `wow-progression`;
  - the World already depends on both.
- No manifest or `Cargo.lock` change.
- Closure at head: PASS (lock mode, 18 packages).

### Accounting

| Crate `src` | Base physical | Head physical | Base production | Head production |
|---|---:|---:|---:|---:|
| `wow-world` | 38,010 (454 files) | 37,414 (453) | **27,884** | **27,244** |
| `wow-world-spell` | 20,343 (94) | 21,010 (97) | 20,026 | 20,692 |
| `wow-world-quest` | 19,071 (62) | 19,165 (63) | 18,487 | 18,581 |

- **`wow-world` production −640:**
  - **Moved:** −625 function lines.
  - **Reclassified production → test:** −3 (`item_template_flags`, now in
    the shim).
  - **Retired:** −30:
    - the two emptied files' headers, imports and impl shells (−10, −9);
    - three root re-exports and the spell-effects `represented_homebind_like_cpp`
      import (−4);
    - the session-root imports whose last production reader moved
      (`ItemClass`, `ItemFlags`, `ItemSpelltriggerType`, `SetProficiency`; −3
      after rustfmt; `PowerType` joins an existing `cfg(test)` import);
    - the two retired `mod` lines, whose test mount now counts as test (−2);
    - separator lines (−2).
  - **Doc headers:** +4 (`effects.rs`, `effects_progress.rs` record the moves).
  - **Host adapter:** +14 (+23 new impl lines, −9 retired).
  - −625 − 3 − 30 + 4 + 14 = −640.
- **Destination production:** spell +666, quest +94.
- **`net_move.py check --base origin/3.4.3`:** S = 601, G = 750 within the
  1,592.1 allowance, 0 duplicate bodies, PASS.
- **`session_method_index`** (fixed tool):

| Measure | Base `aa034f7e0` | After the fixes `cc1920b8c` | Head |
|---|---:|---:|---:|
| Non-forward (crate) | 341 / 9,374 | 341 / 9,374 | **331 / 8,763** |
| Forwards (crate) | 275 / 1,535 | 275 / 1,535 | 274 / 1,532 |
| Staying | 44 / 1,190 | 44 / 1,190 | 44 / 1,190 |
| Outside | 269 / 6,972 | 269 / 6,972 | **259 / 6,361** |

  The base column is the fixed tool run on the base code. `session/spell_effects`
  goes from 13 / 1,110 to 5 / 615: the damage kill cascade, the nearby pair,
  reputation and force deselect remain.

### Policy and ceilings

- **Ceilings tightened, none raised:**
  - **`session-method-index.json`:** `outside_ceiling` 269 / 6,972 → 259 /
    6,361 and `crate_ceiling` 341 / 9,374 → 331 / 8,763. `staying_bound`
    stays at 44 / 1,190.
  - **Hotspot `session/mod.rs`:** production 18,618 → 17,978 and total
    160,239 → 159,599; test unchanged at 141,621.
  - **Physical `session/mod.rs`:** 813 → 808.
- The re-baseline in the follow-up commit is a measurement change, recorded
  above, not a raise: the same code measures 43 → 44 staying methods only
  because the stricter forward rule reclassifies one setter.
- `handlers/spell_effect_session_test_shims.rs` (42 lines, `#![cfg(test)]`)
  needs no physical row.
- `session-ownership-policy.json` is reprinted with `print-baseline` (the
  moved methods, the shim surface, the trait changes); `check --syntax-only`
  PASS. `dependency-policy.json` is unchanged.

### Warnings and tests

Canonical recipe (with `--locked`), base `aa034f7e0` vs head, counted and
compared with `warning_units.py`:

| Build | `wow-world` lib | `wow-world` lib test | All `wow-world*` lib units |
|---|---:|---:|---:|
| production lib | 118 → 118 | — | 245 → 245 |
| lib + `test-fixtures` | 127 → 127 | — | 209 → 209 |
| `--lib --tests` | 127 → 127 | 178 → 178 | 209 → 209 |

- **`compare`:** nothing appears only in the base or only in the head, in
  any build. Before the import cleanup above, the head had `PowerType`,
  `SetProficiency`, `ItemClass`, `ItemFlags` and `ItemSpelltriggerType` as new
  unused session-root imports. The `item_template_flags` wrapper was briefly
  dead too, before it moved to the shim.
- `wow-world-spell` (8 / 5 / 5) and `wow-world-quest` (27 / 17 / 17) keep
  exactly their pre-existing warnings.
- **`wow-world`:** `dead_code` 0 in all three builds. `unconditional_recursion`
  is 0 in every unit of all three builds.
- **Tests.** The test-name multiset is identical: 9,492
  `#[test]`/`#[tokio::test]` functions under `crates/`, 5 ignored, at base and
  at head. No assertion changed. The only test-tree edits are the two
  scenario path rewrites above, at the same line counts.
- **Focused runs during the move:**
  - `cargo test -p wow-world --lib`: 3,680 passed, 1 ignored;
  - `cargo test -p wow-world-spell -p wow-world-quest --lib`: 2 and 25
    passed. These are the crates' own builds, which caught the seer-fixture
    setter that the spell crate's features do not enable.

### Follow-ups

- **12b2b:** the damage kill cascade (loot-ops or application), the
  nearby-entry pair (player-conditions, after the World condition-context
  wrapper retires), the loaded-enchantment / item-mod / set-aura family
  (application, over the aura-application context).
- **12b5:** the reputation effect (with a `wow-world-social` →
  `wow-progression` edge) and force deselect.
- **`map_admission.rs`:** `access_requirement_abort_like_cpp` →
  `wow-world-instances` (outside ratchet since this slice).
- **12d:** the execution host now carries three state hooks and two fixture
  hooks for moved bodies. Fold them into a narrower player-state host when the
  adapters regroup.
- **Shims.** The `#![cfg(test)]` shims retire in the final cleanup.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section. The manifest path,
dirty flag and duration are in the PR body. This section is not edited after
that run, so the PR head is the validated SHA.

## Slice 12b2b result (kill cascade, nearby-entry pair, enchantment / item-mod / set-aura family)

Branch `1363-s12b2b`, base `ca48c58e9` (slice 12b2 merged).

### #1414 follow-ups 1–3: the index tool

Fixed in a separate commit, before anything moved. None of these cases occurs
in the tree today; the measurement on the base is unchanged (staying 44 /
1,190, outside 259 / 6,361, crate 331 / 8,763).

- **Follow-up 1 (blind spots), each fixed or rejected loudly:**
  - **nested-`<>` generics:** `IMPL_RE` accepts up to four levels of `<>` in
    the impl and type generics (`impl<T: Into<Vec<Option<u8>>>> WorldSession`);
  - **`::wow_world::` paths:** a leading `::` is accepted
    (`impl ::wow_world::session::WorldSession`);
  - **type aliases: rejected.** A production `type X = WorldSession;` or
    `use … WorldSession as X;` fails `check` (and makes `index` exit 1),
    because the alias's impls would be invisible. `<WorldSession as Trait>::`
    and cfg-gated aliases are not flagged;
  - **same-line attributes before `fn`** (`#[inline] fn`, `#[cfg(test)] fn`)
    are matched, and a gated one stays excluded;
  - **`extern "C" fn`** is matched;
  - **`&|x|` / `&mut ||` closures** count as closures (non-forward);
  - **`#[path]` mounts outside `src`:** a production one now fails `check`
    (`production #[path] mount outside src`); a gated test mount such as
    `unit_tests/` stays allowed.
- **Follow-up 2 (self-test gaps), all added:** `not(not(test))` (off),
  `any(not(test), test)` (on), `not(any(not(test), test))` (off, which tells
  Kleene `True` from unknown), the nested-fn guard (a `fn helper` inside a
  method body is not indexed), and `check` failing on an unreached source
  (the self-test now runs the same `check_errors` path as `check` on a
  temporary crate).
- **Follow-up 3 (shim physical-row convention): add rows.** Every
  `crates/wow-world/src/handlers/*_test_shims.rs` carries a ratcheting
  `physical-file-policy.json` row at its live value from creation, whatever
  its size. 29 of 31 already did; the two missing rows
  (`player_item_builders_test_shims.rs` 233, `spell_effect_session_test_shims.rs`
  42) are added, and `session_method_index.py check` now fails on a shim
  without a row. Shims in subdirectories (`handlers/<dir>/test_shims.rs`,
  `unit_tests/**/f3_shims.rs`) predate #1363's top-level shim files and are
  outside the convention.
- **Mutation checks**, each run against the self-test (12/12 caught):
  four-level generics cut to one; the leading `::` dropped; the alias
  rejection dropped; the same-line attribute group dropped; `extern` dropped;
  `&` dropped from the closure leads; the outside-`src` error dropped; `not`
  collapsing `not(True)` to unknown; `any` losing its `True` short-circuit;
  the nested-fn depth guard dropped; the unreached-source error dropped from
  `check`; the shim-row check dropped.
- Follow-up 4 (moving the packet-spoof counter into the core admission state)
  is not in this slice; it stays recorded.

### Re-measure and the cut

The fixed index on `216983093` (same code as the base `ca48c58e9`) measured the
three deferred groups:

- `apply_damage_from_caster_like_cpp`: 1 non-forward / 304.
- The nearby-entry pair: `represented_nearby_entry_destination_like_cpp`
  (132) and `represented_nearby_candidate_meets_implicit_conditions_like_cpp`
  (55).
- The loaded item-mod family: `apply_initial_loaded_item_mods_like_cpp` (53),
  `apply_initial_item_set_auras_like_cpp` (44),
  `apply_loaded_equipped_item_enchantments_like_cpp` (77),
  `apply_loaded_equipped_item_enchantment_effects_like_cpp` (68),
  `apply_loaded_enchantment_spell_action_like_cpp` (60), and three 7-line
  split helpers (`initial_loaded_item_mods_can_apply_like_cpp`,
  `loaded_inventory_item_visible_update_like_cpp`,
  `apply_represented_item_bonus_action_state_like_cpp`): 323. The 12b2
  estimate of ~362 counted the one-line wrappers below, which are inlined,
  not moved.

All three fit one PR (814 lines, far under 4.5k). Destinations, by layering:

- **Kill cascade → `wow-world-application`** (`creature_damage.rs`). It is
  orchestration above loot-ops (`ensure_represented_creature_kill_loot_like_cpp`),
  quest (the creature-kill credit through `on_creature_killed_with_generator_like_cpp`)
  and the XP grant; application already depends on spell, loot-ops and
  entities. Order unchanged: damage and tap → combat log → threat sync →
  pull attack start → movement stop → kill loot → XP → reputation → kill
  credit → kill hooks (World test only) → death state → values update.
- **Nearby-entry pair → `wow-world-spell`** (`execution_session/nearby_entry.rs`).
  The World-local `RepresentedPlayerConditionContextLikeCpp::as_context(&WorldSession)`
  wrapper need not retire first: the inventory `VendorSessionHostLikeCpp`
  already carries the projection as `type PlayerConditionContext:
  PlayerConditionContextSourceLikeCpp<Self>`, and the World implements its
  `as_context` with that same wrapper. Spell already depends on inventory
  (the 12b2 durability precedent, `H: SpellEffectHostLikeCpp +
  VendorSessionHostLikeCpp`), so player-conditions is not needed.
- **Loaded item-mod family → `wow-world-application`**
  (`player_items_session/loaded_item_mods.rs`). Auras are applied and
  removed through the application aura-application context, built from the
  World session's fields as the World builder builds it. Order unchanged:
  per equipped slot (sorted by slot, then GUID counter) item bonuses → set
  auras → equip auras → enchantments; per enchantment slot plan → duration →
  effects (record, spell action, unrepresented, state) → visible item.

**Moved: 11 functions, 814 base lines**, all non-forward methods in
`session/**`. Nothing is deferred from the 12b2b scope.

### What moved

| Destination file | Functions (base file) | Base lines |
|---|---|---:|
| `wow-world-application/src/creature_damage.rs` | `apply_damage_from_caster_like_cpp` (304, `spell_effects/effect_combat/damage_and_combat_application.rs`) | 304 |
| `wow-world-application/src/player_items_session/loaded_item_mods.rs` | `apply_initial_loaded_item_mods_like_cpp` (53), `apply_initial_item_set_auras_like_cpp` (44), `initial_loaded_item_mods_can_apply_like_cpp` (7), `apply_represented_item_bonus_action_state_like_cpp` (7) (`player_items/modifiers.rs`); `apply_loaded_equipped_item_enchantments_like_cpp` (77), `apply_loaded_equipped_item_enchantment_effects_like_cpp` (68), `apply_loaded_enchantment_spell_action_like_cpp` (60) (`enchantment.rs`); `loaded_inventory_item_visible_update_like_cpp` (7, `persistence_load.rs`) | 323 |
| `wow-world-spell/src/execution_session/nearby_entry.rs` | `represented_nearby_entry_destination_like_cpp` (132, `spell_effects/destinations.rs`), `represented_nearby_candidate_meets_implicit_conditions_like_cpp` (55, `effects.rs`) | 187 |
| **Total** | 11 functions | **814** |

After rustfmt the bodies take 850 lines (319 + 337 + 194).

- **Hosts.**
  - **Kill cascade:** `H: CreatureDamageHostLikeCpp`, a new application trait
    over `SpellEffectHostLikeCpp + LootSessionHostLikeCpp` (the latter brings
    `WorldEntitiesHostLikeCpp`), with one method,
    `current_group_member_guids_for_tap_like_cpp` (the World `social/group.rs`
    operation). `on_creature_killed_with_generator_like_cpp`,
    `split_world_entities_ref` and
    `represented_unit_values_update_to_update_object_like_cpp` are declared by
    two of the bounded traits, so the body names them by trait (UFCS); every
    World impl of them names the same inherent operation or hub split.
  - **Loaded item mods:** `H: LoadedItemModsHostLikeCpp`, a new application
    trait over `TrainerSessionHostLikeCpp` (the field split),
    `VendorSessionHostLikeCpp` (`split_inventory_mut`),
    `BattlePetSessionHostLikeCpp` (`split_inventory_ref`) and
    `QuestItemStorageHostLikeCpp` (`resolved_inventory_item_object_like_cpp`),
    with one method, `apply_current_player_item_enchantment_plan_like_cpp`
    (the World item-enchantment context builder stays: it has another
    caller).
  - **Nearby pair:** `H: SpellExecutionHostLikeCpp + VendorSessionHostLikeCpp`.
    The context parameter's type becomes `Option<&H::PlayerConditionContext>`
    (the World's associated type is `RepresentedPlayerConditionContextLikeCpp`).
  - **Trait methods kept, now naming the free function.** The World impls of
    `SpellEffectHostLikeCpp::apply_damage_from_caster_like_cpp`,
    `SpellExecutionHostLikeCpp::represented_nearby_entry_destination_like_cpp`
    and `ContinueLoginHostLikeCpp::apply_initial_loaded_item_mods_like_cpp`
    stay: the spell execution/effect entries and the application login call
    them through their hosts. `login.rs` is unchanged, so the login-order
    source test still finds `apply_initial_loaded_item_mods_like_cpp` after the
    character-aura load.
- **Mechanical rules** (`move_token_rules/1363-s12b2b-{kill,item-mods,nearby}.json`
  with pairs alongside): the signatures; `self.core.` → `host.core().`
  (`host.core_mut()` for `mutate_world_creature`, which takes `&mut`);
  `self.catalogs.` → `host.hub_ref().catalogs.`; `crate::session::hub_ref(self)`
  → `host.hub_ref()`; `crate::session::split_inventory_{mut,ref}(self)` →
  `host.split_inventory_{mut,ref}()`; `crate::session::split_world_entities_ref(self)`
  and the two ambiguous host calls → UFCS; `crate::creature_movement::` →
  `wow_world_core::creature_movement::` (the World path is a re-export of
  it); `<crate fn>(self,` → `(host,`; calls of moved names → the local free
  function; `context.as_context(self)` → `(host)`; the inlined wrappers
  below; `self.` → `host.`. Rules never touch string literals.
  - **Result** (`move_token_compare.py --base ca48c58e9`): 8 of 11 identical
    (nearby 1/2, item mods 7/8, kill 0/1). The three others differ only by:
    - rustfmt brace changes around a one-expression closure (the
      implicit-condition area check loses `{ }`, the effect-refs lookup gains
      them);
    - the former World `#[cfg(test)]` fixture step, now
      `#[cfg(any(test, feature = "test-fixtures"))]` guarded so it runs
      exactly in the World test session, as in 12b2:
      - `apply_loaded_equipped_item_enchantment_effects_like_cpp`:
        `self.inventory.record_represented_item_bonus_actions_for_test_like_cpp(…)`
        → `if TrainerSessionHostLikeCpp::world_test_consumer_like_cpp(host) {
        host.split_inventory_mut().0.record_…(…) }`;
      - `apply_damage_from_caster_like_cpp`: the kill-hooks record →
        `if host.test_fixtures_ref_like_cpp().is_some() { … }`.
      In production builds both steps are absent, as before.
- **Inlined one-line wrappers** (each verified identical to the World body
  it replaces):
  - `WorldSession::spell_store` (`self.catalogs.spell_store()`) →
    `host.hub_ref().catalogs.spell_store()`; stays for other callers.
  - `WorldSession::item_storage_template` (`self.catalogs.item_storage_template(id)`)
    → the same over `host.hub_ref().catalogs`; stays.
  - `WorldSession::apply_enchantment_effect_refs` and
    `apply_enchantment_random_suffix_ref` (`self.catalogs.<same>(…)`) → the
    same over `host.hub_ref().catalogs`. Their last production caller moved,
    so both moved verbatim into the test shim (two scenarios call them).
  - `WorldSession::send_tx` (`self.core.send_tx()`) → `host.core().send_tx()`;
    stays.
  - `WorldSession::apply_aura_with_effect_mask_like_cpp`,
    `remove_aura` and `apply_initial_item_equip_auras_like_cpp` (each
    `self.player_aura_application_cx_like_cpp().<op>(…)`, `remove_aura` naming
    `remove_aura_like_cpp`) → `session_aura_application_cx_like_cpp(host).<op>(…)`.
    The first two stay; the equip-aura wrapper lost its last production
    caller and moved verbatim into the test shim.
  - `WorldSession::apply_aura_with_effect_mask_without_update_like_cpp` →
    two levels (it forwards to the private
    `apply_aura_with_effect_mask_provenance_and_update_like_cpp` with
    `AuraCastProvenanceLikeCpp::default()` and `false`, which builds the
    context): `session_aura_application_cx_like_cpp(host).apply_aura_with_effect_mask_provenance_and_update_like_cpp(…, AuraCastProvenanceLikeCpp::default(), false)`.
    It moved into the test shim with the same one-level inline (a scenario
    calls it).
  - The context builder `WorldSession::player_aura_application_cx_like_cpp`
    itself is reproduced as the private
    `session_aura_application_cx_like_cpp<H: TrainerSessionHostLikeCpp>`: it
    takes the hub, spell state, inventory, loot and (under the fixture cfg)
    quest state from `split_trainer_mut`, the same five field borrows
    `split_aura_application_mut` returns, and passes
    `world_test_consumer_like_cpp()` where the World passes `cfg!(test)` (true
    exactly for the World test session). The World builder stays for its
    other callers.
- **World callers** name the free functions:
  `session/spell_effects/effect_host.rs` (the damage entry),
  `session/spell_effects/execution_host.rs` (the nearby entry) and
  `handlers/character/continue_login_host.rs` (the item-mod replay).
- **Shims.** `handlers/item_mods_session_test_shims.rs` (`#![cfg(test)]`,
  mounted from `handlers/mod.rs` outside the hotspot trees, 106 lines) holds
  one-call forwards for the five moved names the scenarios call and the four
  wrappers whose last production caller moved. No scenario call site
  changes; no test file changed.
- **Files retired:** `session/spell_effects/destinations.rs`,
  `session/spell_effects/effect_combat.rs` and
  `effect_combat/damage_and_combat_application.rs`.
- **Visibility.** `pub` and re-exported from the crate root where the World
  calls them: the two hosts, `apply_damage_from_caster_like_cpp`,
  `apply_initial_loaded_item_mods_like_cpp`,
  `apply_loaded_equipped_item_enchantments_like_cpp` and
  `apply_represented_item_bonus_action_state_like_cpp` (the last two for the
  shim), and `represented_nearby_entry_destination_like_cpp`. The other five
  are private to their file.

### Host surface

Moved: **814 function lines** (base).

- **New host: 44 lines, 5.4 %.** The two trait declarations with docs (8 +
  16) and their World impls (8 + 12).
- **With the reproduced aura-context builder (24): 68 lines, 8.4 %.** It is
  adapter code in the destination, counted here for completeness.
- No adapter file was merged; merged and unmerged figures are the same.
- No host method was retired.

### Await counts (per function, base vs head)

| Function | Base | Head |
|---|---:|---:|
| `apply_damage_from_caster_like_cpp` | 3 | 3 |
| every other moved function | 0 | 0 |

The three awaits are, in order, the kill loot, `give_xp` and the kill credit.
The World `SpellEffectHostLikeCpp` impl returns the free function's single
future, as it returned the inherent method's before.

### Recursion

- **Production grep.** Every moved name, every inlined wrapper and both new
  host methods were grepped in production (non-shim) `wow-world`. Each hit is
  a path call (`wow_world_application::<name>(self, …)`,
  `wow_world_spell::<name>(self, …)`, the imported
  `apply_initial_loaded_item_mods_like_cpp(self, …)`) or a trait-impl
  declaration; no `WorldSession::<moved>` or `self.<moved>(` is left.
- The three kept trait impls (damage, nearby, initial item mods) used to say
  `WorldSession::<name>(self, …)`; all three now name the free function.
- **New host methods.** `current_group_member_guids_for_tap_like_cpp` and
  `apply_current_player_item_enchantment_plan_like_cpp` name existing inherent
  methods by `WorldSession::` path, which resolve before any trait method.
- **Free functions.** No moved body calls a host method of its own name.
- `unconditional_recursion`: 0 in every unit of all three canonical builds.

### Registration

No registrar moved. Spell execution, effect dispatch, login, opcode
registrations, their order, `SessionStatus`, `PacketProcessing`, handler names
and thunks are unchanged. No checker constant or `handler-module-policy.json`
change.

### Edges and closure

- No new crate edge: application already depends on spell, loot-ops,
  entities, inventory, quest and core; spell already depends on inventory.
- No manifest or `Cargo.lock` change.
- Closure at head: PASS (lock mode, 18 packages).

### Accounting

| Crate `src` | Base physical | Head physical | Base production | Head production |
|---|---:|---:|---:|---:|
| `wow-world` | 37,414 (453 files) | 36,648 (451) | **27,244** | **26,375** |
| `wow-world-application` | 16,075 (63) | 16,847 (65) | 15,358 | 16,130 |
| `wow-world-spell` | 21,010 (97) | 21,227 (98) | 20,692 | 20,909 |

- **`wow-world` production −869:**
  - **Moved:** −814 function lines.
  - **Reclassified production → test:** −38 (the four wrappers now in the
    shim: equip auras 7, without-update aura 18, the two catalog reads 6 + 7).
  - **Retired:** −28: the three retired files' headers, imports and impl
    shells (`destinations.rs` −10, `effect_combat.rs` −12, the damage file's
    −4 beyond its body) and the two `mod` lines in `spell_effects/mod.rs`
    (−2).
  - **Imports:** −11: session-root (`mod.rs` −5) and `item_modifiers.rs` (−6)
    re-exports and six `wow_entities` names whose last production reader
    moved, dropped or gated into existing `cfg(test)` groups.
  - **Doc headers:** +1 (`effects.rs` records the move).
  - **Host adapter:** +22 (the two World impls, 8 + 12, each with its
    separating blank line); the repointed calls are line-neutral.
  - **Separator:** −1 (a blank line in `modifiers.rs`).
  - −814 − 38 − 28 − 11 + 1 + 22 − 1 = −869.
- **Destinations:** application +772 (656 of it the two moved bodies after
  rustfmt, 319 + 337; the rest the trait declarations, the aura-context
  builder, headers, imports and the module/re-export lines); spell +217 (194
  of it the moved pair; the rest its header, imports, module/re-export lines
  and the one-line doc update on the damage trait method).
- **`net_move.py check --base origin/3.4.3`:** growth 989 within the 1,748.5
  allowance, 0 duplicate bodies, PASS.
- **`session_method_index`:**

| Measure | Base `ca48c58e9` (= after the fixes `216983093`) | Head |
|---|---:|---:|
| Non-forward (crate) | 331 / 8,763 | **320 / 7,949** |
| Forwards (crate) | 274 / 1,532 | 270 / 1,494 |
| Staying | 44 / 1,190 | 44 / 1,190 |
| Outside | 259 / 6,361 | **248 / 5,547** |

  −11 / −814 non-forward are exactly the moved functions; −4 / −38 forwards
  are the four wrappers now in the shim. `session/spell_effects` goes from 5 /
  615 to 2 / 124 (reputation and force deselect remain, for 12b5).

### Policy and ceilings

- **Ceilings tightened, none raised:**
  - **`session-method-index.json`:** `outside_ceiling` 259 / 6,361 → 248 /
    5,547 and `crate_ceiling` 331 / 8,763 → 320 / 7,949; `staying_bound`
    stays at 44 / 1,190.
  - **Hotspot `session/mod.rs`:** production 17,978 → 17,109, test 141,621
    → 141,616, total 159,599 → 158,725. The names that became test-only join
    existing `cfg(test)` groups, and five single-name `cfg(test)`
    `wow_entities` imports merge into one group, so the test figure does not
    grow.
  - **Physical `session/mod.rs`:** 808 → 798.
- **New physical row:** `handlers/item_mods_session_test_shims.rs` at 106,
  under the shim convention above.
- `session-ownership-policy.json` is reprinted with `print-baseline` (the 15
  retired impl items, the 9 shim items, the two new trait-impl items);
  `check --syntax-only` PASS. `dependency-policy.json` is unchanged.

### Warnings and tests

Canonical recipe (with `--locked`), base `ca48c58e9` vs head, counted and
compared with `warning_units.py`:

| Build | `wow-world` lib | `wow-world` lib test | All `wow-world*` lib units |
|---|---:|---:|---:|
| production lib | 118 → 118 | — | 245 → 245 |
| lib + `test-fixtures` | 127 → 127 | — | 209 → 209 |
| `--lib --tests` | 127 → 127 | 178 → 178 | 209 → 209 |

- **`compare`:** nothing appears only in the base or only in the head, in
  any build. Before the import cleanup, the head had the item-mod outcome
  and item-bonus re-exports and six `wow_entities` names as new unused
  imports.
- **`wow-world`:** `dead_code` 0 in all three builds. `unconditional_recursion`
  is 0 in every unit of all three builds.
- `cargo check --locked --tests -p wow-world-application -p wow-world-spell`
  (the crates' own builds): no warning in the moved files.
- **Tests.** No test file changed, so the test-name multiset is identical:
  `git grep` of `#[test]`/`#[tokio::test]` attributes under `crates/` counts
  9,493 at base and at head (this grep also counts the attribute forms the
  12b2 count of 9,492 functions excluded), and `#[ignore` 5 at both. No
  assertion changed.
- **Focused run during the move:** `cargo test -p wow-world --lib`: 3,680
  passed, 1 ignored (as at 12b2).

### Follow-ups

- **12b5:** the reputation effect (with a `wow-world-social` →
  `wow-progression` edge) and force deselect, the last two non-forward
  methods in `session/spell_effects`.
- **#1414 follow-up 4:** the packet-spoof counter and policy
  (`session/admission.rs`) into the core admission state.
- **12d:** `CreatureDamageHostLikeCpp` and `LoadedItemModsHostLikeCpp` each
  carry one method; fold them when the adapters regroup. The reproduced
  aura-context builder can replace the World one once its remaining World
  callers move.
- **Shims.** The `#![cfg(test)]` shims retire in the final cleanup.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section. The manifest path,
dirty flag and duration are in the PR body. This section is not edited after
that run, so the PR head is the validated SHA.

## Slice 12b3 result (spell-state queries and operations → spell, the reputation and force-deselect effects → social)

Branch `1363-s12b3-spell-state`, base `85f5703dd` (slice 12b2b merged).

### #1415 follow-ups: the index tool

Fixed in a separate commit, before anything moved; the measurement on the
base is unchanged (staying 44 / 1,190, outside 248 / 5,547, crate 320 /
7,949).

- **Raw-identifier aliases (Codex P2).** `ALIAS_RE` accepts an optional `r#`
  on the alias name and on `WorldSession` in both forms (`type r#S =
  r#WorldSession;`, `use … WorldSession as r#S;`), and `IMPL_RE` matches
  `impl r#WorldSession`. The self-test adds both alias lines (rejected at
  `aliases.rs:3` and `:4`) and a raw `impl r#WorldSession` file that is
  indexed (`raw_impl`).
- **Mutation checks (review optional follow-up),** each run against the
  self-test, 5 of 5 caught: `r#` dropped from `IMPL_RE`; from the `type`
  alias form; from the `use … as` form; the alias rejection loop dropped; the
  `#[path]`-outside-`src` error dropped.

### Re-measure and the cut

The index on the base lists `session/spell_state` at **35 non-forward / 620
lines** (29 forwards / 174). Six of them are the world-server construction API
already on the allow-list (`set_spell_store`, `set_spell_chain_store`,
`set_spell_linked_store`, `set_spell_area_store`,
`set_spell_acquisition_static_authority_like_cpp`,
`set_spell_runtime_script_authority_like_cpp`; 64 lines), leaving **29 / 556**.
Each was read against its callers and what it reaches:

- **Moved to `wow-world-spell` (16 / 198):**
  - twelve field-split queries over `SpellbookHostLikeCpp` (71):
    `resolved_has_represented_aura_effect_like_cpp`,
    `resolved_total_represented_aura_modifier_by_misc_value_like_cpp`,
    `resolved_total_represented_aura_multiplier_like_cpp`,
    `set_player_aura_authority_complete_like_cpp`,
    `represented_active_glyph_aura_source_is_empty_like_cpp`,
    `represented_add_player_to_map_aura_source_is_empty_like_cpp`,
    `represented_cast_speed_multiplier_like_cpp`,
    `represented_player_is_polymorphed_like_cpp`,
    `represented_update_zone_script_aura_source_is_hit_inert_like_cpp`,
    `represented_update_area_pvp_rule_aura_source_is_empty_like_cpp`,
    `represented_login_passive_spell_cast_gate_like_cpp`,
    `player_target_spell_is_hit_inert_like_cpp`;
  - `has_represented_self_res_spell_like_cpp` (4),
    `interrupt_non_melee_spells_for_far_teleport_like_cpp` (15),
    `apply_login_known_spell_proficiencies_like_cpp` (46) and
    `publish_spell_pull_attack_start_like_cpp` (62).
- **Reclassified to a test shim (2 / 51):**
  `represented_player_spell_rows_like_cpp` (38) and
  `represented_mount_source_spell_usable_like_cpp` (13) were
  `#[allow(dead_code)]` with no production caller; they move verbatim (minus
  the attribute) into the `#![cfg(test)]` shim the scenarios already reach.
- **Stay, with the reason (11 / 307):**
  - `with_player_spell_runtime_like_cpp` (7),
    `mutate_player_spell_runtime_like_cpp` (17, the #754 single-writer
    dispatch) and `player_spell_runtime_snapshot_like_cpp` (16): the host's
    own spell-map seam; each passes the World `cfg!(test)` flag or a
    `#[cfg(test)]` fixture fallback, and the spellbook and aura hosts already
    call them as host methods. Moving them changes the #754 owner and is not
    a move-only step.
  - `can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp`
    (85 at base), its `player_spell_hit_source_identity_complete_like_cpp`
    (9, a `cfg!(test)` bootstrap term) and
    `sync_player_spell_hit_aura_authority_to_canonical_like_cpp` (16): the
    proof reads the quest login aura sources (`wow_world_quest`, which depends
    on spell), the inventory and buyback item sets and the guild id, so it
    sits above spell (application). It now names the moved queries by path.
  - `player_aura_application_cx_like_cpp` (17),
    `apply_login_passive_known_spell_auras_like_cpp` (58) and
    `apply_loaded_known_spell_previous_rank_passive_auras_like_cpp` (54):
    they build or use the `wow_world_application` aura-application context.
  - `apply_represented_login_spell_reset_if_needed_like_cpp` (20, calls
    `wow_world_quest`) and its only callee
    `reset_spells_notification_text_like_cpp` (8).
- **12b5 pair → `wow-world-social` (2 / 124):**
  `apply_reputation_effect_like_cpp` (86) and
  `apply_force_deselect_effect_like_cpp` (38), the last two non-forward
  methods in `session/spell_effects`. The reputation effect needs the
  `wow-world-social` → `wow-progression` edge (`SetReputationOptionsLikeCpp`),
  simulated first: `wow_world_closure.py check --add
  "wow-world-social->wow-progression"` PASS.

**Moved: 18 functions, 322 base lines; reclassified 2 / 51.** Total 373 of
the 620 + 124 measured lines leave `session/**`. (Reconciliation, #1416 follow-up 2: the
index's outside delta is −363, not −373, because three staying bodies grow by
10 lines when they name the moved queries by path; see the index table below.)

### What moved

| Destination file | Functions (base file) | Base lines |
|---|---|---:|
| `wow-world-spell/src/spell_state_session/queries.rs` | the twelve queries above (`spell_state/{aura/effect_queries,aura/spell_hit_authority,aura,aura_publication,cast,spell}.rs`) | 71 |
| `wow-world-spell/src/spell_state_session/operations.rs` | `has_represented_self_res_spell_like_cpp` (4), `interrupt_non_melee_spells_for_far_teleport_like_cpp` (15) (`spell.rs`); `apply_login_known_spell_proficiencies_like_cpp` (46, `spellbook.rs`) | 65 |
| `wow-world-spell/src/spell_state_session/pull_attack_start.rs` | `publish_spell_pull_attack_start_like_cpp` (62, `spell_publication.rs`) | 62 |
| `wow-world-social/src/spell_effects.rs` | `apply_reputation_effect_like_cpp` (86, `spell_effects/effects_progress.rs`), `apply_force_deselect_effect_like_cpp` (38, `spell_effects/effects.rs`) | 124 |
| **Total** | 18 functions | **322** |

Lines per destination: spell 198, social 124.

- **Hosts.**
  - **Queries:** `H: SpellbookHostLikeCpp` (its `split_spell_state_ref/_mut`
    are the World `crate::session::split_spell_state_ref/_mut`).
  - **Self-res:** `H: SpellExecutionHostLikeCpp`
    (`player_resurrection_state_snapshot_like_cpp`).
  - **Interrupt and proficiencies:** `H: SpellStateHostLikeCpp`, a new spell
    trait over `SpellExecutionHostLikeCpp` with two methods,
    `interrupt_player_cast_like_cpp` (the World cast interrupt runs the
    application request lifecycle, above spell) and
    `player_persistent_capability_state_snapshot_like_cpp` (it carries a
    `#[cfg(test)]` fixture fallback). The World impl names both inherent
    methods by `WorldSession::` path.
  - **Pull attack start:** `H: wow_world_entities::CombatSessionHostLikeCpp`
    (it runs `handle_creature_attack_start_like_cpp_command_like_cpp`).
  - **Reputation and force deselect:** `H: SocialSessionHostLikeCpp`, which
    gains one provided fixture accessor,
    `#[cfg(any(test, feature = "test-fixtures"))]
    social_limits_for_test_like_cpp(&mut self) -> Option<&mut SessionSocialLimits>`
    (default `None`; the World impl returns `Some(&mut self.social)` under
    `#[cfg(test)]` only).
  - **Trait methods kept, now naming the free function:** the World impls of
    `PlayerMovementHostLikeCpp` (the two aura reads, the far-teleport
    interrupt), `SocialSessionHostLikeCpp` (the modifier read),
    `CombatSessionHostLikeCpp` (polymorph), the character login host (the
    aura-authority setter), the quest host (hit-inert), `PlayerCastHostLikeCpp`
    (self-res), `ContinueLoginHostLikeCpp` (proficiencies),
    `SpellEffectHostLikeCpp` (pull attack start) and
    `SpellExecutionHostLikeCpp` (reputation, force deselect). The World
    `player_cast::Runtime::cast_time_multiplier` and the staying
    spell-hit-authority and passive-aura bodies call the moved queries by
    `wow_world_spell::` path.
- **Mechanical rules** (`move_token_rules/1363-s12b3-{queries,operations,social}.json`
  with pairs alongside): the signatures; `crate::session::split_spell_state_{ref,mut}(self)`
  → `host.split_spell_state_{ref,mut}()`; `self.core.mutate_world_creature` →
  `host.core_mut().…` (it takes `&mut`); `self.core.` → `host.core().`;
  `self.catalogs.` / `self.config.` → `host.hub_ref().catalogs.` / `.config.`;
  `crate::session::hub_{ref,mut}(self)` → `host.hub_{ref,mut}()`;
  `<crate fn>(self,` → `(host,`; the inlined wrappers below; `self.` →
  `host.`. Rules never touch string literals.
  - **Result** (`move_token_compare.py --base origin/3.4.3`): queries 12 of
    12, operations 4 of 4, social 1 of 2 identical. The force deselect differs
    only by its fixture step: `#[cfg(test)] { …; self.social.record_…(…) }` →
    `#[cfg(any(test, feature = "test-fixtures"))] if let Some(social) =
    host.social_limits_for_test_like_cpp() { …; social.record_…(…) }`, the
    `#[cfg(not(test))] let _ = caster_guid;` gate widened the same way, and
    `RepresentedForceDeselectLikeCpp` named by `crate::` path. The step runs
    exactly in the World test session, as before; in production builds it is
    absent. Its `DEFAULT_VISIBILITY_DISTANCE_YARDS_LIKE_CPP` (100, a
    `#[cfg(test)]` constant of the session root used only here) moves with it.
- **Inlined one-line wrappers** (each verified identical to the World body):
  - `WorldSession::spell_store` (`self.catalogs.spell_store()`) →
    `host.hub_ref().catalogs.spell_store()` (proficiencies); stays for other
    callers.
  - `WorldSession::currency_types_store` (`self.catalogs.currency_types_store()`)
    → the same over `host.hub_ref().catalogs` (reputation). Its last caller
    moved and no scenario calls it, so it is **retired**.
  - `WorldSession::player_guid` (`self.core.player_guid()`) and
    `WorldSession::send_packet` (`self.core.send_packet(pkt)`) → the
    `SessionHubHostLikeCpp` provided methods, which are the same bodies over
    `self.core()`.
- **Shims.** `handlers/spell_state_session_test_shims.rs` (`#![cfg(test)]`,
  mounted from `handlers/mod.rs` outside the hotspot trees, 103 lines) holds
  one-call forwards for the nine moved names the scenarios call and the two
  reclassified reads. One unit-test file changes one line: the
  `effect_queries` `f3_shims.rs` imports `crate::session::*` instead of
  `super::*`, because the emptied `effect_queries.rs` no longer imports the
  session root. No test function changed.
- **Files retired:** `session/spell_effects/effects.rs` and
  `session/spell_effects/effects_progress.rs`.
- **Visibility.** The 18 free functions, `SpellStateHostLikeCpp` and the two
  social functions are `pub` and re-exported from their crate roots (the
  World calls each).

### Host surface

Moved: **322 function lines** (base).

- **New host: 20 production lines, 6.2 %.** The `SpellStateHostLikeCpp`
  declaration with docs (9) and its World impl (11).
- **With the fixture accessor (13): 33 lines, 10.2 %.** The provided
  declaration in social (7, compiled only with `test`/`test-fixtures`) and the
  World `#[cfg(test)]` override (6; the classifier counts it as production
  because it is nested in a production impl).
- No adapter file was merged; merged and unmerged figures are the same. No
  host method was retired.

### Await counts (per function, base vs head)

Every moved function is synchronous: 0 awaits at base and at head, 18 of 18.

### Recursion

- **Production grep.** Every moved name, both inlined wrappers and the three
  new host methods were grepped in production (non-shim) `wow-world`. Each
  remaining hit is a trait-impl declaration whose body names
  `wow_world_spell::<name>(self, …)` / `wow_world_social::<name>(self, …)`,
  or a call by that path; no `WorldSession::<moved>` or `self.<moved>(` is
  left. The ten kept trait impls used to say `WorldSession::<name>(self, …)`;
  all name the free function now.
- **New host methods.** `interrupt_player_cast_like_cpp` and
  `player_persistent_capability_state_snapshot_like_cpp` name existing
  inherent methods by `WorldSession::` path, which resolve before any trait
  method; `social_limits_for_test_like_cpp` returns the field.
- **Free functions.** No moved body calls a host method of its own name.
- `unconditional_recursion`: 0 in every unit of all three canonical builds.

### Registration

No registrar moved. Spell execution and effect dispatch, the login, the
far teleport, the player cast, opcode registrations, their order,
`SessionStatus`, `PacketProcessing`, handler names and thunks are unchanged.
No checker constant or `handler-module-policy.json` change.

### Edges and closure

- **New edge `wow-world-social` → `wow-progression`** (`Cargo.toml`,
  `Cargo.lock` one line, and the `wow-world-social` restricted direct-dependency
  list in `dependency-policy.json`). `wow-progression` is domain-runtime and
  already a dependency of core, entities, inventory, lifecycle, character and
  quest; social reaches no new `wow-world-*` crate.
- Spell's existing edges cover the rest (entities for the attack start).
- Closure: simulated before the move (`--add`, lock) PASS; at head, lock and
  metadata modes, PASS (18 packages).

### Accounting

| Crate `src` | Base physical | Head physical | Base production | Head production |
|---|---:|---:|---:|---:|
| `wow-world` | 36,648 (451 files) | 36,369 (450) | **26,375** | **25,991** |
| `wow-world-spell` | 21,227 (98) | 21,532 (102) | 20,909 | 21,214 |
| `wow-world-social` | 11,456 (30) | 11,616 (31) | 11,340 | 11,498 |

Basis of the production columns (relabelled after the #1417 review): the
**src-tree physical sum**, i.e. the per-file classifier summed over the files
under each crate's `src`. It excludes the 1,110 `wow-world` production lines
mounted by `#[path]` from outside `src`, which the logical crate count
(`logical_hotspot_row(lib.rs, "crate")`) includes; it is not a "shim-aware"
count.

- **`wow-world` production −384:**
  - **Moved:** −322 function lines.
  - **Reclassified production → test:** −51 (the two dead-code reads).
  - **Retired:** −3 (the `currency_types_store` wrapper) and −24 (the two
    retired files' headers, imports and impl shells beyond their bodies:
    `effects.rs` −13, `effects_progress.rs` −11), −1 in
    `spell_effects/mod.rs` (two `mod` lines out, one doc line in).
  - **Imports:** −7 (the session root −5: the attack-start and
    send-if-visible mailbox imports and the `RepresentedPlayerSpellLikeCpp`
    and `ReputationGainSourceLikeCpp` re-exports leave production, the two
    the scenarios still read joining existing groups in
    `session/test_reexports.rs`; `faction_reactions.rs` −2).
  - **Impl shells, doc headers and separators** in the emptied or shrunk
    `spell_state` files: −6 (`effect_queries.rs`), −3 (`aura.rs`), +1 each in
    `aura_publication.rs`, `cast.rs`, `spell_publication.rs` and
    `spellbook.rs`, +3 (`spell.rs`): −2 together.
  - **Repointed call sites (rustfmt):** +4 in `spell_hit_authority.rs` (the
    six repoints wrap, net of the removed setter's separator lines) and +5
    in `aura_application.rs`.
  - **Host adapter:** +11 (the `SpellStateHostLikeCpp` impl) and +6 (the
    `#[cfg(test)]` fixture accessor nested in the social host impl).
  - −322 − 51 − 3 − 24 − 1 − 7 − 2 + 4 + 5 + 11 + 6 = −384.
- **Destinations:** spell +305 (209 of it the moved bodies after rustfmt,
  78 + 68 + 63; the rest the module root with the trait, headers, imports,
  separators and the 16-line crate-root re-export); social +158 (the new
  file's 151 lines less its 2-line `test`/`test-fixtures` constant, the two
  crate-root lines and the 7-line accessor, which the classifier counts
  because it is nested in the trait).
- **`net_move.py check --base origin/3.4.3`:** growth 465 within the 1,254.0
  allowance, 0 duplicate bodies, PASS.
- **`session_method_index`:**

| Measure | Base `85f5703dd` | Head |
|---|---:|---:|
| Non-forward (crate) | 320 / 7,949 | **300 / 7,586** |
| Forwards (crate) | 270 / 1,494 | 269 / 1,491 |
| Staying | 44 / 1,190 | 44 / 1,190 |
| Outside | 248 / 5,547 | **228 / 5,184** |

  −20 non-forward methods are the 18 moved and the 2 reclassified (−373
  lines); the three staying bodies that now name the moved queries by path
  grow by 10 lines (`can_authorize…` 85 → 90, the two passive-aura replays
  58 → 60 and 54 → 57), so the line delta is −363. −1 / −3 forwards is the
  retired `currency_types_store`. `session/spell_state` goes from 35 / 620 to
  17 / 381 (the six setters and the eleven staying methods above);
  `session/spell_effects` from 2 / 124 to 0.

### Policy and ceilings

- **Ceilings tightened, none raised:**
  - **`session-method-index.json`:** `outside_ceiling` 248 / 5,547 → 228 /
    5,184 and `crate_ceiling` 320 / 7,949 → 300 / 7,586; `staying_bound`
    stays at 44 / 1,190.
  - **Hotspot `session/mod.rs`:** production 17,109 → 16,725, test 141,616
    → 141,616, total 158,725 → 158,341.
  - **Physical `session/mod.rs`:** 798 → 791.
- **New physical row:** `handlers/spell_state_session_test_shims.rs` at 103,
  under the shim convention.
- **`dependency-policy.json`:** `wow-progression` added to the
  `wow-world-social` restricted direct-dependency list (the authorised edge
  above); nothing else changes.
- `session-ownership-policy.json` is reprinted with `print-baseline` (the
  retired impl items, the shim items, the `SpellStateHostLikeCpp` impl and
  the accessor, the registry-access row of the attack start now in spell);
  `check --syntax-only` PASS.

### Warnings and tests

Canonical recipe (with `--locked`), base `85f5703dd` vs head, counted and
compared with `warning_units.py`:

| Build | `wow-world` lib | `wow-world` lib test | All `wow-world*` lib units |
|---|---:|---:|---:|
| production lib | 118 → 118 | — | 245 → 245 |
| lib + `test-fixtures` | 127 → 127 | — | 209 → 209 |
| `--lib --tests` | 127 → 127 | 178 → 178 | 209 → 209 |

- **`compare`:** nothing appears only in the base or only in the head, in
  any build. Before the import cleanup the head had the two mailbox imports,
  the two re-exports and `faction_reactions`' gain-source import as new
  unused imports, and `WorldSession::currency_types_store` as new dead code.
- **`wow-world`:** `dead_code` 0 in all three builds. `unconditional_recursion`
  is 0 in every unit of all three builds.
- `cargo check --tests -p wow-world-spell -p wow-world-social`: no warning in
  the new files.
- **Tests.** The test-name multiset is identical: `git grep` of
  `#[test]`/`#[tokio::test]` attributes under `crates/` counts 9,517 at base
  and at head, and `#[ignore` 5 at both. No assertion changed; the only
  test-tree change is the one import line above.
- **Focused run during the move:** `cargo test -p wow-world --lib --
  force_deselect reputation proficienc spell_hit_source self_res polymorph
  pull far_teleport cast_speed`: 98 passed.

### Follow-ups

- **The spell-hit authority proof** (`can_authorize…`, its identity term and
  the sync) and **the login passive-aura replays with the aura-application
  builder and the login spell reset** move to `wow-world-application`, where
  the quest, inventory and aura-application context are reachable; the
  passive-aura replays can use the application's reproduced
  `session_aura_application_cx_like_cpp`.
- **The spell-map seam** (`with_`/`mutate_player_spell_runtime_like_cpp`,
  the snapshot) stays until the `cfg!(test)` fixture flag is replaced by a
  fixture accessor in a dedicated #754 step.
- **12d:** `SpellStateHostLikeCpp` carries two methods; fold it when the
  adapters regroup.
- **Shims.** The `#![cfg(test)]` shims retire in the final cleanup.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section. The manifest path,
dirty flag and duration are in the PR body. This section is not edited after
that run, so the PR head is the validated SHA.

## Slice 12b4 result (represented login-load steps → character, login spell reset → application)

Branch `1363-s12b4-persistence`, base `ef33829da` (slice 12b3 merged).

### #1416 follow-ups 1–2

Separate commit `bcb086f90`, before anything moved; the measurement on the
base is unchanged (staying 44 / 1,190, outside 228 / 5,184, crate 300 / 7,586).

- **`r#` on any path segment.** `IMPL_RE` and the `type` form of `ALIAS_RE`
  accept `(?:(?:r#)?\w+\s*::\s*)*` before `WorldSession`. The self-test adds
  `type SegRaw = crate::r#session::WorldSession;` (rejected at `aliases.rs:5`)
  and a `session/raw_segment.rs` with `impl crate::r#session::WorldSession`
  (indexed as `raw_segment_impl`). Mutation check: dropping the segment `r#`
  from `IMPL_RE`, then from `ALIAS_RE`, each fails the self-test (2 of 2).
- **12b3 line totals.** One sentence in the 12b3 result now reconciles the
  373 moved and reclassified lines with the index's −363 (the three staying
  bodies grew by 10).

### Re-measure and the cut

`session_method_index.py index --methods` on the base: `session/persistence`
20 / 382, `session/player_binding` 3 / 146, `session/player_bootstrap` 1 / 406
(24 / 934, as planned). Each was read against its callers:

- **Moved to `wow-world-character` (7 / 149):** the trait-config authority
  load pair `begin_…` (5) and `complete_represented_trait_config_authority_load_like_cpp`
  (21) (`persistence/load_authority.rs`); the group restore
  `load_represented_group_by_db_store_id_like_cpp` (36) and
  `load_represented_group_difficulties_like_cpp` (28); the void-storage load
  `mark_represented_void_storage_loaded_like_cpp` (4),
  `prepare_represented_void_storage_login_load_like_cpp` (11) and
  `load_represented_void_storage_row_like_cpp` (44) (`persistence/load.rs`).
  Their only production callers are the character login loaders.
- **12b3 deferral moved to `wow-world-application` (2 / 28):**
  `apply_represented_login_spell_reset_if_needed_like_cpp` (20,
  `spell_state/effects.rs`) and its only callee
  `reset_spells_notification_text_like_cpp` (8, `spell_state/spell.rs`).
- **Reclassified to a test shim (1 / 4):** `clear_represented_void_storage_like_cpp`
  (`void_storage_adapter.rs`), inlined into the moved prepare step, has no
  production caller left.
- **Stay, with the reason:**
  - `build_initial_player_for_owner_like_cpp` (406): its callers are the World
    canonical-access operations and test support, not domain crates (the
    plan's condition).
  - `set_player_guid` (95) and `attach_player_controller_like_cpp` (47): the
    session identity boundary; they reset `view`, `quest_state`, `social`,
    `spell_state`, `visibility`, `lifecycle` and fixtures together. A host
    for them would expose most of the session; they stay with the runtime
    shell until 12d.
  - The persistence money fences `begin_exclusive_player_money_persistence_like_cpp`
    and `mutate_and_persist_player_gold_exclusive_like_cpp` (async, #169):
    they are World construction seams that pass four disjoint field borrows
    to the quest coordinator; moving them changes no logic and would need a
    four-way split host method. Not moved, so their awaits are untouched.
  - `load_tutorials_data_like_cpp` (async, world-server caller),
    `load_represented_xp_rest_bonus_like_cpp`, `set_loaded_player_identity_like_cpp`,
    `mark_represented_talents_loaded_like_cpp` (quest reach),
    `player_persistent_capability_state_snapshot_like_cpp` (`cfg(test)`
    fixture fallback), `set_player_loading`, the split forwards
    (`load_character_reputation_rows_like_cpp`, `represented_talents_loaded_like_cpp`,
    `set_loaded_player_name_like_cpp`, `plan_add_currency_vendor_like_cpp`,
    `set_player_gold_like_cpp`): each is either a field-split forward (12c/12d)
    or carries a `cfg(test)` fixture branch that needs its own accessor step.
  - **12b3 deferrals not moved:** the spell-hit authority proof (90 + 9 + 16)
    reads seven World-only sources (equipment authority, guild id, battle-pet
    and war-mode sources, inventory, buyback and item objects) and the
    passive-aura replays (60 + 57) with the aura-application builder (17)
    need a five-way `split_aura_application_mut` host method; either would
    add more host than it moves at this size. They stay for a dedicated
    application step.

**Moved: 9 functions, 177 base lines; reclassified 1 / 4.**

### What moved

| Destination file | Functions (base file) | Base lines |
|---|---|---:|
| `wow-world-character/src/login_session/represented_load.rs` | the trait-config pair (`load_authority.rs`), the group pair and the void-storage trio (`load.rs`) | 149 |
| `wow-world-application/src/continue_login/spell_reset.rs` | the login spell reset (`effects.rs`) and its text (`spell.rs`) | 28 |
| **Total** | 9 functions | **177** |

Lines per destination: character 149, application 28.

- **Hosts.**
  - Trait-config pair: `H: wow_world_spell::SpellbookHostLikeCpp` (the core
    invalidation through `host.core()`). The two
    `CharacterLoginHostLikeCpp` methods stay (their caller's `H` has no
    spellbook bound); the World impls name the free functions.
  - Group and void-storage: `H: CharacterLoginHostLikeCpp`, which gains
    `replace_player_difficulty_preferences_like_cpp` and
    `void_storage_is_unlocked_like_cpp`; the World impl names the inherent
    methods by `WorldSession::` path. The character void-storage loader
    (`inventory_load.rs`) now calls the free functions directly, so the two
    trait methods `prepare_represented_void_storage_login_load_like_cpp` and
    `load_represented_void_storage_row_like_cpp` retire from the trait and the
    World impl. `load_represented_group_by_db_store_id_like_cpp` stays a trait
    method (its World impl names the free function): the source-contract
    scenario `group_membership_login_phase_resets_then_restores_represented_group`
    pins the `host.…(db_store_id)` call text in `group_loading.rs`, and the
    first `final` run caught the repoint, so it was reverted rather than the
    test changed.
  - Spell reset: `H: TalentHostLikeCpp + QuestSessionHostLikeCpp +
    AuraHostLikeCpp`; no new host method (`TalentHostLikeCpp` already carries
    the at-login flag read/remove and `trinity_string_like_cpp`). The two
    `LANG_RESET_SPELLS*` constants move with it.
- **Mechanical rules** (`move_token_rules/1363-s12b4-{login-load,spell-reset}.json`
  with pairs): the signatures; `crate::session::split_inventory_mut(self)` →
  `host.split_inventory_mut()`; `self.core.` → `host.core().`; `<crate fn>(self`
  → `(host`; the inlined wrappers below; the void-row fixture step
  `#[cfg(test)] { if self.core.player_handle_like_cpp.is_none() { self.inventory.…`
  → `#[cfg(any(test, feature = "test-fixtures"))] { if host.test_fixtures_ref_like_cpp().is_some()
  && host.core().player_handle_like_cpp.is_none() { host.split_inventory_mut().0.…`
  with its `#[cfg(not(test))]` widened the same way (the precedent of
  `wow-world-instances` `login_load.rs`: the step runs exactly in the World
  test build); `self.` → `host.`. Rules never touch string literals.
  **Result** (`move_token_compare.py --base origin/3.4.3`): 7 of 7 and 2 of 2
  identical.
- **Inlined one-line wrappers** (each verified identical to the World body):
  `clear_represented_void_storage_like_cpp` (`split_inventory_mut` + the state
  call; retired to the shim), `item_storage_template`
  (`self.catalogs.item_storage_template` → `host.hub_ref().catalogs.…`; stays
  for other callers), `player_guid` (the `SessionHubHostLikeCpp` provided
  method, same body). `resolved_group_guid_like_cpp` resolves to
  `WorldEntitiesHostLikeCpp`, whose World impl names the inherent method.
- **Shims.** `handlers/login_load_test_shims.rs` (`#![cfg(test)]`, 58 lines):
  one-call forwards for the eight moved names the scenarios call, and the
  vault clear. No test file changed.
- **Files retired:** `session/spell_state/effects.rs`.
- **New edge `wow-world-character` → `wow-social`** (`group_guid_by_db_store_id_like_cpp`;
  `Cargo.toml`, `Cargo.lock`, `dependency-policy.json`). Simulated first:
  `wow_world_closure.py check --add "wow-world-character->wow-social"` PASS
  (19 packages). `wow-social` is domain; lifecycle and core already depend on it.

### Host surface

Moved 177 function lines (base). New host: the two
`CharacterLoginHostLikeCpp` declarations with docs (+11) and their World
impls (+6) = 17; retired: two trait declarations (−6) and their World impls
(−10). **Net host +1 line (0.6 %); gross new 17 lines, 9.6 %.** No adapter
file merged; merged and unmerged figures are the same.

### Await counts (per function, base vs head; #169)

| Function | Base | Head |
|---|---:|---:|
| the 9 moved functions | 0 | 0 |

All moved bodies are synchronous; no future is spawned, reordered or held
across a different borrow. The async persistence paths (money fences, tutorial
and account-data loads, instance time restrictions) did not move. Save and
load order is unchanged: the character loaders call the same steps at the
same points (`login_data.rs` and `group_loading.rs` through the kept trait
methods, `inventory_load.rs` through the free functions).

### Recursion

- Every moved name and both new host methods were grepped in production
  (non-shim) `wow-world`: the only hits are the kept trait-impl methods whose
  body names `wow_world_character::<name>(self, …)` /
  `wow_world_application::<name>(self)`, and the two new impl methods naming
  `WorldSession::<name>(self, …)` (inherent first). No moved body calls a host
  method of its own name.
- `unconditional_recursion`: 0 in every unit of all three canonical builds.

### Registration

No registrar, opcode, `SessionStatus`, `PacketProcessing`, handler name or
thunk changes; no `handler-module-policy.json` change.

### Accounting

Crate production by the logical crate count
(`logical_hotspot_row(lib.rs, "crate")`). It counts 27,101 for `wow-world` at
base, 1,110 more than the 25,991 recorded with the 12b3 figures, which used
the src-tree physical sum (excludes 1,110 `#[path]` production lines).

**Corrected after the #1417 review** (follow-up 1): the head figures first
recorded here (26,894, −207; character 9,598, +200) were measured three
lines before the final head. On the merged head `10bdca1a7`:

| Crate | Base | Head | Δ |
|---|---:|---:|---:|
| `wow-world` | 27,101 | 26,897 | **−204** |
| `wow-world-character` | 9,398 | 9,599 | +201 |
| `wow-world-application` | 16,130 | 16,181 | +51 |

On the src-tree basis `wow-world` production is 25,991 − 204 = **25,787**.

**From slice 12b5 on this document uses the logical crate count only.**
Conversion for earlier figures: a src-tree physical sum from 12b3 or 12b4 plus
1,110 gives the logical crate count (e.g. 25,991 + 1,110 = 27,101); figures
before 12b3 were not re-measured on the logical basis.

- `net_move.py check --base origin/3.4.3`: PASS (0 duplicate bodies).
- `session_method_index`:

| Measure | Base `ef33829da` | Head |
|---|---:|---:|
| Non-forward (crate) | 300 / 7,586 | **290 / 7,405** |
| Forwards (crate) | 269 / 1,491 | 269 / 1,491 |
| Staying | 44 / 1,190 | 44 / 1,190 |
| Outside | 228 / 5,184 | **218 / 5,003** |

  −10 / −181 = the 9 moved (177) and the reclassified vault clear (4).
  `session/persistence` 20 / 382 → 13 / 233; `session/spell_state` 17 / 381 →
  15 / 353.

### Policy and ceilings

- **Tightened, none raised:** `session-method-index.json` outside 228 / 5,184
  → 218 / 5,003, crate 300 / 7,586 → 290 / 7,405 (staying bound unchanged);
  hotspot `session/mod.rs` production 16,725 → 16,526, test 141,616 →
  141,616, total 158,341 → 158,142; hotspot `handlers/character/mod.rs`
  production 2,229 → 2,224, total 17,884 → 17,879; physical
  `session/mod.rs` 791 → 787.
- New physical row `handlers/login_load_test_shims.rs` (58, shim convention).
- `dependency-policy.json`: `wow-social` added to `wow-world-character`.
- `session-ownership-policy.json` reprinted with `print-baseline`.
- Imports: the group-id import and the void-storage alias leave production;
  the alias joins `session/test_reexports.rs` (its three-line header is
  reflowed so the test count stays equal).

### Warnings and tests

Canonical recipe (`--locked`, `--jobs 1`), base `ef33829da` vs head:

| Build | `wow-world` lib | `wow-world` lib test | All `wow-world*` lib units |
|---|---:|---:|---:|
| production lib | 118 → 118 | — | 245 → 245 |
| lib + `test-fixtures` | 127 → 127 | — | 209 → 209 |
| `--lib --tests` | 127 → 127 | 178 → 178 | 209 → 209 |

Compared by unit, lint, file and message: nothing only in base or head in
any build. `wow-world` `dead_code` 0 in all three; `unconditional_recursion`
0. Tests: `#[test]`/`#[tokio::test]` 9,517 at base and head, `#[ignore` 5 and
5; no test file changed.

### Follow-ups

- The spell-hit authority proof and the passive-aura replays with the
  aura-application builder (application, with a split host method).
- The persistence money fences, `set_player_guid`/`attach_player_controller`
  and the bootstrap: 12d / runtime shell.
- The `cfg(test)` fixture branches of the xp-rest load, identity load and
  capability snapshot need fixture accessors before they can move.
- Shims retire in the final cleanup.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section; manifest, dirty flag
and duration are in the PR body. This section is not edited after that run.

## Slice 12b5–12b6 result (social, chat, quest, world-entity and visibility session bodies → instances, social, character, quest, entities)

Branch `1363-s12b5-social-quest`, base `10bdca1a7` (slice 12b4 merged). One
PR for both sub-slices: together they move 551 base lines, well under the
~4.5k bound. Move-only (R2): group, guild, duel, gossip and chat packets and
their order, quest state, world-entity and visibility publication order, the
deferred visibility obligation and persistence (#169) are unchanged; no
registrar moved.

From this slice on, crate production figures are the **logical crate count**
(`logical_hotspot_row(lib.rs, "crate")`); see the corrected 12b4 accounting
for the conversion of earlier figures.

### Re-measure and the cut

`session_method_index.py index --methods` on the base: `session/social` 17 /
294, `session/chat` 5 / 118 (12b5: 22 / 412); `session/quest` 7 / 185,
`session/world_entities` 7 / 179, `session/visibility` 10 / 175,
`session/deferred_visibility` 3 / 109 (12b6: 27 / 648). Two of them are
already on the allow-list (`set_quest_store`, `set_group_registry`). Each of
the rest was read against its callers and what it reaches:

- **12b5 moved (14 / 357):**
  - to `wow-world-instances` (`group_membership.rs`, 5 / 106), the group
    queries every caller reaches through a host impl:
    `current_player_is_in_raid_group_like_cpp` (13),
    `represented_player_is_same_raid_with_like_cpp` (15),
    `current_group_member_guids_for_tap_like_cpp` (25),
    `authoritative_group_membership_like_cpp` (19) over
    `InstanceSessionHostLikeCpp`, and
    `reset_group_update_sequence_if_needed_like_cpp` (34) over
    `GroupSessionHostLikeCpp` (its fixture step needs the social split);
  - to `wow-world-social` (4 / 133): `apply_duel_effect_like_cpp` (97) with
    its two duel-state steps `set_represented_duel_arbiter_guid_like_cpp` (4)
    and `set_represented_duel_state_like_cpp` (14) (`spell_effects.rs`,
    private there), and `set_represented_guild_id_like_cpp` (18, `guild.rs`);
  - to `wow-world-character` (`gossip_options.rs`, 3 / 55): the gossip-option
    clear (11), replace (22) and read (22); their only production callers are
    the character gossip bodies and the World player binding;
  - to `wow-world-quest` (`quest_session/dialog.rs`, 2 / 63): the creature
    gossip quest text (42) and its menu-item builder (21), which call quest
    eligibility and menu functions over the quest host.
- **12b6 moved (9 / 179, plus the 3-line forward and the 12-line module
  function they use)** to `wow-world-entities`
  (`visibility_session/session_visibility.rs`) over
  `VisibilitySessionHostLikeCpp`: the seer-detection sync (17, with
  `player_session_never_visible_for_seer_like_cpp` and
  `apply_player_session_visibility_detection_like_cpp`), the registry-backed
  other players (37) with their canonical phase gate (19), the phase shift
  (13) and phase test (4), the resting flag (26), the seer GUID (30), the
  forced visibility update (8, async) and the active-mover completion (25,
  from `deferred_visibility.rs`).
- **Field-split forwards retired (5 / 37):** `current_player_is_in_group_guid_like_cpp`
  (8), `represented_player_at_group_reward_distance_like_cpp` (14),
  `set_owned_player_group_like_cpp` (7), `broadcast_player_packet_to_visible_set_realm_like_cpp`
  (4) and `sync_player_registry_party_member_party_type_like_cpp` (4). Their
  single production caller (a host impl, or `register_in_player_registry`)
  now names the split; the scenarios keep a shim.
- **Not moved: candidates for the staying allow-list** (recorded here only;
  the allow-list is not changed by this PR):
  - `quest_objective_operation_like_cpp` (110, async): the World construction
    seam that builds `QuestRewardCx` and `QuestObjectiveProgressCx` over ten
    disjoint session fields and the fixtures; a host method for it would
    expose most of the session (the 12b4 money-fence reasoning).
  - `represented_can_see_spell_click_on_creature_like_cpp` (92) and
    `represented_gameobject_dynamic_flags_for_player_like_cpp` (47): context
    builders over fixtures, catalogs, quest, inventory, social and
    world-entity fields; the split host method would be larger than the body.
  - `handle_destroy_visible_object_like_cpp_command_like_cpp` (46),
    `apply_deferred_player_visibility_refresh_like_cpp` (36) and
    `flush_pending_visibility_refresh_with_catalogs_like_cpp` (20): session
    command and mailbox-pump application, gated on session state,
    disconnection and the canonical incarnation; called only from
    `session_commands.rs` and the mailbox pump (runtime shell).
  - `trade_handler_cx_like_cpp` (4): the four-way trade split builder
    (host-side construction).
  - `resolved_group_guid_like_cpp` (7): passes the World `cfg!(test)` flag;
    every `resolved_group_guid` trait impl names it (the host's own seam).
- **Deferred (movable later), with the reason:**
  - `clear_represented_resurrection_request_like_cpp` (14) and
    `take_represented_resurrection_request_if_requested_by_like_cpp` (23):
    the take's fixture branch calls a World test-only helper; it needs its own
    fixture accessor step (the pair moves together, to entities).
  - `invalidate_player_quest_status_authority_like_cpp` (7): the quest crate
    already has a free function of that name with another signature, and the
    body goes through the quest-state split seam below.
  - `mutate_player_quest_gameplay_like_cpp` (19) and
    `player_quest_gameplay_snapshot_like_cpp` (6): the quest-state split seam
    named by the quest, social and loot host impls (the #756 retained
    projection, exit with #41); 12c.
  - `represented_viewer_dependent_creature_npc_flags_like_cpp` (14): wraps the
    spell-click context builder; it follows once that builder is a host method.
  - Field-split forwards for 12c: `mutate_canonical_gameobject_by_guid_like_cpp`
    (8; retiring it grew the loot hotspot by a line, so it was restored),
    `canonical_creature_access_like_cpp` (8), `sync_represented_farsight_clear_from_canonical_like_cpp`
    (4, five callers), `send_represented_gameobject_visibility_on_destroy_from_last_update_like_cpp`
    (6, runtime-driver caller), `resolved_represented_active_trade_partner_like_cpp` (6).

**Moved: 25 functions (23 non-forward, 1 forward, 1 module function), 551
base lines; 5 field-split forwards (37) retired.**

### What moved

| Destination file | Functions (base file) | Base lines |
|---|---|---:|
| `wow-world-instances/src/group_membership.rs` | the five group queries (`social/group.rs`) | 106 |
| `wow-world-social/src/spell_effects.rs` | the duel effect and its two steps (`social/duel.rs`) | 115 |
| `wow-world-social/src/guild.rs` | the guild-id install (`social/guild.rs`) | 18 |
| `wow-world-character/src/gossip_options.rs` | the gossip-option trio (`chat/operations.rs`) | 55 |
| `wow-world-quest/src/quest_session/dialog.rs` | the gossip quest text and menu-item builder (`chat/operations.rs`) | 63 |
| `wow-world-entities/src/visibility_session/session_visibility.rs` | nine visibility functions, one forward and one module function (`visibility/operations.rs`), the active-mover completion (`deferred_visibility.rs`) | 194 |
| **Total** | 25 functions | **551** |

Lines per destination (base lines): instances 106, social 133, character 55,
quest 63, entities 194.

- **Hosts.**
  - Group queries: `H: InstanceSessionHostLikeCpp` (`resolved_group_guid`
    through its entities supertrait, `core()`, `player_guid()`); the reset:
    `H: GroupSessionHostLikeCpp` (`split_social_mut()` for the fixture step).
  - Duel and guild: `H: SocialSessionHostLikeCpp`, which gains
    `split_social_mut` (the World impl names `crate::session::split_social_mut`).
  - Gossip options: `H: CharacterLoginHostLikeCpp`; its three gossip-option
    methods retire (the gossip bodies call the free functions), and it gains
    one provided fixture accessor `interaction_for_test_like_cpp` (default
    `None`, compiled only with `test`/`test-fixtures`; the World impl returns
    the field under `#[cfg(test)]` only).
  - Gossip text: `H: QuestSessionHostLikeCpp`; no new host method.
  - Visibility: `H: VisibilitySessionHostLikeCpp` (`lifecycle()`,
    `visibility()`, `visibility_mut()` already exist); its
    `visible_other_players_from_registry_like_cpp` method retires
    (`update.rs` calls the free function).
  - Trait methods kept, now naming the free function: the World impls of
    `InstanceSessionHostLikeCpp` (raid group, membership),
    `WorldEntitiesHostLikeCpp` (same raid, phase shift, phase test),
    `VehicleSessionHostLikeCpp` (same raid), `CreatureDamageHostLikeCpp`
    (tap), `CharacterLoginHostLikeCpp` (reset, gossip text, detection sync,
    forced update), `ContinueLoginHostLikeCpp` (guild id),
    `SpellExecutionHostLikeCpp` (duel, forced update), `CombatSessionHostLikeCpp`
    (resting), `VisibilitySessionHostLikeCpp` (seer),
    `PlayerTeleportHostLikeCpp` (detection sync) and
    `MovementHandlerHostLikeCpp` (active-mover completion). The World inherent
    callers (`player_binding`, `action_bar_adapter`, `persistence/load`,
    `movement/state`, `taxi/operations`, the loot visibility command,
    `player_handler_contexts`, `deferred_visibility`, the visibility flush)
    name the free functions by `wow_world_*::` path.
- **Mechanical rules** (`move_token_rules/1363-s12b5-{group,social,gossip}.json`,
  `1363-s12b6-visibility.json`, with pairs): the signatures; `self.core.` →
  `host.core().`; `crate::session::hub_{ref,mut}(self)` → `host.hub_{ref,mut}()`;
  `crate::session::split_social_mut(self)` → `host.split_social_mut()`;
  `self.catalogs.` → `host.hub_ref().catalogs.`; `self.lifecycle.` →
  `host.lifecycle().`; `self.visibility.clear_last…` →
  `host.visibility_mut().clear_last…`, other `self.visibility.` →
  `host.visibility().`; `wow_world_<dest>::f(self` → `crate::f(host`;
  `self.<moved>(` → `<moved>(host, `; `crate::session::mailbox::` and
  `crate::session::game_time_ms_like_cpp` → their `wow_world_core::session::`
  paths; `cfg!(test)` → `world_test_consumer_like_cpp(host)` (the
  `wow-world-inventory` helper, private to `gossip_options.rs`); the fixture
  steps `#[cfg(test)] if <cond> { … self.<state>.… }` →
  `#[cfg(any(test, feature = "test-fixtures"))] if host.test_fixtures_ref_like_cpp().is_some() && <cond> { … }`
  over the host split or accessor (the 12b4 precedent: the step runs exactly
  in the World test build), the duel record → `if let Some(social) =
  host.social_limits_for_test_like_cpp()` (the 12b3 precedent), the gossip
  read → a let-chain over `interaction_for_test_like_cpp()`, and the seer's
  `#[cfg(test)] return …; #[cfg(not(test))] None` → a gated `if … { return …; }
  None`; one rustfmt-only rule (the duel effect's closure braces); `self.` →
  `host.`. Rules never touch string literals.
  **Result** (`move_token_compare.py --base origin/3.4.3`): group 5 of 5,
  social 4 of 4, gossip 5 of 5, visibility 11 of 11 identical.
- **Inlined wrappers** (each verified identical to the World body):
  `mutate_player_guild_state_like_cpp` (the `#[cfg(test)]` split + social
  call; it stays for the guild shims), `player_guid`, `send_raw_packet`
  (`SessionHubHostLikeCpp` provided methods, same bodies). The duel steps
  `set_represented_duel_state_like_cpp` (`hub_mut` + the social function) and
  `set_represented_duel_arbiter_guid_like_cpp` (`split_social_mut` + the
  social function) moved as private functions rather than being inlined.
- **Shims.** `handlers/social_quest_test_shims.rs` (`#![cfg(test)]`, 88 lines,
  physical row): one-call forwards for the moved names the scenarios call and
  the retired `set_owned_player_group` / duel-arbiter splits. One unit-test
  file changes one line: `social/duel/f3_shims.rs` imports `crate::session::*`
  instead of `super::*`, because the emptied `duel.rs` no longer imports the
  session root (the 12b3 precedent). No test function changed.
- **Manifest.** `wow-world-entities`' `test-fixtures` now also forwards
  `wow-world-lifecycle/test-fixtures` and `wow-world-visibility/test-fixtures`,
  with a self dev-dependency (the `wow-world-instances` pattern, #1409), so the
  moved fixture steps build in a standalone `cargo check -p wow-world-entities
  --tests`; `Cargo.lock` gains the self edge. No production edge is added.

### Host surface

Moved 551 base lines. **New host: 22 lines, 4.0 %** — the `split_social_mut`
declaration (3) and World impl (8), the `interaction_for_test_like_cpp`
provided declaration (7, compiled only with `test`/`test-fixtures`) and World
`#[cfg(test)]` impl (4). **Retired host: 38 lines** — the three gossip-option
declarations (9) and World impls (9), the visible-other-players declaration
(6) and World impl (14). Over all touched host files the diff is +73 / −70
(**net +3, 0.5 %**); the gross additions there (73, 13.2 %) include the
rustfmt wraps of the repointed `wow_world_*::` paths and the three split
inlines. No adapter file merged; merged and unmerged figures are the same.

### Await counts (per function, base vs head; #169)

| Function | Base | Head |
|---|---:|---:|
| `force_update_visibility_with_catalogs_like_cpp` | 1 | 1 |
| the other 24 moved functions | 0 | 0 |
| staying callers repointed (`apply_deferred_player_visibility_refresh`, `flush_pending_visibility_refresh`, the loot visibility command) | 1 each | 1 each |

No future is spawned, reordered or held across a different borrow; the async
quest-objective operation and the money fences did not move.

### Recursion

- Every moved and retired name was grepped in production (non-shim)
  `wow-world`: the only hits are trait-impl declarations whose bodies name
  `wow_world_*::<name>(self, …)` or the explicit split; no
  `WorldSession::<moved>(` or `self.<moved>(` is left. The
  `LootSessionHostLikeCpp::mutate_canonical_gameobject_by_guid_like_cpp` impl
  keeps `self.<name>(` only because the inherent forward was restored.
- Destination crates: no moved body calls a host method of its own name;
  moved-to-moved calls use the free function (`authoritative_group_membership`
  from the tap query, the phase shift from the phase test and the phase gate,
  the detection sync from the active-mover completion).
- `unconditional_recursion`: 0 in every unit of all three canonical builds.

### Registration

No registrar, opcode, `SessionStatus`, `PacketProcessing`, handler name or
thunk changes; no `handler-module-policy.json` change.

### Edges and closure

No new production edge (every destination already reached the crates its
bodies call). `wow_world_closure.py check`: lock and metadata PASS (18
packages).

### Accounting

Logical crate count (`logical_hotspot_row(lib.rs, "crate")`):

| Crate | Base `10bdca1a7` | Head | Δ |
|---|---:|---:|---:|
| `wow-world` | 26,897 | 26,293 | **−604** |
| `wow-world-instances` | 7,988 | 8,128 | +140 |
| `wow-world-social` | 11,498 | 11,655 | +157 |
| `wow-world-character` | 9,599 | 9,692 | +93 |
| `wow-world-quest` | 18,581 | 18,642 | +61 |
| `wow-world-entities` | 20,696 | 20,945 | +249 |

- `wow-world` −604: the 551 moved base lines and the 37 retired forward lines,
  the root imports that leave production and the host retirements, less the
  host additions, the split inlines and the rustfmt wraps of repointed calls.
- `net_move.py check --base origin/3.4.3`: growth 700 within the 1,501.8
  allowance, 0 duplicate bodies, PASS.
- `session_method_index`:

| Measure | Base `10bdca1a7` | Head |
|---|---:|---:|
| Non-forward (crate) | 290 / 7,405 | **262 / 6,835** |
| Forwards (crate) | 269 / 1,491 | 268 / 1,488 |
| Staying | 44 / 1,190 | 44 / 1,190 |
| Outside | 218 / 5,003 | **190 / 4,434** |

  −28 / −573 are the 23 moved and the 5 retired non-forward methods; four
  staying bodies change by +3 lines together (`register_in_player_registry`
  +3 for the inlined split, the visibility flush +3, the deferred refresh −2
  and the loot visibility command −1 for the repointed call), so the line
  delta is −570. The forward is `player_session_never_visible_for_seer`.
  `session/social` 17 / 294 → 4 / 22, `session/chat` 5 / 118 → 0,
  `session/visibility` 10 / 175 → 2 / 24, `session/deferred_visibility`
  3 / 109 → 2 / 82, `session/world_entities` 7 / 179 → 6 / 175,
  `session/quest` unchanged at 7 / 185.

### Policy and ceilings

- **Tightened, none raised:** `session-method-index.json` outside 218 / 5,003
  → 190 / 4,434, crate 290 / 7,405 → 262 / 6,835 (staying bound unchanged);
  hotspot `session/mod.rs` production 16,526 → 15,937, test 141,616 →
  141,615, total 158,142 → 157,552; hotspot `handlers/character/mod.rs`
  production 2,224 → 2,208, total 17,879 → 17,863; physical `session/mod.rs`
  787 → 780.
- New physical row `handlers/social_quest_test_shims.rs` (88, shim convention).
- `session-ownership-policy.json` reprinted with `print-baseline` (the moved
  bodies' direct group- and player-registry rows leave `wow-world`).
- Imports: the gossip text, prepared-menu-item, visibility-snapshot and
  `SessionCommand` imports leave the session root; the three now test-only
  re-exports (`game_time_ms_like_cpp`, `PLAYER_FLAGS_RESTING_LIKE_CPP`,
  `PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP`) join
  `session/test_reexports.rs` in one merged `wow_world_core::session` group,
  so the session-tree test count does not grow (−1).

### Warnings and tests

Canonical recipe (`--locked`, `--jobs 1`), base `10bdca1a7` vs head:

| Build | `wow-world` lib | `wow-world` lib test | All `wow-world*` lib units |
|---|---:|---:|---:|
| production lib | 118 → 118 | — | 245 → 245 |
| lib + `test-fixtures` | 127 → 127 | — | 209 → 209 |
| `--lib --tests` | 127 → 127 | 178 → 178 | 209 → 209 |

Compared by unit, lint, file and message: nothing only in base or head in
any build. `wow-world` `dead_code` 0 in all three; `unconditional_recursion`
0. Tests: `#[test]`/`#[tokio::test]` lines under `crates/` 9,493 at base and
head, `#[ignore` 5 and 5; the only test-file change is the one import line
above.

### Toward `check --done`

After this slice the outside ratchet is **190 methods / 4,434 lines**.
Realistic split, by the cut rules used so far (an estimate, not a
measurement of each method):

- **Movable (≈ 110–130 methods, ≈ 2.4–2.8k lines):** 12b7's leftovers
  (`player_cast` 232, `stand_state_adapter` 311, money / xp / presentation /
  publication ≈ 500, part of `canonical_access`), the remaining
  `player_items` and `spell_state` bodies once their fixture accessors exist,
  the deferred items above, and the 12c field-split forward sweep (the single-split `let`
  forwards the index counts as non-forward; not yet counted one by one).
- **Staying candidates (≈ 60–80 methods, ≈ 1.6–2.0k lines):** the session
  identity and bootstrap (`build_initial_player_for_owner` 406,
  `set_player_guid`, `attach_player_controller`), the persistence money
  fences, the many-field context builders (`quest_objective_operation`, the
  spell-click and gameobject-flag builders, the aura-application and stats
  builders), the session-command and mailbox-pump steps, and the host seams
  that carry the World `cfg!(test)` flag (`resolved_group_guid`, the spell
  runtime trio). `session/map_admission.rs` (176) stays in the outside
  ratchet; it is planned for `wow-world-instances`.

Adding the staying set would raise the allow-list from 44 / 1,190 to roughly
105–125 / 2.8–3.2k; that change belongs to the PR that records the final
allow-list, not to this one.

### Follow-ups

- The resurrection pair (a fixture accessor for its test-only helper), the
  quest-state split seam and the field-split forwards above: 12c.
- The viewer-dependent NPC flags with the spell-click builder: 12d.
- Shims retire in the final cleanup.

### Validation

`VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
runs on the committed head that includes this section; manifest, dirty flag
and duration are in the PR body. This section is not edited after that run.
