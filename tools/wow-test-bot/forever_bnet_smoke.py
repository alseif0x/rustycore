#!/usr/bin/env python3
"""Stdlib-only local WoW Forever 1.60.1 BNet REST/RPC smoke.

The wire operations follow Rust `rpc/session.rs`, `rpc/services/authentication.rs`,
`game_utilities.rs` and C++ fork a22fd98b (Server/Session.cpp,
REST/LoginRESTService.cpp); this is not a claim of retail-client parity.
The bounded result ends at an offline realm list; it does not join a realm.
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
from forever_bnet_wire import (MAX_HTTP_BODY, MAX_HTTP_HEADER, attribute, client_request,
                               fields, parse_http_header, parse_rpc_header, raw, response_attributes,
                               rpc_header, var)

HOST, REST, RPC = "127.0.0.1", 18081, 1119
EMAIL, BUILD, VERSION = "FOREVER@LOCAL.TEST", 70170, "1.60.1"
RESPONSE, CONN, AUTH, GAME, ALIST = 0xFE, 0x65446991, 0x0DECFC01, 0x3FC1274D, 0x71240E35
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

def http_json(conn, path, value):
    body = json.dumps(value, separators=(",", ":")).encode()
    request = (f"POST {path} HTTP/1.1\r\nHost: {HOST}:{REST}\r\n"
               f"Content-Type: application/json\r\nContent-Length: {len(body)}\r\n"
               "Connection: keep-alive\r\n\r\n").encode()
    conn.sock.sendall(request + body)
    status, length = parse_http_header(conn.until(b"\r\n\r\n"))
    return status, json.loads(conn.read(length))


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
    public_b = _hex(challenge.get("public_b"), (2, 512), "public B")
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
    A, B = pow(g, a, n), int(challenge["public_b"], 16)
    h = lambda value: int.from_bytes(hashlib.sha256(value).digest(), "big")
    k = h(n.to_bytes(width, "big") + g.to_bytes(width, "big"))
    u = h(A.to_bytes(width, "big") + B.to_bytes(width, "big"))
    S = pow((B - k * pow(g, x, n)) % n, a + u * x, n)
    m1 = hashlib.sha256(broken(A) + broken(B) + broken(S)).digest()
    m2 = hashlib.sha256(broken(A) + broken(int.from_bytes(m1, "big")) + broken(S)).hexdigest().upper()
    return A, m1, m2


def rest_login(runtime, password, negative):
    conn = TLS(runtime, REST)
    try:
        status, challenge = http_json(conn, "/bnetserver/login/srp/", {
            "inputs": [{"input_id": "account_name", "value": EMAIL}]})
        if status != 200: raise ValueError("SRP challenge HTTP failure")
        validate_srp_challenge(challenge)
        A, m1, expected_m2 = srp_proof(challenge, password)
        if negative:
            bad = bytearray(m1); bad[-1] ^= 1; m1 = bytes(bad)
        status, result = http_json(conn, "/bnetserver/login/", {"inputs": [
            {"input_id": "public_A", "value": A.to_bytes(256, "big").hex().upper()},
            {"input_id": "client_evidence_M1", "value": m1.hex().upper()}]})
        if negative:
            validate_rejected_result(status, result)
            return {"challenge": True, "wrong_m1_rejected": True}
        if status != 200: raise ValueError("SRP proof HTTP failure")
        ticket = result.get("login_ticket")
        if not isinstance(ticket, str) or not ticket: raise ValueError("no login ticket")
        if result.get("server_evidence_M2", "").upper() != expected_m2:
            raise ValueError("server M2 did not verify")
        return {"challenge": True, "valid_m2": True, "ticket": True, "ticket_value": ticket}
    finally: conn.close()


def validate_rejected_result(status, result):
    allowed = {"authentication_state", "error_code", "error_message", "url", "login_ticket", "server_evidence_M2"}
    if status != 200 or not isinstance(result, dict) or set(result) - allowed:
        raise ValueError("unexpected SRP rejection response")
    if result.get("authentication_state") != "DONE": raise ValueError("SRP rejection was not DONE")
    if any(result.get(key) is not None for key in ("error_code", "error_message", "url", "login_ticket", "server_evidence_M2")):
        raise ValueError("SRP rejection contained a result")

class RPCSession:
    def __init__(self, runtime):
        self.tls, self.token, self.logon_error = TLS(runtime, RPC), 1, None

    def close(self): self.tls.close()

    def frame(self):
        size = struct.unpack(">H", self.tls.read(2))[0]
        if not size: raise ValueError("empty RPC header")
        header = parse_rpc_header(self.tls.read(size))
        return header, self.tls.read(header.get(5, 0))

    def respond(self, token):
        header = rpc_header(RESPONSE, 0, token)
        self.tls.sock.sendall(struct.pack(">H", len(header)) + header)

    def send(self, service_hash, method, payload):
        token = self.token; self.token += 1
        header = rpc_header(0, method, token, len(payload), service_hash)
        self.tls.sock.sendall(struct.pack(">H", len(header)) + header + payload)
        while True:
            header, body = self.frame()
            if header.get(1) == RESPONSE and header.get(3) == token:
                return header.get(6, 0), body
            if header.get(11) == ALIST and header.get(2) == 5:
                self.logon_error = next((v for n, w, v in fields(body) if n == 1 and w == 0), None)
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


def rpc_login(runtime, ticket):
    rpc = RPCSession(runtime)
    try:
        status, _ = rpc.send(CONN, 1, var(3, 1))
        if status: raise ValueError("Connect failed")
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
        realm = realm_metadata(response_attributes(body)["Param_RealmList"])
        return {"connect": True, "on_logon_complete": True, "realm_list_ticket": True,
                "realm_list": True, "realm": realm}
    finally: rpc.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", required=True, type=Path)
    args = parser.parse_args(); runtime = args.runtime.resolve()
    password_path = runtime / "account-password"
    if not runtime.is_dir() or not password_path.is_file():
        parser.error("--runtime must name the isolated fixture directory")
    try:
        password = read_password(password_path)
        rejected = rest_login(runtime, password, True)
        valid = rest_login(runtime, password, False); ticket = valid.pop("ticket_value")
        result = {"rest": rejected | valid, "rpc": rpc_login(runtime, ticket),
                  "build": BUILD, "version": VERSION}
        print(json.dumps(result, sort_keys=True)); return 0
    except Exception as error:
        print(f"forever_bnet_smoke: FAIL ({type(error).__name__})", file=sys.stderr); return 1


if __name__ == "__main__": raise SystemExit(main())
