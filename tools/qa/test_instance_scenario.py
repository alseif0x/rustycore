"""Hermetic #45 solo instance scenario checks."""

from __future__ import annotations

import copy
import json
import os
import struct
import sys
import tempfile
import time
from pathlib import Path
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parent))

import death_fixture
import death_scenario
import instance_scenario as scenario


def actor() -> dict:
    selected = copy.deepcopy(next(item for item in death_fixture.ACTORS
                                    if item["name"] == scenario.ACTOR_NAME))
    selected["guid"] = 73
    return selected


def member_actor() -> dict:
    selected = copy.deepcopy(next(item for item in death_fixture.ACTORS
                                    if item["name"] == scenario.MEMBER_NAME))
    selected["guid"] = 74
    return selected


def identity_row(selected: dict, online: str = "0") -> list[str]:
    return [str(selected["guid"]), str(selected["account_id"]), selected["name"], online]


def position_row(selected: dict, map_id: int, instance_id: int,
                 position: tuple[float, float, float], orientation: float = 0.0) -> str:
    x, y, z = position
    return "\t".join([
        str(selected["guid"]), str(selected["account_id"]), selected["name"], "0",
        str(map_id), str(instance_id), str(x), str(y), str(z), str(orientation),
    ])


def inside_state(selected: dict, instance_id: int = 41) -> dict:
    x, y, z = scenario.INSTANCE_POSITION
    return {
        "guid": selected["guid"], "online": 0, "logout_time": 1,
        "map": scenario.INSTANCE_MAP, "instance_id": instance_id,
        "position_x": x, "position_y": y, "position_z": z, "orientation": 0.0,
    }


def outside_state(selected: dict) -> dict:
    x, y, z = selected["home"]
    return {
        "guid": selected["guid"], "online": 0, "logout_time": 1,
        "map": selected["map"], "instance_id": 0,
        "position_x": x, "position_y": y, "position_z": z, "orientation": 0.0,
    }


def sent_reset(sequence: int = 10) -> dict:
    return {
        "sequence": 1, "observation_sequence": sequence, "action_index": 0,
        "action": "reset_instances", "connection": "instance",
        "opcode": scenario.CMSG_RESET_INSTANCES, "body_hex": "",
        "write_attempted": True, "write_completed": True, "write_error": None,
    }


def reset_event(sequence: int = 11, body: bytes | None = None,
                connection: str = "realm") -> dict:
    if body is None:
        body = scenario.INSTANCE_MAP.to_bytes(4, "little")
    return {
        "sequence": 2, "observation_sequence": sequence, "connection": connection,
        "opcode": scenario.SMSG_INSTANCE_RESET, "body_hex": body.hex(),
    }


class InstancePlanTests(unittest.TestCase):
    def test_observer_and_reset_plans_use_native_logout_and_exact_map_body(self) -> None:
        selected = actor()
        for plan in (scenario.observer_plan(selected), scenario.reset_plan(selected, 1)):
            self.assertEqual(plan["character_guid"], selected["guid"])
            self.assertEqual(plan["finish_policy"], "logout")
        self.assertTrue(scenario.observer_plan(selected)["observe_only"])
        disconnect = scenario.disconnect_observer_plan(selected)
        self.assertTrue(disconnect["observe_only"])
        self.assertEqual(disconnect["finish_policy"], "disconnect")
        self.assertEqual(disconnect["actions"], [])
        plan = scenario.reset_plan(selected, 1)
        self.assertFalse(plan.get("observe_only", False))
        self.assertEqual(plan["timeout_ms"], 45_000)
        self.assertEqual(plan["actions"], [
            {"action": "cast", "spell_id": 8690,
             "cast_id": {"low": 8_690_001, "high": 0},
             "target": death_scenario.player_guid(selected, 1)},
            {"action": "wait_for_packet", "connection": "realm",
             "opcode": scenario.SMSG_NEW_WORLD, "timeout_ms": 30_000},
            {"action": "wait", "milliseconds": 2_000},
            {"action": "reset_instances"},
            {"action": "wait_for_packet", "connection": "realm",
             "opcode": scenario.SMSG_INSTANCE_RESET, "timeout_ms": 5_000},
        ])
        self.assertEqual(plan["expect"][0], {
            "event": "packet", "connection": "realm",
            "opcode": 0x2686, "body_hex": "21000000",
        })
        self.assertEqual(plan["expect"][1]["connection"], "realm")
        self.assertEqual(plan["expect"][1]["opcode"], scenario.LOGOUT_COMPLETE)
        self.assertEqual(scenario.CMSG_RESET_INSTANCES, 0x366A)
        self.assertEqual(scenario.INSTANCE_POSITION, (-228.191, 2111.41, 76.8904))


class OfflineSqlTests(unittest.TestCase):
    def test_relocate_is_actor_scoped_exact_and_does_not_copy_facing_as_radians(self) -> None:
        selected = actor()
        db = Mock()
        db.character_identities.return_value = [identity_row(selected)]
        db.execute.return_value = position_row(selected, 33, 0, scenario.INSTANCE_POSITION)

        state = scenario.relocate_inside(db, {}, selected)

        statement = db.transaction.call_args.args[0][0]
        self.assertIn("UPDATE characters SET map=33,instance_id=0", statement)
        self.assertIn("position_x=-228.191,position_y=2111.41,position_z=76.8904", statement)
        self.assertIn("orientation=0.0", statement)
        self.assertNotIn("70", statement.split("WHERE", 1)[0])
        self.assertNotIn("zone=", statement)
        self.assertIn(f"guid={selected['guid']}", statement)
        self.assertIn(f"account={selected['account_id']}", statement)
        self.assertIn("online=0", statement)
        self.assertNotIn("group", statement.lower())
        self.assertNotIn("lock", statement.lower())
        self.assertEqual(state["map"], 33)
        self.assertEqual(state["instance_id"], 0)

    def test_restore_uses_only_journal_home_and_zero_instance(self) -> None:
        selected = actor()
        db = Mock()
        db.character_identities.return_value = [identity_row(selected)]
        db.execute.return_value = position_row(selected, selected["map"], 0, selected["home"])

        state = scenario.restore_outside(db, {}, selected)

        statement = db.transaction.call_args.args[0][0]
        self.assertIn(f"map={selected['map']},instance_id=0", statement)
        self.assertIn("orientation=0.0", statement)
        self.assertNotIn("zone=", statement)
        self.assertEqual(state["map"], selected["map"])
        self.assertEqual(state["instance_id"], 0)

    def test_offline_identity_and_exact_row_guards_abort_mutation_or_verification(self) -> None:
        selected = actor()
        for row in (
            identity_row(selected, "1"),
            [str(selected["guid"]), "999", selected["name"], "0"],
            [str(selected["guid"]), str(selected["account_id"]), "Other", "0"],
            [],
        ):
            db = Mock()
            db.character_identities.return_value = [row] if row else []
            with self.subTest(row=row), self.assertRaises(scenario.ScenarioFailure):
                scenario.relocate_inside(db, {}, selected)
            db.transaction.assert_not_called()

        db = Mock()
        db.character_identities.return_value = [identity_row(selected)]
        db.execute.return_value = position_row(selected, 33, 99, scenario.INSTANCE_POSITION)
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.relocate_inside(db, {}, selected)


class InstanceStateTests(unittest.TestCase):
    def test_reconnect_and_full_relogin_allow_any_valid_current_instance(self) -> None:
        selected = actor()
        initial = inside_state(selected, 41)
        for current_id in (41, 42):
            with self.subTest(reconnect_id=current_id):
                self.assertEqual(
                    scenario.validate_reconnect_state(initial, inside_state(selected, current_id)),
                    current_id,
                )
            with self.subTest(full_relogin_id=current_id):
                self.assertEqual(
                    scenario.validate_full_relogin_state(initial, inside_state(selected, current_id)),
                    current_id,
                )

        for defect in ("zero", "map", "position", "orientation"):
            bad = inside_state(selected, 42)
            if defect == "zero":
                bad["instance_id"] = 0
            elif defect == "map":
                bad["map"] = 0
            elif defect == "position":
                bad["position_x"] += 1.0
            else:
                bad["orientation"] = 70.0
            with self.subTest(defect=defect), self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_reconnect_state(initial, bad)
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_full_relogin_state(initial, inside_state(selected, 0))

    def test_online_unsaved_zero_is_allowed_but_offline_zero_and_wrong_id_fail(self) -> None:
        selected = member_actor()
        unsaved = inside_state(selected, 0)
        unsaved["online"] = 1
        self.assertEqual(
            scenario.validate_inside_state(unsaved, 41, allow_unsaved_online=True),
            41,
        )
        scenario.validate_lock_inside_state(
            lock_inside_state(selected, 0) | {"online": 1}, 42,
            allow_unsaved_online=True,
        )

        offline_zero = inside_state(selected, 0)
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_inside_state(offline_zero, 41, allow_unsaved_online=True)
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_lock_inside_state(
                lock_inside_state(selected, 0), 42,
                allow_unsaved_online=True,
            )
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_inside_state(unsaved | {"instance_id": 42}, 41,
                                           allow_unsaved_online=True)
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_lock_inside_state(
                lock_inside_state(selected, 99) | {"online": 1}, 42,
                allow_unsaved_online=True,
            )

    def test_final_position_must_be_outside_journal_home(self) -> None:
        selected = actor()
        scenario.validate_outside_state(selected, outside_state(selected))
        bad = inside_state(selected)
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_outside_state(selected, bad)


class ResetEvidenceTests(unittest.TestCase):
    def test_one_typed_reset_and_exact_response_are_required(self) -> None:
        evidence = {
            "sent_packets": [sent_reset()],
            "events": [reset_event()],
        }
        facts = scenario.validate_reset_evidence(evidence)
        self.assertEqual(facts["map_id"], 33)
        self.assertLess(facts["request_sequence"], facts["response_sequence"])

        for defect in ("missing_request", "duplicate_request", "missing_response",
                       "duplicate_response", "wrong_body", "wrong_connection", "early"):
            bad = copy.deepcopy(evidence)
            if defect == "missing_request":
                bad["sent_packets"] = []
            elif defect == "duplicate_request":
                bad["sent_packets"].append(sent_reset(12))
            elif defect == "missing_response":
                bad["events"] = []
            elif defect == "duplicate_response":
                bad["events"].append(reset_event(12))
            elif defect == "wrong_body":
                bad["events"] = [reset_event(body=33 .to_bytes(4, "big"))]
            elif defect == "wrong_connection":
                bad["events"] = [reset_event(connection="instance")]
            else:
                bad["events"] = [reset_event(sequence=9)]
            with self.subTest(defect=defect), self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_reset_evidence(bad)


class NativeResetEvidenceTests(unittest.TestCase):
    def test_native_hearthstone_worldport_precedes_reset(self) -> None:
        evidence = {
            "sent_packets": [
                typed("cast", "instance", scenario.CMSG_CAST_SPELL, "", 10),
                sent_reset(20),
            ],
            "events": [
                group_event("realm", scenario.SMSG_NEW_WORLD, "", 15),
                reset_event(21),
            ],
        }
        facts = scenario.validate_solo_reset_evidence(evidence)
        self.assertEqual(facts["hearthstone_spell_id"], 8690)
        self.assertTrue(facts["native_worldport"])
        self.assertLess(facts["cast_sequence"], facts["teleport_sequence"])

        for defect in ("missing_cast", "missing_teleport", "late_teleport"):
            bad = copy.deepcopy(evidence)
            if defect == "missing_cast":
                bad["sent_packets"] = bad["sent_packets"][1:]
            elif defect == "missing_teleport":
                bad["events"] = bad["events"][1:]
            else:
                bad["events"][0] = group_event("realm", scenario.SMSG_NEW_WORLD, "", 25)
            with self.subTest(defect=defect), self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_solo_reset_evidence(bad)


class RunCaseTests(unittest.TestCase):
    def test_report_records_all_passes_lifecycle_and_position_restoration(self) -> None:
        selected = actor()
        identity = {"capture": Path("/capture")}
        journal = {"actors": [selected]}
        entry = inside_state(selected, 41)
        reconnect = inside_state(selected, 41)
        full_relogin = inside_state(selected, 42)
        reset_state = outside_state(selected)
        final = outside_state(selected)
        reset_evidence = {
            "sent_packets": [
                typed("cast", "instance", scenario.CMSG_CAST_SPELL, "", 10),
                sent_reset(20),
            ],
            "events": [
                group_event("realm", scenario.SMSG_NEW_WORLD, "", 15),
                reset_event(21),
            ],
        }
        with tempfile.TemporaryDirectory() as raw:
            paths = []
            for label in ("reset", "entry", "reconnect", "relogin", "final"):
                path = Path(raw) / f"{label}.json"
                path.write_text("{}\n", encoding="utf-8")
                paths.append(path)
            with patch.object(scenario, "relocate_inside") as relocate, \
                    patch.object(scenario, "restore_outside") as restore, \
                    patch.object(scenario.logout_scenario, "_run_single",
                                 side_effect=[reset_evidence, {"lifecycle": "entry"},
                                              {"lifecycle": "disconnect"},
                                              {"lifecycle": "reconnect"}, {"lifecycle": "relogin"}]), \
                    patch.object(scenario, "_save_position_snapshot",
                                 side_effect=[({}, reset_state, paths[0]), ({}, entry, paths[1]),
                                              ({}, reconnect, paths[2]),
                                              ({}, full_relogin, paths[3]), ({}, final, paths[4])]):
                report = scenario._run_case(identity, journal, 1, Mock())
        self.assertEqual(report["scenario"], "instance-solo-reset")
        self.assertEqual(len(report["observations"]), 5)
        self.assertTrue(report["observations"][0]["native_worldport"])
        self.assertEqual(report["observations"][0]["hearthstone_spell_id"], 8690)
        self.assertEqual(report["observations"][1]["instance_id"], 41)
        self.assertTrue(report["observations"][2]["disconnect_reconnect"])
        self.assertNotIn("snapshot_sha256", report["observations"][2])
        self.assertEqual(report["observations"][3]["reconnect_instance_id"], 41)
        self.assertNotIn("reconnect_same_instance", report["observations"][3])
        self.assertEqual(report["observations"][4]["full_relogin_instance_id"], 42)
        self.assertEqual(report["final_state"], final)
        self.assertTrue(report["position_restoration"]["outside_home"])
        self.assertFalse(report["position_restoration"]["group_or_lock_rows_mutated"])
        self.assertEqual(relocate.call_count, 2)
        restore.assert_called_once()

    def test_failure_stops_owned_bots_and_restores_owned_position(self) -> None:
        selected = actor()
        bot = object()
        with patch.object(scenario, "relocate_inside"), \
                patch.object(scenario, "restore_outside") as restore, \
                patch.object(scenario.logout_scenario, "_run_single",
                             side_effect=RuntimeError("stop")), \
                patch.object(scenario.death_scenario, "ACTIVE_BOTS", [bot]), \
                patch.object(scenario.death_scenario, "_poll_actor_offline"), \
                patch.object(scenario.death_scenario, "_stop_owned") as stop:
            with self.assertRaisesRegex(RuntimeError, "stop"):
                scenario._run_case({"capture": Path("/capture")}, {"actors": [selected]}, 1, Mock())
        stop.assert_called_once_with(bot)
        restore.assert_called_once()

    def test_failure_report_is_private_and_versioned(self) -> None:
        report = scenario._failure_report(RuntimeError("secret-password"), scenario.SCENARIO)
        self.assertEqual(report["schema"], "rustycore-instance-scenario-v1")
        self.assertFalse(report["parity_assessed"])
        self.assertNotIn("secret-password", json.dumps(report))


def typed(action: str, connection: str, opcode: int, body: str, sequence: int = 10) -> dict:
    return {
        "sequence": 1, "observation_sequence": sequence, "action_index": 0,
        "action": action, "connection": connection, "opcode": opcode, "body_hex": body,
        "write_attempted": True, "write_completed": True, "write_error": None,
    }


def group_event(connection: str, opcode: int, body: str = "", sequence: int = 11) -> dict:
    return {"sequence": 2, "observation_sequence": sequence,
            "connection": connection, "opcode": opcode, "body_hex": body}


def party_command_result(name: str, command: int = 2, result: int = 0,
                         sequence: int = 11, connection: str = "realm") -> dict:
    encoded_name = name.encode("utf-8")
    header = ((len(encoded_name) << 15) | (command << 11) | (result << 5)).to_bytes(3, "big")
    body = header + b"\x00" * 6 + encoded_name
    return group_event(connection, scenario.SMSG_PARTY_COMMAND_RESULT, body.hex(), sequence)


class GroupPlanTests(unittest.TestCase):
    def test_invite_occupied_failure_and_leave_plans_use_native_routes(self) -> None:
        leader = actor()
        member = member_actor()
        member_plan = scenario.member_invite_plan(member)
        self.assertEqual(member_plan["finish_policy"], "logout")
        self.assertEqual(member_plan["actions"][0]["connection"], "realm")
        self.assertEqual(member_plan["actions"][0]["opcode"], scenario.SMSG_PARTY_INVITE)
        self.assertEqual(member_plan["actions"][1],
                         {"action": "party_invite_response", "accept": True})
        self.assertEqual(member_plan["actions"][2]["opcode"], scenario.SMSG_PARTY_UPDATE)
        self.assertEqual([member_plan["actions"][index]["timeout_ms"] for index in (0, 2)],
                         [30_000, 30_000])

        leader_plan = scenario.leader_invite_plan(leader, member, 1)
        self.assertEqual(leader_plan["actions"][0], {
            "action": "party_invite", "name": member["name"],
            "target": death_scenario.player_guid(member, 1),
        })
        self.assertEqual(leader_plan["actions"][1]["connection"], "realm")
        self.assertEqual(leader_plan["actions"][1]["timeout_ms"], 30_000)
        for plan in (member_plan, leader_plan):
            self.assertEqual(plan["timeout_ms"], 45_000)
            self.assertEqual(
                [rule["opcode"] for rule in plan["expect"]],
                [scenario.LOGOUT_COMPLETE],
            )

        occupied = scenario.member_occupied_plan(member)
        self.assertEqual(occupied["actions"], [{"action": "wait", "milliseconds": 90_000}])
        self.assertEqual(occupied["expect"], [
            {"event": "packet", "connection": "realm",
             "opcode": scenario.SMSG_RESET_FAILED_NOTIFY, "body_hex": ""},
        ])
        self.assertEqual(occupied["finish_policy"], "disconnect")

        failed = scenario.reset_failed_plan(leader)
        self.assertEqual(failed["expect"][0]["body_hex"], "2100000000")
        self.assertEqual(failed["expect"][0]["opcode"], 0x2687)
        leave = scenario.leave_group_plan(leader)
        self.assertEqual(leave["actions"], [
            {"action": "leave_group"},
            {"action": "wait_for_packet", "connection": "realm",
             "opcode": scenario.SMSG_PARTY_COMMAND_RESULT, "timeout_ms": 5_000},
        ])
        self.assertEqual(leave["finish_policy"], "logout")


class GroupOwnershipTests(unittest.TestCase):
    def test_preflight_requires_absent_rows_and_cleanup_stays_actor_scoped(self) -> None:
        leader = actor()
        member = member_actor()
        actors = (leader, member)
        journal = {"actors": list(actors)}
        db = Mock()
        db.character_identities.side_effect = lambda _journal: [identity_row(leader), identity_row(member)]
        db.execute.side_effect = ["", ""]
        preflight = scenario.preflight_group(db, journal, actors)
        self.assertTrue(preflight["cleanup_allowed"])

        db.execute.side_effect = ["", ""]
        scenario.cleanup_group(db, actors, preflight)
        statements = db.transaction.call_args.args[0]
        self.assertEqual(statements, [
            f"DELETE FROM group_member WHERE memberGuid IN ({leader['guid']},{member['guid']})",
            f"DELETE FROM groups WHERE leaderGuid IN ({leader['guid']},{member['guid']})",
        ])
        self.assertFalse(any("lock" in statement.lower() for statement in statements))

        db.execute.reset_mock(side_effect=True)
        db.execute.side_effect = [str(leader["guid"]), ""]
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.preflight_group(db, journal, actors)
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.cleanup_group(db, actors, {})


class GroupResetEvidenceTests(unittest.TestCase):
    def test_occupied_failure_notify_non_expulsion_and_empty_success(self) -> None:
        leader = actor()
        member = member_actor()
        failed = {
            "sent_packets": [typed("reset_instances", "instance", scenario.CMSG_RESET_INSTANCES, "")],
            "events": [group_event("realm", scenario.SMSG_INSTANCE_RESET_FAILED, "2100000000")],
        }
        facts = scenario.validate_reset_failed_evidence(failed)
        self.assertEqual(facts["reason"], 0)
        notify = {"events": [group_event("realm", scenario.SMSG_RESET_FAILED_NOTIFY)]}
        self.assertEqual(scenario.validate_reset_failed_notify_evidence(notify)["body_bytes"], 0)
        scenario.validate_inside_state(inside_state(member, 41), 41)

        success = {"sent_packets": [sent_reset()], "events": [reset_event()]}
        self.assertEqual(scenario.validate_reset_evidence(success)["map_id"], 33)
        leave = {
            "sent_packets": [typed("leave_group", "realm", scenario.CMSG_LEAVE_GROUP, "")],
            "events": [party_command_result(leader["name"])],
        }
        self.assertEqual(
            scenario.validate_leave_group_evidence(leave, leader["name"])["result"], 0)

        bad = copy.deepcopy(failed)
        bad["events"].append(group_event("realm", scenario.SMSG_INSTANCE_RESET_FAILED, "2100000000", 12))
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_reset_failed_evidence(bad)
        bad_notify = copy.deepcopy(notify)
        bad_notify["events"].append(group_event("realm", scenario.SMSG_RESET_FAILED_NOTIFY, "", 12))
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_reset_failed_notify_evidence(bad_notify)

    def test_leave_requires_ordered_successful_party_command_result(self) -> None:
        leader = actor()
        self.assertEqual(
            party_command_result(scenario.ACTOR_NAME)["body_hex"],
            "0590000000000000005161646561746863617374")
        request = typed("leave_group", "realm", scenario.CMSG_LEAVE_GROUP, "", sequence=10)
        valid = {"sent_packets": [request],
                 "events": [party_command_result(leader["name"], sequence=11)]}
        facts = scenario.validate_leave_group_evidence(valid, leader["name"])
        self.assertEqual(facts["command"], 2)
        self.assertEqual(facts["result"], 0)
        self.assertEqual(facts["name"], leader["name"])
        self.assertLess(facts["request_sequence"], facts["response_sequence"])

        malformed = copy.deepcopy(valid)
        malformed["events"][0]["body_hex"] = malformed["events"][0]["body_hex"][:-2]
        missing_request_order = copy.deepcopy(valid)
        del missing_request_order["sent_packets"][0]["observation_sequence"]
        cases = [
            ("missing", {"sent_packets": [request], "events": []}),
            ("malformed", malformed),
            ("missing request sequence", missing_request_order),
            ("wrong command", {"sent_packets": [request], "events": [
                party_command_result(leader["name"], command=1, sequence=11)]}),
            ("wrong result", {"sent_packets": [request], "events": [
                party_command_result(leader["name"], result=1, sequence=11)]}),
            ("wrong party", {"sent_packets": [request], "events": [
                party_command_result("OtherLeader", sequence=11)]}),
            ("wrong connection", {"sent_packets": [request], "events": [
                party_command_result(leader["name"], sequence=11, connection="instance")]}),
            ("unordered", {"sent_packets": [request], "events": [
                party_command_result(leader["name"], sequence=9)]}),
        ]
        for label, evidence in cases:
            with self.subTest(label=label), self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_leave_group_evidence(evidence, leader["name"])

    def test_formation_requires_invite_accept_and_two_party_updates_on_both(self) -> None:
        leader = actor()
        member = member_actor()
        leader_evidence = {
            "sent_packets": [typed("party_invite", "realm", 0x3604, "builder")],
            "events": [
                group_event("realm", scenario.SMSG_PARTY_UPDATE, "01"),
                group_event("realm", scenario.SMSG_PARTY_UPDATE, "02", 12),
            ],
        }
        member_evidence = {
            "sent_packets": [typed("party_invite_response", "realm", 0x3606, "40")],
            "events": [
                group_event("realm", scenario.SMSG_PARTY_UPDATE, "03"),
                group_event("realm", scenario.SMSG_PARTY_UPDATE, "04", 12),
            ],
        }
        scenario.validate_group_formation_evidence(leader_evidence, member_evidence)
        for count in (1, 2, 3):
            leader = copy.deepcopy(leader_evidence)
            member = copy.deepcopy(member_evidence)
            leader["events"] = [
                group_event("realm", scenario.SMSG_PARTY_UPDATE) for _ in range(count)
            ]
            member["events"] = [
                group_event("realm", scenario.SMSG_PARTY_UPDATE) for _ in range(count)
            ]
            with self.subTest(valid_count=count):
                scenario.validate_group_formation_evidence(leader, member)
        for count in (0,):
            bad_leader = copy.deepcopy(leader_evidence)
            bad_member = copy.deepcopy(member_evidence)
            bad_leader["events"] = [
                group_event("realm", scenario.SMSG_PARTY_UPDATE) for _ in range(count)
            ]
            bad_member["events"] = [
                group_event("realm", scenario.SMSG_PARTY_UPDATE) for _ in range(count)
            ]
            with self.subTest(invalid_count=count), self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_group_formation_evidence(bad_leader, bad_member)


class GroupFailureRestorationTests(unittest.TestCase):
    def test_failure_after_relocation_restores_both_positions_and_only_created_group(self) -> None:
        leader = actor()
        member = member_actor()
        actors = (leader, member)
        journal = {"actors": list(actors)}
        leader_evidence = {
            "sent_packets": [typed("party_invite", "realm", 0x3604, "builder")],
            "events": [
                group_event("realm", scenario.SMSG_PARTY_UPDATE),
                group_event("realm", scenario.SMSG_PARTY_UPDATE, "", 12),
            ],
        }
        member_evidence = {
            "sent_packets": [typed("party_invite_response", "realm", 0x3606, "40")],
            "events": [
                group_event("realm", scenario.SMSG_PARTY_UPDATE),
                group_event("realm", scenario.SMSG_PARTY_UPDATE, "", 12),
            ],
        }
        with tempfile.TemporaryDirectory() as raw:
            snapshot = Path(raw) / "snapshot.json"
            snapshot.write_text("{}\n", encoding="utf-8")
            with patch.object(scenario, "preflight_group", return_value={"cleanup_allowed": True}), \
                    patch.object(scenario, "cleanup_group") as cleanup, \
                    patch.object(scenario, "relocate_inside"), \
                    patch.object(scenario, "restore_outside") as restore, \
                    patch.object(scenario, "_group_snapshot",
                                 side_effect=[({}, outside_state(leader), snapshot),
                                              ({}, outside_state(member), snapshot)]), \
                    patch.object(scenario.death_scenario, "start_bot",
                                 side_effect=[None, RuntimeError("member start failed")]), \
                    patch.object(scenario.logout_scenario, "_run_single",
                                 return_value=leader_evidence), \
                    patch.object(scenario.death_scenario, "finish_bot",
                                 return_value=member_evidence), \
                    patch.object(scenario.death_scenario, "ACTIVE_BOTS", [object()]), \
                    patch.object(scenario.death_scenario, "_stop_owned") as stop, \
                    patch.object(scenario.death_scenario, "_poll_actor_offline"):
                with self.assertRaisesRegex(RuntimeError, "member start failed"):
                    scenario._run_group_case(
                        {"capture": Path(raw), "bot_exec": Path("/bot"),
                         "bot_config": Path(raw) / "config.json"},
                        journal, 1, Mock())
        self.assertEqual(restore.call_count, 2)
        cleanup.assert_called_once()
        stop.assert_called_once()


class DifficultyPlanTests(unittest.TestCase):
    def test_difficulty_plan_body_and_reset_packet_exclusion(self) -> None:
        leader = actor()
        plan = scenario.difficulty_plan(leader)
        self.assertEqual(plan["actions"][0],
                         {"action": "set_dungeon_difficulty", "difficulty_id": 2})
        self.assertEqual(plan["actions"][1]["connection"], "realm")
        self.assertEqual([packet["opcode"] for packet in plan["expect"]],
                         [scenario.LOGOUT_COMPLETE])
        self.assertEqual(plan["finish_policy"], "logout")
        member_plan = scenario.member_difficulty_plan(member_actor())
        self.assertEqual(member_plan["actions"], [{"action": "wait", "milliseconds": 90_000}])
        self.assertEqual([packet["opcode"] for packet in member_plan["expect"]],
                         [scenario.LOGOUT_COMPLETE])
        self.assertEqual(member_plan["finish_policy"], "logout")

        response = group_event("realm", scenario.SMSG_SET_DUNGEON_DIFFICULTY, "02000000")
        evidence = {
            "sent_packets": [typed("set_dungeon_difficulty", "instance", 0x3684, "02000000")],
            "events": [response, copy.deepcopy(response)],
        }
        self.assertEqual(scenario.validate_difficulty_evidence(evidence)["difficulty_id"], 2)
        scenario.validate_no_reset_events(evidence)
        scenario.validate_login_difficulty(evidence)
        for bad_events in ([], [group_event(
                "realm", scenario.SMSG_SET_DUNGEON_DIFFICULTY, "01000000")]):
            bad = copy.deepcopy(evidence)
            bad["events"] = bad_events
            with self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_difficulty_evidence(bad)
            with self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_login_difficulty(bad)
        bad = copy.deepcopy(evidence)
        bad["events"].append(group_event("realm", scenario.SMSG_INSTANCE_RESET, "21000000", 12))
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_no_reset_events(bad)


class DifficultyPersistenceTests(unittest.TestCase):
    def test_group_difficulty_requires_exact_group_and_members(self) -> None:
        leader = actor()
        member = member_actor()
        db = Mock()
        db.execute.side_effect = ["7\t73\t2", "73\n74"]
        facts = scenario.require_group_difficulty(db, (leader, member))
        self.assertEqual(facts["difficulty"], 2)
        self.assertEqual(facts["members"], [73, 74])
        db.execute.side_effect = ["7\t73\t1", "73\n74"]
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.require_group_difficulty(db, (leader, member))
        db.execute.side_effect = ["7\t73\t2", "73"]
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.require_group_difficulty(db, (leader, member))

    def test_epoch_dispatch_and_current_sharing_do_not_require_old_instance_id(self) -> None:
        leader = actor()
        member = member_actor()
        identity = {"scenario": scenario.DIFFICULTY_SCENARIO, "epoch": 2}
        with patch.object(scenario, "_run_difficulty_case", return_value={"epoch": 2}) as run:
            report = scenario._run_case(identity, {"actors": [leader, member]}, 1, Mock())
        self.assertEqual(report["epoch"], 2)
        run.assert_called_once()
        current = inside_state(member, 42)
        self.assertEqual(scenario.validate_inside_state(current, 42), 42)
        scenario.validate_inside_state(inside_state(leader, 42), 42)


class DifficultyRestartTests(unittest.TestCase):
    def test_epoch_two_success_cleans_group_and_restores_positions(self) -> None:
        leader = actor()
        member = member_actor()
        journal = {"actors": [leader, member]}
        login = {
            "events": [group_event("realm", scenario.SMSG_SET_DUNGEON_DIFFICULTY, "02000000")]
        }
        leave = {
            "sent_packets": [typed("leave_group", "realm", scenario.CMSG_LEAVE_GROUP, "")],
            "events": [party_command_result(leader["name"])],
        }
        order = []

        def run_single(bot_pass, _identity, observer=False):
            order.append(bot_pass.label)
            return leave if bot_pass.label == "instance-difficulty-leave-001" else login

        def relocate(_db, _journal, selected):
            order.append(f"relocate:{selected['name']}")

        with tempfile.TemporaryDirectory() as raw:
            paths = []
            for label in ("member", "leader", "leave", "final"):
                path = Path(raw) / f"{label}.json"
                path.write_text("{}\n", encoding="utf-8")
                paths.append(path)
            with patch.object(scenario, "require_group_difficulty",
                              return_value={"difficulty": 2}), \
                    patch.object(scenario, "_require_offline"), \
                    patch.object(scenario.logout_scenario, "_run_single",
                                 side_effect=run_single), \
                    patch.object(scenario, "relocate_inside", side_effect=relocate), \
                    patch.object(scenario, "_difficulty_snapshot",
                                 side_effect=[
                                     ({}, inside_state(member, 42), paths[0]),
                                     ({}, inside_state(leader, 42), paths[1]),
                                     ({}, outside_state(leader), paths[2]),
                                     ({}, outside_state(member), paths[3]),
                                 ]), \
                    patch.object(scenario, "restore_outside") as restore, \
                    patch.object(scenario, "cleanup_group") as cleanup, \
                    patch.object(scenario, "_group_rows_absent", return_value=True) as rows_absent:
                report = scenario._run_difficulty_case(
                    {"scenario": scenario.DIFFICULTY_SCENARIO, "epoch": 2,
                     "capture": Path(raw)}, journal, 1, Mock())
        self.assertEqual(report["epoch"], 2)
        self.assertEqual(report["restart_persistence"]["current_shared_instance"], 42)
        self.assertTrue(report["restart_persistence"]["old_instance_id_not_required"])
        self.assertTrue(report["group_cleanup"]["scenario_group_rows_removed"])
        self.assertTrue(report["group_cleanup"]["server_leave_rows_absent"])
        self.assertEqual(order[:3], [
            "instance-difficulty-member-restart-001", f"relocate:{leader['name']}",
            "instance-difficulty-leader-restart-001",
        ])
        self.assertEqual(restore.call_count, 2)
        rows_absent.assert_called_once()
        cleanup.assert_not_called()

    def test_epoch_two_retained_group_rows_fail_before_recovery_cleanup(self) -> None:
        leader = actor()
        member = member_actor()
        journal = {"actors": [leader, member]}
        login = {
            "events": [group_event("realm", scenario.SMSG_SET_DUNGEON_DIFFICULTY, "02000000")]
        }
        leave = {
            "sent_packets": [typed("leave_group", "realm", scenario.CMSG_LEAVE_GROUP, "")],
            "events": [party_command_result(leader["name"])],
        }
        order = []

        def run_single(bot_pass, _identity, observer=False):
            order.append(bot_pass.label)
            return leave if bot_pass.label == "instance-difficulty-leave-001" else login

        def rows_remain(_db, _actors):
            order.append("group_rows_present")
            return False

        def recover_group(_db, _actors, _preflight):
            order.append("recovery_cleanup")

        with tempfile.TemporaryDirectory() as raw:
            paths = []
            for label in ("member", "leader"):
                path = Path(raw) / f"{label}.json"
                path.write_text("{}\n", encoding="utf-8")
                paths.append(path)
            with patch.object(scenario, "require_group_difficulty",
                              return_value={"difficulty": 2}), \
                    patch.object(scenario, "_require_offline"), \
                    patch.object(scenario.logout_scenario, "_run_single", side_effect=run_single), \
                    patch.object(scenario, "relocate_inside"), \
                    patch.object(scenario, "_difficulty_snapshot",
                                 side_effect=[
                                     ({}, inside_state(member, 42), paths[0]),
                                     ({}, inside_state(leader, 42), paths[1]),
                                 ]), \
                    patch.object(scenario, "restore_outside"), \
                    patch.object(scenario, "_group_rows_absent", side_effect=rows_remain), \
                    patch.object(scenario, "cleanup_group", side_effect=recover_group), \
                    patch.object(scenario.death_scenario, "_poll_actor_offline"), \
                    patch.object(scenario.death_scenario, "ACTIVE_BOTS", []), \
                    patch.object(scenario.time, "monotonic", side_effect=[0.0, 0.0, 5.0]), \
                    patch.object(scenario.time, "sleep"):
                with self.assertRaisesRegex(
                        scenario.ScenarioFailure, "accepted leader leave did not remove"):
                    scenario._run_difficulty_case(
                        {"scenario": scenario.DIFFICULTY_SCENARIO, "epoch": 2,
                         "capture": Path(raw)}, journal, 1, Mock())

        self.assertLess(order.index("instance-difficulty-leave-001"),
                        order.index("group_rows_present"))
        self.assertLess(order.index("group_rows_present"), order.index("recovery_cleanup"))
        self.assertEqual(order.count("recovery_cleanup"), 1)

    def test_epoch_two_failure_stops_bots_and_cleans_group(self) -> None:
        leader = actor()
        member = member_actor()
        with patch.object(scenario, "require_group_difficulty",
                          return_value={"difficulty": 2}), \
                patch.object(scenario, "_require_offline"), \
                patch.object(scenario.logout_scenario, "_run_single",
                             side_effect=RuntimeError("restart failed")), \
                patch.object(scenario, "restore_outside") as restore, \
                patch.object(scenario, "cleanup_group") as cleanup, \
                patch.object(scenario.death_scenario, "ACTIVE_BOTS", [object()]), \
                patch.object(scenario.death_scenario, "_stop_owned") as stop, \
                patch.object(scenario.death_scenario, "_poll_actor_offline"):
            with self.assertRaisesRegex(RuntimeError, "restart failed"):
                scenario._run_difficulty_case(
                    {"scenario": scenario.DIFFICULTY_SCENARIO, "epoch": 2,
                     "capture": Path("/capture")}, {"actors": [leader, member]}, 1, Mock())
        self.assertEqual(restore.call_count, 2)
        cleanup.assert_called_once()
        stop.assert_called_once()


def lock_inside_state(selected: dict, instance_id: int = 42) -> dict:
    x, y, z = scenario.LOCK_POSITION
    return {
        "guid": selected["guid"], "online": 0, "logout_time": 1,
        "map": scenario.LOCK_MAP, "instance_id": instance_id,
        "position_x": x, "position_y": y, "position_z": z, "orientation": 0.0,
    }


def lock_row(selected: dict, *, instance_id: int = 42, data: str | None = None,
             completed_mask: int = 1, expiry: int | None = None, extended: int = 0) -> str:
    if data is None:
        data = scenario.ICC_SHARED_SAVE_DATA
    if expiry is None:
        expiry = int(time.time()) + 3600
    return "\t".join([
        str(selected["guid"]), str(scenario.LOCK_MAP), str(scenario.LOCK_ID),
        str(instance_id), str(scenario.LOCK_DIFFICULTY), data, str(completed_mask),
        "0", str(expiry), str(extended),
    ])


def instance_row(instance_id: int = 42) -> str:
    return "\t".join([str(instance_id), scenario.ICC_SHARED_SAVE_DATA, "1", "0"])


def calendar_event(instance_id: int = 42, sequence: int = 40) -> dict:
    # 2026-01-02, Friday, 12:34 in the packed WoW calendar timestamp.
    packed_time = (26 << 24) | (1 << 14) | (5 << 11) | (12 << 6) | 34
    body = struct.pack("<QIiIi", instance_id, packed_time, scenario.LOCK_MAP,
                       scenario.LOCK_DIFFICULTY, 30 * 24 * 60 * 60)
    return group_event("realm", scenario.SMSG_CALENDAR_RAID_LOCKOUT_ADDED,
                       body.hex(), sequence)


def lock_bind_evidence(*, accept: bool, extending: bool = False,
                       instance_id: int = 42) -> dict:
    prompt = group_event("realm", scenario.SMSG_PENDING_RAID_LOCK,
                         scenario.pending_lock_body(extending=extending).hex(), 10)
    response = typed("instance_lock_response", "realm", scenario.CMSG_INSTANCE_LOCK_RESPONSE,
                     "80" if accept else "00", 20)
    events = [prompt]
    if accept:
        events.extend([
            group_event("realm", scenario.SMSG_INSTANCE_SAVE_CREATED, "00", 30),
            calendar_event(instance_id, 40),
        ])
    return {"sent_packets": [response], "events": events}


def raid_info_evidence(*, instance_id: int = 42, mask: int = 1,
                       flags: int = 0x80, response_sequence: int = 20) -> dict:
    body = (struct.pack("<I", 1)
            + struct.pack("<IIQII", scenario.LOCK_MAP, scenario.LOCK_DIFFICULTY,
                          instance_id, 86_400, mask)
            + bytes([flags]))
    return {
        "sent_packets": [typed("request_raid_info", "realm",
                               scenario.CMSG_REQUEST_RAID_INFO, "", 10)],
        "events": [group_event("realm", scenario.SMSG_INSTANCE_INFO,
                               body.hex(), response_sequence)],
    }


class LockPlanAndWireTests(unittest.TestCase):
    def test_member_plan_captures_rejection_and_accepted_publication(self) -> None:
        member = member_actor()
        rejected = scenario.member_lock_plan(member, False)
        self.assertEqual(rejected["timeout_ms"], 70_000)
        self.assertEqual(rejected["actions"][0], {
            "action": "wait_for_packet", "connection": "realm",
            "opcode": scenario.SMSG_PENDING_RAID_LOCK, "timeout_ms": 30_000,
        })
        self.assertEqual(rejected["actions"][1], {
            "action": "instance_lock_response", "accept": False,
        })
        self.assertEqual(rejected["actions"][2], {"action": "wait", "milliseconds": 1_000})
        self.assertEqual(rejected["expect"][0]["body_hex"],
                         scenario.pending_lock_body(extending=False).hex())
        scenario.validate_lock_response_evidence(lock_bind_evidence(accept=False), accept=False)

        accepted = scenario.member_lock_plan(member, True)
        self.assertEqual([action.get("opcode") for action in accepted["actions"][2:]],
                         [scenario.SMSG_INSTANCE_SAVE_CREATED,
                          scenario.SMSG_CALENDAR_RAID_LOCKOUT_ADDED])
        facts = scenario.validate_lock_response_evidence(
            lock_bind_evidence(accept=True), accept=True, expected_instance_id=42)
        self.assertEqual(facts["calendar_lockout"]["instance_id"], 42)
        self.assertEqual(facts["save_created_sequence"], 30)
        self.assertEqual(facts["calendar_lockout"]["sequence"], 40)

    def test_member_bind_evidence_rejects_wrong_route_payload_and_order(self) -> None:
        bad = lock_bind_evidence(accept=True)
        bad["sent_packets"][0]["connection"] = "instance"
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_lock_response_evidence(bad, accept=True)

        bad = lock_bind_evidence(accept=True)
        bad["events"][1]["observation_sequence"] = 15
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_lock_response_evidence(bad, accept=True)

        bad = lock_bind_evidence(accept=False)
        bad["events"].append(group_event("realm", scenario.SMSG_INSTANCE_SAVE_CREATED, "00", 30))
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_lock_response_evidence(bad, accept=False)

        bad = lock_bind_evidence(accept=False)
        bad["events"].append(calendar_event(sequence=30))
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_lock_response_evidence(bad, accept=False)

    def test_expired_extended_leader_auto_confirms_without_client_response(self) -> None:
        leader = actor()
        plan = scenario.leader_auto_lock_plan(leader)
        self.assertEqual(plan["timeout_ms"], 130_000)
        self.assertEqual(plan["actions"], [
            {"action": "wait_for_packet", "connection": "realm",
             "opcode": scenario.SMSG_PENDING_RAID_LOCK, "timeout_ms": 30_000},
            {"action": "wait_for_packet", "connection": "realm",
             "opcode": scenario.SMSG_CALENDAR_RAID_LOCKOUT_ADDED, "timeout_ms": 90_000},
        ])
        self.assertEqual(plan["expect"][0]["body_hex"],
                         scenario.pending_lock_body(extending=True).hex())
        auto_evidence = lock_bind_evidence(accept=True, extending=True)
        auto_evidence["sent_packets"].clear()
        facts = scenario.validate_leader_auto_lock_evidence(
            auto_evidence, expected_instance_id=42)
        self.assertTrue(facts["automatic_confirmation"])
        self.assertFalse(facts["client_response_sent"])
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_leader_auto_lock_evidence(lock_bind_evidence(
                accept=True, extending=True))

    def test_raid_info_plan_and_wire_prove_reloaded_same_id_content(self) -> None:
        leader = actor()
        plan = scenario.raid_info_plan(leader)
        self.assertEqual(plan["actions"][0], {"action": "request_raid_info"})
        self.assertEqual(plan["actions"][1]["connection"], "realm")
        facts = scenario.validate_raid_info_evidence(
            raid_info_evidence(), leader, 42, expected_mask=1)
        self.assertEqual(facts["entry"], {
            "map_id": scenario.LOCK_MAP, "difficulty_id": scenario.LOCK_DIFFICULTY,
            "instance_id": 42, "time_remaining_seconds": 86_400,
            "completed_encounters_mask": 1, "flags": 0x80,
        })

        for malformed in (raid_info_evidence(mask=0), raid_info_evidence(flags=0x00),
                          raid_info_evidence(response_sequence=9)):
            with self.subTest(evidence=malformed), self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_raid_info_evidence(malformed, leader, 42, expected_mask=1)


class LockStateTests(unittest.TestCase):
    def test_initial_seed_requires_distinct_shared_save_and_nonzero_leader_mask(self) -> None:
        leader = actor()
        member = member_actor()
        expiry = int(time.time()) - 3600
        db = Mock()
        db.execute.side_effect = [
            lock_row(leader, data=scenario.ICC_LEADER_LOCK_SEED_DATA,
                     expiry=expiry, extended=1),
            "", instance_row(), "53\t2\t4\t3", "73\n74",
        ]
        facts = scenario.require_lock_fixture(db, (leader, member), 42, expiry)
        self.assertEqual(facts["completed_encounters_mask"], 1)
        self.assertEqual(facts["data"], scenario.ICC_SHARED_SAVE_DATA)
        self.assertNotEqual(scenario.ICC_LEADER_LOCK_SEED_DATA, facts["data"])
        self.assertEqual(facts["group"]["raid_difficulty"], 4)

        db = Mock()
        db.execute.side_effect = [lock_row(leader, data=scenario.ICC_LEADER_LOCK_SEED_DATA,
                                           completed_mask=0, expiry=expiry, extended=1)]
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.require_lock_fixture(db, (leader, member), 42, expiry)

    def test_pre_auto_and_post_bind_rows_preserve_shared_state_and_mask(self) -> None:
        leader = actor()
        member = member_actor()
        expiry = int(time.time()) - 3600
        db = Mock()
        db.execute.side_effect = [
            "\n".join([
                lock_row(leader, data=scenario.ICC_LEADER_LOCK_SEED_DATA,
                         expiry=expiry, extended=1),
                lock_row(member),
            ]),
            instance_row(),
        ]
        before_auto = scenario.require_bound_lock_rows(
            db, (leader, member), 42, leader_renewed=False)
        self.assertEqual(before_auto["completed_encounters_mask"], 1)
        self.assertNotEqual(before_auto["leader_lock"][5], before_auto["member_lock"][5])

        db = Mock()
        db.execute.side_effect = ["\n".join([lock_row(leader), lock_row(member)]), instance_row()]
        after_bind = scenario.require_bound_lock_rows(
            db, (leader, member), 42, leader_renewed=True)
        self.assertEqual(after_bind["leader_lock"][5], after_bind["member_lock"][5])
        self.assertEqual(after_bind["instance_row"][1], after_bind["member_lock"][5])
        self.assertEqual(after_bind["completed_encounters_mask"], 1)


class LockResetAndEpochTests(unittest.TestCase):
    def test_reset_request_is_outside_while_member_stays_inside(self) -> None:
        leader = actor()
        member = member_actor()
        reset_plan = scenario.lock_reset_plan(leader)
        occupant_plan = scenario.lock_occupant_plan(member)
        self.assertEqual(reset_plan["actions"], [
            {"action": "reset_instances"}, {"action": "wait", "milliseconds": 2_000},
        ])
        self.assertEqual(occupant_plan["actions"], [{"action": "wait", "milliseconds": 18_000}])
        reset = {"sent_packets": [typed(
            "reset_instances", "instance", scenario.CMSG_RESET_INSTANCES, "")]}
        scenario.validate_lock_reset_evidence(reset, {"events": []})
        scenario.validate_lock_inside_state(lock_inside_state(member), 42)
        bad = copy.deepcopy(reset)
        bad["events"] = [group_event("realm", scenario.SMSG_INSTANCE_RESET_FAILED, "2100000000")]
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_lock_reset_evidence(bad, {"events": []})

    def test_lock_scenario_gates_exactly_three_epochs(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture = root / "capture"
            capture.mkdir(mode=0o700)
            journal = root / "journal.json"
            bot_config = root / "bot.json"
            world_config = root / "world.conf"
            bot_exec = root / "bot"
            for path in (journal, bot_config, world_config, bot_exec):
                path.write_text("fixture\n", encoding="utf-8")
            bot_exec.chmod(0o700)
            env = {
                "RUSTYCORE_DEATH_JOURNAL": str(journal),
                "RUSTYCORE_DEATH_EPOCH": "epoch-003",
                "RUSTYCORE_DEATH_EPOCH_INDEX": "3",
                "RUSTYCORE_DEATH_BOT_CONFIG": str(bot_config),
                "RUSTYCORE_DEATH_WORLD_CONFIG": str(world_config),
                "RUSTYCORE_DEATH_SCENARIO": scenario.LOCK_SCENARIO,
                "RUSTYCORE_CAPTURE_DIR": str(capture),
                "RUSTYCORE_CAPTURE_REPORT": str(capture / "scenario-report.json"),
                "RUSTYCORE_CAPTURE_BACKEND": "rust",
                "RUSTYCORE_CAPTURE_CANDIDATE_SHA256": "a" * 64,
                "WOW_BOT_EXEC": str(bot_exec),
                "WOW_BOT_EXEC_SHA256": death_scenario._sha256(bot_exec),
            }
            with patch.dict(os.environ, env, clear=True):
                self.assertEqual(scenario._runtime_identity()["epoch"], 3)
                os.environ["RUSTYCORE_DEATH_EPOCH"] = "epoch-004"
                os.environ["RUSTYCORE_DEATH_EPOCH_INDEX"] = "4"
                with self.assertRaises(scenario.ScenarioFailure):
                    scenario._runtime_identity()
                os.environ["RUSTYCORE_DEATH_SCENARIO"] = scenario.DIFFICULTY_SCENARIO
                os.environ["RUSTYCORE_DEATH_EPOCH"] = "epoch-003"
                os.environ["RUSTYCORE_DEATH_EPOCH_INDEX"] = "3"
                with self.assertRaises(scenario.ScenarioFailure):
                    scenario._runtime_identity()


if __name__ == "__main__":
    unittest.main()
