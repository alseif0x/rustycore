# InspectResult reader evidence for client 3.4.3.54261

**Reviewed 2026-11-20.** This note records the packet layout consumed by the
3.4.3.54261 client and the narrow client-supported divergence from the pinned
3.4.3 server writer. It is static executable evidence, not a live packet capture.

## Client and dump provenance

- Build: `WOW-54261patch3.4.3_ClassicRetail` (3.4.3.54261).
- `WowClassic.exe`: 50,589,832 bytes; SHA-256
  `e240f9d88445643d2545ac54b3c3874b2f86336bc8947256584ef07bb5fd66a7`; MD5
  `6d3e38eba91bfd1798a886baa92745d9`.
- Build configuration MD5: `c91609c69ed2ab39d44039390a1be969`.
- Source metadata: [NGDP version record](https://github.com/mdX7/ngdp_data/blob/d8bcfafa67e7408c95d33caa3733729aff17c6fe/US/wow_classic/54261/versions).
- PE preferred image base: `0x140000000`. `.text` is RVA `0x1000`, size
  `39,892,480` bytes. The offline decrypted `.text` dump has SHA-256
  `fcd5f3c695d3f6b5812d0535cd704fa7ee1893e3630ab5951a7cd80c6ad6058b`.

The x86_64 client ran under Wine 10 only to initialize and decrypt its private
image; the reader bytes were copied for offline inspection and the process was
terminated. No game-account credentials were supplied, no client/server session
was tested, and no game-server or database operation was performed. The
executable and memory dump are not included in this repository.

## Observed reader layout

The addresses below are preferred virtual addresses in the verified image.

1. The `InspectResult` reader at `0x1406ff210` calls the model reader at
   `0x140718160` (`0x1406ff23d`). It then reads a `u32` glyph count and resizes
   the glyph list (`0x1406ff250–0x1406ff25f`). The next `u32`, read at
   `0x1406ff26f`, is stored as item level at object offset `0xe0`
   (`0x1406ff27e`). There is no flat talent count between these fields.
2. The following scalar reads are `u8`, `u16`, `u16`, `u32`, `u32`; the reader
   then consumes the declared glyph count as `u16` IDs (`0x1406ff288–0x1406ff338`).
   It calls `TalentInfoUpdate` at `0x140709e20` (`0x1406ff346`). A separate byte
   follows at `0x1406ff356`; its `0x80` guild and `0x40` Azerite bits are handled
   after the talent reader returns.
3. `TalentInfoUpdate` reads a `u32`, `u8`, `u32` header at
   `0x140709e46`, `0x140709e5b`, and `0x140709e72`. Its `u32` group-array
   length drives the group loop. Each group reads `u8`, `u32`, `u8`, `u32`,
   `u8` fields for talent count/array length, glyph count/array length, and
   specialization. Talent records are `u32` plus `u8`; glyph IDs are `u16`.
   At `0x140709ffa`, the reader consumes a final `u8`; it shifts the high bit
   at `0x14070a003` and stores that as `IsPetTalents` at `0x14070a006`. The
   reader returns at `0x14070a013`. The internal block at `0x140709e9b` is
   not the end; the function has split `.pdata` ranges. Thus the caller's byte
   at `0x1406ff356` is a separate guild/Azerite flag byte; the talent reader
   must consume its final byte before the caller reads those flags.
4. The shared primitive readers confirm the widths and cursor advances:
   `0x1402c9990` reads one byte and advances by one, `0x1402c99e0` reads two
   bytes and advances by two, and `0x1402c9a30` reads four bytes and advances
   by four.

The analysis used x86_64 Wine 10 and disassembled the copied text bytes with
the preferred image base plus `.text` RVA (`0x140001000`), for example:

```sh
objdump -D -b binary -m i386:x86-64 --adjust-vma=0x140001000 \
  --start-address=<start> --stop-address=<stop> -M intel text.decrypted.bin
```

## Comparison and evidence boundary

The pinned server source is TrinityCore-derived commit
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`,
`src/server/game/Server/Packets/InspectPackets.cpp:185–218`. Its flat talent
count at the model boundary conflicts with the 54261 reader, which consumes
that word as item level and then reads the nested `TalentInfoUpdate` structure.
The client evidence supports only the two corresponding Rust packet-layout
changes: omit that flat count and serialize the nested talent header/groups as
read above. It does not establish broader inspect behavior or parity for other
client builds.

The current writer and Rust regression tests provide static implementation
coverage. This evidence does **not** prove live `SMSG_INSPECT_RESULT` or inspect
UI acceptance; no client/server capture was made.
