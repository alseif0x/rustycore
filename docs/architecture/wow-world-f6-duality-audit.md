# F6 duality audit — `represented_*` vs canonical, legacy runtime and legacy map manager

Read-only audit produced for issue #1263 criterion **F6**. It records what exists today with
`path:line` anchors in both trees and marks behaviour differences separately from structural
ones. **No Rust code was changed, and no repair is proposed inside a structural slice.**

- Rust tree: `/home/server/rustycore-1241`, branch `1263-f6-b`; §1–§7 as audited at `bd7c106fd`,
  §2.1 re-derived at `359bd3dda`, §2.2 re-derived at `1fe9d79c4` (branch `1263-f6-d`).
- C++ reference: `/home/server/woltk-trinity-legacy` at `a5f8da2e` (3.4.3).
- Method: source inspection — `grep`/`sed`/`git log`/file reads — plus, for §2.1 only, `cargo tree`
  feature resolution (metadata only: no compile, no test, no `validation-v2`, no capture, no
  runtime). Every quantitative figure is labelled by how it was obtained.
- Anything not proven from source is written `unverified`; it is never inferred.

## 1. What "represented" and "canonical" mean in this tree

| Term | Concrete meaning | Anchor |
|---|---|---|
| **canonical** | The live `wow-map` map/entity world (`canonical_map_manager`) plus the one generation-checked `wow_entities::Player` behind `player_handle_like_cpp` | `crates/wow-world-core/src/session/state/session_core.rs:127-131` |
| **represented** | A Rust mirror of C++ state that lives on the `Session`/hub instead of on the C++ owner — sometimes the same owner as C++, sometimes a different (per-session) owner | `crates/wow-world-core/src/session/state/hub.rs:10-27`; `crates/wow-world-entities/src/state.rs:11-15` |
| **`resolved_*`** | The read seam that prefers canonical and returns `None` in production when no canonical owner exists | `crates/wow-world-core/src/session/player_vitals_adapter.rs:89-113` |

Three structural facts drive every row of the divergence table:

1. **`represented_*` is a naming convention, not a data-source guarantee.** 765 `fn represented_*`
   definitions exist under `crates/wow-world*/src` (`grep -rn 'fn represented_'`, re-counted for
   F6-1); 565 of them are compiled into a production `world-server` (the other 200 are `cfg`-gated
   out), and 187 of those 565 read a canonical token inside their own body. Both figures are exact
   cfg-resolved counts, §2.1 — the 89/765 previously quoted here was a 20-line window heuristic.
   Example where the name is a vestige and the source is canonical:
   `crates/wow-world-core/src/session/canonical_access/operations.rs:182` reads
   `canonical_player_snapshot_like_cpp` (it is in §2.1's list). **The name is not evidence of the
   owner.**
2. **A large `represented_*` subset is test-only and absent from production.** 200 of the 765
   definitions are absent from a production build: 152 behind `cfg(any(test, feature =
   "test-fixtures"))` and 48 behind `cfg(test)` — an exact cfg-resolved split, not the ±8-line
   grep heuristic that previously gave 222. The `test-fixtures` feature is
   declared "Enabled only through dev-dependencies; **never in a production build**"
   (`crates/wow-world/Cargo.toml:9-11`), and `world-server` pulls it only under `[dev-dependencies]`
   (`crates/world-server/Cargo.toml:43`). The world-server `[dependencies]` block at
   `crates/world-server/Cargo.toml:10-36` does not enable it.
3. **There are two live map/entity stores, both injected in production.**
   `SessionCore::map_manager` (legacy, `wow-world-core::map_manager`) and
   `SessionCore::canonical_map_manager` (canonical `wow_map::MapManager`) are set side by side at
   `crates/world-server/src/session_factory.rs:414-415`. Both are reachable from the same session.

## 2. `represented_*` accessors, grouped by domain

Counts are **production-reachable** `fn represented_*` definitions per path bucket, resolved from
the actual production feature resolution and each item's `cfg` gates (§2.1 states the method,
totals and the whole-closure breakdown). They replace the `grep`-only 2026-10-03 counts, which
were not cfg-aware. "Represented owner" and "canonical owner" are the actual storage owners read,
not the names.

| Domain | Production-reachable `represented_*` fns (leading buckets, §2.1) | Represented form reads | Canonical form reads | Can they disagree? | What C++ reads |
|---|---|---|---:|---|---|
| **Loot** | 43 `crates/wow-world/src/handlers/loot/*` (`rolls.rs:30`); 19 `crates/wow-world-loot` (`state/roll_access.rs:7`, `src/state.rs:82-84`); 1 `crates/wow-world-core/src/session/loot/requests.rs:5` | Session-local `LootState` tables: `loot_table`, `represented_loot_cache_generations_like_cpp`, `represented_unique_gameobject_uses`, `represented_gameobject_tap_lists`, `represented_loot_rolls` — `crates/wow-world-loot/src/state.rs:44-84`; held per session at `crates/wow-world/src/session/state.rs:134` | `OwnedLootAuthority` on the canonical map creature/gameobject; `LootReleaseAccessLikeCpp` — `crates/wow-world-core/src/session/canonical_access/loot_release.rs:203-282` | **Yes** — three copies of one C++ `Loot` (`Creature::m_loot`, `Creature.h:236`; `GameObject::m_loot`, `GameObject.h:322`) | The owning object's single `Loot`; `WorldSession::DoLootRelease` `Handlers/LootHandler.cpp:270`, `Loot::NotifyLootList` `Loot/Loot.cpp:637` |
| **Quest** | 13 `crates/wow-world/src/handlers/quest/*` (`rewards/validation.rs:11`); 11 `crates/wow-world/src/session/quest/*` (`rewards.rs:87`); 9 `crates/wow-world-application/src/quest/visibility/*` (`gameobject_flags.rs:28`); 6 `crates/wow-world/src/session/quest_dialog.rs:35` | `SessionQuestState`: hide-distance config, pending status updates, objective-progress event queue — `crates/wow-world-application/src/quest/session_state.rs:17-29` | `player.gameplay_state().quests` via `player_quest_gameplay_snapshot_like_cpp` — `crates/wow-world-core/src/session/canonical_access/quest_objectives.rs:102-107` | **No for the quest log** (production resolves canonical only, `crates/wow-world-application/src/quest/objective_progress.rs:291-306`); the represented queue is durable-across-await plumbing | `Player::CanCompleteQuest` `Entities/Player/Player.cpp:14123`, `Player::m_quests` |
| **Movement** | 2 of 7 `wow-world-core/src/session/movement/*` (`transfer.rs:169`); the other 5 are gated | Production movement reads go through `resolved_*`; the `SessionFixtures::movement` mirror (position, flags, fall time, forced speed, counter) is entirely `#[cfg(any(test, feature = "test-fixtures"))]` — `crates/wow-world-core/src/session/state/movement.rs:28-110` | Canonical `Unit::m_movementInfo` / `Player::m_lastFallTime` equivalents via `resolved_fall_information_like_cpp` (`crates/wow-world-core/src/session/movement/fall.rs:19`), `resolved_player_movement_flags_like_cpp` (`crates/wow-world-core/src/session/movement/state.rs:641`) | **No in production** (mirror not compiled); **yes in tests** | `Unit::m_movementInfo`, `Player::m_lastFallTime` on the single `Player`; `Player::Update` swing path `Entities/Player/Player.cpp` (`DoMeleeAttackIfReady`) |
| **Visibility** | 1 `wow-world-visibility`; 5 gated there | `client_visible_guids_like_cpp` — per-session, `Arc`-shared — `crates/wow-world-core/src/session/state/session_core.rs:147`; producers `crates/wow-world/src/session/spell_effects/ticks.rs:78,157`, `crates/wow-world/src/session/spell_effects/effects.rs:446,489` | Same set is the visibility authority; no second copy | **No** — the owner matches C++ (per-player set) | C++ `Player::m_clientGUIDs` (per-player), `Entities/Player/Player.h:2450`; mutated at `Player.cpp:23201/23213`; `HaveAtClient` at `Player.cpp:23031` |
| **Entities (creature/GO)** | 13 `crates/wow-world/src/session/world_entities/*` (`gameobject.rs:17`); 23 of 25 `crates/wow-world-entities` | Legacy `map_manager::WorldCreature` (global store) **and** session-local `WorldEntitiesState`: `represented_creature_auras_like_cpp`, `represented_gameobject_use_states`, `represented_gameobject_phase_shifts` — `crates/wow-world-entities/src/state.rs:19-54` | Canonical `wow_map` creature/gameobject via `canonical_map_manager`; `mutate_world_creature` then `sync_canonical_creature_entity_like_cpp` — `crates/wow-world-core/src/session/world_entities/creature_registry.rs:30-59` | **Yes** — dual entity plus revision-gated whole-entity replacement, `crates/wow-world-core/src/session/creature_canonical_adapter.rs:29-70` | One `Creature` per spawn on the map: `Creature::Update` `Entities/Creature/Creature.cpp:696`; `GameObject::GetGoStateFor` `Entities/GameObject/GameObject.cpp:3795` |
| **Spell** | 24 of 26 `wow-world-spell/src/session/spell_state/*` (`spellbook.rs:39`); 31 `wow-world/src/session/spell_effects/*` (`effect_combat.rs:28`); 8 `wow-world-spell/src/player_cast/wire.rs:85`; 6 `wow-world-spell/src/melee_rules.rs:216` | `PlayerSpellRuntimeState` snapshots taken into `RepresentedPlayerSpellRuntimeLikeCpp` — `crates/wow-world-spell/src/records.rs:155-163`; aura/shapeshift mirrors in `wow-world-spell/src/session/spell_state/*` | The same `PlayerSpellRuntimeState` on the canonical Player, read through `spell_state`/`canonical_access/spell_acquisition.rs` | **Partly** — `records.rs:155` snapshots canonical state, so the mirror is derived; disagreement requires a snapshot held across a mutation | `Player::m_spells`, `Player::m_auraMap`, `Unit::m_shapeshiftForm` on the single Player |
| **Inventory / items** | 59 of 91 `wow-world-inventory/src` (`equipment_slots.rs:25`, `modifiers.rs:21`, `bank.rs:20`, `void_storage.rs:54`, `items.rs:29`); 6 `wow-world-core/src/session/player_items/*` (`valuation.rs:191`) | Inventory/runtime mirrors on the session plus `wow_entities::Player` owned families | `OwnedInventoryAccessLikeCpp` / `player_inventory_runtime_like_cpp` — `crates/wow-world-inventory/src/storage.rs:245-250,510` | **Not established** (`unverified`) — the family migration is per-family and was not traced here | `Player::GetItemByPos`, `Player::m_items` |
| **Progression / stats / talents** | 9 of 13 `crates/wow-world-core/src/session/progression/*` (`talents.rs:125`); 10 `crates/wow-world-core/src/session/player_stat_queries.rs:78`; 1 `crates/wow-world-application/src/stats.rs:45` | Session progression mirrors (honor/XP/talent pending requests) | Canonical Player via `resolved_player_*` (`crates/wow-world-core/src/session/progression_adapters.rs:100-160`) | **Not established** (`unverified`) | `Player::m_xp`, `Player::m_talents` |
| **Instances / difficulty** | 7 of 16 `wow-world-instances` (`difficulty.rs:79`, `fixture_access.rs:21`), 9 gated | Session instance-reset and difficulty mirrors | `InstanceLockMgr` (`wow_instances`) and the canonical map's difficulty | **Not established** (`unverified`) | `sObjectMgr->GetInstanceTemplate`, `Map::GetDifficulty` |
| **Battleground / collections / lifecycle** | 5 of 24 `wow-world-social`, 7 of 11 `wow-world-lifecycle` | `SessionFixtures`/state mirrors in `crates/wow-world-core/src/session/state/battleground.rs:15-52`, `crates/wow-world-core/src/session/state/collections.rs:25-50`, `crates/wow-world-lifecycle/src/state.rs:112-115` | Not traced | **Not established** (`unverified`) | `Battleground`/`CollectionMgr` |

**Reading rule for the table.** Where the represented owner equals the C++ owner (visibility), the
duality is structural. Where the represented owner is the *session* and the C++ owner is the
*object* or the *map* (loot tables, GO use state, tap lists, unique users), it is a behaviour
divergence and is listed in §5.

### 2.1 Exact production-reachable inventory (F6-1)

**Method.** Feature resolution first: `cargo tree -p world-server -e features --edges normal,build
--prefix none` reports **no** `test-fixtures`/`test-support` edge anywhere in the production graph
(`grep -c` = 0), and all 29 workspace lines that enable the feature sit in a `[dev-dependencies]`
section (e.g. `wow-world = { workspace = true, features = ["test-fixtures"] }`,
`crates/world-server/Cargo.toml:42-43`); the production cfg environment is therefore `test =
false` with every `feature = "..."` false. The package set came from `cargo tree -p world-server
--edges normal,build --prefix none --format '{p}'` (41 workspace packages plus the `world-server`
bin). Enumeration is then a build-script-free cfg scan: from each crate root a module-tree walk
follows `mod x;`, `#[path = "..."] mod x;`, inline `mod x { ... }` and file-level `#![cfg(...)]`,
and every `fn represented_*` is recorded with the conjunction of the `#[cfg(...)]` gates of all
enclosing scopes plus its own attribute block; each predicate is parsed (`all`/`any`/`not`/
`feature =`) and evaluated in that environment. Unknown predicates (`unix`, `debug_assertions`)
evaluate true, so a definition is never wrongly reported absent. Cross-checks: the walk found
exactly the `grep -rn 'fn represented_'` count inside every file it visited (1193 = 1193); every
`mod`/`#[path]` target resolved to an existing file (0 unresolved); no production-reachable item
sits under `unit_tests/`, `tests/` or a `f3_shims.rs`/`fixtures.rs` file. No `cargo build`,
`check` or `test` was run, and the walker was a throwaway script in `/tmp` (no repo file added).

| Figure (scope: the whole production package set of `world-server`) | count |
|---|---:|
| `fn represented_*` items reachable from a crate root | 1193 |
| of those, `#[test]` functions (functions whose name merely starts with `represented_`) | 54 |
| **`represented_*` definitions** | **1139** |
| **production-reachable** | **601** |
| gated out of a production build | 538 |
| — by `cfg(test)` | 386 |
| — by `cfg(any(test, feature = "test-fixtures"))` | 152 |
| — own attribute block / enclosing module chain | 293 / 245 |
| distinct production-reachable names | 497 |
| production-reachable whose body reads a canonical token (below) | 187 (180 names) |

The `crates/wow-world*/src` scope of §1/§2 holds 765 of the 1139 definitions: **565
production-reachable**, 200 gated out (152 `test-fixtures` + 48 `cfg(test)`). The other 36
production-reachable definitions are in `wow-entities` 16, `wow-map` 7, `world-server` 3,
`wow-data` 3, `wow-database` 3, `wow-loot` 3, `wow-social` 1.

Per crate, `definitions / production-reachable / gated out` (whole closure):

```
wow-world 618/242/376  wow-world-core 154/100/54  wow-world-application 68/61/7  wow-world-inventory 91/59/32
wow-world-spell 51/41/10  wow-world-entities 25/23/2  wow-world-loot 21/19/2  wow-entities 21/16/5  wow-world-instances 16/7/9
wow-world-lifecycle 11/7/4  wow-world-social 24/5/19  wow-world-visibility 6/1/5  wow-map 7/7/0  wow-data 3/3/0
wow-database 5/3/2  wow-loot 5/3/2  world-server 3/3/0  wow-social 1/1/0  wow-persistence 1/0/1  wow-world-interaction 8/0/8
```

Per bucket, production-reachable and again restricted to `crates/wow-world*/src` so the figures
match §2's rows. Each definition is assigned to the first §2 row whose own path nouns appear in its
path (`loot`; `quest`; `movement`/`taxi`/`transfer`; `visibility`; `entities`/`creature`/
`gameobject`/`map_manager`/`wow-map`; `spell`/`player_cast`/`melee_rules`; `inventory`/
`player_items`/`equipment`/`bank`/`void_storage`/`durability`/`scaling`/`items`; `progression`/
`talents`/`stat_queries`/`stats`; `instances`/`difficulty`; `battleground`/`collection`/
`lifecycle`), remainder `Other` — a bucket §2 does not define:

```
Loot 84  Quest 64  Movement 14  Visibility 3  Entities (creature/GO) 37  Spell 106
Inventory/items 99  Progression/stats/talents 27  Instances/difficulty 13
Battleground/collections/lifecycle 11  Other (no §2 bucket) 107   = 565
```

**Set F6-3 needs.** A production-reachable definition whose body contains an identifier token that
starts with `canonical_` or `resolved_`, is `player_handle_like_cpp`, or ends with
`_snapshot_like_cpp` (body = the tokens between the function's body braces, comments and string
literals excluded by the same lexer). 187 definitions, 180 distinct names; per crate: `wow-world`
72, `wow-world-core` 39, `wow-world-inventory` 28, `wow-world-application` 22, `wow-world-spell` 6,
`wow-world-entities` 5, `wow-world-loot` 5, `wow-world-lifecycle` 4, `wow-world-social` 4,
`wow-world-instances` 2. This is a lexical predicate, not an ownership proof: whether the read
actually reaches the canonical owner is per-name `unverified` until F6-3 classifies it.

```
represented_action_buttons_snapshot_like_cpp represented_active_glyph_aura_source_is_empty_like_cpp represented_active_talent_group_like_cpp represented_area_spirit_healer_access_like_cpp represented_armor_of_stat_percent_like_cpp represented_attack_power_flat_aura_like_cpp
represented_attack_speed_multipliers_like_cpp represented_auto_unequip_offhand_if_need_like_cpp represented_auto_unequip_offhand_reason_like_cpp represented_average_item_level_with_access_like_cpp represented_avg_equipped_item_level_with_access_like_cpp represented_avg_total_item_level_like_cpp
represented_bag_contains_active_item_loot_like_cpp represented_bank_bag_slot_flag_with_access_like_cpp represented_battle_pet_like_cpp represented_battle_pet_query_companion_like_cpp represented_battleground_status_is_wait_leave_like_cpp represented_can_confirm_respec_wipe_like_cpp
represented_can_delay_teleport_like_cpp represented_can_receive_creature_message_to_set_by_guid_with_legacy_fallback_like_cpp represented_can_receive_creature_message_to_set_like_cpp represented_can_see_or_detect_world_creature_like_cpp represented_can_see_spell_click_on_creature_like_cpp represented_can_see_spell_click_on_like_cpp
represented_cast_speed_multiplier_like_cpp represented_championing_faction_for_kill_like_cpp represented_character_pet_aura_source_is_empty_like_cpp represented_condition_projection_like_cpp represented_connected_creature_tappers_like_cpp represented_creature_has_loot_recipient_like_cpp
represented_creature_is_dead_for_loot_visibility_like_cpp represented_creature_kill_reputation_rate_like_cpp represented_creature_loot_group_state_like_cpp represented_creature_position_for_loot_like_cpp represented_current_duel_info_like_cpp represented_current_player_has_incomplete_quest_item_drop_for_item_like_cpp
represented_current_player_has_incomplete_quest_objective_for_item_like_cpp represented_disallowed_mount_form_like_cpp represented_display_power_type_like_cpp represented_duel_opponent_info_like_cpp represented_dungeon_difficulty_packet_like_cpp represented_dungeon_trash_looter_like_cpp
represented_durability_loss_aura_multiplier_like_cpp represented_durability_targets_like_cpp represented_effective_caster_destination_like_cpp represented_empty_inventory_positions_like_cpp represented_equip_spell_fits_shapeshift_with_access_like_cpp represented_equipped_item_in_slot_fits_spell_requirements_like_cpp
represented_expertise_aura_modifier_like_cpp represented_explored_zones_db_string_like_cpp represented_far_teleport_pending_like_cpp represented_flight_speed_rate_like_cpp represented_gameobject_activate_to_quest_like_cpp represented_gameobject_can_interact_with_like_cpp
represented_gameobject_chest_group_state_like_cpp represented_gameobject_gossip_can_interact_with_like_cpp represented_gameobject_loot_install_observation_result_like_cpp represented_gameobject_loot_state_like_cpp represented_gameobject_questgiver_can_interact_with_like_cpp represented_gameobject_use_allowed_by_mover_like_cpp
represented_group_looters_at_reward_distance_like_cpp represented_guild_bank_can_interact_like_cpp represented_handle_spell_click_plan_with_seat_like_cpp represented_has_quest_for_gameobject_like_cpp represented_has_title_like_cpp represented_heirloom_item_set_bonus_over_level_cap_with_access_like_cpp
represented_homebind_like_cpp represented_inventory_descendants_postorder_like_cpp represented_inventory_item_counts_with_access_like_cpp represented_is_on_barber_chair_like_cpp represented_item_bonus_state_for_equipment_set_use_like_cpp represented_item_inventory_type_with_access_like_cpp
represented_item_level_with_access_like_cpp represented_known_spells_like_cpp represented_learn_title_like_cpp represented_legacy_npc_can_interact_with_like_cpp represented_load_cuf_profiles_packet_like_cpp represented_loot_money_recipients_like_cpp
represented_loot_player_context_like_cpp represented_master_loot_candidate_list_like_cpp represented_master_loot_target_eligible_like_cpp represented_master_loot_target_exists_like_cpp represented_melee_armor_mitigation_like_cpp represented_melee_damage_bonus_like_cpp
represented_melee_damage_taken_like_cpp represented_melee_outcome_facts_like_cpp represented_mount_capability_for_type_from_session_like_cpp represented_mount_capability_selection_for_type_like_cpp represented_nearby_entry_destination_like_cpp represented_needed_talent_points_for_learn_like_cpp
represented_next_reset_talents_cost_like_cpp represented_non_bank_item_count_like_cpp represented_notify_loot_item_removed_from_snapshot_like_cpp represented_notify_loot_item_removed_like_cpp represented_notify_loot_list_like_cpp represented_notify_money_removed_like_cpp
represented_npc_can_interact_with_like_cpp represented_object_target_position_like_cpp represented_on_loot_opened_with_catalogs_like_cpp represented_or_canonical_gameobject_owner_guid_like_cpp represented_override_attack_power_by_spell_power_pct_like_cpp represented_owned_loot_authority_like_cpp
represented_pending_quest_sharing_like_cpp represented_pet_position_like_cpp represented_pet_stable_info_by_number_like_cpp represented_player_aura_state_mask_like_cpp represented_player_autoattack_damage_multiplier_like_cpp represented_player_battleground_type_id_or_reject_like_cpp
represented_player_can_use_battleground_object_like_cpp represented_player_charmed_guid_like_cpp represented_player_difficulty_id_for_map_entry_like_cpp represented_player_flags_value_like_cpp represented_player_gear_stats_like_cpp represented_player_gear_stats_with_access_like_cpp
represented_player_group_reward_state_like_cpp represented_player_has_active_vehicle_like_cpp represented_player_has_default_item_entry_like_cpp represented_player_has_flag_like_cpp represented_player_is_polymorphed_like_cpp represented_player_is_same_raid_with_like_cpp
represented_player_mount_liquid_state_like_cpp represented_player_phase_shift_like_cpp represented_player_power_values_like_cpp represented_player_quest_status_like_cpp represented_player_spell_rows_like_cpp represented_player_vote_on_loot_roll_with_generator_like_cpp
represented_player_weapon_proficiency_like_cpp represented_prevent_durability_loss_like_cpp represented_primary_specialization_id_like_cpp represented_query_canonical_pet_name_like_cpp represented_quest_available_conditions_meet_like_cpp represented_quest_giver_status_query_source_like_cpp
represented_quest_login_aura_sources_are_hit_inert_like_cpp represented_ranged_attack_power_flat_aura_like_cpp represented_resistance_aura_flat_like_cpp represented_resistance_aura_multiplier_like_cpp represented_resurrection_requested_by_like_cpp represented_ride_vehicle_interact_like_cpp
represented_run_speed_rate_like_cpp represented_save_cuf_profiles_like_cpp represented_set_action_button_like_cpp represented_set_chosen_title_like_cpp represented_set_difficulty_id_like_cpp represented_set_difficulty_reset_owner_like_cpp
represented_set_taxi_benchmark_mode_like_cpp represented_shapeshift_form_with_fixture_like_cpp represented_spell_area_quest_status_like_cpp represented_spell_base_damage_bonus_done_like_cpp represented_spell_base_healing_bonus_done_like_cpp represented_spell_bonus_coefficient_from_ap_like_cpp
represented_spell_bonus_like_cpp represented_spell_click_creature_snapshot_like_cpp represented_spell_damage_pct_done_like_cpp represented_spell_healing_bonus_done_like_cpp represented_spell_healing_bonus_taken_like_cpp represented_spent_talent_points_count_like_cpp
represented_start_group_loot_rolls_on_first_open_like_cpp represented_talent_reset_cost_like_cpp represented_talent_reset_state_plan_like_cpp represented_talent_reset_time_secs_like_cpp represented_talents_loaded_like_cpp represented_target_can_duel_like_cpp
represented_target_creature_type_mask_like_cpp represented_target_health_pct_like_cpp represented_target_mechanic_mask_like_cpp represented_top_level_item_mod_targets_with_access_like_cpp represented_total_aura_modifier_like_cpp represented_total_aura_multiplier_like_cpp
represented_trait_config_aura_source_is_empty_like_cpp represented_transform_spell_allows_mount_like_cpp represented_transport_destination_world_position_like_cpp represented_unit_aura_state_mask_like_cpp represented_usable_weapon_item_id_like_cpp represented_visibility_source_combat_reach_like_cpp
represented_visibility_source_position_like_cpp represented_weapon_crit_aura_modifier_like_cpp represented_weapon_damage_flat_like_cpp represented_weapon_damage_pct_like_cpp represented_weapon_enchant_damage_like_cpp represented_xp_rest_info_changed_since_like_cpp
```

**`unverified`.** Every figure above comes from the cfg scan, not from a compiler expansion (no
`cargo check` and no `rustc -Zunpretty=expanded` was run), so a `#[cfg]` produced inside a macro
expansion or a `mod`/`#[path]` form outside the four the walker follows would be missed. §2.1
supersedes the heuristic figures in §1 items 1–2, §2's count column, D-11 and §7 rows 1–2.

### 2.2 `resolved_*` absent-owner contract (F6-2)

**Method.** Enumeration: `grep -rn 'fn resolved_' crates/ --include=*.rs` → 216 definition lines; every
signature was read to its `{`/`;` and its return type recorded, then each body was read with
`awk`/`sed`. Caller scan: `grep -rn 'resolved_[a-z_]*(' crates/ --include=*.rs | grep -E
'unwrap_or|unwrap_or_default|unwrap_or_else|\.unwrap\(\)|expect\('`, and every hit was re-read in place.
A site counts as production only when `unit_tests/` and `f3_shims.rs` are absent from its path and its
enclosing item carries neither `cfg(test)` nor `test-fixtures`. "Absent owner" means the session has no
live generation-checked canonical `Player`/entity, i.e. `with_owned_player_like_cpp` /
`player_handle_like_cpp` yield `None`. No Rust was edited and no compiler was run: the `cfg` verdicts
are source reads, not an expansion.

| Figure (scope `crates/`, F6-2 re-derivation) | count |
|---|---:|
| `fn resolved_*` definitions | 216 |
| return `Option<...>` | 210 |
| do not return `Option` — 4 `wow-data` lookup structs (`spell_acquisition/state_2_ops_1.rs:662,761,773,785`), `-> u32` (`login_recovery.rs:16`), `-> Result<_,_>` (`world-server/src/gameobject_loaded_grid.rs:269`) | 6 |
| trait declarations with no body (body lives in an impl elsewhere) | 12 |
| test-only bodies (12 `unit_tests/**/f3_shims.rs` definitions) | 12 |
| **production-reachable `Option`-returning bodies** | **186** |

**Table A — absent-owner behaviour, grouped by body shape.** Every fixture/substitute branch found in
these bodies sits behind `#[cfg(any(test, feature = "test-fixtures"))]` or `#[cfg(test)]`, so in
production the canonical `Option` is returned unchanged — except row D, the only bodies with a
production-compiled substitute (`grep`-checked for `unwrap_or`/`or_else`/`then_some`/`.or(`/`map_or`
outside a `#[cfg(...)]` block).

| # | Shape | n | representative seam | production behaviour when the canonical owner is absent | caller turns `None` into a packet value |
|---|---|---:|---|---|---|
| A | canonical-owner read in the body (`with_owned_player_like_cpp`, `with_owned_player_for_rest_like_cpp`, `player_handle_like_cpp`, `canonical_*`) | 51 | `crates/wow-world-core/src/session/player_vitals_adapter.rs:89` (the F6-2 reference shape) | `None` (the fixture branch is compiled out); `Some` only while the Player handle is live | yes: `resolved_is_in_taxi_flight_like_cpp` (`canonical_access/player_condition.rs:456`), `resolved_known_spells_like_cpp` — table B |
| B | forwards to a session/value accessor (`*_snapshot_like_cpp`, `*_access_like_cpp`) and projects it with `.map` | 63 | `crates/wow-world-core/src/session/player_vitals_adapter.rs:122` → `crates/wow-world-core/src/session/player_presentation.rs:49-66` (canonical read + `test-fixtures` fallback) | same `None` as the accessor; no substitute in the seam | yes: `resolved_player_mounted_like_cpp`, `resolved_player_visible_auras_like_cpp`, `resolved_inventory_item_object_like_cpp`, `resolved_player_inventory_item_object_with_access_like_cpp`, `resolved_inventory_item_objects_like_cpp`, `resolved_player_inventory_slot_count_like_cpp`, `resolved_dungeon_difficulty_id_like_cpp` — table B |
| C | thin forwarder: `split_*_ref` + `state.resolved_*`, a free-function `resolved_*`, or a `_with_access_like_cpp` variant | 62 | `crates/wow-world-core/src/session/player_vitals_adapter.rs:76` → `:89`; `crates/wow-world/src/session/player_items/storage.rs:335` → `crates/wow-world-inventory/src/storage.rs:647` | inherits the callee's row; the body adds no fallback | yes: `resolved_buyback_items_like_cpp` (via `crates/wow-world/src/session/player_items/items.rs:165`), `resolved_player_vitals_like_cpp`, the aura-effect seams — table B |
| D | substitutes a default for an absent **entry**, not an absent owner | 7 | `crates/wow-world-core/src/session/progression/skills.rs:435`, `:559`; `.../canonical_access/quest_reward_owner.rs:130`, `item_sets.rs:126`, `item_enchantment.rs:35`, `quest_eligibility.rs:102`; `crates/wow-world-social/src/guild.rs:110` | `Some(0)` when the skill id is missing from resolved records (`Some(x?.get(&id).unwrap_or(0))`); `None` only if the records accessor itself is `None`; `guild.rs:110` is `authority_complete.then_some(guild_id.unwrap_or(0))` | the fabricated `0` is a value, not a `None`; packet attribution `unverified` |
| E | ownerless catalog lookup | 3 | `crates/wow-data/src/spell_db2/state_1.rs:604`, `state_2.rs:816`; `crates/world-server/src/spell/acquisition_loader.rs:1328` | `None` = no row for that difficulty chain; unrelated to canonical ownership | no packet in the seam; callers not traced |
| F | test-only bodies | 12 | `crates/wow-world/unit_tests/session/**/f3_shims.rs` | not compiled into a production build (§2.1) | n/a |

**Table B — production call sites that turn a `None` into something else (re-read at the site).** The
scan found 67 production sites in 23 distinct seams carrying a conversion idiom inside a six-line call
window; two of them are window artefacts and are *not* substitutions:
`crates/wow-world/src/session/player_cast/checks.rs:22-26` and
`crates/wow-world/src/session/player_items/valuation.rs:19-21` both propagate (`?`) and only carry an
adjacent `.unwrap_or` on another expression.

| seam (definition anchor) | caller | what `None` becomes | packet verdict |
|---|---|---|---|
| `resolved_buyback_items_like_cpp` `crates/wow-world/src/session/player_items/items.rs:165` | `crates/wow-world/src/handlers/character/vendor/sell.rs:375-380` | `ObjectGuid::EMPTY`, pushed into `inv_slot_changes` | **yes** — published by `send_player_values_update_from_entity_bridge` (`sell.rs:381-384`; builder `crates/wow-world/src/session/publication/operations.rs:122-141`). Exact wire field `unverified` |
| `resolved_inventory_item_object_like_cpp` `crates/wow-world/src/session/player_items/storage.rs:339` | `crates/wow-world/src/handlers/spell/ops_1.rs:187-193` | `map_or(true, is_locked)` → treated as locked | **yes** — `send_equip_error(InventoryResult::ItemLocked, …)` → `InventoryChangeFailure` (`crates/wow-world-core/src/session/player_items/equipment.rs:5,16`) |
| same seam | `crates/wow-world/src/handlers/void_storage/transfer.rs:179-185` | `map_or(0, count)` → deposit count 0 | **yes, indirect** — the count feeds the non-bank removal accounting used by `handle_void_storage_transfer_with_generators_like_cpp` |
| same seam | `crates/wow-world/src/handlers/character/items/inventory_moves.rs:101-104` | `ok_or(InventoryResult::ItemNotFound)?` | redirects to the inventory-error result; its send site was not traced |
| `resolved_player_inventory_item_object_with_access_like_cpp` `crates/wow-world-inventory/src/quest_reward.rs:140` | `crates/wow-world-inventory/src/handlers/item_text.rs:34-39` | `QueryItemTextResponse::invalid_like_cpp(query.id)` | **yes** — `send_packet(&response)` in the same function |
| `resolved_is_in_taxi_flight_like_cpp` `crates/wow-world-core/src/session/canonical_access/player_condition.rs:456` | `crates/wow-world-core/src/session/canonical_access/player_condition.rs:441-453` | `true` into `ConditionPlayerSnapshot.is_in_flight` | **yes, indirect** — `wow_conditions` evaluates that field (`crates/wow-conditions/src/evaluate.rs:430`); whether this tree's DB conditions select it is `unverified` |
| `resolved_dungeon_difficulty_id_like_cpp` `crates/wow-world-instances/src/difficulty.rs:239` | `.../difficulty.rs:269-277` (`?`), then `crates/wow-world/src/handlers/character/world_entry.rs:145-147` | the packet builder returns `None` | **yes, as suppression** — `DungeonDifficultySet` and the following `LoginVerifyWorld` (`world_entry.rs:148,156`) are not sent and world entry returns `false` |
| `resolved_player_visible_auras_like_cpp` `crates/wow-world-core/src/session/spell_state/aura_publication.rs:6` (the `HubRef` impl the caller uses; the `PlayerStatsAccessLikeCpp` twin is `canonical_access/player_stats.rs:456`) | `crates/wow-world/src/session/player_presentation.rs:142-144` | `MountCapabilityRejectLikeCpp::Aura` | reject reason, not a packet field; the mount-capability answer's send path is `unverified` |
| `resolved_player_vitals_like_cpp` `crates/wow-world-core/src/session/player_vitals_adapter.rs:76` | `.../spell_effects/effect_combat/healing_application.rs:272-274` | `&'static str` error | **no** — the heal is abandoned, nothing fabricated |
| `resolved_player_mounted_like_cpp` `crates/wow-world-core/src/session/player_vitals_adapter.rs:122` | `crates/wow-world-application/src/aura_removal/initial.rs:20-22` | `&'static str` error | **no** — aura removal aborts |
| `resolved_known_spells_like_cpp` `crates/wow-world/src/session/spell_state/spellbook.rs:562` | `.../spellbook.rs:565-567` (`unwrap_or_default`) | empty `Vec<i32>` | consumed by 20+ `known_spells_like_cpp()` callers; the two read (`spellbook.rs:198-200`, `:321-325`) use it for learn counting and aura-authority invalidation, not a packet field. Packet attribution `unverified` |
| aura seams `resolved_aura_effects_by_spell_aura_type_like_cpp` / `..._with_misc_values_...` / `..._with_spell_and_misc_...` / `..._aura_effect_amounts_...` (`crates/wow-world-core/src/session/spell_state/aura/effect_queries.rs:118,140,159,176`; access-struct twins `crates/wow-world-core/src/session/canonical_access/player_stats.rs:475,490,505,520`) | 45 production sites, e.g. `crates/wow-world-spell/src/session/spell_state/aura.rs:231-233`, `crates/wow-world/src/session/combat/melee.rs:155-159`, `crates/wow-world-inventory/src/stats.rs:43`, `equipment_slots.rs:214-232`, `crates/wow-world-core/src/session/player_stat_queries.rs:23-33` | empty list / `0` / `1.0` | the substitute enters stat, crit, haste and damage arithmetic; packet attribution `unverified` |
| `resolved_inventory_item_objects_like_cpp` `crates/wow-world-inventory/src/storage.rs:618`; `resolved_player_inventory_slot_count_like_cpp` `crates/wow-world-inventory/src/storage_slots.rs:171` | `crates/wow-world-inventory/src/durability.rs:21-22`, `:26-27` | empty target list; slot count `0` | durability targets/filter; packet attribution `unverified` |

**Answer to the F6-2 question.** Of the 186 production-reachable `Option`-returning `resolved_*` seams,
**3 are verified to turn `None` into a packet value** (`resolved_buyback_items_like_cpp`,
`resolved_inventory_item_object_like_cpp`, `resolved_player_inventory_item_object_with_access_like_cpp`),
**1 turns it into a condition-input value** whose packet link is verified only at condition evaluation
(`resolved_is_in_taxi_flight_like_cpp`), and **1 turns it into packet suppression**, taking the rest of
the login sequence with it (`resolved_dungeon_difficulty_id_like_cpp`). The other production seams
either propagate `None` unchanged (rows A–C) or fabricate a default inside the seam (row D).

### 2.2.1 Decision on the absent-owner conversions (reviewer, high effort)

Decision taken by the high-effort reviewer; recorded here so it does not live only in a session log.

**All five conversions are bounded holds (C): behaviour retained pending evidence, expansion
forbidden, no parity claimed.** Rationale for the login case, which is the one that looked
worst: at the equivalent point the reference (`CharacterHandler.cpp:1063`) has **already loaded
a Player**, sends that Player's difficulty and then `LoginVerifyWorld`; a failed `LoadFromDB`
disconnects earlier, so **there is no "unknown owner, continue successfully" path**. Returning
`None` when canonical ownership is unavailable is therefore **correct by design**, and the open
question is narrower and different from what the audit first suggested: whether *this caller*
turns a **temporary** unavailability into a **permanent** login failure. Neither unconditional
continuation (A) nor permanent acceptance (B) is justified yet.

Per case: buyback — an empty slot legitimately uses `EMPTY`, and unknown ownership does not
prove emptiness; inventory lock — genuinely locked items legitimately produce `ItemLocked`;
item text — an invalid response for a genuinely missing item already matches
`HandleItemTextQuery`; taxi — unknown flight state is not flight, and `true` can *satisfy* a
positive Taxi condition, so it is not a universally conservative rejection.

**Closing evidence (needs runtime authority, not granted here):** paired real 3.4.3
reference/Rust captures of a successful login with known difficulty and of a failed load,
including both connections, packet bytes and order, termination, and correlated Rust
owner-generation/readiness traces. Temporary unresolved ownership suppressing a valid login
selects repair of readiness/continuation; invalid ownership supports retaining rejection.

**Implementation:** no code change. Structural slices may preserve these branches; any changed
observable behaviour needs its own scoped slice with regression coverage.

## 3. `legacy_runtime`

**What it is.** A private module of `wow-world::session` holding the creature/player runtime tick
bodies: 18 files, ~7,070 lines, listed in `crates/wow-world/src/session/legacy_runtime/mod.rs:6-19`.
Module header: *"Legacy creature and player runtime tick, separated from the Session root. Each
submodule owns one complete tick responsibility"* (`legacy_runtime/mod.rs:1-4`). Every submodule
header repeats *"Behaviour is preserved; the canonical owner of this state is unchanged"*
(`creature_tick.rs:1-4`, `creature_aggro_tick.rs:1-4`, `creature_lifecycle_tick.rs:1-4`,
`creature_melee_tick.rs:1-4`, `creature_movement_tick.rs:1-4`, `creature_spell_tick.rs:1-4`,
`player_tick.rs:1-4`).

**Sibling legacy-runtime shims in the same family:**

| Surface | Anchor | Note |
|---|---|---|
| Private module + glob re-export | `crates/wow-world/src/session/mod.rs:42,47-49` | comment explains the external path `wow_world::session::run_legacy_*` is preserved |
| Seven public tick entry points | `legacy_runtime/mod.rs:50-56` | "only these seven are public" |
| `current_legacy_runtime_map_key_like_cpp` | `crates/wow-world-core/src/session/instances/map_key.rs:53-62` | represented map key resolution, see D-03 |
| `legacy_creature_*` config/aggro contracts | `crates/wow-world-core/src/session/creature_aggro_contracts.rs` (referenced from `legacy_runtime/*`) | session config, not runtime state |
| `rebind_legacy_creature_loot_authority_like_cpp` | `crates/wow-world-core/src/session/loot/operations.rs:44-78` | writes the legacy store, see D-01 |

**Who calls it.** Only `world-server`, through the `wow_world::session::run_legacy_*` re-exports:

| Caller | Anchor |
|---|---|
| Tick fanout | `crates/world-server/src/runtime/delivery.rs:1575-1668` (`run_legacy_creature_runtime_tick_with_input_and_deliver_once_like_cpp`) |
| Loop task | `crates/world-server/src/runtime/delivery.rs:1777-1795` (`spawn_legacy_creature_runtime_update_loop_like_cpp`) |
| Spawn site | `crates/world-server/src/app.rs:5365-5378` |
| Shutdown join | `crates/world-server/src/app.rs:5611-5623` (`"legacy-creature-runtime"` producer) |

**Is it dead, transitional or load-bearing?** **Load-bearing, and enabled by default.**

- `legacy_creature_global_runtime_enabled_from_config_like_cpp()` returns
  `.unwrap_or(true)` — default **on** — `crates/world-server/src/bootstrap/config.rs:152-156`.
- When on, startup flips the legacy map manager to `RuntimeTickOwner::GlobalLegacy` and logs
  *"legacy creature tick owner set to GlobalLegacy"* — `crates/world-server/src/app.rs:5319-5332`.
- The enum documents this as the production default: *"Production startup flips this to
  `GlobalLegacy` by default so a global map clock drives creature runtime like C++ and
  session-level creature ticks are skipped to avoid double resolution"* —
  `crates/wow-world-core/src/map_manager/runtime_state.rs:196-210`.
- `player_tick.rs:8-20` states the reason it is load-bearing: *"Every logged-in session used to
  run it on its own clock and write shared legacy creature state ungated, so with N players there
  were N+1 concurrent writers of that state."*

**C++ equivalent.** There is no separate C++ "legacy runtime". C++ runs one path:
`World::Update` → `sMapMgr->Update(diff)` (`src/server/game/World/World.cpp:2748`) →
`MapManager::Update` (`src/server/game/Maps/MapManager.cpp:287`) → per-map `Map::Update` →
`Creature::Update` (`src/server/game/Entities/Creature/Creature.cpp:696`), plus
`Player::Update`/`DoMeleeAttackIfReady` on the same clock. Rust instead runs **two production
producer tasks**: `spawn_canonical_map_update_loop` (`crates/world-server/src/app.rs:5345`) and
`spawn_legacy_creature_runtime_update_loop_like_cpp` (`crates/world-server/src/app.rs:5365`).

**Is any of it a behaviour difference rather than a structural artifact?** **Yes — see D-08.**
The tick-owner flag itself (`RUSTYCORE_LEGACY_CREATURE_GLOBAL_RUNTIME`) has no C++ counterpart
(`grep -rn 'LegacyCreatureGlobalRuntime'` over `src/` and `sql/` in the C++ checkout: no match);
the Rust-only key is declared at `crates/world-server/src/lib.rs:182` and read at
`crates/world-server/src/bootstrap/config.rs:153`. On top of that, the tick bodies hold canonical
and legacy guards in an order that only exists because two managers exist
(`player_tick.rs:14-20`: "the execute phase takes canonical then legacy").

## 4. Legacy `map_manager`

**What it is.** `wow-world-core::map_manager`, a global store of `WorldCreature` (canonical
`wow_entities::Creature` **plus** a `CreatureCreateData` packet bridge and a legacy move spline):
module doc *"Legacy map manager and its creature runtime"* — `crates/wow-world-core/src/map_manager/mod.rs:7-11`;
`WorldCreature` at `crates/wow-world-core/src/map_manager/mod.rs:71-90`; 22 files.

**Callers.** It is re-exported from `wow-world` (`crates/wow-world/src/lib.rs:23,50`) and injected
into every session (`crates/world-server/src/session_factory.rs:414`). Within `wow-world*` there
are 128 non-test references to `map_manager` outside the canonical one
(`grep -rn '\.map_manager|map_manager:' crates/wow-world/src crates/wow-world-core/src`, excluding
`unit_tests` and `canonical_map_manager`; the count covers only those two crates); representative
anchors:

| Caller | Anchor |
|---|---|
| Session field | `crates/wow-world-core/src/session/state/session_core.rs:123-125` |
| Creature read/mutate | `crates/wow-world-core/src/session/world_entities/creature_registry.rs:30-59` |
| Legacy loot authority read/rebind | `crates/wow-world-core/src/session/canonical_access/loot_release/authority.rs:228-240`; `crates/wow-world-core/src/session/loot/operations.rs:44-78` |
| Legacy runtime ticks | `crates/wow-world/src/session/legacy_runtime/creature_movement_tick.rs:363-386`, `creature_spell_tick.rs:15-59`, `creature_lifecycle_tick.rs:21-61`, `creature_aggro_tick.rs:258-275`, `creature_melee_tick.rs:179-190`, `player_tick.rs:25-37` |
| Tick-owner decision | `crates/wow-world-core/src/map_manager/runtime_state.rs:360` (`shared_runtime_tick_owner_like_cpp`) |

**C++ counterpart — verified, not assumed.** The reference uses **`MapManager` (map ownership)
plus `sObjectMgr` (static data)**; it has no second mutable creature store. Verified anchors:
`MapManager::Update` `src/server/game/Maps/MapManager.cpp:287`; `sMapMgr->Update` call site
`src/server/game/World/World.cpp:2748`. There is no C++ "legacy map manager" to name: the Rust
`map_manager` is a Rust-side transitional store, and the legacy runtime is its only production
clock.

**Dead, transitional or load-bearing?** **Load-bearing while
`RUSTYCORE_LEGACY_CREATURE_GLOBAL_RUNTIME` is non-zero (default: on)** — §3. It is transitional in
intent (`runtime_state.rs:196-210` calls the `Session` owner the "test/local default") but
reachable and enabled in production today.

## 5. Recorded divergences

Disposition values: `unknown` (not enough evidence to classify), `intentional-departure contract
needed`, `repair needed outside the refactor`. **No row is authorised for repair by this audit.**

| id | Area | Rust anchor | C++ anchor | What differs | Behaviour or structural? | Disposition |
|---|---|---|---|---|---|---|
| D-01 | Loot authority duplication | `crates/wow-world-core/src/session/canonical_access/loot_release/authority.rs:41-176`; `crates/wow-world-core/src/session/creature_canonical_adapter.rs:178-210` | `Creature.h:236` (`std::unique_ptr<Loot> m_loot`), `GameObject.h:322`, `Item.h:249` — one `Loot` per object | Two independently mutable `OwnedLootAuthority` mirrors (legacy `WorldCreature` + canonical map creature) are reconciled by an 8-iteration compare/exchange; `(Active, Active)` collapses to a retired tombstone; sustained contention returns `None` (the request fails) — `authority.rs:150-152` | **Behaviour** — C++ cannot fail or fabricate a tombstone; it mutates the one object | `repair needed outside the refactor` (needs an explicit contract first) |
| D-02 | Creature entity dual authority | `crates/wow-world-core/src/session/creature_canonical_adapter.rs:29-70`; `crates/wow-world-core/src/session/world_entities/creature_registry.rs:30-59` | `Creature::Update` `Entities/Creature/Creature.cpp:696`; `Object.h:604` (`GetMap`) | A legacy mutation is applied to `WorldCreature`, cloned, then pushed to the canonical map; the push can be **rejected** by a health-state-revision/ABA guard, so the mutation stays visible only in the legacy store | **Behaviour** — C++ has one object and no rejection path | `repair needed outside the refactor` |
| D-03 | Map identity fallback | `crates/wow-world-core/src/session/instances/map_key.rs:53-62` | `Object.h:604` `Map* GetMap() const { ASSERT(m_currMap); return m_currMap; }` | When no canonical player map key resolves, the "legacy runtime map key" silently becomes the represented `current_map_id` with `instance_id = 0`; canonical lookups then use that key (`authority.rs:60-64`, `loot/operations.rs:50`, `world_entities/creature_registry.rs:8`, `creature_query.rs:38`) | **Behaviour** — C++ `GetMap()` is authoritative and asserts instead of falling back to `instance 0` | `repair needed outside the refactor` |
| D-04 | Loot-list delivery identity and fanout | `crates/wow-world-core/src/session/canonical_access/loot_release.rs:109-142` (map_id represented, instance_id canonical, owner skipped); caller split `crates/wow-world-application/src/loot_release/mod.rs:318-321` | `Loot::NotifyLootList` `src/server/game/Loot/Loot.cpp:637-655` — one loop over all `_allowedLooters` via `ObjectAccessor::GetPlayer(map, guid)` | The Rust delivery uses `player_map_id_like_cpp()` for the map and the canonical key only for the instance, and the recipient filter is `registry.loot_delivery_recipient(...)`; the owner's copy is sent on a separate path before the others | **Behaviour** (mixed map identity can select the wrong instance's recipient). The send *order* claim is `unverified`: C++ `_allowedLooters` is `GuidUnorderedSet` (`Loot.h:350`), so no C++ order is defined | `intentional-departure contract needed` |
| D-05 | GameObject per-viewer state owner | `crates/wow-world-entities/src/state.rs:40-42` (session-local `BTreeMap`); reader `crates/wow-world-application/src/quest/visibility/gameobject_flags.rs:28-40` | `GameObject.h:512` (`m_perPlayerState`), `GameObject.cpp:3795-3803` (`GetGoStateFor` falls back to the object's `GetGoState()`), created at `GameObject.cpp:1734` | C++ keeps per-viewer GO state **on the object**; Rust keeps it per **session**. The Rust fallback is `state.go_state.unwrap_or(GoState::Ready)` — a session-local mirror — where C++ falls back to the object's current `m_goState` | **Behaviour** (cross-session visibility of a chest already opened by another player; fallback value) | `repair needed outside the refactor` |
| D-06 | GameObject tap list / unique users owner | `crates/wow-world-loot/src/state.rs:77-84` (`represented_unique_gameobject_uses`, `represented_gameobject_tap_lists`); readers `state/cache_access.rs:119-123`, `state/gameobject.rs:12` | `GameObject.h:464` (`m_unique_users`), `GameObject.h:483` (`m_tapList`); populated `GameObject.cpp:1708`, read `GameObject.cpp:2617,3005-3006,3893` | C++ `m_unique_users`/`m_tapList` are object-owned and read cross-session (e.g. `GameObject.cpp:2617` tests whether *this* player has used it; `3005-3006` picks a random prior user) | **Behaviour** | `repair needed outside the refactor` |
| D-07 | GameObject phase shift owner | `crates/wow-world-entities/src/state.rs:54` (`represented_gameobject_phase_shifts`) | `GameObject::GetPhaseShift` (object-owned phase shift; see `GameObject.cpp:2166-2167` per-player state precedent) | Phase shift for a GO is mirrored per session rather than read from the object | **Behaviour** (the comment itself says "until canonical gameobject map ownership lands") | `intentional-departure contract needed` |
| D-08 | Two production tick owners | `crates/world-server/src/app.rs:5345` (`spawn_canonical_map_update_loop`) + `crates/world-server/src/app.rs:5365` (`spawn_legacy_creature_runtime_update_loop_like_cpp`), gated by `bootstrap/config.rs:152-156` (default `true`) and `app.rs:5319-5332` | `World.cpp:2748` → `MapManager.cpp:287` → `Creature.cpp:696` (one clock, one map, one entity) | Rust runs the creature/melee/aggro/lifecycle/spell runtime on a second producer over a second entity store; C++ has exactly one update path and no such config key | **Behaviour** (timing, phase coupling and double-resolution surface) | `repair needed outside the refactor` |
| D-09 | Two map/entity stores | `crates/wow-world-core/src/session/state/session_core.rs:123-128`; injected `crates/world-server/src/session_factory.rs:414-415` | `MapManager.cpp:287`; single map set under `sMapMgr` | Every session holds both a legacy and a canonical manager; which one a reader observes depends on the path taken | **Behaviour** — the same operation can read different creature state | `repair needed outside the refactor` |
| D-10 | Canonical-object lookup order differs between two loot paths | Prefer-canonical: `canonical_access/loot_release/authority.rs:156-162`; prefer-represented: `crates/wow-world-loot/src/state/authority_access.rs:28-34` | `Loot` is reached from the owning object on its own map (`LootHandler.cpp:270`) | For the same gameobject/creature, one path resolves the map from `current_canonical_player_map_key_like_cpp()`, the other from `canonical_object_lookup_map_key_like_cpp(player_map_id_like_cpp())`; with a detached player the two can select different maps | **Behaviour** | `unknown` — the reachable difference needs a detached-player reproduction before classification |
| D-11 | `represented_*` that are test-only | 200 of the 765 `crates/wow-world*/src` definitions are absent from a production build: 152 behind `cfg(any(test, feature = "test-fixtures"))`, 48 behind `cfg(test)` (exact cfg-resolved counts, §2.1; the earlier 222/±8-line figure was a heuristic); e.g. `crates/wow-world-core/src/session/canonical_access/xp_gain/rest.rs:107` | n/a | Tests exercise represented mirrors and their fallbacks that **do not exist in the production binary**; passing unit tests therefore prove a path production does not take | **Structural** (build-configuration divergence, not a C++ behaviour divergence) | `intentional-departure contract needed` — the F6 and #153 audits must not treat these tests as parity evidence |
| D-12 | Owner-first loot-list send ordering | `crates/wow-world-application/src/loot_release/mod.rs:318-321` | `Loot.cpp:637-655`, container `Loot.h:350` (`GuidUnorderedSet`) | The Rust split sends the owner first and the other allowed looters afterwards | **Structural** — C++ order is unspecified for an unordered set, so no divergence is provable | `unknown` (order claim `unverified`) |
| D-13 | `canonical_access` facade rename | `crates/wow-world-core/src/session/canonical_access/mod.rs:1-76` (facade structs); `canonical_access/operations.rs:182` reads canonical | n/a | The facade re-partitions borrows without changing the reads; the name `represented_*` survives on 19 functions there, 2 of which read the canonical Player | **Structural** | none — no behaviour work |

**Count: 13 divergences recorded. 9 are behaviour differences** (D-01…D-09), **1 is a behaviour
difference candidate pending reproduction** (D-10), **2 are structural** (D-11, D-12), **1 is
structural with no action** (D-13). Behaviour-difference total including D-10 as a candidate:
**10 of 13**.

## 6. Concrete work items for the F6 slices

Ordered by risk, each sized to one bounded slice and stated with the evidence that bounds it.
**Nothing here authorises a gameplay repair inside a structural slice.** Of these slices only
F6-1 and F6-2 have been executed, and both only as documentation: §2.1 and §2.2 of this document.

| # | Slice | Bounded by | Risk |
|---|---|---|---|
| F6-1 | **Inventory of the production-reachable `represented_*` surface.** Produce the exact list of `represented_*` definitions compiled into a production build of `world-server` (not the heuristic in §1), grouped by the stage-2 buckets. **Executed — §2.1.** | Purely mechanical: `cargo check` feature resolution or a build-script-free `cfg` scan. No behaviour touched. `crates/wow-world/Cargo.toml:9-11` and `crates/world-server/Cargo.toml:10-43` bound what can be included. | lowest |
| F6-2 | **Re-derive each `resolved_*` seam's absent-owner contract.** For every `resolved_*` that returns `Option`, record what production does when the canonical owner is absent, and whether any caller turns `None` into a packet value. **Executed — §2.2.** | `crates/wow-world-core/src/session/player_vitals_adapter.rs:89-113` is the reference shape; the set of `resolved_*` names is enumerable by grep (216 definition lines, §2.2). Read-only; no code change. | low |
| F6-3 | **Name/source reconciliation for `represented_*` that read canonical.** Split the 187 production-reachable definitions whose body reads a canonical token (180 distinct names, §2.1) into "rename" and "genuine mirror", starting with `canonical_access/operations.rs:182` and `canonical_access/quest_reward_owner.rs:561`. | Bounded by a fixed candidate list; pure rename if the F6-1 inventory confirms no owner change. | low |
| F6-4 | **D-03 map-key fallback removal analysis.** Enumerate every `current_legacy_runtime_map_key_like_cpp` caller (grep: 20 sites incl. `wow-world-loot`, `wow-world-core`, `wow-world-entities`, `wow-world` tests) and state, per caller, what C++ reads there. | `crates/wow-world-core/src/session/instances/map_key.rs:53-62`; caller list is closed and grep-verifiable. Read-only. | medium |
| F6-5 | **D-12/D-04 loot fanout identity and order.** Verify from source whether `loot_delivery_recipient(_, map_id, instance_id)` can select a recipient on a different instance than the canonical one, and whether C++ order is observable at all (container is `GuidUnorderedSet`, `Loot.h:350`). | Two files plus one container type; the D-04 order claim is already flagged `unverified`. | medium |
| F6-6 | **D-05/D-06/D-07 session-local GameObject state.** Decide, per field (`world-entities/src/state.rs:40-42,54`; `wow-world-loot/src/state.rs:77-84`), whether the canonical `GameObject` already carries the equivalent before any code moves. | Field-by-field; each decision is evidence-checkable against `GameObject.h:464,483,512` and `GameObject.cpp:3795`. | medium |
| F6-7 | **D-01/D-02/D-10 loot-authority and creature-entity reconciliation contract.** Write the explicit contract for the mirror reconciliation (including the fail-closed `None` and the retired tombstone) or classify it as an intentional departure. | `authority.rs:41-176` and `creature_canonical_adapter.rs:29-70,178-210` are the whole surface; the C++ side is one `unique_ptr<Loot>` per object (`Creature.h:236`). | high |
| F6-8 | **D-08/D-09 legacy runtime and map manager retirement plan.** Decide the single owner, state the C++ phase order it must reproduce (`World.cpp:2748` → `MapManager.cpp:287` → `Creature.cpp:696`), and list the two-producer and two-store call sites to retire. | Bounded by §3 and §4 caller tables. **Not a structural slice** — it changes which clock and which store produce observable state. | highest |

## 7. Explicitly unverified

| Item | Why |
|---|---|
| The exact production/test split of the 765 `represented_*` definitions | Resolved by F6-1 (§2.1): 565 production-reachable, 200 gated (152 `test-fixtures` + 48 `cfg(test)`). The residual uncertainty is that §2.1 is a cfg scan, not a compiler expansion. |
| The "body reads canonical" figure | Resolved by F6-1 (§2.1): 187 definitions / 180 names, by the stated token predicate. Per-name ownership is still `unverified` (lexical predicate, not a semantic proof). |
| C++ `_allowedLooters` send order (D-12) | Container is `GuidUnorderedSet` (`Loot.h:350`), so no C++ order is defined; Rust's owner-first split cannot be shown divergent from source alone. |
| Inventory/items, progression, instances, battleground, collections, lifecycle duality | Not traced in this audit ("Not established" in §2). |
| Whether the owner's loot-list delivery path reproduces the C++ packet for the owner in every case | The owner branch was located (`loot_release/mod.rs:318-320`) but its packet content was not compared byte-for-byte with `Loot::NotifyLootList`. |
| Whether any `resolved_*` `None` currently reaches a client-visible packet field | Resolved by F6-2 (§2.2): 3 seams turn `None` into a packet value, 1 into a condition input, 1 into packet suppression; the table-B rows marked `unverified` remain open. |
| The canonical shape of every accessor behind §2.2 group B | Only three were read (`player_presentation.rs:49-66`, `rest_progression.rs:165-170`, `spell_hit_authority.rs:57-70`); the other 60 group-B bodies are assigned by body shape, not per-accessor reads. |
| Site-level classification of the 67 production caller hits | The caller scan used a six-line window, which can capture an adjacent expression (2 known artefacts, §2.2); only the table-B rows were re-read at the site. |
| Whether the substituted values in §2.2 table B (aura/stat arithmetic, skill `0`, `known_spells` empty set, durability count `0`) change a packet field | The substitution sites were read; the arithmetic-to-wire path was not traced in this slice. |
| The C++ counterpart of `RUSTYCORE_LEGACY_CREATURE_GLOBAL_RUNTIME` | Verified absent from `src/` and `sql/` in the C++ checkout; absence of a match is not proof that no equivalent mechanism exists elsewhere. |
| Runtime reachability of D-10 | Requires a detached-player or far-teleport reproduction that this read-only audit could not run. |
