"""Hermetic plan, state, secrecy, and login-race checks for death_scenario."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import Mock, patch

import death_fixture
import death_scenario as scenario


def actor(name: str, guid: int) -> dict:
    value = copy.deepcopy(next(item for item in death_fixture.ACTORS if item["name"] == name))
    value["guid"] = guid
    return value


def journal_actors() -> dict[str, dict]:
    return {item["name"]: actor(item["name"], 1000 + index)
            for index, item in enumerate(death_fixture.ACTORS, 1)}


# Native stationary corpse block retained from C++ offer event 69 (at offset 11).
# The single-block envelope lets the matcher exercise the same 166-byte wire shape.
CORPSE_CREATE = bytes.fromhex(
    "0101a00104380a04000000000000d7f011c6299c274366667642000000007900"
    "00000000000000000000000000803f0000000001a01404080000000031000000"
    "00000000000000000000000000000000bf400005000000000000000000000000"
    "0000000000000000000000000000000000000000000000000000000000000000"
    "000000000000000000000000010008000000000000000001000000"
)


def corpse_probe_event(block: bytes = CORPSE_CREATE, *, map_id: int = 0,
                       count: int = 1) -> dict:
    body = struct.pack("<IHBI", count, map_id, 0, len(block)) + bytes(block)
    return {"sequence": 69, "connection_sequence": 60, "connection": "instance",
            "opcode": scenario.UPDATE_OBJECT, "body_hex": body.hex(),
            "body_sha256": hashlib.sha256(body).hexdigest()}


class DeathScenarioTests(unittest.TestCase):
    def test_offer_corpse_probe_validates_native_owner_map_span_and_unique_identity(self) -> None:
        victim = actor("Qadeathfall", 20)
        owner = scenario.player_guid(victim, 1)
        event = corpse_probe_event()
        self.assertEqual(len(bytes.fromhex(event["body_hex"])), 166)
        proof = scenario.select_offer_corpse({"events": [event]}, victim, owner)
        self.assertEqual(proof["owner"], owner)
        self.assertEqual(proof["guid"]["high"] >> 58, 14)
        self.assertEqual(proof["source_event_sequence"], 69)
        self.assertEqual(proof["source_event_sha256"], event["body_sha256"])
        self.assertEqual(proof["block_offset"], 11)
        birth = bytearray(CORPSE_CREATE)
        birth[0] = 2
        self.assertEqual(scenario.select_offer_corpse(
            {"events": [corpse_probe_event(birth)]}, victim, owner)["guid"], proof["guid"])
        another = bytearray(CORPSE_CREATE)
        another[3] = 2  # A different native corpse, with the same owner.
        far = bytearray(CORPSE_CREATE)
        struct.pack_into("<f", far, 14, victim["home"][0] + 10)
        nonfinite = bytearray(CORPSE_CREATE)
        struct.pack_into("<f", nonfinite, 14, float("nan"))
        bad_length = bytearray(CORPSE_CREATE)
        struct.pack_into("<I", bad_length, 30, 0xFFFFFFFF)
        invalid = [
            {"events": []}, {"events": [corpse_probe_event(map_id=1)]},
            {"events": [corpse_probe_event(CORPSE_CREATE[:-1])]},
            {"events": [corpse_probe_event(CORPSE_CREATE + another, count=2)]},
            {"events": [corpse_probe_event(far)]},
            {"events": [corpse_probe_event(nonfinite)]},
            {"events": [corpse_probe_event(bad_length)]},
        ]
        for evidence in invalid:
            with self.subTest(evidence=evidence), self.assertRaises(ValueError):
                scenario.select_offer_corpse(evidence, victim, owner)
        wrong_owner = scenario.player_guid(actor("Qadeathfall", 21), 1)
        with self.assertRaises(ValueError):
            scenario.select_offer_corpse({"events": [event]}, victim, wrong_owner)
        corrupted = {**event, "body_sha256": "0" * 64}
        with self.assertRaises(ValueError):
            scenario.select_offer_corpse({"events": [corrupted]}, victim, owner)

    def test_offer_probes_before_action_passes_and_casts_at_corpse_not_ghost_player(self) -> None:
        actors = journal_actors()
        actors["Qadeathfall"]["guid"] = 20
        events = {"events": [corpse_probe_event()]}
        order = []
        observations = []
        started = []

        def probe(pass_, *_args, **kwargs):
            self.assertEqual(pass_.label, "offer-corpse-probe")
            self.assertEqual(pass_.actor, actors["Qadeathcast"])
            self.assertEqual(pass_.plan, scenario.observer_plan(3000, pass_.actor["guid"]))
            self.assertTrue(kwargs["observer"])
            observations.append({"pass": pass_.label})
            order.append("probe-finished")
            return events

        def start(pass_, **_kwargs):
            order.append(pass_.label)
            started.append(pass_)

        snapshot = {"character_spell": {"columns": ["guid", "spell"],
                    "rows": [[str(actors["Qadeathcast"]["guid"]), "7328"]]}}
        identity = {"bot_exec": Path("unused"), "bot_config": Path("unused"),
                    "capture": Path("unused")}
        with patch.object(scenario, "_run_single", side_effect=probe), \
                patch.object(scenario, "start_bot", side_effect=start), \
                patch.object(scenario, "finish_bot", return_value={}), \
                patch.object(scenario, "_save_snapshot", return_value=(snapshot, {})):
            scenario._run_offer(identity, {}, actors, 1, Mock(), observations)
        self.assertEqual(order, ["probe-finished", "offer-victim", "offer-caster"])
        caster = next(pass_ for pass_ in started if pass_.label == "offer-caster")
        target = observations[0]["offer_corpse_target"]["guid"]
        self.assertNotEqual(target, scenario.player_guid(actors["Qadeathfall"], 1))
        self.assertEqual([action["target"] for action in caster.plan["actions"]
                          if action["action"] == "cast"], [target, target])
        self.assertEqual([rule["target"] for rule in caster.plan["expect"]
                          if rule["event"] in {"start", "go"}],
                         [{"corpse_ally": target}] * 4)

    def test_offline_wait_allows_native_socket_grace_but_stays_bounded_and_identity_checked(self) -> None:
        selected = actor("Qadeathfall", 20)
        selected["account_id"] = 10
        self.assertEqual(scenario.OFFLINE_SECONDS, 90)
        for offline_at, wrong_identity in ((62, False), (None, False), (62, True)):
            clock = [0.0]

            def sleep(seconds):
                clock[0] += seconds

            def identities(_journal):
                online = "0" if offline_at is not None and clock[0] >= offline_at else "1"
                name = "OtherActor" if wrong_identity else selected["name"]
                return [["20", "10", name, online]]

            db = Mock()
            db.character_identities.side_effect = identities
            with patch.object(scenario.time, "monotonic", side_effect=lambda: clock[0]), \
                    patch.object(scenario.time, "sleep", side_effect=sleep):
                if offline_at is None or wrong_identity:
                    with self.assertRaises(scenario.ScenarioFailure):
                        scenario._poll_actor_offline(db, {}, selected)
                    self.assertEqual(clock[0], 90)
                else:
                    scenario._poll_actor_offline(db, {}, selected)
                    self.assertEqual(clock[0], 62)

    def test_healer_durability_uses_effective_max_and_rejects_old_raw_prediction(self) -> None:
        selected = actor("Qadeathfall", 73)
        snapshot = {
            "characters": {"columns": ["guid", "health", "power1"],
                           "rows": [["73", "500", "250"]]},
            "character_aura": {"columns": ["guid", "spell", "maxduration"],
                               "rows": [["73", "15007", "600000"]]},
            "character_inventory": {"columns": ["guid", "bag", "slot", "item"],
                                    "rows": [["73", "0", "4", "100"], ["73", "0", "35", "101"]]},
            "item_instance": {"columns": ["guid", "itementry", "durability"],
                              "rows": [["100", "2121", "19"], ["101", "2121", "25"]]},
        }
        for epoch in (1, 2):
            observations = [{}]
            scenario._check_special_state("healer", epoch, selected, snapshot, observations)
            self.assertEqual(observations[0]["durability_by_slot"][4]["durability"], 19)
            bad = copy.deepcopy(snapshot)
            bad["item_instance"]["rows"][0][2] = "26"
            bad["item_instance"]["rows"][1][2] = "30"
            with self.assertRaises(scenario.ScenarioFailure):
                scenario._check_special_state("healer", epoch, selected, bad, [{}])

    def test_fall_and_recovery_require_saved_equipment_loss_not_backpack_loss(self) -> None:
        selected = actor("Qadeathfall", 73)
        snapshot = {
            "character_inventory": {"columns": ["guid", "bag", "slot", "item"],
                                    "rows": [["73", "0", "4", "100"], ["73", "0", "35", "101"]]},
            "item_instance": {"columns": ["guid", "itementry", "durability"],
                              "rows": [["100", "2121", "34"], ["101", "2121", "40"]]},
        }
        for case, epochs in (("fall-release", (1,)), ("reclaim", (1,)),
                             ("automatic-release", (1,)), ("recovery-dead", (1, 2)),
                             ("recovery-ghost", (1, 2))):
            for epoch in epochs:
                with self.subTest(case=case, epoch=epoch):
                    scenario._check_special_state(case, epoch, selected, snapshot, [{}])
                    for slot, invalid in ((0, "40"), (1, "34")):
                        bad = copy.deepcopy(snapshot)
                        bad["item_instance"]["rows"][slot][2] = invalid
                        with self.assertRaises(scenario.ScenarioFailure):
                            scenario._check_special_state(case, epoch, selected, bad, [{}])
                    with self.assertRaises(scenario.ScenarioFailure):
                        scenario._check_special_state(case, epoch, selected, {}, [{}])
        # Other actors do not inherit Qadeathfall's equipment assumptions.
        for name, case in (("Qadeathcast", "stuck-death"), ("Qadeathwisp", "wisp")):
            snapshot = {}
            if case == "wisp":
                snapshot = {"character_aura": {"columns": ["guid", "spell"],
                                               "rows": [["74", "8326"], ["74", "20584"]]}}
            scenario._check_special_state(case, 1, actor(name, 74), snapshot, [{}])

    def test_wisp_release_requires_both_actual_actor_auras_not_a_persisted_passive(self) -> None:
        selected = actor("Qadeathwisp", 74)
        snapshot = {
            "character_aura": {"columns": ["guid", "spell"],
                               "rows": [["74", "8326"], ["74", "20584"]]},
            "character_spell": {"columns": ["guid", "spell"], "rows": []},
        }
        observations = [{}]
        scenario._check_special_state("wisp", 1, selected, snapshot, observations)
        self.assertEqual({row["spell"] for row in observations[0]["auras"]}, {8326, 20584})
        for missing in (8326, 20584):
            bad = copy.deepcopy(snapshot)
            bad["character_aura"]["rows"] = [row for row in bad["character_aura"]["rows"]
                                             if row[1] != str(missing)]
            bad["character_aura"]["rows"].append(["75", str(missing)])
            with self.subTest(missing=missing), self.assertRaises(scenario.ScenarioFailure):
                scenario._check_special_state("wisp", 1, selected, bad, [{}])
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._check_special_state("wisp", 1, selected, {}, [{}])

    def test_packed_player_guid_and_legal_wisp_seed(self) -> None:
        wisp = actor("Qadeathwisp", 0x123456)
        guid = scenario.player_guid(wisp, 7)
        self.assertEqual(guid["high"], (2 << 58) | (7 << 42))
        self.assertEqual(scenario.packed_guid(guid["low"], guid["high"]),
                         bytes((0x07, 0xA0, 0x56, 0x34, 0x12, 0x1C, 0x08)))
        seeds = {(item["actor"], item["spell"]) for item in death_fixture.SPELL_SEEDS}
        self.assertIn(("Qadeathwisp", 20585), seeds)
        self.assertNotIn(("Qadeathwisp", 20584), seeds)

    def test_stuck_cases_keep_original_scenarios_and_bind_to_journal_caster(self) -> None:
        self.assertEqual(len(scenario.SCENARIOS), 12)
        self.assertEqual(sum(scenario.SCENARIOS.values()), 18)
        self.assertEqual({name: count for name, count in scenario.SCENARIOS.items()
                          if name not in {"stuck-death", "stuck-home"}}, {
            "fall-release": 1, "reclaim": 1, "healer": 2, "wisp": 2,
            "reincarnation": 1, "mob": 1, "automatic-release": 1, "offer": 2,
            "recovery-dead": 2, "recovery-ghost": 2,
        })
        self.assertEqual(scenario.SCENARIOS["stuck-death"], 2)
        self.assertEqual(scenario.SCENARIOS["stuck-home"], 1)
        selected = actor("Qadeathcast", 73)
        death_first = scenario.plan_for("stuck-death", 1, selected, 1, 10)
        death_second = scenario.plan_for("stuck-death", 2, selected, 1, 10)
        home = scenario.plan_for("stuck-home", 1, selected, 1, 10)
        for plan in (death_first, death_second, home):
            self.assertEqual(plan["character_guid"], selected["guid"])
            self.assertEqual(plan["finish_policy"], "logout")
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.plan_for("stuck-death", 3, selected, 1, 10)

    def test_stuck_death_plans_cast_hearthstone_then_kill_and_later_repop(self) -> None:
        selected = actor("Qadeathcast", 74)
        first = scenario.plan_for("stuck-death", 1, selected, 2, 10)
        self.assertEqual([action["action"] for action in first["actions"]],
                         ["cast", "wait_for_packet", "wait", "cast", "wait_for_packet"])
        self.assertEqual([action["spell_id"] for action in first["actions"]
                          if action["action"] == "cast"], [8690, 7355])
        self.assertEqual([action["opcode"] for action in first["actions"]
                          if action["action"] == "wait_for_packet"],
                         [scenario.MOVE_TELEPORT, scenario.MOVE_ROOT])
        self.assertEqual([action["milliseconds"] for action in first["actions"]
                          if action["action"] == "wait"], [2000])
        self.assertTrue(all(action.get("target") == scenario.player_guid(selected, 2)
                            for action in first["actions"] if action["action"] == "cast"))
        self.assertEqual([(rule.get("event"), rule.get("spell_id"), rule.get("opcode"))
                          for rule in first["expect"]], [
            ("prepare", None, None), ("start", 8690, None), ("go", 8690, None),
            ("prepare", None, None), ("start", 7355, None), ("go", 7355, None),
            ("packet", None, scenario.LOGOUT_COMPLETE),
        ])
        self.assertEqual([rule["target"] for rule in first["expect"]
                          if rule["event"] in {"start", "go"}],
                         ["empty", {"unit": {"low": 0, "high": 0}}, "empty", "empty"])
        self.assertNotIn(scenario.DURABILITY_DAMAGE_DEATH,
                         [rule.get("opcode") for rule in first["expect"]])

        second = scenario.plan_for("stuck-death", 2, selected, 2, 10)
        self.assertEqual([action["spell_id"] for action in second["actions"]
                          if action["action"] == "cast"], [7355])
        self.assertFalse(any(action["action"] in {"release_spirit", "spirit_healer_activate", "self_res"}
                             for action in second["actions"]))
        self.assertEqual([rule.get("spell_id") for rule in second["expect"]
                          if rule.get("event") == "go"], [7355])
        self.assertEqual(second["timeout_ms"], 15_000)
        self.assertEqual([rule["event"] for rule in second["expect"]],
                         ["prepare", "start", "go", "packet"])
        self.assertTrue(all(rule["target"] == "empty" for rule in second["expect"]
                            if rule["event"] in {"start", "go"}))

    def test_stuck_home_plan_has_no_hearthstone_cast_or_cooldown_seed_action(self) -> None:
        selected = actor("Qadeathcast", 75)
        plan = scenario.plan_for("stuck-home", 1, selected, 1, 10)
        self.assertEqual([action["spell_id"] for action in plan["actions"]
                          if action["action"] == "cast"], [7355])
        self.assertEqual([action["opcode"] for action in plan["actions"]
                          if action["action"] == "wait_for_packet"], [scenario.MOVE_TELEPORT])
        self.assertEqual([rule.get("spell_id") for rule in plan["expect"]
                          if rule.get("event") == "go"], [7355])
        self.assertEqual(plan["finish_policy"], "logout")
        self.assertEqual([rule["event"] for rule in plan["expect"]],
                         ["prepare", "start", "go", "packet"])
        self.assertTrue(all(rule["target"] == "empty" for rule in plan["expect"]
                            if rule["event"] in {"start", "go"}))

    def test_stuck_death_requires_the_explicit_pvp_durability_guard(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            config = Path(root) / "worldserver.conf"
            config.write_text("DurabilityLoss.InPvP = 0\n", encoding="utf-8")
            scenario._require_stuck_death_config("stuck-death", config)
            scenario._require_stuck_death_config("stuck-home", config)
            for setting in ("1", "true", ""):
                config.write_text(f"DurabilityLoss.InPvP = {setting}\n", encoding="utf-8")
                with self.subTest(setting=setting), self.assertRaises(scenario.ScenarioFailure):
                    scenario._require_stuck_death_config("stuck-death", config)

    def test_expected_packet_order_and_nonrelease_deaths(self) -> None:
        actors = journal_actors()
        fall = scenario.plan_for("fall-release", 1, actors["Qadeathfall"], 1, 10)
        self.assertEqual([rule["opcode"] for rule in fall["expect"]],
                         [scenario.DURABILITY_DAMAGE_DEATH, scenario.PRE_RESURRECT,
                          scenario.LOGOUT_COMPLETE])
        offer = scenario.plan_for("offer", 1, actors["Qadeathfall"], 1, 10)
        self.assertEqual([rule["opcode"] for rule in offer["expect"]],
                         [scenario.DURABILITY_DAMAGE_DEATH, scenario.LOGOUT_COMPLETE])
        reinc = scenario.plan_for("reincarnation", 1, actors["Qadeathreinc"], 1, 10)
        self.assertEqual([rule["opcode"] for rule in reinc["expect"]],
                         [scenario.DURABILITY_DAMAGE_DEATH, scenario.LOGOUT_COMPLETE])
        wisp = scenario.plan_for("wisp", 1, actors["Qadeathwisp"], 1, 10)
        self.assertEqual([rule["opcode"] for rule in wisp["expect"]],
                         [scenario.DURABILITY_DAMAGE_DEATH, scenario.PRE_RESURRECT,
                          scenario.LOGOUT_COMPLETE])
        recover = scenario.plan_for("recovery-dead", 1, actors["Qadeathfall"], 1, 10)
        self.assertEqual(recover["finish_policy"], "disconnect")
        self.assertEqual([rule["opcode"] for rule in recover["expect"]],
                         [scenario.DURABILITY_DAMAGE_DEATH])

    def test_each_bot_plan_has_exactly_one_normal_logout_or_none_on_disconnect(self) -> None:
        actors = journal_actors()
        plans = [
            scenario.plan_for("fall-release", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("reclaim", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("healer", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("healer", 2, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("wisp", 1, actors["Qadeathwisp"], 1, 10),
            scenario.plan_for("wisp", 2, actors["Qadeathwisp"], 1, 10),
            scenario.plan_for("reincarnation", 1, actors["Qadeathreinc"], 1, 10),
            scenario.plan_for("mob", 1, actors["Qadeathmob"], 1, 10),
            scenario.plan_for("automatic-release", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("offer", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("offer", 2, actors["Qadeathfall"], 1, 10, actors),
            scenario.plan_for("recovery-dead", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("recovery-dead", 2, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("recovery-ghost", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("recovery-ghost", 2, actors["Qadeathfall"], 1, 10),
        ]
        for plan in plans:
            count = sum(rule.get("opcode") == scenario.LOGOUT_COMPLETE for rule in plan["expect"])
            self.assertEqual(count, 0 if plan["finish_policy"] == "disconnect" else 1)

    def test_case_windows_and_wisp_relogin_plan_are_bounded(self) -> None:
        actors = journal_actors()
        self.assertEqual(scenario.plan_for("fall-release", 1, actors["Qadeathfall"], 1, 10)["timeout_ms"], 10_000)
        self.assertEqual(scenario.plan_for("reclaim", 1, actors["Qadeathfall"], 1, 10)["timeout_ms"], 45_000)
        self.assertEqual(scenario.plan_for("healer", 2, actors["Qadeathfall"], 1, 10)["timeout_ms"], 3_000)
        mob = scenario.plan_for("mob", 1, actors["Qadeathmob"], 1, 10)
        self.assertEqual(mob["timeout_ms"], 105_000)
        self.assertEqual([action["action"] for action in mob["actions"]],
                         ["attack_swing", "wait_for_packet", "attack_stop", "wait_for_packet"])
        self.assertEqual(
            [(action.get("connection"), action.get("opcode"), action.get("timeout_ms"))
             for action in mob["actions"] if action["action"] == "wait_for_packet"],
            [("instance", scenario.ATTACKER_STATE_UPDATE, 10_000),
             ("instance", scenario.MOVE_ROOT, 90_000)],
        )
        automatic = scenario.plan_for("automatic-release", 1, actors["Qadeathfall"], 1, 10)
        self.assertEqual(automatic["timeout_ms"], 390_000)
        self.assertFalse(any(action["action"] == "release_spirit" for action in automatic["actions"]))
        self.assertEqual(scenario.wisp_expiry_plan(actors["Qadeathwisp"])["actions"],
                         [{"action": "wait", "milliseconds": 75_000}])
        self.assertEqual(scenario.wisp_expiry_plan(actors["Qadeathwisp"])["timeout_ms"], 78_000)

    def test_wisp_epoch_two_heals_then_relogs_without_refalling(self) -> None:
        actors = journal_actors()
        plan = scenario.plan_for("wisp", 2, actors["Qadeathwisp"], 1, 10)
        self.assertEqual([item["action"] for item in plan["actions"]], ["spirit_healer_activate"])
        self.assertEqual(plan["character_guid"], actors["Qadeathwisp"]["guid"])
        observer = scenario.observer_plan(150_000, actors["Qadeathcast"]["guid"])
        self.assertEqual(observer["character_guid"], actors["Qadeathcast"]["guid"])
        self.assertTrue(observer["observe_only"])

    def test_offer_plan_uses_validated_journal_actor_guids(self) -> None:
        actors = journal_actors()
        plan = scenario.plan_for("offer", 2, actors["Qadeathfall"], 3, 10, actors)
        responses = [action for action in plan["actions"] if action["action"] == "resurrect_response"]
        self.assertEqual([item["response"] for item in responses], [0, 1, 0, 0])
        self.assertEqual(responses[0]["offerer"], scenario.player_guid(actors["Qadeathmob"], 3))
        self.assertEqual(responses[2]["offerer"], scenario.player_guid(actors["Qadeathcast"], 3))
        self.assertEqual(plan["actions"][0]["opcode"], scenario.RESURRECT_REQUEST)

    def test_wisp_state_validation_is_epoch_specific_and_checks_corpse_owner(self) -> None:
        selected = actor("Qadeathwisp", 44)
        ghost = {"characters": {"columns": ["guid", "health", "playerFlags"],
                                 "rows": [["44", "1", str(scenario.PLAYER_GHOST)]]},
                 "corpse": {"columns": ["guid"], "rows": [["44"]]}}
        state = scenario.validate_state("wisp", 1, selected, ghost)
        self.assertTrue(state["ghost"])
        bad_health = copy.deepcopy(ghost)
        bad_health["characters"]["rows"][0][1] = "0"
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_state("wisp", 1, selected, bad_health)
        alive = {"characters": {"columns": ["guid", "health", "playerFlags"],
                                 "rows": [["44", "1", "0"]]},
                 "corpse": {"columns": ["guid"], "rows": []}}
        self.assertFalse(scenario.validate_state("wisp", 2, selected, alive)["ghost"])
        wrong_owner = copy.deepcopy(ghost)
        wrong_owner["corpse"]["rows"] = [["45"]]
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_state("wisp", 1, selected, wrong_owner)

    def test_native_logout_persists_ghost_after_unreleased_wire_death(self) -> None:
        selected = actor("Qadeathfall", 55)
        ghost = {"characters": {"columns": ["guid", "health", "playerFlags"],
                                 "rows": [["55", "1", str(scenario.PLAYER_GHOST)]]},
                 "corpse": {"columns": ["guid"], "rows": [["55"]]}}
        for case, epoch in (("offer", 1), ("mob", 1), ("recovery-dead", 1),
                            ("recovery-dead", 2)):
            self.assertEqual(scenario.validate_state(case, epoch, selected, ghost)["health"], 1)
            bad_states = []
            for field, value in ((1, "0"), (1, "100"), (2, "0")):
                bad = copy.deepcopy(ghost)
                bad["characters"]["rows"][0][field] = value
                bad_states.append(bad)
            for rows in ([], [["56"]], [["55"], ["55"]]):
                bad = copy.deepcopy(ghost)
                bad["corpse"]["rows"] = rows
                bad_states.append(bad)
            for bad in bad_states:
                with self.subTest(case=case, epoch=epoch), self.assertRaises(scenario.ScenarioFailure):
                    scenario.validate_state(case, epoch, selected, bad)

    def test_stuck_death_logout_requires_ghost_while_home_remains_alive(self) -> None:
        selected = actor("Qadeathcast", 56)
        ghost = {"characters": {"columns": ["guid", "health", "playerFlags"],
                                 "rows": [["56", "1", str(scenario.PLAYER_GHOST)]]},
                 "corpse": {"columns": ["guid"], "rows": [["56"]]}}
        for epoch in (1, 2):
            self.assertEqual(scenario.validate_state("stuck-death", epoch, selected, ghost)["health"], 1)
        alive = {"characters": {"columns": ["guid", "health", "playerFlags"],
                                 "rows": [["56", "100", "0"]]},
                 "corpse": {"columns": ["guid"], "rows": []}}
        self.assertEqual(scenario.validate_state("stuck-home", 1, selected, alive)["health"], 100)
        dead = copy.deepcopy(alive)
        dead["characters"]["rows"][0][1] = "0"
        no_corpse = copy.deepcopy(ghost)
        no_corpse["corpse"]["rows"] = []
        dead_home = copy.deepcopy(alive)
        dead_home["characters"]["rows"][0][1] = "0"
        for bad in (dead, no_corpse, alive):
            with self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_state("stuck-death", 2, selected, bad)
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_state("stuck-home", 1, selected, dead_home)

    def test_new_death_is_proven_before_finish_and_native_release_belongs_to_shutdown(self) -> None:
        actors = journal_actors()
        for case, name in (("mob", "Qadeathmob"), ("offer", "Qadeathfall"),
                           ("recovery-dead", "Qadeathfall"), ("stuck-death", "Qadeathcast")):
            selected = actors[name]
            plan = scenario.plan_for(case, 1, selected, 1, 10)
            evidence = {"trace": [
                {"sequence": 2416, "kind": "packet_redacted", "connection": "instance",
                 "opcode": scenario.MOVE_ROOT},
                {"sequence": 3033, "kind": "action_started",
                 "action": f"finish_{plan['finish_policy']}"},
            ], "sent_packets": []}
            if plan["finish_policy"] == "logout":
                evidence["trace"].append({"sequence": 4118, "kind": "packet_observed",
                                          "connection": "realm", "opcode": scenario.PRE_RESURRECT})
            persisted = {"guid": selected["guid"], "health": 1, "ghost": True,
                         "owned_corpse_count": 1}
            observation = {"state": persisted}
            scenario._validate_death_shutdown_evidence(case, 1, plan, evidence, observation)
            self.assertEqual(observation["pre_shutdown"]["phase"], "PreShutdown")
            self.assertEqual(observation["pre_shutdown"]["state"], "unreleased_death")
            self.assertEqual(observation["pre_shutdown"]["root_trace_sequences"], [2416])
            shutdown = observation["shutdown_persistence_phase"]
            self.assertEqual(shutdown["phase"], "ShutdownPersistencePhase")
            self.assertEqual(shutdown["offline_state"], persisted)
            for invalid in ("no_root", "late_root", "early_pre_res", "untraced_pre_res",
                            "release", "no_finish"):
                bad = copy.deepcopy(evidence)
                if invalid == "no_root":
                    bad["trace"] = bad["trace"][1:]
                elif invalid == "late_root":
                    bad["trace"][0]["sequence"] = 3034
                elif invalid == "early_pre_res":
                    bad["trace"].append({"sequence": 3032, "kind": "packet_observed",
                                         "connection": "realm", "opcode": scenario.PRE_RESURRECT})
                elif invalid == "untraced_pre_res":
                    bad["events"] = [{"sequence": 0, "opcode": scenario.PRE_RESURRECT}]
                elif invalid == "release":
                    bad["sent_packets"].append({"action": "release_spirit"})
                else:
                    bad["trace"] = [item for item in bad["trace"] if item.get("kind") != "action_started"]
                with self.subTest(case=case, invalid=invalid), self.assertRaises(scenario.ScenarioFailure):
                    scenario._validate_death_shutdown_evidence(case, 1, plan, bad, {"state": persisted})
            manual_plan = copy.deepcopy(plan)
            manual_plan["actions"].append({"action": "release_spirit"})
            with self.assertRaises(scenario.ScenarioFailure):
                scenario._validate_death_shutdown_evidence(case, 1, manual_plan, evidence, {"state": persisted})

    def test_reload_epochs_prove_existing_ghost_without_requiring_a_new_death_root(self) -> None:
        for case, name in (("recovery-dead", "Qadeathfall"), ("stuck-death", "Qadeathcast")):
            selected = actor(name, 56)
            plan = scenario.plan_for(case, 2, selected, 1, 10)
            evidence = {"trace": [{"sequence": 1, "kind": "action_started", "action": "finish_logout"}],
                        "sent_packets": []}
            state = {"guid": 56, "health": 1, "ghost": True, "owned_corpse_count": 1}
            reload_input = {"state": state, "snapshot_sha256": "a" * 64}
            observation = {"state": copy.deepcopy(state)}
            scenario._validate_death_shutdown_evidence(case, 2, plan, evidence, observation, reload_input)
            self.assertEqual(observation["pre_shutdown"]["state"], "persisted_ghost")
            self.assertNotIn("root_trace_sequences", observation["pre_shutdown"])
            for invalid in (None, {"state": {**state, "guid": 57}},
                            {"state": {**state, "ghost": False}},
                            {"state": {**state, "owned_corpse_count": 0}}):
                with self.assertRaises(scenario.ScenarioFailure):
                    scenario._validate_death_shutdown_evidence(case, 2, plan, evidence, {"state": state}, invalid)

    def test_stuck_graveyard_position_uses_quarter_yard_tolerance(self) -> None:
        selected = actor("Qadeathcast", 57)
        snapshot = {"characters": {"columns": ["guid", "map", "position_x", "position_y",
                                                 "position_z"],
                                    "rows": [["57", "0", "-9339.72", "170.2", "61.5622"]]}}
        position = scenario._validate_stuck_graveyard_position(snapshot, selected)
        self.assertEqual(position["map"], 0)
        self.assertLessEqual(position["distance_from_graveyard_106"], 0.25)
        near_home = copy.deepcopy(snapshot)
        near_home["characters"]["rows"][0][2:] = ["-9340.21", "167.61", "61.60"]
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._validate_stuck_graveyard_position(near_home, selected)

    def test_stuck_first_epoch_proves_hearthstone_cooldown_and_ack_before_kill(self) -> None:
        selected = actor("Qadeathcast", 58)
        snapshot = {"character_spell_cooldown": {
            "columns": ["guid", "spell", "time", "categoryId", "categoryEnd"],
            "rows": [["58", "8690", "2000000000", "1176", "2000000000"]],
        }}
        evidence = {
            "events": [
                {"opcode": scenario.SPELL_GO, "spell_id": 8690, "target": {
                    "flags": 0x42, "unit": {"low": 0, "high": 0},
                    "item": {"low": 0, "high": 0},
                    "dst_location": {"transport": {"low": 0, "high": 0},
                                     **dict(zip(("x", "y", "z"), selected["home"]))}}},
                {"opcode": scenario.SPELL_GO, "spell_id": 7355},
                {"opcode": scenario.MOVE_ROOT},
            ],
            "sent_packets": [
                {"action": "cast", "action_index": 0},
                {"action": "ack_move_teleport", "opcode": scenario.MOVE_TELEPORT_ACK,
                 "connection": "instance", "write_attempted": True,
                 "write_completed": True, "write_error": None},
                {"action": "cast", "action_index": 2},
            ],
        }
        observation: dict = {}
        scenario._validate_stuck_evidence("stuck-death", 1, selected, snapshot,
                                          evidence, observation)
        self.assertEqual(observation["stuck_spell_cooldowns"][0]["categoryid"], 1176)
        for defect in ("unit", "flags", "destination", "missing_destination"):
            invalid = copy.deepcopy(evidence)
            target = invalid["events"][0]["target"]
            if defect == "unit":
                target["unit"] = scenario.player_guid(selected, 1)
            elif defect == "flags":
                target["flags"] = 2
            elif defect == "destination":
                target["dst_location"]["x"] += 1
            else:
                del target["dst_location"]
            with self.subTest(defect=defect), self.assertRaises(scenario.ScenarioFailure):
                scenario._validate_stuck_evidence("stuck-death", 1, selected, snapshot,
                                                  invalid, {})
        durability = copy.deepcopy(evidence)
        durability["events"].append({"opcode": scenario.DURABILITY_DAMAGE_DEATH})
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._validate_stuck_evidence("stuck-death", 1, selected, snapshot,
                                              durability, {})

    def test_stuck_home_rejects_any_saved_or_emitted_hearthstone_cooldown(self) -> None:
        selected = actor("Qadeathcast", 59)
        empty_snapshot = {"character_spell_cooldown": {
            "columns": ["guid", "spell", "time", "categoryId", "categoryEnd"], "rows": []}}
        evidence = {
            "events": [{"opcode": scenario.SPELL_GO, "spell_id": 7355}],
            "sent_packets": [
                {"action": "cast", "action_index": 0},
                {"action": "ack_move_teleport", "opcode": scenario.MOVE_TELEPORT_ACK,
                 "connection": "instance", "write_attempted": True,
                 "write_completed": True, "write_error": None},
            ],
        }
        scenario._validate_stuck_evidence("stuck-home", 1, selected, empty_snapshot,
                                          evidence, {})
        emitted = copy.deepcopy(evidence)
        emitted["events"].append({"opcode": scenario.COOLDOWN_EVENT,
                                  "body_hex": (8690).to_bytes(4, "little", signed=True).hex() + "00"})
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._validate_stuck_evidence("stuck-home", 1, selected, empty_snapshot,
                                              emitted, {})
        saved = {"character_spell_cooldown": {
            "columns": ["guid", "spell", "time", "categoryId", "categoryEnd"],
            "rows": [["59", "8690", "2000000000", "1176", "2000000000"]],
        }}
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._validate_stuck_evidence("stuck-home", 1, selected, saved,
                                              evidence, {})

    def test_dead_stuck_repop_requires_packet_evidence_and_exact_graveyard(self) -> None:
        selected = actor("Qadeathcast", 60)
        snapshot = {"characters": {"columns": ["guid", "map", "position_x", "position_y",
                                                 "position_z"],
                                    "rows": [["60", "0", "-9339.72", "170.2", "61.5622"]]},
                    "character_spell_cooldown": {
                        "columns": ["guid", "spell", "time", "categoryId", "categoryEnd"],
                        "rows": [["60", "8690", "2000000000", "1176", "2000000000"]]}}
        evidence = {"events": [{"opcode": scenario.SPELL_GO, "spell_id": 7355}],
                    "sent_packets": []}
        observation: dict = {}
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._validate_stuck_evidence("stuck-death", 2, selected, snapshot,
                                              evidence, observation)
        evidence["events"].append({"opcode": scenario.DEATH_RELEASE_LOC})
        scenario._validate_stuck_evidence("stuck-death", 2, selected, snapshot,
                                          evidence, observation)
        self.assertEqual(observation["graveyard_position"]["map"], 0)
        self.assertEqual(observation["stuck_spell_cooldowns"], [{
            "spell": 8690, "time": 2_000_000_000,
            "categoryid": 1176, "categoryend": 2_000_000_000,
        }])
        absent = copy.deepcopy(snapshot)
        absent["character_spell_cooldown"]["rows"] = []
        invalid_category = copy.deepcopy(snapshot)
        invalid_category["character_spell_cooldown"]["rows"][0][3] = "0"
        invalid_time = copy.deepcopy(snapshot)
        invalid_time["character_spell_cooldown"]["rows"][0][2] = "0"
        invalid_category_end = copy.deepcopy(snapshot)
        invalid_category_end["character_spell_cooldown"]["rows"][0][4] = "0"
        for bad_snapshot in (absent, invalid_category, invalid_time, invalid_category_end):
            with self.assertRaises(scenario.ScenarioFailure):
                scenario._validate_stuck_evidence("stuck-death", 2, selected, bad_snapshot,
                                                  evidence, {})

    def test_mob_requires_combat_attack_stop_then_root_before_finish(self) -> None:
        evidence = {
            "trace": [
                {"sequence": 1, "kind": "action_started", "action": "attack_swing"},
                {"sequence": 2, "kind": "send_attempted", "action": "attack_swing"},
                {"sequence": 3, "kind": "action_completed", "action": "attack_swing"},
                {"sequence": 4, "kind": "packet_observed", "opcode": scenario.ATTACKER_STATE_UPDATE,
                 "connection": "instance", "event_sequence": 0},
                {"sequence": 5, "kind": "action_started", "action": "attack_stop"},
                {"sequence": 6, "kind": "send_attempted", "action": "attack_stop",
                 "opcode": scenario.ATTACK_STOP, "connection": "instance", "body_bytes": 0},
                {"sequence": 7, "kind": "local_write_completed", "action": "attack_stop",
                 "opcode": scenario.ATTACK_STOP, "connection": "instance", "body_bytes": 0},
                {"sequence": 8, "kind": "action_completed", "action": "attack_stop"},
                {"sequence": 9, "kind": "packet_observed", "opcode": scenario.MOVE_ROOT,
                 "connection": "instance"},
                {"sequence": 10, "kind": "action_started", "action": "finish_logout"},
            ],
            "events": [{"sequence": 0, "opcode": scenario.ATTACKER_STATE_UPDATE,
                        "body_bytes": 24, "body_hex": "aa"}],
            "target_selections": [{"action": "attack_swing", "selector": {"entry": 6, "map_id": 0},
                                   "guid": {"low": 90, "high": 1},
                                   "source_event_sha256": "a" * 64}],
        }
        observed: dict = {}
        scenario._validate_mob_evidence(evidence, observed)
        self.assertEqual(observed["mob_evidence"]["damage_phases"],
                         [{"phase": "post_attack_observation", "event": evidence["events"][0]}])
        self.assertEqual(observed["mob_evidence"]["root_phases"], ["post_attack_observation"])
        self.assertEqual(observed["mob_evidence"]["attack_stop_trace_sequence"], 6)
        for invalid in ("stale_combat", "missing_attack_stop", "stop_before_attack",
                        "root_before_stop", "late_root", "missing_target"):
            bad = copy.deepcopy(evidence)
            if invalid == "stale_combat":
                bad["trace"][3]["sequence"] = 0
            elif invalid == "missing_attack_stop":
                bad["trace"] = [item for item in bad["trace"]
                                 if item.get("action") != "attack_stop"]
            elif invalid == "stop_before_attack":
                bad["trace"][5]["sequence"] = 0
            elif invalid == "root_before_stop":
                bad["trace"][8]["sequence"] = 5
            elif invalid == "late_root":
                bad["trace"][8]["sequence"] = 11
            else:
                bad["target_selections"] = []
            with self.subTest(invalid=invalid), self.assertRaises(scenario.ScenarioFailure):
                scenario._validate_mob_evidence(bad, {})

    def test_snapshot_pass_names_do_not_collide_and_wisp_expiry_uses_owner_rows(self) -> None:
        selected = actor("Qadeathwisp", 44)
        output = Path("/private")
        first = scenario._snapshot_path(output, "wisp", 2, selected, "wisp-healer")
        second = scenario._snapshot_path(output, "wisp", 2, selected, "wisp-expiry-relogin")
        self.assertNotEqual(first, second)
        healer = {"auras": [{"spell": 15007, "maxduration": 600_000, "remaintime": 35_098}]}
        expiry = {"auras": []}
        facts = scenario._validate_wisp_sickness(healer, expiry)
        self.assertEqual(facts["expiry_relogin_wait_ms"], 75_000)
        self.assertEqual(facts["initial_max_duration_ms"], 600_000)
        full_minute = copy.deepcopy(healer)
        full_minute["auras"][0]["remaintime"] = 60_000
        scenario._validate_wisp_sickness(full_minute, expiry)
        for field, value in (("maxduration", 60_000), ("maxduration", 0),
                             ("remaintime", 0), ("remaintime", -1), ("remaintime", 60_001)):
            bad = copy.deepcopy(healer)
            bad["auras"][0][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(scenario.ScenarioFailure):
                scenario._validate_wisp_sickness(bad, expiry)
        with self.assertRaisesRegex(scenario.ScenarioFailure, "remained"):
            scenario._validate_wisp_sickness(healer, copy.deepcopy(healer))

    def test_observer_nonzero_exit_requires_observe_only_evidence_and_failure_is_private(self) -> None:
        self.assertTrue(scenario._observer_exit_allowed(1))
        self.assertFalse(scenario._observer_exit_allowed(2))
        selected = actor("Qadeathcast", 70)
        report = {"results": [{"character_guid": 70, "cast_lifecycle": {
            "failure": None, "observe_only": True, "passed": False,
            "finish_completed": True}}]}
        evidence = scenario._result_for(report, selected, observer=True)
        self.assertFalse(evidence["passed"])
        report["results"][0]["cast_lifecycle"]["observe_only"] = False
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._result_for(report, selected, observer=True)
        payload = scenario._failure_report(RuntimeError("secret-password"), "wisp", 2)
        self.assertNotIn("secret-password", json.dumps(payload))
        self.assertNotIn("traceback", json.dumps(payload).casefold())

    def test_ready_marker_is_explicit(self) -> None:
        self.assertTrue(scenario._has_ready_marker("INFO CAST_LIFECYCLE_READY"))
        self.assertFalse(scenario._has_ready_marker("login stream started"))


if __name__ == "__main__":
    unittest.main()
