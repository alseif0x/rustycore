"""Private bounded wire helpers for the Forever BNet smoke."""
import re
import struct

MAX_PROTO, MAX_HTTP_HEADER, MAX_HTTP_BODY = 1 << 20, 16 << 10, 4 << 20


def vi(value):
    out = bytearray()
    while value > 127:
        out.append((value & 127) | 128); value >>= 7
    out.append(value); return bytes(out)


def f(number, wire, value): return vi(number << 3 | wire) + value


def var(number, value): return f(number, 0, vi(value))


def int64(number, value):
    """Encode a signed protobuf int64 (negative values use two's complement)."""
    if type(value) is not int or not -(1 << 63) <= value < (1 << 63):
        raise ValueError("protobuf int64 is out of range")
    return f(number, 0, vi(value & ((1 << 64) - 1)))


def raw(number, value): return f(number, 2, vi(len(value)) + value)


def fixed(number, value): return f(number, 5, struct.pack("<I", value))


def _varint(data, pos):
    value = 0
    for shift in range(0, 70, 7):
        if pos >= len(data): raise ValueError("truncated protobuf varint")
        byte = data[pos]; pos += 1
        if shift == 63 and byte > 1: raise ValueError("protobuf varint overflow")
        value |= (byte & 127) << shift
        if byte < 128: return value, pos
    raise ValueError("protobuf varint overflow")


def fields(data):
    """Decode bounded varint, bytes, and fixed32 protobuf fields."""
    if len(data) > MAX_PROTO: raise ValueError("protobuf input is too large")
    pos = count = 0
    while pos < len(data):
        count += 1
        if count > 4096: raise ValueError("too many protobuf fields")
        key, pos = _varint(data, pos); number, wire = key >> 3, key & 7
        if number == 0: raise ValueError("protobuf field number is zero")
        if wire == 0:
            value, pos = _varint(data, pos); yield number, wire, value
        elif wire == 2:
            size, pos = _varint(data, pos)
            if size > MAX_PROTO or pos + size > len(data):
                raise ValueError("truncated protobuf bytes field")
            yield number, wire, data[pos:pos + size]; pos += size
        elif wire == 5:
            if pos + 4 > len(data): raise ValueError("truncated protobuf fixed32 field")
            yield number, wire, data[pos:pos + 4]; pos += 4
        elif wire == 1:
            if pos + 8 > len(data): raise ValueError("truncated protobuf fixed64 field")
            yield number, wire, data[pos:pos + 8]; pos += 8
        else:
            raise ValueError("unsupported protobuf wire type")


def parse_http_response_header(data):
    """Parse a bounded HTTP response header and retain case-insensitive headers."""
    if len(data) > MAX_HTTP_HEADER: raise ValueError("HTTP header is too large")
    if not data.endswith(b"\r\n\r\n"):
        raise ValueError("incomplete HTTP header")
    try:
        lines = data[:-4].decode("iso-8859-1").split("\r\n")
    except UnicodeDecodeError as error:
        raise ValueError("invalid HTTP header") from error
    if not lines or not lines[0]: raise ValueError("missing HTTP status line")
    parts = lines[0].split()
    if len(parts) < 2 or not parts[0].startswith("HTTP/"):
        raise ValueError("malformed HTTP status")
    try: status = int(parts[1])
    except ValueError as error: raise ValueError("malformed HTTP status") from error
    if status < 100 or status > 599: raise ValueError("invalid HTTP status")
    headers = {}
    for line in lines[1:]:
        if not line or ":" not in line:
            raise ValueError("malformed HTTP header field")
        name, value = line.split(":", 1)
        name, value = name.strip().lower(), value.strip()
        if not name:
            raise ValueError("malformed HTTP header field")
        if name == "content-length" and name in headers and headers[name] != value:
            raise ValueError("conflicting HTTP content length")
        headers[name] = value
    content_length = headers.get("content-length")
    if content_length is None or not content_length.isdigit():
        raise ValueError("invalid HTTP content length")
    length = int(content_length)
    if length > MAX_HTTP_BODY: raise ValueError("HTTP body is too large")
    return status, length, headers


def parse_http_header(data):
    """Compatibility wrapper returning only status and content length."""
    status, length, _ = parse_http_response_header(data)
    return status, length


def extract_session_cookie(set_cookie):
    """Validate the C++ login session cookie and return a Cookie header value."""
    if not isinstance(set_cookie, str):
        raise ValueError("missing session cookie")
    parts = [part.strip() for part in set_cookie.split(";")]
    if not parts or "=" not in parts[0]:
        raise ValueError("invalid session cookie")
    name, value = parts[0].split("=", 1)
    if name != "JSESSIONID" or not re.fullmatch(
            r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}", value):
        raise ValueError("invalid session cookie")
    attributes = {}
    for part in parts[1:]:
        if not part:
            continue
        key, separator, attr_value = part.partition("=")
        attributes[key.strip().lower()] = attr_value.strip().lower() if separator else None
    if (attributes.get("path") != "/bnetserver" or attributes.get("samesite") != "none" or
            "secure" not in attributes or "httponly" not in attributes):
        raise ValueError("invalid session cookie attributes")
    return f"JSESSIONID={value}"


def http_request(path, body, host, cookie=None, close=True):
    """Build a bounded HTTP/1.1 request without exposing body/cookie values."""
    if not isinstance(body, bytes) or len(body) > MAX_HTTP_BODY:
        raise ValueError("HTTP request body is too large")
    if any(token in path or token in host for token in ("\r", "\n")):
        raise ValueError("invalid HTTP request target")
    if cookie is not None and any(token in cookie for token in ("\r", "\n")):
        raise ValueError("invalid HTTP cookie")
    connection = "close" if close else "keep-alive"
    headers = [f"POST {path} HTTP/1.1", f"Host: {host}",
               "Content-Type: application/json", f"Content-Length: {len(body)}",
               f"Connection: {connection}"]
    if cookie is not None:
        headers.append(f"Cookie: {cookie}")
    return ("\r\n".join(headers) + "\r\n\r\n").encode() + body


def rpc_header(service, method, token, size=0, service_hash=None, ciid=None):
    value = var(1, service) + var(2, method) + var(3, token) + var(5, size)
    value += fixed(11, service_hash) if service_hash is not None else b""
    return value + (raw(13, ciid) if ciid is not None else b"")


def parse_rpc_header(data):
    if not data or len(data) > MAX_PROTO: raise ValueError("invalid RPC header length")
    header = {}
    for number, wire, value in fields(data):
        if number in (1, 2, 3, 5, 6) and wire != 0: raise ValueError("RPC varint field has wrong wire type")
        if number == 11 and wire != 5: raise ValueError("RPC service hash has wrong wire type")
        if number == 13 and wire != 2: raise ValueError("RPC CIID has wrong wire type")
        header[number] = value if wire != 5 else int.from_bytes(value, "little")
    if 1 not in header or 3 not in header: raise ValueError("RPC header is missing required fields")
    if header.get(5, 0) > MAX_PROTO: raise ValueError("RPC payload is too large")
    return header


def connect_identity(payload, header):
    """Validate modern TC Connect's ProcessIds, CIID and bindless default.

    ConnectionService::HandleConnect / Session::SendResponse, master 6ebe044c.
    This smoke omits client_id and use_bindless_rpc in its Connect request.
    """
    parts = {n: (w, v) for n, w, v in fields(payload)}
    ids = []
    for number in (1, 2):
        if number not in parts or parts[number][0] != 2:
            raise ValueError("Connect is missing a ProcessId")
        process = {n: (w, v) for n, w, v in fields(parts[number][1])}
        for field in (1, 2):
            if field not in process or process[field][0] != 0 or not 0 < process[field][1] <= 0xFFFFFFFF:
                raise ValueError("invalid Connect ProcessId")
            ids.append(process[field][1])
    expected = ("%08X%08X-%08X%08X" % tuple(ids)).encode("ascii")
    if parts.get(9) != (2, expected) or header.get(13) != expected:
        raise ValueError("Connect CIID mismatch")
    if parts.get(7) != (0, 1): raise ValueError("Connect bindless default mismatch")
    return expected


def v2_logon_record(payload):
    parts = {n: (w, v) for n, w, v in fields(payload)}
    if parts.get(1) != (0, 0) or parts.get(2, (None,))[0] != 2:
        raise ValueError("invalid V2 logon completion")
    record = list(fields(parts[2][1]))
    if next((v for n, w, v in record if n == 1 and w == 0), None) != 1:
        raise ValueError("unexpected V2 account")
    games = [{n: (w, v) for n, w, v in fields(value)}
             for number, wire, value in record if number == 2 and wire == 2]
    if games != [{1: (0, 1), 2: (0, 0x576F57), 3: (0, 2)}]:
        raise ValueError("unexpected V2 game-account handles")
    key = next((v for n, w, v in record if n == 5 and w == 2), b"")
    if len(key) != 64: raise ValueError("invalid V2 session key size")
    return {"account_id": 1, "game_account_id": 1, "region": 2, "session_key_length": 64}


def decode_frame(data):
    if len(data) < 2: raise ValueError("truncated RPC frame length")
    size = struct.unpack(">H", data[:2])[0]
    if not size or len(data) < size + 2: raise ValueError("truncated RPC frame header")
    header = parse_rpc_header(data[2:2 + size]); end = 2 + size + header.get(5, 0)
    if len(data) != end: raise ValueError("RPC frame payload length mismatch")
    return header, data[2 + size:end]


def attribute(name, value, string=False, v2=False):
    value = value.encode() if isinstance(value, str) else value
    field = (4 if string else 5) if v2 else (5 if string else 6)
    return raw(1, name.encode()) + raw(2, raw(field, value))


def v2_int_attribute(name, value):
    """Build a V2 Attribute whose Variant is the signed int64 field."""
    if not isinstance(name, str) or not name:
        raise ValueError("V2 attribute name is invalid")
    return raw(1, name.encode()) + raw(2, int64(2, value))


def client_request(attributes):
    return b"".join(raw(1, item) for item in attributes)


def response_attributes(payload, v2=False):
    result = {}
    for number, wire, item in fields(payload):
        if number != 1 or wire != 2: continue
        parts = {n: v for n, w, v in fields(item) if w == 2}
        name = parts.get(1, b"").decode()
        values = {n: v for n, w, v in fields(parts.get(2, b"")) if w == 2}
        result[name] = values.get(5, values.get(4, b"")) if v2 else values.get(6, values.get(5, b""))
    return result


def v2_attributes(payload):
    """Decode V2 response attributes as (name, kind, value) tuples.

    This intentionally preserves the signed int64 distinction needed by the
    build-70170 LastChar response instead of coercing every value to bytes.
    """
    result = []
    unknown_outer = False
    for number, wire, item in fields(payload):
        if number != 1 or wire != 2:
            unknown_outer = True
            continue
        parts = list(fields(item))
        names = [(w, value) for n, w, value in parts if n == 1]
        variants = [(w, value) for n, w, value in parts if n == 2]
        if len(names) != 1 or names[0][0] != 2 or len(variants) != 1 or variants[0][0] != 2:
            raise ValueError("invalid V2 attribute envelope")
        try:
            name = names[0][1].decode("utf-8")
        except UnicodeDecodeError as error:
            raise ValueError("invalid V2 attribute name") from error
        variant = list(fields(variants[0][1]))
        if len(variant) != 1:
            raise ValueError("invalid V2 attribute variant")
        field, variant_wire, value = variant[0]
        if field == 1 and variant_wire == 0:
            kind, value = "bool", bool(value)
        elif field == 2 and variant_wire == 0:
            kind = "int"
            value = value - (1 << 64) if value >= 1 << 63 else value
        elif field == 3 and variant_wire == 1:
            kind, value = "float", struct.unpack("<d", value)[0]
        elif field == 4 and variant_wire == 2:
            kind = "string"
        elif field == 5 and variant_wire == 2:
            kind = "blob"
        elif field == 6 and variant_wire == 0:
            kind = "uint"
        else:
            raise ValueError("unsupported V2 attribute variant")
        result.append((name, kind, value))
    if unknown_outer and not result:
        raise ValueError("V2 response contains only unknown fields")
    return result
