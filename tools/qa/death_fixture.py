#!/usr/bin/env python3
"""Offline-only #43 death lifecycle fixture preparation and recovery.

The caller must stop every world-server PID and listener under qa-runtime's host
lock before invoking prepare/restore. The acknowledgement flags are assertions
by that guarded caller; this helper does not inspect or control services.
"""

from __future__ import annotations

import argparse
import copy
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from typing import Any

try:
    from capture_fileops import sync_directory, write_journal
except ImportError:  # Imported as tools.qa.death_fixture.
    from tools.qa.capture_fileops import sync_directory, write_journal


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_WORLD_CONFIG = Path("/home/server/trinity-legacy-install/etc/worldserver.conf")
DEFAULT_BOT_CONFIG = ROOT / "tools/wow-test-bot/config.json"
JOURNAL_SCHEMA = "rustycore-death-fixture-v1"
MAX_COUNTER = (1 << 40) - 1
ENCHANTMENT_FIELD_COUNT = 13 * 3
ACK_TEXT = "--ack-disposable-death-fixture"

# Existing character GUIDs are checked and retained, never edited.
ACCOUNTS = (
    {"email": "TESTBOT3@bot.local", "account_id": 10, "old_guid": 16},
    {"email": "TESTBOT5@bot.local", "account_id": 12, "old_guid": 18},
    {"email": "TESTBOT2@bot.local", "account_id": 9, "old_guid": 15},
)


def normalize_testbot_email_rows(rows: list[list[str]]) -> list[list[str]]:
    if not isinstance(rows, (list, tuple)):
        raise FixtureError("database returned a malformed TESTBOT account identity")
    reviewed_emails = {account["email"].lower() for account in ACCOUNTS}
    normalized = []
    for row in rows:
        if (not isinstance(row, (list, tuple)) or len(row) != 2
                or not isinstance(row[0], str) or not row[0].isascii()
                or not row[0].isdecimal() or not isinstance(row[1], str)
                or not row[1].isascii()):
            raise FixtureError("database returned a malformed TESTBOT account identity")
        email = row[1].lower()
        if email not in reviewed_emails:
            raise FixtureError("database returned an unreviewed TESTBOT email identity")
        normalized.append([row[0], email])
    return sorted(normalized, key=lambda row: int(row[0]))


ACTORS = (
    {"name": "Qadeathfall", "account_id": 10, "race": 1, "class": 8,
     "level": 80, "role": "fall", "x": -9340.21, "y": 167.61, "z": 161.60,
     "map": 0, "zone": 12, "health": 1, "power1": 0,
     "home": [-9340.21, 167.61, 61.60]},
    {"name": "Qadeathmob", "account_id": 10, "race": 1, "class": 8,
     "level": 1, "role": "mob", "x": -8783.05, "y": -161.565, "z": 82.0352,
     "map": 0, "zone": 12, "health": 1, "power1": 0,
     "home": [-8783.05, -161.565, 82.0352]},
    {"name": "Qadeathwisp", "account_id": 10, "race": 4, "class": 3,
     "level": 11, "role": "wisp", "x": -9340.21, "y": 167.61, "z": 161.60,
     "map": 0, "zone": 12, "health": 1, "power1": 0,
     "home": [-9340.21, 167.61, 61.60]},
    {"name": "Qadeathreinc", "account_id": 12, "race": 2, "class": 7,
     "level": 80, "role": "reincarnation", "x": 233.264, "y": -4793.62, "z": 110.1872,
     "map": 1, "zone": 14, "health": 1, "power1": 0,
     "home": [233.264, -4793.62, 10.1872]},
    {"name": "Qadeathcast", "account_id": 9, "race": 1, "class": 2,
     "level": 80, "role": "caster", "x": -9340.21, "y": 167.61, "z": 61.60,
     "map": 0, "zone": 12, "health": 100, "power1": 1000,
     "home": [-9340.21, 167.61, 61.60]},
)
ITEM_SEEDS = (
    {"actor": "Qadeathfall", "entry": 2121, "slot": 4, "count": 1, "durability": 40},
    {"actor": "Qadeathfall", "entry": 2121, "slot": 35, "count": 1, "durability": 40},
    {"actor": "Qadeathreinc", "entry": 17030, "slot": 35, "count": 10, "durability": 0},
)
SPELL_SEEDS = (
    {"actor": "Qadeathcast", "spell": 7328},
    {"actor": "Qadeathreinc", "spell": 20608},
    # Night Elf racial Wisp form; ghost-only spell 20584 is not seeded.
    {"actor": "Qadeathwisp", "spell": 20585},
)

# These are explicit owner-key tables from Player::DeleteFromDB/save paths.
# Tables with cross-character/account ownership are deliberately excluded.
CHAR_OWNER_TABLES = (
    ("characters", "guid"), ("character_customizations", "guid"),
    ("character_account_data", "guid"), ("character_declinedname", "guid"),
    ("character_action", "guid"), ("character_arena_stats", "guid"),
    ("character_aura_effect", "guid"), ("character_aura", "guid"),
    ("character_battleground_data", "guid"), ("character_battleground_random", "guid"),
    ("character_cuf_profiles", "guid"), ("character_currency", "CharacterGuid"),
    ("character_gifts", "guid"), ("character_homebind", "guid"),
    ("character_instance_lock", "guid"), ("character_inventory", "guid"),
    ("character_queststatus", "guid"), ("character_queststatus_objectives", "guid"),
    ("character_queststatus_objectives_criteria", "guid"),
    ("character_queststatus_objectives_criteria_progress", "guid"),
    ("character_queststatus_rewarded", "guid"), ("character_reputation", "guid"),
    ("character_spell", "guid"), ("character_spell_cooldown", "guid"),
    ("character_spell_charges", "guid"), ("character_spell_favorite", "guid"),
    ("character_achievement", "guid"),
    ("character_achievement_progress", "guid"), ("character_equipmentsets", "guid"),
    ("character_transmog_outfits", "guid"), ("character_glyphs", "guid"),
    ("character_queststatus_daily", "guid"), ("character_queststatus_weekly", "guid"),
    ("character_queststatus_monthly", "guid"), ("character_queststatus_seasonal", "guid"),
    ("character_talent", "guid"), ("character_skills", "guid"),
    ("character_trait_config", "guid"), ("character_trait_entry", "guid"),
    ("character_stats", "guid"), ("character_fishingsteps", "guid"),
    ("character_favorite_auctions", "guid"), ("character_aura_stored_location", "Guid"),
    ("character_void_storage", "playerGuid"),
    ("corpse", "guid"), ("corpse_customizations", "ownerGuid"),
    ("corpse_phases", "OwnerGuid"),
)
ITEM_OWNER_TABLES = (("item_instance_gems", "itemGuid"), ("item_instance_transmog", "itemGuid"))
ITEM_ID_TABLES = (("item_instance", "guid"), ("character_inventory", "item"),
                  *ITEM_OWNER_TABLES, ("character_void_storage", "itemId"),
                  ("item_loot_items", "container_id"), ("item_loot_money", "container_id"),
                  ("item_refund_instance", "item_guid"),
                  ("item_soulbound_trade_data", "itemGuid"), ("mail_items", "item_guid"),
                  ("auction_items", "itemGuid"))
ITEM_CHILD_TABLES = (*ITEM_OWNER_TABLES, ("item_loot_items", "container_id"),
                     ("item_loot_money", "container_id"))
UNSUPPORTED_OWNER_TABLES = (
    ("character_pet", "owner"), ("mail", "receiver"),
    ("mail", "sender"), ("mail_items", "receiver"),
    ("group_member", "memberGuid"), ("groups", "leaderGuid"),
    ("groups", "looterGuid"), ("groups", "masterLooterGuid"),
    ("guild_member", "guid"), ("guild_member_withdraw", "guid"),
    ("guild_eventlog", "PlayerGuid1"), ("guild_eventlog", "PlayerGuid2"),
    ("guild_bank_eventlog", "PlayerGuid"), ("character_social", "guid"),
    ("character_social", "friend"), ("auctionhouse", "owner"), ("auctionhouse", "bidder"),
)
CHAR_GUID_REFERENCE_TABLES = (*UNSUPPORTED_OWNER_TABLES, ("item_instance", "owner_guid"))
SNAPSHOT_TABLES = (
    ("characters", "guid", "actor"), ("character_homebind", "guid", "actor"),
    ("character_customizations", "guid", "actor"), ("corpse", "guid", "actor"),
    ("corpse_customizations", "ownerGuid", "actor"), ("corpse_phases", "OwnerGuid", "actor"),
    ("character_aura", "guid", "actor"), ("character_aura_effect", "guid", "actor"),
    ("character_currency", "CharacterGuid", "actor"),
    ("character_aura_stored_location", "Guid", "actor"), ("character_inventory", "guid", "actor"),
    ("item_instance", "owner_guid", "actor"), ("item_instance_gems", "itemGuid", "item"),
    ("item_instance_transmog", "itemGuid", "item"), ("character_spell", "guid", "actor"),
    ("character_spell_cooldown", "guid", "actor"), ("character_spell_charges", "guid", "actor"),
    ("character_spell_favorite", "guid", "actor"),
    ("character_queststatus_objectives_criteria", "guid", "actor"),
    ("character_queststatus_objectives_criteria_progress", "guid", "actor"),
    ("character_trait_config", "guid", "actor"), ("character_trait_entry", "guid", "actor"),
    ("character_void_storage", "playerGuid", "actor"),
)


class FixtureError(RuntimeError):
    pass


def sql_quote(value: str) -> str:
    if "\x00" in value:
        raise FixtureError("invalid SQL value")
    return "'" + value.replace("'", "''") + "'"


def checked_counter(value: int) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 1 <= value <= MAX_COUNTER:
        raise FixtureError("reserved identifier is outside the nonzero 40-bit range")
    return value


def target_database_identity(db_info: tuple[str, str, str, str, str]) -> dict[str, str]:
    """Return only stable, noncredential target identity fields."""
    host, port, _user, _password, schema = db_info
    return validate_target_database({"host": host, "port": port, "schema": schema})


def validate_target_database(identity: Any) -> dict[str, str]:
    if (not isinstance(identity, dict) or set(identity) != {"host", "port", "schema"}
            or not all(isinstance(value, str) and value for value in identity.values())
            or not identity["port"].isdigit() or not 1 <= int(identity["port"]) <= 65535
            or not re.fullmatch(r"[A-Za-z0-9_]+", identity["schema"])):
        raise FixtureError("invalid target database identity")
    return {key: identity[key] for key in ("host", "port", "schema")}


def require_same_target_database(db: Any, journal: dict[str, Any]) -> None:
    if validate_target_database(db.target_database) != journal.get("target_database"):
        raise FixtureError("journal target database differs from the configured host, port, or schema")


def parse_db_info(config_path: Path) -> tuple[str, str, str, str, str]:
    """Parse credentials privately; callers must never format this tuple."""
    try:
        text = config_path.read_text(encoding="utf-8")
        values: dict[str, str] = {}
        for line in text.splitlines():
            if line.lstrip().startswith(("#", ";")) or "=" not in line:
                continue
            key, value = line.split("=", 1)
            if key.strip() == "CharacterDatabaseInfo":
                value = value.strip()
                if value.startswith(("\"", "'")):
                    quote = value[0]
                    closing = value.find(quote, 1)
                    if closing < 0 or value[closing + 1:].strip().split("#", 1)[0].strip():
                        raise ValueError
                    value = value[1:closing]
                else:
                    value = value.split("#", 1)[0].strip()
                values[key.strip()] = value
        parts = values["CharacterDatabaseInfo"].split(";")
        if len(parts) != 5 or not all(parts):
            raise ValueError
        host, port, user, password, database = parts
        if not port.isdigit() or not 1 <= int(port) <= 65535 or not re.fullmatch(r"[A-Za-z0-9_]+", database):
            raise ValueError
        return host, port, user, password, database
    except Exception as exc:
        raise FixtureError("could not parse CharacterDatabaseInfo safely") from None


def validate_private_bot_config(path: Path) -> dict[str, Any]:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
        bots = raw["bots"]
        if not isinstance(bots, list):
            raise ValueError
        emails = [bot["account"] for bot in bots]
        if len(emails) != len(set(emails)):
            raise ValueError
        by_email = {bot["account"]: bot for bot in bots}
        for account in ACCOUNTS:
            bot = by_email[account["email"]]
            if (int(bot["account_id"]) != account["account_id"]
                    or int(bot["character_guid"]) != account["old_guid"]
                    or not bot.get("enabled", True)):
                raise ValueError
        return raw
    except Exception:
        # Do not include parser exceptions: malformed files may contain secrets.
        raise FixtureError("private bot config does not match the reviewed TESTBOT identity bindings") from None


def bot_config_for_actor(raw: dict[str, Any], journal: dict[str, Any], actor_name: str) -> dict[str, Any]:
    """Return a one-account in-memory bot config without rewriting credentials."""
    result = copy.deepcopy(raw)
    actor = next((entry for entry in journal["actors"] if entry["name"] == actor_name), None)
    if actor is None:
        raise FixtureError("actor is not in this death fixture journal")
    bots = [bot for bot in result["bots"] if int(bot.get("account_id", -1)) == actor["account_id"]]
    if len(bots) != 1:
        raise FixtureError("private bot config account binding changed")
    bots[0]["character_guid"] = actor["guid"]
    result["bots"] = bots
    return result


class MariaDB:
    def __init__(self, db_info: tuple[str, str, str, str, str], runner=subprocess.run):
        self.host, self.port, self.user, self.password, self.database = db_info
        self.target_database = target_database_identity(db_info)
        self.runner = runner

    def execute(self, statement: str) -> str:
        env = os.environ.copy()
        env["MYSQL_PWD"] = self.password
        # Batch mode escapes embedded tabs/newlines/binary values for safe text parsing.
        args = ["mariadb", "--batch", "--skip-column-names",
                "--host", self.host, "--port", self.port, "--user", self.user,
                "--database", self.database]
        try:
            result = self.runner(args, input=statement, text=True, capture_output=True, env=env)
        except Exception:
            raise FixtureError("MariaDB command could not be started") from None
        if result.returncode != 0:
            # stderr may contain server/config details; never echo it on this path.
            raise FixtureError(f"MariaDB command failed (exit {result.returncode}); recovery journal retained")
        return result.stdout.strip()

    def inspect(self) -> dict[str, Any]:
        ids = ",".join(str(a["account_id"]) for a in ACCOUNTS)
        accounts = self.execute(
            "SELECT a.id,b.email FROM auth.account a JOIN auth.battlenet_accounts b "
            f"ON b.id=a.battlenet_account WHERE a.id IN ({ids}) ORDER BY a.id")
        chars = self.execute(
            "SELECT guid,account,name,online,slot FROM characters WHERE account IN (" + ids + ") ORDER BY account,guid")
        online = self.execute("SELECT COUNT(*) FROM characters WHERE online<>0") or "0"
        names = ",".join(sql_quote(actor["name"]) for actor in ACTORS)
        collisions = self.execute(f"SELECT guid,name FROM characters WHERE name IN ({names}) ORDER BY name")
        reserved_names = self.execute(f"SELECT name,name FROM reserved_name WHERE name IN ({names}) ORDER BY name")
        max_char = max(self._max_rows((*CHAR_OWNER_TABLES, *CHAR_GUID_REFERENCE_TABLES)), default=0)
        max_item = max(self._max_rows(ITEM_ID_TABLES), default=0)
        return {"accounts": self._rows(accounts, 2), "characters": self._rows(chars, 5),
                "online_count": int(online), "name_collisions": self._rows(collisions, 2) +
                    self._rows(reserved_names, 2),
                "max_character_id": max_char, "max_item_id": max_item,
                "unsupported_existing_rows": []}

    def verify_accounts_and_offline(self, journal: dict[str, Any]) -> None:
        ids = ",".join(str(a["account_id"]) for a in ACCOUNTS)
        found = normalize_testbot_email_rows(self._rows(self.execute(
            "SELECT a.id,b.email FROM auth.account a JOIN auth.battlenet_accounts b "
            f"ON b.id=a.battlenet_account WHERE a.id IN ({ids}) ORDER BY a.id"), 2))
        expected = normalize_testbot_email_rows(
            [[str(a["account_id"]), a["email"]] for a in ACCOUNTS])
        selected = ",".join(str(a["account_id"]) for a in ACCOUNTS)
        chars = self._rows(self.execute(
            "SELECT guid,account,online FROM characters WHERE account IN (" +
            selected + ") ORDER BY account,guid"), 3)
        allowed = {(str(a["old_guid"]), str(a["account_id"]), "0") for a in ACCOUNTS}
        allowed.update((str(a["guid"]), str(a["account_id"]), "0") for a in journal["actors"])
        originals_present = all(any(row[0] == str(a["old_guid"])
                                   and row[1] == str(a["account_id"]) and row[2] == "0"
                                   for row in chars) for a in ACCOUNTS)
        if (found != expected or not originals_present or any(tuple(row) not in allowed for row in chars)
                or self.execute("SELECT COUNT(*) FROM characters WHERE online<>0") not in ("", "0")):
            raise FixtureError("account binding or offline state changed; recovery held")

    def character_identities(self, journal: dict[str, Any]) -> list[list[str]]:
        ids = ",".join(str(a["guid"]) for a in journal["actors"])
        return self._rows(self.execute(
            f"SELECT guid,account,name,online FROM characters WHERE guid IN ({ids}) ORDER BY guid"), 4)

    def _max_rows(self, tables: tuple[tuple[str, str], ...]) -> list[int]:
        result = []
        for table, key in tables:
            value = self.execute(f"SELECT COALESCE(MAX({key}),0) FROM {table}") or "0"
            result.append(int(value))
        return result

    def assert_ids_unoccupied(self, char_ids: list[int], item_ids: list[int]) -> None:
        chars = ",".join(str(checked_counter(value)) for value in char_ids)
        items = ",".join(str(checked_counter(value)) for value in item_ids)
        for table, key in CHAR_OWNER_TABLES:
            if self.execute(f"SELECT COUNT(*) FROM {table} WHERE {key} IN ({chars})") not in ("", "0"):
                raise FixtureError("reserved character ID already has an owner-key row")
        for table, key in CHAR_GUID_REFERENCE_TABLES:
            if self.execute(f"SELECT COUNT(*) FROM {table} WHERE {key} IN ({chars})") not in ("", "0"):
                raise FixtureError("reserved character ID already has a linked row")
        for table, key in ITEM_ID_TABLES:
            if self.execute(f"SELECT COUNT(*) FROM {table} WHERE {key} IN ({items})") not in ("", "0"):
                raise FixtureError("reserved item ID already has an owner-key row")

    @staticmethod
    def _rows(text: str, width: int) -> list[list[str]]:
        if not text:
            return []
        rows = [line.split("\t") for line in text.splitlines()]
        if any(len(row) != width for row in rows):
            raise FixtureError("database returned an unexpected row shape")
        return rows

    def rows_under_ids(self, journal: dict[str, Any]) -> dict[str, list[list[str]]]:
        guids = ",".join(str(actor["guid"]) for actor in journal["actors"])
        item_ids = ",".join(str(item["guid"]) for item in journal["items"])
        rows: dict[str, list[list[str]]] = {}
        for table, owner in CHAR_OWNER_TABLES:
            rows[f"{table}.{owner}"] = self._rows(
                self.execute(f"SELECT {owner} FROM {table} WHERE {owner} IN ({guids})"), 1)
        for table, owner in ITEM_CHILD_TABLES:
            rows[f"{table}.{owner}"] = self._rows(
                self.execute(f"SELECT {owner} FROM {table} WHERE {owner} IN ({item_ids})"), 1)
        rows["item_instance.guid"] = self._rows(
            self.execute(f"SELECT guid,owner_guid FROM item_instance WHERE guid IN ({item_ids})"), 2)
        rows["character_inventory.item"] = self._rows(
            self.execute(f"SELECT item,guid FROM character_inventory WHERE item IN ({item_ids})"), 2)
        return rows

    def unsupported_rows(self, journal: dict[str, Any]) -> dict[str, str]:
        guids = ",".join(str(actor["guid"]) for actor in journal["actors"])
        out = {}
        for table, owner in UNSUPPORTED_OWNER_TABLES:
            result = self.execute(f"SELECT COUNT(*) FROM {table} WHERE {owner} IN ({guids})") or "0"
            if int(result):
                out[table] = result
        # Any item found later under a fixture owner but outside its reserved IDs is unsupported.
        result = self.execute(f"SELECT guid,owner_guid FROM item_instance WHERE owner_guid IN ({guids})")
        allowed = {int(item["guid"]) for item in journal["items"]}
        if result:
            unexpected = [row for row in self._rows(result, 2) if int(row[0]) not in allowed]
            if unexpected:
                out["foreign_or_unreserved_item_instance"] = str(len(unexpected))
        result = self.execute(f"SELECT item FROM character_inventory WHERE guid IN ({guids})")
        if result:
            unexpected = [row for row in self._rows(result, 1) if int(row[0]) not in allowed]
            if unexpected:
                out["unreserved_character_inventory"] = str(len(unexpected))
        for table, key in (("mail_items", "item_guid"), ("item_refund_instance", "item_guid"),
                           ("item_soulbound_trade_data", "itemGuid"), ("auction_items", "itemGuid")):
            count = self.execute(f"SELECT COUNT(*) FROM {table} WHERE {key} IN (" +
                                 ",".join(str(value) for value in allowed) + ")") or "0"
            if int(count):
                out[f"unsupported_{table}"] = count
        return out

    def snapshot(self, journal: dict[str, Any]) -> dict[str, dict[str, list[list[str]]]]:
        require_same_target_database(self, journal)
        ids = ",".join(str(actor["guid"]) for actor in journal["actors"])
        item_ids = ",".join(str(item["guid"]) for item in journal["items"])
        result = {}
        for table, owner, kind in SNAPSHOT_TABLES:
            key = item_ids if kind == "item" else ids
            columns = [line.split("\t", 1)[0]
                       for line in self.execute(f"SHOW COLUMNS FROM {table}").splitlines()]
            rows = [line.split("\t") for line in self.execute(
                f"SELECT * FROM {table} WHERE {owner} IN ({key}) ORDER BY {owner}").splitlines() if line]
            result[table] = {"columns": columns, "rows": rows}
        return result

    def transaction(self, statements: list[str]) -> None:
        self.execute("START TRANSACTION;\n" + ";\n".join(statements) + ";\nCOMMIT")


def validate_inspection(info: dict[str, Any]) -> None:
    expected_accounts = normalize_testbot_email_rows(
        [[str(a["account_id"]), a["email"]] for a in ACCOUNTS])
    actual_accounts = normalize_testbot_email_rows(info["accounts"])
    if actual_accounts != expected_accounts:
        raise FixtureError("configured account IDs do not match their TESTBOT email bindings")
    # Existing rows may have arbitrary names; require exact old GUID/account and offline state.
    actual = sorted((int(row[0]), int(row[1]), row[3]) for row in info["characters"])
    expected = sorted((a["old_guid"], a["account_id"], "0") for a in ACCOUNTS)
    if actual != expected or info["online_count"] != 0:
        raise FixtureError("selected accounts must retain only their exact offline original characters")
    if info["name_collisions"]:
        raise FixtureError("one or more proposed death fixture names already exist")
    if info["unsupported_existing_rows"]:
        raise FixtureError("selected existing characters have unsupported linked rows; refusing fixture admission")


def assign_slots(info: dict[str, Any]) -> list[int]:
    occupied: dict[int, set[int]] = {a["account_id"]: set() for a in ACCOUNTS}
    for row in info["characters"]:
        occupied[int(row[1])].add(int(row[4]))
    slots = []
    for actor in ACTORS:
        available = next((slot for slot in range(10) if slot not in occupied[actor["account_id"]]), None)
        if available is None:
            raise FixtureError("selected account has no free character slot")
        slots.append(available)
        occupied[actor["account_id"]].add(available)
    return slots


def allocate_ids(info: dict[str, Any]) -> tuple[list[int], list[int]]:
    first_char = int(info["max_character_id"]) + 1
    first_item = int(info["max_item_id"]) + 1
    char_ids = [checked_counter(first_char + offset) for offset in range(len(ACTORS))]
    item_ids = [checked_counter(first_item + offset) for offset in range(len(ITEM_SEEDS))]
    return char_ids, item_ids


def seed_record(char_ids: list[int], item_ids: list[int], slots: list[int],
                target_database: dict[str, str]) -> dict[str, Any]:
    target_database = validate_target_database(target_database)
    actors = []
    for seed, guid, slot in zip(ACTORS, char_ids, slots, strict=True):
        actors.append({**seed, "guid": checked_counter(guid), "slot": slot})
    items = []
    actor_guid = {actor["name"]: actor["guid"] for actor in actors}
    for seed, guid in zip(ITEM_SEEDS, item_ids, strict=True):
        items.append({**seed, "guid": checked_counter(guid), "owner_guid": actor_guid[seed["actor"]]})
    return {"journal_schema": JOURNAL_SCHEMA, "state": "reserved",
            "target_database": dict(target_database), "actors": actors,
            "items": items, "account_bindings": [{"email": a["email"], "account_id": a["account_id"],
                                                    "preserved_guid": a["old_guid"]} for a in ACCOUNTS],
            "prior_row_absence": {"character_ids": char_ids, "item_ids": item_ids,
                "character_owner_tables": [f"{table}.{owner}" for table, owner in CHAR_OWNER_TABLES],
                "character_reference_tables": [f"{table}.{owner}" for table, owner in CHAR_GUID_REFERENCE_TABLES],
                "item_id_tables": [f"{table}.{owner}" for table, owner in ITEM_ID_TABLES]},
            "seed_facts": {"gender": 0, "at_login": 0, "extra_flags": 0,
                "playerFlags": 0, "inventorySlots": 16, "customizations": "empty; source-valid",
                "homebind": "actor map/zone and ground position",
                "item_slots": {"chest": 4, "backpack_start": 35},
                "spells": list(SPELL_SEEDS), "world_stopped_asserted": True}}


def validate_journal(journal: dict[str, Any], path: Path) -> None:
    if (not isinstance(journal, dict)
            or journal.get("journal_schema") != JOURNAL_SCHEMA
            or journal.get("journal") != str(path)
            or not isinstance(journal.get("actors"), list)
            or len(journal["actors"]) != len(ACTORS)
            or not isinstance(journal.get("items"), list)
            or len(journal["items"]) != len(ITEM_SEEDS)):
        raise FixtureError("unrecognized death fixture journal")
    validate_target_database(journal.get("target_database"))
    if [a.get("name") for a in journal["actors"]] != [a["name"] for a in ACTORS]:
        raise FixtureError("death fixture journal identity drifted")
    if journal.get("state") not in {"prepare-attempted", "prepared", "restore-attempted", "restored"}:
        raise FixtureError("death fixture journal state is invalid")
    actor_ids = []
    for expected, actor in zip(ACTORS, journal["actors"], strict=True):
        if any(actor.get(key) != value for key, value in expected.items()):
            raise FixtureError("death fixture actor identity drifted")
        if not isinstance(actor.get("slot"), int) or not 0 <= actor["slot"] < 10:
            raise FixtureError("death fixture character slot is invalid")
        actor_ids.append(checked_counter(actor.get("guid")))
    item_ids = []
    for expected, item in zip(ITEM_SEEDS, journal["items"], strict=True):
        if any(item.get(key) != value for key, value in expected.items()):
            raise FixtureError("death fixture item identity drifted")
        actor = next(a for a in journal["actors"] if a["name"] == expected["actor"])
        if item.get("owner_guid") != actor["guid"]:
            raise FixtureError("death fixture item owner drifted")
        item_ids.append(checked_counter(item.get("guid")))
    if len(set(actor_ids)) != len(actor_ids) or len(set(item_ids)) != len(item_ids):
        raise FixtureError("death fixture journal IDs are not unique")
    absence = journal.get("prior_row_absence", {})
    if (absence.get("character_ids") != actor_ids or absence.get("item_ids") != item_ids
            or absence.get("character_owner_tables") != [f"{t}.{o}" for t, o in CHAR_OWNER_TABLES]
            or absence.get("character_reference_tables") != [f"{t}.{o}" for t, o in CHAR_GUID_REFERENCE_TABLES]
            or absence.get("item_id_tables") != [f"{t}.{o}" for t, o in ITEM_ID_TABLES]):
        raise FixtureError("death fixture journal reservation record is invalid")
    expected_accounts = [{"email": a["email"], "account_id": a["account_id"],
                          "preserved_guid": a["old_guid"]} for a in ACCOUNTS]
    if journal.get("account_bindings") != expected_accounts:
        raise FixtureError("death fixture account identity drifted")
    if "cleanup_snapshot" in journal and not isinstance(journal["cleanup_snapshot"], dict):
        raise FixtureError("death fixture cleanup snapshot is invalid")


def make_character_insert(actor: dict[str, Any]) -> str:
    cols = ("`guid`,`account`,`name`,`slot`,`race`,`class`,`gender`,`level`,`xp`,`money`,"
            "`inventorySlots`,`playerFlags`,`position_x`,`position_y`,`position_z`,`map`,"
            "`instance_id`,`orientation`,`taximask`,`online`,`at_login`,`extra_flags`,`zone`,"
            "`health`,`power1`")
    values = (actor["guid"], actor["account_id"], sql_quote(actor["name"]), actor["slot"], actor["race"],
              actor["class"], 0, actor["level"], 0, 0, 16, 0, actor["x"], actor["y"], actor["z"],
              actor["map"], 0, 0, sql_quote("0 0 0 0 0 0 0 0 0 0 0 0"), 0, 0, 0,
              actor["zone"], actor["health"], actor["power1"])
    return f"INSERT INTO characters ({cols}) VALUES ({','.join(map(str, values))})"


def seed_statements(journal: dict[str, Any]) -> list[str]:
    out = [make_character_insert(actor) for actor in journal["actors"]]
    out.extend(
        "INSERT INTO character_homebind (guid,mapId,zoneId,posX,posY,posZ,orientation) "
        f"VALUES ({actor['guid']},{actor['map']},{actor['zone']},{actor['home'][0]},"
        f"{actor['home'][1]},{actor['home'][2]},0)" for actor in journal["actors"])
    # Empty, source-valid customization rows are intentionally omitted.
    for item in journal["items"]:
        # Item::SaveToDB emits thirteen slots with three zero fields per slot.
        enchantments = " ".join("0" for _ in range(ENCHANTMENT_FIELD_COUNT))
        out.append("INSERT INTO item_instance (guid,itemEntry,owner_guid,creatorGuid,giftCreatorGuid,count,"
                   "duration,charges,flags,enchantments,durability,playedTime,randomPropertiesId,"
                   "randomPropertiesSeed,context) VALUES ("
                   f"{item['guid']},{item['entry']},{item['owner_guid']},0,0,{item['count']},0,'',0,"
                   f"{sql_quote(enchantments)},{item['durability']},0,0,0,0)")
        out.append("INSERT INTO character_inventory (guid,bag,slot,item) "
                   f"VALUES ({item['owner_guid']},0,{item['slot']},{item['guid']})")
    for spell in SPELL_SEEDS:
        actor = next(a for a in journal["actors"] if a["name"] == spell["actor"])
        out.append(f"INSERT INTO character_spell (guid,spell,active,disabled) "
                   f"VALUES ({actor['guid']},{spell['spell']},1,0)")
    return out


def verify_fresh_journal_path(path: Path) -> None:
    parent = path.parent
    marker = Path(str(path) + ".cleanup-complete")
    if (not path.is_absolute() or parent.is_symlink() or not parent.is_dir()
            or parent.resolve(strict=True) != parent or parent.stat().st_mode & 0o777 != 0o700
            or path.exists() or path.is_symlink() or marker.exists() or marker.is_symlink()):
        raise FixtureError("journal must be new inside a canonical mode-0700 directory")


def create_journal(path: Path, record: dict[str, Any]) -> None:
    verify_fresh_journal_path(path)
    record["journal"] = str(path)
    # Shared helper uses O_EXCL, mode 0600, fsync(file), then fsync(directory).
    write_journal(path, record, create=True)


def read_journal(path: Path) -> dict[str, Any]:
    if (path.is_symlink() or not path.is_file() or path.resolve(strict=True) != path
            or path.parent.is_symlink() or path.parent.resolve(strict=True) != path.parent
            or path.parent.stat().st_mode & 0o777 != 0o700
            or path.stat().st_mode & 0o777 != 0o600):
        raise FixtureError("death fixture journal is missing or unsafe")
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        raise FixtureError("death fixture journal is malformed") from None
    validate_journal(data, path)
    return data


def require_offline_acknowledgement(ack: bool, world_stopped: bool) -> None:
    if not ack or not world_stopped:
        raise FixtureError(f"prepare/restore requires {ACK_TEXT} and --world-stopped")


def prepare(db: Any, config_raw: dict[str, Any], journal_path: Path, *,
            ack: bool = False, world_stopped: bool = False) -> dict[str, Any]:
    require_offline_acknowledgement(ack, world_stopped)
    info = db.inspect()
    validate_inspection(info)
    char_ids, item_ids = allocate_ids(info)
    db.assert_ids_unoccupied(char_ids, item_ids)
    record = seed_record(char_ids, item_ids, assign_slots(info), db.target_database)
    record["state"] = "prepare-attempted"
    create_journal(journal_path, record)
    # Recheck identities, slots, names and reservations after the durable journal exists.
    latest = db.inspect()
    validate_inspection(latest)
    latest_char_ids, latest_item_ids = allocate_ids(latest)
    if (latest_char_ids != char_ids or latest_item_ids != item_ids
            or assign_slots(latest) != [actor["slot"] for actor in record["actors"]]):
        raise FixtureError("offline identity or reserved IDs changed; recovery journal retained")
    db.assert_ids_unoccupied(char_ids, item_ids)
    # Persist the attempt state before SQL so a lost COMMIT is always recoverable.
    db.transaction(seed_statements(record))
    owned = db.rows_under_ids(record)
    if ({int(row[0]) for row in owned.get("characters.guid", [])}
            != {int(actor["guid"]) for actor in record["actors"]}
            or {int(row[0]) for row in owned.get("item_instance.guid", [])}
            != {int(item["guid"]) for item in record["items"]}
            or {int(row[0]) for row in owned.get("character_inventory.item", [])}
            != {int(item["guid"]) for item in record["items"]}):
        raise FixtureError("seed transaction outcome is not verified; recovery journal retained")
    expected_owners = {int(item["guid"]): int(item["owner_guid"]) for item in record["items"]}
    for row in owned["item_instance.guid"]:
        if int(row[0]) not in expected_owners or int(row[1]) != expected_owners[int(row[0])]:
            raise FixtureError("seeded item owner differs from journal; recovery journal retained")
    expected_characters = {int(actor["guid"]): (int(actor["account_id"]), actor["name"])
                           for actor in record["actors"]}
    actual_characters = {int(guid): (int(account_id), name)
                         for guid, account_id, name, online in db.character_identities(record)
                         if online == "0"}
    if actual_characters != expected_characters or db.unsupported_rows(record):
        raise FixtureError("seed transaction identity or owned-row verification failed; journal retained")
    record["state"] = "prepared"
    write_journal(journal_path, record, create=False)
    return record


def cleanup_statements(journal: dict[str, Any]) -> list[str]:
    ids = ",".join(str(a["guid"]) for a in journal["actors"])
    item_ids = ",".join(str(item["guid"]) for item in journal["items"])
    out = [f"DELETE FROM {table} WHERE {owner} IN ({item_ids})" for table, owner in ITEM_CHILD_TABLES]
    # Remove inventory references before their item rows. restore() holds if an
    # actor has any inventory item outside this fixture's reserved IDs.
    out.append(f"DELETE FROM character_inventory WHERE item IN ({item_ids}) AND guid IN ({ids})")
    out.append(f"DELETE FROM item_instance WHERE guid IN ({item_ids}) AND owner_guid IN ({ids})")
    # Corpse ownership is the character GUID, never the runtime corpse-object GUID.
    for table, owner in reversed(CHAR_OWNER_TABLES):
        # Inventory was deleted above with both its exact item and character IDs.
        if table == "character_inventory":
            continue
        out.append(f"DELETE FROM {table} WHERE {owner} IN ({ids})")
    return out


def restore(db: Any, journal_path: Path, *, ack: bool = False, world_stopped: bool = False) -> None:
    require_offline_acknowledgement(ack, world_stopped)
    journal = read_journal(journal_path)
    require_same_target_database(db, journal)
    marker = Path(str(journal_path) + ".cleanup-complete")
    db.verify_accounts_and_offline(journal)
    if marker.exists() or marker.is_symlink():
        if marker.is_symlink() or not marker.is_file():
            raise FixtureError("cleanup marker is unsafe")
        try:
            marker_record = json.loads(marker.read_text(encoding="utf-8"))
        except Exception:
            raise FixtureError("cleanup marker is malformed") from None
        if marker_record != {"journal": str(journal_path), "state": "cleanup-complete"}:
            raise FixtureError("cleanup marker does not match this journal")
        if any(db.rows_under_ids(journal).values()) or db.unsupported_rows(journal):
            raise FixtureError("cleanup marker exists but owned rows remain; recovery held")
        return
    unsupported = db.unsupported_rows(journal)
    if unsupported:
        raise FixtureError("unsupported linked rows found; journal retained and cleanup held")
    rows = db.rows_under_ids(journal)
    expected = {int(a["guid"]): (int(a["account_id"]), a["name"]) for a in journal["actors"]}
    for guid, account_id, name, online in db.character_identities(journal):
        if int(guid) not in expected or (int(account_id), name) != expected[int(guid)] or online != "0":
            raise FixtureError("fixture character ownership or offline state drifted; cleanup held")
    allowed_items = {int(item["guid"]): int(item["owner_guid"]) for item in journal["items"]}
    for row in rows.get("item_instance.guid", []):
        if int(row[0]) not in allowed_items or int(row[1]) != allowed_items[int(row[0])]:
            raise FixtureError("item owner identity drifted; cleanup held")
    for row in rows.get("character_inventory.item", []):
        if int(row[0]) not in allowed_items or int(row[1]) != allowed_items[int(row[0])]:
            raise FixtureError("inventory owner identity drifted; cleanup held")
    if "cleanup_snapshot" not in journal:
        journal["cleanup_snapshot"] = db.snapshot(journal)
    journal["state"] = "restore-attempted"
    write_journal(journal_path, journal, create=False)
    db.transaction(cleanup_statements(journal))
    remaining = db.rows_under_ids(journal)
    if any(remaining.values()) or db.unsupported_rows(journal):
        raise FixtureError("owned rows remain after restore; journal retained")
    journal["state"] = "restored"
    write_journal(journal_path, journal, create=False)
    import tempfile
    fd, temp_name = tempfile.mkstemp(prefix=".death-fixture-cleanup-", dir=marker.parent)
    temp = Path(temp_name)
    with os.fdopen(fd, "w", encoding="utf-8") as stream:
        stream.write(json.dumps({"journal": str(journal_path), "state": "cleanup-complete"}, sort_keys=True) + "\n")
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temp, marker)
    sync_directory(marker.parent)


def safe_plan(db: Any) -> dict[str, Any]:
    info = db.inspect()
    validate_inspection(info)
    char_ids, item_ids = allocate_ids(info)
    db.assert_ids_unoccupied(char_ids, item_ids)
    slots = assign_slots(info)
    return {"mode": "read-only-plan", "target_database": dict(db.target_database),
            "account_bindings": [
                {"email": a["email"], "account_id": a["account_id"], "preserved_character_guid": a["old_guid"]}
                for a in ACCOUNTS],
            "proposed_actors": [{**a, "proposed_guid": guid, "proposed_slot": slot}
                                for a, guid, slot in zip(ACTORS, char_ids, slots)],
            "proposed_item_guids": item_ids, "current_facts": info,
            "runtime_selection": {"in_memory_only": True, "credential_config_rewritten": False,
                                  "accounts": sorted({a["account_id"] for a in ACTORS})}}


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", nargs="?", choices=("plan", "snapshot", "prepare", "restore"), default="plan")
    parser.add_argument("--bot-config", type=Path, default=DEFAULT_BOT_CONFIG)
    parser.add_argument("--world-config", type=Path, default=DEFAULT_WORLD_CONFIG)
    parser.add_argument("--journal", type=Path)
    parser.add_argument(ACK_TEXT, dest="ack", action="store_true")
    parser.add_argument("--world-stopped", action="store_true",
                        help="caller assertion only; the parent runtime guard must verify every PID and port")
    return parser


def main(argv: list[str] | None = None, db_factory=MariaDB) -> int:
    args = build_parser().parse_args(argv)
    try:
        bot_raw = validate_private_bot_config(args.bot_config)
        db = db_factory(parse_db_info(args.world_config))
        if args.mode == "plan":
            result = safe_plan(db)
        elif args.mode == "snapshot":
            if not args.journal:
                raise FixtureError("snapshot requires --journal")
            journal = read_journal(args.journal)
            require_same_target_database(db, journal)
            result = {"mode": "read-only-snapshot", "state": journal.get("state"),
                      "actors": [{"name": a["name"], "guid": a["guid"], "account_id": a["account_id"]}
                                 for a in journal["actors"]], "owned_state": db.snapshot(journal)}
        else:
            if not args.ack or not args.world_stopped:
                raise FixtureError(f"{args.mode} requires {ACK_TEXT} and --world-stopped")
            if not args.journal:
                raise FixtureError(f"{args.mode} requires --journal")
            if args.mode == "prepare":
                prepared = prepare(db, bot_raw, args.journal,
                                   ack=args.ack, world_stopped=args.world_stopped)
                result = {"mode": "prepared", "journal": str(args.journal),
                          "actors": [{"name": a["name"], "guid": a["guid"], "account_id": a["account_id"]}
                                     for a in prepared["actors"]]}
            else:
                restore(db, args.journal, ack=args.ack, world_stopped=args.world_stopped)
                result = {"mode": "restored", "journal": str(args.journal), "verified": True}
        print(json.dumps(result, sort_keys=True, indent=2))
        return 0
    except FixtureError as exc:
        print(str(exc), file=sys.stderr)
        return 2
    except Exception:
        # Unexpected exceptions can carry private config or client diagnostics.
        print("death fixture operation failed; inspect the private journal before retrying", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
