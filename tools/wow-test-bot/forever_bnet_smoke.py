#!/usr/bin/env python3
"""Stdlib-only local WoW Forever 1.60.1 BNet REST/RPC smoke.

The wire operations follow Rust `rpc/session.rs`, `rpc/services/authentication.rs`,
`game_utilities.rs` and C++ fork a22fd98b (Server/Session.cpp,
REST/LoginRESTService.cpp); this is not a claim of retail-client parity.
By default the bounded result ends at an offline realm list. The explicit
discovery mode checks an online realm projection but never joins a world.
"""
import argparse
import hashlib
import json
import secrets
import socket
import ssl
import struct
import sys
import zlib
from pathlib import Path
from forever_bnet_join import client_information, join_metadata, join_request
from forever_bnet_wire import (MAX_HTTP_BODY, MAX_HTTP_HEADER, attribute, client_request,
                               extract_session_cookie, fields, http_request,
                               parse_http_response_header, parse_rpc_header, raw,
                               response_attributes, rpc_header, var, connect_identity,
                               v2_attributes, v2_int_attribute, v2_logon_record)

HOST, REST, RPC = "127.0.0.1", 18081, 1119
EMAIL, BUILD, VERSION = "FOREVER@LOCAL.TEST", 70170, "1.60.1"
RESPONSE, CONN, AUTH, GAME, ALIST = 0xFE, 0x65446991, 0x0DECFC01, 0x3FC1274D, 0x71240E35
AUTH_V2, ALIST_V2 = 0xC02F8216, 0x9DA8116B
ACCOUNT_V2, GAME_V2 = 0x22DC2464, 0x5DBB51C2
MAJOR, MINOR, REVISION = 1, 60, 1
SERVER_NAME = "localhost"
MAX_PASSWORD_CHARS, MAX_PASSWORD_FILE_BYTES = 128, 1024

# wow-crypto/src/bnet_srp6.rs:N_V2_HEX (the BNet SRPv2 RFC-3526 group).
SRP_V2_MODULUS = bytes.fromhex(
    "AC6BDB41324A9A9BF166DE5E1389582FAF72B6651987EE07FC3192943DB56050"
    "A37329CBB4A099ED8193E0757767A13DD52312AB4B03310DCD7F48A9DA04FD50"
    "E8083969EDB767B0CF6095179A163AB3661A05FBD5FAAAE82918A9962F0B93B8"
    "55F97993EC975EEAA80D740ADBF4FF747359D041D5C33EA71D281E446B14773B"
    "CA97B43A23FB801676BD207A436C6481F1D2B9078717461A5B9D32E688F87748"
    "544523B524B0D57D5EA77A2775D2ECFA032CFBDBF52FB3786160279004E57AE6"
    "AF874E7303CE53299CCC041C7BC308D82A5698F3A8D0C38271AE35F8E9DBFBB6"
    "94B5C803D89F7AE435DE236D525F54759B65E372FCD68EF20FA7111F9E4AFF73"
)
EXPECTED_USERNAME = hashlib.sha256(EMAIL.encode("ascii")).hexdigest().upper()  # wow-crypto::srp_username() for this fixed uppercase ASCII email.

class TLS:
    def __init__(self, runtime, port):
        cert = runtime / "bnetserver.cert.pem"
        if not cert.is_file(): raise ValueError("runtime certificate is missing")
        context = ssl.create_default_context(ssl.Purpose.SERVER_AUTH, cafile=str(cert))
        sock = socket.create_connection((HOST, port), timeout=10)
        self.sock = context.wrap_socket(sock, server_hostname=SERVER_NAME)
        if self.sock.getpeercert(binary_form=True) != ssl.PEM_cert_to_DER_cert(
                cert.read_text(encoding="ascii")):
            self.close(); raise ValueError("fixture certificate mismatch")
        self.sock.settimeout(10); self.buf = bytearray()

    def read(self, size):
        if size < 0 or size > MAX_HTTP_BODY: raise ValueError("TLS read is too large")
        while len(self.buf) < size:
            chunk = self.sock.recv(max(4096, size - len(self.buf)))
            if not chunk: raise OSError("peer closed TLS connection")
            self.buf.extend(chunk)
        value = bytes(self.buf[:size]); del self.buf[:size]; return value

    def until(self, marker):
        while True:
            index = self.buf.find(marker)
            if index >= 0:
                end = index + len(marker); value = bytes(self.buf[:end])
                if end > MAX_HTTP_HEADER: raise ValueError("HTTP header is too large")
                del self.buf[:end]; return value
            if len(self.buf) > MAX_HTTP_HEADER: raise ValueError("HTTP header is too large")
            chunk = self.sock.recv(4096)
            if not chunk: raise OSError("peer closed TLS connection")
            self.buf.extend(chunk)

    def close(self):
        try: self.sock.close()
        except (AttributeError, OSError): pass


def parse_password(raw):
    if len(raw) > MAX_PASSWORD_FILE_BYTES: raise ValueError("password file is too large")
    try: value = raw.rstrip(b"\r\n").decode("utf-8")
    except UnicodeDecodeError as error: raise ValueError("password file is not UTF-8") from error
    if not value: raise ValueError("empty password")
    if len(value) > MAX_PASSWORD_CHARS: raise ValueError("password is too long")
    return value


def read_password(path):
    with path.open("rb") as stream:
        return parse_password(stream.read(MAX_PASSWORD_FILE_BYTES + 1))

def http_json(conn, path, value, cookie=None):
    body = json.dumps(value, separators=(",", ":")).encode()
    conn.sock.sendall(http_request(path, body, f"{HOST}:{REST}", cookie, close=True))
    status, length, headers = parse_http_response_header(conn.until(b"\r\n\r\n"))
    return status, json.loads(conn.read(length)), headers


def rest_json(runtime, path, value, cookie=None):
    """Issue one REST request on one TLS connection, honoring Connection: close."""
    conn = TLS(runtime, REST)
    try:
        return http_json(conn, path, value, cookie)
    finally:
        conn.close()


def broken(value):
    return value.to_bytes(max(1, (value.bit_length() + 8) >> 3), "big")


def _hex(value, length, name):
    if not isinstance(value, str): raise ValueError(f"invalid SRP {name}")
    size_ok = len(value) == length if isinstance(length, int) else length[0] <= len(value) <= length[1] and len(value) % 2 == 0
    if not size_ok or any(c not in "0123456789abcdefABCDEF" for c in value):
        raise ValueError(f"invalid SRP {name}")
    try: return bytes.fromhex(value)
    except ValueError as error: raise ValueError(f"invalid SRP {name}") from error


def validate_srp_challenge(challenge):
    if not isinstance(challenge, dict) or challenge.get("version") != 2 or challenge.get("iterations") != 15000:
        raise ValueError("unexpected SRPv2 challenge")
    if challenge.get("hash_function") != "SHA-256": raise ValueError("unexpected SRP hash")
    modulus, generator = _hex(challenge.get("modulus"), len(SRP_V2_MODULUS) * 2, "modulus"), _hex(challenge.get("generator"), 2, "generator")
    if modulus != SRP_V2_MODULUS or generator != b"\x02": raise ValueError("invalid SRP parameters")
    salt = _hex(challenge.get("salt"), 64, "salt")
    username = challenge.get("username")
    if username != EXPECTED_USERNAME:
        raise ValueError("invalid SRP username")
    if "public_B" not in challenge or "public_b" in challenge:
        raise ValueError("invalid SRP public B field")
    public_b = _hex(challenge.get("public_B"), (2, 512), "public B")
    B, N = int.from_bytes(public_b, "big"), int.from_bytes(modulus, "big")
    if not 0 < B < N: raise ValueError("invalid SRP public B")
    return N, int.from_bytes(generator, "big"), salt, username


def srp_proof(challenge, password):
    n, g, salt, user = validate_srp_challenge(challenge)
    width = (n.bit_length() + 7) // 8
    xb = hashlib.pbkdf2_hmac("sha512", f"{user}:{password}".encode(), salt, 15000, 64)
    xu, n1 = int.from_bytes(xb, "big"), n - 1
    if xb[0] & 128:
        rem = ((1 << 512) - xu) % n1; x = 0 if rem == 0 else n1 - rem
    else: x = xu % n1
    a = int.from_bytes(secrets.token_bytes(width), "big") % n1 or 1
    A, B = pow(g, a, n), int(challenge["public_B"], 16)
    h = lambda value: int.from_bytes(hashlib.sha256(value).digest(), "big")
    k = h(n.to_bytes(width, "big") + g.to_bytes(width, "big"))
    u = h(A.to_bytes(width, "big") + B.to_bytes(width, "big"))
    S = pow((B - k * pow(g, x, n)) % n, a + u * x, n)
    m1 = hashlib.sha256(broken(A) + broken(B) + broken(S)).digest()
    m2 = hashlib.sha256(broken(A) + broken(int.from_bytes(m1, "big")) + broken(S)).hexdigest().upper()
    return A, m1, m2


def rest_login(runtime, password, negative):
    status, challenge, headers = rest_json(runtime, "/bnetserver/login/srp/", {
        "inputs": [{"input_id": "account_name", "value": EMAIL}]})
    if status != 200: raise ValueError("SRP challenge HTTP failure")
    validate_srp_challenge(challenge)
    cookie = extract_session_cookie(headers.get("set-cookie"))
    A, m1, expected_m2 = srp_proof(challenge, password)
    public_a = A.to_bytes(256, "big").hex().upper()

    def prove(proof):
        return rest_json(runtime, "/bnetserver/login/", {"inputs": [
            {"input_id": "public_A", "value": public_a},
            {"input_id": "client_evidence_M1", "value": proof.hex().upper()}]}, cookie)

    if negative:
        bad = bytearray(m1); bad[-1] ^= 1
        status, result, _ = prove(bytes(bad))
        validate_rejected_result(status, result)
        # Reuse the original challenge after the failed proof on another TCP.
        status, result, _ = prove(m1)
        ticket = validate_success_result(status, result, expected_m2)
        return {"challenge": True, "session_cookie": True, "wrong_m1_rejected": True,
                "valid_after_wrong_m1": True, "ticket": bool(ticket)}

    status, result, _ = prove(m1)
    ticket = validate_success_result(status, result, expected_m2)
    return {"challenge": True, "session_cookie": True, "valid_m2": True,
            "ticket": True, "ticket_value": ticket}


def validate_rejected_result(status, result):
    allowed = {"authentication_state", "error_code", "error_message", "url", "login_ticket", "server_evidence_M2"}
    if status != 200 or not isinstance(result, dict) or set(result) - allowed:
        raise ValueError("unexpected SRP rejection response")
    if result.get("authentication_state") != "DONE": raise ValueError("SRP rejection was not DONE")
    if any(result.get(key) is not None for key in ("error_code", "error_message", "url", "login_ticket", "server_evidence_M2")):
        raise ValueError("SRP rejection contained a result")


def validate_success_result(status, result, expected_m2):
    if status != 200 or not isinstance(result, dict):
        raise ValueError("SRP proof HTTP failure")
    ticket = result.get("login_ticket")
    if result.get("authentication_state") != "DONE" or not isinstance(ticket, str) or not ticket:
        raise ValueError("no login ticket")
    if result.get("server_evidence_M2", "").upper() != expected_m2:
        raise ValueError("server M2 did not verify")
    return ticket

class RPCSession:
    def __init__(self, runtime):
        self.tls, self.token, self.logon_error = TLS(runtime, RPC), 1, None
        self.ciid = None
        self.challenge = None
        self.v2_record = None
        self.reply_seen = False

    def close(self): self.tls.close()

    def frame(self):
        size = struct.unpack(">H", self.tls.read(2))[0]
        if not size: raise ValueError("empty RPC header")
        header = parse_rpc_header(self.tls.read(size))
        if self.ciid is not None and header.get(13) != self.ciid:
            raise ValueError("RPC response/notification CIID mismatch")
        return header, self.tls.read(header.get(5, 0))

    def respond(self, token):
        header = rpc_header(RESPONSE, 0, token, ciid=self.ciid)
        self.tls.sock.sendall(struct.pack(">H", len(header)) + header)

    def send(self, service_hash, method, payload):
        token = self.token; self.token += 1
        self.reply_seen = False
        header = rpc_header(0, method, token, len(payload), service_hash, self.ciid)
        self.tls.sock.sendall(struct.pack(">H", len(header)) + header + payload)
        while True:
            header, body = self.frame()
            if header.get(1) == RESPONSE and header.get(3) == token:
                self.reply_seen = True
                if service_hash == CONN and method == 1 and not header.get(6, 0):
                    self.ciid = connect_identity(body, header)
                return header.get(6, 0), body
            self.notification(header, body)

    def notification(self, header, body):
        service, method = header.get(11), header.get(2)
        if service == ALIST and method == 5:
            self.logon_error = next((v for n, w, v in fields(body) if n == 1 and w == 0), None)
        elif service == ALIST_V2 and method == 4:
            if self.reply_seen: raise ValueError("V2 challenge arrived after Logon reply")
            self.challenge = {n: v for n, w, v in fields(body) if w == 2}
        elif service == ALIST_V2 and method == 1:
            if not self.reply_seen: raise ValueError("V2 completion arrived before auth reply")
            self.v2_record = v2_logon_record(body)
        else: raise ValueError("unexpected RPC notification")
        self.respond(header.get(3, 0))


def realm_metadata(blob):
    if len(blob) < 4 or len(blob) > MAX_HTTP_BODY: raise ValueError("realm blob truncated")
    expected = int.from_bytes(blob[:4], "little")
    if expected > MAX_HTTP_BODY: raise ValueError("realm blob is too large")
    try:
        decoder = zlib.decompressobj()
        text = decoder.decompress(blob[4:], MAX_HTTP_BODY + 1)
        if decoder.unconsumed_tail or decoder.unused_data or not decoder.eof:
            raise ValueError("invalid realm compression")
        text += decoder.flush(MAX_HTTP_BODY + 1)
        if len(text) > MAX_HTTP_BODY: raise ValueError("realm blob is too large")
    except zlib.error as error: raise ValueError("invalid realm compression") from error
    prefix = b"JSONRealmListUpdates:"
    if len(text) != expected or not text.startswith(prefix): raise ValueError("bad realm blob")
    try: updates = json.loads(text[len(prefix):].rstrip(b"\0")).get("updates", [])
    except (TypeError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("invalid realm JSON") from error
    if not isinstance(updates, list) or len(updates) != 1: raise ValueError("unexpected realm count")
    try:
        state = updates[0]
        realm, version = state["update"], state["update"]["version"]
        deleting = state["deleting"]
        flags, name = realm["flags"], realm["name"]
        realm_address, realm_id, population = realm["wowRealmAddress"], realm["cfgRealmsId"], realm["populationState"]
        numbers = (version["versionMajor"], version["versionMinor"],
                   version["versionRevision"], version["versionBuild"])
    except (KeyError, TypeError, IndexError, AttributeError) as error:
        raise ValueError("invalid realm metadata") from error
    major, minor, revision, build = numbers
    if type(deleting) is not bool or not all(type(value) is int for value in (flags, major, minor, revision, build, realm_address, realm_id, population)):
        raise ValueError("invalid realm metadata")
    if (name != "RustyCore Forever - Login Test" or
            major != MAJOR or minor != MINOR or revision != REVISION or build != BUILD or
            realm_id != 1 or realm_address != 0x02010001 or population != 0 or deleting or not flags & 2):
        raise ValueError("realm metadata mismatch")
    return {"name": name, "build": build, "version": [major, minor, revision],
            "offline": bool(flags & 2), "realm_id": realm_id,
            "wow_realm_address": realm_address, "deleting": deleting}


def rpc_login(runtime, ticket, expect_discovery_realm=False):
    rpc = RPCSession(runtime)
    try:
        status, _ = rpc.send(CONN, 1, b"")
        if status: raise ValueError("Connect failed")
        status, _ = rpc.send(0xFFFFFFFF, 1, b"")
        if status != 1: raise ValueError("status-only error response failed")
        logon = (raw(1, b"WoW") + raw(2, b"Wn64") + raw(3, b"esES") + raw(5, VERSION.encode()) +
                 var(6, BUILD) + raw(12, ticket.encode()))
        status, _ = rpc.send(AUTH, 1, logon)
        if status or rpc.logon_error != 0: raise ValueError("OnLogonComplete failed")
        secret = secrets.token_bytes(32)
        identity = b'JSONRealmListTicketIdentity:{"gameAccountID":1}'
        info = b"JSONRealmListTicketClientInformation:" + json.dumps(
            {"info": {"secret": list(secret)}}, separators=(",", ":")).encode()
        status, body = rpc.send(GAME, 1, client_request([
            attribute("Command_RealmListTicketRequest_v1_classic", "", True),
            attribute("Param_Identity", identity), attribute("Param_ClientInfo", info)]))
        ticket_attrs = response_attributes(body)
        if status or ticket_attrs.get("Param_RealmListTicket") != b"AuthRealmListTicket":
            raise ValueError("realm ticket failed")
        status, body = rpc.send(GAME, 1, client_request([
            attribute("Command_RealmListRequest_v1_classic", "2-1-0", True)]))
        if status: raise ValueError("realm list failed")
        attrs = response_attributes(body)
        result = {"connect": True, "ciid_all_frames": True, "status_only": True,
                  "on_logon_complete": True, "realm_list_ticket": True,
                  "realm_list": True, "realm_list_response_received": True}
        if expect_discovery_realm:
            # This mode asserts LastChar, not the offline RealmList fixture.
            del result["realm_list"]
            result["realm_metadata_checked"] = False
        else:
            result["realm"] = realm_metadata(attrs["Param_RealmList"])
            result["realm_metadata_checked"] = True
        return result
    finally: rpc.close()


def rpc_login_v2(runtime, password, expect_discovery_realm=False, expect_realm_join=False):
    rpc = RPCSession(runtime)
    try:
        status, _ = rpc.send(CONN, 1, b"")
        if status: raise ValueError("V2 Connect failed")
        for service, method in ((ACCOUNT_V2, 101), (ACCOUNT_V2, 104),
                                (ACCOUNT_V2, 201), (ACCOUNT_V2, 203),
                                (GAME_V2, 1), (GAME_V2, 2)):
            status, _ = rpc.send(service, method, b"")
            if status != 3: raise ValueError("post-auth service admitted unauthenticated request")
        logon = var(1, 0x576F57) + raw(2, b"Wn64") + raw(3, b"esES") + var(4, BUILD)
        for method, payload, expected in (
                (99, b"", 0xBC3), (1, b"\x52\xff", 0xBC5), (1, b"", 0x4D),
                (1, var(1, 0x576F57), 0x4F),
                (1, var(1, 0x576F57) + raw(2, b"Wn64"), 0x4E),
                (2, b"", 3), (2, raw(1, b"invalid-local-token"), 3), (3, b"", 3)):
            status, _ = rpc.send(AUTH_V2, method, payload)
            if status != expected: raise ValueError("V2 negative admission mismatch")
        status, _ = rpc.send(AUTH_V2, 0x40000001, logon)
        if status or rpc.challenge is None: raise ValueError("V2 external challenge missing")
        if (rpc.challenge.get(2) != b"web_auth_url" or rpc.challenge.get(3) not in
                (b"https://127.0.0.1:18081/bnetserver/login/", b"https://localhost:18081/bnetserver/login/")):
            raise ValueError("V2 challenge is not the isolated HTTPS target")
        valid = rest_login(runtime, password, False)
        ticket = valid["ticket_value"].encode()
        status, _ = rpc.send(AUTH_V2, 2, raw(1, ticket))
        if status: raise ValueError("V2 auth token rejected")
        rpc.notification(*rpc.frame())
        if rpc.v2_record is None: raise ValueError("V2 logon completion missing")
        status, body = rpc.send(AUTH_V2, 3, var(1, 0x576F57))
        generated = next((v for n, w, v in fields(body) if n == 1 and w == 2), None)
        if status or generated != ticket: raise ValueError("V2 generated token mismatch")
        # A fresh connection exercises the alternate cached-token Logon path.
        cached = RPCSession(runtime)
        try:
            status, _ = cached.send(CONN, 1, b"")
            if status: raise ValueError("V2 cached Connect failed")
            status, _ = cached.send(AUTH_V2, 1, logon + raw(10, raw(1, ticket)))
            if status: raise ValueError("V2 cached Logon failed")
            cached.notification(*cached.frame())
            if cached.v2_record is None: raise ValueError("V2 cached completion missing")
        finally: cached.close()
        post_login = rpc_post_login_v2(rpc, expect_discovery_realm, expect_realm_join)
        return {"external_challenge": True, "negative_admission": True, "reply_before_complete": True,
                "on_logon_complete": True, "generated_token": True, "cached_logon": True,
                "ciid_all_frames": True, "record": rpc.v2_record, "post_login": post_login}
    finally: rpc.close()


def discovery_blob(blob, prefix):
    """Strict bounded Jam JSON envelope, including its terminating NUL."""
    if not isinstance(blob, bytes) or not 4 <= len(blob) <= 0x40000:
        raise ValueError("invalid discovery blob")
    size = int.from_bytes(blob[:4], "little")
    if not 1 <= size <= 0x40000: raise ValueError("invalid discovery size")
    try:
        decoder = zlib.decompressobj()
        value = decoder.decompress(blob[4:], size + 1)
        if (decoder.unconsumed_tail or decoder.unused_data or not decoder.eof or
                len(value) != size or not value.startswith(prefix) or not value.endswith(b"\0")):
            raise ValueError("invalid discovery envelope")
        return json.loads(value[len(prefix):-1])
    except (zlib.error, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("invalid discovery JSON") from error


def last_char_request(filter_value):
    """Build the V2 LastChar request with a signed int64 content filter."""
    return client_request([
        attribute("Command_LastCharPlayedRequest_v1_classic", "70-1-70", True, v2=True),
        v2_int_attribute("Param_ContentSetIDFilter", filter_value),
    ])


def last_char_metadata(payload):
    """Validate the 70170 no-character LastChar routing projection."""
    attrs = v2_attributes(payload)
    expected = ["Param_RealmEntry", "Param_LastPlayedTime", "Param_UtilityInfo"]
    if [name for name, _, _ in attrs] != expected:
        raise ValueError("unexpected LastChar attributes")
    _, entry_kind, entry_blob = attrs[0]
    _, time_kind, last_played = attrs[1]
    _, utility_kind, utility_blob = attrs[2]
    if entry_kind != "blob" or time_kind != "int" or utility_kind != "blob":
        raise ValueError("unexpected LastChar attribute types")
    if type(last_played) is not int or last_played <= 0:
        raise ValueError("invalid LastChar timestamp")
    realm = discovery_blob(entry_blob, b"JamJSONRealmEntry:")
    if not isinstance(realm, dict):
        raise ValueError("invalid LastChar realm entry")
    try:
        version = realm["version"]
        required = (
            realm["wowRealmAddress"], realm["cfgRealmsID"], realm["cfgContentSetID"],
            realm["superDistrictID"], realm["flags"], realm["populationState"],
            realm["useBleepChance"], version["versionMajor"], version["versionMinor"],
            version["versionRevision"], version["versionBuild"],
        )
    except (KeyError, TypeError) as error:
        raise ValueError("incomplete LastChar realm entry") from error
    integer_fields = required[:6] + required[7:]
    if (not all(type(value) is int for value in integer_fields) or
            required[-1] != BUILD or required[0] != 0x02010001 or required[1] != 1 or
            required[2] != 136 or required[3] != 1 or required[4] != 0 or
            required[5] != 1 or type(required[6]) is not float or required[6] != 0.0 or
            version["versionMajor"] != MAJOR or version["versionMinor"] != MINOR or
            version["versionRevision"] != REVISION):
        raise ValueError("LastChar realm metadata mismatch")
    utility = discovery_blob(utility_blob, b"JSONUtilityInfo:")
    if utility != {"realmPermissions": 512}:
        raise ValueError("LastChar utility metadata mismatch")
    return {"wow_realm_address": required[0], "realm_id": required[1],
            "content_set_id": required[2], "super_district_id": required[3],
            "build": required[-1], "population_state": required[5],
            "last_played_time_positive": True, "utility_permissions": 512}


def super_district_metadata(blob):
    value = discovery_blob(blob, b"JSONSuperDistrictList:")
    if not isinstance(value, dict) or set(value) != {"superDistricts"}:
        raise ValueError("invalid district catalog")
    districts = value["superDistricts"]
    if not isinstance(districts, list) or len(districts) > 64:
        raise ValueError("invalid district count")
    ids = set()
    for row in districts:
        if not isinstance(row, dict) or set(row) != {"superDistrictID", "disallowLogin", "holdDownUntilTime"}:
            raise ValueError("invalid district fields")
        identity, held, until = row["superDistrictID"], row["disallowLogin"], row["holdDownUntilTime"]
        if (type(identity) is not int or not 1 <= identity <= 0x7FFFFFFF or identity in ids or
                type(held) is not bool or type(until) is not int or not 0 <= until <= 0xFFFFFFFF):
            raise ValueError("invalid district values")
        ids.add(identity)
    return districts


def rpc_post_login_v2(rpc, expect_discovery_realm=False, expect_realm_join=False):
    status, body = rpc.send(GAME_V2, 1, join_request())
    if status != 0x800000D3 or body:
        raise ValueError("realm join admitted before game-account selection")
    status, _ = rpc.send(GAME_V2, 1, last_char_request(-1))
    if status != 0x800000D3:
        raise ValueError("LastChar admitted before realm ticket")
    handle = raw(1, var(1, 1) + var(2, 0x576F57) + var(3, 2))
    for method, request in ((101, b""), (104, b""), (201, handle), (203, handle)):
        status, body = rpc.send(ACCOUNT_V2, method, request)
        if status: raise ValueError("V2 account request failed")
        if method in (104, 203):
            if body: raise ValueError("fixture unexpectedly restricted")
        else:
            info = next((v for n, w, v in fields(body) if n == 1 and w == 2), None)
            if info is None: raise ValueError("missing account info")
            info = {n: (w, v) for n, w, v in fields(info)}
            if info.get(1) != (0, 1): raise ValueError("incorrect account info ID")
            if method == 101 and info.get(14) not in ((0, 7), (2, b"\x07")):
                raise ValueError("incorrect account privacy flags")
            if method == 201 and info.get(2) != (2, b"WoW1"):
                raise ValueError("incorrect game account display name")
    for service, method, payload, expected in ((ACCOUNT_V2, 999, b"", 0xBC3),
            (ACCOUNT_V2, 201, b"\x0a\xff", 0xBC5),
            (GAME_V2, 99, b"", 0xBC3), (GAME_V2, 1, b"\x0a\xff", 0xBC5),
            (GAME_V2, 1, b"", 0xBC5), (GAME_V2, 2, raw(1, b"unknown"), 0xBC7)):
        status, _ = rpc.send(service, method, payload)
        if status != expected: raise ValueError("post-login negative status mismatch")
    command = b"Command_RealmListRequest_v1_classic"
    status, body = rpc.send(GAME_V2, 2, raw(1, command))
    if status or not body: raise ValueError("V2 subregion list failed")
    district_request = client_request([
        attribute("Command_SuperDistrictListRequest_v1_classic", "2-1-0", True, v2=True)])
    status, _ = rpc.send(GAME_V2, 1, district_request)
    if status != 0x800000D3: raise ValueError("district discovery admitted before account selection")
    identity = b'JSONRealmListTicketIdentity:{"gameAccountID":1}'
    info = client_information(secrets.token_bytes(32))
    malformed_info = b"JSONRealmListTicketClientInformation:" + json.dumps(
        {"info": {"secret": [0] * 32}}, separators=(",", ":")).encode()
    status, body = rpc.send(GAME_V2, 1, client_request([
        attribute("Command_RealmListTicketRequest_v1_classic", "", True, v2=True),
        attribute("Param_Identity", identity, v2=True),
        attribute("Param_ClientInfo", malformed_info, v2=True)]))
    if status != 0x80000132 or body:
        raise ValueError("missing client variant was not denied")
    status, body = rpc.send(GAME_V2, 1, join_request())
    if status != 0x800000D3 or body:
        raise ValueError("failed realm ticket changed game-account selection")
    status, body = rpc.send(GAME_V2, 1, client_request([
        attribute("Command_RealmListTicketRequest_v1_classic", "", True, v2=True),
        attribute("Param_Identity", identity, v2=True), attribute("Param_ClientInfo", info, v2=True)]))
    if status or response_attributes(body, v2=True).get("Param_RealmListTicket") != b"AuthRealmListTicket\0":
        raise ValueError("V2 realm ticket failed")
    for filter_value in (-1, 137):
        status, body = rpc.send(GAME_V2, 1, last_char_request(filter_value))
        if status or v2_attributes(body):
            raise ValueError("unexpected LastChar empty response")
    status, body = rpc.send(GAME_V2, 1, last_char_request(136))
    if status:
        raise ValueError("LastChar content-set request failed")
    discovery = last_char_metadata(body) if expect_discovery_realm else None
    if not expect_discovery_realm and v2_attributes(body):
        raise ValueError("offline LastChar unexpectedly returned a realm")
    for request, expected in (
        (client_request([
            attribute("Command_LastCharPlayedRequest_v1_classic", "70-1-70", True, v2=True),
            attribute("Param_ContentSetIDFilter", "136", True, v2=True),
        ]), 0xBC5),
        (last_char_request((1 << 63) - 1), 0xBC5),
    ):
        status, _ = rpc.send(GAME_V2, 1, request)
        if status != expected:
            raise ValueError("LastChar malformed-filter status mismatch")
    status, body = rpc.send(GAME_V2, 1, district_request)
    if status: raise ValueError("V2 district discovery failed")
    districts = super_district_metadata(response_attributes(body, v2=True).get("Param_SuperDistrictList"))
    status, body = rpc.send(GAME_V2, 1, client_request([
        attribute("Command_FetchBleepProxiesRequest_v1_classic", "", True, v2=True)]))
    if status: raise ValueError("V2 BLEEP discovery failed")
    proxies = discovery_blob(response_attributes(body, v2=True).get("Param_BleepProxyList"), b"JSONBleepProxyList:")
    if proxies != {"proxies": []}: raise ValueError("unexpected fixture proxy")
    status, body = rpc.send(GAME_V2, 1, client_request([
        attribute(command.decode(), "2-1-0", True, v2=True)]))
    if status: raise ValueError("V2 realm list failed")
    result = {"account_services": True, "negative_admission": True,
              "subregions": True, "realm_list_ticket": True,
              "super_districts": districts, "empty_bleep_proxies": True,
              "last_char": True, "realm_list_response_received": True,
              "realm_metadata_checked": not expect_discovery_realm}
    if discovery is not None:
        result["last_char_realm"] = discovery
    else:
        result["realm"] = realm_metadata(response_attributes(body, v2=True)["Param_RealmList"])
    if expect_realm_join:
        status, body = rpc.send(GAME_V2, 1, join_request())
        if status: raise ValueError("modern realm join failed")
        result["realm_join"] = join_metadata(body, discovery_blob)
    elif not expect_discovery_realm:
        status, body = rpc.send(GAME_V2, 1, join_request())
        if status != 0x800000E1 or body:
            raise ValueError("offline realm join was not denied")
        result["offline_realm_join_denied"] = True
    for address in ((1 << 32) | 0x02010001, 0x03010001, 0x02010002):
        status, body = rpc.send(GAME_V2, 1, join_request(address))
        if status != 0x80000069 or body:
            raise ValueError("invalid realm address did not fail closed")
    result["join_address_admission"] = True
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", required=True, type=Path)
    parser.add_argument("--expect-discovery-realm", action="store_true",
                        help="assert the online build-70170 LastChar routing projection")
    parser.add_argument("--expect-realm-join", action="store_true",
                        help="also request a join ticket; writes the isolated game-account session key")
    args = parser.parse_args(); runtime = args.runtime.resolve()
    if args.expect_realm_join and not args.expect_discovery_realm:
        parser.error("--expect-realm-join requires --expect-discovery-realm")
    password_path = runtime / "account-password"
    if not runtime.is_dir() or not password_path.is_file():
        parser.error("--runtime must name the isolated fixture directory")
    try:
        password = read_password(password_path)
        rejected = rest_login(runtime, password, True)
        valid = rest_login(runtime, password, False); ticket = valid.pop("ticket_value")
        result = {"rest": rejected | valid,
                  "rpc": rpc_login(runtime, ticket, args.expect_discovery_realm),
                  "rpc_v2": rpc_login_v2(runtime, password, args.expect_discovery_realm, args.expect_realm_join),
                  "build": BUILD, "version": VERSION}
        print(json.dumps(result, sort_keys=True)); return 0
    except Exception as error:
        print(f"forever_bnet_smoke: FAIL ({type(error).__name__})", file=sys.stderr); return 1


if __name__ == "__main__": raise SystemExit(main())
