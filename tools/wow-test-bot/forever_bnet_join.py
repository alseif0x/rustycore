"""Bounded build-70170 realm-join assertions; never prints tickets or secrets.

Source: advocaite/TrinityCore 02245dcd, RealmList::JoinRealm and
RealmList.proto::RealmJoinTicket. This checks BNet, not a World AuthSession.
"""
import json

from forever_bnet_wire import attribute, client_request, raw, var, v2_attributes


VARIANT = {"platformType": int.from_bytes(b"Win", "big"),
           "clientArch": int.from_bytes(b"x64", "big"),
           "type": int.from_bytes(b"WoWB", "big")}
REALM_ADDRESS = 0x02010001


def client_information(secret):
    if not isinstance(secret, bytes) or len(secret) != 32:
        raise ValueError("invalid client secret size")
    return b"JSONRealmListTicketClientInformation:" + json.dumps(
        {"info": VARIANT | {"secret": list(secret)}}, separators=(",", ":")).encode()


def join_request(address=REALM_ADDRESS):
    if type(address) is not int or not 0 <= address < 1 << 64:
        raise ValueError("invalid realm address")
    address_attribute = raw(1, b"Param_RealmAddress") + raw(2, var(6, address))
    return client_request([
        attribute("Command_RealmJoinRequest_v1_classic", "", True, v2=True),
        address_attribute,
    ])


def join_metadata(payload, decode_blob):
    attributes = v2_attributes(payload)
    if [(name, kind) for name, kind, _ in attributes] != [
            ("Param_RealmJoinTicket", "blob"), ("Param_ServerAddresses", "blob"),
            ("Param_JoinSecret", "blob")]:
        raise ValueError("invalid realm join attributes")
    ticket, addresses, secret = [value for _, _, value in attributes]
    if not 1 <= len(ticket) <= 1024 or len(secret) != 32:
        raise ValueError("invalid realm join lengths")
    try:
        parsed = json.loads(ticket)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("invalid realm join ticket JSON") from error
    expected = {"gameAccount": "1#1", "platform": VARIANT["platformType"],
                "clientArch": VARIANT["clientArch"], "type": VARIANT["type"]}
    if (parsed != expected or not isinstance(parsed, dict) or
            any(type(parsed[key]) is not int for key in ("platform", "clientArch", "type"))):
        raise ValueError("realm join ticket variant mismatch")
    endpoints = decode_blob(addresses, b"JSONRealmListServerIPAddresses:")
    if endpoints != {"families": [{"family": 1, "addresses": [
            {"ip": "127.0.0.1", "port": 18085}]}]}:
        raise ValueError("realm join endpoint is not the isolated fixture")
    family = endpoints["families"][0]
    if type(family["family"]) is not int or type(family["addresses"][0]["port"]) is not int:
        raise ValueError("invalid realm join endpoint types")
    return {"ticket_json": True, "build_variant": True, "server_secret_length": 32,
            "endpoint_loopback": True, "world_port": 18085,
            "world_authentication_tested": False}
