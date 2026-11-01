#!/usr/bin/env python3
"""Model for f3_move_methods.py (#1241 F3): groups, owner types, patterns and the move-kind rules.

Pure functions over a scanned fn record; no I/O. Standard library only.
"""
from __future__ import annotations

import collections
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


CX_CAP = 3
CX = ("cx", "cx-ref")                                       # capped group context: own + <=3 sibling states + hub
CX_SIBLINGS = {}                                             # group -> chosen siblings (set per plan)


def cx_type(group, kind):
    stem = "".join(part.capitalize() for part in group.split("_"))
    return f"{stem}Cx" if kind == "cx" else f"{stem}CxRef"


def is_owner_type(tname):
    return tname in OWNER_TYPES or tname.endswith(("Cx", "CxRef"))


def cx_kind(f, classes, fields):
    """The capped-context kind when every non-hub field is the group's own or a chosen sibling's."""
    g = f["target"]
    if "Cx" not in classes or g not in CX_SIBLINGS or g in HUB or not CX_SIBLINGS[g]:
        return None
    extra = fields - HUB - {g}
    if not extra <= set(CX_SIBLINGS[g]):
        return f"class C (exceeds the Cx cap: needs {sorted(extra - set(CX_SIBLINGS[g]))})"
    kind = "cx-ref" if f["recv"] == "&self" else "cx"
    return preconditions(f, "Cx", kind, classes) or ("Cx", kind, cx_type(g, kind))


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
        if not fields <= HUB:                                # a fixture domain reaching sibling states
            return cx_kind(f, classes, fields) or f"class C (non-hub fields {sorted(fields - HUB)})"
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
        return cx_kind(f, classes, fields) or f"class C (non-hub fields {sorted(fields - HUB - {g})})"
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
    if kind in ("hubref", "hubmut", *SH, *CX) and f["ret_borrow"]:      # elision would tie it to the wrong borrow
        return "returns a borrow (would borrow a temporary hub view)"
    if kind in ("hubref", "hubmut", *SH, *CX) and f.get("writes_store"):
        return "writes catalogs/config (shared in every hub view)"
    if kind in ("hubref", "hubmut", *SH, *CX) and "config" in f["acc"] and not f["file"].startswith("session/"):
        return "reads config outside crate::session"
    return None


def upgrade(f, kind, classes):
    """P -> C-hub when a callee needs the hub view (core: HubRef/HubMut, other groups: state-hub)."""
    if f.get("store_rehomed"):                               # it needs the hub after all: back home
        f["target"], f["store_rehomed"] = f["store_from"], False
    if kind in (*SH, "hubref", "hubmut") and "Cx" in classes and f["target"] in CX_SIBLINGS:  # needs a sibling
        return cx_kind(f, classes, set(f["acc"])) if isinstance(cx_kind(f, classes, set(f["acc"])), tuple) else None
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
    if tname and tname.endswith("CxRef"):
        return "cx-ref"
    if tname and tname.endswith("Cx"):
        return "cx"
    return {"HubRef": "hubref", "HubMut": "hubmut"}.get(tname, "state")


def call_prefix(caller, ctype, callee_type, callee_kind, callee_recv="&self"):
    """Receiver text replacing `self.` for a call from a moved fn, or None if impossible."""
    if caller in CX:
        return cx_call_prefix(caller, ctype, callee_type, callee_kind, callee_recv)
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


def cx_call_prefix(caller, ctype, callee_type, callee_kind, callee_recv):
    """Receivers inside `impl <G>Cx<'_>`: own/sibling states by field, the hub through `self.hub`."""
    mut = caller == "cx"
    if callee_kind in CX:                                    # another fn of the same Cx family
        if callee_type == ctype:
            return "self."
        if callee_type.startswith(ctype.removesuffix("Ref")) and mut and callee_kind == "cx-ref":
            return "self.shared()."
        return None
    fg = FIXTURE_OF.get(callee_type)
    if fg:
        return f"self.hub.fixtures.{fg}." if mut or callee_recv == "&self" else None
    group = {v: k for k, v in STATE_TYPE.items()}.get(callee_type)
    if group in HUB:
        return f"self.hub.{group}."
    if callee_type == "HubRef":
        return "self.hub.shared()." if mut else "self.hub."
    if callee_type == "HubMut":
        return "self.hub." if mut else None
    family = ctype.removesuffix("CxRef").removesuffix("Cx")
    if group is None or group in FIXTURE_TYPE or (group != family_group(family)
                                                  and group not in CX_SIBLINGS.get(family_group(family), ())):
        return None
    if callee_kind == "state":
        return f"self.{group}." if mut or callee_recv == "&self" else None
    if callee_kind == "state-hub":
        return f"self.{group}.", "self.hub.shared()" if mut else "self.hub"
    return (f"self.{group}.", "&mut self.hub") if mut else None


def family_group(stem):
    """`LootCx`/`WorldEntitiesCx` stem -> group name."""
    return "".join("_" + ch.lower() if ch.isupper() else ch for ch in stem).lstrip("_")


def choose_cx_siblings(code, live, loc, groups_wanted, classes, override):
    """Per group: an existing Cx keeps its siblings; else the <=CX_CAP most-needed sibling states."""
    CX_SIBLINGS.clear()
    if "Cx" not in classes:
        return
    hub_text = code.get("session/state/hub.rs", "")
    by_type = {v: k for k, v in STATE_TYPE.items()}
    for g in sorted(groups_wanted):
        if (g not in STATE_TYPE and g not in FIXTURE_TYPE) or g in HUB:
            continue
        have = []                                        # members an existing Cx already reads
        for variant in ("cx", "cx-ref"):
            m = re.search(r"pub\(crate\) struct " + cx_type(g, variant) + r"<'a> \{(.*?)\n\}", hub_text, re.S)
            if m:
                have += [x for x in re.findall(r"pub\(crate\) (\w+):", m.group(1)) if x not in (g, "hub")]
        if have:                                         # union of both variants (members are trimmed)
            CX_SIBLINGS[g] = tuple(dict.fromkeys(have))[:CX_CAP]
        elif g in override:
            CX_SIBLINGS[g] = tuple(override[g])[:CX_CAP]
        else:
            need = collections.Counter()
            for f in live:
                if f["target"] == g:
                    need.update(x for x in set(f["acc"]) - HUB - {g} if x in STATE_TYPE)
                    need.update(by_type[loc[c]] for c in f["calls"] if loc.get(c) in by_type
                                and by_type[loc[c]] not in HUB | {g})
            CX_SIBLINGS[g] = tuple(sorted(x for x, _n in need.most_common(CX_CAP)))


def type_path(tname, rel):
    if tname in HUB_TYPES or tname.endswith(("Cx", "CxRef")):
        return f"crate::session::{tname}"
    return f"crate::session::state::{tname}" if rel.startswith("session/") else f"crate::session::{tname}"
