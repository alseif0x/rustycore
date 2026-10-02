"""Hermetic realm-join contract checks, including opaque-value rejection."""
import json
import unittest

import forever_bnet_join as join
import forever_bnet_smoke as smoke
import forever_bnet_wire as wire
from test_forever_bnet_smoke import json_blob


def payload(ticket=None, secret=b"s" * 32, endpoint="127.0.0.1", port=18085):
    if ticket is None:
        ticket = json.dumps({"gameAccount": "1#1", "platform": join.VARIANT["platformType"],
                             "clientArch": join.VARIANT["clientArch"],
                             "type": join.VARIANT["type"]}).encode()
    return wire.client_request([
        wire.attribute("Param_RealmJoinTicket", ticket, v2=True),
        wire.attribute("Param_ServerAddresses", json_blob(b"JSONRealmListServerIPAddresses:",
            {"families": [{"family": 1, "addresses": [{"ip": endpoint, "port": port}]}]}), v2=True),
        wire.attribute("Param_JoinSecret", secret, v2=True),
    ])


class JoinTests(unittest.TestCase):
    def test_client_information_preserves_variant_and_secret(self):
        prefix, value = join.client_information(bytes(range(32))).split(b":", 1)
        self.assertEqual(prefix, b"JSONRealmListTicketClientInformation")
        self.assertEqual(json.loads(value), {"info": join.VARIANT | {"secret": list(range(32))}})
        for secret in (b"", b"x" * 31, b"x" * 33, "x" * 32):
            with self.assertRaises(ValueError): join.client_information(secret)

    def test_uint_realm_address_wire_preserves_overflow_for_server_rejection(self):
        for value in (join.REALM_ADDRESS, 1 << 32, (1 << 64) - 1):
            attrs = wire.v2_attributes(join.join_request(value))
            self.assertEqual(attrs[1], ("Param_RealmAddress", "uint", value))
        for value in (-1, 1 << 64, True, "1"):
            with self.assertRaises(ValueError): join.join_request(value)

    def test_valid_modern_join_is_not_world_authentication(self):
        result = join.join_metadata(payload(), smoke.discovery_blob)
        self.assertTrue(result["ticket_json"])
        self.assertFalse(result["world_authentication_tested"])
        self.assertNotIn("s" * 32, json.dumps(result))

    def test_invalid_ticket_secret_and_nonfixture_endpoint_fail(self):
        invalid = [payload(ticket=b"1#1"), payload(ticket=b"{}"), payload(ticket=b"\xff"),
                   payload(ticket=b"x" * 1025), payload(secret=b""), payload(secret=b"s" * 31),
                   payload(secret=b"s" * 33), payload(endpoint="192.0.2.1"), payload(port=8085)]
        for value in invalid:
            with self.assertRaises(ValueError): join.join_metadata(value, smoke.discovery_blob)

    def test_attributes_must_be_exact_order_and_types(self):
        attrs = list(wire.fields(payload()))
        for attributes in (attrs[::-1], attrs[:-1], attrs + [attrs[0]]):
            value = b"".join(wire.raw(n, v) for n, _, v in attributes)
            with self.assertRaises(ValueError): join.join_metadata(value, smoke.discovery_blob)


if __name__ == "__main__": unittest.main()
