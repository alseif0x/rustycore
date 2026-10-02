#!/usr/bin/env python3
"""Apply reviewed cross-crate visibility changes for #1263 F4a P4a.

Catalog and map_manager files are moved mechanically. This codemod widens only
declarations with a concrete wow-world consumer; plan is read-only and apply is
repeatable. It intentionally uses exact declaration matches instead of parsing Rust.
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


REPO = Path(__file__).resolve().parents[2]
CATALOG_ROOT = Path("crates/wow-world-core/src/catalogs")
MAP_MANAGER_ROOT = Path("crates/wow-world-core/src/map_manager")

PUBLIC_STRUCTS = {
    "chr.rs": ("ChrCatalogsLikeCpp",),
    "creature.rs": ("CreatureCatalogsLikeCpp",),
    "faction.rs": ("FactionCatalogsLikeCpp",),
    "gameobject.rs": ("GameObjectCatalogsLikeCpp",),
    "item.rs": ("ItemCatalogsLikeCpp",),
    "map.rs": ("MapCatalogsLikeCpp",),
    "quest.rs": ("QuestCatalogsLikeCpp",),
    "spell.rs": ("SpellCatalogsLikeCpp",),
}

# These fields are read or written directly by wow-world. Spell acquisition,
# class-options and label slots stay crate-visible because wow-world uses their
# existing accessors instead.
PUBLIC_FIELDS = {
    "chr.rs": (
        "classes_store",
        "races_store",
        "specialization_store",
    ),
    "creature.rs": (
        "display_info_extra_store",
        "display_info_store",
        "model_data_store",
        "model_info_store",
        "onkill_reputation_store",
        "template_lifecycle_store_like_cpp",
        "template_mount_store",
    ),
    "faction.rs": ("store", "template_store"),
    "gameobject.rs": ("display_info_store",),
    "item.rs": (
        "appearance_store",
        "bonus_db2_store",
        "child_equipment_store",
        "effect_store",
        "extended_cost_store",
        "limit_category_condition_store",
        "limit_category_store",
        "modified_appearance_store",
        "random_enchantment_template_store",
        "random_properties_store",
        "random_suffix_store",
        "search_name_store",
        "set_store",
        "spec_override_store",
        "stats_store",
        "store",
    ),
    "map.rs": ("difficulty_store", "difficulty_x_condition_store", "store"),
    "quest.rs": (
        "faction_reward_store",
        "info_store",
        "money_reward_store",
        "package_item_store",
        "pool_store",
        "store",
        "v2_store",
        "xp_store",
    ),
    "spell.rs": (
        "item_set_spell_store",
        "npc_spell_click_store",
        "pet_default_spell_store",
        "pet_family_spell_store",
        "pet_levelup_spell_store",
        "serverside_spell_store",
        "spell_area_store",
        "spell_aura_options_store",
        "spell_aura_restrictions_store",
        "spell_category_store",
        "spell_chain_store",
        "spell_custom_attribute_store",
        "spell_duration_store",
        "spell_enchant_proc_store",
        "spell_equipped_items_store",
        "spell_group_stack_rule_store",
        "spell_group_store",
        "spell_item_enchantment_condition_store",
        "spell_item_enchantment_store",
        "spell_learn_skill_store",
        "spell_learn_spell_store",
        "spell_levels_store",
        "spell_linked_store",
        "spell_misc_store",
        "spell_pet_aura_store",
        "spell_proc_store",
        "spell_radius_store",
        "spell_range_store",
        "spell_required_store",
        "spell_shapeshift_form_store",
        "spell_store",
        "spell_target_position_store",
        "spell_target_restrictions_store",
        "spell_threat_store",
        "spell_totem_model_store",
    ),
}

# Existing spell catalog operations called from wow-world. The unused
# npc_spell_click_store and spell_proc_store getters remain crate-visible.
PUBLIC_METHODS = {
    "spell.rs": (
        "set_npc_spell_click_store",
        "set_serverside_spell_store",
        "set_spell_acquisition_catalog",
        "set_spell_aura_options_store",
        "set_spell_aura_restrictions_store",
        "set_spell_category_store",
        "set_spell_class_options_store",
        "set_spell_label_store",
        "spell_label_store",
        "spell_class_options_store",
        "set_spell_custom_attribute_store",
        "set_spell_duration_store",
        "set_spell_learn_skill_store",
        "set_spell_learn_spell_store",
        "set_spell_levels_store",
        "set_spell_misc_store",
        "set_spell_proc_store",
        "set_spell_radius_store",
        "set_spell_range_store",
        "set_spell_required_store",
        "set_spell_shapeshift_form_store",
        "set_spell_target_position_store",
        "set_spell_target_restrictions_store",
        "set_spell_totem_model_store",
        "spell_acquisition_catalog",
        "spell_aura_restrictions_store",
        "spell_category_store",
        "spell_chain_store",
        "spell_custom_attribute_store_like_cpp",
        "spell_learn_skill_store_like_cpp",
        "spell_learn_spell_store_like_cpp",
        "spell_levels_store",
        "spell_linked_store_like_cpp",
        "spell_misc_store",
        "spell_range_store",
        "spell_required_store_like_cpp",
        "spell_shapeshift_form_store",
        "spell_store",
        "spell_target_restrictions_store",
    ),
}

# Map-manager methods called by retained wow-world production code or tests.
# Keep reset_creature_spell_schedule_like_cpp crate-visible: its only callers
# moved with the owner. Runtime and motion state fields stay private.
PUBLIC_MAP_MANAGER_METHODS = {
    "combat.rs": (
        "creature_spell_schedule_initialized_like_cpp",
        "mark_creature_spell_schedule_initialized_like_cpp",
        "creature_spell_engagement_epoch_like_cpp",
        "schedule_creature_spell_slot_after_like_cpp",
        "clear_creature_spell_slot_like_cpp",
        "first_due_creature_spell_slot_like_cpp",
        "creature_spell_due_in_ms_for_test",
        "random_creature_spell_delay_like_cpp",
        "random_creature_spell_hit_roll_like_cpp",
    ),
    "movement/motion_master.rs": ("sync_runtime_motion_master_like_cpp",),
    "movement/random_and_waypoint.rs": (
        "update_default_random_movement_after_spline_like_cpp",
        "update_default_waypoint_movement_after_spline_like_cpp",
    ),
    "runtime/creature.rs": (
        "runtime_elapsed_ms_like_cpp",
        "advance_runtime_clock_like_cpp",
        "backdate_runtime_clock_for_test",
        "runtime_rng_authority_complete_like_cpp",
        "invalidate_runtime_rng_authority_like_cpp",
    ),
}


class CodemodError(Exception):
    """A target file or declaration differs from the reviewed shape."""


def widen_declaration(source: str, kind: str, name: str) -> tuple[str, bool]:
    """Change one exact `pub(crate)` declaration to `pub`; accept an applied result."""
    if kind == "struct":
        tail = rf"\s+struct\s+{re.escape(name)}\b"
    elif kind == "field":
        tail = rf"\s+{re.escape(name)}\s*:"
    elif kind == "method":
        tail = rf"\s+(?:const\s+)?fn\s+{re.escape(name)}\s*\("
    else:
        raise CodemodError(f"unknown declaration kind: {kind}")

    pattern = re.compile(
        rf"(?m)^(?P<indent>[ \t]*)(?P<visibility>pub(?:\(crate\))?)(?P<tail>{tail})"
    )
    matches = list(pattern.finditer(source))
    if len(matches) != 1:
        raise CodemodError(
            f"expected one {kind} declaration for {name}, found {len(matches)}"
        )
    match = matches[0]
    if match.group("visibility") == "pub":
        return source, False
    start, end = match.span("visibility")
    return source[:start] + "pub" + source[end:], True


def declarations(filename: str) -> tuple[tuple[str, str], ...]:
    if filename not in PUBLIC_STRUCTS:
        raise CodemodError(f"unexpected catalog file: {filename}")
    return (
        *(("struct", name) for name in PUBLIC_STRUCTS[filename]),
        *(("field", name) for name in PUBLIC_FIELDS.get(filename, ())),
        *(("method", name) for name in PUBLIC_METHODS.get(filename, ())),
    )


def transform(filename: str, source: str) -> tuple[str, list[str]]:
    changed: list[str] = []
    for kind, name in declarations(filename):
        source, widened = widen_declaration(source, kind, name)
        if widened:
            changed.append(f"{kind} {name}")
    return source, changed


def transform_map_manager(filename: str, source: str) -> tuple[str, list[str]]:
    if filename not in PUBLIC_MAP_MANAGER_METHODS:
        raise CodemodError(f"unexpected map_manager file: {filename}")
    changed: list[str] = []
    for name in PUBLIC_MAP_MANAGER_METHODS[filename]:
        source, widened = widen_declaration(source, "method", name)
        if widened:
            changed.append(f"method {name}")
    return source, changed


def run_catalogs(action: str, root: Path) -> None:
    for filename in PUBLIC_STRUCTS:
        path = root / CATALOG_ROOT / filename
        if not path.is_file():
            raise CodemodError(f"missing moved catalog source: {path}")
        original = path.read_text(encoding="utf-8")
        updated, changed = transform(filename, original)
        verb = "would widen" if action == "plan" else "widened"
        if changed:
            print(f"{path}: {verb} {len(changed)} declarations")
            for declaration in changed:
                print(f"  {declaration}")
            if action == "apply":
                path.write_text(updated, encoding="utf-8")
        else:
            print(f"{path}: already applied")


def run_map_manager(action: str, root: Path) -> None:
    for filename in PUBLIC_MAP_MANAGER_METHODS:
        path = root / MAP_MANAGER_ROOT / filename
        if not path.is_file():
            raise CodemodError(f"missing moved map_manager source: {path}")
        original = path.read_text(encoding="utf-8")
        updated, changed = transform_map_manager(filename, original)
        verb = "would widen" if action == "plan" else "widened"
        if changed:
            print(f"{path}: {verb} {len(changed)} declarations")
            for declaration in changed:
                print(f"  {declaration}")
            if action == "apply":
                path.write_text(updated, encoding="utf-8")
        else:
            print(f"{path}: already applied")


def run(action: str, root: Path, stage: str) -> None:
    if stage in ("catalogs", "all"):
        run_catalogs(action, root)
    if stage in ("map-manager", "all"):
        run_map_manager(action, root)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("plan", "apply"))
    parser.add_argument(
        "--stage", choices=("all", "catalogs", "map-manager"), default="all"
    )
    parser.add_argument("--root", type=Path, default=REPO)
    args = parser.parse_args()
    try:
        run(args.action, args.root, args.stage)
    except (CodemodError, OSError) as error:
        print(f"f4_extract_core: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
