# Local Forever client data probe

Operator-only Linux acquisition of `ChrClasses.db2`, `ChrRaces.db2`, and
`Achievement.db2` from
installed `wow_classic_beta` build 70170. It uses the reference's pinned CascLib,
requires a new output directory inside the ignored `target/forever-login`,
does not import missing encryption keys by default or substitute zero-filled data, and
cancels download operations. It never logs in or modifies the installation.

```bash
cmake -S tools/wow-test-bot/client-data-probe \
  -B target/forever-login/client-data-probe-build \
  -DRUSTYCORE_FOREVER_CPP_REF=/absolute/path/to/pinned-reference
cmake --build target/forever-login/client-data-probe-build -j1
ctest --test-dir target/forever-login/client-data-probe-build --output-on-failure
FOREVER_CLIENT_DATA_PROBE_BIN="$PWD/target/forever-login/client-data-probe-build/forever-client-data-probe" \
  python3 tools/wow-test-bot/client-data-probe/test_cli.py
target/forever-login/client-data-probe-build/forever-client-data-probe \
  --ack-local-client-data /absolute/path/to/World-of-Warcraft \
  /absolute/path/to/rustycore/target/forever-login/new-client-data esES
```

If an operator has a privately stored, source-provided TACT key list, the
normal CascLib import path is opt-in and remains offline:

```bash
target/forever-login/client-data-probe-build/forever-client-data-probe \
  --ack-local-client-data /absolute/path/to/World-of-Warcraft \
  /absolute/path/to/rustycore/target/forever-login/new-client-data esES \
  --ack-public-tact-keys /absolute/path/to/rustycore/target/forever-login/tact-keys.txt
```

The key-list file must already exist as a regular non-symlink file under the
ignored Forever fixture root, have mode `0600`, and be smaller than 10 MiB.
The probe calls CascLib's `CascImportKeysFromFile` only after confirming the
local product is build 70170; it never downloads, prints, or stores key
contents. The reference's public provenance is the `wowdev/TACTKeys` repository
(`WoW.txt`); the format and registration path follow the pinned reference's
`src/tools/extractor_common/CascHandles.cpp` `LoadOnlineTactKeys` function
(`02245dcd`). The public TACT metadata source is only a provenance source for
the operator's private file; a key list labelled for another build does not
establish that build 70170's encrypted files can be read.

The complete TACT-key table itself is also an explicit, parameterless opt-in;
it is never extracted by the default command. Add `--ack-tact-key-table` in
any position among the optional flags:

```bash
target/forever-login/client-data-probe-build/forever-client-data-probe \
  --ack-local-client-data /absolute/path/to/World-of-Warcraft \
  /absolute/path/to/rustycore/target/forever-login/new-client-data esES \
  --ack-public-tact-keys /absolute/path/to/rustycore/target/forever-login/tact-keys.txt \
  --ack-tact-key-table
```

This mode reads the complete local `TactKey.db2` through the normal CascLib
path and writes it only to a new private output directory. Before saving, the
probe requires the target WDC5 schema: file data ID `1302850`, table hash
`0xDF2F53CF`, layout hash `0xCBA490FC`, record size 16, one declared and total
field, header `flags=4`, zero parent lookups, and one external ID-list entry
per record in every section. The target header's `id_index=0` is not the
external-ID declaration: `DB2Meta::IndexField=-1` means that C++ uses section
ID tables, while `DB2Header::IndexField` is a separate native header field.
It does not use `CASC_OVERCOME_ENCRYPTED`, download keys, zero-fill missing
data, or print the 16-byte TACT values. The metadata/record contract is
grounded in the pinned reference `DB2FileLoader.h:DB2Header`,
`DB2FileLoader.cpp:LoadHeaders`, `DB2Metadata.h:TactKeyMeta`,
`DB2LoadInfo.h:TactKeyLoadInfo` and `DB2Store.cpp:DB2StorageBase::WriteRecord`
at `02245dcd`; the SQL `tact_key` table is an overlay and is not a substitute
for this normal client baseline.

Character-model and customization tables are a separate, explicit opt-in. They are
never read by the default command. Add `--ack-character-customization-tables` in any
position among the optional flags:

```bash
target/forever-login/client-data-probe-build/forever-client-data-probe \
  --ack-local-client-data /absolute/path/to/World-of-Warcraft \
  /absolute/path/to/rustycore/target/forever-login/new-client-data esES \
  --ack-character-customization-tables
```

The opt-in requests complete files through ordinary local CascLib reads (never
`CASC_OVERCOME_ENCRYPTED`, missing-key zero fill, or a fabricated record). Every
table must have its local section key when a section is encrypted; an unavailable
section aborts before that table is saved. Each output is created with mode `0600`
under a new private output directory and an existing output file is never replaced.
The probe prints only bounded WDC metadata and does not print rows, customization
values, keys, or decrypted blobs.

The six contracts are taken independently from `DB2Metadata.h` at pinned reference
`02245dcd` (the target layout hash is the compatibility gate):

| file | FileDataId | layout hash | DB2Meta FieldCount/FileFieldCount | IndexField/ParentIndexField |
| --- | ---: | ---: | ---: | ---: |
| `ChrModel.db2` | 3384313 | `0x03FAB755` | 17/17 | 2/4 |
| `ChrCustomizationOption.db2` | 3384247 | `0xDCC2A86E` | 13/13 | 1/4 |
| `ChrCustomizationChoice.db2` | 3450554 | `0x9559C358` | 11/11 | 1/2 |
| `ChrCustomizationReq.db2` | 3450453 | `0xCA154412` | 9/9 | -1/-1 |
| `ChrRaceXChrModel.db2` | 3490304 | `0xA203BC29` | 4/4 | -1/0 |
| `ChrCustomizationReqChoice.db2` | 3580359 | `0xF925BC6F` | 2/1 | -1/1 |

For each file the WDC5/version-5 layout hash, declared header field count and
the `DB2FileLoader::LoadHeaders` total-field relationship are checked against that
row. Sparse files are rejected. Section ID-table sizes follow that table's own
`DB2Meta::IndexField`: an in-data index requires zero ID-table bytes, while
`IndexField=-1` requires four bytes per record. This deliberately does not infer
external-ID behavior from TactKey, copy its field count, or compare
`DB2Header::IndexField` to `DB2Meta::IndexField`; the native loader treats those
as distinct contracts. The cross-table relationships are therefore preserved:
race/model rows link `ChrRaceXChrModel` to `ChrModel`, options link models and
requirements, choices link options and requirements, and requirement-choice rows
link choices to requirements. The acquired presence/schema still does not claim
that Rust can decode every field or that character creation/world loading works.

When the full Achievement table is unavailable because its second section has
no local key, an operator may request the bounded known-section artifact with
the parameterless `--ack-available-achievements` option (alone or alongside
the private-key option):

```bash
target/forever-login/client-data-probe-build/forever-client-data-probe \
  --ack-local-client-data /absolute/path/to/World-of-Warcraft \
  /absolute/path/to/rustycore/target/forever-login/new-client-data esES \
  --ack-available-achievements
```

This writes only `Achievement.available.db2`, preserving the original WDC5
prefix through byte 40929. The probe validates the captured Achievement
schema (19 fields, inline ID field 3, table/layout hashes, two sections and
the exact 434-available/9-unavailable boundary) before writing it. It reads
the prefix normally with CascLib, without `CASC_OVERCOME_ENCRYPTED`, and
reports the unknown section's source handling as `Skip`; it never zero-fills
or rewrites a complete `Achievement.db2`. The default command remains strict
and still requests the complete table, so it rejects an unavailable encrypted
section rather than treating this partial artifact as complete data. This
corresponds to the pinned `DB2FileSource::HandleEncryptedSection` / `Skip`
path in `DB2FileLoader.cpp` (`02245dcd`).

Outputs are private client assets, not repository fixtures. Never stage them.
The output root is pinned to this checkout at compilation; merely using a path
containing the fixture name elsewhere, or a symlink escape, is not admitted.
JSON lines report only build/file IDs, byte count, WDC5 format/version, record and
field counts, record size, flags, header ID-field value, total fields, column
metadata size, table/layout hashes and acquisition mode. Header checks follow
`DB2FileLoader::LoadHeaders` at reference `02245dcd`; self-tests use only synthetic
header bytes. They do not establish full record decoding or schema parity.
Select the installed text locale (`esES` or `enUS`). In this pinned CascLib the
locale is applied at storage opening, not by `CascOpenFile`; selecting an
uninstalled locale can report `ERROR_FILE_OFFLINE` even for an installed table.
Extraction does not prove that Rust's old DB2 reader, world database or gameplay
supports this build. Record failures and actual schema evidence in the owning
[Forever runbook](../../../docs/operations/forever-login.md).

The production `wow-data::forever_character_ids` catalog validates the target
WDC5/table/layout/field/ID-source contract. It gives DB2 presence only, not
playable combinations or old `character_progression` field compatibility.
Its read-only example is an explicit actual-file consumer:

```bash
cargo run --locked -j1 -p wow-data --example forever_character_tables -- \
  --ack-local-client-data "$PWD/target/forever-login/new-client-data"
FOREVER_ACK_PRIVATE_DATA_TESTS=1 \
  FOREVER_CHARACTER_TABLES_BIN="$PWD/target/debug/examples/forever_character_tables" \
  FOREVER_CLIENT_DATA_DIRECTORY="$PWD/target/forever-login/new-client-data" \
  python3 tools/wow-test-bot/client-data-probe/test_rust_catalog.py
```

Negative QA changes only temporary private copies, never the extracted source
or installation. No client assets are checked into the tests. Successful ID
decoding still does not establish character selection, creation or world loading.
