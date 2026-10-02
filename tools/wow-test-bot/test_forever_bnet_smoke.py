"""Hermetic parser and fixture-shape tests for forever_bnet_smoke."""
import json
import hashlib
import struct
import sys
import unittest
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import forever_bnet_smoke as smoke
import forever_bnet_wire as wire


def challenge():
    return {
        "version": 2, "iterations": 15000, "hash_function": "SHA-256",
        "modulus": smoke.SRP_V2_MODULUS.hex().upper(), "generator": "02", "salt": "00" * 32,
        "username": smoke.EXPECTED_USERNAME, "public_B": "02",
    }


def realm_blob(name="RustyCore Forever - Login Test", version=(1, 60, 1, 70170), flags=2,
               realm_id=1, realm_address=0x02010001, population=0, deleting=False):
    major, minor, revision, build = version
    document = {"updates": [{"update": {
        "wowRealmAddress": realm_address, "populationState": population, "cfgRealmsId": realm_id,
        "name": name, "flags": flags,
        "version": {"versionMajor": major, "versionMinor": minor,
                     "versionRevision": revision, "versionBuild": build},
    }, "deleting": deleting}]}
    text = b"JSONRealmListUpdates:" + json.dumps(document).encode() + b"\0"
    return len(text).to_bytes(4, "little") + zlib.compress(text)


class ForeverBnetWireTests(unittest.TestCase):
    def test_v2_record_projection_and_notification_order(self):
        game = wire.var(1, 1) + wire.var(2, 0x576F57) + wire.var(3, 2)
        record = wire.var(1, 1) + wire.raw(2, game) + wire.raw(5, b"x" * 64)
        complete = wire.var(1, 0) + wire.raw(2, record)
        self.assertEqual(wire.v2_logon_record(complete), {
            "account_id": 1, "game_account_id": 1, "region": 2, "session_key_length": 64})
        for bad in (b"", wire.var(1, 3), wire.var(1, 0) + wire.raw(2, wire.var(1, 1)),
                    complete.replace(b"x" * 64, b"y" * 63)):
            with self.assertRaises(ValueError): wire.v2_logon_record(bad)
        rpc = object.__new__(smoke.RPCSession)
        rpc.reply_seen, rpc.v2_record, rpc.challenge = False, None, None
        acknowledgements = []
        rpc.respond = acknowledgements.append
        header = {11: smoke.ALIST_V2, 2: 1, 3: 7}
        with self.assertRaises(ValueError): rpc.notification(header, complete)
        rpc.reply_seen = True
        rpc.notification(header, complete)
        self.assertEqual(acknowledgements, [7])
        self.assertIsNotNone(rpc.v2_record)
        with self.assertRaises(ValueError): rpc.notification({11: smoke.ALIST_V2, 2: 4}, b"")

    def test_fields_decode_supported_wire_types(self):
        encoded = wire.var(1, 42) + wire.raw(2, b"abc") + wire.fixed(3, 0xAABBCCDD)
        self.assertEqual(list(wire.fields(encoded)), [
            (1, 0, 42), (2, 2, b"abc"), (3, 5, b"\xdd\xcc\xbb\xaa")])

    def test_fields_reject_truncation_overflow_and_limits(self):
        malformed = (b"\x80", b"\x0a", b"\x0a\x03x", b"\x1d\x01", b"\x0b")
        for value in malformed:
            with self.subTest(value=value), self.assertRaises(ValueError):
                list(wire.fields(value))
        with self.assertRaises(ValueError):
            list(wire.fields(b"\x80" * 10 + b"\x01"))
        with self.assertRaises(ValueError):
            list(wire.fields(b"\x00"))
        with self.assertRaises(ValueError):
            list(wire.fields(b"x" * (wire.MAX_PROTO + 1)))
        with self.assertRaises(ValueError):
            list(wire.fields(wire.raw(1, b"") * 4097))

    def test_rpc_frame_round_trip_and_required_fields(self):
        header = wire.rpc_header(0, 1, 7, 3, smoke.AUTH)
        frame = struct.pack(">H", len(header)) + header + b"abc"
        parsed, payload = wire.decode_frame(frame)
        self.assertEqual(parsed[1], 0)
        self.assertEqual(parsed[3], 7)
        self.assertEqual(parsed[5], 3)
        self.assertEqual(parsed[11], smoke.AUTH)
        self.assertEqual(payload, b"abc")
        with self.assertRaises(ValueError): wire.decode_frame(frame[:1])
        with self.assertRaises(ValueError): wire.decode_frame(frame[:-1])
        with self.assertRaises(ValueError): wire.decode_frame(frame + b"x")
        with self.assertRaises(ValueError): wire.parse_rpc_header(wire.var(2, 1))

    def test_http_header_is_case_insensitive_and_bounded(self):
        self.assertEqual(wire.parse_http_header(
            b"HTTP/1.1 200 OK\r\ncOnTeNt-LeNgTh: 3\r\nX-Test: yes\r\n\r\n"), (200, 3))
        for value in (b"", b"HTTP/1.1 200 OK\r\n\r\n", b"HTTP/1.1 99 Bad\r\nContent-Length: 0\r\n\r\n",
                      f"HTTP/1.1 200 OK\r\nContent-Length: {wire.MAX_HTTP_BODY + 1}\r\n\r\n".encode()):
            with self.subTest(value=value), self.assertRaises(ValueError):
                wire.parse_http_header(value)
        with self.assertRaises(ValueError):
            wire.parse_http_header(b"x" * (wire.MAX_HTTP_HEADER + 1))

    def test_http_transport_preserves_close_and_session_cookie(self):
        cookie = ("JSESSIONID=01234567-89ab-cdef-0123-456789abcdef; "
                  "Path=/bnetserver; Domain=127.0.0.1; Secure; HttpOnly; SameSite=None")
        status, length, headers = wire.parse_http_response_header(
            ("HTTP/1.1 200 OK\r\ncontent-length: 2\r\nset-cookie: " + cookie +
             "\r\nConnection: close\r\n\r\n").encode("ascii"))
        self.assertEqual((status, length), (200, 2))
        session = wire.extract_session_cookie(headers["set-cookie"])
        self.assertEqual(session, "JSESSIONID=01234567-89ab-cdef-0123-456789abcdef")
        request = wire.http_request("/bnetserver/login/", b"{}", "127.0.0.1:18081", session)
        self.assertIn(b"Connection: close\r\n", request)
        self.assertIn(b"Cookie: JSESSIONID=01234567-89ab-cdef-0123-456789abcdef\r\n", request)
        self.assertTrue(request.endswith(b"\r\n\r\n{}"))

    def test_session_cookie_rejects_missing_cpp_attributes_or_bad_uuid(self):
        base = "JSESSIONID=01234567-89ab-cdef-0123-456789abcdef"
        for suffix in ("", "; Secure; HttpOnly; SameSite=None", "; Path=/other; Secure; HttpOnly; SameSite=None",
                       "; Path=/bnetserver; Secure; SameSite=None", "; Path=/bnetserver; Secure; HttpOnly; SameSite=Lax"):
            with self.subTest(suffix=suffix), self.assertRaises(ValueError):
                wire.extract_session_cookie(base + suffix)
        with self.assertRaises(ValueError):
            wire.extract_session_cookie("JSESSIONID=not-a-uuid; Path=/bnetserver; Secure; HttpOnly; SameSite=None")

    def test_modern_connect_identity_and_header_wire(self):
        ciid = b"0000001200000034-0000009A000000BC"
        process = lambda label, epoch: wire.var(1, label) + wire.var(2, epoch)
        base = wire.raw(1, process(0x12, 0x34)) + wire.raw(2, process(0x9A, 0xBC))
        payload = base + wire.var(7, 1) + wire.raw(9, ciid)
        header = wire.parse_rpc_header(wire.rpc_header(0xFE, 0, 1, ciid=ciid))
        self.assertEqual(wire.connect_identity(payload, header), ciid)
        for invalid in (base + wire.raw(9, ciid), base + wire.var(7, 0) + wire.raw(9, ciid),
                        base + wire.var(7, 1), payload.replace(ciid, ciid.lower()),
                        wire.raw(1, process(0x12, 0x34)) + wire.var(7, 1) + wire.raw(9, ciid)):
            with self.subTest(payload=invalid), self.assertRaises(ValueError):
                wire.connect_identity(invalid, header)
        for wrong_header in ({}, {13: ciid.lower()}):
            with self.assertRaises(ValueError): wire.connect_identity(payload, wrong_header)
        with self.assertRaises(ValueError):
            wire.parse_rpc_header(wire.rpc_header(0xFE, 0, 1) + wire.var(13, 1))

    def test_attributes_decode_ticket_value_not_name_only(self):
        body = wire.client_request([wire.attribute("Param_RealmListTicket", b"AuthRealmListTicket")])
        self.assertEqual(wire.response_attributes(body)["Param_RealmListTicket"], b"AuthRealmListTicket")
        wrong = wire.client_request([wire.attribute("Param_RealmListTicket", b"not-a-ticket")])
        self.assertNotEqual(wire.response_attributes(wrong)["Param_RealmListTicket"], b"AuthRealmListTicket")

    def test_modern_attribute_wire_uses_v2_variant_tags(self):
        for value, string in ((b"AuthRealmListTicket\0", False), ("2-1-0", True)):
            v1 = wire.client_request([wire.attribute("Param_Test", value, string)])
            v2 = wire.client_request([wire.attribute("Param_Test", value, string, v2=True)])
            self.assertNotEqual(v1, v2)
            expected = value.encode() if isinstance(value, str) else value
            self.assertEqual(wire.response_attributes(v2, v2=True), {"Param_Test": expected})


class ForeverBnetShapeTests(unittest.TestCase):
    def test_super_district_schema_and_jam_envelope(self):
        def blob(value, prefix=b"JSONSuperDistrictList:", nul=b"\0"):
            data = prefix + json.dumps(value).encode() + nul
            return len(data).to_bytes(4, "little") + zlib.compress(data)
        row = {"superDistrictID": 1, "disallowLogin": False, "holdDownUntilTime": 0}
        self.assertEqual(smoke.super_district_metadata(blob({"superDistricts": [row]})), [row])
        self.assertEqual(smoke.super_district_metadata(blob({"superDistricts": []})), [])
        for value in ([], {}, {"superDistricts": {}}, {"superDistricts": [row, row]},
                      {"superDistricts": [{**row, "superDistrictID": True}]},
                      {"superDistricts": [{**row, "superDistrictID": 0}]},
                      {"superDistricts": [{**row, "holdDownUntilTime": -1}]},
                      {"superDistricts": [{**row, "disallowLogin": 0}]}):
            with self.subTest(value=value), self.assertRaises(ValueError):
                smoke.super_district_metadata(blob(value))
        valid = blob({"superDistricts": [row]})
        for invalid in (None, b"", valid + b"extra", valid[:-1],
                        (0x40001).to_bytes(4, "little") + valid[4:],
                        blob({"superDistricts": [row]}, nul=b""),
                        blob({"superDistricts": [row]}, prefix=b"JSONRealmListUpdates:")):
            with self.subTest(value=invalid), self.assertRaises(ValueError):
                smoke.super_district_metadata(invalid)

    def test_empty_bleep_proxy_envelope(self):
        value = b'JSONBleepProxyList:{"proxies":[]}\0'
        blob = len(value).to_bytes(4, "little") + zlib.compress(value)
        self.assertEqual(smoke.discovery_blob(blob, b"JSONBleepProxyList:"), {"proxies": []})

    def test_tls_identity_is_localhost_and_loopback(self):
        self.assertEqual((smoke.HOST, smoke.SERVER_NAME), ("127.0.0.1", "localhost"))

    def test_srp_challenge_validation_accepts_expected_shape(self):
        self.assertEqual(smoke.EXPECTED_USERNAME,
                         hashlib.sha256(b"FOREVER@LOCAL.TEST").hexdigest().upper())
        self.assertEqual(smoke.validate_srp_challenge(challenge())[1:], (2, b"\0" * 32, smoke.EXPECTED_USERNAME))
        A, m1, m2 = smoke.srp_proof(challenge(), "private")
        self.assertGreater(A, 0)
        self.assertEqual((len(m1), len(m2)), (32, 64))

    def test_srp_challenge_validation_rejects_wrong_hash_and_bounds(self):
        for key, value in (("hash_function", "SHA-512"), ("iterations", 1),
                           ("generator", "03"), ("salt", "00"), ("username", "ab" * 32),
                           ("public_B", "00"), ("modulus", "F" * 510)):
            invalid = challenge(); invalid[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                smoke.validate_srp_challenge(invalid)
        invalid = challenge(); invalid["public_B"] = invalid["modulus"]
        with self.assertRaises(ValueError): smoke.validate_srp_challenge(invalid)
        invalid = challenge(); invalid["modulus"] = "A" * 512
        with self.assertRaises(ValueError): smoke.validate_srp_challenge(invalid)
        invalid = challenge(); invalid["public_b"] = invalid.pop("public_B")
        with self.assertRaises(ValueError): smoke.validate_srp_challenge(invalid)

    def test_password_reader_is_bounded_and_matches_fixture_rules(self):
        self.assertEqual(smoke.parse_password(b" pass \r\n"), " pass ")
        for value in (b"", b"\r\n", b"a" * 129, b"a" * 1025, b"\xff"):
            with self.subTest(value=value), self.assertRaises(ValueError): smoke.parse_password(value)

    def test_rejected_srp_result_matches_rust_and_cpp_shapes(self):
        rust = {"authentication_state": "DONE", "error_code": None, "error_message": None,
                "url": None, "login_ticket": None, "server_evidence_M2": None}
        cpp = {"authentication_state": "DONE"}
        smoke.validate_rejected_result(200, rust)
        smoke.validate_rejected_result(200, cpp)
        for status, result in ((401, cpp), (200, {"authentication_state": "LOGIN"}),
                               (200, {"authentication_state": "DONE", "login_ticket": "present"}),
                               (200, {"authentication_state": "DONE", "extra": None})):
            with self.subTest(status=status, result=result), self.assertRaises(ValueError):
                smoke.validate_rejected_result(status, result)

    def test_realm_metadata_positive_and_negative(self):
        self.assertEqual(smoke.realm_metadata(realm_blob()), {
            "name": "RustyCore Forever - Login Test", "build": 70170,
            "version": [1, 60, 1], "offline": True, "realm_id": 1,
            "wow_realm_address": 0x02010001, "deleting": False})
        for kwargs in ({"version": (1, 60, 2, 70170)}, {"flags": 0}, {"name": "other"},
                       {"deleting": True}, {"realm_id": 2}, {"realm_address": 0}, {"population": 1}):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                smoke.realm_metadata(realm_blob(**kwargs))
        with self.assertRaises(ValueError): smoke.realm_metadata(b"\x00")
        with self.assertRaises(ValueError): smoke.realm_metadata(b"\x04\x00\x00\x00bad")


if __name__ == "__main__":
    unittest.main()
