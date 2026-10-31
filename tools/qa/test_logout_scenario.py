"""Hermetic #54 logout plan, packet-evidence, and disconnect-timing checks."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import struct
import sys
import unittest
from unittest.mock import Mock, patch

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


def response(reason: int, sequence: int) -> dict:
    return event(scenario.LOGOUT_RESPONSE, sequence,
                 reason.to_bytes(4, "little", signed=True) + b"\x00")


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


def admitted_evidence(name: str, *, guid: int = 73) -> dict:
    selected = actor(guid=guid)
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
    if name in {"logout-cancel", "logout-repeated"}:
        evidence["sent_packets"].append(
            sent("logout_cancel", 50, scenario.CMSG_LOGOUT_CANCEL, b"", 6))
        evidence["events"].extend([
            event(scenario.LOGOUT_CANCEL_ACK, 51),
            event(scenario.MOVE_UNROOT, 52),
            event(scenario.STAND_STATE_UPDATE, 53, b"\x00" * 5, "realm"),
            stun_event(selected, 54, 0x1000),
        ])
    return evidence


class LogoutPlanTests(unittest.TestCase):
    def test_exact_response_reason_bodies_and_bounded_actor_set(self) -> None:
        expected = {"logout-combat-denial": 1, "logout-fall-denial": 3,
                    "logout-freeze-denial": 2, "logout-cancel": 0,
                    "logout-timed": 0, "logout-disconnect": 0, "logout-repeated": 0}
        self.assertEqual(set(scenario.SCENARIOS), set(expected))
        for name, reason in expected.items():
            with self.subTest(name=name):
                selected = actor(scenario.SCENARIOS[name])
                plan = scenario.plan_for(name, selected)
                self.assertEqual(plan["character_guid"], selected["guid"])
                self.assertEqual(plan["finish_policy"], "disconnect")
                self.assertLessEqual(plan["timeout_ms"], 35_000)
                responses = [rule for rule in plan["expect"]
                             if rule.get("opcode") == scenario.LOGOUT_RESPONSE]
                self.assertEqual(len(responses), 2 if name == "logout-repeated" else 1)
                for rule in responses:
                    self.assertEqual(rule, {"event": "packet", "connection": "instance",
                        "opcode": scenario.LOGOUT_RESPONSE,
                        "body_hex": (reason.to_bytes(4, "little", signed=True) + b"\x00").hex()})
                    self.assertEqual(bytes.fromhex(rule["body_hex"])[4], 0)
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
        self.assertIn({"action": "wait", "milliseconds": 20_000}, timed["actions"])
        self.assertEqual(timed["actions"][-1],
                         {"action": "wait_for_packet", "connection": "realm",
                          "opcode": scenario.LOGOUT_COMPLETE, "timeout_ms": 5_000})
        self.assertIn({"event": "packet", "connection": "realm",
                       "opcode": scenario.LOGOUT_COMPLETE, "body_hex": ""}, timed["expect"])
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

    def test_timed_completion_must_follow_full_deadline_and_have_no_second_finish(self) -> None:
        selected = actor()
        plan = scenario.plan_for("logout-timed", selected)
        evidence = admitted_evidence("logout-timed")
        evidence["trace"] = [{"kind": "action_completed", "action": "wait",
                              "action_index": 3, "sequence": 30}]
        evidence["events"].append(event(scenario.LOGOUT_COMPLETE, 31, b"", "realm"))
        facts = scenario.validate_evidence("logout-timed", plan, evidence, selected, 1)
        self.assertEqual(facts["minimum_wait_ms"], 20_000)
        for defect in ("early", "missing", "duplicate", "second_finish"):
            bad = copy.deepcopy(evidence)
            if defect == "early":
                bad["events"][-1]["observation_sequence"] = 29
            elif defect == "missing":
                del bad["events"][-1]
            elif defect == "duplicate":
                bad["events"].append(event(scenario.LOGOUT_COMPLETE, 32, b"", "realm"))
            else:
                bad["sent_packets"].append(sent("finish_logout", 32,
                    scenario.CMSG_LOGOUT_REQUEST, b"\x00", None))
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
