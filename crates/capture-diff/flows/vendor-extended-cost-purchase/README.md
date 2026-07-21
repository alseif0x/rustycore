# `vendor-extended-cost-purchase`

This issue-#108 fixture is the exact server-response window after the durable
commit of one real extended-cost item purchase. It contains, in order and on
the realm socket:

1. `SMSG_BUY_SUCCEEDED` (`0x26C6`)
2. `SMSG_ITEM_PUSH_RESULT` (`0x2623`)

It was imported strictly from paired C++ and Rust captures made at Rust
`3c472dd3b164d8973f9481b2eede0d38d1c4fc3b`. The retained schema-v3 manifests
pin the raw capture, executable, source, process/listener, configuration and
cleanup identities; `capture-lineage.json` pins the derived two-packet
artifacts. Each manifest also pins its exact bot executable and validated JSON
report; import retains those reports as `capture-provenance/cpp.bot-report.json`
and `capture-provenance/rust.bot-report.json`, so the persistence/restoration
claims remain independently recheckable. The accepted-divergence baseline is
empty.

The bot used the unique G'eras fixture (Creature entry `18525`, SQL spawn
`96654`, realm `1`, map `530`) and vendor row item `30183`, extended cost
`1642`. On both servers it proved currency `42` changed from `30` to `15`, one
item was present after logout and fresh authentication, the expected
`VendorInventory`, `SetCurrency`, `BuySucceeded` and `ItemPushResult` packets
arrived on their C++ sockets, and cleanup restored the original character
state.

## Narrow semantic normalization

C++ and Rust allocate different nonzero lower 40-bit map-runtime counters for
G'eras. The `SMSG_BUY_SUCCEEDED` comparator omits only that counter and only
for the exact stable identity Creature/realm `1`/map `530`/entry `18525`/
subtype `0`/server `0`. It still requires a nonzero counter, MUID `59`,
`NewQuantity = -1`, `QuantityBought = 1`, canonical packet decoding,
S2C direction and realm routing. Every field of `SMSG_ITEM_PUSH_RESULT`
remains byte-exact.

## Scope boundary

The complete raw bot action ran from `CMSG_BUY_ITEM` through the fixed
`CMSG_PING` fence. Its purchase-owned responses and final inventory state
matched after the reviewed runtime-GUID normalization. C++ also emitted
`SMSG_CRITERIA_UPDATE`; Rust does not yet implement the corresponding
achievement subsystem. This particular raw generation also contains ambient
C++ `SMSG_ON_MONSTER_MOVE` packets unrelated to the purchase. Neither
the achievement gap nor ambient movement is ignored or accepted in this
fixture. Consequently this committed flow claims only the clean post-COMMIT
realm-response window, while the bot proves the complete currency, inventory,
persistence and restoration behavior.

Recheck the committed evidence with:

```bash
cargo +1.88.0 run -q -p capture-diff -- \
  diff vendor-extended-cost-purchase --strict
```
