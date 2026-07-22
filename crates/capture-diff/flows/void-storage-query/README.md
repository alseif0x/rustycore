# Void-storage query contents

This issue-#114 flow isolates the first `SMSG_VOID_STORAGE_CONTENTS` emitted for
an unlocked disposable character whose void store contains one item. Both live
runtimes returned the packet on the instance connection with the same 27-byte
body: one packed item GUID with counter `10000`, an empty packed creator GUID,
slot `0`, item entry `2589`, and empty random-property, bonus, and modifier
fields. There are no accepted divergences or semantic normalizations.

The capture was the wire check that exposed the previous self-consistent QA
error: the Rust codecs and bot both used fixed 16-byte GUIDs, while C++
`ByteBuffer << / >> ObjectGuid` uses `PackedGuid` for every void-storage GUID.
The versioned pair therefore exercises the corrected C++ layout rather than
merely comparing two Rust endpoints that share an encoder.

The live fixture used G'eras (`creature_template` entry `18525`, spawn `96654`)
with a temporary vault-keeper NPC flag. Its known runtime counter is `111` in
the installed C++ reference and `234` in Rust; both GUIDs use realm `1`. The
flag, seeded `character_void_storage` row, player flags, money, and position
were restored after each run, and both capture manifests record normal-runtime
restoration. The debug Rust capture used the wrapper's bounded 16 MiB worker
stack override after a first uncommitted attempt hit the known debug-login
stack boundary; the original PM2 profile was restored afterward.

The narrow flow intentionally proves only the response layout and routing. The
separate multirelogin Rust QA performs unlock, deposit, swap, withdrawal,
CharacterDB checks after each operation, and final cleanup to prove the atomic
persistence behavior implemented by this issue.

The strict import selection is:

```bash
cargo run -q -p capture-diff -- import void-storage-query \
  --cpp target/captures/void-storage-query/cpp.pkt \
  --rust target/captures/void-storage-query/rust \
  --from-opcode s2c:0x2DA1 \
  --until-opcode s2c:0x2DA1 \
  --direction s2c \
  --strict
```
