#!/usr/bin/env python3
"""#1241 F2: group the WorldSession fields into domain sub-states (move-only).

`WorldSession` becomes `core: SessionCore` plus one field per domain. Every
access gains exactly one path segment (`self.loot_table` ->
`self.loot.loot_table`); nothing is renamed, no visibility of a moved field
changes and no logic moves. Existing sub-states that are domain homes
(`lifecycle`, `phase`, `spell_state`, `social`, `quest_state`, `view`) stay
top-level and absorb their new members; the infra sub-states (`transport`,
`admission`, `flags`, `driver`, `directory`, `account_state`, `realm_policy`)
nest inside `SessionCore`. Groups whose members are all `#[cfg(test)]` are
gated as a whole.

Declaration order is drop order. The top-level order `core`, `lifecycle`,
`phase` keeps every side-effecting pair (command rails, `directory`,
`transport` < `lifecycle` < `phase`) in its pre-F2 relative order, and every
member keeps its original relative order inside its group.

Step 1 (`apply`, text): writes the sub-state structs, the new `WorldSession`
body and the nested construction literal. Step 2 (`apply`, compiler-guided):
runs `cargo check -p wow-world --all-targets --message-format=json` and, for
E0609/E0615 whose message names `WorldSession`, inserts the owning group
segment before the reported field ident. rustc's E0615 suggestion (calling the
method of the same name) is never applied. A second `apply` is a no-op.
Standard library only.
"""
from __future__ import annotations

import argparse
import json
import os
import pathlib
import re
import subprocess
import sys
import textwrap

REPO = pathlib.Path(__file__).resolve().parents[2]
CRATE = REPO / "crates/wow-world"
SESSION = CRATE / "src/session"
STATE = SESSION / "state.rs"
STATE_DIR = SESSION / "state"
CONSTRUCTION = SESSION / "construction.rs"
IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
HEADER = (
    "// Copyright (c) 2026 alseif0x\n"
    "// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html\n\n"
)
# Any visibility: the wrapper is `pub(crate)` since the F2 narrowing.
MARKER = re.compile(r"^    pub(?:\([^)]*\))? core: SessionCore,$", re.M)
DROP_ORDER_NOTE = """\
// Declaration order is drop order (#1241 F2). These side-effecting members must
// keep this relative order: `core.session_command_tx`/`session_command_rx`
// (command rail peers observe the close), `core.directory` (quest-complete
// sender), `core.transport` (socket send/receive channels and write fences),
// then `lifecycle` (battle-pet attachment release, rename task aborts, homebind
// sender, live-character claim, loot persistence trackers), then `phase` (the
// producer observes the phase rail closing). Shared `Arc` handles in `core` and
// the loot authorities in `loot` only become observable if this session is the
// last owner, which production composition never makes it.
"""

# (field, struct, file stem or None for an existing sub-state, doc, members).
# Members of an existing sub-state are the fields it absorbs. Order is the
# WorldSession declaration order.
GROUPS = (
    ('core', 'SessionCore', 'session_core', 'Hub state every domain reads: account and realm identity, the session state and command rails, the selected-player binding, the map/registry/instance handles, the id generators and module registry, and the nested transport, admission, driver, directory, account and realm sub-states.', """
        account_id account_name security expansion account_expansion build locale realm_id state
        session_command_tx session_command_rx durable_creature_runtime_commands_like_cpp
        player_guid player_handle_like_cpp player_identity_bootstrap_like_cpp
        player_bootstrap_attached_like_cpp canonical_map_manager map_manager player_registry
        instance_lock_mgr mmap_pathfinder_like_cpp current_map_id client_visible_guids_like_cpp
        guid_generator item_guid_generator_like_cpp equipment_set_guid_generator_like_cpp
        void_storage_item_id_generator_like_cpp module_registry_like_cpp
        driver_phase_trace_like_cpp transport admission flags driver directory account_state
        realm_policy
    """.split()),
    ('lifecycle', 'SessionLifecycleState', None, None, """
        player_flags_test_fixture_like_cpp loot_money_persistence_test_result_like_cpp
        represented_at_login_flags_like_cpp represented_at_login_flag_removals_like_cpp
        loaded_player_customizations_like_cpp
    """.split()),
    ('phase', 'SessionPhaseRail', None, None, """

    """.split()),
    ('loot', 'LootState', 'loot', 'Loot windows and AE-loot views with their authorities and generations, loot rolls, personal loot money and the loot test hooks.', """
        loot_table represented_loot_cache_generations_like_cpp active_loot_guid
        active_loot_view_owners active_loot_view_generations_like_cpp
        active_loot_view_authorities_like_cpp represented_loot_rolls
        represented_loot_roll_criteria_events pass_on_group_loot loot_specialization_id
        represented_personal_loot_money represented_personal_loot_owners
        represented_gameobject_tap_lists represented_unique_gameobject_uses
        represented_locked_dungeon_encounters loot_item_store_test_grants_like_cpp
        loot_item_store_test_success_like_cpp loot_item_store_test_commit_gate_like_cpp
    """.split()),
    ('catalogs', 'SessionCatalogs', 'catalogs', 'Immutable catalog bundles and DB2/world-DB store handles injected at composition; read-only after construction.', """
        items spell_catalogs chr maps factions creatures gameobjects quests
        trainer_store_like_cpp bank_bag_slot_prices_store currency_types_store
        import_price_stores emotes_store emotes_text_store item_class_store
        item_currency_cost_store trinity_string_store heirloom_store toy_store
        combat_ratings_game_table regen_game_tables shield_block_regular_game_table
        transmog_set_item_store item_price_base_store player_stats pvp_item_store
        durability_costs_store durability_quality_store
        item_template_addon_quest_log_item_ids_like_cpp rand_prop_points_store
        item_disenchant_loot_store loot_stores condition_store player_condition_store
        adventure_map_poi_store content_tuning_store curve_store curve_point_store
        scaling_stat_distribution_store scaling_stat_values_store disable_mgr difficulty_store
        lock_store gem_properties_store tact_key_store skill_store trait_definition_store
        trait_tree_skill_line_index skill_line_store skill_tiers_store area_table_store
        fishing_base_skill_store area_trigger_db2_store area_trigger_store
        area_trigger_script_store graveyard_store dungeon_encounter_store
        world_safe_loc_store_like_cpp access_requirement_store lfg_dungeons_store
        lfg_dungeon_store_like_cpp battlemaster_list_store friendship_rep_reaction_store
        paragon_reputation_store reputation_reward_rate_store
        reputation_spillover_template_store creature_equipment_store_like_cpp
        creature_addon_store_like_cpp creature_difficulty_store_like_cpp
        creature_base_stats_store_like_cpp mount_store mount_definition_store_like_cpp
        mount_capability_store mount_type_x_capability_store mount_x_display_store vehicle_store
        vehicle_seat_store vehicle_template_store vehicle_accessory_store terrain_swap_store
        phase_store phase_group_store talent_store num_talents_at_level_store power_type_store
        cinematic_sequences_store movie_store script_name_interner object_mgr_catalogs_like_cpp
        gameobject_template_lifecycle_store_like_cpp quest_poi_store_like_cpp player_xp_table
        exploration_base_xp_store tavern_area_trigger_store waypoint_path_resolver_like_cpp
        player_bootstrap_catalog_test_fixture_like_cpp
    """.split()),
    ('config', 'SessionWorldConfig', 'config', 'Immutable world configuration and rate values (C++ `sWorld` config subsets) and the script dispatchers injected at composition.', """
        legacy_creature_aggro_config_like_cpp characters_per_realm_like_cpp
        declined_names_used_like_cpp feature_system_bpay_store_enabled_like_cpp
        feature_system_character_undelete_enabled_like_cpp max_player_level_config_like_cpp
        max_primary_trade_skills_like_cpp represented_cast_unstuck_enabled_like_cpp
        loot_drop_rates reputation_rates repair_cost_rate_like_cpp
        durability_loss_on_death_rate_like_cpp stats_limits_like_cpp reset_schedule_like_cpp
        vmap_indoor_check_like_cpp enable_ae_loot_like_cpp mmap_runtime_config_like_cpp
        exploration_xp_rate_like_cpp min_discovered_scaled_xp_ratio_like_cpp
        creature_health_rates_like_cpp addon_channel_like_cpp
        chat_fake_message_preventing_like_cpp party_raid_warnings_like_cpp
        allow_gm_group_like_cpp allow_two_side_interaction_group_like_cpp
        party_level_req_like_cpp chat_strict_link_checking_kick_like_cpp
        chat_level_requirements_like_cpp chat_listen_ranges_like_cpp chat_flood_config_like_cpp
        area_trigger_script_dispatcher_like_cpp give_player_xp_script_dispatcher_like_cpp
    """.split()),
    ('identity', 'PlayerIdentityState', 'identity', 'Player identity fixtures: race, class, level, gender, name, create mode, faction template, scale and zone/area state.', """
        player_race player_class player_level player_gender player_name
        player_create_mode_like_cpp player_faction_template_like_cpp
        player_scale_duration_like_cpp player_zone_id_like_cpp player_area_id_like_cpp
        player_zone_area_authority_complete_like_cpp represented_is_outdoors_like_cpp
    """.split()),
    ('inventory', 'InventoryState', 'inventory', 'Player items, bank and equipment sets, money and currencies, and the represented bank, guild-bank and auction request sinks.', """
        player_item_test_fixture_like_cpp inventory_item_objects player_gold player_currencies
        player_equipment_inventory_authority_complete_like_cpp
        represented_bank_bag_slot_flags_like_cpp represented_bank_item_moves_like_cpp
        represented_guild_bank_inventory_moves_like_cpp
        represented_guild_bank_list_requests_like_cpp
        represented_guild_bank_money_moves_like_cpp represented_guild_bank_tab_actions_like_cpp
        represented_guild_repair_bank_state_like_cpp
        represented_guild_repair_bank_withdraws_like_cpp
        represented_auction_replicate_requests_like_cpp represented_auction_place_bids_like_cpp
        represented_auction_remove_items_like_cpp represented_auction_sell_items_like_cpp
        represented_auto_unequip_offhand_requests_like_cpp represented_equipment_sets_like_cpp
        represented_void_storage_items_like_cpp represented_void_storage_loaded_like_cpp
        represented_using_pvp_item_levels_like_cpp represented_transmog_criteria_events
    """.split()),
    ('collections', 'CollectionsState', 'collections', 'Account collections: mounts, heirlooms, toys, item appearances, transmog illusions and completed achievements.', """
        account_mounts_like_cpp represented_account_heirlooms_like_cpp
        represented_account_toys_like_cpp represented_item_appearances_like_cpp
        represented_item_appearance_blocks_like_cpp
        represented_temporary_item_appearances_like_cpp
        represented_favorite_item_appearances_like_cpp represented_transmog_illusions_like_cpp
        represented_completed_achievements_like_cpp
    """.split()),
    ('spell_state', 'SessionSpellState', None, None, """
        active_spell_cast represented_pending_spell_cast_request_like_cpp last_spell_cast_time
        last_spell_cast_time_per_spell represented_character_spell_cooldowns_like_cpp
        represented_character_spell_cooldowns_loaded_like_cpp
        represented_character_spell_charges_like_cpp
        represented_character_spell_charges_loaded_like_cpp
        represented_spell_history_packets_like_cpp represented_override_spells_like_cpp
        represented_override_spells_complete_like_cpp represented_self_res_spells_like_cpp
        player_spell_test_fixture_like_cpp
        represented_spell_acquisition_post_commit_actions_like_cpp
    """.split()),
    ('auras', 'AuraState', 'auras', 'Player aura fixtures: visible auras, aura authority, the spell-hit tombstone, threat-aura snapshots and the shapeshift form.', """
        visible_auras player_aura_authority_complete_like_cpp
        player_spell_hit_aura_authority_tombstoned_like_cpp
        canonical_threat_aura_snapshots_like_cpp represented_shapeshift_form_like_cpp
    """.split()),
    ('progression', 'ProgressionState', 'progression', 'XP, talents, glyphs and respec, skills and proficiencies, reputation and rest fixtures.', """
        player_xp player_next_level_xp player_character_points_like_cpp
        represented_talent_reset_cost_like_cpp represented_talent_reset_time_secs_like_cpp
        represented_active_talent_group_like_cpp represented_bonus_talent_groups_like_cpp
        represented_talents_like_cpp represented_talents_loaded_like_cpp
        represented_glyphs_like_cpp represented_glyphs_loaded_like_cpp
        represented_talent_reset_script_hooks_like_cpp
        represented_talent_respec_criteria_events_like_cpp
        represented_talent_respec_visual_spell_casts_like_cpp
        represented_confirm_respec_wipe_requests_like_cpp
        represented_primary_specialization_id_like_cpp player_skill_test_fixture_like_cpp
        represented_enchanting_skill represented_gray_level_script_overrides_like_cpp
        championing_faction_like_cpp reputation_state_like_cpp watched_faction_index_like_cpp
        rest_mgr_test_fixture_like_cpp represented_weapon_proficiency_like_cpp
        represented_armor_proficiency_like_cpp
    """.split()),
    ('combat', 'CombatState', 'combat', 'Combat target and flags, vitals and powers, GM and immunity flags, PvP flags and timers, death and resurrection.', """
        combat_target in_combat player_alive_like_cpp player_game_master_like_cpp
        player_cheat_god_like_cpp player_normal_damage_immune_like_cpp
        player_environmental_damage_immune_like_cpp player_health_like_cpp
        player_max_health_like_cpp represented_player_powers_like_cpp
        represented_player_max_powers_like_cpp represented_player_base_mana_like_cpp
        player_pvp_hostile_like_cpp player_pvp_enabled_like_cpp player_in_pvp_flag_like_cpp
        player_pvp_end_timer_like_cpp player_contested_pvp_timer_like_cpp
        represented_death_timer_active_like_cpp represented_resurrection_request_like_cpp
        represented_delayed_resurrection_after_teleport_like_cpp
        area_spirit_healer_guid_like_cpp represented_repop_at_graveyard_count selection_guid
    """.split()),
    ('movement', 'MovementState', 'movement', 'Player movement fixtures: position, flags, jump and fall, acks, speeds and force mods, and vehicle movement sinks.', """
        player_position player_movement_flags_like_cpp player_movement_time_like_cpp
        player_movement_jump_like_cpp last_fall_time_like_cpp last_fall_z_like_cpp
        fall_damage_events_like_cpp player_out_of_bounds_like_cpp
        under_map_damage_events_like_cpp player_moved_unit_guid_like_cpp
        movement_ack_events_like_cpp represented_can_swim_to_fly_transition_like_cpp
        represented_mover_fixed_position_vehicle_like_cpp movement_counter_like_cpp
        forced_speed_changes_like_cpp movement_speed_rates_like_cpp
        movement_force_mod_magnitude_changes_like_cpp movement_force_mod_magnitude_like_cpp
        movement_speed_ack_events_like_cpp movement_jump_proc_requests_like_cpp
        player_collision_height_like_cpp delayed_operations_processed_like_cpp
        represented_vehicle_dismiss_movements_like_cpp
        represented_vehicle_base_movements_like_cpp
    """.split()),
    ('teleport', 'TeleportState', 'teleport', 'Near, far and delayed teleport state and acks, the pending teleport, the homebind and spline-done taxi events.', """
        near_teleport_pending_like_cpp represented_far_teleport_pending_like_cpp
        near_teleport_destination_like_cpp near_teleport_destination_zone_area_like_cpp
        represented_delayed_teleport_like_cpp represented_can_delay_teleport_like_cpp
        represented_has_delayed_teleport_like_cpp move_teleport_ack_events_like_cpp
        pending_teleport move_spline_done_taxi_events_like_cpp represented_homebind_like_cpp
    """.split()),
    ('vehicles', 'TaxiVehicleState', 'vehicles', 'Taxi flight and mount/vehicle kit state: destinations, seat state, vehicle requests, transport attach and mount counters.', """
        taxi_destinations_like_cpp represented_activate_taxi_requests_like_cpp
        taxi_flight_state_like_cpp taxi_unit_flags_like_cpp taxi_mounted_like_cpp
        player_mount_display_id_like_cpp player_mount_vehicle_id_like_cpp
        player_mount_vehicle_kit_like_cpp player_mount_vehicle_accessories_like_cpp
        player_mount_vehicle_seat_count_like_cpp player_mount_vehicle_usable_seat_count_like_cpp
        player_vehicle_seat_flags_like_cpp player_vehicle_seat_id_like_cpp
        represented_vehicle_seat_change_requests_like_cpp
        represented_vehicle_seat_spell_click_requests_like_cpp
        represented_vehicle_enter_requests_like_cpp player_on_transport_like_cpp
        player_transport_login_state_like_cpp mount_vehicle_create_requests_like_cpp
        mount_vehicle_remove_requests_like_cpp
        mount_cancel_expected_vehicle_aura_packets_like_cpp
        mount_collision_height_update_requests_like_cpp player_mounted_like_cpp
    """.split()),
    ('pets', 'PetState', 'pets', 'Represented pet state, pet stable, react and command state, pet speeds, temporary (un)summon and mount pet-control counters.', """
        represented_pet_guid_like_cpp represented_temporary_unsummoned_pet_number_like_cpp
        represented_old_pet_spell_like_cpp represented_pet_stable_like_cpp
        represented_character_pet_rows_empty_authority_complete_like_cpp
        represented_pet_created_by_spell_like_cpp represented_pet_react_state_like_cpp
        represented_pet_command_state_like_cpp temporary_mount_pet_react_state_like_cpp
        temporary_pet_unsummon_requests_like_cpp temporary_pet_resummon_requests_like_cpp
        represented_pet_movement_speed_rates_like_cpp
        represented_pet_speed_propagations_like_cpp mount_pet_control_disable_requests_like_cpp
        mount_pet_control_enable_requests_like_cpp mount_pet_resummon_requests_like_cpp
        battle_pet_test_fixture_like_cpp
    """.split()),
    ('social', 'SessionSocialLimits', None, None, """
        group_guid represented_subgroup_like_cpp represented_group_update_sequences_like_cpp
        guild_test_fixture_like_cpp calendar_test_fixture_like_cpp trade_test_fixture_like_cpp
        duel_test_fixture_like_cpp represented_silence_party_talker_like_cpp
        represented_sign_petitions_like_cpp represented_decline_petitions_like_cpp
        represented_query_petitions_like_cpp addon_filter
    """.split()),
    ('battleground', 'BattlegroundState', 'battleground', 'Battleground and arena membership and the represented battlemaster, battlefield and wargame request sinks.', """
        player_battleground_type_id_like_cpp player_battleground_map_id_like_cpp
        represented_battleground_status_like_cpp
        represented_battleground_leave_requests_like_cpp
        represented_battlemaster_hellos_like_cpp represented_battlefield_lists_like_cpp
        represented_battlemaster_joins_like_cpp represented_battlemaster_join_arenas_like_cpp
        represented_battlemaster_join_skirmishes_like_cpp
        represented_battleground_queue_slots_like_cpp represented_battlefield_ports_like_cpp
        represented_wargame_invite_acceptances_like_cpp
        represented_arena_team_id_invited_like_cpp
    """.split()),
    ('instances', 'InstanceState', 'instances', 'Instance binds and reset times, the instance fixture, exploration and area-zone criteria and adventure-map quest starts.', """
        pending_bind represented_confirmed_pending_binds
        represented_instance_reset_times_like_cpp instance_test_fixture_like_cpp
        represented_explored_zones_like_cpp
        represented_reveal_world_map_overlay_criteria_like_cpp
        represented_area_zone_criteria_like_cpp
        represented_adventure_map_start_quest_requests_like_cpp
    """.split()),
    ('world_entities', 'WorldEntitiesState', 'world_entities', 'Session-local represented creature and gameobject runtime: creature auras, kills, spawn and tick, gameobject use and phase state.', """
        represented_creature_auras_like_cpp pending_creature_spawn
        pending_creature_kill_loot_like_cpp pending_creature_kill_rewards_like_cpp
        represented_creature_kill_events_like_cpp creature_tick
        represented_gameobject_use_effects represented_gameobject_use_states
        represented_gameobject_phase_shifts represented_gameobject_criteria_events
        suppress_creature_movement_queued_at_or_before_like_cpp
    """.split()),
    ('visibility', 'VisibilityState', 'visibility', 'Per-client visibility and publication fences: visible transports, last visibility position and farsight, delivered-update guards.', """
        client_visible_transports_like_cpp last_visibility_pos
        last_observed_farsight_object_like_cpp visibility_test_fixture_like_cpp
        represented_dynamic_object_values_updates_delivered_like_cpp
        represented_player_unit_values_updates_delivered_like_cpp
        represented_gameobject_visual_despawns_delivered_like_cpp
        represented_capture_point_removed_delivered_like_cpp
    """.split()),
    ('presentation', 'PlayerPresentationState', 'presentation', 'Player presentation fixtures: unit flags and scale, stand state and emote, action bars, cinematics, CUF profiles and barber requests.', """
        player_unit_flags_like_cpp player_object_scale_like_cpp player_stand_state_like_cpp
        represented_live_applications_like_cpp player_emote_state_like_cpp
        active_player_local_flags_like_cpp active_player_transport_server_time_like_cpp
        active_player_multi_action_bars_like_cpp represented_action_buttons_like_cpp
        represented_action_buttons_loaded_like_cpp represented_cinematic_state_like_cpp
        represented_cinematic_next_camera_events_like_cpp
        represented_cinematic_end_events_like_cpp represented_movie_complete_events_like_cpp
        represented_confirm_barbers_choice_requests_like_cpp
        represented_alter_appearance_requests_like_cpp cuf_profiles_like_cpp
        cuf_profiles_loaded_like_cpp
    """.split()),
    ('interaction', 'InteractionState', 'interaction', 'NPC interaction: C++ `PlayerInteractionData`, gossip options, vendor stock and the support-feature fixture.', """
        player_interaction_data_like_cpp gossip_options support_feature_test_fixture_like_cpp
        vendor_item_counts vendor_buy_item_test_override_like_cpp
    """.split()),
    ('quest_state', 'SessionQuestState', None, None, """
        quest_test_fixture_like_cpp
    """.split()),
    ('view', 'SessionWorldView', None, None, """

    """.split()),
)


class CodemodError(RuntimeError):
    pass


def blank_noncode(text: str) -> str:
    """Blank comments and string/char literal contents, keeping offsets."""
    out = list(text)
    n = len(text)
    i = 0

    def blank(a: int, b: int) -> None:
        for k in range(a, min(b, n)):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            end = text.find("\n", i)
            end = n if end < 0 else end
            blank(i, end)
            i = end
        elif c == "/" and nxt == "*":
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif c in "rb" and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            m = re.match(r"(?:br|r)(#*)\"", text[i:i + 300])
            if m:
                close = '"' + m.group(1)
                end = text.find(close, i + m.end())
                end = n if end < 0 else end + len(close)
                blank(i + m.end(), end - len(close))
                i = end
            else:
                i += 1
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            blank(i + 1, j)
            i = j + 1
        elif c == "'":
            if nxt == "\\":
                end = text.find("'", i + 2 if text[i + 2:i + 3] != "'" else i + 3)
                end = n if end < 0 else end
                blank(i + 1, end)
                i = end + 1
            elif i + 2 < n and text[i + 2] == "'":
                blank(i + 1, i + 2)
                i += 3
            else:
                i += 1
        else:
            i += 1
    return "".join(out)


def matching_close(code: str, open_index: int) -> int:
    depth = 0
    for j in range(open_index, len(code)):
        if code[j] == "{":
            depth += 1
        elif code[j] == "}":
            depth -= 1
            if depth == 0:
                return j
    raise CodemodError(f"unbalanced brace at {open_index}")


def split_segments(code: str, start: int, end: int) -> list[tuple[int, int]]:
    """Top-level comma-separated segments of code[start:end] (absolute offsets)."""
    parts, depth, seg = [], 0, start
    for j in range(start, end):
        ch = code[j]
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        elif ch == "<":
            depth += 1
        elif ch == ">" and code[j - 1] not in "-=":
            depth -= 1
        elif ch == "," and depth == 0:
            parts.append((seg, j))
            seg = j + 1
    parts.append((seg, end))
    return parts


ATTR = re.compile(r"\s*#\s*\[")
VIS = re.compile(r"(pub(?:\s*\([^)]*\))?)\s+")


def strip_attrs(code: str) -> tuple[str, list[str]]:
    attrs, rest = [], code
    while True:
        m = ATTR.match(rest)
        if not m:
            return rest.lstrip(), attrs
        depth = 0
        for j in range(m.end() - 1, len(rest)):
            if rest[j] == "[":
                depth += 1
            elif rest[j] == "]":
                depth -= 1
                if depth == 0:
                    break
        attrs.append(" ".join(rest[m.start():j + 1].split()))
        rest = rest[j + 1:]


class Field:
    def __init__(self, raw: str, code: str):
        self.raw = raw
        rest, attrs = strip_attrs(code)
        self.cfg_test = any(re.fullmatch(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", a) for a in attrs)
        m = VIS.match(rest)
        self.vis = " ".join(m.group(1).split()).replace("( ", "(").replace(" )", ")") if m else ""
        self.vis = re.sub(r"\(\s*in\s+", "(in ", self.vis)
        rest = rest[m.end():] if m else rest
        m = re.match(r"(" + IDENT + r")\s*:", rest)
        if not m:
            raise CodemodError(f"cannot parse field segment: {code.strip()[:80]!r}")
        self.name = m.group(1)


def struct_fields(text: str, code: str, head: re.Pattern) -> tuple[int, int, list[Field]]:
    m = head.search(code)
    if not m:
        raise CodemodError(f"struct not found: {head.pattern}")
    open_index = m.end() - 1
    close = matching_close(code, open_index)
    fields = []
    for a, b in split_segments(code, open_index + 1, close):
        if code[a:b].strip():
            fields.append(Field(text[a:b], code[a:b]))
    return open_index, close, fields


def struct_head(name: str) -> re.Pattern:
    return re.compile(r"\bstruct\s+" + name + r"\b[^{;]*\{")


def emit_fields(raws: list[str]) -> str:
    """Field segments joined back with commas; first segment starts on a fresh line."""
    out = []
    for index, raw in enumerate(raws):
        raw = raw.rstrip()
        if index == 0:
            raw = "\n" + raw.lstrip("\n")
        out.append(raw + ",")
    return "".join(out) + "\n"


def doc_lines(doc: str, indent: str) -> str:
    width = 100 - len(indent) - 4
    return "".join(f"{indent}/// {line}\n" for line in textwrap.wrap(doc, width))


PRELUDE = frozenset({"Option", "Vec", "String", "Box", "Result", "Some", "None", "Self"})


def needs_parent_names(fields: list["Field"]) -> bool:
    """Whether field types name an item imported by `state.rs` (a bare, non-prelude path)."""
    for field in fields:
        code = blank_noncode(field.raw)
        type_text = code.split(":", 1)[1] if ":" in code else ""
        for m in re.finditer(r"(?<![:\w])([A-Z][A-Za-z0-9_]*)\b", type_text):
            if m.group(1) not in PRELUDE:
                return True
    return False


def glob_import(group: list["Field"], test_only: bool) -> str:
    """`use super::*;`, gated like the members that need it, or nothing."""
    if needs_parent_names([f for f in group if not f.cfg_test]):
        return "use super::*;\n\n"
    if needs_parent_names([f for f in group if f.cfg_test]):
        return "use super::*;\n\n" if test_only else "#[cfg(test)]\nuse super::*;\n\n"
    return ""


# Wrapper field and sub-state type visibility: the narrowest that compiles across
# wow-world (default, test-fixtures, all targets), world-server and world-modules.
# Everything starts at `pub(in crate::session)`; these groups are read outside
# `crate::session` (first consumer found by E0616 in parentheses) and need
# `pub(crate)`. Members keep their own declared visibility.
CRATE_VISIBLE = frozenset({
    "core",            # battle_pet_purchase/ops_1.rs
    "lifecycle",       # handlers/character/account.rs
    "loot",            # handlers/character/session_state.rs
    "catalogs",        # handlers/character/session_state/login_data.rs
    "collections",     # unit_tests/handlers/collections/tests/collections.rs
    "spell_state",     # spell_acquisition/adapter.rs
    "auras",           # unit_tests/handlers/character_tests/fixtures_3.rs
    "progression",     # unit_tests/spell_acquisition/application/tests/preparation.rs
    "combat",          # handlers/entities/corpse.rs
    "vehicles",        # canonical_player_sync.rs
    "pets",            # canonical_player_sync.rs
    "social",          # handlers/chat/channels.rs
    "instances",       # handlers/instances/mod.rs
    "world_entities",  # handlers/character/session_state.rs
    "visibility",      # handlers/character/session_state.rs
    "interaction",     # handlers/character/vendor_admission.rs
    "quest_state",     # handlers/quest/handlers/reward_flow.rs
})


def wrapper_visibility(key: str) -> str:
    return "pub(crate)" if key in CRATE_VISIBLE else "pub(in crate::session)"


def plan() -> dict:
    members = {}
    for key, struct, stem, doc, names in GROUPS:
        for name in names:
            if name in members:
                raise CodemodError(f"{name} assigned twice")
            members[name] = key
    return members


def step1() -> bool:
    text = STATE.read_text(encoding="utf-8")
    if MARKER.search(text):
        return False
    code = blank_noncode(text)
    open_index, close, fields = struct_fields(text, code, re.compile(r"\bpub\s+struct\s+WorldSession\s*\{"))
    by_name = {f.name: f for f in fields}
    order = {f.name: i for i, f in enumerate(fields)}
    members = plan()
    retained = {key for key, _, stem, _, _ in GROUPS if stem is None}
    expected = set(members) | retained
    if set(by_name) != expected:
        raise CodemodError(
            f"field set drift: missing={sorted(expected - set(by_name))} "
            f"unplanned={sorted(set(by_name) - expected)}"
        )

    wrappers, new_files, extensions, decls = [], {}, {}, []
    group_meta = {}
    for key, struct, stem, doc, names in GROUPS:
        # Members keep their original relative declaration order.
        group = [by_name[n] for n in sorted(names, key=order.__getitem__)]
        if stem is None:
            own = by_name[key]
            vis = wrapper_visibility(key)
            raw = own.raw
            if vis != own.vis:
                raw = raw.replace(f"{own.vis} {key}:", f"{vis} {key}:", 1)
            wrappers.append(raw)
            extensions[struct] = (group, vis)
            group_meta[key] = (struct, False)
            continue
        vis = wrapper_visibility(key)
        test_only = all(f.cfg_test for f in group)
        cfg = "    #[cfg(test)]\n" if test_only else ""
        wrappers.append(f"\n{doc_lines(doc, '    ')}{cfg}    {vis} {key}: {struct}")
        body = emit_fields([f.raw for f in group])
        new_files[stem] = (
            HEADER
            + f"//! `WorldSession::{key}` sub-state (#1241 F2): moved fields, no logic.\n\n"
            + glob_import(group, test_only)
            + f"{doc_lines(doc, '')}{vis} struct {struct} {{{body}}}\n"
        )
        cfg_line = "#[cfg(test)]\n" if test_only else ""
        decls.append(f"{cfg_line}mod {stem};\n{cfg_line}pub(in crate::session) use {stem}::{struct};\n")
        group_meta[key] = (struct, test_only)

    new_text = text[:open_index + 1] + emit_fields(wrappers) + text[close:]
    head = re.search(r"\npub\s+struct\s+WorldSession\b", new_text)
    new_text = new_text[:head.start() + 1] + DROP_ORDER_NOTE + new_text[head.start() + 1:]
    # Existing sub-states absorb their members at the end of their body.
    for struct, (group, vis) in extensions.items():
        code2 = blank_noncode(new_text)
        m = struct_head(struct).search(code2)
        s_open = m.end() - 1
        s_close = matching_close(code2, s_open)
        prefix = new_text[:s_close].rstrip()
        if group:
            prefix += emit_fields([f.raw for f in group]).rstrip("\n")
        new_text = prefix + "\n" + new_text[s_close:]
        line_start = new_text.rfind("\n", 0, m.start()) + 1
        decl = re.compile(r"pub(?:\([^)]*\))?\s+struct\s+" + struct + r"\b")
        dm = decl.search(new_text, line_start)
        new_text = new_text[:dm.start()] + f"{vis} struct {struct}" + new_text[dm.end():]
    # Module declarations go right before the first struct item.
    first = re.search(r"\n(?:///[^\n]*\n)*(?:#\[[^\n]*\n)*pub(?:\([^)]*\))?\s+struct\s", new_text)
    new_text = new_text[:first.start() + 1] + "".join(decls) + "\n" + new_text[first.start() + 1:]

    construction = rewrite_construction(members, group_meta, order)
    STATE_DIR.mkdir(exist_ok=True)
    for stem, content in new_files.items():
        (STATE_DIR / f"{stem}.rs").write_text(content, encoding="utf-8")
    STATE.write_text(new_text, encoding="utf-8")
    CONSTRUCTION.write_text(construction, encoding="utf-8")
    return True


def rewrite_construction(members: dict, group_meta: dict, order: dict) -> str:
    text = CONSTRUCTION.read_text(encoding="utf-8")
    code = blank_noncode(text)
    m = re.search(r"\n        Self \{", code)
    if not m:
        raise CodemodError("WorldSession::new literal not found")
    open_index = m.end() - 1
    close = matching_close(code, open_index)
    entries = {}
    for a, b in split_segments(code, open_index + 1, close):
        if not code[a:b].strip():
            continue
        rest, _ = strip_attrs(code[a:b])
        n = re.match(r"(" + IDENT + r")\s*(?::|$)", rest.strip())
        if not n:
            raise CodemodError(f"cannot parse literal entry {rest[:60]!r}")
        entries[n.group(1)] = (text[a:b], code[a:b])
    retained = {key for key, _, stem, _, _ in GROUPS if stem is None}
    if set(entries) != set(members) | retained:
        raise CodemodError("construction literal does not initialise exactly the planned fields")
    out = []
    for key, struct, stem, doc, names in GROUPS:
        inner = [entries[n][0] for n in sorted(names, key=order.__getitem__)]
        if stem is None:
            raw, raw_code = entries[key]
            if inner:
                local_open = raw_code.index("{")
                local_close = matching_close(raw_code, local_open)
                head_part = raw[:local_close].rstrip()
                if not head_part.endswith(",") and not head_part.endswith("{"):
                    head_part += ","
                raw = head_part + emit_fields(inner).rstrip("\n") + "\n" + raw[local_close:]
            out.append(raw)
            continue
        _, test_only = group_meta[key]
        cfg = "            #[cfg(test)]\n" if test_only else ""
        out.append(f"\n{cfg}            {key}: {struct} {{{emit_fields(inner)}            }}")
    literal = emit_fields(out)
    imports = "".join(
        ("#[cfg(test)]\n" if test_only else "") + f"use crate::session::state::{struct};\n"
        for key, (struct, test_only) in group_meta.items()
        if key not in retained
    )
    new_text = text[:open_index + 1] + literal + "        " + text[close:]
    anchor = new_text.index("use crate::session::state::")
    return new_text[:anchor] + imports + new_text[anchor:]


# ---- step 2: compiler-guided access rewrite ---------------------------------

TARGET_MESSAGE = re.compile(
    r"^(?:no field `(" + IDENT + r")` on type|attempted to take value of method `("
    + IDENT + r")` on type) `[^`]*\bWorldSession\b[^`]*`$"
)
PATCH_CODES = {"E0609", "E0615"}
LITERAL_CODES = {"E0560", "E0063", "E0026", "E0027", "E0616"}


def cargo_check(log: pathlib.Path, extra: list[str]) -> tuple[int, list[dict]]:
    command = ["cargo", "check", "-p", "wow-world", "--all-targets", "--message-format=json", *extra]
    with log.open("w", encoding="utf-8") as handle:
        result = subprocess.run(command, cwd=REPO, stdout=handle, stderr=subprocess.DEVNULL)
    messages = []
    for line in log.read_text(encoding="utf-8").splitlines():
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if record.get("reason") == "compiler-message":
            messages.append(record["message"])
    return result.returncode, messages


def step2(log_dir: pathlib.Path, extra: list[str], max_rounds: int) -> list[int]:
    members = plan()
    retained = {key for key, _, stem, _, _ in GROUPS if stem is None}
    segment = {name: key for name, key in members.items()}
    rounds = []
    for _ in range(max_rounds):
        status, messages = cargo_check(log_dir / f"f2-round-{len(rounds) + 1}.jsonl", extra)
        edits: dict[pathlib.Path, set[tuple[int, str, str]]] = {}
        unexpected = []
        for message in messages:
            if message.get("level") != "error":
                continue
            code = (message.get("code") or {}).get("code")
            if code in PATCH_CODES:
                m = TARGET_MESSAGE.match(message["message"])
                name = m and (m.group(1) or m.group(2))
                if not name or name not in segment or name in retained:
                    unexpected.append(message["message"])
                    continue
                span = next(s for s in message["spans"] if s["is_primary"])
                path = REPO / span["file_name"]
                edits.setdefault(path, set()).add((span["byte_start"], span["byte_end"], name))
            elif code in LITERAL_CODES or code:
                unexpected.append(f"{code}: {message['message']}")
            elif not message["message"].startswith("aborting due to"):
                unexpected.append(message["message"])
        count = 0
        for path, spans in edits.items():
            data = path.read_bytes()
            for start, end, name in sorted(spans, reverse=True):
                token = data[start:end].decode("utf-8")
                if token != name:
                    raise CodemodError(f"{path}:{start}: span text {token!r} is not {name!r}")
                data = data[:start] + f"{segment[name]}.".encode() + data[start:]
                count += 1
            path.write_bytes(data)
        rounds.append(count)
        print(f"round {len(rounds)}: exit {status}, {count} spans patched", file=sys.stderr)
        if count == 0:
            if status != 0:
                for line in unexpected[:40]:
                    print("unhandled:", line, file=sys.stderr)
                raise CodemodError(f"cargo check still fails with {len(unexpected)} unhandled errors")
            return rounds
    raise CodemodError("no fixed point within the round budget")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    apply = sub.add_parser("apply", help="text step, then the compiler-guided loop")
    apply.add_argument("--text-only", action="store_true")
    apply.add_argument("--features", default=None)
    apply.add_argument("--log-dir", default=str(REPO / "target/f2-codemod"))
    apply.add_argument("--max-rounds", type=int, default=8)
    args = parser.parse_args()
    try:
        changed = step1()
        print(f"text step: {'applied' if changed else 'already applied (no-op)'}", file=sys.stderr)
        if not args.text_only:
            log_dir = pathlib.Path(args.log_dir)
            log_dir.mkdir(parents=True, exist_ok=True)
            extra = ["--features", args.features] if args.features else []
            rounds = step2(log_dir, extra, args.max_rounds)
            print(f"compiler loop: {len(rounds)} round(s), spans per round {rounds}", file=sys.stderr)
    except CodemodError as error:
        print(f"f2 codemod: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
