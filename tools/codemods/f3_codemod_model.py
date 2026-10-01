#!/usr/bin/env python3
"""Model for f3_move_methods.py (#1241 F3): groups, owner types, patterns and the move-kind rules.

Pure functions over a scanned fn record; no I/O. Standard library only.
"""
from __future__ import annotations

import re

IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
HUB = {"core", "catalogs", "config", "fixtures"}
STATE_TYPE = {"core": "SessionCore", "catalogs": "SessionCatalogs", "config": "SessionWorldConfig",
              "lifecycle": "SessionLifecycleState", "loot": "LootState", "inventory": "InventoryState",
              "spell_state": "SessionSpellState", "social": "SessionSocialLimits", "instances": "InstanceState",
              "world_entities": "WorldEntitiesState", "visibility": "VisibilityState",
              "interaction": "InteractionState", "quest_state": "SessionQuestState", "view": "SessionWorldView",
              "phase": "SessionPhaseRail"}
HUB_TYPES = {"HubRef", "HubMut"}
SH = ("state-hub", "state-hubmut")                           # group state + HubRef (`&self`) / `&mut HubMut`
# `fixtures.<group>` (cfg(test)) owner types: cfg(test)-only fns that touch only their group move onto them.
FIXTURE_TYPE = {"identity": "PlayerIdentityState", "collections": "CollectionsState", "auras": "AuraState",
                "progression": "ProgressionState", "combat": "CombatState", "movement": "MovementState",
                "teleport": "TeleportState", "vehicles": "TaxiVehicleState", "pets": "PetState",
                "battleground": "BattlegroundState", "presentation": "PlayerPresentationState"}
FIXTURE_OF = {t: g for g, t in FIXTURE_TYPE.items()}
OWNER_TYPES = set(STATE_TYPE.values()) | HUB_TYPES | set(FIXTURE_TYPE.values())
PERMANENT_THUNKS = {"player_guid"}                           # hot hub accessors: an inline thunk forever
SHELL = "shell"
# File-domain -> default group (F3-E design.md, section 1). Domains not listed are handler shells.
DOMAIN_MAP = {g: d.split() for g, d in {
    "inventory": "session/player_items session/money session/buyback_adapter session/void_storage_adapter "
                 "session/currency_adapter session/item_modifiers session/trade_adapter",
    "spell_state": "session/spell_state session/player_cast spell_acquisition session/effect_learning",
    "spell_effects": "session/spell_effects", "vehicles": "session/taxi",
    "world_entities": "session/world_entities session/gameobject_interaction",
    "movement": "session/movement session/movement_protocol",
    "quest_state": "session/quest session/quest_dialog session/quest_interaction handlers/quest quest",
    "pets": "session/pets session/pet_loading session/pet_dismissal session/battle_pet_adapter",
    "lifecycle": "session/lifecycle session/lifecycle_ops session/persistence session_persistence_capabilities "
                 "session/player_bootstrap",
    "combat": "session/combat session/player_vitals_adapter",
    "instances": "session/instances session/world_state session/map_admission",
    "progression": "session/progression session/rest_progression session/progression_adapters session/xp_grants "
                   "session/faction_reactions session/trait_configs profession",
    "social": "session/social session/social_requests session/chat",
    "battleground": "session/battleground_adapter",
    "visibility": "session/visibility session/deferred_visibility session/object_updates",
    "presentation": "session/player_presentation session/stand_state_adapter session/action_bar_adapter "
                    "session/cinematic_adapter session/raid_profile_values session/character_customization "
                    "session/appearance",
    "interaction": "session/npc_interaction session/support_features",
    "collections": "session/collection_adapter session/collections battle_pet_purchase",
    "loot": "session/loot handlers/loot", "identity": "player",
    "catalogs": "session/catalogs session/spell_pet_catalogs session/player_condition_values",
    "config": "session/runtime_policy_access",
    "core": "session/connection session/connection_identity session/admission session/time_synchronization "
            "session/player_binding session/player_registry_binding session/canonical_access session/publication "
            "session/mailbox session_commands",
}.items()}
DOMAIN = {d: g for g, ds in DOMAIN_MAP.items() for d in ds}

SELF_FIELD = re.compile(r"\bself\s*\.\s*(" + IDENT + r")\b(?!\s*(?:\(|::\s*<))")
GUARD_LET = re.compile(r"\blet\s+(?:mut\s+)?(" + IDENT + r")\s*=[^;{}=]*?\.\s*(?:lock|write|read)\s*\(\s*\)")
FIXTURE_FIELD = re.compile(r"\bself\s*\.\s*fixtures\s*\.\s*(" + IDENT + r")\b")
SELF_CALL = re.compile(r"\bself\s*\.\s*(" + IDENT + r")(\s*(?:::\s*<[^;{}()]*>\s*)?)\(")
STORE_WRITE = re.compile(r"&\s*mut\s+self\s*\.\s*(?:catalogs|config)\b|\bself\s*\.\s*(?:catalogs|config)"
                         r"(?:\s*\.\s*" + IDENT + r")*\s*(?:\[[^\]]*\]\s*)?[-+*/|&^]?=(?!=)")
TYPE_PATH = re.compile(r"\b(?:Self|WorldSession)\s*::\s*(" + IDENT + r")")
BARE_SELF = re.compile(r"\bself\b(?!\s*(?:\.|::))")
ANY_CALL = re.compile(r"(?:\.|::)\s*(" + IDENT + r")\s*(?:::\s*<[^;{}()]*>\s*)?\(")
# A call that can reach a WorldSession method: `self.m(`, `<..session..>.m(`, `Self::m(`, `WorldSession::m(`.
# Narrower than ANY_CALL so same-named methods of other types are not taken for callers; a missed
# caller only costs a thunk the compiler loop restores (E0599).
WS_CALL = re.compile(r"(?:\b(?:self|\w*session\w*)\s*\.\s*|\b(?:Self|WorldSession)\s*::\s*)(" + IDENT +
                     r")\s*(?:::\s*<[^;{}()]*>\s*)?\(")
SESSION_CALL = re.compile(r"\b(" + IDENT + r")\s*\.\s*(" + IDENT + r")\s*(?:::\s*<[^;{}()]*>\s*)?\(")
IMPL_ANY = re.compile(r"(?m)^[ \t]*impl\s*(?:<[^{};]*?>)?\s*(?:" + IDENT + r"\s*::\s*)*(" + IDENT +
                      r")\b\s*(?:<[^{};]*>)?\s*\{")


PRECONDITIONS = ("holds a std lock guard", "writes catalogs/config", "unsupported parameter", "unexpected receiver", "whole-self", "`Self::`", "macro-generated", "returns a borrow",
                 "reads config")


class CodemodError(Exception):
    pass


def guard_call(body):
    """A `let g = ...lock()/read()/write()` guard still live (not dropped) at a later `self.m(..)` call."""
    for m in GUARD_LET.finditer(body):
        end = len(body)
        depth = 0
        for j in range(m.start(), len(body)):                # the guard lives to the end of its block
            depth += {"{": 1, "}": -1}.get(body[j], 0)
            if depth < 0:
                end = j
                break
        drop = re.search(r"\bdrop\s*\(\s*" + m.group(1) + r"\s*\)", body[m.end():end])
        if SELF_CALL.search(body, m.end(), m.end() + drop.start() if drop else end):
            return True
    return False


def kind_of(f, classes):
    """(class, kind, target type) or a blocked reason string."""
    g = f["target"]
    fields = set(f["acc"])
    if len(fields) == 1 and fields <= {"catalogs", "config"} and g not in ("catalogs", "config"):
        f["store_from"], g = g, next(iter(fields))           # a pure store accessor/setter of any group
        f["target"], f["store_rehomed"] = g, True
    if g in FIXTURE_TYPE:
        if set(f["fx"]) - {g}:
            return f"class C (other fixture groups {sorted(set(f['fx']) - {g})})"
        if not fields <= HUB:
            return f"class C (non-hub fields {sorted(fields - HUB)})"
        if f["cfg_test"] and fields <= {"fixtures"} and f["file"].startswith("session/"):
            cls, kind = "P", "state"                          # cfg(test) fn on its fixture group
        else:                                                # production fns reach fixtures via the hub view
            cls, kind = "C-hub", "hubref" if f["recv"] == "&self" else "hubmut"
        return preconditions(f, cls, kind, classes) or (cls, kind, {"hubref": "HubRef", "hubmut": "HubMut"}
                                                         .get(kind, FIXTURE_TYPE[g]))
    if g not in STATE_TYPE:
        return "stateless group: context-owned kind not implemented"
    own = fields <= {g}
    if own:
        cls, kind = "P", "state"
    elif fields <= HUB | {g} and g not in ("catalogs", "config"):
        cls = "C-hub"
        kind = ("hubref" if f["recv"] == "&self" else "hubmut") if g == "core" else \
            "state-hub" if f["recv"] == "&self" else "state-hubmut"
    else:
        return f"class C (non-hub fields {sorted(fields - HUB - {g})})"
    return preconditions(f, cls, kind, classes) or (cls, kind, {"hubref": "HubRef", "hubmut": "HubMut"}
                                                     .get(kind, STATE_TYPE[g]))


def preconditions(f, cls, kind, classes):
    if cls not in classes:
        return f"class {cls} not requested"
    if f["recv"] not in ("&self", "&mut self"):
        return f"unexpected receiver {f['recv']}"
    if f["whole_self"]:
        return "whole-self use (`self` as a value)"
    if f["guard_call"]:
        return "holds a std lock guard across a self method call"
    if f["type_path"]:
        return "`Self::`/`WorldSession::` path in body"
    if f["macro"]:
        return "macro-generated fn"
    return hub_check(f, kind)


def hub_check(f, kind):
    """Precondition failures specific to the hub kinds (also applied when a P fn is upgraded)."""
    if kind in ("hubref", "hubmut", *SH) and f["ret_borrow"]:      # elision would tie it to the wrong borrow
        return "returns a borrow (would borrow a temporary hub view)"
    if kind in ("hubref", "hubmut", *SH) and f.get("writes_store"):
        return "writes catalogs/config (shared in every hub view)"
    if kind in ("hubref", "hubmut", *SH) and "config" in f["acc"] and not f["file"].startswith("session/"):
        return "reads config outside crate::session"
    return None


def upgrade(f, kind, classes):
    """P -> C-hub when a callee needs the hub view (core: HubRef/HubMut, other groups: state-hub)."""
    if f.get("store_rehomed"):                               # it needs the hub after all: back home
        f["target"], f["store_rehomed"] = f["store_from"], False
    if kind != "state" or "C-hub" not in classes or f["target"] in ("catalogs", "config"):
        return None
    if f["target"] not in STATE_TYPE and f["target"] not in FIXTURE_TYPE:
        return "stateless group: context-owned kind not implemented"
    if f["target"] not in ("core", *FIXTURE_TYPE):
        up = "C-hub", "state-hub" if f["recv"] == "&self" else "state-hubmut", STATE_TYPE[f["target"]]
    else:
        up = ("C-hub", "hubref", "HubRef") if f["recv"] == "&self" else ("C-hub", "hubmut", "HubMut")
    return hub_check(f, up[1]) or up


def owner_kind(tname):
    return {"HubRef": "hubref", "HubMut": "hubmut"}.get(tname, "state")


def call_prefix(caller, ctype, callee_type, callee_kind, callee_recv="&self"):
    """Receiver text replacing `self.` for a call from a moved fn, or None if impossible."""
    fg = FIXTURE_OF.get(callee_type)
    if fg and callee_type != ctype:                          # a cfg(test) fixture-group fn
        if caller in ("hubmut", "state-hubmut") or (caller in ("hubref", "state-hub") and callee_recv == "&self"):
            return f"{'hub' if caller in SH else 'self'}.fixtures.{fg}."
        return None
    if caller == "state":
        if callee_type == ctype and callee_kind == "state":
            return "self."
        return None
    if caller in SH:
        mut = caller == "state-hubmut"
        if callee_type == ctype:
            if callee_kind == "state":
                return "self."
            if callee_kind == "state-hub":
                return "self.", "hub.shared()" if mut else "hub"
            return ("self.", "hub") if mut else None
        g = {v: k for k, v in STATE_TYPE.items()}.get(callee_type)
        if g in HUB:
            return f"hub.{g}."
        if callee_type == "HubRef":
            return "hub.shared()." if mut else "hub."
        return "hub." if callee_type == "HubMut" and mut else None
    g = {v: k for k, v in STATE_TYPE.items()}.get(callee_type)
    if g in HUB:
        return f"self.{g}."
    if callee_type == "HubRef":
        return "self." if caller == "hubref" else "self.shared()."
    if callee_type == "HubMut":
        return "self." if caller == "hubmut" else None
    return None
