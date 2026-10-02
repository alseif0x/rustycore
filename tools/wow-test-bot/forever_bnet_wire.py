"""Private bounded wire helpers for the Forever BNet smoke."""
import struct

MAX_PROTO, MAX_HTTP_HEADER, MAX_HTTP_BODY = 1 << 20, 16 << 10, 4 << 20


def vi(value):
    out = bytearray()
    while value > 127:
        out.append((value & 127) | 128); value >>= 7
    out.append(value); return bytes(out)


def f(number, wire, value): return vi(number << 3 | wire) + value


def var(number, value): return f(number, 0, vi(value))


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


def parse_http_header(data):
    if len(data) > MAX_HTTP_HEADER: raise ValueError("HTTP header is too large")
    lines = data.decode("iso-8859-1").splitlines()
    if not lines: raise ValueError("missing HTTP status line")
    parts = lines[0].split()
    if len(parts) < 2 or not parts[0].startswith("HTTP/"):
        raise ValueError("malformed HTTP status")
    try: status = int(parts[1])
    except ValueError as error: raise ValueError("malformed HTTP status") from error
    if status < 100 or status > 599: raise ValueError("invalid HTTP status")
    headers = {}
    for line in lines[1:]:
        if ":" in line:
            name, value = line.split(":", 1); headers[name.strip().lower()] = value.strip()
    try: length = int(headers["content-length"])
    except (KeyError, ValueError) as error: raise ValueError("invalid HTTP content length") from error
    if length < 0 or length > MAX_HTTP_BODY: raise ValueError("HTTP body is too large")
    return status, length


def rpc_header(service, method, token, size=0, service_hash=None):
    value = var(1, service) + var(2, method) + var(3, token) + var(5, size)
    return value + (fixed(11, service_hash) if service_hash is not None else b"")


def parse_rpc_header(data):
    if not data or len(data) > MAX_PROTO: raise ValueError("invalid RPC header length")
    header = {}
    for number, wire, value in fields(data):
        if number in (1, 2, 3, 5, 6) and wire != 0: raise ValueError("RPC varint field has wrong wire type")
        if number == 11 and wire != 5: raise ValueError("RPC service hash has wrong wire type")
        header[number] = value if wire != 5 else int.from_bytes(value, "little")
    if 1 not in header or 3 not in header: raise ValueError("RPC header is missing required fields")
    if header.get(5, 0) > MAX_PROTO: raise ValueError("RPC payload is too large")
    return header


def decode_frame(data):
    if len(data) < 2: raise ValueError("truncated RPC frame length")
    size = struct.unpack(">H", data[:2])[0]
    if not size or len(data) < size + 2: raise ValueError("truncated RPC frame header")
    header = parse_rpc_header(data[2:2 + size]); end = 2 + size + header.get(5, 0)
    if len(data) != end: raise ValueError("RPC frame payload length mismatch")
    return header, data[2 + size:end]


def attribute(name, value, string=False):
    value = value.encode() if isinstance(value, str) else value
    return raw(1, name.encode()) + raw(2, raw(5 if string else 6, value))


def client_request(attributes):
    return b"".join(raw(1, item) for item in attributes)


def response_attributes(payload):
    result = {}
    for number, wire, item in fields(payload):
        if number != 1 or wire != 2: continue
        parts = {n: v for n, w, v in fields(item) if w == 2}
        name = parts.get(1, b"").decode()
        values = {n: v for n, w, v in fields(parts.get(2, b"")) if w == 2}
        result[name] = values.get(6, values.get(5, b""))
    return result
