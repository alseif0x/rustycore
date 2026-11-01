"""Hermetic #54 logout plan, packet-evidence, and timing checks."""

from __future__ import annotations

import copy
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest.mock import ANY, Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parent))

import death_fixture
import death_scenario
import logout_scenario as scenario


def actor(name: str = "Qadeathfall", guid: int = 73) -> dict:
    value = copy.deepcopy(next(item for item in death_fixture.ACTORS if item["name"] == name))
    value["guid"] = guid
    return value


def sent(action: str, sequence: int, opcode: int, body: bytes, action_index: int | None = 0) -> dict:
    return {"action": action, "observation_sequence": sequence, "action_index": action_index,
            "connection": "instance", "opcode": opcode, "body_hex": body.hex(),
            "write_attempted": True, "write_completed": True, "write_error": None}


def event(opcode: int, sequence: int, body: bytes = b"", connection: str = "instance") -> dict:
    return {"sequence": sequence, "observation_sequence": sequence, "connection": connection,
            "opcode": opcode, "body_hex": body.hex(), "body_bytes": len(body),
            "body_sha256": hashlib.sha256(body).hexdigest()}


def response(reason: int, sequence: int, instant: bool = False) -> dict:
    return event(scenario.LOGOUT_RESPONSE, sequence, reason.to_bytes(4, "little", signed=True)
                 + (b"\x80" if instant else b"\x00"))


def stun_event(selected: dict, sequence: int, flags: int, *, unrelated: bool = False) -> dict:
    guid = death_scenario.player_guid(selected, 1)
    packed = death_scenario.packed_guid(guid["low"], guid["high"])
    mask = 0x701 if unrelated else 0x201
    values = struct.pack("<I", 1 << 5) + b"\x02" + struct.pack(">I", mask)
    if unrelated:
        values += struct.pack("<I", 0x11223344)
    values += struct.pack("<I", flags)
    if unrelated:
        values += struct.pack("<I", 0x55667788)
    block = b"\x00" + packed + struct.pack("<I", len(values)) + values
    body = struct.pack("<IH", 1, selected["map"]) + b"\x00" + struct.pack("<I", len(block)) + block
    return event(scenario.UPDATE_OBJECT, sequence, body)


def admitted_evidence(name: str, *, guid: int = 73, actor_name: str = "Qadeathfall") -> dict:
    selected = actor(actor_name, guid)
    requests = [sent("logout_request", 10, scenario.CMSG_LOGOUT_REQUEST, b"\x00", 0)]
    responses = [response(0, 11)]
    roots = [event(scenario.MOVE_ROOT, 12)]
    if name == "logout-repeated":
        requests.append(sent("logout_request", 40, scenario.CMSG_LOGOUT_REQUEST, b"\x00", 4))
        responses.append(response(0, 41))
        roots.append(event(scenario.MOVE_ROOT, 42))
    evidence = {
        "sent_packets": requests,
        "events": responses + roots + [
            event(scenario.STAND_STATE_UPDATE, 13, b"\x00" * 4 + b"\x01", "realm"),
            stun_event(selected, 14, 0x1000 | scenario.UNIT_FLAG_STUNNED),
        ],
        "trace": [], "logout_confirmed": False,
    }
    if name in {"logout-cancel", "logout-repeated", "logout-rbac-revoke"}:
        evidence["sent_packets"].append(
            sent("logout_cancel", 50, scenario.CMSG_LOGOUT_CANCEL, b"", 6))
        evidence["events"].extend([
            event(scenario.LOGOUT_CANCEL_ACK, 51),
            event(scenario.MOVE_UNROOT, 52),
            event(scenario.STAND_STATE_UPDATE, 53, b"\x00" * 5, "realm"),
            stun_event(selected, 54, 0x1000),
        ])
    return evidence


def instant_evidence(name: str) -> tuple[dict, dict]:
    selected = actor(scenario.SCENARIOS[name])
    evidence = {
        "sent_packets": [sent("logout_request", 10, scenario.CMSG_LOGOUT_REQUEST, b"\x00")],
        "events": [response(0, 11, True),
                   event(scenario.LOGOUT_COMPLETE, 12, b"", "realm")],
        "trace": [], "logout_confirmed": True,
    }
    return selected, evidence


class LogoutPlanTests(unittest.TestCase):
    def test_exact_response_reason_bodies_and_bounded_actor_set(self) -> None:
        expected = {"logout-combat-denial": 1, "logout-fall-denial": 3,
                    "logout-freeze-denial": 2, "logout-cancel": 0,
                    "logout-timed": 0, "logout-disconnect": 0, "logout-repeated": 0,
                    "logout-rbac-grant": 0, "logout-rbac-revoke": 0, "logout-resting": 0}
        timeouts = {"logout-combat-denial": 15_000, "logout-fall-denial": 30_000,
                    "logout-freeze-denial": 30_000, "logout-cancel": 15_000,
                    "logout-timed": 35_000, "logout-disconnect": 10_000,
                    "logout-repeated": 35_000, "logout-rbac-grant": 10_000,
                    "logout-rbac-revoke": 15_000, "logout-resting": 10_000}
        self.assertEqual(set(scenario.SCENARIOS), set(expected))
        for name, reason in expected.items():
            with self.subTest(name=name):
                selected = actor(scenario.SCENARIOS[name])
                plan = scenario.plan_for(name, selected)
                self.assertEqual(plan["character_guid"], selected["guid"])
                self.assertEqual(plan["finish_policy"], "disconnect")
                self.assertEqual(plan["timeout_ms"], timeouts[name])
                responses = [rule for rule in plan["expect"]
                             if rule.get("opcode") == scenario.LOGOUT_RESPONSE]
                self.assertEqual(len(responses), 2 if name == "logout-repeated" else 1)
                instant = name in scenario.INSTANT_SCENARIOS
                for rule in responses:
                    self.assertEqual(rule, {"event": "packet", "connection": "instance",
                        "opcode": scenario.LOGOUT_RESPONSE,
                        "body_hex": (reason.to_bytes(4, "little", signed=True)
                                     + (b"\x80" if instant else b"\x00")).hex()})
                    self.assertEqual(bytes.fromhex(rule["body_hex"])[4],
                                     0x80 if instant else 0x00)
                self.assertTrue(all(rule["event"] == "packet" for rule in plan["expect"]))
                requests = [action for action in plan["actions"]
                            if action["action"] == "logout_request"]
                self.assertEqual(len(requests), 2 if name == "logout-repeated" else 1)
                self.assertTrue(all(action == {"action": "logout_request", "idle_logout": False}
                                    for action in requests))
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.plan_for("logout-timed", actor("Qadeathmob"))
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.plan_for("logout-unknown", actor())

    def test_instant_response_uses_observed_bit_packed_bool_byte(self) -> None:
        self.assertEqual(scenario._response(0, instant=True)["body_hex"], "0000000080")
        self.assertEqual(scenario._response(0)["body_hex"], "0000000000")

    def test_setup_actions_and_deadlines_use_typed_logout_and_real_fall(self) -> None:
        combat = scenario.plan_for("logout-combat-denial", actor("Qadeathmob"))
        self.assertEqual([action["action"] for action in combat["actions"]],
                         ["attack_swing", "wait_for_packet", "logout_request", "wait_for_packet"])
        self.assertEqual(combat["actions"][0]["target"]["entry"], 6)
        fall = scenario.plan_for("logout-fall-denial", actor())
        self.assertEqual(fall["actions"][0]["kind"], "heartbeat")
        self.assertGreater(fall["actions"][0]["z"], actor()["home"][2])
        self.assertEqual(fall["actions"][0]["fall"]["fall_time_ms"], 1)
        self.assertFalse(any(action.get("kind") == "fall_land" for action in fall["actions"]))
        self.assertIn({"action": "wait", "milliseconds": scenario.DENIAL_OBSERVE_MS}, fall["actions"])
        freeze = scenario.plan_for("logout-freeze-denial", actor("Qadeathcast"))
        self.assertIn({"action": "wait", "milliseconds": scenario.DENIAL_OBSERVE_MS}, freeze["actions"])

        timed = scenario.plan_for("logout-timed", actor())
        self.assertEqual(timed["timeout_ms"], 35_000)
        self.assertEqual([action["action"] for action in timed["actions"]],
                         ["logout_request", "wait_for_packet", "wait_for_packet", "wait_for_packet"])
        self.assertEqual([action["opcode"] for action in timed["actions"][1:]],
                         [scenario.LOGOUT_RESPONSE, scenario.MOVE_ROOT, scenario.LOGOUT_COMPLETE])
        self.assertFalse(any(action["action"] == "wait" for action in timed["actions"]))
        self.assertEqual(timed["actions"][-1],
                         {"action": "wait_for_packet", "connection": "realm",
                          "opcode": scenario.LOGOUT_COMPLETE, "timeout_ms": 30_000})
        self.assertIn({"event": "packet", "connection": "realm",
                       "opcode": scenario.LOGOUT_COMPLETE, "body_hex": ""}, timed["expect"])
        for name in sorted(scenario.INSTANT_SCENARIOS):
            instant = scenario.plan_for(name, actor(scenario.SCENARIOS[name]))
            self.assertEqual([action["action"] for action in instant["actions"]],
                             ["logout_request", "wait_for_packet", "wait_for_packet"])
            self.assertEqual(instant["actions"][-1],
                             {"action": "wait_for_packet", "connection": "realm",
                              "opcode": scenario.LOGOUT_COMPLETE, "timeout_ms": 5_000})
            self.assertFalse(any(action.get("opcode") == scenario.MOVE_ROOT
                                 for action in instant["actions"]))
        revoke = scenario.plan_for("logout-rbac-revoke", actor("Qadeathreinc"))
        cancel = scenario.plan_for("logout-cancel", actor())
        self.assertEqual(revoke["actions"], cancel["actions"])
        self.assertEqual(revoke["expect"], cancel["expect"])
        repeated = scenario.plan_for("logout-repeated", actor())
        waits = [action["milliseconds"] for action in repeated["actions"] if action["action"] == "wait"]
        self.assertEqual(waits, [12_000, 10_000])
        self.assertGreater(sum(waits), 20_000)
        self.assertLess(waits[1], 20_000)
        self.assertEqual(sum(action["action"] == "logout_cancel" for action in repeated["actions"]), 1)
        disconnect = scenario.plan_for("logout-disconnect", actor())
        self.assertFalse(any(action["action"] == "wait" for action in disconnect["actions"]))
        relogin = scenario.relogin_plan(actor())
        self.assertTrue(relogin["observe_only"])
        self.assertEqual(relogin["finish_policy"], "disconnect")
        self.assertEqual(relogin["actions"], [])

    def test_freeze_seed_is_fixed_scoped_and_only_available_to_its_case(self) -> None:
        selected = actor("Qadeathcast", 99)
        db = Mock()
        scenario._seed_freeze("logout-freeze-denial", db, selected, 3)
        statements = db.transaction.call_args.args[0]
        self.assertEqual(len(statements), 4)
        self.assertEqual(len(scenario.freeze_statements(selected, 3)), 4)
        self.assertTrue(all("99" in statement and "9454" in statement for statement in statements))
        self.assertTrue(all("Qadeath" not in statement for statement in statements))
        guid = death_scenario.player_guid(selected, 3)
        self.assertIn(struct.pack("<QQ", guid["low"], guid["high"]).hex(), statements[2])
        self.assertIn("maxDuration,remainTime", statements[2])
        self.assertIn("-1,-1", statements[2])
        for name in scenario.SCENARIOS:
            if name != "logout-freeze-denial":
                with self.subTest(name=name), self.assertRaises(scenario.ScenarioFailure):
                    scenario._seed_freeze(name, db, selected, 3)
        self.assertEqual(db.transaction.call_count, 1)


class LogoutEvidenceTests(unittest.TestCase):
    def test_denials_reject_root_completion_wrong_reason_and_missing_setup(self) -> None:
        cases = (("logout-combat-denial", 1, "Qadeathmob"),
                 ("logout-fall-denial", 3, "Qadeathfall"),
                 ("logout-freeze-denial", 2, "Qadeathcast"))
        for name, reason, actor_name in cases:
            selected = actor(actor_name)
            evidence = {"sent_packets": [sent("logout_request", 10,
                        scenario.CMSG_LOGOUT_REQUEST, b"\x00")],
                        "events": [response(reason, 11)], "logout_confirmed": False}
            if name == "logout-combat-denial":
                evidence["events"].insert(0, event(scenario.ATTACKER_STATE_UPDATE, 5, b"\x01"))
            if name == "logout-fall-denial":
                evidence["sent_packets"].insert(0, sent("movement_heartbeat", 5, 0, b"fall"))
            with self.subTest(name=name):
                self.assertIn("denial_observation", scenario.validate_evidence(
                    name, scenario.plan_for(name, selected), evidence, selected, 1))
            for defect in ("root", "sit", "complete", "reason", "setup"):
                bad = copy.deepcopy(evidence)
                if defect == "root":
                    bad["events"].append(event(scenario.MOVE_ROOT, 12))
                elif defect == "sit":
                    bad["events"].append(event(scenario.STAND_STATE_UPDATE, 12,
                                               b"\x00" * 4 + b"\x01", "realm"))
                elif defect == "complete":
                    bad["events"].append(event(scenario.LOGOUT_COMPLETE, 12, b"", "realm"))
                    bad["logout_confirmed"] = True
                elif defect == "reason":
                    bad["events"][-1] = response((reason + 1) % 4, 11)
                elif name == "logout-combat-denial":
                    bad["events"] = bad["events"][1:]
                elif name == "logout-fall-denial":
                    bad["sent_packets"] = bad["sent_packets"][1:]
                else:
                    continue
                with self.subTest(name=name, defect=defect), self.assertRaises(scenario.ScenarioFailure):
                    scenario.validate_evidence(name, scenario.plan_for(name, selected), bad, selected, 1)

    def test_cancel_restores_movement_stand_and_exact_stun_bit(self) -> None:
        selected = actor()
        plan = scenario.plan_for("logout-cancel", selected)
        evidence = admitted_evidence("logout-cancel")
        facts = scenario.validate_evidence("logout-cancel", plan, evidence, selected, 1)
        self.assertEqual(facts["sit_count"], 1)
        self.assertEqual(facts["ack_count"], 1)
        self.assertFalse(facts["stun_clear"]["stunned"])
        for defect in ("ack", "unroot", "stand", "stun_clear", "extra_flag", "complete"):
            bad = copy.deepcopy(evidence)
            if defect == "complete":
                bad["events"].append(event(scenario.LOGOUT_COMPLETE, 60, b"", "realm"))
                bad["logout_confirmed"] = True
            else:
                index = {"ack": -4, "unroot": -3, "stand": -2, "stun_clear": -1, "extra_flag": -1}[defect]
                if defect == "extra_flag":
                    bad["events"][index] = stun_event(selected, 54, 0x1001)
                else:
                    del bad["events"][index]
            with self.subTest(defect=defect), self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_evidence("logout-cancel", plan, bad, selected, 1)

    def test_instant_completion_is_exact_and_has_no_countdown_transitions(self) -> None:
        for name in sorted(scenario.INSTANT_SCENARIOS):
            selected, evidence = instant_evidence(name)
            plan = scenario.plan_for(name, selected)
            with self.subTest(name=name):
                facts = scenario.validate_evidence(name, plan, evidence, selected, 1)
                self.assertEqual(facts["complete_event_sequence"], 12)
            for defect in ("missing", "duplicate", "early", "wrong_connection", "root",
                           "sit", "stun", "non_instant", "cancel", "finish"):
                bad = copy.deepcopy(evidence)
                if defect == "missing":
                    del bad["events"][-1]
                elif defect == "duplicate":
                    bad["events"].append(event(scenario.LOGOUT_COMPLETE, 13, b"", "realm"))
                elif defect == "early":
                    bad["events"][-1]["observation_sequence"] = 11
                elif defect == "wrong_connection":
                    bad["events"][-1]["connection"] = "instance"
                elif defect == "root":
                    bad["events"].append(event(scenario.MOVE_ROOT, 13))
                elif defect == "sit":
                    bad["events"].append(event(scenario.STAND_STATE_UPDATE, 13,
                                               b"\x00" * 4 + b"\x01", "realm"))
                elif defect == "stun":
                    bad["events"].append(stun_event(
                        selected, 13, 0x1000 | scenario.UNIT_FLAG_STUNNED))
                elif defect == "non_instant":
                    bad["events"][0] = response(0, 11)
                elif defect == "cancel":
                    bad["sent_packets"].append(
                        sent("logout_cancel", 13, scenario.CMSG_LOGOUT_CANCEL, b""))
                else:
                    bad["sent_packets"].append(sent(
                        "finish_logout", 13, scenario.CMSG_LOGOUT_REQUEST, b"\x00", None))
                with self.subTest(name=name, defect=defect), \
                        self.assertRaises(scenario.ScenarioFailure):
                    scenario.validate_evidence(name, plan, bad, selected, 1)

    def test_rbac_revoke_uses_cancel_validation_and_rejects_instant_complete(self) -> None:
        selected = actor("Qadeathreinc")
        plan = scenario.plan_for("logout-rbac-revoke", selected)
        evidence = admitted_evidence("logout-rbac-revoke", actor_name="Qadeathreinc")
        facts = scenario.validate_evidence("logout-rbac-revoke", plan, evidence, selected, 1)
        self.assertEqual(facts["ack_count"], 1)
        self.assertEqual(facts["sit_count"], 1)
        self.assertFalse(facts["stun_clear"]["stunned"])
        for defect in ("complete", "instant", "missing_cancel"):
            bad = copy.deepcopy(evidence)
            if defect == "complete":
                bad["events"].append(event(scenario.LOGOUT_COMPLETE, 60, b"", "realm"))
                bad["logout_confirmed"] = True
            elif defect == "instant":
                bad["events"][0] = response(0, 11, True)
            else:
                bad["sent_packets"] = [packet for packet in bad["sent_packets"]
                                       if packet["action"] != "logout_cancel"]
            with self.subTest(defect=defect), self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_evidence("logout-rbac-revoke", plan, bad, selected, 1)

    def test_identical_duplicate_stun_updates_pass_but_conflicts_fail(self) -> None:
        selected = actor()
        plan = scenario.plan_for("logout-cancel", selected)
        evidence = admitted_evidence("logout-cancel")
        evidence["events"].extend([
            stun_event(selected, 15, 0x1000 | scenario.UNIT_FLAG_STUNNED),
            stun_event(selected, 55, 0x1000),
        ])
        facts = scenario.validate_evidence("logout-cancel", plan, evidence, selected, 1)
        self.assertEqual(facts["stun_set"]["observation_sequence"], 14)
        self.assertEqual(facts["stun_clear"]["observation_sequence"], 55)
        self.assertEqual(facts["ack_count"], 1)
        self.assertEqual(facts["unroot"], 52)
        self.assertEqual(facts["stand"], 53)
        owner = death_scenario.player_guid(selected, 1)
        self.assertEqual(len(scenario._stun_updates(evidence, owner)), 4)

        conflicting_set = admitted_evidence("logout-cancel")
        conflicting_set["events"].extend([
            stun_event(selected, 15, 0x2000 | scenario.UNIT_FLAG_STUNNED),
            stun_event(selected, 16, 0x1000 | scenario.UNIT_FLAG_STUNNED),
            stun_event(selected, 55, 0x1000),
        ])
        with self.assertRaisesRegex(scenario.ScenarioFailure, "conflicting flags"):
            scenario.validate_evidence("logout-cancel", plan, conflicting_set, selected, 1)
        conflicting_clear = admitted_evidence("logout-cancel")
        conflicting_clear["events"].extend([
            stun_event(selected, 15, 0x1000 | scenario.UNIT_FLAG_STUNNED),
            stun_event(selected, 55, 0x1001),
            stun_event(selected, 56, 0x1000),
        ])
        with self.assertRaisesRegex(scenario.ScenarioFailure, "conflicting flags"):
            scenario.validate_evidence("logout-cancel", plan, conflicting_clear, selected, 1)

    def test_stun_set_may_follow_typed_cancel_but_must_precede_ack(self) -> None:
        selected = actor()
        plan = scenario.plan_for("logout-cancel", selected)
        evidence = admitted_evidence("logout-cancel")
        cancel = next(item for item in evidence["sent_packets"]
                      if item["action"] == "logout_cancel")
        cancel["observation_sequence"] = 72
        updates = [item for item in evidence["events"] if item["opcode"] == scenario.UPDATE_OBJECT]
        for item, sequence in zip(updates, (74, 77), strict=True):
            item["sequence"] = item["observation_sequence"] = sequence
        for opcode, sequence in ((scenario.LOGOUT_CANCEL_ACK, 75),
                                 (scenario.MOVE_UNROOT, 76),
                                 (scenario.STAND_STATE_UPDATE, 76)):
            item = next(item for item in evidence["events"] if item["opcode"] == opcode
                        and item["observation_sequence"] > 50)
            item["sequence"] = item["observation_sequence"] = sequence
        facts = scenario.validate_evidence("logout-cancel", plan, evidence, selected, 1)
        self.assertEqual(facts["stun_set"]["observation_sequence"], 74)
        self.assertEqual(facts["stun_clear"]["observation_sequence"], 77)
        self.assertEqual(facts["unroot"], 76)
        self.assertEqual(facts["stand"], 76)

        after_ack = copy.deepcopy(evidence)
        late_set = next(item for item in after_ack["events"]
                        if item["opcode"] == scenario.UPDATE_OBJECT
                        and item["observation_sequence"] == 74)
        late_set["sequence"] = late_set["observation_sequence"] = 76
        with self.assertRaisesRegex(scenario.ScenarioFailure, "lacks its UNIT_FLAG_STUNNED set"):
            scenario.validate_evidence("logout-cancel", plan, after_ack, selected, 1)

    def test_captured_unit_data_blocks_parse_exact_logout_stun_values(self) -> None:
        selected = actor(guid=20)
        captured_set = bytes.fromhex(
            "01000000000000170000000001a01404080d00000020000000020000020108000400")
        captured_clear = bytes.fromhex(
            "01000000000000170000000001a01404080d00000020000000020000020108000000")
        evidence = admitted_evidence("logout-cancel", guid=20)
        indexes = [index for index, item in enumerate(evidence["events"])
                   if item["opcode"] == scenario.UPDATE_OBJECT]
        self.assertEqual(len(indexes), 2)
        evidence["events"][indexes[0]] = event(scenario.UPDATE_OBJECT, 14, captured_set)
        evidence["events"][indexes[1]] = event(scenario.UPDATE_OBJECT, 54, captured_clear)
        owner = death_scenario.player_guid(selected, 1)
        updates = scenario._stun_updates(evidence, owner)
        self.assertEqual([item["flags"] for item in updates], [0x00040008, 0x00000008])
        self.assertEqual([item["stunned"] for item in updates], [True, False])
        facts = scenario.validate_evidence(
            "logout-cancel", scenario.plan_for("logout-cancel", selected), evidence, selected, 1)
        self.assertEqual(facts["stun_set"]["flags"], 0x00040008)
        self.assertEqual(facts["stun_clear"]["flags"], 0x00000008)

    def test_unit_data_skips_unrelated_fields_and_rejects_truncation(self) -> None:
        selected = actor()
        evidence = admitted_evidence("logout-cancel")
        indexes = [index for index, item in enumerate(evidence["events"])
                   if item["opcode"] == scenario.UPDATE_OBJECT]
        evidence["events"][indexes[0]] = stun_event(
            selected, 14, 0x1000 | scenario.UNIT_FLAG_STUNNED, unrelated=True)
        evidence["events"][indexes[1]] = stun_event(selected, 54, 0x1000, unrelated=True)
        owner = death_scenario.player_guid(selected, 1)
        blocks = scenario._value_blocks(bytes.fromhex(evidence["events"][indexes[0]]["body_hex"]))
        self.assertEqual(len(blocks[0][1]), 21)
        self.assertEqual([item["flags"] for item in scenario._stun_updates(evidence, owner)],
                         [0x1000 | scenario.UNIT_FLAG_STUNNED, 0x1000])
        scenario.validate_evidence(
            "logout-cancel", scenario.plan_for("logout-cancel", selected), evidence, selected, 1)
        truncated = copy.deepcopy(evidence)
        body = bytes.fromhex(truncated["events"][indexes[0]]["body_hex"])
        truncated["events"][indexes[0]] = event(scenario.UPDATE_OBJECT, 14, body[:-1])
        self.assertEqual([item["flags"] for item in scenario._stun_updates(truncated, owner)],
                         [0x1000])
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_evidence(
                "logout-cancel", scenario.plan_for("logout-cancel", selected), truncated, selected, 1)

    def test_repeated_requests_have_one_sit_one_ack_and_restoration(self) -> None:
        selected = actor()
        plan = scenario.plan_for("logout-repeated", selected)
        evidence = admitted_evidence("logout-repeated")
        facts = scenario.validate_evidence("logout-repeated", plan, evidence, selected, 1)
        self.assertEqual(facts["sit_count"], 1)
        self.assertEqual(len(facts["roots"]), 2)
        duplicate = copy.deepcopy(evidence)
        duplicate["events"].append(event(scenario.STAND_STATE_UPDATE, 43,
                                         b"\x00" * 4 + b"\x01", "realm"))
        with self.assertRaisesRegex(scenario.ScenarioFailure, "single sit"):
            scenario.validate_evidence("logout-repeated", plan, duplicate, selected, 1)
        premature = copy.deepcopy(evidence)
        premature["events"].append(event(scenario.LOGOUT_COMPLETE, 45, b"", "realm"))
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_evidence("logout-repeated", plan, premature, selected, 1)
        second_cancel = copy.deepcopy(evidence)
        second_cancel["sent_packets"].append(
            sent("logout_cancel", 55, scenario.CMSG_LOGOUT_CANCEL, b"", 8))
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_evidence("logout-repeated", plan, second_cancel, selected, 1)

    def test_timed_completion_follows_packet_wait_and_has_no_second_finish(self) -> None:
        selected = actor()
        plan = scenario.plan_for("logout-timed", selected)
        evidence = admitted_evidence("logout-timed")
        evidence["trace"] = [
            {"kind": "action_started", "action": "wait_for_packet", "action_index": 3,
             "sequence": 20},
        ]
        evidence["events"].append(event(scenario.LOGOUT_COMPLETE, 25, b"", "realm"))
        facts = scenario.validate_evidence("logout-timed", plan, evidence, selected, 1)
        self.assertEqual(facts["complete_wait_started_trace_sequence"], 20)
        self.assertEqual(facts["complete_event_sequence"], 25)
        self.assertNotIn("minimum_wait_ms", facts)
        for defect in ("early", "missing", "duplicate", "second_finish", "missing_start",
                       "duplicate_start", "wait_before_root"):
            bad = copy.deepcopy(evidence)
            if defect == "early":
                bad["events"][-1]["observation_sequence"] = 19
            elif defect == "missing":
                del bad["events"][-1]
            elif defect == "duplicate":
                bad["events"].append(event(scenario.LOGOUT_COMPLETE, 32, b"", "realm"))
            elif defect == "second_finish":
                bad["sent_packets"].append(sent("finish_logout", 32,
                    scenario.CMSG_LOGOUT_REQUEST, b"\x00", None))
            elif defect == "missing_start":
                bad["trace"] = []
            elif defect == "duplicate_start":
                bad["trace"].append({"kind": "action_started", "action": "wait_for_packet",
                                     "action_index": 3, "sequence": 21})
            else:
                bad["trace"][0]["sequence"] = 11
            with self.subTest(defect=defect), self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_evidence("logout-timed", plan, bad, selected, 1)

    def test_disconnect_evidence_and_owned_stun_parser_reject_bad_shapes(self) -> None:
        selected = actor()
        plan = scenario.plan_for("logout-disconnect", selected)
        evidence = admitted_evidence("logout-disconnect")
        scenario.validate_evidence("logout-disconnect", plan, evidence, selected, 1)
        owner = death_scenario.player_guid(selected, 1)
        updates = scenario._stun_updates(evidence, owner)
        self.assertEqual(len(updates), 1)
        self.assertTrue(updates[0]["stunned"])
        malformed = copy.deepcopy(evidence)
        update = next(item for item in malformed["events"] if item["opcode"] == scenario.UPDATE_OBJECT)
        update["body_hex"] = update["body_hex"][:-2]
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_evidence("logout-disconnect", plan, malformed, selected, 1)
        completed = copy.deepcopy(evidence)
        completed["events"].append(event(scenario.LOGOUT_COMPLETE, 20, b"", "realm"))
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_evidence("logout-disconnect", plan, completed, selected, 1)


class LogoutVariantFixtureTests(unittest.TestCase):
    def test_rbac_setup_and_cleanup_use_only_owned_keys(self) -> None:
        for name, actor_name in (("logout-rbac-grant", "Qadeathcast"),
                                 ("logout-rbac-revoke", "Qadeathreinc")):
            selected = actor(actor_name)
            account = selected["account_id"]
            db = Mock()
            db.execute.return_value = ""
            with self.subTest(name=name, phase="setup"):
                state = scenario._prepare_variant(name, db, selected)
                expected_state = {"account_id": account}
                if name == "logout-rbac-revoke":
                    expected_state["original_access_security"] = None
                self.assertEqual(state, expected_state)
                statements = db.transaction.call_args.args[0]
                permissions = [1] if name == "logout-rbac-grant" else [1, 194]
                inserts = [statement for statement in statements
                           if statement.startswith("INSERT INTO auth.rbac_account_permissions")]
                self.assertEqual(len(inserts), len(permissions))
                for statement, permission in zip(inserts, permissions, strict=True):
                    granted = 1 if name == "logout-rbac-grant" else 0
                    self.assertTrue(statement.endswith(
                        f"VALUES ({account},{permission},{granted},-1)"))
                access_inserts = [statement for statement in statements
                                  if statement.startswith("INSERT INTO auth.account_access")]
                self.assertEqual(len(access_inserts), int(name == "logout-rbac-revoke"))
                if access_inserts:
                    self.assertEqual(access_inserts[0],
                        "INSERT INTO auth.account_access (AccountID,SecurityLevel,RealmID) "
                        f"VALUES ({account},1,-1)")
                self.assertEqual(db.execute.call_count,
                                 len(permissions) + (name == "logout-rbac-revoke"))
            db.reset_mock()
            with self.subTest(name=name, phase="cleanup"):
                scenario._restore_variant(name, db, selected, state)
                statements = db.transaction.call_args.args[0]
                deletes = [statement for statement in statements
                           if statement.startswith("DELETE FROM auth.rbac_account_permissions")]
                self.assertEqual(len(deletes), len(permissions))
                for statement, permission in zip(deletes, permissions, strict=True):
                    granted = 1 if name == "logout-rbac-grant" else 0
                    self.assertTrue(statement.endswith(
                        f"WHERE accountId={account} AND permissionId={permission} "
                        f"AND granted={granted} AND realmId=-1"))
                access_deletes = [statement for statement in statements
                                  if statement.startswith("DELETE FROM auth.account_access")]
                self.assertEqual(len(access_deletes), int(name == "logout-rbac-revoke"))
                if access_deletes:
                    self.assertEqual(access_deletes[0],
                        "DELETE FROM auth.account_access "
                        f"WHERE AccountID={account} AND RealmID=-1")
                self.assertEqual(db.execute.call_count,
                                 len(permissions) + (name == "logout-rbac-revoke"))

    def test_revoke_existing_zero_access_is_updated_and_restored(self) -> None:
        selected = actor("Qadeathreinc")
        db = Mock()
        db.execute.side_effect = ["", "", "12\t0\t-1"]
        state = scenario._prepare_variant("logout-rbac-revoke", db, selected)
        self.assertEqual(state, {"account_id": 12, "original_access_security": 0})
        statements = db.transaction.call_args.args[0]
        self.assertEqual(statements[0],
            "UPDATE auth.account_access SET SecurityLevel=1 WHERE AccountID=12 AND RealmID=-1")
        self.assertFalse(any(statement.startswith("INSERT INTO auth.account_access")
                             for statement in statements))

        db.reset_mock()
        db.execute.side_effect = ["", "", "12\t0\t-1"]
        scenario._restore_variant("logout-rbac-revoke", db, selected, state)
        statements = db.transaction.call_args.args[0]
        self.assertEqual(statements[-1],
            "UPDATE auth.account_access SET SecurityLevel=0 WHERE AccountID=12 AND RealmID=-1")
        self.assertFalse(any(statement.startswith("DELETE FROM auth.account_access")
                             for statement in statements))
        self.assertEqual(db.execute.call_count, 3)

    def test_resting_setup_restores_exact_flags_and_checks_identity(self) -> None:
        selected = actor()
        db = Mock()
        db.execute.return_value = f"{selected['guid']}\t{selected['account_id']}\t65"
        state = scenario._prepare_variant("logout-resting", db, selected)
        self.assertEqual(state, {"original_player_flags": 65})
        self.assertEqual(db.transaction.call_args.args[0], [
            f"UPDATE characters SET playerFlags=playerFlags|{scenario.PLAYER_FLAGS_RESTING} "
            f"WHERE guid={selected['guid']} AND account={selected['account_id']}"])
        scenario._restore_variant("logout-resting", db, selected, state)
        self.assertEqual(db.transaction.call_args_list[-1].args[0], [
            "UPDATE characters SET playerFlags=65 "
            f"WHERE guid={selected['guid']} AND account={selected['account_id']}"])

        wrong_identity = Mock()
        wrong_identity.execute.return_value = f"{selected['guid']}\t999\t0"
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._prepare_variant("logout-resting", wrong_identity, selected)
        wrong_identity.transaction.assert_not_called()

    def test_preexisting_auth_rows_abort_before_transaction(self) -> None:
        for name, actor_name, rows in (
                ("logout-rbac-grant", "Qadeathcast", ["9\t1\t1\t-1"]),
                ("logout-rbac-revoke", "Qadeathreinc", ["", "12\t194\t0\t-1"]),
                ("logout-rbac-revoke", "Qadeathreinc", ["", "", "12\t1\t-1"]),
                ("logout-rbac-revoke", "Qadeathreinc",
                 ["", "", "12\t0\t-1\n12\t0\t-1"])):
            db = Mock()
            db.execute.side_effect = rows
            with self.subTest(name=name, rows=rows), self.assertRaises(scenario.ScenarioFailure):
                scenario._prepare_variant(name, db, actor(actor_name))
            db.transaction.assert_not_called()

    def test_cleanup_runs_on_exception_and_gates_success_report(self) -> None:
        for name, actor_name, account in (("logout-rbac-grant", "Qadeathcast", 9),
                                          ("logout-rbac-revoke", "Qadeathreinc", 12)):
            selected = actor(actor_name)
            identity = {"scenario": name}
            journal = {"actors": [selected]}
            state = {"account_id": account}
            if name == "logout-rbac-revoke":
                state["original_access_security"] = 0
            with self.subTest(name=name), \
                    patch.object(scenario, "_prepare_variant", return_value=state), \
                    patch.object(scenario, "_run_case_body", side_effect=RuntimeError("failed")), \
                    patch.object(scenario, "_restore_variant") as restore:
                with self.assertRaisesRegex(RuntimeError, "failed"):
                    scenario._run_case(identity, journal, 1, Mock())
                restore.assert_called_once_with(name, ANY, selected, state)

        selected = actor("Qadeathcast")
        identity = {"scenario": "logout-rbac-grant"}
        journal = {"actors": [selected]}
        state = {"account_id": 9}
        with patch.object(scenario, "_prepare_variant", return_value=state), \
                patch.object(scenario, "_run_case_body", return_value={"scenario": identity["scenario"]}), \
                patch.object(scenario, "_restore_variant") as restore:
            report = scenario._run_case(identity, journal, 1, Mock())
            self.assertTrue(report["variant_fixture_restored"])
            restore.assert_called_once()
            restore.side_effect = scenario.ScenarioFailure("cleanup failed")
            with self.assertRaisesRegex(scenario.ScenarioFailure, "cleanup failed"):
                scenario._run_case(identity, journal, 1, Mock())


class TimedCaptureTimingTests(unittest.TestCase):
    def write_meta(self, directory: Path, stem: str, direction: str, opcode: int) -> Path:
        path = directory / f"{stem}.meta"
        path.write_text(f"direction={direction}\nconnection_id=0\naddr=127.0.0.1:0\n"
                        f"seq=0\ncounter=0\nopcode=0x{opcode:04X}\nname={stem}\nlen=2\n",
                        encoding="utf-8")
        return path

    def capture_pair(self, directory: Path, elapsed_ms: int) -> tuple[Path, Path]:
        request = self.write_meta(directory, "request", "c2s", scenario.CMSG_LOGOUT_REQUEST)
        complete = self.write_meta(directory, "complete", "s2c", scenario.LOGOUT_COMPLETE)
        request_ns = 1_700_000_000_000_000_000
        complete_ns = request_ns + elapsed_ms * 1_000_000
        os.utime(request, ns=(request_ns, request_ns))
        os.utime(complete, ns=(complete_ns, complete_ns))
        return request, complete

    def test_capture_measurement_selects_exact_direction_and_opcode(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            capture = Path(raw)
            request, complete = self.capture_pair(capture, 19_750)
            self.write_meta(capture, "wrong-complete-direction", "c2s", scenario.LOGOUT_COMPLETE)
            self.write_meta(capture, "wrong-request-direction", "s2c", scenario.CMSG_LOGOUT_REQUEST)
            self.write_meta(capture, "other-opcode", "c2s", scenario.CMSG_LOGOUT_CANCEL)
            facts = scenario._measure_timed_logout(capture)
        self.assertEqual(facts["request_path"], str(request))
        self.assertEqual(facts["complete_path"], str(complete))
        self.assertEqual(facts["elapsed_ms"], 19_750)

    def test_capture_measurement_enforces_inclusive_timing_window(self) -> None:
        for elapsed_ms, valid in ((19_000, True), (25_000, True),
                                  (18_999, False), (25_001, False)):
            with self.subTest(elapsed_ms=elapsed_ms), tempfile.TemporaryDirectory() as raw:
                self.capture_pair(Path(raw), elapsed_ms)
                if valid:
                    self.assertEqual(scenario._measure_timed_logout(Path(raw))["elapsed_ms"],
                                     elapsed_ms)
                else:
                    with self.assertRaisesRegex(scenario.ScenarioFailure, "19-to-25-second"):
                        scenario._measure_timed_logout(Path(raw))

    def test_capture_measurement_rejects_missing_or_duplicate_packets(self) -> None:
        for defect in ("missing_request", "missing_complete", "duplicate_request",
                       "duplicate_complete"):
            with self.subTest(defect=defect), tempfile.TemporaryDirectory() as raw:
                capture = Path(raw)
                request, complete = self.capture_pair(capture, 20_000)
                if defect == "missing_request":
                    request.unlink()
                elif defect == "missing_complete":
                    complete.unlink()
                elif defect == "duplicate_request":
                    self.write_meta(capture, "request-2", "c2s", scenario.CMSG_LOGOUT_REQUEST)
                else:
                    self.write_meta(capture, "complete-2", "s2c", scenario.LOGOUT_COMPLETE)
                with self.assertRaisesRegex(scenario.ScenarioFailure, "exactly one"):
                    scenario._measure_timed_logout(capture)

    def test_run_case_adds_real_capture_measurement_to_observations(self) -> None:
        selected = actor()
        evidence = admitted_evidence("logout-timed")
        evidence["trace"] = [
            {"kind": "action_started", "action": "wait_for_packet", "action_index": 3,
             "sequence": 20},
        ]
        evidence["events"].append(event(scenario.LOGOUT_COMPLETE, 25, b"", "realm"))
        with tempfile.TemporaryDirectory() as raw:
            capture = Path(raw)
            request, complete = self.capture_pair(capture, 20_000)
            snapshot_path = capture / "snapshot.json"
            snapshot_path.write_text("{}\n", encoding="utf-8")
            identity = {"scenario": "logout-timed", "capture": capture}
            with patch.object(scenario, "_run_single", return_value=evidence), \
                    patch.object(scenario, "_save_snapshot",
                                 return_value=({}, {"logout_time": 1}, snapshot_path)):
                report = scenario._run_case(identity, {"actors": [selected]}, 1, Mock())
            observation = report["observations"][0]
        self.assertEqual(observation["request_path"], str(request))
        self.assertEqual(observation["complete_path"], str(complete))
        self.assertEqual(observation["elapsed_ms"], 20_000)
        self.assertEqual(observation["complete_event_sequence"], 25)


class DisconnectTimingTests(unittest.TestCase):
    def test_online_at_two_seconds_then_offline_after_deadline(self) -> None:
        selected = actor()
        clock = [0.0]

        def sleep(seconds: float) -> None:
            clock[0] += seconds

        db = Mock()
        db.character_identities.side_effect = lambda _journal: [[
            str(selected["guid"]), str(selected["account_id"]), selected["name"],
            "1" if clock[0] < 20 else "0"]]
        with patch.object(scenario.time, "monotonic", side_effect=lambda: clock[0]), \
                patch.object(scenario.time, "sleep", side_effect=sleep):
            facts = scenario.prove_disconnect_timing(db, {}, selected, 0.0)
        self.assertEqual(facts["online_check_elapsed_seconds"], 2)
        self.assertEqual(facts["offline_check_elapsed_seconds"], 20)
        self.assertTrue(facts["online_at_two_seconds"])
        self.assertTrue(facts["offline_after_twenty_seconds"])

    def test_early_offline_late_online_and_wrong_identity_fail(self) -> None:
        selected = actor()
        for mode in ("early_offline", "never_offline", "wrong_identity"):
            clock = [0.0]
            db = Mock()

            def identities(_journal, mode=mode):
                if mode == "wrong_identity":
                    return [["999", str(selected["account_id"]), selected["name"], "1"]]
                online = "0" if mode == "early_offline" else "1"
                return [[str(selected["guid"]), str(selected["account_id"]), selected["name"], online]]

            db.character_identities.side_effect = identities
            with self.subTest(mode=mode), \
                    patch.object(scenario.time, "monotonic", side_effect=lambda: clock[0]), \
                    patch.object(scenario.time, "sleep", side_effect=lambda seconds: clock.__setitem__(0, clock[0] + seconds)), \
                    self.assertRaises(scenario.ScenarioFailure):
                scenario.prove_disconnect_timing(db, {}, selected, 0.0)
            self.assertEqual(clock[0], 2 if mode != "never_offline" else 25)

    def test_failure_report_does_not_echo_exception_secrets(self) -> None:
        payload = scenario._failure_report(RuntimeError("secret-password"), "logout-timed")
        encoded = json.dumps(payload)
        self.assertNotIn("secret-password", encoded)
        self.assertEqual(payload["schema"], "rustycore-logout-scenario-v1")
        self.assertFalse(payload["parity_assessed"])


if __name__ == "__main__":
    unittest.main()
