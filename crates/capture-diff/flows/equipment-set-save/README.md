# Equipment-set save ACK

This issue-#112 flow isolates the first `SMSG_EQUIPMENT_SET_ID` produced by two
simultaneous new-set saves. Both C++ and Rust emitted the packet on the instance
connection with the exact body `guid=1`, `type=1`, `set_id=8`. The committed
window is therefore one byte-identical packet with no accepted divergences and
no semantic normalization.

The full Rust bot workflow is intentionally broader than this wire fixture. It
saves one ordinary equipment set and one transmog outfit concurrently, requires
distinct nonzero GUIDs above the combined pre-run table maximum, logs both
characters out, authenticates both again from fresh World session keys, verifies
one exact `SMSG_LOAD_EQUIPMENT_SET` entry per character plus the durable rows,
then removes only the two fixture rows and proves both tables empty.

The C++ capture run reached both new-set ACKs but its installed reference runtime
did not satisfy this bot's post-logout row verifier. Consequently the committed
capture claims only the C++-anchored ACK action; the persistence/relog proof is
the separate Rust runtime QA plus the audited C++ `Player::_SaveEquipmentSets`
and `_LoadEquipmentSets`/`_LoadTransmogOutfits` source paths.

The strict import selection is:

```bash
cargo run -q -p capture-diff -- import equipment-set-save \
  --cpp target/captures/equipment-set-save/cpp.pkt \
  --rust target/captures/equipment-set-save/rust \
  --from-opcode s2c:0x26B2 \
  --until-opcode s2c:0x26B2 \
  --direction s2c \
  --strict
```
