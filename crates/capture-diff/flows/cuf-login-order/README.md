# CUF login order

This issue-#7 flow records the same character, with one non-empty CUF profile,
logging in to the installed legacy C++ server and the issue-#7 Rust build. It
isolates the four-packet post-add window required by C++
`Player::SendInitialPacketsAfterAddToMap`:

1. `SMSG_INIT_WORLD_STATES` (`0x2746`)
2. `SMSG_LOAD_CUF_PROFILES` (`0x25BC`)
3. `SMSG_AURA_UPDATE` (`0x2C1F`)
4. the final `SMSG_PHASE_SHIFT_CHANGE` (`0x2578`) emitted by
   `PhasingHandler::OnMapChange`

Both captures contain that exact order on connection 1. The non-empty
`SMSG_LOAD_CUF_PROFILES` body and final phase-shift body are byte-identical
between C++ and Rust. The pinned baseline retains two unrelated value
divergences: map/zone world-state values in `SMSG_INIT_WORLD_STATES`, and
runtime aura identifiers in `SMSG_AURA_UPDATE`. Neither changes packet
cardinality, routing, or the ordering fixed by issue #7.

The source anchors are
`src/server/game/Entities/Player/Player.cpp:23600-23672`,
`src/server/game/Server/Packets/MiscPackets.cpp:574-605`, and
`src/server/game/Handlers/MiscHandler.cpp:1253-1297` in the installed
TrinityCore reference.

The paired headless bot runs completed world authentication, character
enumeration, login verification, and the stand-state round trip. The temporary
CUF row used to make the packet non-empty was deleted after capture; both RAW
manifests attest that their capture runtimes were stopped and the normal Rust
runtime was restored. This is live wire evidence, not a claim of manual-client
UI validation or of full login-burst parity.

The reviewed import selection is:

```bash
cargo run -q -p capture-diff -- import cuf-login-order \
  --cpp target/captures/cuf-login-order/cpp.pkt \
  --rust target/captures/cuf-login-order/rust \
  --from-opcode s2c:0x2746 \
  --until-opcode s2c:0x2578 \
  --direction s2c
```
