"""Offline regression tests for the #46 four-client stability runner."""

from contextlib import nullcontext
import fcntl
import json
import os
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

import four_client_stability as stability


class FourClientStabilityTests(unittest.TestCase):
    def test_plans_hold_a_session_for_the_wave_and_request_raid_info(self):
        origin = (-8948.0, -80.0, 88.78)
        for character in stability.CHARACTERS:
            plan = stability.build_plan(character, 3, origin, 290)
            actions = plan["actions"]
            movements = [action for action in actions if action["action"] == "movement"]
            self.assertEqual(len(movements), 8)
            self.assertGreaterEqual(
                len({(move["x"], move["y"], move["z"]) for move in movements}), 2
            )
            self.assertEqual(sum(action["milliseconds"] for action in actions
                                 if action["action"] == "wait"), 290_000)
            requests = [action for action in actions if action["action"] == "request_raid_info"]
            self.assertEqual(len(requests), 1)
            self.assertIn({"action": "wait_for_packet", "connection": "realm",
                           "opcode": stability.INSTANCE_INFO, "timeout_ms": 5_000}, actions)
            self.assertEqual(plan["finish_policy"], "logout")
            self.assertEqual(plan["expect"][0]["opcode"], stability.LOGOUT_COMPLETE)

    def test_preflight_plan_keeps_two_positions_and_short_wait_schedule(self):
        plan = stability.build_plan(15, 1, stability.NORTHSHIRE_ORIGIN, 50)
        movements = [action for action in plan["actions"] if action["action"] == "movement"]
        self.assertEqual(len(movements), 2)
        self.assertGreater(len({(row["x"], row["y"], row["z"]) for row in movements}), 1)
        self.assertEqual(plan["timeout_ms"], 65_000)

    def test_bot_report_accepts_confirmed_logout_without_disconnect_policy(self):
        plan = stability.build_plan(15, 3, stability.NORTHSHIRE_ORIGIN, 290)
        last_move = next(action for action in reversed(plan["actions"])
                         if action["action"] == "movement")
        point = (last_move["x"], last_move["y"], last_move["z"])
        evidence = {
            "passed": True,
            "failure": None,
            "finish_policy": "logout",
            "finish_completed": True,
            "logout_request_completed": True,
            "logout_confirmed": True,
            "logout_route": "realm",
            "local_disconnect_recorded": False,
            "server_disconnect_connection": None,
            "sent_packets": [{
                "action": "request_raid_info", "connection": "realm",
                "opcode": stability.REQUEST_RAID_INFO, "body_hex": "",
                "write_completed": True, "observation_sequence": 3,
            }],
            "events": [{
                "connection": "realm", "opcode": stability.INSTANCE_INFO,
                "body_hex": "00000000", "observation_sequence": 4,
            }],
            "movement_state": {"map_id": 0, "x": point[0], "y": point[1], "z": point[2]},
        }
        with tempfile.TemporaryDirectory() as root:
            report = Path(root) / "bot.json"
            report.write_text(json.dumps({"results": [{"character_guid": 15,
                                                         "cast_lifecycle": evidence}]}))
            result = stability.validate_bot_report(report, 15, point)
        self.assertEqual(result["raid_info_lock_count"], 0)
        self.assertTrue(result["disconnected"])

    def test_bot_report_rejects_unconfirmed_logout_or_server_disconnect(self):
        point = (-8948.0, -80.0, 88.78)
        base = {
            "passed": True,
            "failure": None,
            "finish_policy": "logout",
            "finish_completed": True,
            "logout_request_completed": True,
            "logout_confirmed": True,
            "logout_route": "realm",
            "local_disconnect_recorded": False,
            "server_disconnect_connection": None,
            "sent_packets": [{
                "action": "request_raid_info", "connection": "realm",
                "opcode": stability.REQUEST_RAID_INFO, "body_hex": "",
                "write_completed": True, "observation_sequence": 3,
            }],
            "events": [{
                "connection": "realm", "opcode": stability.INSTANCE_INFO,
                "body_hex": "00000000", "observation_sequence": 4,
            }],
            "movement_state": {"map_id": 0, "x": point[0], "y": point[1], "z": point[2]},
        }
        mutations = (
            {"logout_confirmed": False},
            {"server_disconnect_connection": "realm"},
        )
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as root:
                evidence = {**base, **mutation}
                report = Path(root) / "bot.json"
                report.write_text(json.dumps({"results": [{
                    "character_guid": 15, "cast_lifecycle": evidence,
                }]}))
                with self.assertRaises(stability.ScenarioFailure):
                    stability.validate_bot_report(report, 15, point)

    def test_second_wave_verifies_reloaded_positions_from_previous_logout(self):
        with tempfile.TemporaryDirectory() as root:
            root_path = Path(root)
            capture = root_path / "capture"
            capture.mkdir(mode=0o700)
            config = root_path / "bots.json"
            config.write_text(json.dumps({"bots": [
                {"enabled": True, "character_guid": guid,
                 "account": f"qa{guid}", "password": "test-only"}
                for guid in stability.CHARACTERS
            ]}))
            bot = root_path / "wow-test-bot"
            bot.write_bytes(b"offline-test-binary")
            bot.chmod(0o700)
            clock = {"now": 0.0}
            queue_values = iter((0, 1, 2, 3))
            queue_sample_times = []
            previous_positions = {
                guid: [float(guid), float(guid + 1), float(guid + 2)]
                for guid in stability.CHARACTERS
            }

            def fake_wave(**kwargs):
                wave = kwargs["wave"]
                clock["now"] += 6_960 if wave == 1 else 1
                return {
                    "wave": wave,
                    "start_sequence": wave * 100,
                    "end_sequence": wave * 100 + 50,
                    "characters": [{
                        "character_guid": guid,
                        "movement_position": previous_positions[guid],
                    } for guid in stability.CHARACTERS],
                }

            def queue_sample(_pid, _starttime):
                value = next(queue_values)
                queue_sample_times.append(clock["now"])
                return {name: value for name in stability.QUEUE_FIELDS}

            def fake_sleep(seconds):
                clock["now"] += seconds

            cli = stability._parse_args([
                "--capture-dir", str(capture), "--config", str(config),
                "--bot", str(bot), "--duration-seconds", "7200",
                "--wave-seconds", "240",
            ])
            env = {"RUSTYCORE_CAPTURE_CANDIDATE_SHA256": "a" * 64}
            with (
                patch.object(stability, "_process_identity_from_env", return_value=(123, 456)),
                patch.object(stability.instance_soak, "read_memory", return_value={}),
                patch.object(stability.instance_soak, "memory_samples",
                             return_value=nullcontext(lambda: None)),
                patch.object(stability.instance_soak, "queue_sample", side_effect=queue_sample),
                patch.object(stability, "_verify_running_process_sha256") as verify_candidate,
                patch.object(stability, "run_wave", side_effect=fake_wave),
                patch.object(stability.verify_four_client_movement,
                             "verify_relogin_positions", side_effect=lambda _capture, _first,
                             _last, expected: expected) as verify_relogin,
                patch.object(stability.time, "monotonic", side_effect=lambda: clock["now"]),
                patch.object(stability.time, "sleep", side_effect=fake_sleep),
            ):
                report = stability.run(cli, env)

            self.assertEqual(len(report["workload"]), 2)
            verify_candidate.assert_called_once_with(123, 456, "a" * 64)
            self.assertEqual(verify_relogin.call_count, 1)
            verify_relogin.assert_called_once_with(
                capture, 200, 250, previous_positions,
            )
            self.assertEqual(report["workload"][1]["reloaded_positions"], previous_positions)
            self.assertEqual(queue_sample_times, [0.0, 6_960.0, 6_961.0, 7_200.0])
            self.assertTrue(report["command_queue_samples"][-1]["campaign_end"])
            self.assertEqual(report["command_queue_samples"][-1]["command_full"], 3)

    def test_raid_info_count_rejects_truncated_rows(self):
        with self.assertRaisesRegex(stability.ScenarioFailure, "count does not match"):
            stability._raid_info_count("0100000000")

    def test_candidate_pin_must_match_running_process_executable(self):
        with (
            patch.object(stability.instance_soak, "read_memory", return_value={}) as read_memory,
            patch.object(stability, "_sha256", return_value="b" * 64) as sha256,
        ):
            with self.assertRaisesRegex(stability.ScenarioFailure, "does not match the candidate"):
                stability._verify_running_process_sha256(123, 456, "a" * 64)

        sha256.assert_called_once_with(Path("/proc/123/exe"))
        self.assertEqual(read_memory.call_count, 2)

    def test_capture_pruner_keeps_movement_and_relogin_streams(self):
        with tempfile.TemporaryDirectory() as root:
            capture = Path(root)
            unrelated = capture / "scenario-report.json"
            unrelated.write_text("{}")
            pruner = stability.CapturePruner(capture, 1024)
            old = time.time() - 5

            def packet(direction, sequence, name):
                stem = (f"rust-{direction}-{sequence:08d}-counter0-0x2578-"
                        f"{name}-len2")
                binary = capture / f"{stem}.bin"
                metadata = capture / f"{stem}.meta"
                binary.write_bytes(b"xx")
                metadata.write_text("seq=1\n")
                os.utime(binary, (old, old))
                os.utime(metadata, (old, old))
                return binary, metadata

            move = packet("s2c", 10, "MoveUpdate")
            auth = packet("s2c", 11, "AuthChallenge")
            login = packet("s2c", 12, "LoginVerifyWorld")
            pruner.scan_and_prune()
            self.assertTrue(all(path.exists() for path in move))
            self.assertTrue(all(path.exists() for path in login))
            self.assertTrue(all(not path.exists() for path in auth))
            self.assertTrue(unrelated.exists())
            self.assertEqual(pruner.current_range(), (13, 12))

    def test_capture_pruner_refuses_preexisting_packet_files(self):
        with tempfile.TemporaryDirectory() as root:
            capture = Path(root)
            (capture / "rust-s2c-00000001-counter0-0x2578-PhaseShiftChange-len2.bin").write_bytes(b"xx")
            with self.assertRaisesRegex(stability.ScenarioFailure, "pre-existing"):
                stability.CapturePruner(capture, 1024)

    def test_explicit_sequence_boundary_preserves_prior_phase_captures(self):
        with tempfile.TemporaryDirectory() as root:
            capture = Path(root)
            for sequence in (9, 10):
                stem = (f"rust-s2c-{sequence:08d}-counter0-0x2578-"
                        "PhaseShiftChange-len2")
                (capture / f"{stem}.bin").write_bytes(b"xx")
                (capture / f"{stem}.meta").write_text(f"seq={sequence}\n")
            pruner = stability.CapturePruner(capture, 1024, first_sequence=11)
            self.assertEqual(pruner.current_range(), (11, 10))
            self.assertEqual(len(list(capture.glob("rust-*.bin"))), 2)

    def test_sequence_boundary_accepts_explicit_environment_value(self):
        with patch.dict(os.environ, RUSTYCORE_CAPTURE_FIRST_SEQUENCE="11"):
            self.assertEqual(stability._parse_args([]).first_sequence, 11)

    def test_explicit_sequence_boundary_rejects_new_frames_before_boundary(self):
        with tempfile.TemporaryDirectory() as root:
            capture = Path(root)
            pruner = stability.CapturePruner(capture, 1024, first_sequence=11)
            stem = "rust-s2c-00000010-counter0-0x2578-MoveUpdate-len2"
            (capture / f"{stem}.bin").write_bytes(b"xx")
            (capture / f"{stem}.meta").write_text("seq=10\n")
            with self.assertRaisesRegex(stability.ScenarioFailure, "before the explicit sequence"):
                pruner.scan_and_prune()

    def test_process_stdout_queue_line_parser_reads_only_the_required_counters(self):
        line = (b"INFO Instance soak command queue totals command_full=2 "
                b"command_disconnected=1 command_stale=4 retry_full=0\n")
        self.assertEqual(stability.ProcessStdoutQueueSampler._decode_queue_line(line), {
            "command_full": 2, "command_disconnected": 1,
            "command_stale": 4, "retry_full": 0,
        })

    def test_process_stdout_sampler_discards_backlog_and_reads_fresh_same_inode_line(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "world.stdout"
            backlog = (b"INFO Instance soak command queue totals command_full=1 "
                       b"command_disconnected=1 command_stale=1 retry_full=1\n")
            path.write_bytes(backlog)
            path.chmod(0o600)
            pid = 12345
            real_stat = os.stat
            fresh = (b"INFO Instance soak command queue totals command_full=2 "
                     b"command_disconnected=3 command_stale=4 retry_full=5\n")

            def append_fresh(_seconds):
                with path.open("ab") as stream:
                    stream.write(fresh)

            def same_server_stdout(candidate, *args, **kwargs):
                if os.fspath(candidate) == f"/proc/{pid}/fd/1":
                    return real_stat(path)
                return real_stat(candidate, *args, **kwargs)

            with (patch.object(stability.os, "stat", side_effect=same_server_stdout),
                  patch.object(stability.instance_soak, "read_memory", return_value={}),
                  patch.object(stability.time, "sleep", side_effect=append_fresh)):
                sampler = stability.ProcessStdoutQueueSampler(path, pid, 456)
                try:
                    access = fcntl.fcntl(sampler.fd, fcntl.F_GETFL) & os.O_ACCMODE
                    self.assertEqual(access, os.O_RDONLY)
                    sample = sampler.sample()
                finally:
                    sampler.close()
        self.assertEqual({name: sample[name] for name in stability.QUEUE_FIELDS}, {
            "command_full": 2, "command_disconnected": 3,
            "command_stale": 4, "retry_full": 5,
        })

    def test_process_stdout_sampler_discards_a_prior_sample_partial_line(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "world.stdout"
            path.touch(mode=0o600)
            pid = 12345
            real_stat = os.stat
            complete = (b"INFO Instance soak command queue totals command_full=1 "
                        b"command_disconnected=1 command_stale=1 retry_full=1\n")
            partial = b"INFO Instance soak command queue totals command_full=2 "
            continuation = b"command_disconnected=2 command_stale=2 retry_full=2\n"
            sleep_calls = 0
            clock = {"now": 0}

            def same_server_stdout(candidate, *args, **kwargs):
                if os.fspath(candidate) == f"/proc/{pid}/fd/1":
                    return real_stat(path)
                return real_stat(candidate, *args, **kwargs)

            def append_wave_lines(_seconds):
                nonlocal sleep_calls
                sleep_calls += 1
                if sleep_calls == 1:
                    with path.open("ab") as stream:
                        stream.write(complete + partial)
                elif sleep_calls == 2:
                    with path.open("ab") as stream:
                        stream.write(continuation)

            def monotonic():
                value = clock["now"]
                clock["now"] += 1
                return value

            with (patch.object(stability.os, "stat", side_effect=same_server_stdout),
                  patch.object(stability.instance_soak, "read_memory", return_value={}),
                  patch.object(stability.time, "sleep", side_effect=append_wave_lines),
                  patch.object(stability.time, "monotonic", side_effect=monotonic)):
                sampler = stability.ProcessStdoutQueueSampler(path, pid, 456)
                try:
                    first = sampler.sample()
                    self.assertEqual(first["command_full"], 1)
                    with self.assertRaisesRegex(
                            stability.ScenarioFailure, "no fresh queue counters"):
                        sampler.sample()
                finally:
                    sampler.close()

    def test_process_stdout_sampler_buffers_a_fresh_line_split_across_reads(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "world.stdout"
            path.touch(mode=0o600)
            pid = 12345
            real_stat = os.stat
            line = (b"INFO Instance soak command queue totals command_full=6 "
                    b"command_disconnected=7 command_stale=8 retry_full=9\n")
            pieces = iter((line[:41], line[41:]))

            def same_server_stdout(candidate, *args, **kwargs):
                if os.fspath(candidate) == f"/proc/{pid}/fd/1":
                    return real_stat(path)
                return real_stat(candidate, *args, **kwargs)

            def append_next_piece(_seconds):
                with path.open("ab") as stream:
                    stream.write(next(pieces))

            clock = {"now": 0}

            def monotonic():
                value = clock["now"]
                clock["now"] += 1
                return value

            with (patch.object(stability.os, "stat", side_effect=same_server_stdout),
                  patch.object(stability.instance_soak, "read_memory", return_value={}),
                  patch.object(stability.time, "sleep", side_effect=append_next_piece),
                  patch.object(stability.time, "monotonic", side_effect=monotonic)):
                sampler = stability.ProcessStdoutQueueSampler(path, pid, 456)
                try:
                    sample = sampler.sample()
                finally:
                    sampler.close()

        self.assertEqual({name: sample[name] for name in stability.QUEUE_FIELDS}, {
            "command_full": 6, "command_disconnected": 7,
            "command_stale": 8, "retry_full": 9,
        })


if __name__ == "__main__":
    unittest.main()
